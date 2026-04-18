// WebSocket 連線與遊戲主邏輯；登入、房間視野、依出口移動。傳統 MUD 節點連接節點。v0.20.17
(function () {
	const wsScheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const wsUrl = wsScheme + '//' + window.location.host + '/ws';
	const STORAGE_PLAYER_ID = 'singularity_player_id';
	const HEARTBEAT_INTERVAL_MS = 30000;

	let socket = null;
	let heartbeatTimer = null;
	let reconnecting = false;
	let lastPongTime = 0;
	let reconnectDelay = 1500;
	const state = {
		room_id: '',
		room_name: '',
		description: '',
		exits: [],
		entities: [],
		me: null,
		server_unix: 0,
		game_time_sec_since_midnight: 0,
		game_days_since_epoch: 0,
		hexView: null
	};

	const GAME_TIME_SCALE = 24;
	const GAME_SEC_PER_DAY = 86400;
	const DAYS_PER_YEAR = 365;
	const DAYS_PER_MONTH = 30;
	const MONTH_NAMES = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '十一', '十二'];
	const DAY_NAMES = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '十一', '十二', '十三', '十四', '十五', '十六', '十七', '十八', '十九', '二十', '廿一', '廿二', '廿三', '廿四', '廿五', '廿六', '廿七', '廿八', '廿九', '三十'];
	/** 同房實體互動：對齊後端 Borrow / Subdue / Slay（舊 Attack＝送行，後端已映射） */
	const ENTITY_INTERACT_LABELS = { Talk: '對話', Borrow: '借物', Subdue: '留人', Slay: '送行', Trade: '交易', Attack: '攻擊' };
	const ENTITY_INTERACT_ORDER = ['Talk', 'Borrow', 'Trade', 'Subdue', 'Slay'];
	let gameTimeTicker = null;
	var entityNameCache = {}; // name/id -> entity_id（用於 log 人名點擊）
	let logStickToBottom = true;
	let logScrollBound = false;

	function ensureLogScrollBehavior(el) {
		if (!el || logScrollBound) return;
		logScrollBound = true;
		el.addEventListener('scroll', function () {
			var delta = el.scrollHeight - el.clientHeight - el.scrollTop;
			logStickToBottom = delta <= 32;
		}, { passive: true });
	}

	function appendAndMaybeAutoScroll(el, row) {
		if (!el || !row) return;
		ensureLogScrollBehavior(el);
		el.appendChild(row);
		if (logStickToBottom) el.scrollTop = el.scrollHeight;
	}

	function gameSecNow() {
		if (!state.server_unix) return null;
		var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
		var sec = state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		sec = sec % GAME_SEC_PER_DAY;
		if (sec < 0) sec += GAME_SEC_PER_DAY;
		return sec;
	}

	function gameDaysNow() {
		if (!state.server_unix) return null;
		var elapsed = Math.max(0, (Date.now() / 1000) - state.server_unix);
		var secTotal = state.game_days_since_epoch * GAME_SEC_PER_DAY + state.game_time_sec_since_midnight + elapsed * GAME_TIME_SCALE;
		return Math.max(0, Math.floor(secTotal / GAME_SEC_PER_DAY));
	}

	function formatSingularityDate(days) {
		days = Math.max(0, Math.floor(days));
		var dayInYear = days % DAYS_PER_YEAR;
		var year = Math.floor(days / DAYS_PER_YEAR) + 1;
		var month = Math.min(12, Math.floor(dayInYear / DAYS_PER_MONTH) + 1);
		var day = (dayInYear % DAYS_PER_MONTH) + 1;
		if (day < 1) day = 1;
		if (day > 30) day = 30;
		var yearStr = year === 1 ? '元' : (year + '');
		var monthStr = month + '';
		var dayStr = day + '';
		return '奇點曆 ' + yearStr + '年' + monthStr + '月' + dayStr + '日';
	}

	function updateGameTimeDisplay() {
		var sec = gameSecNow();
		var days = gameDaysNow();
		var handEl = document.getElementById('game-time-hand');
		var clockEl = document.getElementById('game-time-clock');
		var labelEl = document.getElementById('game-time-label');
		var dateEl = document.getElementById('game-time-date');
		if (sec == null || days == null) {
			if (dateEl) dateEl.textContent = '奇點曆 —';
			if (clockEl) clockEl.textContent = '--:--';
			if (labelEl) labelEl.textContent = '';
			if (handEl) handEl.setAttribute('transform', 'rotate(0 16 16)');
			return;
		}
		if (handEl) {
			var hourCont = sec / 3600;
			var angle = (hourCont - 12) * 15;
			handEl.setAttribute('transform', 'rotate(' + angle + ' 16 16)');
		}
		if (dateEl) dateEl.textContent = formatSingularityDate(days);
		var h = Math.floor(sec / 3600) % 24;
		var m = Math.floor((sec % 3600) / 60);
		if (clockEl) clockEl.textContent = (h < 10 ? '0' : '') + h + ':' + (m < 10 ? '0' : '') + m;
		var phases = ['深夜','凌晨','破曉','清晨','上午','正午','午後','下午','傍晚','入夜','晚間','半夜'];
		var phase = phases[Math.floor(h / 2)] || '深夜';
		if (labelEl) labelEl.textContent = phase;
	}

	function startGameTimeTicker() {
		if (gameTimeTicker) return;
		gameTimeTicker = setInterval(updateGameTimeDisplay, 500);
	}

	function stopGameTimeTicker() {
		if (gameTimeTicker) {
			clearInterval(gameTimeTicker);
			gameTimeTicker = null;
		}
	}

	// 四條狀態欄：滿條＝該屬性最大值，條寬＝當前值/最大值*100%。
	function updateStatusBars(hpCur, hpMax, innerCur, innerMax, spiritCur, spiritMax, staminaCur, staminaMax) {
		var pct = function (cur, max) {
			if (max == null || max <= 0) return 100;
			var c = Number(cur);
			var m = Number(max);
			return m <= 0 ? 100 : Math.min(100, Math.round((c / m) * 100));
		};
		var barHp = document.getElementById('bar-hp');
		var barSpirit = document.getElementById('bar-spirit');
		var barInner = document.getElementById('bar-inner');
		var barStamina = document.getElementById('bar-stamina');
		if (barHp) barHp.style.width = pct(hpCur, hpMax) + '%';
		if (barSpirit) barSpirit.style.width = pct(spiritCur, spiritMax) + '%';
		if (barInner) barInner.style.width = pct(innerCur, innerMax) + '%';
		if (barStamina) barStamina.style.width = pct(staminaCur, staminaMax) + '%';
	}

	function draw() {
		if (window.mudUpdateRoomView) {
			window.mudUpdateRoomView(state.room_name, state.description, state.exits, state.entities, state.me, state.objects);
		}
		// 出口欄已移除，改由描述內 〔〕 物件點擊觸發 Move；若存在容器則仍可渲染（如 Debug 用）
		if (window.mudRenderExitButtons && document.getElementById('exits-buttons')) {
			window.mudRenderExitButtons('exits-buttons', state.exits, function (direction) {
				if (window.gameSendMoveDirection) window.gameSendMoveDirection(direction);
			});
		}
		var nameEl = document.getElementById('player-name');
		if (nameEl) nameEl.textContent = (state.me && state.me.player_id) ? state.me.player_id : '姓名';
		if (state.me && state.me.hp_max != null) {
			updateStatusBars(state.me.hp_cur, state.me.hp_max, state.me.inner_cur, state.me.inner_max, state.me.spirit_cur, state.me.spirit_max, state.me.stamina_cur, state.me.stamina_max);
		}
	}

	function appendLog(text) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system';
		div.textContent = text;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendNarrative(html, actionType) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-narrative';
		if (actionType) div.classList.add('log-' + actionType.toLowerCase());
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendObjectActionsLine(html) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-object-actions';
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	function indexEntityNameCache(entities) {
		(entities || []).forEach(function (e) {
			var eid = (e.id || e.ID || '').toString();
			if (!eid) return;
			var dname = (e.display_name || '').toString();
			entityNameCache[eid] = eid;
			if (dname) entityNameCache[dname] = eid;
		});
	}

	function resolveEntityIDByName(name) {
		if (!name) return '';
		if (entityNameCache[name]) return entityNameCache[name];
		if (state.entities && state.entities.length) {
			for (var i = 0; i < state.entities.length; i++) {
				var e = state.entities[i];
				var eid = (e.id || e.ID || '').toString();
				var dname = (e.display_name || '').toString();
				if (name === eid || (dname && name === dname)) return eid;
			}
		}
		return '';
	}

	// 單段敘事（表格儲存格不換成 <br>）
	function formatNarrativeSegment(text, asTableCell) {
		if (!text) return '';
		var esc = escapeHtml(String(text))
			.replace(/【([^】]*)】/g, function (_m, rawName) {
				var name = rawName || '';
				var eid = resolveEntityIDByName(name);
				if (eid) {
					return '【<span class="log-object-action narr-name" role="button" tabindex="0" data-entity-id="' + escapeHtml(eid) + '" data-action="Look" data-target-type="entity" data-target-name="' + escapeHtml(name) + '">' + escapeHtml(name) + '</span>】';
				}
				return '<span class="narr-name">【' + escapeHtml(name) + '】</span>';
			})
			.replace(/「([^」]*)」/g, '<span class="narr-dialogue">「$1」</span>');
		if (!asTableCell) {
			esc = esc.replace(/\n/g, '<br>');
		}
		return esc;
	}

	function formatNarrative(text) {
		if (!text) return '';
		if (!window.NarrativeMarkdown || !window.NarrativeMarkdown.splitTextAndTables) {
			return formatNarrativeSegment(text, false);
		}
		var blocks = window.NarrativeMarkdown.splitTextAndTables(text);
		var parts = [];
		for (var bi = 0; bi < blocks.length; bi++) {
			var b = blocks[bi];
			if (b.type === 'table') {
				parts.push(window.NarrativeMarkdown.renderTableHtml(b.rows, function (cell) {
					return formatNarrativeSegment(cell, true);
				}));
			} else {
				parts.push(formatNarrativeSegment(b.text, false));
			}
		}
		return parts.join('');
	}

	// 敘事中的物件名若為當前房間物件則變為可點擊（執行 Move 或 Look）
	// 第一輪：替換 【物件名】 或 〔物件名〕；第二輪：僅在「詞界」替換純文字名（避免「奶茶」命中「琥珀珍珠奶茶」）
	function formatNarrativeWithClickableObjects(html, objects) {
		if (!html || !objects || !objects.length) return html;
		function escapeRegex(s) { return String(s).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }
		var sorted = objects.slice().sort(function (a, b) { return (b.name || '').length - (a.name || '').length; });
		function makeSpan(o) {
			var nameEsc = escapeHtml(o.name);
			var hasMove = o.actions && o.actions.indexOf('Move') !== -1;
			var action = hasMove ? 'Move' : 'Look';
			return '<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(o.id) + '" data-action="' + escapeHtml(action) + '">' + nameEsc + '</span>';
		}
		sorted.forEach(function (o) {
			if (!o.name) return;
			var nameRe = escapeRegex(escapeHtml(o.name));
			html = html.replace(new RegExp('[\u3010\u3014]' + nameRe + '[\u3011\u3015]', 'g'), makeSpan(o));
		});
		// 第二輪：左側不得為漢字或「>」（避免命中複合詞如琥珀珍珠奶茶；避免在已有 span 內再包一層）
		// 右側不得為漢字或「<」（避免切到 </span> 前再度替換）
		sorted.forEach(function (o) {
			if (!o.name) return;
			var nameRe = escapeRegex(escapeHtml(o.name));
			if (!nameRe) return;
			var boundaryRe = new RegExp('(?<![\\u4e00-\\u9fff>])' + nameRe + '(?![\\u4e00-\\u9fff<])', 'g');
			html = html.replace(boundaryRe, function () { return makeSpan(o); });
		});
		return html;
	}

	function isConnected() {
		return socket && socket.readyState === WebSocket.OPEN;
	}

	function startHeartbeat() {
		stopHeartbeat();
		lastPongTime = Date.now();
		if (!document.hidden && isConnected()) {
			heartbeatTimer = setInterval(function () {
				if (document.hidden || !isConnected()) return;
				if (Date.now() - lastPongTime > HEARTBEAT_INTERVAL_MS * 2) {
					appendLog('心跳超時，重新連線…');
					if (socket) socket.close();
					return;
				}
				send({ type: 'ping' });
			}, HEARTBEAT_INTERVAL_MS);
		}
	}

	function stopHeartbeat() {
		if (heartbeatTimer) {
			clearInterval(heartbeatTimer);
			heartbeatTimer = null;
		}
	}

	function connect(options) {
		options = options || {};
		if (socket && socket.readyState !== WebSocket.CLOSED && socket.readyState !== WebSocket.CLOSING) {
			socket.close();
		}
		socket = new WebSocket(wsUrl);
		socket.onopen = function () {
			var wasReconnecting = reconnecting;
			if (reconnecting) reconnecting = false;
			reconnectDelay = 1500;
			appendLog('已連線，請登入');
			var authMsg = document.getElementById('auth-message');
			if (authMsg) authMsg.textContent = '';
			var authHint = document.getElementById('auth-hint');
			if (authHint) authHint.textContent = '已連線，請登入';
			if (wasReconnecting) {
				var idEl = document.getElementById('auth-id');
				var pwEl = document.getElementById('auth-password');
				if (idEl && pwEl && idEl.value.trim() && pwEl.value) {
					send({ type: 'login', player_id: idEl.value.trim(), password: pwEl.value });
				}
			}
		};
		socket.onmessage = function (ev) {
			try {
				const msg = JSON.parse(ev.data);
				switch (msg.type) {
					case 'view':
						state.room_id = msg.room_id || '';
						state.room_name = msg.room_name || '';
						state.description = msg.description || '';
						state.exits = Array.isArray(msg.exits) ? msg.exits : [];
						state.entities = msg.entities || [];
						indexEntityNameCache(state.entities);
						state.objects = Array.isArray(msg.objects) ? msg.objects : [];
						if (typeof msg.server_unix === 'number' && typeof msg.game_time_sec_since_midnight === 'number' && typeof msg.game_days_since_epoch === 'number') {
							var newGameSecAtView = msg.game_days_since_epoch * GAME_SEC_PER_DAY + msg.game_time_sec_since_midnight;
							var currentGameSec = state.server_unix
								? (state.game_days_since_epoch * GAME_SEC_PER_DAY + state.game_time_sec_since_midnight + Math.max(0, (Date.now() / 1000 - state.server_unix)) * GAME_TIME_SCALE)
								: -1;
							if (currentGameSec < 0 || newGameSecAtView >= currentGameSec - 1) {
								state.server_unix = msg.server_unix;
								state.game_time_sec_since_midnight = msg.game_time_sec_since_midnight;
								state.game_days_since_epoch = msg.game_days_since_epoch;
							}
						}
						startGameTimeTicker();
						updateGameTimeDisplay();
						draw();
						// fetchHexView 移至 case 'me'（此處 state.me 尚未設定）
						break;
					case 'grid_view':
						if (window.updateRoomData) window.updateRoomData(msg);
						if (window.onGridView) window.onGridView(msg);
						if (typeof msg.server_unix === 'number' && typeof msg.game_time_sec_since_midnight === 'number') {
							state.server_unix = msg.server_unix;
							state.game_time_sec_since_midnight = msg.game_time_sec_since_midnight;
							state.game_days_since_epoch = msg.game_days_since_epoch;
							startGameTimeTicker();
							updateGameTimeDisplay();
						}
						break;
					case 'me':
						state.me = {
							player_id: msg.player_id,
							room_id: msg.room_id,
							room_name: msg.room_name,
							vit: msg.vit,
							qi: msg.qi,
							dex: msg.dex,
							hp_cur: msg.hp_cur,
							hp_max: msg.hp_max,
							inner_cur: msg.inner_cur,
							inner_max: msg.inner_max,
							spirit_cur: msg.spirit_cur,
							spirit_max: msg.spirit_max,
							stamina_cur: msg.stamina_cur,
							stamina_max: msg.stamina_max,
							display_title: msg.display_title,
							origin_sentence: msg.origin_sentence,
							activated_nodes: msg.activated_nodes || ['N000'],
							topology_costs: msg.topology_costs,
							equipment_slots: msg.equipment_slots || {},
							equipment_names: msg.equipment_names || {},
							equipment_descs: msg.equipment_descs || {}
						};
						if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_PLAYER_ID, msg.player_id);
						window.myPlayerId = msg.player_id;
						showGameAfterLogin();
						updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
						draw();
						appendLog('登入成功：' + msg.player_id + ' @ ' + (msg.room_name || msg.room_id));
						renderStarplatePane(state.me);
						if (!state.hexView) fetchHexView();
	startHeartbeat();
						break;
					case 'pong':
						lastPongTime = Date.now();
						break;
					case 'moved':
						if (state.me && (msg.player_id === state.me.player_id || msg.player_id === state.me.id)) {
							const oldRoomId = state.me.room_id;
							state.me.room_id = msg.room_id;
							state.me.room_name = msg.room_name;

							if (msg.room_id && msg.room_id.startsWith('grid:')) {
								if (window.mapState) window.mapState.moving = false;
							} else if (msg.room_id && msg.room_id.startsWith('hex:') && oldRoomId && oldRoomId !== msg.room_id) {
								const parts = msg.room_id.split(':');
								const nq = parseInt(parts[1]);
								const nr = parseInt(parts[2]);
								if (window.hexCanvas && !isNaN(nq) && !isNaN(nr)) {
									const targetCell = (state.hexView && state.hexView.cells || [])
										.find(c => c.q === nq && c.r === nr);
									const moveCost = (targetCell && targetCell.move_cost) || 1.0;
									const duration = 350 / (60 / moveCost);
									window.hexCanvas.animateTo(nq, nr, duration, function () {
										fetchHexView();
									});
								} else {
									fetchHexView();
								}
							} else {
								fetchHexView();
							}
						}
						draw();
						break;
					case 'blocked':
						if (window.mapState) window.mapState.moving = false;
						if (window.hexCanvas) {
							fetchHexView();
						}
						appendLog('無法往「' + (msg.direction || '') + '」移動');
						break;
					case 'entity_status':
						renderStatusPane(msg);
						renderEquipmentPane(msg);
						renderSkillPane(msg);
						if (msg.is_self) {
							if (msg.hp_max != null) {
								updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
								if (state.me) {
									state.me.vit = msg.vit;
									state.me.qi = msg.qi;
									state.me.dex = msg.dex;
									state.me.hp_cur = msg.hp_cur;
									state.me.hp_max = msg.hp_max;
									state.me.inner_cur = msg.inner_cur;
									state.me.inner_max = msg.inner_max;
									state.me.spirit_cur = msg.spirit_cur;
									state.me.spirit_max = msg.spirit_max;
									state.me.stamina_cur = msg.stamina_cur;
									state.me.stamina_max = msg.stamina_max;
								}
							}
							if (state.me) {
								state.me.display_title = msg.display_title;
								state.me.origin_sentence = msg.origin_sentence;
								state.me.activated_nodes = msg.activated_nodes && msg.activated_nodes.length ? msg.activated_nodes : ['N000'];
								state.me.topology_costs = msg.topology_costs;
								state.me.equipment_slots = msg.equipment_slots || {};
								state.me.equipment_names = msg.equipment_names || {};
								state.me.equipment_descs = msg.equipment_descs || {};
							}
							renderStarplatePane(state.me);
						}
						break;
				case 'action_result':
					if (msg.action === 'Talk') {
						removeLogPendingTalk();
						var narrative = msg.narrative;
						if (!narrative) narrative = '（NPC 無回應）';
						var narrativeHtml = formatNarrative(narrative);
						narrativeHtml = formatNarrativeWithClickableObjects(narrativeHtml, state.objects);
						appendNarrative(narrativeHtml, 'Talk');
						// 對話回覆後直接顯示【對話】【攻擊】【交易】，不用再點一次角色
						var talkTargetId = msg.target_id || '';
						var talkTargetName = msg.target_name || talkTargetId;
						if (talkTargetId) {
							var talkParts = ENTITY_INTERACT_ORDER.map(function (act) {
								var label = ENTITY_INTERACT_LABELS[act] || act;
								return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(talkTargetId) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(talkTargetName) + '">' + escapeHtml(label) + '</span>】';
							});
							appendObjectActionsLine(talkParts.join(''));
						}
					} else if (msg.action === 'Trade') {
						removeLogPendingTrade();
						var tNarrative = msg.narrative;
						if (!tNarrative) tNarrative = '（交易無回應）';
						var tHtml = formatNarrative(tNarrative);
						tHtml = formatNarrativeWithClickableObjects(tHtml, state.objects);
						appendNarrative(tHtml, 'Trade');
						var tradeTargetId = msg.target_id || '';
						var tradeTargetName = msg.target_name || tradeTargetId;
						if (tradeTargetId) {
							var tParts = ENTITY_INTERACT_ORDER.map(function (act) {
								var lab = ENTITY_INTERACT_LABELS[act] || act;
								return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(tradeTargetId) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(tradeTargetName) + '">' + escapeHtml(lab) + '</span>】';
							});
							appendObjectActionsLine(tParts.join(''));
						}
					} else if (msg.action === 'Borrow') {
						var bNarrative = msg.narrative;
						if (!bNarrative) bNarrative = '（借物無回應）';
						var bHtml = formatNarrative(bNarrative);
						bHtml = formatNarrativeWithClickableObjects(bHtml, state.objects);
						appendNarrative(bHtml, 'Borrow');
						var borrowTargetId = msg.target_id || '';
						var borrowTargetName = msg.target_name || borrowTargetId;
						if (borrowTargetId) {
							var bParts = ENTITY_INTERACT_ORDER.map(function (act) {
								var bl = ENTITY_INTERACT_LABELS[act] || act;
								return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(borrowTargetId) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(borrowTargetName) + '">' + escapeHtml(bl) + '</span>】';
							});
							appendObjectActionsLine(bParts.join(''));
						}
					} else if (msg.narrative) {
						var narrativeHtml = formatNarrative(msg.narrative);
						narrativeHtml = formatNarrativeWithClickableObjects(narrativeHtml, state.objects);
						appendNarrative(narrativeHtml, msg.action);
						// 觀看後下一行顯示可執行的其他動作（物件：移動/閱讀…；人物：對話/攻擊）
						if (msg.action === 'Look') {
							var actionLabels = { 'Read': '閱讀', 'Smell': '嗅聞', 'Use': '使用', 'Open': '開啟', 'Sit': '坐下', 'Taste': '品嚐', 'Take': '拾取', 'Chop': '砍伐', 'Operate': '操作', 'Talk': '對話', 'Borrow': '借物', 'Subdue': '留人', 'Slay': '送行', 'Trade': '交易', 'Attack': '攻擊', 'Move': '移動' };
							var others = [];
							var targetId = msg.target_id || '';
							// 優先使用後端在 action_result 帶上的 actions（建築 Look 後必有【移動】等）
							if (msg.actions && Array.isArray(msg.actions) && msg.actions.length) {
								others = msg.actions;
							} else {
								var obj = null;
								if (state.objects && state.objects.length) {
									for (var i = 0; i < state.objects.length; i++) {
										if (state.objects[i].id === msg.target_id || state.objects[i].name === msg.target_name) {
											obj = state.objects[i];
											break;
										}
									}
								}
								if (obj && obj.actions) {
									others = obj.actions.filter(function (a) { return a !== 'Look'; });
									targetId = obj.id;
								} else if (state.entities && state.entities.length) {
									for (var j = 0; j < state.entities.length; j++) {
										if (state.entities[j].id === msg.target_id) {
											var ent = state.entities[j];
											if (ent.actions) {
												others = ent.actions.filter(function (a) { return a !== 'Look'; });
												targetId = ent.id;
											}
											break;
										}
									}
								}
							}
							if (others.length) {
								var targetName = msg.target_name || msg.target_id || '';
								var parts = others.map(function (act) {
									var label = actionLabels[act] || act;
									// 建築名 Look 後後端帶 move_target_id 時，僅【移動】對該 id 送 do_action（同房的門／簾）
									var idForAction = (act === 'Move' && msg.move_target_id) ? msg.move_target_id : targetId;
									return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(idForAction) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(targetName) + '">' + escapeHtml(label) + '</span>】';
								});
								appendObjectActionsLine(parts.join(''));
							}
						} else if (msg.action === 'Subdue' || msg.action === 'Slay') {
							var combatTid = msg.target_id || '';
							var combatTname = msg.target_name || combatTid;
							if (combatTid) {
								var combatParts = ENTITY_INTERACT_ORDER.map(function (act) {
									var cx = ENTITY_INTERACT_LABELS[act] || act;
									return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(combatTid) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(combatTname) + '">' + escapeHtml(cx) + '</span>】';
								});
								appendObjectActionsLine(combatParts.join(''));
							}
						}
					}
					break;
				case 'narrate':
					if (msg.text) {
						appendNarrative(formatNarrative(msg.text), 'ambient');
					}
					break;
				case 'inventory':
					renderInventoryContent(msg);
					break;
				case 'error':
					removeLogPendingTalk();
					removeLogPendingTrade();
					appendLog('錯誤：' + msg.message);
					if (!state.me) {
						var authMsg = document.getElementById('auth-message');
						if (authMsg) authMsg.textContent = msg.message;
					}
					break;
					default:
						appendLog('收到：' + ev.data);
				}
			} catch (e) {
				appendLog('收到：' + ev.data);
			}
		};
		socket.onclose = function () {
			stopHeartbeat();
			state.me = null;
			showAuthScreen();
			var delay = reconnectDelay;
			reconnectDelay = Math.min(reconnectDelay * 2, 30000);
			appendLog('連線關閉，' + (delay / 1000).toFixed(1) + '秒後重新連線…');
			var authHint = document.getElementById('auth-hint');
			if (authHint) authHint.textContent = '連線中…';
			setTimeout(function () { tryReconnect(); }, delay);
		};
		socket.onerror = function () {
			appendLog('連線錯誤');
			var authHint = document.getElementById('auth-hint');
			if (authHint) authHint.textContent = '連線失敗，重試中…';
		};
	}

	function tryReconnect() {
		if (isConnected()) return;
		reconnecting = true;
		appendLog('重新連線中…');
		connect({ reconnect: true });
	}

	function showGameAfterLogin() {
		var authScreen = document.getElementById('auth-screen');
		var app = document.getElementById('app');
		if (authScreen) authScreen.setAttribute('hidden', '');
		if (app) app.removeAttribute('hidden');
	}

	function showAuthScreen() {
		var authScreen = document.getElementById('auth-screen');
		var app = document.getElementById('app');
		if (authScreen) authScreen.removeAttribute('hidden');
		if (app) app.setAttribute('hidden', '');
	}

	function bindAuthForm() {
		var form = document.getElementById('auth-form');
		var authScreen = document.getElementById('auth-screen');
		var app = document.getElementById('app');
		if (!form) return;
		if (state.me) {
			if (authScreen) authScreen.setAttribute('hidden', '');
			if (app) app.removeAttribute('hidden');
		} else {
			if (authScreen) authScreen.removeAttribute('hidden');
			if (app) app.setAttribute('hidden', '');
		}
		form.addEventListener('submit', function (e) {
			e.preventDefault();
			var authMsg = document.getElementById('auth-message');
			if (authMsg) authMsg.textContent = '';
			var idEl = document.getElementById('auth-id');
			var pwEl = document.getElementById('auth-password');
			var id = (idEl && idEl.value) ? idEl.value.trim() : '';
			var password = pwEl ? pwEl.value : '';
			if (!id || !password) {
				if (authMsg) authMsg.textContent = '請填寫 ID 與密碼';
				return;
			}
			if (!socket || socket.readyState !== WebSocket.OPEN) {
				if (authMsg) authMsg.textContent = '請稍候連線後再登入';
				return;
			}
			// 依目前顯示的區塊判斷（按 Enter 時 submitter 可能是第一個按鈕「登入」，會誤送 login）
			var createPanel = document.getElementById('auth-create-actions');
			var isCreate = createPanel && !createPanel.hasAttribute('hidden');
			if (isCreate) {
				var displayChar = (document.getElementById('auth-display-char') && document.getElementById('auth-display-char').value) ? document.getElementById('auth-display-char').value.trim() : '';
				var genderRadio = form.querySelector('input[name="gender"]:checked');
				var gender = (genderRadio && genderRadio.value) ? genderRadio.value : '男';
				if (password.length < 6) {
					if (authMsg) authMsg.textContent = '密碼至少 6 個字元';
					return;
				}
				send({ type: 'create_character', player_id: id, password: password, display_char: displayChar, gender: gender });
			} else {
				send({ type: 'login', player_id: id, password: password });
			}
		});
		document.getElementById('auth-btn-switch').addEventListener('click', function () {
			document.getElementById('auth-hint').textContent = '建立新角色（ID 與密碼登入用）';
			document.getElementById('auth-display-wrap').removeAttribute('hidden');
			document.getElementById('auth-gender-wrap').removeAttribute('hidden');
			document.getElementById('auth-login-actions').setAttribute('hidden', '');
			document.getElementById('auth-create-actions').removeAttribute('hidden');
		});
		document.getElementById('auth-btn-back').addEventListener('click', function () {
			document.getElementById('auth-hint').textContent = '請輸入 ID 與密碼登入';
			document.getElementById('auth-display-wrap').setAttribute('hidden', '');
			document.getElementById('auth-gender-wrap').setAttribute('hidden', '');
			document.getElementById('auth-login-actions').removeAttribute('hidden');
			document.getElementById('auth-create-actions').setAttribute('hidden', '');
		});
	}

	async function fetchHexView() {
		console.log('[fetchHexView] called, me=', state.me && state.me.player_id, 'hexCanvas=', !!window.hexCanvas);
		if (!state.me || !state.me.player_id) return;
		try {
			const resp = await fetch('/api/hex/view?player_id=' + encodeURIComponent(state.me.player_id));
			if (resp.ok) {
				const data = await resp.json();
				console.log('[fetchHexView] got data, cells=', data.cells && data.cells.length, 'entities=', data.entities && data.entities.length);
				
				// 處理黑格、灰霧格、彩格狀態
				if (data.cells) {
					data.cells.forEach(c => {
						// explored=false 且可走 -> 灰霧 (fogged)
						// 不可走地形或 explored=true -> 彩格 (非 fogged)
						if (!c.explored && c.walkable) {
							c.fogged = true;
						} else {
							c.fogged = false;
						}
					});
				}

				state.hexView = data;
				if (window.hexCanvas) {
					window.hexCanvas.updateView(data, state.me.player_id);
				} else {
					console.warn('[fetchHexView] window.hexCanvas is not set!');
				}
			}
		} catch (e) {
			console.error('fetchHexView failed', e);
		}
	}

	window.onBlackHexBoundaryStop = async function (q, r) {
		if (!state.me) return;
		const key = `${q},${r}`;
		// 防止重複發送偵查請求
		if (state.lastScouting === key) return;
		state.lastScouting = key;

		appendLog(`偵查未知區域 (${q}, ${r})…`);
		try {
			const resp = await fetch('/api/hex/scout', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ player_id: state.me.player_id, target_q: q, target_r: r })
			});
			if (resp.ok) {
				const res = await resp.json();
				if (res.ok) {
					appendLog('發現新地形：' + (res.cell ? res.cell.name : '未知'));
					await fetchHexView();
				}
			}
		} catch (e) {
			console.error('scout failed', e);
		} finally {
			setTimeout(() => { state.lastScouting = null; }, 2000);
		}
	};

	window.onHexCrossBoundary = async function (q, r) {
		if (!state.me) return;
		// 預先更新 room_id，防止 WS moved 事件重複觸發 animateTo
		state.me.room_id = 'hex:' + q + ':' + r;
		// 跨格移動同步
		try {
			const resp = await fetch('/api/hex/move', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ player_id: state.me.player_id, to_q: q, to_r: r })
			});
			if (resp.ok) {
				const res = await resp.json();
				if (res.ok) {
					// 跨格成功，刷新視野取得新格周圍資料
					await fetchHexView();
				}
			}
		} catch (e) {
			console.error('hex move failed', e);
		}
	};

	window.onHexExploreComplete = async function (q, r) {
		if (!state.me) return;
		appendLog('精探完成，地形勘測完畢。');
		try {
			const resp = await fetch('/api/hex/explore', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ player_id: state.me.player_id, target_q: q, target_r: r })
			});
			if (resp.ok) {
				await fetchHexView();
			}
		} catch (e) {
			console.error('explore failed', e);
		}
	};

	let hexClickTimer = null;
	window.gameOnHexClick = function (q, r, relX, relY) {
		if (!state.hexView || !state.hexView.center) return;
		const current = state.hexView.center;

		// 不管點在哪格，都算出相對於玩家所在格中心的偏移（用於格內移動方向）
		var intraRelX = relX;
		var intraRelY = relY;
		if (q !== current.q || r !== current.r) {
			// 點擊在其他格：用點擊的地圖絕對位置減去玩家格中心，得到方向
			var HEX_R = 202;
			var SQRT3 = Math.sqrt(3);
			var clickAbsX = HEX_R * SQRT3 * (q + r / 2) + relX;
			var clickAbsY = HEX_R * 3 / 2 * r + relY;
			var playerCenterX = HEX_R * SQRT3 * (current.q + current.r / 2);
			var playerCenterY = HEX_R * 3 / 2 * current.r;
			intraRelX = clickAbsX - playerCenterX;
			intraRelY = clickAbsY - playerCenterY;
		}

		if (hexClickTimer) {
			clearTimeout(hexClickTimer);
			hexClickTimer = null;
			// 雙擊：快跑 (100 px/s)
			if (window.hexCanvas.startIntraHexMove) {
				window.hexCanvas.startIntraHexMove(intraRelX, intraRelY, true);
			}
		} else {
			hexClickTimer = setTimeout(() => {
				hexClickTimer = null;
				// 單擊：步行 (50 px/s)
				if (window.hexCanvas.startIntraHexMove) {
					window.hexCanvas.startIntraHexMove(intraRelX, intraRelY, false);
				}
			}, 300);
		}
	};

	function send(obj) {
		if (socket && socket.readyState === WebSocket.OPEN) {
			socket.send(JSON.stringify(obj));
		}
	}

	function sendMoveByDirection(direction) {
		if (!direction) {
			appendLog('請選擇出口');
			return;
		}
		appendLog('往「' + direction + '」移動');
		send({ type: 'move', direction: direction });
	}

	function fmtNum(x) {
		if (x == null || x === '') return '—';
		var n = Number(x);
		return isNaN(n) ? '—' : Math.round(n);
	}
	function renderStatusPane(msg) {
		var wrap = document.getElementById('status-content');
		if (!wrap) return;
		var isSelf = msg.is_self === true;
		var vit = msg.vit != null ? msg.vit : '—';
		var qi = msg.qi != null ? msg.qi : '—';
		var dex = msg.dex != null ? msg.dex : '—';
		var title = (msg.display_title && msg.display_title.trim()) ? msg.display_title.trim() : '無名之輩';
		var origin = (msg.origin_sentence && msg.origin_sentence.trim()) ? msg.origin_sentence.trim() : '';
		var html = '<dl class="status-dl">';
		html += '<dt>[ 命途 ]</dt><dd>' + escapeHtml(title) + '</dd>';
		if (origin) {
			html += '<dt>[ 本源 ]</dt><dd>「' + escapeHtml(origin) + '」</dd>';
		}
		html += '<dt>[ 維度 ]</dt><dd>體質 ' + vit + ' ｜ 氣脈 ' + qi + ' ｜ 靈敏 ' + dex + '</dd>';
		html += '<dt>[ 四相 ]</dt><dd class="status-four">'
			+ '<span>氣血 ' + fmtNum(msg.hp_cur) + '/' + fmtNum(msg.hp_max) + '</span>'
			+ '<span>內力 ' + fmtNum(msg.inner_cur) + '/' + fmtNum(msg.inner_max) + '</span>'
			+ '<span>精神 ' + fmtNum(msg.spirit_cur) + '/' + fmtNum(msg.spirit_max) + '</span>'
			+ '<span>體力 ' + fmtNum(msg.stamina_cur) + '/' + fmtNum(msg.stamina_max) + '</span>'
			+ '</dd>';
		html += '<dt>[ 持有 ]</dt><dd>鎂：' + (isSelf && msg.magnesium != null ? msg.magnesium : '—') + '</dd>';
		html += '</dl>';
		wrap.innerHTML = html;
	}
	var EQUIP_SLOTS = [
	['head', '【冠盔】'], ['face', '【面甲】'], ['neck', '【護頸】'],
	['undershirt', '【襯衣】'], ['inner_armor', '【內甲】'], ['body', '【掛鎧】'], ['cloak', '【披風】'],
	['shoulder', '【肩鎧】'], ['arm', '【護臂】'], ['wrist', '【護腕】'], ['hand', '【掌套】'],
	['waist', '【腰鎧】'], ['legs', '【襯褲】'], ['leg_armor', '【腿鎧】'], ['feet', '【護靴】'],
	['ring_l', '【左指】'], ['ring_r', '【右指】'], ['trinket', '【佩掛】'],
	['hold_l', '【左持】'], ['hold_r', '【右持】']
	];
	var lastEquipMsg = null;

	function renderEquipmentPane(msg) {
		lastEquipMsg = msg;
		var wrap = document.getElementById('player-modal-pane-equip');
		if (!wrap) return;
		var names = msg.equipment_names || {};
		var slots = msg.equipment_slots || {};
		var descs = msg.equipment_descs || {};
		wrap.innerHTML = '';
		var dl = document.createElement('dl');
		dl.className = 'status-dl equip-dl';
		for (var i = 0; i < EQUIP_SLOTS.length; i++) {
			(function (code, label) {
				var dt = document.createElement('dt');
				dt.textContent = label;
				dl.appendChild(dt);

				var dd = document.createElement('dd');
				var itemName = names[code];
				var itemID = slots[code];
				if (itemName && itemID) {
					dd.className = 'equip-has-item';
					dd.innerHTML = '\u25b8 ' + escapeHtml(itemName);
					dd.addEventListener('click', function () {
						var existing = dl.querySelector('.equip-item-expand');
						var wasThis = existing && existing.getAttribute('data-slot') === code;
						if (existing) existing.remove();
						dl.querySelectorAll('dd.expanded').forEach(function (el) {
							el.classList.remove('expanded');
							el.innerHTML = el.innerHTML.replace('\u25be', '\u25b8');
						});
						if (wasThis) return;

						dd.classList.add('expanded');
						dd.innerHTML = dd.innerHTML.replace('\u25b8', '\u25be');
						var expand = document.createElement('div');
						expand.className = 'equip-item-expand';
						expand.setAttribute('data-slot', code);
						var desc = descs[code] || '';
						var descHtml = desc ? '<div class="equip-item-desc">\u2503 ' + escapeHtml(desc) + '</div>' : '';
						var actionsHtml = '<div class="inventory-item-actions">';
						actionsHtml += '<button type="button" class="inv-action-btn" data-action="unequip">\u8131\u4e0b</button>';
						actionsHtml += '</div>';
						expand.innerHTML = descHtml + actionsHtml;

						dd.after(expand);

						expand.querySelector('.inv-action-btn[data-action="unequip"]').addEventListener('click', function (e) {
							e.stopPropagation();
							send({ type: 'unequip_item', slot: code });
						});
					});
				} else {
					dd.innerHTML = '<span class="text-muted">(\u7a7a)</span>';
				}
				dl.appendChild(dd);
			})(EQUIP_SLOTS[i][0], EQUIP_SLOTS[i][1]);
		}
		wrap.appendChild(dl);
	}
	function renderSkillPane(msg) {
		var wrap = document.getElementById('skill-content');
		if (!wrap) return;
		var html = '<div class="skill-section">';
		html += '<div class="skill-section-title">【 運轉功法 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder">▣ 心法：<span class="text-muted">(未習得)</span></div>';
		html += '<div class="skill-divider"></div>';
		html += '<div class="skill-placeholder"><span class="text-muted">(尚無外功)</span></div>';
		html += '</div></div>';

		html += '<div class="skill-section">';
		html += '<div class="skill-section-title">【 實戰招式池 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder"><span class="text-muted">(無招式)</span></div>';
		html += '</div></div>';

		html += '<div class="skill-section">';
		html += '<div class="skill-section-title">【 語境推演 】</div>';
		html += '<div class="skill-section-body">';
		html += '<div class="skill-placeholder"><span class="text-muted">(無推演)</span></div>';
		html += '</div></div>';

		wrap.innerHTML = html;
	}

	function escapeHtml(s) {
		if (!s) return '';
		var div = document.createElement('div');
		div.textContent = s;
		return div.innerHTML;
	}

	// 二十主樞（361 規格 §2.2）：代碼 N001～N020、名稱
	var HUB_NAMES = ['天極', '脈衝', '震淵', '游離', '弦絲', '曜核', '凜晶', '淵流', '萬象', '解離', '鎮閾', '衡定', '穹壁', '重塑', '逆熵', '神淵', '識閾', '坍縮', '無相', '越權'];
	// Cost 文字化五級（狀態與星盤分頁規格 §5.5）
	function costToLabel(cost) {
		if (cost == null) return { text: '未知', css: 'cost-unknown' };
		if (cost <= 7)  return { text: '暢流', css: 'cost-flow' };
		if (cost <= 11) return { text: '順通', css: 'cost-easy' };
		if (cost <= 16) return { text: '平穩', css: 'cost-mid' };
		if (cost <= 21) return { text: '滯澀', css: 'cost-slow' };
		return { text: '險阻', css: 'cost-hard' };
	}
	function costSpan(cost) {
		var label = costToLabel(cost);
		var val = cost != null ? ' (' + cost.toFixed(2) + ')' : '';
		return '<span class="' + label.css + '">' + label.text + val + '</span>';
	}
	// 760 邊序（361 §6.1.0）：型 A 0..19，型 B 20..119，型 C 120..419，型 D 420..659，型 E 660..759
	function getTypeCCosts(costs, hubIndex) {
		if (!costs || costs.length < 120 + (hubIndex + 1) * 15) return [];
		return costs.slice(120 + hubIndex * 15, 120 + hubIndex * 15 + 15);
	}
	function getCostA(costs, hubIndex) {
		if (!costs || costs.length <= hubIndex) return null;
		return costs[hubIndex];
	}
	function getCostB(costs, hubIndex, blueIdx) {
		var i = 20 + hubIndex * 5 + blueIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	function getCostD(costs, hubIndex, greenIdx) {
		var i = 420 + hubIndex * 12 + greenIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	function getCostE(costs, hubIndex, blueIdx) {
		var i = 660 + hubIndex * 5 + blueIdx;
		return (costs && costs.length > i) ? costs[i] : null;
	}
	var BLUE_NAMES = ['\u8d77', '\u627f', '\u8f49', '\u5354', '\u5408'];
	var GREEN_LABELS = ['G01 \u63a2', 'G02 \u89f8', 'G03 \u7d0d', 'G04 \u84c4', 'G05 \u6ffe', 'G06 \u6790', 'G07 \u878d', 'G08 \u884d', 'G09 \u5f8b', 'G10 \u675f', 'G11 \u91cb', 'G12 \u6563'];
	var GREEN_SHORT = ['G01','G02','G03','G04','G05','G06','G07','G08','G09','G10','G11','G12'];
	var GREEN_PER_BLUE = [[0, 1, 2], [2, 3, 4], [4, 5, 6], [7, 8, 9], [9, 10, 11]];
	var GREEN_SHARED = { 2: ' (\u8207[\u627f]\u5171\u7528)', 4: ' (\u8207[\u8f49]\u5171\u7528)', 9: ' (\u8207[\u5408]\u5171\u7528)' };

	function buildHubExpandHTML(hubIndex, costs) {
		var costA = getCostA(costs, hubIndex);
		var typeCCosts = getTypeCCosts(costs, hubIndex);
		var lines = '<div class="starplate-hub-cost">\u2503 \u62b5\u9054\u672c\u4e3b\u6a1e \u2500 ' + costSpan(costA) + '</div>';
		var idx = 0;
		for (var b = 0; b < 5; b++) {
			var costB = getCostB(costs, hubIndex, b);
			var nextBlue = BLUE_NAMES[(b + 1) % 5];
			var ringE = getCostE(costs, hubIndex, b);
			lines += '<div class="starplate-blue">\u2503 \ud83d\udd35 [' + BLUE_NAMES[b] + '] \u908f\u8f2f\u9598 \u2500 ' + costSpan(costB) + ' (\u672a\u8cab\u901a) <span class="ring-cost">\u27f3\u2192[' + nextBlue + '] ' + costSpan(ringE) + '</span></div>';
			for (var g = 0; g < 3; g++) {
				var greenIdx = GREEN_PER_BLUE[b][g];
				var costC = typeCCosts.length > idx ? typeCCosts[idx] : null;
				var ringD = getCostD(costs, hubIndex, greenIdx);
				var nextGreen = GREEN_SHORT[(greenIdx + 1) % 12];
				var shared = GREEN_SHARED[greenIdx] || '';
				lines += '<div class="starplate-green">\u2503 \u3000\u251c\u2500 \ud83d\udfe2 ' + GREEN_LABELS[greenIdx] + ' \u2500 ' + costSpan(costC) + '\uff1a[ \u7a7a ]' + shared + ' <span class="ring-cost">\u27f3\u2192' + nextGreen + ' ' + costSpan(ringD) + '</span></div>';
				idx++;
			}
		}
		return lines;
	}

	function renderStarplatePane(me) {
		var wrap = document.getElementById('starplate-content');
		if (!wrap) return;
		if (!me || !me.activated_nodes || !Array.isArray(me.activated_nodes)) {
			wrap.innerHTML = '<p class="text-muted">\u50c5\u81ea\u5df1\u53ef\u89c0\u770b\u661f\u76e4\u3002\u8acb\u5148\u767b\u5165\u4e26\u9ede\u64ca\u81ea\u5df1\u958b\u555f\u3002</p>';
			return;
		}
		var activated = me.activated_nodes;
		var count = activated.length;
		var costs = me.topology_costs || [];

		wrap.innerHTML = '';
		var header = document.createElement('div');
		header.className = 'starplate-block';
		header.innerHTML = '<strong>[ \u661f\u76e4\u8cab\u901a\u7387 ]</strong> ' + count + ' / 360';
		wrap.appendChild(header);

		var origin = document.createElement('div');
		origin.className = 'starplate-block';
		origin.innerHTML = '<strong>[ \u6e90\u59cb ]</strong> N000 \u751f\u4e4b\u5947\u9ede (\u5df2\u9ede\u4eae)';
		wrap.appendChild(origin);

		var hubTitle = document.createElement('div');
		hubTitle.className = 'starplate-block';
		hubTitle.innerHTML = '<strong>\u4e8c\u5341\u4e3b\u6a1e</strong>';
		wrap.appendChild(hubTitle);

		var list = document.createElement('ul');
		list.className = 'starplate-hub-list';
		wrap.appendChild(list);

		for (var i = 0; i < HUB_NAMES.length; i++) {
			(function (hubIndex) {
				var nodeId = 'N' + String(hubIndex + 1).padStart(3, '0');
				var costA = getCostA(costs, hubIndex);
				var adaptStr = costA != null ? costSpan(costA) : '\u672a\u77e5';

				var li = document.createElement('li');
				li.className = 'starplate-hub-row';
				li.setAttribute('data-hub', hubIndex);
				li.innerHTML = '\u25b8 [' + HUB_NAMES[hubIndex] + '] ' + nodeId + ' \uff5c \u9069\u6027\uff1a' + adaptStr + ' \uff5c \u8cab\u901a\uff1a0/17';
				list.appendChild(li);

				li.addEventListener('click', function () {
					var existing = list.querySelector('.starplate-hub-expand');
					var wasThis = existing && existing.getAttribute('data-hub') === String(hubIndex);
					if (existing) {
						var prevLi = existing.previousElementSibling;
						if (prevLi) {
							prevLi.classList.remove('expanded');
							prevLi.innerHTML = prevLi.innerHTML.replace('\u25be', '\u25b8');
						}
						existing.remove();
					}
					list.querySelectorAll('.starplate-hub-row.expanded').forEach(function (el) {
						el.classList.remove('expanded');
						el.innerHTML = el.innerHTML.replace('\u25be', '\u25b8');
					});
					if (wasThis) return;

					li.classList.add('expanded');
					li.innerHTML = li.innerHTML.replace('\u25b8', '\u25be');
					var expand = document.createElement('li');
					expand.className = 'starplate-hub-expand';
					expand.setAttribute('data-hub', hubIndex);
					expand.innerHTML = buildHubExpandHTML(hubIndex, costs);
					li.after(expand);
				});
			})(i);
		}
	}

	var lastInventoryMsg = null;

	function renderInventoryContent(msg) {
		lastInventoryMsg = msg;
		var weightEl = document.getElementById('inventory-weight');
		var listEl = document.getElementById('inventory-list');
		if (!weightEl || !listEl) return;
		var cur = msg.current_weight != null ? msg.current_weight.toFixed(1) : '0.0';
		var max = msg.max_weight != null ? msg.max_weight.toFixed(1) : '0.0';
		var overweight = msg.current_weight > msg.max_weight;
		weightEl.innerHTML = '負重：<span class="' + (overweight ? 'inventory-weight-warn' : '') + '">' + cur + ' / ' + max + '</span>';
		if (!msg.items || msg.items.length === 0) {
			listEl.innerHTML = '<div class="inventory-empty">（背包空空如也）</div>';
			return;
		}
		listEl.innerHTML = '';
		var expandedItemId = null;
		for (var i = 0; i < msg.items.length; i++) {
			(function (it) {
				var qtyStr = it.qty > 1 ? (' \u00d7' + it.qty) : '';
				var wStr = it.sub_total != null ? it.sub_total.toFixed(2) : '\u2014';
				var row = document.createElement('div');
				row.className = 'inventory-item';
				row.innerHTML = '<span class="inventory-item-name">\u25b8 ' + escapeHtml(it.name) + qtyStr + '</span>'
					+ '<span class="inventory-item-weight">(' + wStr + ')</span>';
				listEl.appendChild(row);

				row.addEventListener('click', function () {
					var existing = listEl.querySelector('.inventory-item-expand');
					var wasThisItem = existing && existing.getAttribute('data-item-id') === it.item_id;
					if (existing) {
						var prevRow = existing.previousElementSibling;
						if (prevRow) {
							prevRow.classList.remove('expanded');
							var pn = prevRow.querySelector('.inventory-item-name');
							if (pn) pn.innerHTML = pn.innerHTML.replace('\u25be', '\u25b8');
						}
						existing.remove();
					}
					if (wasThisItem) return;

					listEl.querySelectorAll('.inventory-item.expanded').forEach(function (el) {
						el.classList.remove('expanded');
						var en = el.querySelector('.inventory-item-name');
						if (en) en.innerHTML = en.innerHTML.replace('\u25be', '\u25b8');
					});
					row.classList.add('expanded');
					var nameSpan = row.querySelector('.inventory-item-name');
					if (nameSpan) nameSpan.innerHTML = nameSpan.innerHTML.replace('\u25b8', '\u25be');

					var expand = document.createElement('div');
					expand.className = 'inventory-item-expand';
					expand.setAttribute('data-item-id', it.item_id);
					var desc = it.description || '';
					var descHtml = desc ? '<div class="inventory-item-desc">\u2503 ' + escapeHtml(desc) + '</div>' : '';
					var actionsHtml = '<div class="inventory-item-actions">';
					if (it.item_type === 'equipment' && it.slot) {
						if (it.slot === 'hold') {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_l">\u5de6\u624b</button>';
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip" data-target="hold_r">\u53f3\u624b</button>';
						} else {
							actionsHtml += '<button type="button" class="inv-action-btn" data-action="equip">\u7a7f\u6234</button>';
						}
					}
					actionsHtml += '<button type="button" class="inv-action-btn" data-action="drop" disabled>\u4e1f\u68c4</button>';
					actionsHtml += '</div>';
					expand.innerHTML = descHtml + actionsHtml;
					row.after(expand);

					expand.querySelectorAll('.inv-action-btn[data-action="equip"]').forEach(function (btn) {
						btn.addEventListener('click', function (e) {
							e.stopPropagation();
							var payload = { type: 'equip_item', item_id: it.item_id };
							var target = btn.getAttribute('data-target');
							if (target) payload.target_slot = target;
							send(payload);
						});
					});
				});
			})(msg.items[i]);
		}
	}

	function initInventoryModal() {
		var overlay = document.getElementById('inventory-modal-overlay');
		var closeBtn = document.getElementById('inventory-modal-close');
		var openBtn = document.getElementById('btn-inventory');
		if (!overlay || !openBtn) return;

		function openInventory() {
			if (!state.me) return;
			overlay.removeAttribute('hidden');
			overlay.setAttribute('aria-hidden', 'false');
			document.body.style.overflow = 'hidden';
			if (closeBtn) closeBtn.focus();
			document.addEventListener('keydown', onInvKeydown);
			if (isConnected()) send({ type: 'get_inventory' });
		}
		function closeInventory() {
			overlay.setAttribute('hidden', '');
			overlay.setAttribute('aria-hidden', 'true');
			document.body.style.overflow = '';
			document.removeEventListener('keydown', onInvKeydown);
		}
		function onInvKeydown(e) {
			if (e.key === 'Escape') closeInventory();
		}
		openBtn.addEventListener('click', openInventory);
		if (closeBtn) closeBtn.addEventListener('click', closeInventory);
		overlay.addEventListener('click', function (e) {
			if (e.target === overlay) closeInventory();
		});
	}

	function initPlayerModal() {
		var overlay = document.getElementById('player-modal-overlay');
		var modal = document.getElementById('player-modal');
		var titleEl = document.getElementById('player-modal-title');
		var playerName = document.getElementById('player-name');
		var closeBtn = document.getElementById('player-modal-close');
		var logEl = document.querySelector('.log-content');
		var tabs = document.querySelectorAll('.player-modal-tab');
		var panes = document.querySelectorAll('.player-modal-pane');
		if (!overlay || !modal || !playerName) return;

		function openModal(displayName, entityId) {
			if (titleEl) titleEl.textContent = (displayName && displayName.trim()) ? displayName.trim() : '角色';
			var id = (entityId && entityId.trim()) ? entityId.trim() : (state.me && state.me.player_id ? state.me.player_id : '');
			var starplateWrap = document.getElementById('starplate-content');
			if (starplateWrap && id && state.me && state.me.player_id && id !== state.me.player_id) {
				starplateWrap.innerHTML = '<p class="text-muted">僅自己可觀看星盤。</p>';
			}
			if (id && isConnected()) {
				send({ type: 'get_entity_status', entity_id: id });
			} else {
				var wrap = document.getElementById('status-content');
				if (wrap) wrap.innerHTML = '<p class="text-muted">請先登入</p>';
			}
			if (logEl) {
				var h = logEl.clientHeight;
				if (h > 0) modal.style.height = h + 'px';
			}
			overlay.removeAttribute('hidden');
			overlay.setAttribute('aria-hidden', 'false');
			document.body.style.overflow = 'hidden';
			closeBtn.focus();
			document.addEventListener('keydown', onModalKeydown);
		}
		function closeModal() {
			overlay.setAttribute('hidden', '');
			overlay.setAttribute('aria-hidden', 'true');
			document.body.style.overflow = '';
			document.removeEventListener('keydown', onModalKeydown);
			if (playerName) playerName.focus();
		}
		function onModalKeydown(e) {
			if (e.key === 'Escape') closeModal();
		}
		playerName.addEventListener('click', function () {
			var myId = state.me && state.me.player_id;
			openModal(playerName.textContent || myId || '', myId || '');
		});
		playerName.addEventListener('keydown', function (e) {
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault();
				var myId = state.me && state.me.player_id;
				openModal(playerName.textContent || myId || '', myId || '');
			}
		});
		closeBtn.addEventListener('click', closeModal);
		overlay.addEventListener('click', function (e) {
			if (e.target === overlay) closeModal();
		});
		tabs.forEach(function (tab) {
			tab.addEventListener('click', function () {
				var t = tab.getAttribute('data-tab');
				tabs.forEach(function (x) {
					x.classList.toggle('active', x === tab);
					x.setAttribute('aria-selected', x === tab ? 'true' : 'false');
				});
				panes.forEach(function (p) {
					var on = p.getAttribute('data-tab') === t;
					p.classList.toggle('active', on);
					p.hidden = !on;
				});
			});
		});
		window.openCharacterModal = openModal;
	}

	updateGameTimeDisplay();
	window.gameConnect = connect;
	window.gameTryReconnect = tryReconnect;
	function appendLogPendingTalk() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-talk-pending';
		div.setAttribute('data-pending', 'talk');
		div.textContent = '對話中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTalk() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-talk-pending');
		for (var i = 0; i < pending.length; i++) pending[i].remove();
	}

	function appendLogPendingTrade() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-trade-pending';
		div.setAttribute('data-pending', 'trade');
		div.textContent = '交易中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTrade() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-trade-pending');
		for (var j = 0; j < pending.length; j++) pending[j].remove();
	}

	function appendTalkInputRow(entityId, targetName) {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-talk-input';
		var label = document.createElement('span');
		label.className = 'log-talk-label';
		label.textContent = '對 ' + (targetName || entityId) + ' 說：';
		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-talk-input-field';
		input.placeholder = '輸入要說的話，Enter 送出';
		input.setAttribute('maxlength', '200');
		input.setAttribute('aria-label', '對 ' + (targetName || entityId) + ' 說');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-talk-send';
		btn.textContent = '送出';
		wrap.appendChild(label);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		logEl.appendChild(wrap);
		input.focus();
		function sendTalk() {
			var text = (input.value || '').trim();
			var sayText = text || '（搭話）';
			wrap.remove();
			var playerLine = '你對【' + (targetName || entityId) + '】說：「' + sayText + '」';
			appendNarrative(formatNarrative(playerLine), 'Talk');
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Talk', player_input: sayText });
				appendLogPendingTalk();
			}
		}
		btn.addEventListener('click', sendTalk);
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter') {
				e.preventDefault();
				sendTalk();
			}
		});
	}

	function appendTradeInputRow(entityId, targetName) {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-trade-input';
		var label = document.createElement('span');
		label.className = 'log-trade-label';
		label.textContent = '與 ' + (targetName || entityId) + ' 交易：';
		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-trade-input-field';
		input.placeholder = '輸入出價（鎂，整數）或「拒絕」；若尚未報價可留空再送出以開價';
		input.setAttribute('maxlength', '32');
		input.setAttribute('aria-label', '與 ' + (targetName || entityId) + ' 交易出價');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-trade-send';
		btn.textContent = '送出';
		wrap.appendChild(label);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		logEl.appendChild(wrap);
		input.focus();
		function sendTrade() {
			var text = (input.value || '').trim();
			wrap.remove();
			if (text) {
				var playerLine = '你向【' + (targetName || entityId) + '】出價：「' + text + '」';
				appendNarrative(formatNarrative(playerLine), 'Trade');
			}
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Trade', player_input: text });
				appendLogPendingTrade();
			}
		}
		btn.addEventListener('click', sendTrade);
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter') {
				e.preventDefault();
				sendTrade();
			}
		});
	}

	function bindLogObjectActions() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		logEl.addEventListener('click', function (ev) {
			var btn = ev.target.closest && ev.target.closest('.log-object-action');
			if (!btn) return;
			ev.preventDefault();
			var id = btn.getAttribute('data-entity-id');
			var action = btn.getAttribute('data-action');
			var targetType = btn.getAttribute('data-target-type') || '';
			var targetName = btn.getAttribute('data-target-name') || '';
			if (!id || !action || !window.gameSend) return;
			if (targetType === 'entity') {
				var stillHere = false;
				for (var i = 0; i < (state.entities || []).length; i++) {
					if (((state.entities[i].id || '') + '') === (id + '')) {
						stillHere = true;
						break;
					}
				}
				if (!stillHere) {
					appendLog('對方已不在此處。');
					return;
				}
			}
			if (action === 'Talk') {
				appendTalkInputRow(id, targetName);
				return;
			}
			if (action === 'Trade') {
				appendTradeInputRow(id, targetName);
				return;
			}
			window.gameSend({ type: 'do_action', entity_id: id, action: action });
		});
	}
	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') {
			document.addEventListener('DOMContentLoaded', function () { bindAuthForm(); bindLogObjectActions(); });
		} else {
			bindAuthForm();
			bindLogObjectActions();
		}
	}

	window.gameSend = function (msg) {
		if (typeof msg === 'object') send(msg);
		else if (socket && socket.readyState === WebSocket.OPEN) socket.send(msg);
	};
	window.gameState = function () { return state; };
	window.gameSendMoveDirection = sendMoveByDirection;
	window.gameStartTimeTicker = startGameTimeTicker;
	window.gameUpdateTime = updateGameTimeDisplay;
})();

document.addEventListener('DOMContentLoaded', function () {
	if (window.gameConnect) window.gameConnect();
	document.addEventListener('visibilitychange', function () {
		if (document.visibilityState === 'visible' && window.gameTryReconnect) window.gameTryReconnect();
	});
});

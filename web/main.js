// WebSocket 連線與遊戲主邏輯；登入、房間視野、依出口移動。傳統 MUD 節點連接節點。
(function () {
	const wsScheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const wsUrl = wsScheme + '//' + window.location.host + '/ws';
	const STORAGE_PLAYER_ID = 'singularity_player_id';
	const HEARTBEAT_INTERVAL_MS = 30000;

	let socket = null;
	let heartbeatTimer = null;
	let reconnecting = false;

	const state = {
		room_id: '',
		room_name: '',
		description: '',
		exits: [],
		entities: [],
		me: null,
		server_unix: 0,
		game_time_sec_since_midnight: 0,
		game_days_since_epoch: 0
	};

	const GAME_TIME_SCALE = 24;
	const GAME_SEC_PER_DAY = 86400;
	const DAYS_PER_YEAR = 365;
	const DAYS_PER_MONTH = 30;
	const MONTH_NAMES = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '十一', '十二'];
	const DAY_NAMES = ['一', '二', '三', '四', '五', '六', '七', '八', '九', '十', '十一', '十二', '十三', '十四', '十五', '十六', '十七', '十八', '十九', '二十', '廿一', '廿二', '廿三', '廿四', '廿五', '廿六', '廿七', '廿八', '廿九', '三十'];
	let gameTimeTicker = null;

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
		var monthStr = month <= 12 ? MONTH_NAMES[month - 1] : month + '';
		var dayStr = day <= 30 ? DAY_NAMES[day - 1] : day + '';
		return '奇點曆 ' + yearStr + '年' + monthStr + '月' + dayStr + '日';
	}

	function updateGameTimeDisplay() {
		var sec = gameSecNow();
		var days = gameDaysNow();
		var handEl = document.getElementById('game-time-hand');
		var labelEl = document.getElementById('game-time-label');
		var dateEl = document.getElementById('game-time-date');
		if (sec == null || days == null) {
			if (dateEl) dateEl.textContent = '奇點曆 —';
			if (labelEl) labelEl.textContent = '--:--';
			if (handEl) handEl.setAttribute('transform', 'rotate(0 16 16)');
			return;
		}
		if (handEl) {
			var hourCont = sec / 3600;
			var angle = (hourCont - 12) * 15;
			handEl.setAttribute('transform', 'rotate(' + angle + ' 16 16)');
		}
		if (dateEl) dateEl.textContent = formatSingularityDate(days);
		if (labelEl) {
			var h = Math.floor(sec / 3600) % 24;
			var m = Math.floor((sec % 3600) / 60);
			labelEl.textContent = (h < 10 ? '0' : '') + h + ':' + (m < 10 ? '0' : '') + m;
		}
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
			window.mudUpdateRoomView(state.room_name, state.description, state.exits, state.entities, state.me);
		}
		if (window.mudRenderExitButtons) {
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
		const el = document.getElementById('log');
		if (el) {
			el.textContent += text + '\n';
			el.scrollTop = el.scrollHeight;
		}
	}

	function isConnected() {
		return socket && socket.readyState === WebSocket.OPEN;
	}

	function startHeartbeat() {
		stopHeartbeat();
		if (!document.hidden && isConnected()) {
			heartbeatTimer = setInterval(function () {
				if (document.hidden || !isConnected()) return;
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
			if (reconnecting) reconnecting = false;
			appendLog('已連線，請登入');
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
							equipment_names: msg.equipment_names || {}
						};
						if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_PLAYER_ID, msg.player_id);
						showGameAfterLogin();
						updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
						draw();
						appendLog('登入成功：' + msg.player_id + ' @ ' + (msg.room_name || msg.room_id));
						renderStarplatePane(state.me);
						startHeartbeat();
						break;
					case 'pong':
						break;
					case 'moved':
						if (state.me && (msg.player_id === state.me.player_id || msg.player_id === state.me.id)) {
							state.me.room_id = msg.room_id;
							state.me.room_name = msg.room_name;
						}
						appendLog('移動到：' + (msg.room_name || msg.room_id));
						draw();
						break;
					case 'blocked':
						appendLog('無法往「' + (msg.direction || '') + '」移動');
						break;
					case 'entity_status':
						renderStatusPane(msg);
						renderEquipmentPane(msg);
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
							}
							renderStarplatePane(state.me);
						}
						break;
					case 'error':
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
			appendLog('連線關閉，請重新登入');
		};
		socket.onerror = function () {
			appendLog('連線錯誤');
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
		if (authScreen && !app.hidden) authScreen.setAttribute('hidden', '');
		if (app && !state.me) app.setAttribute('hidden', '');
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
		['head', '【首】'], ['face', '【面】'], ['neck', '【頸】'],
		['body', '【衣】'], ['cloak', '【披】'],
		['shoulder', '【肩】'], ['arm', '【臂】'], ['wrist', '【腕】'], ['hand', '【掌】'],
		['waist', '【腰】'], ['legs', '【褲】'], ['feet', '【足】'],
		['ring_l', '【指】左'], ['ring_r', '【指】右'], ['trinket', '【佩】'],
		['hold_l', '【持】左'], ['hold_r', '【持】右']
	];
	function renderEquipmentPane(msg) {
		var wrap = document.getElementById('player-modal-pane-equip');
		if (!wrap) return;
		var names = msg.equipment_names || {};
		var html = '<dl class="status-dl equip-dl">';
		for (var i = 0; i < EQUIP_SLOTS.length; i++) {
			var code = EQUIP_SLOTS[i][0];
			var label = EQUIP_SLOTS[i][1];
			var itemName = names[code];
			html += '<dt>' + label + '</dt><dd>' + (itemName ? escapeHtml(itemName) : '<span class="text-muted">(空)</span>') + '</dd>';
		}
		html += '</dl>';
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
	function renderStarplatePane(me) {
		var wrap = document.getElementById('starplate-content');
		if (!wrap) return;
		if (!me || !me.activated_nodes || !Array.isArray(me.activated_nodes)) {
			wrap.innerHTML = '<p class="text-muted">僅自己可觀看星盤。請先登入並點擊自己開啟。</p>';
			return;
		}
		var activated = me.activated_nodes;
		var count = activated.length;
		var costs = me.topology_costs || [];
		var html = '<div class="starplate-block"><strong>[ 星盤貫通率 ]</strong> ' + count + ' / 360</div>';
		html += '<div class="starplate-block"><strong>[ 源始 ]</strong> N000 生之奇點 (已點亮)</div>';
		html += '<div class="starplate-block starplate-hubs"><strong>二十主樞</strong><ul class="starplate-hub-list">';
		for (var i = 0; i < HUB_NAMES.length; i++) {
			var nodeId = 'N' + String(i + 1).padStart(3, '0');
			var lit = activated.indexOf(nodeId) !== -1 ? '1' : '0';
			var costA = getCostA(costs, i);
			var adaptStr = costA != null ? costSpan(costA) : '未知';
			html += '<li data-hub="' + i + '">[' + HUB_NAMES[i] + '] ' + nodeId + ' ｜ 適性：' + adaptStr + ' ｜ 貫通：0/17 ｜ <button type="button" class="btn-inline">進入觀測</button></li>';
		}
		html += '</ul></div>';
		html += '<div id="starplate-observe" class="starplate-observe" hidden></div>';
		wrap.innerHTML = html;
		wrap.querySelectorAll('.starplate-hub-list button').forEach(function (btn) {
			btn.addEventListener('click', function () {
				var hubIndex = parseInt(btn.closest('li').getAttribute('data-hub'), 10);
				var observeEl = document.getElementById('starplate-observe');
				if (!observeEl) return;
				var typeCCosts = getTypeCCosts(costs, hubIndex);
				var name = HUB_NAMES[hubIndex];
				var costA = getCostA(costs, hubIndex);
				var blueNames = ['起', '承', '轉', '協', '合'];
				var greenLabels = ['G01 探', 'G02 觸', 'G03 納', 'G04 蓄', 'G05 濾', 'G06 析', 'G07 融', 'G08 衍', 'G09 律', 'G10 束', 'G11 釋', 'G12 散'];
				var greenShort = ['G01','G02','G03','G04','G05','G06','G07','G08','G09','G10','G11','G12'];
				var greenIndexPerBlue = [[0, 1, 2], [2, 3, 4], [4, 5, 6], [7, 8, 9], [9, 10, 11]];
				var greenSharedNote = { 2: ' (與[承]共用)', 4: ' (與[轉]共用)', 9: ' (與[合]共用)' };
				var lines = ['<strong>🔭 當前觀測：[', name, '] 星系內部</strong>'];
				lines.push('<div class="starplate-hub-cost">抵達本主樞 ─ ', costSpan(costA), '</div>');
				var idx = 0;
				for (var b = 0; b < 5; b++) {
					var costB = getCostB(costs, hubIndex, b);
					var nextBlue = blueNames[(b + 1) % 5];
					var ringE = getCostE(costs, hubIndex, b);
					lines.push('<div class="starplate-blue">🔵 [', blueNames[b], '] 邏輯閘 ─ ', costSpan(costB), ' (未貫通) <span class="ring-cost">⟳→[', nextBlue, '] ', costSpan(ringE), '</span></div>');
					for (var g = 0; g < 3; g++) {
						var greenIdx = greenIndexPerBlue[b][g];
						var costC = typeCCosts.length > idx ? typeCCosts[idx] : null;
						var ringD = getCostD(costs, hubIndex, greenIdx);
						var nextGreen = greenShort[(greenIdx + 1) % 12];
						var sharedNote = greenSharedNote[greenIdx] || '';
						lines.push('<div class="starplate-green">　├─ 🟢 ', greenLabels[greenIdx], ' ─ ', costSpan(costC), '：[ 空 ]', sharedNote, ' <span class="ring-cost">⟳→', nextGreen, ' ', costSpan(ringD), '</span></div>');
						idx++;
					}
				}
				observeEl.innerHTML = lines.join('');
				observeEl.removeAttribute('hidden');
			});
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
	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') {
			document.addEventListener('DOMContentLoaded', function () { bindAuthForm(); initPlayerModal(); });
		} else {
			bindAuthForm();
			initPlayerModal();
		}
	}
	window.gameSend = function (msg) {
		if (typeof msg === 'object') send(msg);
		else if (socket && socket.readyState === WebSocket.OPEN) socket.send(msg);
	};
	window.gameState = function () { return state; };
	window.gameSendMoveDirection = sendMoveByDirection;
})();

document.addEventListener('DOMContentLoaded', function () {
	if (window.gameConnect) window.gameConnect();
	document.addEventListener('visibilitychange', function () {
		if (document.visibilityState === 'visible' && window.gameTryReconnect) window.gameTryReconnect();
	});
});

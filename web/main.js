// WebSocket 連線與遊戲主邏輯；把後端世界狀態投射成可操作文字表面。v0.39.67
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
		game_days_since_epoch: 0
	};

	/** 同房實體互動：對齊後端 Borrow(偷竊) / Subdue(攻擊) / Slay(致死) */
	const ENTITY_INTERACT_LABELS = { Look: '觀察', Talk: '對話', Borrow: '偷竊', Subdue: '攻擊', Slay: '致死', Trade: '交易', Attack: '攻擊' };
	const ENTITY_INTERACT_ORDER = ['Look', 'Talk', 'Trade', 'Borrow', 'Subdue', 'Slay'];

	function resolveEntityDisplayName(targetId, rawTargetName) {
		if (targetId && window.SwObjectList && typeof window.SwObjectList.getKnownNpcName === 'function') {
			var known = window.SwObjectList.getKnownNpcName(targetId);
			if (known) return known;
		}
		var name = (rawTargetName || '').trim();
		if (name && !/^wt_npc_|^npc_/i.test(name) && !/^陌生人|^眼前之人|^行人$/.test(name)) {
			return name;
		}
		if (targetId) {
			try {
				var leftBtn = document.querySelector('#sw-left-column [data-entity-id="' + targetId + '"]');
				if (leftBtn) {
					var nameSpan = leftBtn.querySelector('.sw-person-name');
					var foundName = (nameSpan ? nameSpan.textContent : leftBtn.textContent).trim();
					if (foundName && !/^wt_npc_|^npc_/i.test(foundName)) return foundName;
				}
			} catch (e) {}
			if (state.entities && state.entities.length) {
				for (var i = 0; i < state.entities.length; i++) {
					var ent = state.entities[i];
					if (ent && ent.id === targetId) {
						var dName = (ent.display_name || ent.name || '').trim();
						if (dName && !/^wt_npc_|^npc_/i.test(dName)) return dName;
					}
				}
			}
		}
		return name || '眼前之人';
	}

	function renderEntityActionButtonsHtml(targetId, targetName) {
		var cleanName = resolveEntityDisplayName(targetId, targetName);
		var sTargetId = String(targetId || '');
		if (sTargetId.indexOf('ambient_') === 0 || sTargetId.indexOf('shanhai_') === 0) {
			var beastActs = [
				{ act: 'Subdue', label: '生擒制伏', cls: 'log-action-subdue' },
				{ act: 'Slay', label: '出手獵殺', cls: 'log-action-slay' }
			];
			return beastActs.map(function (item) {
				return '【<span class="log-object-action ' + item.cls + '" role="button" tabindex="0" data-entity-id="' + escapeHtml(targetId) + '" data-action="' + escapeHtml(item.act) + '" data-target-name="' + escapeHtml(cleanName) + '">' + escapeHtml(item.label) + '</span>】';
			}).join('');
		}
		var peaceActions = [
			{ act: 'Talk', label: '對話' },
			{ act: 'Trade', label: '交易' }
		];
		var html = peaceActions.map(function (item) {
			return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(targetId) + '" data-action="' + escapeHtml(item.act) + '" data-target-name="' + escapeHtml(cleanName) + '">' + escapeHtml(item.label) + '</span>】';
		}).join('');
		html += '【<span class="log-conflict-toggle" role="button" tabindex="0" title="展開偷竊／攻擊／致死選項">衝突 ▾</span>】';
		html += '<span class="log-conflict-group" style="display:none;" hidden>';
		html += '【<span class="log-object-action log-action-steal" role="button" tabindex="0" data-entity-id="' + escapeHtml(targetId) + '" data-action="Borrow" data-target-name="' + escapeHtml(cleanName) + '">偷竊</span>】';
		html += '【<span class="log-object-action log-action-subdue" role="button" tabindex="0" data-entity-id="' + escapeHtml(targetId) + '" data-action="Subdue" data-target-name="' + escapeHtml(cleanName) + '">攻擊</span>】';
		html += '【<span class="log-object-action log-action-slay" role="button" tabindex="0" data-entity-id="' + escapeHtml(targetId) + '" data-action="Slay" data-target-name="' + escapeHtml(cleanName) + '">致死</span>】';
		html += '</span>';
		return html;
	}
	// log 捲動/追加 API 移至 web/sw-log.js（window.SwLog）
	var _L = window.SwLog || {};
	var appendLog = _L.appendLog;
	var appendNarrative = _L.appendNarrative;
	var appendObjectActionsLine = _L.appendObjectActionsLine;

	// narrative HTML 轉義（sw-narrative.js 有自己版本；此處保留供 connect() 中直接用）
	function escapeHtml(s) {
		if (!s) return '';
		var div = document.createElement('div');
		div.textContent = s;
		return div.innerHTML;
	}
	// ── 戰鬥與探索敘事 100% 去數值化過濾 ─────────────────────────────
	function deNumericText(s) {
		if (!s) return '';
		var t = String(s);
		// 替換回合、傷害、氣血扣減等裸露數值為體感描述
		t = t.replace(/第\s*\d+\s*回合/g, '交鋒換氣之際');
		t = t.replace(/受到\s*\d+\s*點傷害/g, '勁力破防，身形微晃');
		t = t.replace(/造成\s*\d+\s*點傷害/g, '勁透內甲，攻勢凌厲');
		t = t.replace(/傷害\s*[-+]?\d+/g, '勁力震盪');
		t = t.replace(/氣血\s*[-]\d+/g, '氣息受挫');
		t = t.replace(/氣血\s*[+]\d+/g, '氣脈舒緩');
		t = t.replace(/HP[:\s]*\d+\s*\/\s*\d+/gi, '');
		t = t.replace(/MP[:\s]*\d+\s*\/\s*\d+/gi, '');
		t = t.replace(/Lv\.\s*\d+/gi, '');
		return t;
	}

	// ── 戰鬥對峙空檔與戰術決策按鈕列控制 ─────────────────────────
	function showTacticalChoices(choices) {
		var choiceList = (Array.isArray(choices) && choices.length) ? choices : ['趁勢搶攻', '專攻部位', '守勢游鬥', '搏命硬撼', '抽身脫離'];
		var container = document.getElementById('sw-tactical-choices');
		if (container) {
			container.hidden = false;
			var btns = container.querySelectorAll('.sw-tactical-btn');
			for (var i = 0; i < btns.length; i++) {
				var btn = btns[i];
				var val = choiceList[i] || btn.getAttribute('data-choice');
				if (val) {
					btn.textContent = val;
					btn.setAttribute('data-choice', val);
					btn.hidden = false;
				} else {
					btn.hidden = true;
				}
			}
			if (container.scrollIntoView) container.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
		}

		var backdrop = document.getElementById('sw-object-popover-backdrop');
		var popLog = document.getElementById('sw-popover-log');
		if (backdrop && !backdrop.hidden && backdrop.style.display !== 'none' && popLog) {
			var oldInlines = popLog.querySelectorAll('.log-tactical-inline');
			for (var oi = 0; oi < oldInlines.length; oi++) {
				oldInlines[oi].remove();
			}
			var entryDiv = document.createElement('div');
			entryDiv.className = 'sw-log-entry log-entry log-tactical-inline';
			entryDiv.innerHTML = '<div class="sw-tactical-hint" style="margin-bottom:6px;">戰術對峙・抉擇契機</div>' +
				'<div class="sw-tactical-actions" style="display:flex;flex-wrap:wrap;gap:8px;">' +
				choiceList.map(function (c) {
					return '<button type="button" class="sw-tactical-btn" data-choice="' + escapeHtml(c) + '">' + escapeHtml(c) + '</button>';
				}).join('') +
				'</div>';
			entryDiv.addEventListener('click', function (e) {
				var b = e.target.closest('.sw-tactical-btn');
				if (!b || b.disabled) return;
				var choice = b.getAttribute('data-choice') || b.textContent.trim();
				if (choice && window.gameSend) {
					var allBtns = entryDiv.querySelectorAll('.sw-tactical-btn');
					for (var bi = 0; bi < allBtns.length; bi++) {
						allBtns[bi].disabled = true;
					}
					window.gameSend({ type: 'tactical_choice', choice: choice });
					if (appendLog) {
						appendLog('凝神對峙，定計【' + choice + '】……');
					}
					hideTacticalChoices();
				}
			});
			popLog.appendChild(entryDiv);
			popLog.scrollTop = popLog.scrollHeight;
		}
	}

	function hideTacticalChoices() {
		var container = document.getElementById('sw-tactical-choices');
		if (container) container.hidden = true;
		var popLog = document.getElementById('sw-popover-log');
		if (popLog) {
			var inlines = popLog.querySelectorAll('.log-tactical-inline');
			for (var i = 0; i < inlines.length; i++) {
				inlines[i].remove();
			}
		}
	}

	function bindTacticalChoices() {
		var container = document.getElementById('sw-tactical-choices');
		if (!container || container._bound) return;
		container._bound = true;
		container.addEventListener('click', function (e) {
			var btn = e.target.closest('.sw-tactical-btn');
			if (!btn || btn.disabled) return;
			var choice = btn.getAttribute('data-choice') || btn.textContent.trim();
			if (choice && window.gameSend) {
				var allBtns = container.querySelectorAll('.sw-tactical-btn');
				for (var bi = 0; bi < allBtns.length; bi++) {
					allBtns[bi].disabled = true;
				}
				window.gameSend({ type: 'tactical_choice', choice: choice });
				if (appendLog) {
					appendLog('凝神對峙，定計【' + choice + '】……');
				}
				hideTacticalChoices();
			}
		});
	}
	function updateStatusBars(hpCur, hpMax, innerCur, innerMax, spiritCur, spiritMax, staminaCur, staminaMax) {
		if (window.SwClock && window.SwClock.updateStatusBars) {
			window.SwClock.updateStatusBars(hpCur, hpMax, innerCur, innerMax, spiritCur, spiritMax, staminaCur, staminaMax);
		}
		var sealsEl = document.getElementById('sw-status-seals');
		if (sealsEl) {
			var html = '';
			if (hpMax != null && hpMax > 0 && hpCur != null && Number(hpCur) < Number(hpMax) * 0.45) {
				html += '<span class="sw-seal-chip is-danger">【傷】</span>';
			}
			if (staminaMax != null && staminaMax > 0 && staminaCur != null && Number(staminaCur) < Number(staminaMax) * 0.3) {
				html += '<span class="sw-seal-chip is-warn">【乏】</span>';
			}
			if (spiritMax != null && spiritMax > 0 && spiritCur != null && Number(spiritCur) < Number(spiritMax) * 0.3) {
				html += '<span class="sw-seal-chip is-warn">【倦】</span>';
			}
			if (window.SwPanes && typeof window.SwPanes.getToolBadgeHtml === 'function') {
				html += window.SwPanes.getToolBadgeHtml();
			}
			sealsEl.innerHTML = html;
		}
	}
	window.updateStatusBars = updateStatusBars;
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
					case 'room_view':
					case 'view':
						state.room_id = msg.room_id || '';
						state.room_name = msg.room_name || '';
						state.description = msg.description || '';
						state.room_terrain = msg.terrain || '';
						state.room_venue_kind = msg.venue_kind || '';
						state.room_is_indoor = Boolean(msg.is_indoor);
						window.__swRoomIsIndoor = state.room_is_indoor;
						window.__swRoomTerrain = state.room_terrain;
						window.__swRoomVenueKind = state.room_venue_kind;
						state.exits = Array.isArray(msg.exits) ? msg.exits : [];
						state.entities = Array.isArray(msg.entities) ? msg.entities : [];
						window.SwNarrative.indexNames(state.entities);
						state.objects = Array.isArray(msg.objects) ? msg.objects : [];
						if (Array.isArray(msg.harvestables)) state.harvestables = msg.harvestables;
						if (msg.weather) state.weather = msg.weather;
						if (Array.isArray(msg.cells) || Array.isArray(msg.grid_cells)) {
							state.cells = Array.isArray(msg.cells) ? msg.cells : msg.grid_cells;
						}
						if (typeof msg.server_unix === 'number' && typeof msg.game_time_sec_since_midnight === 'number' && typeof msg.game_days_since_epoch === 'number') {
							var GAME_TIME_SCALE = 24;
							var GAME_SEC_PER_DAY = 86400;
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
						window.SwClock.startTicker();
						window.SwClock.updateDisplay();
						window.SwClock.draw();
						if (window.onRoomViewUpdate) window.onRoomViewUpdate(msg);
						break;
					case 'grid_view':
						if (Array.isArray(msg.entities)) {
							state.entities = msg.entities;
							window.SwNarrative.indexNames(state.entities);
						}
						if (Array.isArray(msg.objects)) state.objects = msg.objects;
						if (window.updateRoomData) window.updateRoomData(msg);
						if (window.onGridView) window.onGridView(msg);
						if (typeof msg.server_unix === 'number' && typeof msg.game_time_sec_since_midnight === 'number') {
							state.server_unix = msg.server_unix;
							state.game_time_sec_since_midnight = msg.game_time_sec_since_midnight;
							state.game_days_since_epoch = msg.game_days_since_epoch;
							window.SwClock.startTicker();
							window.SwClock.updateDisplay();
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
						window.__hasOpenRouterKey = !!msg.has_openrouter_key;
						if (msg.openrouter_model) window.__openRouterModel = msg.openrouter_model;
						try {
							var savedOrKey = (localStorage.getItem('sw_openrouter_api_key') || '').trim();
							if (savedOrKey && !/^[•*·]+$/.test(savedOrKey)) {
								window.__hasOpenRouterKey = true;
								send({ type: 'set_openrouter_key', player_input: savedOrKey, action: 'sync' });
							}
						} catch (e) {}
						if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_PLAYER_ID, msg.player_id);
						window.myPlayerId = msg.player_id;
						// myPlayerId 到齊後，重新渲染 grid 物件欄/描述欄以過濾自己
						if (window.refreshGridView) window.refreshGridView();
						window.SwAuth.showGame();
						updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
						send({ type: 'get_inventory' });
						window.SwClock.draw();
						renderStarplatePane(state.me);
						startHeartbeat();
						break;
					case 'pong':
						lastPongTime = Date.now();
						break;
					case 'moved':
						if (state.me && (msg.player_id === state.me.player_id || msg.player_id === state.me.id)) {
							state.me.room_id = msg.room_id;
							state.me.room_name = msg.room_name;
							if (window.mapState) window.mapState.moving = false;
							hideTacticalChoices();
						}
						window.SwClock.draw();
						break;
					case 'blocked':
						if (window.mapState) window.mapState.moving = false;
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
							window.__hasOpenRouterKey = !!msg.has_openrouter_key;
							if (msg.openrouter_model) window.__openRouterModel = msg.openrouter_model;
							renderStarplatePane(state.me);
						}
						break;
				case 'action_result':
					var departedId = msg.target_id || (window.__currentPopoverTarget && window.__currentPopoverTarget.id);
					var targetGone = msg.suggested_action === 'target_departed' || (msg.success === false && /邁步離開此處|已離開此處|不在附近|已不在此處/.test(String(msg.narrative || '')));
					if (departedId && targetGone) {
						var canvasEl = document.getElementById('sw-room-canvas');
						if (canvasEl) {
							var staleTokens = canvasEl.querySelectorAll('.sw-canvas-token[data-entity-id="' + departedId + '"], .sw-canvas-token[data-obj-id="' + departedId + '"]');
							for (var sti = 0; sti < staleTokens.length; sti++) {
								staleTokens[sti].remove();
							}
						}
						if (state.entities && state.entities.length) {
							state.entities = state.entities.filter(function (e) { return e && e.id !== departedId; });
						}
						var curPop = window.__currentPopoverTarget;
						if (curPop && (curPop.id === departedId || curPop.key === departedId)) {
							if (window.SwCloseObjectPopover) window.SwCloseObjectPopover();
						}
					}
					if (msg.action === 'Talk') {
						if (window.SwInteractions) window.SwInteractions.removeLogPendingTalk();
						var activePop = window.__currentPopoverTarget;
						var isMismatchedPopover = !!(activePop && activePop.id && msg.target_id && activePop.id !== msg.target_id);
						var narrative = msg.narrative;
						if (!narrative) narrative = '（NPC 無回應）';
						var narrativeHtml = window.SwNarrative.format(narrative);
						narrativeHtml = window.SwNarrative.formatWithClickable(narrativeHtml, state.objects);
						if (isMismatchedPopover) {
							var swLogEl = document.getElementById('sw-log') || document.getElementById('log');
							if (swLogEl) {
								var div = document.createElement('div');
								div.className = 'sw-log-entry log-entry log-narrative log-talk';
								div.innerHTML = narrativeHtml;
								swLogEl.appendChild(div);
								swLogEl.scrollTop = swLogEl.scrollHeight;
							}
							if (window.SwNotify && window.SwNotify.toast) {
								window.SwNotify.toast(narrativeHtml, { kind: 'talk', isHtml: true });
							}
						} else {
							appendNarrative(narrativeHtml, 'Talk');
						}
						// 對話回覆後直接顯示【對話】【攻擊】【交易】，不用再點一次角色
						var talkTargetId = msg.target_id || '';
						var talkTargetName = msg.target_name || talkTargetId;
						if (talkTargetId && talkTargetName && !/^陌生人|^眼前之人|^行人|^npc_|^wt_npc_/i.test(talkTargetName)) {
							if (window.SwObjectList && window.SwObjectList.applyObservationUpdate) {
								window.SwObjectList.applyObservationUpdate({
									target_id: talkTargetId,
									new_state: 'named',
									tag: talkTargetName
								});
							}
						}
						if (!isMismatchedPopover) {
							if (talkTargetId && msg.open_trade && msg.suggested_action === 'Trade') {
								if (window.gameSend) {
									window.gameSend({ type: 'do_action', entity_id: talkTargetId, action: 'Trade', player_input: '' });
									if (window.SwInteractions) window.SwInteractions.appendLogPendingTrade();
								} else if (window.SwInteractions && window.SwInteractions.appendTradeInputRow) {
									window.SwInteractions.appendTradeInputRow(talkTargetId, talkTargetName);
								}
							} else if (talkTargetId) {
								var popoverBackdrop = document.getElementById('sw-object-popover-backdrop');
								var isPopoverOpen = popoverBackdrop && !popoverBackdrop.hidden && popoverBackdrop.style.display !== 'none';
								if (isPopoverOpen && window.SwInteractions && window.SwInteractions.appendTalkInputRow) {
									window.SwInteractions.appendTalkInputRow(talkTargetId, talkTargetName, { autoFocus: false });
								} else {
									appendObjectActionsLine(renderEntityActionButtonsHtml(talkTargetId, talkTargetName));
								}
							}
						}
					} else if (msg.action === 'Trade') {
						if (window.SwInteractions) window.SwInteractions.removeLogPendingTrade();
						var activeTradePop = window.__currentPopoverTarget;
						var isMismatchedTradePopover = !!(activeTradePop && activeTradePop.id && msg.target_id && activeTradePop.id !== msg.target_id);
						var tNarrative = msg.narrative;
						if (!tNarrative) tNarrative = '（交易無回應）';
						var tHtml = window.SwNarrative.format(tNarrative);
						tHtml = window.SwNarrative.formatWithClickable(tHtml, state.objects);
						if (isMismatchedTradePopover) {
							var swTradeLogEl = document.getElementById('sw-log') || document.getElementById('log');
							if (swTradeLogEl) {
								var tDiv = document.createElement('div');
								tDiv.className = 'sw-log-entry log-entry log-narrative log-trade';
								tDiv.innerHTML = tHtml;
								swTradeLogEl.appendChild(tDiv);
								swTradeLogEl.scrollTop = swTradeLogEl.scrollHeight;
							}
							if (window.SwNotify && window.SwNotify.toast) {
								window.SwNotify.toast(tHtml, { kind: 'trade', isHtml: true });
							}
						} else {
							appendNarrative(tHtml, 'Trade');
						}
						var tradeTargetId = msg.target_id || '';
						var tradeTargetName = msg.target_name || tradeTargetId;
						if (!isMismatchedTradePopover && tradeTargetId) {
							var shouldOpenTrade = Boolean(msg.open_trade) && msg.suggested_action !== 'Trade';
							if (shouldOpenTrade) {
								if (window.SwInteractions && window.SwInteractions.openTradeInput) {
									window.SwInteractions.openTradeInput(tradeTargetId, tradeTargetName || tradeTargetId);
								} else if (tradeTargetName) {
									if (!window.__currentPopoverTarget && window.SwOpenObjectPopover) {
										var cTokenEl = document.querySelector('#sw-room-canvas [data-entity-id="' + tradeTargetId + '"]');
										if (cTokenEl && cTokenEl._swToken) {
											window.SwOpenObjectPopover(cTokenEl._swToken);
											appendNarrative(tHtml, 'Trade');
										}
									}
									if (window.SwInteractions && window.SwInteractions.appendTradeInputRow) {
										window.SwInteractions.appendTradeInputRow(tradeTargetId, tradeTargetName);
									}
								}
							} else if (msg.open_trade === false) {
								if (window.SwInteractions && window.SwInteractions.closeTradeInput) {
									window.SwInteractions.closeTradeInput();
								}
								if (tradeTargetName) {
									appendObjectActionsLine(renderEntityActionButtonsHtml(tradeTargetId, tradeTargetName));
								}
							} else if (tradeTargetName) {
								appendObjectActionsLine(renderEntityActionButtonsHtml(tradeTargetId, tradeTargetName));
							}
						}
					} else if (msg.action === 'Borrow') {
						var bNarrative = msg.narrative;
						if (!bNarrative) bNarrative = '（偷竊無回應）';
						var bHtml = window.SwNarrative.format(bNarrative);
						bHtml = window.SwNarrative.formatWithClickable(bHtml, state.objects);
						appendNarrative(bHtml, 'Borrow');
						var borrowTargetId = msg.target_id || '';
						var borrowTargetName = msg.target_name || borrowTargetId;
						if (borrowTargetId) {
							appendObjectActionsLine(renderEntityActionButtonsHtml(borrowTargetId, borrowTargetName));
						}
					} else if (msg.narrative) {
						var narrativeText = msg.narrative;
						var invPaneEl = document.getElementById('player-modal-pane-inventory') || document.getElementById('modal-inventory');
						var invOverlayEl = document.getElementById('player-modal-overlay');
						var isInvAction = msg.action === 'craft_item' || msg.action === 'Use' || (invPaneEl && !invPaneEl.hidden && (!invOverlayEl || !invOverlayEl.hidden));
						if (isInvAction && window.SwPanes && typeof window.SwPanes.showInventoryFeedback === 'function') {
							window.SwPanes.showInventoryFeedback(narrativeText, msg.success === false);
						}
						if (msg.action === 'Look' && narrativeText.indexOf('【正在交談】') !== -1) {
							var dlgIdx = narrativeText.indexOf('【正在交談】');
							extractedDialogue = narrativeText.slice(dlgIdx).replace(/^【正在交談】\s*/, '').trim();
							narrativeText = narrativeText.slice(0, dlgIdx).replace(/\s+$/, '');
						}
						var backdrop = document.getElementById('sw-object-popover-backdrop');
						var popLog = document.getElementById('sw-popover-log');
						var activePop = window.__currentPopoverTarget;
						var isMismatchedPopover = !!(activePop && activePop.id && msg.target_id && activePop.id !== msg.target_id);

						if (msg.action === 'Look' && !isMismatchedPopover && backdrop && !backdrop.hidden && backdrop.style.display !== 'none' && popLog) {
							var existingDialogues = popLog.querySelectorAll('.log-npc-dialogue');
							for (var di = 0; di < existingDialogues.length; di++) {
								var dText = (existingDialogues[di].textContent || '').replace(/^【正在交談】\s*/, '').trim();
								if (dText && narrativeText.indexOf(dText) !== -1) {
									popLog.removeChild(existingDialogues[di]);
								}
							}
							if (window.__popoverLogCache) {
								var G = window.SwGrid;
								var cRoom = (G && G.mapData && G.mapData.currentRoomId) || '';
								var activeT = window.__currentPopoverTarget;
								var cKey = cRoom + '|' + (activeT ? activeT.id : '');
								if (window.__popoverLogCache[cKey]) {
									window.__popoverLogCache[cKey] = window.__popoverLogCache[cKey].filter(function (entry) {
										if (entry.classList && entry.classList.contains('log-npc-dialogue')) {
											var eText = (entry.textContent || '').replace(/^【正在交談】\s*/, '').trim();
											return !(eText && narrativeText.indexOf(eText) !== -1);
										}
										return true;
									});
								}
							}
						}
						var narrativeHtml = window.SwNarrative.format(narrativeText);
						narrativeHtml = window.SwNarrative.formatWithClickable(narrativeHtml, state.objects);
						if (isMismatchedPopover && backdrop && !backdrop.hidden && backdrop.style.display !== 'none') {
							var prevDisplay = backdrop.style.display;
							backdrop.style.display = 'none';
							appendNarrative(narrativeHtml, msg.action);
							backdrop.style.display = prevDisplay;
						} else {
							appendNarrative(narrativeHtml, msg.action);
						}
						if (msg.action === 'Look' && msg.target_id && (!backdrop || backdrop.hidden || backdrop.style.display === 'none')) {
							var G2 = window.SwGrid;
							var cRoom2 = (G2 && G2.mapData && G2.mapData.currentRoomId) || '';
							var lookCacheKey = cRoom2 + '|' + msg.target_id;
							if (!window.__popoverLogCache) window.__popoverLogCache = {};
							if (!window.__popoverLogCache[lookCacheKey]) window.__popoverLogCache[lookCacheKey] = [];
							var lookNode = document.createElement('div');
							lookNode.className = 'sw-log-entry log-entry log-narrative log-look';
							lookNode.innerHTML = narrativeHtml;
							window.__popoverLogCache[lookCacheKey] = window.__popoverLogCache[lookCacheKey].filter(function (entry) {
								return !(entry.classList && entry.classList.contains('log-look'));
							});
							window.__popoverLogCache[lookCacheKey].unshift(lookNode);
						}
						// 觀看後下一行顯示可執行的其他動作（物件：移動/閱讀…；人物：對話/攻擊）
						if (msg.action === 'Look') {
							var actionLabels = window.SwActionLabels || {
								// Core
								'Look': '觀察', 'Read': '閱讀', 'Smell': '嗅聞', 'Use': '使用', 'Open': '開啟',
								'Queue': '候號', 'AcceptJob': '應工', 'Rest': '歇腳', 'BuySoup': '買滿碗熱湯', 'Work': '差事',
								'Gather': '採集', 'Acquire': '採買入袋', 'Wash': '洗漱淨手', 'Sit': '坐下',
								'Taste': '品嚐', 'Take': '拾取', 'Chop': '砍伐', 'Operate': '操作',
								'Move': '前往', 'Talk': '對話', 'Trade': '交易', 'Borrow': '偷竊',
								'Subdue': '攻擊', 'Slay': '致死', 'Attack': '攻擊',
								// Wood / Fire / Camp
								'MakeFire': '生火', 'AddFuel': '添柴', 'SplitWood': '劈柴', 'RefinePlank': '裁鋸木板',
								'BurnCharcoal': '悶燒木炭', 'CraftTorch': '紮製火把', 'CraftHandle': '車削木柄',
								'CollectCharcoal': '收取木炭', 'RoastFood': '炙烤食物', 'RepairStructure': '修繕加固',
								// Ore / Smithing
								'WashOre': '淘洗礦砂', 'CrushOre': '粉碎礦石', 'SmeltIngot': '熔煉金屬',
								'SmeltBloom': '悶燒成塊', 'BuildFieldAnvil': '架設石砧',
								'ForgeTool': '鍛打器具', 'CastHardware': '澆鑄器用', 'SharpenBlade': '砥石磨刃',
								'Quench': '淬火', 'ForgeRepairStrip': '鍛修鐵條',
								// Herbs / Alchemy
								'SetupHerbStove': '架設陶銚',
								'SortHerbs': '分揀藥草', 'DryHerbs': '曝曬藥材', 'GrindPowder': '研製藥粉',
								'DecoctPotion': '煎煮湯劑', 'MakePoultice': '調製敷料', 'DistillEssence': '萃煉純露',
								'WashHerb': '淘洗藥草', 'DryHerb': '曝曬藥材', 'CrushHerb': '研碎藥草', 'BoilDecoction': '煎煮湯劑',
								// Fiber / Paper / Cloth
								'SoakFiber': '漚浸纖維', 'TwistCord': '搓捻麻繩', 'WeaveCloth': '紡織布匹',
								'OilCanvas': '桐油浸布', 'PressPaper': '抄造紙張', 'Stitch': '縫紉', 'TwistRope': '搓捻麻繩',
								// Hide / Shell / Boots
								'ScrapeHide': '刮治生皮', 'BoilFishGlue': '熬製魚膠', 'CalcifyShell': '煅燒貝灰',
								'CobbleBoots': '縫製行靴',
								// Ferry / Wharf / Tasks
								'JoinCheck': '候檢', 'RegisterComplaint': '押名', 'QueueBroth': '等分湯',
								'CarryCargo': '搬貨收鎂', 'HelpRope': '搭手拉繩', 'DryOff': '避風晾衣',
								'WarmHands': '烘手', 'TalkQueue': '攀談船期', 'QueueFerry': '排隊候船',
								'SellReeds': '賣蘆草', 'SellSackcloth': '交麻袋布', 'SellRope': '交麻繩卷',
								'BoardFerry': '憑隊上船', 'StepAshore': '踩泥上岸', 'CheckCargo': '對貨領尾款',
								// Living / Artisan
								'Sleep': '安歇', 'Bath': '洗沐', 'Drink': '汲水飲用', 'Water': '澆灌',
								'Measure': '丈量', 'Calculate': '推算', 'Observe': '觀測', 'TestBrush': '試筆',
								'GrindInk': '研墨', 'Weigh': '秤量', 'Sharpen': '磨利', 'Feed': '餵食',
								'Shake': '搖動', 'Ring': '叩鐘', 'Caulking': '捻縫', 'Pound': '舂搗',
								'PickType': '揀字', 'RollInk': '滾墨', 'Explore': '深入探索',
								// Extended Sockets & Interactions
								'Check': '檢視', 'Climb': '登臨', 'Inspect': '審視', 'Knock': '叩門',
								'Listen': '諦聽', 'LookShip': '眺望舟楫', 'Tap': '輕叩', 'TestEdge': '試刃',
								'Tie': '繫縛', 'Touch': '撫觸', 'Turn': '旋動', 'Watch': '守望',
								'WeaveSackcloth': '織造麻袋布', 'Meditate': '調息',
								'Harvest': '採收', 'Deliver': '交割物資', 'ClaimStrandedCargo': '揭榜管運',
								'DeliverStrandedCargo': '交割銷單', 'Sow': '整壟播種', 'Board': '登船啟航', 'Buy': '投保立契',
								'DepositBank': '存入票號', 'WithdrawBank': '憑摺支取',
								'StoreStash': '存入石櫃', 'RetrieveStash': '取出物資',
								'DispatchCourier': '快騎託運', 'AngleFish': '甩竿垂釣', 'Cook': '野炊烹煮',
								'MillGrain': '水磨碾米', 'PressCheese': '壓製乳酪', 'SmokeFish': '慢火冷燻',
								'CutJade': '切磨寒玉', 'HaulMineCart': '滑車運礦', 'BurnLime': '入窯煅灰',
								'RowSwampRaft': '撐筏入澤', 'PrepareMiasmaGear': '備妥避瘴', 'PlayChess': '對弈手談',
								'SeasonalBanquet': '歲時圍爐',
								'ChartAstrolabe': '拓印星盤',
								'TestEngraving': '測紋校準',
								'SparPillar': '試勁拆招',
								'SoakHotSpring': '祛寒湯浴',
								'GroomAppearance': '修面整容',
								'MendTravelGear': '補綴行裝',
								'ReadWindVane': '觀測風候',
								'ApplyMoxibustion': '百草溫灸',
								'HangVotivePlaque': '懸掛銘牌',
								'TranscribeRareTome': '鈔錄孤本',
								'AttendLecture': '旁聽策論',
								'CalibrateClock': '校時報刻',
								'FitMortiseTenon': '榫卯校修',
								'QuenchBlade': '精鋼回火',
								'DyeIndigoCloth': '藍染浸布',
								'FeedDraftBeast': '飼餵草獸',
								'BoardCoastalShip': '登船遠航',
								'InspectLandRegistry': '查閱地籍',
								'AssistKitchen': '幫廚備膳',
								'RoastMountainTea': '焙茶品茗',
								'SunBooks': '曝書防蠹',
								'RoastHerbs': '炮製藥材',
								'CollateInscriptions': '校勘碑帖',
								'GrindOpticLens': '研磨鏡片',
								'SelectDriedSeafood': '揀選海味',
								'DeliberateCivicPact': '參議鄉約',
								'DrawSpringWell': '汲水潤嗓'
							};
							var resolveActionLabelZh = function (act, targetObj) {
								if (!act) return '操作';
								if (actionLabels && actionLabels[act]) return actionLabels[act];
								if (window.SwResolveActionLabelZh) return window.SwResolveActionLabelZh(act, targetObj);
								if (/^[A-Za-z_]+$/.test(act)) return '操作';
								return act;
							};
							var others = [];
							var targetId = msg.target_id || '';
							// 優先使用後端在 action_result 帶上的 actions（建築 Look 後必有【移動】等）
							if (msg.actions && Array.isArray(msg.actions) && msg.actions.length) {
								others = msg.actions;
							} else {
								var obj = null;
								var pool = (state.objects && state.objects.length) ? state.objects :
									((window.SwGrid && window.SwGrid.mapData && window.SwGrid.mapData.objects) || []);
								if (pool && pool.length) {
									for (var i = 0; i < pool.length; i++) {
										if (pool[i].id === msg.target_id || pool[i].name === msg.target_name) {
											obj = pool[i];
											break;
										}
									}
								}
								if (obj && (obj.actions || obj.sockets)) {
									var objActs = obj.actions || obj.sockets || [];
									others = objActs.filter(function (a) { return a !== 'Look'; });
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
							var targetName = resolveEntityDisplayName(targetId, msg.target_name || '');
							var isPerson = false;
							if (state.entities && state.entities.length) {
								for (var ej = 0; ej < state.entities.length; ej++) {
									if (state.entities[ej].id === targetId) {
										isPerson = true;
										break;
									}
								}
							}
							if (!isPerson && window.__currentPopoverTarget && window.__currentPopoverTarget.id === targetId && (window.__currentPopoverTarget.kind === 'npc' || window.__currentPopoverTarget.kind === 'player')) {
								isPerson = true;
							}
							if (!isPerson && window.SwGrid && window.SwGrid.mapData && Array.isArray(window.SwGrid.mapData.npcs)) {
								for (var nj = 0; nj < window.SwGrid.mapData.npcs.length; nj++) {
									var nItem = window.SwGrid.mapData.npcs[nj];
									if (nItem && (nItem.npc_id === targetId || nItem.id === targetId)) {
										isPerson = true;
										break;
									}
								}
							}
							if (!isPerson && others.some(function (a) { return a === 'Subdue' || a === 'Slay' || a === 'Talk'; })) {
								isPerson = true;
							}
							var sTargetId = String(targetId || '');
							if (!isPerson && (sTargetId.indexOf('npc_') === 0 || sTargetId.indexOf('wt_npc_') === 0 || sTargetId.indexOf('wt_vendor_') === 0 || sTargetId.indexOf('ambient_') === 0 || sTargetId.indexOf('shanhai_') === 0)) {
								isPerson = true;
							}
							if (!isPerson && targetId) {
								try {
									if (document.querySelector('#sw-room-canvas .sw-canvas-token.sw-token-npc[data-entity-id="' + targetId + '"], .sw-obj-npc[data-entity-id="' + targetId + '"]')) {
										isPerson = true;
									}
								} catch (e) {}
							}
							var popoverBackdrop = document.getElementById('sw-object-popover-backdrop');
							var isPopoverOpen = popoverBackdrop && !popoverBackdrop.hidden && popoverBackdrop.style.display !== 'none';
							if (!isMismatchedPopover) {
								if (isPerson) {
									if (window.SwUpdatePopoverActions) {
										window.SwUpdatePopoverActions('person', targetId, targetName);
									}
									if (!isPopoverOpen) {
										appendObjectActionsLine(renderEntityActionButtonsHtml(targetId, targetName));
									}
								} else {
									if (window.SwUpdatePopoverActions) {
										window.SwUpdatePopoverActions('object', targetId, targetName, others, msg.move_target_id);
									}
									if (!isPopoverOpen && others.length) {
										var parts = others.map(function (act) {
											var label = resolveActionLabelZh(act, obj);
											var idForAction = (act === 'Move' && msg.move_target_id) ? msg.move_target_id : targetId;
											return '【<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(idForAction) + '" data-action="' + escapeHtml(act) + '" data-target-name="' + escapeHtml(targetName) + '">' + escapeHtml(label) + '</span>】';
										});
										appendObjectActionsLine(parts.join(''));
									}
								}
							}
							if (extractedDialogue && typeof window.SwRecordNpcDialogue === 'function') {
								window.SwRecordNpcDialogue(msg.target_id || '', '', extractedDialogue);
							}
						} else {
							if (Array.isArray(msg.actions) && window.SwUpdatePopoverActions) {
								var popoverBackdrop = document.getElementById('sw-object-popover-backdrop');
								var isPopoverOpen = popoverBackdrop && !popoverBackdrop.hidden && popoverBackdrop.style.display !== 'none';
								var activeToken = window.__currentPopoverTarget;
								var targetMatches = false;
								if (activeToken) {
									if (msg.target_id && (activeToken.id === msg.target_id || activeToken.key === msg.target_id)) targetMatches = true;
									if (msg.target_name && (activeToken.name === msg.target_name || activeToken.label === msg.target_name)) targetMatches = true;
								}
								if (isPopoverOpen && targetMatches) {
									var postActs = msg.actions.filter(function (a) { return a && a !== 'Look'; });
									window.SwUpdatePopoverActions('object', msg.target_id || '', msg.target_name || '', postActs, msg.move_target_id);
								}
							}
							if (!isMismatchedPopover && (msg.action === 'Subdue' || msg.action === 'Slay')) {
								var combatTid = msg.target_id || '';
								var combatTname = msg.target_name || combatTid;
								if (combatTid) {
									appendObjectActionsLine(renderEntityActionButtonsHtml(combatTid, combatTname));
								}
							}
						}
					}
					if (Array.isArray(msg.tactical_choices) && msg.tactical_choices.length > 0 && typeof showTacticalChoices === 'function') {
						showTacticalChoices(msg.tactical_choices);
					}
					break;
				// ── v5 四類新事件分流 ─────────────────────────────────────────
					case 'room_enter':
						// 進入場所後直接呈現當地事實與可接觸對象，不另灌一段入場散文。
						if (msg.room_id && state.room_id && msg.room_id !== state.room_id) {
							hideTacticalChoices();
						}
						var roomEnterName = (msg.place && msg.place.name) || (msg.terrain && msg.terrain.name) || msg.room_name || '';
						if (msg.room_id) state.room_id = msg.room_id;
						if (roomEnterName) state.room_name = roomEnterName;
						if (msg.terrain && msg.terrain.description) state.description = msg.terrain.description;
						if (msg.weather) state.weather = msg.weather;
						if (Array.isArray(msg.exits)) state.exits = msg.exits;
						if (Array.isArray(msg.objects)) state.objects = msg.objects;
						if (Array.isArray(msg.entities)) {
							state.entities = msg.entities;
							window.SwNarrative.indexNames(state.entities);
						} else if (Array.isArray(msg.npcs)) {
							state.entities = msg.npcs;
							window.SwNarrative.indexNames(state.entities);
						}
						if (state.me) {
							state.me.room_id = state.room_id;
							state.me.room_name = state.room_name;
						}
						if (window.onRoomEnter) window.onRoomEnter(msg);
						break;
				case 'explore_result':
					// 探索結果：追加 log，100% 去數值化過濾，不動 L1-L3
					if (window.onExploreResult) window.onExploreResult(msg);
					if (msg.log) {
						var cleanLog = deNumericText(msg.log);
						var harvestMap = (window.SwPanes && window.SwPanes.ITEM_NAME_MAP) || {
							'harvest_herb': '野地草藥',
							'harvest_wood': '堅韌原木',
							'harvest_ore': '百工礦石',
							'harvest_fish': '鮮活河魚',
							'harvest_crystal': '溫潤碎晶',
							'herb_lingzhi': '光葉藥草',
							'wood_iron_oak': '沼生烏木',
							'harvest_rare_wood': '溫木心材',
							'ore_black_iron': '青脊重礦',
							'ore_silver': '銀紋輝石',
							'ore_copper': '紅土銅原礦',
							'wild_fruit': '山野漿果',
							'tea_leaf': '翠嶺嫩茶',
							'spring_water': '清冽泉露'
						};
						for (var rawId in harvestMap) {
							cleanLog = cleanLog.split(rawId).join(harvestMap[rawId]);
						}
						var exploreHtml = window.SwNarrative ? window.SwNarrative.format(cleanLog) : escapeHtml(cleanLog);
						if (appendNarrative) appendNarrative(exploreHtml, 'explore');
					}
					break;
				case 'tactical_state':
					if (window.onTacticalState) window.onTacticalState(msg);
					else if (window.SwPanes && window.SwPanes.onTacticalState) window.SwPanes.onTacticalState(msg);
					break;
				case 'tactical_standoff':
				case 'combat':
				case 'combat_turn':
				case 'combat_result':
					var combatText = msg.narrative || msg.log || msg.text || '';
					if (combatText) {
						var cleanCombat = deNumericText(combatText);
						var combatHtml = window.SwNarrative ? window.SwNarrative.format(cleanCombat) : escapeHtml(cleanCombat);
						if (appendNarrative) appendNarrative(combatHtml, 'combat');
					}
					if (Array.isArray(msg.tactical_choices) && msg.tactical_choices.length > 0) {
						showTacticalChoices(msg.tactical_choices);
					} else if (msg.type === 'tactical_standoff') {
						showTacticalChoices(['趁勢搶攻', '專攻部位', '守勢游鬥', '抽身脫離']);
					} else if (msg.ended) {
						hideTacticalChoices();
					}
					break;
				case 'interact_result':
					// 物件互動結果：追加 log + 更新 observation_state
					if (window.onInteractResult) window.onInteractResult(msg);
					if (msg.log) {
						var cleanInteract = deNumericText(msg.log);
						var interactHtml = window.SwNarrative ? window.SwNarrative.format(cleanInteract) : escapeHtml(cleanInteract);
						if (appendNarrative) appendNarrative(interactHtml, 'interact');
					}
					break;
				case 'world_tick':
					// 世界自走：更新 L1/L2/L4，不推 log 條目
					if (window.onWorldTick) window.onWorldTick(msg);
					break;
					case 'scout_result':
						// 探路結果：更新方向路口顯示，不推 log
						if (window.onScoutResult) window.onScoutResult(msg);
						break;
				case 'npc_address':
					if (window.SwNotify && window.SwNotify.handleNpcAddress) {
						window.SwNotify.handleNpcAddress(msg);
					}
					break;
				case 'recap_item':
					if (window.SwNotify && window.SwNotify.recordRecap) {
						var rTitle = String(msg.title || '離線行旅回顧');
						var rBody = String(msg.body || '');
						window.SwNotify.recordRecap({
							category: msg.category || 'social',
							speaker: rTitle,
							html: '<strong>' + escapeHtml(rTitle) + '</strong><br>' + escapeHtml(rBody),
							plain: rTitle + '：' + rBody,
							unread: true
						});
					}
					break;
				case 'npc_dialogue':
					if (window.SwRecordNpcDialogue) {
						window.SwRecordNpcDialogue(msg.npc_a, msg.npc_b, msg.text);
					}
					break;
				case 'narrate':
					if (msg.text) {
						var t = String(msg.text);
						if (window.SwRecordTransitHint) {
							var enterMatch = t.match(/^(.+?)(?:自)([北南東西東北東南西北西南]+)(?:面)?(?:【[^】]*】)?(?:方向)?(?:邁步走來|邁步走入|行至此處|走來)/);
							if (enterMatch) {
								var eDir = enterMatch[2].replace(/面$/, '');
								var eCard = (eDir.indexOf('北') !== -1) ? '北' : (eDir.indexOf('南') !== -1) ? '南' : (eDir.indexOf('東') !== -1) ? '東' : '西';
								window.SwRecordTransitHint(enterMatch[1], 'enter', eCard);
							}
							var leaveMatch = t.match(/^(.+?)(?:邁步走入街道，往|轉身朝|匆匆趕往|沿著石板路邁步離去，往|往)([北南東西東北東南西北西南]+)(?:面)?(?:朝)?(?:【[^】]*】)?(?:方向)?(?:行去|離去)/);
							if (leaveMatch) {
								var lDir = leaveMatch[2].replace(/面$/, '');
								var lCard = (lDir.indexOf('北') !== -1) ? '北' : (lDir.indexOf('南') !== -1) ? '南' : (lDir.indexOf('東') !== -1) ? '東' : '西';
								window.SwRecordTransitHint(leaveMatch[1], 'leave', lCard);
							}
						}
						var isRoomStep = /^你(?:(?:往[東南西北])?(?:步入|踏入)【|(?:推開|拉開|穿過門廊步入))/.test(t);
						appendNarrative(window.SwNarrative.format(msg.text), isRoomStep ? 'Move' : 'ambient');
					}
					break;
				case 'inventory':
					renderInventoryContent(msg);
					break;
				case 'error':
					if (window.SwInteractions) window.SwInteractions.removeLogPendingTalk();
					if (window.SwInteractions) window.SwInteractions.removeLogPendingTrade();
					if (!state.me) {
						var authMsg = document.getElementById('auth-message');
						if (authMsg) authMsg.textContent = msg.message;
					} else {
						var invPaneErrEl = document.getElementById('player-modal-pane-inventory') || document.getElementById('modal-inventory');
						var invOverlayErrEl = document.getElementById('player-modal-overlay');
						if (invPaneErrEl && !invPaneErrEl.hidden && (!invOverlayErrEl || !invOverlayErrEl.hidden) && window.SwPanes && typeof window.SwPanes.showInventoryFeedback === 'function') {
							window.SwPanes.showInventoryFeedback(msg.message, true);
						}
						appendLog('錯誤：' + msg.message);
					}
					break;
					default:
						console.warn('[ws unhandled msg]', msg && msg.type);
				}
			} catch (e) {
				console.error('[onmessage throw]', e && (e.stack || e.message || e), 'msg=', ev.data && String(ev.data).slice(0,120));
			}
		};
		socket.onclose = function () {
			stopHeartbeat();
			state.me = null;
			window.SwAuth.showAuth();
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

	// pane 渲染移至 web/sw-panes.js（window.SwPanes）
	var _P = window.SwPanes || {};
	var renderStatusPane = _P.renderStatusPane;
	var renderEquipmentPane = _P.renderEquipmentPane;
	var renderSkillPane = _P.renderSkillPane;
	var renderStarplatePane = _P.renderStarplatePane;
	var renderInventoryContent = _P.renderInventoryContent;

	// modal 初始化移至 web/sw-modals.js
	if (window.initInventoryModal) window.initInventoryModal();
	if (window.initPlayerModal) window.initPlayerModal();

	window.SwClock.updateDisplay();
	window.gameConnect = connect;
	window.gameTryReconnect = tryReconnect;
	// sw-auth.js 需要存取 socket（送登入 send 前先查狀態）
	window._getSocket = function () { return socket; };

	// interactions 與戰術按鈕綁定
	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') {
			document.addEventListener('DOMContentLoaded', function () {
				window.SwAuth.bindForm();
				if (window.SwInteractions) window.SwInteractions.bindLogObjectActions();
				bindTacticalChoices();
			});
		} else {
			window.SwAuth.bindForm();
			if (window.SwInteractions) window.SwInteractions.bindLogObjectActions();
			bindTacticalChoices();
		}
	}

	window.gameSend = function (msg) {
		if (typeof msg === 'object') send(msg);
		else if (socket && socket.readyState === WebSocket.OPEN) socket.send(msg);
	};
	window.gameState = function () { return state; };
	window.gameIsConnected = isConnected;
	window.gameFormatNarrative = function (t) { return window.SwNarrative.format(t); };
	window.gameSendMoveDirection = sendMoveByDirection;
	window.gameStartTimeTicker = function () { window.SwClock.startTicker(); };
	window.gameUpdateTime = function () { window.SwClock.updateDisplay(); };
})();

document.addEventListener('DOMContentLoaded', function () {
	if (window.gameConnect) window.gameConnect();
	document.addEventListener('visibilitychange', function () {
		if (document.visibilityState === 'visible' && window.gameTryReconnect) window.gameTryReconnect();
	});
});

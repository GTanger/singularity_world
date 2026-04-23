// WebSocket 連線與遊戲主邏輯；登入、房間視野、依出口移動。傳統 MUD 節點連接節點。v0.20.46
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

	/** 同房實體互動：對齊後端 Borrow / Subdue / Slay（舊 Attack＝送行，後端已映射） */
	const ENTITY_INTERACT_LABELS = { Talk: '對話', Borrow: '借物', Subdue: '留人', Slay: '送行', Trade: '交易', Attack: '攻擊' };
	const ENTITY_INTERACT_ORDER = ['Talk', 'Borrow', 'Trade', 'Subdue', 'Slay'];

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
						window.SwNarrative.indexNames(state.entities);
						state.objects = Array.isArray(msg.objects) ? msg.objects : [];
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
						break;
					case 'grid_view':
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
						if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_PLAYER_ID, msg.player_id);
						window.myPlayerId = msg.player_id;
						// myPlayerId 到齊後，重新渲染 grid 物件欄/描述欄以過濾自己
						if (window.refreshGridView) window.refreshGridView();
						window.SwAuth.showGame();
						window.SwClock.updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
						window.SwClock.draw();
						appendLog('登入成功：' + msg.player_id + ' @ ' + (msg.room_name || msg.room_id));
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
								window.SwClock.updateStatusBars(msg.hp_cur, msg.hp_max, msg.inner_cur, msg.inner_max, msg.spirit_cur, msg.spirit_max, msg.stamina_cur, msg.stamina_max);
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
						if (window.SwInteractions) window.SwInteractions.removeLogPendingTalk();
						var narrative = msg.narrative;
						if (!narrative) narrative = '（NPC 無回應）';
						var narrativeHtml = window.SwNarrative.format(narrative);
						narrativeHtml = window.SwNarrative.formatWithClickable(narrativeHtml, state.objects);
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
						if (window.SwInteractions) window.SwInteractions.removeLogPendingTrade();
						var tNarrative = msg.narrative;
						if (!tNarrative) tNarrative = '（交易無回應）';
						var tHtml = window.SwNarrative.format(tNarrative);
						tHtml = window.SwNarrative.formatWithClickable(tHtml, state.objects);
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
						var bHtml = window.SwNarrative.format(bNarrative);
						bHtml = window.SwNarrative.formatWithClickable(bHtml, state.objects);
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
						var narrativeHtml = window.SwNarrative.format(msg.narrative);
						narrativeHtml = window.SwNarrative.formatWithClickable(narrativeHtml, state.objects);
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
						appendNarrative(window.SwNarrative.format(msg.text), 'ambient');
					}
					break;
				case 'inventory':
					renderInventoryContent(msg);
					break;
				case 'error':
					if (window.SwInteractions) window.SwInteractions.removeLogPendingTalk();
					if (window.SwInteractions) window.SwInteractions.removeLogPendingTrade();
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
				console.error('[onmessage throw]', e && (e.stack || e.message || e), 'msg=', ev.data && String(ev.data).slice(0,120));
				appendLog('收到：' + ev.data);
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

	// interactions 移至 web/sw-interactions.js（window.SwInteractions）
	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') {
			document.addEventListener('DOMContentLoaded', function () {
				window.SwAuth.bindForm();
				if (window.SwInteractions) window.SwInteractions.bindLogObjectActions();
			});
		} else {
			window.SwAuth.bindForm();
			if (window.SwInteractions) window.SwInteractions.bindLogObjectActions();
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
	console.log('[main.js] v0.20.46 loaded');
})();

document.addEventListener('DOMContentLoaded', function () {
	if (window.gameConnect) window.gameConnect();
	document.addEventListener('visibilitychange', function () {
		if (document.visibilityState === 'visible' && window.gameTryReconnect) window.gameTryReconnect();
	});
});

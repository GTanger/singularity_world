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
							room_name: msg.room_name
						};
						if (typeof localStorage !== 'undefined') localStorage.setItem(STORAGE_PLAYER_ID, msg.player_id);
						showGameAfterLogin();
						draw();
						appendLog('登入成功：' + msg.player_id + ' @ ' + (msg.room_name || msg.room_id));
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

	updateGameTimeDisplay();
	window.gameConnect = connect;
	window.gameTryReconnect = tryReconnect;
	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', bindAuthForm);
		else bindAuthForm();
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

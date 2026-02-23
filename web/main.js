// WebSocket 連線與遊戲主邏輯；登入、房間視野、依出口移動。傳統 MUD 節點連接節點。
(function () {
	const wsScheme = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
	const wsUrl = wsScheme + '//' + window.location.host + '/ws';
	let socket = null;

	const state = {
		room_id: '',
		room_name: '',
		description: '',
		exits: [],
		entities: [],
		me: null
	};

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

	function connect() {
		socket = new WebSocket(wsUrl);
		socket.onopen = function () {
			appendLog('已進入奇點世界');
			const playerId = window.prompt('輸入角色 ID 登入（測試用可填 player1）', 'player1') || 'player1';
			send({ type: 'login', player_id: playerId });
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
						draw();
						break;
					case 'me':
						state.me = {
							player_id: msg.player_id,
							room_id: msg.room_id,
							room_name: msg.room_name
						};
						draw();
						appendLog('登入成功：' + msg.player_id + ' @ ' + (msg.room_name || msg.room_id));
						break;
					case 'moved':
						if (state.me && (msg.player_id === state.me.player_id || msg.player_id === state.me.id)) {
							state.me.room_id = msg.room_id;
							state.me.room_name = msg.room_name;
						}
						appendLog('移動到：' + (msg.room_name || msg.room_id));
						// 伺服器會在 move 後再送一次 view，這裡僅更新 me；若未收到新 view 可稍後重繪
						draw();
						break;
					case 'blocked':
						appendLog('無法往「' + (msg.direction || '') + '」移動');
						break;
					case 'error':
						appendLog('錯誤：' + msg.message);
						break;
					default:
						appendLog('收到：' + ev.data);
				}
			} catch (e) {
				appendLog('收到：' + ev.data);
			}
		};
		socket.onclose = function () {
			appendLog('連線關閉');
		};
		socket.onerror = function () {
			appendLog('連線錯誤');
		};
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

	window.gameConnect = connect;
	window.gameSend = function (msg) {
		if (typeof msg === 'object') send(msg);
		else if (socket && socket.readyState === WebSocket.OPEN) socket.send(msg);
	};
	window.gameState = function () { return state; };
	window.gameSendMoveDirection = sendMoveByDirection;
})();

document.addEventListener('DOMContentLoaded', function () {
	if (window.gameConnect) window.gameConnect();
	// 出口按鈕由 mudRenderExitButtons 在 draw() 時綁定，無需表單 submit
});

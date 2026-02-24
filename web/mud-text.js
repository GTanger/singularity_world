// 純文字 MUD：房間制，依 UI 設計圖填入房間名、描述、同房人物、路徑(出口)。
(function () {
	function escapeHtml(s) {
		if (!s) return '';
		return String(s)
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;');
	}

	function formatDesc(desc) {
		if (!desc) return '';
		var safe = escapeHtml(desc);
		return safe.replace(/【([^】]*)】/g, '<span class="desc-highlight">【$1】</span>');
	}

	function updateRoomView(roomName, description, exits, entities, me) {
		var nameEl = document.getElementById('room-name');
		var descEl = document.getElementById('room-desc');
		var listEl = document.getElementById('entities-list');
		if (nameEl) nameEl.textContent = roomName || '';
		if (descEl) descEl.innerHTML = description ? formatDesc(description) : '';
		if (listEl) {
			listEl.innerHTML = '';
			if (entities && entities.length > 0) {
				var myId = me && (me.player_id || me.playerID);
				entities.forEach(function (e) {
					if (myId && (e.id || e.ID) === myId) return;
					var li = document.createElement('li');
					li.textContent = (e.display_char || e.id || e.ID || '?').toString();
					listEl.appendChild(li);
				});
			}
		}
	}

	function renderExitButtons(containerId, exits, onDirection) {
		var wrap = document.getElementById(containerId);
		if (!wrap) return;
		wrap.innerHTML = '';
		if (!exits || !exits.length) {
			var span = document.createElement('span');
			span.className = 'text-muted';
			span.textContent = '無出口';
			wrap.appendChild(span);
			return;
		}
		exits.forEach(function (ex) {
			var btn = document.createElement('button');
			btn.type = 'button';
			btn.textContent = ex.to_room_name || ex.direction;
			btn.className = 'exit-btn';
			btn.title = ex.direction;
			btn.addEventListener('click', function () {
				if (typeof onDirection === 'function') onDirection(ex.direction);
			});
			wrap.appendChild(btn);
		});
	}

	window.mudUpdateRoomView = function (roomName, description, exits, entities, me) {
		updateRoomView(roomName, description, exits, entities, me);
	};
	window.mudRenderExitButtons = renderExitButtons;
})();

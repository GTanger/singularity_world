// 純文字 MUD：房間制，依 UI 設計圖填入房間名、描述、同房人物、路徑(出口)。
// 實體清單支援插頭插座：點擊展開動作選單（觀看/對話/攻擊），對齊決策 002。
(function () {
	var ACTION_LABELS = {
		'Look': '觀看',
		'Talk': '對話',
		'Attack': '攻擊'
	};

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
					var eid = (e.id || e.ID || '').toString();
					if (myId && eid === myId) return;

					var displayName = e.display_name || eid;
					var actions = e.actions || [];
					var li = document.createElement('li');
					li.className = 'entity-row';
					if (e.kind === 'npc') li.classList.add('entity-npc');
					li.setAttribute('data-entity-id', eid);
					li.innerHTML = '<span class="entity-arrow">\u25b8</span> ' + escapeHtml(displayName);
					li.setAttribute('role', 'button');
					li.setAttribute('tabindex', '0');
					li.title = '點擊展開動作';
					listEl.appendChild(li);

					li.addEventListener('click', function () {
						var existing = listEl.querySelector('.entity-actions-expand');
						var wasThis = existing && existing.getAttribute('data-entity-id') === eid;
						if (existing) {
							var prev = existing.previousElementSibling;
							if (prev) {
								prev.classList.remove('expanded');
								var arrow = prev.querySelector('.entity-arrow');
								if (arrow) arrow.textContent = '\u25b8';
							}
							existing.remove();
						}
						listEl.querySelectorAll('.entity-row.expanded').forEach(function (el) {
							el.classList.remove('expanded');
							var a = el.querySelector('.entity-arrow');
							if (a) a.textContent = '\u25b8';
						});
						if (wasThis) return;

						li.classList.add('expanded');
						var arrow = li.querySelector('.entity-arrow');
						if (arrow) arrow.textContent = '\u25be';

						var expand = document.createElement('li');
						expand.className = 'entity-actions-expand';
						expand.setAttribute('data-entity-id', eid);
						var btns = '';
						for (var i = 0; i < actions.length; i++) {
							var act = actions[i];
							var label = ACTION_LABELS[act] || act;
							btns += '<button type="button" class="entity-action-btn" data-action="' + escapeHtml(act) + '" data-target="' + escapeHtml(eid) + '">' + escapeHtml(label) + '</button>';
						}
						expand.innerHTML = btns;
						li.after(expand);

						expand.querySelectorAll('.entity-action-btn').forEach(function (btn) {
							btn.addEventListener('click', function (ev) {
								ev.stopPropagation();
								var action = btn.getAttribute('data-action');
								var target = btn.getAttribute('data-target');
								if (action === 'Look' && window.openCharacterModal) {
									window.openCharacterModal(displayName, target);
								}
								if (window.gameSend) {
									window.gameSend({ type: 'do_action', entity_id: target, action: action });
								}
							});
						});
					});
					li.addEventListener('keydown', function (ev) {
						if (ev.key === 'Enter' || ev.key === ' ') {
							ev.preventDefault();
							li.click();
						}
					});
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
			btn.textContent = ex.direction;
			btn.className = 'exit-btn';
			btn.title = ex.to_room_name ? ex.direction + ' → ' + ex.to_room_name : ex.direction;
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

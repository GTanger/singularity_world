// 純文字 MUD：房間制，依 UI 設計圖填入房間名、描述、同房人物、路徑(出口)。
// 實體清單支援插頭插座：點擊展開動作選單（觀看/對話/攻擊），對齊決策 002。
// 房間描述內 〔〕 為可互動物件，點擊後在行內浮動下拉展開動作。
(function () {
	var ACTION_LABELS = {
		'Look': '觀看',
		'Talk': '對話',
		'Attack': '攻擊',
		'Read': '閱讀',
		'Smell': '嗅聞'
	};

	var currentRoomObjects = {}; // id -> { id, name, actions }
	var activeObjectSpan = null;

	function escapeHtml(s) {
		if (!s) return '';
		return String(s)
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;');
	}

	function formatDesc(desc, objects) {
		if (!desc) return '';
		currentRoomObjects = {};
		if (objects && objects.length) {
			objects.forEach(function (o) {
				currentRoomObjects[o.id] = { id: o.id, name: o.name, actions: o.actions || [] };
			});
		}
		var safe = escapeHtml(desc);
		safe = safe.replace(/【([^】]*)】/g, '<span class="desc-highlight">【$1】</span>');
		// 全形方頭括號 U+3014 / U+3015：一律產成可點擊 span，有物件資料時才帶 data-object-id
		safe = safe.replace(/\u3014([^\u3015]*)\u3015/g, function (match, name) {
			var obj = null;
			for (var id in currentRoomObjects) {
				if (currentRoomObjects[id].name === name) {
					obj = currentRoomObjects[id];
					break;
				}
			}
			var idAttr = obj ? ' data-object-id="' + escapeHtml(obj.id) + '"' : '';
			return '<span class="desc-object" data-object-name="' + escapeHtml(name) + '"' + idAttr + ' role="button" tabindex="0">\u3014' + escapeHtml(name) + '\u3015</span>';
		});
		// 後備：描述沒有 〔〕 但伺服器有 objects 時，用物件名稱替換成可點擊（DB 舊描述時仍能點）
		if (objects && objects.length && safe.indexOf('desc-object') === -1) {
			objects.forEach(function (o) {
				var name = o.name;
				if (!name) return;
				var escapedName = escapeHtml(name);
				var span = '<span class="desc-object" data-object-id="' + escapeHtml(o.id) + '" data-object-name="' + escapedName + '" role="button" tabindex="0">' + escapedName + '</span>';
				safe = safe.replace(escapedName, span);
			});
		}
		return safe;
	}

	function findDescObjectAncestor(el) {
		while (el && el !== document.body) {
			if (el.classList && el.classList.contains('desc-object')) return el;
			el = el.parentElement;
		}
		return null;
	}

	// 點擊物件即送「觀看」，觀看敘述與其他動作由 main.js 在 log 中顯示
	function sendObjectLook(objectEl) {
		var objectId = objectEl.getAttribute('data-object-id');
		var objectName = objectEl.getAttribute('data-object-name') || '';
		if (!objectId && !objectName) return;
		if (window.gameSend) {
			window.gameSend({ type: 'do_action', entity_id: objectId || objectName, action: 'Look' });
		}
	}

	function updateRoomView(roomName, description, exits, entities, me, objects) {
		var nameEl = document.getElementById('room-name');
		var descEl = document.getElementById('room-desc');
		var listEl = document.getElementById('entities-list');
		if (nameEl) nameEl.textContent = roomName || '';
		if (descEl) {
			descEl.innerHTML = description ? formatDesc(description, objects) : '';
		}
		if (listEl) {
			listEl.innerHTML = '';
			if (entities && entities.length > 0) {
				var myId = me && (me.player_id || me.playerID);
				entities.forEach(function (e) {
					var eid = (e.id || e.ID || '').toString();
					if (myId && eid === myId) return;

					var displayName = e.display_name || eid;
					var li = document.createElement('li');
					li.className = 'entity-row';
					if (e.kind === 'npc') li.classList.add('entity-npc');
					li.setAttribute('data-entity-id', eid);
					li.innerHTML = '<span class="entity-arrow">\u25b8</span> ' + escapeHtml(displayName);
					li.setAttribute('role', 'button');
					li.setAttribute('tabindex', '0');
					li.title = '點擊觀看';
					listEl.appendChild(li);

					li.addEventListener('click', function () {
						if (window.gameSend) {
							window.gameSend({ type: 'do_action', entity_id: eid, action: 'Look' });
						}
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

	document.addEventListener('click', function (ev) {
		var objSpan = findDescObjectAncestor(ev.target);
		var panel = document.getElementById('room-desc-panel');
		if (objSpan && panel && panel.contains(objSpan)) {
			ev.preventDefault();
			ev.stopPropagation();
			sendObjectLook(objSpan);
		}
	});

	window.mudUpdateRoomView = function (roomName, description, exits, entities, me, objects) {
		updateRoomView(roomName, description, exits, entities, me, objects);
	};
	window.mudRenderExitButtons = renderExitButtons;
})();

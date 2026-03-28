// 純文字 MUD：房間制，依 UI 設計圖填入房間名、描述、同房人物、路徑(出口)。
// 實體清單支援插頭插座：點擊展開動作選單（觀看/對話/攻擊），對齊決策 002。
// 房間描述內 〔〕 為可互動物件，點擊後在行內浮動下拉展開動作。
(function () {
	var ACTION_LABELS = {
		'Look': '觀看',
		'Talk': '對話',
		'Borrow': '借物',
		'Subdue': '留人',
		'Slay': '送行',
		'Attack': '攻擊',
		'Trade': '交易',
		'Read': '閱讀',
		'Smell': '嗅聞',
		'Move': '移動'
	};

	var ACTION_ICONS = {
		'Look': '👁',
		'Take': '✋',
		'Gather': '⛏',
		'Open': '📦',
		'Read': '📜',
		'Use': '⚡',
		'Talk': '💬',
		'Strip': '🔮',
		'Butcher': '🔪'
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
		var safe = escapeHtml(desc);
		// 只做排版處理：換行 → <br>，【】→ 綠色強調
		safe = safe.replace(/\n/g, '<br>');
		safe = safe.replace(/【([^】]*)】/g, '<span class="desc-highlight">【$1】</span>');
		
		/* ── 舊版邏輯：將物件名稱替換成可點擊 span（已由下方獨立互動區取代，暫不移除以備回退） ──
		currentRoomObjects = {};
		if (objects && objects.length) {
			objects.forEach(function (o) {
				currentRoomObjects[o.id] = { id: o.id, name: o.name, actions: o.actions || [] };
			});
		}
		// ... (其餘 regex 邏輯) ...
		*/
		return safe;
	}

	function findDescObjectAncestor(el) {
		while (el && el !== document.body) {
			if (el.classList && el.classList.contains('desc-object')) return el;
			el = el.parentElement;
		}
		return null;
	}

	// 點擊物件：有 Move 則直接移動（如〔厚重竹簾〕）；無 Move 才送觀看
	function sendObjectClick(objectEl) {
		var objectId = objectEl.getAttribute('data-object-id');
		var objectName = objectEl.getAttribute('data-object-name') || '';
		var actionsStr = objectEl.getAttribute('data-object-actions') || '';
		var actions = actionsStr ? actionsStr.split(',') : [];
		var hasMove = actions.indexOf('Move') !== -1;
		var action = hasMove ? 'Move' : 'Look';
		if (!objectId && !objectName) return;
		if (window.gameSend) {
			window.gameSend({ type: 'do_action', entity_id: objectId || objectName, action: action });
		}
	}

	function renderRoomActions(exits, objects) {
		var el = document.getElementById('room-actions');
		if (!el) return;
		var html = '';

		// ─── 出口 ───
		if (exits && exits.length > 0) {
			html += '<div class="action-group">';
			html += '<span class="action-label">🚪 出口</span>';
			exits.forEach(function (ex) {
				html += '<button type="button" class="action-btn action-move" '
					+ 'data-direction="' + escapeHtml(ex.direction) + '" '
					+ 'title="前往 ' + escapeHtml(ex.to_room_name || '') + '">'
					+ escapeHtml(ex.direction)
					+ '</button>';
			});
			html += '</div>';
		}

		// ─── 物件（按動作分組）───
		// Move 類型的物件已由 exits 處理，跳過
		var nonMoveObjects = (objects || []).filter(function (o) {
			return !o.actions || o.actions.indexOf('Move') === -1;
		});

		if (nonMoveObjects.length > 0) {
			html += '<div class="action-group">';
			html += '<span class="action-label">👁 可互動</span>';
			nonMoveObjects.forEach(function (o) {
				var actions = o.actions || ['Look'];
				// 主動作 = 第一個非 Look 的動作，沒有就用 Look
				var primary = actions.find(function (a) { return a !== 'Look'; }) || 'Look';
				var icon = ACTION_ICONS[primary] || '❓';
				html += '<button type="button" class="action-btn action-' + primary.toLowerCase() + '" '
					+ 'data-object-id="' + escapeHtml(o.id) + '" '
					+ 'data-object-name="' + escapeHtml(o.name) + '" '
					+ 'data-action="' + escapeHtml(primary) + '" '
					+ 'title="' + escapeHtml(primary + '：' + o.name) + '">'
					+ icon + ' ' + escapeHtml(o.name)
					+ '</button>';
			});
			html += '</div>';
		}

		el.innerHTML = html;
	}

	function updateRoomView(roomName, description, exits, entities, me, objects) {
		var line1El = document.getElementById('room-name-line1');
		var line2El = document.getElementById('room-name-line2');
		var nameWrap = document.getElementById('room-name');
		var descEl = document.getElementById('room-desc');
		var presenceEl = document.getElementById('room-presence');
		// 房間名稱上四下四：若為「X段」則上排 X、下排 N段（置中）
		var name = roomName || '';
		if (line1El && line2El && nameWrap) {
			var match = name.match(/^(.+?)([一二三四五六七八九十百]+段)$/);
			if (match) {
				line1El.textContent = match[1];
				line2El.textContent = match[2];
				line2El.classList.remove('room-name-line2-empty');
			} else {
				line1El.textContent = name;
				line2El.textContent = '';
				line2El.classList.add('room-name-line2-empty');
			}
			nameWrap.setAttribute('aria-label', name);
		}
		if (descEl) {
			descEl.innerHTML = description ? formatDesc(description, objects) : '';
		}
		renderRoomActions(exits, objects);
		if (presenceEl) {
			presenceEl.innerHTML = '';
			var myId = me && (me.player_id || me.playerID);
			var others = [];
			(entities || []).forEach(function (e) {
				var eid = (e.id || e.ID || '').toString();
				if (myId && eid === myId) return;
				others.push({
					id: eid,
					name: e.display_name || eid
				});
			});
			if (!others.length) {
				presenceEl.innerHTML = '<span class="room-presence-label">你看見：</span><span class="text-muted">此處暫無他人</span>';
				return;
			}
			var html = '<span class="room-presence-label">你看見：</span>';
			html += others.map(function (x) {
				return '<span class="room-presence-entity" role="button" tabindex="0" data-entity-id="' + escapeHtml(x.id) + '" data-entity-name="' + escapeHtml(x.name) + '">' + escapeHtml(x.name) + '</span>';
			}).join('<span class="room-presence-sep">、</span>');
			presenceEl.innerHTML = html;
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
			sendObjectClick(objSpan);
		}
	});
	document.addEventListener('click', function (ev) {
		var el = ev.target.closest && ev.target.closest('.room-presence-entity');
		if (!el) return;
		ev.preventDefault();
		var entityID = el.getAttribute('data-entity-id');
		if (!entityID || !window.gameSend) return;
		window.gameSend({ type: 'do_action', entity_id: entityID, action: 'Look' });
	});
	document.addEventListener('keydown', function (ev) {
		var el = ev.target && ev.target.closest && ev.target.closest('.room-presence-entity');
		if (!el) return;
		if (ev.key !== 'Enter' && ev.key !== ' ') return;
		ev.preventDefault();
		var entityID = el.getAttribute('data-entity-id');
		if (!entityID || !window.gameSend) return;
		window.gameSend({ type: 'do_action', entity_id: entityID, action: 'Look' });
	});

	// 出口按鈕
	document.addEventListener('click', function (ev) {
		var btn = ev.target.closest && ev.target.closest('.action-move');
		if (!btn) return;
		var dir = btn.getAttribute('data-direction');
		if (dir && window.gameSend) window.gameSend({ type: 'move', direction: dir });
	});

	// 物件按鈕
	document.addEventListener('click', function (ev) {
		var btn = ev.target.closest && ev.target.closest('.action-btn[data-object-id]');
		if (!btn) return;
		var objectId = btn.getAttribute('data-object-id');
		var action = btn.getAttribute('data-action');
		if (objectId && action && window.gameSend) {
			window.gameSend({ type: 'do_action', entity_id: objectId, action: action });
		}
	});

	window.mudUpdateRoomView = function (roomName, description, exits, entities, me, objects) {
		updateRoomView(roomName, description, exits, entities, me, objects);
	};
	window.mudRenderExitButtons = renderExitButtons;
})();

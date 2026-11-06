// talk/trade 輸入列、log pending 訊息、log 物件點擊派發；自包含，依賴 window.SwLog / window.gameSend / window.gameState / window.gameFormatNarrative。v0.20.45
(function () {
	function getLogEl() {
		var backdrop = document.getElementById('sw-object-popover-backdrop');
		var popLog = document.getElementById('sw-popover-log');
		if (backdrop && !backdrop.hidden && backdrop.style.display !== 'none' && popLog) {
			return popLog;
		}
		return document.getElementById('sw-log') || document.getElementById('log');
	}

	function appendLogPendingTalk() {
		var logEl = getLogEl();
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-talk-pending';
		div.setAttribute('data-pending', 'talk');
		div.textContent = '對話中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTalk() {
		var logEl = getLogEl();
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-talk-pending');
		for (var i = 0; i < pending.length; i++) pending[i].remove();
		var pendingInputs = logEl.querySelectorAll('.log-talk-input.is-pending-reply');
		for (var k = 0; k < pendingInputs.length; k++) {
			var pWrap = pendingInputs[k];
			pWrap.classList.remove('is-pending-reply');
			var pInput = pWrap.querySelector('.log-talk-input-field');
			if (pInput) pInput.disabled = false;
			var pBtn = pWrap.querySelector('.log-talk-send');
			if (pBtn) {
				pBtn.disabled = false;
				pBtn.textContent = '送出';
			}
			var pChips = pWrap.querySelectorAll('.log-talk-chip');
			for (var ci = 0; ci < pChips.length; ci++) {
				pChips[ci].disabled = false;
			}
			if (logEl.lastElementChild !== pWrap) {
				logEl.appendChild(pWrap);
			}
		}
	}

	function appendLogPendingTrade() {
		var logEl = getLogEl();
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-trade-pending';
		div.setAttribute('data-pending', 'trade');
		div.textContent = '交易中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTrade() {
		var logEl = getLogEl();
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-trade-pending');
		for (var j = 0; j < pending.length; j++) pending[j].remove();
	}

	function resolveCleanTargetName(entityId, rawTargetName) {
		if (entityId && window.SwObjectList && typeof window.SwObjectList.getKnownNpcName === 'function') {
			var known = window.SwObjectList.getKnownNpcName(entityId);
			if (known) return known;
		}
		var name = (rawTargetName || '').trim();
		if (name && !/^wt_npc_|^npc_|^wt_citizen_/i.test(name) && !/^陌生人|^眼前之人|^行人$/.test(name)) {
			return name;
		}
		if (entityId) {
			try {
				var leftBtn = document.querySelector('.sw-canvas-token[data-entity-id="' + entityId + '"], .sw-person-card[data-entity-id="' + entityId + '"]');
				if (leftBtn) {
					var nameSpan = leftBtn.querySelector('.sw-token-label, .sw-person-name');
					var foundName = (nameSpan ? nameSpan.textContent : leftBtn.textContent).trim();
					if (foundName && !/^wt_npc_|^npc_|^wt_citizen_/i.test(foundName) && !/^(對話|交易|觀察|打量|切磋|生死戰)$/.test(foundName)) {
						return foundName.replace(/^陌生人\s*\d+$/, '陌生人');
					}
				}
			} catch (e) {}
		}
		if (/^陌生人/.test(name)) {
			return '陌生人';
		}
		return name || '陌生人';
	}

	function shortEntityName(name, fallback) {
		var raw = String(name || fallback || '').trim();
		var parts = raw.split(/[|｜・]/);
		return parts[parts.length - 1].trim() || raw;
	}

	function appendTalkInputRow(entityId, targetName, opts) {
		opts = opts || {};
		var autoFocus = (opts.autoFocus !== false);
		targetName = resolveCleanTargetName(entityId, targetName);
		var logEl = getLogEl();
		if (!logEl) return;
		var sName = shortEntityName(targetName, entityId);
		var targetEntityStr = String(entityId);

		var existingWrap = logEl.querySelector('.log-talk-input[data-entity-id="' + targetEntityStr + '"]');
		var otherInputs = logEl.querySelectorAll('.log-talk-input, .log-trade-input');
		for (var oi = 0; oi < otherInputs.length; oi++) {
			if (otherInputs[oi] !== existingWrap) {
				otherInputs[oi].remove();
			}
		}

		if (existingWrap) {
			existingWrap.classList.remove('is-pending-reply');
			var exInput = existingWrap.querySelector('.log-talk-input-field');
			if (exInput) {
				exInput.disabled = false;
				exInput.placeholder = '對 ' + sName + ' 說些什麼（直接送出即搭話）';
			}
			var exBtn = existingWrap.querySelector('.log-talk-send');
			if (exBtn) {
				exBtn.disabled = false;
				exBtn.textContent = '送出';
			}
			var exChips = existingWrap.querySelectorAll('.log-talk-chip');
			for (var chi = 0; chi < exChips.length; chi++) {
				exChips[chi].disabled = false;
			}
			if (logEl.lastElementChild !== existingWrap) {
				logEl.appendChild(existingWrap);
			}
			var scrollEl = logEl.closest && (logEl.closest('#sw-center-log') || logEl.closest('.sw-center-log') || logEl.closest('.sw-desc-panel')) || logEl;
			if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
			if (existingWrap.scrollIntoView) {
				existingWrap.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
			}
			if (autoFocus && exInput) {
				exInput.focus();
			}
			return;
		}

		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-talk-input';
		wrap.setAttribute('data-entity-id', targetEntityStr);

		// 快捷話語按鈕（一鍵寒暄問候／打聽近況／請教差事）
		var chipsWrap = document.createElement('div');
		chipsWrap.className = 'log-talk-chips-row';
		chipsWrap.style.display = 'flex';
		chipsWrap.style.gap = '6px';
		chipsWrap.style.width = '100%';
		chipsWrap.style.marginBottom = '4px';

		var topics = ['寒暄問候', '打聽近況', '請教差事'];
		for (var ti = 0; ti < topics.length; ti++) {
			var chipBtn = document.createElement('button');
			chipBtn.type = 'button';
			chipBtn.className = 'sw-popover-act-btn log-talk-chip';
			chipBtn.textContent = topics[ti];
			chipBtn.setAttribute('data-topic', topics[ti]);
			chipsWrap.appendChild(chipBtn);
		}
		wrap.appendChild(chipsWrap);

		var inputRow = document.createElement('div');
		inputRow.className = 'log-talk-input-row';
		inputRow.style.display = 'flex';
		inputRow.style.alignItems = 'center';
		inputRow.style.gap = '6px';
		inputRow.style.width = '100%';

		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-talk-input-field';
		input.placeholder = '對 ' + sName + ' 說些什麼（直接送出即搭話）';
		input.setAttribute('maxlength', '200');
		input.setAttribute('aria-label', '對 ' + sName + ' 說');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-talk-send';
		btn.textContent = '送出';
		inputRow.appendChild(input);
		inputRow.appendChild(btn);
		wrap.appendChild(inputRow);
		logEl.appendChild(wrap);
		var scrollEl = logEl.closest && (logEl.closest('#sw-center-log') || logEl.closest('.sw-center-log') || logEl.closest('.sw-desc-panel')) || logEl;
		if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
		if (wrap.scrollIntoView) {
			wrap.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		}
		if (autoFocus) {
			input.focus();
		}
		input.addEventListener('focus', function () {
			if (typeof window.SwResetPopoverIdleTimer === 'function') window.SwResetPopoverIdleTimer();
		});
		input.addEventListener('input', function () {
			if (typeof window.SwResetPopoverIdleTimer === 'function') window.SwResetPopoverIdleTimer();
		});
		function sendTalk(customText) {
			var text = (typeof customText === 'string') ? customText.trim() : (input.value || '').trim();
			var sayText = text || '（搭話）';
			input.value = '';
			wrap.classList.add('is-pending-reply');
			input.disabled = true;
			btn.disabled = true;
			btn.textContent = '候覆…';
			var chipButtons = chipsWrap.querySelectorAll('.log-talk-chip');
			for (var cbi = 0; cbi < chipButtons.length; cbi++) {
				chipButtons[cbi].disabled = true;
			}
			var playerLine = '你對【' + (targetName || entityId) + '】說：「' + sayText + '」';
			var fmt = window.gameFormatNarrative || function (t) { return t; };
			var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
			appendNarr(fmt(playerLine), 'Talk');
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Talk', player_input: sayText });
				appendLogPendingTalk();
			}
			if (logEl.lastElementChild !== wrap) {
				logEl.appendChild(wrap);
			}
			var sEl = logEl.closest && (logEl.closest('#sw-center-log') || logEl.closest('.sw-center-log') || logEl.closest('.sw-desc-panel')) || logEl;
			if (sEl) sEl.scrollTop = sEl.scrollHeight;
			if (wrap.scrollIntoView) {
				wrap.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
			}
		}
		chipsWrap.addEventListener('click', function (e) {
			var chip = e.target.closest && e.target.closest('.log-talk-chip');
			if (chip && !chip.disabled) {
				e.preventDefault();
				e.stopPropagation();
				var topicText = chip.getAttribute('data-topic') || chip.textContent || '';
				sendTalk(topicText);
			}
		});
		btn.addEventListener('click', function (e) {
			if (e && e.stopPropagation) e.stopPropagation();
			if (!btn.disabled) {
				sendTalk();
			}
		});
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter' && !input.disabled) {
				e.preventDefault();
				sendTalk();
			}
		});
	}

	function appendTradeInputRow(entityId, targetName) {
		targetName = resolveCleanTargetName(entityId, targetName);
		var logEl = getLogEl();
		if (!logEl) return;
		var existing = logEl.querySelectorAll('.log-talk-input, .log-trade-input');
		for (var ei = 0; ei < existing.length; ei++) existing[ei].remove();
		var sName = shortEntityName(targetName, entityId);
		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-trade-input';
		var label = document.createElement('span');
		label.className = 'log-trade-label';
		label.textContent = '出價（' + sName + '）：';
		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-trade-input-field';
		input.placeholder = '輸入整數鎂';
		input.setAttribute('inputmode', 'numeric');
		input.setAttribute('maxlength', '32');
		input.setAttribute('aria-label', '向 ' + sName + ' 交易出價');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-trade-send';
		btn.textContent = '出價';
		var cancelBtn = document.createElement('button');
		cancelBtn.type = 'button';
		cancelBtn.className = 'log-trade-cancel';
		cancelBtn.textContent = '拒絕';
		wrap.appendChild(label);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		wrap.appendChild(cancelBtn);
		logEl.appendChild(wrap);
		var scrollEl = logEl.closest && (logEl.closest('#sw-center-log') || logEl.closest('.sw-center-log') || logEl.closest('.sw-desc-panel')) || logEl;
		if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
		if (wrap.scrollIntoView) {
			wrap.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
		}
		input.focus();
		function sendTrade() {
			var text = (input.value || '').trim();
			wrap.remove();
			if (text) {
				var playerLine = '你向【' + (targetName || entityId) + '】出價：「' + text + '」';
				var fmt = window.gameFormatNarrative || function (t) { return t; };
				var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
				appendNarr(fmt(playerLine), 'Trade');
			}
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Trade', player_input: text });
				appendLogPendingTrade();
			}
		}
		btn.addEventListener('click', function (e) {
			if (e && e.stopPropagation) e.stopPropagation();
			sendTrade();
		});
		cancelBtn.addEventListener('click', function (e) {
			if (e && e.stopPropagation) e.stopPropagation();
			wrap.remove();
			var playerLine = '你向【' + (targetName || entityId) + '】回絕了交易：「拒絕」';
			var fmt = window.gameFormatNarrative || function (t) { return t; };
			var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
			appendNarr(fmt(playerLine), 'Trade');
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Trade', player_input: '拒絕' });
				appendLogPendingTrade();
			}
		});
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter') {
				e.preventDefault();
				sendTrade();
			}
		});
	}

	function bindLogObjectActions() {
		var centerEl = document.getElementById('sw-center-log') || getLogEl();
		if (!centerEl) return;
		if (centerEl.__swObjectActionsBound) return;
		centerEl.__swObjectActionsBound = true;

		function handleActionClick(ev) {
			if (window.SwResetPopoverIdleTimer) window.SwResetPopoverIdleTimer();
			var toggle = ev.target.closest && ev.target.closest('.log-conflict-toggle');
			if (toggle) {
				ev.preventDefault();
				var container = toggle.closest('.sw-log-entry') || toggle.closest('.log-conflict-container') || toggle.parentElement;
				var group = container ? container.querySelector('.log-conflict-group') : null;
				if (group) {
					var isHidden = group.style.display === 'none' || group.hidden;
					if (isHidden) {
						var isPopover = !!toggle.closest('#sw-popover-actions');
						group.style.display = isPopover ? 'inline-flex' : 'inline';
						if (isPopover) {
							group.style.gap = '6px';
							group.style.marginLeft = '6px';
						}
						group.removeAttribute('hidden');
						toggle.textContent = '衝突 ▴';
					} else {
						group.style.display = 'none';
						group.setAttribute('hidden', '');
						toggle.textContent = '衝突 ▾';
					}
				}
				return;
			}
			var btn = ev.target.closest && ev.target.closest('.log-object-action');
			if (!btn) return;
			ev.preventDefault();

			var moveDir = btn.getAttribute('data-move-dir');
			if (moveDir) {
				var moveTarget = btn.getAttribute('data-move-target') || '';
				if (window.gameSend) {
					window.gameSend({ type: 'move', direction: moveDir, target_place_id: moveTarget || undefined });
				}
				return;
			}

			var id = btn.getAttribute('data-entity-id');
			var action = btn.getAttribute('data-action');
			var targetType = btn.getAttribute('data-target-type') || '';
			var targetName = btn.getAttribute('data-target-name') || '';
			if (btn.classList.contains('desc-object')) {
				var allDesc = document.querySelectorAll('#sw-l3-terrain .desc-object');
				for (var di = 0; di < allDesc.length; di++) {
					allDesc[di].classList.remove('is-active-target');
				}
				btn.classList.add('is-active-target');
			}

			if (action === 'Explore') {
				if (window.gameSend) window.gameSend({ type: 'explore' });
				return;
			}

			if (!id || !action || !window.gameSend) return;
			if (targetType === 'entity') {
				var st = window.gameState ? window.gameState() : {};
				var entities = (st && Array.isArray(st.entities) && st.entities.length > 0)
					? st.entities
					: ((window.SwGrid && window.SwGrid.mapData && Array.isArray(window.SwGrid.mapData.entities)) ? window.SwGrid.mapData.entities : []);
				var stillHere = false;
				if (window.__currentPopoverTarget && ((window.__currentPopoverTarget.id || '') + '') === (id + '')) {
					stillHere = true;
				}
				for (var i = 0; !stillHere && i < entities.length; i++) {
					if (((entities[i].id || '') + '') === (id + '')) {
						stillHere = true;
						break;
					}
				}
				if (!stillHere && (
					document.querySelector('#sw-room-canvas [data-object-id="' + id + '"]') ||
					document.querySelector('#sw-room-canvas [data-token-key="npc:' + id + '"]') ||
					document.querySelector('#sw-popover-actions [data-entity-id="' + id + '"]')
				)) {
					stillHere = true;
				}
				if (!stillHere) {
					var appendLog = (window.SwLog && window.SwLog.appendLog) || function () {};
					appendLog('對方已不在此處。');
					return;
				}
			}
			if (action === 'Talk') {
				appendTalkInputRow(id, targetName);
				return;
			}
			if (action === 'Trade') {
				window.gameSend({ type: 'do_action', entity_id: id, action: 'Trade', player_input: '' });
				appendLogPendingTrade();
				return;
			}
			window.gameSend({ type: 'do_action', entity_id: id, action: action });
		}

		centerEl.addEventListener('click', handleActionClick);
		centerEl.addEventListener('keydown', function (ev) {
			if (ev.key === 'Enter' || ev.key === ' ') {
				var btn = ev.target.closest && ev.target.closest('.log-object-action');
				if (btn) {
					ev.preventDefault();
					handleActionClick(ev);
				}
			}
		});

		var popoverEl = document.getElementById('sw-object-popover');
		if (popoverEl && !popoverEl.__swObjectActionsBound) {
			popoverEl.__swObjectActionsBound = true;
			popoverEl.addEventListener('click', handleActionClick);
			popoverEl.addEventListener('keydown', function (ev) {
				if (ev.key === 'Enter' || ev.key === ' ') {
					var btn = ev.target.closest && ev.target.closest('.log-object-action');
					if (btn) {
						ev.preventDefault();
						handleActionClick(ev);
					}
				}
			});
		}
	}

	window.SwInteractions = {
		appendLogPendingTalk: appendLogPendingTalk,
		removeLogPendingTalk: removeLogPendingTalk,
		appendLogPendingTrade: appendLogPendingTrade,
		removeLogPendingTrade: removeLogPendingTrade,
		appendTalkInputRow: appendTalkInputRow,
		appendTradeInputRow: appendTradeInputRow,
		bindLogObjectActions: bindLogObjectActions
	};
})();

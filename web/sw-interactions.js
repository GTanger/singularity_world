// talk/trade 輸入列、log pending 訊息、log 物件點擊派發；自包含，依賴 window.SwLog / window.gameSend / window.gameState / window.gameFormatNarrative。v0.20.44
(function () {
	function appendLogPendingTalk() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-talk-pending';
		div.setAttribute('data-pending', 'talk');
		div.textContent = '對話中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTalk() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-talk-pending');
		for (var i = 0; i < pending.length; i++) pending[i].remove();
	}

	function appendLogPendingTrade() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system log-trade-pending';
		div.setAttribute('data-pending', 'trade');
		div.textContent = '交易中…';
		logEl.appendChild(div);
	}

	function removeLogPendingTrade() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var pending = logEl.querySelectorAll('.log-trade-pending');
		for (var j = 0; j < pending.length; j++) pending[j].remove();
	}

	function appendTalkInputRow(entityId, targetName) {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-talk-input';
		var label = document.createElement('span');
		label.className = 'log-talk-label';
		label.textContent = '對 ' + (targetName || entityId) + ' 說：';
		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-talk-input-field';
		input.placeholder = '輸入要說的話，Enter 送出';
		input.setAttribute('maxlength', '200');
		input.setAttribute('aria-label', '對 ' + (targetName || entityId) + ' 說');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-talk-send';
		btn.textContent = '送出';
		wrap.appendChild(label);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		logEl.appendChild(wrap);
		input.focus();
		function sendTalk() {
			var text = (input.value || '').trim();
			var sayText = text || '（搭話）';
			wrap.remove();
			var playerLine = '你對【' + (targetName || entityId) + '】說：「' + sayText + '」';
			var fmt = window.gameFormatNarrative || function (t) { return t; };
			var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
			appendNarr(fmt(playerLine), 'Talk');
			if (window.gameSend) {
				window.gameSend({ type: 'do_action', entity_id: entityId, action: 'Talk', player_input: sayText });
				appendLogPendingTalk();
			}
		}
		btn.addEventListener('click', sendTalk);
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter') {
				e.preventDefault();
				sendTalk();
			}
		});
	}

	function appendTradeInputRow(entityId, targetName) {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		var wrap = document.createElement('div');
		wrap.className = 'log-entry log-trade-input';
		var label = document.createElement('span');
		label.className = 'log-trade-label';
		label.textContent = '與 ' + (targetName || entityId) + ' 交易：';
		var input = document.createElement('input');
		input.type = 'text';
		input.className = 'log-trade-input-field';
		input.placeholder = '輸入出價（鎂，整數）或「拒絕」；若尚未報價可留空再送出以開價';
		input.setAttribute('maxlength', '32');
		input.setAttribute('aria-label', '與 ' + (targetName || entityId) + ' 交易出價');
		var btn = document.createElement('button');
		btn.type = 'button';
		btn.className = 'log-trade-send';
		btn.textContent = '送出';
		wrap.appendChild(label);
		wrap.appendChild(input);
		wrap.appendChild(btn);
		logEl.appendChild(wrap);
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
		btn.addEventListener('click', sendTrade);
		input.addEventListener('keydown', function (e) {
			if (e.key === 'Enter') {
				e.preventDefault();
				sendTrade();
			}
		});
	}

	function bindLogObjectActions() {
		var logEl = document.getElementById('log');
		if (!logEl) return;
		logEl.addEventListener('click', function (ev) {
			var btn = ev.target.closest && ev.target.closest('.log-object-action');
			if (!btn) return;
			ev.preventDefault();
			var id = btn.getAttribute('data-entity-id');
			var action = btn.getAttribute('data-action');
			var targetType = btn.getAttribute('data-target-type') || '';
			var targetName = btn.getAttribute('data-target-name') || '';
			if (!id || !action || !window.gameSend) return;
			if (targetType === 'entity') {
				var st = window.gameState ? window.gameState() : {};
				var entities = st.entities || [];
				var stillHere = false;
				for (var i = 0; i < entities.length; i++) {
					if (((entities[i].id || '') + '') === (id + '')) {
						stillHere = true;
						break;
					}
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
				appendTradeInputRow(id, targetName);
				return;
			}
			window.gameSend({ type: 'do_action', entity_id: id, action: action });
		});
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

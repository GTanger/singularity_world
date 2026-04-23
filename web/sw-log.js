// log 捲動與追加：自包含，零 closure 依賴，純 DOM 操作。v0.20.42
(function () {
	var logStickToBottom = true;
	var logScrollBound = false;

	function ensureLogScrollBehavior(el) {
		if (!el || logScrollBound) return;
		logScrollBound = true;
		el.addEventListener('scroll', function () {
			var delta = el.scrollHeight - el.clientHeight - el.scrollTop;
			logStickToBottom = delta <= 32;
		}, { passive: true });
	}

	function appendAndMaybeAutoScroll(el, row) {
		if (!el || !row) return;
		ensureLogScrollBehavior(el);
		el.appendChild(row);
		var LOG_CAP = 50;
		while (el.children.length > LOG_CAP) {
			el.removeChild(el.firstChild);
		}
		if (logStickToBottom) el.scrollTop = el.scrollHeight;
	}

	function appendLog(text) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-system';
		div.textContent = text;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendNarrative(html, actionType) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-narrative';
		if (actionType) div.classList.add('log-' + actionType.toLowerCase());
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendObjectActionsLine(html) {
		var el = document.getElementById('log');
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'log-entry log-object-actions';
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	window.SwLog = {
		ensureLogScrollBehavior: ensureLogScrollBehavior,
		appendAndMaybeAutoScroll: appendAndMaybeAutoScroll,
		appendLog: appendLog,
		appendNarrative: appendNarrative,
		appendObjectActionsLine: appendObjectActionsLine,
	};
})();

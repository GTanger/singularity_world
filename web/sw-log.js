// log 捲動與追加：v5 重構，寫入 #sw-log（.sw-log-stream）。v0.21.1
// buffer 上限 500 條，貼底自動捲動。
(function () {
	var logStickToBottom = true;
	var logScrollBound = false;
	var LOG_CAP = 500;

	// v5 首選 #sw-log；#log 為相容橋（隱藏，不渲染）
	function getLogEl() {
		return document.getElementById('sw-log') || document.getElementById('log');
	}

	function getScrollEl(logEl) {
		if (!logEl) return null;
		return (logEl.closest && logEl.closest('.sw-desc-panel')) || logEl;
	}

	// scroll 監聽（貼底感知）
	function ensureLogScrollBehavior(el) {
		if (!el || logScrollBound) return;
		logScrollBound = true;
		var scrollEl = getScrollEl(el);
		if (!scrollEl) return;
		scrollEl.addEventListener('scroll', function () {
			var delta = scrollEl.scrollHeight - scrollEl.clientHeight - scrollEl.scrollTop;
			logStickToBottom = delta <= 48;
		}, { passive: true });
	}

	function appendAndMaybeAutoScroll(el, row) {
		if (!el || !row) return;
		ensureLogScrollBehavior(el);
		el.appendChild(row);
		while (el.children.length > LOG_CAP) {
			el.removeChild(el.firstChild);
		}
		if (logStickToBottom) {
			var scrollEl = getScrollEl(el);
			if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
		}
	}

	function appendLog(text) {
		var el = getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-system';
		div.textContent = text;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendNarrative(html, actionType) {
		var el = getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-narrative';
		if (actionType) div.classList.add('log-' + String(actionType).toLowerCase());
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendObjectActionsLine(html) {
		var el = getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-object-actions';
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

	// 相容：window.appendLog（main.js 仍直接呼叫）
	window.appendLog = appendLog;
	console.log('[sw-log] v0.21.1 loaded');
})();

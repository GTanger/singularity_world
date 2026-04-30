// log 捲動與追加：v5 重構，寫入 #sw-log（.sw-log-stream）。v0.21.0
// buffer 上限 500 條，貼底自動捲動。
(function () {
	var logStickToBottom = true;
	var logScrollBound = false;
	var LOG_CAP = 500;

	// v5 首選 #sw-log；#log 為相容橋（隱藏，不渲染）
	function getLogEl() {
		return document.getElementById('sw-log') || document.getElementById('log');
	}

	// scroll 監聽（貼底感知）
	// v5.75：log 拆出 desc-panel 成獨立 grid 區，#sw-log 自身就是 scroll 容器
	function ensureLogScrollBehavior(el) {
		if (!el || logScrollBound) return;
		logScrollBound = true;
		el.addEventListener('scroll', function () {
			var delta = el.scrollHeight - el.clientHeight - el.scrollTop;
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
			el.scrollTop = el.scrollHeight;
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

	// 相容：window.appendLog（被 game-ui.js 和 main.js 舊代碼直接呼叫）
	window.appendLog = appendLog;
	console.log('[sw-log] v0.21.0 loaded');
})();

// 當地近期結果：只保留眼前仍有用的少量回饋，不把主畫面變成終端歷史。v0.26.0
(function () {
	var logStickToBottom = true;
	var logScrollBound = false;
	var LOG_CAP = 40;

	// v5 首選 #sw-log；#log 為相容橋（隱藏，不渲染）
	function getLogEl() {
		var backdrop = document.getElementById('sw-object-popover-backdrop');
		var popLog = document.getElementById('sw-popover-log');
		if (backdrop && !backdrop.hidden && backdrop.style.display !== 'none' && popLog) {
			return popLog;
		}
		return document.getElementById('sw-log') || document.getElementById('log');
	}

	function getScrollEl(logEl) {
		if (!logEl) return null;
		// 若 #sw-log 自身具備獨立捲動（overflow-y: auto/scroll），直接返回 logEl
		var style = (typeof window !== 'undefined' && window.getComputedStyle) ? window.getComputedStyle(logEl) : null;
		if (style && (style.overflowY === 'auto' || style.overflowY === 'scroll')) {
			return logEl;
		}
		if (logEl.id === 'sw-log' || (logEl.classList && logEl.classList.contains('sw-log-stream'))) {
			return logEl;
		}
		return (logEl.closest && (logEl.closest('#sw-center-log') || logEl.closest('.sw-center-log') || logEl.closest('.sw-desc-panel'))) || logEl;
	}

	// scroll 監聽（貼底感知）
	function ensureLogScrollBehavior(el) {
		if (!el || logScrollBound) return;
		logScrollBound = true;
		var scrollEl = getScrollEl(el);
		if (!scrollEl) return;
		scrollEl.addEventListener('scroll', function () {
			var delta = scrollEl.scrollHeight - scrollEl.clientHeight - scrollEl.scrollTop;
			logStickToBottom = delta <= 64;
		}, { passive: true });
		if (typeof window !== 'undefined' && window.addEventListener) {
			window.addEventListener('resize', function () {
				if (logStickToBottom && scrollEl) {
					scrollEl.scrollTop = scrollEl.scrollHeight;
				}
			});
		}
	}
	function appendAndMaybeAutoScroll(el, row) {
		if (!el || !row) return;
		ensureLogScrollBehavior(el);
		var isPopover = (el.id === 'sw-popover-log');
		var isLookRow = !!(row.classList && row.classList.contains('log-look'));
		var activeInputWrap = el.querySelector && (el.querySelector('.log-talk-input') || el.querySelector('.log-trade-input'));
		var isInputRow = row.classList && (row.classList.contains('log-talk-input') || row.classList.contains('log-trade-input'));
		if (isPopover && isLookRow && el.querySelectorAll) {
			var existingLooks = el.querySelectorAll('.log-look');
			if (existingLooks.length > 0) {
				el.replaceChild(row, existingLooks[0]);
				for (var li = 1; li < existingLooks.length; li++) {
					if (existingLooks[li].parentNode === el) {
						el.removeChild(existingLooks[li]);
					}
				}
			} else if (el.firstChild) {
				el.insertBefore(row, el.firstChild);
			} else {
				el.appendChild(row);
			}
		} else if (activeInputWrap && !isInputRow && activeInputWrap !== row) {
			el.insertBefore(row, activeInputWrap);
		} else {
			el.appendChild(row);
		}
		while (el.children.length > LOG_CAP) {
			var candidate = el.firstElementChild || el.firstChild;
			while (candidate && candidate.classList && (candidate.classList.contains('log-talk-input') || candidate.classList.contains('log-trade-input') || candidate.classList.contains('log-look'))) {
				candidate = candidate.nextElementSibling;
			}
			if (candidate) {
				el.removeChild(candidate);
			} else {
				break;
			}
		}
		if (isPopover) {
			var activeTarget = window.__currentPopoverTarget;
			if (activeTarget && activeTarget.id) {
				var G = window.SwGrid;
				var currentRoomId = (G && G.mapData && G.mapData.currentRoomId) || '';
				var cacheKey = window.__currentPopoverCacheKey || (currentRoomId + '|' + activeTarget.id);
				if (!window.__popoverLogCache) window.__popoverLogCache = {};
				if (!window.__popoverLogCache[cacheKey]) window.__popoverLogCache[cacheKey] = [];
				if (isLookRow) {
					var replacedLook = false;
					var nextCache = [];
					for (var ci = 0; ci < window.__popoverLogCache[cacheKey].length; ci++) {
						var cEntry = window.__popoverLogCache[cacheKey][ci];
						if (cEntry && cEntry.classList && cEntry.classList.contains('log-look')) {
							if (!replacedLook) {
								nextCache.push(row.cloneNode(true));
								replacedLook = true;
							}
						} else {
							nextCache.push(cEntry);
						}
					}
					if (!replacedLook) {
						nextCache.unshift(row.cloneNode(true));
					}
					window.__popoverLogCache[cacheKey] = nextCache;
				} else {
					window.__popoverLogCache[cacheKey].push(row.cloneNode(true));
				}
				if (window.__popoverLogCache[cacheKey].length > LOG_CAP) {
					window.__popoverLogCache[cacheKey].shift();
				}
			}
		}
		var scrollEl = getScrollEl(el);
		function snapLogScroll(targetEl) {
			if (!targetEl) return;
			targetEl.style.paddingBottom = '10px';
			var children = targetEl.children;
			if (!children || children.length === 0) return;
			var ch = targetEl.clientHeight;
			if (ch <= 40 || targetEl.scrollHeight <= ch) {
				targetEl.scrollTop = 0;
				return;
			}
			var padTop = 10;
			var maxVisibleHeight = ch - 20;
			var lastChild = children[children.length - 1];
			var bottomEdge = lastChild.offsetTop + lastChild.offsetHeight;
			var firstFitIdx = children.length - 1;
			for (var i = children.length - 1; i >= 0; i--) {
				var spanHeight = bottomEdge - children[i].offsetTop;
				if (spanHeight <= maxVisibleHeight) {
					firstFitIdx = i;
				} else {
					break;
				}
			}
			var targetEntry = children[firstFitIdx];
			var neededSpan = bottomEdge - targetEntry.offsetTop;
			if (neededSpan <= maxVisibleHeight) {
				var slack = Math.max(0, maxVisibleHeight - neededSpan);
				var baseTop = children[0].offsetTop;
				targetEl.style.paddingBottom = (10 + slack) + 'px';
				targetEl.scrollTop = Math.max(0, targetEntry.offsetTop - baseTop);
			} else {
				targetEl.scrollTop = targetEl.scrollHeight;
			}
		}
		snapLogScroll(scrollEl);
		if (el && el !== scrollEl) snapLogScroll(el);
		if (typeof window !== 'undefined' && window.requestAnimationFrame) {
			window.requestAnimationFrame(function () {
				snapLogScroll(scrollEl);
				if (el && el !== scrollEl) snapLogScroll(el);
			});
		}
	}
	var playerModalFeedbackTimer = null;
	function showPlayerModalFeedback(htmlOrText) {
		if (!htmlOrText) return;
		var overlay = document.getElementById('player-modal-overlay');
		if (!overlay || overlay.hidden || overlay.style.display === 'none') return;
		var modal = document.getElementById('player-modal');
		if (!modal) return;

		var fb = document.getElementById('player-modal-feedback');
		if (!fb) {
			fb = document.createElement('div');
			fb.id = 'player-modal-feedback';
			fb.className = 'player-modal-feedback';
			var tabs = modal.querySelector('.player-modal-tabs');
			if (tabs && tabs.nextSibling) {
				modal.insertBefore(fb, tabs.nextSibling);
			} else if (tabs) {
				modal.appendChild(fb);
			} else {
				var body = modal.querySelector('.player-modal-body');
				if (body) {
					modal.insertBefore(fb, body);
				} else {
					modal.appendChild(fb);
				}
			}
		}

		if (typeof htmlOrText === 'string' && (htmlOrText.indexOf('<') !== -1 || htmlOrText.indexOf('&') !== -1)) {
			fb.innerHTML = htmlOrText;
		} else {
			fb.textContent = htmlOrText;
		}
		fb.hidden = false;
		fb.style.display = '';
		fb.style.opacity = '1';

		clearTimeout(playerModalFeedbackTimer);
		playerModalFeedbackTimer = setTimeout(function () {
			if (fb) {
				fb.style.opacity = '0';
				setTimeout(function () {
					if (fb && fb.style.opacity === '0') {
						fb.hidden = true;
						fb.style.display = 'none';
					}
				}, 400);
			}
		}, 8000);
	}

	function appendLog(text) {
		showPlayerModalFeedback(text);
		var el = getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-system';
		div.textContent = text;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendNarrative(html, actionType) {
		showPlayerModalFeedback(html);
		var el = (actionType === 'ambient')
			? (document.getElementById('sw-log') || document.getElementById('log'))
			: getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-narrative';
		if (actionType) div.classList.add('log-' + String(actionType).toLowerCase());
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}
	function appendObjectActionsLine(html) {
		var el = getLogEl();
		if (!el || el.id === 'sw-popover-log') return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-object-actions';
		div.innerHTML = html;
		appendAndMaybeAutoScroll(el, div);
	}

	function clearScene() {
		var el = getLogEl();
		if (el) {
			el.replaceChildren();
			el.scrollTop = 0;
		}
		var swLogEl = document.getElementById('sw-log');
		if (swLogEl && swLogEl !== el) {
			swLogEl.replaceChildren();
			swLogEl.scrollTop = 0;
		}
		var centerEl = document.getElementById('sw-center-log');
		if (centerEl) centerEl.scrollTop = 0;
		logStickToBottom = true;
	}

	window.SwLog = {
		ensureLogScrollBehavior: ensureLogScrollBehavior,
		appendAndMaybeAutoScroll: appendAndMaybeAutoScroll,
		appendLog: appendLog,
		appendNarrative: appendNarrative,
		appendObjectActionsLine: appendObjectActionsLine,
		clearScene: clearScene,
	};

	// 相容：window.appendLog（main.js 仍直接呼叫）
	window.appendLog = appendLog;
	console.log('[sw-log] v0.37.0 loaded');
})();

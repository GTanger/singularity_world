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
		} else {
			if (isPopover && !isLookRow && !isInputRow && el.querySelectorAll) {
				var newText = (row.textContent || '').trim();
				if (newText) {
					var existingRows = el.querySelectorAll('.log-entry:not(.log-look):not(.log-talk-input):not(.log-trade-input)');
					for (var ri = 0; ri < existingRows.length; ri++) {
						if ((existingRows[ri].textContent || '').trim() === newText) {
							if (existingRows[ri].parentNode === el) {
								el.removeChild(existingRows[ri]);
							}
						}
					}
				}
			}
			if (activeInputWrap && !isInputRow && activeInputWrap !== row) {
				el.insertBefore(row, activeInputWrap);
			} else {
				el.appendChild(row);
			}
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
					var cacheNewText = (row.textContent || '').trim();
					if (cacheNewText && !isInputRow) {
						window.__popoverLogCache[cacheKey] = window.__popoverLogCache[cacheKey].filter(function (entry) {
							if (entry.classList && (entry.classList.contains('log-look') || entry.classList.contains('log-talk-input') || entry.classList.contains('log-trade-input'))) {
								return true;
							}
							return (entry.textContent || '').trim() !== cacheNewText;
						});
					}
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
			var baseTop = children[0].offsetTop;
			if (neededSpan <= maxVisibleHeight) {
				var slack = Math.max(0, maxVisibleHeight - neededSpan);
				targetEl.style.paddingBottom = (10 + slack) + 'px';
				targetEl.scrollTop = Math.max(0, targetEntry.offsetTop - baseTop);
			} else {
				targetEl.scrollTop = Math.max(0, targetEntry.offsetTop - baseTop);
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
	// ── 近事錄與浮動通知層 (SwNotify) ──────────────────────────────────
	var _recapEntries = [];
	var _recapSeq = 0;
	var RECAP_CAP = 40;
	var RECAP_STORAGE_KEY = 'sw_recap_entries_v2';
	function isFilteredRecapEntry(plainText, actionType) {
		var act = String(actionType || '').toLowerCase();
		if (act === 'look') return true;
		var s = String(plainText || '').trim();
		if (!s) return true;
		if (/^錯誤：|^往「.+」移動$|^連線|^重新連線|^心跳超時/.test(s)) return true;
		if (/^你仔細打量了/.test(s)) return true;
		if (/^你對【[^】]+】說：「(寒暄|寒暄問候|打聽|打聽近況|請教差事)」$/.test(s)) return true;
		if (/^你上前向【[^】]+】頷首致意，隨口寒暄了幾句。$/.test(s)) return true;
		if (/^你向【[^】]+】打聽起近日坊間的動靜。$/.test(s)) return true;
		if (/^你向【[^】]+】探詢手頭可有能搭把手的差事。$/.test(s)) return true;
		if (/（體質\s*\+|（內息\s*\+|【洗藥】|【洗礦】|【搓繩】/.test(s)) return true;
		return false;
	}
	try {
		localStorage.removeItem('sw_recap_entries_v1');
		var savedRecap = localStorage.getItem(RECAP_STORAGE_KEY);
		if (savedRecap) {
			var parsedRecap = JSON.parse(savedRecap);
			if (Array.isArray(parsedRecap)) {
				_recapEntries = parsedRecap.filter(function (entry) {
					return entry && !isFilteredRecapEntry(entry.plain || getPlainText(entry.html), entry.actionType);
				}).slice(0, RECAP_CAP);
				_recapSeq = _recapEntries.length;
			}
		}
	} catch (_e) {}
	function saveRecapEntries() {
		try {
			localStorage.setItem(RECAP_STORAGE_KEY, JSON.stringify(_recapEntries.slice(0, RECAP_CAP)));
		} catch (_e) {}
	}
	var _activeRecapFilter = 'all';
	var _calloutHideTimer = null;
	var _toastTimer = null;
	var INTENT_BADGE_MAP = {
		'talk': '故人招呼',
		'hunt': '合社招攬',
		'craft': '百工招呼',
		'errand': '差事探詢',
		'trade': '市集招呼',
		'combat': '警覺迎敵'
	};

	function getPlainText(htmlOrText) {
		if (!htmlOrText) return '';
		var s = String(htmlOrText);
		return s
			.replace(/<br\s*\/?>/gi, ' ')
			.replace(/<[^>]*>/g, '')
			.replace(/&nbsp;/g, ' ')
			.replace(/&lt;/g, '<')
			.replace(/&gt;/g, '>')
			.replace(/&quot;/g, '"')
			.replace(/&#39;/g, "'")
			.replace(/&amp;/g, '&')
			.trim();
	}

	function closeBlockingOverlaysForReply() {
		var modalOverlay = document.getElementById('player-modal-overlay');
		if (modalOverlay && !modalOverlay.hidden && modalOverlay.style.display !== 'none') {
			var modalCloseBtn = document.getElementById('player-modal-close');
			if (modalCloseBtn) {
				modalCloseBtn.click();
			} else {
				modalOverlay.setAttribute('hidden', '');
				modalOverlay.setAttribute('aria-hidden', 'true');
				document.body.style.overflow = '';
			}
		}
		var wmOverlay = document.getElementById('sw-world-map-overlay');
		if (wmOverlay && !wmOverlay.hidden && wmOverlay.style.display !== 'none') {
			var wmCloseBtn = document.getElementById('btn-wm-close');
			if (wmCloseBtn) {
				wmCloseBtn.click();
			} else {
				wmOverlay.setAttribute('hidden', '');
				wmOverlay.setAttribute('aria-hidden', 'true');
			}
		}
	}

	function escapeHtml(s) {
		if (!s) return '';
		var div = document.createElement('div');
		div.textContent = s;
		return div.innerHTML;
	}

	function formatRecapTime(ts) {
		if (!ts) return '';
		var d = new Date(ts);
		var h = String(d.getHours());
		var m = String(d.getMinutes());
		if (h.length < 2) h = '0' + h;
		if (m.length < 2) m = '0' + m;
		return h + ':' + m;
	}

	function recordRecap(entry) {
		var drawer = document.getElementById('sw-recap-drawer');
		var isDrawerOpen = !!(drawer && !drawer.hidden && drawer.style.display !== 'none');
		var plainText = (entry.plain || getPlainText(entry.html) || '').trim();
		if (isFilteredRecapEntry(plainText, entry.actionType)) {
			return null;
		}
		for (var i = 0; i < Math.min(_recapEntries.length, 6); i++) {
			if (_recapEntries[i].plain === plainText && _recapEntries[i].speaker === (entry.speaker || '')) {
				_recapEntries[i].ts = entry.ts || Date.now();
				if (isDrawerOpen) {
					renderRecapList();
				}
				return _recapEntries[i];
			}
		}
		var item = {
			id: 'recap_' + (++_recapSeq),
			ts: entry.ts || Date.now(),
			category: entry.category || 'feedback',
			speaker: entry.speaker || '',
			npcId: entry.npcId || '',
			intent: entry.intent || '',
			html: entry.html || '',
			plain: plainText,
			unread: isDrawerOpen ? false : !!entry.unread
		};
		_recapEntries.unshift(item);
		if (_recapEntries.length > RECAP_CAP) {
			_recapEntries.pop();
		}
		saveRecapEntries();
		if (item.unread) {
			var unreadBadge = document.getElementById('sw-recap-unread');
			if (unreadBadge) unreadBadge.hidden = false;
		}
		if (isDrawerOpen) {
			renderRecapList();
		}
		return item;
	}

	function toast(htmlOrText, options) {
		if (!htmlOrText) return;
		var layer = document.getElementById('sw-toast-layer');
		if (!layer) return;

		var modalOv = document.getElementById('player-modal-overlay');
		var wmOv = document.getElementById('sw-world-map-overlay');
		var isOverlayOpen = !!((modalOv && !modalOv.hidden && modalOv.style.display !== 'none') || (wmOv && !wmOv.hidden && wmOv.style.display !== 'none'));
		var kind = (options && options.kind) ? String(options.kind).toLowerCase() : 'system';
		var isHtml = !!(options && options.isHtml);
		if (!isOverlayOpen && typeof SwPushBreath === 'function') {
			if (kind === 'ambient') return;
			SwPushBreath(htmlOrText, { byPlayer: true });
			return;
		}
		layer.classList.toggle('is-modal-above', isOverlayOpen);
		var plain = isHtml ? getPlainText(htmlOrText) : String(htmlOrText);
		var isLong = plain.length > 64;

		var toastEl = document.createElement('div');
		toastEl.className = 'sw-ink-toast' + (kind ? (' is-' + kind) : '');

		var bodySpan = document.createElement('span');
		bodySpan.className = 'sw-toast-content';

		if (isLong) {
			bodySpan.textContent = plain.slice(0, 60) + '… ';
			var recapBtn = document.createElement('button');
			recapBtn.type = 'button';
			recapBtn.className = 'sw-toast-recap-link';
			recapBtn.textContent = '詳見近事';
			recapBtn.addEventListener('click', function (e) {
				e.stopPropagation();
				dismissToast(toastEl);
				openRecap('all');
			});
			bodySpan.appendChild(recapBtn);
		} else {
			if (isHtml) {
				bodySpan.innerHTML = htmlOrText;
			} else {
				bodySpan.textContent = plain;
			}
		}
		toastEl.appendChild(bodySpan);

		var closeBtn = document.createElement('button');
		closeBtn.type = 'button';
		closeBtn.className = 'sw-toast-close';
		closeBtn.setAttribute('aria-label', '關閉通知');
		closeBtn.textContent = '×';
		closeBtn.addEventListener('click', function (e) {
			e.stopPropagation();
			dismissToast(toastEl);
		});
		toastEl.appendChild(closeBtn);

		if (_toastTimer) {
			clearTimeout(_toastTimer);
			_toastTimer = null;
		}
		while (layer.firstElementChild) {
			layer.removeChild(layer.firstElementChild);
		}
		layer.appendChild(toastEl);

		function dismissToast(el) {
			if (!el || el._dismissed) return;
			el._dismissed = true;
			el.classList.add('is-dismissing');
			setTimeout(function () {
				if (el.parentNode) el.parentNode.removeChild(el);
			}, 260);
		}

		var dur = (options && options.duration) || (isLong ? 4000 : 2400);
		_toastTimer = setTimeout(function () {
			_toastTimer = null;
			dismissToast(toastEl);
		}, dur);
	}

	function handleNpcAddress(msg) {
		if (!msg) return;
		var npcId = msg.npc_id || '';
		var npcName = msg.npc_name || '人物';
		var text = msg.text || '';
		var intent = msg.intent || 'talk';

		var cleanText = String(text || '').trim();
		if (cleanText.charAt(0) === '「' && cleanText.charAt(cleanText.length - 1) === '」') {
			cleanText = cleanText.slice(1, -1).trim();
		}
		if (npcName && cleanText.indexOf(npcName) === 0) {
			cleanText = cleanText.slice(npcName.length).replace(/^[\s，、：:」」]+/, '').trim();
		}

		var formattedHtml = (window.SwNarrative && typeof window.SwNarrative.format === 'function')
			? window.SwNarrative.format(cleanText)
			: escapeHtml(cleanText).replace(/\n/g, '<br>');
		recordRecap({
			category: 'address',
			speaker: npcName,
			npcId: npcId,
			intent: intent,
			html: formattedHtml,
			plain: getPlainText(formattedHtml),
			unread: true
		});

		if (window.SwObjectList && typeof window.SwObjectList.recordNpcAddress === 'function') {
			window.SwObjectList.recordNpcAddress(msg);
		}

		var modalOverlay = document.getElementById('player-modal-overlay');
		var isModalOpen = !!(modalOverlay && !modalOverlay.hidden && modalOverlay.style.display !== 'none');
		var wmOverlay = document.getElementById('sw-world-map-overlay');
		var isWmOpen = !!(wmOverlay && !wmOverlay.hidden && wmOverlay.style.display !== 'none');

		var shouldShowCallout = isModalOpen || isWmOpen || (intent === 'combat');
		if (shouldShowCallout) {
			showTopCallout(msg, formattedHtml);
		} else {
			hideTopCallout();
		}
		var popoverBackdrop = document.getElementById('sw-object-popover-backdrop');
		var isPopoverOpen = !!(popoverBackdrop && !popoverBackdrop.hidden && popoverBackdrop.style.display !== 'none');
		if (!isPopoverOpen && typeof SwPushBreath === 'function') {
			var breathSpeaker = (npcName.indexOf('一名') === 0 || npcName.indexOf('【') === 0) ? npcName : ('【' + npcName + '】');
			var breathText = (cleanText.indexOf(npcName) !== 0 && cleanText.indexOf('【') !== 0 && cleanText.indexOf('一名') !== 0)
				? (breathSpeaker + cleanText)
				: cleanText;
			var breathHtml = (window.SwNarrative && typeof window.SwNarrative.format === 'function')
				? window.SwNarrative.format(breathText)
				: escapeHtml(breathText).replace(/\n/g, '<br>');
			SwPushBreath(breathHtml, { byPlayer: false });
		}
	}

	function showTopCallout(msg, formattedHtml) {
		var calloutEl = document.getElementById('sw-top-callout');
		if (!calloutEl) return;
		var badgeEl = document.getElementById('sw-callout-badge');
		var speakerEl = document.getElementById('sw-callout-speaker');
		var textEl = document.getElementById('sw-callout-text');
		var replyBtn = document.getElementById('sw-callout-reply');
		var laterBtn = document.getElementById('sw-callout-later');

		var intent = msg.intent || 'talk';
		var badgeText = INTENT_BADGE_MAP[intent] || '故人招呼';
		var htmlBody = formattedHtml;
		if (htmlBody === undefined || htmlBody === null) {
			var rawText = String(msg.text || '').trim();
			if (rawText.charAt(0) === '「' && rawText.charAt(rawText.length - 1) === '」') {
				rawText = rawText.slice(1, -1).trim();
			}
			var spkName = msg.npc_name || '';
			if (spkName && rawText.indexOf(spkName) === 0) {
				rawText = rawText.slice(spkName.length).replace(/^[\s，、：:」」]+/, '').trim();
			}
			htmlBody = (window.SwNarrative && typeof window.SwNarrative.format === 'function')
				? window.SwNarrative.format(rawText)
				: escapeHtml(rawText).replace(/\n/g, '<br>');
		}
		if (badgeEl) badgeEl.textContent = badgeText;
		if (speakerEl) speakerEl.textContent = msg.npc_name || '人物';
		if (textEl) textEl.innerHTML = htmlBody;
		clearTimeout(_calloutHideTimer);

		if (replyBtn) {
			replyBtn.onclick = function () {
				hideTopCallout();
				closeBlockingOverlaysForReply();
				if (window.SwObjectList && typeof window.SwObjectList.openPopoverByEntityId === 'function') {
					window.SwObjectList.openPopoverByEntityId(msg.npc_id, msg.npc_name);
				}
			};
		}
		if (laterBtn) {
			laterBtn.onclick = function () {
				hideTopCallout();
			};
		}

		calloutEl.removeAttribute('hidden');
		_calloutHideTimer = setTimeout(function () {
			hideTopCallout();
		}, 28000);
	}

	function hideTopCallout() {
		clearTimeout(_calloutHideTimer);
		var calloutEl = document.getElementById('sw-top-callout');
		if (calloutEl) {
			calloutEl.setAttribute('hidden', '');
		}
	}

	function renderRecapList() {
		var listEl = document.getElementById('sw-recap-list');
		if (!listEl) return;

		var filtered = _recapEntries.filter(function (entry) {
			var entryText = entry.plain || entry.html || '';
			if (isFilteredRecapEntry(entryText, entry.actionType)) return false;
			if (_activeRecapFilter === 'all') return true;
			if (_activeRecapFilter === 'address') return entry.category === 'address';
			if (_activeRecapFilter === 'ambient') return entry.category === 'ambient';
			return true;
		});

		if (filtered.length === 0) {
			listEl.innerHTML = '<div class="sw-recap-empty">暫無近事紀錄。</div>';
			return;
		}

		listEl.innerHTML = '';
		for (var i = 0; i < filtered.length; i++) {
			var e = filtered[i];
			var itemDiv = document.createElement('div');
			itemDiv.className = 'sw-recap-item' + (e.unread ? ' is-unread' : '');

			var headerDiv = document.createElement('div');
			headerDiv.className = 'sw-recap-item-header';

			var badgeSpan = document.createElement('span');
			badgeSpan.className = 'sw-recap-item-badge';
			if (e.category === 'address') {
				var intentLabel = INTENT_BADGE_MAP[e.intent] || '搭話';
				badgeSpan.textContent = (e.speaker ? (e.speaker + ' · ') : '') + intentLabel;
			} else if (e.category === 'ambient') {
				badgeSpan.textContent = '街坊動靜';
			} else {
				badgeSpan.textContent = '行旅歷練';
			}
			headerDiv.appendChild(badgeSpan);

			var timeSpan = document.createElement('span');
			timeSpan.className = 'sw-recap-item-time';
			timeSpan.textContent = formatRecapTime(e.ts);
			headerDiv.appendChild(timeSpan);
			itemDiv.appendChild(headerDiv);

			var bodyDiv = document.createElement('div');
			bodyDiv.className = 'sw-recap-item-body';
			bodyDiv.innerHTML = e.html || escapeHtml(e.plain);
			itemDiv.appendChild(bodyDiv);

			if (e.category === 'address' && e.npcId) {
				var actDiv = document.createElement('div');
				actDiv.className = 'sw-recap-item-act';
				var replyBtn = document.createElement('button');
				replyBtn.type = 'button';
				replyBtn.className = 'sw-recap-item-reply-btn';
				replyBtn.textContent = '回應';
				(function (nid, nname) {
					replyBtn.addEventListener('click', function () {
						closeRecap();
						closeBlockingOverlaysForReply();
						if (window.SwObjectList && typeof window.SwObjectList.openPopoverByEntityId === 'function') {
							window.SwObjectList.openPopoverByEntityId(nid, nname);
						}
					});
				})(e.npcId, e.speaker);
				actDiv.appendChild(replyBtn);
				itemDiv.appendChild(actDiv);
			}

			listEl.appendChild(itemDiv);
		}
	}

	function openRecap(filter) {
		var tl = document.getElementById('sw-toast-layer');
		if (tl) tl.innerHTML = '';
		if (_toastTimer) {
			clearTimeout(_toastTimer);
			_toastTimer = null;
		}
		var drawer = document.getElementById('sw-recap-drawer');
		var backdrop = document.getElementById('sw-recap-backdrop');
		if (!drawer) return;
		if (filter) {
			_activeRecapFilter = filter;
			var tabs = drawer.querySelectorAll('.sw-recap-tab');
			for (var t = 0; t < tabs.length; t++) {
				tabs[t].classList.toggle('active', tabs[t].getAttribute('data-filter') === filter);
			}
		}

		_recapEntries.forEach(function (e) { e.unread = false; });
		saveRecapEntries();
		var unreadBadge = document.getElementById('sw-recap-unread');
		if (unreadBadge) unreadBadge.hidden = true;
		drawer.removeAttribute('hidden');
		if (backdrop) backdrop.removeAttribute('hidden');

		renderRecapList();
	}

	function closeRecap() {
		var drawer = document.getElementById('sw-recap-drawer');
		var backdrop = document.getElementById('sw-recap-backdrop');
		if (drawer) drawer.setAttribute('hidden', '');
		if (backdrop) backdrop.setAttribute('hidden', '');
	}

	function toggleRecap() {
		var drawer = document.getElementById('sw-recap-drawer');
		if (drawer && !drawer.hidden && drawer.style.display !== 'none') {
			closeRecap();
		} else {
			openRecap();
		}
	}

	function initRecapListeners() {
		var toggleBtn = document.getElementById('btn-toggle-recap');
		if (toggleBtn && !toggleBtn._bound) {
			toggleBtn._bound = true;
			toggleBtn.addEventListener('click', toggleRecap);
		}
		var closeBtn = document.getElementById('sw-recap-close');
		if (closeBtn && !closeBtn._bound) {
			closeBtn._bound = true;
			closeBtn.addEventListener('click', closeRecap);
		}
		var backdrop = document.getElementById('sw-recap-backdrop');
		if (backdrop && !backdrop._bound) {
			backdrop._bound = true;
			backdrop.addEventListener('click', closeRecap);
		}
		var drawer = document.getElementById('sw-recap-drawer');
		if (drawer && !drawer._tabsBound) {
			drawer._tabsBound = true;
			var tabs = drawer.querySelectorAll('.sw-recap-tab');
			for (var i = 0; i < tabs.length; i++) {
				(function (tab) {
					tab.addEventListener('click', function () {
						for (var j = 0; j < tabs.length; j++) {
							tabs[j].classList.toggle('active', tabs[j] === tab);
						}
						_activeRecapFilter = tab.getAttribute('data-filter') || 'all';
						renderRecapList();
					});
				})(tabs[i]);
			}
		}
		if (!window._swRecapEscapeBound) {
			window._swRecapEscapeBound = true;
			window.addEventListener('keydown', function (e) {
				if (e.key === 'Escape') {
					var d = document.getElementById('sw-recap-drawer');
					if (d && !d.hidden && d.style.display !== 'none') {
						closeRecap();
					}
				}
			});
		}
	}

	if (typeof document !== 'undefined') {
		if (document.readyState === 'loading') {
			document.addEventListener('DOMContentLoaded', initRecapListeners);
		} else {
			initRecapListeners();
		}
	}

	var _breaths = [];
	var _playerFocusUntil = 0;

	function renderBreathsDom() {
		for (var i = 0; i <= 4; i++) {
			var el = document.getElementById('sw-breath-' + i);
			if (!el) continue;
			var item = _breaths[i];
			el.textContent = item ? item.text : '';
			el.title = item ? item.text : '';
			el.hidden = !(item && item.text);
			el.classList.toggle('is-player-breath', !!(item && item.byPlayer && i === 0));
		}
	}

	function SwPushBreath(htmlOrText, options) {
		if (!htmlOrText) return;
		options = options || {};
		var byPlayer = !!options.byPlayer;
		var plain = getPlainText(htmlOrText)
			.replace(/【正在交談】\s*/g, '')
			.replace(/^【[^】]*・當值協作】\s*/g, '')
			.replace(/\s*【產地溯源：[^】]*】/g, '')
			.replace(/^這是白塔半島常見的「([^」]+)」[，、]\s*/g, '$1，')
			.replace(/吐納起落[^。]*。/g, '')
			.replace(/手頭器物已歷[^。]*。/g, '')
			.replace(/隨身攜有[^。]*。/g, '')
			.replace(/身著明顯的當值工作制服，正於場內料理店務與百工差事。/g, '')
			.replace(/\s+/g, ' ')
			.trim();
		if (!plain) return;
		if (/^往「.+」移動$|^連線|^重新連線|^心跳超時/.test(plain)) return;

		var now = Date.now();
		if (_breaths.length > 0 && plain === _breaths[0].text) {
			if (byPlayer) {
				_breaths[0].byPlayer = true;
				_playerFocusUntil = now + 6500;
			}
			renderBreathsDom();
			return;
		}

		// Remove any older duplicate of plain so it doesn't appear twice
		for (var d = _breaths.length - 1; d >= 0; d--) {
			if (_breaths[d].text === plain) {
				_breaths.splice(d, 1);
			}
		}

		if (byPlayer) {
			_playerFocusUntil = now + 6500;
			_breaths.unshift({ text: plain, byPlayer: true });
		} else {
			if (now < _playerFocusUntil && _breaths.length > 0 && _breaths[0].byPlayer) {
				_breaths.splice(1, 0, { text: plain, byPlayer: false });
			} else {
				_breaths.unshift({ text: plain, byPlayer: false });
			}
		}
		if (_breaths.length > 5) {
			_breaths.length = 5;
		}
		renderBreathsDom();
	}
	window.SwPushBreath = SwPushBreath;

	function appendLog(text) {
		if (!text) return;
		initRecapListeners();

		var isBlockedMove = /無法往.+移動|地勢阻隔無法直行|^錯誤：|^往「.+」移動$|^連線|^重新連線|^心跳超時/.test(text);
		if (!isBlockedMove) {
			recordRecap({
				category: 'feedback',
				speaker: '',
				html: escapeHtml(text),
				plain: text,
				unread: false
			});
		}

		var backdrop = document.getElementById('sw-object-popover-backdrop');
		var isPopoverOpen = !!(backdrop && !backdrop.hidden && backdrop.style.display !== 'none');
		var modalOverlay = document.getElementById('player-modal-overlay');
		var isModalOpen = !!(modalOverlay && !modalOverlay.hidden && modalOverlay.style.display !== 'none');
		var wmOverlay = document.getElementById('sw-world-map-overlay');
		var isWmOpen = !!(wmOverlay && !wmOverlay.hidden && wmOverlay.style.display !== 'none');

		SwPushBreath(text, { byPlayer: true });
		if (isModalOpen || isWmOpen) {
			toast(text, { kind: isBlockedMove ? 'blocked' : 'system', isHtml: false });
		}

		var el = getLogEl();
		if (!el) return;
		var div = document.createElement('div');
		div.className = 'sw-log-entry log-entry log-system';
		div.textContent = text;
		appendAndMaybeAutoScroll(el, div);
	}

	function appendNarrative(html, actionType) {
		if (!html) return;
		initRecapListeners();
		var plain = getPlainText(html);
		var isBlockedMove = /無法往.+移動|地勢阻隔無法直行|^錯誤：|^往「.+」移動$|^連線|^重新連線|^心跳超時/.test(plain);
		var category = (actionType === 'ambient') ? 'ambient' : 'feedback';
		if (!isBlockedMove) {
			recordRecap({
				category: category,
				speaker: '',
				actionType: actionType || '',
				html: html,
				plain: plain,
				unread: false
			});
		}

		if (actionType !== 'ambient' && typeof SwPushBreath === 'function') {
			SwPushBreath(html, { byPlayer: true });
		}
		if (actionType !== 'ambient') {
			var modalOverlay = document.getElementById('player-modal-overlay');
			var isModalOpen = !!(modalOverlay && !modalOverlay.hidden && modalOverlay.style.display !== 'none');
			var wmOverlay = document.getElementById('sw-world-map-overlay');
			var isWmOpen = !!(wmOverlay && !wmOverlay.hidden && wmOverlay.style.display !== 'none');

			if (isModalOpen || isWmOpen) {
				toast(html, { kind: isBlockedMove ? 'blocked' : actionType, isHtml: true });
			}
		}

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

	window.SwNotify = {
		toast: toast,
		handleNpcAddress: handleNpcAddress,
		openRecap: openRecap,
		closeRecap: closeRecap,
		toggleRecap: toggleRecap,
		getEntries: function () { return _recapEntries.slice(); },
		recordRecap: recordRecap,
		showTopCallout: showTopCallout,
		hideTopCallout: hideTopCallout
	};

	window.SwLog = {
		ensureLogScrollBehavior: ensureLogScrollBehavior,
		appendAndMaybeAutoScroll: appendAndMaybeAutoScroll,
		appendLog: appendLog,
		appendNarrative: appendNarrative,
		appendObjectActionsLine: appendObjectActionsLine,
		pushBreath: SwPushBreath,
		clearScene: clearScene
	};

	// 相容：window.appendLog（main.js 仍直接呼叫）
	window.appendLog = appendLog;
})();

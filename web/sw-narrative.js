// 敘事格式化與實體名字索引：indexEntityNameCache / resolveEntityIDByName / formatNarrative / formatNarrativeWithClickableObjects。v0.20.45
(function () {
	var entityNameCache = {};

	function escapeHtml(s) {
		if (!s) return '';
		var div = document.createElement('div');
		div.textContent = s;
		return div.innerHTML;
	}

	function indexNames(entities) {
		(entities || []).forEach(function (e) {
			var eid = (e.id || e.ID || '').toString();
			if (!eid) return;
			var dname = (e.display_name || '').toString();
			entityNameCache[eid] = eid;
			if (dname) entityNameCache[dname] = eid;
		});
	}

	function resolveId(name) {
		if (!name) return '';
		if (entityNameCache[name]) return entityNameCache[name];
		var state = window.gameState ? window.gameState() : null;
		if (state && state.entities && state.entities.length) {
			for (var i = 0; i < state.entities.length; i++) {
				var e = state.entities[i];
				var eid = (e.id || e.ID || '').toString();
				var dname = (e.display_name || '').toString();
				if (name === eid || (dname && name === dname)) return eid;
			}
		}
		return '';
	}

	// 單段敘事（表格儲存格不換成 <br>）
	function formatSegment(text, asTableCell) {
		if (!text) return '';
		var esc = escapeHtml(String(text))
			.replace(/【([^】]*)】/g, function (_m, rawName) {
				var name = rawName || '';
				var eid = resolveId(name);
				if (eid) {
					return '【<span class="log-object-action narr-name" role="button" tabindex="0" data-entity-id="' + escapeHtml(eid) + '" data-action="Look" data-target-type="entity" data-target-name="' + escapeHtml(name) + '">' + escapeHtml(name) + '</span>】';
				}
				return '<span class="narr-name">【' + escapeHtml(name) + '】</span>';
			})
			.replace(/「([^」]*)」/g, '<span class="narr-dialogue">「$1」</span>');
		if (!asTableCell) {
			esc = esc.replace(/\n/g, '<br>');
		}
		return esc;
	}

	function format(text) {
		if (!text) return '';
		if (!window.NarrativeMarkdown || !window.NarrativeMarkdown.splitTextAndTables) {
			return formatSegment(text, false);
		}
		var blocks = window.NarrativeMarkdown.splitTextAndTables(text);
		var parts = [];
		for (var bi = 0; bi < blocks.length; bi++) {
			var b = blocks[bi];
			if (b.type === 'table') {
				parts.push(window.NarrativeMarkdown.renderTableHtml(b.rows, function (cell) {
					return formatSegment(cell, true);
				}));
			} else {
				parts.push(formatSegment(b.text, false));
			}
		}
		return parts.join('');
	}

	// 敘事中的物件名若為當前房間物件則變為可點擊（執行 Move 或 Look）
	// 第一輪：替換 【物件名】 或 〔物件名〕；第二輪：僅在「詞界」替換純文字名（避免「奶茶」命中「琥珀珍珠奶茶」）
	function formatWithClickable(html, objects) {
		if (!html || !objects || !objects.length) return html;
		function escapeRegex(s) { return String(s).replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }
		var sorted = objects.slice().sort(function (a, b) { return (b.name || '').length - (a.name || '').length; });
		function makeSpan(o) {
			var nameEsc = escapeHtml(o.name);
			var hasMove = o.actions && o.actions.indexOf('Move') !== -1;
			var action = hasMove ? 'Move' : 'Look';
			return '<span class="log-object-action" role="button" tabindex="0" data-entity-id="' + escapeHtml(o.id) + '" data-action="' + escapeHtml(action) + '">' + nameEsc + '</span>';
		}
		sorted.forEach(function (o) {
			if (!o.name) return;
			var nameRe = escapeRegex(escapeHtml(o.name));
			html = html.replace(new RegExp('[\u3010\u3014]' + nameRe + '[\u3011\u3015]', 'g'), makeSpan(o));
		});
		// 第二輪：左側不得為漢字或「>」（避免命中複合詞；避免在已有 span 內再包一層）
		// 右側不得為漢字或「<」（避免切到 </span> 前再度替換）
		sorted.forEach(function (o) {
			if (!o.name) return;
			var nameRe = escapeRegex(escapeHtml(o.name));
			if (!nameRe) return;
			var boundaryRe = new RegExp('(?<![\\u4e00-\\u9fff>])' + nameRe + '(?![\\u4e00-\\u9fff<])', 'g');
			html = html.replace(boundaryRe, function () { return makeSpan(o); });
		});
		return html;
	}

	window.SwNarrative = {
		indexNames: indexNames,
		resolveId: resolveId,
		format: format,
		formatWithClickable: formatWithClickable,
		escapeHtml: escapeHtml
	};
	// 向後相容 alias（sw-interactions.js 用到 window.gameFormatNarrative）
	window.gameFormatNarrative = format;
})();

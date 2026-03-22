// GFM 風格 Markdown 表格：以 | 列辨識，第二列為 | --- | --- | 分隔列時視為表格區塊。
// 供敘事／描述分段後與 〔〕 可點擊邏輯組合使用。
(function (w) {
	function normLine(line) {
		return String(line).replace(/\uFF5C/g, '|').trimEnd();
	}

	function parseRow(line) {
		var t = normLine(String(line).trim());
		if (t.length < 2 || t.charAt(0) !== '|') return null;
		if (t.charAt(t.length - 1) !== '|') return null;
		var inner = t.slice(1, -1);
		var cells = inner.split('|').map(function (c) {
			return c.trim();
		});
		return cells.length ? cells : null;
	}

	function isSeparatorRow(cells) {
		if (!cells || !cells.length) return false;
		return cells.every(function (c) {
			var s = String(c).replace(/\s/g, '');
			if (!s) return false;
			return /^:?-{3,}:?$/.test(s);
		});
	}

	function peekPipeRows(startIdx, lines) {
		var rows = [];
		var j = startIdx;
		while (j < lines.length) {
			var pr = parseRow(lines[j]);
			if (!pr) break;
			rows.push(pr);
			j++;
		}
		return { end: j, rows: rows };
	}

	/**
	 * @returns {Array<{type:'text',text:string}|{type:'table',rows:string[][]}>}
	 */
	function splitTextAndTables(text) {
		var lines = String(text).split(/\r?\n/);
		var blocks = [];
		var buf = [];
		var i = 0;

		function flushBuf() {
			if (buf.length) {
				blocks.push({ type: 'text', text: buf.join('\n') });
				buf = [];
			}
		}

		while (i < lines.length) {
			var first = parseRow(lines[i]);
			if (first) {
				var peek = peekPipeRows(i, lines);
				if (peek.rows.length >= 2 && isSeparatorRow(peek.rows[1])) {
					flushBuf();
					blocks.push({ type: 'table', rows: peek.rows });
					i = peek.end;
					continue;
				}
				for (var k = i; k < peek.end; k++) {
					buf.push(lines[k]);
				}
				i = peek.end;
				continue;
			}
			buf.push(lines[i]);
			i++;
		}
		flushBuf();
		return blocks;
	}

	/**
	 * @param {string[][]} rows 含表頭列、分隔列、資料列
	 * @param {(cell: string) => string} cellHtml 已含 escape 與 〔〕 等標記
	 */
	function renderTableHtml(rows, cellHtml) {
		if (!rows || rows.length < 2) return '';
		var header = rows[0];
		var body = rows.slice(2);
		var h = '<table class="narrative-md-table"><thead><tr>';
		for (var hi = 0; hi < header.length; hi++) {
			h += '<th>' + cellHtml(String(header[hi])) + '</th>';
		}
		h += '</tr></thead><tbody>';
		for (var ri = 0; ri < body.length; ri++) {
			var r = body[ri];
			h += '<tr>';
			for (var ci = 0; ci < r.length; ci++) {
				h += '<td>' + cellHtml(String(r[ci])) + '</td>';
			}
			h += '</tr>';
		}
		h += '</tbody></table>';
		return h;
	}

	w.NarrativeMarkdown = {
		splitTextAndTables: splitTextAndTables,
		renderTableHtml: renderTableHtml,
		parseRow: parseRow,
		isSeparatorRow: isSeparatorRow
	};
})(typeof window !== 'undefined' ? window : globalThis);

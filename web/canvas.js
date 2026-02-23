// 2D 俯視地圖繪製、區塊與實體。
// 依顯示尺寸決定視窗格數，手機上每格維持 30px、角色圓 24px，畫面變大。
(function () {
	const canvas = document.getElementById('canvas');
	if (!canvas) return;
	const ctx = canvas.getContext('2d');
	const CELL_SIZE = 30;
	const TERRAIN_FONT = 30;
	const ROLE_CIRCLE = 24;
	let VIEWPORT_W = 151;
	let VIEWPORT_H = 151;

	// 地形字對應（實作依據）：map_terrain_world.md「顯示時從 chars 亂序取字與對應 colors」、每一格皆為有色地形字。
	// 表內顏色（hex）＝該地形字顯示用色；與 world/terrain_display.go terrainMetas 對齊（道/地/巷 #c4b8a8，牆石灰 #8c8c8c）。
	const TERRAIN_COLORS = {
		'牆': '#8c8c8c', '門': '#8b7355', '關': '#6b5344', '道': '#c4b8a8', '路': '#c4b8a8', '徑': '#c4b8a8', '巷': '#c4b8a8', '地': '#c4b8a8',
		'草': '#c2d6a4', '木': '#a0d080', '山': '#98af9d', '石': '#d3d3d3', '沼': '#507050',
		'川': '#60a0d0', '水': '#b0d8f0', '荒': '#e3d5ca', '火': '#faa307', '冰': '#e0f2f1',
		'田': '#9ef01a', '谷': '#3d405b', '霧': '#c8c4b8'
	};

	let lastCx = 0, lastCy = 0, lastCameraLx = 0, lastCameraLy = 0;
	let panOffsetX = 0, panOffsetY = 0;
	let dragStartX = 0, dragStartY = 0, panStartX = 0, panStartY = 0, dragging = false;
	let justDragged = false;

	function clear() {
		ctx.fillStyle = '#0d0d0d';
		ctx.fillRect(0, 0, VIEWPORT_W * CELL_SIZE, VIEWPORT_H * CELL_SIZE);
	}

	var CELL_BG_BEIGE = '#f5f0e6'; // 地圖底色（格底）
	function drawCell(px, py, char, bgColor) {
		var color = bgColor || (char && TERRAIN_COLORS[char]) || '#444';
		var isMapBase = (char === '道' || char === '路' || char === '徑' || char === '地' || char === '巷'); // 與地圖底色相同，字同色隱入
		if (isMapBase) {
			ctx.fillStyle = CELL_BG_BEIGE;
			ctx.fillRect(px, py, CELL_SIZE, CELL_SIZE);
			ctx.fillStyle = CELL_BG_BEIGE;
		} else {
			ctx.fillStyle = CELL_BG_BEIGE;
			ctx.fillRect(px, py, CELL_SIZE, CELL_SIZE);
			ctx.fillStyle = color;
		}
		ctx.font = TERRAIN_FONT + "px 'Microsoft JhengHei', 'PingFang TC', 'Noto Sans TC', 'Noto Sans CJK TC', sans-serif";
		ctx.textAlign = 'center';
		ctx.textBaseline = 'middle';
		ctx.fillText(char || '?', px + CELL_SIZE / 2, py + CELL_SIZE / 2);
	}

	function ensureCanvasSize() {
		var wrap = canvas.parentElement;
		var fallback = Math.min(400, (typeof window !== 'undefined' && window.innerWidth) || 300);
		var w = (wrap && wrap.clientWidth) || canvas.clientWidth || fallback;
		var h = (wrap && wrap.clientHeight) || canvas.clientHeight || fallback;
		if (w <= 0 || h <= 0) { w = fallback; h = fallback; }
		VIEWPORT_W = Math.max(1, Math.floor(w / CELL_SIZE));
		VIEWPORT_H = Math.max(1, Math.floor(h / CELL_SIZE));
		var logicalW = VIEWPORT_W * CELL_SIZE;
		var logicalH = VIEWPORT_H * CELL_SIZE;
		var dpr = window.devicePixelRatio || 1;
		window._canvasLogicalW = logicalW;
		window._canvasLogicalH = logicalH;
		if (canvas.width !== logicalW * dpr || canvas.height !== logicalH * dpr) {
			canvas.width = logicalW * dpr;
			canvas.height = logicalH * dpr;
			ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		}
	}

	function drawChunkView(cx, cy, rows, entities, me, colors) {
		ensureCanvasSize();
		clear();
		if (!me || me.x === undefined || me.y === undefined) {
			ctx.fillStyle = '#e0e0e0';
			ctx.font = '16px sans-serif';
			ctx.textAlign = 'center';
			ctx.textBaseline = 'middle';
			ctx.fillText('請先登入', VIEWPORT_W * CELL_SIZE / 2, VIEWPORT_H * CELL_SIZE / 2);
			return;
		}
		// 支援三種格式：扁平 rows[idx]、巢狀 rows[ly][lx]、舊版每行一字串 rows[ly].charAt(lx)
		var cellCount = 151 * 151;
		var isFlat = rows && Array.isArray(rows) && rows.length >= cellCount && (typeof rows[0] === 'string');
		var isNested = rows && Array.isArray(rows) && rows.length >= 151 && rows[0] && Array.isArray(rows[0]) && rows[0].length >= 151;
		var isStringRows = rows && Array.isArray(rows) && rows.length >= 151 && (typeof rows[0] === 'string') && rows[0].length >= 151;
		var hasRows = isFlat || isNested || isStringRows;
		if (!rows || !Array.isArray(rows) || rows.length < 151 || !hasRows) {
			ctx.fillStyle = '#e0e0e0';
			ctx.font = '14px sans-serif';
			ctx.textAlign = 'center';
			ctx.textBaseline = 'middle';
			ctx.fillText('載入地形中…', VIEWPORT_W * CELL_SIZE / 2, VIEWPORT_H * CELL_SIZE / 2);
			return;
		}
		var chunkOriginX = cx * 151;
		var chunkOriginY = cy * 151;
		var cameraLx = me.x - chunkOriginX;
		var cameraLy = me.y - chunkOriginY;
		lastCx = cx;
		lastCy = cy;
		lastCameraLx = cameraLx;
		lastCameraLy = cameraLy;

		var halfW = Math.floor(VIEWPORT_W / 2);
		var halfH = Math.floor(VIEWPORT_H / 2);
		var lx0 = cameraLx + panOffsetX - halfW;
		var ly0 = cameraLy + panOffsetY - halfH;
		var outLeft = lx0 + VIEWPORT_W + 1 <= 0 || ly0 + VIEWPORT_H + 1 <= 0;
		var outRight = lx0 >= 151 || ly0 >= 151;
		if (outLeft || outRight) {
			ctx.fillStyle = '#e0e0e0';
			ctx.font = '14px sans-serif';
			ctx.textAlign = 'center';
			ctx.textBaseline = 'middle';
			ctx.fillText('區塊外，請稍候…', VIEWPORT_W * CELL_SIZE / 2, VIEWPORT_H * CELL_SIZE / 2);
			return;
		}

		for (var dy = 0; dy < VIEWPORT_H + 1; dy++) {
			for (var dx = 0; dx < VIEWPORT_W + 1; dx++) {
				var lx = lx0 + dx;
				var ly = ly0 + dy;
				if (lx < 0 || lx >= 151 || ly < 0 || ly >= 151) {
					ctx.fillStyle = '#0d0d0d';
					ctx.fillRect(dx * CELL_SIZE, dy * CELL_SIZE, CELL_SIZE, CELL_SIZE);
					continue;
				}
				var ch = '';
				if (isFlat) {
					var idx = ly * 151 + lx;
					ch = (rows[idx] !== undefined && rows[idx] !== null && rows[idx] !== '') ? rows[idx] : '草';
				} else if (isNested) {
					var row = rows[ly];
					if (Array.isArray(row) && row.length > lx) ch = row[lx];
					else if (typeof row === 'string' && row.length > lx) ch = row[lx];
				} else if (isStringRows) {
					var sr = rows[ly];
					if (typeof sr === 'string') ch = (sr.charAt && sr.charAt(lx)) || sr[lx] || '';
				}
				if (!ch) ch = '草';
				var idx = ly * 151 + lx;
				var cellColor = (colors && colors[idx]) || TERRAIN_COLORS[ch];
				var cellPx = dx * CELL_SIZE;
				var cellPy = dy * CELL_SIZE;
				try {
					drawCell(cellPx, cellPy, ch, cellColor);
				} catch (err) {
					ctx.fillStyle = '#444';
					ctx.fillRect(cellPx, cellPy, CELL_SIZE, CELL_SIZE);
				}
			}
		}

		(entities || []).forEach(function (e) {
			const lx = e.x - chunkOriginX;
			const ly = e.y - chunkOriginY;
			const dx = lx - lx0;
			const dy = ly - ly0;
			if (dx < -0.5 || dx > VIEWPORT_W + 0.5 || dy < -0.5 || dy > VIEWPORT_H + 0.5) return;
			const px = dx * CELL_SIZE + CELL_SIZE / 2;
			const py = dy * CELL_SIZE + CELL_SIZE / 2;
			ctx.fillStyle = e.kind === 'player' ? '#7ec8e3' : '#c4b8a8';
			ctx.beginPath();
			ctx.arc(px, py, ROLE_CIRCLE / 2, 0, Math.PI * 2);
			ctx.fill();
			ctx.strokeStyle = '#333';
			ctx.lineWidth = 1;
			ctx.stroke();
			ctx.fillStyle = '#111';
			ctx.font = '14px sans-serif';
			ctx.textAlign = 'center';
			ctx.textBaseline = 'middle';
			ctx.fillText(e.display_char || '?', px, py);
		});
	}

	function getClickWorldCoord(offsetX, offsetY) {
		const halfW = Math.floor(VIEWPORT_W / 2);
		const halfH = Math.floor(VIEWPORT_H / 2);
		const lx = lastCameraLx + panOffsetX - halfW + Math.floor(offsetX / CELL_SIZE);
		const ly = lastCameraLy + panOffsetY - halfH + Math.floor(offsetY / CELL_SIZE);
		const wx = lastCx * 151 + lx;
		const wy = lastCy * 151 + ly;
		return { x: wx, y: wy };
	}

	function startDrag(clientX, clientY) {
		if (dragging) return;
		justDragged = false;
		dragging = true;
		dragStartX = clientX;
		dragStartY = clientY;
		panStartX = panOffsetX;
		panStartY = panOffsetY;
	}

	// 拖曳直覺：拖哪就往哪（拖右＝地圖往右移）
	function moveDrag(clientX, clientY) {
		if (!dragging) return;
		var dx = clientX - dragStartX;
		var dy = clientY - dragStartY;
		panOffsetX = panStartX - Math.round(dx / CELL_SIZE);
		panOffsetY = panStartY - Math.round(dy / CELL_SIZE);
		if (window.canvasDrawChunkView && window.gameState) {
			var s = window.gameState();
			if (s) window.canvasDrawChunkView(s.cx, s.cy, s.rows, s.entities, s.me, s.colors);
		}
	}

	function endDrag() {
		// 只有真的拖動過地圖才標記 justDragged，否則會吃掉點擊移動
		if (dragging && (panOffsetX !== panStartX || panOffsetY !== panStartY)) justDragged = true;
		dragging = false;
	}

	function isDragging() {
		return dragging;
	}

	function consumeJustDragged() {
		var j = justDragged;
		justDragged = false;
		return j;
	}

	function resetPan() {
		panOffsetX = 0;
		panOffsetY = 0;
	}

	function initMap() {
		ensureCanvasSize();
		clear();
		ctx.fillStyle = '#666';
		ctx.font = '24px sans-serif';
		ctx.textAlign = 'center';
		ctx.textBaseline = 'middle';
		ctx.fillText('奇點世界', VIEWPORT_W * CELL_SIZE / 2, VIEWPORT_H * CELL_SIZE / 2 - 20);
		ctx.font = '14px sans-serif';
		ctx.fillText('連線後請登入', VIEWPORT_W * CELL_SIZE / 2, VIEWPORT_H * CELL_SIZE / 2 + 20);
	}

	window.canvasClear = clear;
	window.canvasEnsureCanvasSize = ensureCanvasSize;
	window.canvasDrawCell = drawCell;
	window.canvasDrawChunkView = drawChunkView;
	window.canvasInitMap = initMap;
	window.canvasGetClickWorldCoord = getClickWorldCoord;
	window.canvasStartDrag = startDrag;
	window.canvasMoveDrag = moveDrag;
	window.canvasEndDrag = endDrag;
	window.canvasIsDragging = isDragging;
	window.canvasConsumeJustDragged = consumeJustDragged;
	window.canvasResetPan = resetPan;
	window.CELL_SIZE = CELL_SIZE;
	window.TERRAIN_FONT = TERRAIN_FONT;
	window.ROLE_CIRCLE = ROLE_CIRCLE;
})();

document.addEventListener('DOMContentLoaded', function () {
	var canvas = document.getElementById('canvas');
	function doInit() {
		if (window.canvasEnsureCanvasSize) window.canvasEnsureCanvasSize();
		if (window.canvasInitMap) window.canvasInitMap();
	}
	doInit();
	requestAnimationFrame(function () { doInit(); });
	if (canvas) {
		function onResize() {
			if (!window.canvasEnsureCanvasSize) return;
			window.canvasEnsureCanvasSize();
			if (window.gameState && window.canvasDrawChunkView) {
				var s = window.gameState();
				if (s && s.me) window.canvasDrawChunkView(s.cx, s.cy, s.rows, s.entities, s.me, s.colors);
				else if (window.canvasInitMap) window.canvasInitMap();
			} else if (window.canvasInitMap) window.canvasInitMap();
		}
		window.addEventListener('resize', onResize);
	if (canvas) {
		function getCoord(e) {
			var rect = canvas.getBoundingClientRect();
			var lw = window._canvasLogicalW || rect.width;
			var lh = window._canvasLogicalH || rect.height;
			var scaleX = lw / rect.width;
			var scaleY = lh / rect.height;
			var x = (e.clientX !== undefined ? e.clientX : e.touches[0].clientX) - rect.left;
			var y = (e.clientY !== undefined ? e.clientY : e.touches[0].clientY) - rect.top;
			return { x: x * scaleX, y: y * scaleY, clientX: e.clientX !== undefined ? e.clientX : e.touches[0].clientX, clientY: e.clientY !== undefined ? e.clientY : e.touches[0].clientY };
		}
		canvas.addEventListener('mousedown', function (e) {
			if (e.button !== 0) return;
			window.canvasStartDrag(e.clientX, e.clientY);
		});
		canvas.addEventListener('mousemove', function (e) {
			window.canvasMoveDrag(e.clientX, e.clientY);
		});
		canvas.addEventListener('mouseup', function (e) {
			window.canvasEndDrag();
			// 未拖曳時直接當成點擊，避免依賴 click 事件被吃掉的問題
			if (window.canvasConsumeJustDragged && !window.canvasConsumeJustDragged()) {
				var rect = canvas.getBoundingClientRect();
				var lw = window._canvasLogicalW || rect.width;
				var lh = window._canvasLogicalH || rect.height;
				var x = (e.clientX - rect.left) * (lw / rect.width);
				var y = (e.clientY - rect.top) * (lh / rect.height);
				if (window.gameOnCanvasClick) window.gameOnCanvasClick(x, y);
			}
		});
		canvas.addEventListener('mouseleave', function () { window.canvasEndDrag(); });
		canvas.addEventListener('touchstart', function (e) {
			if (e.touches.length === 1) window.canvasStartDrag(e.touches[0].clientX, e.touches[0].clientY);
		}, { passive: true });
		canvas.addEventListener('touchmove', function (e) {
			if (e.touches.length === 1) window.canvasMoveDrag(e.touches[0].clientX, e.touches[0].clientY);
		}, { passive: true });
		canvas.addEventListener('touchend', function (e) {
			if (e.touches.length === 0) {
				window.canvasEndDrag();
				if (window.canvasConsumeJustDragged && !window.canvasConsumeJustDragged() && e.changedTouches && e.changedTouches[0]) {
					var t = e.changedTouches[0];
					var rect = canvas.getBoundingClientRect();
					var lw = window._canvasLogicalW || rect.width;
					var lh = window._canvasLogicalH || rect.height;
					var x = (t.clientX - rect.left) * (lw / rect.width);
					var y = (t.clientY - rect.top) * (lh / rect.height);
					if (window.gameOnCanvasClick) window.gameOnCanvasClick(x, y);
				}
			}
		});
	}
	}
});

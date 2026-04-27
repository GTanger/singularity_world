// sw-grid-render.js：9×9 方格 DOM 主渲染器（renderMap + 短腳連線）。v0.20.48
(function () {
    'use strict';

    /**
     * 根據 window.SwGrid 共享狀態重新渲染 DOM 格子地圖。
     *
     * 佈局原則：玩家當前格置中（相對 grid-map 容器）。
     * 僅渲染已探索 (explored=true) 的格子。
     * 格子之間畫 DOM 短腳（—水平 |垂直）代表當前格出口。
     * 未探索格子完全不渲染（不是灰格，是不存在）。
     */
    function renderMap() {
        var G = window.SwGrid;
        if (!G || !G.gridMapEl) return;

        var gridMapEl   = G.gridMapEl;
        var mapData     = G.mapData;
        var revealedCells = G.revealedCells;
        var DIR_DELTA   = G.DIR_DELTA;
        var CELL_W      = G.CELL_W;
        var CELL_H      = G.CELL_H;
        var CELL_GAP_X  = G.CELL_GAP_X;
        var CELL_GAP_Y  = G.CELL_GAP_Y;
        var STEP_X      = G.STEP_X;
        var STEP_Y      = G.STEP_Y;

        // 建立格子 lookup：key = "x,y"
        var cellMap = {};
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            var key = c.x + ',' + c.y;
            cellMap[key] = c;
            // 同步揭露快取
            if (c.explored) revealedCells.add(key);
        }

        // 計算已揭露格子的邊界
        var minX = Infinity, maxX = -Infinity;
        var minY = Infinity, maxY = -Infinity;
        revealedCells.forEach(function (k) {
            var parts = k.split(',');
            var x = parseInt(parts[0], 10);
            var y = parseInt(parts[1], 10);
            if (cellMap[k]) {
                if (x < minX) minX = x;
                if (x > maxX) maxX = x;
                if (y < minY) minY = y;
                if (y > maxY) maxY = y;
            }
        });

        var px = mapData.playerX;
        var py = mapData.playerY;

        // 若沒有格子則只渲染玩家當前格（佔位）
        if (!isFinite(minX)) {
            minX = maxX = px;
            minY = maxY = py;
        }

        // 清空
        gridMapEl.innerHTML = '';

        // 計算容器大小（格子範圍 + 邊距）
        var cols = maxX - minX + 1;
        var rows = maxY - minY + 1;
        var containerW = cols * STEP_X + CELL_GAP_X;
        var containerH = rows * STEP_Y + CELL_GAP_Y;
        gridMapEl.style.width  = containerW + 'px';
        gridMapEl.style.height = containerH + 'px';

        // 渲染已揭露格子
        revealedCells.forEach(function (k) {
            var cell = cellMap[k];
            if (!cell) return; // 本次視野沒帶到此格，跳過

            var cx = cell.x;
            var cy = cell.y;
            var isPlayer = (cx === px && cy === py);

            // 螢幕座標（Y 軸反轉：grid y 正方向向北，螢幕 y 正方向向下）
            var sx = (cx - minX) * STEP_X + CELL_GAP_X / 2;
            var sy = (maxY - cy) * STEP_Y + CELL_GAP_Y / 2;

            var el = document.createElement('div');
            el.className = 'gmap-cell' + (isPlayer ? ' gmap-cell-player' : '');
            // SW-23：依 category 追加視覺 class
            if (cell.category === 'landmark') {
                el.classList.add('gmap-cell-landmark');
            } else if (cell.category === 'infra') {
                el.classList.add('gmap-cell-infra');
            }
            // SW-23：底色改用 kind 色表（透過 SwGrid.colorFor）
            el.style.backgroundColor = window.SwGrid.colorFor(cell);
            el.style.left = sx + 'px';
            el.style.top  = sy + 'px';
            el.setAttribute('data-x', cx);
            el.setAttribute('data-y', cy);
            // SW-23：tooltip 顯示地標名 + 中文地形類別
            el.title = cell.name && cell.name !== cell.terrain
                ? cell.name + '（' + cell.terrain + '）'
                : (cell.terrain || '');

            var nameEl = document.createElement('span');
            nameEl.className = 'gmap-cell-name';
            nameEl.textContent = cell.name || cell.terrain || '？';
            el.appendChild(nameEl);

            // SW-24：地標 icon（右上角單字漢字）
            var iconChar = window.SwGrid.iconFor(cell);
            if (iconChar) {
                var iconEl = document.createElement('span');
                iconEl.className = 'gmap-cell-icon';
                iconEl.textContent = iconChar;
                el.appendChild(iconEl);
            }

            // 當前格點擊回到中心（覆蓋上方 title）
            if (isPlayer) {
                el.style.cursor = 'pointer';
                el.title = '點此回到中心';
                el.addEventListener('click', function (ev) {
                    ev.stopPropagation();
                    window.centerOnPlayer && window.centerOnPlayer();
                });
            }

            gridMapEl.appendChild(el);
        });

        // ── 玩家格置中計算（更新共享狀態供 applyGridTransform 使用）──
        G._playerCenterX = (px - minX) * STEP_X + CELL_GAP_X / 2 + CELL_W / 2;
        G._playerCenterY = (maxY - py) * STEP_Y + CELL_GAP_Y / 2 + CELL_H / 2;
        G._dragX = 0;
        G._dragY = 0;
        requestAnimationFrame(G.applyGridTransform);

        // ── 當前格短腳：從格子邊緣往外延伸，代表出口方向 ──
        var pCenterX = G._playerCenterX;
        var pCenterY = G._playerCenterY;
        var LEG_X = CELL_GAP_X / 2;
        var LEG_Y = CELL_GAP_Y / 2;

        for (var ei = 0; ei < mapData.exits.length; ei++) {
            var dir   = mapData.exits[ei].direction;
            var delta = DIR_DELTA[dir];
            if (!delta) continue;

            var legEl = document.createElement('div');
            legEl.className = 'gmap-leg';

            if (delta.dx !== 0) {
                // 東/西：水平短腳
                var xStart = delta.dx > 0
                    ? pCenterX + CELL_W / 2
                    : pCenterX - CELL_W / 2 - LEG_X;
                legEl.style.left   = xStart + 'px';
                legEl.style.top    = (pCenterY - 1) + 'px';
                legEl.style.width  = LEG_X + 'px';
                legEl.style.height = '2px';
            } else {
                // 南/北：垂直短腳（北 = dy > 0 = 螢幕 y 減少）
                var yStart = delta.dy > 0
                    ? pCenterY - CELL_H / 2 - LEG_Y
                    : pCenterY + CELL_H / 2;
                legEl.style.left   = (pCenterX - 1) + 'px';
                legEl.style.top    = yStart + 'px';
                legEl.style.width  = '2px';
                legEl.style.height = LEG_Y + 'px';
            }

            gridMapEl.appendChild(legEl);
        }

        // 玩家當前格若後端未帶到，渲染佔位格
        var playerKey = px + ',' + py;
        if (!cellMap[playerKey]) {
            var pEl = document.createElement('div');
            pEl.className = 'gmap-cell gmap-cell-player';
            pEl.style.left = (CELL_GAP_X / 2) + 'px';
            pEl.style.top  = (CELL_GAP_Y / 2) + 'px';
            pEl.setAttribute('data-x', px);
            pEl.setAttribute('data-y', py);
            var pNameEl = document.createElement('span');
            pNameEl.className = 'gmap-cell-name';
            pNameEl.textContent = '？';
            pEl.appendChild(pNameEl);
            var pMarker = document.createElement('span');
            pMarker.className = 'gmap-player-marker';
            pMarker.textContent = '●';
            pEl.appendChild(pMarker);
            gridMapEl.appendChild(pEl);
        }
    }

    window.SwGridRender = { renderMap: renderMap };
    console.log('[sw-grid-render] v0.20.48 loaded | SW-24 地標 icon + fallback 修正');
})();

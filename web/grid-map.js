/**
 * grid-map.js v0.20.35
 * 方格地圖 DOM 渲染器 + 物件欄 + 移動控制
 * 階段 2（SW-18）：sw-minimap 區塊填色塊 9×9 純視覺地圖
 *
 * 依賴：
 *   - main.js 的 WebSocket 已就位，透過 window.gameSend 送出指令
 *   - 後端 grid_view 消息由 main.js 分派至 window.onGridView(msg)
 *
 * 佈局（四區塊）：
 *   [頂欄：日晷時間] — 由 index.html / main.js 掌管
 *   [中左：物件欄][中右：上小房間描述 + 下大地圖]
 *   [log+map row：左 log 暫空 + 右 160×160 色塊 minimap]
 *   [底欄：玩家狀態] — 由 index.html / main.js 掌管
 */
(function () {
    'use strict';

    // ── 格子尺寸常數 ────────────────────────────────────────────────────
    var CELL_W = 72;   // px，格子寬
    var CELL_H = 48;   // px，格子高
    var CELL_GAP_X = 28; // 格子水平間距（連線寬度）
    var CELL_GAP_Y = 20; // 格子垂直間距（連線高度）
    var STEP_X = CELL_W + CELL_GAP_X;
    var STEP_Y = CELL_H + CELL_GAP_Y;

    // ── 揭露快取（玩家本地端；永久紀錄）────────────────────────────────
    // 格式：Set<"x,y">
    // 從後端 grid_view 的 cells[].explored=true 同步，不單獨管理
    var revealedCells = new Set();

    // ── 地形氛圍句快取 ────────────────────────────────────────────────
    // 從 /data/config/terrain_ambience.json 非同步載入；載入前顯示 ''
    var terrainAmbience = null;
    fetch('/data/config/terrain_ambience.json')
        .then(function (r) { return r.json(); })
        .then(function (j) { terrainAmbience = j.terrains || {}; })
        .catch(function (e) { console.warn('[grid-map] terrain_ambience 載入失敗:', e); });

    // 12 個時段：每 2 小時一段，對齊 hours 0-23
    var PHASE_LABELS = ['深夜','凌晨','破曉','清晨','上午','正午','午後','下午','傍晚','入夜','晚間','半夜'];

    /**
     * 依地形 key 與遊戲小時取氛圍句。
     * @param {string} terrain - 地形 key（中文字符，如「木」「草」）
     * @param {number} hour    - 0-23
     * @returns {string}
     */
    function getAmbience(terrain, hour) {
        if (!terrainAmbience || !terrain) return '';
        var phaseLabel = PHASE_LABELS[Math.floor(hour / 2)] || '';
        var terrainMap = terrainAmbience[terrain];
        if (!terrainMap) return '';
        return terrainMap[phaseLabel] || '';
    }

    // ── 目前地圖狀態 ─────────────────────────────────────────────────────
    var mapData = {
        playerX: 0,
        playerY: 0,
        cells: [],      // GridCellView[]
        exits: [],      // ExitView[]
        entities: [],   // ViewEntity[]
        objects: []     // ViewObject[]
    };

    // ── 地形色映射表（對應後端 terrain_name_zh() 回傳的中文 key）────────────
    // 後端 src/grid/reveal.rs terrain_name_zh() 回傳中文字串，非英文 enum 名
    var TERRAIN_COLOR = {
        '草原': '#a8b06a',   // Grassland：黃綠
        '平地': '#b8ba82',   // Plain：淡黃綠
        '林地': '#6b8e5a',   // Forest：深綠
        '疏林': '#7fa868',   // ForestLight：中綠
        '密林': '#4d7040',   // ForestHeavy：暗綠
        '丘陵': '#9a7a55',   // Hills：棕褐
        '山嶺': '#7a6a5a',   // Mountain：灰褐
        '水域': '#6a8aa8',   // Water：藍灰
        '深水': '#5a7a9a',   // WaterDeep：深藍
        '沼地': '#7a9080',   // Swamp：藍綠灰
        '荒漠': '#d8c89a',   // Desert：沙黃
        '凍原': '#d8dce8',   // Tundra：淡藍白
        '叢林': '#4a7848',   // Jungle：暗綠
        '地塊': '#8a8070'    // 預設：灰褐
    };
    // 未知地形預設色（探索中 / 後端未送 terrain）
    var TERRAIN_COLOR_UNKNOWN = '#2a2418';  // 黑格：未探索

    // ── 地標墨點（階段 5b 加 has_landmark 後實啟用；現在 placeholder false）───
    // 未來：if (cell.has_landmark) el.classList.add('sw-minimap-cell--landmark');

    // ── DOM 引用 ─────────────────────────────────────────────────────────
    var gridMapEl = null;
    var roomDescEl = null;
    var objectListEl = null;
    var minimapContainerEl = null;  // .sw-minimap（#sw-minimap-container）

    // ── 工具：方向 ←→ (dx, dy) 轉換（只走四向）──────────────────────────
    var DIR_DELTA = {
        '北': { dx: 0, dy: 1 },
        '東': { dx: 1, dy: 0 },
        '南': { dx: 0, dy: -1 },
        '西': { dx: -1, dy: 0 }
    };

    function dirToDelta(dir) {
        return DIR_DELTA[dir] || null;
    }

    // delta (dx, dy) 轉方向字串
    function deltaToDir(dx, dy) {
        for (var d in DIR_DELTA) {
            if (DIR_DELTA[d].dx === dx && DIR_DELTA[d].dy === dy) return d;
        }
        return null;
    }

    // ── 逃脫 HTML 特殊字元 ────────────────────────────────────────────────
    function esc(s) {
        var d = document.createElement('div');
        d.textContent = String(s || '');
        return d.innerHTML;
    }

    // ── 地圖渲染 ─────────────────────────────────────────────────────────
    /**
     * 根據 mapData 重新渲染 DOM 格子地圖。
     *
     * 佈局原則：玩家當前格置中（相對 grid-map 容器）。
     * 僅渲染已探索 (explored=true) 的格子。
     * 格子之間畫 DOM 連線（—水平 |垂直）代表出口。
     * 未探索格子完全不渲染（不是灰格，是不存在）。
     */
    function renderMap() {
        if (!gridMapEl) return;

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

        // 若沒有格子則只渲染玩家當前格（佔位）
        if (!isFinite(minX)) {
            minX = maxX = mapData.playerX;
            minY = maxY = mapData.playerY;
        }

        var px = mapData.playerX;
        var py = mapData.playerY;

        // 清空
        gridMapEl.innerHTML = '';

        // 計算容器大小（格子範圍 + 邊距）
        var cols = maxX - minX + 1;
        var rows = maxY - minY + 1;
        var containerW = cols * STEP_X + CELL_GAP_X;
        var containerH = rows * STEP_Y + CELL_GAP_Y;
        gridMapEl.style.width = containerW + 'px';
        gridMapEl.style.height = containerH + 'px';
        // position 交給 CSS（.gmap-grid { position: absolute }），不在 JS inline 覆蓋

        // 計算已揭露出口方向（從 exits 欄位）
        var exitDirSet = new Set();
        for (var ei = 0; ei < mapData.exits.length; ei++) {
            exitDirSet.add(mapData.exits[ei].direction);
        }

        // 渲染格子
        revealedCells.forEach(function (k) {
            var cell = cellMap[k];
            if (!cell) return; // 本次視野沒帶到此格則跳過（保留在 revealedCells 但不渲染）

            var cx = cell.x;
            var cy = cell.y;
            var isPlayer = (cx === px && cy === py);

            // 螢幕座標（Y 軸反轉：grid y 正方向向北，螢幕 y 正方向向下）
            var sx = (cx - minX) * STEP_X + CELL_GAP_X / 2;
            var sy = (maxY - cy) * STEP_Y + CELL_GAP_Y / 2;

            var el = document.createElement('div');
            el.className = 'gmap-cell' + (isPlayer ? ' gmap-cell-player' : '');
            el.style.left = sx + 'px';
            el.style.top = sy + 'px';
            el.setAttribute('data-x', cx);
            el.setAttribute('data-y', cy);

            var nameEl = document.createElement('span');
            nameEl.className = 'gmap-cell-name';
            nameEl.textContent = cell.name || '地塊';
            el.appendChild(nameEl);

            // 當前格不加圓點——格名 + 邊框/底色已足夠區分
            // 當前格可點擊「重新錨定回中心」（拖曳後回歸用，見 centerOnPlayer）
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

        // ── 地圖置中：算玩家格在容器內中心座標，供 applyGridTransform 用 ──
        // 玩家走到新格時清除拖曳偏移（強制重新置中）
        _playerCenterX = (px - minX) * STEP_X + CELL_GAP_X / 2 + CELL_W / 2;
        _playerCenterY = (maxY - py) * STEP_Y + CELL_GAP_Y / 2 + CELL_H / 2;
        _dragX = 0;
        _dragY = 0;
        requestAnimationFrame(applyGridTransform);

        // ── 方向 C：只畫「當前格」的四向短腳（mapData.exits）──
        // 遠處已揭露格不連線（只見當前格原則 + 避免 tile-RPG 慣例違和）
        // 短腳從格子邊緣往外延伸 gap 的一半，不到鄰格（不承諾對面有東西）
        var pCenterX = (px - minX) * STEP_X + CELL_GAP_X / 2 + CELL_W / 2;
        var pCenterY = (maxY - py) * STEP_Y + CELL_GAP_Y / 2 + CELL_H / 2;
        var LEG_X = CELL_GAP_X / 2;
        var LEG_Y = CELL_GAP_Y / 2;

        for (var ei = 0; ei < mapData.exits.length; ei++) {
            var dir = mapData.exits[ei].direction;
            var delta = DIR_DELTA[dir];
            if (!delta) continue;

            var legEl = document.createElement('div');
            legEl.className = 'gmap-leg';

            if (delta.dx !== 0) {
                // 東/西：水平短腳
                var xStart = delta.dx > 0
                    ? pCenterX + CELL_W / 2           // 東：從格子右邊緣往外
                    : pCenterX - CELL_W / 2 - LEG_X;  // 西：從格子左邊緣往外
                legEl.style.left = xStart + 'px';
                legEl.style.top = (pCenterY - 1) + 'px';
                legEl.style.width = LEG_X + 'px';
                legEl.style.height = '2px';
            } else {
                // 南/北：垂直短腳（北 = dy > 0 = 螢幕 y 減少）
                var yStart = delta.dy > 0
                    ? pCenterY - CELL_H / 2 - LEG_Y   // 北：從格子上邊緣往外
                    : pCenterY + CELL_H / 2;          // 南：從格子下邊緣往外
                legEl.style.left = (pCenterX - 1) + 'px';
                legEl.style.top = yStart + 'px';
                legEl.style.width = '2px';
                legEl.style.height = LEG_Y + 'px';
            }

            gridMapEl.appendChild(legEl);
        }

        // 玩家格未被探索也要顯示（進格即見，自己所在格必定可見）
        var playerKey = px + ',' + py;
        if (!cellMap[playerKey]) {
            // 後端沒帶過來但玩家在此——渲染佔位格
            var sx = (0) * STEP_X + CELL_GAP_X / 2;
            var sy = (0) * STEP_Y + CELL_GAP_Y / 2;
            var el = document.createElement('div');
            el.className = 'gmap-cell gmap-cell-player';
            el.style.left = sx + 'px';
            el.style.top = sy + 'px';
            el.setAttribute('data-x', px);
            el.setAttribute('data-y', py);
            var nameEl = document.createElement('span');
            nameEl.className = 'gmap-cell-name';
            nameEl.textContent = '？';
            el.appendChild(nameEl);
            var markerEl = document.createElement('span');
            markerEl.className = 'gmap-player-marker';
            markerEl.textContent = '●';
            el.appendChild(markerEl);
            gridMapEl.appendChild(el);
        }

        // 舊 scrollToPlayer 已砍（那是 overflow:auto 時代的遺留，會 shift viewport.scrollLeft
        // 跟 transform 架構衝突，造成玩家移動後視覺偏移）
    }

    // ── 色塊 Minimap 渲染（SW-18，階段 2）────────────────────────────────
    /**
     * 9×9 色塊網格，玩家格固定在正中央（row 4, col 4，0-indexed）。
     * 從 mapData 取玩家位置為中心，向四方各延伸 4 格。
     * 只用 revealedCells + cellMap，不探索新格，不接受任何互動。
     * 容器已由 CSS 鎖 160×160px（.sw-minimap）。
     */
    function renderMinimap() {
        if (!minimapContainerEl) return;

        var GRID_SIZE = 9;           // 9×9
        var HALF = 4;                // 玩家在中心：索引 4（0-based）
        var px = mapData.playerX;
        var py = mapData.playerY;

        // 建 cell lookup（本次視野）
        var cellMap = {};
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            cellMap[c.x + ',' + c.y] = c;
        }

        // 清空容器，建 grid div（#sw-minimap-grid 若已存在就重用，否則新建）
        var gridEl = minimapContainerEl.querySelector('.sw-minimap-grid');
        if (!gridEl) {
            gridEl = document.createElement('div');
            gridEl.className = 'sw-minimap-grid';
            minimapContainerEl.appendChild(gridEl);
        }
        gridEl.innerHTML = '';

        // 渲染 9×9，row 0 = 北 +4，row 8 = 南 -4（螢幕向下 = 遊戲向南）
        for (var row = 0; row < GRID_SIZE; row++) {
            for (var col = 0; col < GRID_SIZE; col++) {
                var worldX = px + (col - HALF);       // col 0 = px-4, col 4 = px, col 8 = px+4
                var worldY = py + (HALF - row);       // row 0 = py+4（北）, row 4 = py, row 8 = py-4（南）
                var key = worldX + ',' + worldY;

                var cellEl = document.createElement('div');
                cellEl.className = 'sw-minimap-cell';

                var isPlayer = (row === HALF && col === HALF);
                var isRevealed = revealedCells.has(key);
                var cell = cellMap[key];

                if (isPlayer) {
                    // 玩家格：優先顯示地形色，加高亮邊框
                    var playerTerrain = cell ? cell.terrain : '';
                    cellEl.style.backgroundColor = TERRAIN_COLOR[playerTerrain] || TERRAIN_COLOR_UNKNOWN;
                    cellEl.classList.add('sw-minimap-cell--here');
                } else if (isRevealed && cell) {
                    // 已探索格：顯示地形色
                    cellEl.style.backgroundColor = TERRAIN_COLOR[cell.terrain] || TERRAIN_COLOR_UNKNOWN;
                    // 地標墨點（placeholder：階段 5b 啟用）
                    // if (cell.has_landmark) cellEl.classList.add('sw-minimap-cell--landmark');
                } else if (isRevealed) {
                    // 在 revealedCells 但本次視野沒帶資料——已探索但未知地形，用暗色
                    cellEl.style.backgroundColor = '#5a5040';
                } else {
                    // 未探索：黑格
                    cellEl.style.backgroundColor = TERRAIN_COLOR_UNKNOWN;
                }

                gridEl.appendChild(cellEl);
            }
        }
    }

    // ── 房間描述渲染 ──────────────────────────────────────────────────────
    function renderRoomDesc() {
        if (!roomDescEl) return;
        var currentCell = mapData.cells.find(function (c) {
            return c.x === mapData.playerX && c.y === mapData.playerY;
        });
        var terrainName = (currentCell && currentCell.name) ? currentCell.name : '未知地形';
        var terrainKey  = (currentCell && currentCell.terrain) ? currentCell.terrain : '';

        var html = '<div class="mud-room-name">【' + esc(terrainName) + '】</div>';

        // 時段與氛圍句
        var hour = 0;
        var phase = '';
        if (window.gameState) {
            var st = window.gameState();
            if (st && typeof st.game_time_sec_since_midnight === 'number') {
                hour  = Math.floor(st.game_time_sec_since_midnight / 3600) % 24;
                phase = PHASE_LABELS[Math.floor(hour / 2)] || '';
            }
        }
        if (phase) html += '<div class="mud-room-phase">' + esc(phase) + '</div>';

        // 地形氛圍句（SW-4）
        var ambience = getAmbience(terrainKey, hour);
        if (ambience) {
            html += '<div class="mud-room-ambience">' + esc(ambience) + '</div>';
        }

        // 出口方向已用地圖連線視覺化，描述欄不再列方向字

        // 在場者：描述欄只列 NPC（玩家已在物件欄顯示，描述欄不爆版）
        var myId = window.myPlayerId || '';
        var npcsHere = (mapData.entities || []).filter(function (e) {
            return e.kind === 'npc' || e.kind === 'Npc';
        });
        if (npcsHere.length > 0) {
            html += '<div class="mud-room-entities">在場：';
            html += npcsHere.map(function (e) {
                return '<span class="mud-entity-name">' + esc(e.display_name || e.id) + '</span>';
            }).join('、');
            html += '</div>';
        }

        roomDescEl.innerHTML = html;
    }

    // ── 物件欄渲染 ────────────────────────────────────────────────────────
    /**
     * 物件欄項目分類：
     *   1. 「探索」永遠顯示（主動揭露出口/資源動作）
     *   2. 在場 NPC（進格即見）
     *   3. 地上物品（進格即見，來自 objects）
     *   4. 資源動作（需探索揭露——若當前格已有 exits 則顯示資源採集）
     */
    function renderObjectList() {
        if (!objectListEl) return;
        var myId = window.myPlayerId || '';
        var html = '';

        var px = mapData.playerX;
        var py = mapData.playerY;
        var hasExits = mapData.exits && mapData.exits.length > 0;

        // 探索按鈕（若出口未揭露才顯示）
        if (!hasExits) {
            html += '<div class="mud-obj-item mud-obj-action" data-action="explore">探索</div>';
        }

        // 「自己」不在物件欄（姓名欄已顯示，不要三化）

        // 在場 NPC
        var npcs = (mapData.entities || []).filter(function (e) {
            return e.kind !== 'player' || e.id !== myId;
        }).filter(function (e) {
            return e.kind === 'npc' || e.kind === 'Npc';
        });
        if (npcs.length > 0) {
            for (var ni = 0; ni < npcs.length; ni++) {
                var npc = npcs[ni];
                html += '<div class="mud-obj-item mud-obj-npc" data-entity-id="' + esc(npc.id) + '">';
                html += esc(npc.display_name || npc.id);
                html += '</div>';
                if (npc.behavior_text) {
                    html += '<div class="mud-npc-behavior">' + esc(npc.behavior_text) + '</div>';
                }
            }
        }

        // 在場其他玩家
        var otherPlayers = (mapData.entities || []).filter(function (e) {
            return e.kind === 'player' && e.id !== myId;
        });
        if (otherPlayers.length > 0) {
            for (var pi = 0; pi < otherPlayers.length; pi++) {
                var p = otherPlayers[pi];
                html += '<div class="mud-obj-item mud-obj-player" data-entity-id="' + esc(p.id) + '">';
                html += esc(p.display_name || p.id);
                html += '</div>';
            }
        }

        // 地上物改為依地形出「動作插座」（規格：萬物皆物件，地上物 = 動作不是具體物名）
        // 具體物品不列舉，改由下方資源動作區塊呈現

        // 資源動作（需已探索，且從格子 objects 推導）
        if (hasExits) {
            // 已探索才顯示資源採集
            var currentCell = null;
            for (var ci = 0; ci < mapData.cells.length; ci++) {
                if (mapData.cells[ci].x === px && mapData.cells[ci].y === py) {
                    currentCell = mapData.cells[ci];
                    break;
                }
            }
            if (currentCell) {
                var terrain = currentCell.terrain;
                var resourceActions = terrainToActions(terrain);
                if (resourceActions.length > 0) {
                    for (var ri = 0; ri < resourceActions.length; ri++) {
                        html += '<div class="mud-obj-item mud-obj-resource" data-resource="' + esc(resourceActions[ri].key) + '">';
                        html += esc(resourceActions[ri].label);
                        html += '</div>';
                    }
                }
            }
        }

        objectListEl.innerHTML = html;
    }

    // 地形 → 可用採集動作
    function terrainToActions(terrain) {
        var map = {
            'Forest':      [{ key: 'gather_wood',  label: '伐木' }, { key: 'gather_herb', label: '採藥' }],
            'ForestLight': [{ key: 'gather_herb',  label: '採藥' }],
            'ForestHeavy': [{ key: 'gather_wood',  label: '伐木' }, { key: 'gather_herb', label: '採藥' }],
            'Grassland':   [{ key: 'gather_herb',  label: '採集' }],
            'Hills':       [{ key: 'gather_ore',   label: '採礦' }],
            'Swamp':       [{ key: 'gather_water', label: '取水' }],
            'Plain':       []
        };
        return map[terrain] || [];
    }

    // ── 移動邏輯 ─────────────────────────────────────────────────────────
    function moveToCell(tx, ty) {
        var px = mapData.playerX;
        var py = mapData.playerY;
        var dx = tx - px;
        var dy = ty - py;
        var dir = deltaToDir(dx, dy);
        if (!dir) {
            // 不是相鄰格，不允許直接移動
            return;
        }
        // 確認此方向有出口
        var hasExit = mapData.exits.some(function (e) { return e.direction === dir; });
        if (!hasExit) {
            if (window.appendLog) window.appendLog('此方向無出口（尚未探索）');
            return;
        }
        if (window.gameSend) {
            window.gameSend({ type: 'move', direction: dir });
        }
    }

    // ── 事件綁定 ─────────────────────────────────────────────────────────
    // SW-18 階段 2：地圖降為純視覺 minimap，舊 gmap-grid 上的點擊移動
    // 已遷移至物件欄出口卡片（階段 3），此處 gate 掉不綁定
    function bindMapClick() {
        // gate：移動輸入由物件欄出口卡片承接（階段 3），地圖不接受點擊
        // （函式保留供未來恢復，不刪）
    }

    // ── Google Maps 式拖曳：SW-18 gate 掉（地圖純視覺，不接受拖曳）──────
    var _suppressNextClick = false;
    function bindMapDrag() {
        // gate：minimap 純視覺，不接受拖曳互動（函式保留供未來恢復）
    }

    function bindObjectListClick() {
        if (!objectListEl) return;
        objectListEl.addEventListener('click', function (e) {
            var item = e.target.closest('.mud-obj-item');
            if (!item) return;

            // 探索動作
            if (item.getAttribute('data-action') === 'explore') {
                if (window.gameSend) {
                    window.gameSend({ type: 'explore' });
                }
                if (window.appendLog) window.appendLog('正在探索當前格…');
                return;
            }

            // 實體互動（NPC / 玩家）
            var eid = item.getAttribute('data-entity-id');
            if (eid && window.gameSend) {
                window.gameSend({ type: 'do_action', entity_id: eid, action: 'Look' });
                return;
            }

            // 地上物互動
            var oid = item.getAttribute('data-object-id');
            if (oid && window.gameSend) {
                window.gameSend({ type: 'do_action', entity_id: oid, action: 'Look' });
                return;
            }

            // 資源採集（SW-3 只做 UI 架構，送 do_action 示意）
            var resource = item.getAttribute('data-resource');
            if (resource && window.appendLog) {
                window.appendLog('採集動作：' + resource + '（功能待 SW-4 實作）');
            }
        });
    }

    // ── 主入口：grid_view 消息接收 ───────────────────────────────────────
    // 緩存最近一次 msg 供 me 到達後重新渲染（避免 myPlayerId 時序問題導致物件欄未過濾自己）
    var _lastGridMsg = null;
    // 玩家格在容器內的中心座標（每次 renderMap 更新）
    var _playerCenterX = 0;
    var _playerCenterY = 0;
    // 拖曳偏移（玩家對齊視口中心 + 拖曳偏移 = 當前視覺位置）
    var _dragX = 0;
    var _dragY = 0;

    // 套 transform：讓玩家格對齊 viewport 中心 + 用戶拖曳偏移
    function applyGridTransform() {
        if (!gridMapEl) return;
        var vp = gridMapEl.parentElement;
        if (!vp) return;
        // 若 viewport 還沒 layout（父 #app 可能還 hidden），clientWidth/Height 會是 0，
        // 此時算出來的 transform 錯（玩家會偏離中心）→ 延遲 retry 直到 layout 就位
        if (vp.clientWidth === 0 || vp.clientHeight === 0) {
            setTimeout(applyGridTransform, 50);
            return;
        }
        var viewCx = vp.clientWidth  / 2;
        var viewCy = vp.clientHeight / 2;
        var tx = viewCx - _playerCenterX + _dragX;
        var ty = viewCy - _playerCenterY + _dragY;
        gridMapEl.style.transform = 'translate(' + tx + 'px, ' + ty + 'px)';
    }

    // 回到中心：清拖曳偏移，玩家格重回視口中心
    window.centerOnPlayer = function (instant) {
        _dragX = 0;
        _dragY = 0;
        // 加臨時 transition 讓回中心平滑（除非 instant）
        if (!instant && gridMapEl) {
            gridMapEl.style.transition = 'transform 200ms ease-out';
            setTimeout(function () { if (gridMapEl) gridMapEl.style.transition = ''; }, 250);
        }
        applyGridTransform();
    };

    // 視口 resize 時重算 transform（保持玩家置中）
    window.addEventListener('resize', function () { applyGridTransform(); });

    window.onGridView = function (msg) {
        _lastGridMsg = msg;
        mapData.playerX = msg.player_x;
        mapData.playerY = msg.player_y;
        mapData.cells   = msg.cells   || [];
        mapData.exits   = msg.exits   || [];
        mapData.entities = msg.entities || [];
        mapData.objects  = msg.objects  || [];

        // 同步揭露快取
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            if (c.explored) revealedCells.add(c.x + ',' + c.y);
        }
        // 玩家當前格必定可見
        revealedCells.add(mapData.playerX + ',' + mapData.playerY);

        renderMap();
        renderMinimap();
        renderRoomDesc();
        renderObjectList();

        // 同時更新舊 game-ui.js 的 mapState（相容用）
        if (window.mapState) {
            window.mapState.playerX = msg.player_x;
            window.mapState.playerY = msg.player_y;
            window.mapState.moving = false;
        }
    };

    // 外部觸發：myPlayerId 更新後重新渲染物件欄和描述欄（過濾「自己」才能生效）
    window.refreshGridView = function () {
        if (!_lastGridMsg) return;
        renderRoomDesc();
        renderObjectList();
    };

    // ── 初始化 ───────────────────────────────────────────────────────────
    function init() {
        gridMapEl = document.getElementById('gmap-grid');
        roomDescEl = document.getElementById('mud-room-desc');
        objectListEl = document.getElementById('mud-obj-list');
        minimapContainerEl = document.getElementById('sw-minimap-container');

        if (!gridMapEl) return; // 非 game.html 頁面，靜默退出

        bindMapClick();
        bindMapDrag();
        bindObjectListClick();

        console.log('[grid-map] v0.20.34 initialized | SW-18 色塊 minimap');
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();

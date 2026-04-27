/**
 * grid-map.js v0.20.47
 * 方格地圖共享狀態 + 移動控制 + 派發中樞
 * 子模組：sw-grid-render.js / sw-room-desc.js / sw-object-list.js
 *
 * 載入順序：子模組先載入，本檔最後（子模組函式在 runtime 呼叫時才讀 SwGrid，不在載入時執行）
 */
(function () {
    'use strict';

    // ── 格子尺寸常數 ────────────────────────────────────────────────
    var CELL_W    = 72;
    var CELL_H    = 48;
    var CELL_GAP_X = 28;
    var CELL_GAP_Y = 20;
    var STEP_X    = CELL_W + CELL_GAP_X;
    var STEP_Y    = CELL_H + CELL_GAP_Y;

    // ── 揭露快取（永久紀錄，從後端 explored=true 同步）──────────────
    var revealedCells = new Set();

    // ── 目前地圖狀態 ─────────────────────────────────────────────────
    var mapData = {
        playerX: 0,
        playerY: 0,
        cells:    [],   // GridCellView[]
        exits:    [],   // ExitView[]
        entities: [],   // ViewEntity[]
        objects:  []    // ViewObject[]
    };

    // ── 地形色映射表（key 對應後端 kind_str() snake_case，SW-23）────
    var TERRAIN_COLOR = {
        // category=terrain（自然地塊）
        'plain':         '#b8ba82',
        'grassland':     '#a8b06a',
        'forest':        '#6b8e5a',
        'forest_light':  '#7fa868',
        'forest_heavy':  '#4d7040',
        'hills':         '#9a7a55',
        'mountain':      '#7a6a5a',
        'water':         '#6a8aa8',
        'water_deep':    '#5a7a9a',
        'swamp':         '#7a9080',
        'desert':        '#d8c89a',
        'tundra':        '#d8dce8',
        'jungle':        '#4a7848',

        // category=landmark（人造地標，統一暖金底色；視覺差異由邊框/icon 負責）
        'urban':         '#a89060',
        'inn':           '#a89060',
        'tavern':        '#a89060',
        'blacksmith':    '#a89060',
        'general_store': '#a89060',
        'clinic':        '#a89060',
        'workshop':      '#a89060',
        'market':        '#a89060',
        'guild_hall':    '#a89060',
        'temple':        '#a89060',
        'farmhouse':     '#a89060',
        'farm_field':    '#c0b878',
        'academy':       '#a89060',
        'library':       '#a89060',
        'barracks':      '#a89060',
        'guard_post':    '#a89060',
        'warehouse':     '#a89060',
        'granary':       '#a89060',
        'dock':          '#a89060',
        'bathhouse':     '#a89060',
        'courthouse':    '#a89060',
        'jail':          '#a89060',
        'town_hall':     '#a89060',
        'bank':          '#a89060',
        'mint':          '#a89060',
        'stables':       '#a89060',
        'caravanserai':  '#a89060',
        'theater':       '#a89060',
        'arena':         '#a89060',
        'observatory':   '#a89060',
        'alchemist':     '#a89060',
        'mage_tower':    '#a89060',
        'embassy':       '#a89060',
        'prison_yard':   '#a89060',

        // category=infra（通行建設）
        'road':          '#8a7a5a',
        'bridge':        '#9a8060',
        'wall':          '#5a4a3a'
    };

    var UNKNOWN = '#2a2418';

    function colorFor(cell) {
        if (!cell) return UNKNOWN;
        return TERRAIN_COLOR[cell.kind] || UNKNOWN;
    }

    // ── 方向 ←→ delta（四向）────────────────────────────────────────
    var DIR_DELTA = {
        '北': { dx: 0, dy:  1 },
        '東': { dx: 1, dy:  0 },
        '南': { dx: 0, dy: -1 },
        '西': { dx: -1, dy: 0 }
    };

    function dirToDelta(dir)  { return DIR_DELTA[dir] || null; }
    function deltaToDir(dx, dy) {
        for (var d in DIR_DELTA) {
            if (DIR_DELTA[d].dx === dx && DIR_DELTA[d].dy === dy) return d;
        }
        return null;
    }

    // ── HTML 逸脫 ────────────────────────────────────────────────────
    function esc(s) {
        var d = document.createElement('div');
        d.textContent = String(s || '');
        return d.innerHTML;
    }

    // ── NPC/玩家顯示名 ───────────────────────────────────────────────
    var CJK_RE = /[\u3400-\u9fff]/;
    function hasProperName(ent) {
        var name = (ent && (ent.display_name || ent.display_title)) || '';
        return !!(name && CJK_RE.test(name));
    }
    function displayNameOf(ent) {
        var name = (ent && (ent.display_name || ent.display_title)) || '';
        if (name && CJK_RE.test(name)) return name;
        var suffix = (ent && ent.id) ? String(ent.id).slice(-3) : '';
        return suffix ? ('人·' + suffix) : '人';
    }

    // ── DOM 引用 ─────────────────────────────────────────────────────
    var gridMapEl          = null;
    var roomDescEl         = null;
    var objectListEl       = null;
    var minimapContainerEl = null;

    // ── 置中計算狀態（由 sw-grid-render 更新）───────────────────────
    var _playerCenterX = 0;
    var _playerCenterY = 0;
    var _dragX = 0;
    var _dragY = 0;

    // ── transform：玩家格對齊 viewport 中心 ─────────────────────────
    function applyGridTransform() {
        if (!gridMapEl) return;
        var vp = gridMapEl.parentElement;
        if (!vp) return;
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

    // ── 回到中心（清拖曳偏移）────────────────────────────────────────
    window.centerOnPlayer = function (instant) {
        _dragX = 0;
        _dragY = 0;
        if (!instant && gridMapEl) {
            gridMapEl.style.transition = 'transform 200ms ease-out';
            setTimeout(function () { if (gridMapEl) gridMapEl.style.transition = ''; }, 250);
        }
        applyGridTransform();
    };

    window.addEventListener('resize', function () { applyGridTransform(); });

    // ── 色塊 Minimap 渲染（9×9，玩家格置中）─────────────────────────
    function renderMinimap() {
        if (!minimapContainerEl) return;
        var GRID_SIZE = 9;
        var HALF = 4;
        var px = mapData.playerX;
        var py = mapData.playerY;

        var cellMap = {};
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            cellMap[c.x + ',' + c.y] = c;
        }

        var gridEl = minimapContainerEl.querySelector('.sw-minimap-grid');
        if (!gridEl) {
            gridEl = document.createElement('div');
            gridEl.className = 'sw-minimap-grid';
            minimapContainerEl.appendChild(gridEl);
        }
        gridEl.innerHTML = '';

        for (var row = 0; row < GRID_SIZE; row++) {
            for (var col = 0; col < GRID_SIZE; col++) {
                var worldX = px + (col - HALF);
                var worldY = py + (HALF - row);
                var key    = worldX + ',' + worldY;

                var cellEl = document.createElement('div');
                cellEl.className = 'sw-minimap-cell';

                var isPlayer   = (row === HALF && col === HALF);
                var isRevealed = revealedCells.has(key);
                var cell       = cellMap[key];

                if (isPlayer) {
                    cellEl.style.backgroundColor = colorFor(cell);
                    cellEl.classList.add('sw-minimap-cell--here');
                } else if (isRevealed && cell) {
                    cellEl.style.backgroundColor = colorFor(cell);
                } else if (isRevealed) {
                    cellEl.style.backgroundColor = '#5a5040';
                } else {
                    cellEl.style.backgroundColor = UNKNOWN;
                }

                gridEl.appendChild(cellEl);
            }
        }
    }

    // ── 移動邏輯 ─────────────────────────────────────────────────────
    function moveToCell(tx, ty) {
        var dx  = tx - mapData.playerX;
        var dy  = ty - mapData.playerY;
        var dir = deltaToDir(dx, dy);
        if (!dir) return;
        var hasExit = mapData.exits.some(function (e) { return e.direction === dir; });
        if (!hasExit) {
            if (window.appendLog) window.appendLog('此方向無出口（尚未探索）');
            return;
        }
        if (window.gameSend) window.gameSend({ type: 'move', direction: dir });
    }

    // gate 掉：移動輸入由物件欄出口卡片承接（保留函式供未來恢復）
    function bindMapClick() {}
    function bindMapDrag()  {}

    // ── 緩存最近一次 grid_view msg ───────────────────────────────────
    var _lastGridMsg = null;

    // ── 主入口：grid_view 消息接收 ───────────────────────────────────
    window.onGridView = function (msg) {
        _lastGridMsg = msg;
        mapData.playerX  = msg.player_x;
        mapData.playerY  = msg.player_y;
        mapData.cells    = msg.cells    || [];
        mapData.exits    = msg.exits    || [];
        mapData.entities = msg.entities || [];
        mapData.objects  = msg.objects  || [];

        // 診斷 log
        try {
            var kindCount = {};
            for (var di = 0; di < mapData.entities.length; di++) {
                var k = mapData.entities[di].kind || '?';
                kindCount[k] = (kindCount[k] || 0) + 1;
            }
            console.log('[grid_view] entities=', mapData.entities.length,
                        'kindCount=', JSON.stringify(kindCount),
                        'exits=', mapData.exits.length,
                        'cells=', mapData.cells.length);
        } catch (e) {}

        // 同步揭露快取
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            if (c.explored) revealedCells.add(c.x + ',' + c.y);
        }
        revealedCells.add(mapData.playerX + ',' + mapData.playerY);

        // 派發渲染
        window.SwGridRender.renderMap();
        renderMinimap();
        window.SwRoomDesc.render();
        window.SwObjectList.render();

        // 相容舊 game-ui.js mapState
        if (window.mapState) {
            window.mapState.playerX = msg.player_x;
            window.mapState.playerY = msg.player_y;
            window.mapState.moving  = false;
        }
    };

    // 外部觸發：myPlayerId 更新後重渲描述欄 + 物件欄
    window.refreshGridView = function () {
        if (!_lastGridMsg) return;
        window.SwRoomDesc.render();
        window.SwObjectList.render();
    };

    // ── 初始化 ───────────────────────────────────────────────────────
    function init() {
        gridMapEl          = document.getElementById('gmap-grid');
        roomDescEl         = document.getElementById('mud-room-desc');
        objectListEl       = document.getElementById('mud-obj-list');
        minimapContainerEl = document.getElementById('sw-minimap-container');

        if (!gridMapEl) return; // 非 game 頁面，靜默退出

        // 暴露共享命名空間（子模組 runtime 讀取）
        window.SwGrid = {
            // 常數
            CELL_W: CELL_W, CELL_H: CELL_H,
            CELL_GAP_X: CELL_GAP_X, CELL_GAP_Y: CELL_GAP_Y,
            STEP_X: STEP_X, STEP_Y: STEP_Y,
            // 狀態
            mapData: mapData,
            revealedCells: revealedCells,
            TERRAIN_COLOR: TERRAIN_COLOR,
            DIR_DELTA: DIR_DELTA,
            // DOM 引用
            gridMapEl: gridMapEl,
            roomDescEl: roomDescEl,
            objectListEl: objectListEl,
            // 置中狀態（sw-grid-render 寫入）
            get _playerCenterX() { return _playerCenterX; },
            set _playerCenterX(v) { _playerCenterX = v; },
            get _playerCenterY() { return _playerCenterY; },
            set _playerCenterY(v) { _playerCenterY = v; },
            get _dragX() { return _dragX; },
            set _dragX(v) { _dragX = v; },
            get _dragY() { return _dragY; },
            set _dragY(v) { _dragY = v; },
            // 共享方法
            applyGridTransform: applyGridTransform,
            esc: esc,
            hasProperName: hasProperName,
            displayNameOf: displayNameOf,
            colorFor: colorFor
        };

        bindMapClick();
        bindMapDrag();
        window.SwObjectList.bindClick();

        console.log('[grid-map] v0.20.47 initialized | SW-23 kind/category 雙欄渲染');
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();

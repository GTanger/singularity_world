// sw-object-list.js：物件欄渲染 + 地形動作插座 + 點擊事件綁定。v0.20.46
(function () {
    'use strict';

    // 地形 → 可用採集動作（對齊後端 terrain_name_zh() 回傳的中文字串）
    function terrainToActions(terrain) {
        var map = {
            '林地': [{ key: 'gather_wood',  label: '伐木' }, { key: 'gather_herb', label: '採藥' }],
            '疏林': [{ key: 'gather_herb',  label: '採藥' }],
            '密林': [{ key: 'gather_wood',  label: '伐木' }, { key: 'gather_herb', label: '採藥' }],
            '叢林': [{ key: 'gather_wood',  label: '伐木' }, { key: 'gather_herb', label: '採藥' }],
            '草原': [{ key: 'gather_herb',  label: '採集' }],
            '丘陵': [{ key: 'gather_ore',   label: '採礦' }],
            '山嶺': [{ key: 'gather_ore',   label: '採礦' }],
            '沼地': [{ key: 'gather_water', label: '取水' }],
            '水域': [{ key: 'gather_water', label: '取水' }],
            '荒漠': [],
            '凍原': [],
            '平地': []
        };
        return map[terrain] || [];
    }

    /**
     * 物件欄渲染。
     *   1. 探索按鈕（未探索才顯示）
     *   2. 出口卡片（出口已揭露才顯示）
     *   3. 有名實體：單卡
     *   4. 無名實體：集體壓縮為「眾·N」
     *   5. 資源動作（已探索才顯示）
     */
    function render() {
        var G = window.SwGrid;
        if (!G || !G.objectListEl) return;

        var mapData       = G.mapData;
        var esc           = G.esc;
        var hasProperName = G.hasProperName;
        var displayNameOf = G.displayNameOf;

        var myId    = window.myPlayerId || '';
        var px      = mapData.playerX;
        var py      = mapData.playerY;
        var hasExits = mapData.exits && mapData.exits.length > 0;
        var html    = '';

        // 探索按鈕（若出口未揭露才顯示）
        if (!hasExits) {
            html += '<div class="mud-obj-item mud-obj-action" data-action="explore">探索</div>';
        }

        // 出口卡片（承接移動輸入）
        if (hasExits) {
            for (var xi = 0; xi < mapData.exits.length; xi++) {
                var ex = mapData.exits[xi];
                html += '<div class="mud-obj-item mud-obj-exit" data-move-dir="' + esc(ex.direction) + '">';
                html += '往 ' + esc(ex.direction);
                if (ex.to_room_name) {
                    html += '<div class="mud-npc-behavior">' + esc(ex.to_room_name) + '</div>';
                }
                html += '</div>';
            }
        }

        // 「自己」不在物件欄（姓名欄已顯示）

        // 在場實體分類：有正式名 vs 無名（raw id）
        var others  = (mapData.entities || []).filter(function (e) { return e.id !== myId; });
        var named   = [];
        var nameless = 0;
        for (var ei = 0; ei < others.length; ei++) {
            if (hasProperName(others[ei])) named.push(others[ei]);
            else nameless++;
        }

        for (var ni = 0; ni < named.length; ni++) {
            var ent = named[ni];
            var cls = (ent.kind === 'player' || ent.kind === 'Player') ? 'mud-obj-player' : 'mud-obj-npc';
            html += '<div class="mud-obj-item ' + cls + '" data-entity-id="' + esc(ent.id) + '">';
            html += esc(displayNameOf(ent));
            html += '</div>';
            if (ent.behavior_text) {
                html += '<div class="mud-npc-behavior">' + esc(ent.behavior_text) + '</div>';
            }
        }

        if (nameless > 0) {
            html += '<div class="mud-obj-item mud-obj-crowd" data-action="crowd_look" title="此格有 '
                 + nameless + ' 位無名者">';
            html += '眾·' + nameless;
            html += '</div>';
        }

        // 資源動作（已探索才顯示）
        if (hasExits) {
            var currentCell = null;
            for (var ci = 0; ci < mapData.cells.length; ci++) {
                if (mapData.cells[ci].x === px && mapData.cells[ci].y === py) {
                    currentCell = mapData.cells[ci];
                    break;
                }
            }
            if (currentCell) {
                var resourceActions = terrainToActions(currentCell.terrain);
                for (var ri = 0; ri < resourceActions.length; ri++) {
                    html += '<div class="mud-obj-item mud-obj-resource" data-resource="' + esc(resourceActions[ri].key) + '">';
                    html += esc(resourceActions[ri].label);
                    html += '</div>';
                }
            }
        }

        G.objectListEl.innerHTML = html;
    }

    /**
     * 物件欄點擊事件綁定。
     * 需在 init 時呼叫一次。
     */
    function bindClick() {
        var G = window.SwGrid;
        if (!G || !G.objectListEl) return;

        G.objectListEl.addEventListener('click', function (e) {
            var item = e.target.closest('.mud-obj-item');
            if (!item) return;

            // 探索動作
            if (item.getAttribute('data-action') === 'explore') {
                if (window.gameSend) window.gameSend({ type: 'explore' });
                if (window.appendLog) window.appendLog('正在探索當前格…');
                return;
            }

            // 出口移動
            var moveDir = item.getAttribute('data-move-dir');
            if (moveDir && window.gameSend) {
                window.gameSend({ type: 'move', direction: moveDir });
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

            // 資源採集（SW-3 架構，SW-4 實作）
            var resource = item.getAttribute('data-resource');
            if (resource && window.appendLog) {
                window.appendLog('採集動作：' + resource + '（功能待 SW-4 實作）');
            }
        });
    }

    window.SwObjectList = { render: render, bindClick: bindClick };
    console.log('[sw-object-list] v0.20.46 loaded');
})();

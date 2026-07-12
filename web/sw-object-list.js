// sw-object-list.js：v5 L4 物件卡片觀察態渲染 + 甲乙編號 + 點擊事件。v0.24.7
(function () {
    'use strict';

    // 天干編號（同標籤 ≥ 2 才用）
    var TIANGAN = ['甲','乙','丙','丁','戊','己','庚','辛','壬','癸'];

    // ── 甲乙編號邏輯（純前端，不持久化）────────────────────────────
    function assignSuffixes(items) {
        // items: [{ label, ... }]
        // 分桶
        var buckets = {};
        for (var i = 0; i < items.length; i++) {
            var label = items[i].label;
            if (!buckets[label]) buckets[label] = [];
            buckets[label].push(i);
        }
        var result = items.slice();
        for (var lbl in buckets) {
            var indices = buckets[lbl];
            if (indices.length >= 2) {
                for (var j = 0; j < indices.length; j++) {
                    result[indices[j]].suffix = TIANGAN[j % TIANGAN.length] || String(j + 1);
                }
            }
        }
        return result;
    }

    // ── 觀察態 → 顯示標籤 ────────────────────────────────────────────
    function labelOf(obj) {
        var state = obj.observation_state;
        if (!state && obj.name && !obj.tag) {
            return obj.name;
        }
        if (state === 'named') {
            return (obj.tag || '') + (obj.name ? obj.name : '');
        } else if (state === 'identified') {
            return obj.tag || '行人';
        } else {
            // stranger 或未定義
            return obj.tag || '行人';
        }
    }

    // 當前 L4 物件列表（供 applyDelta 更新用）
    var _currentObjects = [];

    function exitItemsFromGrid() {
        var G = window.SwGrid;
        if (!G || !G.mapData) return [];
        var mapData = G.mapData;
        var currentRoomName = String(mapData.currentRoomName || '').trim();
        function isCurrentRoomLabel(label) {
            return currentRoomName && String(label || '').trim() === currentRoomName;
        }
        var out = [];
        var seen = {};
        var seenLabels = {};
        var keys = [
            { key: 'north', dir: '北' },
            { key: 'south', dir: '南' },
            { key: 'east',  dir: '東' },
            { key: 'west',  dir: '西' }
        ];

        if (mapData.exits_v5) {
            for (var i = 0; i < keys.length; i++) {
                var k = keys[i];
                var info = mapData.exits_v5[k.key];
                if (!info || info.explored !== true || info.walkable !== true) continue;
                var label = (G.exitLabel || function (x) { return x && (x.name || x.to_room_name) || '可去處'; })(info);
                if (isCurrentRoomLabel(label)) continue;
                var normLabel = String(label || '').trim();
                if (seenLabels[normLabel]) continue;
                out.push({ dir: k.dir, label: label });
                seen[k.dir] = true;
                seenLabels[normLabel] = true;
            }
        }

        var legacy = mapData.exits || [];
        for (var li = 0; li < legacy.length; li++) {
            var ex = legacy[li];
            if (!ex || !ex.direction || seen[ex.direction]) continue;
            var legacyLabel = ex.to_room_name || ex.name || '可去處';
            if (isCurrentRoomLabel(legacyLabel)) continue;
            var normLegacyLabel = String(legacyLabel || '').trim();
            if (seenLabels[normLegacyLabel]) continue;
            out.push({ dir: ex.direction, label: legacyLabel });
            seen[ex.direction] = true;
            seenLabels[normLegacyLabel] = true;
        }
        return out;
    }

    function exitCardsHtml(escFn, addBreak) {
        var exits = exitItemsFromGrid();
        var html = '';
        for (var i = 0; i < exits.length; i++) {
            html += '<span class="sw-obj-card sw-obj-exit" data-move-dir="' + escFn(exits[i].dir) + '" role="button" tabindex="0">';
            html += '【' + escFn(exits[i].label) + '】';
            html += '</span>';
        }
        if (addBreak && exits.length) {
            html += '<span class="sw-l4-break" aria-hidden="true"></span>';
        }
        return html;
    }

    function objectCardsHtml(objects) {
        var source = objects || [];
        var items = source.map(function (obj) {
            return { obj: obj, label: labelOf(obj), suffix: '' };
        });
        items = assignSuffixes(items);

        var html = '';
        for (var i = 0; i < items.length; i++) {
            var item    = items[i];
            var obj     = item.obj;
            var display = '【' + item.label + (item.suffix || '') + '】';
            var cls     = 'sw-obj-card';
            if (obj.type === 'npc')      cls += ' sw-obj-npc';
            else if (obj.type === 'item') cls += ' sw-obj-item-card';
            else if (obj.type === 'resource') cls += ' sw-obj-resource';
            html += '<span class="' + cls + '" data-obj-id="' + _esc(obj.id) + '" role="button" tabindex="0">' +
                    display + '</span>';
        }
        return html;
    }

    // ── v5 L4 渲染 ───────────────────────────────────────────────────
    function renderV5(objects) {
        _currentObjects = objects || [];
        var el = document.getElementById('sw-l4-objects');
        if (!el) return;

        var html = exitCardsHtml(_esc, true);
        html += objectCardsHtml(_currentObjects);
        el.innerHTML = html;
    }

    // ── applyDelta（world_tick / explore_result）────────────────────
    function applyDelta(delta) {
        if (!delta) return;
        var arrived  = delta.arrived || [];
        var departed = delta.departed || [];

        // 移除離場物件
        for (var ri = 0; ri < departed.length; ri++) {
            var depId = departed[ri];
            _currentObjects = _currentObjects.filter(function (o) { return o.id !== depId; });
        }

        // 加入新進場物件
        for (var ai = 0; ai < arrived.length; ai++) {
            var newObj = arrived[ai];
            var exists = false;
            for (var ei = 0; ei < _currentObjects.length; ei++) {
                if (_currentObjects[ei].id === newObj.id) { exists = true; break; }
            }
            if (!exists) _currentObjects.push(newObj);
        }

        renderV5(_currentObjects);
    }

    // ── applyObservationUpdate（interact_result）────────────────────
    function applyObservationUpdate(update) {
        if (!update || !update.target_id) return;
        for (var i = 0; i < _currentObjects.length; i++) {
            if (_currentObjects[i].id === update.target_id) {
                _currentObjects[i].observation_state = update.new_state;
                if (update.tag) _currentObjects[i].tag = update.tag;
                break;
            }
        }
        renderV5(_currentObjects);
    }

    // ── 舊格式相容渲染（grid_view 事件，無 observation_state）───────
    function render() {
        var G = window.SwGrid;
        if (!G || !G.objectListEl) return;

        var mapData       = G.mapData;
        var escFn         = G.esc || _esc;
        var displayNameOf = G.displayNameOf;

        var myId = window.myPlayerId || '';
        var html = exitCardsHtml(escFn);

        // 在場實體（NPC + 其他玩家）
        // L4 只顯示 NPC，排除自己與其他玩家（玩家 id 污染不進 NPC 卡）
        var others = (mapData.entities || []).filter(function (e) {
            return e.id !== myId && e.kind !== 'player' && e.kind !== 'Player';
        });
        for (var ei = 0; ei < others.length; ei++) {
            var ent = others[ei];
            var cls = (ent.kind === 'player' || ent.kind === 'Player') ? 'sw-obj-player' : 'sw-obj-npc';
            html += '<span class="sw-obj-card ' + cls + '" data-entity-id="' + escFn(ent.id) + '" role="button" tabindex="0">';
            html += '【' + escFn(displayNameOf(ent)) + '】';
            html += '</span>';
        }
        G.objectListEl.innerHTML = html;
    }

    function _esc(s) {
        var d = document.createElement('div');
        d.textContent = String(s || '');
        return d.innerHTML;
    }

    // ── 點擊事件綁定 ─────────────────────────────────────────────────
    function bindClick() {
        // L4 物件卡片區
        var l4El = document.getElementById('sw-l4-objects');
        if (l4El) {
            l4El.addEventListener('click', function (e) {
                var card = e.target.closest('.sw-obj-card');
                if (!card) return;

                var objId   = card.getAttribute('data-obj-id');
                var moveDir = card.getAttribute('data-move-dir');
                var entId   = card.getAttribute('data-entity-id');
                if (moveDir && window.gameSend) {
                    window.gameSend({ type: 'move', direction: moveDir });
                } else if (objId && window.gameSend) {
                    // 送 observe 更新觀察態（後端 set_observed + 重推 room_enter）
                    var cardState = card.getAttribute('data-card-state');
                    if (cardState === 'stranger' || cardState === 'observed') {
                        window.gameSend({ type: 'observe', entity_id: objId });
                    }
                    window.gameSend({ type: 'do_action', entity_id: objId, action: 'Look' });
                } else if (entId && window.gameSend) {
                    window.gameSend({ type: 'do_action', entity_id: entId, action: 'Look' });
                }
            });
        }

        // 舊 objectListEl 相容
        var G = window.SwGrid;
        if (G && G.objectListEl && G.objectListEl !== l4El) {
            G.objectListEl.addEventListener('click', function (e) {
                var card = e.target.closest('.sw-obj-card');
                if (!card) return;
                var moveDir = card.getAttribute('data-move-dir');
                var entId   = card.getAttribute('data-entity-id');
                if (moveDir && window.gameSend) window.gameSend({ type: 'move', direction: moveDir });
                else if (entId && window.gameSend) window.gameSend({ type: 'do_action', entity_id: entId, action: 'Look' });
            });
        }
    }

    // ── renderV5NpcCards（後端已算好 display_name + card_state）─────
    // npcs: [{ npc_id, display_name, card_state: "stranger"|"observed"|"named" }]
    function renderV5NpcCards(npcs, objects) {
        var el = document.getElementById('sw-l4-objects');
        if (!el) return;
        var objectList = objects || [];
        if (!npcs || !npcs.length) {
            el.innerHTML = exitCardsHtml(_esc, true) + objectCardsHtml(objectList);
            return;
        }
        var html = exitCardsHtml(_esc, true);
        for (var i = 0; i < npcs.length; i++) {
            var npc = npcs[i];
            var state = npc.card_state || 'stranger';
            var kind = npc.kind || 'npc';
            var cls = 'sw-obj-card ' + (kind === 'crowd' ? 'sw-obj-crowd' : 'sw-obj-npc') + ' card-' + state;
            var idAttr = kind === 'crowd' ? '' : ' data-obj-id="' + _esc(npc.npc_id) + '"';
            var displayName = npc.display_name || (kind === 'crowd' ? '人群' : '');
            if (kind === 'crowd') displayName = displayName.replace(/（還有\d+人）/g, '') || '人群';
            html += '<span class="' + cls + '"' + idAttr + ' data-card-state="' + _esc(state) + '" role="button" tabindex="0">';
            html += '【' + _esc(displayName) + '】';
            html += '</span>';
        }
        html += objectCardsHtml(objectList);
        el.innerHTML = html;
    }

    window.SwObjectList = { render: render, renderV5: renderV5, renderV5NpcCards: renderV5NpcCards, applyDelta: applyDelta, applyObservationUpdate: applyObservationUpdate, bindClick: bindClick };
    console.log('[sw-object-list] v0.24.8 loaded');
})();

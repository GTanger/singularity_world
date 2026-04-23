// sw-room-desc.js：房間描述渲染（地形名 + 時段 + 氛圍句 + 在場 NPC）。v0.20.46
(function () {
    'use strict';

    // ── 地形氛圍句快取（從 /data/config/terrain_ambience.json 非同步載入）────
    var terrainAmbience = null;
    fetch('/data/config/terrain_ambience.json')
        .then(function (r) { return r.json(); })
        .then(function (j) { terrainAmbience = j.terrains || {}; })
        .catch(function (e) { console.warn('[sw-room-desc] terrain_ambience 載入失敗:', e); });

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

    /**
     * 渲染房間描述欄（#mud-room-desc）。
     * 依賴 window.SwGrid.mapData、window.gameState、window.myPlayerId。
     */
    function render() {
        var G = window.SwGrid;
        if (!G || !G.roomDescEl) return;

        var mapData  = G.mapData;
        var esc      = G.esc;
        var displayNameOf = G.displayNameOf;

        var currentCell = null;
        for (var i = 0; i < mapData.cells.length; i++) {
            if (mapData.cells[i].x === mapData.playerX && mapData.cells[i].y === mapData.playerY) {
                currentCell = mapData.cells[i];
                break;
            }
        }
        var terrainName = (currentCell && currentCell.name) ? currentCell.name : '未知地形';
        var terrainKey  = (currentCell && currentCell.terrain) ? currentCell.terrain : '';

        var html = '<div class="mud-room-name">【' + esc(terrainName) + '】</div>';

        // 時段與氛圍句
        var hour  = 0;
        var phase = '';
        if (window.gameState) {
            var st = window.gameState();
            if (st && typeof st.game_time_sec_since_midnight === 'number') {
                hour  = Math.floor(st.game_time_sec_since_midnight / 3600) % 24;
                phase = PHASE_LABELS[Math.floor(hour / 2)] || '';
            }
        }
        if (phase) html += '<div class="mud-room-phase">' + esc(phase) + '</div>';

        var ambience = getAmbience(terrainKey, hour);
        if (ambience) {
            html += '<div class="mud-room-ambience">' + esc(ambience) + '</div>';
        }

        // 出口方向已用地圖短腳視覺化，描述欄不再列方向字

        // 在場者：描述欄只列 NPC（玩家身份由姓名欄承擔）
        var npcsHere = (mapData.entities || []).filter(function (e) {
            return e.kind === 'npc' || e.kind === 'Npc';
        });
        if (npcsHere.length > 0) {
            html += '<div class="mud-room-entities">在場：';
            html += npcsHere.map(function (e) {
                return '<span class="mud-entity-name">' + esc(displayNameOf(e)) + '</span>';
            }).join('、');
            html += '</div>';
        }

        G.roomDescEl.innerHTML = html;
    }

    window.SwRoomDesc = { render: render, getAmbience: getAmbience };
    console.log('[sw-room-desc] v0.20.46 loaded');
})();

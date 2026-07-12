/**
 * grid-map.js v0.24.6  — SW-v5 重構
 * 舊職責（方格地圖渲染 + minimap）已移除。
 * 新職責：維護當前格資料 + 共享狀態命名空間 SwGrid。
 *
 * 子模組：sw-room-desc.js / sw-object-list.js（仍掛在 SwGrid 命名空間）
 */
(function () {
    'use strict';

    // ── 內部方向 ←→ delta（後端通訊用；玩家層不直接顯示方向名）──────
    var DIR_DELTA = {
        '北': { dx: 0, dy:  1 },
        '東': { dx: 1, dy:  0 },
        '南': { dx: 0, dy: -1 },
        '西': { dx: -1, dy: 0 }
    };

    // ── 揭露快取（永久紀錄）────────────────────────────────────────
    var revealedCells = new Set();

    // 舊 game-ui.js 曾建立 window.mapState；現由資料層保留移動狀態。
    var mapState = window.mapState || { moving: false, playerX: 0, playerY: 0 };
    window.mapState = mapState;

    // ── 目前地圖狀態 ─────────────────────────────────────────────────
    var mapData = {
        playerX: 0,
        playerY: 0,
        cells:    [],
        exits:    [],   // ExitView[] — 舊格式：{ direction, to_room_name, explored? }
        entities: [],
        objects:  [],
        npcs:     [],
        currentRoomId: '',
        currentRoomName: '',
        exitsRoomName: '',
        // v5 新：四方鄰格資訊（room_enter 格式）
        exits_v5: null  // { north, south, east, west } 每格含 explored/walkable/name
    };

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

    function exitLabel(info) {
        if (!info) return '可去處';
        return info.display_name || info.label || info.to_room_name || info.room_name || info.name || '可去處';
    }

    // ── 緩存最近一次 grid_view msg ───────────────────────────────────
    var _lastGridMsg = null;

    // ── 主入口：grid_view 消息接收（舊格式，相容）───────────────────
    window.onGridView = function (msg) {
        _lastGridMsg = msg;
        mapData.playerX  = msg.player_x;
        mapData.playerY  = msg.player_y;
        mapData.cells    = msg.cells    || [];
        mapData.exits    = msg.exits    || [];
        mapData.entities = msg.entities || [];
        mapData.objects  = msg.objects  || [];
        mapData.currentRoomName = msg.room_name || mapData.currentRoomName || '';
        mapData.exitsRoomName = msg.room_name || '';

        // 同步揭露快取
        for (var i = 0; i < mapData.cells.length; i++) {
            var c = mapData.cells[i];
            if (c.explored) revealedCells.add(c.x + ',' + c.y);
        }
        revealedCells.add(mapData.playerX + ',' + mapData.playerY);

        // grid_view 只保留資料同步。L1-L4 的玩家層呈現由 room_enter
        // 權威 payload 負責，避免舊相容路徑用 entities.display_name
        // 覆蓋掉觀察態 NPC 卡片並洩漏真名。
        if (!mapData.exits_v5) {
            window.SwRoomDesc.render();
            window.SwObjectList.render();
        } else if (Array.isArray(mapData.npcs)) {
            window.SwObjectList.renderV5NpcCards(mapData.npcs, mapData.objects || []);
        } else {
            window.SwObjectList.renderV5(mapData.objects || []);
        }

        mapState.playerX = msg.player_x;
        mapState.playerY = msg.player_y;
        mapState.moving  = false;
    };

    // ── v5 room_enter 事件處理 ───────────────────────────────────────
    window.onRoomEnter = function (msg) {
        var nextRoomId = msg.room_id || '';
        var sameRoom = !!nextRoomId && mapData.currentRoomId === nextRoomId;
        if (!sameRoom) {
            mapData.exits = [];
            mapData.exitsRoomName = '';
        }
        // 更新 exits_v5
        if (msg.exits) {
            mapData.exits_v5 = msg.exits;
        }
        if (nextRoomId) mapData.currentRoomId = nextRoomId;
        mapData.currentRoomName = (msg.terrain && msg.terrain.name) || mapData.currentRoomName || '';
        // 更新 objects / entities
        if (msg.objects) mapData.objects = msg.objects;
        mapData.npcs = Array.isArray(msg.npcs) ? msg.npcs : null;
        // 渲染 L1-L3 + L4
        window.SwRoomDesc.renderV5(msg, { keepL3: sameRoom });
        // v5 acquaintance：後端帶 npcs 陣列時直接渲染，否則 fallback 舊格式
        if (msg.npcs && Array.isArray(msg.npcs)) {
            window.SwObjectList.renderV5NpcCards(msg.npcs, msg.objects || []);
        } else {
            window.SwObjectList.renderV5(msg.objects || []);
        }
    };

    // ── v5 explore_result 事件 ───────────────────────────────────────
    window.onExploreResult = function (msg) {
        // log 追加
        if (msg.log && window.SwLog && window.SwLog.appendLog) {
            window.SwLog.appendLog(msg.log);
        }
        // objects_delta 更新 L4
        if (msg.objects_delta) {
            window.SwObjectList.applyDelta(msg.objects_delta);
        }
    };

    // ── v5 interact_result 事件 ─────────────────────────────────────
    window.onInteractResult = function (msg) {
        if (msg.log && window.SwLog && window.SwLog.appendLog) {
            window.SwLog.appendLog(msg.log);
        }
        if (msg.observation_update) {
            window.SwObjectList.applyObservationUpdate(msg.observation_update);
        }
    };

    // ── v5 world_tick 事件（不污染 log）──────────────────────────────
    // 四段時辰：dawn(5-10) / day(11-16) / dusk(17-19) / night(20-4)
    // 日兩組（dawn/day）夜兩組（dusk/night），每個天氣 icon 各備四池
    var WEATHER_PROSE_WT = {
        clear: {
            dawn:  ['晨光初現，霧氣將散未散。', '曙色微明，露珠在草尖閃爍。', '東方泛白，群山輪廓漸顯。'],
            day:   ['天色晴朗，遠山清晰可辨。', '陽光透過枝葉灑落，光影斑駁。', '無雲，天藍如洗。'],
            dusk:  ['暮色染紅西天，遠山轉為暗影。', '夕陽西斜，光線轉為金黃。', '殘陽掛樹梢，影子拉得修長。'],
            night: ['夜空澄澈，繁星點點。', '月色清冷，遠山如剪影。', '夜風微涼，無雲遮月。']
        },
        rain: {
            dawn:  ['晨雨清涼，草尖懸著水珠。', '微雨初降，空氣裡盡是泥土腥味。', '雨絲在曙色中若有若無。'],
            day:   ['細雨打濕草徑，土腥瀰漫。', '雨聲淅瀝，霧氣漸起。', '連日陰雨，道路泥濘。'],
            dusk:  ['暮雨蕭蕭，天色提早暗下。', '黃昏雨霧交織，視線朦朧。', '雨打殘陽，光線渾濁。'],
            night: ['夜雨敲窗，聽之久不寐。', '雨聲在黑暗裡更顯清晰。', '夜雨綿綿，寒意透骨。']
        },
        snow: {
            dawn:  ['晨雪初積，腳印是新的。', '曙色映雪，四野泛著冷藍。', '清晨薄雪，呼吸即見白霧。'],
            day:   ['薄雪覆地，腳步聲悶而清冽。', '雪花紛落，四野靜謐。', '寒氣刺骨，呼出白霧。'],
            dusk:  ['暮雪沉沉，天地一色昏黃。', '黃昏飄雪，遠處輪廓淡去。', '夕照雪原，光線冷白。'],
            night: ['夜雪無聲，天地俱白。', '月光照雪，亮如薄晝。', '夜半飛雪，寒風刮面。']
        },
        wind: {
            dawn:  ['晨風帶露，吹得人精神。', '曉風吹過草尖，露珠四濺。', '清晨勁風，遠處旗幟招展。'],
            day:   ['勁風拂過，草浪翻滾。', '樹梢搖曳，落葉旋飛。', '風聲呼嘯，衣角獵獵。'],
            dusk:  ['暮風轉涼，吹得殘陽搖晃。', '黃昏風起，落葉打著旋落下。', '夕陽下風聲嗚咽。'],
            night: ['夜風刺骨，星光也跟著抖。', '夜風穿林，聲如鬼哭。', '寒夜風急，難以立足。']
        },
        plum_rain: {
            dawn:  ['晨梅雨綿，霧氣未散。', '清晨濕氣濃重，衣物盡黏。', '梅雨晨色昏沉，光線難透。'],
            day:   ['梅雨綿延，衣物難乾。', '細雨不歇，霧氣包裹遠處輪廓。', '黴潮之氣，入骨沁涼。'],
            dusk:  ['暮色裡梅雨更顯陰冷。', '黃昏濕霧瀰漫，看不清遠處。', '夕陽被梅雨吞沒，天色提早暗。'],
            night: ['夜雨潮濕，被褥皆冷。', '梅雨夜深，黴味更重。', '夜霧夾雨，難辨方向。']
        },
        fog: {
            dawn:  ['晨霧最濃，數步外已不可見。', '曉霧未開，世界只剩眼前一塊。', '清晨霧鎖山林，雞犬聲遙。'],
            day:   ['濃霧籠罩，三步外已是模糊。', '霧中行路，方向難辨。', '薄霧遊蕩，視線受阻。'],
            dusk:  ['暮霧漸起，天色提早入夜。', '黃昏霧合，殘陽僅剩一團暈黃。', '夕霧低垂，看不清來時路。'],
            night: ['夜霧瀰漫，伸手不見五指。', '黑霧深沉，腳步聲被吞沒。', '夜半霧鎖，世界寂靜如棺。']
        },
        storm: {
            dawn:  ['晨起天即陰沉，雷在遠處低吼。', '曙色未明，烏雲已壓頭頂。', '清晨暴雨將至，鳥雀盡藏。'],
            day:   ['雷聲轟隆，閃電劈裂天際。', '暴雨傾盆，溪流急漲。', '狂風暴雨，難以立足。'],
            dusk:  ['暮色被雷雨吞沒，天地皆暗。', '黃昏雷雨大作，光線僅剩閃電一瞬。', '夕陽湮滅在風暴裡。'],
            night: ['夜半雷雨，天地搖晃。', '黑夜閃電映天，照得片刻如晝。', '深夜暴風雨，門窗皆顫。']
        }
    };
    function _phaseOfHourWT(h) {
        if (h >= 5 && h < 11) return 'dawn';
        if (h >= 11 && h < 17) return 'day';
        if (h >= 17 && h < 20) return 'dusk';
        return 'night';
    }
    function _climatePoolWT(iconKey, hour) {
        var phase = _phaseOfHourWT(hour);
        var bucket = WEATHER_PROSE_WT[iconKey] || WEATHER_PROSE_WT.clear;
        return bucket[phase] || bucket.day || [];
    }
    window.onWorldTick = function (msg) {
        // 更新 L1（時間）
        if (msg.time) window.SwRoomDesc.renderL1(msg.time);
        // 更新 L2（氣候，若有變化）—— 轉換後端 event_type+prose_seed 格式
        if (msg.weather) {
            var wt = msg.weather;
            var iconKey = wt.event_type || wt.icon_key || 'clear';
            var desc = wt.desc;
            var hour = 12;
            if (msg.time && msg.time.hour_str) {
                hour = parseInt(String(msg.time.hour_str).split(':')[0], 10) || 12;
            }
            if (!desc) {
                var pool = _climatePoolWT(iconKey, hour);
                var seed = Math.abs(wt.prose_seed || 0);
                desc = pool[seed % pool.length];
            }
            window.SwRoomDesc.renderL2({ icon_key: iconKey, desc: desc, hour: hour });
        }
        // 更新 L4（objects_delta）
        if (msg.objects_delta) window.SwObjectList.applyDelta(msg.objects_delta);
    };


    // ── v5 scout_result 事件（出口槽揭露回應）──────────────────────
    window.onScoutResult = function (msg) {
        if (!msg.direction || !msg.explored) return;
        // 更新 exits_v5 該方向資訊（前端快取）
        if (!mapData.exits_v5) mapData.exits_v5 = {};
        var key = { '北': 'north', '南': 'south', '東': 'east', '西': 'west' }[msg.direction];
        if (key) {
            mapData.exits_v5[key] = {
                explored: msg.explored,
                walkable: msg.walkable,
                name: msg.name || '可去處'
            };
        }
        if (window.SwObjectList && window.SwObjectList.render) {
            window.SwObjectList.render();
        }
    };

    // 外部觸發：myPlayerId 更新後重渲描述欄 + 物件欄
    window.refreshGridView = function () {
        // myPlayerId 到齊後不再重跑舊 grid_view render；否則 room_enter
        // 的 L4 觀察態會被 grid_view.entities 真名列表覆蓋。
        if (!_lastGridMsg || mapData.exits_v5) return;
        window.SwRoomDesc.render();
        window.SwObjectList.render();
    };

    // ── 初始化 ───────────────────────────────────────────────────────
    function init() {
        // 暴露共享命名空間（子模組 runtime 讀取）
        window.SwGrid = {
            mapData: mapData,
            revealedCells: revealedCells,
            DIR_DELTA: DIR_DELTA,
            exitLabel: exitLabel,
            // DOM 引用（子模組相容用）
            roomDescEl:   document.getElementById('sw-l3-terrain'),
            objectListEl: document.getElementById('sw-l4-objects'),
            esc: esc,
            hasProperName: hasProperName,
            displayNameOf: displayNameOf
        };

        window.SwObjectList.bindClick();

        console.log('[grid-map] v0.24.6 initialized | exits render in L4');
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', init);
    } else {
        init();
    }
})();

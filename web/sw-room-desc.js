// sw-room-desc.js：地名主層、局部定位、時間／氣候與場景事實。v0.27.1
(function () {
    'use strict';

    // ── 地形氛圍句快取（從 /data/config/terrain_ambience.json 非同步載入）────
    var terrainAmbience = null;
    fetch('/data/config/terrain_ambience.json')
        .then(function (r) { return r.json(); })
        .then(function (j) { terrainAmbience = j.terrains || {}; })
        .catch(function (e) { console.warn('[sw-room-desc] terrain_ambience 載入失敗:', e); });

    // 12 個時段：每 2 小時一段
    var PHASE_LABELS = ['深夜','凌晨','破曉','清晨','上午','正午','午後','下午','傍晚','入夜','晚間','半夜'];

    // 氣候圖標映射；clear 需看時辰，夜間不能顯示太陽。
    var WEATHER_ICONS = { rain: '☁', snow: '❄', wind: '🌬', fog: '🌫', storm: '⛈', plum_rain: '🌧' };

    // ── L2 氣候散文池（placeholder；後端有真資料後可擴充）──────────
    // 四段時辰需與畫面上每兩小時一段的時段標籤一致：
    // dawn(4-7) / day(8-15) / dusk(16-19) / night(20-3)。
    // 日兩組（dawn/day）夜兩組（dusk/night），每個天氣 icon 各備四池
    var WEATHER_PROSE = {
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
    function phaseOfHour(h) {
        if (h >= 4 && h < 8) return 'dawn';
        if (h >= 8 && h < 16) return 'day';
        if (h >= 16 && h < 20) return 'dusk';
        return 'night';
    }
    function climatePool(iconKey, hour) {
        var bucket = WEATHER_PROSE[iconKey] || WEATHER_PROSE.clear;
        var phase = phaseOfHour(hour);
        return bucket[phase] || bucket.day || [];
    }

    function weatherIcon(iconKey, hour) {
        if (iconKey === 'clear') {
            return phaseOfHour(hour) === 'night' ? '☾' : '☀';
        }
        return WEATHER_ICONS[iconKey] || '☁';
    }

    // ── L3 地形進格散文池（placeholder；TODO: SW-v5 後端推 room_enter 後改用真資料）──
    var _FOREST_PROSE = ['松針鋪地，林徑幽深，光影稀疏。', '林木交錯，鳥鳴斷續，草氣清涼。', '濃蔭遮天，腳踩枯葉，聲音窸窣。', '樹根盤結，苔蘚攀石，空氣濕潤。'];
    var _PLAIN_PROSE  = ['曠野無垠，草浪隨風起伏。', '平野空曠，視野開闊，無處藏身。', '乾草搖擺，遠處地平線模糊。', '開闊地帶，行路省力，卻毫無遮蔽。'];
    var _HILL_PROSE   = ['緩坡延伸，視野漸開。', '丘陵起伏，草木稀疏，土色赭黃。', '坡頂風大，遠處輪廓清晰。'];
    var _WATER_PROSE  = ['水聲潺潺，石礫在流中若隱若現。', '河岸泥濘，蘆葦叢生。', '水流清涼，波光粼粼。', '溼地之上，霧氣低迷，踩之陷足。'];
    var TERRAIN_PROSE = {
        forest:       _FOREST_PROSE,
        forest_heavy: ['古木參天，林蔭蔽日，光線幾乎不透。', '密林深處，蟲鳴如潮，落葉厚積。'],
        forest_light: ['疏林散落，光影破碎，野草蔓延其間。', '林木稀疏，視線穿過枝椏望見遠山。'],
        mountain: ['山路陡峭，亂石嶙峋，風聲尖銳。', '高處清冷，遠眺雲霧繚繞。', '崖壁嶙峋，攀行需用雙手。', '山風帶走體溫，四肢漸沉。'],
        plain:    _PLAIN_PROSE,
        grassland:['草色連天，風過如浪，腳步輕快。', '原野開闊，牧草齊腰，遠山若隱若現。', '晴空下草原伸展，視野無盡。'],
        water:    _WATER_PROSE,
        water_deep: ['深水湧動，暗流難測，岸邊溼滑。', '水色深邃，倒影模糊，濕氣瀰漫四周。'],
        hill:     _HILL_PROSE,
        hills:    _HILL_PROSE,
        swamp:    ['沼澤泥腳，每步都費力氣。', '腐葉氣息濃重，蚊蟲環繞。', '水面暗沉，不知深淺。'],
        desert:   ['黃沙細碎，風吹過後留下痕跡。', '熱氣蒸騰，視野中有幾分扭曲。', '沙地上偶見枯骨，無人問津。'],
        tundra:   ['凍土龜裂，寸草不生。', '寒風割臉，腳下冰層脆響。', '荒原空曠，天地間只剩蒼茫。'],
        jungle:   ['密林潮熱，藤蔓纏繞，蟲鳥聲震耳欲聾。', '熱帶叢林，每步皆要劈開纏繞的枝葉。']
    };

    // 用 prose_seed 從池中穩定取一條
    function pickProse(pool, seed) {
        if (!pool || !pool.length) return '';
        var idx = Math.abs(seed || 0) % pool.length;
        return pool[idx];
    }

    // L2 輪詢狀態
    var _climateTimer = null;
    var _lastRegionId = 'default';
    var _proseSeedCache = 0;
    var _currentHour = 12; // 由 renderL1 寫入；fetchAndRenderL2 用之決定 clear 走日/夜池
    var _v5ControlsL2 = false;

    function fetchAndRenderL2(regionId) {
        regionId = regionId || _lastRegionId || 'default';
        _lastRegionId = regionId;
        fetch('/api/climate?region_id=' + encodeURIComponent(regionId))
            .then(function (r) { return r.json(); })
            .then(function (data) {
                if (_v5ControlsL2) return;
                var iconKey = data.event_type || 'clear';
                var pool = climatePool(iconKey, _currentHour);
                _proseSeedCache = data.prose_seed || 0;
                renderL2({ icon_key: iconKey, desc: pickProse(pool, _proseSeedCache), hour: _currentHour });
            })
            .catch(function (e) { console.warn('[sw-room-desc] L2 climate fetch 失敗:', e); });
    }

    function startClimatePolling(regionId) {
        if (_v5ControlsL2) return;
        if (_climateTimer) clearInterval(_climateTimer);
        fetchAndRenderL2(regionId);
        _climateTimer = setInterval(function () { fetchAndRenderL2(_lastRegionId); }, 30000);
    }

    function getAmbience(terrain, hour) {
        if (!terrainAmbience || !terrain) return '';
        var phaseLabel = PHASE_LABELS[Math.floor(hour / 2)] || '';
        var terrainMap = terrainAmbience[terrain];
        if (!terrainMap) return '';
        return terrainMap[phaseLabel] || '';
    }

    function esc(s) {
        var d = document.createElement('div');
        d.textContent = String(s || '');
        return d.innerHTML;
    }

    function cleanSceneDesc(text, terrainName) {
        var raw = String(text || '').trim();
        var terrain = String(terrainName || '').trim();
        var oldNorthBankName = /北岸(?:等候處|候渡處|後渡處)/g;
        raw = raw.replace(oldNorthBankName, '北岸渡口');
        terrain = terrain.replace(oldNorthBankName, '北岸渡口');
        var leaks = [
            '目前登入後的起點',
            '登入後的暫存起點',
            '世界地圖尚未接入',
            '尚未寫入場景描述'
        ];
        var leaked = !raw;
        for (var i = 0; i < leaks.length; i++) {
            if (raw.indexOf(leaks[i]) !== -1) {
                leaked = true;
                break;
            }
        }
        if (!leaked) return raw;
        return terrain ? terrain + '。' : '';
    }

    // 第一屏只保留能辨認此地的短句；放寬上限至 340 字元、最多 5 句以容納環境器物描述。
    function compactSceneDesc(text) {
        var raw = String(text || '').replace(/\s+/g, ' ').trim();
        if (!raw) return '';
        var sentences = raw.match(/[^。！？!?]+[。！？!?]?/g) || [raw];
        var chosen = [];
        var length = 0;
        for (var i = 0; i < sentences.length && chosen.length < 5; i++) {
            var sentence = sentences[i].trim();
            if (!sentence) continue;
            if (chosen.length && length + sentence.length > 340) break;
            chosen.push(sentence);
            length += sentence.length;
        }
        return (chosen.join('') || raw).trim();
    }

    // ── 取 DOM ──────────────────────────────────────────────────────
    function l1El() { return document.getElementById('sw-l1-time'); }
    function l2El() { return document.getElementById('sw-l2-climate'); }
    function l3El() { return document.getElementById('sw-l3-terrain'); }
    function placeEl() { return document.getElementById('sw-place-name'); }
    function locatorEl() { return document.getElementById('sw-place-locator'); }

    function splitFullPlaceName(value) {
        var raw = String(value || '').trim();
        var match = raw.match(/^(.+?)（(.+)）$/);
        return match ? { title: match[1].trim(), locator: match[2].trim() } : { title: raw, locator: '' };
    }

    function renderPlace(placeData, terrainData) {
        var el = placeEl();
        if (!el) return;
        var rawName = (placeData && (placeData.full_name || placeData.name)) ||
            (terrainData && terrainData.name) || '';
        var parts = splitFullPlaceName(rawName);
        var explicitParts = splitFullPlaceName(placeData && placeData.name);
        var title = explicitParts.title || parts.title;
        var locator = (placeData && placeData.locator) || explicitParts.locator || parts.locator;
        el.textContent = String(title || '').trim();
        el.hidden = !el.textContent;
        var locatorNode = locatorEl();
        if (locatorNode) {
            locatorNode.textContent = String(locator || '').trim();
            locatorNode.hidden = !locatorNode.textContent;
        }
    }

    // ── L1 時間渲染（簡潔時辰與時間，不畫日晷 SVG）──────────────────
    function renderL1(timeData) {
        var el = l1El();
        if (!el) return;
        if (!timeData) {
            el.innerHTML = '';
            return;
        }
        var hourStr = timeData.hour_str || '--:--';
        var hour = parseInt(hourStr.split(':')[0], 10) || 0;
        _currentHour = hour; // 同步給 L2 climate 池選擇用
        var phase = PHASE_LABELS[Math.floor(hour / 2)] || '';

        el.innerHTML =
            '<span class="desc-text">' + esc(phase) + '</span>' +
            '<span class="desc-clock">' + esc(hourStr) + '</span>';
    }

    // ── L2 氣候渲染 ──────────────────────────────────────────────────
    function renderL2(weatherData) {
        var el = l2El();
        if (!el) return;
        if (!weatherData) {
            el.innerHTML = '';
            return;
        }
        var hour = (typeof weatherData.hour === 'number') ? weatherData.hour : _currentHour;
        var icon = weatherIcon(weatherData.icon_key, hour);
        var desc = weatherData.desc || '';
        el.innerHTML =
            '<span class="desc-icon">' + icon + '</span>' +
            '<span class="desc-text">' + esc(desc) + '</span>';
    }

    // ── 地形 key → icon（L3 + render() 共用）────────────────────────
    var TERRAIN_ICONS = {
        forest: '🌲', forest_heavy: '🌲', forest_light: '🌲',
        mountain: '⛰',
        plain: '🌾', grassland: '🌾',
        water: '🌊', water_deep: '🌊',
        hill: '🗻', hills: '🗻',
        swamp: '🌿',
        desert: '🏜',
        tundra: '❄',
        jungle: '🌴',
        urban: '🏘',
        road: '道',
        bridge: '橋',
        wall: '牆',
        farm_field: '田',
        farmhouse: '舍',
        inn: '宿',
        tavern: '酒',
        blacksmith: '鐵',
        general_store: '店',
        clinic: '藥',
        workshop: '工',
        market: '市',
        guild_hall: '會',
        temple: '廟',
        academy: '學',
        library: '書',
        barracks: '營',
        guard_post: '哨',
        warehouse: '倉',
        granary: '廩',
        dock: '渡',
        bathhouse: '湯',
        courthouse: '律',
        jail: '牢',
        town_hall: '政',
        bank: '銀',
        mint: '鑄',
        stables: '馬',
        caravanserai: '驛',
        theater: '戲',
        arena: '武',
        observatory: '星',
        alchemist: '煉',
        mage_tower: '塔',
        embassy: '館',
        prison_yard: '囚'
    };

    // 判斷物件是否為「可離開當前格」或人物（非靜態器物）
    function isMovableOrPortableObject(obj) {
        if (!obj) return false;
        if (obj.type === 'npc' || obj.kind === 'npc' || obj.kind === 'Npc') return true;
        if (obj.is_ground_item === true || obj.kind === 'ground_item') return true;
        if (obj.move_to_room_id) return true;
        var actions = obj.actions || obj.sockets || [];
        if (Array.isArray(actions)) {
            for (var i = 0; i < actions.length; i++) {
                var a = actions[i];
                if (a === 'Move' || a === 'Take') return true;
            }
        }
        return false;
    }

    // ── L3 地形與內嵌靜態器物渲染 ─────────────────────────────────────
    function renderL3(terrainData, descText) {
        var el = l3El();
        if (!el) return;
        var text = compactSceneDesc(cleanSceneDesc(descText, terrainData && terrainData.name));
        el.innerHTML = text ? '<span class="desc-text">' + esc(text) + '</span>' : '';
    }

    // ── v5 room_enter 全渲染（L1+L2+L3）────────────────────────────
    function renderV5(msg, options) {
        options = options || {};
        _v5ControlsL2 = true;
        renderPlace(msg.place, msg.terrain);
        if (msg.time)    renderL1(msg.time);
        if (msg.weather) {
            var weather = Object.assign({}, msg.weather);
            if (msg.time && typeof msg.time.hour === 'number') weather.hour = msg.time.hour;
            renderL2(weather);
        } else {
            var fallbackHour = (msg.time && typeof msg.time.hour === 'number') ? msg.time.hour : 12;
            var fallbackPool = climatePool('clear', fallbackHour);
            var fallbackSeed = (msg.room_id || '').split('').reduce(function(a, c) { return a + c.charCodeAt(0); }, fallbackHour);
            renderL2({ icon_key: 'clear', desc: pickProse(fallbackPool, fallbackSeed), hour: fallbackHour });
        }

        var l3Node = l3El();
        var currentL3Text = (l3Node && l3Node.textContent || '').trim();
        if (options.keepL3 && currentL3Text) {
            return;
        }

        var descText = cleanSceneDesc(msg.desc || '', msg.terrain && msg.terrain.name);
        renderL3(msg.terrain, descText, msg.objects, msg.harvestables);
    }

    // ── 舊格式相容（grid_view 事件）──────────────────────────────────
    function render() {
        var G = window.SwGrid;
        if (!G) return;

        var mapData  = G.mapData;
        renderPlace(null, { name: mapData.currentRoomName || '' });

        var currentCell = null;
        for (var i = 0; i < mapData.cells.length; i++) {
            if (mapData.cells[i].x === mapData.playerX && mapData.cells[i].y === mapData.playerY) {
                currentCell = mapData.cells[i];
                break;
            }
        }
        // terrain 欄位是中文名稱（如「林地」），kind 欄位才是英文 key（如 "forest"）
        var terrainKey  = (currentCell && currentCell.kind) ? currentCell.kind.toLowerCase() : '';

        // L1（時間）
        var hour  = 0;
        if (window.gameState) {
            var st = window.gameState();
            if (st && typeof st.game_time_sec_since_midnight === 'number') {
                hour = Math.floor(st.game_time_sec_since_midnight / 3600) % 24;
            }
        }
        renderL1({ hour_str: String(hour).padStart(2,'0') + ':00' });

        // L2（氣候）：啟動輪詢（如果尚未啟動）
        // TODO: SW-v5 後端推含 region_id 的 room_enter 後，改由 onRoomEnter 傳入真實 region_id
        startClimatePolling('default');

        // L3（地形與內嵌器物）
        var pool  = TERRAIN_PROSE[terrainKey];
        var prose = pool ? pool[Math.floor(Math.random() * pool.length)] : '';
        var ambience = getAmbience(terrainKey, hour);
        var displayText = ambience || prose || '';
        renderL3({ name: (currentCell && currentCell.terrain) || '' }, displayText, mapData.objects, mapData.harvestables);
    }

    window.SwRoomDesc = { render: render, renderV5: renderV5, renderL1: renderL1, renderL2: renderL2, renderL3: renderL3, renderPlace: renderPlace, getAmbience: getAmbience, startClimatePolling: startClimatePolling };
    console.log('[sw-room-desc] v0.27.0 loaded');
})();

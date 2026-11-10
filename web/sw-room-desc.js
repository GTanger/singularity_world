// sw-room-desc.js：地名主層、局部定位、時間／氣候與場景事實。v0.27.1
(function () {
    'use strict';

    // ── 地形氛圍句快取（從 /data/config/terrain_ambience.json 非同步載入）────
    var terrainAmbience = null;
    fetch('/data/config/terrain_ambience.json')
        .then(function (r) { return r.json(); })
        .then(function (j) { terrainAmbience = j.terrains || {}; })
        .catch(function (e) { console.warn('[sw-room-desc] terrain_ambience 載入失敗:', e); });

    // 六段天色時段（預設降級標籤；與 SwClock.getSixPhase 保持一致）
    var PHASE_LABELS = ['黎明前','黎明前','拂曉','拂曉','晝前','正晝','正晝','正晝','黃昏','黃昏','夜段','夜段'];
    // 氣候圖標映射；clear 需看時段，夜間不能顯示太陽。
    var WEATHER_ICONS = { rain: '☁', snow: '❄', wind: '🌬', fog: '🌫', storm: '⛈', plum_rain: '🌧' };

    // ── L2 氣候散文池（placeholder；後端有真資料後可擴充）──────────
    // 四段時段需與畫面上每兩小時一段的時段標籤一致：
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
        raw = raw.replace(oldNorthBankName, '北灣貨船大埠');
        terrain = terrain.replace(oldNorthBankName, '北灣貨船大埠');
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
        if (window.cleanPlaceDisplayName) {
            title = window.cleanPlaceDisplayName(title);
        } else {
            title = String(title || '').replace(/^文曜・中心樞紐石橋$/, '三水匯流大石橋').replace(/^中心樞紐石橋$/, '三水匯流大石橋').replace(/^文曜・/, '');
        }
        el.textContent = String(title || '').trim();
        el.hidden = !el.textContent;
        el.style.cursor = 'pointer';
        el.title = '點擊檢視或操作當前地塊物件';
        if (!el._cellObjClickBound) {
            el._cellObjClickBound = true;
            el.addEventListener('click', function (ev) {
                ev.stopPropagation();
                if (Array.isArray(_lastObjects) && _lastObjects.length > 0) {
                    var primary = _lastObjects[0];
                    var pid = String(primary.id || primary.object_id || primary.name || '');
                    if (pid) {
                        _activeInlineObjId = (_activeInlineObjId === pid) ? '' : pid;
                        renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
                        if (window.gameSend) {
							window.gameSend({ type: 'do_action', action: 'Look', entity_id: pid, target_id: pid, object_id: pid });
                        }
                    }
                }
            });
        }
        var locatorNode = locatorEl();
        if (locatorNode) {
            locatorNode.textContent = String(locator || '').trim();
            locatorNode.hidden = !locatorNode.textContent;
        }
    }

    // ── L1 時間渲染（簡潔時段與時間，不畫日晷 SVG）──────────────────
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
        var phase = (timeData && timeData.period)
            ? String(timeData.period)
            : ((window.SwClock && typeof window.SwClock.getSixPhase === 'function')
                ? window.SwClock.getSixPhase(hour)
                : '正晝');

        el.innerHTML =
            '<span class="desc-text">' + esc(phase) + ' </span>' +
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
        bank: '匯',
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

    // 模組層級狀態快照
    var _lastRoomId = '';
    var _lastTerrainData = null;
    var _lastDescText = '';
    var _lastObjects = [];
    var _lastHarvestables = [];
    var _activeInlineObjId = '';

    // 判斷物件是否屬於掉在地上的散落物品或跨房轉場點（房間常設陳設一律留在 #sw-l3-terrain 場景內容內）
    function isMovableOrPortableObject(obj) {
        if (!obj) return false;
        if (obj.type === 'npc' || obj.kind === 'npc' || obj.kind === 'Npc' || obj.npc_id) return false;
        if (obj.is_ground_item === true || obj.kind === 'ground_item') return true;
        if (obj.move_to_room_id) return true;
        var actions = obj.actions || obj.sockets || [];
        if (Array.isArray(actions)) {
            if (actions.indexOf('Move') !== -1 || actions.indexOf('Take') !== -1) {
                return true;
            }
        }
        return false;
    }

    function stripTerrainBoilerplate(descText, placeName) {
        var text = cleanSceneDesc(descText, placeName);
        return compactSceneDesc(text);
    }

    // ── L3 地形與內嵌靜態器物渲染 ─────────────────────────────────────
    function renderL3(terrainData, descText, objects, harvestables) {
        var el = l3El();
        if (!el) return;
        if (terrainData) _lastTerrainData = terrainData;
        if (descText !== undefined && descText !== null) _lastDescText = descText;
        if (Array.isArray(objects)) _lastObjects = objects;
        if (Array.isArray(harvestables)) _lastHarvestables = harvestables;

        var pEl = placeEl();
        var placeName = (pEl && pEl.textContent || '').trim() || (_lastTerrainData && _lastTerrainData.name) || '';
        var text = stripTerrainBoilerplate(_lastDescText, placeName);

        // 收集不可移動的場景固定器物與採集點
        var rawFixtures = [];
        if (Array.isArray(_lastObjects)) {
            for (var oi = 0; oi < _lastObjects.length; oi++) {
                var obj = _lastObjects[oi];
                if (!obj) continue;
                var oName = obj.name || obj.label || '';
                if (!oName) continue;
                if (isMovableOrPortableObject(obj)) continue;
                rawFixtures.push(obj);
            }
        }
        if (Array.isArray(_lastHarvestables)) {
            for (var hi = 0; hi < _lastHarvestables.length; hi++) {
                var h = _lastHarvestables[hi];
                if (!h) continue;
                var hName = h.name || h.label || '可採集物';
                rawFixtures.push({
                    id: h.id || h.name || ('harv_' + hi),
                    name: hName,
                    actions: ['Look', 'Gather'],
                    category_tag: 'natural_feature',
                    attribute_tags: ['fixed', 'organic']
                });
            }
        }

        // 去重並排除與地名相同者
        var dedupedFixtures = [];
        var seenLabels = {};
        for (var fi = 0; fi < rawFixtures.length; fi++) {
            var fix = rawFixtures[fi];
            var rawN = String(fix.name || fix.label || '').replace(/^WT-\d+\s*/i, '').trim();
            var shortL = (window.SwObjectList && typeof window.SwObjectList.conciseObjectLabel === 'function')
                ? window.SwObjectList.conciseObjectLabel(rawN) : rawN;
            if (!shortL || shortL === placeName || rawN === placeName) continue;
            if (seenLabels[shortL]) continue;
            seenLabels[shortL] = true;
            dedupedFixtures.push({ obj: fix, rawName: rawN, shortLabel: shortL });
        }

        // 依地貌與器物語義生成自然空間方位子句，將場景器物直接織入描述文句（嚴禁「近處可見 A、B、C」頓號條列）
        function buildSpatialFixtureClause(item, ph, placeStr, curText, idx) {
            var label = item.shortLabel || item.rawName || '';
            var full = (item.rawName || '') + ' ' + label;
            var pStr = String(placeStr || '') + ' ' + String(curText || '');
            var isBridge = /橋/.test(pStr);
            var isGarden = /(花圃|苑|坪|園)/.test(pStr);

            if (/(垂柳石欄|青石護欄|石欄)/.test(full)) {
                if (isBridge) return '橋側立著垂柳拂掠的' + ph;
                if (isGarden) return '水渠畔圍著柳枝輕拂的' + ph;
                return '臨水岸邊立著' + ph;
            }
            if (/水尺碑/.test(full)) return (isBridge ? '橋墩旁嵌有刻劃水位線的' : '渠岸石壁嵌有刻劃水位線的') + ph;
            if (/(指路石碑|路引碑|界碑)/.test(full)) return '道旁矗立著辨明方位的' + ph;
            if (/垂柳石凳/.test(full)) return '柳蔭下安放著石面微涼的' + ph;
            if (/歇腳石鼓/.test(full)) return '石階兩側設有一對供挑夫卸擔的' + ph;
            if (/(歇腳凳|長凳|石凳|條凳)/.test(full)) return '路側設有一列供過客小憩的' + ph;
            if (/航標燈/.test(full)) return '石欄轉折處立著指引舟楫的' + ph;
            if (/街燈/.test(full)) {
                if (isBridge) return '橋頭立著鑲有夜光貝母的' + ph;
                return '街角立著兼作路標的' + ph;
            }
            if (/(引水青石渠|青石渠|導水渠)/.test(full)) return '沿街鋪展著活水長流的' + ph;
            if (/(繫船石柱|繫船玄武樁|生鐵繫纜樁|繫纜樁)/.test(full)) return '駁岸邊深埋著被纜繩磨亮的' + ph;
            if (/石堤樁/.test(full)) return '外堤沿線深打著迎抗大潮的' + ph;
            if (/商船/.test(full)) return '突堤泊位旁停靠著高聳帆檣的' + ph;
            if (/(平底竹筏|竹筏|渡船)/.test(full)) return '淺水泊位旁繫著隨波輕晃的' + ph;
            if (/(卸貨板|跳板)/.test(full)) return '碼頭石階前斜搭著厚實的' + ph;
            if (/(滑輪吊架|吊架)/.test(full)) return '岸邊架設著起卸重貨的' + ph;
            if (/(告示欄|布告欄)/.test(full)) return '街隅立著張貼往來通告的' + ph;
            if (/木牌案/.test(full)) return '廊棚下設有用以核對車馬時刻的' + ph;
            if (/(茶盤几|茶几)/.test(full)) return '窗下置著沏有清茶的' + ph;
            if (/八仙桌/.test(full)) return '廳堂正中擺放著厚實的' + ph;
            if (/書案/.test(full)) return '窗前設有一方陳設筆硯的' + ph;
            if (/(古籍架|書架)/.test(full)) return '靠牆立著排滿典籍輿圖的' + ph;
            if (/(衣櫥|衣櫃)/.test(full)) return '內側靠牆立著防潮辟穢的' + ph;
            if (/(床榻|大床)/.test(full)) return '室隅安放著鋪有羽絨暖被的' + ph;
            if (/搖椅/.test(full)) return '向陽窗欞旁安放著鋪墊羊毛褥的' + ph;
            if (/檯燈/.test(full)) return '案角立著珠光溫潤的' + ph;
            if (/(冰鑑櫃|冰鑑)/.test(full)) return '屋角擺著寒氣繚繞的' + ph;
            if (/沐浴桶/.test(full)) return '隔間內設有一隻連通熱泉的' + ph;
            if (/水栓/.test(full)) return '牆隅砌有接通活水管道的' + ph;
            if (/(儲水陶缸|陶缸)/.test(full)) return '廊下安放著注滿甘冽活水的' + ph;
            if (/洗衣盆/.test(full)) return '簷下水渠旁擱著銅皮包邊的' + ph;
            if (/信箱/.test(full)) return '門扉外側掛著收發書信的' + ph;
            if (/鞋拔/.test(full)) return '玄關門廊旁掛著打磨光滑的' + ph;
            if (/手爐/.test(full)) return '几案一側擱著蓄熱生溫的' + ph;
            if (/櫃檯/.test(full)) return '前堂設有掌理庶務的' + ph;
            if (/(木作檯|工作檯)/.test(full)) return '場中設有一張排列鑿刨工具的' + ph;
            if (/(晾網木架|曬網架)/.test(full)) return '空地旁橫架著晾曬麻網的' + ph;
            if (/貨箱/.test(full)) return '廊下整齊碼放著鐵箍加固的' + ph;
            if (/柴木垛/.test(full)) return '簷角整齊碼放著乾燥密實的' + ph;
            if (/營火堆/.test(full)) return '空地中央燃著劈啪作響的' + ph;
            if (/田壟苗畦/.test(full)) return '清泉溝渠間鋪展著壟溝分明的' + ph;
            if (/冰槽/.test(full)) return '攤前砌有鋪滿碎冰保鮮的' + ph;
            if (/(雙盤秤|吊秤)/.test(full)) return '木架上懸著買賣過磅用的' + ph;
            if (/價目冊/.test(full)) return '帳案上攤開著載明各埠行情的' + ph;
            if (/帳冊夾/.test(full)) return '條案上擺著夾滿複式帳頁的' + ph;
            if (/計數板/.test(full)) return '公案上端放著滑鈕圓潤的' + ph;
            if (/銅鑼/.test(full)) return '側壁高懸著報時防險的' + ph;
            if (/望遠鏡/.test(full)) return '瞭望窗畔架著觀測遠處動靜的' + ph;
            if (/羅盤/.test(full)) return '案頭鎮著校準方位的' + ph;

            var fallbackLoci = ['廊下置有', '一側設有一座', '案頭擺著', '牆邊立著'];
            return fallbackLoci[idx % fallbackLoci.length] + ph;
        }

        // 建立 inline button 與標籤比對替換
        var unmatchedItems = [];
        var activeObjFound = null;
        var placeholderMap = {};

        var VALID_MATERIALS = ['stone', 'wood', 'metal', 'paper', 'fabric', 'ceramic', 'bamboo', 'leather', 'liquid', 'organic', 'composite'];

        for (var di = 0; di < dedupedFixtures.length; di++) {
            var item = dedupedFixtures[di];
            var o = item.obj;
            var objId = String(o.id || o.object_id || o.name || '');
            var rawName = item.rawName;
            var shortLabel = item.shortLabel;
            if (objId === _activeInlineObjId) {
                activeObjFound = o;
            }

            var categoryAttr = '';
            if (o.category_tag) {
                categoryAttr = ' data-obj-category="' + esc(String(o.category_tag)) + '"';
            }

            var rawAttrs = Array.isArray(o.attribute_tags) ? o.attribute_tags.slice() : [];
            if (rawAttrs.indexOf('fixed') === -1 && rawAttrs.indexOf('portable') === -1) {
                rawAttrs.unshift('fixed');
            }
            var hasMat = false;
            for (var ami = 0; ami < rawAttrs.length; ami++) {
                if (VALID_MATERIALS.indexOf(rawAttrs[ami]) !== -1) {
                    hasMat = true;
                    break;
                }
            }
            if (!hasMat) {
                rawAttrs.push('stone');
            }
            var attrList = rawAttrs.join(',');

            var allTags = (Array.isArray(o.tags) && o.tags.length) ? o.tags.join(',') : (o.tag || '');
            var actTargetName = shortLabel || rawName;

            var btnHtml = '<button type="button" class="desc-object log-object-action' + (objId === _activeInlineObjId ? ' is-active-target' : '') + '"' +
                ' data-obj-id="' + esc(objId) + '"' +
                ' data-entity-id="' + esc(objId) + '"' +
                ' data-obj-tag="' + esc(o.tag || '') + '"' +
                categoryAttr +
                ' data-obj-attributes="' + esc(attrList) + '"' +
                (allTags ? (' data-obj-tags="' + esc(allTags) + '"') : '') +
                ' data-target-name="' + esc(actTargetName) + '"' +
                ' data-action="Look">' +
                esc(shortLabel) +
                '</button>';

            var ph = '\uE000' + di + '\uE001';
            placeholderMap[ph] = btnHtml;

            // Stage A-1: 檢查是否直接出現在純文字 text 中
            var matched = false;
            var targetMatches = [rawName, shortLabel];
            for (var mi = 0; mi < targetMatches.length; mi++) {
                var mStr = targetMatches[mi];
                if (!mStr) continue;
                var idx = text.indexOf(mStr);
                if (idx !== -1) {
                    text = text.slice(0, idx) + ph + text.slice(idx + mStr.length);
                    matched = true;
                    break;
                }
            }

            // Stage A-2: 檢查描述中是否已有對應語義子句或同義詞，若有則就地融合
            if (!matched) {
                var fullLabel = rawName + ' ' + shortLabel;
                if (/(歇腳凳|長凳|石凳|條凳|石鼓)/.test(fullLabel)) {
                    if (text.indexOf('街坊常在此歇涼') !== -1) {
                        text = text.replace('街坊常在此歇涼', '樹蔭下設有' + ph + '供街坊歇涼');
                        matched = true;
                    } else if (text.indexOf('坊民在此休憩交談') !== -1) {
                        text = text.replace('坊民在此休憩交談', '石坪旁設有' + ph + '供坊民休憩交談');
                        matched = true;
                    }
                } else if (/(繫船|繫纜樁|石柱)/.test(fullLabel) && text.indexOf('停泊木樁') !== -1) {
                    text = text.replace('停泊木樁', ph);
                    matched = true;
                } else if (/石堤樁/.test(fullLabel) && text.indexOf('防汛石堤') !== -1) {
                    text = text.replace('防汛石堤', ph);
                    matched = true;
                } else if (/沐浴桶/.test(fullLabel) && text.indexOf('熱水洗浴槽') !== -1) {
                    text = text.replace('熱水洗浴槽', ph);
                    matched = true;
                } else if (/床榻/.test(fullLabel) && text.indexOf('溫暖通鋪') !== -1) {
                    text = text.replace('溫暖通鋪', ph);
                    matched = true;
                } else if (/街燈/.test(fullLabel) && text.indexOf('防風燈籠') !== -1) {
                    text = text.replace('防風燈籠', ph);
                    matched = true;
                }
            }

            if (!matched) {
                unmatchedItems.push({ item: item, ph: ph, objId: objId });
            }
        }

        // Stage B: 將尚未出現在原句中的場景固定器物，以具體空間方位子句織入描述句尾（最多織入 4 件核心陳設）
        if (unmatchedItems.length > 0) {
            var MAX_WOVEN = 4;
            var selectedUnmatched = unmatchedItems;
            if (unmatchedItems.length > MAX_WOVEN) {
                selectedUnmatched = unmatchedItems.slice(0, MAX_WOVEN);
                if (_activeInlineObjId) {
                    var hasActiveInSlice = false;
                    for (var si = 0; si < selectedUnmatched.length; si++) {
                        if (selectedUnmatched[si].objId === _activeInlineObjId) {
                            hasActiveInSlice = true;
                            break;
                        }
                    }
                    if (!hasActiveInSlice) {
                        for (var ui = MAX_WOVEN; ui < unmatchedItems.length; ui++) {
                            if (unmatchedItems[ui].objId === _activeInlineObjId) {
                                selectedUnmatched[MAX_WOVEN - 1] = unmatchedItems[ui];
                                break;
                            }
                        }
                    }
                }
            }
            var wovenClauses = [];
            for (var wi = 0; wi < selectedUnmatched.length; wi++) {
                var u = selectedUnmatched[wi];
                wovenClauses.push(buildSpatialFixtureClause(u.item, u.ph, placeName, text, wi));
            }
            var baseTrimmed = text.replace(/[。！？!?]+$/, '').trim();
            if (baseTrimmed) {
                text = baseTrimmed + '，' + wovenClauses.join('，') + '。';
            } else {
                text = wovenClauses.join('，') + '。';
            }
        }

        // 對整段純文字進行 HTML 跳脫
        var escapedText = esc(text);

        // 還原佔位符為原始 btnHtml
        for (var phKey in placeholderMap) {
            if (Object.prototype.hasOwnProperty.call(placeholderMap, phKey)) {
                escapedText = escapedText.split(phKey).join(placeholderMap[phKey]);
            }
        }

        var finalHtml = '<span class="desc-text">' + escapedText + '</span>';
        // 如果有選取的 inline 物件且具備 Look 以外的 socket/action，渲染 inline action chips
        if (_activeInlineObjId && activeObjFound) {
            var actList = activeObjFound.actions || activeObjFound.sockets || [];
            var venueTail = String(placeName || '').replace(/^.*[・·]\s*/, '');
            var isShopOrCanteen = /(行|鋪|肆|工坊|染坊|織坊|木坊|鐵坊|茶坊|酒坊|磨坊|店|樓|食堂|市集|攤|館|閣|局|場|廠|塢|站|社)/.test(venueTail);
            var extraActs = [];
            for (var ai = 0; ai < actList.length; ai++) {
                var code = actList[ai];
                if (code === 'Look') continue;
                if (code === 'Acquire' && !isShopOrCanteen) continue;
                extraActs.push(code);
            }
            if (extraActs.length > 0) {
                var chipsHtml = '<span class="sw-desc-inline-acts">';
                var seenChipLabels = {};
                for (var ci = 0; ci < extraActs.length; ci++) {
                    var actCode = extraActs[ci];
                    var chipLabel = actCode;
                    if (window.SwObjectList && typeof window.SwObjectList.resolveSocketActionLabel === 'function') {
                        chipLabel = window.SwObjectList.resolveSocketActionLabel(actCode, activeObjFound);
                    } else if (window.SwResolveActionLabelZh) {
                        chipLabel = window.SwResolveActionLabelZh(actCode, activeObjFound);
                    }
                    if (!chipLabel || seenChipLabels[chipLabel]) continue;
                    seenChipLabels[chipLabel] = true;
                    chipsHtml += '<button type="button" class="sw-inline-act-chip log-object-action"' +
                        ' data-entity-id="' + esc(_activeInlineObjId) + '"' +
                        ' data-action="' + esc(actCode) + '"' +
                        ' data-target-name="' + esc(activeObjFound.name || activeObjFound.label || '') + '">' +
                        esc(chipLabel) +
                        '</button>';
                }
                chipsHtml += '</span>';

                // 統一接在描述句尾，避免點擊句中物件時切斷句子或造成段落換行跳動
                finalHtml += chipsHtml;
            }
        }

        if (el._lastFinalHtml !== finalHtml) {
            el._lastFinalHtml = finalHtml;
            el.innerHTML = finalHtml;
        }

        // 綁定 inline 點擊監聽以切換 _activeInlineObjId 並展開 chips，點擊 chip 或外部時關閉
        if (!el._inlineClickBound) {
            el._inlineClickBound = true;
            el.addEventListener('click', function (ev) {
                var chipBtn = ev.target && ev.target.closest && ev.target.closest('.sw-desc-inline-acts .sw-inline-act-chip');
                if (chipBtn) {
                    _activeInlineObjId = null;
                    setTimeout(function () {
                        renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
                    }, 0);
                    return;
                }
                var descBtn = ev.target && ev.target.closest && ev.target.closest('.desc-object');
                if (descBtn) {
                    var clickedId = descBtn.getAttribute('data-obj-id');
                    if (clickedId) {
                        _activeInlineObjId = clickedId;
                        setTimeout(function () {
                            renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
                        }, 0);
                    }
                } else if (_activeInlineObjId) {
                    _activeInlineObjId = null;
                    setTimeout(function () {
                        renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
                    }, 0);
                }
            });
            document.addEventListener('click', function (ev) {
                if (!_activeInlineObjId) return;
                var l3 = document.getElementById('sw-l3-terrain');
                var l4 = document.getElementById('sw-l4-objects');
                var pNode = placeEl();
                var isPlayerCell = ev.target && ev.target.closest && ev.target.closest('.sw-cell-player');
                if (isPlayerCell || (pNode && pNode.contains(ev.target)) || (l4 && l4.contains(ev.target))) return;
                if (l3 && !l3.contains(ev.target)) {
                    _activeInlineObjId = null;
                    setTimeout(function () {
                        renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
                    }, 0);
                }
            });
        }
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

        var rawIncomingDesc = String(msg.desc || '').trim();
        var incomingRoomId = String(msg.room_id || (msg.place && msg.place.name) || '');
        var roomChanged = !!incomingRoomId && incomingRoomId !== _lastRoomId;
        if (incomingRoomId) _lastRoomId = incomingRoomId;

        var descText = rawIncomingDesc
            ? cleanSceneDesc(rawIncomingDesc, msg.terrain && msg.terrain.name)
            : ((!roomChanged && _lastDescText) ? _lastDescText : cleanSceneDesc('', msg.terrain && msg.terrain.name));

        if (roomChanged || (rawIncomingDesc && _lastDescText !== descText)) {
            _activeInlineObjId = '';
        }
        var l3Node = l3El();
        var currentL3Text = (l3Node && l3Node.textContent || '').trim();
        if (rawIncomingDesc || roomChanged || !options.keepL3 || !currentL3Text) {
            renderL3(
                msg.terrain || _lastTerrainData,
                descText,
                Array.isArray(msg.objects) ? msg.objects : _lastObjects,
                Array.isArray(msg.harvestables) ? msg.harvestables : _lastHarvestables
            );
        }
        if (!options.keepL3 && typeof window.SwPushBreath === 'function') {
            var pEl = placeEl();
            var pName = (pEl && pEl.textContent || '').trim() || (msg.place && msg.place.name) || '';
            var b0 = document.getElementById('sw-breath-0');
            if (pName && (!b0 || !b0.textContent)) {
                window.SwPushBreath('駐足【' + pName + '】，墨韻隨步履漸次鋪展。', { byPlayer: false });
            }
        }
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
        if (_lastDescText !== displayText) {
            _activeInlineObjId = '';
        }
        renderL3({ name: (currentCell && currentCell.terrain) || '' }, displayText, mapData.objects, mapData.harvestables);
    }

    window.SwRoomDesc = {
        render: render,
        renderV5: renderV5,
        renderL1: renderL1,
        renderL2: renderL2,
        renderL3: renderL3,
        activateCellObject: function (objId) {
            if (!objId && Array.isArray(_lastObjects) && _lastObjects.length > 0) {
                objId = String(_lastObjects[0].id || _lastObjects[0].object_id || _lastObjects[0].name || '');
            }
            if (!objId) return '';
            _activeInlineObjId = String(objId);
            renderL3(_lastTerrainData, _lastDescText, _lastObjects, _lastHarvestables);
            return _activeInlineObjId;
        },
        syncL3Objects: function (objects, harvestables) {
            renderL3(_lastTerrainData, _lastDescText, objects, harvestables);
        },
        renderPlace: renderPlace,
        getAmbience: getAmbience,
        startClimatePolling: startClimatePolling
    };
})();

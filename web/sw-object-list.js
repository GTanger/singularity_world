// sw-object-list.js：在場人物、可接觸物與八方向路線。v0.27.2
(function () {
    'use strict';

    // ── 數字編號邏輯（同標籤 ≥ 2 才用純數字排序，純前端，不持久化）────────────────────────────
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
                    result[indices[j]].suffix = String(j + 1);
                }
            }
        }
        return result;
    }

    // ── 觀察態 → 顯示標籤 ────────────────────────────────────────────
    function labelOf(obj) {
        if (!obj) return '';
        var state = obj.observation_state;
        if (!state && obj.name) {
            return obj.name;
        }
        var isAsciiTag = obj.tag && /^[a-z0-9_]+$/.test(obj.tag);
        var displayTag = isAsciiTag ? '' : (obj.tag || '');
        if (state === 'named') {
            return displayTag + (obj.name ? obj.name : '');
        } else if (state === 'identified') {
            return displayTag || obj.name || '行人';
        } else {
            // stranger 或未定義
            return displayTag || obj.name || '行人';
        }
    }

    // 當前 L4 物件列表（供 applyDelta 更新用）
    var _currentObjects = [];
    var _currentNpcs = [];

    function splitPlaceLabel(value, explicitLocator) {
        var raw = String(value || '').trim();
        var locator = String(explicitLocator || '').trim();
        var match = raw.match(/^(.+?)（(.+)）$/);
        if (match) {
            return {
                title: match[1].trim(),
                locator: locator || match[2].trim()
            };
        }
        return { title: raw || '可去處', locator: locator };
    }

    function exitItemsFromGrid() {
        var G = window.SwGrid;
        if (!G || !G.mapData) return [];
        var mapData = G.mapData;
        var out = [];
        var seen = {};
        var keys = [
            { key: 'east',  dir: '東' },
            { key: 'west',  dir: '西' },
            { key: 'south', dir: '南' },
            { key: 'north', dir: '北' },
            { key: 'up',    dir: '上' },
            { key: 'down',  dir: '下' },
            { key: 'in',    dir: '進' },
            { key: 'out',   dir: '出' }
        ];

        var legacy = mapData.exits || [];
        var hasCompletePlaceRoutes = legacy.some(function (exit) {
            return exit && (exit.target_place_id || exit.to_room_id);
        });

        // 正式場所的 view.exits 保留同方向的全部岔路與路型；v5 方位表每個方向
        // 只能放一筆，會把其餘岔路覆蓋，因此只在沒有完整場所路線時相容使用。
        if (mapData.exits_v5 && !hasCompletePlaceRoutes) {
            for (var i = 0; i < keys.length; i++) {
                var k = keys[i];
                var info = mapData.exits_v5[k.key];
                if (!info || info.walkable !== true) continue;
                var v5Target = info.to_room_id || info.target_place_id || '';
                if (v5Target && mapData.currentRoomId && v5Target === mapData.currentRoomId && k.dir !== '進' && k.dir !== '出') {
                    continue;
                }
                var v5Key = k.dir + '|' + v5Target;
                if (seen[v5Key]) continue;
                var rawLabel = info.full_name ||
                    (G.exitLabel || function (x) { return x && (x.name || x.to_room_name) || '可去處'; })(info);
                var placeLabel = splitPlaceLabel(rawLabel, info.locator);
                out.push({
                    dir: k.dir,
                    label: placeLabel.title,
                    detail: placeLabel.locator,
                    target: v5Target,
                    routeId: info.route_id || '',
                    mode: info.mode || '',
                    cost: info.travel_cost || 0
                });
                seen[v5Key] = true;
            }
        }

        for (var li = 0; li < legacy.length; li++) {
            var ex = legacy[li];
            if (!ex || !ex.direction) continue;
            var target = ex.to_room_id || ex.target_place_id || '';
            if (target && mapData.currentRoomId && target === mapData.currentRoomId && ex.direction !== '進' && ex.direction !== '出') {
                continue;
            }
            var routeKey = ex.direction + '|' + target;
            if (seen[routeKey]) continue;
            var legacyLabel = ex.to_room_name || ex.name || '可去處';
            var legacyPlaceLabel = splitPlaceLabel(legacyLabel, ex.locator);
            out.push({
                dir: ex.direction,
                label: legacyPlaceLabel.title,
                detail: legacyPlaceLabel.locator,
                target: target,
                routeId: ex.route_id || '',
                mode: ex.mode || '',
                cost: ex.travel_cost || 0
            });
            seen[routeKey] = true;
        }
        return disambiguateRoutes(out);
    }

    function routeModeLabel(mode) {
        if (mode === 'road') return '道路';
        if (mode === 'ferry') return '渡船';
        if (mode === 'walk') return '野徑';
        return '岔路';
    }

    function disambiguateRoutes(routes) {
        var buckets = {};
        for (var i = 0; i < routes.length; i++) {
            var key = routes[i].dir + '|' + routes[i].label + '|' + (routes[i].detail || '');
            if (!buckets[key]) buckets[key] = [];
            buckets[key].push(routes[i]);
        }
        for (var bucketKey in buckets) {
            var siblings = buckets[bucketKey];
            if (siblings.length < 2) continue;
            var byMode = {};
            for (var si = 0; si < siblings.length; si++) {
                var mode = siblings[si].mode || '';
                if (!byMode[mode]) byMode[mode] = [];
                byMode[mode].push(siblings[si]);
            }
            for (var modeKey in byMode) {
                var sameMode = byMode[modeKey].slice().sort(function (left, right) {
                    return (left.cost || 0) - (right.cost || 0) ||
                        String(left.target || '').localeCompare(String(right.target || ''));
                });
                for (var mi = 0; mi < sameMode.length; mi++) {
                    var qualifier = routeModeLabel(modeKey);
                    if (sameMode.length === 2 && sameMode[0].cost !== sameMode[1].cost) {
                        qualifier = (mi === 0 ? '近' : '遠') + qualifier;
                    } else if (sameMode.length > 1) {
                        qualifier += String(mi + 1);
                    }
                    sameMode[mi].detail = qualifier +
                        (sameMode[mi].detail ? ' · ' + sameMode[mi].detail : '');
                }
            }
        }
        return routes;
    }

    function refineExitName(item, currentTitle) {
        var raw = String(item.label || '').trim();
        var dir = item.dir || '';
        var cTitle = String(currentTitle || '').trim();
        if (cTitle && raw === cTitle) {
            var shortSame = cTitle.replace(/^.*[・·]\s*/, '');
            return shortSame || cTitle;
        }
        var cRegionMatch = cTitle.match(/^([^・·]+)[・·]/);
        if (cRegionMatch && cRegionMatch[1]) {
            var rPrefix = cRegionMatch[1].trim();
            if (raw.indexOf(rPrefix + '・') === 0 || raw.indexOf(rPrefix + '·') === 0) {
                raw = raw.replace(/^.*[・·]\s*/, '').trim() || raw;
            }
        }
        if (dir === '進') {
            if (/客房|內室|室|房|堂|殿|閣|廬|居/.test(raw)) {
                return raw.indexOf('進') === 0 ? raw : ('進' + raw);
            }
            return '進' + (raw || '內室');
        }
        if (dir === '出') {
            var G = window.SwGrid;
            var inPrivateRoom = !!(G && G.mapData && G.mapData.in_private_room);
            if (inPrivateRoom) {
                return '走出內室（出）';
            }
            if (/客房|內室|室|房|堂|殿|閣|廬|居|街/.test(raw)) {
                return raw.indexOf('出') === 0 ? raw : ('出' + raw);
            }
            return '出' + (raw || '客房');
        }
        if (dir === '上') {
            return raw.indexOf('上') === 0 ? raw : ('上' + raw);
        }
        if (dir === '下') {
            return raw.indexOf('下') === 0 ? raw : ('下' + raw);
        }
        var clean = raw.replace(new RegExp('^' + dir + '側\\s*[·・]\\s*'), '');
        return clean || raw || '通路';
    }

    function exitCardsHtml(escFn) {
        var exits = exitItemsFromGrid();
        if (!exits.length) return '';
        var currentTitle = String(document.getElementById('sw-place-name') &&
            document.getElementById('sw-place-name').textContent || '').trim();

        var G = window.SwGrid;
        var mapData = G && G.mapData;
        var inPrivateRoom = !!(mapData && mapData.in_private_room);

        // 若玩家處於內室（in_private_room），去路只有「出」，顯示清晰的「走出內室（出）」按鈕
        if (inPrivateRoom) {
            var outExit = exits.find(function (e) { return e.dir === '出'; }) || exits[0];
            var destTarget = (outExit && outExit.target) || '';
            var destDir = (outExit && outExit.dir) || '出';
            return '<div class="sw-compass-rose sw-compass-interior" role="navigation" aria-label="內室去路">' +
                   '<button type="button" class="sw-obj-card sw-compass-btn sw-route-card sw-route-exit-room" data-move-dir="' + escFn(destDir) + '" data-move-target="' + escFn(destTarget) + '" title="走出內室（出）">' +
                   '<span class="sw-compass-dest">走出內室（出）</span>' +
                   '</button>' +
                   '</div>';
        }

        // 當出口方向為「進」時，因底部地圖控制列已有專屬 #btn-enter-room，不在 3x3 街區羅盤中重複
        exits = exits.filter(function (ex) {
            return ex.dir !== '進';
        });
        if (!exits.length) return '';
        var shortCenter = currentTitle;
        shortCenter = shortCenter.replace(/^.*[・·]\s*/, '').trim();
        if (!shortCenter) shortCenter = '此處';

        // 3x3 空間方位矩陣：
        // [0][0]=西北/出/上  [0][1]=北           [0][2]=東北/進/上
        // [1][0]=西          [1][1]=中心(自身)   [1][2]=東
        // [2][0]=西南/出/下  [2][1]=南           [2][2]=東南/進/下
        var grid = [
            [null, null, null],
            [null, 'center', null],
            [null, null, null]
        ];

        var unassigned = [];
        for (var i = 0; i < exits.length; i++) {
            var ex = exits[i];
            var dir = ex.dir;
            if (dir === '北' && !grid[0][1]) {
                grid[0][1] = ex;
            } else if (dir === '南' && !grid[2][1]) {
                grid[2][1] = ex;
            } else if (dir === '西' && !grid[1][0]) {
                grid[1][0] = ex;
            } else if (dir === '東' && !grid[1][2]) {
                grid[1][2] = ex;
            } else if (dir === '西北' && !grid[0][0]) {
                grid[0][0] = ex;
            } else if (dir === '東北' && !grid[0][2]) {
                grid[0][2] = ex;
            } else if (dir === '西南' && !grid[2][0]) {
                grid[2][0] = ex;
            } else if (dir === '東南' && !grid[2][2]) {
                grid[2][2] = ex;
            } else if (dir === '進') {
                if (!grid[0][2]) grid[0][2] = ex;
                else if (!grid[2][2]) grid[2][2] = ex;
                else unassigned.push(ex);
            } else if (dir === '出') {
                if (!grid[0][0]) grid[0][0] = ex;
                else if (!grid[2][0]) grid[2][0] = ex;
                else unassigned.push(ex);
            } else if (dir === '上') {
                if (!grid[0][2]) grid[0][2] = ex;
                else if (!grid[0][0]) grid[0][0] = ex;
                else unassigned.push(ex);
            } else if (dir === '下') {
                if (!grid[2][0]) grid[2][0] = ex;
                else if (!grid[2][2]) grid[2][2] = ex;
                else unassigned.push(ex);
            } else {
                unassigned.push(ex);
            }
        }

        var cornerSlots = [[0, 2], [0, 0], [2, 2], [2, 0]];
        for (var ci = 0; ci < cornerSlots.length && unassigned.length > 0; ci++) {
            var cr = cornerSlots[ci][0];
            var cc = cornerSlots[ci][1];
            if (!grid[cr][cc]) {
                grid[cr][cc] = unassigned.shift();
            }
        }

        if (shortCenter.length > 5) {
            shortCenter = shortCenter.replace(/(總部|大堂|傢俱坊|工坊)$/, '');
        }
        var html = '<div class="sw-compass-rose" role="navigation" aria-label="相對方位去路">';
        for (var r = 0; r < 3; r++) {
            if (r !== 1 && !grid[r][0] && !grid[r][1] && !grid[r][2]) {
                continue;
            }
            for (var c = 0; c < 3; c++) {
                var cell = grid[r][c];
                if (r === 1 && c === 1) {
                    html += '<div class="sw-compass-center" title="當前場所：' + escFn(currentTitle) + '">◎ ' + escFn(shortCenter) + '</div>';
                } else if (cell) {
                    var refName = refineExitName(cell, currentTitle);
                    html += '<button type="button" class="sw-obj-card sw-compass-btn sw-route-card" data-move-dir="' + escFn(cell.dir) + '" data-move-target="' + escFn(cell.target || '') + '" title="往' + escFn(cell.dir) + '：' + escFn(refName) + '">';
                    html += '<span class="sw-compass-dest">' + escFn(refName) + '</span>';
                    html += '</button>';
                } else {
                    html += '<div class="sw-compass-slot sw-compass-empty" aria-hidden="true"></div>';
                }
            }
        }
        html += '</div>';

        if (unassigned.length > 0) {
            html += '<div class="sw-compass-extra">';
            for (var u = 0; u < unassigned.length; u++) {
                var uEx = unassigned[u];
                var uRefName = refineExitName(uEx, currentTitle);
                html += '<button type="button" class="sw-obj-card sw-compass-btn sw-route-card" data-move-dir="' + escFn(uEx.dir) + '" data-move-target="' + escFn(uEx.target || '') + '" title="往' + escFn(uEx.dir) + '：' + escFn(uRefName) + '">';
                html += '<span class="sw-compass-dest">' + escFn(uRefName) + '</span>';
                html += '</button>';
            }
            html += '</div>';
        }
        return html;
    }

    // 判斷物件是否屬於掉在地上的散落物品或跨房轉場點（房間常設陳設一律留在 #sw-l3-terrain 場景內容內）
    function isMovableOrPortableObject(obj) {
        if (!obj) return false;
        // NPC 或玩家不歸於隨行通道物件欄（已有專門的人物理論欄位）
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

    function resolveSocketActionLabel(act, obj) {
        return resolveActionLabelZh(act, obj);
    }

    function isFacility(obj) {
        if (!obj) return false;
        if (obj.type === 'npc' || obj.kind === 'npc' || obj.kind === 'Npc' || obj.npc_id) return false;
        if (isMovableOrPortableObject(obj)) return false;
        return true;
    }

    function facilityIcon(obj) {
        var n = String((obj && (obj.name || obj.label)) || '');
        if (/[店鋪市集商行鎂幣庫帳櫃匯票攤賈貿資]/.test(n)) return '$';
        if (/[醫藥藥包湯包溫灸診草藥救傷回春]/.test(n)) return '+';
        if (/[茶席臥榻浴池餐食旅舍精舍房室舍榻椅茶食歇席眠床水盆浴盆洗漱]/.test(n)) return '⌂';
        if (/[鐘樓星軌典籍契書官堂館衛哨營譜冊牘卷碑標律法星盤星儀]/.test(n)) return '⚑';
        if (/[鍛爐木作工具石柱機坊鐵匠器車井盤栓釜鍋窯柱板尺樁秤砧]/.test(n)) return '⚒︎';
        return '⚒︎';
    }
    function conciseObjectLabel(name) {
        var str = String(name || '').trim();
        str = str.replace(/^WT-\d+\s*/i, '').trim();
        if (!str) return '';
        var knownMap = {
            '黃銅告示欄': '告示欄',
            '黃銅公佈欄': '告示欄',
            '公佈欄': '告示欄',
            '黃銅鑲邊告示欄': '告示欄',
            '黃銅鑲邊公佈欄': '告示欄',
            '雕花青石欄': '橋欄',
            '雕花青石欄杆': '橋欄',
            '雕花青石護欄': '橋欄',
            '雕花石欄': '橋欄',
            '青石欄杆': '橋欄',
            '青石護欄': '橋欄',
            '日晷指路柱': '日晷',
            '報時日晷柱': '日晷',
            '單筒望遠鏡': '望遠鏡',
            '黃銅單筒鏡': '望遠鏡',
            '瞭望單筒鏡': '望遠鏡',
            '捲線釣竿': '釣竿',
            '青竹釣竿': '釣竿',
            '老青竹釣竿': '釣竿',
            '老青竹黃銅輪捲線釣竿': '釣竿',
            '石雕拴馬柱': '拴馬柱',
            '青石拴馬柱': '拴馬柱',
            '生鐵地秤': '地秤',
            '青石大水槽': '水槽',
            '活水石槽': '水槽',
            '飲馬石槽': '水槽',
            '溫石熱水槽': '水槽',
            '家用蓄熱溫石熱水槽': '水槽',
            '粗陶大水缸': '水缸',
            '儲水大陶缸': '水缸',
            '黃銅水栓': '水栓',
            '活水銅栓': '水栓',
            '岑氏合金黃銅自來水栓': '水栓',
            '自來活水黃銅水栓': '水栓',
            '松香木大床': '床榻',
            '松木大床': '床榻',
            '羽絨大床': '床榻',
            '松香木彈力大床': '床榻',
            '防蛀樟木衣箱': '衣箱',
            '樟木衣箱': '衣箱',
            '樟木衣櫥': '衣櫥',
            '烘衣木櫥': '衣櫥',
            '松炭溫風烘衣櫥': '衣櫥',
            '夜光貝母檯燈': '檯燈',
            '貝母檯燈': '檯燈',
            '旋蓋夜光貝母檯燈': '檯燈',
            '旋蓋夜光貝母台燈': '檯燈',
            '貝母台燈': '檯燈',
            '夜光貝母台燈': '檯燈',
            '寒水玉保鮮石櫃': '冰鑑櫃',
            '寒水玉雙層保鮮石櫃': '冰鑑櫃',
            '雙層保鮮石櫃': '冰鑑櫃',
            '保鮮石櫃': '冰鑑櫃',
            '防雨竹筒信箱': '信箱',
            '防雨竹編信箱': '信箱',
            '竹筒信箱': '信箱',
            '竹編信箱': '信箱',
            '烏木教案': '教案',
            '紅木書案': '書案',
            '鑲貝書案': '書案',
            '工匠工具架': '工具架',
            '精鋼工具架': '工具架',
            '通商貨架棚': '貨架棚',
            '貝母路燈與防雨竹編路標': '路燈・路標',
            '文曜青石街燈與黃銅公佈欄': '街燈・告示欄',
            '三水川雕花青石欄杆與繫船環': '橋欄・繫船環',
            '自來活水黃銅水栓與溫石熱水槽': '水栓・水槽',
            '松香木彈力大床與儲食石櫃': '床榻・冰鑑櫃',
            '止血水苔與避瘴清心丸架': '水苔・丸架',
            '青鹽海貝薄荷潔牙粉': '海貝潔牙粉',
            '煅燒海貝薄荷潔牙粉': '海貝潔牙粉',
            '薄層重礦烏木膽隨身保溫瓶': '隨身保溫瓶',
            '烏木膽隨身保溫瓶': '隨身保溫瓶',
            '順時針環島海運商船': '環島商船',
            '溫氏導能溫石百草溫灸盒': '百草溫灸盒',
            '校準隨身黃銅懷錶': '黃銅懷錶'
        };
        if (knownMap[str]) {
            return knownMap[str];
        }
        if (str.indexOf('與') !== -1) {
            var parts = str.split('與');
            if (parts.length === 2) {
                var p1 = conciseObjectLabel(parts[0]);
                var p2 = conciseObjectLabel(parts[1]);
                var combined = p1 + '・' + p2;
                if (combined.length <= 9) {
                    return combined;
                }
            }
        }
        var prefixRegex = /^(?:青鹽海貝|煅燒海貝|重礦包邊|薄層重礦|三號坑口|三號坑|老堰頭|霜林口|築港頭|文曜城|文曜|大圳頭|半嶺鎮|半嶺|風櫃尾|烏石鼻|水尾寮|紅土崁|黑沼|北嶺|南哨|外澳|牛埔|風舶角|風舶|黃銅鑲邊|雕花青石|防蛀樟木|夜光貝母|寒水玉雙層|寒水玉|松香木|老青竹黃銅輪|老青竹|紅土光學黃銅|紅土光學|自來活水黃銅|自來活水|三水川雕花|文曜青石|防雨竹筒|防雨竹編|烏木內膽|烏木膽|(?:黃銅|青石|石雕|雕花|生鐵|熟鐵|精鋼|樟木|松木|烏木|紅木|鐵木|楠木|粗陶|白磁|竹編|單筒|捲線|貝母|木|竹|鐵|銅|石|陶)製?)/;
        for (var pi = 0; pi < 2; pi++) {
            var stripped = str.replace(prefixRegex, '').replace(/^[製式膽]/, '').trim();
            if (stripped.length >= 2 && stripped !== str) {
                str = stripped;
                if (knownMap[str]) {
                    return knownMap[str];
                }
            } else {
                break;
            }
        }
        if (/・當值櫃檯$/.test(str)) {
            return '當值櫃檯';
        }
        var bookMatch = str.match(/《([^》]+)》/);
        if (bookMatch && bookMatch[1]) {
            return bookMatch[1];
        }
        str = str.replace(/[《》]/g, '');
        if (str.length > 14 && str.indexOf('・') === -1) {
            return str.slice(-8);
        }
        return str;
    }

    function facilityCardsHtml(objects, npcs) {
        var facilities = (objects || []).filter(isFacility);
        var html = '';

        for (var i = 0; i < facilities.length; i++) {
            var obj = facilities[i];
            var rawLabel = labelOf(obj);
            var displayLabel = conciseObjectLabel(rawLabel);
            var icon = facilityIcon(obj);
            var catAttr = (obj && obj.category_tag) ? ' data-obj-category="' + _esc(obj.category_tag) + '"' : '';
            html += '<button type="button" class="sw-obj-card sw-facility-card sw-obj-item-card"' + catAttr + ' data-obj-id="' + _esc(obj.id) + '" title="' + _esc(rawLabel) + '">';
            html += '<span class="sw-facility-icon">' + icon + '</span> ';
            html += '<span class="sw-facility-name">' + _esc(displayLabel) + '</span>';
            html += '</button>';
        }

        if (!html) {
            return '<span class="sw-empty-hint">此處暫無百工設施或商貿器物</span>';
        }
        return html;
    }

    function discoveryCardsHtml(objects, harvestables, currentPlaceName) {
        // 僅篩選滿足 isMovableOrPortableObject 的物件（如 Move 通道、Take 隨身可攜帶地上物）
        var discoveries = (objects || []).filter(function (obj) {
            return isMovableOrPortableObject(obj);
        });
        var html = '';

        for (var i = 0; i < discoveries.length; i++) {
            var obj = discoveries[i];
            var rawLabel = labelOf(obj);
            var displayLabel = conciseObjectLabel(rawLabel);
            var cls = 'sw-obj-card sw-discovery-card';
            var icon = '✦';
            var actions = obj.actions || obj.sockets || [];
            var isMove = obj.move_to_room_id || (Array.isArray(actions) && actions.indexOf('Move') !== -1);
            var isTake = (Array.isArray(actions) && actions.indexOf('Take') !== -1) || obj.is_ground_item || obj.kind === 'ground_item';
            if (isMove) {
                cls += ' sw-obj-exit-card';
                icon = '🚪';
            } else if (isTake) {
                cls += ' sw-obj-item-card';
                icon = '📦';
            }
            var catAttr = (obj && obj.category_tag) ? ' data-obj-category="' + _esc(obj.category_tag) + '"' : '';
            html += '<button type="button" class="' + cls + '"' + catAttr + ' data-obj-id="' + _esc(obj.id) + '" title="' + _esc(rawLabel) + '">';
            html += '<span class="sw-discovery-icon">' + icon + '</span> ';
            html += '<span class="sw-discovery-name">' + _esc(displayLabel) + '</span>';
            html += '</button>';
        }

        return html;
    }

    function objectCardsHtml(objects) {
        var source = objects || [];
        var items = source.map(function (obj) {
            return { obj: obj, label: labelOf(obj), suffix: '' };
        });
        items = assignSuffixes(items);

        if (!items.length) return '';
        var html = '<div class="sw-object-contacts"><span class="sw-flow-lead">伸手可接觸</span><span class="sw-object-options">';
        for (var i = 0; i < items.length; i++) {
            var item    = items[i];
            var obj     = item.obj;
            var display = item.label + (item.suffix || '');
            var cls     = 'sw-obj-card sw-object-contact';
            if (obj.type === 'npc')      cls += ' sw-obj-npc';
            else if (obj.type === 'item') cls += ' sw-obj-item-card';
            else if (obj.type === 'resource') cls += ' sw-obj-resource';
            var catAttr = (obj && obj.category_tag) ? ' data-obj-category="' + _esc(obj.category_tag) + '"' : '';
            html += '<button type="button" class="' + cls + '"' + catAttr + ' data-obj-id="' + _esc(obj.id) + '">' +
                    _esc(display) + '</button>';
            if (i < items.length - 1) html += '<span class="sw-object-punctuation" aria-hidden="true">、</span>';
        }
        html += '<span class="sw-object-punctuation" aria-hidden="true">。</span></span></div>';
        return html;
    }

    function visibleActivity(text) {
        var activity = String(text || '').trim();
        activity = activity.replace(/（[^）]*側\s*[·・]\s*[^）]*）/g, '').trim();
        if (activity === '正前往休息處') return '正往別處走';
        if (activity === '正前往有人聚集的地方') return '正往人聲處走';
        return activity;
    }
    function concisePersonLabel(displayName, state) {
        var raw = String(displayName || '行人').trim();
        raw = raw.replace(/^一名/, '');
        if (state !== 'named') {
            raw = raw.replace(/(?:[0-9]+|[甲乙丙丁戊己庚辛壬癸])$/, '');
        }
        var prefixPattern = /^(文曜學院|文曜坊間|文曜|三水食堂|三水口|分家巷|講堂街|公議堂|鐘樓|診所|商行|浴場|聯運|入城|三水外坊|講堂外坊|百工外坊|西門外坊|北門外街|南門外街|東門外街|西門外街|北內門坊|南內門坊|東內門坊|西內門坊)/;
        if (raw.indexOf('｜') !== -1 || raw.indexOf('|') !== -1) {
            var parts = raw.split(/[｜|]/);
            var titlePart = (parts[0] || '').trim();
            var namePart = (parts[1] || '').trim();
            if (state === 'named' && namePart) {
                return namePart;
            }
            if (namePart && !titlePart) return namePart;
            if (titlePart) {
                var cleanTitle = titlePart.replace(prefixPattern, '');
                return (cleanTitle.length >= 2 ? cleanTitle : titlePart) || namePart || '行人';
            }
            return namePart || '行人';
        }
        if (state === 'named') return raw;
        var stripped = raw.replace(prefixPattern, '');
        return (stripped.length >= 2 ? stripped : raw) || '行人';
    }

    function buildCurrentExitsLogHtml() {
        var exits = exitItemsFromGrid();
        if (!exits || !exits.length) return '';
        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var currentTitle = splitPlaceLabel(mapData.currentRoomName || '').title;
        var html = '<span class="log-exits-lead">四方去路：</span>';
        for (var i = 0; i < exits.length; i++) {
            var ex = exits[i];
            var refName = refineExitName(ex, currentTitle);
            html += '<button type="button" class="log-object-action log-exit-btn" data-move-dir="' + _esc(ex.dir) + '" data-move-target="' + _esc(ex.target || '') + '">【' + _esc(ex.dir) + '・' + _esc(refName) + '】</button> ';
        }
        return html.trim();
    }

    function appendCurrentExitsToLog() {
        var html = buildCurrentExitsLogHtml();
        if (!html) return;
        if (window.SwLog && window.SwLog.appendObjectActionsLine) {
            window.SwLog.appendObjectActionsLine(html);
        }
    }

    function npcCardsHtml(npcs, others) {
        var list = (npcs || []).slice();
        var otherPlayers = Array.isArray(others) ? others : [];

        for (var oi = 0; oi < otherPlayers.length; oi++) {
            var p = otherPlayers[oi];
            var isParty = !!(p.is_party_member || p.is_companion || p.party);
            list.push({
                npc_id: p.id,
                display_name: (p.display_name && !/^(npc_|wt_|wt-|ambient_|shanhai_|grid:)/.test(p.display_name)) ? p.display_name : ((p.id && !/^(npc_|wt_|wt-|ambient_|shanhai_|grid:)/.test(p.id)) ? p.id : '旅人'),
                card_state: isParty ? 'companion' : 'stranger',
                is_party_member: isParty,
                kind: 'player',
                current_activity: p.current_activity || '駐足四顧'
            });
        }

        // 篩選具體人物（忽略 crowd）
        var filtered = [];
        for (var fi = 0; fi < list.length; fi++) {
            if (list[fi].kind !== 'crowd') {
                filtered.push(list[fi]);
            }
        }

        if (!filtered.length) {
            return '<span class="sw-empty-hint">四顧無人</span>';
        }

        var html = '<div class="sw-people-list">';
        var nameCounts = {};
        var renderedNames = [];
        for (var ci = 0; ci < filtered.length; ci++) {
            var cNpc = filtered[ci];
            var cKind = cNpc.kind || 'npc';
            var cState = cNpc.card_state || 'stranger';
            var cDisp = cNpc.display_name || '行人';
            var shortName = (cKind === 'player')
                ? (cNpc.is_party_member ? ('伴・' + cDisp) : cDisp)
                : concisePersonLabel(cDisp, cState);
            renderedNames.push(shortName);
            nameCounts[shortName] = (nameCounts[shortName] || 0) + 1;
        }
        var nameSeen = {};
        for (var i = 0; i < filtered.length; i++) {
            var npc = filtered[i];
            var state = npc.card_state || 'stranger';
            var kind = npc.kind || 'npc';
            var finalName = renderedNames[i];
            if (nameCounts[finalName] > 1) {
                var seqIdx = nameSeen[finalName] || 0;
                nameSeen[finalName] = seqIdx + 1;
                finalName = finalName + ' ' + String(seqIdx + 1);
            }
            var cls = (kind === 'player') ? 'sw-obj-player' : 'sw-obj-npc';
            var activity = visibleActivity(npc.current_activity);
            html += '<button type="button" class="sw-obj-card sw-person-row ' + cls + ' card-' + _esc(state) + '"' +
                    ' data-entity-id="' + _esc(npc.npc_id) + '" data-obj-id="' + _esc(npc.npc_id) + '" data-card-state="' + _esc(state) + '"' +
                    (activity ? (' data-activity="' + _esc(activity) + '"') : '') + '>' +
                    '<span class="sw-person-name">' + _esc(finalName) + '</span>' +
                    '</button>';
        }
        html += '</div>';
        return html;
    }

    function composeSurface(npcs, objects) {
        var contacts = npcCardsHtml(npcs) + objectCardsHtml(objects);
        return '<div class="sw-contact-column">' + contacts + '</div>';
    }

    // ── 空間文字場畫布與單一浮動框狀態 ─────────────────────────────────
    var _roomId = '';
    var _prevRoomId = '';
    var _roomEnterTime = 0;
    var _prevTokensMap = {};
    var _tokenPositionsByRoom = {};
    var _transitHints = {};
    var _activePopoverToken = null;
    var _activeNpcCallouts = {};
    var _activeNpcChats = {};
    var _allLiveNpcIdsInRoom = {};
    var _prevAllLiveNpcIdsInRoom = {};
    var _totalLiveNpcCount = 0;
    var _npcBreathCooldownById = {};
    function hashStr(str) {
        var h = 0x811c9dc5;
        var s = String(str || '');
        for (var i = 0; i < s.length; i++) {
            h ^= s.charCodeAt(i);
            h = Math.imul(h, 0x01000193);
        }
        return h >>> 0;
    }

    function recordTransitHint(npcNameOrId, kind, dirZh) {
        if (!npcNameOrId) return;
        var raw = String(npcNameOrId).trim();
        var clean = raw.replace(/^一名/, '');
        var now = Date.now();
        _transitHints[clean] = { kind: kind, dir: dirZh, time: now };
        _transitHints[raw] = { kind: kind, dir: dirZh, time: now };
        var parts = raw.split(/[|｜・]/);
        if (parts.length > 1) {
            var pName = parts[parts.length - 1].trim();
            if (pName) _transitHints[pName] = { kind: kind, dir: dirZh, time: now };
        }
    }
    window.SwRecordTransitHint = recordTransitHint;

    function estimateTokenHalfBox(labelLen) {
        var canvasEl = document.getElementById('sw-room-canvas');
        var cw = (canvasEl && canvasEl.clientWidth > 120) ? canvasEl.clientWidth : ((typeof window !== 'undefined' && window.innerWidth) ? (window.innerWidth - 36) : 800);
        var ch = (canvasEl && canvasEl.clientHeight > 120) ? canvasEl.clientHeight : 320;
        var isNarrow = cw <= 520;
        var len = Math.max(2, Number(labelLen) || 4);
        var charPx = isNarrow ? 14.5 : 15.5;
        var estWidthPx = Math.min(cw * 0.66, len * charPx + 26);
        var estHeightPx = isNarrow ? 44 : 48;
        var halfW = Math.min(34, Math.max(6.0, (estWidthPx / 2 / cw) * 100));
        var halfH = Math.min(12.5, Math.max(5.5, (estHeightPx / 2 / ch) * 100));
        return { halfW: halfW, halfH: halfH, bucket: isNarrow ? 'm' : 'd' };
    }

    function checkCollisionWithPlaced(x, y, box, placedPositions) {
        var minClearance = 999;
        for (var k in placedPositions) {
            var p = placedPositions[k];
            var reqDx = (box.halfW + (p.halfW || 8)) + 2.6;
            var reqDy = (box.halfH + (p.halfH || 6)) + 2.6;
            var dx = Math.abs(x - p.x);
            var dy = Math.abs(y - p.y);
            if (dx < reqDx && dy < reqDy) {
                var c = Math.max(dx / reqDx, dy / reqDy);
                if (c < minClearance) minClearance = c;
            }
        }
        return minClearance;
    }

    function computeStablePosition(roomId, tokenKey, labelLen, placedPositions, isAnchored, kind) {
        return { x: 50, y: 50, halfW: 10, halfH: 10 };
    }

    function getEdgeCoords(dir, targetPos, placedPositions, tokenKey) {
        var d = dir || '南';
        var baseCoords = { x: targetPos.x, y: 95 };
        if (d === '北') {
            baseCoords = { x: targetPos.x, y: 5 };
        } else if (d === '南') {
            baseCoords = { x: targetPos.x, y: 95 };
        } else if (d === '西') {
            baseCoords = { x: 5, y: targetPos.y };
        } else if (d === '東') {
            baseCoords = { x: 95, y: targetPos.y };
        }
        if (!placedPositions) return baseCoords;

        var box = { halfW: 16, halfH: 9 };
        var testX = baseCoords.x;
        var testY = baseCoords.y;
        var bestX = testX;
        var bestY = testY;
        var bestClearance = -1;

        // 檢查邊緣位置是否與既有 token 重疊，若重疊則沿邊界向兩側微調推移，避免遮蔽日晷石柱或居民
        for (var offset = 0; offset <= 42; offset += 6) {
            var signs = (offset === 0) ? [0] : [1, -1];
            for (var si = 0; si < signs.length; si++) {
                var s = signs[si];
                var curX = testX;
                var curY = testY;
                if (d === '北' || d === '南') {
                    curX = Math.max(16, Math.min(84, testX + s * offset));
                } else {
                    curY = Math.max(12, Math.min(88, testY + s * offset));
                }
                var clr = checkCollisionWithPlaced(curX, curY, box, placedPositions);
                if (clr >= 1.0) {
                    return { x: curX, y: curY };
                }
                if (clr > bestClearance) {
                    bestClearance = clr;
                    bestX = curX;
                    bestY = curY;
                }
            }
        }
        return { x: bestX, y: bestY };
    }

    function resolveTransitDirection(token, kind) {
        var keys = [token.id, token.label, token.name];
        var now = Date.now();
        for (var i = 0; i < keys.length; i++) {
            var k = keys[i];
            if (k && _transitHints[k]) {
                var hint = _transitHints[k];
                if (now - hint.time < 8000 && (!kind || hint.kind === kind)) {
                    return hint.dir;
                }
            }
        }
        var dirs = ['北', '南', '東', '西'];
        var h = hashStr(token.key || token.id || token.label || 'dir');
        return dirs[h % 4];
    }

    var _tokenEnterTime = {};
    var _strangerSlotById = {};

    function conciseMicroState(rawAct, kind, label, isBeast, tokenKey) {
        if (kind === 'npc') {
            if (label && _transitHints[label] && (Date.now() - _transitHints[label].time <= 9000)) {
                var th = _transitHints[label];
                if (th.dir) {
                    if (th.kind === 'enter' || th.type === 'enter') {
                        return '自' + th.dir + '步入';
                    }
                    if (th.kind === 'leave' || th.type === 'leave') {
                        return '往' + th.dir + '行去';
                    }
                }
            }
            if (label && /櫃檯當值|當值|掌事|主廚|醫師|匠師|鐵匠|木匠|教席/.test(label)) {
                return '櫃檯當值';
            }
            if (label && /（當值）/.test(label)) {
                return '駐場當值';
            }
            if (tokenKey && _tokenEnterTime[tokenKey] && (Date.now() - _tokenEnterTime[tokenKey] < 9000)) {
                return '邁步穿行';
            }
            var meta = window.SwCurrentRoomMeta || {
                terrain: window.__swRoomTerrain,
                venue_kind: window.__swRoomVenueKind,
                is_indoor: window.__swRoomIsIndoor
            };
            var terrainStr = String(meta.terrain || window.__swRoomTerrain || '').toLowerCase();
            var isBridgeOrWater = terrainStr === 'bridge' || terrainStr === 'water';
            var isIndoorPublic = Boolean(meta.venue_kind) || Boolean(meta.is_indoor) || Boolean(window.__swRoomVenueKind) || Boolean(window.__swRoomIsIndoor);
            var isIndoorRoom = typeof meta.is_indoor === 'boolean' ? meta.is_indoor : Boolean(window.__swRoomIsIndoor);
            var dayPool;
            var nightOutdoorPool;
            if (isIndoorPublic) {
                dayPool = ['歇腳小坐', '駐足打量', '緩步穿行', '靜候片刻'];
                nightOutdoorPool = ['歇腳小坐', '駐足打量', '緩步穿行', '靜候片刻'];
            } else if (isBridgeOrWater) {
                dayPool = ['駐足歇腳', '憑欄遠眺', '緩步徐行', '駐足四顧'];
                nightOutdoorPool = ['夜立憑欄', '駐足歇腳', '凝望夜色', '緩步徐行'];
            } else {
                dayPool = ['駐足歇腳', '緩步徐行', '駐足四顧', '沿街穿行'];
                nightOutdoorPool = ['駐足歇腳', '凝望夜色', '緩步徐行', '沿街夜行'];
            }
            var appEl = document.getElementById('app') || document.getElementById('app-grid');
            var tide = appEl ? appEl.getAttribute('data-tide') : 'noon';
            var isProceduralKey = /^npc:npc_/.test(String(tokenKey || ''));
            if (rawAct) {
                var s = String(rawAct);
                if (/打盹|入睡|酣睡|熟睡|睡眠|歇息|養回一點體力|閉目|散步|閒逛|趕路|前往|走去|穿行|行色匆匆|邁步|動身|回返/.test(s)) {
                    if (!isProceduralKey && /趕路|前往|走去|穿行|行色匆匆|邁步|動身|回返/.test(s)) return '邁步穿行';
                    if (tide && tide !== 'night') {
                        return dayPool[hashStr(label || tokenKey || 'npc') % dayPool.length];
                    }
                    if (!isIndoorRoom) {
                        return nightOutdoorPool[hashStr(label || tokenKey || 'npc') % nightOutdoorPool.length];
                    }
                    return '閉目養神';
                }
                if (/指引|認路|解答|接待|迎客/.test(s)) return '指引商旅';
                if ((isIndoorPublic || isIndoorRoom) && /講堂|讀譜|授課|教學/.test(s)) return '講學授課';
                if (/巡邏|巡視|守衛|警戒|站崗|按刀/.test(s)) return '巡視戒備';
                if (/叫賣|兜售|吆喝|擺攤|整理貨|清點|算帳|帳冊/.test(s)) return '理貨招呼';
                if (/交談|閒聊|議論|搭話|寒暄|商議/.test(s)) return '低聲交談';
                if (/飲茶|喝湯|用膳|進食|喝酒|品茗|買碗/.test(s)) return '歇腳飲饌';
                if ((isIndoorPublic || isIndoorRoom) && /讀書|翻閱|誦讀|抄寫|批閱|研讀/.test(s)) return '凝神展卷';
                if (/打鐵|鍛造|劈柴|生火|熬煮|烹煮|勞作|挑水|垂釣|釣魚|米漿|灶前/.test(s)) return '專注勞作';
                if (/張望|四顧|駐足|遠眺|凝望|打量/.test(s)) return '駐足四顧';
                var cleaned = s.replace(/^正在/, '').replace(/^在[^，,。]+[,，]/, '').replace(/[。！!？?，,、\s]/g, '').trim();
                if (cleaned.length >= 2 && cleaned.length <= 5) return cleaned;
            }
            if (isBeast) return '山林游走';
            if (tide === 'night' && !isIndoorRoom) {
                return nightOutdoorPool[hashStr(label || tokenKey || 'npc') % nightOutdoorPool.length];
            }
            return dayPool[hashStr(label || tokenKey || 'npc') % dayPool.length];
        }
        if (kind === 'player') return '駐足同處';
        return '';
    }
    var _inlineOpenTokenKey = null;

    function closeAllInlineTokens(canvasEl) {
        if (_inlineOpenTokenKey && _inlineOpenTokenKey.indexOf('npc:') === 0) {
            var npcId = _inlineOpenTokenKey.slice(4);
            if (!(_activePopoverToken && _activePopoverToken.id === npcId) && typeof window.gameSend === 'function') {
                window.gameSend({ type: 'unfocus_entity', entity_id: npcId });
            }
        }
        _inlineOpenTokenKey = null;
        var root = canvasEl || document.getElementById('sw-room-canvas');
        if (!root) return;
        var openEls = root.querySelectorAll('.sw-canvas-token.is-inline-open');
        for (var i = 0; i < openEls.length; i++) {
            openEls[i].classList.remove('is-inline-open');
        }
    }

    function populateInlineActionChips(chipsContainer, token) {
        if (!chipsContainer || !token) return;
        var targetId = token.id;
        var targetName = token.label || token.name || targetId;
        var rawActs = (token.data && (token.data.actions || token.data.sockets)) || [];
        var actionsSigList = [];

        if (token.kind === 'npc' || token.kind === 'player') {
            if (isWildBeastToken(targetId, targetName)) {
                actionsSigList = [{ code: 'Subdue', label: '生擒制伏' }, { code: 'Slay', label: '出手獵殺' }];
            } else {
                actionsSigList = [
                    { code: 'Greet', label: '寒暄' },
                    { code: 'Ask', label: '打聽' },
                    { code: 'Trade', label: '交易' },
                    { code: 'Chat', label: '詳談…' }
                ];
            }
        } else {
            var nonLook = filterObjectSockets(rawActs);
            var maxInline = Math.min(3, nonLook.length);
            for (var si = 0; si < maxInline; si++) {
                var actCode = nonLook[si];
                var obj = token.data;
                var objTag = (obj && obj.tag) || '';
                var objTags = (obj && obj.tags) || [];
                var actLabel = resolveActionLabelZh(actCode, obj || targetName, targetId, objTag, objTags);
                actionsSigList.push({ code: actCode, label: actLabel });
            }
            if (nonLook.length > 3) {
                actionsSigList.push({ code: 'More', label: '更多…' });
            }
        }

        var sig = String(token.id || '') + '|' + actionsSigList.map(function (a) { return a.code + ':' + a.label; }).join(',');
        if (chipsContainer.getAttribute('data-chips-sig') === sig && chipsContainer.children.length > 0) {
            return;
        }
        chipsContainer.setAttribute('data-chips-sig', sig);
        chipsContainer.innerHTML = '';

        function addChip(label, onClickFn, titleText) {
            var chip = document.createElement('button');
            chip.type = 'button';
            chip.className = 'sw-inline-act-chip';
            chip.textContent = label;
            if (titleText) chip.title = titleText;
            chip.addEventListener('click', function (ev) {
                ev.stopPropagation();
                ev.preventDefault();
                onClickFn();
            });
            chipsContainer.appendChild(chip);
        }
        if (token.kind === 'npc' || token.kind === 'player') {
            if (isWildBeastToken(targetId, targetName)) {
                addChip('生擒制伏', function () {
                    if (window.gameSend) window.gameSend({ type: 'do_action', entity_id: targetId, action: 'Subdue' });
                });
                addChip('出手獵殺', function () {
                    if (window.gameSend) window.gameSend({ type: 'do_action', entity_id: targetId, action: 'Slay' });
                });
            } else {
                addChip('寒暄', function () {
                    closeAllInlineTokens(document.getElementById('sw-room-canvas'));
                    var canvasRoot = document.getElementById('sw-room-canvas');
                    var tokEl = canvasRoot ? canvasRoot.querySelector('.sw-canvas-token[data-token-key="' + token.key + '"]') : null;
                    if (tokEl) {
                        var microSpan = tokEl.querySelector('.sw-token-micro-state');
                        if (microSpan) {
                            microSpan.textContent = '駐足交談';
                            microSpan.hidden = false;
                            microSpan.style.display = '';
                        }
                    }
                    token.microState = '駐足交談';
                    SwOpenObjectPopover(token);
                    var playerLine = '你對【' + (targetName || targetId) + '】說：「寒暄問候」';
                    var fmt = window.gameFormatNarrative || function (t) { return t; };
                    var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
                    appendNarr(fmt(playerLine), 'Talk');
                    if (window.gameSend) {
                        window.gameSend({ type: 'do_action', entity_id: targetId, action: 'Talk', player_input: '寒暄問候' });
                        if (window.SwInteractions && window.SwInteractions.appendLogPendingTalk) {
                            window.SwInteractions.appendLogPendingTalk();
                        }
                    }
                }, '直接搭話寒暄');
                addChip('打聽', function () {
                    closeAllInlineTokens(document.getElementById('sw-room-canvas'));
                    var canvasRoot = document.getElementById('sw-room-canvas');
                    var tokEl = canvasRoot ? canvasRoot.querySelector('.sw-canvas-token[data-token-key="' + token.key + '"]') : null;
                    if (tokEl) {
                        var microSpan = tokEl.querySelector('.sw-token-micro-state');
                        if (microSpan) {
                            microSpan.textContent = '駐足交談';
                            microSpan.hidden = false;
                            microSpan.style.display = '';
                        }
                    }
                    token.microState = '駐足交談';
                    SwOpenObjectPopover(token);
                    var playerLine = '你對【' + (targetName || targetId) + '】說：「打聽近況」';
                    var fmt = window.gameFormatNarrative || function (t) { return t; };
                    var appendNarr = (window.SwLog && window.SwLog.appendNarrative) || function () {};
                    appendNarr(fmt(playerLine), 'Talk');
                    if (window.gameSend) {
                        window.gameSend({ type: 'do_action', entity_id: targetId, action: 'Talk', player_input: '打聽近況' });
                        if (window.SwInteractions && window.SwInteractions.appendLogPendingTalk) {
                            window.SwInteractions.appendLogPendingTalk();
                        }
                    }
                }, '打聽當地近況');
                addChip('交易', function () {
                    if (window.gameSend) window.gameSend({ type: 'do_action', entity_id: targetId, action: 'Trade', player_input: '' });
                }, '探詢交易');
                addChip('詳談…', function () {
                    SwOpenObjectPopover(token);
                    if (window.SwInteractions && typeof window.SwInteractions.appendTalkInputRow === 'function') {
                        window.SwInteractions.appendTalkInputRow(targetId, targetName);
                    }
                }, '展開自由對話與完整互動');
            }
        } else {
            var rawActs = (token.data && (token.data.actions || token.data.sockets)) || [];
            var nonLook = filterObjectSockets(rawActs);
            var maxInline = Math.min(3, nonLook.length);
            for (var i = 0; i < maxInline; i++) {
                (function (actCode) {
                    var obj = token.data;
                    var objTag = (obj && obj.tag) || '';
                    var objTags = (obj && obj.tags) || [];
                    var actLabel = resolveActionLabelZh(actCode, obj || targetName, targetId, objTag, objTags);
                    addChip(actLabel, function () {
                        if (window.gameSend) {
                            var moveRoomId = obj && obj.move_to_room_id;
                            var entId = (actCode === 'Move' && moveRoomId) ? moveRoomId : targetId;
                            window.gameSend({ type: 'do_action', entity_id: entId, action: actCode });
                        }
                    });
                })(nonLook[i]);
            }
            if (nonLook.length > 3) {
                addChip('更多…', function () {
                    SwOpenObjectPopover(token);
                });
            }
        }
    }

    function syncTokenDomContent(tokenEl, token) {
        tokenEl._swToken = token;
        tokenEl.setAttribute('data-token-key', token.key);
        tokenEl.setAttribute('data-obj-id', token.id);
        tokenEl.setAttribute('data-entity-id', token.id);
        tokenEl.setAttribute('data-obj-tag', (token.data && token.data.tag) || '');
        var obj = token.data;
        if (obj && obj.category_tag) {
            tokenEl.dataset.objCategory = String(obj.category_tag);
            tokenEl.setAttribute('data-obj-category', String(obj.category_tag));
        } else {
            tokenEl.removeAttribute('data-obj-category');
            if (tokenEl.dataset) {
                delete tokenEl.dataset.objCategory;
            }
        }
        if (obj && Array.isArray(obj.attribute_tags) && obj.attribute_tags.length > 0) {
            tokenEl.dataset.objAttributes = obj.attribute_tags.join(',');
            tokenEl.setAttribute('data-obj-attributes', obj.attribute_tags.join(','));
        } else {
            tokenEl.removeAttribute('data-obj-attributes');
            if (tokenEl.dataset) {
                delete tokenEl.dataset.objAttributes;
            }
        }
        if (token.data && Array.isArray(token.data.tags)) {
            tokenEl.setAttribute('data-obj-tags', token.data.tags.join(','));
        } else {
            tokenEl.removeAttribute('data-obj-tags');
        }
        var lblSpan = tokenEl.querySelector('.sw-token-label, .sw-token-title');
        if (!lblSpan) {
            lblSpan = document.createElement('span');
            lblSpan.className = 'sw-token-label sw-token-title';
            tokenEl.appendChild(lblSpan);
        } else if (!lblSpan.classList.contains('sw-token-title')) {
            lblSpan.classList.add('sw-token-title');
        }
        lblSpan.textContent = token.label;

        var microSpan = tokenEl.querySelector('.sw-token-micro-state, .sw-token-sub');
        if (!microSpan) {
            microSpan = document.createElement('span');
            microSpan.className = 'sw-token-micro-state sw-token-sub';
            tokenEl.appendChild(microSpan);
        } else if (!microSpan.classList.contains('sw-token-sub')) {
            microSpan.classList.add('sw-token-sub');
        }
        microSpan.textContent = token.microState || '';
        microSpan.hidden = !token.microState;
        microSpan.style.display = token.microState ? '' : 'none';

        var actsWrap = tokenEl.querySelector('.sw-token-inline-acts');
        if (!actsWrap) {
            actsWrap = document.createElement('div');
            actsWrap.className = 'sw-token-inline-acts';
            tokenEl.appendChild(actsWrap);
        }
        if (_inlineOpenTokenKey === token.key) {
            tokenEl.classList.add('is-inline-open');
            populateInlineActionChips(actsWrap, token);
        } else {
            tokenEl.classList.remove('is-inline-open');
        }
    }

    function handleTokenPrimaryTap(tokenEl, token) {
        if (!tokenEl || !token || tokenEl.classList.contains('is-leaving')) return;
        var canvasEl = document.getElementById('sw-room-canvas');

        // 清除該角色頭頂搭話氣泡（若有）
        if (token.id && _activeNpcCallouts[token.id]) {
            delete _activeNpcCallouts[token.id];
            tokenEl.classList.remove('is-calling', 'is-calling-combat');
            var p = tokenEl.querySelector('.sw-token-speech-pill');
            if (p) p.remove();
        }

        if (token.kind === 'facility' || token.kind === 'item') {
            // 點擊環境物件即為查看細閱：直接觸發後端 Look，回傳完整描寫至「當下二息」
            if (token.id !== '__room_explore__' && window.gameSend) {
                window.gameSend({ type: 'do_action', entity_id: token.id, action: 'Look' });
            }
            var rawActs = (token.data && (token.data.actions || token.data.sockets)) || [];
            var extraActs = filterObjectSockets(rawActs);
            if (extraActs.length === 0) {
                closeAllInlineTokens(canvasEl);
                return;
            }
            if (tokenEl.classList.contains('is-inline-open')) {
                closeAllInlineTokens(canvasEl);
                return;
            }
            closeAllInlineTokens(canvasEl);
            _inlineOpenTokenKey = token.key;
            tokenEl.classList.add('is-inline-open');
            var actsWrap = tokenEl.querySelector('.sw-token-inline-acts');
            if (actsWrap) {
                populateInlineActionChips(actsWrap, token);
            }
            return;
        }

        // 若已經處於原位微綻狀態，再次點擊主標籤開啟完整詳情框
        if (tokenEl.classList.contains('is-inline-open')) {
            SwOpenObjectPopover(token);
            return;
        }

        closeAllInlineTokens(canvasEl);
        _inlineOpenTokenKey = token.key;
        tokenEl.classList.add('is-inline-open');
        var actsWrap = tokenEl.querySelector('.sw-token-inline-acts');
        if (actsWrap) {
            populateInlineActionChips(actsWrap, token);
        }

        // 一觸即感：觸發後端 Look 以直接回傳真實環境／人物感官描寫至「當下二息」，並立即使被點選人物短暫駐足
        if (token.id !== '__room_explore__' && window.gameSend) {
            if (token.kind === 'npc') {
                window.gameSend({ type: 'focus_entity', entity_id: token.id });
                _lastFocusSendTime = Date.now();
            }
            if (token.kind === 'npc' && token.data && (token.data.card_state === 'stranger' || token.data.card_state === 'observed')) {
                window.gameSend({ type: 'observe', entity_id: token.id });
            }
            window.gameSend({ type: 'do_action', entity_id: token.id, action: 'Look' });
        }
    }
    function collectCurrentTokens() {
        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var currentRoomId = mapData.currentRoomId || '';
        var myId = window.myPlayerId || '';
        var tokens = [];
        var seenKeys = {};

        var npcs = _currentNpcs || [];
        var validNpcs = [];
        var npcLabelCounts = {};
        for (var ni = 0; ni < npcs.length; ni++) {
            var rawNpc = npcs[ni];
            if (rawNpc.kind === 'crowd') continue;
            var rawId = rawNpc.npc_id || rawNpc.id || ('npc_' + ni);
            var rawKey = 'npc:' + rawId;
            if (seenKeys[rawKey]) continue;
            seenKeys[rawKey] = true;
            var rawName = rawNpc.display_name || '行人';
            if (rawNpc.card_state !== 'named' && rawName.indexOf('街坊居民') !== -1) {
                rawName = '陌生人';
            }
            var rawLabel = concisePersonLabel(rawName, rawNpc.card_state);
            npcLabelCounts[rawLabel] = (npcLabelCounts[rawLabel] || 0) + 1;
            validNpcs.push({ npc: rawNpc, id: rawId, key: rawKey, name: rawName, label: rawLabel });
        }
        // 穩定維護陌生人編號槽位：入場初刻（500ms 內）依序排定 1..N，其後駐留期間絕不因他人離場而原位改號
        var isInitialSettle = (_roomEnterTime > 0) && ((Date.now() - _roomEnterTime) < 500);
        if (isInitialSettle) {
            _strangerSlotById = {};
        }
        var presentNpcIds = {};
        var usedStrangerSlots = {};
        for (var vi = 0; vi < validNpcs.length; vi++) {
            var vid = validNpcs[vi].id;
            presentNpcIds[vid] = true;
            var existingSlot = _strangerSlotById[vid];
            if (existingSlot) {
                if (usedStrangerSlots[existingSlot]) {
                    delete _strangerSlotById[vid];
                } else {
                    usedStrangerSlots[existingSlot] = true;
                }
            }
        }
        for (var oldSid in _strangerSlotById) {
            if (!presentNpcIds[oldSid]) {
                delete _strangerSlotById[oldSid];
            }
        }

        var npcLabelSeen = {};
        for (var i = 0; i < validNpcs.length; i++) {
            var item = validNpcs[i];
            var nName = item.name;
            var nLabel = item.label;
            if (npcLabelCounts[nLabel] > 1) {
                if (nLabel === '陌生人') {
                    var slotNum = _strangerSlotById[item.id];
                    if (!slotNum) {
                        slotNum = 1;
                        while (usedStrangerSlots[slotNum]) {
                            slotNum++;
                        }
                        _strangerSlotById[item.id] = slotNum;
                        usedStrangerSlots[slotNum] = true;
                    }
                    nLabel = '陌生人 ' + String(slotNum);
                    if (nName === '陌生人' || /^陌生人\d*$/.test(nName)) {
                        nName = nLabel;
                    }
                } else {
                    var seqIdx = npcLabelSeen[nLabel] || 0;
                    npcLabelSeen[nLabel] = seqIdx + 1;
                    nLabel = nLabel + ' ' + String(seqIdx + 1);
                }
            }
            var act = visibleActivity(item.npc.current_activity);
            var isBeast = isWildBeastToken(item.id, nName);
            var sub = act || '';
            var micro = conciseMicroState(act, 'npc', nLabel, isBeast, item.key);
            tokens.push({
                key: item.key,
                id: item.id,
                name: nName,
                label: nLabel,
                subtitle: sub,
                microState: micro,
                kind: 'npc',
                tokenClass: 'sw-token-npc',
                data: item.npc
            });
        }

        var otherPlayers = (mapData.entities || []).filter(function (e) {
            return e.id !== myId && (e.kind === 'player' || e.kind === 'Player');
        });
        for (var pi = 0; pi < otherPlayers.length; pi++) {
            var p = otherPlayers[pi];
            var pId = p.id;
            var pKey = 'player:' + pId;
            if (seenKeys[pKey]) continue;
            seenKeys[pKey] = true;
            var pName = p.display_name || p.name || '旅人';
            tokens.push({
                key: pKey,
                id: pId,
                name: pName,
                label: pName,
                subtitle: '',
                microState: '駐足同處',
                kind: 'player',
                tokenClass: 'sw-token-npc sw-token-player',
                data: p
            });
        }

        var objs = _currentObjects || [];
        for (var oi = 0; oi < objs.length; oi++) {
            var obj = objs[oi];
            if (!obj || !obj.id) continue;
            if (obj.move_to_room_id && currentRoomId && obj.move_to_room_id === currentRoomId) {
                continue;
            }
            if (!isMovableOrPortableObject(obj)) {
                continue;
            }
            var oId = obj.id;
            var oKey = 'obj:' + oId;
            if (seenKeys[oKey]) continue;
            seenKeys[oKey] = true;

            var rawLabel = labelOf(obj);
            var oLabel = conciseObjectLabel(rawLabel);
            var fac = isFacility(obj);
            var kind = fac ? 'facility' : 'item';
            var sub = '';
            var tokClass = (fac ? 'sw-token-facility' : 'sw-token-item') + ' sw-token-object';
            tokens.push({
                key: oKey,
                id: oId,
                name: rawLabel,
                label: oLabel,
                subtitle: sub,
                microState: conciseMicroState('', kind, oLabel, false),
                kind: kind,
                tokenClass: tokClass,
                data: obj
            });
        }
        tokens.sort(function (a, b) {
            var aM = String(a.label || a.name || '').match(/^陌生人\s*(\d+)$/);
            var bM = String(b.label || b.name || '').match(/^陌生人\s*(\d+)$/);
            if (aM && bM) {
                return parseInt(aM[1], 10) - parseInt(bM[1], 10);
            }
            return 0;
        });
        return tokens;
    }

    function renderRoomCanvas() {
        if (window.SwRoomDesc && typeof window.SwRoomDesc.syncL3Objects === 'function') {
            var G0 = window.SwGrid;
            var md0 = (G0 && G0.mapData) ? G0.mapData : {};
            window.SwRoomDesc.syncL3Objects(_currentObjects, md0.harvestables || []);
        }
        var canvasEl = document.getElementById('sw-room-canvas');
        if (!canvasEl) return;
        var emptyEl = document.getElementById('sw-room-canvas-empty');

        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var coordKey = (typeof mapData.playerX === 'number' && typeof mapData.playerY === 'number')
            ? (mapData.playerX + ',' + mapData.playerY)
            : ((mapData.player && typeof mapData.player.x === 'number')
                ? (mapData.player.x + ',' + mapData.player.y)
                : (mapData.currentRoomId || _roomId || 'default_room'));
        var inPrivate = !!mapData.in_private_room;
        var rName = mapData.currentRoomName || (document.getElementById('sw-place-name') ? document.getElementById('sw-place-name').textContent.trim() : '');
        var currentRoomId = coordKey;
        var hasNumericCoord = (typeof mapData.playerX === 'number' && typeof mapData.playerY === 'number')
            || (mapData.player && typeof mapData.player.x === 'number');
        var currentRoomKey = hasNumericCoord
            ? (coordKey + '|' + (inPrivate ? 'private' : 'public'))
            : (coordKey + '|' + (inPrivate ? 'private' : 'public') + '|' + rName);
        var isSameRoom = (_prevRoomId === currentRoomKey);
        if (!isSameRoom) {
            _roomEnterTime = Date.now();
            _activeNpcCallouts = {};
            _activeNpcChats = {};
            _strangerSlotById = {};
        }
        var allowTransitAnim = isSameRoom && ((Date.now() - _roomEnterTime) > 1200);
        _roomId = currentRoomId;
        var tokens = collectCurrentTokens();
        if (emptyEl) {
            emptyEl.hidden = true;
        }
        canvasEl.classList.toggle('is-empty-canvas', tokens.length === 0);
        canvasEl.hidden = false;

        // ── 空間空場提示（.sw-canvas-empty-hint）與擁擠徽章（.sw-canvas-crowd-badge）──
        var existingEmptyHint = canvasEl.querySelector('.sw-canvas-empty-hint');
        if (tokens.length === 0) {
            var appEl = document.getElementById('app') || document.getElementById('app-grid');
            var isNight = (appEl && appEl.getAttribute('data-tide') === 'night');
            var emptyText = isNight ? '夜色靜謐，四下暫無行旅駐足' : '街面清朗，四下暫無駐足行人';
            if (!existingEmptyHint) {
                existingEmptyHint = document.createElement('div');
                existingEmptyHint.className = 'sw-canvas-empty-hint';
                canvasEl.appendChild(existingEmptyHint);
            }
            existingEmptyHint.textContent = emptyText;
        } else if (existingEmptyHint) {
            existingEmptyHint.remove();
        }

        if (!canvasEl._bgDismissBound) {
            canvasEl._bgDismissBound = true;
            canvasEl.addEventListener('click', function (ev) {
                if (ev.target === canvasEl || (ev.target && ev.target.id === 'sw-room-canvas-empty')) {
                    closeAllInlineTokens(canvasEl);
                }
            });
        }
        if (!isSameRoom) {
            _inlineOpenTokenKey = null;
            var oldNodes = canvasEl.querySelectorAll('.sw-canvas-token');
            for (var oi = 0; oi < oldNodes.length; oi++) {
                oldNodes[oi].remove();
            }
            var oldCrowdBadges = canvasEl.querySelectorAll('.sw-canvas-crowd-badge');
            for (var cbi = 0; cbi < oldCrowdBadges.length; cbi++) {
                oldCrowdBadges[cbi].remove();
            }
            _prevTokensMap = {};
            _tokenEnterTime = {};
            _strangerSlotById = {};
            _lastPasserbyTransitTime = Date.now();
        }

        var placedPositions = {};
        if (!_tokenPositionsByRoom[currentRoomId]) {
            _tokenPositionsByRoom[currentRoomId] = {};
        }

        var newTokensMap = {};
        for (var i = 0; i < tokens.length; i++) {
            var t = tokens[i];
            newTokensMap[t.key] = t;
        }
        // 排序優先級：1. 固定景物／器物 (facility/item) 永遠最優先錨定；2. 原本已在場的人物；3. 本 Tick 新步入的人物
        var placementOrder = tokens.slice().sort(function (a, b) {
            var aStatic = (a.kind === 'facility' || a.kind === 'item') ? 2 : (isSameRoom && _prevTokensMap[a.key] ? 1 : 0);
            var bStatic = (b.kind === 'facility' || b.kind === 'item') ? 2 : (isSameRoom && _prevTokensMap[b.key] ? 1 : 0);
            if (aStatic !== bStatic) return bStatic - aStatic;
            return ((b.label || '').length - (a.label || '').length);
        });
        var posByKey = {};
        for (var pi = 0; pi < placementOrder.length; pi++) {
            var pTok = placementOrder[pi];
            var isAnchoredTok = (pTok.kind === 'facility' || pTok.kind === 'item' || pTok.kind === 'object') || (isSameRoom && !!_prevTokensMap[pTok.key]);
            posByKey[pTok.key] = computeStablePosition(currentRoomId, pTok.key, (pTok.label || '').length, placedPositions, isAnchoredTok, pTok.kind);
        }
        if (isSameRoom) {
            for (var oldKey in _prevTokensMap) {
                if (!newTokensMap[oldKey]) {
                    var leavingToken = _prevTokensMap[oldKey];
                    var existingEl = canvasEl.querySelector('[data-token-key="' + leavingToken.key + '"]');
                    var isMovableKind = (leavingToken.kind === 'npc' || leavingToken.kind === 'player');
                    if (leavingToken.kind === 'npc' && _allLiveNpcIdsInRoom[leavingToken.id]) {
                        // 仍在場內，僅因 top-5 輪替移出視圖：直接移除 DOM，絕不觸發假離場動畫與廣播
                        if (existingEl) {
                            existingEl.remove();
                        }
                    } else {
                        if (allowTransitAnim && isMovableKind) {
                            var leaveDir = resolveTransitDirection(leavingToken, 'leave');
                            var isPlayerDeparture = (leavingToken.kind === 'player');
                            var nowMs = Date.now();
                            var cooldownOk = (nowMs - (_npcBreathCooldownById[leavingToken.id] || 0) > 20000);
                            if (isPlayerDeparture && cooldownOk && window.SwPushBreath) {
                                _npcBreathCooldownById[leavingToken.id] = nowMs;
                                window.SwPushBreath(leavingToken.label + '邁步往' + leaveDir + '離去。', { byPlayer: false });
                            }
                        }
                        if (existingEl) {
                            existingEl.remove();
                        }
                    }
                }
            }
        }

        for (var j = 0; j < tokens.length; j++) {
            var token = tokens[j];
            var pos = posByKey[token.key];
            var tokenEl = canvasEl.querySelector('[data-token-key="' + token.key + '"]');
            var isMovableKind = (token.kind === 'npc' || token.kind === 'player');
            var isNewToken = !tokenEl && allowTransitAnim && !_prevTokensMap[token.key] && isMovableKind;
            if (!tokenEl) {
                tokenEl = document.createElement('div');
                tokenEl.setAttribute('role', 'button');
                tokenEl.setAttribute('tabindex', '0');
                tokenEl.className = 'sw-canvas-token ' + token.tokenClass;
                tokenEl.setAttribute('data-token-key', token.key);
                tokenEl.setAttribute('data-obj-id', token.id);
                tokenEl.setAttribute('data-entity-id', token.id);
                tokenEl.setAttribute('data-token-kind', token.kind);
                tokenEl.setAttribute('data-obj-tag', (token.data && token.data.tag) || '');
                var obj = token.data;
                if (obj && obj.category_tag) {
                    tokenEl.dataset.objCategory = String(obj.category_tag);
                    tokenEl.setAttribute('data-obj-category', String(obj.category_tag));
                }
                if (obj && Array.isArray(obj.attribute_tags) && obj.attribute_tags.length > 0) {
                    tokenEl.dataset.objAttributes = obj.attribute_tags.join(',');
                    tokenEl.setAttribute('data-obj-attributes', obj.attribute_tags.join(','));
                }
                if (token.data && Array.isArray(token.data.tags)) {
                    tokenEl.setAttribute('data-obj-tags', token.data.tags.join(','));
                }
                syncTokenDomContent(tokenEl, token);
                tokenEl.addEventListener('click', function (e) {
                    if (e.target && e.target.closest && e.target.closest('.sw-inline-act-chip, .sw-token-speech-pill')) return;
                    e.preventDefault();
                    e.stopPropagation();
                    handleTokenPrimaryTap(this, this._swToken || token);
                });
                tokenEl.addEventListener('keydown', function (e) {
                    if (e.key === 'Enter' || e.key === ' ') {
                        if (e.target && e.target.closest && e.target.closest('.sw-inline-act-chip, .sw-token-speech-pill')) return;
                        e.preventDefault();
                        handleTokenPrimaryTap(this, this._swToken || token);
                    }
                });
                if (isNewToken) {
                    _tokenEnterTime[token.key] = Date.now();
                    var enterDir = resolveTransitDirection(token, 'enter');
                    tokenEl.style.left = pos.x + '%';
                    tokenEl.style.top = pos.y + '%';
                    tokenEl.style.opacity = '1';
                    canvasEl.appendChild(tokenEl);
                    var isPlayerArrival = (token.kind === 'player');
                    var arrNowMs = Date.now();
                    var arrCooldownOk = (arrNowMs - (_npcBreathCooldownById[token.id] || 0) > 20000);
                    if (isPlayerArrival && arrCooldownOk && window.SwPushBreath) {
                        _npcBreathCooldownById[token.id] = arrNowMs;
                        window.SwPushBreath(token.label + '自' + enterDir + '走入此處。', { byPlayer: false });
                    }
                } else {
                    tokenEl.style.left = pos.x + '%';
                    tokenEl.style.top = pos.y + '%';
                    tokenEl.style.opacity = '1';
                    canvasEl.appendChild(tokenEl);
                }
            } else {
                tokenEl.classList.remove('is-leaving', 'is-entering');
                tokenEl.style.opacity = '1';
                tokenEl.style.left = pos.x + '%';
                tokenEl.style.top = pos.y + '%';
                syncTokenDomContent(tokenEl, token);
            }
            var oldOverlays = tokenEl.querySelectorAll('.sw-token-speech-pill, .sw-token-chat-dot');
            for (var oi = 0; oi < oldOverlays.length; oi++) {
                oldOverlays[oi].remove();
            }
            tokenEl.classList.remove('is-calling', 'is-calling-combat', 'is-chatting', 'is-priority-host');

            if (token.kind === 'npc') {
                var callout = _activeNpcCallouts[token.id];
                var microNode = tokenEl.querySelector('.sw-token-micro-state');
                if (callout && (Date.now() - callout.time <= 45000)) {
                    tokenEl.classList.add('is-calling');
                    if (callout.intent === 'combat') {
                        tokenEl.classList.add('is-calling-combat');
                    }
                    if (microNode) {
                        microNode.textContent = '💬 欲言搭話';
                        microNode.style.color = '#9e2a2b';
                        microNode.style.fontWeight = '600';
                    }
                } else {
                    if (microNode) {
                        microNode.style.color = '';
                        microNode.style.fontWeight = '';
                    }
                    if (callout) delete _activeNpcCallouts[token.id];
                    var chatTime = _activeNpcChats[token.id];
                    if (chatTime && (Date.now() - chatTime <= 12000)) {
                        tokenEl.classList.add('is-chatting');
                        if (microNode && microNode.textContent && microNode.textContent.indexOf('低語') === -1) {
                            microNode.textContent = microNode.textContent + ' · 低語';
                        }
                    } else if (chatTime) {
                        delete _activeNpcChats[token.id];
                    }
                }

                if (token.data && token.data.interaction_priority) {
                    tokenEl.classList.add('is-priority-host');
                }
            }
            if (_activePopoverToken && _activePopoverToken.key === token.key) {
                _activePopoverToken = token;
                window.__currentPopoverTarget = token;
                var activeTitleEl = document.getElementById('sw-popover-title');
                if (activeTitleEl && token.label) activeTitleEl.textContent = token.label;
                renderPopoverActions(token);
            }
        }
        // 保持 DOM 順序與排序後之 tokens 一致（已識姓名 -> 陌生人 1 -> 陌生人 2...）
        for (var rj = 0; rj < tokens.length; rj++) {
            var rNode = canvasEl.querySelector('[data-token-key="' + tokens[rj].key + '"]:not(.is-leaving)');
            if (rNode && rNode.parentNode === canvasEl) {
                canvasEl.appendChild(rNode);
            }
        }

        // 清除任何不在 newTokensMap 內或重複 key 的殘留 DOM 節點（正在離場漸隱者除外）
        var allDomTokens = canvasEl.querySelectorAll('.sw-canvas-token');
        var seenDomKeys = {};
        for (var di = 0; di < allDomTokens.length; di++) {
            var dNode = allDomTokens[di];
            if (dNode.classList.contains('is-leaving')) continue;
            var dk = dNode.getAttribute('data-token-key') || '';
            if (!dk || !newTokensMap[dk] || seenDomKeys[dk]) {
                dNode.remove();
            } else {
                seenDomKeys[dk] = true;
            }
        }

        // 管理擁擠溢出徽章（.sw-canvas-crowd-badge）
        var existingCrowdBadge = canvasEl.querySelector('.sw-canvas-crowd-badge');
        if (_totalLiveNpcCount > 5) {
            var extraCount = _totalLiveNpcCount - 5;
            if (!existingCrowdBadge) {
                existingCrowdBadge = document.createElement('div');
                existingCrowdBadge.className = 'sw-canvas-crowd-badge';
                canvasEl.appendChild(existingCrowdBadge);
            } else {
                canvasEl.appendChild(existingCrowdBadge); // 確保在尾端
            }
            existingCrowdBadge.innerHTML = '<span class="sw-token-title">＋ 另有 ' + extraCount + ' 人</span>';
        } else if (existingCrowdBadge) {
            existingCrowdBadge.remove();
        }

        _prevRoomId = currentRoomKey;
        _prevTokensMap = newTokensMap;
        _prevAllLiveNpcIdsInRoom = Object.assign({}, _allLiveNpcIdsInRoom);
    }
    var SW_ACTION_LABELS = {
        // Core
        'Look': '觀察', 'Read': '閱讀', 'Smell': '嗅聞', 'Use': '使用', 'Open': '開啟',
        'Queue': '候號', 'AcceptJob': '應工', 'Rest': '歇腳', 'BuySoup': '買滿碗熱湯', 'Work': '差事',
        'Gather': '採集', 'Acquire': '採買入袋', 'Wash': '洗漱淨手', 'Sit': '坐下',
        'Taste': '品嚐', 'Take': '拾取', 'Chop': '砍伐', 'Operate': '操作',
        'Move': '前往', 'Talk': '對話', 'Trade': '交易', 'Borrow': '偷竊',
        'Subdue': '攻擊', 'Slay': '致死', 'Attack': '攻擊',
        // Wood / Fire / Camp
        'MakeFire': '生火', 'AddFuel': '添柴', 'SplitWood': '劈柴', 'RefinePlank': '裁鋸木板',
        'BurnCharcoal': '悶燒木炭', 'CraftTorch': '紮製火把', 'CraftHandle': '車削木柄',
        'CollectCharcoal': '收取木炭', 'RoastFood': '炙烤食物', 'RepairStructure': '修繕加固',
        // Ore / Smithing
        'WashOre': '淘洗礦砂', 'CrushOre': '粉碎礦石', 'SmeltIngot': '熔煉金屬',
        'SmeltBloom': '悶燒成塊', 'BuildFieldAnvil': '架設石砧',
        'ForgeTool': '鍛打器具', 'CastHardware': '澆鑄器用', 'SharpenBlade': '砥石磨刃',
        'Quench': '淬火', 'ForgeRepairStrip': '鍛修鐵條',
        // Herbs / Alchemy
        'SetupHerbStove': '架設陶銚',
        'SortHerbs': '分揀藥草', 'DryHerbs': '曝曬藥材', 'GrindPowder': '研製藥粉',
        'DecoctPotion': '煎煮湯劑', 'MakePoultice': '調製敷料', 'DistillEssence': '萃煉純露',
        'WashHerb': '淘洗藥草', 'DryHerb': '曝曬藥材', 'CrushHerb': '研碎藥草', 'BoilDecoction': '煎煮湯劑',
        // Fiber / Paper / Cloth
        'SoakFiber': '漚浸纖維', 'TwistCord': '搓捻麻繩', 'WeaveCloth': '紡織布匹',
        'OilCanvas': '桐油浸布', 'PressPaper': '抄造紙張', 'Stitch': '縫紉', 'TwistRope': '搓捻麻繩',
        // Hide / Shell / Boots
        'ScrapeHide': '刮治生皮', 'BoilFishGlue': '熬製魚膠', 'CalcifyShell': '煅燒貝灰',
        'CobbleBoots': '縫製行靴',
        // Ferry / Wharf / Tasks
        'JoinCheck': '候檢', 'RegisterComplaint': '押名', 'QueueBroth': '等分湯',
        'CarryCargo': '搬貨收鎂', 'HelpRope': '搭手拉繩', 'DryOff': '避風晾衣',
        'WarmHands': '烘手', 'TalkQueue': '攀談船期', 'QueueFerry': '排隊候船',
        'SellReeds': '賣蘆草', 'SellSackcloth': '交麻袋布', 'SellRope': '交麻繩卷',
        'BoardFerry': '憑隊上船', 'StepAshore': '踩泥上岸', 'CheckCargo': '對貨領尾款',
        // Living / Artisan
        'Sleep': '安歇', 'Bath': '洗沐', 'Drink': '汲水飲用', 'Water': '澆灌',
        'Measure': '丈量', 'Calculate': '推算', 'Observe': '觀測', 'TestBrush': '試筆',
        'GrindInk': '研墨', 'Weigh': '秤量', 'Sharpen': '磨利', 'Feed': '餵食',
        'Shake': '搖動', 'Ring': '叩鐘', 'Caulking': '捻縫', 'Pound': '舂搗',
        'PickType': '揀字', 'RollInk': '滾墨', 'Explore': '深入探索',
        // Extended Sockets & Interactions
        'Check': '檢視', 'Climb': '登臨', 'Inspect': '審視', 'Knock': '叩門',
        'Listen': '諦聽', 'LookShip': '眺望舟楫', 'Tap': '輕叩', 'TestEdge': '試刃',
        'Tie': '繫縛', 'Touch': '撫觸', 'Turn': '旋動', 'Watch': '守望',
        'WeaveSackcloth': '織造麻袋布', 'Meditate': '調息',
        'Harvest': '採收', 'Deliver': '交割物資', 'ClaimStrandedCargo': '揭榜管運',
        'DeliverStrandedCargo': '交割銷單', 'Sow': '整壟播種', 'Board': '登船啟航', 'Buy': '投保立契',
        'DepositBank': '存入票號', 'WithdrawBank': '憑摺支取',
        'StoreStash': '存入石櫃', 'RetrieveStash': '取出物資',
        'DispatchCourier': '快騎託運', 'AngleFish': '甩竿垂釣', 'Cook': '野炊烹煮',
        'MillGrain': '水磨碾米', 'PressCheese': '壓製乳酪', 'SmokeFish': '慢火冷燻',
        'CutJade': '切磨寒玉', 'HaulMineCart': '滑車運礦', 'BurnLime': '入窯煅灰',
        'RowSwampRaft': '撐筏入澤', 'PrepareMiasmaGear': '備妥避瘴', 'PlayChess': '對弈手談',
        'SeasonalBanquet': '歲時圍爐',
        'ChartAstrolabe': '拓印星盤',
        'TestEngraving': '測紋校準',
        'SparPillar': '試勁拆招',
        'SoakHotSpring': '祛寒湯浴',
        'GroomAppearance': '修面整容',
        'MendTravelGear': '補綴行裝',
        'ReadWindVane': '觀測風候',
        'ApplyMoxibustion': '百草溫灸',
        'HangVotivePlaque': '懸掛銘牌',
        'TranscribeRareTome': '鈔錄孤本',
        'AttendLecture': '旁聽策論',
        'CalibrateClock': '校時報刻',
        'FitMortiseTenon': '榫卯校修',
        'QuenchBlade': '精鋼回火',
        'DyeIndigoCloth': '藍染浸布',
        'FeedDraftBeast': '飼餵草獸',
        'BoardCoastalShip': '登船遠航',
        'InspectLandRegistry': '查閱地籍',
        'AssistKitchen': '幫廚備膳',
        'RoastMountainTea': '焙茶品茗',
        'SunBooks': '曝書防蠹',
        'RoastHerbs': '炮製藥材',
        'CollateInscriptions': '校勘碑帖',
        'GrindOpticLens': '研磨鏡片',
        'SelectDriedSeafood': '揀選海味',
        'DeliberateCivicPact': '參議鄉約',
        'DrawSpringWell': '汲水潤嗓'
    };
    window.SwActionLabels = SW_ACTION_LABELS;
    function filterObjectSockets(rawActs) {
        if (!Array.isArray(rawActs)) return [];
        var filtered = [];
        var seen = {};
        for (var i = 0; i < rawActs.length; i++) {
            var a = rawActs[i];
            if (!a || seen[a]) continue;
            // 排除單純查看、細閱、使用、操作等被動審視 socket
            if (a === 'Look' || a === 'Read' || a === 'Use' || a === 'Operate') continue;
            seen[a] = true;
            filtered.push(a);
        }
        return filtered;
    }

    function resolveActionLabelZh(act, targetName, targetId, targetTag, targetTags) {
        var obj = (typeof targetName === 'object' && targetName !== null) ? targetName : null;
        var nameStr = String((obj ? (obj.name || obj.label) : targetName) || '');
        var idStr = String((obj ? obj.id : targetId) || '');
        var tag = String((obj ? obj.tag : targetTag) || '');
        var categoryTag = String((obj ? obj.category_tag : (typeof targetTags === 'object' && !Array.isArray(targetTags) && targetTags ? targetTags.category_tag : '')) || '');
        var attributeTags = Array.isArray(obj ? obj.attribute_tags : (typeof targetTags === 'object' && !Array.isArray(targetTags) && targetTags ? targetTags.attribute_tags : null)) ? (obj ? obj.attribute_tags : targetTags.attribute_tags) : [];
        var tags = Array.isArray(obj ? obj.tags : targetTags) ? (obj ? obj.tags : targetTags) : [];

        function hasTag(t) {
            return tag === t || categoryTag === t || attributeTags.indexOf(t) !== -1 || tags.indexOf(t) !== -1;
        }

        var hasExplicitTags = !!(tag || categoryTag || attributeTags.length > 0 || tags.length > 0);

        if (act === 'Read') {
            if (hasTag('sundial') || hasTag('way_stele') || hasTag('timekeeping') || /sundial/.test(idStr)) return '觀晷認路';
            if (!hasExplicitTags && /日晷|指路/.test(nameStr)) return '觀晷認路';
            return '細閱';
        }
        if (act === 'Rest' || act === 'Sit') {
            if (hasTag('railing')) return '憑欄小憩';
            if (hasTag('bench') || hasTag('bed') || hasTag('seating') || hasTag('bedding')) return '落座歇腳';
            if (!hasExplicitTags) {
                if (/欄|橋|廊|亭/.test(nameStr)) return '憑欄小憩';
                if (/椅|凳|榻|席|床/.test(nameStr)) return '落座歇腳';
            }
            return '小憩歇腳';
        }
        if (act === 'Use' || act === 'Operate' || !act) {
            if (hasTag('telescope') || hasTag('optical') || /telescope/.test(idStr)) return '極目遠眺';
            if (hasTag('fishing_rod') || hasTag('angling') || /angling/.test(idStr)) return '甩竿垂釣';
            if (hasTag('notice_board') || hasTag('way_stele') || hasTag('stationery')) return '細閱';
            if (hasTag('sundial') || hasTag('astrolabe') || hasTag('timekeeping')) return '觀測校刻';
            if (hasTag('railing')) return '憑欄小憩';
            if (hasTag('bench') || hasTag('bed') || hasTag('seating') || hasTag('bedding')) return '憑欄小憩';
            if (hasTag('hygiene') || hasTag('plumbing_bath_hygiene') || /洗衣|浴|盥洗|皂|牙粉/.test(nameStr)) return '盥洗滌淨';
            if (hasTag('mountain_spring') || hasTag('water_tap') || hasTag('water_supply')) return '掬水飲用';
            if (hasTag('street_lamp') || hasTag('desk_lamp') || hasTag('illuminating')) return '撥亮燈火';
            if (hasTag('camp_stove') || hasTag('cauldron') || hasTag('campfire') || hasTag('heating')) return '添火溫煦';
            if (hasTag('bell') || hasTag('musical_instrument')) return '輕叩聽音';
            if (hasTag('zither')) return '撫弄試音';
            if (hasTag('herb_mortar') || hasTag('medical') || hasTag('grindstone')) return '分揀炮製';
            if (hasTag('anvil') || hasTag('forging')) return '試手錘鍊';
            if (hasTag('raft') || hasTag('cargo_ship') || hasTag('transport') || hasTag('sailing')) return '檢視舟車';

            if (!hasExplicitTags) {
                if (/望遠鏡|瞭望|觀測/.test(nameStr)) return '極目遠眺';
                if (/釣竿|釣/.test(nameStr)) return '甩竿垂釣';
                if (/告示|公佈|榜|碑|牌|帖|書|卷|冊|曆|誌/.test(nameStr)) return '細閱';
                if (/日晷|晷|星盤|羅盤|風向|更漏/.test(nameStr)) return '觀測校刻';
                if (/欄|亭|椅|凳|榻|席/.test(nameStr)) return '憑欄小憩';
                if (/井|泉|水|茶|湯|飲/.test(nameStr)) return '掬水飲用';
                if (/燈|燭|火把/.test(nameStr)) return '撥亮燈火';
                if (/爐|灶|炭|鍋/.test(nameStr)) return '添火溫煦';
                if (/鐘|鈴|鼓|鑼|磬/.test(nameStr)) return '輕叩聽音';
                if (/琴|笛|瑟|琵琶/.test(nameStr)) return '撫弄試音';
                if (/藥|臼|碾|篩/.test(nameStr)) return '分揀炮製';
                if (/砧|錘|鍛|磨|刀|劍/.test(nameStr)) return '試手錘鍊';
                if (/車|筏|船|舟|索|纜/.test(nameStr)) return '檢視舟車';
            }
            return '觸探端詳';
        }
        if (SW_ACTION_LABELS[act]) return SW_ACTION_LABELS[act];
        if (/^[A-Za-z_]+$/.test(act)) return '觸探端詳';
        return act;
    }
    window.SwResolveActionLabelZh = resolveActionLabelZh;

    function isWildBeastToken(id, name) {
        var sid = String(id || '');
        if (sid.indexOf('shanhai_') === 0 || sid.indexOf('ambient_') === 0) return true;
        return false;
    }

    function renderPopoverActions(token) {
        var actionsEl = document.getElementById('sw-popover-actions');
        if (!actionsEl) return;
        actionsEl.innerHTML = '';
        if (!token) return;

        if (token.kind === 'explore') {
            var expBtn = document.createElement('button');
            expBtn.type = 'button';
            expBtn.className = 'sw-popover-act-btn log-object-action';
            expBtn.setAttribute('data-entity-id', '__room_explore__');
            expBtn.setAttribute('data-action', 'Explore');
            expBtn.textContent = '深入探索';
            expBtn.addEventListener('click', function () {
                if (window.gameSend) window.gameSend({ type: 'explore' });
            });
            actionsEl.appendChild(expBtn);
            return;
        }

        var targetId = token.id;
        var targetName = token.label;

        if (token.kind === 'npc' || token.kind === 'player') {
            if (isWildBeastToken(targetId, targetName)) {
                var beastActs = [
                    { act: 'Subdue', label: '生擒制伏', cls: 'log-action-subdue' },
                    { act: 'Slay', label: '出手獵殺', cls: 'log-action-slay' }
                ];
                for (var bai = 0; bai < beastActs.length; bai++) {
                    var ba = beastActs[bai];
                    var bBtn = document.createElement('button');
                    bBtn.type = 'button';
                    bBtn.className = 'sw-popover-act-btn log-object-action ' + ba.cls;
                    bBtn.setAttribute('data-entity-id', targetId);
                    bBtn.setAttribute('data-action', ba.act);
                    bBtn.setAttribute('data-target-name', targetName);
                    bBtn.setAttribute('data-target-type', 'entity');
                    bBtn.textContent = ba.label;
                    actionsEl.appendChild(bBtn);
                }
            } else {
                var peaceActions = [
                    { act: 'Talk', label: '對話' },
                    { act: 'Trade', label: '交易' }
                ];
                for (var pi = 0; pi < peaceActions.length; pi++) {
                    var pa = peaceActions[pi];
                    var btn = document.createElement('button');
                    btn.type = 'button';
                    btn.className = 'sw-popover-act-btn log-object-action';
                    btn.setAttribute('data-entity-id', targetId);
                    btn.setAttribute('data-action', pa.act);
                    btn.setAttribute('data-target-name', targetName);
                    btn.setAttribute('data-target-type', 'entity');
                    btn.textContent = pa.label;
                    actionsEl.appendChild(btn);
                }
                var confWrap = document.createElement('span');
                confWrap.className = 'log-conflict-container';
                confWrap.innerHTML = '<button type="button" class="sw-popover-act-btn log-conflict-toggle" title="展開偷竊／攻擊／致死選項">衝突 ▾</button>' +
                    '<span class="log-conflict-group" style="display:none;" hidden>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-steal" data-entity-id="' + _esc(targetId) + '" data-action="Borrow" data-target-name="' + _esc(targetName) + '" data-target-type="entity">偷竊</button>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-subdue" data-entity-id="' + _esc(targetId) + '" data-action="Subdue" data-target-name="' + _esc(targetName) + '" data-target-type="entity">攻擊</button>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-slay" data-entity-id="' + _esc(targetId) + '" data-action="Slay" data-target-name="' + _esc(targetName) + '" data-target-type="entity">致死</button>' +
                    '</span>';
                actionsEl.appendChild(confWrap);
            }
        } else {
            var actionLabels = SW_ACTION_LABELS;
            var acts = (token.data && (token.data.actions || token.data.sockets)) || [];
            var nonLookActs = filterObjectSockets(acts);
            if (nonLookActs.length === 0) {
                actionsEl.innerHTML = '';
                actionsEl.hidden = true;
                return;
            }
            actionsEl.hidden = false;
            var targetTag = (token.data && token.data.tag) || '';
            var targetTags = (token.data && token.data.tags) || [];
            for (var bi = 0; bi < nonLookActs.length; bi++) {
                var act = nonLookActs[bi];
                var oBtn = document.createElement('button');
                oBtn.type = 'button';
                oBtn.className = 'sw-popover-act-btn log-object-action';
                oBtn.setAttribute('data-entity-id', targetId);
                oBtn.setAttribute('data-action', act);
                oBtn.setAttribute('data-target-name', targetName);
                oBtn.setAttribute('data-target-type', 'object');
                var obj = token.data;
                oBtn.textContent = resolveActionLabelZh(act, obj || targetName, targetId, targetTag, targetTags);
                actionsEl.appendChild(oBtn);
            }
        }
    }

    function updatePopoverActions(kind, targetId, targetName, actions, moveTargetId) {
        var actionsEl = document.getElementById('sw-popover-actions');
        if (!actionsEl) return;
        actionsEl.innerHTML = '';
        var actionLabels = SW_ACTION_LABELS;
        if (kind === 'explore') {
            actionsEl.hidden = false;
            var expBtn = document.createElement('button');
            expBtn.type = 'button';
            expBtn.className = 'sw-popover-act-btn log-object-action';
            expBtn.setAttribute('data-entity-id', '__room_explore__');
            expBtn.setAttribute('data-action', 'Explore');
            expBtn.textContent = '深入探索';
            expBtn.addEventListener('click', function () {
                if (window.gameSend) window.gameSend({ type: 'explore' });
            });
            actionsEl.appendChild(expBtn);
            return;
        }
        if (kind === 'person') {
            actionsEl.hidden = false;
            if (isWildBeastToken(targetId, targetName)) {
                var beastActions = [
                    { act: 'Subdue', label: '生擒制伏', cls: 'log-action-subdue' },
                    { act: 'Slay', label: '出手獵殺', cls: 'log-action-slay' }
                ];
                for (var bi2 = 0; bi2 < beastActions.length; bi2++) {
                    var ba2 = beastActions[bi2];
                    var bBtn2 = document.createElement('button');
                    bBtn2.type = 'button';
                    bBtn2.className = 'sw-popover-act-btn log-object-action ' + ba2.cls;
                    bBtn2.setAttribute('data-entity-id', targetId);
                    bBtn2.setAttribute('data-action', ba2.act);
                    bBtn2.setAttribute('data-target-name', targetName);
                    bBtn2.setAttribute('data-target-type', 'entity');
                    bBtn2.textContent = ba2.label;
                    actionsEl.appendChild(bBtn2);
                }
            } else {
                var peaceActions = [
                    { act: 'Talk', label: '對話' },
                    { act: 'Trade', label: '交易' }
                ];
                for (var pi = 0; pi < peaceActions.length; pi++) {
                    var pa = peaceActions[pi];
                    var btn = document.createElement('button');
                    btn.type = 'button';
                    btn.className = 'sw-popover-act-btn log-object-action';
                    btn.setAttribute('data-entity-id', targetId);
                    btn.setAttribute('data-action', pa.act);
                    btn.setAttribute('data-target-name', targetName);
                    btn.setAttribute('data-target-type', 'entity');
                    btn.textContent = pa.label;
                    actionsEl.appendChild(btn);
                }
                var confWrap = document.createElement('span');
                confWrap.className = 'log-conflict-container';
                confWrap.innerHTML = '<button type="button" class="sw-popover-act-btn log-conflict-toggle" title="展開偷竊／攻擊／致死選項">衝突 ▾</button>' +
                    '<span class="log-conflict-group" style="display:none;" hidden>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-steal" data-entity-id="' + _esc(targetId) + '" data-action="Borrow" data-target-name="' + _esc(targetName) + '" data-target-type="entity">偷竊</button>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-subdue" data-entity-id="' + _esc(targetId) + '" data-action="Subdue" data-target-name="' + _esc(targetName) + '" data-target-type="entity">攻擊</button>' +
                    '<button type="button" class="sw-popover-act-btn log-object-action log-action-slay" data-entity-id="' + _esc(targetId) + '" data-action="Slay" data-target-name="' + _esc(targetName) + '" data-target-type="entity">致死</button>' +
                    '</span>';
                actionsEl.appendChild(confWrap);
            }
        } else if (kind === 'object') {
            var nonLook = filterObjectSockets(actions);
            if (!Array.isArray(actions) || nonLook.length === 0) {
                actionsEl.innerHTML = '';
                actionsEl.hidden = true;
                return;
            }
            actionsEl.hidden = false;
            var activeTok = window.__currentPopoverTarget;
            var activeTag = (activeTok && activeTok.data && activeTok.data.tag) || '';
            var activeTags = (activeTok && activeTok.data && activeTok.data.tags) || [];
            for (var i = 0; i < nonLook.length; i++) {
                var act = nonLook[i];
                var label = resolveActionLabelZh(act, (activeTok && activeTok.data) || targetName, targetId, activeTag, activeTags);
                var idForAct = (act === 'Move' && moveTargetId) ? moveTargetId : targetId;
                var oBtn = document.createElement('button');
                oBtn.type = 'button';
                oBtn.className = 'sw-popover-act-btn log-object-action';
                oBtn.setAttribute('data-entity-id', idForAct);
                oBtn.setAttribute('data-action', act);
                oBtn.setAttribute('data-target-name', targetName);
                oBtn.setAttribute('data-target-type', 'object');
                oBtn.textContent = label;
                actionsEl.appendChild(oBtn);
            }
        }
    }
    window.SwUpdatePopoverActions = updatePopoverActions;

    var _popoverIdleTimer = null;
    var _lastFocusSendTime = 0;
    var POPOVER_IDLE_TIMEOUT_MS = 60000;

    function SwResetPopoverIdleTimer() {
        if (_popoverIdleTimer) {
            clearTimeout(_popoverIdleTimer);
            _popoverIdleTimer = null;
        }
        if (!_activePopoverToken) return;
        var timeoutMs = window.__SW_POPOVER_IDLE_MS || POPOVER_IDLE_TIMEOUT_MS;
        _popoverIdleTimer = setTimeout(function () {
            var popoverEl = document.getElementById('sw-object-popover');
            var isFocusInside = !!(popoverEl && document.activeElement && popoverEl.contains(document.activeElement));
            var isPendingTalk = !!document.querySelector('#sw-popover-log .log-pending-talk');
            if (isPendingTalk || isFocusInside) {
                SwResetPopoverIdleTimer();
                return;
            }
            SwCloseObjectPopover();
        }, timeoutMs);
    }
    window.SwResetPopoverIdleTimer = SwResetPopoverIdleTimer;

    function SwOpenObjectPopover(token) {
        if (!token) return;
        if (token && token.id && _activeNpcCallouts[token.id]) {
            delete _activeNpcCallouts[token.id];
            var cEl = document.getElementById('sw-room-canvas');
            var tEl = cEl && cEl.querySelector('[data-token-key="' + token.key + '"]');
            if (tEl) {
                tEl.classList.remove('is-calling', 'is-calling-combat');
                var p = tEl.querySelector('.sw-token-speech-pill');
                if (p) p.remove();
            }
        }
        if (_activePopoverToken && _activePopoverToken.kind === 'npc' && _activePopoverToken.id !== token.id && typeof window.gameSend === 'function') {
            window.gameSend({ type: 'unfocus_entity', entity_id: _activePopoverToken.id });
        }
        _activePopoverToken = token;
        window.__currentPopoverTarget = token;

        var backdrop = document.getElementById('sw-object-popover-backdrop');
        var titleEl = document.getElementById('sw-popover-title');
        var subtitleEl = document.getElementById('sw-popover-subtitle');
        var logEl = document.getElementById('sw-popover-log');

        if (backdrop) {
            backdrop.hidden = false;
            backdrop.style.display = 'flex';
        }
        if (titleEl) titleEl.textContent = token.label;
        if (subtitleEl) subtitleEl.textContent = '';

        renderPopoverActions(token);

        var G = window.SwGrid;
        var currentRoomId = (G && G.mapData && G.mapData.currentRoomId) || _roomId || '';
        var cacheKey = currentRoomId + '|' + token.id + (String(token.id || '').indexOf('ambient_') === 0 ? ('|' + (token.label || '')) : '');
        window.__currentPopoverCacheKey = cacheKey;

        if (logEl) {
            logEl.innerHTML = '';
            if (window.__popoverLogCache && window.__popoverLogCache[cacheKey] && window.__popoverLogCache[cacheKey].length > 0) {
                var frag = document.createDocumentFragment();
                var entries = window.__popoverLogCache[cacheKey];
                var seenTexts = {};
                var seenLook = false;
                var seenNpcDialogue = false;
                for (var ei = 0; ei < entries.length; ei++) {
                    if (entries[ei].classList && (entries[ei].classList.contains('log-talk-input') || entries[ei].classList.contains('log-trade-input'))) {
                        continue;
                    }
                    var isLookEntry = entries[ei].classList && entries[ei].classList.contains('log-look');
                    if (isLookEntry) {
                        if (seenLook) continue;
                        seenLook = true;
                    }
                    var isDialogueEntry = entries[ei].classList && entries[ei].classList.contains('log-npc-dialogue');
                    if (isDialogueEntry) {
                        if (seenNpcDialogue) continue;
                        seenNpcDialogue = true;
                    }
                    var clonedNode = entries[ei].cloneNode(true);
                    if (isLookEntry && clonedNode.innerHTML) {
                        clonedNode.innerHTML = clonedNode.innerHTML.replace(/【正在交談】[\s\S]*$/, '').trim();
                    }
                    var entryText = (clonedNode.textContent || '').trim();
                    if (entryText && seenTexts[entryText]) {
                        continue;
                    }
                    if (entryText) seenTexts[entryText] = true;
                    frag.appendChild(clonedNode);
                }
                logEl.appendChild(frag);
                logEl.scrollTop = logEl.scrollHeight;
            }
        }

        if (token.kind === 'npc' && typeof window.gameSend === 'function') {
            window.gameSend({ type: 'focus_entity', entity_id: token.id });
            _lastFocusSendTime = Date.now();
        }
        SwResetPopoverIdleTimer();
        initPopoverListeners();

        if (token.id !== '__room_explore__' && window.gameSend) {
            if (token.kind === 'npc' && token.data && (token.data.card_state === 'stranger' || token.data.card_state === 'observed')) {
                window.gameSend({ type: 'observe', entity_id: token.id });
            }
            window.gameSend({ type: 'do_action', entity_id: token.id, action: 'Look' });
        }
    }
    window.SwOpenObjectPopover = SwOpenObjectPopover;

    function SwCloseObjectPopover() {
        if (_popoverIdleTimer) {
            clearTimeout(_popoverIdleTimer);
            _popoverIdleTimer = null;
        }
        if (_activePopoverToken && _activePopoverToken.kind === 'npc' && typeof window.gameSend === 'function') {
            window.gameSend({ type: 'unfocus_entity', entity_id: _activePopoverToken.id });
        }
        var backdrop = document.getElementById('sw-object-popover-backdrop');
        if (backdrop) {
            backdrop.hidden = true;
            backdrop.style.display = 'none';
        }
        _activePopoverToken = null;
        window.__currentPopoverTarget = null;
    }
    window.SwCloseObjectPopover = SwCloseObjectPopover;

    function initPopoverListeners() {
        var backdrop = document.getElementById('sw-object-popover-backdrop');
        if (backdrop && !backdrop._bound) {
            backdrop._bound = true;
            backdrop.addEventListener('click', function (e) {
                if (e.target === backdrop) {
                    SwCloseObjectPopover();
                }
            });
        }
        var closeBtn = document.getElementById('sw-popover-close');
        if (closeBtn && !closeBtn._bound) {
            closeBtn._bound = true;
            closeBtn.addEventListener('click', function () {
                SwCloseObjectPopover();
            });
        }
        var popoverEl = document.getElementById('sw-object-popover');
        if (popoverEl && !popoverEl._idleBound) {
            popoverEl._idleBound = true;
            var handlePopoverInteraction = function () {
                SwResetPopoverIdleTimer();
                if (_activePopoverToken && _activePopoverToken.kind === 'npc' && typeof window.gameSend === 'function') {
                    var now = Date.now();
                    if (now - _lastFocusSendTime > 5000) {
                        _lastFocusSendTime = now;
                        window.gameSend({ type: 'focus_entity', entity_id: _activePopoverToken.id });
                    }
                }
            };
            popoverEl.addEventListener('click', handlePopoverInteraction);
            popoverEl.addEventListener('keydown', handlePopoverInteraction);
            popoverEl.addEventListener('input', handlePopoverInteraction);
            popoverEl.addEventListener('pointerdown', handlePopoverInteraction);
        }
        if (!window._swPopoverEscapeBound) {
            window._swPopoverEscapeBound = true;
            window.addEventListener('keydown', function (e) {
                if (e.key === 'Escape') {
                    SwCloseObjectPopover();
                }
            });
        }
    }

    function SwRecordNpcDialogue(npcA, npcB, text) {
        if (!text) return;
        var rawText = String(text).trim();
        if (!rawText) return;
        var fullText = (rawText.indexOf('【正在交談】') !== -1) ? rawText : ('【正在交談】\n' + rawText);
        var formatted = (window.SwNarrative && typeof window.SwNarrative.format === 'function')
            ? window.SwNarrative.format(fullText)
            : fullText.replace(/\n/g, '<br>');

        var G = window.SwGrid;
        var currentRoomId = (G && G.mapData && G.mapData.currentRoomId) || _roomId || '';

        function storeForNpc(npcId) {
            if (!npcId) return;
            var cacheKey = currentRoomId + '|' + npcId;
            if (!window.__popoverLogCache) window.__popoverLogCache = {};
            if (!window.__popoverLogCache[cacheKey]) window.__popoverLogCache[cacheKey] = [];

            var cacheList = window.__popoverLogCache[cacheKey];
            // 確保 cacheList 最多僅保留一筆最新 active dialogue（移除既有的 .log-npc-dialogue 節點）
            window.__popoverLogCache[cacheKey] = cacheList.filter(function (entry) {
                return !(entry.classList && entry.classList.contains('log-npc-dialogue'));
            });
            cacheList = window.__popoverLogCache[cacheKey];

            var node = document.createElement('div');
            node.className = 'sw-log-entry log-entry log-narrative log-npc-dialogue';
            node.innerHTML = formatted;

            cacheList.push(node);
            while (cacheList.length > 30) {
                cacheList.shift();
            }

            var backdrop = document.getElementById('sw-object-popover-backdrop');
            var isVisible = backdrop && !backdrop.hidden && backdrop.style.display !== 'none';
            if (isVisible && window.__currentPopoverTarget && window.__currentPopoverTarget.id === npcId) {
                var popLog = document.getElementById('sw-popover-log');
                if (popLog) {
                    var existingDialogues = popLog.querySelectorAll('.log-npc-dialogue');
                    if (existingDialogues.length > 0) {
                        // 就地置換第一筆為最新對話，並移除多餘的對話節點
                        var replacement = node.cloneNode(true);
                        popLog.replaceChild(replacement, existingDialogues[0]);
                        for (var di = 1; di < existingDialogues.length; di++) {
                            popLog.removeChild(existingDialogues[di]);
                        }
                        popLog.scrollTop = popLog.scrollHeight;
                    } else {
                        if (window.SwLog && typeof window.SwLog.appendAndMaybeAutoScroll === 'function') {
                            var prevLen = cacheList.length;
                            window.SwLog.appendAndMaybeAutoScroll(popLog, node.cloneNode(true));
                            if (cacheList.length > prevLen && cacheList[cacheList.length - 1].textContent === cacheList[prevLen - 1].textContent) {
                                cacheList.pop();
                            }
                        } else {
                            popLog.appendChild(node.cloneNode(true));
                            popLog.scrollTop = popLog.scrollHeight;
                        }
                    }
                }
            }
        }

        if (npcA) storeForNpc(npcA);
        if (npcB && npcB !== npcA) storeForNpc(npcB);
        if (npcA) _activeNpcChats[npcA] = Date.now();
        if (npcB) _activeNpcChats[npcB] = Date.now();
        renderRoomCanvas();
        setTimeout(function () {
            renderRoomCanvas();
        }, 12100);
    }

    function recordNpcAddress(msg) {
        if (!msg || !msg.npc_id) return;
        var npcId = msg.npc_id;
        var rawText = String(msg.text || '').trim();
        if (rawText.indexOf('「') === 0 && rawText.lastIndexOf('」') === rawText.length - 1) {
            var innerRaw = rawText.slice(1, -1).trim();
            if (innerRaw.indexOf('『') !== -1 && innerRaw.indexOf('』') !== -1) {
                rawText = innerRaw.replace(/『/g, '「').replace(/』/g, '」');
            } else if (
                innerRaw.indexOf('見你') !== -1 ||
                innerRaw.indexOf('抬眼') !== -1 ||
                innerRaw.indexOf('朝你') !== -1 ||
                innerRaw.indexOf('傳來細碎動靜') !== -1
            ) {
                rawText = innerRaw;
            }
        }
        var plain = rawText.replace(/<[^>]*>/g, '').trim();
        var quoteCore = '';
        var mFq = plain.match(/『([^』]+)』/);
        if (mFq && mFq[1]) {
            quoteCore = mFq[1].trim();
        } else {
            var mColonQuote = plain.match(/[：:]\s*「([^」]+)」/);
            if (mColonQuote && mColonQuote[1]) {
                quoteCore = mColonQuote[1].trim();
            } else if (plain.indexOf('「') === 0 && plain.lastIndexOf('」') === plain.length - 1) {
                var unwrap = plain.slice(1, -1).trim();
                var colonIdx = unwrap.indexOf('：');
                if (colonIdx === -1) colonIdx = unwrap.indexOf(':');
                quoteCore = (colonIdx !== -1) ? unwrap.slice(colonIdx + 1).trim() : unwrap;
            } else {
                quoteCore = plain;
            }
        }
        quoteCore = quoteCore.replace(/^[「『，。！？、：；\s]+|[」』，。！？、：；\s]+$/g, '').trim();
        var snippet = quoteCore.length > 10 ? (quoteCore.slice(0, 10) + '…') : quoteCore;
        _activeNpcCallouts[npcId] = {
            text: rawText,
            snippet: snippet,
            intent: msg.intent || 'talk',
            npcName: msg.npc_name || '人物',
            time: Date.now()
        };

        var G = window.SwGrid;
        var currentRoomId = (G && G.mapData && G.mapData.currentRoomId) || _roomId || '';
        var cacheKey = currentRoomId + '|' + npcId;
        if (!window.__popoverLogCache) window.__popoverLogCache = {};
        if (!window.__popoverLogCache[cacheKey]) window.__popoverLogCache[cacheKey] = [];

        var cacheList = window.__popoverLogCache[cacheKey];
        // 確保 cacheList 最多僅保留一筆最新 active address（移除既有的 .log-npc-address 節點）
        window.__popoverLogCache[cacheKey] = cacheList.filter(function (entry) {
            return !(entry.classList && entry.classList.contains('log-npc-address'));
        });
        cacheList = window.__popoverLogCache[cacheKey];

        var formatted = (window.SwNarrative && typeof window.SwNarrative.format === 'function')
            ? window.SwNarrative.format(rawText)
            : rawText.replace(/\n/g, '<br>');

        var spkName = String(msg.npc_name || '人物').trim();
        var plainNoBrackets = plain.replace(/[【】]/g, '');
        var alreadyHasSpeakerAttribution = (spkName && plainNoBrackets.indexOf(spkName) === 0) ||
            plainNoBrackets.indexOf('見你') !== -1 ||
            plainNoBrackets.indexOf('抬眼') !== -1 ||
            plainNoBrackets.indexOf('朝你') !== -1;

        var addressNode = document.createElement('div');
        addressNode.className = 'sw-log-entry log-entry log-narrative log-npc-address';
        addressNode.innerHTML = alreadyHasSpeakerAttribution
            ? formatted
            : ('<span class="log-speaker">' + _esc(spkName) + '：</span>' + formatted);
        var lookIdx = -1;
        for (var ci = 0; ci < cacheList.length; ci++) {
            if (cacheList[ci].classList && cacheList[ci].classList.contains('log-look')) {
                lookIdx = ci;
                break;
            }
        }
        if (lookIdx !== -1) {
            cacheList.splice(lookIdx + 1, 0, addressNode);
        } else {
            cacheList.unshift(addressNode);
        }

        var backdrop = document.getElementById('sw-object-popover-backdrop');
        var isVisible = backdrop && !backdrop.hidden && backdrop.style.display !== 'none';
        if (isVisible && window.__currentPopoverTarget && window.__currentPopoverTarget.id === npcId) {
            var popLog = document.getElementById('sw-popover-log');
            if (popLog) {
                var existingAddresses = popLog.querySelectorAll('.log-npc-address');
                if (existingAddresses.length > 0) {
                    popLog.replaceChild(addressNode.cloneNode(true), existingAddresses[0]);
                    for (var ai = 1; ai < existingAddresses.length; ai++) {
                        popLog.removeChild(existingAddresses[ai]);
                    }
                } else {
                    var looks = popLog.querySelectorAll('.log-look');
                    if (looks.length > 0 && looks[0].nextSibling) {
                        popLog.insertBefore(addressNode.cloneNode(true), looks[0].nextSibling);
                    } else if (looks.length > 0) {
                        popLog.appendChild(addressNode.cloneNode(true));
                    } else if (popLog.firstChild) {
                        popLog.insertBefore(addressNode.cloneNode(true), popLog.firstChild);
                    } else {
                        popLog.appendChild(addressNode.cloneNode(true));
                    }
                }
                popLog.scrollTop = popLog.scrollHeight;
            }
        }

        renderRoomCanvas();
        setTimeout(function () {
            renderRoomCanvas();
        }, 45100);
    }

    function openPopoverByEntityId(npcId, fallbackName) {
        if (!npcId) return;
        if (_activeNpcCallouts[npcId]) {
            delete _activeNpcCallouts[npcId];
        }
        var tokens = collectCurrentTokens();
        var found = null;
        for (var i = 0; i < tokens.length; i++) {
            if (tokens[i].id === npcId || tokens[i].key === npcId) {
                found = tokens[i];
                break;
            }
        }
        if (found) {
            SwOpenObjectPopover(found);
            return;
        }

        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var entity = (mapData.entities || []).concat(mapData.npcs || []).find(function (e) {
            return e && (e.id === npcId || e.npc_id === npcId);
        });
        var name = (entity && (entity.name || entity.display_name)) || fallbackName || getKnownNpcName(npcId) || '人物';
        var syntheticToken = {
            id: npcId,
            key: 'npc_' + npcId,
            kind: 'npc',
            name: name,
            label: name,
            tokenClass: 'sw-token-npc',
            data: entity || {}
        };
        SwOpenObjectPopover(syntheticToken);
    }
    window.SwRecordNpcDialogue = SwRecordNpcDialogue;
    // ── 分區直列欄四區同步渲染 ────────────────────────────────────────
    function renderCategorizedPanels(npcs, objects) {
        var peopleEl = document.getElementById('sw-people-items');
        var facilitiesEl = document.getElementById('sw-facilities-items');
        var discoveriesEl = document.getElementById('sw-discoveries-items');
        var exitsEl = document.getElementById('sw-exits-items');

        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var myId = window.myPlayerId || '';
        var otherPlayers = (mapData.entities || []).filter(function (e) {
            return e.id !== myId && (e.kind === 'player' || e.kind === 'Player');
        });

        if (peopleEl) {
            peopleEl.innerHTML = npcCardsHtml(npcs, otherPlayers);
            var peopleSec = document.getElementById('sw-col-people');
            if (peopleSec) {
                var hasPeople = (Array.isArray(npcs) && npcs.length > 0) || otherPlayers.length > 0;
                peopleSec.classList.toggle('is-empty-section', !hasPeople);
            }
        }
        if (facilitiesEl) {
            facilitiesEl.innerHTML = '';
            var facSec = document.getElementById('sw-col-facilities');
            if (facSec) {
                facSec.classList.add('is-empty-section');
                facSec.hidden = true;
                facSec.style.display = 'none';
            }
        }
        if (discoveriesEl) {
            var discHtml = discoveryCardsHtml(objects, mapData.harvestables || [], mapData.currentRoomName);
            discoveriesEl.innerHTML = discHtml;
            var discSec = document.getElementById('sw-col-discoveries');
            if (discSec) {
                var hasDisc = !!discHtml;
                discSec.classList.toggle('is-empty-section', !hasDisc);
                discSec.hidden = !hasDisc;
            }
        }
        if (exitsEl) {
            exitsEl.innerHTML = '';
            var exitsSec = document.getElementById('sw-col-exits');
            if (exitsSec) {
                exitsSec.classList.add('is-empty-section');
                exitsSec.hidden = true;
                exitsSec.style.display = 'none';
            }
        }
        renderRoomCanvas();
    }

    // ── v5 L4 渲染 ───────────────────────────────────────────────────
    function renderV5(objects) {
        _currentObjects = objects || [];
        _currentNpcs = [];
        var el = document.getElementById('sw-l4-objects');
        if (el) {
            el.innerHTML = composeSurface([], _currentObjects);
        }
        renderCategorizedPanels([], _currentObjects);
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

        renderV5NpcCards(_currentNpcs, _currentObjects);
    }

    // ── applyObservationUpdate（interact_result）────────────────────
    var _knownNpcNames = {};

    function getKnownNpcName(npcId) {
        if (!npcId) return '';
        return _knownNpcNames[npcId] || '';
    }

    function isPlaceholderName(name) {
        var s = String(name || '').trim();
        return !s || /^陌生人|^行人|^眼前之人|^npc_|^wt_npc_/i.test(s);
    }

    function applyObservationUpdate(update) {
        if (!update || !update.target_id) return;
        var cleanTag = update.tag ? concisePersonLabel(update.tag, update.new_state || 'named') : '';
        if (update.new_state === 'named' && cleanTag && !isPlaceholderName(cleanTag)) {
            _knownNpcNames[update.target_id] = cleanTag;
        }
        for (var i = 0; i < _currentObjects.length; i++) {
            if (_currentObjects[i].id === update.target_id) {
                _currentObjects[i].observation_state = update.new_state;
                if (cleanTag) _currentObjects[i].tag = cleanTag;
                break;
            }
        }
        for (var ni = 0; ni < _currentNpcs.length; ni++) {
            if (_currentNpcs[ni].npc_id === update.target_id) {
                _currentNpcs[ni].card_state = update.new_state || _currentNpcs[ni].card_state;
                if (cleanTag) _currentNpcs[ni].display_name = cleanTag;
                break;
            }
        }
        if (window.SwGrid && window.SwGrid.mapData) {
            var md = window.SwGrid.mapData;
            if (Array.isArray(md.npcs)) {
                for (var mi = 0; mi < md.npcs.length; mi++) {
                    if (md.npcs[mi] && md.npcs[mi].npc_id === update.target_id) {
                        md.npcs[mi].card_state = update.new_state || md.npcs[mi].card_state;
                        if (cleanTag) md.npcs[mi].display_name = cleanTag;
                    }
                }
            }
            if (Array.isArray(md.entities)) {
                for (var mei = 0; mei < md.entities.length; mei++) {
                    if (md.entities[mei] && md.entities[mei].id === update.target_id && cleanTag) {
                        md.entities[mei].display_name = cleanTag;
                    }
                }
            }
        }
        if (cleanTag) {
            if (_activePopoverToken && _activePopoverToken.id === update.target_id) {
                _activePopoverToken.label = cleanTag;
                var popTitleEl = document.getElementById('sw-popover-title');
                if (popTitleEl) popTitleEl.textContent = cleanTag;
            }
            try {
                var actionBtns = document.querySelectorAll('.log-object-action[data-entity-id="' + update.target_id + '"]');
                for (var bi = 0; bi < actionBtns.length; bi++) {
                    actionBtns[bi].setAttribute('data-target-name', cleanTag);
                }
            } catch (e) {}
        }
        renderV5NpcCards(_currentNpcs, _currentObjects);
    }

    // ── 舊格式相容渲染（grid_view 事件，無 observation_state）───────
    function render() {
        var G = window.SwGrid;
        if (!G) return;

        var mapData       = G.mapData;
        var escFn         = G.esc || _esc;
        var displayNameOf = G.displayNameOf;

        var myId = window.myPlayerId || '';
        var html = '<div class="sw-contact-column">';

        syncNpcEntities(mapData.entities || [], mapData.objects || []);
    }

    function _esc(s) {
        var d = document.createElement('div');
        d.textContent = String(s || '');
        return d.innerHTML;
    }

    // ── 點擊事件綁定 ─────────────────────────────────────────────────
    function bindContainerClicks(container) {
        if (!container || container._bound) return;
        container._bound = true;
        container.addEventListener('click', function (e) {
            var card = e.target.closest('.sw-obj-card');
            if (!card) return;

            var objId   = card.getAttribute('data-obj-id');
            var moveDir = card.getAttribute('data-move-dir');
            var moveTarget = card.getAttribute('data-move-target') || '';
            var entId   = card.getAttribute('data-entity-id');
            var harvestId = card.getAttribute('data-harvest-id');
            var sceneryName = card.getAttribute('data-scenery-name');

            if (moveDir && window.gameSend) {
                window.gameSend({ type: 'move', direction: moveDir, target_place_id: moveTarget });
                return;
            }
            var prevActive = document.querySelectorAll('.sw-obj-card.is-active-target');
            for (var ai = 0; ai < prevActive.length; ai++) {
                prevActive[ai].classList.remove('is-active-target');
            }
            card.classList.add('is-active-target');
            if (harvestId && window.gameSend) {
                window.gameSend({ type: 'do_action', entity_id: harvestId, action: 'Gather' });
            } else if (sceneryName && window.gameSend) {
                window.gameSend({ type: 'do_action', entity_id: sceneryName, action: 'Look' });
            } else if (objId && window.gameSend) {
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

    function bindClick() {
        var targetContainers = [
            document.getElementById('sw-people-items'),
            document.getElementById('sw-facilities-items'),
            document.getElementById('sw-discoveries-items'),
            document.getElementById('sw-exits-items'),
            document.getElementById('sw-l4-objects'),
            document.getElementById('sw-sidebar-content')
        ];
        for (var i = 0; i < targetContainers.length; i++) {
            bindContainerClicks(targetContainers[i]);
        }

        var G = window.SwGrid;
        if (G && G.objectListEl) {
            bindContainerClicks(G.objectListEl);
        }
        initPopoverListeners();
    }

    // ── renderV5NpcCards（後端已算好安全顯示名、觀察態與真實活動）─────
    function renderV5NpcCards(npcs, objects, meta) {
        var prevVisible = {};
        for (var pvi = 0; pvi < _currentNpcs.length; pvi++) {
            if (_currentNpcs[pvi] && _currentNpcs[pvi].npc_id) {
                prevVisible[_currentNpcs[pvi].npc_id] = pvi + 1;
            }
        }
        var normalized = (npcs || []).map(function (npc) {
            var copy = Object.assign({}, npc);
            if (copy.npc_id) {
                var isBeastOrAmbient = String(copy.npc_id).indexOf('shanhai_') === 0 || String(copy.npc_id).indexOf('ambient_') === 0;
                if (isBeastOrAmbient && copy.display_name && !/^陌生人/.test(copy.display_name)) {
                    copy.card_state = copy.card_state === 'stranger' ? 'observed' : (copy.card_state || 'observed');
                } else if (copy.card_state === 'named' && copy.display_name && !isPlaceholderName(copy.display_name)) {
                    var clean = concisePersonLabel(copy.display_name, 'named');
                    _knownNpcNames[copy.npc_id] = clean;
                    copy.display_name = clean;
                } else if (_knownNpcNames[copy.npc_id]) {
                    copy.card_state = 'named';
                    copy.display_name = _knownNpcNames[copy.npc_id];
                }
            }
            return copy;
        });
        var crowdCard = null;
        var nonCrowd = [];
        for (var ni = 0; ni < normalized.length; ni++) {
            if (normalized[ni].kind === 'crowd') {
                crowdCard = normalized[ni];
            } else {
                nonCrowd.push(normalized[ni]);
            }
        }
        var VISIBLE_LIMIT = 5;
        var currentLiveIds = {};
        for (var lidi = 0; lidi < nonCrowd.length; lidi++) {
            var lid = nonCrowd[lidi].npc_id || nonCrowd[lidi].id;
            if (lid) currentLiveIds[lid] = true;
        }
        if (meta && typeof meta.totalLiveCount === 'number') {
            _totalLiveNpcCount = meta.totalLiveCount;
            _allLiveNpcIdsInRoom = meta.liveIdsMap || currentLiveIds;
        } else {
            var G0 = window.SwGrid;
            var md0 = (G0 && G0.mapData) ? G0.mapData : null;
            var hintTotal = (md0 && typeof md0.total_npc_count === 'number') ? md0.total_npc_count : 0;
            _totalLiveNpcCount = Math.max(nonCrowd.length, hintTotal);
            _allLiveNpcIdsInRoom = Object.assign({}, _allLiveNpcIdsInRoom || {}, currentLiveIds);
        }
        if (nonCrowd.length > VISIBLE_LIMIT) {
            nonCrowd.sort(function (a, b) {
                var aCall = (_activeNpcCallouts && a.npc_id && _activeNpcCallouts[a.npc_id]) ? 1 : 0;
                var bCall = (_activeNpcCallouts && b.npc_id && _activeNpcCallouts[b.npc_id]) ? 1 : 0;
                if (aCall !== bCall) return bCall - aCall;
                var aPri = a.interaction_priority ? 1 : 0;
                var bPri = b.interaction_priority ? 1 : 0;
                if (aPri !== bPri) return bPri - aPri;
                var aPrev = (a.npc_id && prevVisible[a.npc_id]) ? prevVisible[a.npc_id] : 9999;
                var bPrev = (b.npc_id && prevVisible[b.npc_id]) ? prevVisible[b.npc_id] : 9999;
                if (aPrev !== bPrev) return aPrev - bPrev;
                var aNamed = (a.card_state === 'named') ? 1 : 0;
                var bNamed = (b.card_state === 'named') ? 1 : 0;
                return bNamed - aNamed;
            });
            nonCrowd = nonCrowd.slice(0, VISIBLE_LIMIT);
            if (!crowdCard) {
                crowdCard = { kind: 'crowd', display_name: '其餘人影', card_state: 'crowd' };
            }
        }
        _currentNpcs = crowdCard ? nonCrowd.concat([crowdCard]) : nonCrowd;
        var strangerCards = _currentNpcs.filter(function (c) {
            return c.kind !== 'crowd' && c.card_state !== 'named' && /^陌生人/.test(c.display_name || '');
        });
        for (var sci = 0; sci < strangerCards.length; sci++) {
            strangerCards[sci].display_name = '陌生人';
        }
        _currentObjects = objects || [];
        var el = document.getElementById('sw-l4-objects');
        if (el) {
            el.innerHTML = composeSurface(_currentNpcs, _currentObjects);
        }
        renderCategorizedPanels(_currentNpcs, _currentObjects);
    }
    // RoomView 是世界 tick 的即時真相；保留觀察態名稱，補進／移除實際在場人物。
    var _lastPasserbyTransitTime = Date.now();

    function transferProceduralSlotInPlace(oldId, newId) {
        if (!oldId || !newId || oldId === newId) return;
        var oldKey = 'npc:' + oldId;
        var newKey = 'npc:' + newId;
        if (_strangerSlotById[oldId]) {
            _strangerSlotById[newId] = _strangerSlotById[oldId];
            delete _strangerSlotById[oldId];
        }
        for (var rk in _tokenPositionsByRoom) {
            if (_tokenPositionsByRoom[rk] && _tokenPositionsByRoom[rk][oldKey]) {
                _tokenPositionsByRoom[rk][newKey] = _tokenPositionsByRoom[rk][oldKey];
                delete _tokenPositionsByRoom[rk][oldKey];
            }
        }
        if (_prevTokensMap[oldKey]) {
            _prevTokensMap[newKey] = _prevTokensMap[oldKey];
            _prevTokensMap[newKey].key = newKey;
            _prevTokensMap[newKey].id = newId;
            delete _prevTokensMap[oldKey];
        }
        if (_inlineOpenTokenKey === oldKey) {
            _inlineOpenTokenKey = newKey;
        }
        if (_activePopoverToken && _activePopoverToken.key === oldKey) {
            _activePopoverToken.key = newKey;
            _activePopoverToken.id = newId;
        }
        var canvasEl = document.getElementById('sw-room-canvas');
        if (canvasEl) {
            var domNode = canvasEl.querySelector('[data-token-key="' + oldKey + '"]');
            if (domNode) {
                domNode.setAttribute('data-token-key', newKey);
                domNode.setAttribute('data-obj-id', newId);
                domNode.setAttribute('data-entity-id', newId);
            }
        }
    }

    function syncNpcEntities(entities, objects) {
        var previous = {};
        var previousOrder = [];
        var crowd = null;
        var GMap = (window.SwGrid && window.SwGrid.mapData) ? window.SwGrid.mapData : null;
        if (GMap && Array.isArray(GMap.npcs)) {
            for (var gi = 0; gi < GMap.npcs.length; gi++) {
                var gNpc = GMap.npcs[gi];
                if (!gNpc) continue;
                if (gNpc.kind === 'crowd') {
                    crowd = gNpc;
                } else if (gNpc.npc_id) {
                    previous[gNpc.npc_id] = gNpc;
                }
            }
        }
        for (var i = 0; i < _currentNpcs.length; i++) {
            if (_currentNpcs[i].kind === 'crowd') {
                crowd = _currentNpcs[i];
            } else if (_currentNpcs[i].npc_id) {
                previous[_currentNpcs[i].npc_id] = _currentNpcs[i];
                previousOrder.push(_currentNpcs[i].npc_id);
            }
        }
        var source = entities || [];
        var live = {};
        var syncLiveIds = {};
        for (var ei = 0; ei < source.length; ei++) {
            var ent = source[ei];
            if (ent.kind !== 'npc' && ent.kind !== 'Npc') continue;
            live[ent.id] = ent;
            syncLiveIds[ent.id] = true;
        }
        _allLiveNpcIdsInRoom = syncLiveIds;
        _totalLiveNpcCount = Object.keys(syncLiveIds).length;
        var next = [];
        var selected = {};
        var VISIBLE_LIMIT = 5;
        function isWildOrAmbient(eid, entity) {
            if (!eid) return false;
            var s = String(eid);
            if (s.indexOf('shanhai_') === 0 || s.indexOf('ambient_') === 0) return true;
            return false;
        }
        function resolveDisplayName(entity, safeRecord) {
            var eid = entity && entity.id;
            if (eid && _knownNpcNames[eid]) {
                return _knownNpcNames[eid];
            }
            if (isWildOrAmbient(eid, entity)) {
                return (safeRecord && safeRecord.display_name && !/^陌生人/.test(safeRecord.display_name))
                    ? safeRecord.display_name
                    : (entity.display_title || entity.display_name || '生靈');
            }
            if (entity && entity.display_name && !isPlaceholderName(entity.display_name)) {
                var cleanEntName = concisePersonLabel(entity.display_name, 'named');
                if (eid && cleanEntName && !isPlaceholderName(cleanEntName)) {
                    _knownNpcNames[eid] = cleanEntName;
                }
                return cleanEntName || entity.display_name;
            }
            if (safeRecord && safeRecord.card_state === 'named' && safeRecord.display_name && !isPlaceholderName(safeRecord.display_name)) {
                return safeRecord.display_name;
            }
            return '陌生人';
        }
        function resolveCardState(entity, safeRecord) {
            var eid = entity && entity.id;
            if (eid && _knownNpcNames[eid]) {
                return 'named';
            }
            if (isWildOrAmbient(eid, entity)) {
                return (safeRecord && safeRecord.card_state && safeRecord.card_state !== 'stranger')
                    ? safeRecord.card_state
                    : 'observed';
            }
            if (entity && entity.display_name && !isPlaceholderName(entity.display_name)) {
                return 'named';
            }
            return safeRecord ? safeRecord.card_state : 'stranger';
        }
        function npcSelectionRank(ent) {
            var idStr = String((ent && ent.id) || '');
            var actStr = String((ent && ent.current_activity) || '');
            var isProcedural = /^npc_/.test(idStr);
            var isInTransit = /^正(動身|步行)前往/.test(actStr);
            if (!isProcedural && !isInTransit) return 4;
            if (!isProcedural && isInTransit) return 3;
            if (isProcedural && !isInTransit) return 2;
            return 1;
        }
        var sortedCandidates = source.slice().sort(function (a, b) {
            return npcSelectionRank(b) - npcSelectionRank(a);
        });

        // 1. 優先放入主動搭話／互動焦點／玩家正展開內聯晶片或詳情框的人物
        for (var pri = 0; pri < source.length && next.length < VISIBLE_LIMIT; pri++) {
            var priorityEntity = source[pri];
            if (priorityEntity.kind !== 'npc' && priorityEntity.kind !== 'Npc') continue;
            var prioritySafe = previous[priorityEntity.id];
            var priCardState = resolveCardState(priorityEntity, prioritySafe);
            var isCalling = !!(_activeNpcCallouts && _activeNpcCallouts[priorityEntity.id]);
            var isInlineOpen = (_inlineOpenTokenKey === 'npc:' + priorityEntity.id);
            var isPopoverOpen = !!(_activePopoverToken && _activePopoverToken.id === priorityEntity.id);
            if (!priorityEntity.interaction_priority && !isCalling && !isInlineOpen && !isPopoverOpen && priCardState !== 'named') continue;
            var priName = resolveDisplayName(priorityEntity, prioritySafe);
            next.push({
                kind: 'npc',
                npc_id: priorityEntity.id,
                display_name: priName,
                card_state: resolveCardState(priorityEntity, prioritySafe),
                current_activity: priorityEntity.current_activity || '',
                interaction_priority: !!priorityEntity.interaction_priority
            });
            selected[priorityEntity.id] = true;
        }

        var nowMs = Date.now();
        var isRoomJustEntered = (previousOrder.length === 0) || ((_roomEnterTime > 0) && ((nowMs - _roomEnterTime) < 1500));

        // 2. 入場初刻優先錨定本場所駐場核心人物（rank 4，如樞紐石橋旁指引商旅之陸常貴）；駐留期間則先保留已在畫面中之人物，不因路過角色而硬擠換位
        if (isRoomJustEntered) {
            for (var ci = 0; ci < sortedCandidates.length && next.length < VISIBLE_LIMIT; ci++) {
                var curatedEnt = sortedCandidates[ci];
                if ((curatedEnt.kind !== 'npc' && curatedEnt.kind !== 'Npc') || selected[curatedEnt.id]) continue;
                if (npcSelectionRank(curatedEnt) < 4) break;
                var curatedSafe = previous[curatedEnt.id];
                next.push({
                    kind: 'npc',
                    npc_id: curatedEnt.id,
                    display_name: resolveDisplayName(curatedEnt, curatedSafe),
                    card_state: resolveCardState(curatedEnt, curatedSafe),
                    current_activity: curatedEnt.current_activity || '',
                    interaction_priority: !!curatedEnt.interaction_priority
                });
                selected[curatedEnt.id] = true;
            }
        }

        // 3. 保留玩家視野中原本在場的人；對背景隨機路人施加 15 秒從容節流，避免每 3 秒集體洗牌進出
        var st = (typeof window.gameState === 'function') ? window.gameState() : null;
        var px = (st && typeof st.x === 'number') ? st.x : ((GMap && typeof GMap.playerX === 'number') ? GMap.playerX : ((GMap && GMap.player && typeof GMap.player.x === 'number') ? GMap.player.x : null));
        var py = (st && typeof st.y === 'number') ? st.y : ((GMap && typeof GMap.playerY === 'number') ? GMap.playerY : ((GMap && GMap.player && typeof GMap.player.y === 'number') ? GMap.player.y : null));
        var curCoordSuffix = (px !== null && py !== null) ? ('_' + px + '_' + py) : '';
        var passerbyTransitUsedThisTick = false;

        // 3A. 先保留仍在場（live）之既有視野人物，確保其 selected 標記完整
        for (var pi = 0; pi < previousOrder.length && next.length < VISIBLE_LIMIT; pi++) {
            var priorId = previousOrder[pi];
            if (selected[priorId]) continue;
            var priorEntity = live[priorId];
            var priorSafe = previous[priorId];
            if (!priorEntity) {
                var sid = String(priorId);
                var coordMatches = !curCoordSuffix || sid.slice(-curCoordSuffix.length) === curCoordSuffix;
                if (priorSafe && coordMatches && (sid.indexOf('shanhai_') === 0 || sid.indexOf('ambient_') === 0)) {
                    next.push(priorSafe);
                    selected[priorId] = true;
                }
                continue;
            }
            next.push({
                kind: 'npc',
                npc_id: priorId,
                display_name: resolveDisplayName(priorEntity, priorSafe),
                card_state: resolveCardState(priorEntity, priorSafe),
                current_activity: priorEntity.current_activity || '',
                interaction_priority: !!priorEntity.interaction_priority
            });
            selected[priorId] = true;
        }

        // 3B. 處理本 Tick 離開之既有視野人物：15 秒內原位平順交接予未在場上之駐留街坊，每 15 秒最多允許 1 人滑出滑入
        for (var dpi = 0; dpi < previousOrder.length && next.length < VISIBLE_LIMIT; dpi++) {
            var depId = previousOrder[dpi];
            if (selected[depId] || live[depId]) continue;
            var depSafe = previous[depId];
            var depSid = String(depId);
            var isProceduralStranger = /^npc_/.test(depSid) && (!depSafe || depSafe.card_state === 'stranger');
            if (!isProceduralStranger) {
                passerbyTransitUsedThisTick = true;
                _lastPasserbyTransitTime = nowMs;
                continue;
            }
            var canAnimatePasserby = !passerbyTransitUsedThisTick && ((nowMs - _lastPasserbyTransitTime) >= 15000);
            if (canAnimatePasserby) {
                passerbyTransitUsedThisTick = true;
                _lastPasserbyTransitTime = nowMs;
                continue;
            }
            var replacementEnt = null;
            for (var ri = 0; ri < sortedCandidates.length; ri++) {
                var cand = sortedCandidates[ri];
                if ((cand.kind !== 'npc' && cand.kind !== 'Npc') || selected[cand.id] || previous[cand.id]) continue;
                if (/^npc_/.test(String(cand.id || ''))) {
                    replacementEnt = cand;
                    break;
                }
            }
            if (replacementEnt) {
                transferProceduralSlotInPlace(depId, replacementEnt.id);
                var repSafe = previous[replacementEnt.id] || depSafe;
                next.push({
                    kind: 'npc',
                    npc_id: replacementEnt.id,
                    display_name: resolveDisplayName(replacementEnt, repSafe),
                    card_state: resolveCardState(replacementEnt, repSafe),
                    current_activity: replacementEnt.current_activity || '',
                    interaction_priority: !!replacementEnt.interaction_priority
                });
                selected[replacementEnt.id] = true;
            }
        }

        // 4. 補齊剩餘空缺槽位
        for (var li = 0; li < sortedCandidates.length && next.length < VISIBLE_LIMIT; li++) {
            var ent = sortedCandidates[li];
            if ((ent.kind !== 'npc' && ent.kind !== 'Npc') || selected[ent.id]) continue;
            var safe = previous[ent.id];
            var entName = resolveDisplayName(ent, safe);
            next.push({
                kind: 'npc',
                npc_id: ent.id,
                display_name: entName,
                card_state: resolveCardState(ent, safe),
                current_activity: ent.current_activity || '',
                interaction_priority: !!ent.interaction_priority
            });
            selected[ent.id] = true;
        }
        var strangers = next.filter(function (npc) { return npc.card_state === 'stranger' && (npc.display_name === '行人' || /^陌生人/.test(npc.display_name || '')); });
        for (var si = 0; si < strangers.length; si++) {
            strangers[si].display_name = '陌生人';
        }
        if (Object.keys(live).length > next.length) {
            next.push(crowd || { kind: 'crowd', display_name: '其餘人影', card_state: 'crowd' });
        }
        renderV5NpcCards(next, objects || _currentObjects, {
            totalLiveCount: Object.keys(live).length,
            liveIdsMap: syncLiveIds
        });
    }

    window.SwObjectList = {
        render: render,
        renderV5: renderV5,
        renderV5NpcCards: renderV5NpcCards,
        syncNpcEntities: syncNpcEntities,
        applyDelta: applyDelta,
        applyObservationUpdate: applyObservationUpdate,
        getKnownNpcName: getKnownNpcName,
        bindClick: bindClick,
        buildCurrentExitsLogHtml: buildCurrentExitsLogHtml,
        appendCurrentExitsToLog: appendCurrentExitsToLog,
        renderRoomCanvas: renderRoomCanvas,
        openPopover: SwOpenObjectPopover,
        closePopover: SwCloseObjectPopover,
        resetIdleTimer: SwResetPopoverIdleTimer,
        recordNpcDialogue: SwRecordNpcDialogue,
        recordNpcAddress: recordNpcAddress,
        openPopoverByEntityId: openPopoverByEntityId,
        conciseObjectLabel: conciseObjectLabel,
        isMovableOrPortableObject: isMovableOrPortableObject,
        resolveSocketActionLabel: resolveSocketActionLabel
    };
    function cleanDiegeticBreathText(raw, isPlayerBreath) {
        var s = String(raw || '').trim();
        if (!s) return '';
        // 1. 剝除偶發之設計/架構標註（如【事件包...】、【劇本演繹...】）
        s = s.replace(/^【[^】]*(?:事件包|劇本包|劇本演繹|聯動)[^】]*】\s*/g, '');
        s = s.replace(/\s*\n+\s*/g, ' ').trim();
        // 2. 若為環境氣息（非玩家主動操作），過濾多句冗長段落，取第一句完整敘述（≤ 56 字），避免呼吸墨韻區文字臃塞
        if (!isPlayerBreath) {
            var firstSentenceMatch = s.match(/^[^。！？\n]+[。！？][」』”]*/);
            if (firstSentenceMatch && firstSentenceMatch[0]) {
                var firstSen = firstSentenceMatch[0].trim();
                if (firstSen.length <= 56) {
                    s = firstSen;
                } else {
                    s = firstSen.slice(0, 56) + '…';
                }
            } else if (s.length > 56) {
                var sub = s.slice(0, 56);
                var lastPunctIdx = -1;
                for (var pi = 55; pi >= 14; pi--) {
                    var ch = sub.charAt(pi);
                    if (ch === '。' || ch === '！' || ch === '？' || ch === '；' || ch === '，') {
                        lastPunctIdx = pi;
                        break;
                    }
                }
                if (lastPunctIdx >= 14) {
                    s = sub.slice(0, lastPunctIdx + 1).replace(/[，；]$/, '。');
                } else {
                    s = sub + '…';
                }
            }
            var openQuotes = (s.match(/「/g) || []).length;
            var closeQuotes = (s.match(/」/g) || []).length;
            if (openQuotes > closeQuotes) {
                s = s + '」';
            }
            if (s.replace(/[。！？…\s]/g, '').length < 6) {
                return '';
            }
        }
        return s;
    }
    window.cleanDiegeticBreathText = cleanDiegeticBreathText;

    function installBreathSanitizer() {
        if (typeof window === 'undefined') return;
        if (typeof window.SwPushBreath === 'function' && !window.SwPushBreath._sanitized) {
            var origPush = window.SwPushBreath;
            var sanitizedPush = function (htmlOrText, options) {
                var byPlayer = !!(options && options.byPlayer);
                var cleaned = cleanDiegeticBreathText(htmlOrText, byPlayer);
                if (!cleaned) return;
                return origPush(cleaned, options);
            };
            sanitizedPush._sanitized = true;
            window.SwPushBreath = sanitizedPush;
        }
    }
    installBreathSanitizer();
    if (typeof document !== 'undefined') {
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', installBreathSanitizer);
        } else {
            setTimeout(installBreathSanitizer, 0);
        }
    }
    function cleanPlaceDisplayName(name) {
        var s = String(name || '').trim();
        if (!s) return '';
        if (s.indexOf('文曜・中心樞紐石橋') !== -1 || s.indexOf('中心樞紐石橋') !== -1) {
            s = s.replace('文曜・中心樞紐石橋', '三水匯流大石橋').replace('中心樞紐石橋', '三水匯流大石橋');
        }
        if (s.indexOf('文曜・') === 0) {
            s = s.slice(3).trim();
        }
        return s;
    }
    window.cleanPlaceDisplayName = window.cleanPlaceDisplayName || cleanPlaceDisplayName;

    function shouldFilterFromRecap(text) {
        var s = String(text || '').trim();
        if (/^你仔細打量了/.test(s)) return true;
        if (/^你對【[^】]+】說：「(寒暄|寒暄問候|打聽|打聽近況|請教差事)」$/.test(s)) return true;
        return false;
    }

    function installRecapFilter() {
        if (window.SwNotify && window.SwNotify.recordRecap && !window.SwNotify.recordRecap._filtered) {
            var origRecordRecap = window.SwNotify.recordRecap;
            var filteredWrapper = function (entry) {
                if (!entry) return origRecordRecap(entry);
                var plain = entry.plain || (entry.html ? entry.html.replace(/<[^>]*>/g, '').trim() : '');
                if (shouldFilterFromRecap(plain)) {
                    return null;
                }
                return origRecordRecap(entry);
            };
            filteredWrapper._filtered = true;
            window.SwNotify.recordRecap = filteredWrapper;
        }
    }
    installRecapFilter();
    if (typeof document !== 'undefined') {
        if (document.readyState === 'loading') {
            document.addEventListener('DOMContentLoaded', installRecapFilter);
        } else {
            setTimeout(installRecapFilter, 0);
        }
    }
    if (typeof window !== 'undefined') {
        var _resizeTimer = null;
        window.addEventListener('resize', function () {
            clearTimeout(_resizeTimer);
            _resizeTimer = setTimeout(function () {
                renderRoomCanvas();
            }, 80);
        });
    }
})();
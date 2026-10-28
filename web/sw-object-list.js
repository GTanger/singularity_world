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

    // 判斷物件是否具備「可離開當前格」或「可隨身攜帶」性質
    function isMovableOrPortableObject(obj) {
        if (!obj) return false;
        // NPC 或玩家不歸於隨行通道物件欄（已有專門的人物理論欄位）
        if (obj.type === 'npc' || obj.kind === 'npc' || obj.kind === 'Npc' || obj.npc_id) return false;
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

    function isFacility(obj) {
        if (!obj) return false;
        if (obj.type === 'npc' || obj.kind === 'npc' || obj.kind === 'Npc' || obj.npc_id) return false;
        if (isMovableOrPortableObject(obj)) return false;
        return true;
    }

    function facilityIcon(obj) {
        var n = String((obj && (obj.name || obj.label)) || '');
        if (/[店鋪市集商行銀錢庫帳櫃銀票攤賈貿資]/.test(n)) return '$';
        if (/[醫藥藥包湯包溫灸診草藥救傷回春]/.test(n)) return '+';
        if (/[茶席臥榻浴池餐食客棧旅舍精舍房室舍榻椅茶食歇席眠床水盆浴盆洗漱]/.test(n)) return '⌂';
        if (/[鐘樓星軌典籍契書官堂府衙館衛哨營譜冊牘卷碑標律法星盤星儀]/.test(n)) return '⚑';
        if (/[鍛爐木作工具石柱機坊鐵匠器車井盤栓釜鍋窯柱板尺樁秤砧]/.test(n)) return '⚒︎';
        return '⚒︎';
    }
    function conciseObjectLabel(name) {
        var str = String(name || '').trim();
        if (!str) return '';
        var knownMap = {
            '貝母路燈與防雨竹編路標': '貝母路燈・竹編路標',
            '文曜青石街燈與黃銅公佈欄': '青石街燈・黃銅公佈欄',
            '三水川雕花青石欄杆與繫船環': '雕花石欄・繫船鐵環',
            '自來活水黃銅水栓與溫石熱水槽': '黃銅水栓・溫石熱水槽',
            '松香木彈力大床與保鮮石櫃': '彈力大床・保鮮石櫃',
            '止血水苔與避瘴清心丸架': '止血水苔・清心丸架'
        };
        if (knownMap[str]) {
            return knownMap[str];
        }
        if (str.length > 8 && str.indexOf('與') !== -1) {
            var parts = str.split('與');
            if (parts.length === 2) {
                var p1 = parts[0].trim();
                var p2 = parts[1].trim();
                var combined = p1 + '・' + p2;
                if (combined.length <= 9) {
                    return combined;
                }
                var shortP1 = p1.replace(/^(三水川|文曜|自來活水)/, '');
                var shortP2 = p2.replace(/^(防雨)/, '');
                var refined = shortP1 + '・' + shortP2;
                if (refined.length <= 9) {
                    return refined;
                }
                return combined;
            }
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
            html += '<button type="button" class="sw-obj-card sw-facility-card sw-obj-item-card" data-obj-id="' + _esc(obj.id) + '" title="' + _esc(rawLabel) + '">';
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
            html += '<button type="button" class="' + cls + '" data-obj-id="' + _esc(obj.id) + '" title="' + _esc(rawLabel) + '">';
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
            html += '<button type="button" class="' + cls + '" data-obj-id="' + _esc(obj.id) + '">' +
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
                display_name: p.display_name || p.id || '旅人',
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
        var estHeightPx = isNarrow ? 31 : 35;
        var halfW = Math.min(34, Math.max(5.5, (estWidthPx / 2 / cw) * 100));
        var halfH = Math.min(9.5, Math.max(4.2, (estHeightPx / 2 / ch) * 100));
        return { halfW: halfW, halfH: halfH, bucket: isNarrow ? 'm' : 'd' };
    }

    function checkCollisionWithPlaced(x, y, box, placedPositions) {
        var minClearance = 999;
        for (var k in placedPositions) {
            var p = placedPositions[k];
            var reqDx = (box.halfW + (p.halfW || 8)) + 1.8;
            var reqDy = (box.halfH + (p.halfH || 5)) + 1.8;
            var dx = Math.abs(x - p.x);
            var dy = Math.abs(y - p.y);
            if (dx < reqDx && dy < reqDy) {
                var c = Math.max(dx / reqDx, dy / reqDy);
                if (c < minClearance) minClearance = c;
            }
        }
        return minClearance;
    }

    function computeStablePosition(roomId, tokenKey, labelLen, placedPositions) {
        var box = estimateTokenHalfBox(labelLen);
        var cacheRoomKey = roomId + ':' + box.bucket;
        if (!_tokenPositionsByRoom[cacheRoomKey]) {
            _tokenPositionsByRoom[cacheRoomKey] = {};
        }
        var minX = Math.min(46, Math.ceil(box.halfW + 2.5));
        var maxX = Math.max(54, Math.floor(97.5 - box.halfW));
        var spanX = Math.max(8, maxX - minX);
        var minY = Math.ceil(box.halfH + 3);
        var maxY = Math.floor(97 - box.halfH);
        var spanY = Math.max(10, maxY - minY);

        var cached = _tokenPositionsByRoom[cacheRoomKey][tokenKey];
        if (cached) {
            var cx = Math.max(minX, Math.min(maxX, cached.x));
            var cy = Math.max(minY, Math.min(maxY, cached.y));
            if (checkCollisionWithPlaced(cx, cy, box, placedPositions) >= 0.98) {
                var cPos = { x: cx, y: cy, halfW: box.halfW, halfH: box.halfH };
                _tokenPositionsByRoom[cacheRoomKey][tokenKey] = cPos;
                placedPositions[tokenKey] = cPos;
                return cPos;
            }
        }
        var h = hashStr(roomId + '|' + tokenKey);
        var bestX = minX + (h % spanX);
        var bestY = minY + (Math.floor(h / 97) % spanY);
        var bestClearance = -1;

        for (var attempt = 0; attempt < 160; attempt++) {
            var x, y;
            if (attempt === 0) {
                x = bestX;
                y = bestY;
            } else {
                var u = ((h % 997) / 997 + attempt * 0.61803398875) % 1;
                var v = (((Math.floor(h / 997) % 991) / 991) + attempt * 0.7548776662) % 1;
                x = Math.round(minX + u * spanX);
                y = Math.round(minY + v * spanY);
            }
            var clearance = checkCollisionWithPlaced(x, y, box, placedPositions);
            if (clearance >= 1.0) {
                bestX = x;
                bestY = y;
                break;
            }
            if (clearance > bestClearance) {
                bestClearance = clearance;
                bestX = x;
                bestY = y;
            }
        }

        var pos = { x: bestX, y: bestY, halfW: box.halfW, halfH: box.halfH };
        _tokenPositionsByRoom[cacheRoomKey][tokenKey] = pos;
        placedPositions[tokenKey] = pos;
        return pos;
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

        var box = { halfW: 10, halfH: 6 };
        var testX = baseCoords.x;
        var testY = baseCoords.y;
        var bestX = testX;
        var bestY = testY;
        var bestClearance = -1;

        // 檢查邊緣位置是否與既有 token 重疊，若重疊則沿邊界向兩側微調推移，避免遮蔽日晷石柱或居民
        for (var offset = 0; offset <= 36; offset += 6) {
            var signs = (offset === 0) ? [0] : [1, -1];
            for (var si = 0; si < signs.length; si++) {
                var s = signs[si];
                var curX = testX;
                var curY = testY;
                if (d === '北' || d === '南') {
                    curX = Math.max(8, Math.min(92, testX + s * offset));
                } else {
                    curY = Math.max(8, Math.min(92, testY + s * offset));
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
            var rawLabel = concisePersonLabel(rawName, rawNpc.card_state);
            npcLabelCounts[rawLabel] = (npcLabelCounts[rawLabel] || 0) + 1;
            validNpcs.push({ npc: rawNpc, id: rawId, key: rawKey, name: rawName, label: rawLabel });
        }
        var npcLabelSeen = {};
        for (var i = 0; i < validNpcs.length; i++) {
            var item = validNpcs[i];
            var nName = item.name;
            var nLabel = item.label;
            if (npcLabelCounts[nLabel] > 1) {
                var seqIdx = npcLabelSeen[nLabel] || 0;
                npcLabelSeen[nLabel] = seqIdx + 1;
                nLabel = nLabel + ' ' + String(seqIdx + 1);
                if (nName === '陌生人') {
                    nName = nLabel;
                }
            }
            var act = visibleActivity(item.npc.current_activity);
            var sub = act ? ('在場人物 · ' + act) : '在場人物';
            tokens.push({
                key: item.key,
                id: item.id,
                name: nName,
                label: nLabel,
                subtitle: sub,
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
                subtitle: '在場玩家',
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
            var oId = obj.id;
            var oKey = 'obj:' + oId;
            if (seenKeys[oKey]) continue;
            seenKeys[oKey] = true;

            var rawLabel = labelOf(obj);
            var oLabel = conciseObjectLabel(rawLabel);
            var fac = isFacility(obj);
            var kind = fac ? 'facility' : 'item';
            var sub = fac ? '場景器物' : '可攜物品';
            var tokClass = fac ? 'sw-token-facility' : 'sw-token-item';

            tokens.push({
                key: oKey,
                id: oId,
                name: rawLabel,
                label: oLabel,
                subtitle: sub,
                kind: kind,
                tokenClass: tokClass,
                data: obj
            });
        }
        var harvs = mapData.harvestables || [];
        for (var hi = 0; hi < harvs.length; hi++) {
            var hObj = harvs[hi];
            if (!hObj) continue;
            var hId = hObj.id || hObj.name || ('harv_' + hi);
            var hKey = 'harv:' + hId;
            if (seenKeys[hKey]) continue;
            seenKeys[hKey] = true;
            var hName = hObj.name || hObj.label || '可採集物';
            tokens.push({
                key: hKey,
                id: hId,
                name: hName,
                label: conciseObjectLabel(hName),
                subtitle: '環境資源',
                kind: 'facility',
                tokenClass: 'sw-token-item',
                data: { id: hId, name: hName, actions: ['Look', 'Gather'] }
            });
        }

        return tokens;
    }

    function renderRoomCanvas() {
        var canvasEl = document.getElementById('sw-room-canvas');
        if (!canvasEl) return;
        var emptyEl = document.getElementById('sw-room-canvas-empty');

        var G = window.SwGrid;
        var mapData = (G && G.mapData) ? G.mapData : {};
        var currentRoomId = mapData.currentRoomId || 'default_room';
        var inPrivate = !!mapData.in_private_room;
        var rName = mapData.currentRoomName || '';
        // 包含 in_private_room 與 roomName，避免進出客房時座標相同但場景更換卻誤判為同一房間
        var currentRoomKey = currentRoomId + '|' + (inPrivate ? 'private' : 'public') + '|' + rName;

        var tokens = collectCurrentTokens();
        var isSameRoom = (_prevRoomId === currentRoomKey);
        if (!isSameRoom) {
            _roomEnterTime = Date.now();
        }
        var allowTransitAnim = isSameRoom && ((Date.now() - _roomEnterTime) > 1200);
        _roomId = currentRoomId;

        if (emptyEl) {
            emptyEl.hidden = (tokens.length > 0);
        }

        if (!isSameRoom) {
            var oldNodes = canvasEl.querySelectorAll('.sw-canvas-token');
            for (var oi = 0; oi < oldNodes.length; oi++) {
                oldNodes[oi].remove();
            }
            _prevTokensMap = {};
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
        var placementOrder = tokens.slice().sort(function (a, b) {
            return ((b.label || '').length - (a.label || '').length);
        });
        var posByKey = {};
        for (var pi = 0; pi < placementOrder.length; pi++) {
            var pTok = placementOrder[pi];
            posByKey[pTok.key] = computeStablePosition(currentRoomId, pTok.key, (pTok.label || '').length, placedPositions);
        }

        if (isSameRoom) {
            for (var oldKey in _prevTokensMap) {
                if (!newTokensMap[oldKey]) {
                    var leavingToken = _prevTokensMap[oldKey];
                    var existingEl = canvasEl.querySelector('[data-token-key="' + leavingToken.key + '"]');
                    var isMovableKind = (leavingToken.kind === 'npc' || leavingToken.kind === 'player');
                    if (existingEl && allowTransitAnim && isMovableKind) {
                        var leaveDir = resolveTransitDirection(leavingToken, 'leave');
                        var oldPos = _tokenPositionsByRoom[currentRoomId][leavingToken.key] || { x: 50, y: 50 };
                        var departPos = getEdgeCoords(leaveDir, oldPos, placedPositions, leavingToken.key);
                        var cue = existingEl.querySelector('.sw-token-dir-cue');
                        if (!cue) {
                            cue = document.createElement('span');
                            cue.className = 'sw-token-dir-cue';
                            existingEl.appendChild(cue);
                        }
                        cue.textContent = '往' + leaveDir + '離去';

                        existingEl.classList.add('is-leaving');
                        (function (node, dx, dy) {
                            requestAnimationFrame(function () {
                                node.style.left = dx + '%';
                                node.style.top = dy + '%';
                                node.style.opacity = '0';
                            });
                            setTimeout(function () {
                                if (node && node.parentNode) node.remove();
                            }, 650);
                        })(existingEl, departPos.x, departPos.y);
                    } else if (existingEl) {
                        existingEl.remove();
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
                tokenEl = document.createElement('button');
                tokenEl.type = 'button';
                tokenEl.className = 'sw-canvas-token ' + token.tokenClass;
                tokenEl.setAttribute('data-token-key', token.key);
                tokenEl.setAttribute('data-obj-id', token.id);
                tokenEl.setAttribute('data-entity-id', token.id);
                tokenEl.setAttribute('data-token-kind', token.kind);
                tokenEl.textContent = token.label;
                tokenEl._swToken = token;
                tokenEl.addEventListener('click', function (e) {
                    e.preventDefault();
                    SwOpenObjectPopover(this._swToken || token);
                });
                if (isNewToken) {
                    var enterDir = resolveTransitDirection(token, 'enter');
                    var startPos = getEdgeCoords(enterDir, pos, placedPositions, token.key);
                    tokenEl.classList.add('is-entering');
                    tokenEl.style.left = startPos.x + '%';
                    tokenEl.style.top = startPos.y + '%';
                    tokenEl.style.opacity = '0';

                    var enterCue = document.createElement('span');
                    enterCue.className = 'sw-token-dir-cue';
                    enterCue.textContent = '自' + enterDir + '走入';
                    tokenEl.appendChild(enterCue);

                    canvasEl.appendChild(tokenEl);

                    (function (node, targetX, targetY, cueSpan) {
                        requestAnimationFrame(function () {
                            node.classList.remove('is-entering');
                            node.style.left = targetX + '%';
                            node.style.top = targetY + '%';
                            node.style.opacity = '1';
                        });
                        setTimeout(function () {
                            if (cueSpan && cueSpan.parentNode) cueSpan.remove();
                        }, 1600);
                    })(tokenEl, pos.x, pos.y, enterCue);
                } else {
                    tokenEl.style.left = pos.x + '%';
                    tokenEl.style.top = pos.y + '%';
                    tokenEl.style.opacity = '1';
                    canvasEl.appendChild(tokenEl);
                }
            } else {
                tokenEl._swToken = token;
                tokenEl.style.left = pos.x + '%';
                tokenEl.style.top = pos.y + '%';
                if (tokenEl.firstChild && tokenEl.firstChild.nodeType === Node.TEXT_NODE) {
                    tokenEl.firstChild.textContent = token.label;
                } else {
                    tokenEl.textContent = token.label;
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

        // 依據實際 DOM 像素尺寸做 AABB 防重疊與防裁切微調，並回寫穩定快取
        var cw = canvasEl.clientWidth || 0;
        var ch = canvasEl.clientHeight || 0;
        if (cw > 100 && ch > 100 && tokens.length > 1) {
            var bucket = (cw <= 520) ? 'm' : 'd';
            var cacheRoomKey = currentRoomId + ':' + bucket;
            if (!_tokenPositionsByRoom[cacheRoomKey]) {
                _tokenPositionsByRoom[cacheRoomKey] = {};
            }
            var items = [];
            for (var ti = 0; ti < tokens.length; ti++) {
                var tk = tokens[ti];
                var el = canvasEl.querySelector('[data-token-key="' + tk.key + '"]');
                var p = posByKey[tk.key];
                if (!el || !p || el.classList.contains('is-entering') || el.classList.contains('is-leaving')) continue;
                var hw = Math.min(44, Math.max(5, ((el.offsetWidth || 64) / 2 / cw) * 100));
                var hh = Math.min(18, Math.max(4, ((el.offsetHeight || 32) / 2 / ch) * 100));
                items.push({ key: tk.key, el: el, x: p.x, y: p.y, hw: hw, hh: hh });
            }
            var moved = false;
            for (var iter = 0; iter < 24; iter++) {
                var anyOverlap = false;
                for (var a = 0; a < items.length; a++) {
                    var ia = items[a];
                    ia.x = Math.max(ia.hw + 2, Math.min(98 - ia.hw, ia.x));
                    ia.y = Math.max(ia.hh + 2.5, Math.min(97.5 - ia.hh, ia.y));
                    for (var b = a + 1; b < items.length; b++) {
                        var ib = items[b];
                        var reqX = ia.hw + ib.hw + 1.6;
                        var reqY = ia.hh + ib.hh + 1.6;
                        var dx = ia.x - ib.x;
                        var dy = ia.y - ib.y;
                        var adx = Math.abs(dx);
                        var ady = Math.abs(dy);
                        if (adx < reqX && ady < reqY) {
                            anyOverlap = true;
                            moved = true;
                            var penX = reqX - adx;
                            var penY = reqY - ady;
                            if (penY <= penX) {
                                var sy = (dy >= 0) ? 1 : -1;
                                ia.y += sy * (penY * 0.55);
                                ib.y -= sy * (penY * 0.55);
                            } else {
                                var sx = (dx >= 0) ? 1 : -1;
                                ia.x += sx * (penX * 0.55);
                                ib.x -= sx * (penX * 0.55);
                            }
                        }
                    }
                }
                if (!anyOverlap) break;
            }
            if (moved) {
                for (var mi = 0; mi < items.length; mi++) {
                    var it = items[mi];
                    it.x = Math.round(Math.max(it.hw + 2, Math.min(98 - it.hw, it.x)) * 10) / 10;
                    it.y = Math.round(Math.max(it.hh + 2.5, Math.min(97.5 - it.hh, it.y)) * 10) / 10;
                    it.el.style.left = it.x + '%';
                    it.el.style.top = it.y + '%';
                    _tokenPositionsByRoom[cacheRoomKey][it.key] = { x: it.x, y: it.y, halfW: it.hw, halfH: it.hh };
                }
            }
        }

        _prevRoomId = currentRoomKey;
        _prevTokensMap = newTokensMap;
    }
    var SW_ACTION_LABELS = {
        // Core
        'Look': '觀察', 'Read': '閱讀', 'Smell': '嗅聞', 'Use': '使用', 'Open': '開啟',
        'Queue': '候號', 'AcceptJob': '應工', 'Rest': '歇腳', 'BuySoup': '買滿碗熱湯',
        'Gather': '採集', 'Acquire': '採買入袋', 'Wash': '洗漱淨手', 'Sit': '坐下',
        'Taste': '品嚐', 'Take': '拾取', 'Chop': '砍伐', 'Operate': '操作',
        'Move': '前往', 'Talk': '對話', 'Trade': '交易', 'Borrow': '偷竊',
        'Subdue': '攻擊', 'Slay': '致死', 'Attack': '攻擊',
        // Wood / Fire / Camp
        'MakeFire': '生火', 'AddFuel': '添柴', 'SplitWood': '劈柴', 'RefinePlank': '裁鋸木板',
        'BurnCharcoal': '悶燒木炭', 'CraftTorch': '扎製火把', 'CraftHandle': '車削木柄',
        'CollectCharcoal': '收取木炭', 'RoastFood': '炙烤食物', 'RepairStructure': '修繕加固',
        // Ore / Smithing
        'WashOre': '淘洗礦砂', 'CrushOre': '粉碎礦石', 'SmeltIngot': '熔煉金屬',
        'ForgeTool': '鍛打器具', 'CastHardware': '澆鑄五金', 'SharpenBlade': '砥石磨刃',
        'Quench': '淬火', 'ForgeRepairStrip': '鍛修鐵條',
        // Herbs / Alchemy
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
        'PickType': '揀字', 'RollInk': '滾墨', 'Explore': '深入探索'
    };
    window.SwActionLabels = SW_ACTION_LABELS;

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
        } else {
            var actionLabels = SW_ACTION_LABELS;
            var acts = (token.data && (token.data.actions || token.data.sockets)) || [];
            var nonLookActs = Array.isArray(acts) ? acts.filter(function (a) { return a && a !== 'Look'; }) : [];
            if (nonLookActs.length === 0) {
                actionsEl.innerHTML = '';
                actionsEl.hidden = true;
                return;
            }
            actionsEl.hidden = false;
            for (var bi = 0; bi < nonLookActs.length; bi++) {
                var act = nonLookActs[bi];
                var oBtn = document.createElement('button');
                oBtn.type = 'button';
                oBtn.className = 'sw-popover-act-btn log-object-action';
                oBtn.setAttribute('data-entity-id', targetId);
                oBtn.setAttribute('data-action', act);
                oBtn.setAttribute('data-target-name', targetName);
                oBtn.setAttribute('data-target-type', 'object');
                oBtn.textContent = actionLabels[act] || act;
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
        } else if (kind === 'object') {
            var nonLook = Array.isArray(actions) ? actions.filter(function (a) { return a && a !== 'Look'; }) : [];
            if (!Array.isArray(actions) || nonLook.length === 0) {
                actionsEl.innerHTML = '';
                actionsEl.hidden = true;
                return;
            }
            actionsEl.hidden = false;
            for (var i = 0; i < nonLook.length; i++) {
                var act = nonLook[i];
                var label = actionLabels[act] || act;
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
    var POPOVER_IDLE_TIMEOUT_MS = 20000;

    function SwResetPopoverIdleTimer() {
        if (_popoverIdleTimer) {
            clearTimeout(_popoverIdleTimer);
            _popoverIdleTimer = null;
        }
        if (!_activePopoverToken) return;
        var timeoutMs = window.__SW_POPOVER_IDLE_MS || POPOVER_IDLE_TIMEOUT_MS;
        _popoverIdleTimer = setTimeout(function () {
            SwCloseObjectPopover();
        }, timeoutMs);
    }
    window.SwResetPopoverIdleTimer = SwResetPopoverIdleTimer;

    function SwOpenObjectPopover(token) {
        if (!token) return;
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
        if (subtitleEl) subtitleEl.textContent = token.subtitle || '';

        renderPopoverActions(token);

        var G = window.SwGrid;
        var currentRoomId = (G && G.mapData && G.mapData.currentRoomId) || _roomId || '';
        var cacheKey = currentRoomId + '|' + token.id;
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

        // 在場實體（NPC + 其他玩家）
        // L4 只顯示 NPC，排除自己與其他玩家（玩家 id 污染不進 NPC 卡）
        var others = (mapData.entities || []).filter(function (e) {
            return e.id !== myId && e.kind !== 'player' && e.kind !== 'Player';
        });
        var convertedNpcs = [];
        for (var ei = 0; ei < others.length; ei++) {
            var ent = others[ei];
            var knownName = _knownNpcNames[ent.id] || '';
            var rawDisp = knownName || displayNameOf(ent);
            var cState = knownName ? 'named' : 'stranger';
            html += '<button type="button" class="sw-obj-card sw-person-row card-' + cState + '" data-entity-id="' + escFn(ent.id) + '" data-obj-id="' + escFn(ent.id) + '" data-card-state="' + cState + '">';
            html += '<span class="sw-person-name">' + escFn(rawDisp) + '</span>';
            html += '</button>';
            convertedNpcs.push({
                npc_id: ent.id,
                display_name: rawDisp,
                card_state: cState,
                current_activity: ent.current_activity || ''
            });
        }
        html += '</div>' + exitCardsHtml(escFn);
        if (G.objectListEl) {
            G.objectListEl.innerHTML = html;
        }
        var l4El = document.getElementById('sw-l4-objects');
        if (l4El && l4El !== G.objectListEl) {
            l4El.innerHTML = html;
        }
        renderCategorizedPanels(convertedNpcs, mapData.objects || []);
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
    function renderV5NpcCards(npcs, objects) {
        _currentNpcs = (npcs || []).map(function (npc) {
            var copy = Object.assign({}, npc);
            if (copy.npc_id) {
                if (copy.card_state === 'named' && copy.display_name && !isPlaceholderName(copy.display_name)) {
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
        var strangerCards = _currentNpcs.filter(function (c) {
            return c.kind !== 'crowd' && c.card_state !== 'named' && /^陌生人/.test(c.display_name || '');
        });
        if (strangerCards.length === 1) {
            strangerCards[0].display_name = '陌生人';
        } else if (strangerCards.length > 1) {
            for (var sci = 0; sci < strangerCards.length; sci++) {
                strangerCards[sci].display_name = '陌生人' + String(sci + 1);
            }
        }
        _currentObjects = objects || [];
        var el = document.getElementById('sw-l4-objects');
        if (el) {
            el.innerHTML = composeSurface(_currentNpcs, _currentObjects);
        }
        renderCategorizedPanels(_currentNpcs, _currentObjects);
    }
    // RoomView 是世界 tick 的即時真相；保留觀察態名稱，補進／移除實際在場人物。
    function syncNpcEntities(entities, objects) {
        var previous = {};
        var previousOrder = [];
        var crowd = null;
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
        for (var ei = 0; ei < source.length; ei++) {
            var ent = source[ei];
            if (ent.kind !== 'npc' && ent.kind !== 'Npc') continue;
            if (ent.id && String(ent.id).indexOf('ambient_') === 0) continue;
            live[ent.id] = ent;
        }

        var next = [];
        var selected = {};
        var VISIBLE_LIMIT = 5;
        function resolveDisplayName(entity, safeRecord) {
            var eid = entity && entity.id;
            if (eid && _knownNpcNames[eid]) {
                return _knownNpcNames[eid];
            }
            if (safeRecord && safeRecord.card_state === 'named' && safeRecord.display_name && !isPlaceholderName(safeRecord.display_name)) {
                return safeRecord.display_name;
            }
            if (safeRecord && safeRecord.display_name) {
                return safeRecord.display_name;
            }
            return '陌生人';
        }
        function resolveCardState(entity, safeRecord) {
            var eid = entity && entity.id;
            if (eid && _knownNpcNames[eid]) {
                return 'named';
            }
            return safeRecord ? safeRecord.card_state : 'stranger';
        }

        for (var pri = 0; pri < source.length; pri++) {
            var priorityEntity = source[pri];
            if ((priorityEntity.kind !== 'npc' && priorityEntity.kind !== 'Npc') || !priorityEntity.interaction_priority || (priorityEntity.id && String(priorityEntity.id).indexOf('ambient_') === 0)) continue;
            var prioritySafe = previous[priorityEntity.id];
            var priName = resolveDisplayName(priorityEntity, prioritySafe);
            next.push({
                kind: 'npc',
                npc_id: priorityEntity.id,
                display_name: priName,
                card_state: resolveCardState(priorityEntity, prioritySafe),
                current_activity: priorityEntity.current_activity || '',
                interaction_priority: true
            });
            selected[priorityEntity.id] = true;
        }
        // 先保留玩家原本正在辨認的人；有人離開後，才讓新到者補進視野。
        for (var pi = 0; pi < previousOrder.length && next.length < VISIBLE_LIMIT; pi++) {
            var priorId = previousOrder[pi];
            var priorEntity = live[priorId];
            if (!priorEntity || selected[priorId]) continue;
            var priorSafe = previous[priorId];
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
        for (var li = 0; li < source.length && next.length < VISIBLE_LIMIT; li++) {
            var ent = source[li];
            if ((ent.kind !== 'npc' && ent.kind !== 'Npc') || selected[ent.id] || (ent.id && String(ent.id).indexOf('ambient_') === 0)) continue;
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
            strangers[si].display_name = '陌生人' + (strangers.length > 1 ? String(si + 1) : '');
        }
        if (Object.keys(live).length > next.length) {
            next.push(crowd || { kind: 'crowd', display_name: '其餘人影', card_state: 'crowd' });
        }
        renderV5NpcCards(next, objects || _currentObjects);
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
        recordNpcDialogue: SwRecordNpcDialogue
    };
    if (typeof window !== 'undefined') {
        var _resizeTimer = null;
        window.addEventListener('resize', function () {
            clearTimeout(_resizeTimer);
            _resizeTimer = setTimeout(function () {
                renderRoomCanvas();
            }, 80);
        });
    }
    console.log('[sw-object-list] v0.37.1 loaded');
})();
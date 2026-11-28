/* 《奇點世界・銀鷗外島》純自然水彩底圖 × 模組化透明貼圖 × 真方位輿圖引擎（v20261128r8）
 * - 純自然水彩底圖（樹・陸・山・河・海）＋ /map-stamps.json 透明手繪城鎮／聚落／特殊地標／動態農田貼圖
 * - 嚴格正北朝上、無連線、無格線、無室內走圖、點空白處完全不移動
 * - 野外直接於統一全島水彩底圖精準定位放大（徹底淘汰旋轉方向錯亂之共用小方塊圖），每步隨真座標滑動
 * - 嚴格方位保真防重疊排布（零疊壓、零截斷、絕不把「北」推成「西」）
 * - 非方位局部支線（登望海台／進屋／切捷徑等）留給文字區「此處門戶」，不混入八向方位盤
 */
(function () {
  'use strict';

  var VER = '20261128r11';
  var ISLAND_TERRAIN_URL = '/maps/island_terrain_base.webp?v=' + VER;
  var STAMPS_MANIFEST_URL = '/map-stamps.json?v=' + VER;
  var ISLAND_BOUNDS = { minX: 1357.71, maxX: 1507.06, minY: 627.98, maxY: 793.21 };
  var MAJOR_TOWNS_OVERVIEW = [
    { slug: 'white_gull_harbor', cellId: 4616, name: '白鷗港', pos: [1472.82, 700.58] },
    { slug: 'gull_tail_port', cellId: 4613, name: '鷗尾漁港', pos: [1436.8, 711.0] },
    { slug: 'west_strait_ferry', cellId: 4321, name: '西峽渡口', pos: [1375.92, 662.82] },
    { slug: 'greypine_post', cellId: 4525, name: '灰松林驛', pos: [1416.94, 687.79] },
    { slug: 'north_cape_beacon', cellId: 4073, name: '北岬燈塔', pos: [1425.0, 636.7] }
  ];

  var stampsManifest = null;
  var stampsLoading = false;
  var stampsListeners = [];

  function ensureStampsManifest(onReady) {
    if (stampsManifest) return stampsManifest;
    if (typeof onReady === 'function') stampsListeners.push(onReady);
    if (stampsLoading) return null;
    stampsLoading = true;
    fetch(STAMPS_MANIFEST_URL, { credentials: 'same-origin' })
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (data) {
        stampsLoading = false;
        if (!data || !data.stamps) return;
        stampsManifest = data;
        var cbs = stampsListeners.slice();
        stampsListeners = [];
        for (var i = 0; i < cbs.length; i++) {
          try { cbs[i](data); } catch (_) {}
        }
      })
      .catch(function () {
        stampsLoading = false;
      });
    return null;
  }

  function esc(str) {
    return String(str == null ? '' : str)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function clamp(v, lo, hi) {
    return Math.max(lo, Math.min(hi, v));
  }

  function shortPlaceName(fullName) {
    var s = String(fullName || '').trim();
    if (!s) return '';
    var parts = s.split('・');
    var tail = parts.length > 1 ? parts[parts.length - 1] : s;
    return tail.replace(/（[^）]*）/g, '').trim() || tail;
  }

  function formatRegionShort(cellTitle, fallbackName) {
    var raw = String(cellTitle || '').trim();
    if (!raw) return shortPlaceName(fallbackName);
    // 保留核心地名並去掉過長括號註記，例如「博昂鄉鎮・望海驛道（白鷗港南）」→「白鷗港南・望海驛道」或「望海驛道」
    var mParen = raw.match(/（([^）]+)）/);
    var core = raw.replace(/（[^）]*）/g, '').trim();
    var parts = core.split('・').filter(Boolean);
    if (parts.length === 1) {
      return parts[0];
    }
    var last = parts[parts.length - 1];
    var head = parts[0].replace(/鄉鎮$|林谷$/, '');
    if (mParen && mParen[1] && mParen[1].length <= 6 && last.length <= 6) {
      return mParen[1];
    }
    if (head && last && head !== last) {
      var joined = head + last;
      if (joined.length <= 8) return joined;
    }
    return last || shortPlaceName(fallbackName);
  }

  function cleanDirWord(dir) {
    return String(dir || '').replace(/（[^）]*）/g, '').trim();
  }

  function parseCardinalDir(dir) {
    var d = cleanDirWord(dir);
    if (d.indexOf('東北') >= 0) return { label: '東北', vec: [0.7071, -0.7071], cardinal: true };
    if (d.indexOf('西北') >= 0) return { label: '西北', vec: [-0.7071, -0.7071], cardinal: true };
    if (d.indexOf('東南') >= 0) return { label: '東南', vec: [0.7071, 0.7071], cardinal: true };
    if (d.indexOf('西南') >= 0) return { label: '西南', vec: [-0.7071, 0.7071], cardinal: true };
    if (d.indexOf('北') >= 0) return { label: '北', vec: [0, -1], cardinal: true };
    if (d.indexOf('南') >= 0) return { label: '南', vec: [0, 1], cardinal: true };
    if (d.indexOf('東') >= 0) return { label: '東', vec: [1, 0], cardinal: true };
    if (d.indexOf('西') >= 0) return { label: '西', vec: [-1, 0], cardinal: true };
    return null;
  }

  function dirToVector(dir) {
    var c = parseCardinalDir(dir);
    if (c) return c.vec;
    var d = cleanDirWord(dir);
    if (d.indexOf('上') >= 0) return [0.28, -0.92];
    if (d.indexOf('下') >= 0) return [-0.28, 0.92];
    if (d.indexOf('進') >= 0 || d.indexOf('內') >= 0 || d.indexOf('入') >= 0) return [0.82, -0.35];
    if (d.indexOf('出') >= 0 || d.indexOf('外') >= 0 || d.indexOf('退') >= 0) return [-0.82, 0.35];
    return null;
  }

  function projectIslandPercent(pos) {
    if (!Array.isArray(pos) || pos.length < 2) return [50, 50];
    var cx = (ISLAND_BOUNDS.minX + ISLAND_BOUNDS.maxX) * 0.5;
    var cy = (ISLAND_BOUNDS.minY + ISLAND_BOUNDS.maxY) * 0.5;
    var span = Math.max(ISLAND_BOUNDS.maxX - ISLAND_BOUNDS.minX, ISLAND_BOUNDS.maxY - ISLAND_BOUNDS.minY);
    var px = 50 + ((pos[0] - cx) / span) * 86;
    var py = 50 + ((pos[1] - cy) / span) * 86;
    return [clamp(px, 5, 95), clamp(py, 5, 95)];
  }

  function resolveTownDistrict(townSpec, nid) {
    if (!townSpec || !Array.isArray(townSpec.districts) || !townSpec.districts.length || !nid) {
      return null;
    }
    for (var i = 0; i < townSpec.districts.length; i++) {
      var d = townSpec.districts[i];
      var mems = d.members || [];
      for (var j = 0; j < mems.length; j++) {
        if (nid === mems[j] || nid.indexOf(mems[j] + ':') === 0) return d;
      }
      if (nid.indexOf(':inn_room_') >= 0 && mems.indexOf(nid.split(':').slice(0, 2).join(':') + ':inn_upstairs') >= 0) {
        return d;
      }
    }
    return townSpec.districts[0];
  }

  function lookupTownAnchor(townSpec, nid) {
    if (!townSpec || !nid) return null;
    var i;
    var pills = townSpec.pills || [];
    for (i = 0; i < pills.length; i++) {
      if (pills[i].nid === nid) {
        return { x: pills[i].x, y: pills[i].y, label: pills[i].label, kind: pills[i].kind || 'main', exact: true };
      }
      if (nid.indexOf(pills[i].nid + ':') === 0) {
        return { x: pills[i].x, y: pills[i].y, label: pills[i].label, kind: 'alley_sub', exact: false };
      }
    }
    var alleys = townSpec.alleys || [];
    for (i = 0; i < alleys.length; i++) {
      if (alleys[i].nid === nid) {
        return { x: alleys[i].lx, y: alleys[i].ly, label: alleys[i].name, kind: 'alley', exact: true };
      }
      if (nid.indexOf(alleys[i].nid + ':') === 0) {
        return { x: alleys[i].lx, y: alleys[i].ly, label: alleys[i].name, kind: 'alley_sub', exact: false };
      }
    }
    if (nid.indexOf(':inn_room_') >= 0 || nid.indexOf(':inn_upstairs') >= 0) {
      var prefix = nid.split(':').slice(0, 2).join(':');
      var up = lookupTownAnchor(townSpec, prefix + ':inn_upstairs');
      if (up) return { x: up.x, y: up.y, label: up.label, kind: 'inn_sub', exact: false };
      var tav = lookupTownAnchor(townSpec, prefix + ':rusty_anchor_tavern');
      if (tav) return { x: tav.x, y: tav.y, label: tav.label, kind: 'inn_sub', exact: false };
    }
    if (nid.indexOf(':tally_house') >= 0) {
      var p2 = nid.split(':').slice(0, 2).join(':');
      var ts = lookupTownAnchor(townSpec, p2 + ':tally_house') || lookupTownAnchor(townSpec, p2 + ':south_dock');
      if (ts) return { x: ts.x, y: ts.y, label: '港務記籌寮屋', kind: 'house_sub', exact: ts.exact };
    }
    return null;
  }

  function buildTownButtons(scene, townSpec) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var refW = townSpec.width || 900;
    var refH = townSpec.height || 1200;
    var curAnchor = lookupTownAnchor(townSpec, curNid) || { x: refW * 0.5, y: refH * 0.5, label: shortPlaceName(scene.name), kind: 'main', exact: false };
    var buttons = [];
    var mappedExitIds = {};

    var curLabel = curAnchor.exact ? curAnchor.label : (shortPlaceName(scene.name) || curAnchor.label);
    buttons.push({
      nid: curNid,
      label: curLabel,
      xp: clamp((curAnchor.x / refW) * 100, 8, 92),
      yp: clamp((curAnchor.y / refH) * 100, 7, 93),
      isCurrent: true,
      kind: curAnchor.kind || 'main',
      title: scene.name || curLabel
    });

    var exits = Array.isArray(scene.exits) ? scene.exits : [];
    var usedNids = {};
    usedNids[curNid] = true;

    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (!ex || !ex.to || usedNids[ex.to]) continue;
      var anc = lookupTownAnchor(townSpec, ex.to);
      // 民宅、樓上客房與非方位局部動作保留於文字區「此處門戶」
      if ((ex.to_kind === 'house' || ex.to_kind === 'house_vacant') && !(anc && anc.exact)) continue;
      if (ex.to.indexOf(':inn_room_') >= 0 && curNid.indexOf(':inn_upstairs') >= 0) continue;

      var xp, yp, label;
      var shortDest = shortPlaceName(ex.to_name);
      var card = parseCardinalDir(ex.dir);
      var dWord = card ? card.label : cleanDirWord(ex.dir);
      var isLeave = !!((ex.to_cell_id && ex.to_cell_id !== curCellId) || String(ex.to || '').indexOf('oi:way_') === 0);

      if (anc && anc.exact) {
        xp = (anc.x / refW) * 100;
        yp = (anc.y / refH) * 100;
        if (Math.hypot(xp - buttons[0].xp, yp - buttons[0].yp) < 4.5) {
          var vSub = dirToVector(ex.dir) || [0, -1];
          xp = clamp(buttons[0].xp + vSub[0] * 13, 12, 88);
          yp = clamp(buttons[0].yp + vSub[1] * 8.5, 8, 92);
        }
        label = anc.label || shortDest;
      } else if (isLeave) {
        var vOut = card ? card.vec : dirToVector(ex.dir);
        if (!vOut && Array.isArray(scene.pos) && Array.isArray(ex.to_pos)) {
          var dxW = ex.to_pos[0] - scene.pos[0];
          var dyW = ex.to_pos[1] - scene.pos[1];
          var lnW = Math.hypot(dxW, dyW) || 1;
          vOut = [dxW / lnW, dyW / lnW];
        }
        vOut = vOut || [0, 1];
        xp = clamp(buttons[0].xp + vOut[0] * 16, 10, 90);
        yp = clamp(buttons[0].yp + vOut[1] * 11, 6, 94);
        var regionDest = formatRegionShort(ex.to_cell_title, ex.to_name);
        label = (dWord ? ('出城向' + dWord + ' · ') : '出城 · ') + regionDest;
      } else {
        var vLoc = dirToVector(ex.dir);
        if (!vLoc && Array.isArray(scene.pos) && Array.isArray(ex.to_pos)) {
          var ldx = ex.to_pos[0] - scene.pos[0];
          var ldy = ex.to_pos[1] - scene.pos[1];
          var lln = Math.hypot(ldx, ldy);
          if (lln > 0.001) vLoc = [ldx / lln, ldy / lln];
        }
        vLoc = vLoc || [0, -1];
        xp = clamp(buttons[0].xp + vLoc[0] * 15, 10, 90);
        yp = clamp(buttons[0].yp + vLoc[1] * 11, 8, 92);
        label = (dWord && dWord !== shortDest ? (dWord + ' · ') : '') + shortDest;
      }

      usedNids[ex.to] = true;
      mappedExitIds[ex.to] = true;
      buttons.push({
        nid: ex.to,
        label: label,
        xp: clamp(xp, 8, 92),
        yp: clamp(yp, 6, 94),
        dirVec: card ? card.vec : null,
        isCurrent: false,
        isSecret: ex.kind === 'secret',
        isLeaveCell: isLeave,
        kind: ex.to_kind || ex.kind || 'exit',
        title: ex.ambient_cue || ex.to_cell_title || ex.to_name || label
      });
    }

    return { buttons: buttons, mappedExitIds: mappedExitIds };
  }

  // 野外同地塊內各子節點微位移（讓每一步都在銀鷗島真地圖上看得見腳步前進，絕不釘死在原地）
  function computeSubNodeOffset(scene, curPt, exits) {
    var nid = String(scene.node_id || '');
    var offX = 0;
    var offY = 0;
    // 若節點有跨地塊出口，依該出口方向微偏向邊緣（如 approach_a 偏往北城方向）
    var crossVecs = [];
    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (ex && ex.to_cell_id && ex.to_cell_id !== scene.cell_id) {
        if (Array.isArray(ex.to_cell_center)) {
          var tp = projectIslandPercent(ex.to_cell_center);
          var dx = tp[0] - curPt[0];
          var dy = tp[1] - curPt[1];
          var ln = Math.hypot(dx, dy);
          if (ln > 0.1) crossVecs.push([dx / ln, dy / ln]);
        } else {
          var c = parseCardinalDir(ex.dir);
          if (c) crossVecs.push(c.vec);
        }
      }
    }
    if (crossVecs.length === 1 && nid.indexOf(':0') < 0) {
      offX = crossVecs[0][0] * 0.95;
      offY = crossVecs[0][1] * 0.95;
    } else if (nid.indexOf(':approach_a') >= 0) {
      offY = -0.85;
    } else if (nid.indexOf(':approach_b') >= 0) {
      offY = 0.85;
    } else if (nid.indexOf(':high') >= 0) {
      offX = 0.65; offY = -0.55;
    } else if (nid.indexOf(':low') >= 0) {
      offX = -0.65; offY = 0.55;
    } else if (nid.indexOf(':far') >= 0) {
      offX = -0.75; offY = -0.45;
    }
    return [clamp(curPt[0] + offX, 6, 94), clamp(curPt[1] + offY, 6, 94)];
  }

  // 野外統一銀鷗島水彩底圖節點建構：採用節點真實世界座標（scene.pos / ex.to_pos），每步皆於水彩底圖上平滑推進零等待
  function buildIslandLocalButtons(scene, sm) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var exits = Array.isArray(scene.exits) ? scene.exits : [];
    var rawPos = (Array.isArray(scene.pos) && scene.pos.length >= 2)
      ? scene.pos
      : (sm.cell_center || [1472.82, 700.58]);
    var curPt = projectIslandPercent(rawPos);
    var curLabel = shortPlaceName(scene.name) || formatRegionShort(sm.cell_title, '此處');

    var mappedExitIds = {};
    var buttons = [{
      nid: curNid,
      label: curLabel,
      xp: curPt[0],
      yp: curPt[1],
      isCurrent: true,
      kind: scene.kind || 'wild',
      title: scene.name || sm.cell_title || curLabel
    }];

    var used = {};
    used[curNid] = true;
    var fallbackIdx = 0;

    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (!ex || !ex.to || used[ex.to]) continue;
      // 僅把民宅屋舍留在文字區「此處門戶」；所有野外山徑、捷徑、坡口、路標、村口一律直接顯示於地圖上！
      if (ex.to_kind === 'house' || ex.to_kind === 'house_vacant') continue;

      var isCross = !!(ex.to_cell_id && ex.to_cell_id !== curCellId);
      var isWay = String(ex.to || '').indexOf('oi:way_') === 0;
      var card = parseCardinalDir(ex.dir);

      used[ex.to] = true;
      mappedExitIds[ex.to] = true;

      // 優先採真實 2D 座標向量（scene.pos -> ex.to_pos）
      var vec = null;
      if (Array.isArray(scene.pos) && Array.isArray(ex.to_pos)) {
        var pdx = ex.to_pos[0] - scene.pos[0];
        var pdy = ex.to_pos[1] - scene.pos[1];
        var pln = Math.hypot(pdx, pdy);
        if (pln > 0.001) vec = [pdx / pln, pdy / pln];
      }
      if (!vec && card) vec = card.vec;
      if (!vec && isCross && Array.isArray(ex.to_cell_center)) {
        var tPt0 = projectIslandPercent(ex.to_cell_center);
        var dx0 = tPt0[0] - curPt[0];
        var dy0 = tPt0[1] - curPt[1];
        var ln0 = Math.hypot(dx0, dy0);
        if (ln0 > 0.05) vec = [dx0 / ln0, dy0 / ln0];
      }
      if (!vec) vec = dirToVector(ex.dir);
      if (!vec) {
        var ang = (fallbackIdx++ / Math.max(1, exits.length)) * Math.PI * 2 - Math.PI * 0.5;
        vec = [Math.cos(ang), Math.sin(ang)];
      }

      // 以真實座標投影為基準，適度保底最小間距（確保緊鄰按鈕好點且方向完全忠於地形）
      var xp, yp;
      var dstRaw = (Array.isArray(ex.to_pos) && ex.to_pos.length >= 2) ? ex.to_pos : ex.to_cell_center;
      if (Array.isArray(dstRaw) && dstRaw.length >= 2) {
        var realPt = projectIslandPercent(dstRaw);
        var rdx = realPt[0] - curPt[0];
        var rdy = realPt[1] - curPt[1];
        var rDist = Math.hypot(rdx, rdy);
        if (rDist > 0.12) {
          var clampedR = clamp(rDist, 4.4, 7.8);
          xp = curPt[0] + (rdx / rDist) * clampedR;
          yp = curPt[1] + (rdy / rDist) * clampedR;
        } else {
          xp = curPt[0] + vec[0] * 4.8;
          yp = curPt[1] + vec[1] * 4.8;
        }
      } else {
        var localR = (isCross || isWay) ? 5.6 : 4.6;
        xp = curPt[0] + vec[0] * localR;
        yp = curPt[1] + vec[1] * localR;
      }

      var shortDest = (isCross && !isWay)
        ? formatRegionShort(ex.to_cell_title, ex.to_name)
        : shortPlaceName(ex.to_name);
      var dWord = card ? card.label : cleanDirWord(ex.dir);
      var label;
      if (!card && !isCross && !isWay && dWord && dWord.length >= 2) {
        label = dWord;
      } else {
        label = (dWord && dWord !== shortDest) ? (dWord + ' · ' + shortDest) : shortDest;
      }

      buttons.push({
        nid: ex.to,
        label: label,
        xp: clamp(xp, 4, 96),
        yp: clamp(yp, 4, 96),
        dirVec: card ? card.vec : vec,
        isCurrent: false,
        isSecret: ex.kind === 'secret',
        isLeaveCell: isCross || isWay,
        isSubTrail: !card && !isCross && !isWay,
        kind: ex.to_kind || ex.kind || 'exit',
        title: ex.ambient_cue || ex.to_cell_title || ex.to_name || label
      });
    }

    return { buttons: buttons, mappedExitIds: mappedExitIds };
  }

  function buildDungeonButtons(scene) {
    var curNid = scene.node_id;
    var curLabel = shortPlaceName(scene.name) || '此處';
    var mappedExitIds = {};
    var buttons = [{
      nid: curNid,
      label: curLabel,
      xp: 50,
      yp: 54,
      isCurrent: true,
      kind: scene.kind || 'dungeon',
      title: scene.name || curLabel
    }];

    var exits = Array.isArray(scene.exits) ? scene.exits : [];
    var used = {};
    used[curNid] = true;
    var dFallback = 0;

    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (!ex || !ex.to || used[ex.to]) continue;
      var card = parseCardinalDir(ex.dir);
      var vec = card ? card.vec : dirToVector(ex.dir);
      if (!vec) {
        var dang = (dFallback++ / Math.max(1, exits.length)) * Math.PI * 2 - Math.PI * 0.5;
        vec = [Math.cos(dang), Math.sin(dang)];
      }
      used[ex.to] = true;
      mappedExitIds[ex.to] = true;

      var xp = clamp(50 + vec[0] * 30, 14, 86);
      var yp = clamp(54 + vec[1] * 28, 12, 88);
      var shortDest = shortPlaceName(ex.to_name);
      var dWord = card ? card.label : cleanDirWord(ex.dir);
      var label = (dWord && dWord !== shortDest) ? (dWord + ' · ' + shortDest) : shortDest;

      buttons.push({
        nid: ex.to,
        label: label,
        xp: xp,
        yp: yp,
        dirVec: vec,
        isCurrent: false,
        isSecret: ex.kind === 'secret',
        isLeaveCell: !!(ex.to_cell_id && ex.to_cell_id !== scene.cell_id),
        kind: ex.to_kind || ex.kind || 'exit',
        title: ex.ambient_cue || ex.to_name || label
      });
    }
    return { buttons: buttons, mappedExitIds: mappedExitIds };
  }

  function estimatePillPx(b, compact) {
    var fontPx = compact ? 11 : 12;
    var txt = String(b.label || '');
    var units = 0;
    for (var i = 0; i < txt.length; i++) {
      units += txt.charCodeAt(i) > 127 ? 1.04 : 0.62;
    }
    var padX = b.isCurrent ? 28 : 18;
    var w = Math.ceil(units * fontPx + padX);
    var h = b.isCurrent ? (compact ? 26 : 30) : (compact ? 25 : 28);
    return { w: Math.min(w, compact ? 144 : 176), h: h };
  }

  function computeStageFrame(wrapW, wrapH, stageW, stageH, buttons, mode) {
    if (!wrapW || !wrapH) return { width: 320, height: 320, tx: 0, ty: 0 };
    var fitScale = Math.min(wrapW / stageW, wrapH / stageH);
    if (mode === 'overview') {
      var ow = Math.round(stageW * fitScale);
      var oh = Math.round(stageH * fitScale);
      return { width: ow, height: oh, tx: Math.round((wrapW - ow) * 0.5), ty: Math.round((wrapH - oh) * 0.5) };
    }

    var scale = fitScale;
    var focusX = 0.5;
    var focusY = 0.5;

    if (mode === 'island_local' && buttons && buttons.length) {
      // 野外統一銀鷗島水彩底圖：放大至約 ~26% 島寬，讓玩家看清周遭海岸、河流、林地與鄰村貼圖
      var cur = buttons[0];
      var minX = cur.xp / 100, maxX = minX, minY = cur.yp / 100, maxY = minY;
      for (var ii = 1; ii < buttons.length; ii++) {
        var ibx = buttons[ii].xp / 100, iby = buttons[ii].yp / 100;
        if (ibx < minX) minX = ibx;
        if (ibx > maxX) maxX = ibx;
        if (iby < minY) minY = iby;
        if (iby > maxY) maxY = iby;
      }
      var spanX = Math.max(0.22, (maxX - minX) + 0.14);
      var spanY = Math.max(0.20, (maxY - minY) + 0.14);
      var targetScale = Math.min(wrapW / (stageW * spanX), wrapH / (stageH * spanY));
      var minLocalScale = Math.max(wrapW / stageW, wrapH / stageH) * 1.45;
      var maxLocalScale = Math.max(wrapW / stageW, wrapH / stageH) * 3.8;
      scale = clamp(targetScale, minLocalScale, maxLocalScale);
      focusX = ((minX + maxX) * 0.5) * 0.35 + (cur.xp / 100) * 0.65;
      focusY = ((minY + maxY) * 0.5) * 0.35 + (cur.yp / 100) * 0.65;
    } else if (mode === 'cell') {
      var coverScale = Math.max(wrapW / stageW, wrapH / stageH);
      scale = clamp(coverScale * 0.92, fitScale, fitScale * 1.38);
      if (wrapW > wrapH * 1.15) scale = Math.max(scale, wrapW / stageW);
      if (buttons && buttons.length) {
        focusX = buttons[0].xp / 100;
        focusY = buttons[0].yp / 100;
      }
    } else if (mode === 'town' && buttons && buttons.length) {
      var coverW = wrapW / stageW;
      var coverH = wrapH / stageH;
      var curB = buttons[0];
      var tMinX = curB.xp / 100, tMaxX = tMinX, tMinY = curB.yp / 100, tMaxY = tMinY;
      for (var ti = 1; ti < buttons.length; ti++) {
        var tbx = buttons[ti].xp / 100, tby = buttons[ti].yp / 100;
        if (tbx < tMinX) tMinX = tbx;
        if (tbx > tMaxX) tMaxX = tbx;
        if (tby < tMinY) tMinY = tby;
        if (tby > tMaxY) tMaxY = tby;
      }
      var isWideFrame = (wrapW / Math.max(1, wrapH)) > (stageW / stageH) * 1.05;
      if (isWideFrame) {
        // 手機底部橫框：框齊當前與所有直接相連點，並填滿寬度零黑邊
        var tSpanY = Math.max(0.32, (tMaxY - tMinY) + 0.18);
        var desiredY = wrapH / (stageH * tSpanY);
        scale = clamp(Math.min(desiredY, coverW * 1.25), coverW, coverW * 1.35);
        focusX = ((tMinX + tMaxX) * 0.5) * 0.45 + (curB.xp / 100) * 0.55;
        focusY = ((tMinY + tMaxY) * 0.5) * 0.45 + (curB.yp / 100) * 0.55;
      } else {
        // 桌機右側欄：讓單張 1024×1024 正北城圖優雅滿寬呈現，必要時微跟隨當前節點
        scale = clamp(Math.max(fitScale, coverW * 1.02), fitScale, Math.max(coverW, coverH) * 1.18);
        focusX = ((tMinX + tMaxX) * 0.5) * 0.35 + (curB.xp / 100) * 0.65;
        focusY = ((tMinY + tMaxY) * 0.5) * 0.35 + (curB.yp / 100) * 0.65;
      }
    }

    var w = Math.round(stageW * scale);
    var h = Math.round(stageH * scale);
    var tx = Math.round(wrapW * 0.5 - w * focusX);
    var ty = Math.round(wrapH * 0.5 - h * focusY);
    if (w <= wrapW) tx = Math.round((wrapW - w) * 0.5);
    else tx = clamp(tx, wrapW - w, 0);
    if (h <= wrapH) ty = Math.round((wrapH - h) * 0.5);
    else ty = clamp(ty, wrapH - h, 0);
    return { width: w, height: h, tx: tx, ty: ty };
  }

  // 真實像素空間方位保真防重疊排布：零疊壓、零截斷，且絕不把按鈕推出原本八向方位扇區
  function layoutButtonsInPixels(buttons, frame, wrapW, wrapH, compact) {
    if (!buttons || !buttons.length) return [];
    var padEdgeX = 8;
    var padEdgeY = 8;
    var gapX = compact ? 5 : 7;
    var gapY = compact ? 4 : 6;

    // 留出左上角指北針（46×26）與右上角小按鈕安全邊距
    var visX0 = Math.max(0, -frame.tx) + padEdgeX;
    var visX1 = Math.min(frame.width, wrapW - frame.tx) - padEdgeX;
    var visY0 = Math.max(0, -frame.ty) + padEdgeY;
    var visY1 = Math.min(frame.height, wrapH - frame.ty) - padEdgeY;

    var items = buttons.map(function (b) {
      var sz = estimatePillPx(b, compact);
      var px = clamp((b.xp / 100) * frame.width, visX0 + sz.w * 0.5, Math.max(visX0 + sz.w * 0.5, visX1 - sz.w * 0.5));
      var py = clamp((b.yp / 100) * frame.height, visY0 + sz.h * 0.5, Math.max(visY0 + sz.h * 0.5, visY1 - sz.h * 0.5));
      return {
        b: b,
        px: px,
        py: py,
        origX: px,
        origY: py,
        w: sz.w,
        h: sz.h
      };
    });

    var curItem = items[0];

    for (var iter = 0; iter < 30; iter++) {
      var moved = false;
      for (var i = 0; i < items.length; i++) {
        for (var j = i + 1; j < items.length; j++) {
          var a = items[i], c = items[j];
          var needX = (a.w + c.w) * 0.5 + gapX;
          var needY = (a.h + c.h) * 0.5 + gapY;
          var dx = c.px - a.px;
          var dy = c.py - a.py;
          var ox = needX - Math.abs(dx);
          var oy = needY - Math.abs(dy);
          if (ox > 0 && oy > 0) {
            moved = true;
            var pushAlongY = oy <= ox * 0.88;
            var sgnX = dx === 0 ? (j % 2 ? 1 : -1) : (dx > 0 ? 1 : -1);
            var sgnY = dy === 0 ? (j % 2 ? 1 : -1) : (dy > 0 ? 1 : -1);
            var wA = a.b.isCurrent ? 0.08 : 0.5;
            var wC = c.b.isCurrent ? 0.08 : 0.5;
            if (pushAlongY) {
              var shiftY = oy + 1.2;
              a.py -= sgnY * shiftY * wA;
              c.py += sgnY * shiftY * wC;
              a.px -= sgnX * Math.min(3.5, ox * 0.12) * wA;
              c.px += sgnX * Math.min(3.5, ox * 0.12) * wC;
            } else {
              var shiftX = ox + 1.2;
              a.px -= sgnX * shiftX * wA;
              c.px += sgnX * shiftX * wC;
              a.py -= sgnY * Math.min(3.0, oy * 0.12) * wA;
              c.py += sgnY * Math.min(3.0, oy * 0.12) * wC;
            }
          }
        }
      }
      // 方位半平面護欄：北向出口絕不低於當前點、南向絕不高于當前點、東向絕不偏左、西向絕不偏右
      for (var k = 0; k < items.length; k++) {
        var it = items[k];
        if (!it.b.isCurrent && it.b.dirVec && curItem) {
          var dv = it.b.dirVec;
          if (dv[1] < -0.5 && it.py > curItem.py - 12) it.py = curItem.py - 12;
          if (dv[1] > 0.5 && it.py < curItem.py + 12) it.py = curItem.py + 12;
          if (dv[0] > 0.5 && it.px < curItem.px + 14) it.px = curItem.px + 14;
          if (dv[0] < -0.5 && it.px > curItem.px - 14) it.px = curItem.px - 14;
        }
        var minPx = visX0 + it.w * 0.5;
        var maxPx = Math.max(minPx, visX1 - it.w * 0.5);
        var minPy = visY0 + it.h * 0.5;
        var maxPy = Math.max(minPy, visY1 - it.h * 0.5);
        it.px = clamp(it.px, minPx, maxPx);
        it.py = clamp(it.py, minPy, maxPy);
        // 避開左上角「▲ 北」指北徽記（寬 54px × 高 28px）
        var compRight = visX0 + 54;
        var compBot = visY0 + 28;
        if (it.px - it.w * 0.5 < compRight && it.py - it.h * 0.5 < compBot) {
          it.py = Math.min(maxPy, compBot + it.h * 0.5 + 3);
        }
      }
      if (!moved) break;
    }

    return items.map(function (it) {
      return { b: it.b, px: Math.round(it.px), py: Math.round(it.py) };
    });
  }

  function renderStampsHtml(manifest, activeFarmCells, curCellId) {
    if (!manifest || !manifest.stamps || !Array.isArray(manifest.placements)) return '';
    var cat = manifest.stamps;
    var parts = [];
    var cellCoords = {};

    for (var i = 0; i < manifest.placements.length; i++) {
      var p = manifest.placements[i];
      var def = cat[p.stamp];
      if (!def) continue;
      cellCoords[p.cell_id] = [p.xPct, p.yPct];
      var ay = Math.round((def.anchorY || 0.58) * 100);
      var cls = 'oi-snm-stamp' + (p.cell_id === curCellId ? ' is-active-cell' : '');
      parts.push(
        '<img class="' + cls + '" src="' + esc(def.url) + '" alt="" draggable="false" ' +
        'style="left:' + p.xPct.toFixed(2) + '%;top:' + p.yPct.toFixed(2) + '%;width:' + p.wPct.toFixed(2) + '%;transform:translate(-50%,-' + ay + '%);">'
      );
    }

    // 動態農田／藥圃貼圖（有實際開墾田壟時於該地塊旁長出金黃麥田貼圖！）
    if (Array.isArray(activeFarmCells) && activeFarmCells.length && cat.farm) {
      var farmDef = cat.farm;
      var fay = Math.round((farmDef.anchorY || 0.56) * 100);
      for (var f = 0; f < activeFarmCells.length; f++) {
        var fcid = activeFarmCells[f];
        var xy = cellCoords[fcid];
        if (!xy) continue;
        parts.push(
          '<img class="oi-snm-stamp is-farm" src="' + esc(farmDef.url) + '" alt="" draggable="false" ' +
          'style="left:' + clamp(xy[0] + 1.9, 5, 95).toFixed(2) + '%;top:' + clamp(xy[1] + 1.4, 5, 95).toFixed(2) + '%;width:2.50%;transform:translate(-50%,-' + fay + '%);">'
        );
      }
    }

    return parts.join('');
  }

  function renderNodesHtml(laidOut, isBlocked) {
    return laidOut.map(function (it) {
      var b = it.b;
      var cls = 'oi-snm-node ' + (b.isCurrent ? 'is-current' : 'is-reachable');
      if (b.isSecret) cls += ' is-secret';
      if (b.isLeaveCell) cls += ' is-leave';
      if (b.isSubTrail) cls += ' is-subtrail';
      var posStyle = 'left:' + it.px + 'px;top:' + it.py + 'px;';
      if (b.isCurrent) {
        return '<div class="' + cls + '" style="' + posStyle + '" aria-current="location" title="' + esc(b.title) + '">' +
          '<span class="oi-snm-dot"></span><span class="oi-snm-lbl">' + esc(b.label) + '</span></div>';
      }
      return '<button type="button" class="' + cls + '" style="' + posStyle + '" data-node-hop="' + esc(b.nid) + '"' +
        (isBlocked ? ' disabled' : '') + ' title="' + esc(b.title) + '">' +
        '<span class="oi-snm-lbl">' + esc(b.label) + '</span></button>';
    }).join('');
  }

  function renderGlowsHtml(glows, isNight, stageW, stageH) {
    if (!isNight || !Array.isArray(glows) || !glows.length) return '';
    return glows.map(function (g) {
      var lx = ((g[0] / stageW) * 100).toFixed(2);
      var ly = ((g[1] / stageH) * 100).toFixed(2);
      var rw = ((g[2] * 2 / stageW) * 100).toFixed(2);
      var rh = ((g[2] * 2 / stageH) * 100).toFixed(2);
      return '<div class="oi-snm-glow" style="left:' + lx + '%;top:' + ly + '%;width:' + rw + '%;height:' + rh + '%;"></div>';
    }).join('');
  }

  function ensureStageDom(container) {
    if (container._snmDom && container.contains(container._snmDom.viewport)) {
      return container._snmDom;
    }
    container.innerHTML =
      '<div class="oi-snm-viewport">' +
        '<div class="oi-snm-stage">' +
          '<img class="oi-snm-img" alt="" draggable="false">' +
          '<div class="oi-snm-stamps-layer"></div>' +
          '<div class="oi-snm-night-shade" hidden></div>' +
          '<div class="oi-snm-glows-layer"></div>' +
          '<div class="oi-snm-nodes-layer"></div>' +
        '</div>' +
        '<div class="oi-snm-compass" aria-hidden="true"><span class="oi-snm-compass-arrow">▲</span><span>北</span></div>' +
      '</div>';

    var viewport = container.querySelector('.oi-snm-viewport');
    var stage = container.querySelector('.oi-snm-stage');
    var dom = {
      viewport: viewport,
      stage: stage,
      img: container.querySelector('.oi-snm-img'),
      stampsLayer: container.querySelector('.oi-snm-stamps-layer'),
      nightShade: container.querySelector('.oi-snm-night-shade'),
      glowsLayer: container.querySelector('.oi-snm-glows-layer'),
      nodesLayer: container.querySelector('.oi-snm-nodes-layer'),
      imgUrl: '',
      lastNodeId: '',
      mode: '',
      frame: { width: 320, height: 320, tx: 0, ty: 0 },
      drag: null
    };

    viewport.addEventListener('pointerdown', function (ev) {
      if (ev.target.closest('[data-node-hop]')) return;
      var wrapW = container.clientWidth || 360;
      var wrapH = container.clientHeight || 240;
      if (dom.frame.width <= wrapW + 2 && dom.frame.height <= wrapH + 2) return;
      dom.drag = {
        pid: ev.pointerId,
        sx: ev.clientX,
        sy: ev.clientY,
        tx0: dom.frame.tx,
        ty0: dom.frame.ty,
        moved: false
      };
      stage.classList.add('is-dragging');
      try { viewport.setPointerCapture(ev.pointerId); } catch (_) {}
    });

    viewport.addEventListener('pointermove', function (ev) {
      if (!dom.drag || dom.drag.pid !== ev.pointerId) return;
      var dx = ev.clientX - dom.drag.sx;
      var dy = ev.clientY - dom.drag.sy;
      if (Math.hypot(dx, dy) > 4) dom.drag.moved = true;
      var wrapW = container.clientWidth || 360;
      var wrapH = container.clientHeight || 240;
      var ntx = dom.frame.width <= wrapW ? Math.round((wrapW - dom.frame.width) * 0.5) : clamp(dom.drag.tx0 + dx, wrapW - dom.frame.width, 0);
      var nty = dom.frame.height <= wrapH ? Math.round((wrapH - dom.frame.height) * 0.5) : clamp(dom.drag.ty0 + dy, wrapH - dom.frame.height, 0);
      dom.frame.tx = Math.round(ntx);
      dom.frame.ty = Math.round(nty);
      stage.style.left = dom.frame.tx + 'px';
      stage.style.top = dom.frame.ty + 'px';
    });

    function endDrag(ev) {
      if (!dom.drag || dom.drag.pid !== ev.pointerId) return;
      dom.drag = null;
      stage.classList.remove('is-dragging');
    }
    viewport.addEventListener('pointerup', endDrag);
    viewport.addEventListener('pointercancel', endDrag);

    container._snmDom = dom;
    if (typeof ResizeObserver !== 'undefined' && !container._snmResizeObs) {
      var ro = new ResizeObserver(function () {
        var rw = container.clientWidth || 0;
        var rh = container.clientHeight || 0;
        if (rw < 40 || rh < 40) return;
        if (dom.lastWrapW === rw && dom.lastWrapH === rh) return;
        if (dom.lastCall && dom.lastCall.mode === 'local') {
          renderLocal(container, dom.lastCall.view, dom.lastCall.opts);
        } else if (dom.lastCall && dom.lastCall.mode === 'overview') {
          renderIslandOverview(container, dom.lastCall.view, dom.lastCall.mapData, dom.lastCall.opts);
        }
      });
      ro.observe(container);
      container._snmResizeObs = ro;
    }
    return dom;
  }

  function renderLocal(container, view, opts) {
    if (!container || !view || !view.scene) return { layerBadge: '', mappedExitIds: {} };
    var scene = view.scene;
    var sm = scene.static_map || {};
    var isNight = !!(opts && opts.isNight);
    var isBlocked = !!(view.skirmish || view.transit);
    var townMap = sm.town_slug && window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[sm.town_slug];

    var imgUrl = '';
    var stageW = 1024;
    var stageH = 1024;
    var stageCls = 'oi-snm-stage full-bleed';
    var bgCol = isNight ? '#152229' : '#6aa6ba';
    var glows = [];
    var built = { buttons: [], mappedExitIds: {} };
    var layerBadge = '';
    var frameMode = 'island_local';
    var showStamps = false;

    if (scene.dungeon && sm.dungeon_room && sm.dungeon_room.room_url) {
      imgUrl = sm.dungeon_room.room_url;
      stageW = 1024;
      stageH = 1024;
      bgCol = '#12100e';
      built = buildDungeonButtons(scene);
      layerBadge = '地底石室';
      frameMode = 'cell';
    } else if (townMap) {
      var district = resolveTownDistrict(townMap, scene.node_id);
      var spec = district || townMap;
      imgUrl = spec.imgUrl;
      stageW = spec.width || 900;
      stageH = spec.height || 1200;
      stageCls = spec.cropMode === 'wg_legacy' ? 'oi-snm-stage wg-legacy' : 'oi-snm-stage full-bleed';
      bgCol = isNight ? (spec.bgNight || '#14191e') : (spec.bgDay || '#dfd5be');
      glows = spec.glows || [];
      built = buildTownButtons(scene, spec);
      layerBadge = townMap.title ? townMap.title.split('・')[0] : (sm.cell_title || '城鎮');
      frameMode = 'town';
    } else {
      // 全島野外／鄉村／驛道統一走純自然水彩底圖 ＋ 模組化透明貼圖（正北朝上、真座標平滑移動、零旋轉斷層）
      imgUrl = sm.island_master_url || ISLAND_TERRAIN_URL;
      stageW = 1024;
      stageH = 1024;
      bgCol = isNight ? '#15242d' : '#65a2b6';
      built = buildIslandLocalButtons(scene, sm);
      layerBadge = formatRegionShort(sm.cell_title, scene.name) || '銀鷗外島';
      frameMode = 'island_local';
      showStamps = true;
    }

    var curWrapW = container.clientWidth || 0;
    var curWrapH = container.clientHeight || 0;
    var wrapW = curWrapW || 360;
    var wrapH = curWrapH || 240;
    var compact = wrapW < 440 || wrapH < 320;
    var frame = computeStageFrame(wrapW, wrapH, stageW, stageH, built.buttons, frameMode);
    var laidOut = layoutButtonsInPixels(built.buttons, frame, wrapW, wrapH, compact);

    var dom = ensureStageDom(container);
    var sizeChanged = !dom.lastWrapW || !dom.lastWrapH || dom.lastWrapW !== curWrapW || dom.lastWrapH !== curWrapH;
    dom.lastWrapW = curWrapW;
    dom.lastWrapH = curWrapH;
    dom.lastCall = { mode: 'local', view: view, opts: opts };
    var imgChanged = dom.imgUrl !== imgUrl || dom.mode !== 'local';
    var snapAnim = imgChanged || sizeChanged;

    dom.viewport.className = 'oi-snm-viewport' + (isNight ? ' is-night' : '') + (frameMode === 'cell' ? ' is-cell' : '');
    dom.viewport.style.background = bgCol;
    if (imgChanged) {
      dom.stage.classList.add('no-anim');
      dom.img.src = imgUrl;
      dom.img.alt = layerBadge;
      dom.imgUrl = imgUrl;
      dom.mode = 'local';
    }
    dom.stage.className = stageCls + (snapAnim ? ' no-anim' : '');
    dom.stage.style.width = frame.width + 'px';
    dom.stage.style.height = frame.height + 'px';
    dom.stage.style.left = frame.tx + 'px';
    dom.stage.style.top = frame.ty + 'px';
    dom.frame = { width: frame.width, height: frame.height, tx: frame.tx, ty: frame.ty };
    dom.lastNodeId = scene.node_id;

    if (snapAnim) {
      requestAnimationFrame(function () {
        dom.stage.classList.remove('no-anim');
      });
    }

    if (showStamps) {
      var mf = ensureStampsManifest(function () {
        if (dom.lastCall && dom.lastCall.mode === 'local') {
          renderLocal(container, dom.lastCall.view, dom.lastCall.opts);
        }
      });
      var stHtml = renderStampsHtml(mf, sm.active_farm_cells, scene.cell_id);
      if (dom.stampsLayer.innerHTML !== stHtml) dom.stampsLayer.innerHTML = stHtml;
    } else if (dom.stampsLayer.innerHTML !== '') {
      dom.stampsLayer.innerHTML = '';
    }

    dom.nightShade.hidden = !isNight;
    dom.nightShade.className = 'oi-snm-night-shade';

    var glowsHtml = renderGlowsHtml(glows, isNight, stageW, stageH);
    if (dom.glowsLayer.innerHTML !== glowsHtml) dom.glowsLayer.innerHTML = glowsHtml;

    var nodesHtml = renderNodesHtml(laidOut, isBlocked);
    if (dom.nodesLayer.innerHTML !== nodesHtml) dom.nodesLayer.innerHTML = nodesHtml;

    return {
      layerBadge: layerBadge,
      buttonCount: laidOut.length,
      mappedExitIds: built.mappedExitIds
    };
  }

  function renderIslandOverview(container, view, mapData, opts) {
    if (!container) return;
    var scene = (view && view.scene) || {};
    var sm = scene.static_map || {};
    var isNight = !!(opts && opts.isNight);
    var imgUrl = (mapData && mapData.island_master_url) || sm.island_master_url || ISLAND_TERRAIN_URL;

    var curPos = (Array.isArray(scene.pos) && scene.pos.length >= 2) ? scene.pos : (sm.cell_center || [1472.82, 700.58]);
    var curPt = projectIslandPercent(curPos);

    var visitedMap = {};
    if (mapData && Array.isArray(mapData.visited_cells)) {
      for (var v = 0; v < mapData.visited_cells.length; v++) {
        visitedMap[mapData.visited_cells[v].cell_id] = true;
      }
    }
    if (scene.cell_id) visitedMap[scene.cell_id] = true;

    var townPinsHtml = MAJOR_TOWNS_OVERVIEW.map(function (t) {
      var pt = projectIslandPercent(t.pos);
      var isHere = scene.cell_id === t.cellId;
      var cls = 'oi-snm-ov-town' + (isHere ? ' is-here' : (visitedMap[t.cellId] ? ' is-visited' : ''));
      // 標籤微下移至貼圖腳下，不遮擋手繪城鎮小圖
      return '<div class="' + cls + '" style="left:' + pt[0].toFixed(2) + '%;top:' + clamp(pt[1] + 3.2, 6, 95).toFixed(2) + '%;">' +
        esc(t.name) + '</div>';
    }).join('');

    var curPinHtml =
      '<div class="oi-snm-ov-me" style="left:' + curPt[0].toFixed(2) + '%;top:' + curPt[1].toFixed(2) + '%;" title="' + esc(scene.name || '目前所在') + '">' +
        '<span class="oi-snm-dot"></span>' +
        '<span class="oi-snm-lbl">' + esc(formatRegionShort(sm.cell_title, scene.name) || '在此') + '</span>' +
      '</div>';

    var wrapW = container.clientWidth || 360;
    var wrapH = container.clientHeight || 360;
    var frame = computeStageFrame(wrapW, wrapH, 1024, 1024, null, 'overview');

    var dom = ensureStageDom(container);
    dom.lastCall = { mode: 'overview', view: view, mapData: mapData, opts: opts };
    var imgChanged = dom.imgUrl !== imgUrl || dom.mode !== 'overview';
    dom.viewport.className = 'oi-snm-viewport is-overview' + (isNight ? ' is-night' : '');
    dom.viewport.style.background = isNight ? '#15242d' : '#65a2b6';
    if (imgChanged) {
      dom.img.src = imgUrl;
      dom.img.alt = '銀鷗外島全島手繪輿圖';
      dom.imgUrl = imgUrl;
      dom.mode = 'overview';
    }
    dom.stage.className = 'oi-snm-stage full-bleed no-anim';
    dom.stage.style.width = frame.width + 'px';
    dom.stage.style.height = frame.height + 'px';
    dom.stage.style.left = frame.tx + 'px';
    dom.stage.style.top = frame.ty + 'px';
    dom.frame = { width: frame.width, height: frame.height, tx: frame.tx, ty: frame.ty };

    var mf = ensureStampsManifest(function () {
      if (dom.lastCall && dom.lastCall.mode === 'overview') {
        renderIslandOverview(container, dom.lastCall.view, dom.lastCall.mapData, dom.lastCall.opts);
      }
    });
    var stHtml = renderStampsHtml(mf, sm.active_farm_cells, scene.cell_id);
    if (dom.stampsLayer.innerHTML !== stHtml) dom.stampsLayer.innerHTML = stHtml;

    dom.nightShade.hidden = !isNight;
    dom.nightShade.className = 'oi-snm-night-shade is-soft';
    if (dom.glowsLayer.innerHTML !== '') dom.glowsLayer.innerHTML = '';

    var ovHtml = townPinsHtml + curPinHtml;
    if (dom.nodesLayer.innerHTML !== ovHtml) dom.nodesLayer.innerHTML = ovHtml;
  }

  // 頁面載入即預取貼圖配置，首次進野外零閃爍
  ensureStampsManifest();

  window.StaticNodeMap = {
    renderLocal: renderLocal,
    renderIslandOverview: renderIslandOverview
  };
})();

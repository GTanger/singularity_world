/* 《奇點世界・銀鷗外島》三層靜態手繪水彩圖點點點渲染器
 * - 無連線、無格線、無室內走圖、點空白處不移動
 * - 當前所在按鈕發亮、與當前相連的可走出口淡化／變暗、其餘無相連點完全隱藏
 * - 第 1 層：5 大核心城鎮手繪詳圖（TOWN_ILLUSTRATED_MAPS）
 * - 第 2 層：88 張鄉野／村莊／山林單張地塊手繪圖（/maps/cells/cell_XXXX.webp）＋ 試煉坑室圖
 * - 第 3 層：全島總覽手繪大輿圖（/maps/seam_lab/master_island_2k.webp）
 */
(function () {
  'use strict';

  var ISLAND_MASTER_FALLBACK = '/maps/seam_lab/master_island_2k.webp?v=20261127r1';
  var ISLAND_BOUNDS = { minX: 1357.71, maxX: 1507.06, minY: 627.98, maxY: 793.21 };
  var MAJOR_TOWNS_OVERVIEW = [
    { slug: 'white_gull_harbor', cellId: 4616, name: '白鷗港', pos: [1472.82, 700.58] },
    { slug: 'gull_tail_port', cellId: 4613, name: '鷗尾漁港', pos: [1436.8, 711.0] },
    { slug: 'west_strait_ferry', cellId: 4321, name: '西峽渡口', pos: [1375.92, 662.82] },
    { slug: 'greypine_post', cellId: 4525, name: '灰松林驛', pos: [1416.94, 687.79] },
    { slug: 'north_cape_beacon', cellId: 4073, name: '北岬燈塔', pos: [1425.0, 636.7] }
  ];

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
    return parts.length > 1 ? parts[parts.length - 1] : s;
  }

  function cleanDirWord(dir) {
    return String(dir || '').replace(/（[^）]*）/g, '').trim();
  }

  function dirToVector(dir) {
    var d = cleanDirWord(dir);
    if (d.indexOf('東北') >= 0) return [0.72, -0.72];
    if (d.indexOf('西北') >= 0) return [-0.72, -0.72];
    if (d.indexOf('東南') >= 0) return [0.72, 0.72];
    if (d.indexOf('西南') >= 0) return [-0.72, 0.72];
    if (d.indexOf('北') >= 0) return [0, -1];
    if (d.indexOf('南') >= 0) return [0, 1];
    if (d.indexOf('東') >= 0) return [1, 0];
    if (d.indexOf('西') >= 0) return [-1, 0];
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
    return [clamp(px, 6, 94), clamp(py, 6, 94)];
  }

  function lookupTownAnchor(townSpec, nid) {
    if (!townSpec || !nid) return null;
    var i;
    var pills = townSpec.pills || [];
    for (i = 0; i < pills.length; i++) {
      if (pills[i].nid === nid) {
        return { x: pills[i].x, y: pills[i].y, label: pills[i].label, kind: pills[i].kind || 'main', exact: true };
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
    // 客棧二樓客房歸屬酒館位置（不切室內圖，直接在酒館按鈕上發亮）
    if (nid.indexOf(':inn_room_') >= 0 || nid.indexOf(':inn_upstairs') >= 0) {
      var prefix = nid.split(':').slice(0, 2).join(':');
      var tav = lookupTownAnchor(townSpec, prefix + ':rusty_anchor_tavern');
      if (tav) {
        return { x: tav.x, y: tav.y, label: tav.label, kind: 'inn_sub', exact: false };
      }
    }
    return null;
  }

  function resolveCollisions(nodes, minDxPct, minDyPct, minPadX, maxPadX, minPadY, maxPadY) {
    var iter, i, j;
    for (iter = 0; iter < 10; iter++) {
      for (i = 0; i < nodes.length; i++) {
        for (j = i + 1; j < nodes.length; j++) {
          var a = nodes[i], b = nodes[j];
          var dx = b.xp - a.xp;
          var dy = b.yp - a.yp;
          if (Math.abs(dx) < minDxPct && Math.abs(dy) < minDyPct) {
            var pushY = (dy >= 0 ? 1 : -1) * ((minDyPct - Math.abs(dy)) * 0.52 + 0.6);
            var pushX = (dx === 0 && dy === 0 ? 1 : (dx >= 0 ? 1 : -1)) * ((minDxPct - Math.abs(dx)) * 0.22);
            if (a.isCurrent) {
              b.yp = clamp(b.yp + pushY * 1.7, minPadY, maxPadY);
              b.xp = clamp(b.xp + pushX * 1.4, minPadX, maxPadX);
            } else if (b.isCurrent) {
              a.yp = clamp(a.yp - pushY * 1.7, minPadY, maxPadY);
              a.xp = clamp(a.xp - pushX * 1.4, minPadX, maxPadX);
            } else {
              a.yp = clamp(a.yp - pushY, minPadY, maxPadY);
              b.yp = clamp(b.yp + pushY, minPadY, maxPadY);
              a.xp = clamp(a.xp - pushX, minPadX, maxPadX);
              b.xp = clamp(b.xp + pushX, minPadX, maxPadX);
            }
          }
        }
      }
    }
  }

  function buildTownButtons(scene, townSpec) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var curAnchor = lookupTownAnchor(townSpec, curNid) || { x: 450, y: 600, label: shortPlaceName(scene.name), kind: 'main', exact: false };
    var buttons = [];

    var curLabel = curAnchor.exact
      ? curAnchor.label
      : (shortPlaceName(scene.name) || curAnchor.label);
    buttons.push({
      nid: curNid,
      label: curLabel,
      xp: clamp((curAnchor.x / 900) * 100, 10, 90),
      yp: clamp((curAnchor.y / 1200) * 100, 7, 93),
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
      // 巷內多戶民宅／客棧個別客房收於多文字區點選，避免 10 顆民宅鈕疊壓地圖
      if (ex.to_kind === 'house' || ex.to_kind === 'house_vacant') {
        continue;
      }
      if (ex.to.indexOf(':inn_room_') >= 0 && curNid.indexOf(':inn_upstairs') >= 0) {
        continue;
      }

      var anc = lookupTownAnchor(townSpec, ex.to);
      var xp, yp, label;
      var shortDest = shortPlaceName(ex.to_name);
      var dWord = cleanDirWord(ex.dir);

      if (anc && anc.exact) {
        // 若剛好跟當前位置重合（例如酒館一樓 <-> 二樓），微微錯開以免蓋住發亮主按鈕
        xp = (anc.x / 900) * 100;
        yp = (anc.y / 1200) * 100;
        if (Math.hypot(xp - buttons[0].xp, yp - buttons[0].yp) < 4.5) {
          var vSub = dirToVector(ex.dir) || [0, -1];
          xp = clamp(buttons[0].xp + vSub[0] * 14, 12, 88);
          yp = clamp(buttons[0].yp + vSub[1] * 8.5, 8, 92);
        }
        label = anc.label || shortDest;
      } else if (ex.to_cell_id && ex.to_cell_id !== curCellId) {
        // 城門外／跨地塊出口：依方位擺在城門外側
        var vOut = dirToVector(ex.dir);
        if (!vOut && Array.isArray(scene.pos) && Array.isArray(ex.to_pos)) {
          var dxW = ex.to_pos[0] - scene.pos[0];
          var dyW = ex.to_pos[1] - scene.pos[1];
          var lnW = Math.hypot(dxW, dyW) || 1;
          vOut = [dxW / lnW, dyW / lnW];
        }
        vOut = vOut || [0, -1];
        xp = clamp(buttons[0].xp + vOut[0] * 18, 12, 88);
        yp = clamp(buttons[0].yp + vOut[1] * 10.5, 6.5, 93.5);
        label = (dWord ? (dWord + ' · ') : '') + shortDest;
      } else {
        // 其他同城相連出口（如回酒館一樓／回巷口）
        var vLoc = dirToVector(ex.dir) || [0, 1];
        xp = clamp(buttons[0].xp + vLoc[0] * 16, 12, 88);
        yp = clamp(buttons[0].yp + vLoc[1] * 9.5, 8, 92);
        label = (dWord && dWord !== shortDest ? (dWord + ' · ') : '') + shortDest;
      }

      usedNids[ex.to] = true;
      buttons.push({
        nid: ex.to,
        label: label,
        xp: clamp(xp, 10, 90),
        yp: clamp(yp, 6.5, 93.5),
        isCurrent: false,
        isSecret: ex.kind === 'secret',
        isLeaveCell: !!(ex.to_cell_id && ex.to_cell_id !== curCellId),
        kind: ex.to_kind || ex.kind || 'exit',
        title: ex.ambient_cue || ex.to_name || label
      });
    }

    resolveCollisions(buttons, 19, 7.2, 10, 90, 6.5, 93.5);
    return buttons;
  }

  function buildCellButtons(scene) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var curLabel = shortPlaceName(scene.name) || '此處';
    var buttons = [{
      nid: curNid,
      label: curLabel,
      xp: 50,
      yp: 50,
      isCurrent: true,
      kind: scene.kind || 'wild',
      title: scene.name || curLabel
    }];

    var exits = Array.isArray(scene.exits) ? scene.exits : [];
    var used = {};
    used[curNid] = true;
    var fallbackIdx = 0;

    for (var i = 0; i < exits.length; i++) {
      var ex = exits[i];
      if (!ex || !ex.to || used[ex.to]) continue;
      if (ex.to_kind === 'house' || ex.to_kind === 'house_vacant') {
        continue;
      }
      used[ex.to] = true;

      var vec = dirToVector(ex.dir);
      if (!vec && Array.isArray(scene.pos) && Array.isArray(ex.to_pos)) {
        var dx = ex.to_pos[0] - scene.pos[0];
        var dy = ex.to_pos[1] - scene.pos[1];
        var ln = Math.hypot(dx, dy);
        if (ln > 0.001) vec = [dx / ln, dy / ln];
      }
      if (!vec) {
        var ang = (fallbackIdx++ / Math.max(1, exits.length)) * Math.PI * 2 - Math.PI * 0.5;
        vec = [Math.cos(ang), Math.sin(ang)];
      }

      var isCross = !!(ex.to_cell_id && ex.to_cell_id !== curCellId);
      var dist = isCross ? 34 : 25;
      var xp = clamp(50 + vec[0] * dist, 15, 85);
      var yp = clamp(50 + vec[1] * dist, 13, 87);
      var shortDest = shortPlaceName(ex.to_name);
      var dWord = cleanDirWord(ex.dir);
      var label = (dWord && dWord !== shortDest) ? (dWord + ' · ' + shortDest) : shortDest;

      buttons.push({
        nid: ex.to,
        label: label,
        xp: xp,
        yp: yp,
        isCurrent: false,
        isSecret: ex.kind === 'secret',
        isLeaveCell: isCross,
        kind: ex.to_kind || ex.kind || 'exit',
        title: ex.ambient_cue || ex.to_name || label
      });
    }

    resolveCollisions(buttons, 24, 12.5, 14, 86, 12, 88);
    return buttons;
  }

  function computeStageFrame(wrapW, wrapH, stageW, stageH, buttons, isOverview) {
    if (!wrapW || !wrapH) return { width: 320, height: 320, tx: 0, ty: 0 };
    var fitScale = Math.min(wrapW / stageW, wrapH / stageH);
    if (isOverview || !buttons || !buttons.length) {
      var ow = Math.round(stageW * fitScale);
      var oh = Math.round(stageH * fitScale);
      return { width: ow, height: oh, tx: Math.round((wrapW - ow) * 0.5), ty: Math.round((wrapH - oh) * 0.5) };
    }

    // 行動端矮寬窗格（例如 390×250）遇到直長城鎮圖（3:4）時，自動框住「當前發亮點＋所有相連按鈕」以利拇指點按
    var aspectDiff = (wrapW / Math.max(1, wrapH)) / (stageW / stageH);
    var scale = fitScale;
    var focusX = 0.5;
    var focusY = 0.5;

    if (aspectDiff > 1.28) {
      var minX = 1, maxX = 0, minY = 1, maxY = 0;
      for (var i = 0; i < buttons.length; i++) {
        var bx = buttons[i].xp / 100;
        var by = buttons[i].yp / 100;
        if (bx < minX) minX = bx;
        if (bx > maxX) maxX = bx;
        if (by < minY) minY = by;
        if (by > maxY) maxY = by;
      }
      var padX = 0.17;
      var padY = 0.16;
      var boxW = Math.max(0.42, (maxX - minX) + padX * 2);
      var boxH = Math.max(0.36, (maxY - minY) + padY * 2);
      var focusScale = Math.min(wrapW / (stageW * boxW), wrapH / (stageH * boxH));
      var coverWScale = wrapW / stageW;
      scale = clamp(Math.min(focusScale, coverWScale * 1.18), fitScale, fitScale * 1.95);
      focusX = (minX + maxX) * 0.5;
      focusY = (minY + maxY) * 0.5;
    }

    var w = Math.round(stageW * scale);
    var h = Math.round(stageH * scale);
    var tx = Math.round(wrapW * 0.5 - w * focusX);
    var ty = Math.round(wrapH * 0.5 - h * focusY);
    if (w <= wrapW) {
      tx = Math.round((wrapW - w) * 0.5);
    } else {
      tx = clamp(tx, wrapW - w, 0);
    }
    if (h <= wrapH) {
      ty = Math.round((wrapH - h) * 0.5);
    } else {
      ty = clamp(ty, wrapH - h, 0);
    }
    return { width: w, height: h, tx: tx, ty: ty };
  }

  function renderButtonsHtml(buttons, isBlocked) {
    return buttons.map(function (b) {
      var cls = 'oi-snm-node ' + (b.isCurrent ? 'is-current' : 'is-reachable');
      if (b.isSecret) cls += ' is-secret';
      if (b.isLeaveCell) cls += ' is-leave';
      if (b.isCurrent) {
        return '<div class="' + cls + '" style="left:' + b.xp.toFixed(2) + '%;top:' + b.yp.toFixed(2) + '%;" aria-current="location" title="' + esc(b.title) + '">' +
          '<span class="oi-snm-dot"></span><span class="oi-snm-lbl">' + esc(b.label) + '</span></div>';
      }
      return '<button type="button" class="' + cls + '" style="left:' + b.xp.toFixed(2) + '%;top:' + b.yp.toFixed(2) + '%;" data-node-hop="' + esc(b.nid) + '"' +
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

  function renderLocal(container, view, opts) {
    if (!container || !view || !view.scene) return;
    var scene = view.scene;
    var sm = scene.static_map || {};
    var isNight = !!(opts && opts.isNight);
    var isBlocked = !!(view.skirmish || view.transit);
    var townMap = sm.town_slug && window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[sm.town_slug];

    var imgUrl = '';
    var stageW = 1024;
    var stageH = 1024;
    var stageCls = 'oi-snm-stage full-bleed';
    var bgCol = isNight ? '#141815' : '#dfd5be';
    var glows = [];
    var buttons = [];
    var layerBadge = '';

    if (scene.dungeon && sm.dungeon_room && sm.dungeon_room.room_url) {
      imgUrl = sm.dungeon_room.room_url;
      stageW = 1024;
      stageH = 1024;
      bgCol = '#12100e';
      buttons = buildCellButtons(scene);
      layerBadge = '地底石室';
    } else if (townMap) {
      imgUrl = townMap.imgUrl;
      stageW = 900;
      stageH = 1200;
      stageCls = townMap.cropMode === 'wg_legacy' ? 'oi-snm-stage wg-legacy' : 'oi-snm-stage full-bleed';
      bgCol = isNight ? (townMap.bgNight || '#14191e') : (townMap.bgDay || '#dfd5be');
      glows = townMap.glows || [];
      buttons = buildTownButtons(scene, townMap);
      layerBadge = townMap.title ? townMap.title.split('・')[0].replace(/手繪水彩海港輿圖$/, '') : (sm.cell_title || '城鎮');
    } else if (sm.cell_map && sm.cell_map.url) {
      imgUrl = sm.cell_map.url;
      stageW = 1024;
      stageH = 1024;
      bgCol = isNight ? (sm.cell_map.bg_night || '#141815') : (sm.cell_map.bg_day || '#d8cfba');
      glows = sm.cell_map.glows || [];
      buttons = buildCellButtons(scene);
      layerBadge = sm.cell_title || shortPlaceName(scene.name);
    } else {
      imgUrl = sm.island_master_url || ISLAND_MASTER_FALLBACK;
      stageW = 1024;
      stageH = 1024;
      bgCol = isNight ? '#10181f' : '#35505e';
      buttons = buildCellButtons(scene);
      layerBadge = sm.cell_title || '銀鷗外島';
    }

    var wrapW = container.clientWidth || 360;
    var wrapH = container.clientHeight || 260;
    var frame = computeStageFrame(wrapW, wrapH, stageW, stageH, buttons, false);

    var html =
      '<div class="oi-snm-viewport' + (isNight ? ' is-night' : '') + '" style="background:' + esc(bgCol) + ';">' +
        '<div class="' + stageCls + '" style="width:' + frame.width + 'px;height:' + frame.height + 'px;left:' + frame.tx + 'px;top:' + frame.ty + 'px;">' +
          '<img class="oi-snm-img" src="' + esc(imgUrl) + '" alt="' + esc(layerBadge) + '" draggable="false">' +
          (isNight ? '<div class="oi-snm-night-shade"></div>' : '') +
          renderGlowsHtml(glows, isNight, stageW, stageH) +
          renderButtonsHtml(buttons, isBlocked) +
        '</div>' +
      '</div>';

    if (container.innerHTML !== html) {
      container.innerHTML = html;
    }
    return { layerBadge: layerBadge, buttonCount: buttons.length };
  }

  function renderIslandOverview(container, view, mapData, opts) {
    if (!container) return;
    var scene = (view && view.scene) || {};
    var sm = scene.static_map || {};
    var isNight = !!(opts && opts.isNight);
    var imgUrl = (mapData && mapData.island_master_url) || sm.island_master_url || ISLAND_MASTER_FALLBACK;

    var curPos = sm.cell_center || scene.pos || [1472.82, 700.58];
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
      return '<div class="' + cls + '" style="left:' + pt[0].toFixed(2) + '%;top:' + pt[1].toFixed(2) + '%;">' +
        esc(t.name) + '</div>';
    }).join('');

    var curPinHtml =
      '<div class="oi-snm-ov-me" style="left:' + curPt[0].toFixed(2) + '%;top:' + curPt[1].toFixed(2) + '%;" title="' + esc(scene.name || '目前所在') + '">' +
        '<span class="oi-snm-dot"></span>' +
        '<span class="oi-snm-lbl">' + esc(sm.cell_title || shortPlaceName(scene.name) || '在此') + '</span>' +
      '</div>';

    var wrapW = container.clientWidth || 360;
    var wrapH = container.clientHeight || 360;
    var frame = computeStageFrame(wrapW, wrapH, 1024, 1024, null, true);

    var html =
      '<div class="oi-snm-viewport is-overview' + (isNight ? ' is-night' : '') + '" style="background:#263d4a;">' +
        '<div class="oi-snm-stage full-bleed" style="width:' + frame.width + 'px;height:' + frame.height + 'px;left:' + frame.tx + 'px;top:' + frame.ty + 'px;">' +
          '<img class="oi-snm-img" src="' + esc(imgUrl) + '" alt="銀鷗外島全島手繪總覽" draggable="false">' +
          (isNight ? '<div class="oi-snm-night-shade is-soft"></div>' : '') +
          townPinsHtml +
          curPinHtml +
        '</div>' +
      '</div>';

    if (container.innerHTML !== html) {
      container.innerHTML = html;
    }
  }

  window.StaticNodeMap = {
    renderLocal: renderLocal,
    renderIslandOverview: renderIslandOverview
  };
})();

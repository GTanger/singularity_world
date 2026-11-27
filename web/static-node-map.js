/* 《奇點世界・銀鷗外島》三層靜態手繪水彩圖點點點渲染器（v20261127r2）
 * - 持久化 DOM（同張底圖不重建 <img>，支援 0.28s 平滑鏡頭跟隨＋拖曳偷看地圖）
 * - 無連線、無格線、無室內走圖、點空白處完全不移動
 * - 真實像素空間防重疊排布（8 出口大路口零疊壓、零截斷）
 * - 當前所在按鈕發亮、與當前相連的可走出口淡化／變暗、其餘無相連點完全隱藏
 */
(function () {
  'use strict';

  var ISLAND_MASTER_FALLBACK = '/maps/seam_lab/master_island_2k.webp?v=20261127r2';
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
    if (nid.indexOf(':inn_room_') >= 0 || nid.indexOf(':inn_upstairs') >= 0) {
      var prefix = nid.split(':').slice(0, 2).join(':');
      var tav = lookupTownAnchor(townSpec, prefix + ':rusty_anchor_tavern');
      if (tav) {
        return { x: tav.x, y: tav.y, label: tav.label, kind: 'inn_sub', exact: false };
      }
    }
    if (nid.indexOf(':tally_house') >= 0) {
      var p2 = nid.split(':').slice(0, 2).join(':');
      var ts = lookupTownAnchor(townSpec, p2 + ':tavern_st');
      if (ts) {
        return { x: ts.x, y: ts.y, label: '港務記籌寮屋', kind: 'house_sub', exact: false };
      }
    }
    return null;
  }

  function buildTownButtons(scene, townSpec) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var curAnchor = lookupTownAnchor(townSpec, curNid) || { x: 450, y: 600, label: shortPlaceName(scene.name), kind: 'main', exact: false };
    var buttons = [];
    var mappedExitIds = {};

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
      // 巷內民宅／記籌寮屋／客棧分房收斂於多文字區「此處門戶」
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
        xp = (anc.x / 900) * 100;
        yp = (anc.y / 1200) * 100;
        if (Math.hypot(xp - buttons[0].xp, yp - buttons[0].yp) < 4.5) {
          var vSub = dirToVector(ex.dir) || [0, -1];
          xp = clamp(buttons[0].xp + vSub[0] * 14, 12, 88);
          yp = clamp(buttons[0].yp + vSub[1] * 8.5, 8, 92);
        }
        label = anc.label || shortDest;
      } else if (ex.to_cell_id && ex.to_cell_id !== curCellId) {
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
        var vLoc = dirToVector(ex.dir) || [0, 1];
        xp = clamp(buttons[0].xp + vLoc[0] * 16, 12, 88);
        yp = clamp(buttons[0].yp + vLoc[1] * 9.5, 8, 92);
        label = (dWord && dWord !== shortDest ? (dWord + ' · ') : '') + shortDest;
      }

      usedNids[ex.to] = true;
      mappedExitIds[ex.to] = true;
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

    return { buttons: buttons, mappedExitIds: mappedExitIds };
  }

  function buildCellButtons(scene) {
    var curNid = scene.node_id;
    var curCellId = scene.cell_id;
    var curLabel = shortPlaceName(scene.name) || '此處';
    var mappedExitIds = {};
    // 當前點略微下偏至 55%，避免正中央壓死地塊手繪主角（如斷橋／坑口／營地）
    var buttons = [{
      nid: curNid,
      label: curLabel,
      xp: 50,
      yp: 55,
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
      mappedExitIds[ex.to] = true;

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
      var xp = clamp(50 + vec[0] * dist, 16, 84);
      var yp = clamp(52 + vec[1] * dist, 14, 86);
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

    return { buttons: buttons, mappedExitIds: mappedExitIds };
  }

  function estimatePillPx(b, compact) {
    var fontPx = compact ? 11 : 12;
    var txt = String(b.label || '');
    var units = 0;
    for (var i = 0; i < txt.length; i++) {
      units += txt.charCodeAt(i) > 127 ? 1.04 : 0.62;
    }
    var padX = (b.isCurrent ? 28 : 18);
    var w = Math.ceil(units * fontPx + padX);
    var h = b.isCurrent ? (compact ? 26 : 30) : (compact ? 25 : 28);
    return { w: Math.min(w, compact ? 138 : 168), h: h };
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

    if (mode === 'cell') {
      // 方形地塊圖在直長桌機欄或矮寬手機欄適度放大補滿，消除粗死黑邊
      var coverScale = Math.max(wrapW / stageW, wrapH / stageH);
      scale = clamp(coverScale * 0.92, fitScale, fitScale * 1.38);
      if (wrapW > wrapH * 1.15) {
        scale = Math.max(scale, wrapW / stageW);
      }
      if (buttons && buttons.length) {
        focusX = buttons[0].xp / 100;
        focusY = buttons[0].yp / 100;
      }
    } else if (mode === 'town' && buttons && buttons.length) {
      var coverW = wrapW / stageW;
      // 行動端絕不縮得比視窗窄（徹底消除左右黑邊與按鈕擠爆！）
      var isWideFrame = (wrapW / Math.max(1, wrapH)) > (stageW / stageH) * 1.05;
      var curB = buttons[0];
      var minX = curB.xp / 100, maxX = minX, minY = curB.yp / 100, maxY = minY;
      for (var i = 1; i < buttons.length; i++) {
        var bx = buttons[i].xp / 100;
        var by = buttons[i].yp / 100;
        if (bx < minX) minX = bx;
        if (bx > maxX) maxX = bx;
        if (by < minY) minY = by;
        if (by > maxY) maxY = by;
      }
      if (isWideFrame) {
        // 寬度永遠填滿視窗，依出口分布微調縮放（8 出口時穩守 coverW 滿寬，2～3 出口時微微放大聚焦）
        var spanY = Math.max(0.24, maxY - minY + 0.14);
        var desiredYScale = wrapH / (stageH * spanY);
        var targetScale = buttons.length >= 6 ? coverW : Math.min(desiredYScale, coverW * 1.14);
        scale = clamp(targetScale, coverW, coverW * 1.16);
        focusX = (minX + maxX) * 0.5;
        // 稍微偏重當前發亮點，同時把全體出口包進可視窗格內（權重總和嚴守 1.00！）
        focusY = ((minY + maxY) * 0.5) * 0.55 + (curB.yp / 100) * 0.45;
      } else {
        scale = fitScale;
        focusX = 0.5;
        focusY = 0.5;
      }
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

  // 真實像素空間碰撞解算：保證多出口大路口在手機與桌機皆 0 重疊、0 裁出視口
  function layoutButtonsInPixels(buttons, frame, wrapW, wrapH, compact) {
    if (!buttons || !buttons.length) return [];
    var padEdgeX = 6;
    var padEdgeY = 6;
    var gapX = compact ? 5 : 7;
    var gapY = compact ? 4 : 6;

    // 算出相對於 stage 的可見視窗矩形，確保所有按鈕 100% 落在看得到且點得到的區域內
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

    for (var iter = 0; iter < 28; iter++) {
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
            // 優先沿重疊較小的一軸推開（保持原本方位感！）
            var pushAlongY = oy <= ox * 0.85;
            var sgnX = dx === 0 ? (j % 2 ? 1 : -1) : (dx > 0 ? 1 : -1);
            var sgnY = dy === 0 ? (j % 2 ? 1 : -1) : (dy > 0 ? 1 : -1);
            var wA = a.b.isCurrent ? 0.14 : 0.5;
            var wC = c.b.isCurrent ? 0.14 : 0.5;
            if (pushAlongY) {
              var shiftY = oy + 1.2;
              a.py -= sgnY * shiftY * wA;
              c.py += sgnY * shiftY * wC;
              a.px -= sgnX * Math.min(4, ox * 0.12) * wA;
              c.px += sgnX * Math.min(4, ox * 0.12) * wC;
            } else {
              var shiftX = ox + 1.2;
              a.px -= sgnX * shiftX * wA;
              c.px += sgnX * shiftX * wC;
              a.py -= sgnY * Math.min(3, oy * 0.12) * wA;
              c.py += sgnY * Math.min(3, oy * 0.12) * wC;
            }
          }
        }
      }
      for (var k = 0; k < items.length; k++) {
        var it = items[k];
        var minPx = visX0 + it.w * 0.5;
        var maxPx = Math.max(minPx, visX1 - it.w * 0.5);
        var minPy = visY0 + it.h * 0.5;
        var maxPy = Math.max(minPy, visY1 - it.h * 0.5);
        it.px = clamp(it.px, minPx, maxPx);
        it.py = clamp(it.py, minPy, maxPy);
      }
      if (!moved) break;
    }

    return items.map(function (it) {
      return {
        b: it.b,
        px: Math.round(it.px),
        py: Math.round(it.py)
      };
    });
  }

  function renderNodesHtml(laidOut, isBlocked) {
    return laidOut.map(function (it) {
      var b = it.b;
      var cls = 'oi-snm-node ' + (b.isCurrent ? 'is-current' : 'is-reachable');
      if (b.isSecret) cls += ' is-secret';
      if (b.isLeaveCell) cls += ' is-leave';
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

  // 持久化舞台與單指拖曳偷看（拖曳只平移鏡頭，點擊空白處絕不觸發移動）
  function ensureStageDom(container) {
    if (container._snmDom && container.contains(container._snmDom.viewport)) {
      return container._snmDom;
    }
    container.innerHTML =
      '<div class="oi-snm-viewport">' +
        '<div class="oi-snm-stage">' +
          '<img class="oi-snm-img" alt="" draggable="false">' +
          '<div class="oi-snm-night-shade" hidden></div>' +
          '<div class="oi-snm-glows-layer"></div>' +
          '<div class="oi-snm-nodes-layer"></div>' +
        '</div>' +
      '</div>';

    var viewport = container.querySelector('.oi-snm-viewport');
    var stage = container.querySelector('.oi-snm-stage');
    var dom = {
      viewport: viewport,
      stage: stage,
      img: container.querySelector('.oi-snm-img'),
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
    var bgCol = isNight ? '#1b201d' : '#dfd5be';
    var glows = [];
    var built = { buttons: [], mappedExitIds: {} };
    var layerBadge = '';
    var frameMode = 'cell';

    if (scene.dungeon && sm.dungeon_room && sm.dungeon_room.room_url) {
      imgUrl = sm.dungeon_room.room_url;
      stageW = 1024;
      stageH = 1024;
      bgCol = '#12100e';
      built = buildCellButtons(scene);
      layerBadge = '地底石室';
      frameMode = 'cell';
    } else if (townMap) {
      imgUrl = townMap.imgUrl;
      stageW = 900;
      stageH = 1200;
      stageCls = townMap.cropMode === 'wg_legacy' ? 'oi-snm-stage wg-legacy' : 'oi-snm-stage full-bleed';
      bgCol = isNight ? (townMap.bgNight || '#14191e') : (townMap.bgDay || '#dfd5be');
      glows = townMap.glows || [];
      built = buildTownButtons(scene, townMap);
      layerBadge = townMap.title ? townMap.title.split('・')[0].replace(/手繪水彩海港輿圖$/, '') : (sm.cell_title || '城鎮');
      frameMode = 'town';
    } else if (sm.cell_map && sm.cell_map.url) {
      imgUrl = sm.cell_map.url;
      stageW = 1024;
      stageH = 1024;
      bgCol = isNight ? '#1d2420' : (sm.cell_map.bg_day || '#d8cfba');
      glows = sm.cell_map.glows || [];
      built = buildCellButtons(scene);
      layerBadge = sm.cell_title || shortPlaceName(scene.name);
      frameMode = 'cell';
    } else {
      imgUrl = sm.island_master_url || ISLAND_MASTER_FALLBACK;
      stageW = 1024;
      stageH = 1024;
      bgCol = isNight ? '#16242d' : '#35505e';
      built = buildCellButtons(scene);
      layerBadge = sm.cell_title || '銀鷗外島';
      frameMode = 'cell';
    }

    var wrapW = container.clientWidth || 360;
    var wrapH = container.clientHeight || 240;
    var compact = wrapW < 440 || wrapH < 320;
    var frame = computeStageFrame(wrapW, wrapH, stageW, stageH, built.buttons, frameMode);
    var laidOut = layoutButtonsInPixels(built.buttons, frame, wrapW, wrapH, compact);

    var dom = ensureStageDom(container);
    var imgChanged = dom.imgUrl !== imgUrl || dom.mode !== 'local';

    dom.viewport.className = 'oi-snm-viewport' + (isNight ? ' is-night' : '') + (frameMode === 'cell' ? ' is-cell' : '');
    dom.viewport.style.background = bgCol;
    if (imgChanged) {
      dom.stage.classList.add('no-anim');
      dom.img.src = imgUrl;
      dom.img.alt = layerBadge;
      dom.imgUrl = imgUrl;
      dom.mode = 'local';
    }
    dom.stage.className = stageCls + (imgChanged ? ' no-anim' : '');
    dom.stage.style.width = frame.width + 'px';
    dom.stage.style.height = frame.height + 'px';
    dom.stage.style.left = frame.tx + 'px';
    dom.stage.style.top = frame.ty + 'px';
    dom.frame = { width: frame.width, height: frame.height, tx: frame.tx, ty: frame.ty };
    dom.lastNodeId = scene.node_id;

    if (imgChanged) {
      // 下一幀恢復平滑鏡頭轉場（切同城相鄰點時自動絲滑滑動！）
      requestAnimationFrame(function () {
        dom.stage.classList.remove('no-anim');
      });
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
    var frame = computeStageFrame(wrapW, wrapH, 1024, 1024, null, 'overview');

    var dom = ensureStageDom(container);
    var imgChanged = dom.imgUrl !== imgUrl || dom.mode !== 'overview';
    dom.viewport.className = 'oi-snm-viewport is-overview' + (isNight ? ' is-night' : '');
    dom.viewport.style.background = '#263d4a';
    if (imgChanged) {
      dom.img.src = imgUrl;
      dom.img.alt = '銀鷗外島全島手繪總覽';
      dom.imgUrl = imgUrl;
      dom.mode = 'overview';
    }
    dom.stage.className = 'oi-snm-stage full-bleed no-anim';
    dom.stage.style.width = frame.width + 'px';
    dom.stage.style.height = frame.height + 'px';
    dom.stage.style.left = frame.tx + 'px';
    dom.stage.style.top = frame.ty + 'px';
    dom.frame = { width: frame.width, height: frame.height, tx: frame.tx, ty: frame.ty };

    dom.nightShade.hidden = !isNight;
    dom.nightShade.className = 'oi-snm-night-shade is-soft';
    if (dom.glowsLayer.innerHTML !== '') dom.glowsLayer.innerHTML = '';

    var ovHtml = townPinsHtml + curPinHtml;
    if (dom.nodesLayer.innerHTML !== ovHtml) dom.nodesLayer.innerHTML = ovHtml;
  }

  window.StaticNodeMap = {
    renderLocal: renderLocal,
    renderIslandOverview: renderIslandOverview
  };
})();

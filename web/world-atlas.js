/* 奇點世界｜銀鷗島實況全開大地圖・設計者審閱台控制器 */
(function () {
  'use strict';

  var urlToken = new URLSearchParams(location.search).get('token') || '';

  var state = {
    snap: null,
    scale: 'island', // 'island' | 'town:<slug>' | 'dungeon'
    leftTab: 'residents',
    mobilePane: 'map',
    resTown: 'all',
    resAct: 'all',
    resQuery: '',
    showUnbuilt: true,
    showOcc: true,
    autoPoll: true,
    selectedNodeId: 'oi:white_gull_harbor:dock',
    selectedCellId: 4616,
    selectedActorId: '',
    nodeDetail: null,
    cam: { cx: 1425, cy: 695, w: 195, h: 215 },
    defaultCam: { cx: 1425, cy: 695, w: 195, h: 215 },
    drag: { active: false, moved: false, sx: 0, sy: 0, scx: 0, scy: 0 }
  };

  function byId(id) { return document.getElementById(id); }

  function esc(s) {
    return String(s == null ? '' : s)
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  function pad2(n) { return n < 10 ? ('0' + n) : String(n); }

  function detHash(str) {
    var h = 2166136261;
    var s = String(str || '');
    for (var i = 0; i < s.length; i++) {
      h ^= s.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return h >>> 0;
  }

  function apiFetch(path) {
    var headers = {};
    if (urlToken) headers['x-creator-token'] = urlToken;
    return fetch(path, { headers: headers, credentials: 'same-origin' }).then(function (r) {
      return r.json();
    });
  }

  function setMobilePane(pane) {
    state.mobilePane = pane;
    var ws = byId('waWorkspace');
    if (ws) ws.setAttribute('data-mobile-pane', pane);
    Array.prototype.forEach.call(document.querySelectorAll('#waMobileTabs button'), function (b) {
      b.classList.toggle('active', b.getAttribute('data-mpane') === pane);
    });
  }

  function resetCameraForScale(scaleKey) {
    if (scaleKey === 'island') {
      var b = (state.snap && state.snap.map && state.snap.map.bounds) || [1340, 605, 1525, 805];
      var w = Math.max(40, b[2] - b[0]);
      var h = Math.max(40, b[3] - b[1]);
      state.defaultCam = { cx: (b[0] + b[2]) / 2, cy: (b[1] + b[3]) / 2, w: w, h: h };
    } else if (scaleKey.indexOf('town:') === 0) {
      var tSlugCam = scaleKey.slice(5);
      if (window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[tSlugCam]) {
        state.defaultCam = { cx: 450, cy: 600, w: 900, h: 1200 };
      } else {
        var tObjCam = (state.snap && state.snap.map && state.snap.map.town_micro_maps || {})[tSlugCam];
        if (tObjCam && cellMapForCell(tObjCam.cell_id)) {
          state.defaultCam = { cx: 450, cy: 450, w: 920, h: 920 };
        } else {
          state.defaultCam = { cx: 320, cy: 430, w: 640, h: 860 };
        }
      }
    } else if (scaleKey.indexOf('cell:') === 0 || scaleKey.indexOf('dngroom:') === 0) {
      state.defaultCam = { cx: 450, cy: 450, w: 920, h: 920 };
    } else {
      state.defaultCam = { cx: 450, cy: 450, w: 940, h: 940 };
    }
    state.cam = {
      cx: state.defaultCam.cx,
      cy: state.defaultCam.cy,
      w: state.defaultCam.w,
      h: state.defaultCam.h
    };
  }

  function switchScale(scaleKey, keepCamera) {
    state.scale = scaleKey;
    Array.prototype.forEach.call(document.querySelectorAll('#scaleBar button'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-scale') === scaleKey);
    });
    var cellSel = byId('waCellMapSelect');
    if (cellSel) {
      var selVal = '';
      if (scaleKey.indexOf('cell:') === 0) selVal = scaleKey;
      else if (scaleKey.indexOf('town:burg_') === 0) selVal = 'cell:' + scaleKey.slice(10);
      cellSel.value = selVal;
      cellSel.classList.toggle('active', !!selVal);
    }
    if (!keepCamera) resetCameraForScale(scaleKey);
    renderMapStage();
  }

  function polyPointsAttr(poly) {
    return (poly || []).map(function (pt) { return pt[0] + ',' + pt[1]; }).join(' ');
  }

  function cellFillColor(cell, isNight) {
    if (cell.kind === 'settlement') return isNight ? '#68482e' : '#dfc39d';
    if (cell.kind === 'rural_settlement') return isNight ? '#4f4028' : '#e6d7b4';
    if (cell.kind === 'ruin_dungeon') return isNight ? '#3d2a20' : '#cab59c';
    if (cell.kind === 'plague_ruin') return isNight ? '#3a2d3b' : '#cfc2cb';
    if (cell.kind === 'road_corridor') return isNight ? '#364a38' : '#b7cba2';
    if (cell.flux >= 30) return isNight ? '#2c4d54' : '#a8cbbd';
    if (cell.height >= 24) return isNight ? '#4d4438' : '#c6b699';
    if (cell.is_coast) return isNight ? '#3d4b3c' : '#d2d7ab';
    return isNight ? '#2b4534' : '#9bbd8e';
  }

  function townSlugForCell(cid) {
    if (!state.snap || !state.snap.map) return null;
    var towns = state.snap.map.town_micro_maps || {};
    var found = null;
    Object.keys(towns).forEach(function (slug) {
      if (towns[slug].cell_id === cid) found = slug;
    });
    return found;
  }

  function buildIslandSvg() {
    var m = state.snap.map;
    var isNight = document.body.classList.contains('night');
    var seaOuter = isNight ? '#131f26' : '#c2d7df';
    var seaInner = isNight ? '#1b2c36' : '#d8e7ed';
    var rippleCol = isNight ? '#466878' : '#85aab8';
    var ink = isNight ? '#f5ecdf' : '#261b11';
    var halo = isNight ? 'rgba(23,20,17,0.88)' : 'rgba(252,249,242,0.90)';

    var vb = (state.cam.cx - state.cam.w / 2).toFixed(2) + ' ' +
             (state.cam.cy - state.cam.h / 2).toFixed(2) + ' ' +
             state.cam.w.toFixed(2) + ' ' + state.cam.h.toFixed(2);

    var parts = [];
    parts.push('<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="銀鷗島無迷霧全開山海輿圖">');
    parts.push(
      '<defs>' +
      '<radialGradient id="waSeaGrad" cx="50%" cy="50%" r="65%">' +
      '<stop offset="0%" stop-color="' + seaInner + '" />' +
      '<stop offset="100%" stop-color="' + seaOuter + '" />' +
      '</radialGradient>' +
      '</defs>'
    );
    parts.push('<rect x="900" y="200" width="1100" height="1100" fill="url(#waSeaGrad)" />');

    var rings = m.ring_polygons || [];
    rings.forEach(function (rp) {
      parts.push('<polygon points="' + polyPointsAttr(rp.polygon) + '" fill="' + (isNight ? '#221d17' : '#ebe2ce') + '" fill-opacity="0.45" stroke="' + (isNight ? '#4d4336' : '#a8997e') + '" stroke-width="0.25" stroke-dasharray="1.2,1.0" />');
    });

    var cells = m.cells || [];
    cells.forEach(function (c) {
      if (c.is_coast) {
        var wp = polyPointsAttr(c.polygon);
        parts.push('<polygon points="' + wp + '" fill="none" stroke="' + rippleCol + '" stroke-width="2.1" stroke-linejoin="round" opacity="0.34" />');
      }
    });

    cells.forEach(function (c) {
      var isSel = state.selectedCellId === c.cell_id;
      var fill = cellFillColor(c, isNight);
      var stroke = isSel ? '#c93824' : (c.kind === 'settlement' ? '#8b3a1e' : (isNight ? '#5c5244' : '#786a56'));
      var sw = isSel ? '0.95' : (c.kind === 'settlement' ? '0.55' : '0.30');
      parts.push(
        '<polygon data-cell-id="' + c.cell_id + '" style="cursor:pointer;" points="' + polyPointsAttr(c.polygon) + '" fill="' + fill + '" stroke="' + stroke + '" stroke-width="' + sw + '" stroke-linejoin="round">' +
        '<title>地塊 #' + c.cell_id + '｜' + esc(c.title) + (c.canon_unbuilt_burg ? ('（正典未建：' + esc(c.canon_unbuilt_burg.name_zh) + '）') : '') + '｜在場居民 ' + c.resident_count + ' 人</title>' +
        '</polygon>'
      );

      var cx = c.center[0], cy = c.center[1];
      var reliefStroke = isNight ? '#998b76' : '#5e503d';
      if (c.kind !== 'settlement') {
        if (c.height >= 24) {
          parts.push('<path pointer-events="none" d="M ' + (cx - 2.0) + ',' + (cy + 0.9) + ' L ' + (cx - 0.7) + ',' + (cy - 1.2) + ' L ' + (cx + 0.5) + ',' + (cy + 0.7) + ' M ' + (cx - 0.1) + ',' + (cy + 0.9) + ' L ' + (cx + 1.1) + ',' + (cy - 0.8) + ' L ' + (cx + 2.1) + ',' + (cy + 0.9) + '" fill="none" stroke="' + reliefStroke + '" stroke-width="0.34" opacity="0.72" />');
        } else if (c.flux >= 30) {
          parts.push('<path pointer-events="none" d="M ' + (cx - 1.8) + ',' + cy + ' Q ' + (cx - 0.9) + ',' + (cy - 0.6) + ' ' + cx + ',' + cy + ' T ' + (cx + 1.8) + ',' + cy + '" fill="none" stroke="' + (isNight ? '#7ab8cc' : '#2d6e84') + '" stroke-width="0.36" />');
        } else if (!c.is_coast) {
          parts.push('<circle pointer-events="none" cx="' + cx + '" cy="' + (cy - 0.4) + '" r="0.75" fill="' + (isNight ? '#466e52' : '#6b9460') + '" opacity="0.76" />');
        }
      }
    });

    (m.sea_corridors || []).forEach(function (s) {
      parts.push('<polyline pointer-events="none" points="' + polyPointsAttr(s.points) + '" fill="none" stroke="' + (isNight ? '#76b0cc' : '#2c6887') + '" stroke-width="0.48" stroke-dasharray="1.4,1.2" />');
    });

    (m.highways || []).forEach(function (h) {
      var hp = polyPointsAttr(h.points);
      parts.push('<polyline pointer-events="none" points="' + hp + '" fill="none" stroke="' + (isNight ? '#261e16' : '#f5ebd6') + '" stroke-width="1.05" stroke-linecap="round" />');
      parts.push('<polyline pointer-events="none" points="' + hp + '" fill="none" stroke="' + (isNight ? '#d9824c' : '#993d1d') + '" stroke-width="0.58" stroke-linecap="round" />');
    });

    // 海峽對岸未實裝誠實標註（置於西北外海安全視域內，避免左緣裁切）
    parts.push(
      '<g pointer-events="none" transform="translate(1372, 616)">' +
      '<rect x="-23" y="-3.2" width="46" height="6.2" rx="1.2" fill="' + (isNight ? 'rgba(45,35,20,0.92)' : 'rgba(247,236,210,0.94)') + '" stroke="#b37b1b" stroke-width="0.35" stroke-dasharray="1.0,0.7" />' +
      '<text x="0" y="0.7" text-anchor="middle" font-size="1.95" font-weight="700" fill="' + (isNight ? '#f0c46c' : '#784f08') + '">← 對岸蒼淵主陸（未實裝）</text>' +
      '</g>'
    );

    // 35 處基層市集鎮／農礦漁村／檢疫橋哨／封村遺址與即時運行中之 11 具實體車船
    if (state.showUnbuilt) {
      var rIdx = 0;
      cells.forEach(function (c) {
        if (c.kind !== 'rural_settlement' && c.kind !== 'plague_ruin') return;
        rIdx++;
        var ux = c.center[0], uy = c.center[1];
        var isRuin = c.kind === 'plague_ruin';
        var shortTitle = String(c.title || '').replace('（封村遺址）', '');
        var label = isRuin ? ('⛩' + shortTitle) : (shortTitle + (state.showOcc ? ('(' + c.resident_count + ')') : ''));
        var bw = Math.max(8.2, label.length * 1.48 + 2.2);
        var placeAbove = (rIdx % 2 === 0);
        var ry = placeAbove ? (uy - 3.45) : (uy + 1.05);
        var ty = placeAbove ? (uy - 1.85) : (uy + 2.65);
        var rSlug = (!isRuin) ? townSlugForCell(c.cell_id) : null;
        var jumpAttr = rSlug ? (' data-town-jump="' + esc(rSlug) + '"') : '';
        var strokeCol = isRuin ? '#7a4b78' : '#2e6e4c';
        var bgCol = isRuin
          ? (isNight ? 'rgba(46,32,48,0.90)' : 'rgba(244,234,244,0.92)')
          : (isNight ? 'rgba(26,44,34,0.90)' : 'rgba(240,250,242,0.94)');
        var fgCol = isRuin ? (isNight ? '#dfb2de' : '#572855') : (isNight ? '#9ce0b8' : '#184d30');
        parts.push(
          '<g data-cell-id="' + c.cell_id + '"' + jumpAttr + ' style="cursor:pointer;">' +
          '<title>地塊 #' + c.cell_id + '｜' + esc(c.title) + '｜在場 ' + c.resident_count + ' 人・資源存量 ' + (c.stock_now || 0) + '/' + (c.stock_cap || 0) + '</title>' +
          '<circle cx="' + ux + '" cy="' + uy + '" r="0.95" fill="' + bgCol + '" stroke="' + strokeCol + '" stroke-width="0.36" />' +
          '<rect x="' + (ux - bw / 2).toFixed(2) + '" y="' + ry.toFixed(2) + '" width="' + bw.toFixed(2) + '" height="2.25" rx="0.55" fill="' + bgCol + '" stroke="' + strokeCol + '" stroke-width="0.24" />' +
          '<text x="' + ux + '" y="' + ty.toFixed(2) + '" text-anchor="middle" font-size="1.42" font-weight="700" fill="' + fgCol + '">' + esc(label) + '</text>' +
          '</g>'
        );
      });

      (m.transit_vehicles || []).forEach(function (v, vi) {
        var vx = v.pos[0] + ((vi % 3) - 1) * 1.15;
        var vy = v.pos[1] + (v.mode === 'barge' ? -1.35 : 1.25);
        var icon = v.mode === 'barge' ? '⛵' : '🐎';
        var vCol = v.mode === 'barge' ? '#1f5f8b' : '#9c421a';
        parts.push(
          '<g data-node-id="' + esc(v.node_id) + '" style="cursor:pointer;" transform="translate(' + vx.toFixed(2) + ',' + vy.toFixed(2) + ')">' +
          '<title>' + esc(v.name_zh) + '｜現處：' + esc(v.node_name) + '｜乘客 ' + v.passenger_count + ' 人・貨箱 ' + v.cargo_crates + ' 箱</title>' +
          '<circle cx="0" cy="0" r="1.45" fill="#fffaf0" stroke="' + vCol + '" stroke-width="0.34" />' +
          '<text x="0" y="0.62" text-anchor="middle" font-size="1.65">' + icon + '</text>' +
          '</g>'
        );
      });
    }

    // 試煉坑暗門標記與 5 座已落實聚落
    cells.forEach(function (c) {
      var vx = c.center[0], vy = c.center[1];
      if (c.has_hidden_entrance) {
        parts.push(
          '<g data-enter-dungeon="1" style="cursor:pointer;" transform="translate(' + vx + ',' + (vy - 2.4) + ')">' +
          '<rect x="-7.5" y="-1.8" width="15" height="2.6" rx="0.6" fill="#2b1e1a" stroke="#e09f4c" stroke-width="0.3" />' +
          '<text x="0" y="-0.05" text-anchor="middle" font-size="1.65" font-weight="700" fill="#f7d59c">⛰ 試煉坑暗門</text>' +
          '</g>'
        );
      }
      if (c.kind === 'settlement') {
        var slug = townSlugForCell(c.cell_id) || 'white_gull_harbor';
        var lbl = c.title + (state.showOcc ? (' (' + c.resident_count + ')') : '');
        var lw = Math.max(12, lbl.length * 1.95 + 2.8);
        parts.push(
          '<g data-town-jump="' + esc(slug) + '" data-cell-id="' + c.cell_id + '" style="cursor:pointer;">' +
          '<g transform="translate(' + vx + ',' + vy + ')">' +
          '<rect x="-1.6" y="-1.2" width="3.2" height="2.5" rx="0.3" fill="#fcf6e8" stroke="#6e2c14" stroke-width="0.42" />' +
          '<path d="M -1.9,-1.2 L 0,-2.5 L 1.9,-1.2 Z" fill="#b34727" stroke="#6e2c14" stroke-width="0.32" />' +
          '</g>' +
          '<rect x="' + (vx - lw / 2).toFixed(2) + '" y="' + (vy - 5.4).toFixed(2) + '" width="' + lw.toFixed(2) + '" height="3.0" rx="0.7" fill="' + halo + '" stroke="#8b3a1e" stroke-width="0.32" />' +
          '<text x="' + vx + '" y="' + (vy - 3.3).toFixed(2) + '" text-anchor="middle" font-size="2.05" font-weight="700" fill="' + ink + '">' + esc(lbl) + '</text>' +
          '</g>'
        );
      } else if (state.showOcc && c.resident_count > 0) {
        parts.push(
          '<g pointer-events="none" transform="translate(' + (vx + 2.0) + ',' + (vy - 1.6) + ')">' +
          '<circle cx="0" cy="0" r="1.25" fill="#8b3a1e" stroke="#fdfaf4" stroke-width="0.25" />' +
          '<text x="0" y="0.55" text-anchor="middle" font-size="1.45" font-weight="700" fill="#fdfaf4">' + c.resident_count + '</text>' +
          '</g>'
        );
      }
    });

    parts.push('</svg>');
    return parts.join('');
  }

  function buildIllustratedTownSvg(slug) {
    var cfg = window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[slug];
    if (!cfg) return buildOtherTownSvg(slug);
    var ns = state.snap.node_summary || {};
    var isNight = document.body.classList.contains('night');
    var vb = (state.cam.cx - state.cam.w / 2).toFixed(1) + ' ' +
             (state.cam.cy - state.cam.h / 2).toFixed(1) + ' ' +
             state.cam.w.toFixed(1) + ' ' + state.cam.h.toFixed(1);

    var parts = [];
    parts.push('<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="' + esc(cfg.title) + '">');
    parts.push('<rect x="-500" y="-500" width="2000" height="2200" fill="' + (isNight ? (cfg.bgNight || '#14191e') : (cfg.bgDay || '#dfd5be')) + '" />');
    parts.push('<image href="' + esc(cfg.imgUrl) + '" x="0" y="0" width="900" height="1200" preserveAspectRatio="xMidYMid slice" />');
    if (isNight) {
      parts.push(
        '<defs><radialGradient id="waGlowGrad" cx="50%" cy="50%" r="50%">' +
        '<stop offset="0%" stop-color="#ffb85c" stop-opacity="0.46" />' +
        '<stop offset="55%" stop-color="#ff8c2b" stop-opacity="0.16" />' +
        '<stop offset="100%" stop-color="#ff8c2b" stop-opacity="0" />' +
        '</radialGradient></defs>'
      );
      parts.push('<rect x="0" y="0" width="900" height="1200" fill="rgba(12,16,24,0.48)" pointer-events="none" />');
      (cfg.glows || []).forEach(function (gl) {
        parts.push('<circle cx="' + gl[0] + '" cy="' + gl[1] + '" r="' + gl[2] + '" fill="url(#waGlowGrad)" pointer-events="none" />');
      });
    }

    (cfg.alleys || []).forEach(function (ad) {
      var sIdx = ad.startIdx != null ? ad.startIdx : 1;
      var mIdx = ad.maxIdx || 12;
      var denom = Math.max(1, mIdx - sIdx + 1);
      for (var hi = sIdx; hi <= mIdx; hi++) {
        var hNid = ad.nid + ':h' + hi;
        var info = ns[hNid];
        if (!info) continue;
        var t = (hi - sIdx + 0.5) / denom;
        var hx = ad.p0[0] + (ad.p1[0] - ad.p0[0]) * Math.min(1, t);
        var hy = ad.p0[1] + (ad.p1[1] - ad.p0[1]) * Math.min(1, t) + ((hi % 2 === 0) ? 18 : -18);
        var isSel = state.selectedNodeId === hNid;
        var fill = info.vacant_for_sale ? '#e0ab38' : (info.built_homestead ? '#2d8a6b' : '#b85335');
        var r = isSel ? 9.5 : 6.8;
        parts.push(
          '<g data-node-id="' + esc(hNid) + '" style="cursor:pointer;">' +
          '<title>' + esc(info.name) + (info.vacant_for_sale ? '（市政待售空屋）' : '') + '｜屋內在場 ' + info.occupant_count + ' 人・木箱物資 ' + info.box_item_count + ' 件</title>' +
          '<circle cx="' + hx.toFixed(1) + '" cy="' + hy.toFixed(1) + '" r="' + r + '" fill="' + fill + '" stroke="' + (isSel ? '#ffffff' : '#26180f') + '" stroke-width="' + (isSel ? '2.6' : '1.4') + '" />' +
          (state.showOcc && info.occupant_count > 0
            ? '<text x="' + hx.toFixed(1) + '" y="' + (hy + 3.4).toFixed(1) + '" text-anchor="middle" font-size="9.5" font-family="sans-serif" font-weight="700" fill="#fffaf0">' + info.occupant_count + '</text>'
            : '') +
          '</g>'
        );
      }
    });

    function townPill(nid, label, x, y, kind) {
      var info = ns[nid] || { occupant_count: 0, box_item_count: 0 };
      var isSel = state.selectedNodeId === nid;
      var txt = label + (state.showOcc && info.occupant_count > 0 ? (' · ' + info.occupant_count + '人') : '');
      var fs = kind === 'alley' ? 12.5 : 14.2;
      var pw = Math.max(58, txt.length * (fs * 0.92) + 16);
      var ph = fs + 11;
      var bg = isSel ? '#8b3a1e' : (kind === 'shop' ? '#fdf6e4' : (kind === 'main' ? '#f7ecd5' : 'rgba(250,244,232,0.92)'));
      var fg = isSel ? '#fffaf2' : (kind === 'shop' ? '#6e2811' : '#24180e');
      var br = isSel ? '#ffd27d' : (kind === 'shop' ? '#9e4725' : '#6e5a44');
      parts.push(
        '<g data-node-id="' + esc(nid) + '" style="cursor:pointer;" transform="translate(' + x + ',' + y + ')">' +
        '<title>' + esc(info.name || label) + '｜在場 ' + info.occupant_count + ' 人・木箱存貨 ' + info.box_item_count + ' 件</title>' +
        '<rect x="' + (-pw / 2).toFixed(1) + '" y="' + (-ph / 2).toFixed(1) + '" width="' + pw.toFixed(1) + '" height="' + ph.toFixed(1) + '" rx="7" fill="' + bg + '" stroke="' + br + '" stroke-width="' + (isSel ? '2.5' : '1.4') + '" />' +
        '<text x="0" y="' + (fs * 0.35).toFixed(1) + '" text-anchor="middle" font-size="' + fs + '" font-weight="700" fill="' + fg + '">' + esc(txt) + '</text>' +
        '</g>'
      );
    }

    (cfg.pills || []).forEach(function (pl) {
      townPill(pl.nid, pl.atlasLabel || pl.label, pl.x, pl.y, pl.kind === 'street' ? 'main' : pl.kind);
    });
    (cfg.alleys || []).forEach(function (ad) {
      townPill(ad.nid, ad.name, ad.lx, ad.ly, 'alley');
    });

    parts.push('</svg>');
    return parts.join('');
  }

  function buildOtherTownSvg(slug) {
    var towns = (state.snap.map && state.snap.map.town_micro_maps) || {};
    var activeTown = towns[slug];
    if (!activeTown) {
      return '<div class="wa-empty-hint">找不到該城鎮街廓資料</div>';
    }
    var ns = state.snap.node_summary || {};
    var isNight = document.body.classList.contains('night');
    var vb = (state.cam.cx - state.cam.w / 2).toFixed(1) + ' ' +
             (state.cam.cy - state.cam.h / 2).toFixed(1) + ' ' +
             state.cam.w.toFixed(1) + ' ' + state.cam.h.toFixed(1);

    var landCol = isNight ? '#241e17' : '#f2e8d2';
    var seaCol = isNight ? '#152730' : '#a8c8d4';
    var roadCol = isNight ? '#57493a' : '#d5c5a7';
    var roadKerb = isNight ? '#382f25' : '#948266';
    var ink = isNight ? '#f4eadb' : '#241910';
    var coastDir = activeTown.coast_dir || (activeTown.has_port ? 'east' : 'none');

    var parts = [];
    parts.push('<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="' + esc(activeTown.name) + '街廓輿圖">');
    parts.push('<rect x="-600" y="-600" width="1800" height="2100" fill="' + landCol + '" />');

    var dockPos = [516, 406];
    if (coastDir === 'east') {
      parts.push('<rect x="486" y="-600" width="800" height="2100" fill="' + seaCol + '" />');
      parts.push('<rect x="476" y="394" width="90" height="24" rx="2" fill="#b38659" stroke="#472d1c" stroke-width="1.4" />');
      dockPos = [522, 406];
    } else if (coastDir === 'west') {
      parts.push('<rect x="-600" y="-600" width="750" height="2100" fill="' + seaCol + '" />');
      parts.push('<rect x="70" y="394" width="92" height="24" rx="2" fill="#b38659" stroke="#472d1c" stroke-width="1.4" />');
      dockPos = [114, 406];
    } else if (coastDir === 'south') {
      parts.push('<rect x="-600" y="786" width="1800" height="800" fill="' + seaCol + '" />');
      parts.push('<rect x="462" y="394" width="88" height="22" rx="2" fill="#b38659" stroke="#472d1c" stroke-width="1.4" />');
      dockPos = [508, 406];
    } else if (coastDir === 'north') {
      parts.push('<rect x="-600" y="-600" width="1800" height="650" fill="' + seaCol + '" />');
      parts.push('<rect x="462" y="394" width="88" height="22" rx="2" fill="#b38659" stroke="#472d1c" stroke-width="1.4" />');
      dockPos = [508, 406];
    }

    parts.push('<line x1="276" y1="68" x2="276" y2="758" stroke="' + roadKerb + '" stroke-width="22" stroke-linecap="round" />');
    parts.push('<line x1="276" y1="68" x2="276" y2="758" stroke="' + roadCol + '" stroke-width="17" stroke-linecap="round" />');
    if (activeTown.has_port) {
      var dx = coastDir === 'west' ? 156 : 486;
      parts.push('<line x1="276" y1="406" x2="' + dx + '" y2="406" stroke="' + roadCol + '" stroke-width="14" stroke-linecap="round" />');
    }
    parts.push('<circle cx="276" cy="404" r="40" fill="' + roadCol + '" stroke="' + roadKerb + '" stroke-width="2.4" />');

    var alleys = activeTown.alleys || [];
    var roofs = activeTown.roofs || [];
    var totalRows = Math.max(1, Math.ceil(alleys.length / 2));

    alleys.forEach(function (al, idx) {
      var isEast = al.side === 'east';
      var r = Math.floor(idx / 2);
      var y = totalRows >= 5 ? (135 + r * 135) : (180 + r * 185);
      if (Math.abs(y - 404) < 48) y += 58;
      var x0 = isEast ? 290 : 262;
      var x1 = isEast ? 442 : 102;
      parts.push('<line x1="' + x0 + '" y1="' + y + '" x2="' + x1 + '" y2="' + y + '" stroke="' + roadCol + '" stroke-width="8" stroke-linecap="round" />');

      var aNid = al.node_id || ('oi:' + slug + ':' + idx);
      var aInfo = ns[aNid] || { occupant_count: 0 };
      var aSel = state.selectedNodeId === aNid;
      var aTxt = al.name + (state.showOcc && aInfo.occupant_count > 0 ? ('·' + aInfo.occupant_count) : '');
      var lx = isEast ? (x1 - 4) : (x1 + 4);
      parts.push(
        '<text data-node-id="' + esc(aNid) + '" style="cursor:pointer;" x="' + lx + '" y="' + (y - 22) + '" text-anchor="' + (isEast ? 'end' : 'start') + '" font-size="11.5" font-weight="700" fill="' + (aSel ? '#c93824' : ink) + '">' + esc(aTxt) + '</text>'
      );

      var myRoofs = roofs.filter(function (rf) { return rf.alley_idx === idx; });
      myRoofs.forEach(function (rf, k) {
        var sideSign = (k % 2 === 0) ? -1 : 1;
        var colIdx = Math.floor(k / 2);
        var colTotal = Math.max(1, Math.ceil(myRoofs.length / 2));
        var t = (colIdx + 0.5) / colTotal;
        var rx = x0 + (x1 - x0) * (0.10 + t * 0.84);
        var ry = y + sideSign * 14;
        var rInfo = (rf.node_id && ns[rf.node_id]) || { occupant_count: 0, box_item_count: 0 };
        var rSel = rf.node_id && state.selectedNodeId === rf.node_id;
        var fill = rf.vacant ? '#dfb351' : '#bf6842';
        parts.push(
          '<g ' + (rf.node_id ? ('data-node-id="' + esc(rf.node_id) + '" style="cursor:pointer;"') : '') + '>' +
          '<title>' + esc(rf.label) + '｜在場 ' + rInfo.occupant_count + ' 人・木箱 ' + rInfo.box_item_count + ' 件</title>' +
          '<rect x="' + (rx - 9.5).toFixed(1) + '" y="' + (ry - 6).toFixed(1) + '" width="19" height="12" rx="1.5" fill="' + fill + '" stroke="' + (rSel ? '#ffffff' : '#452b1a') + '" stroke-width="' + (rSel ? '2.2' : '1.0') + '" />' +
          (state.showOcc && rInfo.occupant_count > 0
            ? '<text x="' + rx.toFixed(1) + '" y="' + (ry + 3.2).toFixed(1) + '" text-anchor="middle" font-size="8.5" font-family="sans-serif" font-weight="700" fill="#fffaf2">' + rInfo.occupant_count + '</text>'
            : '') +
          '</g>'
        );
      });
    });

    function townNodePill(nid, label, x, y) {
      if (!nid) return;
      var info = ns[nid] || { occupant_count: 0 };
      var isSel = state.selectedNodeId === nid;
      var txt = label + (state.showOcc && info.occupant_count > 0 ? (' · ' + info.occupant_count + '人') : '');
      var pw = Math.max(52, txt.length * 12 + 14);
      parts.push(
        '<g data-node-id="' + esc(nid) + '" style="cursor:pointer;" transform="translate(' + x + ',' + y + ')">' +
        '<rect x="' + (-pw / 2).toFixed(1) + '" y="-11" width="' + pw.toFixed(1) + '" height="22" rx="6" fill="' + (isSel ? '#8b3a1e' : '#fcf6e8') + '" stroke="#7a3519" stroke-width="1.3" />' +
        '<text x="0" y="4" text-anchor="middle" font-size="11.5" font-weight="700" fill="' + (isSel ? '#fffaf2' : '#261910') + '">' + esc(txt) + '</text>' +
        '</g>'
      );
    }

    var sp = activeTown.spine || {};
    townNodePill(sp.north_gate, activeTown.walled ? '北城門' : '北哨柵口', 276, 52);
    townNodePill(sp.smithy_st, '鐵匠街', 276, 260);
    townNodePill(sp.plaza, '市集廣場', 276, 404);
    townNodePill(sp.tavern_st, '驛站街', 276, 512);
    townNodePill(sp.south_gate, activeTown.walled ? '南城門' : '南哨柵口', 276, 772);
    if (activeTown.has_port) townNodePill(sp.dock, '港務碼頭', dockPos[0], dockPos[1] - 20);

    var lmPos = [[198, 294], [354, 294], [198, 492], [354, 492]];
    (activeTown.landmarks || []).forEach(function (lm, i) {
      if (lm && lm.node_id && lmPos[i]) {
        townNodePill(lm.node_id, lm.label, lmPos[i][0], lmPos[i][1]);
      }
    });
    if (ns['oi:' + slug + ':inn_upstairs']) {
      townNodePill('oi:' + slug + ':inn_upstairs', '酒館二樓宿房', 198, 522);
    }

    parts.push('</svg>');
    return parts.join('');
  }

  function buildDungeonSvg() {
    var rooms = (state.snap.map && state.snap.map.dungeon_rooms) || [];
    var tiledRooms = rooms.filter(function (r) { return r && r.tile && r.tile.url && r.tile.box; });

    var vb = (state.cam.cx - state.cam.w / 2).toFixed(1) + ' ' +
             (state.cam.cy - state.cam.h / 2).toFixed(1) + ' ' +
             state.cam.w.toFixed(1) + ' ' + state.cam.h.toFixed(1);
    var parts = [];
    parts.push('<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="試煉坑地底石窟手繪全開輿圖">');
    parts.push('<rect x="-800" y="-800" width="2600" height="2600" fill="#11100e" />');

    if (tiledRooms.length) {
      tiledRooms.forEach(function (r) {
        var b = r.tile.box;
        parts.push(
          '<image href="' + esc(r.tile.url) + '" x="' + b[0] + '" y="' + b[1] + '" width="' + b[2] + '" height="' + b[3] + '" pointer-events="none" />'
        );
      });

      tiledRooms.forEach(function (r) {
        var isSel = state.selectedNodeId === r.id;
        var anc = r.tile.anchor || [450, 450];
        var lp = r.tile.label_pos || anc;
        var shortName = (r.name || '').split('・').pop() || r.name;
        var occ = r.occupant_count || 0;
        var txt = shortName + (state.showOcc && occ > 0 ? (' · ' + occ + '人') : '');
        var fs = 14.0;
        var pw = Math.max(82, txt.length * 14.2 + 20);
        var ph = 26;
        var bg = isSel ? 'rgba(139, 58, 30, 0.94)' : 'rgba(18, 14, 10, 0.88)';
        var br = isSel ? '#ffd27d' : 'rgba(217, 164, 67, 0.52)';
        var fg = isSel ? '#fffaf0' : '#f5ebd8';

        parts.push(
          '<g data-node-id="' + esc(r.id) + '" style="cursor:pointer;">' +
          '<title>' + esc(r.name) + '｜在場 ' + occ + ' 人｜' + esc(r.desc || '') + '</title>' +
          '<circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="68" fill="rgba(0,0,0,0.001)" />' +
          (isSel
            ? '<circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="18" fill="rgba(201,56,36,0.22)" stroke="#f0b643" stroke-width="2.2" />' +
              '<circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="7.5" fill="#c93824" stroke="#fdf8ee" stroke-width="2.2" />'
            : '') +
          (state.showOcc && occ > 0
            ? '<circle cx="' + (anc[0] + 34) + '" cy="' + (anc[1] - 28) + '" r="13" fill="#8b3a1e" stroke="#ffd27d" stroke-width="1.8" />' +
              '<text x="' + (anc[0] + 34) + '" y="' + (anc[1] - 23.5) + '" text-anchor="middle" font-size="12" font-family="sans-serif" font-weight="700" fill="#fffaf0">' + occ + '</text>'
            : '') +
          '<g transform="translate(' + lp[0] + ',' + lp[1] + ')">' +
          '<rect x="' + (-pw / 2).toFixed(1) + '" y="' + (-ph / 2).toFixed(1) + '" width="' + pw.toFixed(1) + '" height="' + ph + '" rx="6.5" fill="' + bg + '" stroke="' + br + '" stroke-width="' + (isSel ? '2.4' : '1.4') + '" />' +
          '<text x="0" y="' + (fs * 0.36).toFixed(1) + '" text-anchor="middle" font-size="' + fs + '" font-weight="700" fill="' + fg + '">' + esc(txt) + '</text>' +
          '</g>' +
          '</g>'
        );
      });

      parts.push('</svg>');
      return parts.join('');
    }

    var byRoom = {};
    rooms.forEach(function (r) { byRoom[r.id] = r; });

    rooms.forEach(function (r) {
      var x1 = r.pos[0] * 115, y1 = r.pos[1] * 98;
      (r.exits || []).forEach(function (ex) {
        var dst = byRoom[ex.to];
        if (dst) {
          parts.push('<line x1="' + x1 + '" y1="' + y1 + '" x2="' + (dst.pos[0] * 115) + '" y2="' + (dst.pos[1] * 98) + '" stroke="#6e5d49" stroke-width="6" />');
        }
      });
    });

    rooms.forEach(function (r) {
      var cx = r.pos[0] * 115, cy = r.pos[1] * 98;
      var isSel = state.selectedNodeId === r.id;
      var shortName = (r.name || '').split('・').pop() || r.name;
      var txt = shortName + (r.occupant_count > 0 ? (' (' + r.occupant_count + ')') : '');
      parts.push(
        '<g data-node-id="' + esc(r.id) + '" style="cursor:pointer;">' +
        '<rect x="' + (cx - 46) + '" y="' + (cy - 22) + '" width="92" height="44" rx="8" fill="' + (isSel ? '#8b3a1e' : '#2d251e') + '" stroke="' + (isSel ? '#f4c76c' : '#96846e') + '" stroke-width="2.2" />' +
        '<text x="' + cx + '" y="' + (cy + 4) + '" text-anchor="middle" font-size="11.5" font-weight="700" fill="#f5ecdf">' + esc(txt) + '</text>' +
        '</g>'
      );
    });
    parts.push('</svg>');
    return parts.join('');
  }

  function cellMapForCell(cid) {
    if (!state.snap || !state.snap.map) return null;
    var cells = state.snap.map.cells || [];
    for (var i = 0; i < cells.length; i++) {
      if (cells[i].cell_id === cid && cells[i].cell_map && cells[i].cell_map.url) {
        return cells[i];
      }
    }
    return null;
  }

  function dngRoomForId(nid) {
    var rooms = (state.snap && state.snap.map && state.snap.map.dungeon_rooms) || [];
    for (var i = 0; i < rooms.length; i++) {
      if (rooms[i].id === nid && rooms[i].tile && rooms[i].tile.room_url) return rooms[i];
    }
    return null;
  }

  function buildSingleSceneSvg(label, cfg) {
    if (!cfg || !cfg.url) return '<div class="wa-empty-hint">尚未配置單格地景圖</div>';
    var isNight = document.body.classList.contains('night') && !cfg.is_subterranean;
    var vb = (state.cam.cx - state.cam.w / 2).toFixed(1) + ' ' +
             (state.cam.cy - state.cam.h / 2).toFixed(1) + ' ' +
             state.cam.w.toFixed(1) + ' ' + state.cam.h.toFixed(1);
    var bg = cfg.is_subterranean ? '#12100e' : (isNight ? (cfg.bg_night || '#141618') : (cfg.bg_day || '#dbd1bc'));
    var parts = [
      '<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="' + esc(label) + '">',
      '<rect x="-600" y="-600" width="2100" height="2100" fill="' + bg + '" />',
      '<image href="' + esc(cfg.url) + '" x="0" y="0" width="900" height="900" preserveAspectRatio="xMidYMid slice" />'
    ];
    if (isNight) {
      parts.push('<defs><radialGradient id="waCellGlowGrad" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="#ffb85c" stop-opacity="0.48" /><stop offset="55%" stop-color="#ff8c2b" stop-opacity="0.18" /><stop offset="100%" stop-color="#ff8c2b" stop-opacity="0" /></radialGradient></defs>');
      parts.push('<rect x="0" y="0" width="900" height="900" fill="rgba(12,16,24,0.46)" pointer-events="none" />');
      (cfg.glows || []).forEach(function (gl) {
        parts.push('<circle cx="' + gl[0] + '" cy="' + gl[1] + '" r="' + gl[2] + '" fill="url(#waCellGlowGrad)" pointer-events="none" />');
      });
    }
    parts.push('</svg>');
    return parts.join('');
  }

  function renderMapStage() {
    if (!state.snap) return;
    var host = byId('svgHost');
    var leg = byId('mapLegend');
    if (state.scale === 'island') {
      host.innerHTML = buildIslandSvg();
      leg.innerHTML =
        '<span>🏰 紅頂主城＝5 大核心鎮</span>' +
        '<span>🟢 綠標＝29 座活態鄉鎮村塢</span>' +
        '<span>⛩ 紫標＝南岸封村遺址</span>' +
        '<span>🐎/⛵ 即時運行驛車與貨駁</span>';
    } else if (state.scale.indexOf('cell:') === 0) {
      var targetCid = Number(state.scale.slice(5));
      var cInfo = cellMapForCell(targetCid);
      host.innerHTML = buildSingleSceneSvg('地塊 #' + targetCid, cInfo && cInfo.cell_map);
      leg.innerHTML = '<span>🖼️ 單格手繪地景輿圖：#' + targetCid + ' ' + esc((cInfo && cInfo.title) || '') + '（純景物無出入口貼標，對齊實際動線方位）</span>';
    } else if (state.scale.indexOf('dngroom:') === 0) {
      var targetNid = state.scale.slice(8);
      var rInfo = dngRoomForId(targetNid);
      host.innerHTML = buildSingleSceneSvg((rInfo && rInfo.name) || targetNid, rInfo && { url: rInfo.tile.room_url, is_subterranean: true });
      leg.innerHTML = '<span>🕯️ 試煉坑單室細部圖：' + esc((rInfo && rInfo.name) || '') + '（純景物零出入口標示，對齊實際石道方位）</span>';
    } else if (state.scale.indexOf('town:') === 0) {
      var curTownSlug = state.scale.slice(5);
      if (window.TOWN_ILLUSTRATED_MAPS && window.TOWN_ILLUSTRATED_MAPS[curTownSlug]) {
        host.innerHTML = buildIllustratedTownSvg(curTownSlug);
        leg.innerHTML =
          '<span>🔴 支巷圓點＝常住民宅門牌（數字為屋內即時人數）</span>' +
          '<span>🟡 金點＝市政待售空屋</span>' +
          '<span>🟢 綠點＝居民自立新建屋</span>';
      } else {
        var tMicro = (state.snap.map && state.snap.map.town_micro_maps || {})[curTownSlug];
        var burgCell = tMicro && cellMapForCell(tMicro.cell_id);
        if (burgCell && burgCell.cell_map) {
          host.innerHTML = buildSingleSceneSvg('地塊 #' + burgCell.cell_id, burgCell.cell_map);
          leg.innerHTML = '<span>🏘️ 鄉野聚落手繪外景輿圖：#' + burgCell.cell_id + ' ' + esc(burgCell.title || '') + '（純景物零出入口標示，對齊實際動線與海岸）</span>';
        } else {
          host.innerHTML = buildOtherTownSvg(curTownSlug);
          leg.innerHTML =
            '<span>🏠 斜瓦屋頂＝各巷家戶（可點選查看屋內人物與存糧箱）</span>' +
            '<span>🟡 金頂＝待售空屋</span>';
        }
      }
    } else {
      host.innerHTML = buildDungeonSvg();
      leg.innerHTML = '<span>⛏ 地底試煉坑全 6 室無迷霧展開（點選石室查看首殺與符文狀態）</span>';
    }
  }

  function renderHeader() { window.WorldAtlasPanels.renderHeader(state); }
  function renderResidentsTab() { window.WorldAtlasPanels.renderResidentsTab(state); }
  function renderPulseTab() { window.WorldAtlasPanels.renderPulseTab(state); }
  function renderAuditTab() { window.WorldAtlasPanels.renderAuditTab(state); }
  function renderInspector() { window.WorldAtlasPanels.renderInspector(state, townSlugForCell); }

  function focusOnActor(actorId) {
    if (!state.snap) return;
    var found = null;
    (state.snap.residents || []).forEach(function (r) {
      if (r.actor_id === actorId) found = r;
    });
    if (!found) return;
    state.selectedActorId = actorId;
    state.selectedCellId = found.cell_id;
    state.selectedNodeId = found.current_node_id;

    if (found.current_node_id.indexOf('oi:dng_') === 0) {
      switchScale('dungeon', false);
    } else {
      var tSlug = townSlugForCell(found.cell_id);
      if (tSlug) {
        switchScale('town:' + tSlug, false);
      } else {
        switchScale('island', false);
      }
    }
    renderResidentsTab();
    loadNodeInspector(found.current_node_id, actorId);
  }

  function loadNodeInspector(queryId, highlightActorId) {
    apiFetch('/api/world_atlas/node?id=' + encodeURIComponent(queryId)).then(function (d) {
      if (!d || !d.ok) return;
      state.nodeDetail = d;
      state.selectedNodeId = d.node_id;
      state.selectedCellId = d.cell_id;
      if (highlightActorId) state.selectedActorId = highlightActorId;
      renderMapStage();
      renderInspector();
      var mBar = byId('mobilePickBar');
      if (mBar) {
        mBar.classList.remove('hidden');
        mBar.innerHTML =
          '<span>已選：<b>' + esc(d.name) + '</b>（在場 ' + (d.occupants || []).length + ' 人）</span>' +
          '<button type="button" id="btnOpenMobInsp">查看詳情 →</button>';
        var ob = byId('btnOpenMobInsp');
        if (ob) ob.onclick = function () { setMobilePane('right'); };
      }
    });
  }

  function populateCellMapSelect() {
    var sel = byId('waCellMapSelect');
    if (!sel || sel.dataset.filled === '1' || !state.snap || !state.snap.map) return;
    var cells = (state.snap.map.cells || []).filter(function (c) { return c.cell_map && c.cell_map.url; });
    cells.sort(function (a, b) { return a.cell_id - b.cell_id; });
    var burgs = cells.filter(function (c) { return c.kind === 'rural_settlement'; });
    var ruins = cells.filter(function (c) { return c.kind === 'plague_ruin'; });
    var others = cells.filter(function (c) { return c.kind !== 'rural_settlement' && c.kind !== 'plague_ruin'; });
    function grpHtml(label, arr) {
      if (!arr.length) return '';
      return '<optgroup label="' + esc(label) + '">' + arr.map(function (c) {
        return '<option value="cell:' + c.cell_id + '">#' + c.cell_id + ' ' + esc(c.title) + '</option>';
      }).join('') + '</optgroup>';
    }
    sel.innerHTML =
      '<option value="">🖼️ 村塢／封村手繪地景（' + cells.length + ' 格）…</option>' +
      grpHtml('🏘️ 活態鄉野村塢（' + burgs.length + ' 村）', burgs) +
      grpHtml('⛩️ 南岸封村遺址（' + ruins.length + ' 處）', ruins) +
      grpHtml('🌲 試煉坑口與驛道（' + others.length + ' 處）', others);
    sel.dataset.filled = '1';
  }

  function refreshSnapshot(isInitial) {
    return apiFetch('/api/world_atlas/snapshot').then(function (snap) {
      if (!snap || !snap.ok) return;
      state.snap = snap;
      populateCellMapSelect();
      if (isInitial) {
        resetCameraForScale(state.scale);
      }
      renderHeader();
      renderResidentsTab();
      renderPulseTab();
      renderAuditTab();
      renderMapStage();

      if (isInitial) {
        var rawHash = '';
        try { rawHash = decodeURIComponent(location.hash.replace(/^#/, '')); } catch (_) { rawHash = ''; }
        if (rawHash && rawHash.indexOf('res:') === 0) {
          focusOnActor(rawHash);
          return;
        }
        if (rawHash && rawHash.indexOf('oi:') === 0) {
          loadNodeInspector(rawHash, '');
          return;
        }
        loadNodeInspector(state.selectedNodeId, '');
      } else if (state.selectedNodeId) {
        loadNodeInspector(state.selectedNodeId, state.selectedActorId);
      }
    });
  }

  function bindEvents() {
    // 頂欄控制
    byId('chkUnbuilt').onchange = function (e) { state.showUnbuilt = e.target.checked; renderMapStage(); };
    byId('chkOcc').onchange = function (e) { state.showOcc = e.target.checked; renderMapStage(); };
    byId('chkAutoPoll').onchange = function (e) { state.autoPoll = e.target.checked; };
    byId('btnRefresh').onclick = function () { refreshSnapshot(false); };
    byId('btnTheme').onclick = function () { document.body.classList.toggle('night'); renderMapStage(); };

    // 行動端三欄切換
    Array.prototype.forEach.call(document.querySelectorAll('#waMobileTabs button'), function (btn) {
      btn.onclick = function () { setMobilePane(btn.getAttribute('data-mpane')); };
    });

    // 左欄分頁
    Array.prototype.forEach.call(document.querySelectorAll('#leftTabs button'), function (btn) {
      btn.onclick = function () {
        var tab = btn.getAttribute('data-ltab');
        state.leftTab = tab;
        Array.prototype.forEach.call(document.querySelectorAll('#leftTabs button'), function (b) {
          b.classList.toggle('active', b === btn);
        });
        byId('ltabResidents').classList.toggle('hidden', tab !== 'residents');
        byId('ltabPulse').classList.toggle('hidden', tab !== 'pulse');
        byId('ltabAudit').classList.toggle('hidden', tab !== 'audit');
      };
    });

    // 居民篩選
    byId('resSearch').oninput = function (e) { state.resQuery = e.target.value; renderResidentsTab(); };
    Array.prototype.forEach.call(document.querySelectorAll('#resTownPills button'), function (btn) {
      btn.onclick = function () {
        state.resTown = btn.getAttribute('data-town');
        Array.prototype.forEach.call(document.querySelectorAll('#resTownPills button'), function (b) {
          b.classList.toggle('active', b === btn);
        });
        renderResidentsTab();
      };
    });
    Array.prototype.forEach.call(document.querySelectorAll('#resActPills button'), function (btn) {
      btn.onclick = function () {
        state.resAct = btn.getAttribute('data-act');
        Array.prototype.forEach.call(document.querySelectorAll('#resActPills button'), function (b) {
          b.classList.toggle('active', b === btn);
        });
        renderResidentsTab();
      };
    });

    // 尺度切換列
    Array.prototype.forEach.call(document.querySelectorAll('#scaleBar button'), function (btn) {
      btn.onclick = function () {
        var sc = btn.getAttribute('data-scale');
        switchScale(sc, false);
        if (sc === 'dungeon') {
          loadNodeInspector('oi:dng_pit:entry', '');
        } else if (sc.indexOf('dngroom:') === 0) {
          loadNodeInspector(sc.slice(8), '');
        } else if (sc.indexOf('cell:') === 0) {
          loadNodeInspector('cell:' + sc.slice(5), '');
        } else if (sc.indexOf('town:') === 0) {
          loadNodeInspector('oi:' + sc.slice(5) + ':plaza', '');
        }
      };
    });
    var cellSelEl = byId('waCellMapSelect');
    if (cellSelEl) {
      cellSelEl.onchange = function () {
        var v = cellSelEl.value;
        if (!v) return;
        switchScale(v, false);
        if (v.indexOf('cell:') === 0) {
          loadNodeInspector('cell:' + v.slice(5), '');
        }
      };
    }

    // 縮放按鈕
    byId('btnZoomIn').onclick = function () {
      state.cam.w = Math.max(state.defaultCam.w * 0.16, state.cam.w * 0.72);
      state.cam.h = Math.max(state.defaultCam.h * 0.16, state.cam.h * 0.72);
      renderMapStage();
    };
    byId('btnZoomOut').onclick = function () {
      state.cam.w = Math.min(state.defaultCam.w * 1.8, state.cam.w * 1.38);
      state.cam.h = Math.min(state.defaultCam.h * 1.8, state.cam.h * 1.38);
      renderMapStage();
    };
    byId('btnZoomReset').onclick = function () {
      resetCameraForScale(state.scale);
      renderMapStage();
    };

    // 地圖滑鼠／觸控拖曳與滾輪縮放
    var vp = byId('mapViewport');
    vp.addEventListener('pointerdown', function (e) {
      if (e.target.closest('.wa-map-zoom') || e.target.closest('.wa-mobile-pick-bar')) return;
      state.drag.active = true;
      state.drag.moved = false;
      state.drag.sx = e.clientX;
      state.drag.sy = e.clientY;
      state.drag.scx = state.cam.cx;
      state.drag.scy = state.cam.cy;
      vp.classList.add('dragging');
    });
    window.addEventListener('pointermove', function (e) {
      if (!state.drag.active) return;
      var dx = e.clientX - state.drag.sx;
      var dy = e.clientY - state.drag.sy;
      if (Math.abs(dx) > 4 || Math.abs(dy) > 4) state.drag.moved = true;
      var rect = vp.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) {
        var scaleUnit = Math.max(state.cam.w / rect.width, state.cam.h / rect.height);
        state.cam.cx = state.drag.scx - dx * scaleUnit;
        state.cam.cy = state.drag.scy - dy * scaleUnit;
        renderMapStage();
      }
    });
    window.addEventListener('pointerup', function () {
      state.drag.active = false;
      vp.classList.remove('dragging');
    });
    vp.addEventListener('wheel', function (e) {
      e.preventDefault();
      var factor = e.deltaY > 0 ? 1.18 : 0.84;
      state.cam.w = Math.max(state.defaultCam.w * 0.14, Math.min(state.defaultCam.w * 2.0, state.cam.w * factor));
      state.cam.h = Math.max(state.defaultCam.h * 0.14, Math.min(state.defaultCam.h * 2.0, state.cam.h * factor));
      renderMapStage();
    }, { passive: false });

    // 地圖點擊委派
    byId('svgHost').addEventListener('click', function (e) {
      if (state.drag.moved) return;
      var dngEl = e.target.closest('[data-enter-dungeon]');
      if (dngEl) {
        switchScale('dungeon', false);
        loadNodeInspector('oi:dng_pit:entry', '');
        return;
      }
      var townEl = e.target.closest('[data-town-jump]');
      if (townEl) {
        var tSlug = townEl.getAttribute('data-town-jump');
        var targetPlaza = (tSlug.indexOf('burg_') === 0)
          ? ('oi:' + tSlug + ':yard')
          : ('oi:' + tSlug + ':plaza');
        switchScale('town:' + tSlug, false);
        loadNodeInspector(targetPlaza, '');
        return;
      }
      var nodeEl = e.target.closest('[data-node-id]');
      if (nodeEl) {
        state.selectedActorId = '';
        loadNodeInspector(nodeEl.getAttribute('data-node-id'), '');
        return;
      }
      var cellEl = e.target.closest('[data-cell-id]');
      if (cellEl) {
        state.selectedActorId = '';
        var cid = Number(cellEl.getAttribute('data-cell-id'));
        state.selectedCellId = cid;
        loadNodeInspector('cell:' + cid, '');
      }
    });

    // 左欄與右欄事件委派
    document.body.addEventListener('click', function (e) {
      var pickRes = e.target.closest('[data-pick-actor]');
      if (pickRes) {
        var aid = pickRes.getAttribute('data-pick-actor');
        focusOnActor(aid);
        if (window.innerWidth <= 1040) setMobilePane('right');
        return;
      }
      var jumpAct = e.target.closest('[data-jump-actor]');
      if (jumpAct) {
        focusOnActor(jumpAct.getAttribute('data-jump-actor'));
        if (window.innerWidth <= 1040) setMobilePane('right');
        return;
      }
      var focusCell = e.target.closest('[data-focus-cell]');
      if (focusCell) {
        var fcid = Number(focusCell.getAttribute('data-focus-cell'));
        switchScale('island', false);
        if (state.snap && state.snap.map) {
          (state.snap.map.cells || []).forEach(function (c) {
            if (c.cell_id === fcid) {
              state.cam.cx = c.center[0];
              state.cam.cy = c.center[1];
              state.cam.w = 62;
              state.cam.h = 68;
            }
          });
        }
        loadNodeInspector('cell:' + fcid, '');
        if (window.innerWidth <= 1040) setMobilePane('map');
        return;
      }
      var occEl = e.target.closest('[data-select-occ]');
      if (occEl) {
        state.selectedActorId = occEl.getAttribute('data-select-occ');
        renderInspector();
        return;
      }
      var townJumpBtn = e.target.closest('button[data-town-jump]');
      if (townJumpBtn) {
        var slug2 = townJumpBtn.getAttribute('data-town-jump');
        var targetPlaza2 = (slug2.indexOf('burg_') === 0)
          ? ('oi:' + slug2 + ':yard')
          : ('oi:' + slug2 + ':plaza');
        switchScale('town:' + slug2, false);
        loadNodeInspector(targetPlaza2, '');
        if (window.innerWidth <= 1040) setMobilePane('map');
        return;
      }
      var cellJumpBtn = e.target.closest('button[data-cell-map-jump]');
      if (cellJumpBtn) {
        var jcid = Number(cellJumpBtn.getAttribute('data-cell-map-jump'));
        switchScale('cell:' + jcid, false);
        loadNodeInspector('cell:' + jcid, '');
        if (window.innerWidth <= 1040) setMobilePane('map');
        return;
      }
      var dngRoomBtn = e.target.closest('button[data-dngroom-jump]');
      if (dngRoomBtn) {
        var dnid = dngRoomBtn.getAttribute('data-dngroom-jump');
        switchScale(dnid === 'overview' ? 'dungeon' : ('dngroom:' + dnid), false);
        if (dnid !== 'overview') loadNodeInspector(dnid, '');
        if (window.innerWidth <= 1040) setMobilePane('map');
        return;
      }
      var jumpNodeBtn = e.target.closest('#inspectorBody [data-node-id]');
      if (jumpNodeBtn) {
        var nid2 = jumpNodeBtn.getAttribute('data-node-id');
        loadNodeInspector(nid2, '');
      }
    });
  }

  bindEvents();
  window.addEventListener('hashchange', function () {
    var h = '';
    try { h = decodeURIComponent(location.hash.replace(/^#/, '')); } catch (_) { h = ''; }
    if (h.indexOf('res:') === 0) focusOnActor(h);
    else if (h.indexOf('oi:') === 0 || h.indexOf('cell:') === 0) loadNodeInspector(h, '');
  });
  refreshSnapshot(true);
  setInterval(function () {
    if (state.autoPoll && document.visibilityState !== 'hidden') {
      refreshSnapshot(false);
    }
  }, 5000);
})();

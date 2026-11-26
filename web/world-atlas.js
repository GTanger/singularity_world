/* 奇點世界｜銀鷗島實況全開大地圖・無迷霧水彩活輿圖總控台 */
(function () {
  'use strict';

  var urlToken = new URLSearchParams(location.search).get('token') || '';

  var state = {
    snap: null,
    scale: 'island', // 'island' | 'town:<slug>' | 'cell:<cid>' | 'band:<cid>' | 'dungeon' | 'dngroom:<nid>'
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
    gridCam: { cx: 430, cy: 465, pxPerTile: 0.72 },
    svgCam: { cx: 450, cy: 450, w: 940, h: 940 },
    defaultSvgCam: { cx: 450, cy: 450, w: 940, h: 940 },
    drag: { active: false, moved: false, sx: 0, sy: 0, scx: 0, scy: 0 },
    poiTargets: [],
    overlayHits: []
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

  function apiFetch(path) {
    var headers = {};
    if (urlToken) headers['x-creator-token'] = urlToken;
    return fetch(path, { headers: headers, credentials: 'same-origin' }).then(function (r) {
      return r.json();
    });
  }

  function isDungeonScale(sc) {
    return sc === 'dungeon' || sc.indexOf('dngroom:') === 0;
  }

  function setMobilePane(pane) {
    state.mobilePane = pane;
    var ws = byId('waWorkspace');
    if (ws) ws.setAttribute('data-mobile-pane', pane);
    Array.prototype.forEach.call(document.querySelectorAll('#waMobileTabs button'), function (b) {
      b.classList.toggle('active', b.getAttribute('data-mpane') === pane);
    });
    if (pane === 'map') {
      requestAnimationFrame(renderMapStage);
    }
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

  function dngRoomForId(nid) {
    var rooms = (state.snap && state.snap.map && state.snap.map.dungeon_rooms) || [];
    for (var i = 0; i < rooms.length; i++) {
      if (rooms[i].id === nid && rooms[i].tile && rooms[i].tile.room_url) return rooms[i];
    }
    return null;
  }

  function fitIslandGridCamera() {
    var vp = byId('mapViewport');
    var sub = state.snap && state.snap.map && state.snap.map.substrate;
    var b = (sub && sub.camera_bounds) || (state.snap && state.snap.map && state.snap.map.camera_bounds) || [1360, 628, 1506, 792];
    if (!sub) return { cx: 430, cy: 465, pxPerTile: 0.72 };
    var gx0 = (b[0] - sub.wx0) * sub.scale;
    var gy0 = (b[1] - sub.wy0) * sub.scale;
    var gx1 = (b[2] - sub.wx0) * sub.scale;
    var gy1 = (b[3] - sub.wy0) * sub.scale;
    var spanW = Math.max(40, gx1 - gx0);
    var spanH = Math.max(40, gy1 - gy0);
    var cw = Math.max(240, (vp && vp.clientWidth) || 720);
    var ch = Math.max(240, (vp && vp.clientHeight) || 680);
    var px = Math.max(0.42, Math.min(3.4, Math.min(cw / spanW, ch / spanH) * 0.90));
    return {
      cx: Math.max(0, Math.min(sub.grid_w || 860, (gx0 + gx1) * 0.5)),
      cy: Math.max(0, Math.min(sub.grid_h || 930, (gy0 + gy1) * 0.5)),
      pxPerTile: px
    };
  }

  function resetCameraForScale(scaleKey) {
    var sub = state.snap && state.snap.map && state.snap.map.substrate;
    if (isDungeonScale(scaleKey)) {
      if (scaleKey.indexOf('dngroom:') === 0) {
        state.defaultSvgCam = { cx: 450, cy: 450, w: 920, h: 920 };
      } else {
        state.defaultSvgCam = { cx: 450, cy: 450, w: 940, h: 940 };
      }
      state.svgCam = {
        cx: state.defaultSvgCam.cx,
        cy: state.defaultSvgCam.cy,
        w: state.defaultSvgCam.w,
        h: state.defaultSvgCam.h
      };
      return;
    }

    if (scaleKey === 'island') {
      state.gridCam = fitIslandGridCamera();
    } else if (scaleKey.indexOf('town:') === 0 && sub) {
      var slug = scaleKey.slice(5);
      var tObj = (state.snap.map.town_micro_maps || {})[slug];
      var pt = (sub.node_anchors && (sub.node_anchors['oi:' + slug + ':plaza'] || sub.node_anchors['oi:' + slug + ':yard'])) ||
        (tObj && sub.cell_grid_centers && sub.cell_grid_centers[String(tObj.cell_id)]);
      if (pt) {
        state.gridCam = { cx: pt[0], cy: pt[1], pxPerTile: 12.5 };
      }
    } else if (scaleKey.indexOf('cell:') === 0 && sub) {
      var cid = scaleKey.slice(5);
      var cpt = sub.cell_grid_centers && sub.cell_grid_centers[cid];
      if (cpt) {
        state.gridCam = { cx: cpt[0], cy: cpt[1], pxPerTile: 10.5 };
      }
    } else if (scaleKey.indexOf('band:') === 0 && sub) {
      var bcid = scaleKey.slice(5);
      var bnid = 'oi:cell' + bcid + ':camp';
      var bpt = (sub.node_anchors && sub.node_anchors[bnid]) || (sub.cell_grid_centers && sub.cell_grid_centers[bcid]);
      if (bpt) {
        state.gridCam = { cx: bpt[0], cy: bpt[1], pxPerTile: 10.5 };
      }
    }
  }

  function switchScale(scaleKey, keepCamera) {
    state.scale = scaleKey;
    Array.prototype.forEach.call(document.querySelectorAll('#scaleBar button'), function (btn) {
      btn.classList.toggle('active', btn.getAttribute('data-scale') === scaleKey);
    });
    var cellSel = byId('waCellMapSelect');
    if (cellSel) {
      var selVal = '';
      if (scaleKey.indexOf('cell:') === 0 || scaleKey.indexOf('band:') === 0) selVal = scaleKey;
      else if (scaleKey.indexOf('town:burg_') === 0) selVal = 'cell:' + scaleKey.slice(10);
      cellSel.value = selVal;
      cellSel.classList.toggle('active', !!selVal);
    }
    if (!keepCamera) resetCameraForScale(scaleKey);
    renderMapStage();
  }

  function buildDungeonSvg() {
    var rooms = (state.snap.map && state.snap.map.dungeon_rooms) || [];
    var tiledRooms = rooms.filter(function (r) { return r && r.tile && r.tile.url && r.tile.box; });
    var vb = (state.svgCam.cx - state.svgCam.w / 2).toFixed(1) + ' ' +
             (state.svgCam.cy - state.svgCam.h / 2).toFixed(1) + ' ' +
             state.svgCam.w.toFixed(1) + ' ' + state.svgCam.h.toFixed(1);
    var parts = [
      '<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="試煉坑地底石窟手繪全開輿圖">',
      '<rect x="-800" y="-800" width="2600" height="2600" fill="#11100e" />'
    ];
    if (tiledRooms.length) {
      tiledRooms.forEach(function (r) {
        var b = r.tile.box;
        parts.push('<image href="' + esc(r.tile.url) + '" x="' + b[0] + '" y="' + b[1] + '" width="' + b[2] + '" height="' + b[3] + '" pointer-events="none" />');
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
          '<title>' + esc(r.name) + '｜在場 ' + occ + ' 人</title>' +
          '<circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="68" fill="rgba(0,0,0,0.001)" />' +
          (isSel ? '<circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="18" fill="rgba(201,56,36,0.22)" stroke="#f0b643" stroke-width="2.2" /><circle cx="' + anc[0] + '" cy="' + anc[1] + '" r="7.5" fill="#c93824" stroke="#fdf8ee" stroke-width="2.2" />' : '') +
          (state.showOcc && occ > 0 ? '<circle cx="' + (anc[0] + 34) + '" cy="' + (anc[1] - 28) + '" r="13" fill="#8b3a1e" stroke="#ffd27d" stroke-width="1.8" /><text x="' + (anc[0] + 34) + '" y="' + (anc[1] - 23.5) + '" text-anchor="middle" font-size="12" font-family="sans-serif" font-weight="700" fill="#fffaf0">' + occ + '</text>' : '') +
          '<g transform="translate(' + lp[0] + ',' + lp[1] + ')">' +
          '<rect x="' + (-pw / 2).toFixed(1) + '" y="' + (-ph / 2).toFixed(1) + '" width="' + pw.toFixed(1) + '" height="' + ph + '" rx="6.5" fill="' + bg + '" stroke="' + br + '" stroke-width="' + (isSel ? '2.4' : '1.4') + '" />' +
          '<text x="0" y="' + (fs * 0.36).toFixed(1) + '" text-anchor="middle" font-size="' + fs + '" font-weight="700" fill="' + fg + '">' + esc(txt) + '</text>' +
          '</g></g>'
        );
      });
    }
    parts.push('</svg>');
    return parts.join('');
  }

  function buildDngRoomSvg(rInfo) {
    var roomUrl = rInfo && rInfo.tile && rInfo.tile.room_url;
    if (!roomUrl) return '<div class="wa-empty-hint">尚未配置該石室細部圖</div>';
    var vb = (state.svgCam.cx - state.svgCam.w / 2).toFixed(1) + ' ' +
             (state.svgCam.cy - state.svgCam.h / 2).toFixed(1) + ' ' +
             state.svgCam.w.toFixed(1) + ' ' + state.svgCam.h.toFixed(1);
    return '<svg viewBox="' + vb + '" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="' + esc(rInfo.name) + '">' +
      '<rect x="-600" y="-600" width="2100" height="2100" fill="#12100e" />' +
      '<image href="' + esc(roomUrl) + '" x="0" y="0" width="900" height="900" preserveAspectRatio="xMidYMid slice" />' +
      '</svg>';
  }

  function toScreenPt(gx, gy, cam, viewW, viewH) {
    return [
      viewW * 0.5 + (gx - cam.cx) * cam.pxPerTile,
      viewH * 0.5 + (gy - cam.cy) * cam.pxPerTile
    ];
  }

  function drawAtlasOverlays(canvas, sub, cam, isNight) {
    var rect = canvas.getBoundingClientRect();
    var viewW = Math.max(180, Math.round(rect.width));
    var viewH = Math.max(180, Math.round(rect.height));
    var dpr = Math.min(window.devicePixelRatio || 1, 2);
    var ctx = canvas.getContext('2d');
    var hits = [];
    var m = state.snap.map || {};
    var ns = state.snap.node_summary || {};
    var lw = (state.snap.autonomy_pulse && state.snap.autonomy_pulse.live_world) || {};
    var z = cam.pxPerTile;

    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';

    // 1. 放大街廓視角：屋宅／未貼牌街口直接著色標點；已有書法名牌的店鋪／巷道／城門將人數徽記停靠右上角，絕不壓蓋文字
    if (state.showOcc && z >= 4.2 && sub.node_anchors) {
      var poiById = {};
      (state.poiTargets || []).forEach(function (pt) {
        if (pt.id) poiById[pt.id] = pt;
      });
      var anchors = sub.node_anchors;
      Object.keys(anchors).forEach(function (nid) {
        if (nid.indexOf('oi:dng_') === 0) return;
        var info = ns[nid];
        if (!info) return;
        var isHouse = nid.indexOf(':h') > 0 || info.kind === 'homestead';
        var poi = !isHouse ? poiById[nid] : null;
        if (poi) {
          var px0 = poi.x0, py0 = poi.y0, px1 = poi.x1, py1 = poi.y1;
          hits.push({ x0: px0, y0: py0, x1: px1, y1: py1, nodeId: nid, cellId: info.cell_id || 0 });
          if (info.occupant_count > 0 || state.selectedNodeId === nid) {
            var cntTxt = String(info.occupant_count || 0);
            var bw = Math.max(15, cntTxt.length * 6.5 + 8);
            var bh = 14;
            var bx = poi.x1 - 1, by = poi.y0 - 1;
            ctx.fillStyle = info.has_squatter ? '#7a3b6e'
              : (state.selectedNodeId === nid ? '#c93824' : (info.kind === 'shop' ? '#9e4725' : '#8b3a1e'));
            ctx.strokeStyle = state.selectedNodeId === nid ? '#ffffff' : '#fdf8ee';
            ctx.lineWidth = 1.2;
            ctx.beginPath();
            if (ctx.roundRect) ctx.roundRect(bx - bw / 2, by - bh / 2, bw, bh, 7);
            else ctx.rect(bx - bw / 2, by - bh / 2, bw, bh);
            ctx.fill();
            ctx.stroke();
            if (info.occupant_count > 0) {
              ctx.font = '700 9.5px sans-serif';
              ctx.fillStyle = '#fffaf2';
              ctx.fillText(cntTxt, bx, by + 0.5);
            }
          }
          return;
        }

        var pt = anchors[nid];
        var s = toScreenPt(pt[0], pt[1], cam, viewW, viewH);
        if (s[0] < -20 || s[0] > viewW + 20 || s[1] < -20 || s[1] > viewH + 20) return;
        var isSel = state.selectedNodeId === nid;
        var r = isSel ? 8.0 : (isHouse ? 5.6 : (info.kind === 'shop' ? 5.4 : 4.5));
        var hitR = Math.max(10, r + 4);
        var coveredByPoi = z < 16.0 && (state.poiTargets || []).some(function (p) {
          return s[0] + r > p.x0 && s[0] - r < p.x1 && s[1] + r > p.y0 && s[1] - r < p.y1;
        });
        if (coveredByPoi && !isSel) {
          hits.push({ x0: s[0] - hitR, y0: s[1] - hitR, x1: s[0] + hitR, y1: s[1] + hitR, nodeId: nid, cellId: info.cell_id || 0 });
          return;
        }
        var fill = info.has_squatter ? '#7a3b6e'
          : info.ruined_house ? '#6e655f'
          : info.vacant_for_sale ? '#e0ab38'
          : info.built_homestead ? '#2d8a6b'
          : info.kind === 'shop' ? '#9e4725'
          : isHouse ? '#b85335'
          : '#584b3c';
        ctx.fillStyle = fill;
        ctx.strokeStyle = isSel ? '#ffffff' : (isNight ? '#10151f' : '#26180f');
        ctx.lineWidth = isSel ? 2.2 : 1.15;
        ctx.beginPath();
        ctx.arc(s[0], s[1], r, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();

        if (info.occupant_count > 0 && (isHouse || z >= 7.0)) {
          ctx.font = '700 9px sans-serif';
          ctx.fillStyle = '#fffaf2';
          ctx.fillText(String(info.occupant_count), s[0], s[1] + 0.5);
        }
        var hitR = Math.max(10, r + 4);
        hits.push({ x0: s[0] - hitR, y0: s[1] - hitR, x1: s[0] + hitR, y1: s[1] + hitR, nodeId: nid, cellId: info.cell_id || 0 });
      });
    } else if (state.showOcc && z < 4.2 && sub.cell_grid_centers) {
      // 全島巨觀人數徽記：優先停靠地標名牌右上角，無名牌之荒野格才置於格心並避讓名牌碰撞
      var poiByCell = {};
      (state.poiTargets || []).forEach(function (pt) {
        if (pt.cell_id && (!poiByCell[pt.cell_id] || pt.tier === 'core_town')) {
          poiByCell[pt.cell_id] = pt;
        }
      });
      var boxHits = (state.poiTargets || []).map(function (pt) {
        return [pt.x0 - 2, pt.y0 - 2, pt.x1 + 2, pt.y1 + 2];
      });
      var boxOverlaps = function (x0, y0, x1, y1) {
        for (var bi = 0; bi < boxHits.length; bi++) {
          var b = boxHits[bi];
          if (x0 < b[2] && x1 > b[0] && y0 < b[3] && y1 > b[1]) return true;
        }
        return false;
      };
      (m.cells || []).forEach(function (c) {
        if (!c.resident_count || c.resident_count <= 0) return;
        var cpt = sub.cell_grid_centers[String(c.cell_id)];
        if (!cpt) return;
        var sx, sy;
        var poi = poiByCell[c.cell_id];
        var txt = String(c.resident_count);
        var bw = Math.max(15, txt.length * 6.5 + 8);
        var bh = 14;
        if (poi) {
          sx = poi.x1 - 1;
          sy = poi.y0 - 1;
        } else {
          var sc = toScreenPt(cpt[0], cpt[1], cam, viewW, viewH);
          sx = sc[0];
          sy = sc[1];
          if (boxOverlaps(sx - bw / 2, sy - bh / 2, sx + bw / 2, sy + bh / 2)) {
            sy = sc[1] + 13;
          }
        }
        if (sx < 8 || sx > viewW - 8 || sy < 8 || sy > viewH - 8) return;
        ctx.fillStyle = c.kind === 'settlement' ? '#8b3a1e' : 'rgba(92, 54, 30, 0.92)';
        ctx.strokeStyle = '#fdf8ee';
        ctx.lineWidth = 1.1;
        ctx.beginPath();
        if (ctx.roundRect) ctx.roundRect(sx - bw / 2, sy - bh / 2, bw, bh, 7);
        else ctx.rect(sx - bw / 2, sy - bh / 2, bw, bh);
        ctx.fill();
        ctx.stroke();
        ctx.font = '700 9.5px sans-serif';
        ctx.fillStyle = '#fffaf2';
        ctx.fillText(txt, sx, sy + 0.5);
        boxHits.push([sx - bw / 2, sy - bh / 2, sx + bw / 2, sy + bh / 2]);
        hits.push({ x0: sx - bw / 2 - 2, y0: sy - bh / 2 - 2, x1: sx + bw / 2 + 2, y1: sy + bh / 2 + 2, cellId: c.cell_id });
      });
    }

    // 2. 實體車船（11 具）與一～四刀動態危機標記（斷橋／佔屋／異變首領）
    if (state.showUnbuilt) {
      var vehBoxes = (state.poiTargets || []).map(function (pt) {
        return [pt.x0 - 2, pt.y0 - 2, pt.x1 + 2, pt.y1 + 2];
      });
      (m.transit_vehicles || []).forEach(function (v, vi) {
        var gx = (v.gx != null ? v.gx : 380) + ((vi % 3) - 1) * 2.6;
        var gy = (v.gy != null ? v.gy : 410) + (v.mode === 'barge' ? 4.5 : 3.8);
        var s = toScreenPt(gx, gy, cam, viewW, viewH);
        for (var step = 0; step < 3; step++) {
          var hit = false;
          for (var bi = 0; bi < vehBoxes.length; bi++) {
            var b = vehBoxes[bi];
            if (s[0] + 10 > b[0] && s[0] - 10 < b[2] && s[1] + 10 > b[1] && s[1] - 10 < b[3]) {
              hit = true;
              break;
            }
          }
          if (!hit) break;
          s[1] += 14;
        }
        if (s[0] < -16 || s[0] > viewW + 16 || s[1] < -16 || s[1] > viewH + 16) return;
        ctx.fillStyle = isNight ? 'rgba(24, 28, 36, 0.94)' : 'rgba(255, 250, 240, 0.95)';
        ctx.strokeStyle = v.mode === 'barge' ? '#2c6f9e' : '#9c421a';
        ctx.lineWidth = 1.4;
        ctx.beginPath();
        ctx.arc(s[0], s[1], 9.5, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();
        ctx.font = '11px sans-serif';
        ctx.fillStyle = '#24180d';
        ctx.fillText(v.mode === 'barge' ? '⛵' : '🐎', s[0], s[1] + 0.5);
        vehBoxes.push([s[0] - 10, s[1] - 10, s[0] + 10, s[1] + 10]);
        hits.push({ x0: s[0] - 11, y0: s[1] - 11, x1: s[0] + 11, y1: s[1] + 11, nodeId: v.node_id, cellId: v.cell_id });
      });

      var hy = lw.hydrology || {};
      var wd = lw.wilds || {};
      var drawBadge = function (gx, gy, label, bg, fg, target) {
        var s = toScreenPt(gx, gy, cam, viewW, viewH);
        if (s[0] < 10 || s[0] > viewW - 10 || s[1] < 10 || s[1] > viewH - 10) return;
        ctx.font = '700 10.5px "Noto Sans TC", sans-serif';
        var bw = ctx.measureText(label).width + 10;
        var bh = 18;
        ctx.fillStyle = bg;
        ctx.strokeStyle = fg;
        ctx.lineWidth = 1.1;
        ctx.beginPath();
        if (ctx.roundRect) ctx.roundRect(s[0] - bw / 2, s[1] + 8, bw, bh, 4);
        else ctx.rect(s[0] - bw / 2, s[1] + 8, bw, bh);
        ctx.fill();
        ctx.stroke();
        ctx.fillStyle = fg;
        ctx.fillText(label, s[0], s[1] + 8 + bh / 2 + 0.5);
        target.x0 = s[0] - bw / 2;
        target.y0 = s[1] + 8;
        target.x1 = s[0] + bw / 2;
        target.y1 = s[1] + 8 + bh;
        hits.push(target);
      };

      (hy.washed_out_bridges || []).forEach(function (b) {
        var pt = sub.cell_grid_centers && sub.cell_grid_centers[String(b.cell_id)];
        if (pt) drawBadge(pt[0], pt[1], '🌊斷橋', 'rgba(227,242,253,0.92)', '#1565c0', { cellId: b.cell_id });
      });
      (wd.squatters || []).forEach(function (sq) {
        var pt = sub.node_anchors && sub.node_anchors[sq.node_id];
        if (pt) drawBadge(pt[0], pt[1], '⚠️佔屋', 'rgba(249,230,226,0.94)', '#8a251b', { nodeId: sq.node_id, cellId: sq.cell_id });
      });
      (wd.mutated_bosses || []).forEach(function (mb) {
        var pt = (sub.node_anchors && (sub.node_anchors[mb.node_id] || sub.node_anchors['oi:cell4529:0'])) ||
          (sub.cell_grid_centers && sub.cell_grid_centers[String(mb.cell_id)]);
        if (pt) drawBadge(pt[0], pt[1], '🔮異變', 'rgba(243,229,245,0.94)', '#6a1b9a', { nodeId: mb.node_id, cellId: mb.cell_id });
      });
    }

    ctx.restore();
    state.overlayHits = hits;
  }

  function renderMapStage() {
    if (!state.snap || !state.snap.map) return;
    var canvas = byId('waCanvas');
    var svgHost = byId('svgHost');
    var leg = byId('mapLegend');
    var isNight = document.body.classList.contains('night');

    if (isDungeonScale(state.scale)) {
      if (canvas) canvas.classList.add('hidden');
      if (svgHost) svgHost.classList.remove('hidden');
      if (state.scale.indexOf('dngroom:') === 0) {
        var rInfo = dngRoomForId(state.scale.slice(8));
        svgHost.innerHTML = buildDngRoomSvg(rInfo);
        leg.innerHTML = '<span>🕯️ 試煉坑單室細部圖：' + esc((rInfo && rInfo.name) || '') + '</span>';
      } else {
        svgHost.innerHTML = buildDungeonSvg();
        leg.innerHTML = '<span>⛏ 地底試煉坑全 6 室無迷霧展開（點選石室查看首殺、動態佔巢與異變首領）</span>';
      }
      return;
    }

    if (svgHost) svgHost.classList.add('hidden');
    if (canvas) canvas.classList.remove('hidden');
    var sub = state.snap.map.substrate;
    if (!canvas || !sub || !window.IslandParchmentMap) return;

    var selPt = (sub.node_anchors && sub.node_anchors[state.selectedNodeId]) ||
      (sub.cell_grid_centers && sub.cell_grid_centers[String(state.selectedCellId)]) ||
      null;
    var subView = Object.assign({}, sub, { player_pos: selPt });
    var res = window.IslandParchmentMap.renderIsland(
      canvas,
      { substrate: subView, is_night: isNight },
      state.gridCam,
      {
        isNight: isNight,
        slotIdx: state.snap.clock && state.snap.clock.tide_slot,
        showAllSettlements: state.showUnbuilt
      }
    );
    state.poiTargets = (res && res.poiTargets) || [];
    drawAtlasOverlays(canvas, subView, state.gridCam, isNight);

    var closeZoom = state.gridCam.pxPerTile >= 4.2;
    leg.innerHTML = closeZoom
      ? '<span>🔴 巷弄占住門牌（數字＝在場人數）</span><span>🟡 待售空屋</span><span>🟢 新起宅邸</span><span>🟣 流寇佔屋</span><span>🪨 傾頹</span>'
      : '<span>🗺️ 100% 無迷霧全島水彩活態底盤</span><span>⛺ 紅框＝灰耳營寨</span><span>⛩ 封村遺址</span><span>🐎/⛵ 11 具即時車船</span>';
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
      var sub = state.snap.map && state.snap.map.substrate;
      var pt = (sub && sub.node_anchors && sub.node_anchors[found.current_node_id]) ||
        (sub && sub.cell_grid_centers && sub.cell_grid_centers[String(found.cell_id)]);
      if (pt) {
        state.gridCam = { cx: pt[0], cy: pt[1], pxPerTile: 13.0 };
      }
      var tSlug = townSlugForCell(found.cell_id);
      switchScale(tSlug ? ('town:' + tSlug) : 'island', true);
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
    var cells = (state.snap.map.cells || []).slice().sort(function (a, b) { return a.cell_id - b.cell_id; });
    var burgs = cells.filter(function (c) { return c.kind === 'rural_settlement'; });
    var ruins = cells.filter(function (c) { return c.kind === 'plague_ruin'; });
    var bands = ((state.snap.autonomy_pulse && state.snap.autonomy_pulse.live_world && state.snap.autonomy_pulse.live_world.wilds && state.snap.autonomy_pulse.live_world.wilds.bands) || []).slice();
    function grpHtml(label, arr, prefix) {
      if (!arr.length) return '';
      return '<optgroup label="' + esc(label) + '">' + arr.map(function (c) {
        var cid = c.cell_id;
        var title = c.title || c.band_name_zh || ('#' + cid);
        return '<option value="' + prefix + cid + '">#' + cid + ' ' + esc(title) + '</option>';
      }).join('') + '</optgroup>';
    }
    sel.innerHTML =
      '<option value="">🏘️ 鄉野聚落／封村／營寨快選…</option>' +
      grpHtml('🏘️ 鄉野聚落（' + burgs.length + ' 處）', burgs, 'cell:') +
      grpHtml('⛩️ 南岸封村遺址（' + ruins.length + ' 處）', ruins, 'cell:') +
      grpHtml('⛺ 灰耳部族營寨（' + bands.length + ' 處）', bands, 'band:') +
      '<optgroup label="🌲 試煉坑"><option value="cell:4529">#4529 黑煤鎮塌陷坑舊址</option></optgroup>';
    sel.dataset.filled = '1';
  }

  function pointInPolygon(px, py, poly) {
    if (!poly || poly.length < 3) return false;
    var inside = false;
    for (var i = 0, j = poly.length - 1; i < poly.length; j = i++) {
      var xi = poly[i][0], yi = poly[i][1];
      var xj = poly[j][0], yj = poly[j][1];
      if (((yi > py) !== (yj > py)) && (px < ((xj - xi) * (py - yi)) / ((yj - yi) || 1e-12) + xi)) {
        inside = !inside;
      }
    }
    return inside;
  }

  function handleCanvasClick(lx, ly) {
    // 1. 最上層動態徽記／實體車船／屋宅點
    for (var i = state.overlayHits.length - 1; i >= 0; i--) {
      var h = state.overlayHits[i];
      if (lx >= h.x0 && lx <= h.x1 && ly >= h.y0 && ly <= h.y1) {
        state.selectedActorId = '';
        if (h.nodeId) loadNodeInspector(h.nodeId, '');
        else if (h.cellId) loadNodeInspector('cell:' + h.cellId, '');
        return;
      }
    }
    // 2. 地標書法名牌
    for (var pIdx = state.poiTargets.length - 1; pIdx >= 0; pIdx--) {
      var p = state.poiTargets[pIdx];
      if (lx >= p.x0 && lx <= p.x1 && ly >= p.y0 && ly <= p.y1) {
        state.selectedActorId = '';
        loadNodeInspector(p.id, '');
        return;
      }
    }
    var sub = state.snap && state.snap.map && state.snap.map.substrate;
    var canvas = byId('waCanvas');
    if (!sub || !canvas) return;
    var rect = canvas.getBoundingClientRect();
    var gx = state.gridCam.cx + (lx - rect.width * 0.5) / state.gridCam.pxPerTile;
    var gy = state.gridCam.cy + (ly - rect.height * 0.5) / state.gridCam.pxPerTile;

    // 3. 近景時吸附最近空間節點錨點
    if (state.gridCam.pxPerTile >= 4.2 && sub.node_anchors) {
      var bestNid = '';
      var bestD = 18 / state.gridCam.pxPerTile;
      Object.keys(sub.node_anchors).forEach(function (nid) {
        if (nid.indexOf('oi:dng_') === 0) return;
        var pt = sub.node_anchors[nid];
        var d = Math.hypot(pt[0] - gx, pt[1] - gy);
        if (d < bestD) {
          bestD = d;
          bestNid = nid;
        }
      });
      if (bestNid) {
        state.selectedActorId = '';
        loadNodeInspector(bestNid, '');
        return;
      }
    }

    // 4. 反推所屬 Voronoi 地塊
    var wx = sub.wx0 + gx / sub.scale;
    var wy = sub.wy0 + gy / sub.scale;
    var cells = (state.snap.map && state.snap.map.cells) || [];
    for (var ci = 0; ci < cells.length; ci++) {
      if (pointInPolygon(wx, wy, cells[ci].polygon)) {
        state.selectedActorId = '';
        state.selectedCellId = cells[ci].cell_id;
        loadNodeInspector('cell:' + cells[ci].cell_id, '');
        return;
      }
    }
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
        if (rawHash && (rawHash.indexOf('oi:') === 0 || rawHash.indexOf('cell:') === 0)) {
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
    byId('chkUnbuilt').onchange = function (e) { state.showUnbuilt = e.target.checked; renderMapStage(); };
    byId('chkOcc').onchange = function (e) { state.showOcc = e.target.checked; renderMapStage(); };
    byId('chkAutoPoll').onchange = function (e) { state.autoPoll = e.target.checked; };
    byId('btnRefresh').onclick = function () { refreshSnapshot(false); };
    byId('btnTheme').onclick = function () { document.body.classList.toggle('night'); renderMapStage(); };

    Array.prototype.forEach.call(document.querySelectorAll('#waMobileTabs button'), function (btn) {
      btn.onclick = function () { setMobilePane(btn.getAttribute('data-mpane')); };
    });

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

    Array.prototype.forEach.call(document.querySelectorAll('#scaleBar button'), function (btn) {
      btn.onclick = function () {
        var sc = btn.getAttribute('data-scale');
        switchScale(sc, false);
        if (sc === 'dungeon') loadNodeInspector('oi:dng_pit:entry', '');
        else if (sc.indexOf('dngroom:') === 0) loadNodeInspector(sc.slice(8), '');
        else if (sc.indexOf('town:') === 0) loadNodeInspector('town:' + sc.slice(5), '');
      };
    });
    var cellSelEl = byId('waCellMapSelect');
    if (cellSelEl) {
      cellSelEl.onchange = function () {
        var v = cellSelEl.value;
        if (!v) return;
        switchScale(v, false);
        if (v.indexOf('cell:') === 0) loadNodeInspector(v, '');
        else if (v.indexOf('band:') === 0) loadNodeInspector('oi:cell' + v.slice(5) + ':camp', '');
      };
    }

    function zoomBy(factor) {
      if (isDungeonScale(state.scale)) {
        state.svgCam.w = Math.max(state.defaultSvgCam.w * 0.16, Math.min(state.defaultSvgCam.w * 1.9, state.svgCam.w / factor));
        state.svgCam.h = Math.max(state.defaultSvgCam.h * 0.16, Math.min(state.defaultSvgCam.h * 1.9, state.svgCam.h / factor));
      } else {
        state.gridCam.pxPerTile = Math.max(0.42, Math.min(24.0, state.gridCam.pxPerTile * factor));
      }
      renderMapStage();
    }
    byId('btnZoomIn').onclick = function () { zoomBy(1.38); };
    byId('btnZoomOut').onclick = function () { zoomBy(1 / 1.38); };
    byId('btnZoomReset').onclick = function () {
      resetCameraForScale(state.scale);
      renderMapStage();
    };

    var vp = byId('mapViewport');
    vp.addEventListener('pointerdown', function (e) {
      if (e.target.closest('.wa-map-zoom') || e.target.closest('.wa-mobile-pick-bar')) return;
      state.drag.active = true;
      state.drag.moved = false;
      state.drag.sx = e.clientX;
      state.drag.sy = e.clientY;
      if (isDungeonScale(state.scale)) {
        state.drag.scx = state.svgCam.cx;
        state.drag.scy = state.svgCam.cy;
      } else {
        state.drag.scx = state.gridCam.cx;
        state.drag.scy = state.gridCam.cy;
      }
      vp.classList.add('dragging');
    });
    window.addEventListener('pointermove', function (e) {
      if (!state.drag.active) return;
      var dx = e.clientX - state.drag.sx;
      var dy = e.clientY - state.drag.sy;
      if (Math.abs(dx) > 4 || Math.abs(dy) > 4) state.drag.moved = true;
      if (isDungeonScale(state.scale)) {
        var rect = vp.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          var su = Math.max(state.svgCam.w / rect.width, state.svgCam.h / rect.height);
          state.svgCam.cx = state.drag.scx - dx * su;
          state.svgCam.cy = state.drag.scy - dy * su;
        }
      } else {
        state.gridCam.cx = Math.max(0, Math.min(860, state.drag.scx - dx / state.gridCam.pxPerTile));
        state.gridCam.cy = Math.max(0, Math.min(930, state.drag.scy - dy / state.gridCam.pxPerTile));
      }
      renderMapStage();
    });
    window.addEventListener('pointerup', function () {
      state.drag.active = false;
      vp.classList.remove('dragging');
    });
    vp.addEventListener('wheel', function (e) {
      e.preventDefault();
      zoomBy(e.deltaY > 0 ? (1 / 1.18) : 1.18);
    }, { passive: false });

    byId('waCanvas').addEventListener('click', function (e) {
      if (state.drag.moved) return;
      var rect = e.currentTarget.getBoundingClientRect();
      handleCanvasClick(e.clientX - rect.left, e.clientY - rect.top);
    });

    byId('svgHost').addEventListener('click', function (e) {
      if (state.drag.moved) return;
      var nodeEl = e.target.closest('[data-node-id]');
      if (nodeEl) {
        state.selectedActorId = '';
        loadNodeInspector(nodeEl.getAttribute('data-node-id'), '');
      }
    });

    document.body.addEventListener('click', function (e) {
      var pickRes = e.target.closest('[data-pick-actor]');
      if (pickRes) {
        focusOnActor(pickRes.getAttribute('data-pick-actor'));
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
        switchScale('cell:' + fcid, false);
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
        switchScale('town:' + slug2, false);
        loadNodeInspector('town:' + slug2, '');
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
      var jumpNodeBtn = e.target.closest('#inspectorBody [data-node-id], #pulseBody [data-node-id]');
      if (jumpNodeBtn) {
        var nid2 = jumpNodeBtn.getAttribute('data-node-id');
        var sub = state.snap && state.snap.map && state.snap.map.substrate;
        if (nid2.indexOf('oi:dng_') === 0) {
          switchScale('dungeon', false);
        } else if (sub && sub.node_anchors && sub.node_anchors[nid2]) {
          var pt = sub.node_anchors[nid2];
          state.gridCam = { cx: pt[0], cy: pt[1], pxPerTile: Math.max(state.gridCam.pxPerTile, 11.5) };
          if (isDungeonScale(state.scale)) state.scale = 'island';
        }
        loadNodeInspector(nid2, '');
        if (window.innerWidth <= 1040) setMobilePane('map');
      }
    });

    if (typeof ResizeObserver === 'function') {
      new ResizeObserver(function () {
        if (state.scale === 'island') state.gridCam = fitIslandGridCamera();
        renderMapStage();
      }).observe(vp);
    }
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

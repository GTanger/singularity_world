/* 《奇點世界・銀鷗外島》零外部圖檔・全島單一無縫水彩古卷活態底盤 Canvas2D 輿圖引擎 */
(function () {
  'use strict';

  var CELL = {
    SEA: 0,
    RIVER: 1,
    LAND: 2,
    MOUNTAIN: 3,
    TREE: 4,
    FARM: 5,
    FOG: 6,
    REEF_SAND: 7
  };

  var ROAD = {
    NONE: 0,
    TRACK: 1,
    BRIDGE: 2,
    COBBLE: 3,
    WALL: 4
  };

  var BLDG = {
    NONE: 0,
    ROOF: 1,
    TOWER: 2,
    RUIN: 3,
    VACANT: 4
  };

  var decodeCache = {
    terrainKey: '',
    overlayKey: '',
    W: 860,
    H: 930,
    terrain: null,
    road: null,
    bldg: null
  };

  var paperCache = { key: '', canvas: null };
  var foliageCache = { key: '', canvas: null };
  var macroCache = { key: '', canvas: null };
  var roofTexCache = new Map();

  function hash2(x, y) {
    var n = (Math.imul(x | 0, 374761393) + Math.imul(y | 0, 668265263)) ^ 0x5bf03635;
    n = Math.imul(n ^ (n >>> 13), 1274126177);
    return ((n ^ (n >>> 16)) >>> 0) / 4294967295;
  }

  function decodeRleB64(b64, expectedLen) {
    var bin = atob(b64 || '');
    var out = new Uint8Array(expectedLen);
    var w = 0;
    for (var i = 0; i + 1 < bin.length && w < expectedLen; i += 2) {
      var run = bin.charCodeAt(i);
      var val = bin.charCodeAt(i + 1);
      var end = Math.min(expectedLen, w + run);
      out.fill(val, w, end);
      w = end;
    }
    return out;
  }

  function ensureDecodedSubstrate(sub) {
    if (!sub || !sub.terrain_rle_b64 || !sub.overlay_rle_b64) return null;
    var W = sub.grid_w || 860;
    var H = sub.grid_h || 930;
    var expected = W * H;
    if (
      decodeCache.terrainKey === sub.terrain_rle_b64 &&
      decodeCache.overlayKey === sub.overlay_rle_b64 &&
      decodeCache.terrain
    ) {
      return decodeCache;
    }
    var terrain = decodeRleB64(sub.terrain_rle_b64, expected);
    var ov = decodeRleB64(sub.overlay_rle_b64, expected);
    var road = new Uint8Array(expected);
    var bldg = new Uint8Array(expected);
    for (var i = 0; i < expected; i++) {
      road[i] = ov[i] & 0x07;
      bldg[i] = (ov[i] >> 3) & 0x07;
    }
    decodeCache.terrainKey = sub.terrain_rle_b64;
    decodeCache.overlayKey = sub.overlay_rle_b64;
    decodeCache.W = W;
    decodeCache.H = H;
    decodeCache.terrain = terrain;
    decodeCache.road = road;
    decodeCache.bldg = bldg;
    roofTexCache.clear();
    return decodeCache;
  }

  function resolveTimeMode(isNightUi, slotIdx) {
    if (isNightUi) return 'night';
    if (typeof slotIdx === 'number' && (slotIdx >= 30 && slotIdx <= 38)) return 'dusk';
    if (typeof slotIdx === 'number' && (slotIdx >= 10 && slotIdx <= 12)) return 'dusk';
    return 'day';
  }

  function getLightVector(timeMode) {
    if (timeMode === 'dusk') return { dx: 0.90, dy: 0.43, sx: 3.4, sy: 1.7 };
    if (timeMode === 'night') return { dx: 0.46, dy: 0.89, sx: 2.0, sy: 2.4 };
    return { dx: 0.72, dy: 0.69, sx: 2.2, sy: 2.4 };
  }

  function getPalette(timeMode) {
    var roofSetsDay = [
      { slopes: [[196, 99, 68], [180, 86, 57], [154, 70, 45], [126, 54, 34]], ridge: [232, 162, 124] },
      { slopes: [[124, 134, 146], [110, 120, 132], [92, 102, 114], [74, 82, 92]], ridge: [184, 194, 204] },
      { slopes: [[184, 124, 78], [168, 108, 66], [144, 88, 52], [118, 70, 40]], ridge: [226, 176, 128] },
      { slopes: [[168, 78, 56], [152, 68, 48], [128, 54, 38], [104, 42, 30]], ridge: [212, 140, 112] },
      { slopes: [[204, 158, 74], [186, 140, 62], [158, 116, 48], [128, 92, 36]], ridge: [238, 198, 118] }
    ];
    var roofSetsNight = [
      { slopes: [[126, 72, 58], [108, 60, 48], [88, 48, 40], [68, 38, 33]], ridge: [198, 136, 98] },
      { slopes: [[82, 92, 108], [70, 80, 95], [58, 66, 80], [46, 54, 66]], ridge: [148, 158, 174] },
      { slopes: [[118, 86, 62], [102, 74, 52], [84, 58, 42], [66, 46, 34]], ridge: [190, 146, 104] },
      { slopes: [[110, 60, 52], [94, 50, 43], [76, 40, 36], [58, 30, 28]], ridge: [182, 120, 92] },
      { slopes: [[138, 108, 54], [118, 92, 46], [96, 74, 36], [74, 56, 28]], ridge: [196, 162, 92] }
    ];
    if (timeMode === 'night') {
      return {
        paper: '#2d3544',
        fogFill: '#232934',
        fogStroke: 'rgba(130, 146, 172, 0.34)',
        shoreSand: 'rgba(62, 72, 86, 0.48)',
        shorePool: 'rgba(10, 15, 24, 0.30)',
        reefFill: '#455159',
        sea: '#1b2b3b',
        seaDeep: '#121e2d',
        seaRim: [
          ['rgba(94, 152, 184, 0.08)', 18],
          ['rgba(122, 178, 206, 0.11)', 10],
          ['rgba(150, 204, 226, 0.13)', 5],
          ['rgba(180, 224, 242, 0.16)', 2.2]
        ],
        seaRipple: 'rgba(170, 214, 236, 0.28)',
        waveInk: 'rgba(140, 186, 212, 0.34)',
        forestOlive: '#2b433d',
        forestDeep: '#1b3036',
        forestRimLit: 'rgba(178, 218, 224, 0.24)',
        forestRimShade: 'rgba(6, 12, 16, 0.52)',
        forestShadow: 'rgba(6, 9, 14, 0.52)',
        forestInk: '#0e1718',
        forestInnerArc: 'rgba(136, 184, 178, 0.24)',
        mtnBase: 'rgba(26, 33, 44, 0.56)',
        mtnLit: '#4c5b6e',
        mtnLitBot: '#2e3746',
        mtnShade: '#1c2430',
        mtnInk: '#121822',
        wheats: ['#403d34', '#37352e', '#474338', '#312f29'],
        furrow: 'rgba(176, 168, 134, 0.22)',
        roadHalo: 'rgba(10, 14, 22, 0.26)',
        roadInk: '#1a1e26',
        roadFill: '#47423a',
        bridge: '#694e38',
        cobbleFill: '#545c6a',
        cobbleLit: '#687180',
        wallFace: '#68707d',
        wallShade: '#4a515d',
        wallBase: '#333a44',
        wallBeam: '#1f232c',
        roofSets: roofSetsNight,
        courtyard: '#3b4350',
        courtyardSeam: 'rgba(195, 180, 150, 0.20)',
        towerLit: '#707782',
        towerShade: '#3e454f',
        towerRim: '#a4adb8',
        towerCapLit: '#8c7e70',
        towerCapShade: '#4f463f',
        shadow: 'rgba(6, 9, 15, 0.56)',
        ink: '#10151f',
        shoreInk: '#0d141d',
        labelBg: 'rgba(18, 22, 30, 0.88)',
        labelInk: '#f7ecda',
        labelBorder: 'rgba(218, 172, 86, 0.45)',
        lantern: 1
      };
    }

    var isDusk = timeMode === 'dusk';
    return {
      paper: isDusk ? '#ead7b3' : '#ece1c9',
      fogFill: isDusk ? '#dfd0b0' : '#e2d6bc',
      fogStroke: 'rgba(128, 108, 82, 0.44)',
      shoreSand: isDusk ? 'rgba(222, 184, 128, 0.46)' : 'rgba(218, 198, 158, 0.52)',
      shorePool: isDusk ? 'rgba(128, 86, 50, 0.14)' : 'rgba(86, 70, 50, 0.13)',
      reefFill: isDusk ? '#c8b384' : '#c9be9a',
      sea: isDusk ? '#6d8a98' : '#7f9aa4',
      seaDeep: isDusk ? '#526a7a' : '#65818d',
      seaRim: isDusk
        ? [['rgba(246, 202, 146, 0.10)', 18], ['rgba(252, 214, 162, 0.13)', 10], ['rgba(255, 226, 178, 0.16)', 5], ['rgba(74, 56, 54, 0.18)', 2]]
        : [['rgba(182, 214, 212, 0.10)', 18], ['rgba(198, 226, 224, 0.13)', 10], ['rgba(218, 238, 234, 0.16)', 5], ['rgba(44, 62, 70, 0.16)', 2]],
      seaRipple: isDusk ? 'rgba(255, 236, 202, 0.44)' : 'rgba(246, 251, 252, 0.46)',
      waveInk: isDusk ? 'rgba(72, 60, 62, 0.42)' : 'rgba(48, 68, 76, 0.42)',
      forestOlive: isDusk ? '#8d8f58' : '#8b9662',
      forestDeep: isDusk ? '#37585b' : '#396260',
      forestRimLit: isDusk ? 'rgba(248, 212, 130, 0.40)' : 'rgba(220, 230, 162, 0.32)',
      forestRimShade: isDusk ? 'rgba(34, 24, 28, 0.34)' : 'rgba(26, 36, 26, 0.28)',
      forestShadow: isDusk ? 'rgba(44, 26, 22, 0.34)' : 'rgba(32, 26, 16, 0.30)',
      forestInk: '#272019',
      forestInnerArc: 'rgba(34, 30, 20, 0.28)',
      mtnBase: isDusk ? 'rgba(186, 152, 122, 0.26)' : 'rgba(176, 164, 144, 0.22)',
      mtnLit: isDusk ? '#eedec4' : '#ebe4d3',
      mtnLitBot: isDusk ? '#dfd0b4' : '#dfd4be',
      mtnShade: isDusk ? '#8f7b72' : '#968b7a',
      mtnInk: '#352b22',
      wheats: isDusk ? ['#deb472', '#d2a666', '#dfb87c', '#c99c62'] : ['#e0c78b', '#d4b87c', '#e4ce97', '#cab177'],
      furrow: 'rgba(110, 80, 42, 0.42)',
      roadHalo: 'rgba(136, 108, 72, 0.14)',
      roadInk: '#544230',
      roadFill: isDusk ? '#dcbe8d' : '#dcc9a3',
      bridge: '#8d5d39',
      cobbleFill: isDusk ? '#b8ad96' : '#b7b0a0',
      cobbleLit: isDusk ? '#dfd2b8' : '#dcd4c2',
      wallFace: isDusk ? '#e6d3b4' : '#ece2ce',
      wallShade: isDusk ? '#cbb38e' : '#d2c5ad',
      wallBase: '#8f8676',
      wallBeam: '#4f3a29',
      roofSets: roofSetsDay,
      courtyard: isDusk ? '#cfc0a2' : '#d2cab6',
      courtyardSeam: 'rgba(84, 68, 50, 0.28)',
      towerLit: isDusk ? '#a89278' : '#9f937f',
      towerShade: isDusk ? '#68594e' : '#6c6152',
      towerRim: isDusk ? '#e0cab0' : '#d4c8b2',
      towerCapLit: isDusk ? '#b88266' : '#a87d65',
      towerCapShade: '#694a3a',
      shadow: isDusk ? 'rgba(42, 24, 22, 0.34)' : 'rgba(32, 24, 15, 0.32)',
      ink: '#2a1f17',
      shoreInk: '#342b22',
      labelBg: 'rgba(248, 240, 222, 0.90)',
      labelInk: '#24180d',
      labelBorder: 'rgba(110, 44, 20, 0.34)',
      lantern: isDusk ? 0.52 : 0
    };
  }

  function ensurePaperSurface(width, height, pal, timeMode) {
    var key = Math.round(width) + 'x' + Math.round(height) + ':' + timeMode;
    if (paperCache.key === key && paperCache.canvas) return paperCache.canvas;
    var c = paperCache.canvas || document.createElement('canvas');
    c.width = Math.max(1, Math.round(width));
    c.height = Math.max(1, Math.round(height));
    var pctx = c.getContext('2d');
    pctx.fillStyle = pal.paper;
    pctx.fillRect(0, 0, c.width, c.height);

    var lw = 128, lh = 96;
    var low = document.createElement('canvas');
    low.width = lw; low.height = lh;
    var lctx = low.getContext('2d');
    var limg = lctx.createImageData(lw, lh);
    var seed = 9187;
    function rnd() { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return (seed >>> 16) / 65536; }
    var isDark = timeMode === 'night';
    for (var y = 0; y < lh; y++) {
      for (var x = 0; x < lw; x++) {
        var o = (y * lw + x) * 4;
        var wv = Math.sin(x * 0.22 + y * 0.13) * 0.4 + Math.cos(x * 0.11 - y * 0.27) * 0.4 + (rnd() - 0.5) * 0.6;
        if (wv >= 0) {
          limg.data[o] = isDark ? 225 : 255;
          limg.data[o + 1] = isDark ? 210 : 248;
          limg.data[o + 2] = isDark ? 185 : 230;
          limg.data[o + 3] = Math.min(26, Math.round(wv * (isDark ? 14 : 21)));
        } else {
          limg.data[o] = isDark ? 8 : 112;
          limg.data[o + 1] = isDark ? 10 : 86;
          limg.data[o + 2] = isDark ? 14 : 52;
          limg.data[o + 3] = Math.min(22, Math.round(-wv * (isDark ? 15 : 14)));
        }
      }
    }
    lctx.putImageData(limg, 0, 0);
    pctx.drawImage(low, 0, 0, c.width, c.height);

    pctx.strokeStyle = isDark ? 'rgba(220, 210, 190, 0.03)' : 'rgba(105, 82, 52, 0.028)';
    pctx.lineWidth = 0.7;
    pctx.beginPath();
    for (var ly = 2; ly < c.height; ly += 5) {
      pctx.moveTo(0, ly);
      pctx.lineTo(c.width, ly);
    }
    pctx.stroke();

    var cx = c.width * 0.5, cy = c.height * 0.5;
    var rad = Math.hypot(cx, cy);
    var vig = pctx.createRadialGradient(cx, cy, rad * 0.52, cx, cy, rad * 1.03);
    vig.addColorStop(0, 'rgba(0,0,0,0)');
    vig.addColorStop(1, isDark ? 'rgba(6, 8, 12, 0.30)' : 'rgba(92, 68, 40, 0.13)');
    pctx.fillStyle = vig;
    pctx.fillRect(0, 0, c.width, c.height);

    paperCache.key = key;
    paperCache.canvas = c;
    return c;
  }

  function ensureFoliagePattern(pal, timeMode) {
    var key = timeMode + ':' + pal.forestOlive;
    if (foliageCache.key === key && foliageCache.canvas) return foliageCache.canvas;
    var tile = 96;
    var c = foliageCache.canvas || document.createElement('canvas');
    c.width = tile; c.height = tile;
    var fctx = c.getContext('2d');
    fctx.clearRect(0, 0, tile, tile);
    var seed = 7331;
    function rnd() { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return (seed >>> 16) / 65536; }
    var isNight = timeMode === 'night';
    var inkStroke = isNight ? 'rgba(10, 18, 20, 0.26)' : 'rgba(26, 36, 22, 0.16)';
    var litStroke = isNight ? 'rgba(170, 212, 216, 0.14)' : 'rgba(244, 246, 198, 0.18)';

    for (var k = 0; k < 44; k++) {
      var x = rnd() * tile, y = rnd() * tile;
      var w = 3.0 + rnd() * 2.4;
      for (var ox = -tile; ox <= tile; ox += tile) {
        for (var oy = -tile; oy <= tile; oy += tile) {
          var px = x + ox, py = y + oy;
          if (px + w < 0 || px - w > tile || py + w < 0 || py - w > tile) continue;
          fctx.strokeStyle = inkStroke;
          fctx.lineWidth = 0.85;
          fctx.beginPath();
          fctx.moveTo(px - w, py + 0.5);
          fctx.quadraticCurveTo(px - w * 0.45, py - w * 0.72, px, py - 0.3);
          fctx.quadraticCurveTo(px + w * 0.45, py - w * 0.72, px + w, py + 0.5);
          fctx.stroke();

          fctx.strokeStyle = litStroke;
          fctx.lineWidth = 0.65;
          fctx.beginPath();
          fctx.arc(px - 0.6, py - w * 0.35, w * 0.28, Math.PI * 1.05, Math.PI * 1.85);
          fctx.stroke();
        }
      }
    }
    foliageCache.key = key;
    foliageCache.canvas = c;
    return c;
  }

  function extractClosedLoops(field, fw, fh, threshold) {
    var segs = [];
    function valAt(x, y) {
      return (x < 0 || x >= fw || y < 0 || y >= fh) ? 0 : field[y * fw + x];
    }
    function interp(a, b) {
      var den = b - a;
      return Math.abs(den) < 1e-5 ? 0.5 : Math.max(0.05, Math.min(0.95, (threshold - a) / den));
    }
    for (var y = -1; y < fh; y++) {
      for (var x = -1; x < fw; x++) {
        var v0 = valAt(x, y), v1 = valAt(x + 1, y);
        var v2 = valAt(x + 1, y + 1), v3 = valAt(x, y + 1);
        var mask = (v0 >= threshold ? 8 : 0) | (v1 >= threshold ? 4 : 0) | (v2 >= threshold ? 2 : 0) | (v3 >= threshold ? 1 : 0);
        if (mask === 0 || mask === 15) continue;

        var top = [x + 0.5 + interp(v0, v1), y + 0.5];
        var right = [x + 1.5, y + 0.5 + interp(v1, v2)];
        var bottom = [x + 0.5 + interp(v3, v2), y + 1.5];
        var left = [x + 0.5, y + 0.5 + interp(v0, v3)];
        var centerAbove = (v0 + v1 + v2 + v3) * 0.25 >= threshold;

        switch (mask) {
          case 1: case 14: segs.push([left, bottom]); break;
          case 2: case 13: segs.push([bottom, right]); break;
          case 3: case 12: segs.push([left, right]); break;
          case 4: case 11: segs.push([top, right]); break;
          case 5:
            if (centerAbove) { segs.push([left, top]); segs.push([bottom, right]); }
            else { segs.push([left, bottom]); segs.push([top, right]); }
            break;
          case 6: case 9: segs.push([top, bottom]); break;
          case 7: case 8: segs.push([left, top]); break;
          case 10:
            if (centerAbove) { segs.push([left, bottom]); segs.push([top, right]); }
            else { segs.push([left, top]); segs.push([bottom, right]); }
            break;
        }
      }
    }

    function keyOf(p) { return Math.round(p[0] * 1000) + ',' + Math.round(p[1] * 1000); }
    var adj = new Map();
    for (var sIdx = 0; sIdx < segs.length; sIdx++) {
      var k0 = keyOf(segs[sIdx][0]);
      var k1 = keyOf(segs[sIdx][1]);
      if (!adj.has(k0)) adj.set(k0, []);
      adj.get(k0).push([sIdx, 0]);
      if (!adj.has(k1)) adj.set(k1, []);
      adj.get(k1).push([sIdx, 1]);
    }

    var used = new Uint8Array(segs.length);
    var loops = [];
    for (var i = 0; i < segs.length; i++) {
      if (used[i]) continue;
      used[i] = 1;
      var loop = [segs[i][0], segs[i][1]];
      var curKey = keyOf(segs[i][1]);
      for (var guard = 0; guard < segs.length; guard++) {
        var list = adj.get(curKey);
        if (!list) break;
        var moved = false;
        for (var j = 0; j < list.length; j++) {
          var si = list[j][0], ei = list[j][1];
          if (used[si]) continue;
          used[si] = 1;
          var nextPt = ei === 0 ? segs[si][1] : segs[si][0];
          loop.push(nextPt);
          curKey = keyOf(nextPt);
          moved = true;
          break;
        }
        if (!moved) break;
      }
      if (loop.length >= 3) loops.push(loop);
    }
    return loops;
  }

  function chaikinSmoothLoop(pts, passes) {
    var cur = pts;
    for (var p = 0; p < passes; p++) {
      var next = [];
      for (var i = 0; i < cur.length; i++) {
        var a = cur[i], b = cur[(i + 1) % cur.length];
        next.push([a[0] * 0.75 + b[0] * 0.25, a[1] * 0.75 + b[1] * 0.25]);
        next.push([a[0] * 0.25 + b[0] * 0.75, a[1] * 0.25 + b[1] * 0.75]);
      }
      cur = next;
    }
    return cur;
  }

  function buildContourPath(loops, ox, oy, cw, ch, passes) {
    var path = new Path2D();
    for (var li = 0; li < loops.length; li++) {
      var loop = passes ? chaikinSmoothLoop(loops[li], passes) : loops[li];
      path.moveTo((ox + loop[0][0]) * cw, (oy + loop[0][1]) * ch);
      for (var i = 1; i < loop.length; i++) {
        path.lineTo((ox + loop[i][0]) * cw, (oy + loop[i][1]) * ch);
      }
      path.closePath();
    }
    return path;
  }

  function sampleGrad(field, fw, fh, gx, gy) {
    var eps = 0.25;
    function at(sx, sy) {
      var cx = Math.max(0, Math.min(fw - 1.001, sx));
      var cy = Math.max(0, Math.min(fh - 1.001, sy));
      var x0 = Math.floor(cx), y0 = Math.floor(cy);
      var tx = cx - x0, ty = cy - y0;
      var v00 = field[y0 * fw + x0], v10 = field[y0 * fw + x0 + 1];
      var v01 = field[(y0 + 1) * fw + x0], v11 = field[(y0 + 1) * fw + x0 + 1];
      return (v00 * (1 - tx) + v10 * tx) * (1 - ty) + (v01 * (1 - tx) + v11 * tx) * ty;
    }
    return [at(gx + eps, gy) - at(gx - eps, gy), at(gx, gy + eps) - at(gx, gy - eps)];
  }

  function fnvHash(str) {
    var h = 2166136261;
    for (var i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619);
    }
    return (h >>> 0).toString(36);
  }

  /* ═══ 全島巨觀水彩快取圖層（縮放平移零重算） ═══ */
  function renderOutlineCells(ctx, m, pal, tileScale) {
    var outlines = (m && m.outline_cells) || [];
    if (!outlines.length || !m.substrate) return;
    var wx0 = m.substrate.wx0, wy0 = m.substrate.wy0, gs = m.substrate.scale;
    ctx.save();
    ctx.strokeStyle = pal.fogStroke;
    ctx.lineWidth = tileScale > 1.0 ? Math.max(1.0, Math.min(2.2, tileScale * 0.14)) : 0.8;
    ctx.setLineDash(tileScale > 1.0 ? [5, 4] : [3, 3]);
    ctx.beginPath();
    for (var oi = 0; oi < outlines.length; oi++) {
      var poly = outlines[oi].polygon;
      if (!poly || !poly.length) continue;
      ctx.moveTo((poly[0][0] - wx0) * gs * tileScale, (poly[0][1] - wy0) * gs * tileScale);
      for (var pi = 1; pi < poly.length; pi++) {
        ctx.lineTo((poly[pi][0] - wx0) * gs * tileScale, (poly[pi][1] - wy0) * gs * tileScale);
      }
      ctx.closePath();
    }
    ctx.stroke();
    ctx.restore();
  }

  function ensureMacroCanvas(dec, m, pal, light, timeMode) {
    var key = fnvHash(dec.terrainKey) + ':' + fnvHash(dec.overlayKey) + ':' + timeMode + ':' + (m.is_night ? 'N' : 'D');
    if (macroCache.key === key && macroCache.canvas) return macroCache.canvas;

    var W = dec.W, H = dec.H;
    var c = macroCache.canvas || document.createElement('canvas');
    c.width = W;
    c.height = H;
    var mctx = c.getContext('2d');
    mctx.clearRect(0, 0, W, H);

    var terrain = dec.terrain, road = dec.road, bldg = dec.bldg;
    var MW = W >> 1, MH = H >> 1;

    // 1. 巨觀地塊底紙、未知鄰接迷霧輪廓、農田與退潮沙灘
    var knownField = new Float32Array(MW * MH);
    var landMask = new Float32Array(MW * MH);
    var waterField = new Float32Array(MW * MH);
    var treeField = new Float32Array(MW * MH);

    for (var my = 0; my < MH; my++) {
      var gy0 = my * 2;
      for (var mx = 0; mx < MW; mx++) {
        var gx0 = mx * 2;
        var wAcc = 0, lAcc = 0, tAcc = 0, kAcc = 0;
        for (var dy = 0; dy < 2; dy++) {
          var row = (gy0 + dy) * W + gx0;
          for (var dx = 0; dx < 2; dx++) {
            var tv = terrain[row + dx];
            if (tv !== CELL.FOG) kAcc += 0.25;
            if (tv === CELL.SEA || tv === CELL.RIVER) wAcc += 0.25;
            if (tv !== CELL.FOG && tv !== CELL.SEA) lAcc += 0.25;
            if (tv === CELL.TREE) tAcc += 0.25;
          }
        }
        knownField[my * MW + mx] = kAcc;
        waterField[my * MW + mx] = wAcc;
        landMask[my * MW + mx] = lAcc;
        treeField[my * MW + mx] = tAcc;
      }
    }

    // 已知區域外緣柔和暈開成羊皮紙底色（無直角界線），未探索處透明露底層 fogFill
    var knownLoops = extractClosedLoops(knownField, MW, MH, 0.36);
    if (knownLoops.length) {
      var knownPath = buildContourPath(knownLoops, 0, 0, 2, 2, 2);
      mctx.save();
      mctx.strokeStyle = pal.fogStroke;
      mctx.lineWidth = 1.4;
      mctx.setLineDash([4, 4]);
      mctx.stroke(knownPath);
      mctx.restore();
    }

    // 陸地輪廓先鋪水彩底紙色
    var landLoops = extractClosedLoops(landMask, MW, MH, 0.35);
    var landPath = buildContourPath(landLoops, 0, 0, 2, 2, 1);
    mctx.fillStyle = pal.paper;
    mctx.fill(landPath, 'evenodd');

    // 鄰接一環未踏足 Voronoi 虛線邊界
    renderOutlineCells(mctx, m, pal, 1.0);

    // 農田與退潮沙洲（先於水系繪製）
    for (var y = 0; y < H; y += 2) {
      for (var x = 0; x < W; x += 2) {
        var t = terrain[y * W + x];
        if (t === CELL.FARM) {
          mctx.fillStyle = pal.wheats[((x >> 2) + (y >> 2)) & 3];
          mctx.fillRect(x, y, 2, 2);
        } else if (t === CELL.REEF_SAND) {
          mctx.fillStyle = pal.reefFill;
          mctx.fillRect(x, y, 2, 2);
        }
      }
    }

    // 2. 平滑水系（海洋＋河流連通一體）
    var waterLoops = extractClosedLoops(waterField, MW, MH, 0.44);
    var waterPath = buildContourPath(waterLoops, 0, 0, 2, 2, 2);
    mctx.save();
    mctx.strokeStyle = pal.shoreSand;
    mctx.lineWidth = 5;
    mctx.stroke(waterPath);
    mctx.fillStyle = pal.sea;
    mctx.fill(waterPath, 'evenodd');
    mctx.clip(waterPath, 'evenodd');
    for (var ri = 0; ri < pal.seaRim.length; ri++) {
      mctx.strokeStyle = pal.seaRim[ri][0];
      mctx.lineWidth = pal.seaRim[ri][1] * 0.55;
      mctx.stroke(waterPath);
    }
    mctx.restore();
    mctx.strokeStyle = pal.shoreInk;
    mctx.lineWidth = 0.8;
    mctx.stroke(waterPath);

    // 3. 山巒疊影
    mctx.fillStyle = pal.mtnBase;
    for (var my2 = 2; my2 < H - 2; my2 += 3) {
      for (var mx2 = 2; mx2 < W - 2; mx2 += 3) {
        if (terrain[my2 * W + mx2] !== CELL.MOUNTAIN) continue;
        var mh = 4.2 + hash2(mx2, my2) * 3.5;
        var mw = 3.8 + hash2(my2, mx2) * 2.2;
        mctx.fillStyle = pal.mtnLit;
        mctx.beginPath(); mctx.moveTo(mx2, my2 - mh); mctx.lineTo(mx2 - mw, my2); mctx.lineTo(mx2, my2 + 0.5); mctx.closePath(); mctx.fill();
        mctx.fillStyle = pal.mtnShade;
        mctx.beginPath(); mctx.moveTo(mx2, my2 - mh); mctx.lineTo(mx2 + mw, my2); mctx.lineTo(mx2, my2 + 0.5); mctx.closePath(); mctx.fill();
        mctx.strokeStyle = pal.mtnInk;
        mctx.lineWidth = 0.6;
        mctx.beginPath(); mctx.moveTo(mx2 - mw, my2); mctx.lineTo(mx2, my2 - mh); mctx.lineTo(mx2 + mw, my2); mctx.stroke();
      }
    }

    // 4. 森林樹海（陰影 + 橄欖深綠漸層 + 書法墨線）
    var fLoops = extractClosedLoops(treeField, MW, MH, 0.42);
    if (fLoops.length) {
      var fPath = buildContourPath(fLoops, 0, 0, 2, 2, 1);
      mctx.save();
      mctx.translate(1.5, 2.0);
      mctx.fillStyle = pal.forestShadow;
      mctx.fill(fPath, 'evenodd');
      mctx.restore();
      mctx.fillStyle = pal.forestOlive;
      mctx.fill(fPath, 'evenodd');
      mctx.strokeStyle = pal.forestInk;
      mctx.lineWidth = 0.9;
      mctx.stroke(fPath);
    }

    // 5. 全島幹道、石板街、橋樑棧橋與城牆
    for (var y5 = 0; y5 < H; y5++) {
      var r5 = y5 * W;
      for (var x5 = 0; x5 < W; x5++) {
        var rv = road[r5 + x5];
        if (rv === ROAD.NONE) continue;
        if (rv === ROAD.BRIDGE) mctx.fillStyle = pal.bridge;
        else if (rv === ROAD.WALL) mctx.fillStyle = pal.towerShade;
        else if (rv === ROAD.COBBLE) mctx.fillStyle = pal.cobbleLit;
        else mctx.fillStyle = pal.roadFill;
        mctx.fillRect(x5, y5, 1.35, 1.35);
      }
    }

    // 6. 全島屋舍與石塔微縮輪廓
    for (var y6 = 0; y6 < H; y6++) {
      var r6 = y6 * W;
      for (var x6 = 0; x6 < W; x6++) {
        var bv = bldg[r6 + x6];
        if (bv === BLDG.NONE) continue;
        mctx.fillStyle = pal.shadow;
        mctx.fillRect(x6 + 0.6, y6 + 0.8, 1.2, 1.2);
        if (bv === BLDG.TOWER) mctx.fillStyle = pal.towerCapLit;
        else if (bv === BLDG.RUIN) mctx.fillStyle = pal.wallBase;
        else if (bv === BLDG.VACANT) mctx.fillStyle = 'rgb(' + pal.roofSets[4].slopes[0].join(',') + ')';
        else mctx.fillStyle = 'rgb(' + pal.roofSets[0].slopes[0].join(',') + ')';
        mctx.fillRect(x6, y6, 1.25, 1.25);
      }
    }

    macroCache.key = key;
    macroCache.canvas = c;
    return c;
  }

  /* ═══ 微觀精雕渲染（僅計算可見格窗 [gx0..gx1, gy0..gy1]，完全移植精雕古卷工藝） ═══ */
  function analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, predIdx) {
    var vw = gx1 - gx0 + 1;
    var vh = gy1 - gy0 + 1;
    var compId = new Int32Array(vw * vh).fill(-1);
    var comps = [];
    var dirs = [[1, 0], [-1, 0], [0, 1], [0, -1]];
    for (var ly = 0; ly < vh; ly++) {
      var gy = gy0 + ly;
      for (var lx = 0; lx < vw; lx++) {
        var gx = gx0 + lx;
        var li = ly * vw + lx;
        if (!predIdx(gy * W + gx) || compId[li] !== -1) continue;
        var cid = comps.length;
        var cells = [[gx, gy]];
        compId[li] = cid;
        var head = 0;
        while (head < cells.length) {
          var cx = cells[head][0], cy = cells[head][1];
          head++;
          for (var d = 0; d < 4; d++) {
            var nx = cx + dirs[d][0], ny = cy + dirs[d][1];
            if (nx >= gx0 && nx <= gx1 && ny >= gy0 && ny <= gy1) {
              var nli = (ny - gy0) * vw + (nx - gx0);
              if (predIdx(ny * W + nx) && compId[nli] === -1) {
                compId[nli] = cid;
                cells.push([nx, ny]);
              }
            }
          }
        }
        comps.push(cells);
      }
    }
    return { compId: compId, comps: comps, vw: vw, vh: vh };
  }

  function buildTileComponentPath(inSetFn, gx0, gy0, gx1, gy1, cw, ch) {
    var segs = [];
    for (var y = gy0; y <= gy1; y++) {
      for (var x = gx0; x <= gx1; x++) {
        if (!inSetFn(x, y)) continue;
        if (!inSetFn(x, y - 1)) segs.push([[x, y], [x + 1, y]]);
        if (!inSetFn(x + 1, y)) segs.push([[x + 1, y], [x + 1, y + 1]]);
        if (!inSetFn(x, y + 1)) segs.push([[x + 1, y + 1], [x, y + 1]]);
        if (!inSetFn(x - 1, y)) segs.push([[x, y + 1], [x, y]]);
      }
    }
    var keyOf = function (p) { return p[0] + ',' + p[1]; };
    var nextMap = new Map();
    for (var i = 0; i < segs.length; i++) {
      var k = keyOf(segs[i][0]);
      if (!nextMap.has(k)) nextMap.set(k, []);
      nextMap.get(k).push(i);
    }
    var used = new Uint8Array(segs.length);
    var path = new Path2D();
    for (var j = 0; j < segs.length; j++) {
      if (used[j]) continue;
      used[j] = 1;
      var loop = [segs[j][0], segs[j][1]];
      var cur = segs[j][1];
      for (var g = 0; g < segs.length; g++) {
        var list = nextMap.get(keyOf(cur));
        if (!list) break;
        var found = false;
        for (var m = 0; m < list.length; m++) {
          var si = list[m];
          if (used[si]) continue;
          used[si] = 1;
          cur = segs[si][1];
          loop.push(cur);
          found = true;
          break;
        }
        if (!found) break;
      }
      path.moveTo(loop[0][0] * cw, loop[0][1] * ch);
      for (var p = 1; p < loop.length; p++) path.lineTo(loop[p][0] * cw, loop[p][1] * ch);
      path.closePath();
    }
    return path;
  }

  function renderMergedRoof(ctx, cells, compId, cid, gx0, gy0, vw, vh, pxTile, dpr, pal, light, timeMode, buildingLights, isVacant) {
    var minX = 1e9, maxX = -1, minY = 1e9, maxY = -1;
    var sumX = 0, sumY = 0;
    for (var i = 0; i < cells.length; i++) {
      var x = cells[i][0], y = cells[i][1];
      sumX += x + 0.5; sumY += y + 0.5;
      if (x < minX) minX = x;
      if (x > maxX) maxX = x;
      if (y < minY) minY = y;
      if (y > maxY) maxY = y;
    }
    var inComp = function (gx, gy) {
      var lx = gx - gx0, ly = gy - gy0;
      return lx >= 0 && lx < vw && ly >= 0 && ly < vh && compId[ly * vw + lx] === cid;
    };
    var bldgPath = buildTileComponentPath(inComp, minX, minY, maxX, maxY, pxTile, pxTile);
    var wallH = Math.max(2.6, pxTile * 0.44);
    var roofVariant = isVacant ? 4 : (Math.floor(hash2(minX * 5 + 3, minY * 7 + 1) * 4) & 3);
    var rPalette = pal.roofSets[roofVariant];

    // 1. 落地斜影
    ctx.save();
    ctx.translate(light.sx * (pxTile / 15), light.sy * (pxTile / 15) + wallH * 0.5);
    ctx.fillStyle = pal.shadow;
    ctx.fill(bldgPath, 'evenodd');
    ctx.restore();

    // 2. 2.5D 南向白灰木構外牆 ＋ 石基 ＋ 門窗
    var windowLitSpots = [];
    for (var ci = 0; ci < cells.length; ci++) {
      var gx = cells[ci][0], gy = cells[ci][1];
      if (inComp(gx, gy + 1)) continue;
      var wx = gx * pxTile, wy = (gy + 1) * pxTile;
      var leftClosed = !inComp(gx - 1, gy);
      var rightClosed = !inComp(gx + 1, gy);

      ctx.fillStyle = pal.wallFace;
      ctx.fillRect(wx, wy, pxTile + 0.3, wallH);
      ctx.fillStyle = pal.wallBase;
      ctx.fillRect(wx, wy + wallH - Math.max(1.0, wallH * 0.28), pxTile + 0.3, Math.max(1.0, wallH * 0.28));
      ctx.fillStyle = 'rgba(20, 14, 10, 0.34)';
      ctx.fillRect(wx, wy, pxTile + 0.3, Math.max(1.0, wallH * 0.30));

      ctx.strokeStyle = pal.ink;
      ctx.lineWidth = 0.8;
      ctx.beginPath();
      ctx.moveTo(wx, wy + wallH); ctx.lineTo(wx + pxTile, wy + wallH);
      if (leftClosed) { ctx.moveTo(wx + 0.2, wy); ctx.lineTo(wx + 0.2, wy + wallH); }
      if (rightClosed) { ctx.moveTo(wx + pxTile - 0.2, wy); ctx.lineTo(wx + pxTile - 0.2, wy + wallH); }
      ctx.stroke();

      var cellH = hash2(gx * 13 + 7, gy * 19 + 3);
      if (cellH > 0.28) {
        var isDoor = cellH > 0.66;
        var ow = Math.max(1.6, pxTile * 0.30);
        var oh = isDoor ? wallH * 0.66 : wallH * 0.42;
        var ox = wx + (pxTile - ow) * 0.5;
        var oy = isDoor ? (wy + wallH - oh) : (wy + wallH * 0.34);
        if (pal.lantern > 0 && !isVacant) {
          ctx.fillStyle = timeMode === 'night' ? '#ffcf76' : '#f2ad54';
          ctx.fillRect(ox, oy, ow, oh);
          windowLitSpots.push([ox + ow * 0.5, oy + oh * 0.65, Math.max(6, pxTile * 0.7)]);
        } else {
          ctx.fillStyle = pal.wallBeam;
          ctx.fillRect(ox, oy, ow, oh);
        }
      }
    }

    // 3. 屋頂直骨架四色錯縫魚鱗瓦（以棟幾何指紋快取，平移縮放零重算！）
    var maskBits = [];
    for (var my = minY; my <= maxY; my++) {
      for (var mx = minX; mx <= maxX; mx++) {
        maskBits.push(inComp(mx, my) ? '1' : '0');
      }
    }
    var texScale = Math.max(10, Math.min(26, Math.round(pxTile)));
    var cacheKey = maskBits.join('') + ':' + (maxX - minX + 1) + 'x' + (maxY - minY + 1) + ':' + roofVariant + ':' + timeMode + ':' + texScale;
    var roofTex = roofTexCache.get(cacheKey);
    if (!roofTex) {
      if (roofTexCache.size > 420) roofTexCache.clear();
      var pw = Math.max(1, (maxX - minX + 1) * texScale);
      var ph = Math.max(1, (maxY - minY + 1) * texScale);
      roofTex = document.createElement('canvas');
      roofTex.width = pw;
      roofTex.height = ph;
      var octx = roofTex.getContext('2d');
      var emptyCells = [];
      for (var ey = minY - 1; ey <= maxY + 1; ey++) {
        for (var ex = minX - 1; ex <= maxX + 1; ex++) {
          if (!inComp(ex, ey)) emptyCells.push([ex - minX, ey - minY]);
        }
      }
      var gwStride = pw + 2;
      var hGrid = new Float32Array(gwStride * (ph + 2));
      for (var py = -1; py <= ph; py++) {
        var rgy = (py + 0.5) / texScale;
        var rOff = (py + 1) * gwStride;
        for (var px = -1; px <= pw; px++) {
          var rgx = (px + 0.5) / texScale;
          var minD = 999;
          for (var k = 0; k < emptyCells.length; k++) {
            var cx2 = emptyCells[k][0], cy2 = emptyCells[k][1];
            var dx2 = rgx < cx2 ? (cx2 - rgx) : (rgx > cx2 + 1 ? (rgx - (cx2 + 1)) : 0);
            var dy2 = rgy < cy2 ? (cy2 - rgy) : (rgy > cy2 + 1 ? (rgy - (cy2 + 1)) : 0);
            var dm = dx2 > dy2 ? dx2 : dy2;
            if (dm < minD) minD = dm;
          }
          hGrid[rOff + px + 1] = minD;
        }
      }
      var img = octx.createImageData(pw, ph);
      var data = img.data;
      var faces = new Uint8Array(pw * ph);
      for (var py2 = 0; py2 < ph; py2++) {
        var rgy2 = (py2 + 0.5) / texScale;
        var rOff2 = (py2 + 1) * gwStride;
        for (var px2 = 0; px2 < pw; px2++) {
          var rgx2 = (px2 + 0.5) / texScale;
          var hi = rOff2 + px2 + 1;
          var h0 = hGrid[hi];
          var dhx = hGrid[hi + 1] - hGrid[hi - 1];
          var dhy = hGrid[hi + gwStride] - hGrid[hi - gwStride];
          var f = Math.abs(dhy) >= Math.abs(dhx) - 1e-4 ? (dhy >= 0 ? 0 : 3) : (dhx >= 0 ? 1 : 2);
          faces[py2 * pw + px2] = f;
          var base = rPalette.slopes[f];
          var r = base[0], g = base[1], b = base[2];
          var along = (f === 0 || f === 3) ? rgx2 : rgy2;
          var rowNum = Math.floor(h0 * 4.8);
          var course = h0 * 4.8 - rowNum;
          var mod = (course < 0.20 ? -15 : (course > 0.85 ? 7 : 0)) + Math.sin(along * 14.0 + (rowNum & 1) * Math.PI) * 6.5;
          if (h0 < 0.09) mod -= 12;
          if (pal.lantern > 0 && !isVacant && h0 < 0.25) {
            var ws = pal.lantern * (0.25 - h0) * 1.15;
            r += ws * 26; g += ws * 12; b -= ws * 7;
          }
          r += mod; g += mod * 0.92; b += mod * 0.85;
          var o = (py2 * pw + px2) * 4;
          data[o] = r < 0 ? 0 : (r > 255 ? 255 : r);
          data[o + 1] = g < 0 ? 0 : (g > 255 ? 255 : g);
          data[o + 2] = b < 0 ? 0 : (b > 255 ? 255 : b);
          data[o + 3] = 255;
        }
      }
      var rc = rPalette.ridge;
      for (var py3 = 0; py3 < ph - 1; py3++) {
        for (var px3 = 0; px3 < pw - 1; px3++) {
          var idx3 = py3 * pw + px3;
          var f3 = faces[idx3];
          if (f3 !== faces[idx3 + 1] || f3 !== faces[idx3 + pw]) {
            var o3 = idx3 * 4;
            data[o3] = Math.round(data[o3] * 0.14 + rc[0] * 0.86);
            data[o3 + 1] = Math.round(data[o3 + 1] * 0.14 + rc[1] * 0.86);
            data[o3 + 2] = Math.round(data[o3 + 2] * 0.14 + rc[2] * 0.86);
          }
        }
      }
      octx.putImageData(img, 0, 0);
      roofTexCache.set(cacheKey, roofTex);
    }

    ctx.save();
    ctx.clip(bldgPath, 'evenodd');
    ctx.drawImage(
      roofTex,
      minX * pxTile,
      minY * pxTile,
      (maxX - minX + 1) * pxTile,
      (maxY - minY + 1) * pxTile
    );
    ctx.restore();

    ctx.strokeStyle = pal.ink;
    ctx.lineWidth = 1.05;
    ctx.stroke(bldgPath);

    // 4. 大棟屋舍石煙囪與炊煙
    if (cells.length >= 2 && pxTile >= 8) {
      var chx = (sumX / cells.length) * pxTile + pxTile * 0.15;
      var chy = (sumY / cells.length) * pxTile - pxTile * 0.18;
      var csz = Math.max(2.2, pxTile * 0.25);
      ctx.fillStyle = pal.towerLit;
      ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.7;
      ctx.fillRect(chx - csz * 0.5, chy - csz * 0.5, csz, csz);
      ctx.strokeRect(chx - csz * 0.5, chy - csz * 0.5, csz, csz);
      ctx.fillStyle = pal.ink;
      ctx.fillRect(chx - csz * 0.2, chy - csz * 0.2, csz * 0.4, csz * 0.4);
      if (!isVacant) {
        ctx.strokeStyle = timeMode === 'night' ? 'rgba(218, 224, 236, 0.22)' : 'rgba(84, 76, 68, 0.24)';
        ctx.lineWidth = 0.9;
        ctx.beginPath();
        ctx.moveTo(chx, chy - csz * 0.5);
        ctx.bezierCurveTo(chx + 1.6, chy - csz * 1.4, chx - 1.1, chy - csz * 2.2, chx + 2.2, chy - csz * 3.1);
        ctx.stroke();
      }
    }

    if (pal.lantern > 0 && !isVacant) {
      buildingLights.push({
        cx: (sumX / cells.length) * pxTile,
        cy: (sumY / cells.length) * pxTile,
        span: pxTile * (1.0 + Math.sqrt(cells.length) * 0.7),
        windowLitSpots: windowLitSpots
      });
    }
  }

  function drawBroadleafSingleTree(ctx, cx, cy, pxTile, pal, light) {
    var rScale = Math.max(0.65, pxTile / 14);
    ctx.fillStyle = pal.forestShadow;
    ctx.beginPath();
    ctx.ellipse(cx + light.sx * 0.7, cy + light.sy * 0.7, 4.4 * rScale, 1.7 * rScale, light.dy * 0.25, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = pal.forestInk;
    ctx.lineWidth = 1.2 * rScale;
    ctx.beginPath();
    ctx.moveTo(cx, cy + 1.4 * rScale); ctx.lineTo(cx, cy - 2.8 * rScale);
    ctx.stroke();

    var clumps = [
      [cx + 1.8 * rScale, cy - 3.6 * rScale, 2.3 * rScale],
      [cx - 1.6 * rScale, cy - 3.5 * rScale, 2.2 * rScale],
      [cx + 0.1 * rScale, cy - 5.6 * rScale, 2.6 * rScale]
    ];
    ctx.fillStyle = pal.forestDeep;
    for (var i = 0; i < clumps.length; i++) {
      ctx.beginPath(); ctx.arc(clumps[i][0] + 0.4, clumps[i][1] + 0.4, clumps[i][2], 0, Math.PI * 2); ctx.fill();
    }
    ctx.fillStyle = pal.forestOlive;
    for (var j = 0; j < clumps.length; j++) {
      ctx.beginPath(); ctx.arc(clumps[j][0], clumps[j][1], clumps[j][2] * 0.9, 0, Math.PI * 2); ctx.fill();
    }
    ctx.strokeStyle = pal.forestInk;
    ctx.lineWidth = 0.78;
    for (var k = 0; k < clumps.length; k++) {
      ctx.beginPath(); ctx.arc(clumps[k][0], clumps[k][1], clumps[k][2], -0.3, Math.PI * 0.92); ctx.stroke();
    }
  }

  function drawConiferSingleTree(ctx, cx, cy, pxTile, pal, light) {
    var rScale = Math.max(0.65, pxTile / 14);
    ctx.fillStyle = pal.forestShadow;
    ctx.beginPath();
    ctx.ellipse(cx + light.sx * 0.8, cy + light.sy * 0.8, 3.6 * rScale, 1.4 * rScale, light.dy * 0.25, 0, Math.PI * 2);
    ctx.fill();

    ctx.strokeStyle = pal.forestInk;
    ctx.lineWidth = 1.2 * rScale;
    ctx.beginPath(); ctx.moveTo(cx, cy + 1.5 * rScale); ctx.lineTo(cx, cy - 2.3 * rScale); ctx.stroke();

    var tiers = [
      { yBot: cy - 1.4 * rScale, yTop: cy - 5.0 * rScale, hw: 3.1 * rScale },
      { yBot: cy - 3.4 * rScale, yTop: cy - 7.0 * rScale, hw: 2.5 * rScale },
      { yBot: cy - 5.4 * rScale, yTop: cy - 9.0 * rScale, hw: 1.8 * rScale }
    ];
    for (var i = 0; i < tiers.length; i++) {
      var t = tiers[i];
      ctx.fillStyle = pal.forestOlive;
      ctx.beginPath(); ctx.moveTo(cx, t.yTop); ctx.lineTo(cx - t.hw, t.yBot); ctx.lineTo(cx, t.yBot + 0.3); ctx.closePath(); ctx.fill();
      ctx.fillStyle = pal.forestDeep;
      ctx.beginPath(); ctx.moveTo(cx, t.yTop); ctx.lineTo(cx + t.hw, t.yBot); ctx.lineTo(cx, t.yBot + 0.3); ctx.closePath(); ctx.fill();
      ctx.strokeStyle = pal.forestInk; ctx.lineWidth = 0.8;
      ctx.beginPath(); ctx.moveTo(cx - t.hw, t.yBot); ctx.lineTo(cx, t.yTop); ctx.lineTo(cx + t.hw, t.yBot); ctx.stroke();
    }
  }

  function renderLocalCraftLayer(ctx, dec, cam, viewW, viewH, dpr, pal, light, timeMode) {
    var W = dec.W, H = dec.H;
    var terrain = dec.terrain, road = dec.road, bldg = dec.bldg;
    var z = cam.pxPerTile;
    var pad = 6;
    var halfCols = (viewW * 0.5) / z;
    var halfRows = (viewH * 0.5) / z;
    var gx0 = Math.max(0, Math.floor(cam.cx - halfCols) - pad);
    var gx1 = Math.min(W - 1, Math.ceil(cam.cx + halfCols) + pad);
    var gy0 = Math.max(0, Math.floor(cam.cy - halfRows) - pad);
    var gy1 = Math.min(H - 1, Math.ceil(cam.cy + halfRows) + pad);
    var vw = gx1 - gx0 + 1, vh = gy1 - gy0 + 1;
    if (vw < 2 || vh < 2) return;

    // 1. 已知視野平滑雲霧弧邊＋農田底紋（消滅方格直角邊，先於水系以免壓蓋水岸）
    var knownLocalField = new Float32Array(vw * vh);
    for (var ky = 0; ky < vh; ky++) {
      var kgy = gy0 + ky;
      for (var kx = 0; kx < vw; kx++) {
        var kgx = gx0 + kx;
        var kAcc = 0, kWs = 0;
        for (var kdy = -1; kdy <= 1; kdy++) {
          for (var kdx = -1; kdx <= 1; kdx++) {
            var knx = Math.max(0, Math.min(W - 1, kgx + kdx));
            var kny = Math.max(0, Math.min(H - 1, kgy + kdy));
            var kt = terrain[kny * W + knx];
            var kwt = (kdx === 0 && kdy === 0) ? 4.5 : (Math.abs(kdx) + Math.abs(kdy) === 1 ? 1.2 : 0.5);
            if (kt !== CELL.FOG) kAcc += kwt;
            kWs += kwt;
          }
        }
        knownLocalField[ky * vw + kx] = kAcc / kWs + Math.sin(kgx * 1.1 + kgy * 1.5) * 0.018;
      }
    }
    var knownLocalLoops = extractClosedLoops(knownLocalField, vw, vh, 0.44);
    if (knownLocalLoops.length) {
      var knownLocalPath = buildContourPath(knownLocalLoops, gx0, gy0, z, z, 2);
      ctx.save();
      ctx.fillStyle = pal.paper;
      ctx.fill(knownLocalPath, 'evenodd');
      ctx.strokeStyle = pal.fogStroke;
      ctx.lineWidth = Math.max(1.2, z * 0.14);
      ctx.setLineDash([4, 4]);
      ctx.stroke(knownLocalPath);
      ctx.restore();
    }

    for (var y = gy0; y <= gy1; y++) {
      var rowOff = y * W;
      for (var x = gx0; x <= gx1; x++) {
        var tv = terrain[rowOff + x];
        if (tv === CELL.FARM) {
          var pSlot = ((Math.floor(hash2(x >> 1, y >> 1) * 4) + ((x >> 1) & 1)) & 3);
          ctx.fillStyle = pal.wheats[pSlot];
          ctx.fillRect(x * z - 0.2, y * z - 0.2, z + 0.4, z + 0.4);
          ctx.strokeStyle = pal.furrow;
          ctx.lineWidth = 0.7;
          ctx.beginPath();
          for (var f = 0.2; f < 0.95; f += 0.32) {
            ctx.moveTo(x * z, (y + f) * z);
            ctx.lineTo((x + 1) * z, (y + f + 0.14) * z);
          }
          ctx.stroke();
        }
      }
    }

    // 農田灌木田埂與麥草垛
    var farmComps = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) { return terrain[idx] === CELL.FARM; });
    for (var fi = 0; fi < farmComps.comps.length; fi++) {
      var fc = farmComps.comps[fi];
      if (fc.length >= 6) {
        var sample = fc[Math.floor(fc.length * 0.4)];
        var hx = (sample[0] + 0.5) * z, hy = (sample[1] + 0.5) * z;
        var hr = Math.max(2.2, z * 0.36);
        ctx.fillStyle = pal.shadow;
        ctx.beginPath(); ctx.ellipse(hx + 1, hy + 1, hr, hr * 0.65, 0, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = timeMode === 'night' ? '#705e3a' : '#e0b85c';
        ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.7;
        ctx.beginPath(); ctx.arc(hx, hy, hr, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      }
    }

    // 2. 合併海水與河流＋潮間沙灘＋河口安全斷開水紋
    var wField = new Float32Array(vw * vh);
    var riverNear = new Uint8Array(vw * vh);
    for (var wy = 0; wy < vh; wy++) {
      var gy = gy0 + wy;
      for (var wx = 0; wx < vw; wx++) {
        var gx = gx0 + wx;
        var acc = 0, ws = 0;
        for (var dy = -1; dy <= 1; dy++) {
          for (var dx = -1; dx <= 1; dx++) {
            var nx = Math.max(0, Math.min(W - 1, gx + dx));
            var ny = Math.max(0, Math.min(H - 1, gy + dy));
            var t = terrain[ny * W + nx];
            var wt = (dx === 0 && dy === 0) ? 5.0 : (Math.abs(dx) + Math.abs(dy) === 1 ? 1.15 : 0.45);
            if (t === CELL.SEA || t === CELL.RIVER) acc += wt;
            if (t === CELL.RIVER) riverNear[wy * vw + wx] = 1;
            ws += wt;
          }
        }
        wField[wy * vw + wx] = acc / ws + Math.sin(gx * 1.4 + gy * 1.8) * 0.014;
      }
    }
    var wLoops = extractClosedLoops(wField, vw, vh, 0.47);
    if (wLoops.length) {
      var wPath = buildContourPath(wLoops, gx0, gy0, z, z, 2);
      ctx.save();
      ctx.strokeStyle = pal.shoreSand;
      ctx.lineWidth = Math.max(4, z * 0.65);
      ctx.stroke(wPath);
      ctx.strokeStyle = pal.shorePool;
      ctx.lineWidth = Math.max(2, z * 0.32);
      ctx.stroke(wPath);
      ctx.fillStyle = pal.sea;
      ctx.fill(wPath, 'evenodd');
      ctx.clip(wPath, 'evenodd');

      // 退潮露出之潮間淺灘礁坪
      for (var ry = gy0; ry <= gy1; ry++) {
        for (var rx = gx0; rx <= gx1; rx++) {
          if (terrain[ry * W + rx] === CELL.REEF_SAND) {
            ctx.fillStyle = pal.reefFill;
            ctx.beginPath(); ctx.arc((rx + 0.5) * z, (ry + 0.5) * z, z * 0.72, 0, Math.PI * 2); ctx.fill();
          }
        }
      }
      for (var sri = 0; sri < pal.seaRim.length; sri++) {
        ctx.strokeStyle = pal.seaRim[sri][0];
        ctx.lineWidth = pal.seaRim[sri][1];
        ctx.stroke(wPath);
      }

      // 局部距離場水紋（遇見河流即自動提筆斷開，絕不攔腰封死河口）
      var lDist = new Float32Array(vw * vh).fill(99);
      for (var ly = 0; ly < vh; ly++) {
        for (var lx = 0; lx < vw; lx++) {
          var gt = terrain[(gy0 + ly) * W + (gx0 + lx)];
          if (gt !== CELL.SEA && gt !== CELL.RIVER) lDist[ly * vw + lx] = 0;
        }
      }
      for (var ly1 = 0; ly1 < vh; ly1++) {
        for (var lx1 = 0; lx1 < vw; lx1++) {
          var i1 = ly1 * vw + lx1;
          if (lDist[i1] === 0) continue;
          if (lx1 > 0) lDist[i1] = Math.min(lDist[i1], lDist[i1 - 1] + 1);
          if (ly1 > 0) lDist[i1] = Math.min(lDist[i1], lDist[i1 - vw] + 1);
        }
      }
      for (var ly2 = vh - 1; ly2 >= 0; ly2--) {
        for (var lx2 = vw - 1; lx2 >= 0; lx2--) {
          var i2 = ly2 * vw + lx2;
          if (lDist[i2] === 0) continue;
          if (lx2 + 1 < vw) lDist[i2] = Math.min(lDist[i2], lDist[i2 + 1] + 1);
          if (ly2 + 1 < vh) lDist[i2] = Math.min(lDist[i2], lDist[i2 + vw] + 1);
        }
      }
      var rLevels = [[0.65, 0.85, 0.95], [1.35, 0.52, 0.75]];
      for (var rl = 0; rl < rLevels.length; rl++) {
        var rLoops = extractClosedLoops(lDist, vw, vh, rLevels[rl][0]);
        var rPath = new Path2D();
        for (var rli = 0; rli < rLoops.length; rli++) {
          var sm = chaikinSmoothLoop(rLoops[rli], 1);
          var pen = false;
          for (var si = 0; si <= sm.length; si++) {
            var p = sm[si % sm.length];
            var pxIdx = Math.max(0, Math.min(vw - 1, Math.round(p[0])));
            var pyIdx = Math.max(0, Math.min(vh - 1, Math.round(p[1])));
            var onEdge = p[0] < 0.4 || p[0] > vw - 1.4 || p[1] < 0.4 || p[1] > vh - 1.4;
            if (!onEdge && !riverNear[pyIdx * vw + pxIdx]) {
              var sx = (gx0 + p[0]) * z, sy = (gy0 + p[1]) * z;
              if (!pen) { rPath.moveTo(sx, sy); pen = true; }
              else rPath.lineTo(sx, sy);
            } else {
              pen = false;
            }
          }
        }
        ctx.strokeStyle = pal.seaRipple;
        ctx.globalAlpha = rLevels[rl][1];
        ctx.lineWidth = rLevels[rl][2];
        ctx.stroke(rPath);
      }
      ctx.globalAlpha = 1;
      ctx.restore();
      ctx.strokeStyle = pal.shoreInk;
      ctx.lineWidth = 1.05;
      ctx.stroke(wPath);
    }

    // 3. 道路、鵝卵石城街、雙轍土路、棧橋碼頭與城牆
    ctx.save();
    ctx.lineCap = 'round'; ctx.lineJoin = 'round';
    var trackPath = new Path2D();
    var cobblePath = new Path2D();
    var trackSegs = [];
    var cobbleSegs = [];
    var bridgeCells = [];
    var wallCells = [];

    function roadPt(gx, gy) {
      var wob = 0.08 * z;
      return [(gx + 0.5) * z + Math.sin(gx * 1.1 + gy * 1.7) * wob, (gy + 0.5) * z + Math.cos(gx * 1.3 - gy * 0.8) * wob];
    }

    for (var y3 = gy0; y3 <= gy1; y3++) {
      for (var x3 = gx0; x3 <= gx1; x3++) {
        var rv = road[y3 * W + x3];
        if (rv === ROAD.NONE) continue;
        if (rv === ROAD.BRIDGE) { bridgeCells.push([x3, y3]); continue; }
        if (rv === ROAD.WALL) { wallCells.push([x3, y3]); continue; }
        var p0 = roadPt(x3, y3);
        var isCob = rv === ROAD.COBBLE;
        var tgtPath = isCob ? cobblePath : trackPath;
        var tgtSegs = isCob ? cobbleSegs : trackSegs;
        var linked = false;
        var dirs3 = [[1, 0], [0, 1], [1, 1], [1, -1]];
        for (var d3 = 0; d3 < 4; d3++) {
          var nx = x3 + dirs3[d3][0], ny = y3 + dirs3[d3][1];
          if (nx < 0 || nx >= W || ny < 0 || ny >= H) continue;
          var nrv = road[ny * W + nx];
          if (nrv === ROAD.TRACK || nrv === ROAD.COBBLE || nrv === ROAD.BRIDGE) {
            if (d3 >= 2 && (road[y3 * W + nx] > 0 || road[ny * W + x3] > 0)) continue;
            var p1 = roadPt(nx, ny);
            tgtPath.moveTo(p0[0], p0[1]); tgtPath.lineTo(p1[0], p1[1]);
            tgtSegs.push([x3, y3, p0, p1]);
            linked = true;
          }
        }
        if (!linked) {
          tgtPath.moveTo(p0[0] - 0.6, p0[1]); tgtPath.lineTo(p0[0] + 0.6, p0[1]);
        }
      }
    }

    var trackW = Math.max(2.6, z * 0.34);
    ctx.strokeStyle = pal.roadHalo; ctx.lineWidth = trackW + 3.6; ctx.stroke(trackPath);
    ctx.strokeStyle = pal.roadInk; ctx.lineWidth = trackW + 1.4; ctx.setLineDash([5, 2]); ctx.stroke(trackPath);
    ctx.setLineDash([]); ctx.strokeStyle = pal.roadFill; ctx.lineWidth = trackW; ctx.stroke(trackPath);

    var cobbleW = Math.max(3.6, z * 0.56);
    ctx.strokeStyle = pal.roadHalo; ctx.lineWidth = cobbleW + 3.8; ctx.stroke(cobblePath);
    ctx.strokeStyle = pal.roadInk; ctx.lineWidth = cobbleW + 1.4; ctx.stroke(cobblePath);
    ctx.strokeStyle = pal.roadFill; ctx.lineWidth = cobbleW; ctx.stroke(cobblePath);

    // 鎮街鵝卵石塊與郊外雙轍印
    for (var cs = 0; cs < cobbleSegs.length; cs++) {
      var seg = cobbleSegs[cs];
      var dx = seg[3][0] - seg[2][0], dy = seg[3][1] - seg[2][1];
      var len = Math.hypot(dx, dy);
      if (len < 1) continue;
      var nx = -dy / len, ny = dx / len;
      var steps = Math.max(1, Math.round(len / 3.2));
      for (var k = 0; k < steps; k++) {
        var hs = hash2(seg[0] * 19 + k, seg[1] * 23 - k);
        var t = (k + 0.3 + hs * 0.45) / steps;
        var sx = seg[2][0] + dx * t + nx * ((hs - 0.5) * cobbleW * 0.55);
        var sy = seg[2][1] + dy * t + ny * ((hs - 0.5) * cobbleW * 0.55);
        var sr = Math.max(0.85, cobbleW * 0.22);
        ctx.fillStyle = hs > 0.5 ? pal.cobbleLit : pal.cobbleFill;
        ctx.strokeStyle = pal.roadInk; ctx.lineWidth = 0.28;
        ctx.beginPath();
        ctx.ellipse(sx, sy, sr * 1.25, sr * 0.78, Math.atan2(dy, dx) + hs, 0, Math.PI * 2);
        ctx.fill(); ctx.stroke();
      }
    }

    var rutPath = new Path2D();
    for (var ts = 0; ts < trackSegs.length; ts++) {
      var tseg = trackSegs[ts];
      var dx2 = tseg[3][0] - tseg[2][0], dy2 = tseg[3][1] - tseg[2][1];
      var len2 = Math.hypot(dx2, dy2);
      if (len2 < 1) continue;
      var nx2 = (-dy2 / len2) * trackW * 0.22, ny2 = (dx2 / len2) * trackW * 0.22;
      rutPath.moveTo(tseg[2][0] + nx2, tseg[2][1] + ny2); rutPath.lineTo(tseg[3][0] + nx2, tseg[3][1] + ny2);
      rutPath.moveTo(tseg[2][0] - nx2, tseg[2][1] - ny2); rutPath.lineTo(tseg[3][0] - nx2, tseg[3][1] - ny2);
    }
    ctx.strokeStyle = timeMode === 'night' ? 'rgba(20, 22, 28, 0.36)' : 'rgba(92, 68, 42, 0.26)';
    ctx.lineWidth = 0.65; ctx.setLineDash([3, 2]); ctx.stroke(rutPath); ctx.setLineDash([]);

    // 橋樑與海港長棧橋
    for (var bi = 0; bi < bridgeCells.length; bi++) {
      var bx = bridgeCells[bi][0] * z, by = bridgeCells[bi][1] * z;
      ctx.fillStyle = pal.shadow; ctx.fillRect(bx + 1.5, by + 1.8, z, z);
      ctx.fillStyle = pal.bridge; ctx.fillRect(bx + 0.3, by + 0.3, z - 0.6, z - 0.6);
      ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.85;
      ctx.strokeRect(bx + 0.3, by + 0.3, z - 0.6, z - 0.6);
      ctx.strokeStyle = pal.seaRipple; ctx.lineWidth = 0.6;
      ctx.beginPath();
      ctx.moveTo(bx + z + 1, by + z * 0.5);
      ctx.quadraticCurveTo(bx + z + 3, by + z * 0.3, bx + z + 5, by + z * 0.6);
      ctx.stroke();
    }

    // 厚石城牆（含立體影與石垛）
    for (var wi = 0; wi < wallCells.length; wi++) {
      var wxp = wallCells[wi][0] * z, wyp = wallCells[wi][1] * z;
      ctx.fillStyle = pal.shadow; ctx.fillRect(wxp + 1.5, wyp + 2.0, z, z);
      ctx.fillStyle = pal.towerShade; ctx.fillRect(wxp + 0.5, wyp + 0.5, z - 1.0, z - 1.0);
      ctx.fillStyle = pal.towerLit; ctx.fillRect(wxp + 1.2, wyp + 1.0, z - 2.4, z * 0.55);
      ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.85; ctx.strokeRect(wxp + 0.5, wyp + 0.5, z - 1.0, z - 1.0);
    }
    ctx.restore();

    // 4. 森林連片書法樹海 ＋ 獨立闊葉／針葉樹
    var fAna = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) { return terrain[idx] === CELL.TREE; });
    var mergedMask = new Uint8Array(vw * vh);
    var singles = [];
    for (var ly3 = 0; ly3 < vh; ly3++) {
      for (var lx3 = 0; lx3 < vw; lx3++) {
        var li3 = ly3 * vw + lx3;
        var fcid = fAna.compId[li3];
        if (fcid !== -1) {
          if (fAna.comps[fcid].length >= 4) mergedMask[li3] = 1;
          else singles.push([gx0 + lx3, gy0 + ly3]);
        }
      }
    }
    var fField = new Float32Array(vw * vh);
    var anyMergedTree = false;
    for (var fy = 0; fy < vh; fy++) {
      for (var fx = 0; fx < vw; fx++) {
        var accF = 0;
        for (var dyf = -1; dyf <= 1; dyf++) {
          for (var dxf = -1; dxf <= 1; dxf++) {
            var nfx = Math.max(0, Math.min(vw - 1, fx + dxf));
            var nfy = Math.max(0, Math.min(vh - 1, fy + dyf));
            var nli = nfy * vw + nfx;
            var wt = (dxf === 0 && dyf === 0) ? 0.56 : (Math.abs(dxf) + Math.abs(dyf) === 1 ? 0.085 : 0.025);
            var gv = terrain[(gy0 + nfy) * W + (gx0 + nfx)];
            var val = (gv === CELL.SEA || gv === CELL.RIVER || gv === CELL.MOUNTAIN) ? -0.75 : (mergedMask[nli] ? 1.0 : 0.0);
            accF += val * wt;
          }
        }
        fField[fy * vw + fx] = accF + Math.sin(fx * 1.1 + fy * 1.4) * 0.025;
        if (mergedMask[fy * vw + fx]) anyMergedTree = true;
      }
    }
    if (anyMergedTree) {
      var flLoops = extractClosedLoops(fField, vw, vh, 0.48);
      if (flLoops.length) {
        var canopyPath = new Path2D();
        var inkLit = new Path2D();
        var inkMid = new Path2D();
        var inkShade = new Path2D();
        var stepPx = Math.max(5.5, z * 0.55);
        for (var fli = 0; fli < flLoops.length; fli++) {
          var rawL = flLoops[fli];
          var pts = [];
          for (var rp = 0; rp < rawL.length; rp++) pts.push([(gx0 + rawL[rp][0]) * z, (gy0 + rawL[rp][1]) * z]);
          var sampled = [pts[0]];
          var accLen = 0;
          for (var sp = 1; sp < pts.length; sp++) {
            var pa = pts[sp - 1], pb = pts[sp];
            var segLen = Math.hypot(pb[0] - pa[0], pb[1] - pa[1]);
            if (segLen < 0.05) continue;
            var tPrev = 0;
            while (accLen + segLen * (1 - tPrev) >= stepPx) {
              tPrev += (stepPx - accLen) / segLen;
              sampled.push([pa[0] + (pb[0] - pa[0]) * tPrev, pa[1] + (pb[1] - pa[1]) * tPrev]);
              accLen = 0;
            }
            accLen += segLen * (1 - tPrev);
          }
          if (sampled.length < 3) continue;
          canopyPath.moveTo(sampled[0][0], sampled[0][1]);
          for (var si2 = 0; si2 < sampled.length; si2++) {
            var a2 = sampled[si2], b2 = sampled[(si2 + 1) % sampled.length];
            var mx2 = (a2[0] + b2[0]) * 0.5, my2 = (a2[1] + b2[1]) * 0.5;
            var clen = Math.hypot(b2[0] - a2[0], b2[1] - a2[1]);
            if (clen < 0.5) continue;
            var grad2 = sampleGrad(fField, vw, vh, mx2 / z - gx0 - 0.5, my2 / z - gy0 - 0.5);
            var nx3 = -grad2[0], ny3 = -grad2[1];
            var nl = Math.hypot(nx3, ny3);
            if (nl > 1e-4) { nx3 /= nl; ny3 /= nl; }
            else { nx3 = (b2[1] - a2[1]) / clen; ny3 = -(b2[0] - a2[0]) / clen; }
            var bulge = clen * (0.46 + 0.14 * Math.sin(mx2 * 0.14 + my2 * 0.18));
            var cpx = mx2 + nx3 * bulge, cpy = my2 + ny3 * bulge;
            canopyPath.quadraticCurveTo(cpx, cpy, b2[0], b2[1]);
            var dot = nx3 * (-light.dx) + ny3 * (-light.dy);
            var bkt = dot > 0.25 ? inkLit : (dot < -0.22 ? inkShade : inkMid);
            bkt.moveTo(a2[0], a2[1]);
            bkt.quadraticCurveTo(cpx, cpy, b2[0], b2[1]);
          }
          canopyPath.closePath();
        }
        ctx.save();
        ctx.translate(light.sx * 0.85, light.sy * 1.15);
        ctx.fillStyle = pal.forestShadow;
        ctx.fill(canopyPath, 'evenodd');
        ctx.restore();

        ctx.save();
        ctx.clip(canopyPath, 'evenodd');
        ctx.fillStyle = pal.forestOlive;
        ctx.fillRect(gx0 * z, gy0 * z, vw * z, vh * z);
        var fpat = ctx.createPattern(ensureFoliagePattern(pal, timeMode), 'repeat');
        if (fpat) { ctx.fillStyle = fpat; ctx.fillRect(gx0 * z, gy0 * z, vw * z, vh * z); }
        var invPath = new Path2D();
        invPath.rect(gx0 * z - 40, gy0 * z - 40, vw * z + 80, vh * z + 80);
        invPath.addPath(canopyPath);
        if (pal.forestRimLit) {
          ctx.save(); ctx.translate(light.dx * 2.2, light.dy * 2.2); ctx.fillStyle = pal.forestRimLit; ctx.fill(invPath, 'evenodd'); ctx.restore();
        }
        if (pal.forestRimShade) {
          ctx.save(); ctx.translate(-light.dx * 2.8, -light.dy * 3.2); ctx.fillStyle = pal.forestRimShade; ctx.fill(invPath, 'evenodd'); ctx.restore();
        }
        ctx.restore();

        ctx.lineCap = 'round';
        ctx.strokeStyle = pal.forestInk;
        ctx.lineWidth = 0.78; ctx.stroke(inkLit);
        ctx.lineWidth = 1.18; ctx.stroke(inkMid);
        ctx.lineWidth = 1.65; ctx.stroke(inkShade);
      }
    }
    for (var sgi = 0; sgi < singles.length; sgi++) {
      var sx2 = singles[sgi][0], sy2 = singles[sgi][1];
      var scx = (sx2 + 0.5) * z + (hash2(sx2, sy2) - 0.5) * 2.0;
      var scy = (sy2 + 0.65) * z + (hash2(sy2, sx2) - 0.5) * 2.0;
      if (hash2(sx2 + 17, sy2 + 31) > 0.55) drawConiferSingleTree(ctx, scx, scy, z, pal, light);
      else drawBroadleafSingleTree(ctx, scx, scy, z, pal, light);
    }

    // 5. 微觀山巒山峰與皴線
    for (var my4 = gy0; my4 <= gy1; my4 += 2) {
      for (var mx4 = gx0; mx4 <= gx1; mx4 += 2) {
        if (terrain[my4 * W + mx4] !== CELL.MOUNTAIN) continue;
        var px4 = (mx4 + 0.5) * z, py4 = (my4 + 0.85) * z;
        var pw4 = z * (1.2 + hash2(mx4, my4) * 0.45);
        var ph4 = z * (1.5 + hash2(my4, mx4) * 0.5);
        ctx.fillStyle = pal.mtnLit;
        ctx.beginPath(); ctx.moveTo(px4, py4 - ph4); ctx.lineTo(px4 - pw4, py4); ctx.lineTo(px4 + pw4 * 0.1, py4 + 2); ctx.closePath(); ctx.fill();
        ctx.fillStyle = pal.mtnShade;
        ctx.beginPath(); ctx.moveTo(px4, py4 - ph4); ctx.lineTo(px4 + pw4 * 0.1, py4 + 2); ctx.lineTo(px4 + pw4, py4); ctx.closePath(); ctx.fill();
        ctx.strokeStyle = pal.mtnInk; ctx.lineWidth = 1.05;
        ctx.beginPath(); ctx.moveTo(px4 - pw4, py4); ctx.lineTo(px4, py4 - ph4); ctx.lineTo(px4 + pw4, py4); ctx.moveTo(px4, py4 - ph4); ctx.lineTo(px4 + pw4 * 0.1, py4 + 2); ctx.stroke();
      }
    }

    // 6. 四合院石板天井與青石古井 ＋ 2.5D 屋舍（占住屋 + 待售屋）與圓柱石砌塔樓 ＋ 入夜燈火
    var outsideBldg = new Uint8Array(vw * vh);
    var bq = [];
    for (var bx0 = 0; bx0 < vw; bx0++) {
      var topI = bx0, botI = (vh - 1) * vw + bx0;
      if (bldg[gy0 * W + (gx0 + bx0)] === BLDG.NONE) { outsideBldg[topI] = 1; bq.push(topI); }
      if (bldg[gy1 * W + (gx0 + bx0)] === BLDG.NONE && !outsideBldg[botI]) { outsideBldg[botI] = 1; bq.push(botI); }
    }
    for (var by0 = 0; by0 < vh; by0++) {
      var lI = by0 * vw, rI = by0 * vw + (vw - 1);
      if (bldg[(gy0 + by0) * W + gx0] === BLDG.NONE && !outsideBldg[lI]) { outsideBldg[lI] = 1; bq.push(lI); }
      if (bldg[(gy0 + by0) * W + gx1] === BLDG.NONE && !outsideBldg[rI]) { outsideBldg[rI] = 1; bq.push(rI); }
    }
    var bqh = 0;
    while (bqh < bq.length) {
      var curB = bq[bqh++];
      var cxB = curB % vw, cyB = (curB / vw) | 0;
      var nbsB = [
        cxB > 0 ? curB - 1 : -1,
        cxB + 1 < vw ? curB + 1 : -1,
        cyB > 0 ? curB - vw : -1,
        cyB + 1 < vh ? curB + vw : -1
      ];
      for (var nbk = 0; nbk < 4; nbk++) {
        var niB = nbsB[nbk];
        if (niB >= 0 && !outsideBldg[niB] && bldg[(gy0 + ((niB / vw) | 0)) * W + (gx0 + (niB % vw))] === BLDG.NONE) {
          outsideBldg[niB] = 1;
          bq.push(niB);
        }
      }
    }
    var yardAna = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) {
      var gxY = idx % W, gyY = (idx / W) | 0;
      var liY = (gyY - gy0) * vw + (gxY - gx0);
      return bldg[idx] === BLDG.NONE && outsideBldg[liY] === 0;
    });
    if (yardAna.comps.length > 0) {
      ctx.save();
      ctx.lineWidth = 0.6;
      for (var yci = 0; yci < yardAna.comps.length; yci++) {
        var yc = yardAna.comps[yci];
        var sxY = 0, syY = 0;
        for (var yii = 0; yii < yc.length; yii++) {
          var ugx = yc[yii][0], ugy = yc[yii][1];
          sxY += ugx + 0.5; syY += ugy + 0.5;
          var ux = ugx * z, uy = ugy * z;
          ctx.fillStyle = pal.courtyard;
          ctx.fillRect(ux - 0.3, uy - 0.3, z + 0.6, z + 0.6);
          ctx.strokeStyle = pal.courtyardSeam;
          ctx.strokeRect(ux + 0.5, uy + 0.5, z - 1, z - 1);
          if (pal.lantern > 0) {
            ctx.fillStyle = 'rgba(255, 186, 86, ' + (0.24 * pal.lantern).toFixed(3) + ')';
            ctx.fillRect(ux - 0.2, uy - 0.2, z + 0.4, z + 0.4);
          }
        }
        var wcx = (sxY / yc.length) * z, wcy = (syY / yc.length) * z + z * 0.1;
        var wr = Math.max(2.0, z * 0.34);
        ctx.fillStyle = pal.shadow;
        ctx.beginPath(); ctx.arc(wcx + 0.8, wcy + 0.9, wr, 0, Math.PI * 2); ctx.fill();
        ctx.fillStyle = pal.towerLit;
        ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.7;
        ctx.beginPath(); ctx.arc(wcx, wcy, wr, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
        ctx.fillStyle = pal.seaDeep;
        ctx.beginPath(); ctx.arc(wcx, wcy, wr * 0.52, 0, Math.PI * 2); ctx.fill();
      }
      ctx.restore();
    }

    var buildingLights = [];
    var roofAna = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) { return bldg[idx] === BLDG.ROOF; });
    for (var rci = 0; rci < roofAna.comps.length; rci++) {
      renderMergedRoof(ctx, roofAna.comps[rci], roofAna.compId, rci, gx0, gy0, vw, vh, z, dpr, pal, light, timeMode, buildingLights, false);
    }
    var vacAna = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) { return bldg[idx] === BLDG.VACANT; });
    for (var vci = 0; vci < vacAna.comps.length; vci++) {
      renderMergedRoof(ctx, vacAna.comps[vci], vacAna.compId, vci, gx0, gy0, vw, vh, z, dpr, pal, light, timeMode, buildingLights, true);
    }

    var towerAna = analyzeLocalComponents(W, H, gx0, gy0, gx1, gy1, function (idx) { return bldg[idx] === BLDG.TOWER; });
    var towerCenters = [];
    for (var twi = 0; twi < towerAna.comps.length; twi++) {
      var tcells = towerAna.comps[twi];
      var sx3 = 0, sy3 = 0;
      for (var ti = 0; ti < tcells.length; ti++) { sx3 += tcells[ti][0] + 0.5; sy3 += tcells[ti][1] + 0.5; }
      var tcx = (sx3 / tcells.length) * z, tcy = (sy3 / tcells.length) * z;
      var rad = z * (tcells.length === 1 ? 0.50 : (0.50 * Math.sqrt(tcells.length) + 0.08));
      var twH = Math.max(3.0, rad * 0.46);
      ctx.fillStyle = pal.shadow;
      ctx.beginPath(); ctx.arc(tcx + 2, tcy + 2 + twH * 0.85, rad * 0.96, 0, Math.PI * 2); ctx.fill();
      var sGrad = ctx.createLinearGradient(tcx - rad, tcy, tcx + rad, tcy);
      sGrad.addColorStop(0, pal.towerShade); sGrad.addColorStop(0.3, pal.towerLit); sGrad.addColorStop(1, pal.wallBase);
      ctx.fillStyle = sGrad; ctx.strokeStyle = pal.ink; ctx.lineWidth = 1.0;
      ctx.beginPath(); ctx.arc(tcx, tcy + twH, rad * 0.92, 0, Math.PI, false); ctx.lineTo(tcx - rad * 0.92, tcy); ctx.arc(tcx, tcy, rad * 0.92, Math.PI, 0, true); ctx.closePath(); ctx.fill(); ctx.stroke();
      ctx.fillStyle = pal.lantern > 0 ? '#ffcf76' : pal.ink;
      ctx.fillRect(tcx - Math.max(1.2, rad * 0.14), tcy + twH * 0.35, Math.max(2.4, rad * 0.28), twH * 0.48);

      var rGrad = ctx.createRadialGradient(tcx - rad * 0.25, tcy - rad * 0.25, rad * 0.1, tcx, tcy, rad);
      rGrad.addColorStop(0, pal.towerLit); rGrad.addColorStop(1, pal.towerShade);
      ctx.fillStyle = rGrad; ctx.strokeStyle = pal.ink;
      ctx.beginPath(); ctx.arc(tcx, tcy, rad, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      ctx.fillStyle = pal.towerCapLit;
      ctx.beginPath(); ctx.arc(tcx, tcy, rad * 0.64, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      towerCenters.push({ cx: tcx, cy: tcy, rad: rad, twH: twH });
    }

    // 封村遺址斷牆
    for (var y7 = gy0; y7 <= gy1; y7++) {
      for (var x7 = gx0; x7 <= gx1; x7++) {
        if (bldg[y7 * W + x7] === BLDG.RUIN) {
          var rx7 = x7 * z + 1, ry7 = y7 * z + 1;
          ctx.fillStyle = pal.shadow; ctx.fillRect(rx7 + 1.5, ry7 + 1.5, z - 2.5, z * 0.5);
          ctx.fillStyle = pal.wallBase; ctx.strokeStyle = pal.ink; ctx.lineWidth = 0.8;
          ctx.fillRect(rx7, ry7, z - 2.5, z * 0.5);
          ctx.strokeRect(rx7, ry7, z - 2.5, z * 0.5);
        }
      }
    }

    if (pal.lantern > 0) {
      ctx.save();
      var la = pal.lantern;
      for (var li = 0; li < buildingLights.length; li++) {
        var b = buildingLights[li];
        var lg = ctx.createRadialGradient(b.cx, b.cy, 2, b.cx, b.cy, b.span * 1.6);
        lg.addColorStop(0, 'rgba(255, 194, 92, ' + (0.34 * la).toFixed(3) + ')');
        lg.addColorStop(1, 'rgba(255, 130, 40, 0)');
        ctx.fillStyle = lg;
        ctx.beginPath(); ctx.arc(b.cx, b.cy, b.span * 1.6, 0, Math.PI * 2); ctx.fill();
        var wSpots = b.windowLitSpots || [];
        for (var wi2 = 0; wi2 < wSpots.length; wi2++) {
          var ws2 = wSpots[wi2];
          var sg = ctx.createRadialGradient(ws2[0], ws2[1], 0.5, ws2[0], ws2[1], ws2[2]);
          sg.addColorStop(0, 'rgba(255, 226, 136, ' + (0.65 * la).toFixed(3) + ')');
          sg.addColorStop(0.5, 'rgba(255, 168, 64, ' + (0.25 * la).toFixed(3) + ')');
          sg.addColorStop(1, 'rgba(255, 140, 40, 0)');
          ctx.fillStyle = sg;
          ctx.beginPath(); ctx.arc(ws2[0], ws2[1], ws2[2], 0, Math.PI * 2); ctx.fill();
        }
      }
      for (var tli = 0; tli < towerCenters.length; tli++) {
        var tc = towerCenters[tli];
        var tg = ctx.createRadialGradient(tc.cx, tc.cy + tc.twH * 0.4, 2, tc.cx, tc.cy + tc.twH * 0.4, tc.rad * 2.5);
        tg.addColorStop(0, 'rgba(255, 204, 110, ' + (0.44 * la).toFixed(3) + ')');
        tg.addColorStop(1, 'rgba(255, 140, 45, 0)');
        ctx.fillStyle = tg;
        ctx.beginPath(); ctx.arc(tc.cx, tc.cy + tc.twH * 0.4, tc.rad * 2.5, 0, Math.PI * 2); ctx.fill();
      }
      if (bridgeCells.length > 0 && timeMode === 'night') {
        var brEnds = [bridgeCells[0], bridgeCells[bridgeCells.length - 1]];
        for (var bri = 0; bri < brEnds.length; bri++) {
          var brx = (brEnds[bri][0] + 0.5) * z, bry = (brEnds[bri][1] + 0.5) * z;
          var brR = Math.max(10, z * 1.15);
          var bg2 = ctx.createRadialGradient(brx, bry, 1, brx, bry, brR);
          bg2.addColorStop(0, 'rgba(255, 205, 112, 0.44)');
          bg2.addColorStop(1, 'rgba(255, 145, 45, 0)');
          ctx.fillStyle = bg2;
          ctx.beginPath(); ctx.arc(brx, bry, brR, 0, Math.PI * 2); ctx.fill();
        }
      }
      ctx.restore();
    }
  }

  /* ═══ 螢幕空間：書法名牌（碰撞防壓字）、直通出口標記、當前火漆印與八芒羅盤 ═══ */
  function roundRectPath(ctx, x, y, w, h, r) {
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.lineTo(x + w - r, y);
    ctx.quadraticCurveTo(x + w, y, x + w, y + r);
    ctx.lineTo(x + w, y + h - r);
    ctx.quadraticCurveTo(x + w, y + h, x + w - r, y + h);
    ctx.lineTo(x + r, y + h);
    ctx.quadraticCurveTo(x, y + h, x, y + h - r);
    ctx.lineTo(x, y + r);
    ctx.quadraticCurveTo(x, y, x + r, y);
    ctx.closePath();
  }

  function renderLabelsAndCompass(ctx, sub, cam, viewW, viewH, pal, hitTargets, poiTargets, viewOpts) {
    var z = cam.pxPerTile;
    var toScreen = function (gx, gy) {
      return [
        viewW * 0.5 + (gx - cam.cx) * z,
        viewH * 0.5 + (gy - cam.cy) * z
      ];
    };

    var placed = [];
    function overlaps(bx, by, bw, bh) {
      for (var i = 0; i < placed.length; i++) {
        var o = placed[i];
        if (bx < o[0] + o[2] && bx + bw > o[0] && by < o[1] + o[3] && by + bh > o[1]) return true;
      }
      return false;
    }

    var isClose = z >= 4.2;
    var showAllSettlements = !!(viewOpts && viewOpts.showAllSettlements);
    var tierRank = { core_town: 0, band: 1, ruin: 1, built: 1, rural_town: 2, gate: 3, shop: 3, alley: 4 };
    var pois = (sub.pois || []).slice().sort(function (a, b) {
      return (tierRank[a.tier] || 5) - (tierRank[b.tier] || 5);
    });
    ctx.save();
    ctx.textBaseline = 'middle';
    ctx.textAlign = 'center';

    // 優先序：直通鄰接出口 -> 主鎮／部族營寨／遺址／鄉鎮 -> 地標／城門／屋宅
    var adj = sub.adjacent_exits || [];
    if (isClose) {
      for (var ai = 0; ai < adj.length; ai++) {
        var ex = adj[ai];
        var pt = toScreen(ex.gx, ex.gy);
        if (pt[0] < -40 || pt[0] > viewW + 40 || pt[1] < -20 || pt[1] > viewH + 20) continue;
        ctx.font = '700 11.5px "Noto Serif TC", serif';
        var tw = ctx.measureText(ex.label).width + 14;
        var th = 21;
        var nearPlayer = sub.player_pos && Math.hypot(ex.gx - sub.player_pos[0], ex.gy - sub.player_pos[1]) < 1.5;
        var bx = pt[0] - tw * 0.5, by = pt[1] - (nearPlayer ? 36 : 24);
        if (overlaps(bx, by, tw, th)) continue;
        placed.push([bx, by, tw, th]);
        ctx.fillStyle = pal.labelBg;
        ctx.strokeStyle = '#c94c2c';
        ctx.lineWidth = 1.25;
        roundRectPath(ctx, bx, by, tw, th, 5);
        ctx.fill(); ctx.stroke();
        ctx.fillStyle = pal.labelInk;
        ctx.fillText(ex.label, pt[0], by + th * 0.5 + 0.5);
        hitTargets.push({ nid: ex.id, x0: bx, y0: by, x1: bx + tw, y1: by + th });
      }
    }

    for (var pi = 0; pi < pois.length; pi++) {
      var p = pois[pi];
      var show = false;
      if (!isClose) {
        show = p.tier === 'core_town' || p.tier === 'ruin' || p.tier === 'band' || ((showAllSettlements || z >= 1.8) && p.tier === 'rural_town');
      } else {
        show = p.tier === 'gate' || p.tier === 'shop' || p.tier === 'built' || (z >= 7.0 && p.tier === 'alley') || p.tier === 'rural_town' || p.tier === 'ruin' || p.tier === 'band';
      }
      if (!show || !p.label) continue;
      var spt = toScreen(p.gx, p.gy);
      if (spt[0] < 10 || spt[0] > viewW - 10 || spt[1] < 12 || spt[1] > viewH - 12) continue;
      var fSize = p.tier === 'core_town' ? 12.5 : (p.tier === 'alley' ? 10.5 : (p.tier === 'band' ? 11.0 : 11.5));
      ctx.font = '700 ' + fSize + 'px "Noto Serif TC", serif';
      var tw2 = ctx.measureText(p.label).width + 12;
      var th2 = fSize + 8;
      var bx2 = spt[0] - tw2 * 0.5, by2 = spt[1] - (isClose ? 20 : 18);
      if (sub.player_pos && Math.hypot(p.gx - sub.player_pos[0], p.gy - sub.player_pos[1]) < 1.8) {
        by2 = spt[1] - 34;
      }
      if (overlaps(bx2, by2, tw2, th2)) continue;
      placed.push([bx2, by2, tw2, th2]);
      ctx.fillStyle = pal.labelBg;
      ctx.strokeStyle = p.tier === 'band' ? '#b34125' : pal.labelBorder;
      ctx.lineWidth = p.tier === 'band' ? 1.35 : 1.0;
      roundRectPath(ctx, bx2, by2, tw2, th2, 4);
      ctx.fill(); ctx.stroke();
      ctx.fillStyle = pal.labelInk;
      ctx.fillText(p.label, spt[0], by2 + th2 * 0.5 + 0.5);
      if (p.nav_id) {
        hitTargets.push({ nid: p.nav_id, x0: bx2, y0: by2, x1: bx2 + tw2, y1: by2 + th2 });
      }
      if (poiTargets) {
        poiTargets.push({
          id: p.id,
          cell_id: p.cell_id || 0,
          tier: p.tier,
          label: p.label,
          gx: p.gx,
          gy: p.gy,
          x0: bx2,
          y0: by2,
          x1: bx2 + tw2,
          y1: by2 + th2
        });
      }
    }

    // 獨立 ？傳聞 問號批註（赭黃羊皮虛線籤條：不更動底圖地貌，親臨現場後自動消解）
    var rumors = sub.rumor_pins || [];
    for (var ri = 0; ri < rumors.length; ri++) {
      var rp = rumors[ri];
      if (!rp || !rp.label) continue;
      var rpt = toScreen(rp.gx, rp.gy);
      if (rpt[0] < 10 || rpt[0] > viewW - 10 || rpt[1] < 12 || rpt[1] > viewH - 12) continue;
      ctx.font = '700 11px "Noto Serif TC", serif';
      var twR = ctx.measureText(rp.label).width + 14;
      var thR = 20;
      var bxR = rpt[0] - twR * 0.5;
      var byR = rpt[1] - (isClose ? 24 : 20);
      var tries = 0;
      while (overlaps(bxR, byR, twR, thR) && tries < 3) {
        byR -= 22;
        tries++;
      }
      if (overlaps(bxR, byR, twR, thR)) continue;
      placed.push([bxR, byR, twR, thR]);
      ctx.save();
      ctx.fillStyle = pal.lantern > 0 ? 'rgba(66, 48, 28, 0.92)' : 'rgba(252, 238, 202, 0.94)';
      ctx.strokeStyle = '#b84a24';
      ctx.lineWidth = 1.2;
      ctx.setLineDash([4, 2]);
      roundRectPath(ctx, bxR, byR, twR, thR, 4);
      ctx.fill();
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = pal.lantern > 0 ? '#f6dfb2' : '#7b2a14';
      ctx.fillText(rp.label, rpt[0], byR + thR * 0.5 + 0.5);
      ctx.restore();
    }

    // 2.5D 店鋪／民居屋頂直點熱區 ＋ 支巷精簡門號小徽章（徹底消除 8 戶鄰接浮標互相吃字）
    var bHits = sub.building_hits || [];
    if (z >= 6.0 && bHits.length) {
      for (var bhi = 0; bhi < bHits.length; bhi++) {
        var bh = bHits[bhi];
        var bp0 = toScreen(bh.gx0, bh.gy0);
        var bp1 = toScreen(bh.gx1, bh.gy1);
        var bxMin = Math.min(bp0[0], bp1[0]), bxMax = Math.max(bp0[0], bp1[0]);
        var byMin = Math.min(bp0[1], bp1[1]), byMax = Math.max(bp0[1], bp1[1]);
        if (bxMax < 0 || bxMin > viewW || byMax < 0 || byMin > viewH) continue;
        var pad = Math.max(2, Math.min(8, (22 - (bxMax - bxMin)) * 0.5));
        var hPadY = Math.max(2, Math.min(8, (22 - (byMax - byMin)) * 0.5));
        hitTargets.push({
          nid: bh.nid,
          x0: bxMin - pad,
          y0: byMin - hPadY,
          x1: bxMax + pad,
          y1: byMax + hPadY
        });
        if (z >= 9.0 && bh.door_badge) {
          var dcx = (bxMin + bxMax) * 0.5, dcy = (byMin + byMax) * 0.5;
          ctx.font = '700 9.5px "Noto Serif TC", serif';
          var dtw = ctx.measureText(bh.door_badge).width + 6;
          var dth = 13;
          ctx.fillStyle = pal.labelBg;
          ctx.strokeStyle = pal.labelBorder;
          ctx.lineWidth = 0.85;
          roundRectPath(ctx, dcx - dtw * 0.5, dcy - dth * 0.5, dtw, dth, 3);
          ctx.fill();
          ctx.stroke();
          ctx.fillStyle = pal.labelInk;
          ctx.fillText(bh.door_badge, dcx, dcy + 0.5);
        }
      }
    }

    // 旅人當前位置朱紅火漆雙環印記
    if (sub.player_pos) {
      var mp = toScreen(sub.player_pos[0], sub.player_pos[1]);
      ctx.fillStyle = 'rgba(201, 56, 36, 0.22)';
      ctx.strokeStyle = '#d99f38';
      ctx.lineWidth = 2.0;
      ctx.beginPath(); ctx.arc(mp[0], mp[1], 11, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
      ctx.fillStyle = '#c93824';
      ctx.strokeStyle = '#fdf8ee';
      ctx.lineWidth = 1.8;
      ctx.beginPath(); ctx.arc(mp[0], mp[1], 4.8, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
    }

    // 古典八芒航海羅盤玫瑰（固定左下角，防壓上方標題橫條）
    if (viewW >= 240 && viewH >= 220) {
      var cr = viewW < 360 ? 15 : 19;
      var cxR = 28 + cr, cyR = viewH - (viewW < 360 ? 58 : 28) - cr;
      ctx.save();
      ctx.translate(cxR, cyR);
      ctx.globalAlpha = 0.82;
      ctx.strokeStyle = pal.ink; ctx.lineWidth = 1.0;
      ctx.beginPath(); ctx.arc(0, 0, cr, 0, Math.PI * 2); ctx.stroke();
      ctx.beginPath(); ctx.arc(0, 0, cr * 0.78, 0, Math.PI * 2); ctx.setLineDash([2, 2]); ctx.stroke(); ctx.setLineDash([]);
      ctx.fillStyle = '#b33922';
      ctx.beginPath(); ctx.moveTo(0, -cr - 2); ctx.lineTo(3.5, -4); ctx.lineTo(0, -7); ctx.lineTo(-3.5, -4); ctx.closePath(); ctx.fill();
      ctx.fillStyle = pal.ink;
      ctx.beginPath(); ctx.moveTo(0, cr + 2); ctx.lineTo(3.5, 4); ctx.lineTo(0, 7); ctx.lineTo(-3.5, 4); ctx.closePath(); ctx.fill();
      ctx.beginPath(); ctx.moveTo(-cr - 2, 0); ctx.lineTo(-4, -3.5); ctx.lineTo(-7, 0); ctx.lineTo(-4, 3.5); ctx.closePath(); ctx.fill();
      ctx.beginPath(); ctx.moveTo(cr + 2, 0); ctx.lineTo(4, -3.5); ctx.lineTo(7, 0); ctx.lineTo(4, 3.5); ctx.closePath(); ctx.fill();
      ctx.font = '700 10px "Noto Serif TC", serif';
      ctx.fillStyle = pal.labelInk;
      ctx.fillText('北', 0, -cr - 9);
      ctx.restore();
    }
    ctx.restore();
  }

  /* ═══ 地底坑道局部手繪石室草圖（0 外部圖檔、0 未探洩漏） ═══ */
  function renderDungeon(canvas, m, dngCam, opts) {
    var dng = m && m.dungeon_local;
    if (!canvas || !dng || !dng.rooms || !dng.rooms.length) return;
    var rect = canvas.getBoundingClientRect();
    var viewW = Math.max(180, Math.round(rect.width));
    var viewH = Math.max(180, Math.round(rect.height));
    var dpr = Math.min(window.devicePixelRatio || 1, 2);
    if (canvas.width !== Math.round(viewW * dpr) || canvas.height !== Math.round(viewH * dpr)) {
      canvas.width = Math.round(viewW * dpr);
      canvas.height = Math.round(viewH * dpr);
    }
    var ctx = canvas.getContext('2d');
    ctx.save();
    ctx.scale(dpr, dpr);
    ctx.fillStyle = '#15120f';
    ctx.fillRect(0, 0, viewW, viewH);

    var rooms = dng.rooms;
    var byId = {};
    var curRoom = rooms[0];
    for (var i = 0; i < rooms.length; i++) {
      byId[rooms[i].id] = rooms[i];
      if (rooms[i].current) curRoom = rooms[i];
    }
    var stepPx = Math.min(viewW, viewH) * 0.28 * (dngCam ? dngCam.zoom : 1.0);
    var cx = (dngCam && dngCam.cx != null) ? dngCam.cx : curRoom.pos[0];
    var cy = (dngCam && dngCam.cy != null) ? dngCam.cy : curRoom.pos[1];
    function rScreen(r) {
      return [viewW * 0.5 + (r.pos[0] - cx) * stepPx, viewH * 0.5 + (r.pos[1] - cy) * stepPx];
    }

    // 已知甬道與未知迷霧門洞
    ctx.lineCap = 'round';
    for (var ri = 0; ri < rooms.length; ri++) {
      var r = rooms[ri];
      var p0 = rScreen(r);
      var exits = r.exits || [];
      for (var ei = 0; ei < exits.length; ei++) {
        var dst = exits[ei].to ? byId[exits[ei].to] : null;
        if (dst) {
          var p1 = rScreen(dst);
          ctx.strokeStyle = '#3d342a'; ctx.lineWidth = Math.max(10, stepPx * 0.18);
          ctx.beginPath(); ctx.moveTo(p0[0], p0[1]); ctx.lineTo(p1[0], p1[1]); ctx.stroke();
          ctx.strokeStyle = '#6f5f4d'; ctx.lineWidth = Math.max(4, stepPx * 0.08);
          ctx.beginPath(); ctx.moveTo(p0[0], p0[1]); ctx.lineTo(p1[0], p1[1]); ctx.stroke();
        }
      }
    }

    for (var rj = 0; rj < rooms.length; rj++) {
      var rm = rooms[rj];
      var rp = rScreen(rm);
      var bw = Math.max(66, stepPx * 0.66), bh = Math.max(36, stepPx * 0.36);
      if (rm.current) {
        var tg = ctx.createRadialGradient(rp[0], rp[1], 4, rp[0], rp[1], bw * 1.1);
        tg.addColorStop(0, 'rgba(255, 184, 88, 0.32)');
        tg.addColorStop(1, 'rgba(255, 120, 40, 0)');
        ctx.fillStyle = tg;
        ctx.beginPath(); ctx.arc(rp[0], rp[1], bw * 1.1, 0, Math.PI * 2); ctx.fill();
      }
      ctx.fillStyle = rm.current ? '#662b19' : '#29231d';
      ctx.strokeStyle = rm.current ? '#efc268' : '#7b6c58';
      ctx.lineWidth = rm.current ? 2.2 : 1.4;
      roundRectPath(ctx, rp[0] - bw * 0.5, rp[1] - bh * 0.5, bw, bh, 6);
      ctx.fill(); ctx.stroke();

      var title = (rm.name || '').split('・').pop() || rm.name;
      ctx.fillStyle = '#f4ead8';
      ctx.font = '700 11.5px "Noto Serif TC", serif';
      ctx.textAlign = 'center'; ctx.textBaseline = 'middle';
      ctx.fillText(title, rp[0], rp[1]);
    }
    ctx.restore();
    return { hitTargets: [] };
  }

  /* ═══ 公開入口：render(canvas, mapData, cam, viewOpts) ═══ */
  function renderIsland(canvas, m, cam, viewOpts) {
    if (!canvas || !m || !m.substrate) return { hitTargets: [], renderMs: 0 };
    var t0 = performance.now();
    var dec = ensureDecodedSubstrate(m.substrate);
    if (!dec) return { hitTargets: [], renderMs: 0 };

    var rect = canvas.getBoundingClientRect();
    var viewW = Math.max(180, Math.round(rect.width));
    var viewH = Math.max(180, Math.round(rect.height));
    var dpr = Math.min(window.devicePixelRatio || 1, 2);
    var cwPx = Math.round(viewW * dpr);
    var chPx = Math.round(viewH * dpr);
    if (canvas.width !== cwPx || canvas.height !== chPx) {
      canvas.width = cwPx;
      canvas.height = chPx;
    }

    var isNightUi = !!(viewOpts && viewOpts.isNight);
    var slotIdx = viewOpts ? viewOpts.slotIdx : null;
    var timeMode = resolveTimeMode(isNightUi, slotIdx);
    var pal = getPalette(timeMode);
    var light = getLightVector(timeMode);

    var ctx = canvas.getContext('2d');
    ctx.save();
    ctx.scale(dpr, dpr);

    // 1. 螢幕空間固定棉漿宣紙底色＋全視窗統一柔霧基底（消滅 [0..W] 垂直接縫）
    var paper = ensurePaperSurface(viewW, viewH, pal, timeMode);
    ctx.drawImage(paper, 0, 0, viewW, viewH);
    ctx.save();
    ctx.globalAlpha = 0.62;
    ctx.fillStyle = pal.fogFill;
    ctx.fillRect(0, 0, viewW, viewH);
    ctx.restore();

    // 2. 鏡頭座標變換
    var z = cam.pxPerTile;
    var panX = viewW * 0.5 - cam.cx * z;
    var panY = viewH * 0.5 - cam.cy * z;
    ctx.save();
    ctx.translate(panX, panY);
    ctx.scale(z, z);

    // 全島巨觀水彩層（始終襯底，平滑連接遠近鏡頭）
    var macro = ensureMacroCanvas(dec, m, pal, light, timeMode);
    ctx.restore();

    var detailAlpha = Math.max(0, Math.min(1, (z - 4.2) / 2.2));
    if (detailAlpha < 0.99) {
      ctx.save();
      ctx.translate(panX, panY);
      ctx.scale(z, z);
      ctx.imageSmoothingEnabled = z < 1.8;
      ctx.drawImage(macro, 0, 0, dec.W, dec.H);
      ctx.restore();
    }

    // 3. 放大近看時無縫切入視窗裁剪精雕微觀圖層，並全縮放保留鄰接未知虛線輪廓
    if (detailAlpha > 0.02) {
      ctx.save();
      if (detailAlpha < 0.99) ctx.globalAlpha = detailAlpha;
      ctx.translate(viewW * 0.5 - cam.cx * z, viewH * 0.5 - cam.cy * z);
      renderLocalCraftLayer(ctx, dec, cam, viewW, viewH, dpr, pal, light, timeMode);
      renderOutlineCells(ctx, m, pal, z);
      ctx.restore();
    }

    // 4. 書法名牌、直通鄰近出口按鈕、朱紅火漆當前位置、羅盤玫瑰
    var hitTargets = [];
    var poiTargets = [];
    renderLabelsAndCompass(ctx, m.substrate, cam, viewW, viewH, pal, hitTargets, poiTargets, viewOpts);
    ctx.restore();

    return {
      hitTargets: hitTargets,
      poiTargets: poiTargets,
      renderMs: performance.now() - t0
    };
  }

  window.IslandParchmentMap = {
    renderIsland: renderIsland,
    renderDungeon: renderDungeon
  };
})();

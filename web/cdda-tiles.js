// 《奇點世界》CDDA Chibi_Ultica (UltiCa + MSXotto+) 圖塊與即時紙娃娃渲染模組
(() => {
  'use strict';

  const ATLAS_VER = '20261119a02';
  let atlasMeta = null;
  let atlasImg = null;
  let atlasReady = false;
  const readyWaiters = [];

  function notifyReady() {
    window.__swCddaReady = true;
    while (readyWaiters.length) {
      const fn = readyWaiters.shift();
      try { fn(); } catch (_) {}
    }
  }

  (async function loadAtlas() {
    try {
      const res = await fetch(`/assets/cdda/atlas.json?v=${ATLAS_VER}`);
      if (!res.ok) return;
      atlasMeta = await res.json();
      const img = new Image();
      img.onload = () => {
        atlasImg = img;
        atlasReady = true;
        notifyReady();
      };
      img.src = `/assets/cdda/atlas.png?v=${ATLAS_VER}`;
    } catch (_) {}
  })();

  function isReady() {
    return atlasReady && Boolean(atlasImg && atlasMeta && atlasMeta.frames);
  }

  function onReady(fn) {
    if (isReady()) {
      fn();
    } else {
      readyWaiters.push(fn);
    }
  }

  function frameOf(key) {
    return atlasMeta && atlasMeta.frames ? atlasMeta.frames[key] : null;
  }

  // 繪製填滿格 (x0, y0) ~ (x1, y1) 之基礎地塊（共用整數邊界消除縮放毛細縫）
  function drawFillTile(ctx, key, x0, y0, x1, y1) {
    if (!isReady()) return false;
    const fr = frameOf(key);
    if (!fr) return false;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(atlasImg, fr.x, fr.y, fr.w, fr.h, x0, y0, x1 - x0, y1 - y0);
    return true;
  }

  // 以 32px 為基準格單元繪製帶 ox/oy 偏移之物件／人物圖層（支援 32x48 / 32x64 挑高物件）
  function drawSprite32(ctx, key, tileX, tileY, tileW, tileH) {
    if (!isReady()) return false;
    const fr = frameOf(key);
    if (!fr) return false;
    const sx = tileW / 32;
    const sy = tileH / 32;
    const dx = Math.round(tileX + fr.ox * sx);
    const dy = Math.round(tileY + fr.oy * sy);
    const dw = Math.max(1, Math.round(fr.w * sx));
    const dh = Math.max(1, Math.round(fr.h * sy));
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(atlasImg, fr.x, fr.y, fr.w, fr.h, dx, dy, dw, dh);
    return true;
  }

  // 四連通 NSEW 遮罩（世界座標 +my 為北 / 上，+mx 為東 / 右）
  function neighborMask(mx, my, pred) {
    let m = 0;
    if (pred(mx, my + 1)) m |= 1; // North
    if (pred(mx + 1, my)) m |= 2; // East
    if (pred(mx, my - 1)) m |= 4; // South
    if (pred(mx - 1, my)) m |= 8; // West
    return m;
  }

  function isWater(g) {
    return g === 'stream' || g === 'spring' || g === 'deep';
  }

  function pickGrassVariant(h) {
    return `ground_grass_${h & 3}`;
  }

  function drawGroundTile(ctx, g, mx, my, x0, y0, x1, y1, cellMap, hashFn) {
    if (!isReady()) return false;
    const h = hashFn(mx, my);
    if (g === 'red') return drawFillTile(ctx, 'ground_red', x0, y0, x1, y1);
    if (g === 'pebble') return drawFillTile(ctx, 'ground_pebble', x0, y0, x1, y1);
    if (g === 'sand') return drawFillTile(ctx, 'ground_sand', x0, y0, x1, y1);
    if (g === 'mudflat') return drawFillTile(ctx, 'ground_mudflat', x0, y0, x1, y1);
    if (g === 'rock') return drawFillTile(ctx, 'ground_rock', x0, y0, x1, y1);
    if (g === 'cliff') {
      const m = neighborMask(mx, my, (nx, ny) => cellMap.get(`${nx},${ny}`)?.g === 'cliff');
      return drawFillTile(ctx, `ground_cliff_${m}`, x0, y0, x1, y1);
    }
    if (isWater(g)) {
      const m = neighborMask(mx, my, (nx, ny) => {
        const ng = cellMap.get(`${nx},${ny}`)?.g;
        return isWater(ng);
      });
      if (m !== 15) {
        drawFillTile(ctx, pickGrassVariant(h), x0, y0, x1, y1);
      }
      if (g === 'deep') {
        drawFillTile(ctx, `ground_water_sh_${m}`, x0, y0, x1, y1);
        const dm = neighborMask(mx, my, (nx, ny) => cellMap.get(`${nx},${ny}`)?.g === 'deep');
        return drawFillTile(ctx, `ground_water_dp_${dm}`, x0, y0, x1, y1);
      }
      return drawFillTile(ctx, `ground_water_sh_${m}`, x0, y0, x1, y1);
    }
    return drawFillTile(ctx, pickGrassVariant(h), x0, y0, x1, y1);
  }

  function drawInteriorFloor(ctx, isHomeZone, mx, x0, y0, x1, y1) {
    if (!isReady()) return false;
    if (isHomeZone) {
      return drawFillTile(ctx, `ground_tatami_${Math.abs(mx) & 1}`, x0, y0, x1, y1);
    }
    return drawFillTile(ctx, 'ground_floor', x0, y0, x1, y1);
  }

  function drawFloorOrRoad(ctx, kind, mx, my, x0, y0, x1, y1, cellMap) {
    if (!isReady()) return false;
    if (kind === 'paving' || kind === 'lamp') {
      const m = neighborMask(mx, my, (nx, ny) => {
        const s = cellMap.get(`${nx},${ny}`)?.s;
        return s === 'paving' || s === 'lamp' || s === 'arcade';
      });
      return drawFillTile(ctx, `ground_paving_${m}`, x0, y0, x1, y1);
    }
    if (kind === 'arcade') {
      return drawFillTile(ctx, 'ground_floor', x0, y0, x1, y1);
    }
    if (kind === 'bridge') {
      const spanNS =
        cellMap.get(`${mx},${my + 1}`)?.s === 'bridge' ||
        cellMap.get(`${mx},${my - 1}`)?.s === 'bridge' ||
        !(cellMap.get(`${mx - 1},${my}`)?.s === 'bridge' || cellMap.get(`${mx + 1},${my}`)?.s === 'bridge');
      if (spanNS) {
        return drawFillTile(ctx, 'ground_bridge', x0, y0, x1, y1);
      }
      const fr = frameOf('ground_bridge');
      if (!fr) return false;
      ctx.save();
      ctx.translate((x0 + x1) * 0.5, (y0 + y1) * 0.5);
      ctx.rotate(Math.PI / 2);
      ctx.imageSmoothingEnabled = false;
      const w = x1 - x0;
      const h = y1 - y0;
      ctx.drawImage(atlasImg, fr.x, fr.y, fr.w, fr.h, -h * 0.5, -w * 0.5, h, w);
      ctx.restore();
      return true;
    }
    if (kind === 'ridge' || kind === 'herb_seed' || kind === 'herb_plot' || kind === 'crop_seed' || kind === 'crop_plot') {
      drawFillTile(ctx, 'ground_red', x0, y0, x1, y1);
      drawFillTile(ctx, 'farm_ridge', x0, y0, x1, y1);
      return true;
    }
    return false;
  }

  function buildingStyleForCode(bCode) {
    if (bCode === 'amu_smithy' || bCode === 'sanshui_shop' || bCode === 'shuiwei_granary') {
      return { wall: 'wall_brick', roof: 'roof_shingle' };
    }
    if (bCode === 'jian_clinic') {
      return { wall: 'wall_adobe', roof: 'roof_tile' };
    }
    return { wall: 'wall_wood', roof: 'roof_tile' };
  }

  function drawWallTile(ctx, mx, my, x0, y0, x1, y1, bCode, isWallOrDoorFn) {
    if (!isReady()) return false;
    const { wall } = buildingStyleForCode(bCode);
    const m = neighborMask(mx, my, isWallOrDoorFn);
    return drawFillTile(ctx, `${wall}_${m}`, x0, y0, x1, y1);
  }

  function drawDoorTile(ctx, x0, y0, x1, y1, isOpen) {
    if (!isReady()) return false;
    return drawFillTile(ctx, isOpen ? 'door_open' : 'door_closed', x0, y0, x1, y1);
  }

  function drawRoofTile(ctx, mx, my, x0, y0, x1, y1, bCode, isRoofAtFn) {
    if (!isReady()) return false;
    const { roof } = buildingStyleForCode(bCode);
    const m = neighborMask(mx, my, isRoofAtFn);
    return drawFillTile(ctx, `${roof}_${m}`, x0, y0, x1, y1);
  }

  function resolveItemKey(itemOrCode) {
    if (!itemOrCode) return null;
    if (typeof itemOrCode === 'string') {
      const codeMap = {
        bamboo: 'item_bamboo',
        stone: 'item_stone',
        shoot: 'item_shoot',
        meat: 'item_meat',
        roast_shoot: 'item_roast_shoot',
        roast_meat: 'item_roast_meat',
        blade: 'item_blade',
        pick: 'item_pick',
        hat: 'item_hat',
        pack: 'item_pack',
        herb: 'item_herb',
        salve: 'item_salve',
        tuber: 'item_tuber',
        roast_tuber: 'item_roast_tuber',
        alloy_blade: 'item_alloy_blade',
        alloy_pick: 'item_alloy_pick',
      };
      if (codeMap[itemOrCode]) return codeMap[itemOrCode];
    }
    const ch = typeof itemOrCode === 'string' ? itemOrCode : String(itemOrCode.c || itemOrCode.code || '');
    const lbl = typeof itemOrCode === 'string' ? itemOrCode : String(itemOrCode.label || '');
    if (ch === '藥' || ch === 'herb' || /青草藥/.test(lbl)) return 'item_herb';
    if (ch === '膏' || ch === 'salve' || /藥膏/.test(lbl)) return 'item_salve';
    if (ch === '烤' || ch === 'roast_tuber' || /烤甘薯/.test(lbl)) return 'item_roast_tuber';
    if (ch === '薯' || ch === 'tuber' || /甘薯/.test(lbl)) return 'item_tuber';
    if (ch === '笠' || ch === 'hat' || /竹笠/.test(lbl)) return 'item_hat';
    if (ch === '簍' || ch === 'pack' || /背簍/.test(lbl)) return 'item_pack';
    if (ch === '煨' || ch === 'roast_shoot' || /煨/.test(lbl)) return 'item_roast_shoot';
    if (ch === '炙' || ch === 'roast_meat' || /炙/.test(lbl)) return 'item_roast_meat';
    if (ch === '筍' || ch === 'shoot' || /筍/.test(lbl)) return 'item_shoot';
    if (ch === '石' || ch === 'stone' || /石/.test(lbl)) return 'item_stone';
    if (ch === '肉' || ch === 'meat' || /肉/.test(lbl)) return 'item_meat';
    if (ch === '刃' || ch === 'alloy_blade' || /導能獵刀/.test(lbl)) return 'item_alloy_blade';
    if (ch === '鍤' || ch === 'alloy_pick' || /開山鎬/.test(lbl)) return 'item_alloy_pick';
    if (ch === '刀' || ch === 'blade' || /刀|劍/.test(lbl)) return 'item_blade';
    if (ch === '鎬' || ch === '斧' || ch === 'pick' || /鎬|斧/.test(lbl)) return 'item_pick';
    return 'item_bamboo';
  }

  function drawItemSprite(ctx, item, cx, cy, sizePx) {
    if (!isReady()) return false;
    const key = resolveItemKey(item);
    const fr = frameOf(key);
    if (!fr) return false;
    const drawSz = Math.max(12, Math.round(sizePx * 0.92));
    const dx = Math.round(cx - drawSz * 0.5);
    const dy = Math.round(cy - drawSz * 0.5);
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(atlasImg, fr.x, fr.y, fr.w, fr.h, dx, dy, drawSz, drawSz);

    const n = Number(item && item.n);
    if (n > 1 && sizePx >= 15) {
      const txt = `×${n}`;
      ctx.save();
      ctx.font = 'bold 9.5px "Noto Sans TC", sans-serif';
      ctx.textAlign = 'right';
      ctx.textBaseline = 'bottom';
      ctx.lineJoin = 'round';
      ctx.lineWidth = 2.6;
      ctx.strokeStyle = 'rgba(24, 20, 16, 0.88)';
      ctx.strokeText(txt, cx + sizePx * 0.40, cy + sizePx * 0.42);
      ctx.fillStyle = '#f7efe0';
      ctx.fillText(txt, cx + sizePx * 0.40, cy + sizePx * 0.42);
      ctx.restore();
    }
    return true;
  }

  function createItemIconCanvas(itemOrCode, cssPx = 24) {
    const cv = document.createElement('canvas');
    cv.width = cssPx;
    cv.height = cssPx;
    cv.className = 'rim-item-icon';
    cv.style.width = `${cssPx}px`;
    cv.style.height = `${cssPx}px`;
    const draw = () => {
      const c2 = cv.getContext('2d');
      if (!c2 || !isReady()) return;
      c2.clearRect(0, 0, cssPx, cssPx);
      const fr = frameOf(resolveItemKey(itemOrCode));
      if (!fr) return;
      c2.imageSmoothingEnabled = false;
      const pad = 2;
      const box = cssPx - pad * 2;
      c2.drawImage(atlasImg, fr.x, fr.y, fr.w, fr.h, pad, pad, box, box);
    };
    onReady(draw);
    return cv;
  }

  function drawStructureProp(ctx, sKind, mx, my, x0, y0, x1, y1, isIndoor, bCode, nowMs, groundItem, sleeper) {
    if (!isReady()) return false;
    const w = x1 - x0;
    const h = y1 - y0;
    if (sKind === 'campfire') {
      if (isIndoor) {
        drawSprite32(ctx, 'struct_woodstove', x0, y0, w, h);
        const flicker = 0.20 + Math.sin(nowMs * 0.012 + mx) * 0.08;
        ctx.save();
        ctx.fillStyle = `rgba(255, 166, 64, ${flicker})`;
        ctx.beginPath();
        ctx.arc(x0 + w * 0.5, y0 + h * 0.68, w * 0.22, 0, Math.PI * 2);
        ctx.fill();
        ctx.restore();
      } else {
        drawSprite32(ctx, 'struct_firering', x0, y0, w, h);
        const pulse = 0.86 + Math.sin(nowMs * 0.014 + mx) * 0.14;
        ctx.save();
        ctx.globalAlpha = pulse;
        drawSprite32(ctx, 'struct_fire', x0, y0 - h * 0.10, w, h);
        ctx.restore();
      }
      return true;
    }
    if (sKind === 'workbench') {
      if (bCode === 'amu_smithy') {
        return drawSprite32(ctx, 'struct_anvil', x0, y0, w, h);
      }
      if (bCode === 'jian_clinic' || bCode === 'yingfeng_inn' || bCode === 'chunshui_diner' || bCode === 'acha_teahouse') {
        return drawSprite32(ctx, 'struct_counter', x0, y0, w, h);
      }
      return drawSprite32(ctx, 'struct_workbench', x0, y0, w, h);
    }
    if (sKind === 'stockpile') {
      if (isIndoor) {
        drawSprite32(ctx, 'struct_rack', x0, y0, w, h);
      } else {
        ctx.save();
        ctx.globalAlpha = 0.85;
        drawFillTile(ctx, 'ground_floor', x0 + w * 0.06, y0 + h * 0.08, x1 - w * 0.06, y1 - h * 0.08);
        ctx.restore();
      }
      if (groundItem) {
        drawItemSprite(ctx, groundItem, (x0 + x1) * 0.5, (y0 + y1) * 0.44, w * 0.78);
      }
      return true;
    }
    if (sKind === 'bed') {
      drawSprite32(ctx, isIndoor ? 'struct_bed' : 'struct_makeshift_bed', x0, y0, w, h);
      if (sleeper) {
        drawHumanPaperDoll(ctx, sleeper, (x0 + x1) * 0.5, (y0 + y1) * 0.45, w, {
          sleeping: true,
        });
      }
      return true;
    }
    if (sKind === 'lamp') {
      return drawSprite32(ctx, 'struct_street_light', x0, y0, w, h);
    }
    if (sKind === 'herb_seed') return drawSprite32(ctx, 'farm_herb_seed', x0, y0, w, h);
    if (sKind === 'herb_plot') return drawSprite32(ctx, 'farm_herb_mature', x0, y0, w, h);
    if (sKind === 'crop_seed') return drawSprite32(ctx, 'farm_seedling', x0, y0, w, h);
    if (sKind === 'crop_plot') return drawSprite32(ctx, 'farm_harvest', x0, y0, w, h);
    return false;
  }

  function drawNaturalProp(ctx, pType, mx, my, x0, y0, x1, y1, nowMs, shakeRem, hashFn) {
    if (!isReady()) return false;
    const w = x1 - x0;
    const h = y1 - y0;
    const hh = hashFn(mx, my);
    if (pType === 'bamboo') {
      const sway = Math.sin(nowMs * 0.0024 + mx * 0.7 + my * 0.5) * w * 0.025;
      const shake = shakeRem > 0 ? Math.sin(nowMs * 0.06) * w * 0.14 * (shakeRem / 0.28) : 0;
      const sprite = (hh % 5 === 0) ? 'prop_bamboo_young' : 'prop_bamboo_tall';
      return drawSprite32(ctx, sprite, Math.round(x0 + sway + shake), y0, w, h);
    }
    if (pType === 'shoot') return drawSprite32(ctx, 'prop_shoot', x0, y0, w, h);
    if (pType === 'herb') return drawSprite32(ctx, 'prop_herb', x0, y0, w, h);
    if (pType === 'stone') return drawSprite32(ctx, `prop_stone_${hh & 1}`, x0, y0, w, h);
    return false;
  }

  function hashStr(s) {
    let h = 2166136261 >>> 0;
    const str = String(s || '');
    for (let i = 0; i < str.length; i++) {
      h ^= str.charCodeAt(i);
      h = Math.imul(h, 16777619) >>> 0;
    }
    return h >>> 0;
  }

  function buildPaperDollLayers(a, opts = {}) {
    const isFemale = a.gender === 'F' || a.gender === '女';
    const g = isFemale ? 'f' : 'm';
    const role = String(a.role || '');
    const name = String(a.name || '');
    const h = hashStr(`${a.id || 0}|${a.soul_seed || 0}|${name}`);

    const eyes = ['brown', 'amber', 'hazel', 'gray'];
    const eyeCol = eyes[h % eyes.length];

    const isElder = /老農|濟安|醫師/.test(role) || /大伯|濟安/.test(name);
    const mStyles = ['crewcut', 'short', 'short_no_fringe', 'medium', 'buzzcut', 'long'];
    const fStyles = ['long', 'ponytail', 'medium', 'short', 'short_no_fringe'];
    const styles = isFemale ? fStyles : mStyles;
    const style = styles[(h >>> 4) % styles.length];
    const hairCol = isElder ? (((h >>> 8) % 2 === 0) ? 'gray' : 'white') : (((h >>> 8) % 4 === 0) ? 'brown' : 'black');
    let hairKey = `pd_hair_${g}_${style}_${hairCol}`;
    if (!frameOf(hairKey)) {
      hairKey = `pd_hair_${g}_${isFemale ? 'long' : 'short'}_black`;
    }

    let shoes = 'pd_shoes_boots';
    let pants = 'pd_pants_work';
    let shirt = `pd_shirt_${g}_${(h >>> 11) % 6}`;
    let outer = null;
    let gloves = null;

    if (/醫|藥/.test(role)) {
      shoes = 'pd_shoes_dress';
      shirt = `pd_shirt_${g}_1`;
      outer = 'pd_coat_lab';
    } else if (/廚/.test(role)) {
      shoes = 'pd_shoes_dress';
      shirt = `pd_shirt_${g}_0`;
      outer = 'pd_apron_white';
    } else if (/茶|焙/.test(role)) {
      shoes = 'pd_shoes_dress';
      shirt = `pd_shirt_${g}_5`;
      outer = isFemale ? 'pd_apron_floral' : 'pd_apron_white';
    } else if (/鐵|匠|木|工/.test(role)) {
      shoes = 'pd_shoes_hiking';
      pants = 'pd_pants_cargo';
      shirt = `pd_shirt_${g}_3`;
      outer = 'pd_apron_leather';
      gloves = 'pd_gloves_leather';
    } else if (/商|貨郎|宿/.test(role)) {
      shoes = 'pd_shoes_dress';
      pants = 'pd_pants_jeans';
      shirt = `pd_shirt_${g}_4`;
    } else {
      shoes = (h & 1) ? 'pd_shoes_hiking' : 'pd_shoes_boots';
      pants = ((h >>> 2) & 1) ? 'pd_pants_cargo' : 'pd_pants_jeans';
      outer = ((h >>> 3) & 1) ? 'pd_vest_leather' : 'pd_vest_cloth';
    }

    let pack = null;
    if (opts.showPack) {
      if (/商|貨郎/.test(role)) pack = 'pd_pack_travel';
      else if (isFemale && !/獵|農/.test(role)) pack = 'pd_pack_basket';
      else pack = 'pd_pack_wicker';
    }

    const toolStr = String(opts.tool || a.equipped_code || a.tool || '');
    let wield = null;
    if (toolStr && !opts.sleeping) {
      if (toolStr === 'alloy_blade' || /導能獵刀/.test(toolStr)) wield = 'pd_wield_alloy_blade';
      else if (toolStr === 'blade' || /刀|劍/.test(toolStr)) wield = 'pd_wield_blade';
      else if (toolStr === 'alloy_pick' || /開山鎬/.test(toolStr)) wield = 'pd_wield_alloy_pick';
      else if (toolStr === 'pick' || /鎬|斧/.test(toolStr)) wield = 'pd_wield_pick';
    }

    const layers = [];
    if (!opts.noShadow && !opts.sleeping) layers.push('pd_shadow');
    layers.push(isFemale ? 'pd_body_f' : 'pd_body_m');
    layers.push(`pd_eye_${eyeCol}`);
    if (!opts.sleeping) {
      layers.push(shoes);
      layers.push(pants);
    }
    layers.push(shirt);
    if (outer && !opts.sleeping) layers.push(outer);
    if (gloves && !opts.sleeping) layers.push(gloves);
    if (pack && !opts.sleeping) layers.push(pack);
    layers.push(hairKey);
    if (opts.wearHat && !opts.sleeping) layers.push('pd_hat_straw');
    if (wield) layers.push(wield);
    return layers;
  }

  function drawHumanPaperDoll(ctx, a, cx, cy, tileSize, opts = {}) {
    if (!isReady()) return false;
    const layers = buildPaperDollLayers(a, opts);
    const isDowned = a.act === '力竭倒地' && !opts.noDownedRotate;
    const flipX = !isDowned && !opts.noFlip && typeof opts.facing === 'number' && Math.cos(opts.facing) < -0.25;
    const bobY = opts.bobY || 0;

    ctx.save();
    ctx.translate(Math.round(cx), Math.round(cy + bobY));
    if (isDowned) ctx.rotate(-Math.PI / 2);
    if (flipX) ctx.scale(-1, 1);

    // 對齊 32px 底板（角色腳部貼近 tile 底部）
    const baseX = -tileSize * 0.5;
    const baseY = opts.anchorTop ? 0 : -tileSize * 0.66;
    for (const lk of layers) {
      drawSprite32(ctx, lk, baseX, baseY, tileSize, tileSize);
    }
    ctx.restore();
    return true;
  }

  function drawBeast(ctx, a, cx, cy, cellPx, facing, moveBlend, walkPhase) {
    if (!isReady()) return false;
    const isPheasant = a.glyph === '雉' || /雞|雉|禽/.test(a.name || a.role || '');
    const key = isPheasant ? 'beast_pheasant' : 'beast_boar';
    const tileSize = Math.max(18, Math.round(cellPx * (isPheasant ? 0.90 : 1.05)));
    const bob = Math.abs(Math.sin(walkPhase || 0)) * (moveBlend || 0) * (cellPx * 0.06);
    const flipX = typeof facing === 'number' && Math.cos(facing) < -0.2;

    ctx.save();
    ctx.translate(Math.round(cx), Math.round(cy - bob));
    if (flipX) ctx.scale(-1, 1);
    drawSprite32(ctx, key, -tileSize * 0.5, -tileSize * 0.58, tileSize, tileSize);
    ctx.restore();
    return true;
  }

  window.CddaTiles = {
    isReady,
    onReady,
    frameOf,
    drawFillTile,
    drawSprite32,
    drawGroundTile,
    drawInteriorFloor,
    drawFloorOrRoad,
    drawWallTile,
    drawDoorTile,
    drawRoofTile,
    drawStructureProp,
    drawNaturalProp,
    drawItemSprite,
    resolveItemKey,
    createItemIconCanvas,
    drawHumanPaperDoll,
    drawBeast,
    buildPaperDollLayers,
  };
})();

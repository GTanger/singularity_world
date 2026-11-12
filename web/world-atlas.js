(() => {
  'use strict';
  const $ = (id) => document.getElementById(id);
  const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  const fmt = (n) => Number(n).toLocaleString('en-US');

  const LEGENDS = {
    terrain: ['山海實景：20m 真實高程雙線性暈渲與連續水系。', [
      ['#70a85c', '沖積沃土低地（0～60m）'],
      ['#b08a5c', '紅土礫石台地（150～350m）'],
      ['#1c54b2', '湍急界河與山澗'],
      ['#c8d0c8', '對岸未闢霧境'],
      ['#162c62', '外海深水'],
    ]],
    material: ['母岩土層：地表承重與掘地母岩分類。', [
      ['#c45c3c', '紅土礫石層（可起三層）'],
      ['#78be60', '沖積砂質壤土（可起兩層）'],
      ['#ecd88c', '濱海風積沙（僅宜平房）'],
      ['#6e6250', '潮間淤泥灘（無法起樓）'],
      ['#d2dce8', '河床石英卵石'],
      ['#5a5662', '斷層裸岩盤'],
    ]],
    water: ['水脈湧泉：地表水態與地下水深淺分佈。', [
      ['#00ffe6', '斷層湧泉眼'],
      ['#1446be', '湍急深溪（水密界河）'],
      ['#4696eb', '常流山澗'],
      ['#beaa78', '季節乾溝'],
      ['#ece2c4', '乾爽高台（深層地下水）'],
    ]],
    energy: ['散逸能量：各格天然能量基底（80～100）。', [
      ['#e63228', '豐沛匯聚（95～100）'],
      ['#fadc46', '溫潤充盈（88～94）'],
      ['#283c96', '平緩基底（80～87）'],
    ]],
    landmass: ['自然陸塊界：四連通水密界河切割之自然陸塊。', [
      ['#46b45a', '首發開放陸塊 #01'],
      ['#f8faff', '四連通水密界河'],
      ['#a690c4', '北岸扇原台地（未開放）'],
      ['#dcaa5a', '東岸內陸盆地（未開放）'],
      ['#60aabe', '南岸長嶺平原（未開放）'],
    ]],
  };

  let meta = null, mode = 'terrain', canvas, ctx;
  let viewX = 120, viewY = -160, zoom = 0.4;
  const imgs = {};
  let win = null, winTimer = 0, winKey = '';
  let entities = [], entKind = 'all', entQuery = '';
  let origin = { w: 1, h: 1 }, raf = 0, rafFallback = 0, selected = null;
  const pts = new Map();

  const flushDraw = () => {
    if (raf) { cancelAnimationFrame(raf); raf = 0; }
    if (rafFallback) { clearTimeout(rafFallback); rafFallback = 0; }
    draw();
  };
  const need = () => {
    if (raf || rafFallback) return;
    raf = requestAnimationFrame(flushDraw);
    rafFallback = setTimeout(flushDraw, 60);
  };
  const toast = (t) => { const e = $('toast'); e.textContent = t; e.hidden = false; clearTimeout(toast.t); toast.t = setTimeout(() => { e.hidden = true; }, 3500); };
  const getJson = async (u) => { const r = await fetch(u); if (!r.ok) throw new Error(`${u} → ${r.status}`); return r.json(); };

  function resize() {
    const r = window.devicePixelRatio || 1, b = canvas.getBoundingClientRect();
    canvas.width = Math.max(1, Math.round(b.width * r)); canvas.height = Math.max(1, Math.round(b.height * r));
    origin = { w: b.width, h: b.height };
    ctx.setTransform(r, 0, 0, r, 0, 0);
    need();
  }
  const cx = () => origin.w / 2, cy = () => origin.h / 2;
  const toGrid = (sx, sy) => ({ x: Math.round(viewX + (sx - cx()) / zoom), y: Math.round(viewY - (sy - cy()) / zoom) });
  const toScreen = (gx, gy) => ({ x: cx() + (gx - viewX) * zoom, y: cy() - (gy - viewY) * zoom });

  function loadImg(m) {
    if (imgs[m]) return imgs[m];
    const im = new Image(); im.onload = () => { $('hud-load').textContent = ''; need(); };
    im.onerror = () => { $('hud-load').textContent = '底圖載入失敗，請重新整理'; };
    im.src = `/api/substrate/basemap.png?mode=${m}`;
    $('hud-load').textContent = '載入底圖中…';
    return (imgs[m] = im);
  }

  function draw() {
    const W = origin.w, H = origin.h;
    ctx.fillStyle = '#d9cfb6'; ctx.fillRect(0, 0, W, H);
    if (!meta) return;
    const im = loadImg(mode);
    if (im.complete && im.naturalWidth) {
      ctx.imageSmoothingEnabled = zoom < 6;
      ctx.imageSmoothingQuality = 'high';
      const left = cx() + (meta.min_gx - 0.5 - viewX) * zoom, top = cy() - (meta.max_gy + 0.5 - viewY) * zoom;
      ctx.drawImage(im, left, top, meta.width * zoom, meta.height * zoom);
    }
    if (zoom >= 14) drawWindow();
    drawOrigin(); drawEntities(); drawSelected(); drawScale();
    $('hud-zoom').textContent = `縮放 ${zoom < 1 ? zoom.toFixed(2) : zoom.toFixed(1)}×`;
  }

  function haloText(txt, x, y, fill) {
    ctx.lineJoin = 'round';
    ctx.lineWidth = 2.6;
    ctx.strokeStyle = 'rgba(245, 239, 226, 0.88)';
    ctx.strokeText(txt, x, y);
    ctx.fillStyle = fill;
    ctx.fillText(txt, x, y);
  }

  function drawWindow() {
    if (!win) return;
    const lines = zoom >= 14;
    for (const c of win) {
      const p = toScreen(c.x, c.y), s = zoom;
      if (p.x < -s || p.y < -s || p.x > origin.w + s || p.y > origin.h + s) continue;
      if (lines) {
        ctx.lineWidth = 1;
        ctx.strokeStyle = 'rgba(30, 27, 24, 0.22)';
        ctx.strokeRect(p.x - s / 2, p.y - s / 2, s, s);
      }
      if (c.sym) {
        const symPx = Math.min(22, Math.max(11, Math.round(s * 0.42)));
        ctx.font = `${symPx}px serif`;
        ctx.textAlign = 'center';
        ctx.textBaseline = 'middle';
        haloText(c.sym, p.x, zoom >= 64 ? p.y - 2 : p.y + 1, '#1e1b18');
      }
      if (c.cue && zoom < 54) {
        ctx.fillStyle = '#1d4e6e';
        ctx.beginPath();
        ctx.arc(p.x + s / 2 - 4, p.y - s / 2 + 4, 2.2, 0, 7);
        ctx.fill();
      }
      if (zoom >= 36 && c.elev_m != null) {
        const em = Math.round(Number(c.elev_m));
        const etxt = `${em}m`;
        ctx.font = '9px sans-serif';
        ctx.textAlign = 'left';
        ctx.textBaseline = 'top';
        haloText(etxt, p.x - s / 2 + 3, p.y - s / 2 + 2, '#5a5043');
      }
      if (zoom >= 54 && c.cue) {
        ctx.font = '9px serif';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'top';
        haloText(c.cue, p.x + s / 2 - 2, p.y - s / 2 + 2, '#1d4e6e');
      }
      if (zoom >= 64 && c.name) {
        ctx.font = '10px serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'bottom';
        haloText(c.name, p.x, p.y + s / 2 - 2, '#2d5a38');
      }
    }
  }
  function fetchWindow() {
    clearTimeout(winTimer);
    if (zoom < 14) { win = null; return; }
    winTimer = setTimeout(async () => {
      const a = toGrid(0, origin.h), b = toGrid(origin.w, 0);
      let x0 = a.x, x1 = b.x, y0 = a.y, y1 = b.y;
      const mx = Math.round((x0 + x1) / 2), my = Math.round((y0 + y1) / 2);
      if (x1 - x0 > 79) { x0 = mx - 39; x1 = mx + 40; }
      if (y1 - y0 > 79) { y0 = my - 39; y1 = my + 40; }
      const q = `min_x=${x0}&max_x=${x1}&min_y=${y0}&max_y=${y1}`;
      if (q === winKey) return; winKey = q;
      try { const d = await getJson(`/api/substrate/window?${q}`); win = Array.isArray(d) ? d : (d.cells || []); need(); } catch (e) { console.warn(e); }
    }, 80);
  }

  function drawOrigin() {
    const p = toScreen(0, 0);
    const r = zoom >= 24 ? Math.min(zoom * 0.34, 18) : Math.max(7, zoom * 1.2);
    ctx.lineWidth = 2.5; ctx.strokeStyle = '#9e2a2b'; ctx.beginPath(); ctx.arc(p.x, p.y, r, 0, 7); ctx.stroke();
    ctx.lineWidth = 1.2; ctx.strokeStyle = '#f5efe2'; ctx.beginPath(); ctx.arc(p.x, p.y, r + 2.2, 0, 7); ctx.stroke();
    if (zoom < 24) {
      ctx.font = 'bold 11px serif';
      ctx.textAlign = 'left'; ctx.textBaseline = 'middle';
      haloText('世界原點', p.x + r + 6, p.y, '#9e2a2b');
    }
  }
  function drawEntities() {
    pts.clear();
    const showName = zoom >= 3;
    for (const e of entities) {
      if (typeof e.x !== 'number') continue;
      const p = toScreen(e.x, e.y);
      if (p.x < -20 || p.y < -20 || p.x > origin.w + 20 || p.y > origin.h + 20) continue;
      const creature = isCreature(e);
      ctx.fillStyle = creature ? '#8b5a2b' : '#9e2a2b'; ctx.strokeStyle = '#fffaf0'; ctx.lineWidth = 1.5;
      ctx.beginPath(); ctx.arc(p.x, p.y, Math.max(3, Math.min(8, zoom * 0.5)), 0, 7); ctx.fill(); ctx.stroke();
      if (showName && e.name) { ctx.fillStyle = '#1e1b18'; ctx.font = '11px serif'; ctx.textAlign = 'left'; ctx.textBaseline = 'middle'; ctx.fillText(e.name, p.x + 9, p.y); }
    }
  }
  function drawSelected() {
    if (!selected) return;
    const p = toScreen(selected.x, selected.y), s = Math.max(zoom, 10);
    ctx.lineWidth = 2; ctx.strokeStyle = '#9e2a2b'; ctx.strokeRect(p.x - s / 2, p.y - s / 2, s, s);
  }
  function drawScale() {
    const m = (zoom > 0 ? 1 : 1) * 20; // 每格 20 m
    const opts = [[250, '5 km'], [50, '1 km'], [10, '200 m'], [1, '20 m']];
    let pick = opts[opts.length - 1];
    for (const o of opts) if (o[0] * zoom <= 160) { pick = o; break; }
    const w = pick[0] * zoom, x = origin.w - 20 - w, y = origin.h - 22;
    ctx.strokeStyle = '#1e1b18'; ctx.lineWidth = 2; ctx.beginPath();
    ctx.moveTo(x, y - 5); ctx.lineTo(x, y); ctx.lineTo(x + w, y); ctx.lineTo(x + w, y - 5); ctx.stroke();
    ctx.fillStyle = '#1e1b18'; ctx.font = '12px serif'; ctx.textAlign = 'center'; ctx.textBaseline = 'bottom';
    ctx.fillText(pick[1], x + w / 2, y - 6);
    void m;
  }

  function setView(x, y, z) { viewX = x; viewY = y; zoom = Math.min(96, Math.max(0.05, z)); winKey = ''; fetchWindow(); need(); }
  function fit() {
    const spanW = meta ? meta.width * 0.85 : 1740;
    const spanH = meta ? meta.height * 0.92 : 2590;
    const cx0 = meta ? Math.round((meta.min_gx + meta.max_gx) / 2) : 0;
    const cy0 = meta ? Math.round((meta.min_gy + meta.max_gy) / 2) : -128;
    const z = Math.min(origin.w / spanW, origin.h / spanH);
    setView(cx0, cy0, Math.min(0.6, Math.max(0.08, z)));
  }
  function zoomAt(sx, sy, f) {
    const g = { x: viewX + (sx - cx()) / zoom, y: viewY - (sy - cy()) / zoom };
    const nz = Math.min(96, Math.max(0.05, zoom * f));
    zoom = nz; viewX = g.x - (sx - cx()) / zoom; viewY = g.y + (sy - cy()) / zoom;
    winKey = ''; fetchWindow(); need();
  }

  function bind() {
    const ptrs = new Map(); let moved = 0, pinch = 0;
    canvas.addEventListener('pointerdown', (e) => { canvas.setPointerCapture(e.pointerId); ptrs.set(e.pointerId, { x: e.offsetX, y: e.offsetY }); moved = 0; pinch = 0; canvas.classList.add('drag'); });
    canvas.addEventListener('pointermove', (e) => {
      const g = toGrid(e.offsetX, e.offsetY); $('hud-pos').textContent = `(${g.x}, ${g.y})`;
      const p = ptrs.get(e.pointerId); if (!p) return;
      if (ptrs.size === 1) { viewX -= (e.offsetX - p.x) / zoom; viewY += (e.offsetY - p.y) / zoom; moved += Math.abs(e.movementX) + Math.abs(e.movementY); fetchWindow(); }
      p.x = e.offsetX; p.y = e.offsetY;
      if (ptrs.size === 2) {
        const [a, b] = [...ptrs.values()], d = Math.hypot(a.x - b.x, a.y - b.y); moved += 10;
        if (pinch) zoomAt((a.x + b.x) / 2, (a.y + b.y) / 2, d / pinch); pinch = d;
      }
      need();
    });
    const up = (e) => {
      const was = ptrs.has(e.pointerId); ptrs.delete(e.pointerId); pinch = 0;
      if (!ptrs.size) canvas.classList.remove('drag');
      if (was && e.type === 'pointerup' && moved < 5 && !ptrs.size) { const g = toGrid(e.offsetX, e.offsetY); inspect(g.x, g.y); }
    };
    canvas.addEventListener('pointerup', up); canvas.addEventListener('pointercancel', up);
    canvas.addEventListener('wheel', (e) => { e.preventDefault(); zoomAt(e.offsetX, e.offsetY, Math.exp(-e.deltaY * 0.0015)); }, { passive: false });
    $('z-in').onclick = () => zoomAt(cx(), cy(), 1.6); $('z-out').onclick = () => zoomAt(cx(), cy(), 1 / 1.6);
    $('btn-fit').onclick = fit; $('btn-origin').onclick = () => setView(0, 0, 56);
    $('menu-btn').onclick = () => $('sidebar').classList.toggle('open');
    $('insp-close').onclick = () => { $('inspector').classList.add('hidden'); selected = null; need(); };
    document.querySelectorAll('.mode').forEach((b) => { b.onclick = () => setMode(b.dataset.mode); });
    $('btn-refresh').onclick = loadEntities;
    $('ent-search').oninput = (e) => { entQuery = e.target.value.trim().toLowerCase(); renderEntities(); };
    document.querySelectorAll('#ent-filter .chip').forEach((c) => { c.onclick = () => { entKind = c.dataset.k; document.querySelectorAll('#ent-filter .chip').forEach((x) => x.classList.toggle('on', x === c)); renderEntities(); }; });
    window.addEventListener('resize', resize);
  }

  function setMode(m) {
    mode = m;
    document.querySelectorAll('.mode').forEach((b) => b.classList.toggle('active', b.dataset.mode === m));
    const [desc, rows] = LEGENDS[m];
    $('legend').innerHTML = `<p>${esc(desc)}</p>` + rows.map(([c, t]) => `<div class="row"><i style="background:${c}"></i><span>${esc(t)}</span></div>`).join('');
    need();
  }

  const row = (k, v) => (v == null || v === '' ? '' : `<div class="kv"><span>${esc(k)}</span><span>${esc(v)}</span></div>`);
  const signed = (v) => (v == null ? null : `${Number(v) >= 0 ? '+' : ''}${Number(v).toFixed(1)} m`);
  async function inspect(x, y) {
    selected = { x, y }; need();
    const panel = $('inspector'), body = $('insp-body');
    panel.classList.remove('hidden'); body.innerHTML = '<p class="muted">載入中…</p>';
    try {
      const c = await getJson(`/api/substrate/cell?x=${x}&y=${y}`);
      if (!selected || selected.x !== x || selected.y !== y) return;
      const s = c.slots || {};
      const open = c.is_open_landmass === true || c.landmass_open === true || /首發|已開放/.test(c.landmass_label || '');
      const slot = (v) => (Array.isArray(v) ? v.join('、') : v);
      const cards = (c.canvas_cards || []).map((k) => `<div class="card">${esc(typeof k === 'string' ? k : [k.title, k.hint || k.text || k.body || k.desc].filter(Boolean).join('：'))}</div>`).join('');
      const rainText = c.rain_cover_label || (c.rain_cover ? '上空有遮蔽（可避雨）' : '露天無遮');
      body.innerHTML = `<h3><span class="sym">${esc(c.symbol)}</span>${esc(c.micro_name || '無名之地')}</h3>
        <div class="muted">座標 (${x}, ${y})・${esc(c.zone_name || '')}</div>
        <span class="tag ${open ? '' : 'fog'}">${esc(c.landmass_label || (open ? '已開放陸塊' : '對岸未闢霧境'))}</span>
        <div class="sec"><h4>物理地形</h4>${row('高程', signed(c.elev_m))}${row('坡度', c.slope_pct != null ? `${Number(c.slope_pct).toFixed(1)}%` : null)}${row('邊緣感官', c.edge_cue)}</div>
        <div class="sec"><h4>母岩與水脈</h4>${row('母岩', c.material_label)}${row('承重上限', c.max_floors_label)}${row('地表水', c.surface_water_label)}${row('地下水', c.groundwater_label)}${row('散逸能量基底', c.base_energy)}</div>
        <div class="sec"><h4>垂直四層槽位</h4>${row('地下二層 U2', slot(s.u2))}${row('地下一層 U1', slot(s.u1))}${row('地表鋪面 S0', slot(s.s0_pave))}${row('地表伴生 S0', slot(s.s0_companions))}${row('上空 V1', slot(s.v1))}${row('遮雨', rainText)}</div>
        ${c.terrain_line ? `<div class="line">${esc(c.terrain_line)}</div>` : ''}${cards}`;
    } catch (e) { body.innerHTML = `<p class="muted">單格資訊載入失敗：${esc(e.message)}</p>`; }
  }

  const isCreature = (e) => /creature|shanhai|beast/.test(`${e.kind || ''} ${e.id || ''}`);
  function renderEntities() {
    $('st-ent').textContent = `${entities.length} 位`;
    const list = entities.filter((e) => (entKind === 'all' || (entKind === 'creature') === isCreature(e)) &&
      (!entQuery || `${e.name || ''} ${e.title || ''} ${e.activity || ''}`.toLowerCase().includes(entQuery))).slice(0, 200);
    const box = $('ent-list'); box.innerHTML = '';
    for (const e of list) {
      const b = document.createElement('button'); b.className = 'ent';
      b.innerHTML = `${esc(e.name || e.id)}<small>${esc([e.title, e.activity].filter(Boolean).join('・'))} (${e.x}, ${e.y})</small>`;
      b.onclick = () => { setView(e.x, e.y, Math.max(zoom, 48)); inspect(e.x, e.y); $('sidebar').classList.remove('open'); };
      box.appendChild(b);
    }
    if (!list.length) box.innerHTML = '<span class="muted">沒有符合的在場實體</span>';
  }
  async function loadEntities() {
    try { const d = await getJson('/api/eco/entities'); entities = Array.isArray(d) ? d : []; renderEntities(); need(); } catch (e) { console.warn('人物位置更新失敗', e); }
  }

  async function init() {
    canvas = $('atlas-canvas'); ctx = canvas.getContext('2d');
    bind(); resize(); setMode('terrain'); fit();
    loadEntities(); setInterval(loadEntities, 8000);
    try {
      meta = await getJson('/api/substrate/meta');
      if (!meta.available) { toast('地形母圖尚未就緒'); return; }
      $('st-cells').textContent = fmt(meta.landmass_01_cells);
      $('st-area').textContent = `${meta.landmass_01_area_km2} km²`;
      $('st-spring').textContent = `${meta.spring_count} 處`;
      $('landmarks').innerHTML = '';
      for (const l of meta.landmarks || []) {
        const lx = l.gx ?? l.x, ly = l.gy ?? l.y;
        const b = document.createElement('button'); b.className = 'pill lm';
        b.innerHTML = `${esc(l.name)}<small>${esc(l.badge || '')}</small>`; b.title = l.desc || '';
        const targetZ = (lx === 0 && (ly === 0 || ly === 1)) ? Math.max(l.zoom || 56, 56) : (l.zoom || 24);
        b.onclick = () => { setView(lx, ly, targetZ); inspect(lx, ly); $('sidebar').classList.remove('open'); };
        $('landmarks').appendChild(b);
      }
      fit(); loadImg(mode);
    } catch (e) { toast(`地圖資訊載入失敗：${e.message}`); }
  }
  window.addEventListener('DOMContentLoaded', init);
})();

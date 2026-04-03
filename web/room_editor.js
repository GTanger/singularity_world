// ================================================================
// room_editor.js v2.0.0 — Canvas 六角格編輯器（對接 /api/hex/）
// ================================================================

// ── 常數 ─────────────────────────────────────────────────────────
const HEX_R = 28;
const HEX_SQRT3 = Math.sqrt(3);
const ZOOM_MIN = 0.15;
const ZOOM_MAX = 5.0;
const ZOOM_STEP = 0.1;

// 預計算六角頂點偏移（flat-top，避免每格 12 次三角函數）
const HEX_VERTS = Array.from({ length: 6 }, (_, i) => {
  const a = Math.PI / 3 * i - Math.PI / 6;
  return { dx: Math.cos(a), dy: Math.sin(a) };
});

const HEX_DIRECTIONS = [
  { dq: 1, dr: 0, label: '東' },
  { dq: 1, dr: -1, label: '東北' },
  { dq: 0, dr: -1, label: '西北' },
  { dq: -1, dr: 0, label: '西' },
  { dq: -1, dr: 1, label: '西南' },
  { dq: 0, dr: 1, label: '東南' },
];

const TERRAIN_FILL = {
  water: '#1a3a5c', water_deep: '#12304e',
  forest: '#1a4a2e', forest_heavy: '#14402a', forest_light: '#1e5434',
  mountain: '#3a3a44', hills: '#2e3e34',
  desert: '#5c4a2a', swamp: '#2a3e34',
  tundra: '#2a3a4a', grassland: '#2a4a2e', jungle: '#1a3a1e',
  plain: '#222e38', urban: '#2a2e3a', road: '#2e3238',
  default: '#1e2838',
};

// ── 狀態 ─────────────────────────────────────────────────────────
const state = {
  cells: new Map(),     // "q,r" → cell object
  barriers: new Set(),  // "q,r,dir" strings
  portals: [],
  selectedCoord: null,  // {q,r}
  hoverHex: null,
  mode: 'move',         // move | link-one | link-two
  linkFrom: null,       // {q,r}
  showEdges: true,
  raf: 0,
};

const cam = { x: 0, y: 0, zoom: 1.0 };
const drag = { active: false, moved: false, sx: 0, sy: 0, cx: 0, cy: 0, pid: -1 };
const pinch = { active: false, dist0: 0, zoom0: 0 };
const pointers = new Map();

// ── UI ───────────────────────────────────────────────────────────
const canvas = document.getElementById('hex-canvas');
const ctx = canvas.getContext('2d');
const $ = id => document.getElementById(id);
const ui = {
  wrap: $('wrap'), panel: $('editor-panel'), status: $('status'),
  fId: $('f-id'), fName: $('f-name'), fZone: $('f-zone'),
  fTags: $('f-tags'), fDesc: $('f-desc'), fObjects: $('f-objects'),
  objectsForm: $('objects-form'), zoneFilter: $('zone-filter'),
  ctxMenu: $('ctx-menu'), ctxCreate: $('ctx-create-room'), ctxImport: $('ctx-import-watabou'),
  watabouInput: $('watabou-json-input'),
  pathFrom: $('path-from'), pathTo: $('path-to'),
  pathDir: $('path-dir'), pathReverse: $('path-reverse'),
  pathReverseDir: $('path-reverse-dir'),
};

// ── Hex 數學 ─────────────────────────────────────────────────────
function axialToPixel(q, r) {
  return { x: HEX_R * HEX_SQRT3 * (q + r / 2), y: HEX_R * 1.5 * r };
}
function cubeRound(qf, rf) {
  const sf = -qf - rf;
  let rq = Math.round(qf), rr = Math.round(rf), rs = Math.round(sf);
  const dq = Math.abs(rq - qf), dr = Math.abs(rr - rf), ds = Math.abs(rs - sf);
  if (dq > dr && dq > ds) rq = -rr - rs;
  else if (dr > ds) rr = -rq - rs;
  return { q: rq, r: rr };
}
function pixelToAxial(x, y) {
  return cubeRound((HEX_SQRT3 / 3 * x - y / 3) / HEX_R, (2 / 3 * y) / HEX_R);
}
function hk(q, r) { return `${q},${r}`; }

// ── Camera ───────────────────────────────────────────────────────
function w2s(wx, wy) {
  const dpr = window.devicePixelRatio || 1;
  const cw = canvas.width / dpr, ch = canvas.height / dpr;
  return { x: (wx - cam.x) * cam.zoom + cw / 2, y: (wy - cam.y) * cam.zoom + ch / 2 };
}
function s2w(sx, sy) {
  const dpr = window.devicePixelRatio || 1;
  const cw = canvas.width / dpr, ch = canvas.height / dpr;
  return { x: (sx - cw / 2) / cam.zoom + cam.x, y: (sy - ch / 2) / cam.zoom + cam.y };
}
function s2hex(sx, sy) { const w = s2w(sx, sy); return pixelToAxial(w.x, w.y); }

// ── Canvas 尺寸 ─────────────────────────────────────────────────
function resizeCanvas() {
  const rect = ui.wrap.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  canvas.width = rect.width * dpr;
  canvas.height = rect.height * dpr;
  canvas.style.width = rect.width + 'px';
  canvas.style.height = rect.height + 'px';
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

// ── 渲染（批次繪製 + LOD）────────────────────────────────────────
function addHex(cx, cy, r) {
  ctx.moveTo(cx + r * HEX_VERTS[0].dx, cy + r * HEX_VERTS[0].dy);
  for (let i = 1; i < 6; i++) ctx.lineTo(cx + r * HEX_VERTS[i].dx, cy + r * HEX_VERTS[i].dy);
  ctx.closePath();
}

function render() {
  const dpr = window.devicePixelRatio || 1;
  const w = canvas.width / dpr, h = canvas.height / dpr;
  ctx.clearRect(0, 0, w, h);

  const sR = HEX_R * cam.zoom;
  const drawGrid = sR >= 4;
  const drawLabels = sR >= 12;

  // 可見範圍
  const tl = s2w(0, 0), br = s2w(w, h);
  const pad = HEX_R * 2;
  const minR = Math.floor((tl.y - pad) / (1.5 * HEX_R)) - 1;
  const maxR = Math.ceil((br.y + pad) / (1.5 * HEX_R)) + 1;
  const minQ = Math.floor((tl.x - pad) / (HEX_SQRT3 * HEX_R)) - 2;
  const maxQ = Math.ceil((br.x + pad) / (HEX_SQRT3 * HEX_R)) + 2;

  // 收集分類
  const emptyList = [];
  const filledByColor = new Map();
  const labelList = [];
  let hoverSp = null, selectedSp = null, linkFromSp = null;

  for (let r = minR; r <= maxR; r++) {
    for (let q = minQ; q <= maxQ; q++) {
      const wp = axialToPixel(q, r);
      const sp = w2s(wp.x, wp.y);
      if (sp.x < -sR * 1.5 || sp.x > w + sR * 1.5 || sp.y < -sR * 1.5 || sp.y > h + sR * 1.5) continue;

      const cell = state.cells.get(hk(q, r));
      const isHover = state.hoverHex && state.hoverHex.q === q && state.hoverHex.r === r;
      const isSel = state.selectedCoord && state.selectedCoord.q === q && state.selectedCoord.r === r;
      const isLinkFrom = state.linkFrom && state.linkFrom.q === q && state.linkFrom.r === r;

      if (cell) {
        const color = TERRAIN_FILL[cell.terrain] || TERRAIN_FILL.default;
        if (!filledByColor.has(color)) filledByColor.set(color, []);
        filledByColor.get(color).push(sp);
        if (drawLabels) labelList.push({ sp, name: cell.name || `${q},${r}`, isSel });
      } else if (drawGrid) {
        emptyList.push(sp);
      }

      if (isHover) hoverSp = sp;
      if (isSel) selectedSp = sp;
      if (isLinkFrom) linkFromSp = sp;
    }
  }

  // 1. 空白格（一次 fill + stroke）
  if (emptyList.length > 0) {
    ctx.beginPath();
    for (const sp of emptyList) addHex(sp.x, sp.y, sR);
    ctx.fillStyle = 'rgba(255,255,255,0.015)';
    ctx.fill();
    ctx.strokeStyle = 'rgba(255,255,255,0.07)';
    ctx.lineWidth = 0.5;
    ctx.stroke();
  }

  // 2. 已填充格（按顏色批次）
  for (const [color, list] of filledByColor) {
    ctx.beginPath();
    for (const sp of list) addHex(sp.x, sp.y, sR);
    ctx.fillStyle = color;
    ctx.fill();
    ctx.strokeStyle = '#3a4a5c';
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  // 3. 標籤
  if (drawLabels && labelList.length > 0) {
    const fontSize = Math.max(8, Math.min(14, 11 * cam.zoom));
    ctx.font = `600 ${fontSize}px "Noto Sans TC", sans-serif`;
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    for (const { sp, name, isSel } of labelList) {
      ctx.fillStyle = isSel ? '#e0f2fe' : '#b0c4de';
      ctx.fillText(name.slice(0, 3), sp.x, sp.y);
    }
  }

  // 4. Hover
  if (hoverSp) {
    ctx.beginPath();
    addHex(hoverSp.x, hoverSp.y, sR);
    ctx.fillStyle = 'rgba(125,211,252,0.10)';
    ctx.fill();
    ctx.strokeStyle = 'rgba(125,211,252,0.5)';
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  // 5. 選取
  if (selectedSp) {
    ctx.beginPath();
    addHex(selectedSp.x, selectedSp.y, sR + 2);
    ctx.strokeStyle = '#7dd3fc';
    ctx.lineWidth = 2.5;
    ctx.stroke();
  }

  // 6. 連線起點
  if (linkFromSp) {
    ctx.beginPath();
    addHex(linkFromSp.x, linkFromSp.y, sR + 1);
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 2;
    ctx.stroke();
  }

  // 7. 連線預覽
  if (linkFromSp && hoverSp && linkFromSp !== selectedSp) {
    ctx.beginPath();
    ctx.moveTo(linkFromSp.x, linkFromSp.y);
    ctx.lineTo(hoverSp.x, hoverSp.y);
    ctx.strokeStyle = '#fbbf24';
    ctx.lineWidth = 1.5;
    ctx.setLineDash([6, 4]);
    ctx.stroke();
    ctx.setLineDash([]);
  }

  // 8. 狀態列
  const zoomPct = Math.round(cam.zoom * 100);
  const coordTxt = state.hoverHex ? `(${state.hoverHex.q},${state.hoverHex.r})` : '';
  const modeTxt = state.mode === 'move' ? '選取' : state.mode === 'link-one' ? '單向連線' : '雙向連線';
  setStatus(`${modeTxt} ｜ ${zoomPct}% ｜ ${coordTxt} ｜ ${state.cells.size} 格`);
  const zb = $('btn-zoom-reset');
  if (zb) zb.textContent = `${zoomPct}%`;
}

function scheduleRender() {
  if (state.raf) return;
  state.raf = requestAnimationFrame(() => { state.raf = 0; render(); });
}

// ── 互動事件 ─────────────────────────────────────────────────────
function canvasXY(ev) {
  const r = canvas.getBoundingClientRect();
  return { x: ev.clientX - r.left, y: ev.clientY - r.top };
}

function pDist() {
  if (pointers.size < 2) return 0;
  const pts = Array.from(pointers.values());
  return Math.hypot(pts[1].x - pts[0].x, pts[1].y - pts[0].y);
}

canvas.addEventListener('pointerdown', ev => {
  const p = canvasXY(ev);
  pointers.set(ev.pointerId, p);
  if (pointers.size === 2) {
    drag.active = false;
    pinch.active = true;
    pinch.dist0 = Math.max(1, pDist());
    pinch.zoom0 = cam.zoom;
    return;
  }
  if (ev.button <= 1) {
    canvas.setPointerCapture(ev.pointerId);
    drag.active = true; drag.moved = false;
    drag.sx = p.x; drag.sy = p.y;
    drag.cx = cam.x; drag.cy = cam.y;
    drag.pid = ev.pointerId;
  }
});

canvas.addEventListener('pointermove', ev => {
  const p = canvasXY(ev);
  pointers.set(ev.pointerId, p);
  if (pinch.active && pointers.size >= 2) {
    cam.zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, pinch.zoom0 * pDist() / pinch.dist0));
    scheduleRender();
    return;
  }
  if (drag.active && drag.pid === ev.pointerId) {
    const dx = p.x - drag.sx, dy = p.y - drag.sy;
    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) drag.moved = true;
    cam.x = drag.cx - dx / cam.zoom;
    cam.y = drag.cy - dy / cam.zoom;
    scheduleRender();
    return;
  }
  const hex = s2hex(p.x, p.y);
  if (!state.hoverHex || state.hoverHex.q !== hex.q || state.hoverHex.r !== hex.r) {
    state.hoverHex = hex;
    scheduleRender();
  }
});

canvas.addEventListener('pointerup', ev => {
  pointers.delete(ev.pointerId);
  if (pointers.size < 2) pinch.active = false;
  if (drag.active && drag.pid === ev.pointerId) {
    try { canvas.releasePointerCapture(ev.pointerId); } catch (_) {}
    const wasDrag = drag.moved;
    drag.active = false;
    if (!wasDrag && ev.button === 0) handleClick(canvasXY(ev));
  }
});

canvas.addEventListener('pointercancel', ev => {
  pointers.delete(ev.pointerId);
  if (pointers.size < 2) pinch.active = false;
  drag.active = false;
});

canvas.addEventListener('pointerleave', () => {
  if (state.hoverHex) { state.hoverHex = null; scheduleRender(); }
});

canvas.addEventListener('wheel', ev => {
  ev.preventDefault();
  const p = canvasXY(ev);
  const before = s2w(p.x, p.y);
  cam.zoom = Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, cam.zoom * (ev.deltaY > 0 ? 0.88 : 1.14)));
  const after = s2w(p.x, p.y);
  cam.x += before.x - after.x;
  cam.y += before.y - after.y;
  scheduleRender();
}, { passive: false });

canvas.addEventListener('contextmenu', ev => {
  ev.preventDefault();
  showCtx(ev.clientX, ev.clientY, s2hex(canvasXY(ev).x, canvasXY(ev).y));
});

function handleClick(p) {
  hideCtx();
  const hex = s2hex(p.x, p.y);
  const key = hk(hex.q, hex.r);
  const cell = state.cells.get(key);

  if (state.mode === 'link-one' || state.mode === 'link-two') {
    if (!cell) return;
    if (!state.linkFrom) {
      state.linkFrom = hex;
      setStatus(`連線起點：${cell.name}，再點一格作終點`);
      scheduleRender();
    } else {
      const from = state.linkFrom;
      state.linkFrom = null;
      if (from.q !== hex.q || from.r !== hex.r) completeWall(from, hex);
      scheduleRender();
    }
    return;
  }

  if (cell) {
    selectCell(hex);
  } else {
    createCell(hex.q, hex.r);
  }
}

// ── 右鍵選單 ─────────────────────────────────────────────────────
let ctxHex = null;

function showCtx(cx, cy, hex) {
  ctxHex = hex;
  const rect = ui.wrap.getBoundingClientRect();
  ui.ctxMenu.style.left = (cx - rect.left + 4) + 'px';
  ui.ctxMenu.style.top = (cy - rect.top + 4) + 'px';
  ui.ctxMenu.style.display = 'block';
  const exists = state.cells.has(hk(hex.q, hex.r));
  ui.ctxCreate.textContent = exists ? `已有格子 (${hex.q},${hex.r})` : `在 (${hex.q},${hex.r}) 建立格子`;
  ui.ctxCreate.disabled = exists;
}
function hideCtx() { ui.ctxMenu.style.display = 'none'; }
document.addEventListener('click', ev => { if (!ui.ctxMenu.contains(ev.target)) hideCtx(); });
ui.ctxCreate.onclick = () => { hideCtx(); if (ctxHex) createCell(ctxHex.q, ctxHex.r); };

// ── API ──────────────────────────────────────────────────────────
async function api(path, opt = {}) {
  const res = await fetch(path, { headers: { 'Content-Type': 'application/json' }, ...opt });
  const txt = await res.text();
  let body = {};
  try { body = txt ? JSON.parse(txt) : {}; } catch (_) {}
  if (!res.ok) throw new Error(body.error || `HTTP ${res.status}`);
  return body;
}

async function loadGrid(centerView = true) {
  const data = await api('/api/hex/grid');
  state.cells.clear();
  state.barriers.clear();
  for (const c of (data.cells || [])) {
    state.cells.set(hk(c.coord.q, c.coord.r), { ...c, q: c.coord.q, r: c.coord.r });
  }
  for (const b of (data.barriers || [])) {
    state.barriers.add(`${b.coord.q},${b.coord.r},${b.dir}`);
  }
  state.portals = data.portals || [];

  if (centerView && state.cells.size > 0) {
    let sx = 0, sy = 0, n = 0;
    for (const c of state.cells.values()) {
      const p = axialToPixel(c.q, c.r);
      sx += p.x; sy += p.y; n++;
    }
    if (n) { cam.x = sx / n; cam.y = sy / n; }
  }

  refreshPathSelects();
  refreshZoneFilter();
  scheduleRender();
}

// ── Cell CRUD ────────────────────────────────────────────────────
async function createCell(q, r) {
  try {
    await api('/api/hex/cell', {
      method: 'PUT',
      body: JSON.stringify({ q, r, terrain: 'plain', name: `(${q},${r})` }),
    });
    await loadGrid(false);
    selectCell({ q, r });
    setStatus(`已建立 (${q},${r})`);
  } catch (e) { setStatus(`建立失敗：${e.message}`, true); }
}

async function deleteSelected() {
  if (!state.selectedCoord) { setStatus('請先選取', true); return; }
  const { q, r } = state.selectedCoord;
  if (!confirm(`刪除 (${q},${r})？`)) return;
  try {
    await api(`/api/hex/cell/${q}/${r}`, { method: 'DELETE' });
    state.selectedCoord = null;
    await loadGrid(false);
    setStatus(`已刪除 (${q},${r})`);
  } catch (e) { setStatus(`刪除失敗：${e.message}`, true); }
}

function selectCell(coord) {
  state.selectedCoord = coord;
  const cell = state.cells.get(hk(coord.q, coord.r));
  if (cell) fillEditor(cell);
  refreshPathSelects();
  scheduleRender();
}

function fillEditor(cell) {
  ui.fId.value = `(${cell.q},${cell.r})`;
  ui.fName.value = cell.name || '';
  ui.fZone.value = cell.zone || '';
  ui.fTags.value = (cell.tags || []).join(', ');
  ui.fDesc.value = cell.description || '';
  writeObjectsJson(cell.objects || []);
  renderObjectsForm(cell.objects || []);
}

async function saveCurrentCell() {
  if (!state.selectedCoord) { setStatus('請先選取', true); return; }
  const { q, r } = state.selectedCoord;
  let objects = [];
  try { objects = readObjectsFromJson(); } catch (e) { setStatus(`JSON 格式錯：${e.message}`, true); return; }
  const cell = state.cells.get(hk(q, r));
  try {
    await api('/api/hex/cell', {
      method: 'PUT',
      body: JSON.stringify({
        q, r,
        terrain: cell?.terrain || 'plain',
        name: ui.fName.value,
        zone: ui.fZone.value,
        tags: parseTags(ui.fTags.value),
        description: ui.fDesc.value,
        objects,
      }),
    });
    setStatus('已儲存');
    await loadGrid(false);
    selectCell({ q, r });
  } catch (e) { setStatus(`儲存失敗：${e.message}`, true); }
}

async function completeWall(from, to) {
  try {
    await api('/api/hex/wall', {
      method: 'PUT',
      body: JSON.stringify({ aq: from.q, ar: from.r, bq: to.q, br: to.r }),
    });
    await loadGrid(false);
    setStatus('牆壁已建立');
  } catch (e) { setStatus(`失敗：${e.message}`, true); }
}

// ── Watabou 匯入 ────────────────────────────────────────────────
async function importWatabouAtAnchor(anchor, jsonText) {
  const parsed = JSON.parse(jsonText);
  const hexes = parsed?.hexes;
  if (!hexes || typeof hexes !== 'object') throw new Error('不是 Watabou hex JSON');
  const cells = [];
  for (const cell of Object.values(hexes)) {
    if (!cell || !Number.isFinite(cell.q) || !Number.isFinite(cell.r)) continue;
    cells.push({ q: Number(cell.q), r: Number(cell.r), terrain: String(cell.terrain || 'plain') });
  }
  if (!cells.length) throw new Error('無 hexes');
  const minQ = Math.min(...cells.map(c => c.q));
  const minR = Math.min(...cells.map(c => c.r));

  const batch = cells.map(src => {
    const tq = anchor.q + (src.q - minQ);
    const tr = anchor.r + (src.r - minR);
    return { q: tq, r: tr, terrain: src.terrain, name: `(${tq},${tr}) ${src.terrain}`, zone: src.terrain.split('-')[0] || 'wild', tags: ['watabou', src.terrain] };
  });

  setStatus(`匯入 ${batch.length} 格...`);
  await api('/api/hex/cells', { method: 'PUT', body: JSON.stringify(batch) });
  await loadGrid(false);
  setStatus(`匯入完成：${batch.length} 格`);
}

if (ui.ctxImport && ui.watabouInput) {
  ui.ctxImport.onclick = () => {
    hideCtx();
    if (!ctxHex) return;
    ui.watabouInput.dataset.q = ctxHex.q;
    ui.watabouInput.dataset.r = ctxHex.r;
    ui.watabouInput.value = '';
    ui.watabouInput.click();
  };
  ui.watabouInput.onchange = async () => {
    const f = ui.watabouInput.files?.[0];
    if (!f) return;
    const q = Number(ui.watabouInput.dataset.q), r = Number(ui.watabouInput.dataset.r);
    try { await importWatabouAtAnchor({ q, r }, await f.text()); } catch (e) { setStatus(`匯入失敗：${e.message}`, true); }
  };
}

// ── 工具函式 ─────────────────────────────────────────────────────
function setStatus(msg, isError = false) {
  ui.status.style.color = isError ? '#fda4af' : '#8f9bb3';
  ui.status.textContent = msg;
}
function parseTags(s) { return (s || '').split(',').map(t => t.trim()).filter(Boolean); }

// ── Zone 濾鏡 ────────────────────────────────────────────────────
function refreshZoneFilter() {
  const zones = new Map();
  for (const c of state.cells.values()) { if (c.zone) zones.set(c.zone, (zones.get(c.zone) || 0) + 1); }
  ui.zoneFilter.innerHTML = '<option value="">全部顯示</option>';
  for (const [z, n] of Array.from(zones).sort((a, b) => a[0].localeCompare(b[0]))) {
    const o = document.createElement('option'); o.value = z; o.textContent = `${z}（${n}）`;
    ui.zoneFilter.appendChild(o);
  }
}
ui.zoneFilter.onchange = () => scheduleRender();

// ── Path 下拉 ────────────────────────────────────────────────────
function refreshPathSelects() {
  const ids = Array.from(state.cells.entries()).sort((a, b) => a[0].localeCompare(b[0]));
  for (const sel of [ui.pathFrom, ui.pathTo]) {
    if (!sel) continue;
    const prev = sel.value;
    sel.innerHTML = '<option value="">— 選擇 —</option>';
    for (const [key, c] of ids) {
      const o = document.createElement('option'); o.value = key;
      o.textContent = c.name ? `${c.name}（${key}）` : key;
      sel.appendChild(o);
    }
    if (prev) sel.value = prev;
  }
}

// ── 物件表單 ─────────────────────────────────────────────────────
function readObjectsFromJson() {
  if (!ui.fObjects.value.trim()) return [];
  const d = JSON.parse(ui.fObjects.value);
  if (!Array.isArray(d)) throw new Error('必須是陣列');
  return d;
}
function writeObjectsJson(objs) { ui.fObjects.value = JSON.stringify(objs || [], null, 2); }
function safeGetObjects() { try { return readObjectsFromJson(); } catch (_) { return []; } }

function renderObjectsForm(objs) {
  ui.objectsForm.innerHTML = '';
  (objs || []).forEach((obj, idx) => {
    const row = document.createElement('div'); row.className = 'obj-row';
    const head = document.createElement('div'); head.className = 'obj-head';
    const iId = document.createElement('input'); iId.placeholder = 'id'; iId.value = obj.id || '';
    const iName = document.createElement('input'); iName.placeholder = '名稱'; iName.value = obj.name || '';
    const btnDel = document.createElement('button'); btnDel.textContent = '刪除';
    head.append(iId, iName, btnDel); row.appendChild(head);

    const behState = { sockets: [...(obj.sockets || [])], responses: { ...(obj.responses || {}) } };
    const sync = () => {
      const list = safeGetObjects(); const cur = list[idx] || {};
      cur.id = iId.value.trim(); cur.name = iName.value.trim();
      cur.sockets = behState.sockets; cur.responses = { ...behState.responses };
      list[idx] = cur; writeObjectsJson(list);
    };
    [iId, iName].forEach(el => el.addEventListener('input', sync));
    btnDel.onclick = () => { const list = safeGetObjects(); list.splice(idx, 1); writeObjectsJson(list); renderObjectsForm(list); };

    const behRows = document.createElement('div'); behRows.className = 'beh-rows';
    function drawBeh() {
      behRows.innerHTML = '';
      behState.sockets.forEach(sock => {
        const br = document.createElement('div'); br.className = 'beh-row';
        const lbl = document.createElement('span'); lbl.className = 'beh-lbl'; lbl.textContent = sock;
        const ta = document.createElement('textarea'); ta.value = behState.responses[sock] || ''; ta.rows = 2;
        ta.oninput = () => { behState.responses[sock] = ta.value; sync(); };
        const btnR = document.createElement('button'); btnR.textContent = '✕';
        btnR.onclick = () => { behState.sockets = behState.sockets.filter(s => s !== sock); delete behState.responses[sock]; drawBeh(); sync(); };
        br.append(lbl, ta, btnR); behRows.appendChild(br);
      });
    }
    drawBeh();

    const addBar = document.createElement('div'); addBar.className = 'beh-add-bar';
    ['Look', 'Read', 'Use', 'Talk', 'Move'].forEach(b => {
      const btn = document.createElement('button'); btn.textContent = `＋${b}`;
      btn.onclick = () => { if (!behState.sockets.includes(b)) { behState.sockets.push(b); behState.responses[b] = ''; drawBeh(); sync(); } };
      addBar.appendChild(btn);
    });

    row.append(behRows, addBar);
    ui.objectsForm.appendChild(row);
  });
}

ui.fObjects.addEventListener('input', () => { try { renderObjectsForm(readObjectsFromJson()); } catch (_) {} });

// ── 按鈕事件 ─────────────────────────────────────────────────────
function setMode(mode) {
  state.mode = mode;
  state.linkFrom = null;
  $('mode-off').classList.toggle('active', mode === 'move');
  $('mode-one').classList.toggle('active', mode === 'link-one');
  $('mode-two').classList.toggle('active', mode === 'link-two');
  const mm = $('m-mode');
  if (mm) mm.textContent = mode === 'move' ? '選取' : mode === 'link-one' ? '單向' : '雙向';
  scheduleRender();
}

$('mode-off').onclick = () => setMode('move');
$('mode-one').onclick = () => setMode('link-one');
$('mode-two').onclick = () => setMode('link-two');

$('btn-refresh').onclick = async () => {
  try { await api('/api/hex/reload', { method: 'POST' }); await loadGrid(false); setStatus('已重新載入'); } catch (e) { setStatus(e.message, true); }
};
$('btn-delete').onclick = deleteSelected;
$('btn-save').onclick = saveCurrentCell;

$('btn-toggle-edges').onclick = () => {
  state.showEdges = !state.showEdges;
  $('btn-toggle-edges').textContent = state.showEdges ? '隱藏連線' : '顯示連線';
  scheduleRender();
};

$('btn-add-object').onclick = () => {
  const list = safeGetObjects();
  list.push({ id: '', name: '', sockets: ['Look'], responses: { Look: '你看見它。' } });
  writeObjectsJson(list); renderObjectsForm(list);
};

$('btn-template').onclick = () => {
  const tpl = [{ id: 'obj_example', name: '可互動物件', sockets: ['Look', 'Use', 'Talk'], responses: { Look: '你看見它有些年歲。', Use: '條件不足。' } }];
  writeObjectsJson(tpl); renderObjectsForm(tpl);
};

if ($('path-use-selected')) {
  $('path-use-selected').onclick = () => {
    if (!state.selectedCoord) { setStatus('請先選取', true); return; }
    const k = hk(state.selectedCoord.q, state.selectedCoord.r);
    if (ui.pathFrom) ui.pathFrom.value = k;
  };
}

$('btn-zoom-in').onclick = () => { cam.zoom = Math.min(ZOOM_MAX, cam.zoom * 1.2); scheduleRender(); };
$('btn-zoom-out').onclick = () => { cam.zoom = Math.max(ZOOM_MIN, cam.zoom * 0.83); scheduleRender(); };
$('btn-zoom-reset').onclick = () => { cam.zoom = 1.0; scheduleRender(); };

if ($('btn-panel-toggle')) $('btn-panel-toggle').onclick = () => ui.panel.classList.toggle('open');
if ($('m-panel')) $('m-panel').onclick = () => ui.panel.classList.toggle('open');
if ($('m-del')) $('m-del').onclick = deleteSelected;
if ($('m-mode')) {
  $('m-mode').onclick = () => setMode(state.mode === 'move' ? 'link-one' : state.mode === 'link-one' ? 'link-two' : 'move');
}

document.addEventListener('keydown', e => {
  if (!(e.ctrlKey || e.metaKey) || (e.key !== 's' && e.key !== 'S')) return;
  e.preventDefault();
  saveCurrentCell();
}, true);

// ── 初始化 ───────────────────────────────────────────────────────
function init() {
  resizeCanvas();
  window.addEventListener('resize', () => { resizeCanvas(); scheduleRender(); });
  loadGrid(true).then(() => renderObjectsForm([])).catch(e => setStatus(`初始化失敗：${e.message}`, true));
}
init();

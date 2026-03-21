const state = {
  nodes: new Map(),
  edges: [],
  layout: {},
  zoom: 1,
  selectedId: '',
  selectedIds: new Set(),
  selectedEdge: null,
  mode: 'move', // move | link-one | link-two
  drag: null,
  linkDrag: null,
  marquee: null,
  panning: null,
  pinch: null,
  touchLinkFromId: '',
  suppressMapClickClear: false,
  suppressNodeClickOnce: false,
  hasAutoFocused: false,
  renderRaf: 0,
};
const MAP_BASE_WIDTH = 2400;
const MAP_BASE_HEIGHT = 1600;
const ZOOM_MIN = 0.4;
const ZOOM_MAX = 2.5;
const ZOOM_STEP = 0.1;
const NODE_SIZE = 50;
const NODE_HALF = NODE_SIZE / 2;
const MAP_MARGIN = 240;

const ui = {
  map: document.getElementById('map'),
  svg: document.getElementById('edges'),
  wrap: document.getElementById('wrap'),
  status: document.getElementById('status'),
  fId: document.getElementById('f-id'),
  fName: document.getElementById('f-name'),
  fZone: document.getElementById('f-zone'),
  fTags: document.getElementById('f-tags'),
  fDesc: document.getElementById('f-desc'),
  fObjects: document.getElementById('f-objects'),
  objectsForm: document.getElementById('objects-form'),
  panel: document.getElementById('editor-panel'),
  btnPanelToggle: document.getElementById('btn-panel-toggle'),
  mAdd: document.getElementById('m-add'),
  mDel: document.getElementById('m-del'),
  mMode: document.getElementById('m-mode'),
  mPanel: document.getElementById('m-panel'),
  pathFrom: document.getElementById('path-from'),
  pathTo: document.getElementById('path-to'),
  pathDir: document.getElementById('path-dir'),
  pathReverse: document.getElementById('path-reverse'),
  pathReverseDir: document.getElementById('path-reverse-dir'),
  btnAddPath: document.getElementById('btn-add-path'),
  pathUseSelected: document.getElementById('path-use-selected'),
};

function setStatus(msg, isError = false) {
  ui.status.style.color = isError ? '#fda4af' : '#8f9bb3';
  const zoomText = `縮放 ${Math.round(state.zoom * 100)}%`;
  ui.status.textContent = msg ? `${msg} ｜ ${zoomText}` : zoomText;
}

async function persistLayout() {
  try {
    await api('/api/room-editor/layout', {
      method: 'PUT',
      body: JSON.stringify({ positions: state.layout }),
    });
    setStatus('座標已儲存');
  } catch (e) {
    setStatus(`儲存座標失敗：${e.message}`, true);
  }
}

async function api(path, opt = {}) {
  const res = await fetch(path, {
    headers: { 'Content-Type': 'application/json' },
    ...opt,
  });
  const txt = await res.text();
  let body = {};
  try {
    body = txt ? JSON.parse(txt) : {};
  } catch (_) {}
  if (!res.ok) throw new Error(body.error || `HTTP ${res.status}`);
  return body;
}

function edgeKey(e) {
  return `${e.from}::${e.to}`;
}

function parseTags(s) {
  return (s || '').split(',').map((t) => t.trim()).filter(Boolean);
}

function ensurePos(id, idx) {
  if (state.layout[id]) return state.layout[id];
  const col = idx % 10;
  const row = Math.floor(idx / 10);
  const p = { x: 80 + col * 90, y: 80 + row * 90 };
  state.layout[id] = p;
  return p;
}

function inferDirection(fromId, toId) {
  const a = state.layout[fromId];
  const b = state.layout[toId];
  if (!a || !b) return '';
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  if (Math.abs(dx) >= Math.abs(dy)) return dx >= 0 ? '東' : '西';
  return dy >= 0 ? '南' : '北';
}

function oppositeDirection(dir) {
  const map = { 東: '西', 西: '東', 南: '北', 北: '南', east: 'west', west: 'east', north: 'south', south: 'north' };
  return map[dir] || '';
}

function toMapPoint(clientX, clientY) {
  // 重要：以可滾動容器 wrap 為基準，避免 map rect 與 scrollLeft 疊加造成偏移。
  const rect = ui.wrap.getBoundingClientRect();
  return {
    x: (clientX - rect.left + ui.wrap.scrollLeft) / state.zoom,
    y: (clientY - rect.top + ui.wrap.scrollTop) / state.zoom,
  };
}

function scheduleRender() {
  if (state.renderRaf) return;
  state.renderRaf = requestAnimationFrame(() => {
    state.renderRaf = 0;
    render();
  });
}

/** 追蹤所有作用中 pointer，供雙指縮放與手勢判斷（Pointer Events 單一路徑） */
const activePointers = new Map();

function pinchFromActivePointers() {
  if (activePointers.size < 2) return null;
  const pts = Array.from(activePointers.values());
  const a = pts[0];
  const b = pts[1];
  const dist = Math.hypot(b.clientX - a.clientX, b.clientY - a.clientY);
  const mid = { x: (a.clientX + b.clientX) / 2, y: (a.clientY + b.clientY) / 2 };
  return { dist, mid };
}

function readObjectsFromJson() {
  if (!ui.fObjects.value.trim()) return [];
  const data = JSON.parse(ui.fObjects.value);
  if (!Array.isArray(data)) throw new Error('物件 JSON 必須是陣列');
  return data;
}

function writeObjectsJson(objs) {
  ui.fObjects.value = JSON.stringify(objs || [], null, 2);
}

function normalizeObject(obj) {
  return {
    id: String(obj?.id || '').trim(),
    name: String(obj?.name || '').trim(),
    owner: String(obj?.owner != null ? obj.owner : '').trim(),
    sockets: Array.isArray(obj?.sockets) ? obj.sockets.map((s) => String(s).trim()).filter(Boolean) : [],
    responses: obj?.responses && typeof obj.responses === 'object' ? obj.responses : {},
    move_to_room_id: String(obj?.move_to_room_id != null ? obj.move_to_room_id : '').trim(),
  };
}

function sortedRoomIds() {
  return Array.from(state.nodes.keys()).sort();
}

/** 填入房間下拉；valueToSelect 若仍存在會保留選取 */
function fillRoomSelect(selectEl, valueToSelect) {
  if (!selectEl) return;
  const prev = valueToSelect || selectEl.value || '';
  selectEl.innerHTML = '';
  const empty = document.createElement('option');
  empty.value = '';
  empty.textContent = '— 選擇房間 —';
  selectEl.appendChild(empty);
  for (const rid of sortedRoomIds()) {
    const n = state.nodes.get(rid);
    const opt = document.createElement('option');
    opt.value = rid;
    opt.textContent = n && n.name ? `${n.name}（${rid}）` : rid;
    selectEl.appendChild(opt);
  }
  if (prev && state.nodes.has(prev)) selectEl.value = prev;
}

function refreshPathSelects() {
  const pf = ui.pathFrom && ui.pathFrom.value;
  const pt = ui.pathTo && ui.pathTo.value;
  fillRoomSelect(ui.pathFrom, pf);
  fillRoomSelect(ui.pathTo, pt);
  if (ui.pathFrom && !ui.pathFrom.value && state.selectedId && state.nodes.has(state.selectedId)) {
    ui.pathFrom.value = state.selectedId;
  }
}

function renderObjectsForm(objs) {
  ui.objectsForm.innerHTML = '';
  const roomIds = sortedRoomIds();
  (objs || []).forEach((raw, idx) => {
    const obj = normalizeObject(raw);
    const row = document.createElement('div');
    row.className = 'obj-row';

    const head = document.createElement('div');
    head.className = 'obj-head';
    const inpId = document.createElement('input');
    inpId.dataset.k = 'id';
    inpId.placeholder = 'object id';
    inpId.value = obj.id;
    const inpName = document.createElement('input');
    inpName.dataset.k = 'name';
    inpName.placeholder = '物件名稱';
    inpName.value = obj.name;
    const btnDel = document.createElement('button');
    btnDel.type = 'button';
    btnDel.dataset.act = 'del';
    btnDel.textContent = '刪除';
    head.appendChild(inpId);
    head.appendChild(inpName);
    head.appendChild(btnDel);
    row.appendChild(head);

    const ownLab = document.createElement('div');
    ownLab.className = 'field';
    ownLab.innerHTML = '<label>owner（可空）</label>';
    const inpOwner = document.createElement('input');
    inpOwner.dataset.k = 'owner';
    inpOwner.type = 'text';
    inpOwner.placeholder = '';
    inpOwner.value = obj.owner;
    ownLab.appendChild(inpOwner);
    row.appendChild(ownLab);

    const moveLab = document.createElement('div');
    moveLab.className = 'field';
    const moveLbl = document.createElement('label');
    moveLbl.textContent = '移動目標房 move_to_room_id（玩家執行 Move 時切房）';
    const selMove = document.createElement('select');
    selMove.dataset.k = 'move_to_room_id';
    const optNone = document.createElement('option');
    optNone.value = '';
    optNone.textContent = '— 不切換房間 —';
    selMove.appendChild(optNone);
    for (const rid of roomIds) {
      const n = state.nodes.get(rid);
      const o = document.createElement('option');
      o.value = rid;
      o.textContent = n && n.name ? `${n.name}（${rid}）` : rid;
      if (rid === obj.move_to_room_id) o.selected = true;
      selMove.appendChild(o);
    }
    moveLab.appendChild(moveLbl);
    moveLab.appendChild(selMove);
    row.appendChild(moveLab);

    const sockLab = document.createElement('div');
    sockLab.className = 'field';
    sockLab.innerHTML = '<label>sockets（逗號分隔）</label>';
    const inpSock = document.createElement('input');
    inpSock.dataset.k = 'sockets';
    inpSock.value = obj.sockets.join(', ');
    sockLab.appendChild(inpSock);
    row.appendChild(sockLab);

    const respLab = document.createElement('div');
    respLab.className = 'field';
    respLab.innerHTML = '<label>responses（JSON 物件）</label>';
    const taResp = document.createElement('textarea');
    taResp.dataset.k = 'responses';
    taResp.value = JSON.stringify(obj.responses || {}, null, 2);
    respLab.appendChild(taResp);
    row.appendChild(respLab);

    btnDel.addEventListener('click', (ev) => {
      ev.preventDefault();
      const list = safeGetObjects();
      list.splice(idx, 1);
      writeObjectsJson(list);
      renderObjectsForm(list);
    });

    const sync = () => {
      const list = safeGetObjects();
      const cur = normalizeObject(list[idx]);
      cur.id = inpId.value.trim();
      cur.name = inpName.value.trim();
      cur.owner = inpOwner.value.trim();
      cur.move_to_room_id = selMove.value.trim();
      cur.sockets = inpSock.value.split(',').map((s) => s.trim()).filter(Boolean);
      try {
        const parsed = JSON.parse(taResp.value || '{}');
        cur.responses = parsed && typeof parsed === 'object' ? parsed : {};
      } catch (_) {}
      list[idx] = cur;
      writeObjectsJson(list);
    };
    row.querySelectorAll('input,textarea,select').forEach((el) => {
      el.addEventListener('input', sync);
      el.addEventListener('change', sync);
    });
    ui.objectsForm.appendChild(row);
  });
}

function safeGetObjects() {
  try {
    return readObjectsFromJson().map(normalizeObject);
  } catch (_) {
    return [];
  }
}

function getMarqueeRect() {
  if (!state.marquee) return null;
  const x = Math.min(state.marquee.x0, state.marquee.x1);
  const y = Math.min(state.marquee.y0, state.marquee.y1);
  const w = Math.abs(state.marquee.x1 - state.marquee.x0);
  const h = Math.abs(state.marquee.y1 - state.marquee.y0);
  return { x, y, w, h };
}

function getNodeBounds() {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let count = 0;
  for (const [id] of state.nodes.entries()) {
    const p = state.layout[id];
    if (!p) continue;
    minX = Math.min(minX, p.x);
    minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x + NODE_SIZE);
    maxY = Math.max(maxY, p.y + NODE_SIZE);
    count++;
  }
  if (count === 0) return null;
  return { minX, minY, maxX, maxY, count };
}

function focusViewportToNodes() {
  const b = getNodeBounds();
  if (!b) return;
  const pad = 40;
  ui.wrap.scrollLeft = Math.max(0, b.minX * state.zoom - pad);
  ui.wrap.scrollTop = Math.max(0, b.minY * state.zoom - pad);
}

/**
 * 連線箭頭（對齊 map_viewer vis-network：arrows.type: 'arrow'、arrowStrikethrough: false）
 * 單向：僅 marker-end；雙向：marker-start + marker-end（等同 to+from enabled）
 */
function appendSvgEdgeArrowDefs(svg) {
  const NS = 'http://www.w3.org/2000/svg';
  const defs = document.createElementNS(NS, 'defs');
  const mk = (id, orient) => {
    const marker = document.createElementNS(NS, 'marker');
    marker.setAttribute('id', id);
    // 與 vis 細線 (width≈1.5) 成比例的箭頭
    marker.setAttribute('markerWidth', '3');
    marker.setAttribute('markerHeight', '3');
    marker.setAttribute('refX', '3');
    marker.setAttribute('refY', '1.5');
    marker.setAttribute('orient', orient);
    marker.setAttribute('markerUnits', 'strokeWidth');
    const path = document.createElementNS(NS, 'path');
    path.setAttribute('d', 'M0,0 L0,3 L3,1.5 z');
    path.setAttribute('fill', 'currentColor');
    marker.appendChild(path);
    defs.appendChild(marker);
  };
  mk('room-ed-arrow-fwd', 'auto');
  mk('room-ed-arrow-rev', 'auto-start-reverse');
  svg.appendChild(defs);
}

/** 從方形中心沿單位向量射線到邊界的距離（與 halfPx 同座標系） */
function exitDistCenterToSquareEdge(ux, uy, halfPx) {
  let t = Infinity;
  if (Math.abs(ux) > 1e-12) t = Math.min(t, halfPx / Math.abs(ux));
  if (Math.abs(uy) > 1e-12) t = Math.min(t, halfPx / Math.abs(uy));
  return t === Infinity ? halfPx : t;
}

/** 連線端點避開房格內部，箭頭畫在方塊外（座標為地圖像素，已含 zoom） */
function shortenEdgeLinePx(ax, ay, bx, by, halfPx, outPx) {
  const dx = bx - ax;
  const dy = by - ay;
  const L = Math.hypot(dx, dy);
  if (L < 1e-6) return { x1: ax, y1: ay, x2: bx, y2: by };
  const ux = dx / L;
  const uy = dy / L;
  const tEdge = exitDistCenterToSquareEdge(ux, uy, halfPx);
  const pad = outPx;
  const x1 = ax + ux * (tEdge + pad);
  const y1 = ay + uy * (tEdge + pad);
  const x2 = bx - ux * (tEdge + pad);
  const y2 = by - uy * (tEdge + pad);
  const L2 = Math.hypot(x2 - x1, y2 - y1);
  if (L2 < 6) return { x1: ax, y1: ay, x2: bx, y2: by };
  return { x1, y1, x2, y2 };
}

async function normalizeLayoutToTopLeft() {
  const b = getNodeBounds();
  if (!b) return;
  const targetX = 40;
  const targetY = 40;
  const dx = targetX - b.minX;
  const dy = targetY - b.minY;
  if (dx === 0 && dy === 0) {
    setStatus('座標已在左上，無需重整');
    return;
  }
  for (const [id] of state.nodes.entries()) {
    const p = state.layout[id];
    if (!p) continue;
    p.x = Math.max(0, p.x + dx);
    p.y = Math.max(0, p.y + dy);
  }
  render();
  await persistLayout();
  focusViewportToNodes();
  setStatus('已重整座標到左上');
}

function render() {
  const b = getNodeBounds();
  let worldW = MAP_BASE_WIDTH;
  let worldH = MAP_BASE_HEIGHT;
  if (b) {
    worldW = Math.max(worldW, b.maxX + MAP_MARGIN);
    worldH = Math.max(worldH, b.maxY + MAP_MARGIN);
  }
  if (state.linkDrag) {
    worldW = Math.max(worldW, state.linkDrag.x + MAP_MARGIN);
    worldH = Math.max(worldH, state.linkDrag.y + MAP_MARGIN);
  }
  if (state.marquee) {
    const mr = getMarqueeRect();
    if (mr) {
      worldW = Math.max(worldW, mr.x + mr.w + MAP_MARGIN);
      worldH = Math.max(worldH, mr.y + mr.h + MAP_MARGIN);
    }
  }

  ui.map.style.width = `${worldW * state.zoom}px`;
  ui.map.style.height = `${worldH * state.zoom}px`;
  const zoomBtn = document.getElementById('btn-zoom-reset');
  if (zoomBtn) zoomBtn.textContent = `${Math.round(state.zoom * 100)}%`;

  ui.map.querySelectorAll('.node').forEach((el) => el.remove());
  ui.map.querySelectorAll('.marquee').forEach((el) => el.remove());
  ui.svg.innerHTML = '';
  appendSvgEdgeArrowDefs(ui.svg);

  const edgeSet = new Set(state.edges.map(edgeKey));
  const halfPx = NODE_HALF * state.zoom;
  const arrowOutPx = 2;
  for (const e of state.edges) {
    const from = state.layout[e.from];
    const to = state.layout[e.to];
    if (!from || !to) continue;
    const ax = (from.x + NODE_HALF) * state.zoom;
    const ay = (from.y + NODE_HALF) * state.zoom;
    const bx = (to.x + NODE_HALF) * state.zoom;
    const by = (to.y + NODE_HALF) * state.zoom;
    const { x1, y1, x2, y2 } = shortenEdgeLinePx(ax, ay, bx, by, halfPx, arrowOutPx);
    const both = edgeSet.has(`${e.to}::${e.from}`);
    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('x1', String(x1));
    line.setAttribute('y1', String(y1));
    line.setAttribute('x2', String(x2));
    line.setAttribute('y2', String(y2));
    const selected = state.selectedEdge && state.selectedEdge.from === e.from && state.selectedEdge.to === e.to;
    line.setAttribute('class', `${both ? 'line line-both' : 'line'}${selected ? ' line-selected' : ''}`);
    if (both) {
      line.setAttribute('marker-start', 'url(#room-ed-arrow-fwd)');
      line.setAttribute('marker-end', 'url(#room-ed-arrow-rev)');
      line.removeAttribute('stroke-dasharray');
    } else {
      line.setAttribute('marker-end', 'url(#room-ed-arrow-fwd)');
      line.setAttribute('stroke-dasharray', `${6 * state.zoom} ${4 * state.zoom}`);
    }
    line.style.pointerEvents = 'stroke';
    line.style.cursor = 'pointer';
    line.addEventListener('click', (ev) => {
      ev.stopPropagation();
      state.selectedEdge = { from: e.from, to: e.to };
      setStatus(`已選取連線：${e.from} -> ${e.to}（右鍵可刪除）`);
      render();
    });
    line.addEventListener('contextmenu', async (ev) => {
      ev.preventDefault();
      ev.stopPropagation();
      const reverse = edgeSet.has(`${e.to}::${e.from}`);
      const text = reverse ? `刪除連線 ${e.from} -> ${e.to}？\n按「確定」會連反向也一起刪除。` : `刪除連線 ${e.from} -> ${e.to}？`;
      if (!confirm(text)) return;
      try {
        await api('/api/room-editor/link', {
          method: 'DELETE',
          body: JSON.stringify({ from: e.from, to: e.to, reverse }),
        });
        state.selectedEdge = null;
        setStatus('連線已刪除');
        await loadGraph(false);
      } catch (err) {
        setStatus(`刪線失敗：${err.message}`, true);
      }
    });
    ui.svg.appendChild(line);
  }

  if (state.linkDrag) {
    const p = state.layout[state.linkDrag.fromId];
    if (p) {
      const ax = (p.x + NODE_HALF) * state.zoom;
      const ay = (p.y + NODE_HALF) * state.zoom;
      const bx = state.linkDrag.x * state.zoom;
      const by = state.linkDrag.y * state.zoom;
      const dx = bx - ax;
      const dy = by - ay;
      const L = Math.hypot(dx, dy);
      let x1 = ax;
      let y1 = ay;
      if (L > 1e-6) {
        const ux = dx / L;
        const uy = dy / L;
        const tEdge = exitDistCenterToSquareEdge(ux, uy, halfPx);
        const tUse = Math.min(tEdge + arrowOutPx, L - 4);
        if (tUse > 0) {
          x1 = ax + ux * tUse;
          y1 = ay + uy * tUse;
        }
      }
      const g = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      g.setAttribute('x1', String(x1));
      g.setAttribute('y1', String(y1));
      g.setAttribute('x2', String(bx));
      g.setAttribute('y2', String(by));
      g.setAttribute('class', 'line ghost');
      g.setAttribute('marker-end', 'url(#room-ed-arrow-fwd)');
      g.setAttribute('stroke-dasharray', `${6 * state.zoom} ${4 * state.zoom}`);
      g.style.pointerEvents = 'none';
      ui.svg.appendChild(g);
    }
  }

  let i = 0;
  for (const [id, n] of state.nodes.entries()) {
    const p = ensurePos(id, i++);
    const el = document.createElement('div');
    const selected = state.selectedIds.has(id) || state.selectedId === id;
    el.className = `node ${selected ? 'selected' : ''}`;
    el.dataset.id = id;
    const nodePx = NODE_SIZE * state.zoom;
    el.style.left = `${p.x * state.zoom}px`;
    el.style.top = `${p.y * state.zoom}px`;
    el.style.width = `${nodePx}px`;
    el.style.height = `${nodePx}px`;
    el.style.borderRadius = `${Math.max(4, 8 * state.zoom)}px`;
    const shortLabel = (n.name || id || '?').trim().slice(0, 1) || '?';
    el.title = `${n.name || id} (${id})`;
    el.innerHTML = `<div class="name">${shortLabel}</div>`;
    const nameEl = el.querySelector('.name');
    if (nameEl) {
      nameEl.style.fontSize = `${Math.max(12, 20 * state.zoom)}px`;
    }
    bindNode(el, id);
    ui.map.appendChild(el);
  }

  ui.map.appendChild(ui.svg);

  if (state.marquee) {
    const mr = getMarqueeRect();
    if (mr) {
      const box = document.createElement('div');
      box.className = 'marquee';
      box.style.left = `${mr.x * state.zoom}px`;
      box.style.top = `${mr.y * state.zoom}px`;
      box.style.width = `${mr.w * state.zoom}px`;
      box.style.height = `${mr.h * state.zoom}px`;
      ui.map.appendChild(box);
    }
  }
}

function fillEditor(id) {
  const n = state.nodes.get(id);
  if (!n) return;
  ui.fId.value = n.id;
  ui.fName.value = n.name || '';
  ui.fZone.value = n.zone || '';
  ui.fTags.value = (n.tags || []).join(', ');
  ui.fDesc.value = n.description || '';
  writeObjectsJson(n.objects || []);
  renderObjectsForm(n.objects || []);
}

function selectNode(id, additive = false) {
  state.selectedEdge = null;
  if (additive) {
    if (state.selectedIds.has(id)) state.selectedIds.delete(id);
    else state.selectedIds.add(id);
  } else {
    state.selectedIds = new Set([id]);
  }
  state.selectedId = id;
  fillEditor(id);
  refreshPathSelects();
  render();
}

function setSelectedForTouchStart(id) {
  state.selectedEdge = null;
  if (!state.selectedIds.has(id)) {
    state.selectedIds = new Set([id]);
  }
  state.selectedId = id;
  fillEditor(id);
  refreshPathSelects();
}

async function completeLink(fromId, toId) {
  if (!fromId || !toId || fromId === toId) return;
  const fromName = state.nodes.get(fromId)?.name || fromId;
  const toName = state.nodes.get(toId)?.name || toId;
  const guess = inferDirection(fromId, toId) || toName;
  const dir = prompt(`設定 ${fromName} -> ${toName} 的方向名稱`, guess) || '';
  if (!dir.trim()) return;

  const reverse = state.mode === 'link-two';
  let reverseDir = '';
  if (reverse) reverseDir = prompt(`設定 ${toName} -> ${fromName} 的方向名稱`, oppositeDirection(dir.trim()) || fromName) || fromName;

  try {
    await api('/api/room-editor/link', {
      method: 'POST',
      body: JSON.stringify({ from: fromId, to: toId, direction: dir.trim(), reverse, reverse_direction: reverseDir.trim() }),
    });
    setStatus('連線完成');
    await loadGraph(false);
  } catch (e) {
    setStatus(`連線失敗：${e.message}`, true);
  }
}

function bindNode(el, id) {
  el.addEventListener('click', (ev) => {
    if (state.suppressNodeClickOnce) {
      state.suppressNodeClickOnce = false;
      return;
    }
    ev.stopPropagation();
    selectNode(id, ev.ctrlKey || ev.metaKey);
  });

  el.addEventListener('pointerdown', (ev) => {
    if (ev.pointerType === 'mouse' && ev.button !== 0) return;
    // 雙指（或第二指）時由 capture 層處理縮放，節點不啟動拖曳
    if (activePointers.size > 1) return;

    ev.stopPropagation();
    const additive = ev.ctrlKey || ev.metaKey;

    if (ev.pointerType === 'touch') {
      setSelectedForTouchStart(id);
    } else if (additive) {
      selectNode(id, true);
    } else if (!state.selectedIds.has(id)) {
      selectNode(id, false);
    } else {
      state.selectedId = id;
      fillEditor(id);
      render();
    }

    if (state.mode === 'move') {
      // 必須在穩定元素上 capture：render() 會移除並重建 .node，否則捕獲失效、pointer 狀態錯亂導致縮放異常
      try {
        ui.wrap.setPointerCapture(ev.pointerId);
      } catch (_) {}
      const ids = state.selectedIds.size > 0 ? Array.from(state.selectedIds) : [id];
      const anchor = state.layout[id];
      const start = toMapPoint(ev.clientX, ev.clientY);
      state.drag = {
        captureEl: ui.wrap,
        pointerId: ev.pointerId,
        id,
        ids,
        dx: start.x - anchor.x,
        dy: start.y - anchor.y,
        moved: false,
        base: ids.reduce((acc, rid) => {
          const p = state.layout[rid];
          acc[rid] = { x: p.x, y: p.y };
          return acc;
        }, {}),
      };
      return;
    }

    if (state.mode === 'link-one' || state.mode === 'link-two') {
      if (ev.pointerType === 'touch') {
        if (!state.touchLinkFromId) {
          state.touchLinkFromId = id;
          setStatus(`連線起點：${id}，請再點一個房格作為終點`);
        } else if (state.touchLinkFromId === id) {
          state.touchLinkFromId = '';
          setStatus('已取消連線選取');
        } else {
          const fromId = state.touchLinkFromId;
          state.touchLinkFromId = '';
          void completeLink(fromId, id);
        }
        return;
      }
      try {
        ui.map.setPointerCapture(ev.pointerId);
      } catch (_) {}
      const p = toMapPoint(ev.clientX, ev.clientY);
      state.linkDrag = { fromId: id, x: p.x, y: p.y, pointerId: ev.pointerId };
      render();
    }
  });

  el.addEventListener('pointerup', async (ev) => {
    // 觸控在節點上放開時，不應攔截到讓 document:pointerup 收不到，
    // 否則 activePointers 會殘留，後續誤判成雙指，導致手機連線失效。
    if (ev.pointerType === 'touch') {
      activePointers.delete(ev.pointerId);
      if (activePointers.size < 2) state.pinch = null;
      return;
    }
    ev.stopPropagation();
    if (!state.linkDrag) return;
    if (state.linkDrag.pointerId !== ev.pointerId) return;
    try {
      ui.map.releasePointerCapture(ev.pointerId);
    } catch (_) {}
    const fromId = state.linkDrag.fromId;
    const toId = id;
    state.linkDrag = null;
    activePointers.delete(ev.pointerId);
    if (activePointers.size < 2) state.pinch = null;
    render();
    await completeLink(fromId, toId);
  });
}

/** 雙指落下時先於節點邏輯：取消單指手勢並進入縮放 */
document.addEventListener(
  'pointerdown',
  (ev) => {
    activePointers.set(ev.pointerId, { clientX: ev.clientX, clientY: ev.clientY });
    if (activePointers.size === 2) {
      const t = pinchFromActivePointers();
      if (t) {
        if (state.drag) {
          try {
            if (state.drag.captureEl) state.drag.captureEl.releasePointerCapture(state.drag.pointerId);
          } catch (_) {}
          const moved = !!state.drag.moved;
          state.drag = null;
          if (moved) {
            state.suppressNodeClickOnce = true;
            void persistLayout();
          }
        }
        if (state.linkDrag) {
          try {
            ui.map.releasePointerCapture(state.linkDrag.pointerId);
          } catch (_) {}
          state.linkDrag = null;
        }
        if (state.marquee) state.marquee = null;
        if (state.panning) {
          try {
            ui.wrap.releasePointerCapture(state.panning.pointerId);
          } catch (_) {}
          state.panning = null;
        }
        state.pinch = {
          startDist: Math.max(1, t.dist),
          startZoom: state.zoom,
          startMid: t.mid,
        };
      }
    }
  },
  true,
);

document.addEventListener(
  'pointermove',
  (ev) => {
    if (activePointers.has(ev.pointerId)) {
      activePointers.set(ev.pointerId, { clientX: ev.clientX, clientY: ev.clientY });
    }

    if (ev.pointerType === 'mouse' && ev.buttons === 0) {
      if (state.drag) {
        try {
          if (state.drag.captureEl) state.drag.captureEl.releasePointerCapture(state.drag.pointerId);
        } catch (_) {}
        const moved = !!state.drag.moved;
        state.drag = null;
        if (moved) {
          state.suppressNodeClickOnce = true;
          void persistLayout();
        }
      }
      if (state.linkDrag) {
        try {
          ui.map.releasePointerCapture(state.linkDrag.pointerId);
        } catch (_) {}
        state.linkDrag = null;
      }
      if (state.marquee) state.marquee = null;
      if (state.panning) {
        try {
          ui.wrap.releasePointerCapture(state.panning.pointerId);
        } catch (_) {}
        state.panning = null;
      }
    }

    if (activePointers.size === 2 && state.pinch) {
      const t = pinchFromActivePointers();
      if (t) {
        const scale = t.dist / Math.max(1, state.pinch.startDist);
        const nextZoom = clampZoom(state.pinch.startZoom * scale);
        setZoom(nextZoom, t.mid.x, t.mid.y);
      }
      return;
    }

    if (state.drag) {
      const mp = toMapPoint(ev.clientX, ev.clientY);
      const anchorX = Math.max(0, mp.x - state.drag.dx);
      const anchorY = Math.max(0, mp.y - state.drag.dy);
      const baseAnchor = state.drag.base[state.drag.id];
      const ox = anchorX - baseAnchor.x;
      const oy = anchorY - baseAnchor.y;
      if (Math.abs(ox) > 1 || Math.abs(oy) > 1) state.drag.moved = true;
      state.drag.ids.forEach((rid) => {
        const bp = state.drag.base[rid];
        state.layout[rid].x = Math.max(0, bp.x + ox);
        state.layout[rid].y = Math.max(0, bp.y + oy);
      });
      scheduleRender();
      return;
    }

    if (state.linkDrag) {
      const p = toMapPoint(ev.clientX, ev.clientY);
      state.linkDrag.x = p.x;
      state.linkDrag.y = p.y;
      scheduleRender();
      return;
    }

    if (state.marquee) {
      const p = toMapPoint(ev.clientX, ev.clientY);
      state.marquee.x1 = p.x;
      state.marquee.y1 = p.y;
      scheduleRender();
      return;
    }

    if (state.panning) {
      const dx = ev.clientX - state.panning.startX;
      const dy = ev.clientY - state.panning.startY;
      ui.wrap.scrollLeft = Math.max(0, state.panning.baseScrollLeft - dx);
      ui.wrap.scrollTop = Math.max(0, state.panning.baseScrollTop - dy);
    }
  },
  { passive: false },
);

async function handlePointerUpEnd(ev) {
  activePointers.delete(ev.pointerId);
  if (activePointers.size < 2) state.pinch = null;

  if (state.drag && state.drag.pointerId === ev.pointerId) {
    try {
      if (state.drag.captureEl) state.drag.captureEl.releasePointerCapture(ev.pointerId);
    } catch (_) {}
    if (state.drag.moved) state.suppressNodeClickOnce = true;
    state.drag = null;
    await persistLayout();
  }

  if (state.linkDrag && state.linkDrag.pointerId === ev.pointerId) {
    const fromId = state.linkDrag.fromId;
    try {
      ui.map.releasePointerCapture(ev.pointerId);
    } catch (_) {}
    state.linkDrag = null;
    render();
    // setPointerCapture(map) 時 pointerup 的 target 是 map，節點上的 listener 不會觸發，須用座標找終點房格
    let toId = '';
    try {
      const stack = document.elementsFromPoint(ev.clientX, ev.clientY);
      for (let i = 0; i < stack.length; i++) {
        const el = stack[i];
        if (el.classList && el.classList.contains('node') && el.dataset && el.dataset.id) {
          toId = el.dataset.id;
          break;
        }
      }
    } catch (_) {}
    if (toId && toId !== fromId) {
      await completeLink(fromId, toId);
    }
  }

  if (state.marquee && state.marquee.pointerId === ev.pointerId) {
    const mr = getMarqueeRect();
    const next = new Set();
    if (mr && (mr.w > 3 || mr.h > 3)) {
      for (const [id] of state.nodes.entries()) {
        const p = state.layout[id];
        if (!p) continue;
        const cx = p.x + NODE_HALF;
        const cy = p.y + NODE_HALF;
        if (cx >= mr.x && cx <= mr.x + mr.w && cy >= mr.y && cy <= mr.y + mr.h) next.add(id);
      }
    }
    state.marquee = null;
    if (next.size > 0) {
      state.selectedIds = next;
      const first = Array.from(next)[0];
      state.selectedId = first;
      fillEditor(first);
      setStatus(`已框選 ${next.size} 個房格`);
      state.suppressMapClickClear = true;
    }
    render();
  }

  if (state.panning && state.panning.pointerId === ev.pointerId) {
    try {
      ui.wrap.releasePointerCapture(ev.pointerId);
    } catch (_) {}
    state.panning = null;
  }
}

document.addEventListener('pointerup', (ev) => {
  void handlePointerUpEnd(ev);
});

document.addEventListener('pointercancel', (ev) => {
  activePointers.delete(ev.pointerId);
  if (activePointers.size < 2) state.pinch = null;
  if (state.drag && state.drag.pointerId === ev.pointerId) {
    try {
      if (state.drag.captureEl) state.drag.captureEl.releasePointerCapture(ev.pointerId);
    } catch (_) {}
  }
  if (state.linkDrag && state.linkDrag.pointerId === ev.pointerId) {
    try {
      ui.map.releasePointerCapture(ev.pointerId);
    } catch (_) {}
  }
  if (state.panning && state.panning.pointerId === ev.pointerId) {
    try {
      ui.wrap.releasePointerCapture(ev.pointerId);
    } catch (_) {}
  }
  state.drag = null;
  state.linkDrag = null;
  state.marquee = null;
  state.panning = null;
  state.touchLinkFromId = '';
  render();
});

// 視窗失焦時也清空拖曳，避免回到頁面後仍處於「抓著房格」狀態。
window.addEventListener('blur', () => {
  if (!state.drag && !state.linkDrag && !state.marquee && !state.panning && !state.pinch) return;
  if (state.drag) {
    try {
      if (state.drag.captureEl) state.drag.captureEl.releasePointerCapture(state.drag.pointerId);
    } catch (_) {}
  }
  if (state.linkDrag) {
    try {
      ui.map.releasePointerCapture(state.linkDrag.pointerId);
    } catch (_) {}
  }
  if (state.panning) {
    try {
      ui.wrap.releasePointerCapture(state.panning.pointerId);
    } catch (_) {}
  }
  state.drag = null;
  state.linkDrag = null;
  state.marquee = null;
  state.panning = null;
  state.pinch = null;
  activePointers.clear();
  render();
});

ui.map.addEventListener('pointerdown', (ev) => {
  const target = ev.target;
  const tag = (target && target.tagName || '').toLowerCase();
  if (tag === 'input' || tag === 'textarea' || tag === 'button') return;

  if (ev.pointerType === 'mouse' && ev.button === 1) {
    ev.preventDefault();
    try {
      ui.wrap.setPointerCapture(ev.pointerId);
    } catch (_) {}
    state.panning = {
      pointerId: ev.pointerId,
      startX: ev.clientX,
      startY: ev.clientY,
      baseScrollLeft: ui.wrap.scrollLeft,
      baseScrollTop: ui.wrap.scrollTop,
    };
    return;
  }
  if (ev.pointerType === 'mouse' && ev.button !== 0) return;
  if (activePointers.size > 1) return;

  if (ev.target !== ui.map && ev.target !== ui.svg) return;

  if (ev.pointerType === 'touch') {
    try {
      ui.wrap.setPointerCapture(ev.pointerId);
    } catch (_) {}
    state.panning = {
      pointerId: ev.pointerId,
      startX: ev.clientX,
      startY: ev.clientY,
      baseScrollLeft: ui.wrap.scrollLeft,
      baseScrollTop: ui.wrap.scrollTop,
    };
    return;
  }

  if (state.mode !== 'move') return;
  const p = toMapPoint(ev.clientX, ev.clientY);
  state.marquee = { x0: p.x, y0: p.y, x1: p.x, y1: p.y, pointerId: ev.pointerId };
  state.selectedEdge = null;
  render();
});

ui.map.addEventListener('click', (ev) => {
  if (state.suppressMapClickClear) {
    state.suppressMapClickClear = false;
    return;
  }
  if (ev.target !== ui.map && ev.target !== ui.svg) return;
  state.selectedId = '';
  state.selectedIds.clear();
  state.selectedEdge = null;
  state.touchLinkFromId = '';
  render();
});

ui.fObjects.addEventListener('input', () => {
  try {
    const objs = readObjectsFromJson();
    renderObjectsForm(objs);
  } catch (_) {
    // 手打 JSON 過程中可暫時無效，不中斷編輯
  }
});

async function loadGraph(scrollToSelected = true) {
  const data = await api('/api/room-editor/graph');
  state.nodes.clear();
  data.nodes.forEach((n, i) => {
    state.nodes.set(n.id, n);
    if (!data.layout[n.id]) ensurePos(n.id, i);
  });
  state.edges = data.edges || [];
  state.layout = { ...(data.layout || {}), ...(state.layout || {}) };

  if (state.selectedId && !state.nodes.has(state.selectedId)) {
    state.selectedId = '';
    state.selectedIds.clear();
  }

  state.selectedIds = new Set(Array.from(state.selectedIds).filter((id) => state.nodes.has(id)));

  if (state.selectedId) fillEditor(state.selectedId);
  refreshPathSelects();
  render();

  if (scrollToSelected && state.selectedId && state.layout[state.selectedId]) {
    const p = state.layout[state.selectedId];
    ui.wrap.scrollTo({ left: Math.max(0, p.x - 260), top: Math.max(0, p.y - 220), behavior: 'smooth' });
  } else if (!state.hasAutoFocused) {
    focusViewportToNodes();
    state.hasAutoFocused = true;
  }
}


function clampZoom(z) {
  return Math.max(ZOOM_MIN, Math.min(ZOOM_MAX, z));
}

function setZoom(nextZoom, focusClientX, focusClientY) {
  const prev = state.zoom;
  const next = clampZoom(nextZoom);
  if (Math.abs(prev - next) < 0.001) return;

  const wrapRect = ui.wrap.getBoundingClientRect();
  const fx = focusClientX ?? (wrapRect.left + wrapRect.width / 2);
  const fy = focusClientY ?? (wrapRect.top + wrapRect.height / 2);
  const world = toMapPoint(fx, fy);
  state.zoom = next;
  render();

  const viewportX = fx - wrapRect.left;
  const viewportY = fy - wrapRect.top;
  ui.wrap.scrollLeft = Math.max(0, world.x * state.zoom - viewportX);
  ui.wrap.scrollTop = Math.max(0, world.y * state.zoom - viewportY);
}

function setMode(mode) {
  state.mode = mode;
  state.linkDrag = null;
  state.marquee = null;
  document.getElementById('mode-one').classList.toggle('active', mode === 'link-one');
  document.getElementById('mode-two').classList.toggle('active', mode === 'link-two');
  document.getElementById('mode-off').classList.toggle('active', mode === 'move');
  if (ui.mMode) {
    const text = mode === 'move' ? '一般拖曳' : (mode === 'link-one' ? '單向連線' : '雙向連線');
    ui.mMode.textContent = text;
  }
  render();
}

document.getElementById('mode-one').onclick = () => setMode('link-one');
document.getElementById('mode-two').onclick = () => setMode('link-two');
document.getElementById('mode-off').onclick = () => setMode('move');
if (ui.btnPanelToggle) ui.btnPanelToggle.onclick = () => ui.panel.classList.toggle('open');
if (ui.mPanel) ui.mPanel.onclick = () => ui.panel.classList.toggle('open');
if (ui.mAdd) ui.mAdd.onclick = () => document.getElementById('btn-add').click();
if (ui.mDel) ui.mDel.onclick = () => document.getElementById('btn-delete').click();
if (ui.mMode) {
  ui.mMode.onclick = () => {
    const next = state.mode === 'move' ? 'link-one' : (state.mode === 'link-one' ? 'link-two' : 'move');
    setMode(next);
    setStatus(`模式：${ui.mMode.textContent}`);
  };
}

ui.wrap.addEventListener('wheel', (ev) => {
  ev.preventDefault();
  const delta = ev.deltaY > 0 ? -ZOOM_STEP : ZOOM_STEP;
  setZoom(state.zoom + delta, ev.clientX, ev.clientY);
}, { passive: false });

document.getElementById('btn-refresh').onclick = async () => {
  try {
    await loadGraph(false);
    setStatus('已重新載入');
  } catch (e) {
    setStatus(`載入失敗：${e.message}`, true);
  }
};
document.getElementById('btn-normalize').onclick = async () => {
  if (!confirm('將所有房格平移回左上並覆寫目前座標，是否繼續？')) return;
  await normalizeLayoutToTopLeft();
};

document.getElementById('btn-add').onclick = async () => {
  const id = prompt('新房間 ID（英文數字底線）');
  if (!id) return;
  const name = prompt('房間名稱', id) || id;
  try {
    await api('/api/room-editor/room', {
      method: 'POST',
      body: JSON.stringify({ id: id.trim(), name: name.trim(), description: '' }),
    });
    state.selectedId = id.trim();
    state.selectedIds = new Set([id.trim()]);
    setStatus('房間已新增');
    await loadGraph(true);
  } catch (e) {
    setStatus(`新增失敗：${e.message}`, true);
  }
};

document.getElementById('btn-copy').onclick = async () => {
  if (!state.selectedId) {
    setStatus('請先選一個房間', true);
    return;
  }
  const id = prompt('複製後新 ID', `${state.selectedId}_copy`);
  if (!id) return;
  const name = prompt('新房名', id) || id;
  try {
    await api('/api/room-editor/room', {
      method: 'POST',
      body: JSON.stringify({ id: id.trim(), name: name.trim(), clone_from: state.selectedId }),
    });
    const p = state.layout[state.selectedId];
    if (p) state.layout[id.trim()] = { x: p.x + 200, y: p.y + 40 };
    await api('/api/room-editor/layout', { method: 'PUT', body: JSON.stringify({ positions: state.layout }) });
    state.selectedId = id.trim();
    state.selectedIds = new Set([id.trim()]);
    setStatus('房間已複製');
    await loadGraph(true);
  } catch (e) {
    setStatus(`複製失敗：${e.message}`, true);
  }
};

document.getElementById('btn-delete').onclick = async () => {
  const selected = state.selectedIds && state.selectedIds.size > 0
    ? Array.from(state.selectedIds)
    : (state.selectedId ? [state.selectedId] : []);

  if (selected.length === 0) {
    setStatus('請先選一個房間', true);
    return;
  }

  const tip = selected.length > 1
    ? `確定刪除已選 ${selected.length} 個房間？`
    : `確定刪除房間 ${selected[0]}？`;
  if (!confirm(tip)) return;

  let okCount = 0;
  let failCount = 0;
  for (const rid of selected) {
    try {
      await api(`/api/room-editor/room/${encodeURIComponent(rid)}`, { method: 'DELETE' });
      delete state.layout[rid];
      okCount++;
    } catch (_) {
      failCount++;
    }
  }
  await api('/api/room-editor/layout', { method: 'PUT', body: JSON.stringify({ positions: state.layout }) });
  state.selectedId = '';
  state.selectedIds.clear();
  await loadGraph(false);
  if (failCount > 0) {
    setStatus(`批次刪除完成：成功 ${okCount}、失敗 ${failCount}`, true);
  } else {
    setStatus(`批次刪除完成：共刪除 ${okCount} 個房間`);
  }
};

document.getElementById('btn-add-object').onclick = () => {
  const list = safeGetObjects();
  list.push({ id: '', name: '', sockets: ['Look'], responses: { Look: '你看見它。' } });
  writeObjectsJson(list);
  renderObjectsForm(list);
};

document.getElementById('btn-template').onclick = () => {
  const tpl = [
    {
      id: 'obj_example',
      name: '可互動物件',
      sockets: ['Look', 'Use', 'Talk'],
      responses: {
        Look: '你看見它有些年歲。',
        Use: '你試著操作，但還需要更多條件。',
      },
    },
  ];
  writeObjectsJson(tpl);
  renderObjectsForm(tpl);
};

async function saveCurrentRoom() {
  if (!state.selectedId) {
    setStatus('請先選一個房間', true);
    return;
  }
  let objects = [];
  try {
    objects = readObjectsFromJson().map(normalizeObject);
  } catch (e) {
    setStatus(`物件 JSON 格式錯誤：${e.message}`, true);
    return;
  }
  try {
    await api(`/api/room-editor/room/${encodeURIComponent(state.selectedId)}`, {
      method: 'PUT',
      body: JSON.stringify({
        name: ui.fName.value,
        zone: ui.fZone.value,
        tags: parseTags(ui.fTags.value),
        description: ui.fDesc.value,
        objects,
      }),
    });
    setStatus('房間已儲存');
    await loadGraph(false);
    selectNode(state.selectedId);
  } catch (e) {
    setStatus(`儲存失敗：${e.message}`, true);
  }
}

document.getElementById('btn-save').onclick = () => {
  saveCurrentRoom();
};

if (ui.pathUseSelected) {
  ui.pathUseSelected.addEventListener('click', () => {
    if (!state.selectedId || !state.nodes.has(state.selectedId)) {
      setStatus('請先在地圖上選一個房格', true);
      return;
    }
    ui.pathFrom.value = state.selectedId;
    setStatus(`起點已設為：${state.nodes.get(state.selectedId)?.name || state.selectedId}`);
  });
}

if (ui.btnAddPath) {
  ui.btnAddPath.addEventListener('click', async () => {
    const from = ui.pathFrom && ui.pathFrom.value;
    const to = ui.pathTo && ui.pathTo.value;
    const dir = (ui.pathDir && ui.pathDir.value) || '';
    const dirTrim = dir.trim();
    const reverse = ui.pathReverse && ui.pathReverse.checked;
    let reverseDir = (ui.pathReverseDir && ui.pathReverseDir.value) || '';
    reverseDir = reverseDir.trim();
    if (!from || !to) {
      setStatus('請選擇起點與終點房間', true);
      return;
    }
    if (from === to) {
      setStatus('起點與終點不可相同', true);
      return;
    }
    if (!dirTrim) {
      setStatus('請填「此端方向名稱」', true);
      return;
    }
    if (reverse && !reverseDir) {
      const n = state.nodes.get(from);
      reverseDir = (n && n.name) || from;
    }
    try {
      await api('/api/room-editor/link', {
        method: 'POST',
        body: JSON.stringify({
          from,
          to,
          direction: dirTrim,
          reverse,
          reverse_direction: reverseDir,
        }),
      });
      setStatus('路徑已建立（exits + Move／move_to_room_id）');
      if (ui.pathDir) ui.pathDir.value = '';
      if (ui.pathReverseDir) ui.pathReverseDir.value = '';
      await loadGraph(false);
    } catch (e) {
      setStatus(`建立路徑失敗：${e.message}`, true);
    }
  });
}

/** Ctrl+S（Windows/Linux）或 Cmd+S（macOS）儲存目前選取的房間 */
document.addEventListener(
  'keydown',
  (e) => {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key !== 's' && e.key !== 'S') return;
    e.preventDefault();
    saveCurrentRoom();
  },
  true
);

(async () => {
  try {
    await loadGraph(false);
    renderObjectsForm([]);
    setStatus('已載入房間地圖');
  } catch (e) {
    setStatus(`初始化失敗：${e.message}`, true);
  }
})();

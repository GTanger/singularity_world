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
  suppressMapClickClear: false,
  suppressNodeClickOnce: false,
  hasAutoFocused: false,
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
    sockets: Array.isArray(obj?.sockets) ? obj.sockets.map((s) => String(s).trim()).filter(Boolean) : [],
    responses: obj?.responses && typeof obj.responses === 'object' ? obj.responses : {},
  };
}

function renderObjectsForm(objs) {
  ui.objectsForm.innerHTML = '';
  (objs || []).forEach((raw, idx) => {
    const obj = normalizeObject(raw);
    const row = document.createElement('div');
    row.className = 'obj-row';
    row.innerHTML = `
      <div class="obj-head">
        <input data-k="id" placeholder="object id" value="${obj.id.replace(/"/g, '&quot;')}" />
        <input data-k="name" placeholder="物件名稱" value="${obj.name.replace(/"/g, '&quot;')}" />
        <button data-act="del">刪除</button>
      </div>
      <div class="field"><label>sockets（逗號分隔）</label><input data-k="sockets" value="${obj.sockets.join(', ')}" /></div>
      <div class="field"><label>responses（JSON 物件）</label><textarea data-k="responses">${JSON.stringify(obj.responses || {}, null, 2)}</textarea></div>
    `;
    row.querySelector('[data-act="del"]').addEventListener('click', (ev) => {
      ev.preventDefault();
      const list = safeGetObjects();
      list.splice(idx, 1);
      writeObjectsJson(list);
      renderObjectsForm(list);
    });
    const sync = () => {
      const list = safeGetObjects();
      const cur = normalizeObject(list[idx]);
      cur.id = row.querySelector('[data-k="id"]').value.trim();
      cur.name = row.querySelector('[data-k="name"]').value.trim();
      cur.sockets = row.querySelector('[data-k="sockets"]').value.split(',').map((s) => s.trim()).filter(Boolean);
      try {
        const parsed = JSON.parse(row.querySelector('[data-k="responses"]').value || '{}');
        cur.responses = parsed && typeof parsed === 'object' ? parsed : {};
      } catch (_) {
        // 保留原值，等儲存時再報錯
      }
      list[idx] = cur;
      writeObjectsJson(list);
    };
    row.querySelectorAll('input,textarea').forEach((el) => el.addEventListener('input', sync));
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

  const edgeSet = new Set(state.edges.map(edgeKey));
  for (const e of state.edges) {
    const from = state.layout[e.from];
    const to = state.layout[e.to];
    if (!from || !to) continue;
    const x1 = (from.x + NODE_HALF) * state.zoom;
    const y1 = (from.y + NODE_HALF) * state.zoom;
    const x2 = (to.x + NODE_HALF) * state.zoom;
    const y2 = (to.y + NODE_HALF) * state.zoom;
    const both = edgeSet.has(`${e.to}::${e.from}`);
    const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
    line.setAttribute('x1', String(x1));
    line.setAttribute('y1', String(y1));
    line.setAttribute('x2', String(x2));
    line.setAttribute('y2', String(y2));
    const selected = state.selectedEdge && state.selectedEdge.from === e.from && state.selectedEdge.to === e.to;
    line.setAttribute('class', `${both ? 'line line-both' : 'line'}${selected ? ' line-selected' : ''}`);
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
      const g = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      g.setAttribute('x1', String((p.x + NODE_HALF) * state.zoom));
      g.setAttribute('y1', String((p.y + NODE_HALF) * state.zoom));
      g.setAttribute('x2', String(state.linkDrag.x * state.zoom));
      g.setAttribute('y2', String(state.linkDrag.y * state.zoom));
      g.setAttribute('class', 'line ghost');
      ui.svg.appendChild(g);
    }
  }

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
  render();
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

  el.addEventListener('mousedown', (ev) => {
    ev.stopPropagation();
    const additive = ev.ctrlKey || ev.metaKey;
    if (additive) {
      selectNode(id, true);
    } else if (!state.selectedIds.has(id)) {
      selectNode(id, false);
    } else {
      // 已在群組內時保留多選，僅更新主要編輯目標
      state.selectedId = id;
      fillEditor(id);
      render();
    }

    if (state.mode === 'move') {
      const ids = state.selectedIds.size > 0 ? Array.from(state.selectedIds) : [id];
      const anchor = state.layout[id];
      const start = toMapPoint(ev.clientX, ev.clientY);
      state.drag = {
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
      const p = toMapPoint(ev.clientX, ev.clientY);
      state.linkDrag = { fromId: id, x: p.x, y: p.y };
      render();
    }
  });

  el.addEventListener('mouseup', async (ev) => {
    ev.stopPropagation();
    if (!state.linkDrag) return;
    const fromId = state.linkDrag.fromId;
    const toId = id;
    state.linkDrag = null;
    render();
    if (fromId === toId) return;

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
  });
}

document.addEventListener('mousemove', (ev) => {
  // 保險機制：若放開事件漏掉（例如滑出視窗），buttons=0 時立即解除所有拖曳狀態。
  if (ev.buttons === 0) {
    if (state.drag) {
      const moved = !!state.drag.moved;
      state.drag = null;
      if (moved) {
        state.suppressNodeClickOnce = true;
        void persistLayout();
      }
    }
    if (state.linkDrag) state.linkDrag = null;
    if (state.marquee) state.marquee = null;
    if (state.panning) state.panning = null;
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
    render();
    return;
  }

  if (state.linkDrag) {
    const p = toMapPoint(ev.clientX, ev.clientY);
    state.linkDrag.x = p.x;
    state.linkDrag.y = p.y;
    render();
    return;
  }

  if (state.marquee) {
    const p = toMapPoint(ev.clientX, ev.clientY);
    state.marquee.x1 = p.x;
    state.marquee.y1 = p.y;
    render();
    return;
  }

  if (state.panning) {
    const dx = ev.clientX - state.panning.startX;
    const dy = ev.clientY - state.panning.startY;
    ui.wrap.scrollLeft = Math.max(0, state.panning.baseScrollLeft - dx);
    ui.wrap.scrollTop = Math.max(0, state.panning.baseScrollTop - dy);
  }
});

// 視窗失焦時也清空拖曳，避免回到頁面後仍處於「抓著房格」狀態。
window.addEventListener('blur', () => {
  if (!state.drag && !state.linkDrag && !state.marquee && !state.panning) return;
  state.drag = null;
  state.linkDrag = null;
  state.marquee = null;
  state.panning = null;
  render();
});

document.addEventListener('mouseup', async () => {
  if (state.drag) {
    if (state.drag.moved) state.suppressNodeClickOnce = true;
    state.drag = null;
    await persistLayout();
  }

  if (state.linkDrag) {
    state.linkDrag = null;
    render();
  }

  if (state.marquee) {
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

  if (state.panning) {
    state.panning = null;
  }
});

ui.map.addEventListener('mousedown', (ev) => {
  // 中鍵按住可平移畫布
  if (ev.button === 1) {
    ev.preventDefault();
    state.panning = {
      startX: ev.clientX,
      startY: ev.clientY,
      baseScrollLeft: ui.wrap.scrollLeft,
      baseScrollTop: ui.wrap.scrollTop,
    };
    return;
  }
  // 僅左鍵允許框選
  if (ev.button !== 0) return;
  if (ev.target !== ui.map && ev.target !== ui.svg) return;
  if (state.mode !== 'move') return;
  const p = toMapPoint(ev.clientX, ev.clientY);
  state.marquee = { x0: p.x, y0: p.y, x1: p.x, y1: p.y };
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
  render();
}

document.getElementById('mode-one').onclick = () => setMode('link-one');
document.getElementById('mode-two').onclick = () => setMode('link-two');
document.getElementById('mode-off').onclick = () => setMode('move');

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

document.getElementById('btn-save').onclick = async () => {
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
};

(async () => {
  try {
    await loadGraph(false);
    renderObjectsForm([]);
    setStatus('已載入房間地圖');
  } catch (e) {
    setStatus(`初始化失敗：${e.message}`, true);
  }
})();

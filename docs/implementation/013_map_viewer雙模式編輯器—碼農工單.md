# 013 — Map Viewer 雙模式編輯器（碼農工單）

> **你只需要照做，不要自己發明新東西。**
> 改動只涉及 `web/map_viewer.html`（單檔，HTML + JS + CSS 全寫在裡面）。
> **不要引入新依賴。不要動後端。不要動其他檔案。**
> vis-network 已透過 CDN 引入，直接用。

---

## 總覽

把現有 `map_viewer.html`（純閱讀的蒲公英地圖）改成閱讀／編輯雙模式。
閱讀模式維持現狀。編輯模式可新增房間、拖線連房、編輯房間屬性、刪除房間和連線。

**後端 API 全部現成，不需要改後端。**

---

## API 速查表

所有請求需帶 query `?mg_key=<管理金鑰>`（空字串時不需要）。

| 方法 | 路徑 | 用途 | body 格式 |
|------|------|------|-----------|
| GET | `/api/room-editor/graph` | 取得所有節點+連線+layout | — |
| POST | `/api/room-editor/room` | 新增房間 | `{id, name, description?, zone?, tags?, objects?}` |
| PUT | `/api/room-editor/room/:id` | 更新房間 | `{name, description?, zone?, tags?, objects?}` |
| DELETE | `/api/room-editor/room/:id` | 刪除房間 | — |
| POST | `/api/room-editor/link` | 建立連線 | `{from, to, direction?, reverse?, reverse_direction?}` |
| DELETE | `/api/room-editor/link` | 刪除連線 | `{from, to, reverse?}` |
| PUT | `/api/room-editor/layout` | 儲存座標 | `{positions: {room_id: {x, y}, ...}}` |
| POST | `/api/room-editor/reload` | 重載 store | — |

---

## 步驟 1：HTML 結構改造

### 1a. 替換 header

把現有 `<header>` 整個替換為：

```html
<header>
  <h1>浮生地圖</h1>
  <div class="mode-switch">
    <button id="btn-mode-read" class="mode-btn active">閱讀</button>
    <button id="btn-mode-edit" class="mode-btn">編輯</button>
  </div>
  <div class="field-inline">
    <label>Zone</label>
    <select id="zone-filter">
      <option value="">全部</option>
    </select>
  </div>
  <div class="login-form" id="loginForm">
    <input type="text" id="inputId" placeholder="角色 ID" autocomplete="username" />
    <input type="password" id="inputPw" placeholder="密碼" autocomplete="current-password" />
    <button type="button" id="btnLogin">登入</button>
  </div>
  <span id="error-msg"></span>
  <div class="status" id="status">正在載入…</div>
</header>
```

### 1b. 在 `<div id="mynetwork">` 下方加側面板

在 `<div id="mynetwork"></div>` **之後**（body 內、`</script>` 之前），插入：

```html
<aside class="side-panel" id="sidePanel">
  <h3 id="panel-title">房間屬性</h3>
  <div class="field"><label>ID</label><input id="f-id" /></div>
  <div class="field"><label>名稱</label><input id="f-name" /></div>
  <div class="field"><label>Zone</label><input id="f-zone" /></div>
  <div class="field"><label>Tags（逗號分隔）</label><input id="f-tags" /></div>
  <div class="field"><label>描述</label><textarea id="f-desc"></textarea></div>
  <div class="panel-buttons">
    <button id="btn-save">儲存</button>
    <button id="btn-delete" class="danger">刪除房間</button>
  </div>
</aside>
```

### 1c. 連線對話框

在側面板之後加：

```html
<div class="modal-overlay" id="linkModal" style="display:none;">
  <div class="modal">
    <h3>建立連線</h3>
    <p id="link-desc"></p>
    <div class="field"><label>方向名稱（從起點看）</label><input id="link-dir" placeholder="留空則用終點房名" /></div>
    <div class="field">
      <label><input type="checkbox" id="link-reverse" checked /> 同時建立反向連線</label>
    </div>
    <div class="field" id="link-rev-wrap"><label>反向方向名稱</label><input id="link-rev-dir" placeholder="留空則用起點房名" /></div>
    <div class="panel-buttons">
      <button id="link-ok">確定</button>
      <button id="link-cancel">取消</button>
    </div>
  </div>
</div>
```

### 1d. 新增房間對話框

```html
<div class="modal-overlay" id="addModal" style="display:none;">
  <div class="modal">
    <h3>新增房間</h3>
    <div class="field"><label>名稱</label><input id="add-name" placeholder="輸入房間名稱" /></div>
    <div class="panel-buttons">
      <button id="add-ok">確定</button>
      <button id="add-cancel">取消</button>
    </div>
  </div>
</div>
```

---

## 步驟 2：CSS 新增

在現有 `<style>` 區塊末尾（`</style>` 之前）加入以下樣式：

```css
/* ─── 模式切換 ─── */
.mode-switch { display: flex; gap: 4px; }
.mode-btn {
  background: #1a1a2e; color: #a0a0b0; border: 1px solid #0f3460;
  padding: 6px 14px; border-radius: 6px; cursor: pointer; font-size: 0.9rem;
}
.mode-btn.active { background: #0f3460; color: #e8e8e8; border-color: #e94560; }

/* ─── Zone 濾鏡 ─── */
.field-inline { display: flex; align-items: center; gap: 6px; }
.field-inline label { color: #a0a0b0; font-size: 0.85rem; white-space: nowrap; }
.field-inline select {
  background: #1a1a2e; color: #e8e8e8; border: 1px solid #0f3460;
  border-radius: 6px; padding: 6px 10px; font-size: 0.85rem;
}

/* ─── 側面板 ─── */
.side-panel {
  display: none; /* 預設隱藏，編輯模式選中節點才顯示 */
  position: fixed; right: 0; top: 0; bottom: 0; width: 320px;
  background: #16213e; border-left: 1px solid #0f3460;
  padding: 16px; overflow-y: auto; z-index: 10;
}
.side-panel.open { display: block; }
.side-panel h3 { margin: 0 0 12px; font-size: 1rem; color: #e8e8e8; }
.side-panel .field { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; }
.side-panel .field label { color: #a0a0b0; font-size: 0.8rem; }
.side-panel .field input,
.side-panel .field textarea {
  background: #1a1a2e; color: #e8e8e8; border: 1px solid #0f3460;
  border-radius: 6px; padding: 8px; font-size: 0.9rem;
}
.side-panel .field textarea { min-height: 80px; resize: vertical; }
.panel-buttons { display: flex; gap: 8px; margin-top: 12px; }
.panel-buttons button {
  flex: 1; padding: 8px; border-radius: 6px; cursor: pointer; font-size: 0.9rem;
  border: 1px solid #0f3460; background: #1a1a2e; color: #e8e8e8;
}
.panel-buttons button:hover { background: #0f3460; }
.panel-buttons .danger { border-color: #5c1d26; color: #fecdd3; }
.panel-buttons .danger:hover { background: #5c1d26; }
.side-panel.open ~ #mynetwork { margin-right: 320px; } /* 不要，用下面的方式 */

/* ─── 對話框 ─── */
.modal-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,.6);
  display: flex; align-items: center; justify-content: center; z-index: 50;
}
.modal {
  background: #16213e; border: 1px solid #0f3460; border-radius: 12px;
  padding: 20px; width: min(90vw, 400px);
}
.modal h3 { margin: 0 0 12px; color: #e8e8e8; }
.modal .field { display: flex; flex-direction: column; gap: 4px; margin: 8px 0; }
.modal .field label { color: #a0a0b0; font-size: 0.85rem; }
.modal .field input {
  background: #1a1a2e; color: #e8e8e8; border: 1px solid #0f3460;
  border-radius: 6px; padding: 8px;
}
.modal p { color: #a0a0b0; font-size: 0.9rem; margin: 4px 0 12px; }

/* ─── 佈局 ─── */
body.edit-mode #mynetwork { width: calc(100% - 320px); }
body.edit-mode .side-panel { display: block; }

/* ─── 手機 ─── */
@media (max-width: 768px) {
  header { padding: 8px 12px; gap: 8px; }
  .login-form { display: none; } /* 手機不需要登入 */
  body.edit-mode #mynetwork { width: 100%; }
  body.edit-mode .side-panel {
    position: fixed; left: 0; right: 0; bottom: 0; top: auto;
    width: 100%; height: 50vh; border-left: none; border-top: 1px solid #0f3460;
  }
}
```

**注意**：刪掉上面 `.side-panel.open ~ #mynetwork` 那行註解（我故意留下來提醒你不要用那種選擇器），改用 `body.edit-mode #mynetwork` 的方式。

---

## 步驟 3：JS 全面改寫

把 `<script>` 區塊整個替換。以下是完整 JS（包含現有功能 + 新功能）：

```javascript
(function () {
  'use strict';

  // ─── 常數 ───
  var API_GRAPH = '/api/room-editor/graph';
  var API_ROOM = '/api/room-editor/room';
  var API_LINK = '/api/room-editor/link';
  var API_LAYOUT = '/api/room-editor/layout';
  var API_RELOAD = '/api/room-editor/reload';
  var DATA_URL = '/data/rooms.json';
  var MG_KEY = ''; // 管理金鑰，空字串=不需要

  // 手動指定的 zone 配色（可自行增刪）
  var ZONE_COLORS = {};
  var DEFAULT_NODE_COLOR = '#9e9eb8';

  // 高對比色池（深色底 #1a1a2e 上清晰可辨的飽和色，色相均勻分散）
  var COLOR_POOL = [
    '#00e5ff', '#ff5252', '#7c4dff', '#ffd740', '#69f0ae',
    '#ff4081', '#40c4ff', '#e040fb', '#ff9100', '#aeea00',
    '#18ffff', '#b388ff', '#64ffda', '#ffc107', '#76ff03',
    '#ff8c42', '#00e676', '#f06292', '#26a69a', '#b0bec5',
    '#ce93d8', '#ffab40', '#80deea', '#ef5350', '#aed581',
    '#4fc3f7', '#ffcc80', '#81c784', '#e57373', '#90caf9'
  ];
  var autoColorIdx = 0;

  // 取得 zone 顏色：有手動指定就用，沒有就從色池自動分配
  function zoneColor(z) {
    if (!z) return DEFAULT_NODE_COLOR;
    if (ZONE_COLORS[z]) return ZONE_COLORS[z];
    // 自動分配：跳過已被手動指定佔用的顏色
    var used = {};
    Object.keys(ZONE_COLORS).forEach(function (k) { used[ZONE_COLORS[k]] = true; });
    for (var i = 0; i < COLOR_POOL.length; i++) {
      var idx = (autoColorIdx + i) % COLOR_POOL.length;
      if (!used[COLOR_POOL[idx]]) {
        ZONE_COLORS[z] = COLOR_POOL[idx];
        autoColorIdx = idx + 1;
        return COLOR_POOL[idx];
      }
    }
    // 色池用完就回頭重複（30 色應該夠用）
    ZONE_COLORS[z] = COLOR_POOL[autoColorIdx % COLOR_POOL.length];
    autoColorIdx++;
    return ZONE_COLORS[z];
  }

  // ─── 狀態 ───
  var mode = 'read'; // 'read' | 'edit'
  var network = null;
  var nodesDS = null;
  var edgesDS = null;
  var currentPlayerRoomId = null;
  var allNodes = [];   // 原始節點陣列（來自 API）
  var allEdges = [];   // 原始連線陣列（來自 API）
  var selectedNodeId = null;
  var zoneFilterValue = '';
  var pendingEdge = null; // addEdge 時暫存 {from, to}

  // ─── DOM ───
  var $ = function (id) { return document.getElementById(id); };
  var ui = {
    status: $('status'),
    error: $('error-msg'),
    zoneFilter: $('zone-filter'),
    btnModeRead: $('btn-mode-read'),
    btnModeEdit: $('btn-mode-edit'),
    sidePanel: $('sidePanel'),
    fId: $('f-id'),
    fName: $('f-name'),
    fZone: $('f-zone'),
    fTags: $('f-tags'),
    fDesc: $('f-desc'),
    btnSave: $('btn-save'),
    btnDelete: $('btn-delete'),
    // link modal
    linkModal: $('linkModal'),
    linkDesc: $('link-desc'),
    linkDir: $('link-dir'),
    linkReverse: $('link-reverse'),
    linkRevDir: $('link-rev-dir'),
    linkRevWrap: $('link-rev-wrap'),
    linkOk: $('link-ok'),
    linkCancel: $('link-cancel'),
    // add modal
    addModal: $('addModal'),
    addName: $('add-name'),
    addOk: $('add-ok'),
    addCancel: $('add-cancel'),
    // login
    inputId: $('inputId'),
    inputPw: $('inputPw'),
    btnLogin: $('btnLogin')
  };

  // ─── 工具函式 ───
  function mgQuery() {
    return MG_KEY ? '?mg_key=' + encodeURIComponent(MG_KEY) : '';
  }

  function setStatus(t) { ui.status.textContent = t; }
  function setError(t) { ui.error.textContent = t; }


  function apiPost(url, body) {
    return fetch(url + mgQuery(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    }).then(function (r) { return r.json(); });
  }
  function apiPut(url, body) {
    return fetch(url + mgQuery(), {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    }).then(function (r) { return r.json(); });
  }
  function apiDelete(url, body) {
    return fetch(url + mgQuery(), {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: body ? JSON.stringify(body) : undefined
    }).then(function (r) { return r.json(); });
  }

  // ─── ID 自動生成 ───
  // 取連線目標 ID 的前五層（用 _ 分割），第六層用流水號
  function generateId(targetNodeId) {
    var parts = targetNodeId.split('_');
    var prefix = parts.slice(0, 5).join('_');
    // 掃描所有現有 ID，找同 prefix 的最大數字
    var max = -1;
    allNodes.forEach(function (n) {
      var nParts = n.id.split('_');
      var nPrefix = nParts.slice(0, 5).join('_');
      if (nPrefix === prefix) {
        var tail = nParts[5];
        var num = parseInt(tail, 10);
        if (!isNaN(num) && num > max) max = num;
      }
    });
    return prefix + '_' + (max + 1);
  }

  // ─── Zone 濾鏡 ───
  function refreshZoneFilter() {
    var zones = {};
    allNodes.forEach(function (n) {
      if (n.zone) zones[n.zone] = (zones[n.zone] || 0) + 1;
    });
    var sorted = Object.keys(zones).sort();
    var prev = zoneFilterValue;
    ui.zoneFilter.innerHTML = '<option value="">全部</option>';
    sorted.forEach(function (z) {
      var opt = document.createElement('option');
      opt.value = z;
      opt.textContent = z + '（' + zones[z] + '）';
      ui.zoneFilter.appendChild(opt);
    });
    if (prev && zones[prev]) ui.zoneFilter.value = prev;
  }

  function applyZoneFilter() {
    if (!nodesDS) return;
    var visibleNodeIds = {};
    allNodes.forEach(function (n) {
      if (!zoneFilterValue || n.zone === zoneFilterValue) {
        visibleNodeIds[n.id] = true;
      }
    });
    // 更新 nodes DataSet
    var toShow = allNodes.filter(function (n) { return visibleNodeIds[n.id]; });
    var toShowEdges = allEdges.filter(function (e) {
      return visibleNodeIds[e.from] || visibleNodeIds[e.to];
    });
    nodesDS.clear();
    edgesDS.clear();
    toShow.forEach(function (n) { nodesDS.add(makeVisNode(n)); });
    toShowEdges.forEach(function (e) { edgesDS.add(makeVisEdge(e)); });
  }

  // ─── vis-network 節點/邊構建 ───
  function makeVisNode(n) {
    var color = zoneColor(n.zone);
    var exits = 0;
    allEdges.forEach(function (e) { if (e.from === n.id) exits++; });
    var size = 8 + Math.max(0, exits - 1) * 3;
    var isPlayer = currentPlayerRoomId && n.id === currentPlayerRoomId;
    if (isPlayer) size *= 1.4;
    var node = {
      id: n.id,
      label: n.name + '\n(' + n.id + ')',
      title: buildTooltip(n),
      size: size,
      color: color
    };
    if (isPlayer) {
      node.color = {
        background: color,
        border: 'rgba(233, 69, 96, 0.75)',
        highlight: { background: color, border: '#e94560' },
        hover: { background: color, border: 'rgba(233, 69, 96, 0.75)' }
      };
      node.borderWidth = 6;
      node.borderWidthSelected = 7;
    }
    return node;
  }

  function buildTooltip(n) {
    var lines = ['【' + n.name + '】'];
    if (n.zone) lines.push('區域：' + n.zone);
    if (n.tags && n.tags.length) lines.push('標籤：' + n.tags.join('、'));
    if (n.description) lines.push('描述：' + n.description);
    return lines.join('\n');
  }

  function makeVisEdge(e) {
    // 檢查是否雙向
    var rev = allEdges.some(function (r) { return r.from === e.to && r.to === e.from; });
    // 避免重複繪製雙向線：只保留 from < to 的那條
    if (rev && e.from > e.to) return null;
    var edge = {
      id: e.from + '|' + e.to,
      from: e.from,
      to: e.to,
      width: 1.5
    };
    if (rev) {
      edge.arrows = { to: { enabled: true, type: 'arrow' }, from: { enabled: true, type: 'arrow' } };
      edge.dashes = false;
    } else {
      edge.arrows = { to: { enabled: true, type: 'arrow' } };
      edge.dashes = [6, 4];
    }
    return edge;
  }

  // ─── 繪圖 ───
  function drawGraph() {
    var container = document.getElementById('mynetwork');
    var visNodes = [];
    var visEdges = [];
    var visibleIds = {};
    allNodes.forEach(function (n) {
      if (!zoneFilterValue || n.zone === zoneFilterValue) {
        visibleIds[n.id] = true;
        visNodes.push(makeVisNode(n));
      }
    });
    var edgeSeen = {};
    allEdges.forEach(function (e) {
      if (!visibleIds[e.from] && !visibleIds[e.to]) return;
      var ve = makeVisEdge(e);
      if (!ve) return;
      if (edgeSeen[ve.id]) return;
      edgeSeen[ve.id] = true;
      visEdges.push(ve);
    });

    if (!nodesDS) {
      nodesDS = new vis.DataSet(visNodes);
      edgesDS = new vis.DataSet(visEdges);
    } else {
      nodesDS.clear();
      edgesDS.clear();
      visNodes.forEach(function (n) { nodesDS.add(n); });
      visEdges.forEach(function (e) { edgesDS.add(e); });
    }

    var options = getVisOptions();
    if (!network) {
      network = new vis.Network(container, { nodes: nodesDS, edges: edgesDS }, options);
      bindNetworkEvents();
    } else {
      network.setData({ nodes: nodesDS, edges: edgesDS });
      network.setOptions(options);
    }
    if (currentPlayerRoomId) {
      network.focus(currentPlayerRoomId, { scale: 1.2, animation: true });
    }
  }

  function getVisOptions() {
    var base = {
      nodes: {
        font: { color: '#e8e8e8', size: 12, face: 'Noto Sans TC, Microsoft JhengHei, sans-serif' },
        borderWidth: 1, borderWidthSelected: 2, shape: 'dot'
      },
      edges: {
        color: { color: '#4a5568', highlight: '#e94560' },
        smooth: { type: 'continuous', roundness: 0.2 },
        arrowStrikethrough: false
      },
      interaction: { hover: true, zoomView: true, dragView: true, tooltipDelay: 100 }
    };
    if (mode === 'read') {
      base.physics = {
        enabled: true,
        solver: 'forceAtlas2Based',
        forceAtlas2Based: {
          gravitationalConstant: -80, centralGravity: 0.01,
          springLength: 120, springConstant: 0.08,
          damping: 0.4, avoidOverlap: 0.5
        },
        stabilization: { iterations: 200, updateInterval: 25 }
      };
      base.manipulation = { enabled: false };
    } else {
      // 編輯模式：關閉物理（拖哪放哪），開啟 manipulation
      base.physics = { enabled: false };
      base.manipulation = {
        enabled: true,
        addNode: onAddNode,
        addEdge: onAddEdge,
        deleteNode: onDeleteNode,
        deleteEdge: onDeleteEdge,
        editNode: onEditNode
      };
      base.interaction.dragNodes = true;
    }
    return base;
  }

  function bindNetworkEvents() {
    network.on('click', function (params) {
      if (mode !== 'edit') return;
      if (params.nodes.length === 1) {
        selectNode(params.nodes[0]);
      } else {
        deselectNode();
      }
    });

    // 編輯模式：拖曳結束後存座標
    network.on('dragEnd', function (params) {
      if (mode !== 'edit') return;
      if (params.nodes.length === 0) return;
      var positions = network.getPositions(params.nodes);
      apiPut(API_LAYOUT, { positions: positions }).catch(function () {});
    });
  }

  // ─── 編輯模式：manipulation 回調 ───

  // addNode：vis-network 在使用者點「新增節點」按鈕並點擊畫布後呼叫
  function onAddNode(nodeData, callback) {
    // 顯示新增對話框
    ui.addName.value = '';
    ui.addModal.style.display = 'flex';
    ui.addName.focus();

    ui.addOk.onclick = function () {
      var name = ui.addName.value.trim();
      if (!name) { ui.addName.focus(); return; }
      ui.addModal.style.display = 'none';

      // 暫時用 timestamp 作臨時 ID，連線後會自動產生正式 ID
      var tempId = 'new_' + Date.now();
      var room = { id: tempId, name: name, description: '', zone: '', tags: [] };

      apiPost(API_ROOM, room).then(function (res) {
        if (!res.ok) { setError(res.error || '新增失敗'); callback(null); return; }
        room.id = res.id || tempId;
        allNodes.push(room);
        refreshZoneFilter();
        nodeData.id = room.id;
        nodeData.label = room.name + '\n(' + room.id + ')';
        nodeData.color = DEFAULT_NODE_COLOR;
        nodeData.size = 8;
        callback(nodeData);
        // 存座標
        var pos = {}; pos[room.id] = { x: nodeData.x, y: nodeData.y };
        apiPut(API_LAYOUT, { positions: pos }).catch(function () {});
        selectNode(room.id);
        setStatus('已新增：' + room.name);
      }).catch(function (err) { setError(err.message); callback(null); });
    };
    ui.addCancel.onclick = function () {
      ui.addModal.style.display = 'none';
      callback(null);
    };
  }

  // addEdge：使用者從一個節點拖線到另一個節點後呼叫
  function onAddEdge(edgeData, callback) {
    if (edgeData.from === edgeData.to) { callback(null); return; }
    pendingEdge = { from: edgeData.from, to: edgeData.to, callback: callback, edgeData: edgeData };

    var fromNode = findNode(edgeData.from);
    var toNode = findNode(edgeData.to);
    ui.linkDesc.textContent = (fromNode ? fromNode.name : edgeData.from) + ' → ' + (toNode ? toNode.name : edgeData.to);
    ui.linkDir.value = '';
    ui.linkRevDir.value = '';
    ui.linkReverse.checked = true;
    ui.linkRevWrap.style.display = '';
    ui.linkModal.style.display = 'flex';
    ui.linkDir.focus();

    ui.linkReverse.onchange = function () {
      ui.linkRevWrap.style.display = ui.linkReverse.checked ? '' : 'none';
    };
  }

  // link modal 確定
  ui.linkOk.addEventListener('click', function () {
    if (!pendingEdge) return;
    var pe = pendingEdge;
    pendingEdge = null;
    ui.linkModal.style.display = 'none';

    var body = {
      from: pe.from,
      to: pe.to,
      direction: ui.linkDir.value.trim(),
      reverse: ui.linkReverse.checked,
      reverse_direction: ui.linkRevDir.value.trim()
    };

    apiPost(API_LINK, body).then(function (res) {
      if (!res.ok) { setError(res.error || '連線失敗'); pe.callback(null); return; }

      // 加入 allEdges
      allEdges.push({ from: pe.from, to: pe.to, direction: body.direction });
      if (body.reverse) {
        allEdges.push({ from: pe.to, to: pe.from, direction: body.reverse_direction });
      }

      // 連線後自動繼承：zone、tags
      var fromNode = findNode(pe.from);
      var toNode = findNode(pe.to);
      // 對新建的房間（zone 為空的那一方），繼承對方的 zone/tags
      inheritProps(fromNode, toNode);
      inheritProps(toNode, fromNode);

      drawGraph(); // 重繪以正確顯示雙向/單向
      setStatus('已連線');
    }).catch(function (err) { setError(err.message); pe.callback(null); });
  });

  ui.linkCancel.addEventListener('click', function () {
    if (pendingEdge) { pendingEdge.callback(null); pendingEdge = null; }
    ui.linkModal.style.display = 'none';
  });

  // 連線後自動繼承 zone/tags：如果 target 的 zone 為空，從 source 繼承
  function inheritProps(target, source) {
    if (!target || !source) return;
    if (target.zone || !source.zone) return; // target 已有 zone 或 source 沒有，不繼承
    target.zone = source.zone;
    if ((!target.tags || target.tags.length === 0) && source.tags && source.tags.length > 0) {
      target.tags = source.tags.slice();
    }
    // 生成正式 ID（如果是臨時 ID）
    var newId = generateId(source.id);
    if (target.id.startsWith('new_') && newId !== target.id) {
      var oldId = target.id;
      target.id = newId;
      // 更新 allEdges 裡的引用
      allEdges.forEach(function (e) {
        if (e.from === oldId) e.from = newId;
        if (e.to === oldId) e.to = newId;
      });
    }
    // 儲存更新到後端
    apiPut(API_ROOM + '/' + encodeURIComponent(target.id), {
      name: target.name,
      description: target.description || '',
      zone: target.zone,
      tags: target.tags || [],
      objects: target.objects || []
    }).catch(function () {});
  }

  function onDeleteNode(data, callback) {
    if (!data.nodes.length) { callback(null); return; }
    var id = data.nodes[0];
    var node = findNode(id);
    if (!confirm('確定刪除房間「' + (node ? node.name : id) + '」？')) { callback(null); return; }
    apiDelete(API_ROOM + '/' + encodeURIComponent(id)).then(function (res) {
      if (!res.ok) { setError(res.error || '刪除失敗'); callback(null); return; }
      allNodes = allNodes.filter(function (n) { return n.id !== id; });
      allEdges = allEdges.filter(function (e) { return e.from !== id && e.to !== id; });
      refreshZoneFilter();
      deselectNode();
      callback(data);
      setStatus('已刪除：' + id);
    }).catch(function (err) { setError(err.message); callback(null); });
  }

  function onDeleteEdge(data, callback) {
    if (!data.edges.length) { callback(null); return; }
    // edge id 格式是 "from|to"
    var edgeId = data.edges[0];
    var parts = edgeId.split('|');
    if (parts.length !== 2) { callback(data); return; }
    var from = parts[0], to = parts[1];
    // 檢查是否雙向
    var isBidir = allEdges.some(function (e) { return e.from === to && e.to === from; });
    apiDelete(API_LINK, { from: from, to: to, reverse: isBidir }).then(function (res) {
      if (!res.ok) { setError(res.error || '刪除連線失敗'); callback(null); return; }
      allEdges = allEdges.filter(function (e) {
        if (e.from === from && e.to === to) return false;
        if (isBidir && e.from === to && e.to === from) return false;
        return true;
      });
      callback(data);
      setStatus('已刪除連線');
    }).catch(function (err) { setError(err.message); callback(null); });
  }

  function onEditNode(nodeData, callback) {
    // 雙擊節點時觸發，用側面板編輯
    selectNode(nodeData.id);
    callback(null); // 取消 vis-network 預設的編輯 UI
  }

  // ─── 側面板 ───
  function findNode(id) {
    for (var i = 0; i < allNodes.length; i++) {
      if (allNodes[i].id === id) return allNodes[i];
    }
    return null;
  }

  function selectNode(id) {
    selectedNodeId = id;
    var n = findNode(id);
    if (!n) return;
    ui.fId.value = n.id;
    ui.fName.value = n.name || '';
    ui.fZone.value = n.zone || '';
    ui.fTags.value = (n.tags || []).join(', ');
    ui.fDesc.value = n.description || '';
  }

  function deselectNode() {
    selectedNodeId = null;
    ui.fId.value = '';
    ui.fName.value = '';
    ui.fZone.value = '';
    ui.fTags.value = '';
    ui.fDesc.value = '';
  }

  // 儲存按鈕
  ui.btnSave.addEventListener('click', function () {
    if (!selectedNodeId) return;
    var n = findNode(selectedNodeId);
    if (!n) return;

    var newId = ui.fId.value.trim();
    var body = {
      name: ui.fName.value.trim(),
      description: ui.fDesc.value.trim(),
      zone: ui.fZone.value.trim(),
      tags: ui.fTags.value.split(',').map(function (s) { return s.trim(); }).filter(Boolean),
      objects: n.objects || []
    };

    // 如果 ID 被修改了，需要先建立新房間再刪除舊的
    if (newId && newId !== selectedNodeId) {
      // 建新的
      var createBody = Object.assign({ id: newId }, body);
      apiPost(API_ROOM, createBody).then(function (res) {
        if (!res.ok) { setError(res.error || 'ID 更改失敗'); return; }
        // 刪舊的
        return apiDelete(API_ROOM + '/' + encodeURIComponent(selectedNodeId));
      }).then(function () {
        // 更新本地
        n.id = newId;
        n.name = body.name;
        n.description = body.description;
        n.zone = body.zone;
        n.tags = body.tags;
        allEdges.forEach(function (e) {
          if (e.from === selectedNodeId) e.from = newId;
          if (e.to === selectedNodeId) e.to = newId;
        });
        selectedNodeId = newId;
        refreshZoneFilter();
        drawGraph();
        setStatus('已儲存（ID 已更改）');
      }).catch(function (err) { setError(err.message); });
    } else {
      apiPut(API_ROOM + '/' + encodeURIComponent(selectedNodeId), body).then(function (res) {
        if (!res.ok) { setError(res.error || '儲存失敗'); return; }
        n.name = body.name;
        n.description = body.description;
        n.zone = body.zone;
        n.tags = body.tags;
        refreshZoneFilter();
        drawGraph();
        setStatus('已儲存：' + n.name);
      }).catch(function (err) { setError(err.message); });
    }
  });

  // 刪除按鈕
  ui.btnDelete.addEventListener('click', function () {
    if (!selectedNodeId) return;
    var n = findNode(selectedNodeId);
    if (!confirm('確定刪除房間「' + (n ? n.name : selectedNodeId) + '」？')) return;
    apiDelete(API_ROOM + '/' + encodeURIComponent(selectedNodeId)).then(function (res) {
      if (!res.ok) { setError(res.error || '刪除失敗'); return; }
      allNodes = allNodes.filter(function (nd) { return nd.id !== selectedNodeId; });
      allEdges = allEdges.filter(function (e) { return e.from !== selectedNodeId && e.to !== selectedNodeId; });
      deselectNode();
      refreshZoneFilter();
      drawGraph();
      setStatus('已刪除');
    }).catch(function (err) { setError(err.message); });
  });

  // ─── 模式切換 ───
  function switchMode(newMode) {
    mode = newMode;
    ui.btnModeRead.classList.toggle('active', mode === 'read');
    ui.btnModeEdit.classList.toggle('active', mode === 'edit');
    document.body.classList.toggle('edit-mode', mode === 'edit');
    deselectNode();

    if (mode === 'edit') {
      // 編輯模式：從 editor API 載入（含 layout 座標）
      loadFromEditorAPI();
    } else {
      // 閱讀模式：從 rooms.json 載入（含物理引擎佈局）
      loadFromRoomsJson();
    }
  }

  ui.btnModeRead.addEventListener('click', function () { switchMode('read'); });
  ui.btnModeEdit.addEventListener('click', function () { switchMode('edit'); });

  // ─── 資料載入 ───
  function loadFromRoomsJson() {
    setStatus('載入中…');
    fetch(DATA_URL).then(function (r) { return r.json(); }).then(function (json) {
      var rooms = json.rooms || [];
      allNodes = rooms.map(function (r) {
        return { id: r.id, name: r.name || r.id, zone: r.zone || '', tags: r.tags || [], description: r.description || '', objects: r.objects || [] };
      });
      allEdges = [];
      rooms.forEach(function (r) {
        if (!Array.isArray(r.objects)) return;
        r.objects.forEach(function (obj) {
          if (obj.move_to_room_id) {
            allEdges.push({ from: r.id, to: obj.move_to_room_id, direction: obj.name || '' });
          }
        });
      });
      refreshZoneFilter();
      drawGraph();
      setStatus('已載入 ' + allNodes.length + ' 房間，' + allEdges.length + ' 連線（閱讀模式）');
    }).catch(function (err) {
      setError(err.message || '載入失敗');
    });
  }

  function loadFromEditorAPI() {
    setStatus('載入編輯資料…');
    fetch(API_GRAPH + mgQuery()).then(function (r) { return r.json(); }).then(function (data) {
      allNodes = (data.nodes || []).map(function (n) {
        return { id: n.id, name: n.name, zone: n.zone || '', tags: n.tags || [], description: n.description || '', objects: n.objects || [] };
      });
      allEdges = (data.edges || []).map(function (e) {
        return { from: e.from, to: e.to, direction: e.direction || '' };
      });
      // 套用 layout 座標
      var layout = data.layout || {};
      refreshZoneFilter();
      drawGraph();
      // 設定已知座標
      var positions = {};
      var hasPos = false;
      allNodes.forEach(function (n) {
        if (layout[n.id]) {
          positions[n.id] = { x: layout[n.id].x, y: layout[n.id].y };
          hasPos = true;
        }
      });
      if (hasPos && network) {
        network.setOptions({ physics: { enabled: false } });
        Object.keys(positions).forEach(function (id) {
          try { nodesDS.update({ id: id, x: positions[id].x, y: positions[id].y }); } catch (_) {}
        });
        network.fit();
      }
      // 清理孤兒：new_ 開頭且零連線的房間靜默刪除
      var orphans = allNodes.filter(function (n) {
        if (!n.id.startsWith('new_')) return false;
        return !allEdges.some(function (e) { return e.from === n.id || e.to === n.id; });
      });
      orphans.forEach(function (o) {
        apiDelete(API_ROOM + '/' + encodeURIComponent(o.id)).catch(function () {});
        allNodes = allNodes.filter(function (n) { return n.id !== o.id; });
      });
      if (orphans.length) drawGraph();

      setStatus('已載入 ' + allNodes.length + ' 房間，' + allEdges.length + ' 連線（編輯模式）' + (orphans.length ? '（已清理 ' + orphans.length + ' 個孤兒）' : ''));
    }).catch(function (err) {
      setError(err.message || '載入失敗');
    });
  }

  // ─── Zone 濾鏡事件 ───
  ui.zoneFilter.addEventListener('change', function () {
    zoneFilterValue = ui.zoneFilter.value;
    drawGraph();
  });

  // ─── 登入 ───
  function doLogin() {
    var id = ui.inputId.value.trim();
    var pw = ui.inputPw.value.trim();
    if (!id || !pw) { setError('請輸入角色 ID 與密碼'); return; }
    setError('');
    ui.btnLogin.disabled = true;
    ui.btnLogin.textContent = '驗證中…';
    fetch('/api/player-room?id=' + encodeURIComponent(id) + '&pw=' + encodeURIComponent(pw))
      .then(function (r) {
        if (!r.ok) return r.json().then(function (b) { throw new Error(b.error || '驗證失敗'); });
        return r.json();
      })
      .then(function (data) {
        currentPlayerRoomId = data.room_id || null;
        drawGraph();
        if (currentPlayerRoomId && network) {
          network.focus(currentPlayerRoomId, { scale: 1.2, animation: true });
        }
      })
      .catch(function (err) { setError(err.message || '登入失敗'); })
      .finally(function () { ui.btnLogin.disabled = false; ui.btnLogin.textContent = '登入'; });
  }
  ui.btnLogin.addEventListener('click', doLogin);
  ui.inputPw.addEventListener('keydown', function (e) { if (e.key === 'Enter') doLogin(); });
  ui.inputId.addEventListener('keydown', function (e) { if (e.key === 'Enter') ui.inputPw.focus(); });

  // ─── 鍵盤快捷鍵 ───
  document.addEventListener('keydown', function (e) {
    if (mode !== 'edit') return;
    // Ctrl+S 儲存
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      ui.btnSave.click();
    }
    // Delete 刪除選中
    if (e.key === 'Delete' && selectedNodeId && document.activeElement.tagName !== 'INPUT' && document.activeElement.tagName !== 'TEXTAREA') {
      ui.btnDelete.click();
    }
  });

  // ─── 啟動 ───
  loadFromRoomsJson();
})();
```

---

## 步驟 4：更新 `<title>`

```html
<title>浮生地圖 v0.13.0</title>
```

---

## 步驟 5：更新版本號

把 JS 裡的 `DATA_URL` 版本號改成 `0.13.0`：

```javascript
var DATA_URL = '/data/rooms.json?v=0.13.0';
```

---

## 注意事項（碼農請讀）

1. **不要動後端**。所有 API 都已經存在，不需要新增路由
2. **不要引入新的 CDN 或套件**。vis-network 已經引入了
3. **ZONE_COLORS 預設為空物件**。`zoneColor()` 函式會自動從 30 色高對比色池分配未使用的顏色給新偵測到的 zone。手動指定的配色優先。不要硬編碼 zone 配色
4. **ID 自動生成邏輯**：取連線目標 ID 前五層（`_` 分割），第六層用流水號。例如連到 `zonelife_city_life_01f_cofe_0`，新 ID 就是 `zonelife_city_life_01f_cofe_2`（假設 0 和 1 已存在）
5. **新增房間時只需輸入名稱**。zone、tags、描述全部留空。連線後自動繼承連線對象的 zone 和 tags
6. **描述永遠留空**，不自動生成。之後會另外處理
7. **ID 可事後修改**。側面板的 ID 欄位是可編輯的，儲存時如果 ID 改了就建新刪舊
8. **閱讀模式用物理引擎**（蒲公英自動佈局），**編輯模式關物理**（拖哪放哪，拖完存座標）
9. **單檔**。HTML + CSS + JS 全寫在 `map_viewer.html` 裡面，不要拆檔案

---

## 驗證清單

- [ ] 開啟 `/map_viewer`，預設閱讀模式，蒲公英佈局正常顯示
- [ ] Zone 下拉選單列出所有 zone（含數量），選擇後正確過濾
- [ ] 點「編輯」切換到編輯模式，畫面出現側面板，蒲公英物理消失
- [ ] 點 vis-network 工具列的「新增節點」→ 點畫布 → 彈出名稱輸入框 → 輸入名稱 → 新節點出現
- [ ] 從一個節點拖線到另一個節點 → 彈出連線對話框 → 可選單向/雙向 → 確定後連線出現
- [ ] 新建房間連線後，zone/tags 自動繼承、ID 自動生成
- [ ] 點選節點 → 側面板顯示房間資料 → 可修改 → 點儲存
- [ ] 側面板修改 ID → 儲存 → ID 正確更新
- [ ] 選中節點按 Delete 或點「刪除房間」→ 確認後刪除
- [ ] 選中連線按 Delete → 刪除連線（雙向線兩邊都刪）
- [ ] 拖曳節點放開後座標自動存到後端
- [ ] 切回閱讀模式 → 側面板消失，物理引擎恢復
- [ ] 手機上可正常切換模式、新增、連線
- [ ] 登入功能仍正常（閱讀模式顯示玩家位置）

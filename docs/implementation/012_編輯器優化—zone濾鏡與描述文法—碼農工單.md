# 012 — 編輯器優化：Zone 濾鏡 + 描述文法（碼農工單）

> **你只需要照做，不要自己發明新東西。**
> **（2026-04：`web/room_editor.html`／`room_editor.js` 已刪除；以下路徑僅作歷史工單留存。）**
> 前端改動原只涉及 `web/room_editor.html` 和 `web/room_editor.js`，後端改動只涉及 `src/gametext/mod.rs`（或新增 `src/gametext/room_desc.rs`）。
> **不要引入新依賴。不要改其他檔案。**

---

## 任務一：Room Editor Zone 濾鏡

### 目標

在 room_editor 左側面板頂部加一個 zone 下拉選單。選了某個 zone 後，畫布上只顯示該 zone 的房間節點和相關連線，其餘隱藏。選「全部」恢復顯示所有。

### 步驟 1：在 HTML 加下拉選單

打開 `web/room_editor.html`，在 `.status` 元素（約第 113 行 `<div class="status" id="status">` ）的**下方**，`.toolbar` 的**上方**，插入：

```html
<div class="field" style="margin-bottom:8px;">
  <label>Zone 濾鏡</label>
  <select id="zone-filter" style="width:100%;border:1px solid var(--line);background:#0d1220;color:var(--text);border-radius:6px;padding:8px;">
    <option value="">全部顯示</option>
  </select>
</div>
```

### 步驟 2：在 JS 接上濾鏡邏輯

打開 `web/room_editor.js`。

#### 2a. 在 `const ui = {` 區塊（約第 34 行起）裡，加一行：

```javascript
  zoneFilter: document.getElementById('zone-filter'),
```

加在任意位置即可（例如 `fDesc` 下面）。

#### 2b. 在 `const state = {` 區塊（約第 3 行起）裡，加一行：

```javascript
  zoneFilterValue: '', // 目前選中的 zone，空字串 = 全部
```

#### 2c. 新增函式 `refreshZoneFilter()`，放在 `refreshPathSelects()` 函式**下方**（約第 232 行後）：

```javascript
function refreshZoneFilter() {
  const zones = new Set();
  for (const n of state.nodes.values()) {
    if (n.zone) zones.add(n.zone);
  }
  const sorted = Array.from(zones).sort();
  const prev = state.zoneFilterValue;
  ui.zoneFilter.innerHTML = '<option value="">全部顯示</option>';
  for (const z of sorted) {
    const opt = document.createElement('option');
    opt.value = z;
    opt.textContent = `${z}（${Array.from(state.nodes.values()).filter(n => n.zone === z).length}）`;
    ui.zoneFilter.appendChild(opt);
  }
  if (prev && zones.has(prev)) ui.zoneFilter.value = prev;
}
```

#### 2d. 新增函式 `isNodeVisible(id)`，放在 `refreshZoneFilter()` 下方：

```javascript
function isNodeVisible(id) {
  if (!state.zoneFilterValue) return true;
  const n = state.nodes.get(id);
  return n && n.zone === state.zoneFilterValue;
}
```

#### 2e. 修改 `render()` 函式

找到 `render()` 函式（約第 511 行）。

**節點部分**：找到繪製節點的迴圈。搜尋 `state.nodes.forEach` 或 `for (const [id, n] of state.nodes)`。在迴圈開頭加一行：

```javascript
if (!isNodeVisible(id)) continue; // zone 濾鏡
```

**連線部分**：找到繪製 edge 的迴圈 `for (const e of state.edges)`（約第 567 行）。在 `const from = state.layout[e.from];` 之前加：

```javascript
if (!isNodeVisible(e.from) && !isNodeVisible(e.to)) continue; // zone 濾鏡：兩端都不可見才隱藏
```

**群組部分**：找到繪製群組的迴圈 `state.groups.forEach((grp, gi) => {`（約第 543 行）。在 `const positions = grp.map(...)` 那行改成：

```javascript
const positions = grp.filter(id => isNodeVisible(id)).map((id) => state.layout[id]).filter(Boolean);
```

#### 2f. 綁定事件

找到檔案底部的初始化區域（搜尋 `window.addEventListener` 或 `DOMContentLoaded` 或 `loadGraph`）。在 graph 載入完成後（通常在 `loadGraph()` 的 `.then` 或 `await` 之後），加入：

```javascript
refreshZoneFilter();
ui.zoneFilter.addEventListener('change', () => {
  state.zoneFilterValue = ui.zoneFilter.value;
  scheduleRender();
});
```

也在 `refreshPathSelects()` 被呼叫的地方附近呼叫一次 `refreshZoneFilter()`，確保新增/刪除房間後 zone 列表更新。

### 驗證

1. 打開 room_editor 頁面
2. 下拉選單應列出所有 zone（如 `citylife_1f（11）`、`citylife_3f（64）` 等）
3. 選一個 zone → 畫布只顯示該 zone 的節點和連線
4. 選「全部顯示」→ 恢復全部
5. 手機上下拉選單要能正常操作

---

## 任務二：房間描述文法（自動生成 Look 描述）

### 目標

新增一個描述模板系統。根據房間的 zone + tags，從預定義的片段庫中隨機組裝出 Look 描述。用於：
1. 新建房間時自動填入預設描述
2. 提供一個「重新生成描述」按鈕讓使用者重骰

### 步驟 1：在 `data/config/` 新增 `room_desc_templates.json`

建立檔案 `data/config/room_desc_templates.json`，內容如下（這是種子模板，後續可擴充）：

```json
{
  "fragments": {
    "atmosphere": {
      "market": ["人聲鼎沸的街道。", "熱鬧的攤位前擠滿了人。", "嘈雜的叫賣聲不絕於耳。"],
      "residential": ["安靜的走廊，門扉緊閉。", "幾扇門半掩著，透出微光。", "走廊盡頭的燈泡忽明忽暗。"],
      "office": ["整潔的空間，桌椅排列整齊。", "牆上掛著幾幅通告。", "空氣中瀰漫著紙墨的氣味。"],
      "food": ["空氣中飄著食物的香氣。", "幾張桌子上擺著未收的碗盤。", "灶台上的火焰搖曳不定。"],
      "shop": ["貨架上擺滿了各式商品。", "店主在櫃檯後忙碌著。", "玻璃櫥窗映著街上的人影。"],
      "transit": ["人來人往，腳步匆匆。", "這裡是必經之路。", "幾個路人匆忙走過。"],
      "default": ["四周安靜。", "空間不大不小。", "光線從某處透了進來。"]
    },
    "sound": {
      "market": ["叫賣聲此起彼落。", "銅板碰撞的聲響不時傳來。"],
      "residential": ["偶爾傳來關門聲。", "遠處有人在說話。"],
      "food": ["鍋鏟翻炒的聲音從後方傳來。", "碗碟碰撞的清脆聲響。"],
      "shop": ["算盤珠子撥動的聲音。", "門鈴偶爾響起。"],
      "default": ["周圍很安靜。", "風從遠處吹來。"]
    },
    "smell": {
      "food": ["烤肉的油煙混著香料味撲面而來。", "鮮湯的蒸氣帶著暖意。", "剛出爐的麵包香氣四溢。"],
      "coffee": ["咖啡豆的焦香在空氣中盤旋。", "新鮮研磨的咖啡香撲鼻。"],
      "market": ["各種氣味混雜在一起。", "新鮮蔬果的清新味道。"],
      "default": []
    }
  },
  "pattern": [
    { "slot": "atmosphere", "required": true },
    { "slot": "sound", "required": false },
    { "slot": "smell", "required": false }
  ]
}
```

### 步驟 2：前端——在 room_editor.js 加描述生成

在 `web/room_editor.js` 中，加入以下函式（放在 `renderObjectsForm` 前面即可）：

```javascript
let descTemplates = null;

async function loadDescTemplates() {
  try {
    const res = await fetch('/data/config/room_desc_templates.json');
    descTemplates = await res.json();
  } catch (_) {
    descTemplates = null;
  }
}

function generateRoomDesc(zone, tags) {
  if (!descTemplates) return '';
  const frags = descTemplates.fragments;
  const pattern = descTemplates.pattern;
  const parts = [];

  for (const p of pattern) {
    const slotPool = frags[p.slot];
    if (!slotPool) continue;

    // 依優先順序找匹配的 key：tags → zone 關鍵字 → default
    let candidates = [];
    // 先找 tags 匹配
    for (const tag of (tags || [])) {
      if (slotPool[tag] && slotPool[tag].length > 0) {
        candidates = candidates.concat(slotPool[tag]);
      }
    }
    // 沒有則找 zone 關鍵字
    if (candidates.length === 0) {
      for (const key of Object.keys(slotPool)) {
        if (key !== 'default' && zone && zone.includes(key)) {
          candidates = candidates.concat(slotPool[key]);
        }
      }
    }
    // 還是沒有則用 default
    if (candidates.length === 0 && slotPool['default']) {
      candidates = slotPool['default'];
    }
    if (candidates.length === 0) {
      if (p.required) parts.push('');
      continue;
    }
    // 隨機挑一條
    parts.push(candidates[Math.floor(Math.random() * candidates.length)]);
  }

  return parts.filter(Boolean).join('');
}
```

### 步驟 3：在新增房間時自動填入描述

找到新增房間的邏輯（搜尋 `POST` 和 `room-editor/room`），在房間建立成功後，如果描述為空，自動生成：

```javascript
// 在房間建立成功、選中新房間後
if (!ui.fDesc.value.trim()) {
  const zone = ui.fZone.value;
  const tags = parseTags(ui.fTags.value);
  const desc = generateRoomDesc(zone, tags);
  if (desc) {
    ui.fDesc.value = desc;
    // 觸發自動儲存或標記需要儲存
  }
}
```

### 步驟 4：加「重新生成描述」按鈕

在 `room_editor.html` 中，找到描述 textarea（`<textarea id="f-desc"`），在它**下方**加一個按鈕：

```html
<button type="button" id="btn-regen-desc" style="font-size:12px;padding:4px 8px;margin-top:4px;">重新骰描述</button>
```

在 `room_editor.js` 中綁定事件：

```javascript
document.getElementById('btn-regen-desc').addEventListener('click', () => {
  const zone = ui.fZone.value;
  const tags = parseTags(ui.fTags.value);
  const desc = generateRoomDesc(zone, tags);
  if (desc) ui.fDesc.value = desc;
});
```

### 步驟 5：初始化時載入模板

在 `loadGraph()` 附近（或 `DOMContentLoaded`），加入：

```javascript
loadDescTemplates();
```

### 驗證

1. 新增一間房間，zone 設為 `citylife_1f`，tags 設為 `food`
2. 描述欄應自動出現食物相關描述（如「空氣中飄著食物的香氣。鍋鏟翻炒的聲音從後方傳來。」）
3. 點「重新骰描述」→ 描述更換為不同組合
4. 沒有 tags 的房間 → 使用 default 描述
5. 模板 JSON 可以直接編輯擴充，不需改代碼

---

## 完成後

```bash
# 前端改動不需要 cargo build，但確認後端沒被改壞：
cargo build --release && cargo clippy -- -D warnings
```

手動測試：
1. 開啟 room_editor 頁面，確認 zone 下拉選單出現
2. 選不同 zone，確認節點正確過濾
3. 新建房間，確認描述自動生成
4. 點「重新骰描述」，確認描述變化

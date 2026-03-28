# 015 Plan B：前端描述與互動分離（碼農工單）

## 目標

將房間內容的渲染從「描述文字內嵌互動物件」改為「描述 + 獨立互動區」。
描述變成純氛圍文字，互動物件（出口、可看/可拿/可採集的東西）由前端自動渲染。

---

## 改動範圍

| 檔案 | 改什麼 |
|------|--------|
| `web/mud-text.js` | `updateRoomView()` 和 `formatDesc()` 改造 |
| `web/index.html` | 新增互動區容器 |
| `web/style.css` | 互動區樣式 |

**不動後端**。後端已經分開傳 `description`、`exits`、`objects`，前端只是沒用好。

---

## 現況問題

目前 `formatDesc()` 做了兩輪正則替換：
1. 把 `〔物件名〕` 替換成可點擊 span
2. 把描述文字中的純物件名也替換成可點擊 span

問題：
- 描述裡嵌入了互動元素（Move/Look），改描述就得改互動，改互動就得改描述
- 物件名的正則匹配容易誤包（短名包長名子串）
- 新房間（Parser A 產出）的 description 是空的或純氛圍文字，沒有 〔〕 標記

---

## 改法

### 1. HTML 結構（index.html）

在 `#room-desc` 後面加互動區：

```html
<section id="room-desc-panel" class="panel room-desc-panel">
    <div id="room-desc" class="room-desc"></div>
    <!-- ▼ 新增：互動區 ▼ -->
    <div id="room-actions" class="room-actions"></div>
    <!-- ▲ 新增 ▲ -->
    <div id="room-presence" class="room-presence" aria-live="polite"></div>
</section>
```

### 2. 互動區渲染邏輯（mud-text.js）

在 `updateRoomView()` 裡，呼叫新函式 `renderRoomActions(exits, objects)`：

```javascript
function renderRoomActions(exits, objects) {
    var el = document.getElementById('room-actions');
    if (!el) return;
    var html = '';

    // ─── 出口 ───
    if (exits && exits.length > 0) {
        html += '<div class="action-group">';
        html += '<span class="action-label">🚪 出口</span>';
        exits.forEach(function (ex) {
            html += '<button class="action-btn action-move" '
                + 'data-direction="' + escapeHtml(ex.direction) + '" '
                + 'title="前往 ' + escapeHtml(ex.to_room_name || '') + '">'
                + escapeHtml(ex.direction)
                + '</button>';
        });
        html += '</div>';
    }

    // ─── 物件（按動作分組）───
    // Move 類型的物件已由 exits 處理，跳過
    var nonMoveObjects = (objects || []).filter(function (o) {
        return !o.actions || o.actions.indexOf('Move') === -1;
    });

    if (nonMoveObjects.length > 0) {
        html += '<div class="action-group">';
        html += '<span class="action-label">👁 可互動</span>';
        nonMoveObjects.forEach(function (o) {
            var actions = o.actions || ['Look'];
            // 主動作 = 第一個非 Look 的動作，沒有就用 Look
            var primary = actions.find(function (a) { return a !== 'Look'; }) || 'Look';
            var icon = ACTION_ICONS[primary] || '❓';
            html += '<button class="action-btn action-' + primary.toLowerCase() + '" '
                + 'data-object-id="' + escapeHtml(o.id) + '" '
                + 'data-object-name="' + escapeHtml(o.name) + '" '
                + 'data-action="' + escapeHtml(primary) + '" '
                + 'title="' + escapeHtml(primary + '：' + o.name) + '">'
                + icon + ' ' + escapeHtml(o.name)
                + '</button>';
        });
        html += '</div>';
    }

    el.innerHTML = html;
}
```

### 3. 動作 icon 映射

```javascript
var ACTION_ICONS = {
    'Look': '👁',
    'Take': '✋',
    'Gather': '⛏',
    'Open': '📦',
    'Read': '📜',
    'Use': '⚡',
    'Talk': '💬',
    'Strip': '🔮',     // 剝名
    'Butcher': '🔪'    // 分解
};
```

### 4. 事件綁定

互動區的按鈕點擊事件（在 `mud-text.js` 底部，跟現有的物件點擊同區域）：

```javascript
// 出口按鈕
document.addEventListener('click', function (ev) {
    var btn = ev.target.closest('.action-move');
    if (!btn) return;
    var dir = btn.getAttribute('data-direction');
    if (dir) window.gameSend({ type: 'move', direction: dir });
});

// 物件按鈕
document.addEventListener('click', function (ev) {
    var btn = ev.target.closest('.action-btn[data-object-id]');
    if (!btn) return;
    var objectId = btn.getAttribute('data-object-id');
    var action = btn.getAttribute('data-action');
    if (objectId && action) {
        window.gameSend({ type: 'do_action', entity_id: objectId, action: action });
    }
});
```

### 5. formatDesc 簡化

`formatDesc()` **保留**，但不再做物件名的正則替換。改為：

```javascript
function formatDesc(description, objects) {
    var safe = escapeHtml(description);
    // 只做排版處理：換行 → <br>，【】→ 綠色強調
    safe = safe.replace(/\n/g, '<br>');
    safe = safe.replace(/【([^】]*)】/g, '<span class="desc-highlight">【$1】</span>');
    // 不再替換 〔〕 和物件名 → 互動由獨立區域處理
    return safe;
}
```

**向後相容**：如果舊房間的 description 裡有 `〔物件名〕`，這些文字會原樣顯示（不再變成按鈕）。因為互動區已經獨立渲染了這些物件，所以不會丟失功能。

### 6. CSS（style.css）

```css
/* ─── 互動區 ─── */
.room-actions {
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
}

.action-group {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
}

.action-label {
    color: #a0a0b0;
    font-size: 0.85rem;
    margin-right: 4px;
    white-space: nowrap;
}

.action-btn {
    background: #1a1a2e;
    color: #e8e8e8;
    border: 1px solid #2a2a4a;
    border-radius: 6px;
    padding: 5px 12px;
    font-size: 0.9rem;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
}

.action-btn:hover {
    background: #0f3460;
    border-color: #e94560;
}

/* 出口按鈕醒目一點 */
.action-move {
    border-color: #0f3460;
    color: #7ee787;
}

/* Take = 綠色系 */
.action-take { color: #69f0ae; }

/* Gather = 黃色系 */
.action-gather { color: #ffd740; }

/* Strip/Butcher = 紫色系 */
.action-strip, .action-butcher { color: #ce93d8; }
```

---

## 渲染結果示意

### 改造前（現在）

```
電梯間無數電梯，其中一臺開了門。
＿＿＿＿＿＿＿＿＿＿＿＿＿＿＿
〔① 〕〔②〕〔③〕〔④〕...
```

所有互動（移動、觀看）嵌在描述文字裡。

### 改造後

```
電梯間無數電梯，其中一臺開了門。       ← 純描述（氛圍文字）

🚪 出口  [①] [②] [③] [④] ...       ← 自動渲染，從 exits 資料
👁 可互動  [📜 告示牌] [✋ 散落鎂幣]   ← 自動渲染，從 objects 資料

你看見：老李、小王                      ← 不變
```

### 屍體態變示例

殺死野獸後，後端更新 objects：

```
一頭倒下的巨狼橫陳在地。               ← 描述

🚪 出口  [南] [北]
👁 可互動  [🔮 巨狼屍體] [🔪 巨狼屍體]  ← Strip（剝名）和 Butcher（分解）

你看見：老李
```

分解後 objects 更新，前端自動重新渲染：

```
一頭倒下的巨狼橫陳在地。

🚪 出口  [南] [北]
👁 可互動  [🔮 巨狼前腿] [🔮 巨狼後腿] [🔮 巨狼軀幹]  ← 肢體各帶 Strip

你看見：老李
```

**前端不需要知道態變邏輯**——後端換 objects 清單，前端照渲染。

---

## 呼叫 updateRoomView 的位置

`main.js` 第 139 行：

```javascript
function draw() {
    if (window.mudUpdateRoomView) {
        window.mudUpdateRoomView(state.room_name, state.description, state.exits, state.entities, state.me, state.objects);
    }
}
```

**不需要改 main.js**。`mudUpdateRoomView` 的簽名不變，只是 `mud-text.js` 裡的實作改了。

---

## 注意事項

1. **不動後端**。WebSocket 的 `view` 消息格式不變
2. **不刪舊邏輯**。`formatDesc` 裡的 〔〕 替換可以註解掉但保留，方便回退
3. **mobile 適配**。互動區用 flex-wrap，按鈕在小螢幕上會自動換行
4. **鍵盤支援**。按鈕天然支援 Tab 導航和 Enter 觸發，不需要額外處理
5. **描述可以為空**。Parser A 產出的新房間 description 是空的，互動區仍然正常渲染
6. **emoji 在 action icon 裡使用是因為碼農容易對齊**，之後可以換成 SVG icon

---

## 驗收標準

- [ ] 房間載入後，互動區自動顯示出口和物件
- [ ] 點擊出口按鈕能移動
- [ ] 點擊物件按鈕能觸發對應動作（Look/Take/Gather/Strip/Butcher）
- [ ] 描述文字裡不再有可點擊的嵌入物件
- [ ] 空描述的房間仍正常顯示互動區
- [ ] 手機瀏覽器上按鈕不會溢出螢幕

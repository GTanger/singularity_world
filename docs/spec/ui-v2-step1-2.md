# UI v2 規格：Step 1-2（靜態骨架 + 地圖互動）

> 給碼農的實作規格。不要自行發揮，照做。
> 有疑問問，不要猜。

## 總覽

把現有 canvas hex 地圖前端，改成**純文字方格地圖 + 抽屜式 UI**。
手機優先（375px 寬為基準），桌面自適應。

**不動的東西：**
- `web/main.js` 的 WebSocket 連線邏輯（Step 3 才接）
- `web/narrative-markdown.js`
- `web/sw.js`
- 後端全部不動

**新建的東西：**
- `web/game.html` — 新遊戲頁面（不改 `index.html`，保留舊版 fallback）
- `web/game.css` — 新樣式
- `web/grid-map.js` — 地圖渲染 + 互動
- `web/game-ui.js` — 抽屜、彈窗、懸浮條邏輯

**技術限制：**
- 純 HTML/CSS/JS，不用框架
- 不用 canvas，全部 DOM 渲染
- CSS Grid 排版地圖格線
- 手機 viewport 滿版，禁止頁面滾動（`overflow: hidden`）

---

## 畫面結構

```
┌─────────────────────────────┐
│  奇點曆  │  日 晷  │  時間   │ ← 頂部懸浮條（fixed, z:100）
├─────────────────────────────┤
│                             │
│                             │
│      文字方格地圖            │ ← 佔滿中間，overflow:hidden
│      （角色永遠置中）         │    拖曳可平移
│                             │
│                             │
│                         [⊕] │ ← 定位鈕（fixed, 右下角, z:100）
├─────────────────────────────┤
│ 氣血/體力 │ 葉 卅 │ 能量/精神│ ← 底部懸浮條（fixed, z:100）
└─────────────────────────────┘
```

Look 模式（左右抽屜同時滑出）：

```
┌─────────────────────────────┐
│  奇點曆  │  日 晷  │  時間   │
├────────┬────────────────────┤
│        │                    │
│ 物件欄  │   房間描述          │
│        │                    │
│(寬 30%)│──────────────────── │
│        │                    │
│        │   log（純輸出）      │
│        │                    │
│        │(寬 70%)            │
├────────┴────────────────────┤
│ 氣血/體力 │ 葉 卅 │ 能量/精神│
└─────────────────────────────┘
```

---

## Step 1：靜態骨架

### 1.1 game.html

```html
<!DOCTYPE html>
<html lang="zh-Hant">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, user-scalable=no">
  <title>奇點世界</title>
  <link rel="stylesheet" href="game.css">
</head>
<body>
  <!-- 頂部懸浮條 -->
  <header id="top-bar">
    <div class="bar-cell" id="calendar">奇點曆</div>
    <div class="bar-cell" id="sundial">日 晷</div>
    <div class="bar-cell" id="clock">時間</div>
  </header>

  <!-- 地圖容器 -->
  <main id="map-viewport">
    <div id="map-grid">
      <!-- JS 動態生成格子 -->
    </div>
  </main>

  <!-- 定位鈕 -->
  <button id="locate-btn" title="回到角色位置">⊕</button>

  <!-- 左抽屜：物件欄 -->
  <aside id="drawer-left" class="drawer drawer-hidden">
    <div id="object-list">
      <!-- JS 動態填充 -->
    </div>
  </aside>

  <!-- 右抽屜：房間描述 + log -->
  <aside id="drawer-right" class="drawer drawer-hidden">
    <div id="room-desc"></div>
    <div id="log"></div>
  </aside>

  <!-- 底部懸浮條 -->
  <footer id="bottom-bar">
    <div class="bar-cell stat-group">
      <span class="stat hp">氣血</span>
      <span class="stat stamina">體力</span>
    </div>
    <div class="bar-cell" id="player-name">葉 卅</div>
    <div class="bar-cell stat-group">
      <span class="stat energy">能量</span>
      <span class="stat spirit">精神</span>
    </div>
  </footer>

  <!-- 角色彈窗（背包/裝備） -->
  <div id="player-modal" class="modal hidden">
    <div class="modal-content">
      <div id="modal-tabs">
        <button class="tab active" data-tab="inventory">背包</button>
        <button class="tab" data-tab="equipment">裝備</button>
        <button class="tab" data-tab="status">狀態</button>
      </div>
      <div id="modal-body"></div>
      <button id="modal-close">✕</button>
    </div>
  </div>

  <!-- 物件動作選單 -->
  <div id="action-menu" class="hidden">
    <ul id="action-list"></ul>
  </div>

  <script src="grid-map.js"></script>
  <script src="game-ui.js"></script>
</body>
</html>
```

### 1.2 game.css 規格

**色彩（沿用現有暗色系）：**
```
--bg:           #1e2328
--bg-cell:      #2a2f35
--bg-cell-current: #3a4550
--bg-unexplored:#0a0a0a
--text:         #e6edf3
--text-dim:     #8b949e
--border:       #444c56
--accent:       #58a6ff
--hp:           #e06060
--stamina:      #60a0e0
--energy:       #a060e0
--spirit:       #60e0a0
--passage:      #8b949e
```

**佈局關鍵規則：**

- `body`：`margin:0; overflow:hidden; height:100dvh;`（用 `dvh` 處理手機鍵盤彈出）
- `#top-bar`：`position:fixed; top:0; height:40px; z-index:100;` 三等分 flex
- `#bottom-bar`：`position:fixed; bottom:0; height:48px; z-index:100;` 三等分 flex
- `#map-viewport`：`position:fixed; top:40px; bottom:48px; left:0; right:0; overflow:hidden;`
- `#map-grid`：`display:grid; position:absolute;`（JS 控制 transform 做平移）
- `#locate-btn`：`position:fixed; bottom:60px; right:12px; z-index:100;` 圓形 40×40
- `.drawer`：`position:fixed; top:40px; bottom:48px; z-index:90; transition: transform 0.3s ease;`
- `#drawer-left`：`left:0; width:30%; transform:translateX(-100%);`
- `#drawer-right`：`right:0; width:70%; transform:translateX(100%);`
- `.drawer.open`：`transform:translateX(0);`
- `#player-modal`：`position:fixed; inset:0; z-index:200; background:rgba(0,0,0,0.7);`

**地圖格子：**

- 每格固定寬高（暫定 `72px × 56px`，碼農可微調使手機 5 欄合理）
- 格子之間有 passage 區域：
  - 東西向 passage（`=`）：格子之間水平間距 `16px`，居中顯示 `=`
  - 南北向 passage（`‖`）：格子之間垂直間距 `16px`，居中顯示 `‖`
  - 對角不存在 passage（只有東西南北）
- 格子文字居中，字體 `14px`
- 當前格：邊框高亮 `--accent`
- 未探索格：背景 `--bg-unexplored`，不顯示文字
- 已探索格：背景 `--bg-cell`，顯示地形名

**物件欄（左抽屜）：**

- 列表式，每個物件一行
- 物件格式：`[圖標] 名稱`（圖標暫用 emoji 或文字符號）
- 分類標題（如「地上物品」「在場人物」）用 `--text-dim` 顏色
- 每個物件可點擊

**右抽屜：**

- 上半 `#room-desc`：flex:1，可捲動，padding 16px
- 下半 `#log`：flex:1，可捲動，padding 16px，字體稍小
- 中間有 1px 分隔線

**底部懸浮條細節：**

- 氣血/體力、能量/精神 各自上下排列（氣血在上體力在下）
- 角色名置中，可點擊（觸發角色彈窗）
- 屬性條用色塊表示：氣血紅、體力藍、能量紫、精神綠
- 數值格式：`當前/最大`，字體 `12px`

**頂部懸浮條細節：**

- 三等分
- 奇點曆格式：`奇點 X年 Y月 Z日`
- 日晷：暫時純文字顯示時段（晨/午/昏/夜）
- 時間：`HH:MM` 格式

---

## Step 2：地圖互動

### 2.1 grid-map.js

**假資料結構（Step 1-2 用，Step 3 換真資料）：**

```javascript
// 地圖格子
const MOCK_CELLS = {
  "0,0":  { terrain: "小屋", explored: true },
  "1,0":  { terrain: "樹林", explored: true },
  "-1,0": { terrain: "樹林", explored: true },
  "0,1":  { terrain: "山脈", explored: true },
  "0,-1": { terrain: "河邊", explored: true },
  "2,0":  { terrain: "樹林", explored: true },
  "-2,0": { terrain: "脈", explored: false },
  "1,1":  { terrain: "樹林", explored: false },
  "-1,1": { terrain: "山脈", explored: true },
  "1,-1": { terrain: "河", explored: true },
  "-1,-1":{ terrain: "樹林", explored: true },
  "0,2":  { terrain: "山脈", explored: false },
  "0,-2": { terrain: "河", explored: true },
};

// 通道（連通性）：key = "x1,y1|x2,y2"（座標小的在前）
const MOCK_PASSAGES = [
  "0,0|1,0",    // 小屋 ↔ 東邊樹林
  "-1,0|0,0",   // 西邊樹林 ↔ 小屋
  "0,0|0,1",    // 小屋 ↔ 北邊山脈
  "0,-1|0,0",   // 南邊河邊 ↔ 小屋
  "1,0|2,0",    // 樹林 ↔ 東邊樹林
  "-2,0|-1,0",  // 脈 ↔ 西邊樹林
  "-1,0|-1,1",  // 西樹林 ↔ 西北山脈
  "0,-2|0,-1",  // 南河 ↔ 河邊
  "1,0|1,1",    // 東樹林 ↔ 東北樹林
  "0,1|0,2",    // 北山脈 ↔ 更北山脈
  "-1,-1|0,-1", // 西南樹林 ↔ 河邊
  "0,-1|1,-1",  // 河邊 ↔ 東南河
];

// 物件列表（look 時顯示）
const MOCK_OBJECTS = {
  "0,0": [
    { type: "npc",    name: "老張",   actions: ["對話", "交易", "觀察"] },
    { type: "item",   name: "野果 ×3", actions: ["撿起", "觀察"] },
    { type: "facility", name: "工作檯", actions: ["使用", "觀察"] },
  ],
  "1,0": [
    { type: "resource", name: "木材", actions: ["採集", "觀察"] },
    { type: "animal",   name: "野兔",  actions: ["觀察", "追趕"] },
  ],
};

// 房間描述
const MOCK_ROOM_DESC = {
  "0,0": "一間簡陋的木屋，屋頂鋪著乾草。門口堆著幾塊劈好的木柴，空氣中有淡淡的煙燻味。老張正蹲在工作檯前敲敲打打。",
  "1,0": "茂密的樹林，陽光從枝葉間灑落。地上有不少落葉，偶爾能聽到鳥鳴。一隻野兔從灌木叢探出頭來。",
};
```

**地圖渲染邏輯：**

```
function renderMap():
  1. 取得 map-viewport 的寬高
  2. 計算可見範圍（以角色位置為中心，向四周延伸足夠格數填滿畫面）
  3. 清空 #map-grid
  4. 用 CSS Grid 建構格線：
     - grid-template-columns: repeat(cols, [cell] 72px [passage] 16px) [cell] 72px
     - grid-template-rows:    repeat(rows, [cell] 56px [passage] 16px) [cell] 56px
  5. 遍歷可見範圍內的座標：
     a. 建立格子 div（.cell），放入對應 grid 位置
     b. 已探索：顯示地形名，背景 --bg-cell
     c. 未探索：黑格，背景 --bg-unexplored
     d. 當前格：加 .current class（高亮邊框）
  6. 遍歷通道列表，在相鄰格之間的 passage 位置放符號：
     a. 東西通道：在水平間距中放 "="
     b. 南北通道：在垂直間距中放 "‖"
     c. 符號顏色 --passage
```

**座標系：** x 東增，y 北增（與 WORKBOARD 一致）。
畫面上 x 軸向右，y 軸向上（螢幕 row 0 = 最北）。

### 2.2 互動邏輯

**點擊移動：**

```
格子點擊事件：
  1. 取得點擊格子的 (x, y)
  2. if 格子 == 當前格 且已探索 → triggerLook()
  3. if 格子 == 當前格 且未探索 → 忽略（長按才觸發探索）
  4. if 格子是當前格的東西南北相鄰格 且有通道 → move(x, y)
  5. 其他 → 忽略
```

**移動動畫：**

```
function move(targetX, targetY):
  1. 設 state.moving = true（防抖鎖）
  2. 計算 #map-grid 的 transform 位移差
  3. CSS transition 平移地圖（duration: 200ms）
  4. transition 結束後：
     a. 更新 state.playerX, state.playerY
     b. state.moving = false
     c. 重新渲染地圖（recenter）
  
  * state.moving == true 時，所有點擊事件忽略（防止趕路時觸發 look）
```

**長按探索：**

```
格子 pointerdown 事件：
  1. if 格子 != 當前格 或 已探索 → return
  2. 開始計時器（3 秒）
  3. 顯示進度條（格子邊框漸變填滿，或格子內百分比文字）
  4. pointerup / pointerleave → 取消計時器，重置進度條

計時器完成（3 秒到）：
  1. 設 cell.explored = true
  2. 重新渲染該格子（顯示地形名）
  3. 自動觸發 triggerLook()
```

**Look（開抽屜）：**

```
function triggerLook():
  1. 填充左抽屜：
     a. 清空 #object-list
     b. 從 MOCK_OBJECTS 取當前格物件
     c. 按 type 分組（在場人物 / 地上物品 / 資源 / 設施 / 野獸）
     d. 每組一個標題 + 物件列表
  2. 填充右抽屜：
     a. #room-desc 填入 MOCK_ROOM_DESC 對應文字
     b. #log 保留既有內容（log 是累積的）
  3. 加 class .open 到兩個 drawer → CSS transition 滑出
```

**關閉抽屜：**

```
手勢偵測：
  - 左抽屜上 swipe left → 關閉
  - 右抽屜上 swipe right → 關閉
  - 任一側關閉 → 兩側同時移除 .open class

實作方式：
  - pointerdown 記起始 X
  - pointermove 計算 deltaX
  - pointerup 時 |deltaX| > 50px 判定為 swipe
  - 左抽屜：deltaX < -50 = swipe left = 關閉
  - 右抽屜：deltaX > 50 = swipe right = 關閉
```

**物件動作選單：**

```
物件點擊事件：
  1. 取得物件的 actions 陣列
  2. 填充 #action-list
  3. 定位 #action-menu 到點擊位置附近
  4. 顯示 #action-menu
  5. 點擊動作項 → log 追加一條訊息（如「你撿起了 野果 ×3」）→ 關閉選單
  6. 點擊選單外 → 關閉選單
```

**角色彈窗：**

```
#player-name 點擊：
  1. 顯示 #player-modal
  2. 預設顯示「背包」tab
  3. tab 切換：背包 / 裝備 / 狀態
  4. 內容用假資料填充
  5. 點 ✕ 或點背景遮罩關閉
```

**地圖拖曳：**

```
#map-viewport 拖曳：
  1. pointerdown 記起始座標 + #map-grid 當前 transform
  2. pointermove 即時更新 #map-grid transform（跟手）
  3. pointerup 結束拖曳
  
  * 拖曳與格子點擊區分：
    - pointerdown + pointerup 間位移 < 10px → 視為點擊
    - 位移 >= 10px → 視為拖曳，不觸發點擊事件
```

**定位鈕：**

```
#locate-btn 點擊：
  1. 計算角色格子應在的中心位置
  2. CSS transition 平滑移回（duration: 300ms）
```

### 2.3 探索進度條視覺

```
未探索格長按時：
  - 格子背景從 #0a0a0a 漸變到 --bg-cell（3 秒線性過渡）
  - 同時格子中央顯示進度文字：「探索中...」
  - 完成瞬間：背景定住 --bg-cell，文字替換為地形名，邊框閃一下 --accent
  - 中途放手：背景回 #0a0a0a，文字消失
```

---

## 驗收標準（Step 1-2 完成時必須滿足）

1. **手機 375px 寬度**下，地圖格子至少顯示 5 欄 × 7 列（含 passage 間距）
2. 頂底懸浮條**不被地圖遮擋**，不被抽屜遮擋
3. 左右抽屜滑出/收回**動畫流暢**（no jank）
4. 連續快速點擊 5 次相鄰格移動，**不觸發 look，不卡頓**
5. 長按探索有**明確視覺進度**，中途放手可取消
6. 物件動作選單**不超出螢幕**（靠邊時自動調整位置）
7. 地圖拖曳**跟手**，回彈定位鈕正常
8. 角色彈窗可正常開關，tab 切換正常
9. 暗色主題，**不出現白色閃屏**
10. 所有互動用**假資料**即可運作，不依賴後端

---

## 不要做的事

- 不要接 WebSocket（Step 3）
- 不要改後端任何 Rust 代碼
- 不要改 `index.html`、`canvas.js`、`main.js`（舊版保留）
- 不要加框架（React/Vue/Svelte）
- 不要自己設計配色（用上面定義的 CSS variables）
- 不要加音效、粒子效果、花俏動畫
- 不要加鍵盤操作（除了未來對話輸入框，但 Step 1-2 不做）
- 不要做 RWD breakpoint，手機優先，桌面自然放大即可

# UI v5 前端重寫規格

> 狀態：待派碼農
> 拍板日：2026-04-29
> Source of truth：`/home/tanger/Projects/obsidian-vault/wiki/UI-規格.md`（v5 全文）

---

<!-- ===== 四查證據包（spec-writer 自填，主 agent 驗收用）===== -->

### 查 1：主入口
- 檔案：`web/index.html`
- 關鍵段（行 77-141）：
  - 行 78：`<header id="top-bar" class="game-grid-header sw-topbar">` — 頂欄**活著**，需撤除
  - 行 102-105：`<div class="mud-card-stage" id="mud-obj-list">` — 物件卡片舞台**活著**（v5 改造，非整塊撤）
  - 行 107-111：`<div id="gmap-viewport" hidden>` + 內層 `gmap-grid` — 舊大地圖已 hidden，保留是為讓 grid-map.js init 不 return 中斷；v5 可整塊砍
  - 行 116-123：`.sw-logmap-row` 含 `.sw-log` + `.sw-minimap` — log 區**活著**；minimap 區需砍或改路口 UI
  - 行 126-139：`<footer>` 底欄 stats bar — **活著**，v5 保留並補玩家名
- 結論：頂欄活著需撤，物件卡片區活著需改造，minimap 行需大改（改成路口形狀 UI），底欄活著保留

### 查 2：git log
- 目標檔：`web/index.html` / `web/grid-map.js` / `web/game-ui.js` / `web/main.js`（共查）
- 近 20 commits（與前端相關的）：
  - `ddd33215` Revert "feat(SW-23)…" — SW-23/24 彩格地圖渲染已 revert，現行 grid-map.js 回到 revert 前狀態
  - `30d05a33` Revert "feat(SW-24)…" — 同上
  - `95699acb` refactor: 拆 grid-map.js — render/room-desc/object-list 抽出
  - `4df3573f` fix: 階段 3 實質翻案——sw-main 改卡片主舞台，大 gmap 退場 — **可疑**：gmap 退場但 DOM 保留隱藏節點
  - `891937b8` refactor: 砍 earth.ygggt.com 觀景窗概念
  - `83381e3b` refactor: 頂欄日晷去互動 + 四層 grid 骨架（SW-17）
- 可疑 commits 確認：
  - `4df3573f`：gmap 退場 = `gmap-viewport` 被 hidden，但 DOM 保留；grid-map.js init() 遇不到 grid 就 return，功能死了但代碼還在。v5 整塊砍乾淨。
  - SW-23/24 revert：彩格地圖雙欄渲染已回滾，前端現在沒有 kind/category 渲染邏輯。
- diff 看過：是

### 查 3：對齊判斷
- 主 agent 意圖：v5 重寫——撤頂欄、改描述欄四層、路口形狀 UI 取代 minimap、物件卡片觀察態
- 當前實際運作代碼：頂欄活著（sw-topbar）、描述欄為舊版 mud-room-desc、minimap 區半死（DOM 存在但 minimap 空）、grid-map.js 的 gmap 功能已死但代碼還在
- 對齊：**是** — 意圖改的東西（頂欄、minimap、描述欄）都還在 DOM，沒有「改已退場的東西」問題

### 查 4：現存 spec
- 已搜：`ls docs/spec/` + `grep -rn "v5|路口|描述欄|四層" docs/spec/`
- 現有 spec：`SW-22_room_payload地形與地標補強.md`（後端）、`ui-v2-step1-2.md`（舊版 UI 分步，已過時）
- 衝突：
  - SW-22 規格仍有效：它定義了後端 `GridCellView` 的 `kind`/`category`/`name` 欄位，**本規格的路口 UI 需要 `name` 欄位顯示地名**，SW-22 需先完成或同步進行。在下方「前置條件」標明。
  - `ui-v2-step1-2.md` 是舊分步規格，與 v5 架構衝突，**但不需整併**——v5 是全新重寫，舊 spec 自然作廢，可在完成後標記廢止。

<!-- ===== 證據包結束，以下為規格內容 ===== -->

---

## 背景

現行前端是 v3/v4 混態：頂欄顯示日期時辰（漫畫 HUD 感）、舊 minimap 區殘留骨架、物件欄架構留著但邏輯待接。v5 拍板的核心轉向：把所有環境資訊收進描述欄散文化（動畫感），路口形狀 UI 取代俯瞰地圖，操作收斂到五方一體（四向移動 + 中央探索）。

設計哲學：**動畫，不是漫畫**。操作極簡，窗口乾淨，玩家注意力從 UI 元件回到描述欄。

---

## 前置條件

SW-22（後端 `GridCellView` 補 `kind`/`category`/`name`）需先完成或同步進行。路口 UI 的地名顯示依賴 `name` 欄位；`explored` + `walkable` 布林路口形狀已在當前 payload 中。

---

## 改動範圍

### 刪除

| 檔 | 刪什麼 | 說明 |
|---|---|---|
| `web/index.html` | `<header id="top-bar" ...>` 整塊（行 78-92） | 頂欄整條撤，日晷移進描述欄 L1 |
| `web/index.html` | `<div id="gmap-viewport" hidden>` 整塊（行 109-111） | 舊大地圖 DOM 清乾淨；grid-map.js 要同步調整 init() 使其不依賴這個節點 |
| `web/index.html` | `.sw-minimap` 元素（行 120-122） | 路口 UI 取代位置 |
| `web/game.css` / `web/game-grid.css` | `.sw-topbar` / `.game-grid-header` 相關樣式 | 頂欄 CSS 全砍 |
| `web/game-ui.js` | 左側物件欄（`#mud-action-list` 或類似）若還有殘存動作欄 DOM 操作 | 需確認實際存在的 DOM id，砍對 |

### 新增 / 改造

| 檔 | 改什麼 | 說明 |
|---|---|---|
| `web/index.html` | 三區佈局 DOM（見下方 DOM 規格） | 描述欄 / 路口形狀 UI / 底欄 |
| `web/sw-room-desc.js` | 四層渲染邏輯（L1-L4） | 現在只有舊版散文，需重構為四層 |
| `web/sw-object-list.js` | 物件卡片觀察態三段渲染 + 甲乙編號邏輯 | 現行無觀察態邏輯 |
| `web/grid-map.js` | 砍地圖渲染，改為路口形狀 UI 渲染 | 可能整個重寫或拆出新檔 `sw-junction.js` |
| `web/main.js` | WebSocket 訊息路由調整（見下方 WS 格式） | 進格/探索/互動/自走四類分流 |
| `web/game.css` 或 `web/game-grid.css` | 三區佈局 CSS + 路口形狀 UI CSS | 保留米色奇幻風，砍彩格 CSS |
| `web/sw.js` | CACHE 版本字串 bump | 前端更新才會讓 SW 快取失效 |

---

## 1. 三區佈局 DOM 結構

```html
<!-- 取代現行 #app-grid 內層結構 -->
<div id="app" class="sw-app">

  <!-- 上區：描述欄（四層 + 互動 log 流）-->
  <div id="sw-desc-panel" class="sw-desc-panel">
    <!-- L1 時間段（JS 填充，世界自走時重抽）-->
    <div id="sw-l1-time" class="sw-desc-layer sw-l1"></div>
    <!-- L2 範圍氣候段（JS 填充，氣候事件變化時重抽）-->
    <div id="sw-l2-climate" class="sw-desc-layer sw-l2"></div>
    <!-- L3 當前格初始段（JS 填充，踏入抽一次，離格前不變）-->
    <div id="sw-l3-terrain" class="sw-desc-layer sw-l3"></div>
    <!-- L4 物件卡片（JS 動態渲染，在場即顯）-->
    <div id="sw-l4-objects" class="sw-desc-layer sw-l4"></div>
    <!-- 互動 log 流（探索 + 物件互動結果追加）-->
    <div id="sw-log" class="sw-log-stream"></div>
  </div>

  <!-- 中區：路口形狀 UI（四方 + 中央探索）-->
  <div id="sw-junction" class="sw-junction">
    <div class="junction-north" id="exit-north"></div>
    <div class="junction-row">
      <div class="junction-west" id="exit-west"></div>
      <button class="junction-center" id="btn-explore">探索</button>
      <div class="junction-east" id="exit-east"></div>
    </div>
    <div class="junction-south" id="exit-south"></div>
  </div>

  <!-- 下區：屬性條 + 玩家名 -->
  <footer id="sw-statusbar" class="sw-statusbar">
    <div class="stats-left">
      <div class="stat-mini" title="氣血"><div class="stat-track"><div class="stat-fill hp" id="bar-hp"></div></div></div>
      <div class="stat-mini" title="體力"><div class="stat-track"><div class="stat-fill stamina" id="bar-stamina"></div></div></div>
    </div>
    <div id="player-name" class="grid-player-name"></div>
    <div class="stats-right">
      <div class="stat-mini" title="能量"><div class="stat-track"><div class="stat-fill energy" id="bar-inner"></div></div></div>
      <div class="stat-mini" title="精神"><div class="stat-track"><div class="stat-fill spirit" id="bar-spirit"></div></div></div>
    </div>
  </footer>

</div>
```

**現行 id 對應表**（改 DOM 時確認這些 id 有無被 JS 引用，需同步更新）：

| 舊 id / class | v5 對應 | 動作 |
|---|---|---|
| `#top-bar` / `.sw-topbar` | 無 | 整塊砍 |
| `#mud-room-desc` | `#sw-l3-terrain`（L3）+ L1/L2 | 拆成四層 |
| `#mud-obj-list` / `.mud-card-stage` | `#sw-l4-objects` | 改 id，渲染邏輯重寫 |
| `#log` / `.sw-log` | `#sw-log` | 改 id class |
| `#sw-minimap-container` / `.sw-minimap` | `#sw-junction` | 整塊替換 |
| `#gmap-viewport` / `#gmap-grid` | 無 | 整塊砍 |
| `#bar-hp` / `#bar-stamina` / `#bar-inner` / `#bar-spirit` | 同名保留 | 不動 |
| `#player-name` | 同名保留 | 不動 |

---

## 2. 頂欄移除

**`web/index.html`**：刪除 `<header id="top-bar" ...>` 整塊（含日晷 SVG、日期 span、時辰 span）。

**`web/sw-clock.js`**：日晷圖標 + 時辰數字的渲染邏輯**不刪**，改為更新 `#sw-l1-time` 而非頂欄 span。日晷 SVG 搬進 L1 渲染輸出的 HTML 字串裡，不再是獨立 DOM 節點。

**`web/game-grid.css`**：`.game-grid-header` / `.sw-topbar` / `.bar-cell` / `.game-time-svg` 等頂欄專用樣式整塊砍。

---

## 3. 物件欄移除

現行架構：物件欄在 `#mud-obj-list`（`.mud-card-stage`）。v5 這個區塊**保留但改造為 L4 物件卡片**（見第 6 節），不是整塊撤——撤掉的是「左側獨立動作欄」的概念。

如果 `web/game-ui.js` 裡有獨立的左側動作欄（採集/伐木/採礦/修煉等按鈕列表），整塊砍。這些動作全部收進中區路口 UI 的 [探索] 按鈕，後台依地形抽取，前端不再暴露個別動作按鈕。

---

## 4. 描述欄四層渲染（`web/sw-room-desc.js`）

### 責任分工

| 層 | DOM id | 更新時機 | 資料來源 |
|---|---|---|---|
| L1 時間 | `#sw-l1-time` | 每個世界時辰 tick（世界自走） | WS `world_tick` 事件的 `time` 欄位 |
| L2 範圍氣候 | `#sw-l2-climate` | 氣候事件變化時 | WS `world_tick` 事件的 `weather` 欄位 |
| L3 當前格 | `#sw-l3-terrain` | 進格時（`room_enter` 事件）抽一次，離格前不重抽 | WS `room_enter` 事件的 `terrain` / `desc` 欄位 |
| L4 物件卡片 | `#sw-l4-objects` | 進格 + 物件狀態變化 | WS `room_enter` / `object_update` 事件的物件列表 |

### L1 時間渲染格式

```html
<!-- 輸出 HTML 例（JS 塞進 #sw-l1-time）-->
<span class="desc-icon">⊙</span>
<span class="desc-text">晨光初白，霧氣未散</span>
<span class="desc-clock">06:30</span>
```

散文從描述池抽取（描述池是另一個任務，此規格留 placeholder）。時辰數字從後端 `time.hour_str` 取得，直接顯示，**不做本地計算**。

### L2 氣候渲染格式

```html
<span class="desc-icon">☁</span>
<span class="desc-text">連日陰雨，山北霧氣未散</span>
```

圖標由後端 `weather.icon_key` 對應前端映射表（`"rain"→☁`、`"snow"→❄`、`"clear"→☀`、`"wind"→🌬`）。

### L3 地形渲染格式

```html
<span class="desc-icon">🌲</span>
<span class="desc-text">松針鋪地，林徑深處草木不知何年生長。</span>
```

散文 placeholder：L3 文字來自後端 `room_enter.desc` 欄位，前端直接渲染。描述池由另一任務補充。

### 互動 log 流（`#sw-log`）

- 使用 `div.sw-log-entry` 追加，不清空
- buffer 上限 500 條，超過刪最舊（`removeChild(firstChild)`）
- 滾動行為：新條目追加後 `scrollTop = scrollHeight`（貼底）
- **離格不清空**：進格時只追加 L1-L3 內容，不 clear log
- **世界自走（NPC 走動/時辰/氣候）不推 log 條目**，只更新對應層狀態

---

## 5. 路口形狀 UI（`web/grid-map.js` 或新檔 `web/sw-junction.js`）

### 四方向邏輯

進格後後端回傳四方鄰格資訊，前端對每個方向獨立判斷：

| 後端資料狀態 | 前端顯示 | 點擊效果 |
|---|---|---|
| `explored: true` + `walkable: true` | 地名（優先地標名，其次地形名） | 發送移動請求 |
| `explored: true` + `walkable: false` | **空白**（不顯示任何內容） | 無反應 |
| `explored: false` | `▒▒▒` | 發送探路請求（單向揭露鄰格） |
| 無此方向（邊界） | **空白** | 無反應 |

**地名顯示優先序**（從後端 `name` 欄位）：
1. 有地標名（如 `<inn-name>`、`<temple-name>`）→ 顯示地標名
2. 無地標 → 顯示地形名（如「疏林」「山腳」「湖濱」）
3. 不顯示距離、海拔、id 任何其他屬性

### 形狀即資訊

CSS 只要對四個方向 `div` 顯示/隱藏就能呈現路口形狀，不需要額外「形狀判斷邏輯」：

- 四方都有可顯示內容 → 十字
- 僅三方 → T 字
- 僅兩方對向 → 直線通道
- 僅一方 → 盡頭死路

**不加牆符號、不加 `X` 標記**：空白就是空白，玩家自己理解這裡沒路。

### 中央探索按鈕

`#btn-explore`：常駐，不依地形狀態消失。點擊 → 發送 `action: "explore"` WS 訊息。

後端負責判斷當格是否可探索、抽取結果並推回。前端只負責：點 → 送請求 → 等 `explore_result` 事件回來 → 追加 log。

---

## 6. 物件卡片觀察態渲染（`web/sw-object-list.js`）

### 資料結構（後端給）

後端在 `room_enter` 與 `object_update` 事件中推送物件列表，每個物件含：

```json
{
  "id": "<uuid>",
  "type": "npc" | "item" | "resource",
  "tag": "<npc-occupation-or-type>",
  "name": "<npc-true-name>",
  "observation_state": "stranger" | "identified" | "named"
}
```

- `observation_state` 是**該玩家對該物件的識別狀態**，由後端維護（持久布林）
- `tag` = 觀察後得到的身份標籤（如「獵戶」「行人」「商人」），陌生時前端不用
- `name` = 知名後顯示的姓名，陌生/觀察狀態不用

### 卡片顯示邏輯

```
observation_state === "stranger" → 顯示標籤「行人」（或通用分類詞）
observation_state === "identified" → 顯示 tag
observation_state === "named" → 顯示「tag + name」（如「獵戶<npc-name>」）
```

### 數字編號規則（**純前端渲染，不持久化**）

每次渲染 L4 時，對當格在場物件列表執行：

1. 對每個物件取其「顯示標籤字串」（依上面邏輯算出）
2. 按顯示標籤分桶
3. 同桶 ≥ 2 個物件 → 按列表順序編 `1, 2, 3...`（如 `【陌生人1】`、`【陌生人2】`）
4. 同桶只有 1 個 → 不加編號

範例：

| 當格物件 | observation_state | 顯示 |
|---|---|---|
| npc-A（tag=行人/陌生人）stranger | stranger | 【陌生人1】 |
| npc-B（tag=行人/陌生人）stranger | stranger | 【陌生人2】 |
| npc-C（tag=獵戶）identified | identified | 【獵戶】 |
| item-柴刀 | — | 【柴刀】 |
### 點擊卡片

點擊任意物件卡片 → 彈出互動選單（現行 modal 機制保留）。選單選項由後端依物件類型與觀察態決定，前端只負責渲染選單項並發送對應動作。

---

## 7. WebSocket 訊息格式（前端視角）

### 前端發送（client → server）

```json
{ "action": "move", "direction": "north" | "south" | "east" | "west" }
{ "action": "scout", "direction": "north" | "south" | "east" | "west" }
{ "action": "explore" }
{ "action": "interact", "target_id": "<uuid>", "verb": "observe" | "talk" | "pick_up" | "attack" }
```

### 後端推送（server → client）

**進格事件** `room_enter` → 觸發 L1-L3 重抽 + L4 物件卡片重渲染：

```json
{
  "event": "room_enter",
  "time": { "hour_str": "06:30", "period": "dawn", "sundial_angle": 90 },
  "weather": { "icon_key": "rain", "desc": "..." },
  "terrain": { "icon_key": "forest", "name": "疏林" },
  "desc": "松針鋪地...",
  "exits": {
    "north": { "explored": true, "walkable": true, "name": "山腳" },
    "south": { "explored": false, "walkable": null, "name": null },
    "east":  { "explored": true, "walkable": false, "name": "峭壁" },
    "west":  { "explored": true, "walkable": true, "name": "<landmark-name>" }
  },
  "objects": [ /* 物件列表，含 observation_state */ ]
}
```

**探索結果事件** `explore_result` → 追加 log 散文段，不動 L1-L3：

```json
{
  "event": "explore_result",
  "log": "拾得野莓三顆，露水未乾。",
  "objects_delta": [ /* 新增/消失的物件（可選）*/ ]
}
```

**物件互動結果事件** `interact_result` → 追加 log 散文段：

```json
{
  "event": "interact_result",
  "log": "獵戶打量你一眼，沒有說話。",
  "observation_update": { "target_id": "<uuid>", "new_state": "identified", "tag": "獵戶" }
}
```

**世界自走更新事件** `world_tick` → 更新 L1-L4 狀態，**不推 log**：

```json
{
  "event": "world_tick",
  "time": { "hour_str": "07:00", "period": "morning", "sundial_angle": 105 },
  "weather": { "icon_key": "clear", "desc": "..." },
  "objects_delta": {
    "arrived": [ /* 新進場物件 */ ],
    "departed": [ "<uuid>", ... ]
  }
}
```

前端收到 `world_tick`：
- 更新 L1（日晷 + 時辰散文）
- 更新 L2（若 `weather` 有變）
- 更新 L4（依 `objects_delta` 加/移卡片）
- **不在 `#sw-log` 追加任何條目**

---

## 8. CSS 風格

**保留**：
- 米色奇幻背景（`#1e2328` 深色 / `#c8b8a2` 米色文字）
- `Chiron GoRound TC` / `LXGW WenKai Mono TC` 字體
- 底欄 stat bar 樣式（`bar-hp` / `bar-stamina` / `bar-inner` / `bar-spirit`）

**砍掉**：
- 所有彩格俯瞰元素（`.gmap-*` CSS、格子顏色映射、地形色塊）
- `.game-grid-header` / `.sw-topbar` 相關
- minimap 相關 `.sw-minimap`
- 若有 `.mud-action-list` 類左側動作欄樣式 → 整塊砍

**新增**：
- `.sw-desc-panel`：上區佈局，flex column，overflow-y scroll，`flex: 1`
- `.sw-desc-layer`：每層描述段的 padding/margin，L4 用 flex-wrap 排卡片
- `.sw-junction`：中區，固定高度，grid 佈局（3×3，四方 + 中央）
- `.junction-*`：各方位 div，點擊態文字樣式
- `.sw-log-stream`：log 流區，字體稍小，行高稍鬆
- 物件卡片 `【】` 樣式：inline-block，邊框或底線，hover 態

---

## 驗收標準

- [ ] **登入後看不到頂欄**：頁面中不存在日期條 / 日晷 SVG HUD，時辰資訊只出現在描述欄 L1 散文中
- [ ] **進格時描述欄三層自動更新**：踏入新格後 L1（時辰）、L2（氣候）、L3（地形散文）各自更新，且 log 不清空，歷史紀錄可上滑閱讀
- [ ] **路口形狀 UI 反映真實拓撲**：已揭露可走的方向顯示地名（可點移動）、未揭露顯示 `▒▒▒`（可點探路）、無路方向為空白，整體形狀與實際通路吻合
- [ ] **中央探索按鈕可用**：點 [探索] 後 log 追加採集/戰鬥/空手散文，L1-L3 不重繪
- [ ] **物件卡片觀察態正確**：首見 NPC 顯示「行人甲」類通用標籤，觀察後更新為職業標籤，知名後顯示「標籤+名字」；同標籤兩人以上才有甲乙編號
- [ ] **世界自走不污染 log**：等待一個時辰後 log 無新條目，但 L1 時辰散文已更新
- [ ] **三件套 bump 確認**：`web/index.html` CSS/JS `?v=` 版本字串已 bump、`web/sw.js` CACHE 清單版本號已 bump，瀏覽器強制重整後拿到新版前端

---

## 不做

- 後端 Rust 代碼（另一份規格）
- 描述池文案內容（地形/時辰/氣候散文模板，另一個任務）
- 詞盤投射在 log 散文中的具體寫法（先留 placeholder，log 文字由後端推送，前端只渲染）
- 探索節流後台邏輯（時辰成本 + 撲空遞增，後端決定，前端只顯示結果）
- 戰鬥預警「繼續/退出」選單（前端 UI 另行補充，本規格不定義）
- 日期世界內取得機制（屬於 NPC 對話 + 告示物件設計，不是前端任務）
- `ui-v2-step1-2.md` 舊規格整併（v5 重寫後舊 spec 自然廢止，由主 agent 決定是否標記廢止）

---

## 占位符對照

- `<inn-name>` — 路口 UI 地名範例，設計者後續填具體命名
- `<npc-name>` — 物件卡片「獵戶 X」的名字欄，後端給
- `<temple-name>` / `<landmark-name>` — 地標名占位

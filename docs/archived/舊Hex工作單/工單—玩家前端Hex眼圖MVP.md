# 工單：玩家前端 Hex 眼圖 MVP

> 優先級：P0  
> 目標：將現有文字版 MUD 主畫面替換為 Hex 格網 canvas 眼圖，玩家可在 (0,0) 草原格上看到字圓並移動  
> 驗收環境：手機 PWA（390×844 基準），桌面次之  
> 依據文件：`docs/reference/map_terrain_world.md` §4.1–4.6

---

## 一、範圍

### 做

1. 登入後主畫面改為全螢幕 hex canvas
2. 渲染玩家所在格 + 視距 5 格內所有彩格（pointy-top hex）
3. 玩家字圓錨定畫面中心，可拖曳平移查看
4. 點擊螢幕 → 字圓朝目標方向直線移動 → 撞不可走地形/實體停止
5. 移動速度 = `60 / move_cost` px/s
6. 底部狀態條（氣血/精神/內力/體力）半透明覆蓋在 canvas 上
7. 同格 NPC 字圓顯示

### 不做

- 格內子座標（本 MVP 用整格定位）
- 格內物件渲染（樹、礦等）
- 互動選單 / 插座系統
- 大地圖總覽
- 探索揭露（黑格生成）
- 敘事浮現文字
- 戰鬥 / 對話 / 交易

---

## 二、畫面佈局

```
┌──────────────────────────────┐
│  [時鐘]              [背包]  │  ← 頂列，半透明浮層
│                              │
│                              │
│        hex canvas            │  ← 全螢幕，佔滿視口
│      (玩家在中心)             │
│                              │
│                              │
│  ░░░░░░░░░░░░░░░░░░░░░░░░░░ │  ← 底部狀態條，半透明
└──────────────────────────────┘
```

### 移除的 DOM 元素

- `#room-desc-panel`（房間描述欄）→ 刪除
- `#narrative-panel`（敘事紀錄欄）→ 刪除
- `#room-name-wrap`（房間名稱）→ 刪除
- 頂列五按鈕中的「保留」「地圖」「訊息」「設定」→ 刪除，只保留「背包」

### 保留不動

- `#auth-screen`（登入/創角）
- `#player-modal-overlay`（角色面板）
- `#inventory-modal-overlay`（背包彈窗）

---

## 三、Hex Canvas 渲染

### 3.1 座標系統

- **Axial 座標 (q, r)**，pointy-top（與 editor-leptos 一致）
- 像素換算（pointy-top）：
  ```
  px_x = HEX_R * √3 * (q + r/2)
  px_y = HEX_R * 3/2 * r
  ```
- **HEX_R = 57px**（螢幕渲染用半徑）
- 格寬 flat-to-flat = 57 × √3 ≈ 98px

### 3.2 相機

- 玩家字圓永遠在 canvas 中心
- 相機 offset = 玩家 hex 座標轉像素
- 所有其他格子的像素位置 = 自身像素 - 相機 offset + canvas 中心
- **可拖曳平移**：手指/滑鼠拖曳移動相機（跟現有 canvas.js 邏輯類似），鬆手後不自動回彈

### 3.3 格子繪製

每個彩格：
1. 畫六角形填色（顏色來自 `Terrain::color()`，後端已有，見 `src/hex/cell.rs`）
2. 畫六角形邊框（1px，`#1a1a1a`）
3. 格內中心寫一個地形字（從 `terrain_display.rs` 的候選字取，字色白或黑視背景亮度定）

視距外的格子不畫（canvas 背景色 `#0f1923` 即黑格效果）。

### 3.4 字圓繪製

- 玩家字圓：圓形，直徑 14px，填色 `#7ec8e3`，圓內寫 `display_char`（一個中文字）
- NPC 字圓：同直徑，填色 `#c4b8a8`
- 字圓畫在地形之上
- 同格多個字圓時，水平偏移排列避免重疊

---

## 四、移動邏輯

### 4.1 點擊觸發

1. 玩家點擊 canvas（非拖曳）
2. 計算點擊位置相對於玩家的**方向**
3. 找出該方向最近的 hex 鄰居（6 方向取最接近的）
4. 發送移動指令到後端

### 4.2 後端通訊

**現有後端已支援 hex 移動**，不需要改後端。

前端發送（沿用現有 WebSocket 協議）：
```json
{"Move": {"direction": "東北"}}
```

方向字串對應（`HexDir::label_zh()`）：
- 東北、東、東南、西南、西、西北

後端回傳 `RoomView`，前端用新的 hex 座標重新渲染。

### 4.3 移動動畫

- 收到後端確認後，字圓從舊位置**平滑滑動**到新位置
- 動畫時長 = 格距 / actual_speed
  - Road: 98px / 120px/s ≈ 0.8 秒
  - Grassland: 98px / 60px/s ≈ 1.6 秒
- 動畫期間不接受新的移動輸入
- 動畫結束後，重置相機讓玩家回到中心

### 4.4 不可走判定

前端可做預判（已知地形 walkable=false 就不送請求），後端也會擋。撞牆時不移動、不動畫。

---

## 五、資料來源

### 5.1 格子資料

後端 `get_hex_room_view` 回傳的 `RoomView` 包含：
- 當前格資訊（terrain、name、description、objects）
- 六方向可走鄰居（exits）
- 同格實體（entities）

**問題：現有 API 只回當前格 + 鄰居，不回視距 5 格內所有格。**

需要新增一個後端 API 或擴展現有 WebSocket 訊息：

```
GET /api/hex/view?player_id=xxx
```

回傳：
```json
{
  "center": {"q": 0, "r": 0},
  "cells": [
    {"q": 0, "r": 0, "terrain": "grassland", "name": "草原", "color": "#4a7c59"},
    {"q": 1, "r": 0, "terrain": "road", "name": "道路", "color": "#8b7355"},
    ...
  ],
  "entities": [
    {"id": "xxx", "q": 0, "r": 0, "display_char": "我", "kind": "player"},
    {"id": "npc1", "q": 1, "r": 0, "display_char": "商", "kind": "npc"}
  ]
}
```

cells 包含視距 5 格半徑內所有已存在的彩格。沒有格的座標 = 黑格，不列入。

**這是本工單唯一需要改後端的地方。**

### 5.2 移動後更新

玩家移動一格後，後端推送新的 view 資料（或前端主動 re-fetch）。前端用新資料重繪整個 canvas。

---

## 六、CSS 改動

### 6.1 canvas 全螢幕

```css
#hex-canvas {
  position: fixed;
  top: 0; left: 0;
  width: 100vw;
  height: 100vh;
  z-index: 0;
  background: #0f1923;
  touch-action: none;  /* 防止瀏覽器手勢干擾 */
}
```

### 6.2 頂列浮層

```css
#top-bar {
  position: fixed;
  top: 0; left: 0; right: 0;
  z-index: 10;
  background: rgba(15, 25, 35, 0.7);
  backdrop-filter: blur(4px);
  /* 只保留時鐘 + 背包按鈕 */
}
```

### 6.3 底部狀態條

```css
#status-panel {
  position: fixed;
  bottom: 0; left: 0; right: 0;
  z-index: 10;
  background: rgba(15, 25, 35, 0.7);
  backdrop-filter: blur(4px);
}
```

---

## 七、檔案改動清單

| 檔案 | 動作 | 說明 |
|------|------|------|
| `web/index.html` | 改 | 刪描述欄/敘事欄/房間名、canvas 改 id、頂列精簡 |
| `web/canvas.js` | **重寫** | 方格 151×151 → hex pointy-top 渲染 |
| `web/main.js` | 改 | WebSocket 訊息處理改接 hex view；移動改發方向指令 |
| `web/style.css` | 改 | canvas 全螢幕、頂列/狀態條浮層化 |
| `web/mud-text.js` | 刪或清空 | MUD 文字渲染不再需要 |
| `src/server/http_api.rs` | 加 | 新增 `GET /api/hex/view` 端點 |
| `src/game/room.rs` | 加 | 新增 `get_hex_area_view()` 回傳視距內所有格 |

---

## 八、驗收標準

在手機 PWA 上：

1. ✅ 登入後看到 hex 格網 canvas，(0,0) 草原格在中心
2. ✅ 玩家字圓顯示在畫面中心，顯示 `display_char`
3. ✅ 周圍已存在的彩格有顏色和地形字
4. ✅ 視距外（>5 格）為黑色背景
5. ✅ 點擊螢幕某方向，字圓平滑移動一格（走到可走的鄰居格）
6. ✅ 點擊不可走方向（山/水/牆），不移動
7. ✅ 道路上移動明顯比草原快（0.8s vs 1.6s）
8. ✅ 拖曳可平移查看周圍格子
9. ✅ 頂列時鐘和背包按鈕可見可用
10. ✅ 底部狀態條可見，半透明不擋地圖
11. ✅ 同格 NPC 字圓可見

---

## 九、技術提示

- `src/hex/cell.rs` 的 `Terrain::color()` 已有每種地形的 hex 色碼，直接用
- `src/hex/coord.rs` 的 `HexCoord` 有完整的鄰居、距離計算
- `src/hex/grid.rs` 的 `HexGrid` 有 `cells_in_range(center, radius)` 可直接取視距內格子
- 前端 hex 像素計算參考 `editor-leptos/src/hex_grid.rs` 的 `hex_to_pixel`
- 現有 canvas.js 的拖曳邏輯（startDrag/moveDrag/endDrag）可複用，只要把方格座標換成 hex 像素
- `move_cost` 值在 `Terrain::move_cost()` 已實作，API 回傳時帶上即可

# NPC 模板系統 — 檢索文檔

> 最後更新：2026-02-12  
> 對齊：討論 001「身份與職業分離—角色無身份綁定」、實作文件 `docs/implementation/NPC生成流程調整—依討論001.md`。

---

## 設計前提（討論 001 共識）

- **角色本體無身份綁定**：NPC 與玩家一致，生成時**只帶萬人萬相 seed**，職業與身份**不在生成當下綁定**。
- **職業／身份由指派而來**：場所主人給予**工作（職業）**，場所與在場行為給予**外人認同的身份**；離開場所即一般平民。
- **對話 vs 動作**：**對話模板**依職業掛載，任時任地可用；**動作模板**（含職業專屬插座）僅在**綁定場所內**（room_ids）可執行。
- **場所「在場」**：採 **room_ids**（作法 B），場所帶一組房間 id 清單，僅列出的房間算在場；不採 zone，以利地圖檢視器以街／巷著色。

---

## 目錄結構

```
data/
├── npc_behaviors.json          ← 定點 NPC 行為文本（經理、服務生），依職業 key 使用；啟動時載入
├── rooms.json                  ← 房間定義（tags + zone）
└── templates/                  ← 職業與模板系統（本資料夾）
    ├── README.md               ← 本文件
    ├── occupations.json        ← 職業型別表：id → 對話/行為檔、在場時開放的 action_sockets（無 spawn_weight）
    ├── archetypes.json         ← （選用）量產用職業原型 + movement，供未來「由 archetype 生成移動型 NPC」參考
    ├── dialogues/              ← 對話模板：依職業掛載，任時任地可用
    │   ├── public_dialogue.json  ← 公共對話（無職業時 fallback，建議存在）
    │   ├── merchant.json
    │   ├── blacksmith.json
    │   └── ...（經理/服務生可對應 manager.json、waiter.json 或沿用 npc_behaviors 敘事）
    └── behaviors/              ← 行為／動作模板：依職業；僅在綁定場所內生效
        ├── merchant.json
        ├── blacksmith.json
        └── ...
```

---

## 1. occupations.json — 職業型別表

定義「職業」對應的對話檔、行為檔，以及在**綁定場所內**才開放的**動作插座**。不帶 spawn_weight；誰擔任該職業由**指派（assignments）**寫入。

| 欄位 | 說明 |
|------|------|
| `name` | 職業中文名 |
| `dialogue_file` | 對話模板路徑（任時任地可用） |
| `behavior_file` | 行為／動作模板路徑（在場才生效） |
| `action_sockets` | 在場時才開放的動詞清單（如 `["Trade"]`）；空則僅預設 Talk / Attack / Look |

### 現有職業（已上線）

| occupation_id | 名稱 | action_sockets | 備註 |
|---------------|------|----------------|------|
| 經理 | 經理 | `[]` | 行為敘事來自 npc_behaviors.json roles.經理 |
| 服務生 | 服務生 | `[]` | 同上，roles.服務生 |

後端：`db/occupation.go` 的 `LoadOccupations`、`GetSocketsForNPC(db, entityID, roomID)` 依指派與 **room_ids 在場** 決定是否加上 `action_sockets`。

---

## 2. 場所（venues）與指派（assignments）

- **venues**：id、name、**room_ids**（JSON 陣列）。「在場」＝當前房間在該場所的 room_ids 內。
- **assignments**：entity_id、occupation_id、venue_id、assigned_by（可空）。表示「誰、在何場所、擔任何職業」。

職稱（外人認知）由指派推導；列可執行動作與執行時，僅在 **EntityInVenueAtRoom** 時才開放該職業的 action_sockets。見 `db/assignment.go`、`db/occupation.go`。

---

## 3. rooms.json — 房間標籤與 zone

每個房間有 `tags` 和 `zone`，供尋路與地圖著色使用。**場所範圍**不綁 zone，改由 venues.room_ids 精準列出。

### 標籤一覽（已使用）

| Tag | 說明 | 適用房間 |
|-----|------|---------|
| `spawn` | 重生點 | lobby |
| `inn` | 客棧 | life_hall |
| `tavern` | 酒館/食堂 | life_dining, life_wine_cellar |
| `social` | 社交場所 | life_hall, life_dining |
| `trade` | 可交易 | life_hall |
| `food` | 有食物 | life_dining |
| `outdoor` | 戶外 | life_garden, life_backyard |
| `garden` | 花園/庭院 | life_garden |
| `gate` | 入口/城門 | life_garden |
| `kitchen` | 廚房 | life_kitchen |
| `staff` | 員工區 | life_kitchen, life_backyard, life_storage |
| `storage` | 倉庫 | life_storage, life_wine_cellar |
| `rest` | 休息處 | life_storage |
| `corridor` | 走廊 | life_corridor_2f/3f |
| `lodging` | 住宿區 | 迴廊 + 客房 |
| `room` | 客房 | 所有客房 |
| `luxury` | 高級 | 三樓客房 |
| `study` | 書房 | life_xuan_1 |

### 新增房間標籤建議

- **地標型**：`gate`, `market`, `temple`, `forge`, `stable`, `dock`
- **功能型**：`inn`, `tavern`, `trade`, `food`, `lodging`, `storage`
- **環境型**：`outdoor`, `indoor`, `underground`, `road`, `wilderness`
- **社交型**：`social`, `private`, `staff`

---

## 4. NPC 移動系統 — 三種模式

（與「身份／職業分離」獨立：移動可由 archetype 或另表定義，供地圖型 NPC 使用。）

### 4.1 regional（區域型）

在指定 `wander_rooms` 內隨機跳轉；適用定點 NPC（經理、服務生、鐵匠、農夫）。`npc_behaviors.json` 的 wander_rooms 即此模式。

### 4.2 route（路線型）

沿 waypoints 移動，BFS 填路徑；`route_mode`: bounce / loop / one_way。適用行腳商人、巡邏守衛。

### 4.3 pathfind（自動尋路型）

依 `destination_tags`、`wander_range` 隨機選目標，BFS 尋路。適用旅人、酒客、乞丐、學者等。

詳見 `db/npc_movement.go`、`db/pathfind.go`；archetypes.json 內各原型的 movement 為參考格式。

---

## 5. dialogues/*.json — 對話模板

依**職業**掛載，**任時任地**可用（與是否在場所無關）。

### 結構

```json
{
  "greet":           { "lines": [...] },
  "idle": {
    "morning": [...], "noon": [...], "evening": [...], "night": [...]
  },
  "talk":            { "lines": [...] },
  "trade_announce": { "buy": [...], "sell": [...] }
}
```

### 佔位符

| 佔位符 | 替換為 |
|--------|--------|
| `{name}` | NPC 真名 |
| `{goods}` | 販賣品類（若從 goods_pool 抽） |

### 公共對話

建議存在 **public_dialogue.json**，結構同上，供無職業或無命中關鍵字時 fallback。對話介面未來可接**關鍵字檢索**（display→keyword→pool）、自由輸入與點選並存。

---

## 6. behaviors/*.json — 行為／動作模板

定義該職業的**日程、巡邏、交易、性格**；**僅在綁定場所內**時，該職業的動作插座與相關邏輯才生效。

結構含 schedule、wander、trade、personality 等；與 npc_behaviors.json 的 roles 可並存（目前經理／服務生敘事來自 npc_behaviors，動作插座由 occupations.action_sockets 與在場檢查決定）。

---

## 7. 與現有系統的關係

| 項目 | 用途 | 狀態 |
|------|------|------|
| `data/npc_behaviors.json` | 經理、服務生閒置/進房/換班/巡邏文本 | 已上線，依職業 key 使用 |
| `data/rooms.json` | 房間 + tags/zone | 已更新 |
| `data/templates/occupations.json` | 職業型別表（對話/行為檔、action_sockets） | 已上線 |
| DB 表 venues / assignments | 場所 room_ids、誰任職何場所 | 已上線 |
| `db/assignment.go` | Venue、Assignment、GetNPCTitleFromAssignments、EntityInVenueAtRoom | 已上線 |
| `db/occupation.go` | LoadOccupations、GetSocketsForNPC、IsDefaultSocket | 已上線 |
| `db/pathfind.go` | BFS 尋路 | 已上線 |
| `db/npc_movement.go` | 三種移動模式 | 已上線 |
| 列可執行動作 / do_action | NPC 用 GetSocketsForNPC；非預設動作檢查在場 | 已上線 |

### 演進路線

1. **已完成** — 定點 NPC 活化（npc_behaviors → db/behavior.go）
2. **已完成** — 房間 tags/zone、尋路與移動管理器
3. **已完成** — 身份與職業分離：venues、assignments、occupations.json；生成只帶 seed；職稱與插座依指派與在場
4. **下一步** — 對話關鍵字檢索、公共對話 fallback、從 templates/dialogues 依職業載入
5. **之後** — 三層模擬、萬人 NPC（可再銜接 archetypes 的 movement 與 spawn 權重）

---

## 8. AI 內容生成指引

### 新增職業步驟（對齊討論 001）

1. 在 **occupations.json** 新增職業條目（name、dialogue_file、behavior_file、action_sockets）
2. 在 **dialogues/** 新增該職業對話 JSON（可含關鍵字→pool 對照）
3. 在 **behaviors/** 新增該職業行為 JSON（或沿用 npc_behaviors 的 role key）
4. 指派由遊戲邏輯／管理寫入 assignments（誰、何職業、何場所），重啟或熱載後生效

### Prompt 範例

```
請依照以下 JSON 格式，為「獵人」職業生成對話模板：
- greet / idle(morning,noon,evening,night) / talk / trade_announce(buy,sell)
規則：繁體中文；【{name}】包 NPC 名；「」包台詞；{goods} 替換商品類型。
（附上 merchant.json 作為格式範例）
```

---

## 9. 快速查閱

### 預設 Agent 插座（無需在場）

Talk、Attack、Look。其餘為職業 action_sockets，僅在綁定場所內由 GetSocketsForNPC 加入並可執行。

### 場所「浮生客棧」room_ids（範例）

由 SeedVenues 寫入 DB，涵蓋 life_garden、life_hall、life_dining、life_kitchen、life_backyard、life_storage、life_wine_cellar、life_corridor_2f/3f、各客房 life_ri_1/2、life_yue_1/2、… 等。擴建地圖時需維護 venues.room_ids 或由程式依規則更新。

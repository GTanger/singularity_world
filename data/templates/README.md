# NPC 模板系統 — 檢索文檔

> 最後更新：2026-03-03

---

## 目錄結構

```
data/
├── npc_behaviors.json          ← 現有定點 NPC 行為文本（經理、服務生），伺服器啟動時載入
├── rooms.json                  ← 房間定義檔（含 tags + zone）
└── templates/                  ← NPC 原型 & 模板系統（本資料夾）
    ├── README.md               ← 本文件
    ├── archetypes.json         ← 職業原型總表：10 種職業 + movement 定義
    ├── dialogues/              ← 對話模板：每個職業一個 JSON
    │   ├── merchant.json       ← 商人
    │   ├── blacksmith.json     ← 鐵匠
    │   ├── scholar.json        ← 學者
    │   ├── traveler.json       ← 旅人
    │   ├── drunkard.json       ← 酒客
    │   ├── beggar.json         ← 乞丐
    │   ├── guard.json          ← 守衛
    │   ├── herbalist.json      ← 藥師
    │   ├── performer.json      ← 賣藝人
    │   └── farmer.json         ← 農夫
    └── behaviors/              ← 行為模板：每個職業一個 JSON
        ├── merchant.json
        ├── blacksmith.json
        ├── scholar.json
        ├── traveler.json
        ├── drunkard.json
        ├── beggar.json
        ├── guard.json
        ├── herbalist.json
        ├── performer.json
        └── farmer.json
```

---

## 1. archetypes.json — 職業原型總表

定義每種 NPC 職業的**基礎屬性**與**移動模式**。

| 欄位 | 說明 |
|------|------|
| `name` | 職業中文名 |
| `dialogue_file` | 指向 `dialogues/` 下的對話模板 |
| `behavior_file` | 指向 `behaviors/` 下的行為模板 |
| `spawn_weight` | 生成權重（越高越常見） |
| `trade_tendency` | 交易傾向 |
| `wealth_range` | `[min, max]` 初始財富 |
| `goods_pool` | 可能攜帶的物品池 |
| `movement` | **移動模式定義**（見 §3） |

### 現有 10 種原型

| ID | 名稱 | 權重 | 移動模式 | 範圍 | 財富 | 特徵 |
|----|------|------|---------|------|------|------|
| `merchant` | 商人 | 15 | **route** (bounce) | 城鎮間 | 80~600 | 精明、沿商路巡迴 |
| `blacksmith` | 鐵匠 | 8 | **regional** | 1~2 格 | 120~900 | 定點、鍛造坊 |
| `scholar` | 學者 | 5 | **pathfind** | 15 格 | 30~300 | 書院/茶館隨機遊走 |
| `traveler` | 旅人 | 12 | **pathfind** | 50 格 | 20~400 | 滿世界走、偏好客棧 |
| `drunkard` | 酒客 | 10 | **pathfind** | 8 格 | 5~150 | 短程晃蕩、偏好酒館 |
| `beggar` | 乞丐 | 6 | **pathfind** | 20 格 | 0~20 | 人多處遊蕩 |
| `guard` | 守衛 | 8 | **route** (bounce) | 巡邏路線 | 50~200 | 沿定點巡邏 |
| `herbalist` | 藥師 | 4 | **pathfind** | 10 格 | 60~500 | 田野/市集採藥 |
| `performer` | 賣藝人 | 3 | **pathfind** | 30 格 | 10~100 | 追人群、哪熱鬧去哪 |
| `farmer` | 農夫 | 10 | **regional** | 2~3 格 | 15~200 | 田地/住處/市集 |

---

## 2. rooms.json — 房間標籤系統

每個房間現在有 `tags` 和 `zone` 欄位，供 NPC 尋路決策使用。

```json
{
  "id": "life_hall",
  "name": "浮生大廳",
  "tags": ["inn", "lobby", "social", "trade"],
  "zone": "浮生客棧",
  "description": "..."
}
```

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

### 新增房間標籤規則

擴張地圖時，建議使用以下標準 tag：

- **地標型**：`gate`, `market`, `temple`, `forge`, `stable`, `dock`
- **功能型**：`inn`, `tavern`, `trade`, `food`, `lodging`, `storage`
- **環境型**：`outdoor`, `indoor`, `underground`, `road`, `wilderness`
- **社交型**：`social`, `private`, `staff`

---

## 3. NPC 移動系統 — 三種模式

### 3.1 regional（區域型）

NPC 在指定的 `wander_rooms` 列表內隨機跳轉。

```json
{
  "type": "regional",
  "wander_rooms": ["life_hall", "life_dining"]
}
```

- 適用：定點 NPC（鐵匠、農夫、店主、經理、服務生）
- 不需要尋路引擎
- 現有 `npc_behaviors.json` 中的 wander_rooms 即為此模式

### 3.2 route（路線型）

NPC 沿預定義的 waypoints 移動，中間路徑由 **BFS 自動計算**。

```json
{
  "type": "route",
  "speed": 1,
  "route_mode": "bounce",
  "route_waypoints": [
    {"room": "town_a_market", "stay_hours": [4, 8], "activity": "trade"},
    {"room": "crossroad_1",   "stay_hours": [0, 1], "activity": "pass_through"},
    {"room": "town_b_market", "stay_hours": [6, 12], "activity": "trade"}
  ]
}
```

- **NPC 只需知道途經點，中間的路全由 BFS 自動填充**
- `route_mode`:
  - `"bounce"` — 走到最後一點折返（A→B→C→B→A→...）
  - `"loop"` — 走完循環回第一點（A→B→C→A→B→...）
  - `"one_way"` — 到終點後定居，轉為 regional
- `speed` — 每次移動 tick 走幾格（騎馬 = 2~3）
- `stay_hours` — `[min, max]` 遊戲小時，到達後停留多久
- 適用：行腳商人、巡邏守衛、信使

### 3.3 pathfind（自動尋路型）

NPC 隨機挑選範圍內符合 tag 的房間，BFS 計算路徑，逐格走過去。

```json
{
  "type": "pathfind",
  "speed": 1,
  "destination_tags": ["inn", "tavern", "gate", "social"],
  "wander_range": 50,
  "stay_hours": [2, 6]
}
```

- NPC 不走固定路線，行為類似真人玩家的漫無目的遊走
- `destination_tags` — 目標房間必須帶有的 tag（任一符合即可）
- `wander_range` — 最大 BFS 搜尋深度（步數）
- `stay_hours` — 到達後隨機停留時間
- 到達停留結束後，再隨機選下一個目標
- 適用：旅人、酒客、乞丐、賣藝人、學者、藥師

### 移動速度

| speed | 說明 | 真實速度（200ms tick × 75 = 15 秒/步） |
|-------|------|---------------------------------------|
| 1 | 步行 | 每 15 秒走 1 格 |
| 2 | 快走 | 每 15 秒走 2 格 |
| 3 | 騎馬 | 每 15 秒走 3 格 |

---

## 4. 尋路引擎 — db/pathfind.go

### 核心 API

| 函數 | 說明 |
|------|------|
| `GetGraph()` | 取得全域 RoomGraph 實例 |
| `BuildGraph(db)` | 從 DB 建立鄰接表（啟動時呼叫） |
| `FindPath(from, to)` | BFS 最短路徑，回傳 `[]string`（不含起點） |
| `FindNearestByTag(origin, tag, maxDist)` | 找最近帶指定 tag 的房間 |
| `FindRoomsWithinDist(origin, tags, maxDist)` | 找範圍內所有符合 tag 的房間 |
| `RoomName(roomID)` | 快取查房間名 |
| `Neighbors(roomID)` | 取相鄰房間 |

### 效能

| 房間數 | BFS 時間 | 說明 |
|--------|---------|------|
| 100 | < 0.1ms | 瞬間 |
| 1,000 | < 1ms | 無感 |
| 10,000 | ~5ms | 可接受 |
| 100,000 | ~50ms | 需快取熱門路線 |

---

## 5. NPC 移動管理器 — db/npc_movement.go

### 核心 API

| 類型/函數 | 說明 |
|----------|------|
| `TravelerManager` | 管理所有地圖型 NPC 的即時移動狀態 |
| `NewTravelerManager()` | 建立管理器實例 |
| `Register(entityID, MovementDef)` | 註冊 NPC 進入移動系統 |
| `Unregister(entityID)` | 移除 NPC |
| `Tick(db, graph, gameHour)` | 推進所有 traveler 一步，回傳 `[]NPCStep` |

### 運作流程

```
1. 伺服器啟動
   └→ BuildGraph(db)：讀 rooms + exits 建鄰接表

2. NPC 註冊
   └→ travelerMgr.Register("老張", MovementDef{Type: "pathfind", ...})

3. Game Loop 每 15 秒
   └→ travelerMgr.Tick(db, graph, gameHour)
       ├→ 各 NPC 如果 path 為空 → BFS 計算新路徑
       ├→ 沿 path 走 speed 步
       ├→ 回傳 []NPCStep（誰從哪到哪）
       └→ main.go 對有玩家的房間推送敘事 + 刷新視野

4. 到達目的地
   └→ 隨機停留 stay_hours 遊戲小時
   └→ 停留結束 → 計算下一個目標 → 重複步驟 3
```

---

## 6. dialogues/*.json — 對話模板

每個職業的**所有語句**，用於 NPC 與玩家互動時隨機抽取。

### 結構

```json
{
  "greet":           { "lines": [...] },
  "idle": {
    "morning": [...], "noon": [...], "evening": [...], "night": [...]
  },
  "talk":            { "lines": [...] },
  "trade_announce": {
    "buy": [...], "sell": [...]
  }
}
```

### 佔位符

| 佔位符 | 替換為 |
|--------|--------|
| `{name}` | NPC 個體真名（如「王富貴」） |
| `{goods}` | 販賣品類（從 `goods_pool` 隨機抽取） |

### 格式約定

- NPC 名稱：`【{name}】`
- 台詞：`「」`
- 動作描述：第三人稱直接敘述，不加引號

---

## 7. behaviors/*.json — 行為模板

定義每種職業的**日程作息、巡邏路線、交易邏輯、性格參數**。

### 結構

```json
{
  "schedule":    { "entries": [...] },
  "wander":      { "preferred_locations": [...], "wander_chance": 0.25, ... },
  "trade":       { "buy_markup": 0.7, "sell_markup": 1.4, ... },
  "personality": { "aggression": 0.1, "friendliness": 0.7, ... }
}
```

### 日程 (schedule)

| 欄位 | 說明 |
|------|------|
| `hour_start` / `hour_end` | 遊戲時鐘 0-23 |
| `activity` | `wake_up` / `trade` / `rest` / `sleep` / `patrol` / `socialize` / `beg` |
| `location_pref` | 該時段偏好的地點 tag 列表 |
| `description` | 該時段敘述 |

### 交易 (trade)

| 欄位 | 說明 |
|------|------|
| `buy_markup` | 收購倍率（< 1.0 = 壓價） |
| `sell_markup` | 販賣倍率（> 1.0 = 加價） |
| `preferred_goods` | 偏好商品 |
| `refused_goods` | 拒絕商品 |
| `haggle_tolerance` | 最大議價次數 |
| `haggle_texts` | 議價台詞：`accept` / `reject` / `counter` |

### 性格 (personality)

五維參數 (0.0~1.0)：

| 欄位 | 說明 |
|------|------|
| `aggression` | 攻擊性 |
| `friendliness` | 友善度 |
| `curiosity` | 好奇心 |
| `greed` | 貪婪度 |
| `bravery` | 勇氣 |
| `reaction_to_combat` | `flee` / `stand_ground` / `confront` |
| `reaction_to_theft` | `alert_guard` / `confront` / 敘述文字 |

---

## 8. 與現有系統的關係

| 檔案 | 用途 | 狀態 |
|------|------|------|
| `data/npc_behaviors.json` | **定點 NPC**（經理、服務生）的閒置/反應/換班文本 | 已上線 |
| `data/rooms.json` | 房間定義 + **tags/zone** | 已更新 |
| `db/pathfind.go` | **BFS 尋路引擎** + 房間圖快取 | 已上線 |
| `db/npc_movement.go` | **三種移動模式**管理器 | 已上線 |
| `data/templates/` | **量產 NPC** 的原型 + 對話 + 行為模板 | 已建立 |

### 演進路線

1. **已完成** — 定點 NPC 活化（`npc_behaviors.json` → `db/behavior.go` → game loop）
2. **已完成** — 房間標籤系統（`rooms.json` tags/zone → DB → pathfind.go）
3. **已完成** — 尋路引擎（BFS `db/pathfind.go`）+ 移動管理器（`db/npc_movement.go`）
4. **已完成** — 模板檔案結構（archetypes + dialogues + behaviors，含 movement 定義）
5. **下一步** — 模板讀取引擎（`db/archetype.go`），從模板 + SoulSeed 生成 NPC 個體
6. **之後** — 三層模擬架構（全模擬 / 輕量 / 統計），實現萬人 NPC

---

## 9. AI 內容生成指引

### 給 AI 的 Prompt 範例

```
請依照以下 JSON 格式，為「獵人」職業生成對話模板：
- greet: 8 句進房招呼
- idle: morning/noon/evening/night 各 8 句
- talk: 8 句交談回應
- trade_announce: buy/sell 各 8 句

規則：
1. 繁體中文
2. 【{name}】包 NPC 名
3. 「」包台詞
4. {goods} 替換商品類型
5. 獵人性格：沉默寡言、敏銳、熟悉山林、不善社交

（附上 merchant.json 作為格式範例）
```

### 新增職業步驟

1. 在 `archetypes.json` 新增原型（含 movement 定義）
2. 在 `dialogues/` 新增對話 JSON
3. 在 `behaviors/` 新增行為 JSON
4. 重啟伺服器生效

---

## 10. 快速查閱表

### 各職業的對話模板句數

| 職業 | greet | idle (每時段) | talk | buy | sell | 總句數 |
|------|-------|-------------|------|-----|------|--------|
| 商人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 鐵匠 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 學者 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 旅人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 酒客 | 8 | 7 | 8 | 8 | 8 | ~70 |
| 乞丐 | 8 | 7 | 8 | 8 | 8 | ~70 |
| 守衛 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 藥師 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 賣藝人 | 8 | 8 | 8 | 8 | 8 | ~72 |
| 農夫 | 8 | 8 | 8 | 8 | 8 | ~72 |
| **合計** | | | | | | **~716** |

### 各職業移動模式總覽

| 職業 | 模式 | 速度 | 範圍 | 停留 | 目標偏好 |
|------|------|------|------|------|---------|
| 商人 | route/bounce | 1 | 城鎮間 | 4~8h | 市集 → 十字路 → 市集 |
| 鐵匠 | regional | - | 1~2 格 | - | 鍛造坊、住處 |
| 學者 | pathfind | 1 | 15 格 | 3~8h | study, inn, social |
| 旅人 | pathfind | 1 | **50 格** | 2~6h | inn, tavern, gate |
| 酒客 | pathfind | 1 | 8 格 | 2~10h | tavern, social, food |
| 乞丐 | pathfind | 1 | 20 格 | 1~4h | gate, social, food |
| 守衛 | route/bounce | 1 | 巡邏線 | 0~1h | 城門 → 城牆 → 街道 |
| 藥師 | pathfind | 1 | 10 格 | 2~6h | outdoor, garden, food |
| 賣藝人 | pathfind | 1 | 30 格 | 1~4h | social, tavern, gate |
| 農夫 | regional | - | 2~3 格 | - | 田地、住處、市集 |

# NPC 活化系統

> 最後更新：2026-03-07
> 目標：**玩家＝NPC＝玩家**，假以亂真

---

## 零、設計哲學

奇點世界的 NPC 不是「會說話的路牌」，而是與玩家共用同一套規則的**對等存在**：

- 同樣的 Character 結構（SoulSeed、三軸屬性、裝備、背包、鎂）
- 同樣的戰鬥公式（combat.Resolve）
- 同樣的房間移動機制（SetEntityRoom）
- 同樣的插頭插座互動（Look、Talk、Attack、Trade）

差異僅在於驅動方式：玩家由人類操作，NPC 由模板 + 行為引擎驅動。當兩者在同一個房間裡，玩家不應該能靠行為模式區分誰是 NPC。

---

## 一、系統總覽

### 1.1 已完成模組

| 層級 | 模組 | 檔案 | 說明 |
|------|------|------|------|
| **數據層** | 定點行為文本 | `data/npc_behaviors.json` | 經理/服務生的閒置、反應、換班、巡邏文本 |
| | 職業原型 | `data/templates/archetypes.json` | 10 種職業的基礎屬性 + 移動模式定義 |
| | 對話模板 | `data/templates/dialogues/*.json` | 10 種職業 × ~72 句 = ~716 句 |
| | 行為模板 | `data/templates/behaviors/*.json` | 10 種職業的日程/巡邏/交易/性格參數 |
| | 房間標籤 | `data/rooms.json` | 30 個房間 × tags + zone |
| **引擎層** | 行為讀取 | `db/behavior.go` | 載入快取 npc_behaviors.json，PickIdleEmote、GetMovementDefForTitle（movement.speed） |
| | 尋路引擎 | `db/pathfind.go` | BFS 鄰接圖，FindPath / FindRoomsWithinDist |
| | 移動管理 | `db/npc_movement.go` | 四種移動模式：**schedule** / regional / route / pathfind |
| | 排班系統 | `db/schedule.go` | GetScheduleTarget、ApplySchedules（只回傳敘事用清單，不傳送）；實際移動由 TravelerManager 排班型尋路 |
| | NPC 建立 | `db/npc.go` | InsertNPC（SoulSeed + 屬性 + 穿搭）；SeedNPCs 預設為空，OpenDB 時會刪除舊浮生客棧四名 |
| **推送層** | 敘事廣播 | `server/broadcast.go` | SendNarrateToRoom / RefreshRoomViews |
| | 訊息協定 | `server/protocol.go` | NarrateMsg（ambient 敘事推送） |
| **前端** | 敘事渲染 | `web/main.js` | `case 'narrate'` → appendNarrative + ambient 樣式 |
| | 樣式 | `web/style.css` | `.log-ambient` 灰色小字 |

### 1.2 架構圖

```
┌──────────────── 數據層 ────────────────────┐
│                                              │
│  npc_behaviors.json    templates/             │
│  (定點 NPC 文本)       ├ archetypes.json      │
│                        ├ dialogues/*.json     │
│                        └ behaviors/*.json     │
│                                              │
│  rooms.json (tags + zone)                    │
└──────────────────────────────────────────────┘
         │                    │
         ▼                    ▼
┌──────────────── 引擎層 ────────────────────┐
│                                              │
│  db/behavior.go         db/pathfind.go       │
│  (文本查詢快取)          (BFS 鄰接圖)         │
│                                              │
│  db/schedule.go         db/npc_movement.go   │
│  (排班目標/敘事)          (四種移動模式)        │
│                                              │
│  db/npc.go                                   │
│  (NPC 實體建立)                               │
└──────────────────────────────────────────────┘
         │
         ▼
┌──────────────── 主迴圈 ────────────────────┐
│                                              │
│  main.go  game.Loop (200ms tick)             │
│    ├─ 每遊戲小時：ApplySchedules → 僅發「出發」敘事（不傳送）；排班目標由 Tick 逐格尋路 │
│    ├─ 每 15 秒：TravelerManager.Tick → 排班/路線/尋路型逐格移動，抵達時發敘事          │
│    └─ 每 5-12 秒：閒置動作 + 區域巡邏（在班且於 wander_rooms 內時 10% 瞬移）            │
│                                              │
└──────────────────────────────────────────────┘
         │
         ▼
┌──────────────── 推送層 ────────────────────┐
│                                              │
│  server/broadcast.go                         │
│    ├─ SendNarrateToRoom → NarrateMsg         │
│    └─ RefreshRoomViews → RoomViewMsg         │
│                                              │
│  server/handler.go                           │
│    └─ handleMove → 進房反應（enter_reaction）  │
│                                              │
└──────────────────────────────────────────────┘
         │
         ▼
┌──────────────── 前端 ──────────────────────┐
│                                              │
│  main.js                                     │
│    case 'narrate' → appendNarrative('ambient')│
│    case 'room_view' → 更新實體欄              │
│                                              │
│  style.css                                   │
│    .log-ambient { color: #9e9e8e; 0.9rem }   │
│                                              │
└──────────────────────────────────────────────┘
```

---

## 二、現有 NPC 行為能力

### 2.1 定點 NPC 與排班（現狀）

**預設種子**：浮生客棧四名（陳正明、林小雯、張明德、王阿財）已從 `defaultNPCs` 移除；OpenDB 時會刪除既有 DB 中該四筆實體與排班。目前 **SeedNPCs 不建立任何預設 NPC**，需手動 `InsertNPC` + `InsertSchedule` + `InsertAssignment` 或日後接模板生成器。

凡在 `npc_schedules` 有排班的 NPC，啟動時會以 **MoveSchedule** 註冊到 TravelerManager：依遊戲小時目標為 work_room（在班）或 rest_room（下班），**BFS 尋路逐格移動**，家可遠在十格外。

具備以下行為的 NPC（只要在 npc_behaviors 有對應職稱且有排班）：

| 行為 | 觸發條件 | 頻率 | 資料來源 |
|------|---------|------|---------|
| **排班上下班** | 依 gameHour 目標 work/rest | 每 15 秒推一步（TravelerManager.Tick） | `npc_schedules` + BFS 尋路；出發敘事每遊戲小時、抵達敘事在真正走到時 |
| **閒置動作** | 在班 + 有玩家在場 | 每 5~12 秒 | `npc_behaviors.json` → idle → 時段 |
| **進房反應** | 玩家移動進同房間 | 即時（0.5~1.5s 延遲） | `npc_behaviors.json` → enter_reactions |
| **區域巡邏** | 在班 + 10% 機率 | 每 5~12 秒 | `npc_behaviors.json` → wander_rooms（瞬移一房間） |
| **時段感知** | 自動 | 隨遊戲時鐘 | morning/noon/evening/night 各有不同閒置台詞 |
| **前端同步** | 移動後 | 即時 | RefreshRoomViews + NarrateMsg |

### 2.2 互動能力（已上線）

| 動作 | 觸發 | 效果 |
|------|------|------|
| **Look** | 玩家點擊 NPC → 觀看 | Log 顯示外觀敘事（不開彈窗） |
| **Talk** | 玩家點擊 NPC → 交談 | Log 顯示模板對話 |
| **Attack** | 玩家點擊 NPC → 攻擊 | 戰鬥結算 → Log 顯示結果 |

---

## 三、NPC 移動系統

### 3.1 四種移動模式

| 模式 | 代碼 | 說明 | 尋路 | 適用 |
|------|------|------|------|------|
| **schedule** | `MoveSchedule` | 依 gameHour 目標 work_room（在班）或 rest_room（下班），BFS 逐格走 | BFS 全自動 | 有排班的 NPC（經理、服務生等）；家可十格外 |
| **regional** | `MoveRegional` | 在指定房間列表內隨機跳 | 不需要 | 定點 NPC 區域巡邏（wander_rooms） |
| **route** | `MoveRoute` | 沿 waypoints 巡迴 | BFS waypoint 間路徑 | 行腳商人、巡邏守衛、信使 |
| **pathfind** | `MovePathfind` | 隨機挑符合 tag 的房間，BFS 走過去 | BFS 全自動 | 旅人、酒客、乞丐、賣藝人 |

移動格幅（每次 tick 走幾「格」房間）：由 `npc_behaviors.json` 各職稱的 `movement.speed` 定義，`GetMovementDefForTitle(title)` 取得；預設 1。

### 3.2 尋路引擎

- **演算法**：BFS（廣度優先搜尋），O(V+E)
- **資料結構**：`RoomGraph` — 啟動時從 DB 建立鄰接表 + tags/zone/name 快取
- **效能**：100 房 < 0.1ms，10,000 房 ~5ms
- **API**：

```go
graph := db.GetGraph()
graph.BuildGraph(database)                          // 啟動時建圖
path := graph.FindPath("life_storage", "life_hall") // → ["life_backyard", "life_kitchen", "life_dining", "life_hall"]
room, dist := graph.FindNearestByTag("life_hall", "tavern", 10) // → "life_dining", 1
rooms := graph.FindRoomsWithinDist("life_hall", []string{"outdoor"}, 5) // → ["life_garden", "life_backyard"]
```

### 3.3 移動管理器

有排班的 NPC 在 **main 啟動時** 會依 `GetAllSchedules` 自動以 `MoveSchedule` 註冊；無需手動 Register。若新增 route/pathfind 型 NPC，可手動註冊：

```go
mgr := db.NewTravelerManager()
// 排班型：由 main 依 npc_schedules 自動註冊，目標 GetScheduleTargetRoom(db, entityID, hour)

// 手動註冊例（pathfind）：
mgr.Register("老張", db.MovementDef{
    Type:            db.MovePathfind,
    Speed:           1,
    DestinationTags: []string{"inn", "tavern"},
    WanderRange:     50,
    StayHours:       [2]int{2, 6},
})
// 每 15 秒在 game loop 中：
steps := mgr.Tick(database, graph, gameHour)
// steps: [{EntityID:"老張", OldRoom:"life_hall", NewRoom:"life_dining", NpcName:"旅人"}]
```

### 3.4 房間標籤系統

NPC 不需要硬編碼房間 ID，透過 tags 做決策：

| Tag 類型 | 範例 | 用途 |
|---------|------|------|
| 地標 | `gate`, `market`, `temple`, `forge` | 旅人找城門、商人找市集 |
| 功能 | `inn`, `tavern`, `trade`, `food` | 酒客找酒館、藥師找食材 |
| 環境 | `outdoor`, `indoor`, `road`, `wilderness` | 藥師採藥去戶外 |
| 社交 | `social`, `private`, `staff` | 賣藝人去人多的地方 |

---

## 四、模板系統

### 4.1 檔案結構

```
data/templates/
├── archetypes.json         10 種職業原型（屬性 + 移動模式）
├── dialogues/*.json        對話模板（greet / idle / talk / trade_announce）
├── behaviors/*.json        行為模板（schedule / wander / trade / personality）
└── README.md               模板系統檢索文檔
```

### 4.2 佔位符規則

| 佔位符 | 替換為 | 範例 |
|--------|--------|------|
| `{name}` | NPC 個體真名 | 「王富貴」 |
| `{goods}` | 販賣品類 | 「布匹」 |
| `{dest}` | 目的房間名 | 「喫食堂」 |
| `{from}` | 來源房間名 | 「假山庭院」 |

### 4.3 格式約定

- NPC 名稱：`【{name}】`
- 台詞：`「」`
- 動作描述：第三人稱直接敘述
- 範例：`【{name}】抬眼打量你，笑瞇瞇道：「客官裡邊請，看看有什麼合意的？」`

### 4.4 現有內容量

| 職業 | 對話句數 | 行為日程條數 | 巡邏文本 | 議價文本 |
|------|---------|------------|---------|---------|
| 商人 | ~72 | 9 | 5+5 | 4+4+4 |
| 鐵匠 | ~72 | 9 | 4+4 | 4+4+4 |
| 學者 | ~72 | 9 | 4+4 | 4+4+4 |
| 旅人 | ~72 | 9 | 4+4 | 3+3+3 |
| 酒客 | ~70 | 8 | 4+4 | 3+3+3 |
| 乞丐 | ~70 | 7 | 6+6 | 3+3+3 |
| 守衛 | ~72 | 9 | 4+4 | 3+3+3 |
| 藥師 | ~72 | 9 | 4+4 | 3+3+3 |
| 賣藝人 | ~72 | 8 | 4+4 | 3+3+3 |
| 農夫 | ~72 | 9 | 4+4 | 3+3+3 |
| **合計** | **~716** | **86** | **~86** | **~96** |

---

## 五、演進路線圖

### 第一階段：NPC 有呼吸 ✅ 已完成

讓 NPC 不再是「站樁」，而是會動、會反應、有作息。

| 項目 | 狀態 | 效果 |
|------|------|------|
| 閒置動作（時段感知） | ✅ | 早晨擦桌子、深夜打哈欠 |
| 進房反應 | ✅ | 玩家進來，NPC 抬頭打招呼 |
| 換班敘事 | ✅ | 「陳正明合上帳本，朝後院走去」 |
| 區域巡邏 | ✅ | 服務生偶爾去食堂再回來 |
| 前端即時同步 | ✅ | NPC 移動後人物欄立刻更新 |

### 第二階段：NPC 有腳 ✅ 已完成

讓 NPC 能走出固定區域，在地圖上自由移動。

| 項目 | 狀態 | 效果 |
|------|------|------|
| 房間標籤（tags/zone） | ✅ | NPC 能按功能找房間 |
| BFS 尋路引擎 | ✅ | 給起終點自動算路 |
| 四種移動模式（含 schedule） | ✅ | 排班尋路 / 定點 / 路線 / 自由遊走 |
| 移動管理器整合 game loop | ✅ | 每 15 秒推進一步 |
| 10 種職業模板（含 movement） | ✅ | 模板定義好，待生成引擎串接 |

### 第三階段：NPC 有嘴 ⬜ 待做

讓 NPC 不只是「看著你微微頷首」，而是能交談和交易。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| Talk 串接對話模板 | ⬜ | 玩家點 Talk → 從 dialogues/*.json 抽句 | 小 |
| Trade 交易流程 | ⬜ | 出價 → 議價 → 成交/拒絕 | 中 |
| 模板 NPC 生成器 | ⬜ | 讀 archetypes.json → 批量生成 NPC 個體 | 中 |
| NPC 喊價 | ⬜ | NPC 主動在 log 喊 trade_announce | 小 |

### 第四階段：NPC 有記憶 ⬜ 待做

讓 NPC 記住玩家，產生「這個人認識我」的感覺。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| 短期記憶（npc_memory 表） | ⬜ | 見面次數、上次見面、好感度 | 小 |
| 對話分級（初見/熟客/密友） | ⬜ | 「你是生面孔」→「又是你，老主顧了」 | 小 |
| 交易記憶 | ⬜ | 「上次的布匹用得還行吧？」 | 小 |

### 第五階段：NPC 有眼 ⬜ 待做

讓 NPC 能「看見」並反應玩家的行為。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| 戰鬥反應 | ⬜ | 看到打架→逃跑/圍觀/報官 | 小 |
| 偷竊反應 | ⬜ | 看到偷東西→喊叫/攔截 | 小 |
| NPC 之間互動 | ⬜ | 商人聊價格、守衛趕乞丐 | 中 |
| 觀測坍縮整合 | ⬜ | 進房觸發觀測、離開恢復 | 中 |

### 第六階段：NPC 有心 ⬜ 待做

讓 NPC 有情緒變化和個性表現。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| 情緒狀態機 | ⬜ | neutral/happy/angry/scared/drunk/tired | 中 |
| 情緒影響對話 | ⬜ | 開心的商人更大方、生氣的守衛更凶 | 小 |
| 情緒觸發事件 | ⬜ | 被攻擊→angry、深夜→tired | 小 |
| 性格偏移（SoulSeed） | ⬜ | 同職業不同人有不同脾氣 | 中 |

### 第七階段：NPC 有社會 ⬜ 遠期

讓 NPC 構成一個有機的社會網絡。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| 傳聞系統（Gossip） | ⬜ | NPC 傳遞消息，越傳越誇張 | 大 |
| 生命週期 | ⬜ | 出生、成長、老化、退休/死亡 | 大 |
| 社會關係 | ⬜ | 朋友/敵人/師徒/買賣夥伴 | 大 |
| LLM 動態對話 | ⬜ | 重要 NPC 即興回應 | 大 |

### 第八階段：萬人規模 ⬜ 遠期

讓世界從幾十人擴展到數萬人。

| 項目 | 狀態 | 效果 | 工作量 |
|------|------|------|--------|
| 模板批量生成 | ⬜ | 一次生成數百 NPC | 中 |
| 三層模擬 | ⬜ | 全模擬 / 輕量 / 統計 | 大 |
| 惰性實體化 | ⬜ | 只有玩家附近的 NPC 才完整模擬 | 大 |
| AI 模板擴充 | ⬜ | 丟格式給 Gemini 生成新職業內容 | 中 |

---

## 六、程式碼速查

### 6.1 核心檔案

| 檔案 | 行數 | 職責 |
|------|------|------|
| `db/behavior.go` | ~162 | 載入 npc_behaviors.json、PickIdleEmote、PickEnterReaction、GetShiftFlavor、GetWanderFlavor |
| `db/pathfind.go` | ~190 | RoomGraph、BuildGraph、FindPath（BFS）、FindNearestByTag、FindRoomsWithinDist |
| `db/npc_movement.go` | ~280 | TravelerManager、Register/Tick/Unregister、四種移動模式（含 MoveSchedule）邏輯 |
| `db/schedule.go` | ~125 | NPCSchedule、GetScheduleTarget/GetScheduleTargetRoom、ApplySchedules（只回傳不傳送） |
| `db/npc.go` | ~88 | InsertNPC、InsertSchedule、SeedNPCs（defaultNPCs 目前為空） |
| `db/room.go` | ~370 | Room（含 tags/zone）、SyncRoomsFromFile、GetRoomsByTag/Zone |
| `server/broadcast.go` | ~50 | SendNarrateToRoom、GetPlayerRoomMap、RefreshRoomViews |
| `server/handler.go` | - | handleMove 中的進房反應 goroutine |
| `main.go` | ~200 | game loop 整合排班 + 閒置 + 巡邏 + 地圖移動 |

### 6.2 資料表

| 表 | 欄位 | 用途 |
|----|------|------|
| `rooms` | id, name, description, **tags**, **zone** | 房間定義（含標籤） |
| `exits` | from_room_id, direction, to_room_id | 出口（鄰接關係） |
| `entities` | id, kind, display_title, soul_seed, ... | 玩家 + NPC 共用 |
| `entity_room` | entity_id, room_id | 實體當前位置 |
| `npc_schedules` | entity_id, work_room, rest_room, shift_start, shift_end | NPC 排班 |

### 6.3 WebSocket 訊息

| 類型 | 方向 | 用途 |
|------|------|------|
| `narrate` | Server → Client | 環境敘事（閒置/反應/換班/巡邏） |
| `room_view` | Server → Client | 房間視野更新（含實體列表） |
| `action_result` | Server → Client | 玩家動作結果（Look/Talk/Attack） |
| `moved` | Server → All | 某實體移動了（廣播） |

---

## 七、Game Loop 時序

```
每 200ms tick：
│
├─ RunViewSimulation（視野內實體即時模擬）
│
├─ 計算遊戲時間 → hour
│
├─ [每遊戲小時] NPC 排班（僅敘事，不傳送）
│   ├─ ApplySchedules(db, hour) → []ScheduleMove（誰「應」去 work/rest）
│   ├─ 對每個 move：只推「出發」敘事（shift_leave 或「出門往店裡去了」）到 OldRoom
│   └─ 不呼叫 SetEntityRoom；實際位置由 TravelerManager 排班型逐格更新
│
├─ [每 15 秒] TravelerManager.Tick（排班/路線/尋路型逐格移動）
│   ├─ travelerMgr.Tick(db, graph, hour) → []NPCStep
│   ├─ 排班型：currentRoom != 目標時 BFS 尋路，依 speed 走若干格，SetEntityRoom
│   ├─ 抵達排班目標時：推送 shift_arrive 或「回到了住處」
│   └─ RefreshRoomViews（來源 + 目的房間）
│
└─ [每 5-12 秒] 定點 NPC 閒置 & 區域巡邏
    ├─ 取所有在班 NPC（GetAllSchedules）
    ├─ 10% 機率巡邏：若 wander_rooms 有多房，瞬移一房 + 敘事 + RefreshRoomViews
    └─ 有玩家在場 → 閒置動作敘事（每輪最多一人）
```

---

## 七點五、邏輯與流程檢核（2026-03-07）

截至目前實作已對齊以下流程，單元測試與啟動煙霧通過：

| 檢核項 | 說明 |
|--------|------|
| **排班不傳送** | `ApplySchedules` 只回傳 `[]ScheduleMove`（誰應去哪），不呼叫 `SetEntityRoom`；測試驗證回傳內容且實體仍留原房。 |
| **排班目標 API** | `GetScheduleTarget(db, entityID, hour)` 回傳 `(Room, IsWork)`；`GetScheduleTargetRoom` 僅回傳房間；有單元測試。 |
| **MoveSchedule 尋路** | `TravelerManager.Tick` 內對 `MoveSchedule` 呼叫 `GetScheduleTargetRoom` → `FindPath(current, target)` → 依 `Speed` 步進並 `SetEntityRoom`。 |
| **啟動註冊** | main 啟動時 `GetAllSchedules` → 對每筆 `GetMovementDefForTitle`、`Type=MoveSchedule`、`Register`；無排班時 traveler 數為 0。 |
| **敘事分離** | 每遊戲小時對 `ApplySchedules` 的 moves 只發「出發」敘事；抵達敘事在 `travelSteps` 中依 `GetScheduleTarget` 判斷是否為 work/rest 再發。 |
| **閒置與巡邏** | 仍依 `GetAllSchedules` + `IsOnDuty`；在班時 10% 區域巡邏（`SetEntityRoom` 到 wander_rooms 一房）或閒置動作；與排班型可並存（巡邏可能與排班路徑交錯，目前可接受）。 |

---

## 八、「跑穩」的定義

活化系統「跑穩」意味著以下指標皆滿足：

### 8.1 功能穩定

| 指標 | 驗收方式 |
|------|---------|
| 排班準時 | 日夜班交接時發「出發」敘事，NPC 依尋路逐格走到 work/rest，抵達時發敘事並更新前端 |
| 閒置不洗版 | 5~12 秒一句，不會連續刷屏 |
| 進房反應自然 | 延遲 0.5~1.5 秒，不會每次都同一句 |
| 巡邏後歸位 | NPC 巡邏完能正常回到工作區 |
| 地圖移動順暢 | schedule/route/pathfind NPC 逐格移動，不跳房（排班型家可十格外） |
| 前端同步 | NPC 任何移動，玩家的人物欄都即時更新 |

### 8.2 效能穩定

| 指標 | 目標值 |
|------|--------|
| 單 tick 處理時間 | < 10ms（30 房 + 4 NPC） |
| BFS 路徑計算 | < 1ms（< 1000 房間） |
| 記憶體佔用（RoomGraph） | < 1MB（< 10,000 房間） |
| WebSocket 推送延遲 | < 50ms |

### 8.3 體驗穩定

| 指標 | 驗收方式 |
|------|---------|
| NPC 不會「穿牆」 | 移動都走合法出口路徑 |
| NPC 不會「卡死」 | path 為空時能重新計算目標 |
| NPC 不會「分身」 | 同一個 NPC 同時只在一個房間 |
| 敘事不重複 | 連續兩次閒置不出同一句 |
| 時段銜接平滑 | morning → noon 切換時不會出現矛盾的敘事 |

---

## 九、新增職業速查

### 9.1 新增定點 NPC（有排班）

1. 呼叫 `db.InsertNPC(db, id, displayChar, gender, "")` 建立實體
2. `db.SetEntityRoom(db, id, workRoom)` 設初始房間
3. `db.InsertSchedule(db, id, workRoom, restRoom, shiftStart, shiftEnd)` 設排班
4. `db.InsertAssignment(db, id, title, venueID, "")` 設職稱（可選）
5. 在 `data/npc_behaviors.json` 的 `roles` 加該職稱的文本（含 `movement.speed` 可選）
6. 重啟伺服器後，該 NPC 會自動以 MoveSchedule 註冊，依班表尋路上下班

### 9.2 新增模板職業

1. `data/templates/archetypes.json` 加原型（含 movement）
2. `data/templates/dialogues/` 加對話 JSON（可用 AI 生成）
3. `data/templates/behaviors/` 加行為 JSON（可用 AI 生成）
4. 重啟伺服器

### 9.3 AI 生成 Prompt 範例

```
請依照以下 JSON 格式，為「獵人」職業生成對話模板。

結構：
- greet.lines: 8 句（玩家進房招呼）
- idle.morning/noon/evening/night: 各 8 句
- talk.lines: 8 句（主動交談回應）
- trade_announce.buy: 8 句, sell: 8 句

規則：
1. 繁體中文
2. 【{name}】包 NPC 名，「」包台詞
3. {goods} 替換商品
4. 獵人性格：沉默寡言、敏銳、熟悉山林、不善社交

（附上 merchant.json 作為格式範例）
```

---

## 十、相關文檔

| 文檔 | 位置 | 說明 |
|------|------|------|
| **NPC 活化模擬測試報告** | `docs/testing/NPC活化系統模擬測試報告.md` | 檢索範圍、已／未實作對照（含馬斯洛）、模擬測試案例與結果、代碼註釋建議 |
| **NPC 活化實作清單與規劃** | `docs/implementation/NPC活化系統—實作清單與規劃.md` | 細部拆解：數據層／實體／soul_seed 展開／行為／移動／主迴圈／互動／未實作（需求驅動）、依賴與驗收、階段排程 |
| **NPC 活化引擎與數據對照** | `docs/implementation/NPC活化系統—引擎與數據對照.md` | 每環節對應 .go 與 .json／數據池；已載入與否、JSON 可掌控／生成清單 |
| 模板系統檢索 | `data/templates/README.md` | 模板格式、欄位、佔位符、快速查閱表 |
| 第一版可做清單 | `docs/第一版可做清單.md` | MVP 進度追蹤（§十 NPC 行為） |
| 協作約定 | `docs/COLLABORATION.md` | 主管與 AI 的角色分工 |
| 技術約束 | `docs/技術約束規則.md` | Go / SQLite / WebSocket 等約束 |
| 人物角色模板 | `docs/reference/人物角色模板.md` | 玩家/NPC 共用結構定義 |

---

*奇點世界專案 — NPC 活化系統文檔 v1.0*

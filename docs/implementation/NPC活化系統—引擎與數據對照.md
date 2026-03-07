# NPC 活化系統 — 引擎與數據對照（.go ＋ .json／數據池）

> 產出日期：2026-03-07  
> 原則：**每個環節 = 一組 .go 引擎 ＋ 一至多個 .json 或數據庫／池**；.json 多為您可掌控、可生成之內容。

---

## 使用說明

- **引擎**：以 `.go` 程式檔實作的邏輯（讀取、計算、寫入）。
- **數據／池**：`.json` 檔案或 DB 表，供引擎讀取或寫入；標註 **可掌控／生成** 表示內容可由您編輯或由工具／AI 生成。
- **已載入**：啟動或執行時該 .go 會讀取該 .json 或 DB。**未載入**：檔案存在但尚無 .go 讀取，屬預留或待串接。

---

## 一、行為文本與移動參數（定點 NPC）

| 項目 | 說明 |
|------|------|
| **環節** | 閒置／進房反應／換班／巡邏敘事、時段對應、**移動格幅（speed）**、巡邏房列表 |
| **.go 引擎** | `db/behavior.go` — LoadBehaviors、GetBehaviors、PickIdleEmote、PickEnterReaction、GetShiftFlavor、GetWanderFlavor、GetWanderRooms、GetTimePeriod、**GetMovementDefForTitle**、RoleMovementConfig |
| **.json 數據** | **`data/npc_behaviors.json`**（已載入，main 啟動時 LoadBehaviors） |
| **JSON 可掌控** | ✅ 是。您可新增／修改職稱（roles）、各職稱的 idle／enter_reactions／shift_*／wander_*、**movement.speed**、time_periods。改完重啟即生效。 |
| **結構要點** | `roles.{職稱}` 下：idle（morning/noon/evening/night 各一組句陣列）、enter_reactions、shift_arrive、shift_leave、wander_rooms、wander_leave、wander_arrive、**movement**（含 speed）。 |

---

## 二、職業與插座（NPC 在場時可執行動作）

| 項目 | 說明 |
|------|------|
| **環節** | 職業定義、該職業在場時開放的動作插座（如 Trade）、對話/行為檔檔名參照 |
| **.go 引擎** | `db/occupation.go` — LoadOccupations、GetOccupationActionSockets、GetSocketsForNPC（結合 assignments 與房間是否在場所內） |
| **.json 數據** | **`data/templates/occupations.json`**（已載入，main 啟動時 LoadOccupations） |
| **JSON 可掌控** | ✅ 是。您可新增職業、name、dialogue_file、behavior_file、**action_sockets**。目前用於「該職業在場時多出哪些插座」（例：Trade）。 |
| **結構要點** | `occupations.{occupation_id}`：name、dialogue_file、behavior_file、action_sockets（字串陣列）。 |

---

## 三、房間與出口（尋路圖與標籤）

| 項目 | 說明 |
|------|------|
| **環節** | 房間 id／name／description／**tags**／**zone**；出口 from–direction–to；同步進 DB 後供尋路與 NPC 決策 |
| **.go 引擎** | `db/room.go` — **SyncRoomsFromFile**、SeedRooms；讀取 JSON 後 INSERT/UPDATE `rooms` 與 `exits` 表。`db/pathfind.go` — BuildGraph 從 **DB** 讀 rooms＋exits 建鄰接表與 tags/zone/name 快取，**不直接讀 JSON**。 |
| **.json 數據** | **`data/rooms.json`**（已載入，OpenDB 時 SeedRooms → SyncRoomsFromFile；另 main 提供 /data/rooms.json 給地圖檢視器） |
| **JSON 可掌控** | ✅ 是。您可新增／修改房間、出口、**tags**、**zone**。Sync 後 DB 更新，BuildGraph 下次建圖時生效（重啟或重新 BuildGraph）。 |
| **結構要點** | rooms 陣列：id、name、description、tags（陣列）、zone；exits 陣列：from、direction、to。 |
| **備註** | 尋路引擎讀的是 **DB**（rooms、exits 表），不是直接讀 JSON；JSON 是「來源 of truth」經 Sync 寫入 DB。 |

---

## 四、房間內非人物件（可互動物件）

| 項目 | 說明 |
|------|------|
| **環節** | 房間內物件的 id／name／owner／sockets／responses（Look 等回傳文本） |
| **.go 引擎** | `db/room_object.go` — **LoadRoomObjects**、GetObjectsInRoom、GetObjectByID、ObjectHasSocket 等；server 用於 GetRoomView、do_action 目標為物件時 |
| **.json 數據** | **`data/room_objects.json`**（已載入，main 啟動時 LoadRoomObjects） |
| **JSON 可掌控** | ✅ 是。您可依房間為 key，每房一組物件陣列；每物件 id、name、owner、sockets、responses（動作→文本）。改完重啟即生效。 |
| **結構要點** | 頂層 key 為 room_id；value 為物件陣列。物件：id、name、owner、sockets（陣列）、responses（動作→字串）。 |

---

## 五、實體與 soul_seed（無 .json；程式＋DB）

| 項目 | 說明 |
|------|------|
| **環節** | NPC／玩家創角、soul_seed 產生、體敏氣／本源句／性格／760 邊權皆由**同一 seed 展開**（不落 JSON） |
| **.go 引擎** | `db/entity.go` — GenerateSoulSeed、ExpandSoulSeedToBaseStats、ExpandSoulSeedToOriginSentence、**ExpandSoulSeedToPersonality**、GetPersonalityForEntity、InsertEntity、GetEntity。`db/npc.go` — InsertNPC（呼叫 GenerateSoulSeed、ExpandSoulSeedToBaseStats、寫入 entities）。`db/topology.go` — ExpandSoulSeedToTopologyCosts。 |
| **.json 數據** | **無**。種子為程式產生（crypto/rand）；三軸常數在 entity.go 內（ampMin/Max 等）。 |
| **數據池** | **DB**：`entities` 表（id、kind、soul_seed、vit、qi、dex、…）。 |
| **可掌控** | 三軸常數與公式在 .go 內；若未來要「可調參數」可抽成設定檔（未實作）。 |

---

## 六、排班與移動目標（無 .json；DB）

| 項目 | 說明 |
|------|------|
| **環節** | 誰在幾點～幾點上班、工作房／休息房；依 gameHour 回傳目標房（work 或 rest） |
| **.go 引擎** | `db/schedule.go` — NPCSchedule、GetAllSchedules、**GetScheduleTarget**、**GetScheduleTargetRoom**、ApplySchedules（只回傳不傳送）。 |
| **.json 數據** | **無**。排班寫在 DB。 |
| **數據池** | **DB**：`npc_schedules` 表（entity_id、work_room、rest_room、shift_start、shift_end）。 |
| **可掌控** | 透過程式或未來管理介面寫入 npc_schedules；若日後要做「排班模板」可另做 .json＋匯入（未實作）。 |

---

## 七、指派與職稱（無 .json；DB）

| 項目 | 說明 |
|------|------|
| **環節** | 誰、什麼職業、哪個場所；GetNPCTitle 先查指派再 fallback display_title |
| **.go 引擎** | `db/assignment.go` — InsertAssignment、GetAssignmentsForEntity、**GetNPCTitleFromAssignments**；db/schedule.go GetNPCTitle。 |
| **.json 數據** | **無**。指派寫在 DB。 |
| **數據池** | **DB**：`assignments` 表（entity_id、occupation_id、venue_id、assigned_by）。`venues` 表（SeedVenues 在 .go 內寫死浮生客棧，無 JSON）。 |
| **可掌控** | 透過 InsertAssignment 或未來求職／管理流程寫入；venues 目前程式寫死，可改為 JSON 或 DB 種子。 |

---

## 八、尋路與四種移動模式

| 項目 | 說明 |
|------|------|
| **環節** | BFS 尋路、四種移動模式（schedule／regional／route／pathfind）、TravelerManager 註冊與 Tick |
| **.go 引擎** | `db/pathfind.go` — RoomGraph、BuildGraph（讀 **DB** rooms＋exits）、FindPath、FindNearestByTag、FindRoomsWithinDist。`db/npc_movement.go` — MovementDef、TravelerManager、Register、Tick、computeNextPath（schedule 用 GetScheduleTargetRoom；route/pathfind 用 waypoints/tags）。 |
| **.json 數據** | **無直接對應**。schedule 目標來自 DB（npc_schedules）；regional 的 WanderRooms 來自 **npc_behaviors.json** 的 roles.{職稱}.wander_rooms；route 的 waypoints 規劃上可來自 **templates/behaviors/*.json**（目前未載入）。pathfind 用 tags（房間圖從 DB 來，tags 來自 rooms 表，表內容可來自 rooms.json Sync）。 |
| **數據池** | **DB**：rooms、exits（來源可為 rooms.json）。**npc_behaviors.json**：wander_rooms、movement.speed（已用）。**templates/behaviors/*.json**：route 等參數（未載入）。 |
| **可掌控** | rooms.json 可改房間與 tags；npc_behaviors 可改 wander_rooms、movement；若接上 behaviors/*.json 則可改 route waypoints 等。 |

---

## 九、主迴圈整合（無獨立 .json）

| 項目 | 說明 |
|------|------|
| **環節** | 每遊戲小時排班敘事、每 15 秒 Tick 移動、每 5–12 秒閒置與巡邏 |
| **.go 引擎** | `main.go` — game.Loop、ApplySchedules、TravelerManager.Tick、GetAllSchedules、GetMovementDefForTitle、Register、idleTickCount、GetWanderRooms、PickIdleEmote、SetEntityRoom（巡邏瞬移）、SendNarrateToRoom、RefreshRoomViews。`game/` — GameTimeNow、Loop。 |
| **.json 數據** | 主迴圈不直接讀 .json；依賴前述引擎已載入的 npc_behaviors、occupations、DB。 |

---

## 十、玩家與 NPC 互動（Talk／Look／Attack）

| 項目 | 說明 |
|------|------|
| **環節** | Look／Talk／Attack 的 handler、Talk 選句（固定 8 句＋性格 Boldness 偏移）、插座列表 |
| **.go 引擎** | `server/handler.go` — handleDoAction、**buildTalkNarrative**（接收 personality，Boldness 偏移）、buildLookNarrative、buildAttackNarrative；進房反應 goroutine 用 PickEnterReaction。`db/occupation.go` — GetSocketsForNPC。 |
| **.json 數據** | **Talk 選句**：目前**未**用 .json，8 句寫在 buildTalkNarrative 內；性格來自 soul_seed（無 JSON）。**對話模板**：`data/templates/dialogues/*.json`（**未載入**，規劃為 Talk 串接模板後從此抽句）。 |
| **可掌控** | 若實作「Talk 串接對話模板」，則 **dialogues/*.json** 為您可掌控／生成的句庫（key 如 greet、talk、trade_announce 等）。 |

---

## 十一、模板池（已存在檔案，尚未被 .go 載入）

| 項目 | 說明 |
|------|------|
| **環節** | 職業原型、對話句庫、行為參數（日程／巡邏／交易／性格）— 供未來「模板 NPC 生成」與 Talk 串接使用 |
| **.go 引擎** | **目前無**。archetypes／dialogues／behaviors 的載入與使用待實作。 |
| **.json 數據** | **`data/templates/archetypes.json`**（10 種職業屬性＋移動模式）— 未載入。<br>**`data/templates/dialogues/*.json`**（各職業 greet／idle／talk／trade_announce 等）— 未載入。<br>**`data/templates/behaviors/*.json`**（各職業 schedule／wander／trade／personality）— 未載入。 |
| **可掌控** | ✅ 是。這些 JSON 皆為您可編輯、可生成之內容；待 .go 串接後即可驅動「依模板生成 NPC」「Talk 從句庫抽句」等。 |

---

## 十二、對照總表（一覽）

| 環節 | .go 引擎（檔案） | .json／數據池 | 已載入？ | JSON 可掌控／生成 |
|------|------------------|---------------|----------|-------------------|
| 行為文本＋移動參數 | `db/behavior.go` | `data/npc_behaviors.json` | ✅ | ✅ |
| 職業與插座 | `db/occupation.go` | `data/templates/occupations.json` | ✅ | ✅ |
| 房間與出口 | `db/room.go`（Sync）＋`db/pathfind.go`（讀 DB） | `data/rooms.json` → DB rooms/exits | ✅ | ✅ |
| 房間物件 | `db/room_object.go` | `data/room_objects.json` | ✅ | ✅ |
| 實體與 soul_seed | `db/entity.go`、`db/npc.go`、`db/topology.go` | 無；DB entities | — | 常數在 .go |
| 排班 | `db/schedule.go` | DB npc_schedules | — | 可做排班模板 JSON（未做） |
| 指派與場所 | `db/assignment.go` | DB assignments、venues | — | venues 目前 .go 寫死 |
| 尋路與移動 | `db/pathfind.go`、`db/npc_movement.go` | DB rooms/exits；npc_behaviors（wander_rooms/speed） | ✅ | rooms＋npc_behaviors ✅ |
| 主迴圈 | `main.go`、`game/` | 依賴上述 | — | — |
| Talk／Look／Attack | `server/handler.go`、`db/occupation.go` | Talk 目前無；規劃 dialogues/*.json | ⬜ | 串接後 dialogues ✅ |
| 模板池（原型／對話／行為） | 無（待實作） | `archetypes.json`、`dialogues/*.json`、`behaviors/*.json` | ⬜ | ✅ |

---

## 十三、您可掌控的 .json 清單（建議維護優先）

| 檔案 | 用途 | 目前是否被引擎讀取 |
|------|------|----------------------|
| **data/npc_behaviors.json** | 定點 NPC 閒置／進房／換班／巡邏文本、時段、**movement.speed**、wander_rooms | ✅ 是 |
| **data/templates/occupations.json** | 職業名、對話/行為檔參照、**action_sockets** | ✅ 是 |
| **data/rooms.json** | 房間 id/name/description/**tags**/**zone**、出口 | ✅ 是（Sync 進 DB） |
| **data/room_objects.json** | 各房內物件 id/name/sockets/responses | ✅ 是 |
| **data/templates/archetypes.json** | 職業原型與移動模式（待生成引擎） | ⬜ 否 |
| **data/templates/dialogues/*.json** | 各職業句庫（待 Talk 串接） | ⬜ 否 |
| **data/templates/behaviors/*.json** | 各職業日程/巡邏/交易參數（待串接） | ⬜ 否 |

---

*NPC 活化系統 — 引擎與數據對照 v1.0 — 2026-03-07*

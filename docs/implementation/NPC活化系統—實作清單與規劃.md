# NPC 活化系統 — 實作清單與規劃

> 產出日期：2026-03-07  
> 用途：將 NPC 活化系統**細部拆解**為可勾選、可依賴的實作項目，方便理解與排程。

---

## 使用方式

- **狀態圖例**：✅ 已完成、🟡 部分完成、⬜ 未實作、📄 僅文件／討論。
- **依賴**：做某項前建議先完成其依賴項。
- **驗收**：完成後可依「驗收」欄自檢或寫測試。
- 規劃時可依「階段」或「依賴」決定先做哪一塊。

---

## 一、數據層（資料與設定檔）

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| D1 | 定點行為文本 | 職稱（經理/服務生等）的閒置、進房反應、換班、巡邏、**movement.speed** 文本 | `data/npc_behaviors.json` | ✅ | — | 有 roles、idle/enter_reactions/shift_*/wander_*、movement.speed |
| D2 | 職業原型 | 10 種職業的屬性與移動模式定義 | `data/templates/archetypes.json` | ✅ | — | 模板存在，待生成引擎讀取 |
| D3 | 對話模板 | 各職業 greet/idle/talk/trade_announce 等句庫 | `data/templates/dialogues/*.json` | ✅ | — | ~716 句，Talk 尚未從此抽句 |
| D4 | 行為模板 | 各職業 schedule/wander/trade/personality 參數 | `data/templates/behaviors/*.json` | ✅ | — | 與 archetypes 對齊 |
| D5 | 房間與標籤 | 房間 id/name、**tags**、**zone**，供尋路與 NPC 決策 | `data/rooms.json`、DB `rooms` 表 | ✅ | — | rooms 有 tags/zone；SyncRoomsFromFile 可載入 |
| D6 | 場所與職缺 | 場所 id、名稱、room_ids；**max_staff**（職缺上限） | DB `venues` 表、討論 002 | 🟡 | — | venues 有；max_staff 與求職邏輯 ⬜ |

---

## 二、實體與身份

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| E1 | NPC 創角與 soul_seed | 創角時產生 **soul_seed**，寫入 entities；體敏氣由同 seed 展開寫入 | `db/npc.go` InsertNPC | ✅ | — | InsertNPC 後 GetEntity 必有 SoulSeed；vit/qi/dex 與 ExpandSoulSeedToBaseStats 一致 |
| E2 | 玩家創角與 soul_seed | 同上，kind=player | `db/entity.go` InsertEntity | ✅ | — | 同上 |
| E3 | 職稱來自指派 | 職稱先查 **assignments**（entity_id + occupation_id + venue_id），無則 fallback entities.display_title | `db/assignment.go` GetNPCTitleFromAssignments；`db/schedule.go` GetNPCTitle | ✅ | D5 | GetNPCTitle(無指派) 回傳空或 display_title |
| E4 | 排班表 | 誰、工作房、休息房、班次起迄（gameHour） | DB `npc_schedules`；`db/schedule.go` NPCSchedule、GetAllSchedules | ✅ | E1 | 有排班則 main 會註冊 MoveSchedule |
| E5 | 鎂欄位 | 實體當前鎂餘額，供狀態與未來消耗／求職 | `entities.magnesium`、entity.Character.Magnesium | ✅ | — | 讀寫存在；**無消耗、無閾值**（見需求驅動） |

---

## 三、soul_seed 展開（三軸 → 數值／文本／性格）

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| S1 | 三軸常數 | amp/freq/phase 區間，與人物屬性彙整一致 | `db/entity.go` ampMin/Max, freqMin/Max, phaseMin/Max | ✅ | — | 與 BaseStats／OriginSentence／Personality 共用 |
| S2 | BaseStats | 同 seed 前 3 次 RNG → 體質／氣脈／靈敏（clamp 1–30） | `db/entity.go` ExpandSoulSeedToBaseStats | ✅ | S1 | 同 seed 結果確定性；InsertNPC 寫入 vit/qi/dex |
| S3 | OriginSentence | 同 RNG 序 → 一句「本源」語感（狀態/星盤分頁） | `db/entity.go` ExpandSoulSeedToOriginSentence | ✅ | S1 | 同 seed 語義與三軸一致（如霸道→高 amp） |
| S4 | Personality | 同 RNG 序 → Boldness/Sensitivity/Orderliness ∈ [0,1] | `db/entity.go` Personality、ExpandSoulSeedToPersonality | ✅ | S1 | 同 seed 確定性；TestSim_SoulSeedDeterminism |
| S5 | GetPersonalityForEntity | 依 entity_id 查 soul_seed，有則展開 Personality | `db/entity.go` GetPersonalityForEntity | ✅ | S4, E1 | 有 seed→(p,true)；無/查無→(零值,false) |
| S6 | TopologyCosts | 同 seed 第 4～763 次 RNG → 760 條邊權（星盤） | `db/topology.go` ExpandSoulSeedToTopologyCosts | ✅ | S1 | 總和 10000；不與前三項搶前 3 次 RNG |

---

## 四、行為引擎（查文本、移動定義）

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| B1 | 行為檔載入 | 啟動時讀 npc_behaviors.json，快取 | `db/behavior.go` LoadBehaviors、GetBehaviors | ✅ | D1 | main 呼叫 LoadBehaviors；Pick* 依職稱取句 |
| B2 | 閒置動作 | 依職稱＋時段（morning/noon/evening/night）隨機一句 | PickIdleEmote(title, period, npcName) | ✅ | B1, D1 | 有玩家在房時 main 每 5–12 秒推一句 |
| B3 | 進房反應 | 玩家進房後延遲 0.5–1.5s，同房 NPC 隨機一句 | PickEnterReaction；server/handler handleMove | ✅ | B1, D1 | 進房後 log 出現 NPC 反應 |
| B4 | 換班敘事 | 上班／下班一句（shift_arrive / shift_leave） | GetShiftFlavor(title, npcName, arriving) | ✅ | B1, D1 | 每遊戲小時出發敘事；抵達時 main 發 arrive 敘事 |
| B5 | 巡邏敘事 | 離開／到達房間一句（wander_leave / wander_arrive） | GetWanderFlavor(title, npcName, roomName, leaving) | ✅ | B1, D1 | 區域巡邏時 main 替換並推送 |
| B6 | 巡邏房間列表 | 職稱對應的 wander_rooms | GetWanderRooms(title) | ✅ | B1, D1 | 10% 機率時從中選一房瞬移 |
| B7 | 移動定義（含格幅） | 職稱 → MovementDef（Speed、WanderRooms、Type 預設 regional） | GetMovementDefForTitle(title)；RoleMovementConfig | ✅ | B1, D1 | movement.speed 來自 JSON；預設 1 |
| B8 | 時段對應 | gameHour → morning/noon/evening/night | GetTimePeriod(gameHour)；time_periods | ✅ | D1 | 閒置動作依此時段選 idle 子表 |

---

## 五、尋路與移動

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| M1 | 房間圖 | 從 DB rooms + exits 建鄰接表與 tags/zone/name 快取 | `db/pathfind.go` RoomGraph、BuildGraph | ✅ | D5 | 啟動時 BuildGraph；FindPath 可用 |
| M2 | BFS 尋路 | 起點→終點最短路徑（不含起點） | FindPath(from, to) | ✅ | M1 | 不可達回傳 nil；TestSim_ScheduleAndTravelerTick 依此走一步 |
| M3 | 依 tag/距離查房 | FindNearestByTag、FindRoomsWithinDist（pathfind 型用） | `db/pathfind.go` | ✅ | M1 | pathfind 型選目標房時使用 |
| M4 | 四種移動模式 | **schedule**（排班目標 work/rest）、regional、route、pathfind | `db/npc_movement.go` MovementType、computeNextPath | ✅ | M1, E4, B7 | schedule 用 GetScheduleTargetRoom；其餘用 waypoints/tags |
| M5 | 排班目標 API | 依 gameHour 回傳應前往房間＋是否為上班地 | GetScheduleTarget、GetScheduleTargetRoom | ✅ | E4 | 在班→work_room；下班→rest_room；TestGetScheduleTarget |
| M6 | 移動管理器 | 註冊 NPC、每 tick 依 Speed 步進、寫回 entity_room | TravelerManager Register/Tick、SetEntityRoom | ✅ | M1–M5 | Tick 回傳 []NPCStep；DB 位置更新 |
| M7 | 排班不傳送 | 每遊戲小時只回傳「應前往」清單，不寫 entity_room | ApplySchedules；main 只發出發敘事 | ✅ | E4, M5 | ApplySchedules 不呼叫 SetEntityRoom；TestApplySchedules |
| M8 | 啟動註冊排班 NPC | main 啟動時 GetAllSchedules → 每人 GetMovementDefForTitle、Type=MoveSchedule、Register | `main.go` | ✅ | E4, B7, M6 | 有排班則 traveler 數 > 0；Tick 會尋路 |

---

## 六、主迴圈時序（整合點）

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| L1 | 遊戲時間 | 真實時間 → gameHour 0–23 | game.GameTimeNow；main 每 tick 取 hour | ✅ | — | 排班、時段、停留皆依 hour |
| L2 | 每遊戲小時：排班敘事 | ApplySchedules(hour) → 對每個 move 只發「出發」敘事（OldRoom） | main.go | ✅ | M7, B4 | 不傳送；敘事正確（下班/出門上工） |
| L3 | 每 15 秒：Tick 移動 | TravelerManager.Tick(db, graph, hour) → 步進、SetEntityRoom、發抵達敘事 | main.go | ✅ | M6, M5, B4 | 排班型走到 work/rest 時發 shift_arrive 或「回到了住處」 |
| L4 | 每 5–12 秒：閒置＋巡邏 | 在班 NPC：10% 巡邏（瞬移 wander_rooms）或發閒置動作（有玩家在房） | main.go | ✅ | B2, B6, B5 | 不洗版；巡邏後 RefreshRoomViews |
| L5 | 視野內模擬 | RunViewSimulation（第一版 no-op） | game/view_sim.go；main | ✅ | — | 可擴充為視野內 NPC 輕量邏輯 |

---

## 七、玩家與 NPC 互動（插座）

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| I1 | Look | 玩家對 NPC 觀看 → 外觀敘事（不開彈窗） | server/handler handleDoAction；buildLookNarrative | ✅ | — | 回傳 action_result + Narrative |
| I2 | Talk（固定句＋性格權重） | 玩家對 NPC 交談 → 8 句固定池，有 soul_seed 時依 **Boldness** 偏移選句 | buildTalkNarrative；handler 傳入 Personality | ✅ | S4, S5 | 同 NPC 同 session 可重現；高 Boldness 偏後半句 |
| I3 | Talk 串接對話模板 | 玩家點 Talk → 從 **dialogues/*.json** 依職業/key 抽句（未做） | — | ⬜ | D3, E3 | 需定義 key（greet/talk 等）與抽選規則 |
| I4 | Attack | 玩家對 NPC 攻擊 → 戰鬥公式、Log 結果 | buildAttackNarrative；combat.Resolve | ✅ | — | 勝負敘事；第一版不扣血 |
| I5 | Trade | 出價→議價→成交/拒絕（未做） | — | ⬜ | 經濟/鎂、物品 | 討論 002／經濟彙整 |
| I6 | NPC 插座列表 | 預設 Talk/Attack/Look；有指派時依場所加職業插座 | GetSocketsForNPC(entityID, roomID) | ✅ | E3, db/occupation.go | 前端依此顯示可點動作 |
| I7 | 進房反應觸發 | 玩家 handleMove 進房 → 同房 NPC 隨機一人延遲發 enter_reaction | server/handler handleMove | ✅ | B3 | 進房後 log 出現一句 NPC 反應 |

---

## 八、推送與前端

| 編號 | 項目 | 說明 | 位置 | 狀態 | 依賴 | 驗收 |
|------|------|------|------|------|------|------|
| P1 | 敘事廣播 | 對指定房間發 ambient 敘事（narrate） | SendNarrateToRoom；server/broadcast.go | ✅ | — | 該房玩家收到 narrate |
| P2 | 房間視野更新 | 房間內實體列表、出口等更新後推 room_view | RefreshRoomViews、BroadcastRoomViews | ✅ | — | NPC 移動後人物欄更新 |
| P3 | 前端 narrate 渲染 | 收到 narrate → appendNarrative、ambient 樣式 | web/main.js case 'narrate'；style.css .log-ambient | ✅ | — | 灰色小字、不洗版 |

---

## 九、未實作（僅文件／討論）

以下為**設計共識或規格**，代碼尚未實作；規劃時可單獨列為「需求驅動／求職」專案。

| 編號 | 項目 | 說明 | 參考文件 | 狀態 | 建議依賴 |
|------|------|------|----------|------|----------|
| N1 | 鎂消耗 | 隨時間或消費（食宿、裝備損耗）遞減 | 討論 002 | 📄 | economy、物品消耗 |
| N2 | 鎂閾值與求職意圖 | NPC 鎂低於閾值 → 觸發「求職意圖」 | 討論 002 | 📄 | N1、決策入口 |
| N3 | 場所職缺 | venues 或場所表有 **max_staff**；當前 assignment 數 < max_staff → 有缺 | 討論 002 | 📄 | D6、E3 |
| N4 | 求職撮合 tick | 週期性掃「無職且鎂低」NPC、「有空缺」場所 → 匹配 → InsertAssignment | 討論 002 | 📄 | N2, N3, M1（距離） |
| N5 | 離職／解僱 | 更好機會離職（刪舊 assignment、寫新）；場所主人解僱（刪 assignment） | 討論 002 | 📄 | assignment 刪除 API、決策 |
| N6 | 決策引擎入口 | Think(ctx, entityID) 或等價；取得 Personality、鎂、指派 → 選動作 | 討論 002、三軸性格規劃 Phase 2 | 📄 | S5, E5, E3 |
| N7 | 性格影響決策 | 例：Orderliness 高→留任權重高；Boldness 高→高風險工作偏好 | 三軸推導性格—實作規劃 Phase 2 | 📄 | N6, S5 |
| N8 | Talk 使用 Sensitivity | 對話選句除 Boldness 外，依 Sensitivity 加權（規劃 Phase 3 提及） | 三軸推導性格—實作規劃 | 🟡 | I2 已有 Boldness；可擴充 |

---

## 十、階段與建議排程（對應 NPC 活化系統 §五）

| 階段 | 涵蓋項目 | 狀態 | 建議下一步 |
|------|----------|------|------------|
| **一、有呼吸** | 閒置、進房反應、換班敘事、區域巡邏、前端同步 | ✅ | 維持回歸測試 |
| **二、有腳** | 房間標籤、BFS、四種移動、排班尋路、移動格幅、主迴圈 Tick | ✅ | 同上 |
| **三、有嘴** | Talk 固定句+性格 ✅；**Talk 串接 dialogues 模板** ⬜；Trade ⬜；模板 NPC 生成器 ⬜；NPC 喊價 ⬜ | 🟡 | 先做 I3（Talk 串接模板），再排 Trade |
| **四、有記憶** | npc_memory 表、對話分級、交易記憶 | ⬜ | 依產品優先級 |
| **五、有眼** | 戰鬥/偷竊反應、NPC 間互動、觀測坍縮 | ⬜ | 依產品優先級 |
| **六、有心** | 情緒狀態機、情緒影響對話、性格偏移（SoulSeed 已部分用於 Talk） | 🟡 | 可先擴充 I2 用 Sensitivity |
| **需求驅動** | N1–N7（鎂消耗、閾值、職缺、撮合、離職、決策引擎、性格權重） | 📄 | 單獨規劃「求職與需求」專案 |

---

## 十一、依賴關係簡圖（僅列關鍵）

```
D1(npc_behaviors) ─┬─ B1,B2,B3,B4,B5,B6,B7,B8
D5(rooms/tags) ─────┼─ E3(職稱), M1(圖), M2(尋路)
E1(NPC+seed) ──────┼─ S2,S4, S5(GetPersonality), E4(排班)
E4(npc_schedules) ─┼─ M5(目標), M7(ApplySchedules), M8(註冊)
S4(Personality) ───┼─ S5, I2(Talk 權重)
M1(BuildGraph) ────┼─ M2,M3, M6(Tick)
B7(MovementDef) ───┴─ M8(Register 時 Type+Speed)
```

---

## 十二、相關文件

| 文件 | 用途 |
|------|------|
| **`docs/implementation/NPC活化系統—引擎與數據對照.md`** | **每環節拆成 .go 與 .json／數據池；哪些 JSON 已載入、可掌控／生成** |
| `docs/NPC活化系統.md` | 總覽、架構圖、演進路線、跑穩定義、程式碼速查 |
| `docs/testing/NPC活化系統模擬測試報告.md` | 已/未實作對照、模擬測試案例與結果 |
| `docs/discussions/002_NPC需求驅動與求職機制.md` | 馬斯洛、求職、職缺、撮合、離職（未實作） |
| `docs/implementation/三軸推導性格—實作規劃.md` | Personality、Phase 1–3、Talk 權重規格 |
| `data/templates/README.md` | 模板格式、佔位符、職業一覽 |

---

*NPC 活化系統實作清單 v1.0 — 2026-03-07*

# NPC 相關設定 — 已實做與未實做

> 對照：`docs/NPC活化系統.md`、`docs/implementation/NPC活化系統—實作清單與實作計畫.md`、討論 002／003。  
> 最後更新：2026-02-12

---

## 一、已實做

### 數據與設定

| 項目 | 說明 | 位置／備註 |
|------|------|------------|
| 定點行為文本 | 閒置、進房反應、換班、巡邏台詞 | `data/npc_behaviors.json`、`db/behavior.go` |
| 職業原型 | 10 種職業屬性＋移動模式 | `data/templates/archetypes.json` |
| 對話模板 | 約 10 職業 × ~72 句 | `data/templates/dialogues/*.json`、`db/dialogue.go` |
| 行為模板 | 日程／巡邏／交易／性格參數 | `data/templates/behaviors/*.json` |
| 房間標籤與場所 | tags、zone、venues | `data/rooms`、`data/venues.json`、`db/room.go`、`store` |
| NPC 池設定 | 總量與補滿間隔 | `config.NPCPoolSize`、`NPCSpawnIntervalSec`（env：`NPC_POOL_SIZE`、`NPC_SPAWN_INTERVAL_SEC`）；main 定時 SpawnOneNPCFromPool |

### 突破線 A–I（活化系統）

| 階段 | 名稱 | 實做內容 |
|------|------|----------|
| **A** | 鎂消耗 | 每日扣鎂、EvtBroke／DispDaily；`db/npc_expense.go`、main 每遊戲日 |
| **B** | Brain 停留 | 到達後停留 1–5 遊戲小時；`npc_movement.go` computeStay、MoveBrain |
| **C** | 性格偏移決策 | 意圖候選＋性格加權；`db/decision.go` personalityWeightedSelect、Decide |
| **D** | NPC 事件日誌 | LogNPCEvent、GetRecentEvents、store.RecentByEntity；背版／對話可引用 |
| **E** | disposition（心境值） | Entity.Disposition、AdjustDisposition、PickIdleEmote(disposition) |
| **F** | NPC 間互動 | 微互動（PickMicroInteraction）＋**NPC 間 AI 對話**（閒置／排班／隨機觸發、CallAITalkNPCToNPC、主題劇本、NpcNpcSummaries）；見討論 003 |
| **G** | 背版組裝 | BuildIdentity（職稱／場所／性格／心境／事件）；Talk 前帶入 |
| **H** | archival 記憶 | 寫入＋關鍵字檢索、節流與 per-NPC 上限；store + `db/archival.go` |
| **I** | CallAITalk 接入 | 玩家↔NPC 對話：背版＋記憶＋LLM（Ollama）＋模板 fallback；會後寫入記憶 |

### 移動與排班

| 項目 | 說明 |
|------|------|
| 四種移動模式 | schedule（排班）、regional、route、pathfind；BFS 尋路、TravelerManager.Tick |
| 排班系統 | GetScheduleTarget、ApplySchedules；每遊戲小時出發敘事、每 15 秒 Tick 逐格移動 |
| 觀測驅動 | 僅玩家房＋鄰房 NPC 跑決策；`buildActiveRoomIDs`、`db/unobserved.go` |
| 腦驅動意圖 | seek_job、beg、gather、trade、wander、work、idle；到達效果（加鎂、加物品、SeekJob 撮合） |

### 玩家與 NPC 互動（已上線）

| 動作 | 狀態 | 說明 |
|------|------|------|
| Look | ✅ | 點擊→外觀敘事（log） |
| Talk | ✅ | 背版＋記憶＋LLM 回覆＋**Sensitivity 口吻**（冷淡/熱絡、長短）；fallback 模板句；會後寫入記憶（僅 consolidation，不每輪寫） |
| Attack | ✅ | 戰鬥結算、Log 結果 |
| 進房反應 | ✅ | 玩家進房時 NPC 打招呼（npc_behaviors enter_reactions） |
| 插座列表 | ✅ | GetSocketsForNPC（Talk／Look／Attack／Trade 等，依場所） |

### 其他已實做

- **NPC 間對話記憶**：NpcNpcSummaries（每對 NPC 一句摘要）、觸發前讀／完成後寫；玩家優先（60s／15s 檢查）。
- **主題劇本**：`data/npc_to_npc_topics.json`（交班／閒聊／打聽）、`db/npc_topics.go`。
- **實體與身份**：soul_seed、職稱來自 assignment、排班表、鎂欄位；InsertNPC、InsertSchedule、SeedNPCs（預設空）。
- **對話結束 consolidation**：逾時 2 分鐘視為一場結束，整場壓成 1～3 條寫入 archival，並更新該 NPC 的 summary；Talk 長期記憶僅由此路徑寫入（`server/conversation_buffer.go`）。
- **Talk 使用 Sensitivity 權重**：背版與 LLM 口吻提示（冷淡/熱絡、回覆簡短/多說一兩句）；模板 fallback 與 buildTalkNarrative 選句依 Sensitivity 加權（高→偏長/熱絡、低→偏短/冷淡）；`db/backstory.go`、`ai/talk.go`、`db/dialogue.go` PickLineWeighted、handler buildTalkNarrative。

---

## 二、未實做或部分實做

### 第三階段「有嘴」（主文件仍標 ⬜）

| 項目 | 狀態 | 說明 |
|------|------|------|
| Talk 串接「純」對話模板 | 🟡 | 目前 Talk 已接 LLM＋背版＋記憶；模板為 fallback，未單獨「只抽模板句」模式 |
| **Trade 交易流程** | ⬜ 延後 | 插座有 Trade；**出價→議價→成交/拒絕**待**世界物流定版**後再實做（目前無物品可交易）。 |
| **模板 NPC 生成器** | ⬜ | 讀 archetypes 批量生成 NPC 個體未做；NPC 靠手動或 SpawnOneNPCFromPool |
| **NPC 喊價** | ⬜ | NPC 主動在 log 發 trade_announce 未做 |

### 第四階段「有記憶」（部分）

| 項目 | 狀態 | 說明 |
|------|------|------|
| 對話結束 consolidation | ✅ | 已實做：逾時 2 分鐘後下一句 Talk 時，上一場整場壓成 1～3 條寫入 archival；Talk 不每輪寫入 |
| 語意檢索（embedding＋向量） | ⬜ | 目前為多關鍵字評分檢索 |
| blocks 可更新（選做） | ⬜ | summary／relationship 由 archival 定期整理寫回 |

### 第五階段「有眼」

| 項目 | 狀態 |
|------|------|
| 戰鬥反應（看到打架→逃跑/圍觀/報官） | ⬜ 延後，待**戰鬥討論定版** |
| 偷竊反應 | ⬜ 延後，待**戰鬥／事件討論定版** |
| 觀測坍縮整合（進房觸發觀測、離開恢復） | ⬜ |
| **NPC 之間互動** | **✅**（已列入突破線 F／討論 003） |

### 第六階段「有心」

| 項目 | 狀態 |
|------|------|
| 情緒狀態機（neutral/happy/angry/…） | ⬜（僅有 disposition 數值，非完整狀態機） |
| 情緒影響對話 | ⬜ |
| 情緒觸發事件 | ⬜ |
| 性格偏移（SoulSeed）在決策中完整加權 | 🟡（三軸已傳入，V1 意圖優先序固定） |

### 需求驅動（討論 002）— 部分

| 項目 | 狀態 | 說明 |
|------|------|------|
| 鎂低→求職意圖、SeekJob 撮合 | ✅ | 決策引擎已有；到達時寫 assignment |
| 場所 max_staff／職缺檢查 | 🟡 | GetAssignmentCountByVenue 有；venues 的 max_staff 資料層為 🟡 |
| 離職／解僱／流動（換更好機會） | ⬜ | 未做 |
| 僧多粥少：狩獵／採集／行腳商／乞討／賣藝 | 🟡 | 意圖與尋路有（gather/beg/trade 等），實際掉落／收益依現有邏輯 |

### 第七～八階段（遠期）

| 項目 | 狀態 |
|------|------|
| 傳聞系統（Gossip） | ⬜ |
| 生命週期（出生／成長／老化／死亡） | ⬜ |
| 社會關係（朋友／敵人／師徒） | ⬜ |
| LLM 動態對話（即興） | 🟡（已有 CallAITalk／CallAITalkNPCToNPC，未擴充為「即興事件」） |
| 萬人規模／三層模擬／惰性實體化／AI 模板擴充 | ⬜ |

### 其他未實做

| 項目 | 說明 |
|------|------|
| NPC 主動攻擊另一 NPC | 戰鬥規則統一，但無 NPC 主動 Attack 另一 NPC 的觸發 |
| NPC 與 NPC 交易 | 無 NPC 間自動交易／以物易物 |
| Web LLM 佔位 | 討論 004：策略已定，Web 僅佔位未實做 |

---

**彙整入口**：上列已／未實做皆對照 [NPC活化系統](../NPC活化系統.md) 與其 [實作清單](../implementation/NPC活化系統—實作清單與實作計畫.md)；細部討論與記憶對照見該二文件內連結。

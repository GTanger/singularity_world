# NPC 對話記憶與背版 — 粉碎性實作步驟與檔案流程

> 對齊：[NPC對話記憶與背版—設計](NPC對話記憶與背版—設計.md)、[對話記憶系統—彙整與探討](../reference/對話記憶系統—彙整與探討.md) 共識、[決策 007](../decisions/007_NPC_AI_API與預設使用規則.md)、[活化清單 §十「四、有記憶」](NPC活化系統—實作清單與規劃.md)。

---

## 一、總覽與依賴

**狀態圖例**：✅ 已完成　⬜ 未實作　🟡 部分完成

| 狀態 | 階段 | 目標 | 產出檔案／改動 |
|:----:|------|------|----------------|
| ⬜ | **階段 1** | 固定背版（identity）只讀，Talk 時帶入 context | `db/backstory.go`（新建）、`server/handler.go`、可選 `config/config.go` |
| ⬜ | **階段 2** | archival 儲存＋寫入＋檢索，Talk 前 top-k 注入 | `store` 擴充或 `data/npc_archival.json`、`db/archival.go`（新建）、`server/handler.go`、可選 embedding |
| ⬜ | **階段 3** | blocks 可更新（summary／relationship） | 背版 JSON 或 store 擴充、定時／觸發整理 |

以下為**粉碎性步驟**：每步對應「做什麼 → 改／建哪個檔案 → 函式／型別名稱」。

---

## 二、階段 1：固定背版只讀（identity）

### 1.1 新增背版組裝模組（identity 從既有資料算出）

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 1.1.1 | 新增 `db/backstory.go`，package db | **新建** `db/backstory.go` | 本檔只做「讀取／組裝」背版，不寫入。 |
| ⬜ | 1.1.2 | 定義 `BuildIdentity(db *sql.DB, entityID string) string` | `db/backstory.go` | 回傳該 NPC 的 identity 字串（1～3 句）。 |
| ⬜ | 1.1.3 | 在 BuildIdentity 內：`list := GetAssignmentsForEntity(db, entityID)`；若 `len(list)==0`，回傳 `"你是" + GetNPCTitle(db, entityID) + "。"` | `db/backstory.go` | 無指派時僅職稱。 |
| ⬜ | 1.1.4 | 若 `len(list)>0`：取 `list[0].VenueID`、`list[0].OccupationID`；`venue := store.Default.GetVenue(venueID)`；若 venue≠nil 則 `venueName := venue.Name`，否則 `venueName := venueID` | `db/backstory.go` | 需 import `singularity_world/store`。 |
| ⬜ | 1.1.5 | 組句：`"你是" + title + "，在" + venueName + "。"`（title = GetNPCTitle）；可選第二句：從 `db.LoadOccupations` 對應的 name 或 archetypes 取「人設一句」；若無則省略 | `db/backstory.go` | 控制總長度數十字～百字。 |
| ⬜ | 1.1.6 | 撰寫單元測試 `TestBuildIdentity`：mock 或使用 test DB／store 有指派與無指派各一 | **新建** `db/backstory_test.go` 或於既有 `*_test.go` 加測 | 驗收：無指派回傳含職稱；有指派回傳含職稱＋場所名。 |

**產出**：呼叫 `db.BuildIdentity(database, targetID)` 即可取得該 NPC 的 identity 字串。

---

### 1.2 Talk 分支帶入背版

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 1.2.1 | 在 `handleDoAction` 的 `case "Talk":` 中，在呼叫 `buildTalkNarrative` 前加：`npcBackstory := db.BuildIdentity(database, targetID)`（僅當 target.Kind=="npc" 時有意義） | `server/handler.go` | 若 target 為 NPC 則傳入背版。 |
| ⬜ | 1.2.2 | 修改 `buildTalkNarrative` 簽名：新增參數 `npcBackstory string`，即 `buildTalkNarrative(playerID string, target *entity.Character, personality *db.Personality, npcBackstory string) string` | `server/handler.go` | 保留既有參數順序，最後加 npcBackstory。 |
| ⬜ | 1.2.3 | 在 `buildTalkNarrative` 內：若 `npcBackstory != ""`，可將該字串用於組敘事（例如當作「角色設定」前綴，目前若仍為固定 8 句選句則可先僅傳入不顯示，或接下一階段給 CallAITalk 用） | `server/handler.go` | 階段 1 可只傳入、不在玩家可見敘事中直接拼字，預留給 AI 用。 |
| ⬜ | 1.2.4 | 呼叫處改為 `buildTalkNarrative(c.PlayerID, target, p, npcBackstory)` | `server/handler.go` | 完成接線。 |

**產出**：每次 Talk 都能取得並傳入該 NPC 的 identity；後續 CallAITalk 可直接使用此字串。

---

### 1.3（可選）007 設定項與 CallAITalk 插槽

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 1.3.1 | 在 `config.Server` 或獨立設定結構增加：`UseAIForTalk bool`、`AINPCIDs []string`、`AIFallbackOnError bool`；若無則先以常數或環境變數讀取 | `config/config.go` 或新建 `config/ai.go` | 對應 007 §3。 |
| ⬜ | 1.3.2 | 新增 `ai.CallAITalk(playerInput string, npcBackstory string, npcMemorySnippets []string, npcContext map[string]string) (reply string, err error)`；內部先 `return "", errors.New("not implemented")` 或直接 fallback | **新建** `ai/talk.go`（或 `server/ai.go`） | 介面先存在，handler 可依設定呼叫。 |
| ⬜ | 1.3.3 | 在 Talk 分支：若 `cfg.UseAIForTalk` 且（`len(cfg.AINPCIDs)==0` 或 `targetID` 在名單內），則呼叫 `ai.CallAITalk(玩家輸入, npcBackstory, nil, context)`；若 err≠nil 或 reply 空則 fallback 現有 `buildTalkNarrative` | `server/handler.go` | 完成 007「何時打 AI、失敗 fallback」。 |

**產出**：設定開關與 CallAITalk 插槽就緒；背版字串可傳入 AI，記憶片段階段 2 再填。

---

## 三、階段 2：archival 儲存、寫入、檢索

### 3.1 儲存結構與持久化

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 3.1.1 | 定義單條記憶結構：`ArchivalEntry { EntityID, Content, Tag, CreatedAt }`；Tag 可為 `fact`／`preference`／`event`／`player_xxx` | **新建** `db/archival.go` 或 `model/archival.go` | 與設計 doc §四對齊。 |
| ⬜ | 3.1.2 | 決定儲存後端：**A** 單一 JSON 檔 `data/npc_archival.json`（陣列，依 entity_id 分區查詢）；**B** 擴充 store：`Archival []ArchivalEntry`，啟動時載入、寫入時 append 並寫回 JSON | `store/store.go` 或新建 `data/npc_archival.json` + 讀寫函式 | 建議先 A：一檔一表，實作簡單。 |
| ⬜ | 3.1.3 | 若選 A：新增 `db/archival.go`，內有 `LoadArchival(path string) ([]ArchivalEntry, error)`、`SaveArchival(path string, entries []ArchivalEntry) error`；path 預設 `data/npc_archival.json` | `db/archival.go` | 讀寫分離，方便測試。 |
| ⬜ | 3.1.4 | 若選 B：在 store 增加 `Archival []ArchivalEntry`、`archivalPath`；`loadArchival()`、`AppendArchival(entry)` 並寫回檔案 | `store/store.go` | 與現有 entities/assignments 模式一致。 |

**產出**：可依 entity_id 取得該 NPC 全部記憶條目、可 append 新條目並持久化。

---

### 3.2 寫入 API（依 entity_id）

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 3.2.1 | 定義 `InsertArchival(entityID, content, tag string) error`：組 `ArchivalEntry`、append 至儲存、寫回 | `db/archival.go` 或 store | tag 可選，空則 `""` 或 `"event"`。 |
| ⬜ | 3.2.2 | 若使用 store：`store.Default.AppendArchival(entry)` 並在 AppendArchival 內寫回 `data/npc_archival.json` | `store/store.go` | 需處理併發（mu）。 |
| ⬜ | 3.2.3 | 節流：同一 entity_id 在「同一場對話」內寫入上限 M 條（如 M=5）；可在 handler 或專門的「對話 session 快取」中計數 | `server/handler.go` 或 `server/talk_session.go` | 對應對話記憶系統 §7.4。 |

**產出**：Talk 結束或本輪結束時可呼叫 `InsertArchival(entityID, content, tag)` 寫入 1～3 條。

---

### 3.3 檢索 API（依 entity_id + query → top-k）

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 3.3.1 | 定義 `SearchArchival(entityID string, query string, topK int) ([]string, error)` 回傳該 NPC 的記憶內容字串 slice（最多 topK 條） | `db/archival.go` | 介面先定。 |
| ⬜ | 3.3.2 | **簡易版（無 embedding）**：該 entity_id 全部條目依 `CreatedAt` 倒序取前 topK 條，或依 query 關鍵字過濾（strings.Contains）後再取 topK | `db/archival.go` | 階段 2 可先上線。 |
| ⬜ | 3.3.3 | **語意版（可選）**：對每條 `Content` 做 embedding；query 做 embedding；用餘弦相似度排序取 topK；需 embedding 模型（如本地 bge-m3）與向量儲存（可另一 JSON 或內存 map） | 新建 `embedding/embed.go`、擴充 `db/archival.go` 或 `db/archival_search.go` | 設計 doc、007 提及可本地。 |
| ⬜ | 3.3.4 | 在 Talk 分支（或 BuildIdentity 之後）：`snippets, _ := db.SearchArchival(database, targetID, playerInput, 5)`；將 snippets 傳入 `buildTalkNarrative` 或 `ai.CallAITalk(..., snippets, ...)` | `server/handler.go` | 完成「Talk 前檢索 → 注入 context」。 |

**產出**：Talk 前能依 entity_id 與玩家輸入（或最近一句）取回 top-k 記憶並帶入 prompt。

---

### 3.4 Talk 結束寫入記憶（consolidation 簡版）

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 3.4.1 | 在 `handleDoAction` 的 Talk 成功回覆後，呼叫「本輪寫入」：例如 `AfterTalkWriteMemory(database, c.PlayerID, targetID, playerInput, narrative)` | `server/handler.go` | 可抽成獨立函式。 |
| ⬜ | 3.4.2 | `AfterTalkWriteMemory` 內：以規則組 1 條內容（例如「玩家說：{playerInput}；NPC 回：{narrative}」縮寫），或呼叫簡單 LLM 抽取一句「要旨」；然後 `InsertArchival(targetID, content, "event")` | `server/handler.go` 或新建 `server/talk_memory.go` | 階段 2 可先用規則寫 1 條，不呼叫 LLM。 |
| ⬜ | 3.4.3 | 節流：同一 session（同 player + 同 NPC）本場最多寫 M 條（如 5）；超過則只累積不寫，或對話結束時再合併成 1 條 | `server/handler.go` 或 talk_session | 對應 §7.4 寫入節流。 |

**產出**：對話結束或本輪結束時可寫入 1～3 條至該 NPC 的 archival；再次 Talk 時能被檢索到（驗收 記憶2、記憶3）。

---

## 四、階段 3：blocks 可更新（summary／relationship）

| 狀態 | 步驟 | 動作 | 檔案 | 說明 |
|:----:|------|------|------|------|
| ⬜ | 4.1 | 定義背版 blocks 儲存：`BackstoryBlocks { EntityID, Identity, Summary, Relationship }`；Identity 可繼續由 BuildIdentity 算出或改為從此讀取 | `db/backstory.go` 或 `model/backstory.go`、`data/npc_backstory_blocks.json` | 與設計 doc §三對照。 |
| ⬜ | 4.2 | 實作 `GetBackstoryBlocks(entityID) *BackstoryBlocks`、`UpdateBackstoryBlock(entityID, blockName, content string)` | `db/backstory.go`、store 或 JSON 讀寫 | 讀取時 Identity 可覆寫為 blocks 內值（若有）。 |
| ⬜ | 4.3 | 觸發更新：每 N 次對話或定時，將該 NPC 的 archival 最近若干條交給 LLM 產出 summary／relationship 摘要，再呼叫 UpdateBackstoryBlock | 新邏輯在 main 週期或 Talk 計數後 | 設計 doc 階段 3。 |
| ⬜ | 4.4 | Talk 組 prompt 時改為：背版 = GetBackstoryBlocks 的 Identity + Summary + Relationship（若無則 Summary/Relationship 空）；其餘同階段 2 | `server/handler.go` | 背版「會長大」。 |

---

## 五、檔案清單與生成／修改順序

### 新建檔案

| 狀態 | 序 | 檔案 | 時機 |
|:----:|---:|------|------|
| ⬜ | 1 | `db/backstory.go` | 階段 1.1 |
| ⬜ | 2 | `db/backstory_test.go` | 階段 1.1 |
| ⬜ | 3 | `ai/talk.go`（或 `server/ai.go`） | 階段 1.3（可選） |
| ⬜ | 4 | `db/archival.go`（及可選 `model/archival.go`） | 階段 3.1 |
| ⬜ | 5 | `data/npc_archival.json` | 階段 3.1（首次寫入時建立，可為 `[]`） |
| ⬜ | 6 | `server/talk_memory.go` 或 `server/talk_session.go` | 階段 3.4（可選，邏輯也可放在 handler） |
| ⬜ | 7 | `embedding/embed.go`（語意檢索時） | 階段 3.3 語意版 |
| ⬜ | 8 | `data/npc_backstory_blocks.json` | 階段 4 |

### 修改既有檔案

| 狀態 | 序 | 檔案 | 改動要點 |
|:----:|---:|------|----------|
| ⬜ | 1 | `server/handler.go` | Talk 分支：BuildIdentity、SearchArchival、AfterTalkWriteMemory、可選 CallAITalk；buildTalkNarrative 增參 npcBackstory[, snippets] |
| ⬜ | 2 | `config/config.go`（或 `config/ai.go`） | UseAIForTalk、AINPCIDs、AIFallbackOnError |
| ⬜ | 3 | `store/store.go` | 若 archival 放 store：Archival、loadArchival、AppendArchival、archivalPath |

### 依賴關係（建議實作順序）

```
1.1 db/backstory.go + BuildIdentity
    → 1.2 handler Talk 帶入 npcBackstory
    → 1.3（可選）config + ai.CallAITalk stub

3.1 db/archival.go + 儲存結構 + Load/Save 或 store 擴充
    → 3.2 InsertArchival
    → 3.3 SearchArchival（簡易版）
    → 3.4 handler AfterTalkWriteMemory + 節流
    → 3.3 語意版（可選）
```

---

## 六、與可驗收子項對照

| 狀態 | 驗收項 | 對應步驟 |
|:----:|--------|----------|
| ⬜ | **記憶1**：Talk 前能依 entity_id 取回背版＋top-k 記憶 | 1.1 BuildIdentity、3.1～3.3 儲存與 SearchArchival、1.2／3.3.4 handler 接線 |
| ⬜ | **記憶2**：對話結束能將 1～3 條寫入該 NPC 的 archival | 3.1～3.2 儲存與 InsertArchival、3.4 AfterTalkWriteMemory |
| ⬜ | **記憶3**：同一 entity_id 再次 Talk 能檢索到前次寫入 | 3.3 SearchArchival、3.4 寫入；端到端測試 |

---

*奇點世界專案 — NPC 對話記憶與背版實作步驟 v1*

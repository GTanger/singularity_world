# 記憶系統對照 NPC 活化系統

> 對齊：**NPC 活化系統**第四階段「有記憶」、**對話記憶系統**設計三階段、**突破線 G–I** 與目前程式實作。  
> **活化總綱（無最終版／本地 LLM 可換）**：見 [NPC活化系統.md](../NPC活化系統.md) §零點五。

---

## 一、對照總表

| NPC 活化系統（第四階段） | 對話記憶系統設計 | 突破線 | 實作狀態 | 備註 |
|--------------------------|------------------|:-----:|----------|------|
| **背版（identity）只讀** | 階段 1：固定背版 | **G** | ✅ | `db/backstory` BuildIdentity；Talk 前帶入 |
| **archival 儲存＋寫入** | 階段 2：archival 寫入 | **H** | ✅ | **對話結束 consolidation**：逾時 2 分鐘視為一場結束，整場壓成 1～3 條寫入；節流＋每 NPC 上限 100 條 |
| **archival 檢索（top-k）** | 階段 2：檢索注入 | **H** | ✅ | SearchArchival：**多關鍵字評分**（query 拆詞，命中越多越前），無 query 取最新；**玩家↔NPC Talk** 用 `SearchArchivalForPlayerTalk`：寒暄等零命中時**不** fallback 到無關最新條，避免污染 prompt |
| **可驗收：Talk 前背版＋top-k** | 讀取流程 | **G+H** | ✅ | handler：BuildIdentity + SearchArchivalForPlayerTalk → CallAITalk／雲端 Talk |
| **可驗收：結束寫入 archival** | 寫入時機 | **H** | ✅ | 逾時 2 分鐘後下一句 Talk 時，將上一場 1～3 條寫入（`server/conversation_buffer`） |
| **可驗收：再次 Talk 能檢索到** | 檢索驗收 | **H** | ✅ | 同一 entity_id 再 Talk，多關鍵字可帶出相關記憶 |
| **節流（本場上限 M 條）** | 寫入節流 | — | ✅ | consolidation 每場最多 3 條；另每 NPC 每 10 分鐘最多 3 條、總量上限 100 條 |
| **blocks 可更新（選做）** | 階段 3：summary | — | ✅ | **與玩家的最近印象**：consolidation 時寫入 `npc_summaries.json`，BuildIdentity 帶入背版 |
| **語意檢索** | 階段 2 進階 | — | ⬜ | 目前多關鍵字評分；embedding＋向量未做 |

---

## 二、活化系統中的位置

- **突破線 G–I**（實作清單）：**G** 背版組裝、**H** archival 記憶（寫入＋檢索）、**I** CallAITalk 接入 → **均已標為 ✅**。
- **主文件** `NPC活化系統.md` §五 第四階段：表內仍為 ⬜，與實作清單不一致；**實際程式已達成**「背版只讀、archival 存／取、Talk 前 top-k、會後寫入、節流與 per-NPC 上限」。

---

## 三、記憶流程與檔案對應

| 步驟 | 活化／記憶對應 | 檔案／函式 |
|------|----------------|------------|
| Talk 前讀背版 | 第四階段「背版只讀」、G | `db/backstory` BuildIdentity |
| Talk 前檢索記憶 | 第四階段「archival 檢索 top-k」、H | `db/archival` **SearchArchivalForPlayerTalk**（Talk 專用；一般檢索仍為 SearchArchival）；store GetArchivalByEntity |
| 組 prompt 送 LLM | 第四階段「可驗收 1」、I | `ai/talk` CallAITalk(snippets)；handler 傳 backstory＋snippets＋styleExamples |
| 回覆後寫入 | 第四階段「archival 儲存＋寫入」、H | `server/conversation_buffer` FlushConversationAndAppend；逾時 2 分鐘整場壓 1～3 條 → InsertArchival；並 SetNpcSummary |
| 持久化 | 6.3 記憶相關 | `npc_archival.json`、`npc_summaries.json`；store load/persist |

---

## 四、與設計差異（尚未實作）

| 設計／文件 | 現況 |
|------------|------|
| **語意檢索**：embedding + 向量、query 語意相似 | 已實作**多關鍵字評分**（拆詞命中數排序）；embedding＋向量未做 |

**體驗驗證**：與同一 NPC 連續對話數輪 → 隔 2 分鐘再 Talk，應寫入 1～3 條精華；再次對話時背版出現「與玩家的最近印象：最近聊過：…」；輸入多個詞（如「錢 裝備」）時檢索會優先帶出同時命中多詞的記憶。

---

## 五、相關文件

- [NPC 活化系統](../NPC活化系統.md) §五 第四階段、§六 6.3
- [NPC 活化系統—實作清單與實作計畫](../implementation/NPC活化系統—實作清單與實作計畫.md) 突破線 G–I、H 實作狀況
- [NPC 對話記憶與背版—設計](../implementation/NPC對話記憶與背版—設計.md) 階段 1～3
- [對話記憶系統—彙整與探討](對話記憶系統—彙整與探討.md) 共識與流程

---

## 六、NPC 間對話與活化系統對照（003）

| 活化系統位置 | 003 NPC 交互對話系統 | 實作狀態 | 備註 |
|--------------|----------------------|:--------:|------|
| **突破線 F（微互動）** | 同房兩 NPC 敘事 | ✅ | F 保留為 **fallback**：Ollama 未配置或 CallAITalkNPCToNPC 失敗時，仍用 `PickMicroInteraction` 發微互動句 |
| **第五階段「NPC 之間互動」** | 觸發（閒置／排班／隨機）、AI 一來一往、主題劇本、NpcNpcSummaries | ✅ | 見 [003](discussions/003_NPC交互對話系統.md) §十 實作確認清單；main `tryTriggerNpcNpcInRoom`、ai/talk `CallAITalkNPCToNPC` |
| **主迴圈** | 閒置 tick、排班時段有動靜的房、隨機 80～120 tick 一房 | ✅ | 與活化 §一 1.2 架構圖「每 5-12 秒：閒置動作 + …」對齊；NPC 間對話在同一迴圈內觸發 |
| **玩家優先** | 60s 內有 Talk 的房不觸發；生成後 15s 內有 Talk 不播 | ✅ | session.LastTalkAt、RoomHasPlayerWithRecentTalk；不佔玩家 LLM 權限 |
| **記憶** | 每對 NPC 一筆摘要（NpcNpcSummaries），觸發前讀、完成後寫 | ✅ | 與第四階段「archival」分開：玩家↔NPC 用 archival；NPC↔NPC 用 NpcNpcSummaries（store + npc_npc_summaries.json） |

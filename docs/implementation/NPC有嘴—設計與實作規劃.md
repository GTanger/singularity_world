# NPC 有嘴 — 設計與實作規劃

> 對齊：[NPC活化系統](../NPC活化系統.md) §五 第三階段、[實作清單](NPC活化系統—實作清單與規劃.md) I2/I3、[三軸推導性格](三軸推導性格—實作規劃.md) §3、[NPC生成流程調整](NPC生成流程調整—依討論001.md)、[決策 007](../decisions/007_NPC_AI_API與預設使用規則.md) §5.4.1。

---

## 收斂摘要（設計共識）

以下為嘴巴（Talk）設計的收斂結論，實作時以本摘要＋各節細節為準。

| 面向 | 共識 |
|------|------|
| **目標** | NPC 能「說話」：先接對話模板（I3），可選接對話模型（CallAITalk）；與記憶（第四階段）接同一條 Talk 流程。 |
| **UI** | 點 Talk → 顯示**可對話主題**數項（如「你好」「有什麼可以幫忙的？」等）＋最後一項**「其他」**；點「其他」才開輸入框。每次送出皆為**一句話**（主題或自輸入）。 |
| **後端優先** | **一律優先對話模型**：有 `player_input` 且 use_ai_for_talk 且該 NPC 允許 → CallAITalk(player_input, 背版, 記憶)；失敗／超時／未啟用 → fallback 模板（I3 抽句）。 |
| **協定** | Talk 的 do_action **必帶** `player_input`（或 `text`）；後端依此一句話決定回覆。 |
| **模板與 AI** | 同一套 dialogues 池：預設抽句（I3）與 AI 提示＋fallback 共用；CallAITalk fallback 時用 PickFromDialogue 抽一句。 |
| **實作順序** | 1）協定加 player_input；2）載入 dialogue、PickFromDialogue；3）handler 優先 CallAITalk、fallback I3；4）前端可對話主題＋「其他」；5）可選 talk_topics 設定、關鍵字檢索。 |

**未決／可選**：可對話主題清單由前端寫死或後端提供；關鍵字檢索第一版可不做；use_ai_for_talk_click_only 在本設計下可不使用。

---

## 〇、實作順序：先人後嘴

**要生嘴巴前，得先有人。** 世界裡若沒有可點擊的 NPC（實體在房間內），玩家就沒有【對話】對象，Talk 串接對話池（I3）無從驗收。

| 步驟 | 說明 |
|------|------|
| **先有人** | 世界裡至少要有**可對話的實體**：在若干房間內存在 NPC，玩家進房後人物欄看得到、點得到。 |
| **再生嘴** | 再實作 I3（載入 dialogues、PickFromDialogue、handler 用模板回覆）、可對話主題、多輪延續等。 |

**「生人」可採任一方式，不必等完整「模板 NPC 生成器」：**

- **最小種子**：在 `defaultNPCs` 放 1～2 筆，`SeedNPCs` 啟動時建立並放進某房（如 lobby 或浮生客棧）；有指派時可掛職業對話，無指派用公版。  
- **手動／腳本**：手動或寫小腳本呼叫 `InsertNPC` + `SetEntityRoom`（+ 可選 `InsertAssignment`），把實體寫入 store/entities 與 entity_room。  
- **模板生成器**：日後再做「讀 archetypes → 批量生成 NPC」；I3 前用種子或手動即可。

與 Antigravity 分工：對方生成對話池（dialogues）；本端**先確保世界有人**，再接上對話池（有嘴）。

---

## 一、目標與階段項目

**目標**：讓 NPC 能「說話」——Talk 有內容、有口吻；先接對話模板，再可選接 AI（CallAITalk），與記憶（第四階段）接在同一條 Talk 流程上。

| 項目 | 狀態 | 效果 |
|------|:----:|------|
| Talk 固定句＋性格權重（I2） | ✅ | 8 句固定池＋Boldness 偏移；已實作於 buildTalkNarrative |
| **Talk 串接對話模板（I3）** | ⬜ | 玩家點 Talk → 從 dialogues/*.json 依職業＋key 抽句 |
| Trade 交易流程 | ⬜ | 出價→議價→成交/拒絕 |
| 模板 NPC 生成器 | ⬜ | 讀 archetypes → 批量生成 NPC |
| NPC 喊價（trade_announce） | ⬜ | NPC 主動在 log 喊句 |

**建議下一步**：**先有人**（最小種子或手動放 NPC）→ **再 I3**（Talk 串接模板）→ 再排 Trade。

---

## 二、資料來源與掛載

| 項目 | 說明 |
|------|------|
| **對話來源** | `data/templates/dialogues/*.json`（依職業）；無職業或無命中時 fallback `data/templates/dialogues/public_dialogue.json`（公版，已建）。 |
| **職業從哪來** | `entity_id` → assignments → `occupation_id` → `occupations.json` 的 `dialogue_file` → 載入該職業的 dialogue 檔。 |
| **現況** | `dialogues/*.json` 已有 ~716 句（10 職業），**Go 尚未讀取**；目前 Talk 只吃 handler 內 8 句。`GetNPCTitle` 已自 assignments 推導職稱（E3 ✅）。 |

**載入時機**：啟動時載入 `occupations.json`、各職業 dialogue 檔（或第一次 Talk 時 lazy load）；需新模組或擴充 `db/behavior.go`／新檔讀取 templates。

---

## 三、Talk 抽句規則（I3）

### 3.1 key 與情境

| key | 用途 | 備註 |
|-----|------|------|
| `greet` | 玩家進房招呼 | 目前進房反應已用 npc_behaviors；Talk 點擊時可視為「主動交談」 |
| `talk` | 玩家點 Talk 時的主回應 | **I3 預設**：點 Talk 用此 key |
| `idle` | 閒置敘事 | 已有 npc_behaviors idle；若改從 dialogues 取可依時段子 key（morning/noon/evening/night） |

第一版 I3 可只實作 **talk** key；greet 留給「進房反應改從 dialogues 取」時再用。

### 3.2 性格權重（三軸推導性格 §3.2）

| 維度 | 規則 |
|------|------|
| **Boldness** | lines 前半＝溫和、後半＝強勢；加權隨機使高 Boldness 偏大 index（與現有 8 句邏輯一致）。 |
| **Sensitivity**（可選） | 高 → 偏較長句或情緒較多句；可依 `len(line)` 或預標 `weight_tier`。 |
| **Orderliness**（可選） | 高 → 偏正式；可為每句加 `"tone": "formal"|"casual"` 或依 index 分區。 |

實作介面：**PickFromDialogue(occupationID, key, personality *Personality) string**，內部依 Personality 權重抽一句；替換佔位符 `{name}`、`{goods}` 等後回傳。

### 3.3 佔位符

| 佔位符 | 替換為 |
|--------|--------|
| `{name}` | NPC 個體名（如 target.ID） |
| `{goods}` | 販賣品類（可自 archetypes／behaviors 取，或暫用「雜貨」） |

---

## 四、與 AI 的關係（007 §5.4.1）

- **同一套對話池、兩種用途**：  
  - **預設**：I3 用 `PickFromDialogue(occupation, key)` 抽一句回傳。  
  - **AI**：從同一 dialogue 檔抽 4～6 句（如 greet＋talk）當 **system／few-shot**；fallback 時再用同一套 PickFromDialogue 抽一句，玩家感受不到斷層。
- CallAITalk 的 fallback 與 I3 共用同一套抽句邏輯。

---

## 五、Talk UI 與後端優先順序（加入 AI 後）

**原則**：預設列出數項**可對話主題**，最後一項為**「其他」**，點「其他」才開輸入框；**無論點擊主題或自行輸入，一律優先接對話模型**（CallAITalk），失敗或未啟用才 fallback 模板。

### 5.1 UI 設計

| 元素 | 說明 |
|------|------|
| **可對話主題** | 點 Talk 後顯示數項預設句子，例如：「你好」「有什麼可以幫忙的？」「這裡有什麼賣？」「附近有什麼好去的？」等；可依 NPC 職業或場所調整清單。 |
| **其他** | 清單**最後一項**為「其他」；點擊後**開啟對話輸入框**，玩家輸入一句再送出。 |
| **送出的內容** | 點主題 → 送該句當 `player_input`；點「其他」並輸入 → 送輸入內容當 `player_input`。亦即**每次 Talk 都帶一句話**（主題或自輸入），沒有「純點擊無內容」。 |

### 5.2 後端優先順序

| 順序 | 條件 | 行為 |
|------|------|------|
| 1 | `use_ai_for_talk` 且該 NPC 允許（如 ai_npc_ids 空或在其內） | **優先 CallAITalk(player_input, 背版, 記憶, ...)** |
| 2 | API 失敗、超時、未啟用或未帶 player_input | **Fallback 模板**（I3 抽句或關鍵字選 key）；與 007 一致。 |

亦即：**點擊主題 = 送該句 → 先走對話模型；自行輸入 = 送輸入 → 先走對話模型**。不再區分「只有輸入才走 AI」。

### 5.3 007 設定對應

- **use_ai_for_talk**：是否啟用「優先對話模型」；`true` 時一律先打 CallAITalk，失敗才模板。
- **use_ai_for_talk_click_only**：在本設計下**可不使用**或視為 `false`——因每次都有 `player_input`（主題或自輸入），沒有「純點擊無內容」情境。若保留，可解讀為「僅當有 player_input 才打 API」（與本設計一致）。

### 5.4 協定

- **ClientMsg**（`do_action` Talk）：**必帶** `player_input`（或 `text`）——點主題時為該句，點「其他」時為玩家輸入。後端一律視為「玩家說的一句話」。
- **ActionResultMsg**：仍回傳 `Narrative`（NPC 回覆）；前端可顯示「玩家：xxx」「NPC：yyy」。

### 5.5 可對話主題來源

- 可寫死在前端（依職業或場所切換不同清單），或由後端／設定檔提供（例如 `data/templates/talk_topics.json` 或依 dialogue 的 key 列出）。第一版可前端寫死 3～5 句＋「其他」。

### 5.6 多輪延續（第二輪起不需重複點 NPC）

**問題**：玩家對 NPC 發起第一輪對話、NPC 答覆後，若要求玩家「再點一次 NPC → 再點 Talk」才能說第二句話，體驗會很糟。

**共識**：**不需重新點擊 NPC、也不需再點 Log 裡的【對話】**。同一段對話應能連續多輪，直到玩家主動結束。

| 面向 | 設計 |
|------|------|
| **觸發延續的時機** | 每次 **Talk 的 action_result** 回傳並在 Log 顯示 NPC 回覆後，前端即視為「仍在與該 NPC 對話中」。 |
| **第二輪起怎麼操作** | 在 NPC 回覆**下方或同一區塊**，**再次顯示**「可對話主題」＋「其他」（或保留一欄輸入框／「繼續說」）。玩家直接選主題或輸入內容 → 前端送 **同一 entity_id** 的 `do_action Talk`＋`player_input`，即為第二輪、第三輪……。 |
| **對話對象維持** | 前端維護「當前對話對象」`entity_id`（即上一輪 Talk 的 target）。只要玩家在該脈絡下選主題或輸入，一律對該 entity_id 送 Talk；**不需**再從人物欄或 Log 的【對話】進入。 |
| **結束對話** | 玩家選擇「結束對話」、或點擊其他實體（Look/Talk 別人）、或移動離開、或關閉對話區時，清除「當前對話對象」，下次要聊需重新點該 NPC → Look → 【對話】。 |

**實作要點**：action_result（action 為 Talk）處理完後，除 append NPC 回覆到 Log 外，**同時**在 Log 內或固定對話區再次渲染「可對話主題＋其他」或輸入框，並把該次 target_id 存為「當前對話對象」；之後玩家在此區的輸入／點選皆對該 target 送 Talk，直到結束。

### 5.7 與記憶的接點

- 每次 Talk 都有 `player_input` → 讀背版＋檢索 top-k 記憶 → 送 CallAITalk 或 fallback 模板；對話結束寫入 archival 時，`player_input` 與 NPC 回覆一併寫入。

### 5.8 為何需要延伸、如何延伸（不重複公版）

**問題**：與同一個 NPC 對話次數變多時，對話模型不能一直用公版／固定（類似）內容答覆，否則會像複讀機。

**做法**：**延伸靠背版＋archival，公版僅 fallback**。

| 角色 | 說明 |
|------|------|
| **公版／職業模板** | 僅在**未開對話模型**或 **CallAITalk 失敗／超時**時使用（PickFromDialogue 抽一句）；負責「保底」，不負責越聊越延伸。 |
| **背版＋archival** | 每次 Talk 前：把該 NPC 的背版＋**檢索到的 top-k 記憶**送進 CallAITalk → 模型看到「這個人是誰、和玩家發生過什麼」→ 回覆會延續、接任務、提過的事，不再重複公版句。每次 Talk 後：本輪精華寫入 archival，下次檢索得到。 |

因此「跟同一個 NPC 對話次數變多」的延伸機制 = 第四階段「有記憶」：背版＋archival＋檢索注入＋對話結束寫入。若記憶尚未實作，可先做「當場最近 N 輪」送進模型，讓同一場對話有延續；長期仍建議上 archival。

---

## 六、依賴與驗收

| 依賴 | 說明 |
|------|------|
| D3 | 對話模板 `data/templates/dialogues/*.json` 已存在 |
| E3 | 職稱來自指派（GetNPCTitle）；已有 |
| occupations | `data/templates/occupations.json` 需有 dialogue_file 對應（現有為經理/服務生，與 archetypes 的 merchant/blacksmith 等可能需對照） |

**I3 驗收**：玩家對有指派的 NPC 點 Talk → 回傳句來自該職業 dialogue 的 talk.lines，且依 Boldness 有偏移；無職業時 fallback 公共池或現有 8 句。

---

## 七、建議實作順序

| 序 | 步驟 | 說明 |
|----|------|------|
| 1 | 協定：Talk 必帶 player_input | ClientMsg 增加 `player_input`（或 `text`）；後端無則 fallback 模板或拒收。 |
| 2 | 載入 dialogue 檔 | 依 occupations 或 archetypes 的 dialogue_file 路徑載入 JSON；可新檔 `db/dialogue.go` 或擴充 behavior。 |
| 3 | 實作 PickFromDialogue(occupationID, key, personality) | 回傳一條替換過佔位符的句子；Boldness 權重先做，Sensitivity/Orderliness 可選。 |
| 4 | handler Talk：優先 CallAITalk、fallback I3 | 有 player_input 且 use_ai_for_talk 且 NPC 允許 → CallAITalk；否則 PickFromDialogue(..., "talk", personality)。 |
| 5 | 前端：可對話主題＋「其他」 | 點 Talk 顯示主題清單（3～5 句＋「其他」）；點主題送該句、點「其他」開輸入框再送。 |
| 6 | （可選）public_dialogue.json、talk_topics 設定 | 無職業 fallback；主題清單可改由後端／設定檔提供。 |
| 7 | （後續）關鍵字檢索 | 第一版可不做；若做則 player_input 與 talk.lines 做關鍵字匹配再抽句。 |

---

## 八、相關文件

| 文件 | 用途 |
|------|------|
| [NPC活化系統](../NPC活化系統.md) | §五 第三階段、§四 模板系統、§六 程式碼速查 |
| [NPC活化系統—實作清單與規劃](NPC活化系統—實作清單與規劃.md) | I2/I3、D3、E3、建議下一步 |
| [三軸推導性格—實作規劃](三軸推導性格—實作規劃.md) | §3 對話權重、Phase 3、dialogues 抽句細則 |
| [NPC生成流程調整—依討論001](NPC生成流程調整—依討論001.md) | 對話掛載、public_dialogue、載入來源 |
| [決策 007](../decisions/007_NPC_AI_API與預設使用規則.md) | §5.4.1 對話池＝預設＋AI 提示詞、fallback 一致 |
| [交易—從對話到交易與面板設計](交易—從對話到交易與面板設計.md) | 有交易標籤 NPC：從對話開啟交易、交易面板彈窗與手風琴設計 |
| [data/templates/README.md](../../data/templates/README.md) | 模板格式、佔位符、職業一覽 |

---

*奇點世界專案 — NPC 有嘴 設計與實作規劃 v1（已收斂）*

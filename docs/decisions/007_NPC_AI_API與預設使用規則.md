# 決策：NPC 何時接 AI API、何時用預設

**狀態**：已決斷  
**對齊**：討論 002（NPC 像真人玩家）、第一版可做清單 §十

---

## 1. 原則

- **預設**：所有 NPC、所有行為（含 Talk 的模板回覆）一律走**預設邏輯**（模板／規則／npc_behaviors）。
- **例外**：僅當同時滿足「當前是玩家對**單一 NPC** 的 **Talk**」「該 NPC 被允許使用 AI」時，才呼叫 AI API；**呼叫失敗、超時或未啟用時，一律 fallback 預設**。

---

## 2. 維度對照

| 維度 | 走 AI API | 走預設 |
|------|-----------|--------|
| **對象** | 玩家**正在互動**的那一個 NPC（例如本回合 Talk 的目標） | 其餘所有 NPC |
| **動作類型** | **Talk**（玩家輸入一句，等 NPC 回一句） | 閒置動作、進房反應、換班敘事、巡邏、決策（求職／採集／移動） |
| **身份／白名單** | （可選）僅 `ai_npc_ids` 內的 NPC 才允許打 API | 未列名或未設定則一律預設 |

---

## 3. 設定項（建議）

實作時可採設定檔或後端常數，例如：

| 設定 | 說明 | 建議預設 |
|------|------|----------|
| `use_ai_for_talk` | 是否對 Talk 使用 AI 回覆 | `false`（上線再開） |
| `use_ai_for_talk_click_only` | 若 `true`：僅「玩家自由輸入一句」才用 AI；點選預設選項仍走模板 | `false` |
| `ai_npc_ids` | 空＝所有 NPC 符合條件時都可打 API；有值＝僅列出的 entity_id 才打 API | `[]` |
| `ai_fallback_on_error` | API 失敗或超時時是否 fallback 預設模板 | `true` |

**何時接 API**：`use_ai_for_talk == true` 且（若 `ai_npc_ids` 非空則該 NPC 在名單內）且本動作為「玩家對該 NPC 的 Talk」。  
**其餘**：皆用預設。

---

## 4. 實作要點

- 在 **Talk** 分支（如 `handleDoAction` 的 `case "Talk"`）內做一次判斷：是否符合「用 AI」條件；符合則呼叫 AI API，否則走既有模板／關鍵字檢索。
- 呼叫 AI 時傳入：玩家輸入、NPC id／職稱／當前房間／近期情境（可選）；回傳約束為一句話或 JSON 欄位，便于 fallback。
- 超時與錯誤處理：一律 fallback 預設，不讓玩家卡在無回應。

---

## 5. 本地模型（8b / 12b / 14b）運用

007 只規定「何時打 AI API」「失敗 fallback」，**不綁定雲端或廠商**；本地小模型可作為同一個「AI API」後端。

### 5.1 後端抽象

- 實作一個 **`CallAITalk(playerInput, npcContext) → (reply string, err)`**，內部可接：
  - 本地推理服務（Ollama、llama.cpp server、vLLM、OpenAI-compatible 本地端等）
  - 或雲端 API（同一介面，僅換 endpoint / key）
- 仍由 §3 設定決定「是否打、對誰打」；打的就是上述抽象，成功回傳一句話，失敗則 err → fallback 預設。

### 5.2 模型選用策略

| 策略 | 說明 | 適用情境 |
|------|------|----------|
| **單一模型** | 設定一個 `ai_model`（如 `llama3.2:8b`），所有准用 AI 的 NPC 共用 | 開發／測試；或資源有限只跑一台 |
| **依 NPC 分級** | 重要 NPC（白名單或標記）用 14b，其餘 12b／8b；可設 `ai_model_14b`、`ai_model_8b` 或 per-NPC override | 關鍵角色品質優先、路人省算力 |
| **依負載切換** | 預設 14b；並發高或延遲過大時自動改用 12b／8b（需簡單延遲或 queue 偵測） | 單機多人在線、避免卡頓 |

### 5.3 建議設定項（擴充 §3）

| 設定 | 說明 | 建議預設 |
|------|------|----------|
| `ai_endpoint` | 本地或遠端 API 位址（如 `http://127.0.0.1:11434/api/chat`） | 依環境 |
| `ai_model` | 預設模型名稱（Ollama 為 tag，如 `llama3.2:8b`） | 依本機安裝 |
| `ai_model_by_tier` | （可選）`{ "default": "8b", "key_npc": "14b" }`，NPC 依 tier 或 id 對應 | 不設則全用 `ai_model` |
| `ai_timeout_sec` | 單次 Talk 呼叫逾時秒數 | `10`（本地可 5～15） |
| `ai_max_context_turns` | 傳給模型的最近對話輪數（小模型建議 1～2） | `2` |

### 5.4 Prompt 與小模型

- **系統提示**：簡短一句角色＋一句約束（如「你是飛霜大街的店員，只回一句話、不超過 30 字」），避免長篇世界觀塞滿 context。
- **輸入**：玩家當句＋（可選）最近 1～2 輪對話；其餘情境（房間名、職稱）可壓成一行。
- **輸出**：約束為「一句話」或單一 JSON 欄位，便于解析與 fallback。

### 5.4.1 對話池兼作 AI 提示詞

**同一套對話池、兩種用途**：`data/templates/dialogues/*.json`（各職業 greet／idle／talk／trade_announce 等）既可當 **預設抽句**（I3 模板 Talk），也可當 **AI 的提示素材**。

- **當提示詞**：依 NPC 職業選對應 dialogue 檔，從該檔任選數句（例如 greet＋talk 各 2～3 句）塞進 **system** 或 **user 前綴**，作為「語氣／口吻範例」或 few-shot，讓模型回覆貼近該職業；小模型 context 有限，只取 4～6 句即可。
- **Fallback 一致**：AI 失敗時從**同一池**抽句回傳，玩家感受不到斷層；實作上 CallAITalk 的 fallback 可與 I3 共用同一套 PickFromDialogue(occupation, key)。
- **好處**：不需維護兩套「角色口吻」；撰寫/擴充對話模板時，同時提升預設品質與 AI 表現。

### 5.5 與 007 的對應

- 「何時接 API」仍完全依 §1～§3：`use_ai_for_talk`、`ai_npc_ids`、僅 Talk。
- 本地 8b／12b／14b 只是 **CallAITalk 的實作後端**；失敗或超時仍 **fallback 預設**，不改變 007 原則。

### 5.6 範例：本機 Ollama 模型與 tier 對應

本機現有模型（`ollama list`）可對應到 §5.2 的 8b／12b／14b 分級，實作時用 **Ollama tag** 當 `ai_model` 或 `ai_model_by_tier` 的值。

| Tier | 建議 tag（擇一） | 備註 |
|------|------------------|------|
| **8b** | `llama3.1:8b` | 輕量、延遲低，適合路人／大量 NPC |
| **12b** | `mistral-nemo:12b`、`gemma3:12b` | 平衡品質與速度；Nemo 偏對話、Gemma 通用 |
| **14b** | `qwen3:14b`、`phi4:14b-q4_K_M` | 關鍵 NPC；Qwen3 中文佳、Phi4 推理穩 |

**單一模型**：例如 `ai_endpoint: "http://127.0.0.1:11434/api/chat"`、`ai_model: "llama3.1:8b"`，所有准用 AI 的 Talk 都走 8b。  
**分級**：`ai_model_by_tier: { "default": "llama3.1:8b", "key_npc": "qwen3:14b" }`，白名單內關鍵 NPC 用 14b，其餘 8b。

其餘本機模型（如 `qwen3.5:9b`、`deepseek-r1:14b`、`qwen2.5-coder:14b`）可依需求替換上表；embedding 模型（`bge-m3`、`nomic-embed-text`）留給 RAG／檢索用，不參與 Talk 回覆。

---

## 6. 背版與記憶（整合目標）

**目標**：每次與 NPC 對話都為該 NPC 累積**記憶點**，其**背版**（身份＋人設＋與玩家的關係）隨對話越長越生動、寫實、立體；最終整合進遊戲 Talk（模板或 AI）的 context。

- **CallAITalk 傳入**：除玩家輸入、職稱、房間、對話池例句外，可傳入該 NPC 的**背版**與**精選記憶**（語意檢索最近／相關幾條），使回覆有延續性與個人史。
- **每次對話後**：可將本輪精華寫入該 NPC 的長期記憶（archival）；背版可為固定區塊（identity／summary／relationship）或由記憶定期整理更新。
- **與對話池並存**：背版／記憶負責「這個人是誰、發生過什麼」；對話池負責口吻範例；兩者一起組 prompt。詳見 [NPC對話記憶與背版—設計](../implementation/NPC對話記憶與背版—設計.md)。

---

*奇點世界專案 — 決策紀錄*

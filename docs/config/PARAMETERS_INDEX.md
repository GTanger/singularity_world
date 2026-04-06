# 可調參數索引（目前專案實際有哪些）

**專案主索引**：[`../文檔索引.md`](../文檔索引.md)。

> 你只是要「概念」也沒問題：**已經搬進 JSON 的只有下面兩份**；其餘邏輯仍可能在 `src/config` 或環境變數裡。改 JSON 後通常要**重啟伺服器**。

---

## 1. `data/config/server_defaults.json`

**用途**：連線、時間刻度、地圖路徑、Ollama、部分 NPC↔NPC 與求職相關**伺服器級**預設。  
**程式**：`config.DefaultServer()`、`config.DesignConstants()`；環境變數（如 `PORT`、`OLLAMA_*`）仍可覆寫。

| 區塊 | 欄位（鍵名） | 大略意思 |
|------|----------------|----------|
| `design` | `cell_size_px`, `role_circle_px`, `terrain_font_px` | 前端格子／角色圓／地形字級像素 |
| `server` | `port`, `db_path` | HTTP 埠、SQLite 路徑 |
| | `max_websocket_conn` | 同時 WS 連線上限 |
| | `tick_interval_ms`, `economy_tick_interval_ms` | 主 tick、經濟 tick 間隔 |
| | `chunk_size`, `maps_path` | 地圖 chunk、地圖目錄 |
| | `session_retain_minutes` | session 保留分鐘 |
| | `game_time_scale` | 遊戲時間相對真實時間倍率 |
| | `npc_pool_size`, `npc_spawn_interval_sec` | NPC 池大小、生成間隔（秒） |
| | `ollama_disable` | `true`＝**關閉 Ollama**（NPC↔NPC、玩家走 Ollama 皆不呼叫）；**不影響** `player_talk_api_*` 雲端。環境變數 `OLLAMA_DISABLE=1` 同效 |
| | `ollama_base_url`, `ollama_model` | Ollama；`ollama_disable` 時不生效 |
| | `player_talk_api_base_url` | 玩家對 NPC **Talk** 專用：OpenAI 相容 API 根路徑，須以 `/v1` 結尾（例 `https://api.openai.com/v1`）。非空且 `player_talk_api_model` 非空時**優先**於 Ollama。環境變數 `PLAYER_TALK_API_BASE_URL` |
| | `player_talk_api_model` | 雲端模型 id（預設例：`qwen/qwen3.5-flash-02-23` @ OpenRouter）。環境變數 `PLAYER_TALK_API_MODEL` |
| | （無 JSON 鍵） | **金鑰**僅環境變數：`PLAYER_TALK_API_KEY` 或 `OPENAI_API_KEY`（Bearer）；勿寫入版本庫。可放專案根 `.env`（已 `.gitignore`）；`./start` 會在啟動 `bin/server-rust` 前自動載入 |
| | `seek_job_mg_threshold`, `job_match_when_stable` | 求職鎂門檻、穩定時是否配對 |
| | `npc_npc_quality_max_runes` 等 | NPC 對話品質字元上限、社交 tick 隨機範圍、對話分數門檻（0 常代表「用程式別處預設」） |

`game_time_epoch` **不在此檔**：見 `data/game_epoch.unix` 或 `GAME_TIME_EPOCH_UNIX`。

---

## 2. `data/config/simulation.json`

**用途**：主迴圈裡的**模擬／社交／傳聞／idle／巡邏**等數值（不含埠號與 Ollama）。  
**程式**：啟動時 `config.LoadSimulation("")`，執行時用 `config.Sim()`。

| 區塊 | 大略意思 |
|------|-----------|
| `max_assignments_per_venue` | 每場所求職／指派上限 |
| `room_event` | 房間事件數量上限、事件 TTL、關聯傳聞 TTL |
| `quality_gate_default_max_runes` | 對話品質預設字元上限 |
| `npc_npc_pair_pick` | 挑誰跟誰聊：thread 新鮮度、冷卻、分數雜訊、同場所加分、上次對話懲罰、熟悉度除數等 |
| `npc_npc_social` | 玩家剛聊過的冷卻、廣播略過時間、預設對話分數門檻、玩家八卦偏置、傳聞 top-k、近期房間事件窗 |
| `dialogue_anchor_rumor` | 從對話錨定成傳聞：最少字元、TTL、權重、來源分數 |
| `dyad_update` | 雙人關係：熟悉度增量、情緒上下限、吵架標籤門檻 |
| `brain_beg` | 大腦「乞討」鎂：最小值與隨機上界（exclusive） |
| `economy_pulse` | 經濟 pulse 觸發的遊戲日模數、經濟傳聞 TTL |
| `spawn_rumor_ttl_sec`, `job_match_rumor_ttl_sec` | 新生／求職配對傳聞過期秒數 |
| `rumor_decay_interval_sec`, `rumor_digest_interval_min` | 傳聞衰減間隔、摘要輪詢分鐘 |
| `idle` | 閒置 NPC 行為：首次觸發與間隔的 min+span（隨機範圍） |
| `random_npc_dialogue_ticks` | 隨機 NPC 對話 tick：初值與有玩家時重置範圍 |
| `job_matching_interval_sec`, `travel_tick_interval` | 求職輪詢、旅行 tick 間隔（秒） |
| `wander_roll_max`, `micro_interaction_chance_percent` | 巡邏骰上限、微互動機率 |
| `disposition_time_of_day` | 時段對性情修正：清晨／深夜小時範圍與 delta |

---

## 3. 文案／提示詞（不是「數值參數」，但也是專檔）

| 檔案 | 用途 |
|------|------|
| `data/config/gametext.json` | 遊戲字串、詞表、模板句 |
| `data/templates/llm_prompts.json` | 送進 LLM 的提示詞區塊 |

詳見 [`gametext_and_prompts.md`](./gametext_and_prompts.md)。

---

## 4. 還沒集中進 JSON 的

`db/`、`server/` 等套件裡可能仍有寫死的常數（例如批次上限、bcrypt cost）。若要**全部**可調，需要再開一輪掃描與設計；目前**沒有**保證「所有魔術數字」都在上面兩份 JSON。

---

## 快速找檔

```
data/config/server_defaults.json   ← 伺服器／設計／Ollama
data/config/simulation.json        ← 主迴圈與模擬行為
```

型別與預設值後備邏輯：`config/server_defaults`、`config/simulation`。

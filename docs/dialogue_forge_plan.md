# NPC 對話鍛造爐 (Dialogue Forge) — 三層模型實作計畫

## 背景

現有 NPC 後台對話系統已完整搬遷至 Rust，但 `ollama_model` 設定為空導致系統未啟用。用戶希望重啟 NPC 閒聊機制，並透過三層模型分工（0.8B 話癆 → 4B 日報提煉 → 雲端關鍵萃取）實現兩個目標：

1. **世界詞典**：NPC 對話中自發產出的地名、勢力、現象等詞彙，經群體投票沉澱後供地圖設計參考
2. **社會共識**：透過循環提煉逐步凝聚出「世界現象級」與「區域現象級」的輿情趨勢

## User Review Required

> [!IMPORTANT]
> 本計畫分 3 個 Phase 漸進實施。Phase 1 為基礎設施（啟用模型 + 事件種子 + 世界詞典表），是後續一切的前提。建議先跑 Phase 1 觀察一週效果，再決定是否推進 Phase 2/3。

> [!WARNING]
> Phase 3 需要雲端 API Key。請確認偏好的供應商（OpenAI / Anthropic / 其他）以及計費上限。

---

## Phase 1：基礎設施 — 啟用話癆 + 事件種子 + 世界詞典

### 配置啟用

#### [MODIFY] [server_defaults.json](file:///home/tanger/Projects/singularity_world/data/config/server_defaults.json)

設定 `ollama_model` 為 `sorc/qwen3.5-instruct:0.8b`，啟用 NPC 後台對話。

#### [MODIFY] [simulation.json](file:///home/tanger/Projects/singularity_world/data/config/simulation.json)

調整參數加速對話頻率：
- `random_npc_dialogue_ticks.initial_min`: `80` → `30`（加快首次觸發）
- `random_npc_dialogue_ticks.initial_span`: `40` → `20`
- `npc_npc_social.default_dialogue_score_threshold`: `35` → `25`（降低門檻，容忍更多產出）
- `npc_npc_pair_pick.pair_last_talk_penalty_sec`: `120` → `45`（允許同對 NPC 更頻繁對話）

---

### 事件種子注入器

#### [NEW] [event_seeds.json](file:///home/tanger/Projects/singularity_world/data/config/event_seeds.json)

約 30-50 條事件種子模板，按類別分組（天氣/經濟/安全/社會），例如：
```json
{
  "seeds": [
    { "category": "weather", "text": "城外傳來不明雷鳴，空氣中帶有臭氧味" },
    { "category": "economy", "text": "今早糧價漲了三成，攤販們議論紛紛" },
    { "category": "security", "text": "巡邏隊在霜林邊緣發現了可疑足跡" }
  ],
  "inject_interval_game_hours": 6
}
```

#### [MODIFY] [simulation_loop.rs](file:///home/tanger/Projects/singularity_world/src/server/simulation_loop.rs)

在主迴圈中新增事件種子注入邏輯：
- 每 N 個遊戲小時從種子池隨機抽取一條
- 注入至對應 zone 的 `recent_events`，同時寫入 rumor 系統
- 同一條種子 48 小時內不重複使用

---

### 世界詞典表 (World Lexicon)

#### [MODIFY] [sql.rs](file:///home/tanger/Projects/singularity_world/src/store/sql.rs)

新增 PostgreSQL 表 `world_lexicon`：
```sql
CREATE TABLE IF NOT EXISTS world_lexicon (
    term        TEXT PRIMARY KEY,
    category    TEXT DEFAULT '',          -- terrain/faction/place/phenomenon
    first_seen  BIGINT DEFAULT 0,
    last_seen   BIGINT DEFAULT 0,
    mention_count INT DEFAULT 1,
    unique_pairs INT DEFAULT 1,           -- 有多少不同的 NPC 對提過
    source_rooms TEXT[] DEFAULT '{}',     -- 出現過的房間
    status      TEXT DEFAULT 'candidate', -- candidate → nominated → confirmed
    confirmed_by TEXT DEFAULT ''          -- 'auto' 或管理員 ID
);
```

#### [NEW] [lexicon.rs](file:///home/tanger/Projects/singularity_world/src/db/lexicon.rs)

世界詞典 CRUD 模組：
- `upsert_lexicon_term(term, room_id, pair_key)` — 新增或更新詞條
- `promote_lexicon_candidates()` — 定期檢查晉升條件（≥3 對 NPC 提及 → nominated；≥5 次且跨 ≥2 zone → 通知管理員）
- `decay_lexicon()` — 48 小時無人再提 → 降權或淘汰

#### [MODIFY] [trigger.rs](file:///home/tanger/Projects/singularity_world/src/npcnpc/trigger.rs)

在對話成功後（現有 anchor 寫入邏輯之後），新增：
- 對每個 anchor 呼叫 `upsert_lexicon_term`
- 記錄 pair_key（`{a_id}|{b_id}`）用於統計獨立 NPC 對數

---

## Phase 2：日報提煉 (Daily Distillation)

> [!NOTE]
> Phase 2 依賴 Phase 1 跑出足夠資料後才有意義。建議 Phase 1 運行一週後再實施。

### 提煉引擎

#### [NEW] [distiller.rs](file:///home/tanger/Projects/singularity_world/src/ai/distiller.rs)

每遊戲日（或每 N 小時現實時間）觸發一次，使用 `sorc/qwen3.5-claude-4.6-opus:4b`：
- 從 `npc_archival` 讀取當天未提煉的對話
- 從 `world_lexicon` 讀取候選詞條
- 呼叫 4B 模型產出結構化日報（JSON）：新詞彙、趨勢話題、氛圍指數
- 日報寫入新表 `daily_digest`，向量化後存入 `pgvector`

#### [MODIFY] [config/mod.rs](file:///home/tanger/Projects/singularity_world/src/config/mod.rs)

新增 `distill_model` 設定欄位（獨立於 `ollama_model`），預設 `sorc/qwen3.5-claude-4.6-opus:4b`。

#### [MODIFY] [simulation_loop.rs](file:///home/tanger/Projects/singularity_world/src/server/simulation_loop.rs)

新增日報提煉計時器，週期性觸發 `distiller::run_daily_distillation()`。

---

## Phase 3：關鍵萃取 (Cloud Extraction)

> [!NOTE]
> Phase 3 需要雲端 API Key，且依賴 Phase 2 累積的日報資料。

### 雲端萃取引擎

#### [NEW] [cloud_extract.rs](file:///home/tanger/Projects/singularity_world/src/ai/cloud_extract.rs)

累積門檻觸發（非定時）：
- 條件：`world_lexicon` 中 `nominated` 詞條 ≥ 20，或連續 3 天日報出現跨 zone 同一話題
- 呼叫雲端 API，輸入：7 天日報 + 世界設定摘要 + 候選詞條清單
- 輸出：世界現象級事件建議、新區域設計草案、社會氛圍報告
- 結果寫入 `world_phenomena` 表，並通知管理員

#### [MODIFY] [config/mod.rs](file:///home/tanger/Projects/singularity_world/src/config/mod.rs)

新增雲端 API 設定：`cloud_api_provider`、`cloud_api_key`、`cloud_api_model`。

---

## Verification Plan

### Phase 1 驗證
- 啟動伺服器，確認 `ollama_model` 為 `sorc/qwen3.5-instruct:0.8b`
- 觀察 NPC 後台對話是否正常觸發（查看 `sw.log` 中的 `inc_stat` 輸出）
- 確認 `world_lexicon` 表有資料寫入
- 確認事件種子定期注入

### Phase 2 驗證
- 確認日報提煉使用 4B 模型而非 0.8B
- 檢查 `daily_digest` 表內容品質
- 驗證向量化結果正確存入 pgvector

### Phase 3 驗證
- 模擬累積門檻觸發
- 確認雲端 API 呼叫成功且結果合理
- 驗證管理員通知機制

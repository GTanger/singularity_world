# NPC 對話鍛造爐 Phase 1 — Walkthrough

## 概述

Phase 1 實施了三層 NPC 對話鍛造爐的基礎設施：啟用 0.8B 本地模型進行高頻 NPC 閒聊、建立事件種子注入機制防止語義坍縮、以及建立世界詞典追蹤 NPC 自發產出的詞彙。

## 變更摘要

### 配置啟用
- [server_defaults.json](file:///home/tanger/Projects/singularity_world/data/config/server_defaults.json)：`ollama_disable: false`，模型切換為 `sorc/qwen3.5-instruct:0.8b`，dialogue score threshold 設為 25
- [simulation.json](file:///home/tanger/Projects/singularity_world/data/config/simulation.json)：對話 tick 加速（30+20, 50+30），pair cooldown 從 120s 降至 45s

### 事件種子注入
- [event_seeds.json](file:///home/tanger/Projects/singularity_world/data/config/event_seeds.json)：30 條事件種子，分佈於天氣、經濟、安全、社會、異象 5 個類別
- [simulation_loop.rs](file:///home/tanger/Projects/singularity_world/src/server/simulation_loop.rs)：新增 `inject_event_seed()` 函式，每 10 分鐘注入一條種子至 rumor 系統，同一條種子 48 小時內不重複

### 世界詞典
- [sql.rs](file:///home/tanger/Projects/singularity_world/src/store/sql.rs)：新增 `world_lexicon` PostgreSQL 表
- [lexicon.rs](file:///home/tanger/Projects/singularity_world/src/db/lexicon.rs)：CRUD 模組（upsert / promote / decay / list）
- [trigger.rs](file:///home/tanger/Projects/singularity_world/src/npcnpc/trigger.rs)：每個 anchor 寫入 rumor 後同時寫入 `world_lexicon`
- 晉升機制：≥3 對 NPC 提及 → nominated；≥5 次且跨多房間 → promoted
- 衰減機制：48 小時無人再提的 candidate 自動刪除

## 驗證結果

| 項目 | 結果 |
|---|---|
| `cargo clippy -- -D warnings` | ✅ 通過 |
| `cargo test` | ✅ 通過（`src/` 單元測試；數量以 `cargo test` 輸出為準） |
| `checkrooms -strict` | ✅ 通過 |
| Git commit | ✅ `788bba3` pushed to origin/master |

## 下一步

- 觀察一週 NPC 對話產出效果
- 確認 `world_lexicon` 表有資料沉澱
- 決定是否推進 Phase 2（4B 日報提煉）

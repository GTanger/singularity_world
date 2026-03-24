# Go → Rust 後端遷移計畫

> 建立日期：2026-03-24  
> 決策依據：[決策 010](../decisions/010_go_to_rust_migration.md)  
> 狀態：**進行中**（Phase 0～1 已落地；Phase 2 部分落地；Phase 3 進行中 — `economy`／`game`／`combat`／`world` 主線已搬）

> **實際進度快照**（2026-02-12 對帳）：`src/**/*.rs` 合計 **8422** 行（`Cargo.toml` 34 行），已可 **`cargo check`**／**`cargo clippy -D warnings`**。✅ **已落地**：`store`／`db`、`game`、`world`、`combat`、`ai`、`event`、`gametext`、`npc`、`npcnpc`（types／state／helpers）、**`server` 之 `protocol`（WS JSON 型別）+ `session`（`SessionStore`／`sanitize_talk_snippet`）**。⏳ **仍待**：`server` 之 **HTTP／WebSocket handler／Hub**、`npcnpc` **`TryTriggerNpcNpcInRoom`**、Go `npc/` 決策主迴圈與 Go **`server/`** 其餘對齊。**上線與 `./start` 仍以 Go 為準**（見 [`docs/dev/README.md`](../dev/README.md)）。

---

## 一、遷移原則

1. **前端不動**：web/ 目錄零修改，Rust 後端提供完全相同的 HTTP/WebSocket API
2. **資料不動**：data/ 目錄的 JSON 結構不改，Rust 版本讀寫同樣的檔案
3. **API 契約不動**：WebSocket 訊息格式、HTTP 端點路徑、回應結構全部保持一致
4. **逐模組遷移**：不是一次重寫，而是按依賴順序逐層搬移
5. **每層都能編譯、每層都能測試**：不接受「先寫骨架再填邏輯」

---

## 二、模組對應表

**行數欄位**：**Go（參考）** = 該套件目錄內 `*.go` 合計且**排除 `*_test.go`**（遷移前規模）。**Rust（現況）** = 對應 `src/…` 之 `.rs` 行數；占位模組可能僅 1～2 行。數字會隨提交變動，更新本表請用文末命令對帳。

### 第零層：型別定義（無依賴）

| Go | Rust | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| model/ | src/model/ | 30 | 44 | Room、Exit 型別 |
| entity/ | src/entity/ | 66 | 138 | Character 等結構 |

### 第一層：設定與基礎（僅依賴第零層）

| Go | Rust | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| config/ | src/config/ | 551 | 316 | 伺服器參數、環境變數覆蓋 |
| event/ | src/event/ | 64 | 63 | `append`、`last_by_entity`、`mark_observed`、`events_in_range`（對齊 journal.go） |
| gametext/ | src/gametext/ | 418 | 413 | 文案載入與格式化 |

### 第二層：資料層（依賴第零、一層）

| Go | Rust | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| store/ | src/store/ | 2970 | 1593 | `InsertAssignment` 去重、`get_first_occupation_id_for_venue` 等（對齊 Go store） |
| db/ | src/db/ | 3448 | ~1450 | 含 **`text`／`npc_social`／`equip`／`npc_spawn`／`assignment`／`sched`／`npc_*`**；**`get_room`／`rune_lcs_similarity`／`upsert_npc_rumor`** 等 |

### 第三層：遊戲邏輯（依賴第零～二層）

| Go | Rust | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| game/ | src/game/ | 518 | 402 | `game_time_now`、`spawn_loop`；**`zone`／`ChunkView`／`view_sim`／`observe`**（對齊 `zone.go`、`chunk_view.go`、`view_sim.go`、`observe.go`） |
| combat/ | src/combat/ | 232 | 336 | `Resolve`、`ResolveV2`、`CombatOpt`、單元測試 |
| economy/ | src/economy/ | 28 | 20 | `run`、`transfer_magnesium` → db |
| npc/ | src/npc/ | 1735 | ~395 | **`topics`**：`load_npc_npc_topics`、`Find*`／`PickRandom*`／`debug_topic_weights_for_pair` + `db` 建立／種子／`equip` |
| npcnpc/ | src/npcnpc/ | 1202 | 669 | **`types`／`state`／`helpers`**；**`TryTriggerNpcNpcInRoom`** 仍待（依 `server`／Ollama） |
| ai/ | src/ai/ | 956 | ~1382 | `sanitize`、`prompts`、`talk`、`openai_chat`、**`scorer`**（多檔合計） |
| world/ | src/world/ | 216 | 522 | `Grid`、`load_chunk`、`can_move_to`、`terrain_from_rune`、`display_terrain`／`display_terrain_rng`、`TerrainMeta` |

### 第四層：對外介面（依賴全部）

| Go | Rust | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| server/ | src/server/ | 3607 | ~653 | **`protocol` + `session` 已搬**；HTTP/WS、Hub、room editor API 仍待 |
| main.go + 根層 *.go | src/main.rs + lib.rs | 753 | 26 | 入口 9 + `lib.rs` 17；Go 為專案根目錄非 test 之 `*.go` 合計 |

### 工具（獨立，可最後處理或保持 Go）

| Go | 處理方式 | Go（參考） | Rust（現況） | 說明 |
|---|---|---:|---:|---|
| cmd/checkrooms + internal/roomcheck | 遷移或維持 Go | 377 | — | 房間 JSON 契約檢查（`check.go`+`main.go`+`check_test.go`） |
| cmd/sw-set-password | 遷移 | 48 | — | 密碼設定工具 |

**對帳命令（專案根）**：

```bash
# Go：單套件、排除測試（例：db）
find db -maxdepth 1 -name '*.go' ! -name '*_test.go' -print0 | xargs -0 wc -l | tail -1

# Rust：單模組（例：store）
wc -l src/store/mod.rs

# Rust：全 src
find src -name '*.rs' -print0 | xargs -0 wc -l | tail -1
```

---

## 三、遷移順序

### Phase 0：專案骨架

- 初始化 Cargo workspace
- 建立 `Cargo.toml`，引入 axum、tokio、serde、serde_json、tokio-tungstenite
- 確認 `cargo check` 通過空專案
- 建立目錄結構對應

驗收：`cargo check` 通過，目錄結構存在。

---

### Phase 1：型別定義 + 設定（第零、一層）

遷移 model、entity、config、event、gametext。

這一層的重點：
- 把 Go 的字串狀態全部改為 Rust enum（EntityKind、MoveState、Gender、EventType 等）
- 用 serde derive 確保 JSON 序列化與現有格式完全相容
- 每個 enum 都必須 `#[serde(rename_all = "snake_case")]` 對齊現有 JSON

驗收：
- `cargo check` 通過
- 寫單元測試：讀取現有 `data/config/` JSON 能正確 deserialize
- 所有 enum 有窮舉測試

---

### Phase 2：Store + DB（第二層）

遷移 store 和 db。這是最大的單一工作量（合計 6418 行）。

重點：
- store 用 `Arc<RwLock<Store>>` 取代 Go 的全域 `Default` + `sync.RWMutex`
- 所有 JSON 讀寫用 serde，型別明確
- **所有錯誤必須用 `Result<T, E>` 傳播**——這是遷移的核心價值所在
- db 層的每個函數回傳 `Result`，呼叫端必須處理

驗收：
- `cargo check` 通過
- 讀取現有 `data/rooms/editor/` 全部 598 間房間無錯誤
- 讀取現有 `data/entities.json`、`data/runtime/*` 無錯誤
- store 初始化失敗時，明確報錯而非靜默跳過
- **已補**：整合測試 `tests/store_init_integration.rs` — 執行 `cargo test --test store_init_integration`（需專案根含完整 `data/`）驗證 `store::init(data/rooms, data/runtime, data)` 成功且房間數 ≥ 598

---

### Phase 3：遊戲邏輯（第三層）

遷移 game、combat、economy、npc、npcnpc、ai。

重點：
- NPC 決策意圖用 enum，`match` 必須窮舉
- AI/LLM 呼叫的 HTTP client 用 reqwest（tokio 相容）
- 戰鬥結算的 ResolveV2 邏輯 1:1 對應

驗收：
- `cargo check` 通過
- NPC 決策邏輯單元測試
- LLM 呼叫能打到本機 Ollama

---

### Phase 4：Server + 入口（第四層）

遷移 server 和 main。

重點：
- axum 路由對應現有所有 HTTP 端點
- WebSocket 訊息格式 100% 相容（前端不改一行）
- room editor API 完整對應
- session 管理

驗收：
- `cargo check` 通過
- 前端 `web/index.html` 連上 Rust 後端能正常遊玩
- room editor 能正常使用
- WebSocket 訊息格式與 Go 版完全一致（可用 diff 對比）

---

### Phase 5：整合測試 + 工具

- 端對端：前端 + Rust 後端 + 現有 JSON 資料，完整遊玩流程
- checkrooms 工具遷移或確認 Go 版仍可用
- start 腳本更新為 Rust build

最終驗收：
- `cargo check` 通過（零警告）
- `cargo test` 全過
- `cargo clippy` 零警告
- 前端功能與 Go 版完全一致
- Go 原始碼可歸檔

---

## 四、Go 版本處置

遷移期間 Go 版本**停止新功能開發**。遷移完成後，Go 原始碼移入 `archive/go/`（同上次 Rust 封存的做法）。

---

## 五、風險與對策

| 風險 | 對策 |
|------|------|
| AI 代理用 `.unwrap()` 逃避錯誤處理 | review 指標：`grep -r "unwrap()" --include="*.rs"` 計數，類比現在的 `_ =` |
| WebSocket 訊息格式不一致導致前端壞掉 | Phase 4 必須有訊息格式對比測試 |
| store 併發模型從 goroutine 轉 tokio 時引入 bug | Phase 2 加壓力測試 |
| 遷移期拖太長，遊戲內容停滯 | 嚴格按 Phase 推進，每個 Phase 有明確驗收，不跳躍 |
| serde 序列化與現有 JSON 格式不完全對齊 | Phase 1 就寫 roundtrip 測試：讀現有 JSON → 序列化回去 → diff 為零 |

---

## 六、進度追蹤

| Phase | 狀態 | 說明 |
|-------|------|------|
| Phase 0：專案骨架 | **完成** | `Cargo.toml`（axum、tokio、serde、bcrypt、reqwest、anyhow、thiserror、tracing 等）、`src/` 模組樹、`cargo check` 通過 |
| Phase 1：型別 + 設定 | **進行中** | `model`、`entity`、`config`、`gametext` 落檔；**`event` 日誌 API** 已對齊 Go journal；§三 **驗收項**（全 `data/config` 單測、enum 窮舉）仍待補齊 |
| Phase 2：Store + DB | **進行中** | `store`、`db` 已移植主體片段；**真實資料載入**已有 `cargo test --test store_init_integration`；`init` 內對可選 JSON 仍多為「缺檔略過、壞檔才錯」— 與 §三「失敗明確報錯」對齊尚待收斂 |
| Phase 3：遊戲邏輯 | **進行中** | **`economy`、`game`、`combat`、`world`（含 terrain_display）**；**`ai` 全套**；`npc`／`npcnpc`／`game` 其餘仍待搬 |
| Phase 4：Server + 入口 | **未開始** | `server` 占位；`main.rs` 僅日誌初始化，尚無 HTTP/WebSocket |
| Phase 5：整合測試 | **未開始** | `cargo clippy`／E2E／`start` 改 Rust 等 |

更新本表時可自行掃描：`find src -name '*.rs' -print0 | xargs -0 wc -l | sort -n`

---

*奇點世界專案 — Go → Rust 遷移計畫 v1.9*

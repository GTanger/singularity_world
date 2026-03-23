# Go → Rust 後端遷移計畫

> 建立日期：2026-03-24
> 決策依據：[決策 010](../decisions/010_go_to_rust_migration.md)
> 狀態：**設計中**

---

## 一、遷移原則

1. **前端不動**：web/ 目錄零修改，Rust 後端提供完全相同的 HTTP/WebSocket API
2. **資料不動**：data/ 目錄的 JSON 結構不改，Rust 版本讀寫同樣的檔案
3. **API 契約不動**：WebSocket 訊息格式、HTTP 端點路徑、回應結構全部保持一致
4. **逐模組遷移**：不是一次重寫，而是按依賴順序逐層搬移
5. **每層都能編譯、每層都能測試**：不接受「先寫骨架再填邏輯」

---

## 二、模組對應表

### 第零層：型別定義（無依賴）

| Go | Rust | 行數 | 說明 |
|---|---|---|---|
| model/ | src/model/ | 30 | Room、Exit 型別 |
| entity/ | src/entity/ | 66 | Character 結構 |

### 第一層：設定與基礎（僅依賴第零層）

| Go | Rust | 行數 | 說明 |
|---|---|---|---|
| config/ | src/config/ | 551 | 伺服器參數、環境變數覆蓋 |
| event/ | src/event/ | 64 | 事件日誌 Append/Query |
| gametext/ | src/gametext/ | 418 | 文案載入與格式化 |

### 第二層：資料層（依賴第零、一層）

| Go | Rust | 行數 | 說明 |
|---|---|---|---|
| store/ | src/store/ | 2970 | JSON 記憶體層，唯一資料源 |
| db/ | src/db/ | 3448 | 資料存取介面（讀寫 store） |

### 第三層：遊戲邏輯（依賴第零～二層）

| Go | Rust | 行數 | 說明 |
|---|---|---|---|
| game/ | src/game/ | 518 | 遊戲時間、視野、觀測 |
| combat/ | src/combat/ | 232 | 戰鬥結算 |
| economy/ | src/economy/ | 28 | 經濟引擎（目前極簡） |
| npc/ | src/npc/ | 1735 | NPC 決策、移動、行為 |
| npcnpc/ | src/npcnpc/ | 1202 | NPC 間對話觸發 |
| ai/ | src/ai/ | 956 | LLM 呼叫、評分 |
| world/ | src/world/ | 216 | 地圖格點（目前暫封） |

### 第四層：對外介面（依賴全部）

| Go | Rust | 行數 | 說明 |
|---|---|---|---|
| server/ | src/server/ | 3607 | HTTP/WebSocket、session、room editor API |
| main.go + 根層 | src/main.rs | 753 | 入口、路由註冊、模擬主迴圈 |

### 工具（獨立，可最後處理或保持 Go）

| Go | 處理方式 | 說明 |
|---|---|---|
| cmd/checkrooms | 遷移或保持 Go | 房間資料檢查工具 |
| cmd/sw-set-password | 遷移 | 密碼設定工具 |
| internal/roomcheck | 隨 checkrooms | 檢查邏輯 |

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
| Phase 0：專案骨架 | 未開始 | |
| Phase 1：型別 + 設定 | 未開始 | |
| Phase 2：Store + DB | 未開始 | |
| Phase 3：遊戲邏輯 | 未開始 | |
| Phase 4：Server + 入口 | 未開始 | |
| Phase 5：整合測試 | 未開始 | |

---

*奇點世界專案 — Go → Rust 遷移計畫 v1.0*

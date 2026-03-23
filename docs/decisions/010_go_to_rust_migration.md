# 決策 010：後端從 Go 遷移至 Rust

**狀態**：已決斷
**日期**：2026-03-24
**依據**：主管決斷（經深度討論）

---

## 1. 為什麼換

### 1.1 核心問題：語言無法強制 AI 代理的紀律

本專案由設計師主導、AI 代理實作。技術約束規則明文禁止 `_ = someFunc()`，但截至決策日，程式碼中有 **159 處**忽略錯誤的寫法。Go 的編譯器不把這當錯誤，`go vet` 也不攔。

這不是 AI 不聽話，是 Go 允許 AI 這樣做而不受懲罰。

### 1.2 後果不是效能問題，是靜默故障

被忽略的錯誤不會讓程式變慢，會讓程式**表面正常但底層不一致**：
- store 初始化時 17 個 JSON 載入全用 `_ =`，任何一個檔案損壞都不會報錯
- NPC 移動寫入位置失敗被吞掉，NPC 永遠到不了目的地
- 設計師會在遊戲行為層面花時間除錯，永遠不會懷疑是底層操作失敗

### 1.3 Go 的天花板

已嘗試所有 Go 能提供的防護：規則文件、`go vet`、`go test`、`checkrooms`。但 Go 語言層級不提供：
- 強制錯誤處理（`Result<T, E>`）
- 窮舉 enum match
- 編譯期 null safety（`Option<T>`）
- 編譯期並發安全（ownership）

這些是語言設計決定，不會改變。

### 1.4 Rust 的定位

對本專案而言，Rust 的價值不是效能，是**約束 AI 代理的韁繩**。`cargo check` 一條指令覆蓋的範圍，比 Go 的 `go vet` + `go test` + `checkrooms` 加起來還廣。

---

## 2. 跟上次的差異

### 2.1 上次失敗原因（2025-12 ~ 2026-01）

| 問題 | 說明 |
|------|------|
| Bevy ECS | 與 borrow checker 衝突，AI 代理持續撞牆 |
| Dioxus WASM 前端 | 不成熟、編譯 3-5 分鐘、WASM 除錯地獄 |
| 全棧 Rust | 前後端同時用 Rust，複雜度太高 |

**結論：問題不是 Rust，是技術選型過重。**

### 2.2 這次的邊界

- **只重寫後端**，前端（原生 HTML/CSS/JS）不動
- **不用 ECS**，不用 Bevy，不用 WASM
- **JSON 資料完全沿用**，598 間房間、entities、runtime 資料不動
- 技術棧極簡：axum + serde + tokio

---

## 3. 技術棧

| 層 | 選擇 | 對應現有 Go |
|---|---|---|
| HTTP 框架 | axum | 標準庫 net/http |
| WebSocket | tokio-tungstenite | gorilla/websocket |
| JSON | serde + serde_json | encoding/json |
| 密碼雜湊 | argon2 或 bcrypt crate | golang.org/x/crypto/bcrypt |
| async runtime | tokio | goroutine |
| 前端 | 不動 | 不動 |
| 資料 | JSON 檔案，不動 | JSON 檔案 |

**禁止引入**：Bevy、任何 ECS 框架、WASM、前端框架、ORM、資料庫。

---

## 4. 現有規模

- 132 個 Go 檔案，18110 行（不含測試）
- 19 個套件
- 外部依賴僅 2 個（gorilla/websocket、golang.org/x/crypto）
- 前端：HTML/CSS/JS，與後端語言無關

---

*奇點世界專案 — 決策紀錄*

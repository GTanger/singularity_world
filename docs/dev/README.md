# dev/ — 開發與實作規劃入口

**父層索引**：設計／規格總表見 [`../文檔索引.md`](../文檔索引.md)；`docs/` 子目錄樹速查見 [`../INDEX.md`](../INDEX.md)。

## 建置與品質閘門 (Standard Quality Gates)

本專案採實施 **「開發即測試」** 的嚴格紀律。所有變更在進入 `./start` 或部署前必須通過以下閘門：

- **靜態檢查**：`cargo clippy` (不可有任何 warning)
- **邏輯測試**：`cargo test`
- **資料契約**：`cargo run --bin checkrooms` (驗證房間 JSON 格式與觸發關鍵字)

以上流程皆已封裝於 **`./start`** 腳本中。

**合併／交付前（不建置、不啟動時）**：專案根目錄執行 **`make verify`**，等同 `./start` 前段閘門（`cargo clippy -D warnings`、`cargo test`、`checkrooms`）。CI 或 pre-push 宜與此對齊，避免只跑 `cargo build`。

### 遷移現況：Rust 原生化

依據 [決策 010](../decisions/010_go_to_rust_migration.md) 與 [遷移說明](../migration/README.md)：
- **主線服務**：由 **Rust（Axum）** 提供。
- **後續開發**：新功能在 `src/` 下實作。

---

*開發導覽：若需調整遊戲主循環，請至 `src/game/simulation_loop.rs`；若需擴展 API，請至 `src/server/http_api.rs`。*

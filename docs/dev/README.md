# dev/ — 開發與實作規劃入口

## 建置與品質閘門 (Standard Quality Gates)

本專案採實施 **「開發即測試」** 的嚴格紀律。所有變更在進入 `start-rust` 或部署前必須通過以下閘門：

- **靜態檢查**：`cargo clippy` (不可有任何 warning)
- **邏輯測試**：`cargo test` (包含 `store_init_integration` 真實資料載入測試)
- **資料契約**：`cargo run --bin checkrooms` (驗證房間 JSON 格式與觸發關鍵字)

以上流程皆已封裝於 **`./start-rust`** 腳本中。

### 遷移現況：Rust 原生化

依據 [決策 010](../decisions/010_go_to_rust_migration.md) 與 [遷移計畫](../migration/README.md)：
- **主線服務**：已全面切換由 **Rust (Axum)** 提供。
- **Go 原始碼**：已全數歸檔至 `archive/go/`，僅供備查。
- **後續開發**：所有新功能（如：NPC 社交、物品系統擴展）皆應直接在 Rust `src/` 下實作。

---

*開發導覽：若需調整遊戲主循環，請至 `src/game/simulation_loop.rs`；若需擴展 API，請至 `src/server/http_api.rs`。*

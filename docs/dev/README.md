# dev/ — 開發與實作規劃入口（008 P8）

- **`implementation/`** → 符號連結至 [`../implementation/`](../implementation/)（既有實作規劃檔仍保留原路徑，避免一次改壞全庫連結）。
- 之後新增「遷移日記、測試報告」等可置於本目錄同層。

## 建置紀律（嚴格閘門）

- 專案根目錄 **`make verify`** = `go vet` + `go test ./...` + `checkrooms -brackets -strict`，與 **`./start`** 啟動前階段一致。
- 原則：**契約與靜態檢查先於可執行檔**；房間 JSON 的 `〔〕` 與 `Move`／`Look` 觸發字為資料契約，違反即失敗，不採「先上線再補」。

### 遷移期：Rust 後端（與 Go 並行）

依 [決策 010](../decisions/010_go_to_rust_migration.md)、[遷移計畫](../migration/README.md)：目前**可玩服務仍由 Go 提供**；Rust 在 `src/` 逐步替換。

- **每次提交／大改 Rust 後建議**：`cargo check`；條件允許時加上 **`cargo clippy --tests -- -D warnings`**、`cargo test`（含 **`cargo test --test store_init_integration`** 驗證 store 載入完整 `data/`，與遷移 Phase 2／5 對齊）。
- **與 `make verify` 關係**：在 Rust 未接管 `./start` 前，**兩套都做**才不會「Go 過關、Rust 編不過」。待 Phase 4/5 完成後，將 **`verify`／`start` 改為以 `cargo` 為主**（計畫已載於 `docs/migration/README.md` §三 Phase 5）。

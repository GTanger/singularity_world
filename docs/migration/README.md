# 後端遷移（已完成）

後端已統一為 **Rust**（`Cargo.toml`、`src/`）。歷史對照表與舊版行數對帳已下架，避免與現況脫節。

- **啟動**：專案根目錄 **`./start`**（閘門＋release＋trunk＋systemd，見根目錄腳本）。
- **驗證**：`cargo clippy -- -D warnings`、`cargo test`、`cargo build --release`。

細節以程式與 `docs/decisions/` 現行決策為準。

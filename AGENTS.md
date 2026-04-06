# AGENTS.md（代理入口）

本檔依 [Harness Engineering](https://openai.com/index/harness-engineering/) 慣例維持**短小**：根目錄只放入口與鏈結，**不**把長篇偏好與工作區細節塞在同一檔（避免單檔膨脹、難維護）。

## 必讀鏈結（順序）

1. **`.cursorrules`** — 角色、技術棧、硬規則、必讀文檔提示。
2. **`docs/文檔索引.md`** — 設計／規格之**文檔索引**；任務前依主題查表閱讀。
3. **`docs/AGENTS_LEARNED.md`** — 累積之**使用者偏好**與**工作區事實**（全文；代理必讀）。

## 建置與「重啟」

> **改完代碼就要重啟**：同一輪任務結尾必跑專案根目錄 **`./start`**（閘門＋建置＋Hex `trunk`＋服務重啟）。**禁止**只改檔、不跑流程就當交付完成。

- **「重啟」只有一種**：專案根目錄 **`./start`**（`cargo clippy`、`test`、`checkrooms`、`cargo build --release`、`editor-leptos` 之 `trunk build`、`systemctl --user restart`）。**不另分**「僅後端／僅前端」捷徑；**不得**只 `systemctl restart` 而跳過閘門與建置（`/hex-editor` 載入 **`editor-leptos/dist`**，未經 `./start` 內之 `trunk build` 則畫面不會更新）。
- **每次修改本倉庫程式碼**（`src/`、`editor-leptos/`、`web/` 等）後，代理**必須**在該次任務內**自動**執行 **`./start` 自檢**（全閘門＋建置＋重啟），**不得**只改檔不啟動。若環境無法執行須明說阻塞與已做到的最後一步。
- 純 Markdown／註解／與建置無涉之檔案若未改程式碼，不在此強制；**一旦動到程式**，一律 **`./start`**。

## 與 `CLAUDE.md`、Cursor 的關係

- **`CLAUDE.md`**：協作角色與技術大綱（給 Claude／審局語境）。
- 本檔與 **`docs/AGENTS_LEARNED.md`**：代理行為與專案累積事實；與設計衝突時以 **`docs/`** 內已定案規格為準。

## 更新慣例

代理做錯、卡住、或使用者補充可重複規則時：**改 `docs/AGENTS_LEARNED.md`**（或對應設計文檔），並保持本檔精簡。

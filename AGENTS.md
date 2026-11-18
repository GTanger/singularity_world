# AGENTS.md（代理入口）

本檔依 [Harness Engineering](https://openai.com/index/harness-engineering/) 慣例維持**短小**：根目錄只放入口與鏈結，**不**把長篇偏好與工作區細節塞在同一檔（避免單檔膨脹、難維護）。

## 必讀鏈結（順序）

1. **`.traerules`** — Trae／歷史上下文入口（目前指向 `.cobridge/AI_CONTEXT.md`）。若不存在 `.cursorrules`，不得假設它是入口。
2. **`WORKBOARD.md`** — 記錄 `2026-07-16` 階段性「回文字」與 `2026-10-15` 世界流動四接頭之底層因果演進歷史；**現行玩家主表面已於 `2026-11-17` 正式演進定案為「輕量版 RimWorld 可縮放 2D 平面開放世界大地圖」**（`20m` 母格 ↔ `2m` 施工細格、沙鹿竹林里 ↔ 大肚台地西麓 `240×160`、中性個人底欄＋接觸插座、宣紙寫生三層萬人萬相，詳見 `docs/design/people_and_relations_plugin_socket_spec.md`、`docs/design/shalu_zhulin_terrain_and_objects.md` 與 `singularity-game-design` 技能）。
3. **`docs/文檔索引.md`** — 設計／規格之**唯一主索引**；任務前依主題查表閱讀。
   **`docs/INDEX.md`** 僅為 `docs/` 目錄／008 模組速查，與主索引並列時以 `文檔索引.md` 為準。
4. **`docs/AGENTS_LEARNED.md`** — 累積之**使用者偏好**與**工作區事實**（全文；代理必讀）。

## 建置與「重啟」

- **修改程式碼必跑專案根目錄 `./start`**：凡異動 `src/`、`editor-leptos/`、`web/` 等程式碼，結尾必須自動跑 `./start` 完成「閘門＋建置＋前端資源＋`systemctl --user restart`」，嚴禁拆出只重啟或只建後端的捷徑（純 Markdown／註解若未動程式可不重編）。
- **本機防鬼打牆總閘門（`tra-sw-gate`）與「手機版必須使用實體手機驗收」鐵律**：`./start` 已內建串聯 `/home/tanger/.local/bin/tra-sw-gate`（六道關卡：Git/台帳/`SESSION.md` 200 行預算、`code_guard_rs` 全變更檔正統性與體積、`zh-TW` + `DESIGN.md` + `sw.js` 快取版號 + 故事帳本 + 306 篇 Wiki OKF、白塔契約 + `checkrooms` + 四大驗算器 + PostgreSQL 實況對帳、`bin/server-rust` 新鮮度、HTTP `:1721` 探測）。**凡執行《奇點世界》手機版視覺驗收，只要電視後木架備用手機（`Pixel 9 Pro XL` `46051FDAS00CD5` / `46141FDAS001C8` 或 `vivo X80`）在線，必須使用實體手機透過 `tra-sw-gate --phone-qa`（底層呼叫 `/home/tanger/.local/bin/tra-phone-web-qa` 驅動真機 Android Chrome 渲染、實際登入點擊並擷取 `/dev/shm/sw-phone-qa-*.png` 真機截圖）親自驗收，嚴禁以桌面／筆電瀏覽器 `emulate` 縮放視口替代真機驗收！** 常用子命令：`tra-sw-gate --deploy`（Release 熱更新＋驗收）、`tra-sw-gate --phone-qa`（實體手機版視覺與互動驗收）、`tra-sw-gate --checkpoint '<訊息>'`（Git 提交＋PostgreSQL/Bundle 雙重熱備份）、`tra-sw-gate --push`（推送 GitHub 遠端＋熱備份）、`tra-sw-gate --blast <符號>`（符號爆炸半徑）、`tra-sw-gate --db-diag`（PostgreSQL 一鍵診斷）、`tra-sw-gate --html-report`（`am` 藍圖體檢看板）、`tra-sandbox test-sw`（RAMDisk 沙盒驗證）。
### 運維事故恢復例外（不是改碼捷徑）

- 若本輪**沒有修改 repo 程式碼**，而 `./start` 會把既有、無關且尚未審查的 dirty source 一併重建部署，則 unit／環境／日誌接線的事故恢復可以只重啟**既有已部署 binary**。
- 這條例外必須先確認沒有連線玩家、保存會被 runtime 寫動的資料快照、確認 binary 雜湊未變；重啟後要驗證資料庫連線、HTTP 健康與日誌，並在收尾明列。只要本輪碰過程式碼，仍須走 `./start`，不得引用本例外。

## 與 `CLAUDE.md`、Cursor 的關係

- **`CLAUDE.md`**：協作角色與技術大綱（給 Claude／審局語境）。
- 本檔與 **`docs/AGENTS_LEARNED.md`**：代理行為與專案累積事實；與設計衝突時以 **`docs/`** 內已定案規格為準。

## 更新慣例

代理做錯、卡住、或使用者補充可重複規則時：**改 `docs/AGENTS_LEARNED.md`**（或對應設計文檔），並保持本檔精簡。

- **記憶／入庫**：只寫可重複、對主線有意義的條目；對齊使用者**意圖與重點**，勿記儀式步驟、枝節、或代理自編的「合規」。使用者已刪的規則勿以變體復活；不確定是否夠格入庫時，寧可少寫。

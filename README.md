# 奇點世界（Singularity World）

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**  
體驗端以手機使用情境為主，現階段為 PWA 運作模式。

---

## 專案現況

- **後端：Rust 服務**（Axum 0.8 + Tokio）。
- **資料儲存**：**PostgreSQL 為唯一權威持久層**；執行期真理以資料庫為準。`data/` 下 JSON 僅作種子、靜態設定與開發備份（細節見 `AGENTS.md`、`docs/技術約束規則.md`）。
- **主要路徑**：單一六角格世界模型（HexGrid）；實體與資源點等遊戲狀態以 PG 為主。
- **前端**：原生 HTML/CSS/JS (PWA)，提供遊戲頁面與管理工具組（`/hex-editor/`、星圖）。
- **進度**：Phase 5 遷移完成，現正於 Hex 基礎上建立「資源與經濟 (Phase 6)」架構。

---

## 快速啟動

### 啟動前嚴格閘門（品質與契約優先）

本專案將 **靜態檢查與資料契約** 視為建置前置條件。`./start`（systemd 重啟）：

1. **`cargo clippy -- -D warnings`**：程式碼品質檢查（不可有任何警告）。
2. **`cargo test`**：單元與整合測試。
3. **`cargo run --bin checkrooms -- -brackets -strict`**：房間 JSON 契約檢查（驗證觸發字與括號對應）。
4. **`cargo build --release`** → `bin/server-rust`。
5. **`trunk build --release`**（`editor-leptos/dist`，供 `/hex-editor/`）。需已安裝 **`cargo install trunk`**；若未裝會中止。

### 一鍵啟動 (Rust)

在專案根目錄執行：

```bash
./start        # 推薦：含 systemd 重啟 singularity.service
```

用途：通過閘門後，建置後端與 Hex 編輯器並啟動伺服器（預設埠：1721）。

---

## 常用路由與 API

- **遊戲端**：`/`, `/ws` (WebSocket)
- **工具端**：`/hex-editor/`, `/star_chart`, `/admin`
- **資料 API**：
    - `/api/hex/grid`：全量地圖格網資料
    - `/api/action/gather`：資源採集接口 (PG 驅動)
    - `/api/topology`：星圖演化拓撲

---

## 主要目錄結構

```text
singularity_world/
├── src/
│   ├── main.rs         # 程式進入點
│   ├── server/         # Axum 路由、WS Hub、Session、Hex API
│   ├── store/          # PostgreSQL 權威驅動；拒絕依賴執行期 JSON
│   ├── world/          # 世界地圖、區塊管理、資源點系統 (resource.rs)
│   ├── hex/            # 六角格座標計算、揭露與玩家視野
│   ├── game/           # 遊戲主循環、NPC 模擬、動作分派 (do_action)
│   ├── combat/         # 戰鬥結算判定
│   ├── npc/            # NPC 決策 (10.21)、話語池、社交觸發
│   ├── ai/             # Ollama LLM 整合、Prompt 管理
│   └── bin/            # 獨立工具（如 checkrooms）
├── data/               # 房間 JSON (種子)、設定檔、資料快照
├── web/                # 前端靜態資源 (HTML/CSS/JS)
└── docs/               # 設計文獻、API 規範、實作建議
```

---

## 文件導覽

**主索引（設計／規格／AI 任務前查表）**：[`docs/文檔索引.md`](docs/文檔索引.md) — 必讀四份、決策表、規格表與 `reference/` 大表皆在此。

| 用途 | 路徑 |
| --- | --- |
| 資源點系統規範 | [`docs/design/資源點實作規範.md`](docs/design/資源點實作規範.md) |
| 累積記憶與偏好 | [`docs/AGENTS_LEARNED.md`](docs/AGENTS_LEARNED.md) |
| 代理精簡入口 | 根目錄 [`AGENTS.md`](AGENTS.md) |

---

*奇點世界 — 一個能量過度充沛的奇幻世界，萬物因此瘋長。*

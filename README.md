# 奇點世界（Singularity World）

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**  
體驗端以手機使用情境為主，現階段為 PWA 運作模式。

---

## 專案現況

- **後端：Rust 服務**（Axum 0.8 + Tokio）。
- **資料儲存**：**PostgreSQL 為唯一權威持久層**；執行期真理以資料庫為準。`data/` 下 JSON 僅作種子、靜態設定與開發備份（細節見 `AGENTS.md`、`docs/技術約束規則.md`）。
- **主要路徑**：白塔半島文字 GUI 活世界。305 個正式可通行區域與 1,644 條路線
  承載玩家及居民；舊 119 格已退出 active 世界。玩家以東、西、南、北、上、下、
  進、出行動，沒有圖像玩家地圖。
- **前端**：原生 HTML/CSS/JS (PWA)，把 PostgreSQL 權威世界投射成可操作文字 GUI，
  另提供星圖與管理工具。
- **進度**：以 `WORKBOARD.md` 的方向定調與 runtime check／正式服務實況為準；舊
  Hex、方格玩家世界、觀景窗與歷史模擬器均已退役或凍結。

---

## 快速啟動

### 啟動前嚴格閘門（品質與契約優先）

本專案將 **靜態檢查與資料契約** 視為建置前置條件。`./start`（systemd 重啟）：

1. **`cargo clippy -- -D warnings`**：程式碼品質檢查（不可有任何警告）。
2. **`cargo test`**：單元與整合測試。
3. **`cargo run --bin white-tower-runtime-check`**：完整來源、場所、路線、資源、名稱與
   active runtime 契約檢查。
4. **`cargo run --bin checkrooms -- -brackets -strict`**：相容房間 JSON 契約檢查。
5. **`cargo build --release`** → `bin/server-rust`，再重啟正式服務。

### 一鍵啟動 (Rust)

在專案根目錄執行：

```bash
./start        # 推薦：完整閘門 + 建置 + systemd 重啟 singularity.service
```

用途：通過閘門後，建置後端與前端資源並啟動伺服器（預設埠：1721）。

---

## 常用路由與 API

- **遊戲端**：`/`, `/ws` (WebSocket)
- **工具端**：`/star_chart`, `/admin`
- **資料 API**：
    - `/api/action/gather`：資源採集接口 (PG 驅動)
    - `/api/topology`：星圖演化拓撲

---

## 主要目錄結構

```text
singularity_world/
├── src/
│   ├── main.rs         # 程式進入點
│   ├── server/         # Axum 路由、WS Hub、Session、API
│   ├── store/          # PostgreSQL 權威驅動；拒絕依賴執行期 JSON
│   ├── world/          # 白塔拓撲、正式場所／路線、旅程、物理地理與資源
│   ├── game/           # 遊戲主循環、NPC 模擬、動作分派 (do_action)
│   ├── combat/         # 戰鬥結算判定
│   ├── npc/            # NPC 決策、話語池、社交觸發
│   ├── ai/             # Ollama LLM 整合、Prompt 管理
│   └── bin/            # 獨立工具（如 checkrooms、migrate）
├── data/               # 房間 JSON (種子)、設定檔、資料快照
├── web/                # 前端靜態資源 (HTML/CSS/JS)
└── docs/               # 設計文獻、API 規範、實作建議
```

---

## 文件導覽

**主索引（設計／規格／AI 任務前查表）**：[`docs/文檔索引.md`](docs/文檔索引.md) — 必讀四份、決策表、規格表與 `reference/` 大表皆在此。

| 用途 | 路徑 |
| --- | --- |
| 方向定調 | [`WORKBOARD.md`](WORKBOARD.md) |
| 技術約束 | [`docs/技術約束規則.md`](docs/技術約束規則.md) |
| 累積記憶與偏好 | [`docs/AGENTS_LEARNED.md`](docs/AGENTS_LEARNED.md) |
| 代理精簡入口 | 根目錄 [`AGENTS.md`](AGENTS.md) |

---

*奇點世界 — 世界先活著，玩家在其中活出自己的故事。*

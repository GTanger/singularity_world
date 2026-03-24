# 奇點世界（Singularity World）

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**  
體驗端以手機使用情境為主，現階段為 PWA 運作模式。

---

## 專案現況

- **後端：Rust 服務**（Axum 0.8 + Tokio），全面取代原 Go 實作。
- **資料儲存**：JSON/store 作為唯一執行期資料來源（No DB）。
- **主要路徑**：`data/rooms/editor/`（房間資料）、`data/entities.json`（實體狀態）、`data/runtime/`（執行期快照）。
- **前端**：原生 HTML/CSS/JS (PWA)，提供遊戲頁面與管理工具組（房間編輯器、地圖、星圖）。
- **進度**：Phase 0–5 遷移已全數完成。

---

## 快速啟動

### 啟動前嚴格閘門（品質與契約優先）

本專案將 **靜態檢查與資料契約** 視為建置前置條件。`./start-rust` 會依序執行：

1. **`cargo clippy -- -D warnings`**：程式碼品質檢查（不可有任何警告）。
2. **`cargo test`**：單元與整合測試。
3. **`cargo run --bin checkrooms -- -brackets -strict`**：房間 JSON 契約檢查（驗證觸發字與括號對應）。

### 一鍵啟動 (Rust)

在專案根目錄執行：

```bash
./start-rust
```

用途：通過閘門後，建置 Release 版本並啟動伺服器（預設埠：1721）。

### 奇點 + Chatmery 一起啟動

```bash
./start-with-chatmery
```

安裝 user-level 開機自啟（systemd）：

```bash
./start-with-chatmery --install
```

---

## 常用路由與 API

- **遊戲端**：`/` (Index), `/ws` (WebSocket)
- **工具端**：`/map_viewer`, `/room_editor`, `/star_chart`, `/admin`
- **資料 API**：
    - `/data/rooms.json`：全量地圖資料
    - `/api/design-constants`：UI 常數同步
    - `/api/room-editor/graph`：可視化編輯器圖資
    - `/api/topology`：星圖演化拓撲

---

## 主要目錄結構

```text
singularity_world/
├── src/
│   ├── main.rs         # 程式進入點
│   ├── lib.rs          # 模組外括
│   ├── server/         # Axum 路由、WS Hub、Session、Room Editor API
│   ├── store/          # JSON 資料載入與 RwLock 狀態管理
│   ├── db/             # 業務邏輯封裝（密碼驗證、地圖查詢、拓撲計算）
│   ├── game/           # 遊戲主循環、NPC 模擬、動作分派 (do_action)
│   ├── world/          # 世界地圖、區塊管理、地形顯示
│   ├── combat/         # 戰鬥結算判定
│   ├── npc/            # NPC 決策、話語池、社交觸發
│   ├── ai/             # Ollama LLM 整合、Prompt 管理、內容過濾
│   ├── bin/            # 獨立工具（如 checkrooms）
│   └── roomcheck/      # 房間契約檢查邏輯庫
├── data/               # 房間 JSON、設定檔、實體快照
├── web/                # 前端靜態資源 (HTML/CSS/JS)
├── archive/go/         # 已遷移的 Go 原始碼備份（不納入版控）
└── docs/               # 設計文獻、遷移進度、API 文法
```

---

## 文件導覽

- **遷移對帳**：[`docs/migration/README.md`](docs/migration/README.md)
- **文檔索引**：`docs/文檔索引.md`
- **技術約束**：`docs/技術約束規則.md`
- **世界觀主參考**：`docs/reference/世界觀：Token降維與生命演化.md`

決策記錄（ADRs）位於 `docs/decisions/`。

---

*奇點世界 — 萬物皆為 Token，演化永不止息。*

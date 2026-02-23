# 奇點世界 (Singularity World)

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**

體驗端為手機；**無上架計畫**，目前從未考慮上架。

## 專案結構

```
singularity_world/
├── main.go              # 程式入口，啟動伺服器
├── config/              # 可調參數集中管理
├── server/              # WebSocket 連線管理、玩家 session
├── game/                # 遊戲主迴圈、視野區域、觀測坍縮
├── entity/              # 角色/建築實體、插頭插座匹配
├── world/               # 地圖格點、地形、阻擋、移動碰撞
├── economy/             # 經濟引擎 goroutine、交易與鎂流轉
├── combat/              # 戰鬥判定與 log
├── event/               # 事件日誌寫入/查詢、事件類型
├── db/                  # SQLite 連線、schema.sql
├── data/                # 執行期資料：world.db、maps/ 區塊地圖 .txt
│   └── maps/            # 區塊地形字 .txt（151×151 字/檔），檔名 {cx}_{cy}.txt
└── web/                 # 前端：index.html, style.css, main.js, canvas.js, ui.js
```

**一鍵啟動**：在專案根目錄執行 `./start`（會自動建置、開埠 1721、啟動伺服器；對應 Cloudflare Tunnel 與 https://sw.ygggt.com）。

手動建置：`go build -o bin/server .`。手動執行：`./bin/server`（預設埠 8080）；經 Tunnel 對外則 `PORT=1721 ./bin/server`。

## 文件

**完整文檔見 [docs/文檔索引.md](docs/文檔索引.md)。**

**AI 進行專案索引或檢索時，須先閱讀：**
1. [協作約定](docs/COLLABORATION.md)
2. [技術約束規則](docs/技術約束規則.md)  
再依文檔索引查找設計與規格。

- [協作約定](docs/COLLABORATION.md) — 主管與 AI 協作原則（最高意志、全方位支援）。
- [技術約束規則](docs/技術約束規則.md) — Go／原生前端／SQLite／WebSocket；程式碼風格、協作流程、禁止事項。
- [文檔索引](docs/文檔索引.md) — 設計、決策、規格、參考、下一步之完整索引。
- [決策紀錄](docs/decisions/) — 001 戰鬥統一、002 插頭插座、003 城鎮即大地圖、004 技術棧與架構。
- [第一版可做清單](docs/第一版可做清單.md) — MVP 要做／不做、優先順序。
- [下一步](docs/NEXT_STEPS.md) — 目前狀態與建議下一步。

---

*奇點世界專案*

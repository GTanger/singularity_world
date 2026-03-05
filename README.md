# 奇點世界 (Singularity World)

遊戲專案：**奇點世界**  
英文專案名：`singularity_world`

**設計核心：從有限框架延伸出無限可能。**

體驗端為手機；**無上架計畫**，目前從未考慮上架。

---

## 專案結構

```
singularity_world/
├── main.go              # 程式入口，啟動伺服器
├── config/               # 可調參數集中管理（含遊戲時間 epoch）
├── server/               # WebSocket 連線管理、玩家 session、登錄／創角
├── game/                 # 遊戲主迴圈、視野區域、觀測坍縮
├── entity/               # 角色／建築實體、插頭插座匹配
├── world/                # 地圖格點、地形、阻擋、移動碰撞
├── economy/              # 經濟引擎、交易與鎂流轉（對齊經濟彙整 §四）
├── combat/               # 戰鬥判定與 log
├── event/                # 事件日誌寫入／查詢
├── db/                   # SQLite、schema、entity／room／auth
├── data/                 # 執行期資料：world.db、game_epoch.unix、maps/
│   └── maps/             # 區塊地形字 .txt，檔名 {cx}_{cy}.txt
└── web/                  # 前端：index.html, style.css, main.js, canvas.js, ui.js, mud-text.js
```

**一鍵啟動**：在專案根目錄執行 `./start`（建置、開埠 1721、啟動伺服器；對應 Cloudflare Tunnel 與 https://sw.ygggt.com）。

手動建置：`go build -o bin/server .`。手動執行：`./bin/server`（預設埠 8080）；經 Tunnel 對外則 `PORT=1721 ./bin/server`。

---

## 文件

**完整文檔見 [docs/文檔索引.md](docs/文檔索引.md)。**

**AI 進行專案索引或檢索時，須先閱讀：**
1. [協作約定](docs/COLLABORATION.md)
2. [技術約束規則](docs/技術約束規則.md)
3. [世界觀：Token 降維與生命演化](docs/reference/世界觀：Token降維與生命演化.md) — 撰寫任何敘事、對白、系統日誌或文檔時須符合此定調，避免文不對題。

再依文檔索引查找設計與規格。

### 協作與約束
- [協作約定](docs/COLLABORATION.md) — 主管與 AI 協作原則（最高意志、全方位支援）。
- [技術約束規則](docs/技術約束規則.md) — Go／原生前端／SQLite／WebSocket；程式碼風格、協作流程、禁止事項。

### 設計與決策
- [文檔索引](docs/文檔索引.md) — 設計、決策、規格、參考之完整索引。
- [設計關鍵字](docs/DESIGN_KEYWORDS.md) — 視角、世界、移動、角色、戰鬥、經濟等。
- [決策紀錄](docs/decisions/) — 001 戰鬥統一、002 插頭插座、003 城鎮即大地圖、004 技術棧與架構、005 空間與視野、006 登錄與玩家模板。

### 規格與彙整（單一檔案）
- [詞盤彙整](docs/reference/詞盤彙整.md) — 詞盤系統：主管設想、本體論、竅穴／詞元對照表、語意邊界；鎂產消見經濟彙整。
- [經濟彙整](docs/reference/經濟彙整.md) — 經濟：概念、鎂產消閉環、單人×萬名 NPC、規劃建議與實作階段。
- [人物屬性彙整](docs/reference/人物屬性彙整.md) — 人物屬性：A+B 架構、出身與念紋加權、疊加公式、與詞盤對照表共用。
- [人物角色模板](docs/reference/人物角色模板.md) — 玩家／NPC 共用實體欄位（識別、位置、屬性、插座、經濟、觀測）。
- [移動與地圖規格](docs/reference/spec_移動與地圖.md) — 輸入／輸出／狀態／介面。

### 清單與下一步
- [第一版可做清單](docs/第一版可做清單.md) — MVP 要做／不做、優先順序。
- [下一步](docs/NEXT_STEPS.md) — 目前狀態與建議下一步。

---

*奇點世界專案*

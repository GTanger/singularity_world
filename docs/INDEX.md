# docs/ 目錄速查（008 附錄）

> **完整設計／規格索引（含 AI 必讀四份、決策表、大表）請以 [`文檔索引.md`](./文檔索引.md) 為唯一主入口。**  
> 本檔僅對齊 [`decisions/008_架構整頓規劃.md`](./decisions/008_架構整頓規劃.md)：提供 `docs/` 子目錄導航與程式模組速查，避免與主索引重複貼長表。

## 程式模組（精簡）

| 路徑 | 說明 |
|------|------|
| `npc/` | NPC 決策引擎、TravelerManager、話題／微互動／行為 JSON、求職撮合、未觀測 tick；**import `db`**，**`db` 不 import `npc`**（008 P4） |
| `server/` | WebSocket；**P5** 已按領域拆分（`handler`、`action_*` 等模組；現行見 `src/server/`） |
| `npcnpc/`（repo 根目錄套件） | NPC↔NPC 同房觸發、房間事件滑窗、社交除錯 API；對齊 008 P3 |

## 架構與決策

| 路徑 | 說明 |
|------|------|
| [decisions/008_架構整頓規劃.md](./decisions/008_架構整頓規劃.md) | 架構整頓主文件（含執行進度） |
| [decisions/009_觀測分級與行程約束.md](./decisions/009_觀測分級與行程約束.md) | **觀測分級**、行程優先於擲骰、在崗決定性、LLM／雲端定位（由討論 005 升格） |
| [decisions/011_世界場所與居民位置單一真相.md](./decisions/011_世界場所與居民位置單一真相.md) | 多尺度場所、八方向路線、玩家／NPC 共用位置真相、旅程續行與舊方格素材遷移 |
| [decisions/012_Azgaar白塔半島物理地理橋接.md](./decisions/012_Azgaar白塔半島物理地理橋接.md) | 完整 Azgaar 作者資料、固定來源與白塔物理地理橋接邊界 |
| [decisions/013_白塔半島正式活世界啟用.md](./decisions/013_白塔半島正式活世界啟用.md) | 正式白塔 runtime、舊格退役、全人物遷移、居民生活、資源守衡與文字 GUI 驗收 |
| [decisions/014_Azgaar白塔半島經濟橋接.md](./decisions/014_Azgaar白塔半島經濟橋接.md) | 完整作者經濟來源、人物工作／交易帳本、有限市場閉環的前置分層 |
| [decisions/015_白塔半島有限市場閉環.md](./decisions/015_白塔半島有限市場閉環.md) | 3,780 筆有限庫存、來源供需、居民消費與承運、玩家實際市場成交 |
| [decisions/016_Azgaar上游供應場所實體化.md](./decisions/016_Azgaar上游供應場所實體化.md) | 4,976 筆 Burg→Market 輪廓、277 個來源供應站、真人工作與來源到市場的實體物流 |
| [decisions/](./decisions/) | 其餘 ADR |
| [implementation/](./implementation/) | 實作規劃、遷移說明 |
| [dev/](./dev/) | 開發入口（品質閘門；含連結至 implementation） |
| [archived/](./archived/) | 歷史草稿、冷凍設定、舊 Hex 工作單 |
| [../tools/](../tools/) | 一次性工具（JS／Python 等，008 P6） |
| [config/](./config/) | 設定與參數索引（`PARAMETERS_INDEX.md`、`gametext_and_prompts.md`） |

## 規格與參考

| 路徑 | 說明 |
|------|------|
| [reference/](./reference/) | 技術規格彙整（NPC、房間、經濟、戰鬥等）— 條目級清單見 [`文檔索引.md`](./文檔索引.md) 第五節起 |
| [design/](./design/) | 系統設計長文（例：NPC 間對話） |

## 世界觀與草稿

| 路徑 | 說明 |
|------|------|
| [archived/現代地球設定/](./archived/現代地球設定/) | 現代地球設定與歷史模擬器冷凍資料 |
| [discussions/](./discussions/) | 討論稿（多數已轉決策） |
| [discussions/005_觀測分級與行程約束_雲端與程式模擬.md](./discussions/005_觀測分級與行程約束_雲端與程式模擬.md) | 討論脈絡與筆記；**已定案見 [009](./decisions/009_觀測分級與行程約束.md)**；§五實作待補 |

## 其他

| 路徑 | 說明 |
|------|------|
| [reference/autoresearch_backend.md](./reference/autoresearch_backend.md) | autoresearch 後端說明 |

---

*歷史資料統一放入 `archived/`；若再搬移 `discussions/`，請同步更新此表連結。*

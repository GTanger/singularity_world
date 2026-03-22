# 文檔導覽（008 P8 起點）

> 對齊 `docs/decisions/008_架構整頓規劃.md`：先建立索引，再漸進搬移子目錄，避免一次改壞連結。

## 程式模組（精簡）

| 路徑 | 說明 |
|------|------|
| `npc/` | NPC 決策引擎、TravelerManager、話題／微互動／行為 JSON、求職撮合、未觀測 tick；**import `db`**，**`db` 不 import `npc`**（008 P4） |
| `server/` | WebSocket；**P5** 已拆 `handler_*.go`（`handler_action` → `*_dispatch` + `action_entity_*`／`action_object_*`／`action_narrative_*` 樹葉檔） |
| `npcnpc/`（repo 根目錄套件） | NPC↔NPC 同房觸發、房間事件滑窗、社交除錯 API；對齊 008 P3 |

## 架構與決策

| 路徑 | 說明 |
|------|------|
| [decisions/008_架構整頓規劃.md](./decisions/008_架構整頓規劃.md) | 架構整頓主文件（含執行進度） |
| [decisions/](./decisions/) | 其餘 ADR |
| [implementation/](./implementation/) | 實作規劃、遷移說明 |
| [dev/](./dev/) | 開發入口（含連結至 implementation；008 P8） |
| [archive/](./archive/) | 歷史草稿（含原 gemini 概念檔） |
| [../tools/](../tools/) | 一次性工具（Go／JS／Python，008 P6） |
| [config/](./config/) | 設定與參數索引（`PARAMETERS_INDEX.md`、`gametext_and_prompts.md`） |

## 規格與參考

| 路徑 | 說明 |
|------|------|
| [reference/](./reference/) | 技術規格彙整（NPC、房間、經濟、戰鬥等） |
| [design/](./design/) | 系統設計長文（例：NPC 間對話） |

## 世界觀與草稿

| 路徑 | 說明 |
|------|------|
| [archive/gemini概念設計檔/](./archive/gemini概念設計檔/) | 早期概念草稿（維護頻率低） |
| [discussions/](./discussions/) | 討論稿（多數已轉決策） |

## 其他

| 路徑 | 說明 |
|------|------|
| [reference/autoresearch_backend.md](./reference/autoresearch_backend.md) | autoresearch 後端說明 |
| [文檔索引.md](./文檔索引.md) | 既有中文索引（若與本檔重疊，以本檔為 008 對齊入口） |

---

*`gemini概念設計檔/` 已於 008 P8 遷入 `archive/`；若再搬移 `discussions/`，請同步更新此表連結。*

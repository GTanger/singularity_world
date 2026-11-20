# 奇點世界（Singularity World）

英文專案名：`singularity_world`

設計核心是從有限框架延伸出無限可能，體驗端以手機直立單手操作為首要情境，採原生 HTML、CSS 與 JavaScript 建構漸進式網頁應用程式（PWA），桌機端則同步支援雙欄閱讀與常駐彩色輿圖。

---

## 世界觀與核心架構現況

《奇點世界》以「高維混沌滲入低維宇宙」及「不干涉、自然演化」為底層根律，在寧靜大洋、蒼淵主陸與東南外海銀鷗島（850 座不重名城鎮、100 個政權、71 項無火藥冷兵器與民生百工物產）之上運作。全系統以 Rust（Axum 0.8 + Tokio）與 PostgreSQL 作為唯一執行期權威持久層，`data/` 目錄下的 JSON 僅供首次創世種子與靜態地理編譯使用，執行期真理一律以資料庫狀態為準。

### 1. 全世界人物共用單一機制（`UnifiedMindBody`）

玩家角色、銀鷗島 674 名常駐居民，以及執行期隨時動態熱生成的千人、萬人新角色，在記憶體與資料庫中完全共用同一套 `UnifiedMindBody` 結構，僅以 `controller_id: Option<String>` 區分當下是否由人類帳號附身操控（離線入睡時同樣進入自主生理與睡眠記憶整合迴圈）：

| 架構層面 | 實作機制 |
| --- | --- |
| 唯一身分與萬人萬相 | 每位人物具備 `soul_seed BIGINT UNIQUE NOT NULL` 全域唯一條碼，派生 6 維先天動力「脾氣」、16×8 情境反射與價值偏序「個性」，以及綁定記憶 ID 且須經 3 夜反芻方能推移的「性格」（全程零形容詞標籤寫死） |
| 獨立有界記憶與慢思考 | 感官五道閘過濾後進入短期記憶（STM），就寢時整合為個人長期情節記憶（熱上限 256 則，含個人探索迷霧），超出 16 則之人生轉折點自動經 `cold_outbox` 封存至 PostgreSQL `oi_turning_points_archive`，重大懸念則排入老化補償慢思考佇列於鎖外非同步推演 |
| 萬人級 $O(1)$ 索引與隔離 | 記憶體內原子維護 `node_occupants`、`controller_to_actor`、SHA-256 不可逆公開代幣 `public_token_to_actor`（`p:<16hex>`，隱藏玩家登入帳號與底層種子）與 `used_soul_seeds` 四組索引 |
| 批次守恆落盤 | 採用 `ActorMap`（`Arc<UnifiedMindBody>` 寫入時複製 ＋ `dirty_keys` 髒鍵追蹤）搭配 PostgreSQL `UNNEST` 單筆交易批次寫入，嚴格執行 `Commit-Before-Publish`（資料庫提交成功後始更新記憶體主狀態），並與 `oi_world_globals`、`oi_contracts`、`oi_world_ledger` 維持全島銅幣複式簿記零誤差守恆 |

### 2. 三尺度手繪迷霧輿圖與多平面路由併存

主入口（`/`）採用三區鎖定沉浸式文字舞台，結合按個人真實踏足紀錄揭露的三尺度輿圖（外島 Voronoi 山海多邊形輿圖、白鷗港 WebP 水彩街廓與百戶城鎮魚骨展開圖、地底石室局部草圖），角色移動嚴格沿城鎮街道、支巷與門戶逐節點推進。前階段開發之台灣 20m DEM 立體底板（`SWL0`）與 2D 聚落沙盒，亦完整保留於 `/rim` 及 `/world_atlas` 併存備查。

---

## 啟動、部署與品質閘門

本專案將靜態檢查、契約驗算與線上探測視為建置與部署的強制前置條件，修改程式碼後請在專案根目錄執行 `./start` 一鍵完成完整閘門、建置與服務重啟：

```bash
./start        # 執行完整品質閘門 + Release 建置 + 重啟 systemd singularity.service（預設埠 1721）
```

啟動流程與本機守門員 `tra-sw-gate` 會依序驗證下列項目：

1. `cargo clippy -- -D warnings` 與 `code_guard_rs`（零警告、零 `unsafe`、單檔行數與正統性守衛）。
2. `cargo test`（含 `outer_island` 記憶體內動態熱新增 1,500 名共 2,175 人擴充壓測、v1 無損遷移與銅幣守恆測試）。
3. `white-tower-runtime-check`、`white-tower-living-world-check` 與 `checkrooms -- -brackets -strict` 契約檢查。
4. 台灣正體中文（`zh-TW`）詞彙守衛、PWA Service Worker 快取版號一致性與 `SESSION.md` 200 行交班預算。
5. `bin/server-rust` 執行檔新鮮度與 `http://127.0.0.1:1721/` 線上零內部識別碼洩漏探測。

### 常用開發與驗收指令

```bash
tra-sw-gate                        # 執行 6 道防鬼打牆靜態與線上實況關卡
tra-sw-gate --deploy               # Release 編譯並熱更新 singularity.service
tra-sw-gate --phone-qa             # 驅動實體 Android 手機（Pixel 9 Pro XL）執行真機渲染與互動驗收
tra-sw-gate --checkpoint '<訊息>'  # Git 提交、推送到遠端 origin/master 並執行 PostgreSQL + Bundle 雙重熱備份
```

---

## 主要路由與 HTTP 端點

| 分類 | 路徑 | 說明 |
| --- | --- | --- |
| 主世界入口 | `/` | 銀鷗外島與白鷗港三區鎖定文字舞台 ＋ 三尺度手繪彩色輿圖（PWA） |
| 外島世界 API | `/api/island/{state,map,setup,move,act,talk}` | `UnifiedMindBody` 狀態投射、個人迷霧輿圖、身世登岸、逐節點移動、百工交易與當面交談 |
| 通行認證 API | `/api/rim/{auth,me,logout}` | 旅人帳號登入、註冊與同源 `HttpOnly` Session 管理 |
| 保留平面與工具 | `/rim`、`/world_atlas`、`/star_chart`、`/admin`、`/ws` | 2D 先鋒聚落畫布、20m `SWL0` 山海底板全圖、星盤拓撲、管理後台與相容 WebSocket 通道 |

---

## 目錄結構

```text
singularity_world/
├── src/
│   ├── main.rs               # 伺服器程式進入點
│   ├── grid/
│   │   ├── outer_island/     # UnifiedMindBody 對等人物、有界記憶、慢思考、效用模擬與 UNNEST 持久化
│   │   ├── substrate/        # 20m DEM 物理底板（SWL0）、立體四層槽與 /rim 聚落模擬
│   │   └── zhulin/           # 竹林地形插件、空間節點與無職稱社交模擬
│   ├── pg/                   # PostgreSQL 稀疏覆寫表與認證 Session 持久化
│   ├── server/               # Axum HTTP 路由、island_api、rim_auth、模擬主迴圈與 WS Hub
│   ├── store/                # PostgreSQL 連線池與基礎儲存層
│   ├── world/                # 白塔半島場所、路線與經濟橋接契約
│   ├── combat/               # 人版數理成相與交鋒判定模組
│   ├── ai/                   # 本機與外部 LLM 對話提示詞與轉譯輔助
│   └── bin/                  # 獨立契約驗算與遷移工具（checkrooms、runtime-check 等）
├── data/
│   └── world/                # 全大陸正體中文總冊 JSON 與外島創世靜態編譯包
├── scripts/                  # Azgaar 全大陸正體轉譯與外島資料編譯腳本
├── web/                      # 原生 PWA 前端（index.html、island-ui.js/css、sw.js、maps/）
└── docs/                     # 世界觀正典、系統架構規格與決策紀錄
```

---

## 核心文件導覽

執行設計或開發任務前，請先查閱 [`docs/文檔索引.md`](docs/文檔索引.md) 取得完整規格分群。

| 用途 | 路徑 |
| --- | --- |
| 當前交班狀態（200 行內） | [`SESSION.md`](SESSION.md) |
| 文件總索引 | [`docs/文檔索引.md`](docs/文檔索引.md) |
| 世界觀與 16 題系統總綱 | [`docs/design/奇點世界_高維混沌與奇幻百戶大世界總綱_2026-11-20.md`](docs/design/奇點世界_高維混沌與奇幻百戶大世界總綱_2026-11-20.md) |
| 全大陸中文地名政權物產戰史總冊 | [`docs/design/奇點世界_全大陸中文地名政權物產戰史總冊_2026-11-20.md`](docs/design/奇點世界_全大陸中文地名政權物產戰史總冊_2026-11-20.md) |
| 對等實體人物與有界記憶架構正典 | [`docs/design/奇點世界_對等實體人物與獨立有界記憶架構正典_2026-11-20.md`](docs/design/奇點世界_對等實體人物與獨立有界記憶架構正典_2026-11-20.md) |
| 歷史演進定調與技術約束 | [`WORKBOARD.md`](WORKBOARD.md)、[`docs/技術約束規則.md`](docs/技術約束規則.md)、[`AGENTS.md`](AGENTS.md) |

---

*世界先活著，旅人在其中活出自己的故事。*

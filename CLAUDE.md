# Singularity World

## 語言與溝通
- 一律使用繁體中文（台灣用字）回覆和寫註解
- 程式碼變數名和函式名維持英文
- 我不寫程式碼，你是唯一的實作者。我負責設計方向和最終判斷

## 技術棧（不可更動）
- 後端：Rust（axum 0.8 + tokio + serde + bcrypt + reqwest + anyhow/thiserror + tracing + rand + uuid + futures-util + tower-http）
- 前端：原生 HTML/CSS/JS（PWA），無框架
- 資料：JSON/store，無資料庫（無 SQLite、無 PostgreSQL）
- 通訊：WebSocket（axum 內建）
- AI：Ollama 本地模型（透過 HTTP API，reqwest）
- 部署：單機，Linux Mint，Cloudflare Tunnel

## 專案結構
```
src/main.rs              — 入口：初始化 + 啟動伺服器
src/lib.rs               — 模組宣告 + run_server() 公開入口
src/ai/                  — LLM 呼叫（玩家對話 + NPC↔NPC 對話）
src/config/              — 可調參數（環境變數覆蓋）
src/store/               — JSON 記憶體層（唯一資料源）
src/db/                  — 資料存取介面（讀寫 store）
src/entity/              — 角色實體、枚舉型別
src/model/               — Room、Exit 等共用結構
src/game/                — 遊戲時間、視野
src/economy/             — 經濟引擎
src/world/               — 地圖格點、移動
src/npc/                 — NPC 行為（brain、wander、social）
src/npcnpc/              — NPC↔NPC 對話
src/combat/              — 戰鬥系統
src/event/               — 事件常數與紀錄
src/gametext/            — 文案模板（從 JSON 載入）
src/server/              — axum HTTP/WebSocket、session、simulation loop
web/                     — 前端
data/runtime/            — 執行期 JSON（archival、summaries、threads、dyads、rumors）
docs/                    — 設計文件（修改前必讀相關文件）
```

## 必讀文件（修改相關系統前先讀）
- `docs/COLLABORATION.md` — 協作約定
- `docs/技術約束規則.md` — 禁止事項與程式碼風格
- `docs/reference/世界觀：Token降維與生命演化.md` — 世界觀定調
- `docs/design/NPC間對話—記憶與情境完整設計.md` — NPC 對話系統設計規格

## 常用指令
```bash
cargo build --release && PORT=1721 ./target/release/singularity_world   # 建置並啟動
cargo clippy -- -D warnings                                              # 靜態檢查（零警告）
cargo test                                                               # 跑測試
```

## 硬規則
- **不要自作主張**。收到指令就執行，不確定就問。你是鏡子不是駕駛
- **不要捏造資料**。不確定的事說不確定。寧可少說也不要編造
- **不要引入新依賴**。除非我明確同意。不加新 crate、不加 npm package
- **不要改 store 結構**。JSON schema 變動影響全局，必須先確認
- **改動前看 diff 範圍**。能改一行就不改十行。不要重構我沒要求重構的東西
- **測試再提交**。改完後跑 `cargo build` 確認能編譯，`cargo clippy` 零警告，有 test 的跑 test

## NPC 對話系統重點
- 模型很小（2B~7B），prompt 精準度決定輸出品質
- **玩家↔NPC Talk 規則（分場景條列，改稿對照）**：`data/templates/PLAYER_TALK_WEB_LLM.md`；**執行時**仍以 `data/templates/llm_prompts.json` 為準，改規則須同步 JSON  
- **語用背景**（為什麼寒暄不是空話）：`docs/reference/玩家NPC對話與交際語用.md`
- `qualityGateNpcLine` 是品質門檻，寧嚴勿鬆
- archival/summary 寫入前必須去重，防止垃圾持久化汙染記憶
- 玩家 Talk 記憶檢索用 `SearchArchivalForPlayerTalk`（寒暄零命中勿硬塞最新節錄）
- NPC↔NPC 對話的設計目標：眾口鑠金——NPC 群體透過對話堆疊出世界觀

## 世界觀關鍵詞
- 沃土風
- 鎂（貨幣單位）
- 詞盤、竅穴、念紋（修煉體系）
- 量子坍縮（觀測機制）

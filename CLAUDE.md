# Singularity World

## 語言與溝通
- 一律使用繁體中文（台灣用字）回覆和寫註解
- 程式碼變數名和函式名維持英文
- 我不寫程式碼，你是唯一的實作者。我負責設計方向和最終判斷

## 技術棧（不可更動）
- 後端：Go（標準庫 + gorilla/websocket），無框架
- 前端：原生 HTML/CSS/JS（PWA），無框架
- 資料：JSON/store，無資料庫（無 SQLite、無 PostgreSQL）
- 通訊：WebSocket
- AI：Ollama 本地模型（透過 HTTP API）
- 部署：單機，Linux Mint，Cloudflare Tunnel

## 專案結構
```
main.go              — 遊戲主迴圈、NPC 社交觸發、debug API
ai/talk.go           — LLM 呼叫（玩家對話 + NPC↔NPC 對話）
config/config.go     — 可調參數（環境變數覆蓋）
store/store.go       — JSON 記憶體層（唯一資料源）
db/                  — 資料存取介面（讀寫 store）
entity/              — 角色實體
game/                — 遊戲時間、視野
economy/             — 經濟引擎
world/               — 地圖格點、移動
server/              — WebSocket、session
web/                 — 前端
data/runtime/        — 執行期 JSON（archival、summaries、threads、dyads、rumors）
docs/                — 設計文件（修改前必讀相關文件）
```

## 必讀文件（修改相關系統前先讀）
- `docs/COLLABORATION.md` — 協作約定
- `docs/技術約束規則.md` — 禁止事項與程式碼風格
- `docs/reference/世界觀：Token降維與生命演化.md` — 世界觀定調
- `docs/design/NPC間對話—記憶與情境完整設計.md` — NPC 對話系統設計規格

## 常用指令
```bash
go build -o bin/server . && PORT=1721 ./bin/server   # 建置並啟動
./start                                                # 一鍵啟動
go vet ./...                                           # 靜態檢查
```

## 硬規則
- **不要自作主張**。收到指令就執行，不確定就問。你是鏡子不是駕駛
- **不要捏造資料**。不確定的事說不確定。寧可少說也不要編造
- **不要引入新依賴**。除非我明確同意。不加新 go module、不加 npm package
- **不要改 store 結構**。JSON schema 變動影響全局，必須先確認
- **改動前看 diff 範圍**。能改一行就不改十行。不要重構我沒要求重構的東西
- **測試再提交**。改完後跑 `go build` 確認能編譯，有 test 的跑 test

## NPC 對話系統重點
- 模型很小（2B~7B），prompt 精準度決定輸出品質
- `qualityGateNpcLine` 是品質門檻，寧嚴勿鬆
- archival/summary 寫入前必須去重，防止垃圾持久化汙染記憶
- NPC↔NPC 對話的設計目標：眾口鑠金——NPC 群體透過對話堆疊出世界觀

## 世界觀關鍵詞
- 沃土風
- 鎂（貨幣單位）
- 詞盤、竅穴、念紋（修煉體系）
- 量子坍縮（觀測機制）

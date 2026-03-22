# Config & text assets（專檔維護）

**參數有哪些、各檔分工**：見同目錄 [`PARAMETERS_INDEX.md`](./PARAMETERS_INDEX.md)（依目前程式實際抽出項目整理）。

## 文案與提示詞

| File | Role |
|------|------|
| `data/config/gametext.json` | 遊戲內與 API 字串：WebSocket 錯誤、NPC 社交前綴、dyad 標籤、對話評分詞表、清理標記、經濟傳聞句、移動文案、大腦到達／事件日誌範本等。啟動時由 `main` 呼叫 `gametext.MustLoad()`。 |
| `data/templates/llm_prompts.json` | 給 LLM 的 system／user 區塊（玩家↔NPC、NPC↔NPC）、世界現象段落、meta 過濾、折疊分隔符。由 `ai/prompts.go` 延遲載入。 |
| `data/templates/PLAYER_TALK_WEB_LLM.md` | 玩家↔NPC **分場景回答規則**條列（給企劃／Web 助理對照）；**不**自動進 API，須與 `llm_prompts.json` 同步。語用背景見 [`玩家NPC對話與交際語用.md`](../reference/玩家NPC對話與交際語用.md)。 |

**原則**：可編輯內容放在 JSON；Go 只做載入、`fmt.Sprintf` 與接線。註解僅供人讀，不等於送進模型的提示詞。

## 可調參數（數值／伺服器預設）

| File | Role |
|------|------|
| `data/config/server_defaults.json` | 設計常數（cell／role／terrain 像素）、埠號、WS 上限、tick、chunk、地圖路徑、session 分鐘、遊戲時間倍率、NPC pool、生成間隔、Ollama、求職／NPC↔NPC 相關欄位等。`config.DefaultServer()` 優先讀此檔，失敗用內建 struct，再以環境變數覆寫。 |
| `data/config/simulation.json` | 主迴圈間隔、NPC 社交／傳聞／配對計分、dyad、乞討鎂區間、經濟 pulse、idle、隨機 NPC 對話 tick、求職輪詢、travel、巡邏／微互動機率、時段 disposition 等。`main` 啟動時 `config.LoadSimulation("")`；程式內用 `config.Sim()` 讀取。 |

其他套件（例如 `db/`、`server/`）內若仍有寫死常數，之後可再掃描並決定是否併入上述 JSON 或另開專檔。

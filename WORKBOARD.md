# 工作板

> 最後更新：2026-04-17
> 這不是文檔，是桌上攤開的紙。每次對話開始、上下文壓縮時自動注入。

## 現狀診斷

| 項目 | 狀態 |
|------|------|
| 底層 | 還是 hex（91 格 axial、8 NPC 在 hex 活動）— 方格層尚未動工 |
| 玩家端 | canvas hex 地圖還掛著，**已決定棄用**，改純文字 MUD |
| 觀景窗 | earth.ygggt.com 正常（除錯用，繼續吃 hex） |
| UI v2 骨架 | `web/grid-map.js` 紙色 DOM 地圖骨架在（2026-04-11 建），需接點擊 + 連線 |
| NPC 行為引擎 | `hex_ai.rs` — 需求驅動 + BFS + 真實採集扣量 |
| 資源恢復 | density_multiplier 已接（0人×3、1人×1、2人×0.6、3+人×0.3） |
| 歷史模擬器 | 規格 v2 + axioms 骨架完成（2026-04-17），獨立 repo 待建，未動工 |

## 方向轉折

圖形地圖出戲。回歸文字——MUD 式純文字體驗。
hex 六方沒有「北」和「南」，文字裡不自然。底層改方格，但**渲染不是棋盤，是「方格 + 連線」**。

## 已拍板設計（2026-04-16 收斂）

### 拓撲：方格 + 連線（不是棋盤）

```
【森 林】—【山 脈】—【森 林】
    |                |
【森 林】—【小 屋】—【草 原】
    |        |
```

- 格子之間用線連接，線就是出口
- 玩家當前格置中
- 格內顯示當前格名稱（地形名或地標名）

### 揭露系統

- **未探索一律不顯示**
- **探索是主動動作**，按下去才揭露當前格的連線出口
- **一旦揭露永久釘死**（不會再隱藏）
- **物件欄動作**（採集、伐木、採礦等）也需靠探索才揭露
- **地上物進格即見**（不需探索，玩家丟的物品也一樣看得到）
- **在場者進格即見**（在場 NPC 不需探索）

### 物件欄：LPC MUD「萬物皆物件」

物件欄包含（按探索狀態）：
- 在場 NPC（進格即見）
- 地上物品（進格即見）
- 玩家物品（永遠在）
- 資源動作：採集／伐木／採礦／取水（**需探索揭露**）
- 「探索」動作本身（永遠在）

### UI 佈局

```
┌─────────────────────────────┐
│         日晷時間（固定）       │
├───────┬─────────────────────┤
│       │   房間描述（上小）    │
│ 物件欄 ├─────────────────────┤
│ （窄） │   地圖（下大）        │
│       │   點擊鄰近格移動      │
├───────┴─────────────────────┤
│         姓名／狀態（固定）     │
└─────────────────────────────┘
```

### 移動：點擊地圖

- 單擊鄰近出口格 → 移動到下一格（指尖 MUD）

## 已定案（2026-04-16 收斂完成，所有 OPEN Q 已拍板）

- [x] **Q1 方向鍵 → 不存在**。整個 UI 沒有獨立方向鍵元件，點擊鄰近格本身就是方向輸入
- [x] **Q2 進探分層 → 確認**。進入只看地上物+在場者，主動探索才揭露出口和資源動作
- [x] **Q3 舊 hex 揭露 → 降級 deprecated**。保留檔案但標作廢，五個跨格修正坑仍有技術參考

## 核心任務（重排）

### 第一階：方格引擎（底層）— 未開始
- [ ] `src/grid/` 模組：SquareCoord(x,y)、SquareGrid、鄰居、BFS 尋路
- [ ] 地形系統復用（Terrain enum 不動）
- [ ] 資源物件復用（HexObject → GridObject）
- [ ] 世界生成：seed → (x,y) 確定性地形 + 資源
- [ ] PG 表 `grid_cells(x, y, terrain, zone, objects_json, ...)`
- [ ] **新增 `grid_player_reveal` 表**（對齊現有 hex_player_reveal，永久釘死語義）

### 第二階：NPC 接方格 — 未開始
- [ ] `hex_ai.rs` → `npc_grid_ai.rs`：HexCoord → SquareCoord
- [ ] entity 表加 `grid_x, grid_y`（不動 hex_q/hex_r，保留給觀景窗）
- [ ] NPC spawn 寫入方格座標（SW-2）
- [ ] simulation_loop 接 grid tick

### 第三階：文字前端 — 被 UI 設計擋住（SW-3 blocked 已解，可推進）
- [ ] 按上拍板 UI 佈局實作（`grid-map.js` 已有骨架，需接連線 + 點擊）
- [ ] 房間描述：地形 × 時段氛圍句（156 句模板）+ 出口描述 + 在場者
- [ ] 物件欄：依萬物皆物件概念列出 NPC / 地上物 / 玩家物 / 揭露後的動作
- [ ] 地圖區：格子 + 連線 DOM 渲染，點擊移動
- [ ] 「探索」動作 UI 接資料層揭露
- [ ] WebSocket：移動指令改接點擊事件（不靠方向指令）
- [ ] 砍 canvas.js 渲染

### 第四階：文字品質
- [ ] 地形 × 12 時段氛圍句 156 句（手填，不用 AI）
- [ ] NPC 行為敘事模板（採集、漫遊、發呆、抵達、離開）
- [ ] 環境氛圍（時段、天氣、人數影響描述）
- [ ] 玩家動作回饋文字

## 跨對話 TODO（shodh-memory）

| ID | 狀態 | 內容 |
|----|------|------|
| SW-1 | ○ | ~~方向 enum 8→4~~ **已作廢**（新設計用點擊移動，不列舉方向） |
| SW-2 | ○ | NPC spawn 寫入方格座標 |
| SW-3 | ✓→○ | 純文字 MUD 前端 UI 實作（blocker 已解，可推進） |
| SW-6 | ✓ | WORKBOARD.md 更新（2026-04-17 納入歷史模擬器進度）|
| SW-7 | ○ | 歷史模擬器 epoch_seed.toml 雛形（Opus 待寫，下一步）|
| SW-8 | ○ | 歷史模擬器 M0 骨架（新 repo + L1 純地理層，碼農領域）|

## 不做（等文字版活了再說）

- 飢餓/死亡
- 製造/配方
- 功能格行為（Blacksmith 等）
- 物品品質/工具門檻
- NPC 對話（LLM）——先讓世界有文字質感，再開口說話
- 戰鬥

## 110V 原則（不變）

Token 物理是敘事層，代碼只用通則：
- 有人→恢復慢，沒人→恢復快（density_multiplier）
- 沒有冷卻計時器，人走了下一 tick 立刻倍速
- 不需要 token_density 變數
- 採集物品名稱直接用 GridObject.name

## 關鍵檔案

| 檔案 | 狀態/改什麼 |
|------|--------|
| `src/grid/`（新） | 方格座標、鄰居、BFS、地形、資源 |
| `src/npc/hex_ai.rs` | 已完成 hex 版，需 fork 為 `npc_grid_ai.rs` |
| `src/npc/decision.rs` | 評分引擎座標無關，不用動 |
| `src/server/simulation_loop.rs` | 接 grid tick |
| `src/game/room.rs` | RoomView 生成改接 square grid + 揭露狀態 |
| `web/main.js` | 砍 canvas，改 DOM 地圖 + 點擊移動 |
| `web/grid-map.js` | 2026-04-11 骨架在，需加連線渲染 + 點擊事件 + 揭露 |
| `web/game-ui.js` | 物件欄、動作選單、stats bar（已存在） |
| `src/hex/`（保留） | 觀景窗繼續用，不動 |

## 阻塞與疑問

- entity 表：新建 `grid_x/grid_y`（不動 hex_q/hex_r，hex 欄位留給觀景窗）
- 觀景窗不同步方格世界，兩套座標系獨立
- 舊 RoomGraph 暫留不碰，NPC 遷到新 grid 後自然萎縮
- Q1/Q2/Q3 已全部拍板（見「已定案」區），無 OPEN 阻塞

## 並行專案：歷史模擬器

獨立 Rust binary（未來 repo `/home/tanger/Projects/singularity_simulator/`，未建）。跑解放日 → 混沌紀 500 年 → 凍結世界初始快照 → 灌回主遊戲 PG。

### 2026-04-17 進度
- 規格 v2 完成：`docs/design/歷史模擬器—規格草案.md`（680 行，整合 28 條世界觀衝擊：世界觀公理章/陣營架構章/敘事硬約束章/Hidden Director 層）
- **新 repo 建好骨架**：`/home/tanger/Projects/singularity_simulator/`
  - `Cargo.toml`（依賴：rusqlite + toml + h3o + reqwest/tokio + clap）
  - `src/main.rs`（CLI 分派骨架，init/run/inspect/snapshot/replay/extract-legends）
  - `src/lib.rs`（9 個模組宣告：axioms/geo/population/agent/director/event/store/llm/time）
  - `axioms/token_physics.toml` + `tribes.toml` + `epoch_seed.toml`（由 draft 搬入）
  - `migrations/0001_initial_schema.sql`（完整 SQLite DDL + 玩家可見層過濾 view）
  - `samples/tick0_events.jsonl`（11 筆樣本：五話 tick 0 既成事件 + 虛擬第一年事件）
  - `prompts/prompt_context_template.md`（LLM 跑量硬契約模板，7 項 context 組裝規範）
  - `README.md` + `.gitignore`
- **未做**：cargo build 驗證、git init、L1 實作（碼農領域）

### 里程碑
M0 骨架+L1 純地理層跑 500 年 → M1 L2 人口流體 → M2 L3 陣營 Agent → M3 聚落 Agent + L3.5 Hidden Director → M4 snapshot 灌回主遊戲 PG → M5 Web UI 時間軸

### 與主遊戲對接
M4 時 snapshot 灌回 PG，方格 MUD 才切換到新世界狀態。現階段主遊戲仍用 hex 觀景窗既有世界，兩者並行不相衝。

### 敘事外放
三線分工：Opus 骨幹潤（30-100 骨幹事件）+ Sonnet 4.6 校對 + Qwen3.6 Plus 跑量。離線手工流程，不進模擬器 pipeline。五話為正典錨點（`docs/stories/001_降臨.md` ~ `005_是解放日.md`），續篇 `006_是親子日.md` / `007_是送別日.md`。禁用詞表/允許模式清單/第四話紅光 few-shot 為硬契約。

### 底牌（不寫進規格）
- 煙火日 = 母腦誕生事件的具體歷史身份
- 詞盤系統除熵教外的其餘源流（熵教為已知源流之一）

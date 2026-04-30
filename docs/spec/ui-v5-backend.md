<!-- ===== 四查證據包（spec-writer 自填，主 agent 驗收用）===== -->

### 查 1：主入口
- 檔案：`src/server/http_api/mod.rs`
- 關鍵段（全文）：
  ```
  mod admin; mod player; mod rooms; mod eco;
  pub use admin::{design_constants, wipe_entities, AdminQuery};
  pub use player::{player_room, topology};
  pub use eco::{eco_map, eco_entities};
  pub use rooms::{add_exit, create_room, delete_room, ...};
  ```
- 結論：`/api/explore`、`/api/time` **不存在**，`freshness`、`climate`、`acquaintance` 均不存在——全部是新增，無退場/隱藏風險

### 查 2：git log
- `src/server/http_api/player.rs`
  - 近 20 commits：`1ae00a44 refactor: 砍 hex 觀景窗全部` / `ef2e22a0 refactor(server/http_api): 按域拆分`
  - 可疑 commits：`1ae00a44`「砍 hex 觀景窗全部」→ hex 相關退場，**方格 player API 活著**
- `src/server/simulation_loop/`
  - 近 commits：`3aa2bcf2 feat(grid): Manhattan 四向 + 建築隔離法則 + GridNpcManager 重寫` / `6241d79a refactor(server/simulation_loop): 按節拍分層拆分`
  - 無退場/deprecate 訊號
- `src/grid/`
  - `3aa2bcf2`（最新）Manhattan 四向拍板，`GridCell` 只含 `explored: bool`，**無 freshness**
- `src/game/gametime.rs`
  - `game_time_now()` 活著，回傳 `(sec, hour, min, days)`，hour 0-23

### 查 3：對齊判斷
- 主 agent 意圖：為 v5 UI 加探索 endpoint、時辰/氣候/物件觀察態後端
- 當前實際運作代碼：HTTP API 無 explore/time；`GridCell` 無 freshness；DB 無 acquaintance 表；`gametime.rs` 有 hour 0-23 基礎
- 對齊：**是**——全部為新增，無退場衝突

### 查 4：現存 spec
- 已搜：`ls docs/spec/` → `SW-22_room_payload地形與地標補強.md`、`ui-v2-step1-2.md`
- 衝突/重複：無（SW-22 管 GridCellView payload 欄位，本規格擴充不同欄位；ui-v2-step1-2 管前端 step）

<!-- ===== 證據包結束，以下為規格內容 ===== -->

---

# ui-v5-backend：後端 v5 API 規格

> 對應 UI 規格版本：v5（拍板 2026-04-29）
> source of truth：`obsidian-vault/wiki/UI-規格.md`

## 背景

UI v5 拍板了五方一體路口 UI + 描述欄四層結構 + 探索深掘模型。後端目前缺：
- 探索 endpoint（`/api/explore`）
- 時辰查詢 endpoint（`GET /api/time`）
- 單格新鮮度（`freshness`）欄位
- 24 時辰態分段定義
- 區域氣候系統
- 物件卡片觀察態持久化表
- WebSocket 訊息分類補強（log vs state-update）

## 改動範圍

| 檔案 | 改什麼 |
|---|---|
| `src/grid/cell.rs` 或 `src/grid/grid.rs` | `GridCell` 加 `freshness: Freshness` 欄位（enum） |
| `src/server/http_api/player.rs` | 新增 `explore` handler（POST /api/explore） |
| `src/server/http_api/mod.rs` | pub use 新 handler，路由掛載 |
| `src/server/run.rs` | 路由表加 POST /api/explore、GET /api/time |
| `src/server/http_api/player.rs`（或新拆模組） | 新增 `game_time` handler（GET /api/time） |
| `src/server/protocol.rs` | 新增 `ExploreLogMsg`、`StateUpdateMsg` 兩個 struct |
| `src/server/simulation_loop/world.rs`（或新子模組） | freshness 回復 tick、氣候事件推進 |
| `src/game/gametime.rs` | 新增 `time_period(hour: i32) -> TimePeriod` 函式（純計算，不改現有函式） |
| DB migration（新檔） | `grid_cells` 加 `freshness` 欄位；新建 `players_npc_acquaintance` 表；新建 `region_climate` 表 |

## 詳細規格

---

### 1. 新 endpoint：POST /api/explore

#### 請求

```
POST /api/explore
Content-Type: application/json

{
  "player_id": "<player-id>",
  "password":  "<password>"
}
```

驗證：同現行 `player_room` 的 `db::verify_password`。

#### 後台多軸抽取

抽取輸入（後端讀，前端不送）：

| 軸 | 來源 | 說明 |
|---|---|---|
| 地形 | 玩家當前格 `GridCell.terrain` | 決定資源池與生物池 |
| 時辰段 | `TimePeriod::from_hour(game_hour)` | 見第 4 節定義 |
| 氣候 | `region_climate` 表查玩家所在區域 | 見第 5 節 |
| 人氣 | 玩家當前格 `density_multiplier`（現有欄位） | 無人＝肥沃、剛被探＝撲空傾向 |
| 玩家詞盤 | `activated_nodes`（現有欄位） | 影響結果類型與強度，**具體公式留 placeholder** |
| 新鮮度 | 玩家當前格 `GridCell.freshness` | 見第 2 節 |

#### 結果三類

```
採集成功   → outcome: "harvest"
戰鬥遭遇   → outcome: "combat"
空手/異象  → outcome: "empty"
```

#### 回傳 JSON

```json
{
  "outcome": "harvest",
  "log":     "撥開枯葉，拾得野莓三顆，露水未乾。",
  "loot": [
    { "item_id": "<item-id>", "name": "<item-name>", "qty": 3 }
  ],
  "stat_delta": {
    "hp_cur": 0,
    "inner_cur": 0
  },
  "time_period_after": "晝前",
  "freshness_after": "half",
  "warning_prose": ""
}
```

欄位說明：
- `loot`：採集成功時非空；戰鬥/空手時為空陣列
- `stat_delta`：屬性變化量（戰鬥扣血、昏倒事件填值）；無變化全為 0
- `time_period_after`：探索後的時辰段名稱（前端更新 L1）
- `freshness_after`：探索後該格新鮮度（`"fresh"` / `"half"` / `"empty"`）
- `warning_prose`：若本次探索前抽到「預兆」散文則填入，前端顯示給玩家做進退選擇；**空字串代表無預兆，直接執行**

#### 預兆邏輯（預警可避）

- 每次 explore 先做一輪「預兆抽取」：當地形/時辰組合的戰鬥概率超過設定閾值（配置項，預設 30%）時，有機率（另一配置項）觸發預兆
- 觸發預兆時：**不執行完整探索**，只推進時辰成本（見第 2 節 A 軸），回傳 `outcome: "warning"`，`warning_prose` 填預兆散文，其他欄位為空
- 玩家收到 warning 後：可再次 POST /api/explore → 強制執行（後端識別連續同格 warning 後的下一次為「確認進入」，不再觸發預兆，直接結算）
- 實作提示：可在 session 或請求中帶 `confirmed: true` 欄位，後端讀此欄位決定跳過預兆

#### 戰鬥結算（一探到底）

- 遭遇後自動結算，全程後端計算，無中途 [逃] endpoint
- 結算結果寫入 `stat_delta`
- 死亡兜底：`hp_cur <= 0` → 觸發昏倒事件（見第 3 節死亡 110V）
- 戰鬥 `log` 散文需帶玩家詞盤元素，具體詞盤→散文映射**留 placeholder**，碼農填空

---

### 2. 探索節流雙軸

#### A. 時辰成本

每次 POST /api/explore（包含 `outcome: "warning"` 的預兆探索）：

- 後端讀當前 `game_time_now()` 的 `hour`，計算目前所在 `TimePeriod`（0-5，共 6 段，見第 4 節）
- `time_period_index` +1（mod 6），寫回**玩家實體的遊戲時間偏移**
- 實作方式：玩家表加 `time_offset_periods: i32` 欄位（預設 0），`game_hour` 計算時加上此偏移再 mod 24；世界時辰繼續正常走，玩家個人「體感時辰」超前
- 前端從 `time_period_after` 欄位得知推進後的時辰段

#### B. 單格新鮮度（110V 布林擴展）

`GridCell` 加欄位：

```rust
pub freshness: Freshness,
```

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Freshness {
    Fresh,
    Half,
    Empty,
}
```

- 同一玩家同一格連續 explore：`Fresh → Half → Empty`
  - 由後端在 explore handler 裡讀當前值、結算後寫新值
  - 抽取池權重：`Fresh` 正常、`Half` 採集量 ×0.5、`Empty` 強制 `outcome: "empty"`
- 玩家**離格**（移動到其他格）→ 下一個 simulation tick 回 `Fresh`
  - simulation_loop 裡加一段：掃所有 `freshness != Fresh` 的格，若無玩家在場則回 `Fresh`
  - 110V：純布林切換，不計時不計數
- 資料庫：`grid_cells` 表加 `freshness TEXT NOT NULL DEFAULT 'fresh'`（或 enum column）

---

### 3. 戰鬥四條邊界

（已在第 1 節展開實作面，此處作行為驗收錨）

1. **預警可避**：`outcome: "warning"` + `warning_prose` 非空 → 玩家可看散文後選退出，不再次呼叫即視為退出；再次呼叫帶 `confirmed: true` 視為確認
2. **一探到底**：後端無 `/api/flee`、`/api/combat_action` endpoint；戰鬥全後端結算
3. **詞盤投射**：戰鬥 `log` 散文必須含 `<詞盤元素>` placeholder（碼農先以 `activated_nodes[0]` 代入格式字串，設計者後補具體描述池）
4. **死亡 110V**：`hp_cur <= 0` → `stat_delta.hp_cur` 設為 `1`（兜底不死）；回傳 `outcome: "faint"`；觸發「昏倒」散文；後端執行傳送至最近聚落（現有 `move_by_grid_direction` 邏輯延伸）；掉部分物品（掉落清單寫入 `loot` 欄位，qty 為負代表失去）

---

### 4. 24 時辰系統

#### TimePeriod 定義

在 `src/game/gametime.rs` 新增純函式：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePeriod {
    Dawn,       // 拂曉  hour 4-7
    Forenoon,   // 晝前  hour 8-10
    Noon,       // 正晝  hour 11-14
    Dusk,       // 黃昏  hour 15-17
    Night,      // 夜段  hour 18-22
    Predawn,    // 黎明前 hour 23-3
}

pub fn time_period(hour: i32) -> TimePeriod { ... }
pub fn time_period_label(p: TimePeriod) -> &'static str { ... }
```

小時範圍邊界為設計建議，碼農可在配置中調整。

#### tick 推進

- 現有 `game_time_now(real_unix, epoch, scale)` 已提供連續時間，**不需另建 tick 計時器**
- `time_period(hour)` 是純映射函式，回傳離散段位
- 「1 個現實秒 = 多少遊戲秒」由現有 `cfg.game_time_scale` 控制，不新增配置項

#### GET /api/time

```
GET /api/time?player_id=<id>
```

回傳：

```json
{
  "game_hour":        22,
  "game_min":         15,
  "time_period":      "夜段",
  "time_period_idx":  4,
  "server_unix":      1700000000
}
```

用途：前端 L1 層定期輪詢（或 WS state-update 包含時辰變化時主動重繪）。

---

### 5. 範圍氣候系統（L2）

#### 設計原則

- 氣候是**區域屬性**，不是格屬性
- 變化週期：小時級（慢變），不跟 explore 掛鉤
- 氣候事件：`連日陰雨` / `北風南下` / `入梅` / `初雪封山`

#### 資料庫：新表 `region_climate`

```sql
CREATE TABLE region_climate (
    region_id       TEXT PRIMARY KEY,
    event_type      TEXT NOT NULL DEFAULT 'clear',  -- clear/rain/wind/plum_rain/snow
    started_at_unix INTEGER NOT NULL,
    prose_seed      INTEGER NOT NULL                -- 散文池抽取種子，前端用
);
```

`region_id` 對應 `GridCell.zone`（現有欄位）。

#### 氣候推進

- simulation_loop 每 N 分鐘（配置項，建議 60 現實秒）觸發一次氣候 tick
- 氣候 tick：對每個 region 抽是否發生轉變（概率由配置決定）；發生轉變 → UPDATE region_climate
- 氣候變化時，simulation_loop 廣播 `StateUpdateMsg`（見第 7 節），前端收到後重抽 L2 散文

#### GET /api/climate

```
GET /api/climate?region_id=<region-id>
```

回傳：

```json
{
  "region_id":   "<region-id>",
  "event_type":  "rain",
  "prose_seed":  12345
}
```

前端用 `prose_seed` 從本地散文池抽取散文，不由後端生成具體文字。

---

### 6. 物件卡片觀察態

#### 資料庫：新表 `players_npc_acquaintance`

```sql
CREATE TABLE players_npc_acquaintance (
    player_id   TEXT NOT NULL,
    npc_id      TEXT NOT NULL,
    observed    BOOLEAN NOT NULL DEFAULT FALSE,  -- 點卡片觀察過
    named       BOOLEAN NOT NULL DEFAULT FALSE,  -- 交談自報名
    PRIMARY KEY (player_id, npc_id)
);
```

#### 行為規則

| 動作 | 觸發 | DB 變化 |
|---|---|---|
| 玩家首次同格見 NPC | 自動（進格/NPC 進場） | INSERT OR IGNORE（兩欄均 false） |
| 玩家點卡片「觀察」 | 物件互動 action=observe | `observed = true` |
| NPC 交談自報名 | 物件互動 action=talk + NPC 有名字屬性 | `named = true` |

#### 前端渲染（後端只給狀態）

後端在 `ViewEntity` 加欄位：

```rust
#[serde(rename = "observed")]
pub observed: bool,
#[serde(rename = "named")]
pub named: bool,
```

- `observed=false, named=false` → 前端渲染「行人」類標籤
- `observed=true, named=false` → 前端渲染職業標籤（NPC 的 `display_title` 欄位）
- `observed=true, named=true` → 前端渲染「職業＋姓名」
- **甲乙丙編號**：純前端運算，依當格在場 NPC 的標籤桶排序，後端不提供

---

### 7. WebSocket 推送訊息補強

#### 兩類訊息性質分清

| 訊息類型 | 觸發 | 前端行為 |
|---|---|---|
| `log`（現有 `NarrateMsg`） | 玩家動作（explore/互動） | 追加到 log buffer |
| `state_update`（新增） | 世界自走（NPC 移動、時辰推進、氣候變化） | 只更新 L1-L4 狀態，不入 log buffer |

#### 新增 StateUpdateMsg

在 `src/server/protocol.rs` 新增：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateUpdateMsg {
    #[serde(rename = "type")]
    pub msg_type: String,            // 固定值 "state_update"
    pub trigger: String,             // "npc_move" | "time_advance" | "climate_change"
    pub payload: serde_json::Value,  // 具體變化內容（前端按 trigger 解析）
}
```

`payload` 範例：

```json
// trigger: "time_advance"
{ "time_period": "夜段", "time_period_idx": 4 }

// trigger: "climate_change"
{ "region_id": "<region-id>", "event_type": "rain", "prose_seed": 12345 }

// trigger: "npc_move"
{ "npc_id": "<npc-id>", "left_cell": true }
```

#### ExploreLogMsg（玩家動作回應）

在 `src/server/protocol.rs` 新增（亦可改用現有 `NarrateMsg` 加欄位，碼農擇一）：

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExploreLogMsg {
    #[serde(rename = "type")]
    pub msg_type: String,            // 固定值 "explore_log"
    pub outcome: String,             // "harvest" | "combat" | "empty" | "warning" | "faint"
    pub log: String,                 // 散文段
    pub loot: Vec<LootItem>,
    pub stat_delta: StatDelta,
    pub time_period_after: String,
    pub freshness_after: String,
    pub warning_prose: String,
}
```

---

### 8. 資料庫遷移

三個異動需獨立 migration 檔（`migrations/` 目錄，按現有命名慣例）：

#### M1：grid_cells 加 freshness

```sql
ALTER TABLE grid_cells ADD COLUMN freshness TEXT NOT NULL DEFAULT 'fresh';
```

#### M2：新建 players_npc_acquaintance

```sql
CREATE TABLE players_npc_acquaintance (
    player_id   TEXT NOT NULL,
    npc_id      TEXT NOT NULL,
    observed    BOOLEAN NOT NULL DEFAULT FALSE,
    named       BOOLEAN NOT NULL DEFAULT FALSE,
    PRIMARY KEY (player_id, npc_id)
);
```

#### M3：新建 region_climate

```sql
CREATE TABLE region_climate (
    region_id       TEXT PRIMARY KEY,
    event_type      TEXT NOT NULL DEFAULT 'clear',
    started_at_unix INTEGER NOT NULL,
    prose_seed      INTEGER NOT NULL
);
```

#### players 表加 time_offset_periods

```sql
ALTER TABLE players ADD COLUMN time_offset_periods INTEGER NOT NULL DEFAULT 0;
```

---

## 驗收標準

- [ ] `POST /api/explore` 回傳包含 `outcome`、`log`、`loot`、`stat_delta`、`time_period_after`、`freshness_after`
- [ ] 同格連續三次 explore：第一次 freshness 為 fresh，第二次 half，第三次 empty 且 outcome 強制 empty
- [ ] 玩家移動離格後，下一個 simulation tick 該格 freshness 回 fresh（驗收方式：DB 查 grid_cells）
- [ ] `GET /api/time` 回傳 `game_hour`、`time_period`，時辰段對應正確（拂曉 hour 4-7、夜段 hour 18-22）
- [ ] `GET /api/climate` 回傳指定區域的氣候事件，氣候變化後 WS 推送 `state_update` type=climate_change
- [ ] `players_npc_acquaintance` 表：觀察動作後 `observed=true`；NPC 自報名後 `named=true`；跨格跨登入持久
- [ ] WS 推送：explore 產生的 log 訊息 type 為 `explore_log`（或 `narrate`）；NPC 移動產生 type 為 `state_update`，前端 log buffer 不收 state_update
- [ ] 死亡兜底：`hp_cur <= 0` 後 outcome=faint，玩家傳送至最近聚落，`hp_cur >= 1`

## 不做

- 前端代碼、CSS、散文描述池內容
- 詞盤→戰鬥公式（留 placeholder 給設計者後補）
- 氣候對採集量的具體加成公式（配置項留空，調參階段填）
- 具體時辰散文（見另份描述池 spec 或設計者直接填 DB）
- `/api/flee`、`/api/combat_action` 等中途戰鬥 endpoint（明確不做）
- 地圖 API（地圖是世界內物件，不是介面）

## 占位符對照

- `<player-id>` — 玩家實體 ID，對應 `players` 表主鍵
- `<item-id>` — 物品 ID，對應現有物品表主鍵
- `<item-name>` — 物品顯示名
- `<npc-id>` — NPC 實體 ID
- `<region-id>` — 區域識別碼，對應 `GridCell.zone`
- `<詞盤元素>` — 戰鬥散文中的詞盤投射 placeholder，碼農先用 `activated_nodes[0]` 代入，設計者後補描述池

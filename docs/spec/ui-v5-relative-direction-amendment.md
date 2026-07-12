# ui-v5-relative-direction-amendment：路口 UI 從絕對方位改相對方位

> 類型：修訂規格（對 ui-v5-frontend.md / ui-v5-backend.md 的差異補丁）
> 拍板日：2026-04-30
> 優先於：ui-v5-frontend.md 第 5 節（路口形狀 UI）、第 7 節（WS 訊息格式）；ui-v5-backend.md 第 1 節 exits_v5 組裝

---

<!-- ===== 四查證據包（spec-writer 自填，主 agent 驗收用）===== -->

### 查 1：主入口
- 檔案：`web/index.html`（由 ui-v5-frontend.md 四查證據包確認，commit `78d4cfbf` 已實作 v5 三區）
- 關鍵段（grid-map.js 行 21-24、302-322）：
  ```
  行 21-24：
    '北': 'exit-north',
    '南': 'exit-south',
    '東': 'exit-east',
    '西': 'exit-west'
  行 302-305：
    exitEls.north = document.getElementById('exit-north');
    exitEls.south = document.getElementById('exit-south');
    exitEls.east  = document.getElementById('exit-east');
    exitEls.west  = document.getElementById('exit-west');
  行 320-322：
    window.gameSend({ type: 'move', direction: moveDir });
    window.gameSend({ type: 'scout', direction: scoutDir });
  ```
- 結論：四個出口槽 `exit-north/south/east/west` **活著**，是本次要改的目標

### 查 2：git log
- 目標檔 1：`web/grid-map.js`
  - 近 20 commits（相關）：
    - `78d4cfbf` feat(ui-v5): 路口形狀 UI 實作，建立 exit-north/south/east/west DOM 槽
    - `b5bf8ca9` chore(ui-v5): 砍前端 v5 前殘留地圖死碼
    - `95699acb` refactor(web): 拆 grid-map.js
  - 可疑 commits：無退場訊號，exit-north/south/east/west 是 78d4cfbf 新增的活代碼
  - diff 看過：是（確認 exitEls 物件鍵為 north/south/east/west）

- 目標檔 2：`src/server/handler/movement.rs`
  - 近 commits：
    - `d3970e3c` feat(explore): exits_v5 組裝邏輯在此
    - `3aa2bcf2` feat(grid): Manhattan 四向拍板
  - 可疑 commits：無
  - diff 看過：是（exits_v5 用 `("north", 0, 1), ("south", 0, -1), ("east", 1, 0), ("west", -1, 0)` 四元組組裝）

- 目標檔 3：`src/grid/coord.rs`
  - 近 commits：`3aa2bcf2`（最新）Manhattan 四向；Direction enum North/East/South/West **活著**
  - 可疑 commits：無退場
  - diff 看過：是（Direction enum 有 opposite() 方法，可用於 back 計算）

- `facing` 欄位：全專案 grep 無任何 `facing` 或 `entry_from` 欄位存在

### 查 3：對齊判斷
- 主 agent 意圖：前端四槽改 back/forward/left/right 相對方位；後端在 exits_v5 組裝時依玩家 facing 旋轉；Player entity 加 `facing` 欄位
- 當前實際運作代碼：exits_v5 組裝在 `movement.rs` 行 297-319（絕對方位四元組），前端 exitEls 以 north/south/east/west 為鍵，兩者均活著
- 對齊：**是** — 要改的代碼存在且在運作；facing 欄位不存在但這是新增不是對退場代碼操作

### 查 4：現存 spec
- 已搜：`grep -rn "north|south|east|west|絕對|方位" docs/spec/`
- 衝突段：
  - `ui-v5-frontend.md` 行 315-316：WS 發送 `direction: "north"|"south"|"east"|"west"` → 本修訂替換為 `"back"|"forward"|"left"|"right"`
  - `ui-v5-frontend.md` 行 333-336：`room_enter.exits` 鍵為 north/south/east/west → 本修訂替換為 back/forward/left/right
  - `ui-v5-backend.md`：exits_v5 組裝邏輯需追加 facing 旋轉步驟
  - 衝突處理：本修訂規格**優先**，碼農照本修訂規格實作；ui-v5-frontend.md / ui-v5-backend.md 對應段落以本檔為準

<!-- ===== 證據包結束，以下為規格內容 ===== -->

---

## 背景

v5 路口形狀 UI 初版用北/南/東/西絕對方位標定四槽。設計者 2026-04-30 拍板：四槽改為**相對於玩家進入方向**（來向/前方/左/右），保留十字版面拓撲。理由：絕對方位讓玩家三秒建立 2D 地圖座標系，違背「擺脫地圖思維陷阱」初衷。相對方位迫使玩家記住「過三個路口左轉」，不是「村口在世界東南」。

---

## 改動範圍

| 檔案 | 改什麼 |
|---|---|
| `src/grid/coord.rs` | 新增 `RelativeDir` enum（Back/Forward/Left/Right）+ `Direction::to_relative(facing)` + `RelativeDir::to_absolute(facing)` 兩個轉換函式 |
| `src/server/handler/movement.rs` | exits_v5 組裝段：依玩家當前 `facing` 把四絕對方位旋轉成 back/forward/left/right；移動成功後更新玩家 `facing` |
| `src/db/`（players 表相關模組） | `players` 表加 `facing TEXT NOT NULL DEFAULT 'north'`；新增 `get_player_facing(player_id)` / `set_player_facing(player_id, dir)` |
| `web/grid-map.js` | `exitEls` 鍵改 back/forward/left/right；DOM id 改 `exit-back/forward/left/right`；WS 發送 `direction: "back"|"forward"|"left"|"right"` |
| `web/index.html` | 路口四槽 div id 改名（見前端節） |

**不改**：
- `Direction` enum 本體（North/East/South/West 保留，後端內部仍用絕對方位）
- `grid_cells` 表（(x,y) 方格拓撲不動）
- L1-L3 描述欄邏輯
- 探索節流 / freshness / 氣候邏輯

---

## 1. 後端改動

### 1-A. Player 加 `facing` 欄位

**DB migration（新增 migration 檔）：**

```sql
ALTER TABLE players ADD COLUMN facing TEXT NOT NULL DEFAULT 'north';
```

`facing` 值域：`'north' | 'east' | 'south' | 'west'`（對齊現有 Direction enum 的 snake_case 序列化）

**Rust 層：**
- `src/db/` 相關模組新增：
  - `get_player_facing(player_id: &str) -> Result<Direction>`
  - `set_player_facing(player_id: &str, dir: Direction) -> Result<()>`

**Spawn 預設 facing：**
玩家創角或首次 spawn 時，`facing` 寫入 `Direction::North`（預設面北）。碼農在現有 spawn/初始進格邏輯處補一次 `set_player_facing` 呼叫。

### 1-B. `coord.rs` 新增旋轉工具

在 `src/grid/coord.rs` 新增：

```rust
/// 相對方位（玩家視角）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RelativeDir {
    Back,
    Forward,
    Left,
    Right,
}

impl Direction {
    /// 把絕對方位轉換成相對於 `facing` 的相對方位。
    /// facing = 玩家當前面向（剛才移動過來的方向）
    /// back = facing 的反向（來路）
    /// forward = facing 本身（繼續直走）
    /// left/right = facing 的左右
    pub fn to_relative(self, facing: Direction) -> RelativeDir {
        // facing 定義「前方」，以順時針 0/1/2/3 算偏移
        let order = [Direction::North, Direction::East, Direction::South, Direction::West];
        let facing_idx = order.iter().position(|&d| d == facing).unwrap_or(0);
        let self_idx   = order.iter().position(|&d| d == self).unwrap_or(0);
        match (self_idx + 4 - facing_idx) % 4 {
            0 => RelativeDir::Forward,
            1 => RelativeDir::Right,
            2 => RelativeDir::Back,
            3 => RelativeDir::Left,
            _ => unreachable!(),
        }
    }
}

impl RelativeDir {
    /// 把相對方位轉回絕對方位（依當前 facing）。
    pub fn to_absolute(self, facing: Direction) -> Direction {
        let order = [Direction::North, Direction::East, Direction::South, Direction::West];
        let facing_idx = order.iter().position(|&d| d == facing).unwrap_or(0);
        let offset: usize = match self {
            RelativeDir::Forward => 0,
            RelativeDir::Right   => 1,
            RelativeDir::Back    => 2,
            RelativeDir::Left    => 3,
        };
        order[(facing_idx + offset) % 4]
    }
}
```

### 1-C. `movement.rs` exits_v5 組裝段替換

**現行邏輯（行 297-319）** 用固定四元組 `("north", 0, 1)...` 直接把 Direction 轉字串塞進 exits_v5。

**替換為：**

```
1. 讀玩家當前 facing = get_player_facing(player_id) 或從本次移動更新後的 facing 取
2. 對 Direction::ALL 四個絕對方向：
   a. 查鄰格（同現行邏輯）
   b. 計算 rel = abs_dir.to_relative(facing)
   c. 以 rel 的 lowercase 字串（"back"/"forward"/"left"/"right"）為 exits_v5 的 key
   d. value 同現行：{ explored, walkable, name }
3. exits_v5 插入 JSON，key 現在是相對方位
```

**移動成功後更新 facing：**

在玩家成功移動到新格後（現行已有移動成功判斷點），追加：

```
set_player_facing(player_id, move_direction)
// move_direction = 本次移動的絕對方向（即玩家「面向」）
```

**move/scout action 接收相對方位：**

前端現在送 `direction: "back"|"forward"|"left"|"right"`。後端 handler 收到後：

```
1. 讀玩家當前 facing = get_player_facing(player_id)
2. abs_dir = RelativeDir::parse(direction_str).to_absolute(facing)
3. 以 abs_dir 執行現有移動邏輯（不變）
```

**WS payload `room_enter` 的 `exits` 鍵：**

現行：`"north"`, `"south"`, `"east"`, `"west"`
替換後：`"back"`, `"forward"`, `"left"`, `"right"`

前端不再接收也不需要知道任何絕對方位。

---

## 2. 前端改動

### 2-A. `web/index.html` 四槽 id 改名

```html
<!-- 改前 -->
<div class="junction-north" id="exit-north"></div>
<div class="junction-west"  id="exit-west"></div>
<div class="junction-east"  id="exit-east"></div>
<div class="junction-south" id="exit-south"></div>

<!-- 改後 -->
<div class="junction-forward" id="exit-forward"></div>
<div class="junction-left"    id="exit-left"></div>
<div class="junction-right"   id="exit-right"></div>
<div class="junction-back"    id="exit-back"></div>
```

**版面永遠固定**：forward 在上、back 在下、left 在左、right 在右。版面不旋轉，只是資料來源換了。CSS class 可從 `junction-north/south/east/west` 改為 `junction-forward/back/left/right`，樣式邏輯不變。

### 2-B. `web/grid-map.js` 鍵名替換

```js
// 改前
var idKeys = { '北': 'north', '南': 'south', '東': 'east', '西': 'west' };
exitEls.north = document.getElementById('exit-north');
exitEls.south = document.getElementById('exit-south');
exitEls.east  = document.getElementById('exit-east');
exitEls.west  = document.getElementById('exit-west');

// 改後
// idKeys 整段刪除（後端直接送 back/forward/left/right，無需中文對照表）
exitEls.back    = document.getElementById('exit-back');
exitEls.forward = document.getElementById('exit-forward');
exitEls.left    = document.getElementById('exit-left');
exitEls.right   = document.getElementById('exit-right');
```

exits_v5 渲染迴圈：key 現在是 `back/forward/left/right`，對應到 `exitEls[key]`，邏輯不變。

### 2-C. WS 發送 direction 改相對方位

```js
// 改前
window.gameSend({ type: 'move',  direction: moveDir  });  // moveDir = 'north'|'south'|...
window.gameSend({ type: 'scout', direction: scoutDir });  // scoutDir = 'north'|'south'|...

// 改後
window.gameSend({ type: 'move',  direction: relDir });    // relDir = 'back'|'forward'|'left'|'right'
window.gameSend({ type: 'scout', direction: relDir });
```

`relDir` 直接從 `exitEls` 的 key 取（點哪個槽就送哪個 key），不需要前端做任何方位換算。

---

## 3. 驗收標準

- [ ] 玩家走進新格，路口 UI 下方槽（`#exit-back`）顯示剛才來的那格地名（或 `▒▒▒` 若未揭露）
- [ ] 連走三格後，路口 UI 四槽完全不出現「北」「南」「東」「西」字眼；前端 HTML 原始碼中亦無 `exit-north/south/east/west` id
- [ ] 點 `#exit-back` 能回原格；回到原格後，原本的 `#exit-forward` 位置出現的是「剛才走過來的那格地名」（即新的 back），符合來向直覺
- [ ] 斷線重連後，槽位顯示與斷線前一致（DB `facing` 欄位正確持久化）
- [ ] 玩家 spawn（首次進格）：`facing` 預設 `north`，forward 在上顯示北方鄰格，back 在下顯示南方鄰格
- [ ] scout 探路：點 `▒▒▒` 槽位送 `direction: "back"|"forward"|"left"|"right"`，後端正確轉絕對方向執行探路，揭露後顯示該方向地名
- [ ] 三件套 bump 確認：`web/index.html` JS/CSS `?v=` 版本字串已 bump、`web/sw.js` CACHE 版本號已 bump

---

## 4. 不做

- 不改 `Direction` enum 本體（North/East/South/West 後端內部繼續用）
- 不改方格 (x,y) 拓撲結構
- 不做「回頭路地名不同」（需 room graph，留未來迭代）
- 不對 L1-L3 描述欄、探索節流、氣候、物件卡片邏輯做任何修改
- 不改 `ui-v5-frontend.md` / `ui-v5-backend.md` 原始檔案——本修訂規格**覆蓋**兩份規格中涉及方位的段落，其餘段落以原規格為準

---

## 占位符對照

無（本規格不含地名、人名、物品名）

---

## 需更新的原規格段落（碼農對照清單）

實作時以本修訂規格為準，以下段落視同作廢：

| 原規格 | 段落 | 作廢內容 |
|---|---|---|
| `ui-v5-frontend.md` | 第 5 節「路口形狀 UI」四方向邏輯表 | exit-north/south/east/west 槽 id |
| `ui-v5-frontend.md` | 第 7 節 WS 發送格式 | `direction: "north"\|"south"\|"east"\|"west"` |
| `ui-v5-frontend.md` | 第 7 節 `room_enter` payload 範例 | exits 鍵 north/south/east/west |
| `ui-v5-backend.md` | exits_v5 組裝邏輯 | 固定 ("north",0,1)... 四元組，替換為 facing 旋轉版 |

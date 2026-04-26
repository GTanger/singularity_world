# SW-22 後端 room payload 補地形類型 + 地標標記

> 狀態：待派碼農
> 優先級：P1（前端地圖渲染卡這個）
> 對齊：`docs/design/地圖渲染—單一管線與雙前端共用.md`（後端唯一真理）

## 痛點

當前 `GridCellView` 把 `Terrain` enum 經 `terrain_name_zh()` 攤平成中文字串後送前端：

```rust
// src/server/protocol.rs:360
pub struct GridCellView {
    pub x: i32,
    pub y: i32,
    pub terrain: String,  // ← 「鐵匠鋪」「神殿」全打成「地塊」
    pub name: String,
    pub explored: bool,
    pub walkable: bool,
}
```

兩個問題：

1. **地形類型遺失**：`terrain_name_zh` 對地標格（Inn/Tavern/Blacksmith/Market/Temple/GuildHall/Clinic/Workshop/GeneralStore/Farmhouse 等）走 `_ => "地塊"`，前端拿到全是「地塊」二字，無法切色/切 icon。
2. **可走性語意混淆**：`walkable` 是 bool，但移動成本（道路 0.5、林地 1.5、沼澤 2.0）無法表達；前端要做「黯沉/明亮」漸層也沒料。

## 規格

### 1. `GridCellView` 結構擴充

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridCellView {
    pub x: i32,
    pub y: i32,
    pub kind: String,       // 新增：Terrain enum 變體名（snake_case）
    pub terrain: String,    // 保留：中文標籤（向後相容）
    pub name: String,
    pub category: String,   // 新增："terrain" | "landmark" | "infra"
    pub explored: bool,
    pub walkable: bool,
}
```

`kind` 範例值：`"plain"` / `"forest_heavy"` / `"mountain"` / `"water_deep"` / `"inn"` / `"blacksmith"` / `"temple"` / `"road"` / `"bridge"`。

`category` 由 `Terrain` 變體分類得出（**這個分類是後端權威，前端不要重做**）：

- `"terrain"`：自然地塊。Plain/Forest/ForestLight/ForestHeavy/Mountain/Hills/Water/WaterDeep/Desert/Swamp/Tundra/Grassland/Jungle
- `"landmark"`：人造地標格。Urban/Inn/Tavern/Blacksmith/GeneralStore/Clinic/Workshop/Market/GuildHall/Temple/Farmhouse/FarmField
- `"infra"`：通行建設。Road/Bridge/Wall

### 2. 補完 `terrain_name_zh`

`src/grid/reveal.rs:55` 的 fallback `_ => "地塊"` 砍掉，把所有 `Terrain` 變體都列名：

```rust
Terrain::Urban => "聚落",
Terrain::Road => "道路",
Terrain::Bridge => "橋",
Terrain::Wall => "城牆",
Terrain::FarmField => "農田",
Terrain::Farmhouse => "農舍",
Terrain::Inn => "客棧",
Terrain::Tavern => "酒館",
Terrain::Blacksmith => "鐵匠鋪",
Terrain::GeneralStore => "雜貨店",
Terrain::Clinic => "藥鋪",
Terrain::Workshop => "工坊",
Terrain::Market => "市集",
Terrain::GuildHall => "公會",
Terrain::Temple => "神殿",
```

無 fallback：`match` 必須窮舉，新增 `Terrain` 變體未補映射時編譯失敗（這是要的）。

### 3. 新增 `Terrain::kind_str()` 與 `Terrain::category()`

`src/grid/cell.rs`：

```rust
impl Terrain {
    /// 序列化標記（snake_case），送前端用。
    pub fn kind_str(self) -> &'static str {
        match self {
            Terrain::Plain => "plain",
            Terrain::Forest => "forest",
            Terrain::ForestLight => "forest_light",
            Terrain::ForestHeavy => "forest_heavy",
            // ... 全列舉
        }
    }

    /// 分類：terrain / landmark / infra
    pub fn category(self) -> &'static str {
        match self {
            Terrain::Plain | Terrain::Forest | Terrain::ForestLight
            | Terrain::ForestHeavy | Terrain::Mountain | Terrain::Hills
            | Terrain::Water | Terrain::WaterDeep | Terrain::Desert
            | Terrain::Swamp | Terrain::Tundra | Terrain::Grassland
            | Terrain::Jungle => "terrain",

            Terrain::Urban | Terrain::Inn | Terrain::Tavern
            | Terrain::Blacksmith | Terrain::GeneralStore | Terrain::Clinic
            | Terrain::Workshop | Terrain::Market | Terrain::GuildHall
            | Terrain::Temple | Terrain::Farmhouse | Terrain::FarmField
                => "landmark",

            Terrain::Road | Terrain::Bridge | Terrain::Wall => "infra",
        }
    }
}
```

### 4. 修改點

**`src/game/room.rs:171` `get_grid_cells_around`**：tuple 從 6-tuple 擴成 8-tuple（加 `kind`、`category`）。

```rust
pub fn get_grid_cells_around(x: i32, y: i32, radius: i32)
    -> Vec<(i32, i32, String, String, String, String, bool, bool)>
//        x    y   kind    terrain category name   explored walkable
```

**`src/server/handler/movement.rs:112` `cells_raw.into_iter().map`**：對齊新 tuple 寫入 `GridCellView`。

### 5. 前端不動

本工單不改前端。前端拿到 `kind`/`category` 後可獨立工單漸進使用（先把 `kind == "inn"` 切金色 icon 之類），那是 SW-23+ 範圍。

## 驗收

- [ ] `cargo build && cargo clippy -- -D warnings && cargo test` 全綠
- [ ] WebSocket 訊息抓包：`grid_view.cells[0]` 含 `kind`、`category`、`terrain`、`name`、`explored`、`walkable` 六欄
- [ ] 站在 Inn 旁邊看 `cells`：那格 `kind="inn"`、`category="landmark"`、`terrain="客棧"`
- [ ] `terrain_name_zh` `match` 窮舉編譯通過（移除 `_ =>` 後仍編得過）
- [ ] 既有前端不爆（`terrain` 欄位仍是中文字串、其他欄位照舊）

## 不在範圍

- 前端渲染地標 icon／顏色（後續工單）
- `walkable` 改 `move_cost` 浮點（暫不動，前端用不到）
- exits 補 terrain 類型（已是中文，前端 ExitView 用得到，改動範圍類似但獨立）
- DB schema 變動（純 in-memory grid 結構序列化）

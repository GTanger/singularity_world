# 020 Watabou 城市空間引擎 — 組長工單

> 狀態：待執行
> 負責：組長（Cursor Auto）
> 審核：Claude Opus

## 目標

將 Watabou GeoJSON 解析為城市內部房間，注入現有 Room store。
玩家走進有城市資料的 burg → 進入城市 → 在路口間移動 → 離城回世界地圖。
**不需要新導航模式**——城市路口就是普通 Room，所有現有代碼零改動。

## 核心設計

### 原則
- GeoJSON 是城市定義，不生成 JSON 房間檔——啟動時解析，注入記憶體 store
- 城市路口 = `model::Room`，與世界房間完全相同結構
- 可進入建築 = 子 Room，掛在路口的 exits 上
- 城門路口連回世界地圖，世界房間「進城」連到城門路口

### 檔案位置
- 城市 GeoJSON：`data/cities/{burg_id}.json`（例：`data/cities/32.json` = 宜林）
- 新模組：`src/city/mod.rs`、`src/city/parser.rs`、`src/city/builder.rs`
- 在 `src/lib.rs` 加 `pub mod city;`

## 資料結構

```rust
// src/city/mod.rs

/// 解析後的城市原始幾何資料
pub struct CityGeo {
    pub burg_id: u32,
    pub roads: Vec<Road>,           // LineString 路段
    pub buildings: Vec<Building>,    // 建築多邊形 + 面積
    pub districts: Vec<District>,    // 區域多邊形
    pub walls: Option<Walls>,        // 城牆幾何
    pub river: Option<River>,        // 河流幾何
    pub squares: Vec<Polygon>,       // 廣場
}

/// 路口節點（中間產物，最終轉為 Room）
pub struct Junction {
    pub id: String,                  // "city_{burg_id}_j{N}"
    pub x: f64,
    pub y: f64,
    pub district_idx: Option<usize>, // 所屬 district
    pub near_wall: bool,             // 靠近城牆（候選城門）
    pub near_river: bool,
    pub near_square: bool,
    pub nearby_buildings: Vec<usize>, // 附近建築索引
    pub connected_to: Vec<String>,   // 相鄰路口 id
}

/// 可進入建築（轉為子 Room）
pub struct EnterableBuilding {
    pub id: String,                  // "city_{burg_id}_b{N}"
    pub x: f64,
    pub y: f64,
    pub area: f64,
    pub junction_id: String,         // 掛在哪個路口
    pub building_type: BuildingType, // 推斷的用途
}

pub enum BuildingType {
    Shop,       // 廣場旁大型
    Tavern,     // 城門旁
    Temple,     // 靠近 district 中心的大型
    Warehouse,  // 河岸旁大型
    Residence,  // 預設
}
```

## 演算法步驟

### Step 1：解析 GeoJSON（parser.rs）

讀取 Watabou GeoJSON，提取：
- `roads`（id="roads"）→ GeometryCollection 內的 LineString
- `buildings`（id="buildings"）→ MultiPolygon，計算每棟面積
- `districts`（id="districts"）→ GeometryCollection 內的 Polygon
- `walls`（id="walls"）→ 城牆幾何
- `rivers`（id="rivers"）→ 河流幾何
- `squares`（id="squares"）→ 廣場多邊形

### Step 2：偵測路口（builder.rs）

1. 收集所有路段的端點（每條 LineString 的首尾座標）
2. 以 DBSCAN 或簡易距離聚類（eps=15.0）合併鄰近端點 → 路口
3. 每個路口取聚類中心座標
4. 建立鄰接表：兩個路口之間有路段連接 → 加 edge
5. **密度控制**：若路口數 > 500，提高 eps 重跑；若 < 50，降低 eps

### Step 3：標記路口特徵

對每個路口：
- **歸區**：點對多邊形測試，判斷屬於哪個 district
- **靠牆**：距離城牆 < 50 單位 → `near_wall = true`（候選城門）
- **靠河**：距離河流 < 30 單位 → `near_river = true`
- **靠廣場**：在廣場多邊形內或距離 < 20 → `near_square = true`
- **附近建築**：距離路口中心 < 40 單位的建築列表

### Step 4：篩選可進入建築

條件（以上皆是，取聯集）：
1. **面積最大**：全城面積前 5% 的建築
2. **位置優先**：廣場旁（距 square < 30）、城門旁（掛在 near_wall 路口）、河岸旁（距 river < 30）
3. **每區覆蓋**：每個 district 至少 1 棟可進入

推斷 BuildingType：
- 廣場旁最大 → Shop
- 城門路口旁 → Tavern
- district 中心最大 → Temple（如果 burg 有 temple tag）
- 河岸旁大型 → Warehouse
- 其餘 → Residence

### Step 5：District 命名

依空間特徵自動命名（繁體中文）：

| 特徵 | 命名 |
|------|------|
| 含城塞（citadel 區域中心） | 城塞區 |
| 靠河 + 多建築 | 河畔商區 |
| 靠城牆 + shantytown 區域 | 外城棚區 |
| 含廣場 | 廣場區 / 中央市集 |
| 含寺廟（temple） | 廟堂區 |
| 靠港口（coast 側） | 港灣區 |
| 建築密集 | 民居區 |
| 建築稀疏 + 靠外圍 | 農田區 |

若特徵重疊，取最顯著的。同名時加方位（東/西/南/北）。

### Step 6：生成 Room 並注入 Store

每個路口 → 一個 `model::Room`：
```rust
Room {
    id: "city_32_j042",
    name: "{district_name}路口",  // e.g. "河畔商區路口"
    tags: vec!["city_junction", "near_river"],
    zone: "city_32",  // 城市獨立 zone
    description: String::new(),  // 稍後由 LLM 填
    objects: vec![],
}
```

每個可進入建築 → 一個 `model::Room`：
```rust
Room {
    id: "city_32_b007",
    name: "河岸貨棧",  // 依 BuildingType 命名
    tags: vec!["city_building", "warehouse"],
    zone: "city_32",
    description: String::new(),
    objects: vec![],
}
```

Exits 規則：
- 路口之間：`direction` = 方位（依座標差計算八方位：東/西/南/北/東北/...）
- 路口 → 建築：`direction` = "進入{building_name}"
- 建築 → 路口：`direction` = "離開"
- **城門路口 → 世界地圖**：取 near_wall 路口中連接數最少的（死角 = 城門），exits 加上該 burg 世界房間的 exits（官道往 X、小徑往 Y）
- **世界房間 → 城門**：修改 burg 世界房間，加一條 exit「進城」→ 主城門路口

### Step 7：整合進啟動流程

在 `src/store/mod.rs` 的 `load()` 尾部：

```rust
// 載入城市 GeoJSON 並注入房間
if let Ok(entries) = std::fs::read_dir("data/cities") {
    for entry in entries.flatten() {
        if entry.path().extension().map_or(false, |e| e == "json") {
            match city::load_and_inject(&mut s, &entry.path()) {
                Ok(stats) => tracing::info!(
                    "[city] {} 載入: {} 路口, {} 建築",
                    stats.burg_id, stats.junctions, stats.buildings
                ),
                Err(e) => tracing::warn!("[city] 載入失敗 {:?}: {}", entry.path(), e),
            }
        }
    }
}
```

## 測試計畫

1. `cargo build --release` 通過
2. `cargo clippy -- -D warnings` 零警告
3. 放入 `data/cities/32.json`（宜林的 Watabou GeoJSON），啟動伺服器
4. 驗證 log 顯示路口數量合理（200-500）
5. 登入 → 走到宜林 → 看到「進城」exit → 進城 → 能在路口間移動
6. 城門路口有「官道往滕昌」等世界出口 → 能走回世界地圖
7. 可進入建築 → 進入/離開正常
8. 描述欄位暫時為空白或佔位文字（LLM 生成另案處理）

## 硬規則

- **不引入新 crate**。幾何計算（點對多邊形、距離）手寫，不用 geo crate
- **不改 model::Room 結構**。城市房間用現有欄位
- **不改現有導航邏輯**。城市路口就是普通房間，走 exits 就是現有的 move 邏輯
- **不生成 JSON 檔案**。城市房間只存在於記憶體 store，不寫入 data/rooms/
- tools/ 下的腳本 gitignore 不用管

## 交付清單

- [ ] `src/city/mod.rs` + `parser.rs` + `builder.rs`
- [ ] `src/lib.rs` 加 `pub mod city;`
- [ ] `src/store/mod.rs` 啟動流程加載城市
- [ ] `data/cities/32.json`（從 `docs/map/city/darkwood_mount.json` 複製）
- [ ] 此工單標記為已完成

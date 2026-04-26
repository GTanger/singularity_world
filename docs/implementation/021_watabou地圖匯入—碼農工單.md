# 021 Watabou 世界地圖匯入 — 碼農工單

> 狀態：待執行
> 優先級：P1
> 前置：無（grid_cells 表已存在）
> 主線約束：Watabou 的 odd-q hex 只代表**外部輸入格式**；匯入後的執行期世界一律是 square grid MUD、四向鄰居、PostgreSQL 權威。不得新增 Hex runtime abstraction、不得復活六向移動／觀景窗。

## 目標

把 Watabou Perilous Shores 輸出的世界地圖 JSON（odd-q hex layout）匯入成方格世界資料。**多張地圖可拼接**，每張地圖細分為 10×10 方格。城鎮在大地圖上撲開（不是單點連結子地圖）。

## 設計拍板（六項，全是硬規則）

| 項目 | 決定 |
|---|---|
| 細分倍率 | 1 hex → 10×10 方格（每張地圖 ≈ 120,300 cells） |
| odd-q → 方格映射 | (q,r) 視為 hex 中心 → 算世界座標 → 細分；鄰居四向重算 |
| `terrain == null` | 視為「平原」 |
| rivers / searoutes | 跨格邊界水道；road 經過的格設為「橋」可走，否則屏障 |
| danger 副本入口 | 不破例離開大地圖；該格地形特別兇險（high-risk 標記） |
| 拼接策略 | 硬接 + roads/rivers lazy stitch；terrain 落差不平滑 |

## 資料結構

### 新增表 `world_maps`

```sql
CREATE TABLE world_maps (
    map_id          TEXT PRIMARY KEY,        -- e.g. "lake_of_darkness"
    name            TEXT NOT NULL,
    origin_url      TEXT,                    -- Watabou seed URL
    world_offset_x  INTEGER NOT NULL,        -- 該 tile 左上角的世界方格座標
    world_offset_y  INTEGER NOT NULL,
    width_hex       INTEGER NOT NULL,        -- 來源 hex 寬（如 41）
    height_hex      INTEGER NOT NULL,        -- 來源 hex 高（如 27）
    subdiv          INTEGER NOT NULL DEFAULT 10,
    tags            JSONB,                   -- bp.tags 原樣存
    imported_at     TIMESTAMPTZ DEFAULT now()
);
```

### `grid_cells` 擴充

世界座標權威：`(world_x, world_y)` 全域唯一鍵。
新增 `source_map_id TEXT REFERENCES world_maps(map_id)` 標出處。

```sql
ALTER TABLE grid_cells
    ADD COLUMN IF NOT EXISTS source_map_id TEXT REFERENCES world_maps(map_id),
    ADD COLUMN IF NOT EXISTS terrain TEXT,           -- plain/forest-light/forest-dead/forest-dark/swamp/mountain/rocks/water
    ADD COLUMN IF NOT EXISTS is_bridge BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_river BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS is_road  BOOLEAN DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS settlement_seed JSONB,  -- 從 town.link 解析的種子
    ADD COLUMN IF NOT EXISTS danger_marker JSONB;    -- danger.name / danger.link
CREATE UNIQUE INDEX IF NOT EXISTS idx_grid_cells_world_xy ON grid_cells(world_x, world_y);
```

## Parser 純函數

`src/world/watabou_import.rs`（新檔）

```rust
pub struct WatabouTile {
    pub map_id: String,
    pub origin_url: Option<String>,
    pub width_hex: i32,
    pub height_hex: i32,
    pub world_offset_x: i32,
    pub world_offset_y: i32,
    pub subdiv: i32,                         // 預設 10
}

pub struct ImportedCell {
    pub world_x: i32,
    pub world_y: i32,
    pub terrain: Terrain,
    pub is_river: bool,
    pub is_road: bool,
    pub is_bridge: bool,
    pub settlement_seed: Option<SettlementSeed>,
    pub danger_marker: Option<DangerMarker>,
}

pub fn watabou_tile_to_grid(json: &serde_json::Value, tile: &WatabouTile)
    -> Result<Vec<ImportedCell>, ImportError>;
```

純函數，無 DB 副作用。寫入交給上層 `import_watabou_map(tile, json) -> Result<()>`。

## 座標映射

odd-q layout（Watabou bp.tags 含 `"odd-q"`）：偶數 q 行不偏移、奇數 q 行 y +0.5。

```rust
// hex (q,r) → tile-local 方格中心 (cx, cy)
fn hex_center_local(q: i32, r: i32, subdiv: i32) -> (i32, i32) {
    let cx = q * subdiv + subdiv / 2;
    let cy = r * subdiv + subdiv / 2 + if q % 2 != 0 { subdiv / 2 } else { 0 };
    (cx, cy)
}

// hex → 涵蓋的 tile-local 方格集合（菱形近似 = (q,r) 對應的 subdiv×subdiv 區塊）
// 第一版用矩形塊：tile-local x ∈ [q*subdiv, q*subdiv+subdiv), y ∈ [r*subdiv + (q奇?subdiv/2:0), 同+subdiv)
// 實際 hex 邊形不重要，反正鄰格四向重算

// tile-local → world
fn to_world(local_x: i32, local_y: i32, tile: &WatabouTile) -> (i32, i32) {
    (local_x + tile.world_offset_x, local_y + tile.world_offset_y)
}
```

四向鄰居：`(world_x±1, world_y)` / `(world_x, world_y±1)`。**不**沿用 hex 六向鄰居。

## 翻譯規則

### terrain

```
null              → "plain"
"forest-light"    → "forest-light"
"forest-dead"     → "forest-dead"
"forest-dark"     → "forest-dark"
"forest"          → "forest-light"   # 兼容
"swamp"           → "swamp"
"mountain"        → "mountain"
"rocks"           → "rocks"
"water"           → "water"          # 預設不可走，除非疊 road（橋）
```

### rivers / roads / searoutes

每條是一串 hex 座標折線。轉換為「途經的 tile-local 方格」用 Bresenham 或簡單線段採樣（每個中心點之間取 `subdiv` 個樣本，標 `is_river` 或 `is_road`）。

- `is_road` 蓋過 `is_river` → `is_bridge = true`，可走
- 純 `is_river` 且 terrain != water → 該格不可走（屏障）
- `searoute` 視為 `is_river`（船道，現階段不可走）

### town.link 解析

Watabou 城鎮 link 範例：
```
https://watabou.github.io/city-generator/?size=40&seed=123&citadel=1&walls=1&shantytown=1&temple=1&plaza=1&coast=1&river=0
```

解 query string 存進 `settlement_seed`：
```json
{"size": 40, "seed": 123, "citadel": true, "walls": true,
 "shantytown": true, "temple": true, "plaza": true, "coast": true, "river": false}
```

**只存種子，不展開聚落**。聚落生成器（WORKBOARD §九 道路驅動 + 功能格池 + 三層放置）後續吃這顆種子在城鎮中心方格周圍 4-8 格半徑內展開。城鎮中心 = `hex_center_local(q, r, subdiv)`。

### danger

```json
{"name": "...", "link": "..."}  → 存 danger_marker
```

對應方格 terrain 維持原樣，但記號上標 high-risk（後續資源點/遭遇系統讀這欄）。

## 拼接（lazy stitch）

第一版**不做平滑**。新地圖匯入時：

1. 算 `(world_offset_x, world_offset_y)` —— 由匯入 CLI 指定（人工拍板拼哪邊）
2. 直接寫 cells；`(world_x, world_y)` 衝突 → 報錯，不覆蓋
3. 跨地圖的 road/river：呼叫 `stitch_boundaries(map_a, map_b, edge)` 在邊界 ±2 格內找最近的 road 端點對接

`stitch_boundaries` 第一版可不實作，留 TODO，靠匯入時 `world_offset` 對齊處理大部分情況。

## CLI 工具

`src/bin/import_watabou.rs`

```bash
cargo run --release --bin import_watabou -- \
    --json data/maps/lake_of_darkness.json \
    --map-id lake_of_darkness \
    --offset-x 0 --offset-y 0 \
    --subdiv 10
```

流程：
1. 讀 JSON
2. 校驗 `bp.tags` 含 `"odd-q"`，否則 abort
3. 寫 `world_maps` 一筆
4. 跑 parser → 批次 INSERT 到 `grid_cells`（用 COPY 或 batched INSERT，~120K 筆）
5. 印統計：`{terrain 分布, town 數, danger 數, road/river 格數}`

## 驗收標準

- [ ] `lake_of_darkness.json` 成功匯入，`SELECT COUNT(*) FROM grid_cells WHERE source_map_id='lake_of_darkness'` ≈ 120,300
- [ ] terrain 分布合理（water/forest/mountain 比例與 JSON 原樣對齊 ±5%）
- [ ] 37 個 town 都有 `settlement_seed`
- [ ] 3 個 danger 都有 `danger_marker`
- [ ] 隨機抽 10 個 road 格驗證 `is_road=true`
- [ ] 隨機抽 5 個 river+road 交點驗證 `is_bridge=true`
- [ ] 二次匯入同 map_id → 報衝突錯誤（不覆蓋）
- [ ] `cargo build && cargo clippy -- -D warnings && cargo test` 全綠

## 後續工單（不在本工單範圍）

- **022 聚落展開生成器**：吃 `settlement_seed` + 中心方格座標 → 道路驅動產出 4-8 格半徑的城鎮細節
- **023 lazy stitch 邊界縫合**：road/river 跨 tile 對接演算法
- **024 探索揭露整合**：玩家進入未揭露格 → 觸發描述生成（沿用 020 工單管線）

## 注意事項

- Watabou JSON 沒給 hex pixel 座標，用 (q, r) 推。`bp.width=41` `bp.height=27` 的單位是 hex 個數
- searoute 第一版視同 river，不開船道機制
- `terrain="water"` + `is_road=true` = 橋，可走；單純 water 不可走
- 不要試圖讓 hex 六向鄰居在方格世界復活，全部改寫成四向
- 城鎮種子留著，**展開**是另一支工單，本工單不碰

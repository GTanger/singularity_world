//! 方格地圖容器（SquareGrid）與格子單元（GridCell）。
//!
//! - `SquareGrid` 持有 world_seed 及所有已揭露格子
//! - `GridCell` 為格子單元，含 `Terrain` 與 `GridObject`
//! - 揭露邏輯由 `reveal` 模組負責，此模組只管容器

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::cell::{Freshness, GridObject, Terrain};

use super::coord::{Direction, SquareCoord};

// ─── 顯式出口 ───

/// 顯式出口：方向 + 目標座標。村落格用這個取代座標鄰接自動計算。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExplicitExit {
    pub dir: Direction,
    pub target: SquareCoord,
}

// ─── 格子單元 ───

/// 方格地圖單元（格上資源與地形）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridCell {
    pub coord: SquareCoord,
    pub terrain: Terrain,
    pub name: String,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub zone: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,

    /// 格子內的地上物（資源、地標）
    #[serde(default)]
    pub objects: Vec<GridObject>,

    /// 是否已被玩家探索（影響前端渲染）
    #[serde(default)]
    pub explored: bool,

    /// 探索新鮮度（連續探索同一格的資源豐厚度衰減）
    #[serde(default)]
    pub freshness: Freshness,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explicit_exits: Vec<ExplicitExit>,
}

impl GridCell {
    pub fn new(coord: SquareCoord, terrain: Terrain, name: impl Into<String>) -> Self {
        Self {
            coord,
            terrain,
            name: name.into(),
            zone: String::new(),
            tags: Vec::new(),
            description: String::new(),
            objects: Vec::new(),
            explored: false,
            freshness: Freshness::Fresh,
            explicit_exits: Vec::new(),
        }
    }
}

// ─── 自訂 Serialize / Deserialize（cells 轉 Vec 以相容 JSON）───

/// 方格地圖：world_seed + 已揭露格子集合。
///
/// 格子以 `SquareCoord` 為鍵存入 `HashMap`；序列化時轉成 `Vec`
/// 與 `HexGrid` 慣例一致。未插入 = 黑格（未揭露）。
///
/// 相鄰格之間預設八方皆通；只有不可行走地形（Water、Mountain 等）阻斷移動。
#[derive(Debug, Clone, Default)]
pub struct SquareGrid {
    /// 揭露用 PRNG 種子（與座標混合；見 `reveal` 模組）
    world_seed: u64,
    cells: Arc<HashMap<SquareCoord, GridCell>>,
    /// 資料是否被修改（格數不變但內容變動時設為 true）
    dirty: bool,
}

/// 序列化用的中間結構（cells 轉 Vec，符合 JSON 慣例）
#[derive(Serialize, Deserialize)]
struct SquareGridSer {
    #[serde(default)]
    world_seed: u64,
    cells: Vec<GridCell>,
}

impl Serialize for SquareGrid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let ser = SquareGridSer {
            world_seed: self.world_seed,
            cells: self.cells.values().cloned().collect(),
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SquareGrid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser = SquareGridSer::deserialize(deserializer)?;
        let cells = ser.cells.into_iter().map(|c| (c.coord, c)).collect();
        Ok(SquareGrid {
            world_seed: ser.world_seed,
            cells: Arc::new(cells),
            dirty: false,
        })
    }
}

// ─── Cell CRUD ───────────────────────────────────────────────────

impl SquareGrid {
    /// 建立空地圖（world_seed 預設 0，可用 `set_world_seed` 覆蓋）
    pub fn new() -> Self {
        Self::default()
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn set_world_seed(&mut self, seed: u64) {
        self.world_seed = seed;
    }

    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// 插入格子；若座標已存在則覆蓋並回傳舊值。
    pub fn insert(&mut self, cell: GridCell) -> Option<GridCell> {
        let coord = cell.coord;
        self.dirty = true;
        Arc::make_mut(&mut self.cells).insert(coord, cell)
    }

    /// 移除格子；不存在時回傳 `None`。
    pub fn remove(&mut self, coord: SquareCoord) -> Option<GridCell> {
        let r = Arc::make_mut(&mut self.cells).remove(&coord);
        if r.is_some() {
            self.dirty = true;
        }
        r
    }
    /// 是否為顯式儲存的格子（而非依據種子即時生成的荒野格）
    pub fn has_explicit_cell(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
    }

    /// 是否啟用白塔半島 800×1,000 無縫荒野基底層
    #[inline]
    pub fn has_peninsula_wilderness(&self) -> bool {
        self.world_seed == 2026_07_03 || self.cells.len() > 2000
    }

    /// 取得格子（未揭露回傳 `None`）
    pub fn get(&self, coord: SquareCoord) -> Option<&GridCell> {
        if let Some(c) = self.cells.get(&coord) {
            return Some(c);
        }
        if self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord) {
            return super::r#gen::procedural_wilderness_cell_ref(coord);
        }
        None
    }

    /// 取得格子（可寫）——呼叫者修改內容後應呼叫 `mark_dirty()`。
    pub fn get_mut(&mut self, coord: SquareCoord) -> Option<&mut GridCell> {
        if !self.cells.contains_key(&coord)
            && self.has_peninsula_wilderness()
            && super::r#gen::in_peninsula_bounds(coord)
        {
            let cell = super::r#gen::materialize_wilderness_cell(coord);
            Arc::make_mut(&mut self.cells).insert(coord, cell);
            self.dirty = true;
        }
        Arc::make_mut(&mut self.cells).get_mut(&coord)
    }

    /// 查詢格子是否已揭露
    pub fn contains(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
            || (self.has_peninsula_wilderness() && super::r#gen::in_peninsula_bounds(coord))
    }

    /// 已揭露格子總數
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// 虛擬全圖格數（半島正典為 800,000 格）
    pub fn total_virtual_cells(&self) -> usize {
        if self.has_peninsula_wilderness() {
            super::r#gen::TOTAL_CELLS
        } else {
            self.cells.len()
        }
    }


    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// 遍歷所有已揭露格子
    pub fn cells(&self) -> impl Iterator<Item = &GridCell> {
        self.cells.values()
    }

    /// 遍歷所有已揭露格子（可寫；修改後應標記 dirty）
    pub fn cells_mut(&mut self) -> impl Iterator<Item = &mut GridCell> {
        Arc::make_mut(&mut self.cells).values_mut()
    }

    /// 遍歷所有已揭露座標
    pub fn coords(&self) -> impl Iterator<Item = &SquareCoord> {
        self.cells.keys()
    }
}

// ─── 鄰接 & 移動 ─────────────────────────────────────────────────

impl SquareGrid {
    /// 回傳已存在的相鄰格（八方向，不考慮地形通行性）
    pub fn neighbors(&self, coord: SquareCoord) -> Vec<&GridCell> {
        coord
            .neighbors()
            .into_iter()
            .filter_map(|n| self.get(n))
            .collect()
    }

    /// 能否從 `from` 步行到相鄰的 `to`？（別名方法）
    pub fn can_move_between(&self, from: SquareCoord, to: SquareCoord) -> bool {
        self.can_walk(from, to)
    }

    /// 能否從 `from` 步行到相鄰的 `to`？
    ///
    /// 若 `from` 有顯式出口（explicit_exits），以出口表為準；
    /// 否則回退到座標鄰接邏輯（Manhattan 距離 1、地形可走、街道與公開建築格暢通、建築隔離法則）。
    pub fn can_walk(&self, from: SquareCoord, to: SquareCoord) -> bool {
        let Some(from_cell) = self.get(from) else {
            return false;
        };
        let Some(to_cell) = self.get(to) else {
            return false;
        };

        // 顯式出口覆蓋座標邏輯
        if !from_cell.explicit_exits.is_empty() {
            return from_cell.explicit_exits.iter().any(|e| e.target == to);
        }

        // Fallback: 座標鄰接邏輯
        if from.manhattan_distance(to) != 1 {
            return false;
        }
        if !to_cell.terrain.walkable() {
            return false;
        }

        // 街道 :: 與相鄰公開建築格（$ ⚒︎ + ⌂ ⚑ 𖠿 ∆）互相開放通行
        let is_bldg = |t: Terrain| -> bool {
            t.category() == "landmark"
        };
        let is_road = |t: Terrain| -> bool {
            t == Terrain::Road
        };

        if is_road(from_cell.terrain) && is_bldg(to_cell.terrain) {
            return true;
        }
        if is_bldg(from_cell.terrain) && is_road(to_cell.terrain) {
            return true;
        }

        // 城鎮內河水道隔離：禁止由建築或城鎮街道直接踏入深渠水體（須經石橋 ≍ 通行）
        if (from_cell.terrain == Terrain::Water || to_cell.terrain == Terrain::Water)
            && (!from_cell.zone.is_empty() || !to_cell.zone.is_empty() || is_bldg(from_cell.terrain) || is_bldg(to_cell.terrain))
        {
            return false;
        }

        // 建築隔離法則：禁止建築物 -> 建築物 的直接穿牆（同名複合建築格除外）
        if is_bldg(from_cell.terrain) && is_bldg(to_cell.terrain) {
            if from_cell.name != to_cell.name {
                return false;
            }
        }
        true
    }

    /// 動態改變網格地形（10 條動態地形演變鏈）。
    ///
    /// 成功時回傳 `Ok((old_terrain, new_terrain, 描述字串))`，並將網格標記為 dirty。
    pub fn mutate_cell_terrain(
        &mut self,
        coord: SquareCoord,
        action: &str,
    ) -> Result<(Terrain, Terrain, String), String> {
        let cell = self.get_mut(coord).ok_or_else(|| {
            format!("座標 ({}, {}) 處無有效網格", coord.x, coord.y)
        })?;
        let old = cell.terrain;
        let (new_terrain, desc) = match (old, action) {
            // 1. 開山挖礦 / 坍方還山
            (Terrain::Mountain, "mine" | "quarry" | "excavate") => (
                Terrain::MinePass,
                "劈石開道，山體岩壁被鑿出可供通行的礦道山隘。".to_string(),
            ),
            (Terrain::MinePass, "collapse" | "restore_mountain") => (
                Terrain::Mountain,
                "岩層坍塌封死礦道，重歸高聳嶙峋的實心山脊。".to_string(),
            ),

            // 2. 伐林清地
            (
                Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
                "deforest" | "clear_trees" | "logging",
            ) => (
                Terrain::Plain,
                "伐倒巨木灌叢、清理樹樁，林地化為一片開闊平野。".to_string(),
            ),

            // 3. 開墾整地
            (Terrain::Plain | Terrain::Grassland, "plow" | "till" | "cultivate") => (
                Terrain::EmptyFurrow,
                "犁開熟土、起壟除草，平野闢為待播種之空田壟。".to_string(),
            ),

            // 4. 依栽種變化、收成與休耕復育
            (Terrain::EmptyFurrow, "plant_paddy" | "irrigate" | "plant_rice") => (
                Terrain::FarmField,
                "引清流灌渠入壟，阡陌縱橫，注滿水光映照之水田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_dry_crop" | "plant_wheat" | "dry_field") => (
                Terrain::DryField,
                "耙土覆麥、起溝避澇，田壟化作耐旱麥浪之旱田。".to_string(),
            ),
            (Terrain::EmptyFurrow, "plant_herbs" | "herb_garden") => (
                Terrain::HerbGarden,
                "移栽靈草仙葩、精細分畦，泥壟闢成藥香瀰漫之藥圃。".to_string(),
            ),
            (Terrain::EmptyFurrow, "afforest" | "plant_trees") => (
                Terrain::Forest,
                "栽植樹苗防風固土，田壟漸次鬱閉為青蔥林木。".to_string(),
            ),
            (
                Terrain::FarmField | Terrain::DryField | Terrain::HerbGarden,
                "harvest" | "clear_field",
            ) => (
                Terrain::EmptyFurrow,
                "收割豐實作物，田畦重歸平整空壟，靜候下季播種。".to_string(),
            ),
            (Terrain::EmptyFurrow, "fallow" | "abandon_field") => (
                Terrain::Plain,
                "停歇耕作任野草蔓延，田壟風化還原為原野平地。".to_string(),
            ),

            // 5. 踩踏鋪路 / 道路廢弛
            (Terrain::Plain | Terrain::Grassland, "pave_road" | "tread_path" | "build_road") => (
                Terrain::Road,
                "夯實路基鋪設平整石板，踩出一條四通八達之道途。".to_string(),
            ),
            (Terrain::Road, "break_road" | "abandon_road" | "remove_road") => (
                Terrain::Plain,
                "掘起石板任草木復生，道路風化還原為尋常土地。".to_string(),
            ),

            // 6. 百工造橋 / 橋樑毀損
            (Terrain::Water, "build_bridge") => (
                Terrain::Bridge,
                "立墩架梁、鋪設石木橋板，清波之上升起連通兩岸之飛橋。".to_string(),
            ),
            (Terrain::Bridge, "demolish_bridge" | "collapse_bridge") => (
                Terrain::Water,
                "橋梁拆除倒塌，河川水流暢通，復還為一泓碧水。".to_string(),
            ),

            // 7. 水文漲退
            (Terrain::Plain | Terrain::Grassland, "flood" | "inundate") => (
                Terrain::Swamp,
                "潮汛上漲、地勢凹陷聚水，土地浸軟化作蘆荻泥澤。".to_string(),
            ),
            (Terrain::Swamp, "drain" | "reclaim_swamp") => (
                Terrain::Plain,
                "疏浚導流、填石排澇，泥濘澤地重現為堅實乾爽之平原。".to_string(),
            ),

            // 8. 聚落營造與拆除
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_house" | "build_home",
            ) => (
                Terrain::House,
                "起基立柱、覆瓦築垣，落成一棟溫暖樸實之民宅。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_market" | "build_shop",
            ) => (
                Terrain::Market,
                "搭建貨架攤棚、懸掛商號錦旗，闢出一處繁華商貿之市集。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_workshop" | "build_blacksmith",
            ) => (
                Terrain::Workshop,
                "置爐安砧、排設風箱機關，落成一座百工鍛造之工坊。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_clinic" | "build_apothecary",
            ) => (
                Terrain::Clinic,
                "配置診療床、草木調理台與自然脈絡感應儀，落成一間脈絡診所。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_inn" | "build_tavern",
            ) => (
                Terrain::Inn,
                "結綵挑燈、陳設几榻佳釀，營造一處迎送行旅之現代行館。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_fence",
            ) => (
                Terrain::Fence,
                "深埋排樁、橫綁堅木防禦，立起阻絕侵擾之木柵圍欄。".to_string(),
            ),
            (
                Terrain::Plain | Terrain::EmptyFurrow,
                "build_wall",
            ) => (
                Terrain::Wall,
                "條石錯縫、灌漿版築，壘砌起厚重巍峨之實心城牆。".to_string(),
            ),
            (
                Terrain::House
                | Terrain::Market
                | Terrain::Workshop
                | Terrain::Clinic
                | Terrain::Inn
                | Terrain::Fence
                | Terrain::Wall
                | Terrain::Blacksmith
                | Terrain::GeneralStore
                | Terrain::Farmhouse
                | Terrain::Warehouse
                | Terrain::Granary,
                "demolish" | "tear_down",
            ) => (
                Terrain::Plain,
                "棟樑傾覆、磚石清運，建物平毀為一片乾淨平地。".to_string(),
            ),

            // 9. 荒野復育 / 植被演替
            (Terrain::Plain | Terrain::Grassland, "grow_forest" | "wild_growth") => (
                Terrain::Forest,
                "草木滋長、嘉木成蔭，日光穿透繁枝形成幽謐林境。".to_string(),
            ),

            // 10. 沙石沉積 / 海蝕沖刷
            (Terrain::Water, "sediment" | "deposit_sand") => (
                Terrain::Beach,
                "潮沙堆積、水勢退讓，湧浪邊緣沉澱出細密沙礫之海灘。".to_string(),
            ),
            (Terrain::Beach, "erode" | "sea_wash") => (
                Terrain::Water,
                "波濤沖刷侵蝕岸線，沙灘隨浪散逸復沒入清澈淺水。".to_string(),
            ),
            (Terrain::Beach, "solidify_land") => (
                Terrain::Plain,
                "耐鹽草根固著泥沙，海灘化作堅實平整之沿岸草原。".to_string(),
            ),

            _ => {
                return Err(format!(
                    "地形 {:?} 不支援演變動作 '{}'",
                    old, action
                ));
            }
        };

        cell.terrain = new_terrain;
        self.dirty = true;
        Ok((old, new_terrain, desc))
    }

    /// BFS 最短路徑（跳數）；回傳包含起點與終點的座標序列。
    ///
    /// - `from == to`：回傳 `Some(vec![from])`
    /// - 起點或終點不在地圖中：回傳 `None`
    /// - 不可達：回傳 `None`
    pub fn find_path(&self, from: SquareCoord, to: SquareCoord) -> Option<Vec<SquareCoord>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.contains(from) || !self.contains(to) {
            return None;
        }

        let mut visited: HashSet<SquareCoord> = HashSet::new();
        let mut parent: HashMap<SquareCoord, SquareCoord> = HashMap::new();
        let mut queue: VecDeque<SquareCoord> = VecDeque::new();

        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            for next in current.neighbors() {
                if !self.can_walk(current, next) || visited.contains(&next) {
                    continue;
                }
                visited.insert(next);
                parent.insert(next, current);

                if next == to {
                    // 回溯路徑：from … to
                    let mut path = vec![to];
                    let mut c = to;
                    while let Some(&p) = parent.get(&c) {
                        path.push(p);
                        c = p;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }
        None
    }
}

// ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn plain(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Plain, format!("({x},{y})"))
    }

    fn water(x: i32, y: i32) -> GridCell {
        GridCell::new(
            SquareCoord::new(x, y),
            Terrain::WaterDeep,
            format!("深水({x},{y})"),
        )
    }

    fn road(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Road, format!("街道({x},{y})"))
    }

    fn shop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Market, format!("商號({x},{y})"))
    }

    fn workshop(x: i32, y: i32) -> GridCell {
        GridCell::new(SquareCoord::new(x, y), Terrain::Workshop, format!("工坊({x},{y})"))
    }

    // ── 基本 CRUD ───────────────────────────────────────────────

    #[test]
    fn insert_and_get() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        assert_eq!(g.len(), 2);
        assert!(g.contains(SquareCoord::new(0, 0)));
        assert!(!g.contains(SquareCoord::new(5, 5)));
        assert!(g.get(SquareCoord::new(1, 0)).is_some());
        assert_eq!(
            g.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::Plain
        );
    }

    #[test]
    fn insert_overwrites_and_returns_old() {
        let mut g = SquareGrid::new();
        // 第一次插入，無舊值
        let old = g.insert(plain(0, 0));
        assert!(old.is_none());

        // 第二次插入同座標，應覆蓋並回傳舊格子
        let mut overwrite = plain(0, 0);
        overwrite.terrain = Terrain::Forest;
        let old2 = g.insert(overwrite);
        assert!(old2.is_some());
        assert_eq!(old2.unwrap().terrain, Terrain::Plain);
        assert_eq!(
            g.get(SquareCoord::new(0, 0)).unwrap().terrain,
            Terrain::Forest
        );
    }

    #[test]
    fn remove_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        let removed = g.remove(SquareCoord::new(0, 0));
        assert!(removed.is_some());
        assert!(g.is_empty());
        assert!(g.remove(SquareCoord::new(0, 0)).is_none());
    }

    #[test]
    fn get_mut_modifies_in_place() {
        let mut g = SquareGrid::new();
        g.insert(plain(2, 3));
        g.get_mut(SquareCoord::new(2, 3)).unwrap().explored = true;
        assert!(g.get(SquareCoord::new(2, 3)).unwrap().explored);
    }

    #[test]
    fn world_seed_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xDEAD_BEEF_1234_5678);
        assert_eq!(g.world_seed(), 0xDEAD_BEEF_1234_5678);
    }

    // ── 序列化 ──────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip() {
        let mut g = SquareGrid::new();
        g.set_world_seed(0xABCD_EF01_2345_6789);
        g.insert(plain(0, 0));
        g.insert(water(1, 0));

        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();

        assert_eq!(back.world_seed(), 0xABCD_EF01_2345_6789);
        assert_eq!(back.len(), 2);
        assert!(back.contains(SquareCoord::new(0, 0)));
        assert_eq!(
            back.get(SquareCoord::new(1, 0)).unwrap().terrain,
            Terrain::WaterDeep
        );
    }

    #[test]
    fn serde_zone_and_tags() {
        let mut cell = plain(5, 5);
        cell.zone = "野外".into();
        cell.tags = vec!["危險".into(), "資源豐富".into()];

        let mut g = SquareGrid::new();
        g.insert(cell);
        let json = serde_json::to_string(&g).unwrap();
        let back: SquareGrid = serde_json::from_str(&json).unwrap();
        let c = back.get(SquareCoord::new(5, 5)).unwrap();
        assert_eq!(c.zone, "野外");
        assert_eq!(c.tags, vec!["危險", "資源豐富"]);
    }

    // ── 尋路：直線 ──────────────────────────────────────────────

    #[test]
    fn find_path_straight_line() {
        let mut g = SquareGrid::new();
        // 水平直線：(0,0) → (4,0)
        for x in 0..=4_i32 {
            g.insert(plain(x, 0));
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(4, 0))
            .unwrap();
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(4, 0)));
        // BFS 直線最短路：5 格（含起終點）
        assert_eq!(path.len(), 5);
    }

    #[test]
    fn find_path_diagonal() {
        // (0,0) → (3,3)：四向制下 Manhattan 距離 6，路徑 7 格（含起終點）。
        // 需要插入完整 4x4 可走區塊。
        let mut g = SquareGrid::new();
        for x in 0..=3_i32 {
            for y in 0..=3_i32 {
                g.insert(plain(x, y));
            }
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path.len(), 7);
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(3, 3)));
    }

    #[test]
    fn find_path_same_cell() {
        let mut g = SquareGrid::new();
        g.insert(plain(3, 3));
        let path = g
            .find_path(SquareCoord::new(3, 3), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path, vec![SquareCoord::new(3, 3)]);
    }

    // ── 尋路：繞過障礙 ──────────────────────────────────────────

    #[test]
    fn find_path_around_water_obstacle() {
        // 佈局（y 軸向北為正）：
        //   (0,0) — 水(1,0) — (2,0)
        //   (0,-1) — (1,-1) — (2,-1)
        //
        // 直路被水擋，BFS 應繞南方一行（y = -1）
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(water(1, 0)); // 障礙
        g.insert(plain(2, 0));
        g.insert(plain(0, -1));
        g.insert(plain(1, -1));
        g.insert(plain(2, -1));

        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(2, 0))
            .unwrap();

        // 路徑不得踩水格
        assert!(!path.contains(&SquareCoord::new(1, 0)));
        // 起點與終點正確
        assert_eq!(path.first(), Some(&SquareCoord::new(0, 0)));
        assert_eq!(path.last(), Some(&SquareCoord::new(2, 0)));
    }

    // ── 尋路：不可達 ────────────────────────────────────────────

    #[test]
    fn find_path_unreachable() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10)); // 孤立格，中間無路

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(10, 10))
                .is_none()
        );
    }

    #[test]
    fn find_path_blocked_by_water_ring() {
        // 目標格 (3,3) 被水格完整包圍，從 (0,0) 不可達
        //
        // 水格：(2,2)(3,2)(4,2)
        //       (2,3)     (4,3)
        //       (2,4)(3,4)(4,4)
        // 目標：(3,3)
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0)); // 起點（離目標有段距離）

        // 圍牆（全水）
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                if dx != 0 || dy != 0 {
                    g.insert(water(3 + dx, 3 + dy));
                }
            }
        }
        g.insert(plain(3, 3)); // 目標，被水圍死

        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
                .is_none()
        );
    }

    #[test]
    fn find_path_missing_endpoint_is_none() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        // 終點不在地圖中
        assert!(
            g.find_path(SquareCoord::new(0, 0), SquareCoord::new(1, 0))
                .is_none()
        );
    }

    #[test]
    fn road_and_building_traversal() {
        let mut g = SquareGrid::new();
        g.insert(road(0, 0));
        g.insert(shop(1, 0));
        g.insert(workshop(2, 0));

        // 街道 :: 踏入商貿 $
        assert!(g.can_move_between(SquareCoord::new(0, 0), SquareCoord::new(1, 0)));
        // 商貿 $ 踏回街道 ::
        assert!(g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(0, 0)));
        // 建築隔離法則：商貿 $ 直接穿牆到工坊 ⚒︎ 應被禁止
        assert!(!g.can_move_between(SquareCoord::new(1, 0), SquareCoord::new(2, 0)));
    }

    #[test]
    fn dynamic_mutation_chains() {
        let mut g = SquareGrid::new();
        let c = SquareCoord::new(0, 0);

        // 1. 開山挖礦 Mountain -> MinePass -> Mountain
        g.insert(GridCell::new(c, Terrain::Mountain, "山峰".to_string()));
        let (old, new_t, _) = g.mutate_cell_terrain(c, "mine").unwrap();
        assert_eq!(old, Terrain::Mountain);
        assert_eq!(new_t, Terrain::MinePass);
        let (_, new_t2, _) = g.mutate_cell_terrain(c, "collapse").unwrap();
        assert_eq!(new_t2, Terrain::Mountain);

        // 2. 伐林清地 Forest -> Plain
        g.insert(GridCell::new(c, Terrain::Forest, "樹林".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "deforest").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 3. 開墾整地 Plain -> EmptyFurrow
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plow").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);

        // 4. 栽種變化 EmptyFurrow -> FarmField -> EmptyFurrow -> DryField
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_paddy").unwrap();
        assert_eq!(new_t, Terrain::FarmField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_dry_crop").unwrap();
        assert_eq!(new_t, Terrain::DryField);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "harvest").unwrap();
        assert_eq!(new_t, Terrain::EmptyFurrow);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "plant_herbs").unwrap();
        assert_eq!(new_t, Terrain::HerbGarden);

        // 5. 鋪路 Plain <-> Road
        g.insert(GridCell::new(c, Terrain::Plain, "草野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "pave_road").unwrap();
        assert_eq!(new_t, Terrain::Road);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "break_road").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 6. 造橋 Water <-> Bridge
        g.insert(GridCell::new(c, Terrain::Water, "溪流".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_bridge").unwrap();
        assert_eq!(new_t, Terrain::Bridge);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish_bridge").unwrap();
        assert_eq!(new_t, Terrain::Water);

        // 7. 水文漲退 Plain <-> Swamp
        g.insert(GridCell::new(c, Terrain::Plain, "荒野".to_string()));
        let (_, new_t, _) = g.mutate_cell_terrain(c, "flood").unwrap();
        assert_eq!(new_t, Terrain::Swamp);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "drain").unwrap();
        assert_eq!(new_t, Terrain::Plain);

        // 8. 聚落營造 Plain -> House -> demolish -> Plain
        let (_, new_t, _) = g.mutate_cell_terrain(c, "build_house").unwrap();
        assert_eq!(new_t, Terrain::House);
        let (_, new_t, _) = g.mutate_cell_terrain(c, "demolish").unwrap();
        assert_eq!(new_t, Terrain::Plain);
    }
}

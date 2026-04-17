//! 方格地圖容器（SquareGrid）與格子單元（GridCell）。
//!
//! 設計意圖與 HexGrid 對稱：
//! - `SquareGrid` 持有 world_seed 及所有已揭露格子
//! - `GridCell` 對應 `HexCell`，共用 `Terrain` 與 `HexObject` 型別
//! - 揭露邏輯由 `reveal` 模組負責，此模組只管容器

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::hex::{HexObject, Terrain};

use super::coord::SquareCoord;

// ─── 格子單元 ───

/// 方格地圖單元（與 HexCell 對稱，共用 Terrain 與 HexObject 型別）
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

    /// 格子內的地上物（資源、地標）；復用 HexObject 結構，之後再改名
    #[serde(default)]
    pub objects: Vec<HexObject>,

    /// 是否已被玩家探索（影響前端渲染）
    #[serde(default)]
    pub explored: bool,
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
    cells: HashMap<SquareCoord, GridCell>,
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
            cells,
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
        self.cells.insert(coord, cell)
    }

    /// 移除格子；不存在時回傳 `None`。
    pub fn remove(&mut self, coord: SquareCoord) -> Option<GridCell> {
        let r = self.cells.remove(&coord);
        if r.is_some() { self.dirty = true; }
        r
    }

    /// 取得格子（未揭露回傳 `None`）
    pub fn get(&self, coord: SquareCoord) -> Option<&GridCell> {
        self.cells.get(&coord)
    }

    /// 取得格子（可寫）——呼叫者修改內容後應呼叫 `mark_dirty()`。
    pub fn get_mut(&mut self, coord: SquareCoord) -> Option<&mut GridCell> {
        self.cells.get_mut(&coord)
    }

    /// 查詢格子是否已揭露
    pub fn contains(&self, coord: SquareCoord) -> bool {
        self.cells.contains_key(&coord)
    }

    /// 已揭露格子總數
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// 遍歷所有已揭露格子
    pub fn cells(&self) -> impl Iterator<Item = &GridCell> {
        self.cells.values()
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
            .filter_map(|n| self.cells.get(&n))
            .collect()
    }

    /// 能否從 `from` 步行到相鄰的 `to`？
    ///
    /// 條件：`to` 存在於地圖、兩格相鄰（Chebyshev 距離 1）、目標地形可走。
    pub fn can_walk(&self, from: SquareCoord, to: SquareCoord) -> bool {
        if from.chebyshev_distance(to) != 1 {
            return false;
        }
        match self.cells.get(&to) {
            Some(c) => c.terrain.walkable(),
            None => false,
        }
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
            Terrain::Water,
            format!("水({x},{y})"),
        )
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
        assert_eq!(g.get(SquareCoord::new(0, 0)).unwrap().terrain, Terrain::Forest);
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
            Terrain::Water
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
        // 對角線：(0,0) → (3,3)，八方向移動下 Chebyshev 距離 3 步即可達
        let mut g = SquareGrid::new();
        for i in 0..=3_i32 {
            g.insert(plain(i, i));
        }
        let path = g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
            .unwrap();
        assert_eq!(path.len(), 4);
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

        assert!(g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(10, 10))
            .is_none());
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

        assert!(g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(3, 3))
            .is_none());
    }

    #[test]
    fn find_path_missing_endpoint_is_none() {
        let mut g = SquareGrid::new();
        g.insert(plain(0, 0));
        // 終點不在地圖中
        assert!(g
            .find_path(SquareCoord::new(0, 0), SquareCoord::new(1, 0))
            .is_none());
    }
}

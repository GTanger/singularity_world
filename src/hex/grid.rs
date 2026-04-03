use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::cell::HexCell;
use super::coord::{HexCoord, HexDir};

fn is_false(b: &bool) -> bool {
    !*b
}

/// 非相鄰連接（傳送門、隧道、城門捷徑等）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portal {
    pub name: String,
    pub from: HexCoord,
    pub to: HexCoord,
    pub bidirectional: bool,
    /// 是否計入官方聯外子圖（聚落 §一／§二）。預設 **false**——一般 portal 僅探索圖。
    #[serde(default, skip_serializing_if = "is_false")]
    pub counts_as_official_link: bool,
}

/// 交通邊：道路或水路（幹線／渡口等）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Road,
    Water,
}

/// 官方幹線 vs 捷徑（聚落 §1.1、§16.2）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportLinkClass {
    #[default]
    Official,
    Shortcut,
}

/// 邊端點：格座標或聚落 id（後者待 settlement 表對接）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TransportEndpoint {
    Settlement { settlement_id: String },
    Cell(HexCoord),
}

/// 明式交通邊（幹線、渡口）；持久化於 PostgreSQL `hex_transport_edges`（備份 `grid.json`）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportEdge {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub endpoint_a: TransportEndpoint,
    pub endpoint_b: TransportEndpoint,
    pub mode: TransportMode,
    #[serde(default = "default_true")]
    pub operational: bool,
    #[serde(default)]
    pub link_class: TransportLinkClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

fn default_true() -> bool {
    true
}

/// 連通圖層：官方聯外子圖 vs 探索全圖（聚落 §12.4）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkLayer {
    /// 僅 `can_walk`、官方幹線邊、`counts_as_official_link` portal
    Official,
    /// 併入捷徑、全部 portal、shortcut 邊
    Exploration,
}

/// 序列化用的障壁記錄
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct BarrierEntry {
    coord: HexCoord,
    dir: HexDir,
}

/// 六角格地圖
///
/// - **cells**：所有已存在的格子
/// - **barriers**：單方向通行阻斷（牆、懸崖等）
/// - **portals**：非相鄰連接
/// - **transport_edges**：明式幹線／渡口（含 `link_class`）
///
/// 相鄰格之間預設六方皆通；只有 barrier 和不可行走地形會阻斷。
#[derive(Debug, Clone, Default)]
pub struct HexGrid {
    /// 野外揭露 PRNG 種子（與座標混合；見 `reveal` 模組）
    world_seed: u64,
    cells: HashMap<HexCoord, HexCell>,
    barriers: HashSet<(HexCoord, HexDir)>,
    portals: Vec<Portal>,
    transport_edges: Vec<TransportEdge>,
}

// ── 自訂 Serialize / Deserialize（barriers 轉成 Vec 以相容 JSON）──

#[derive(Serialize, Deserialize)]
struct HexGridSer {
    #[serde(default)]
    world_seed: u64,
    cells: Vec<HexCell>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    barriers: Vec<BarrierEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    portals: Vec<Portal>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    transport_edges: Vec<TransportEdge>,
}

impl Serialize for HexGrid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let ser = HexGridSer {
            world_seed: self.world_seed,
            cells: self.cells.values().cloned().collect(),
            barriers: self
                .barriers
                .iter()
                .map(|(c, d)| BarrierEntry {
                    coord: *c,
                    dir: *d,
                })
                .collect(),
            portals: self.portals.clone(),
            transport_edges: self.transport_edges.clone(),
        };
        ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HexGrid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ser = HexGridSer::deserialize(deserializer)?;
        let cells = ser.cells.into_iter().map(|c| (c.coord, c)).collect();
        let barriers = ser
            .barriers
            .into_iter()
            .map(|b| (b.coord, b.dir))
            .collect();
        Ok(HexGrid {
            world_seed: ser.world_seed,
            cells,
            barriers,
            portals: ser.portals,
            transport_edges: ser.transport_edges,
        })
    }
}

// ── Cell CRUD ────────────────────────────────────────────────────

impl HexGrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// 由持久化層（PostgreSQL 正規化表）組裝；不重新驗證拓撲。
    pub fn from_parts(
        world_seed: u64,
        cells: HashMap<HexCoord, HexCell>,
        barriers: HashSet<(HexCoord, HexDir)>,
        portals: Vec<Portal>,
        transport_edges: Vec<TransportEdge>,
    ) -> Self {
        Self {
            world_seed,
            cells,
            barriers,
            portals,
            transport_edges,
        }
    }

    pub fn world_seed(&self) -> u64 {
        self.world_seed
    }

    pub fn set_world_seed(&mut self, seed: u64) {
        self.world_seed = seed;
    }

    pub fn insert(&mut self, cell: HexCell) -> Option<HexCell> {
        let coord = cell.coord;
        self.cells.insert(coord, cell)
    }

    pub fn remove(&mut self, coord: HexCoord) -> Option<HexCell> {
        self.barriers.retain(|(c, _)| *c != coord);
        self.cells.remove(&coord)
    }

    pub fn get(&self, coord: HexCoord) -> Option<&HexCell> {
        self.cells.get(&coord)
    }

    pub fn get_mut(&mut self, coord: HexCoord) -> Option<&mut HexCell> {
        self.cells.get_mut(&coord)
    }

    pub fn contains(&self, coord: HexCoord) -> bool {
        self.cells.contains_key(&coord)
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cells(&self) -> impl Iterator<Item = &HexCell> {
        self.cells.values()
    }

    pub fn coords(&self) -> impl Iterator<Item = &HexCoord> {
        self.cells.keys()
    }

    /// AABB 包圍盒（最小 q/r, 最大 q/r）
    pub fn bounds(&self) -> Option<(HexCoord, HexCoord)> {
        let mut it = self.cells.keys();
        let first = *it.next()?;
        let (mut min_q, mut min_r) = (first.q, first.r);
        let (mut max_q, mut max_r) = (first.q, first.r);
        for c in it {
            min_q = min_q.min(c.q);
            min_r = min_r.min(c.r);
            max_q = max_q.max(c.q);
            max_r = max_r.max(c.r);
        }
        Some((HexCoord::new(min_q, min_r), HexCoord::new(max_q, max_r)))
    }
}

// ── Barriers ─────────────────────────────────────────────────────

impl HexGrid {
    /// 單方向阻斷：從 coord 往 dir 不可通行
    pub fn add_barrier(&mut self, coord: HexCoord, dir: HexDir) {
        self.barriers.insert((coord, dir));
    }

    /// 雙向阻斷（牆）：A↔B 兩邊都擋
    pub fn add_wall(&mut self, a: HexCoord, b: HexCoord) {
        if let Some(dir) = a.direction_to(b) {
            self.barriers.insert((a, dir));
            self.barriers.insert((b, dir.opposite()));
        }
    }

    pub fn remove_barrier(&mut self, coord: HexCoord, dir: HexDir) {
        self.barriers.remove(&(coord, dir));
    }

    pub fn remove_wall(&mut self, a: HexCoord, b: HexCoord) {
        if let Some(dir) = a.direction_to(b) {
            self.barriers.remove(&(a, dir));
            self.barriers.remove(&(b, dir.opposite()));
        }
    }

    pub fn is_blocked(&self, coord: HexCoord, dir: HexDir) -> bool {
        self.barriers.contains(&(coord, dir))
    }

    pub fn barrier_count(&self) -> usize {
        self.barriers.len()
    }

    /// 供持久化正規化表遍歷（`HashSet` 無固定順序）。
    pub fn barriers_iter(&self) -> impl Iterator<Item = (HexCoord, HexDir)> + '_ {
        self.barriers.iter().map(|(c, d)| (*c, *d))
    }
}

// ── Portals ──────────────────────────────────────────────────────

impl HexGrid {
    pub fn add_portal(&mut self, portal: Portal) {
        self.portals.push(portal);
    }

    pub fn portals(&self) -> &[Portal] {
        &self.portals
    }

    pub fn add_transport_edge(&mut self, edge: TransportEdge) {
        self.transport_edges.push(edge);
    }

    pub fn transport_edges(&self) -> &[TransportEdge] {
        &self.transport_edges
    }

    /// 從 coord 出發可用的傳送門目的地（**全部** portal，探索圖用）
    pub fn portal_destinations(&self, coord: HexCoord) -> Vec<HexCoord> {
        self.portals
            .iter()
            .filter_map(|p| {
                if p.from == coord {
                    Some(p.to)
                } else if p.bidirectional && p.to == coord {
                    Some(p.from)
                } else {
                    None
                }
            })
            .collect()
    }
}

// ── 鄰接 & 移動 ─────────────────────────────────────────────────

impl HexGrid {
    /// 已存在的相鄰格（不考慮通行性）
    pub fn neighbors(&self, coord: HexCoord) -> Vec<&HexCell> {
        coord
            .neighbors()
            .into_iter()
            .filter_map(|n| self.cells.get(&n))
            .collect()
    }

    /// 能否從 from 步行到相鄰的 to？
    ///
    /// 條件：to 存在、相鄰、地形可走、雙方無障壁阻斷。
    pub fn can_walk(&self, from: HexCoord, to: HexCoord) -> bool {
        if !from.is_adjacent(to) {
            return false;
        }
        let target = match self.cells.get(&to) {
            Some(c) => c,
            None => return false,
        };
        if !target.terrain.walkable() {
            return false;
        }
        // barrier(A, dir) = 不能從 A 往 dir 離開
        // 牆 = add_wall 在兩端各加一條 barrier，自然雙擋
        if let Some(dir) = from.direction_to(to)
            && self.barriers.contains(&(from, dir))
        {
            return false;
        }
        true
    }

    /// 從 from 可步行到達的所有相鄰格
    pub fn walkable_neighbors(&self, from: HexCoord) -> Vec<&HexCell> {
        from.neighbors()
            .into_iter()
            .filter(|n| self.can_walk(from, *n))
            .filter_map(|n| self.cells.get(&n))
            .collect()
    }

    /// 從 from 出發可達鄰步（**探索圖**：步行 + 全 portal + 全部 operational 交通邊）
    pub fn reachable_from(&self, from: HexCoord) -> Vec<HexCoord> {
        self.reachable_from_layer(from, LinkLayer::Exploration)
    }

    /// 依層級：官方子圖或探索圖（見 `LinkLayer`）
    pub fn reachable_from_layer(&self, from: HexCoord, layer: LinkLayer) -> Vec<HexCoord> {
        let mut seen = HashSet::<HexCoord>::new();
        let mut out = Vec::new();
        let mut push = |c: HexCoord| {
            if self.contains(c) && seen.insert(c) {
                out.push(c);
            }
        };
        for n in from.neighbors() {
            if self.can_walk(from, n) {
                push(n);
            }
        }
        for p in &self.portals {
            if layer == LinkLayer::Official && !p.counts_as_official_link {
                continue;
            }
            let dest = if p.from == from {
                Some(p.to)
            } else if p.bidirectional && p.to == from {
                Some(p.from)
            } else {
                None
            };
            if let Some(d) = dest {
                push(d);
            }
        }
        for e in &self.transport_edges {
            if !e.operational {
                continue;
            }
            if layer == LinkLayer::Official && e.link_class != TransportLinkClass::Official {
                continue;
            }
            let (TransportEndpoint::Cell(a), TransportEndpoint::Cell(b)) =
                (&e.endpoint_a, &e.endpoint_b)
            else {
                continue;
            };
            if *a == from {
                push(*b);
            } else if *b == from {
                push(*a);
            }
        }
        out
    }

    /// BFS 最短路徑（跳數）；預設 **探索圖**（與舊行為一致）
    pub fn find_path(&self, from: HexCoord, to: HexCoord) -> Option<Vec<HexCoord>> {
        self.find_path_layer(from, to, LinkLayer::Exploration)
    }

    /// BFS 最短路徑，指定連通層
    pub fn find_path_layer(
        &self,
        from: HexCoord,
        to: HexCoord,
        layer: LinkLayer,
    ) -> Option<Vec<HexCoord>> {
        if from == to {
            return Some(vec![from]);
        }
        if !self.contains(from) || !self.contains(to) {
            return None;
        }

        let mut visited = HashSet::new();
        let mut parent: HashMap<HexCoord, HexCoord> = HashMap::new();
        let mut queue = VecDeque::new();

        visited.insert(from);
        queue.push_back(from);

        while let Some(current) = queue.pop_front() {
            for next in self.reachable_from_layer(current, layer) {
                if !self.contains(next) || visited.contains(&next) {
                    continue;
                }
                visited.insert(next);
                parent.insert(next, current);

                if next == to {
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
    use crate::hex::cell::Terrain;

    fn make_cell(q: i32, r: i32, terrain: Terrain) -> HexCell {
        HexCell::new(HexCoord::new(q, r), terrain, format!("({q},{r})"))
    }

    fn plain(q: i32, r: i32) -> HexCell {
        make_cell(q, r, Terrain::Plain)
    }

    #[test]
    fn empty_grid() {
        let g = HexGrid::new();
        assert!(g.is_empty());
        assert_eq!(g.len(), 0);
        assert!(g.bounds().is_none());
    }

    #[test]
    fn insert_and_get() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        assert_eq!(g.len(), 2);
        assert!(g.contains(HexCoord::new(0, 0)));
        assert!(!g.contains(HexCoord::new(5, 5)));
    }

    #[test]
    fn neighbors_only_existing() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        g.insert(plain(0, 1));
        let ns = g.neighbors(HexCoord::ORIGIN);
        assert_eq!(ns.len(), 2);
    }

    #[test]
    fn adjacent_walkable() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        assert!(g.can_walk(HexCoord::new(0, 0), HexCoord::new(1, 0)));
    }

    #[test]
    fn water_blocks_walk() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(make_cell(1, 0, Terrain::Water));
        assert!(!g.can_walk(HexCoord::new(0, 0), HexCoord::new(1, 0)));
    }

    #[test]
    fn barrier_blocks_walk() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        g.add_wall(HexCoord::new(0, 0), HexCoord::new(1, 0));
        assert!(!g.can_walk(HexCoord::new(0, 0), HexCoord::new(1, 0)));
        assert!(!g.can_walk(HexCoord::new(1, 0), HexCoord::new(0, 0)));

        g.remove_wall(HexCoord::new(0, 0), HexCoord::new(1, 0));
        assert!(g.can_walk(HexCoord::new(0, 0), HexCoord::new(1, 0)));
    }

    #[test]
    fn one_way_barrier() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        g.add_barrier(HexCoord::new(0, 0), HexDir::E);
        assert!(!g.can_walk(HexCoord::new(0, 0), HexCoord::new(1, 0)));
        assert!(g.can_walk(HexCoord::new(1, 0), HexCoord::new(0, 0)));
    }

    #[test]
    fn find_path_straight() {
        let mut g = HexGrid::new();
        for q in 0..=5 {
            g.insert(plain(q, 0));
        }
        let path = g
            .find_path(HexCoord::new(0, 0), HexCoord::new(5, 0))
            .unwrap();
        assert_eq!(path.len(), 6);
        assert_eq!(path[0], HexCoord::new(0, 0));
        assert_eq!(path[5], HexCoord::new(5, 0));
    }

    #[test]
    fn find_path_around_obstacle() {
        // (0,0)→(1,0) 被水擋，繞道 (1,-1)→(2,-1)→(2,0)
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(make_cell(1, 0, Terrain::Water));
        g.insert(plain(2, 0));
        g.insert(plain(1, -1));
        g.insert(plain(2, -1));
        let path = g
            .find_path(HexCoord::new(0, 0), HexCoord::new(2, 0))
            .unwrap();
        assert_eq!(path.len(), 4);
        assert!(!path.contains(&HexCoord::new(1, 0)));
    }

    #[test]
    fn find_path_unreachable() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10));
        assert!(g.find_path(HexCoord::new(0, 0), HexCoord::new(10, 10)).is_none());
    }

    #[test]
    fn portal_bypass() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10));
        g.add_portal(Portal {
            name: "傳送門".into(),
            from: HexCoord::new(0, 0),
            to: HexCoord::new(10, 10),
            bidirectional: true,
            counts_as_official_link: false,
        });
        let path = g
            .find_path(HexCoord::new(0, 0), HexCoord::new(10, 10))
            .unwrap();
        assert_eq!(path, vec![HexCoord::new(0, 0), HexCoord::new(10, 10)]);
    }

    #[test]
    fn bounds_calculation() {
        let mut g = HexGrid::new();
        g.insert(plain(-3, 2));
        g.insert(plain(5, -1));
        g.insert(plain(0, 0));
        let (lo, hi) = g.bounds().unwrap();
        assert_eq!(lo, HexCoord::new(-3, -1));
        assert_eq!(hi, HexCoord::new(5, 2));
    }

    #[test]
    fn serde_roundtrip() {
        let mut g = HexGrid::new();
        g.set_world_seed(0x1234_5678_ABCD_EF01);
        g.insert(plain(0, 0));
        g.insert(make_cell(1, 0, Terrain::Forest));
        g.add_wall(HexCoord::new(0, 0), HexCoord::new(1, 0));
        g.add_portal(Portal {
            name: "test".into(),
            from: HexCoord::ORIGIN,
            to: HexCoord::new(1, 0),
            bidirectional: false,
            counts_as_official_link: false,
        });

        let json = serde_json::to_string(&g).unwrap();
        let back: HexGrid = serde_json::from_str(&json).unwrap();
        assert_eq!(back.world_seed(), 0x1234_5678_ABCD_EF01);
        assert_eq!(back.len(), 2);
        assert_eq!(back.barrier_count(), 2);
        assert_eq!(back.portals().len(), 1);
        assert!(!back.can_walk(HexCoord::ORIGIN, HexCoord::new(1, 0)));
        assert!(!back.portals()[0].counts_as_official_link);
    }

    #[test]
    fn serde_portal_counts_official_roundtrip() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.add_portal(Portal {
            name: "幹線門".into(),
            from: HexCoord::ORIGIN,
            to: HexCoord::new(2, 0),
            bidirectional: true,
            counts_as_official_link: true,
        });
        let json = serde_json::to_string(&g).unwrap();
        assert!(json.contains("counts_as_official_link"));
        let back: HexGrid = serde_json::from_str(&json).unwrap();
        assert!(back.portals()[0].counts_as_official_link);
    }

    #[test]
    fn serde_transport_edges_roundtrip() {
        use TransportEndpoint as Te;
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(3, 0));
        g.add_transport_edge(TransportEdge {
            id: Some("e1".into()),
            endpoint_a: Te::Cell(HexCoord::ORIGIN),
            endpoint_b: Te::Cell(HexCoord::new(3, 0)),
            mode: TransportMode::Road,
            operational: true,
            link_class: TransportLinkClass::Official,
            weight: Some(1.0),
        });
        g.add_transport_edge(TransportEdge {
            id: None,
            endpoint_a: Te::Settlement {
                settlement_id: "set_a".into(),
            },
            endpoint_b: Te::Cell(HexCoord::new(3, 0)),
            mode: TransportMode::Water,
            operational: true,
            link_class: TransportLinkClass::Shortcut,
            weight: None,
        });
        let json = serde_json::to_string_pretty(&g).unwrap();
        let back: HexGrid = serde_json::from_str(&json).unwrap();
        assert_eq!(back.transport_edges().len(), 2);
        assert_eq!(
            back.transport_edges()[1].link_class,
            TransportLinkClass::Shortcut
        );
    }

    #[test]
    fn official_layer_ignores_default_portal() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10));
        g.add_portal(Portal {
            name: "捷徑門".into(),
            from: HexCoord::new(0, 0),
            to: HexCoord::new(10, 10),
            bidirectional: true,
            counts_as_official_link: false,
        });
        assert!(g
            .find_path_layer(HexCoord::new(0, 0), HexCoord::new(10, 10), LinkLayer::Official)
            .is_none());
        assert!(g
            .find_path_layer(HexCoord::new(0, 0), HexCoord::new(10, 10), LinkLayer::Exploration)
            .is_some());
    }

    #[test]
    fn official_layer_uses_flagged_portal() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(10, 10));
        g.add_portal(Portal {
            name: "官道門".into(),
            from: HexCoord::new(0, 0),
            to: HexCoord::new(10, 10),
            bidirectional: true,
            counts_as_official_link: true,
        });
        let p = g.find_path_layer(
            HexCoord::new(0, 0),
            HexCoord::new(10, 10),
            LinkLayer::Official,
        );
        assert_eq!(p, Some(vec![HexCoord::new(0, 0), HexCoord::new(10, 10)]));
    }

    #[test]
    fn shortcut_transport_only_in_exploration() {
        use TransportEndpoint as Te;
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(4, 0));
        g.add_transport_edge(TransportEdge {
            id: None,
            endpoint_a: Te::Cell(HexCoord::ORIGIN),
            endpoint_b: Te::Cell(HexCoord::new(4, 0)),
            mode: TransportMode::Road,
            operational: true,
            link_class: TransportLinkClass::Shortcut,
            weight: None,
        });
        assert!(g
            .find_path_layer(HexCoord::ORIGIN, HexCoord::new(4, 0), LinkLayer::Official)
            .is_none());
        assert!(g
            .find_path_layer(HexCoord::ORIGIN, HexCoord::new(4, 0), LinkLayer::Exploration)
            .is_some());
    }

    #[test]
    fn remove_cleans_barriers() {
        let mut g = HexGrid::new();
        g.insert(plain(0, 0));
        g.insert(plain(1, 0));
        g.add_wall(HexCoord::new(0, 0), HexCoord::new(1, 0));
        assert_eq!(g.barrier_count(), 2);
        g.remove(HexCoord::new(0, 0));
        assert_eq!(g.barrier_count(), 1);
    }
}

//! 銀鷗外島「全島單一無縫連續底盤」核心結構與靜態自然地景／幹道編譯器。
//!
//! 實時濾鏡與封包編碼位於 `island_substrate_pack.rs`；34 座城鎮街廓編譯位於 `island_substrate_towns.rs`。

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::OnceLock;

use crate::combat::wordplate::splitmix64;

use super::island_substrate_towns::compile_settlements;
use super::types::{VoronoiCellDef, WorldStaticDoc};

pub use super::island_substrate_pack::build_viewer_substrate_json;

/// 全島連續底盤世界座標原點 X（島嶼單位）。
pub const WX0: f64 = 1346.0;
/// 全島連續底盤世界座標原點 Y（島嶼單位）。
pub const WY0: f64 = 618.0;
/// 每島嶼單位切分為 `5` 格（`0.20` 單位／格）。
pub const GRID_SCALE: f64 = 5.0;
/// 全島網格寬度（格數）。
pub const GRID_W: usize = 860;
/// 全島網格高度（格數）。
pub const GRID_H: usize = 930;

/// 底盤地貌代碼（與前端 `island-parchment-map.js` 一對一）。
pub const TERRAIN_SEA: u8 = 0;
pub const TERRAIN_RIVER: u8 = 1;
pub const TERRAIN_LAND: u8 = 2;
pub const TERRAIN_MOUNTAIN: u8 = 3;
pub const TERRAIN_TREE: u8 = 4;
pub const TERRAIN_FARM: u8 = 5;
pub const TERRAIN_FOG: u8 = 6;
pub const TERRAIN_REEF_SAND: u8 = 7;

/// 道路與防禦構造代碼（低 3 bits 編碼於 overlay）。
pub const ROAD_NONE: u8 = 0;
pub const ROAD_COUNTRY_TRACK: u8 = 1;
pub const ROAD_BRIDGE_OR_PIER: u8 = 2;
pub const ROAD_TOWN_COBBLE: u8 = 3;
pub const ROAD_STONE_WALL: u8 = 4;

/// 建築量體代碼（右移 3 bits 編碼於 overlay）。
pub const BLDG_NONE: u8 = 0;
pub const BLDG_ROOF: u8 = 1;
pub const BLDG_TOWER: u8 = 2;
pub const BLDG_RUIN_WALL: u8 = 3;
pub const BLDG_VACANT_ROOF: u8 = 4;

/// 預先編譯之靜態全島連續底盤（以 `seed_hash` 單次初始化快取）。
#[derive(Debug, Clone)]
pub struct CompiledIslandSubstrate {
    pub seed_hash: String,
    pub cell_ids: Vec<u32>,
    pub cell_index_by_id: BTreeMap<u32, u16>,
    pub cell_owner: Vec<u16>,
    pub base_terrain: Vec<u8>,
    pub base_road: Vec<u8>,
    pub base_bldg: Vec<u8>,
    pub sea_dist_by_cell_edge: Vec<u8>,
    pub node_anchors: BTreeMap<String, [f32; 2]>,
    pub house_footprint_cells: BTreeMap<String, Vec<usize>>,
    pub poi_base_entries: Vec<SubstratePoiSeed>,
}

/// 地圖書法名牌與可定位地標種子。
#[derive(Debug, Clone)]
pub struct SubstratePoiSeed {
    pub id: String,
    pub cell_id: u32,
    pub label: String,
    pub tier: &'static str,
    pub gx: f32,
    pub gy: f32,
}

static SUBSTRATE_CACHE: OnceLock<CompiledIslandSubstrate> = OnceLock::new();

/// 取得全域單例之靜態全島無縫底盤；若世界檔案 `seed_hash` 一致則零重算直傳。
pub fn compiled_island_substrate(doc: &WorldStaticDoc) -> &'static CompiledIslandSubstrate {
    let cached = SUBSTRATE_CACHE.get_or_init(|| compile_substrate(doc));
    if cached.seed_hash == doc.seed_hash {
        cached
    } else {
        Box::leak(Box::new(compile_substrate(doc)))
    }
}

#[inline]
pub fn world_to_grid(wx: f64, wy: f64) -> (isize, isize) {
    let gx = ((wx - WX0) * GRID_SCALE).floor() as isize;
    let gy = ((wy - WY0) * GRID_SCALE).floor() as isize;
    (gx, gy)
}

#[inline]
pub fn world_to_grid_f32(wx: f64, wy: f64) -> [f32; 2] {
    [
        ((wx - WX0) * GRID_SCALE) as f32,
        ((wy - WY0) * GRID_SCALE) as f32,
    ]
}

#[inline]
pub(crate) fn idx_of(gx: isize, gy: isize) -> Option<usize> {
    if gx >= 0 && gy >= 0 && (gx as usize) < GRID_W && (gy as usize) < GRID_H {
        Some((gy as usize) * GRID_W + (gx as usize))
    } else {
        None
    }
}

fn point_in_polygon(px: f64, py: f64, poly: &[[f64; 2]]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let [xi, yi] = poly[i];
        let [xj, yj] = poly[j];
        let crosses = (yi > py) != (yj > py);
        if crosses {
            let denom = yj - yi;
            if denom.abs() > 1e-12 {
                let x_hit = (xj - xi) * (py - yi) / denom + xi;
                if px < x_hit {
                    inside = !inside;
                }
            }
        }
        j = i;
    }
    inside
}

pub(crate) fn noise01(x: isize, y: isize, salt: u64) -> f64 {
    let k = ((x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((y as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ salt;
    let h = splitmix64(k);
    (h & 0xFFFF) as f64 / 65535.0
}

fn compile_substrate(doc: &WorldStaticDoc) -> CompiledIslandSubstrate {
    let total = GRID_W * GRID_H;
    let mut cell_ids: Vec<u32> = doc.cells.values().map(|c| c.cell_id).collect();
    cell_ids.sort_unstable();
    let mut cell_index_by_id = BTreeMap::new();
    for (i, &cid) in cell_ids.iter().enumerate() {
        cell_index_by_id.insert(cid, i as u16);
    }

    let mut cell_owner = vec![u16::MAX; total];
    let mut base_terrain = vec![TERRAIN_SEA; total];
    let mut base_road = vec![ROAD_NONE; total];
    let mut base_bldg = vec![BLDG_NONE; total];

    for (cidx, &cid) in cell_ids.iter().enumerate() {
        let Some(cell) = doc.cells.get(&cid.to_string()) else {
            continue;
        };
        if cell.polygon.len() < 3 {
            continue;
        }
        let mut min_wx = f64::INFINITY;
        let mut max_wx = f64::NEG_INFINITY;
        let mut min_wy = f64::INFINITY;
        let mut max_wy = f64::NEG_INFINITY;
        for &[px, py] in &cell.polygon {
            min_wx = min_wx.min(px);
            max_wx = max_wx.max(px);
            min_wy = min_wy.min(py);
            max_wy = max_wy.max(py);
        }
        let gx0 = (((min_wx - WX0) * GRID_SCALE).floor() as isize - 1).max(0);
        let gx1 = (((max_wx - WX0) * GRID_SCALE).ceil() as isize + 1).min(GRID_W as isize - 1);
        let gy0 = (((min_wy - WY0) * GRID_SCALE).floor() as isize - 1).max(0);
        let gy1 = (((max_wy - WY0) * GRID_SCALE).ceil() as isize + 1).min(GRID_H as isize - 1);

        let owner_u16 = cidx as u16;
        for gy in gy0..=gy1 {
            let wy = WY0 + (gy as f64 + 0.5) / GRID_SCALE;
            for gx in gx0..=gx1 {
                let wx = WX0 + (gx as f64 + 0.5) / GRID_SCALE;
                if point_in_polygon(wx, wy, &cell.polygon) {
                    let pos = (gy as usize) * GRID_W + (gx as usize);
                    cell_owner[pos] = owner_u16;
                    base_terrain[pos] = TERRAIN_LAND;
                }
            }
        }
    }

    let sea_dist_by_cell_edge = compute_dist_from_predicate(&base_terrain, |t| t == TERRAIN_SEA);

    let mut town_clear_centers: Vec<(isize, isize, isize)> = Vec::new();
    for tval in doc.town_micro_maps.values() {
        let Some(cid) = tval
            .get("cell_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
        else {
            continue;
        };
        let is_core = tval.get("tier").is_none();
        if let Some(cell) = doc.cells.get(&cid.to_string()) {
            let (cgx, cgy) = world_to_grid(cell.center[0], cell.center[1]);
            let rad = if is_core { 12 } else { 5 };
            town_clear_centers.push((cgx, cgy, rad));
        }
    }

    compile_rivers_to_sea(doc, &cell_owner, &mut base_terrain, &town_clear_centers);
    compile_mountains(
        doc,
        &cell_ids,
        &cell_owner,
        &mut base_terrain,
        &town_clear_centers,
    );

    let mut node_anchors = BTreeMap::new();
    for (nid, nd) in &doc.nodes {
        if !nd.dungeon {
            node_anchors.insert(nid.clone(), world_to_grid_f32(nd.pos[0], nd.pos[1]));
        }
    }
    let mut house_footprint_cells = BTreeMap::new();
    let mut poi_base_entries = Vec::new();

    compile_settlements(
        doc,
        &cell_index_by_id,
        &cell_owner,
        &mut base_terrain,
        &mut base_road,
        &mut base_bldg,
        &mut node_anchors,
        &mut house_footprint_cells,
        &mut poi_base_entries,
    );
    ensure_all_rivers_reach_sea(&mut base_terrain);

    compile_highways_and_exits(
        doc,
        &cell_owner,
        &base_terrain,
        &mut base_road,
        &base_bldg,
        &node_anchors,
    );

    compile_farms_and_forests(
        doc,
        &cell_ids,
        &cell_owner,
        &sea_dist_by_cell_edge,
        &mut base_terrain,
        &base_road,
        &base_bldg,
    );

    for cell in doc.cells.values() {
        if !matches!(cell.kind.as_str(), "plague_ruin" | "ruin_dungeon") {
            continue;
        }
        let [gx, gy] = world_to_grid_f32(cell.center[0], cell.center[1]);
        poi_base_entries.push(SubstratePoiSeed {
            id: format!("cell:{}", cell.cell_id),
            cell_id: cell.cell_id,
            label: cell
                .title
                .split('（')
                .next()
                .unwrap_or(&cell.title)
                .to_string(),
            tier: "ruin",
            gx,
            gy,
        });
    }

    CompiledIslandSubstrate {
        seed_hash: doc.seed_hash.clone(),
        cell_ids,
        cell_index_by_id,
        cell_owner,
        base_terrain,
        base_road,
        base_bldg,
        sea_dist_by_cell_edge,
        node_anchors,
        house_footprint_cells,
        poi_base_entries,
    }
}

fn compute_dist_from_predicate<F>(terrain: &[u8], pred: F) -> Vec<u8>
where
    F: Fn(u8) -> bool,
{
    let total = GRID_W * GRID_H;
    let mut dist = vec![u8::MAX; total];
    let mut queue = VecDeque::new();
    for (i, &t) in terrain.iter().enumerate() {
        if pred(t) {
            dist[i] = 0;
            queue.push_back(i);
        }
    }
    while let Some(cur) = queue.pop_front() {
        let d = dist[cur];
        if d >= 24 {
            continue;
        }
        let cx = cur % GRID_W;
        let cy = cur / GRID_W;
        let next_d = d + 1;
        for nopt in [
            if cx > 0 { Some(cur - 1) } else { None },
            if cx + 1 < GRID_W { Some(cur + 1) } else { None },
            if cy > 0 { Some(cur - GRID_W) } else { None },
            if cy + 1 < GRID_H {
                Some(cur + GRID_W)
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        {
            if dist[nopt] > next_d {
                dist[nopt] = next_d;
                queue.push_back(nopt);
            }
        }
    }
    dist
}

fn near_town_plaza(gx: isize, gy: isize, towns: &[(isize, isize, isize)]) -> bool {
    towns.iter().any(|&(tx, ty, r)| {
        let dx = gx - tx;
        let dy = gy - ty;
        dx * dx + dy * dy <= r * r
    })
}

fn carve_river_segment(
    p0: (isize, isize),
    p1: (isize, isize),
    radius: isize,
    terrain: &mut [u8],
    towns: &[(isize, isize, isize)],
) {
    let steps = (p1.0 - p0.0).abs().max((p1.1 - p0.1).abs()).max(1);
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let perp_x = -(p1.1 - p0.1) as f64 / steps as f64;
        let perp_y = (p1.0 - p0.0) as f64 / steps as f64;
        let bend = (t * std::f64::consts::PI).sin() * 2.1;
        let mut cx = (p0.0 as f64 + (p1.0 - p0.0) as f64 * t + perp_x * bend).round() as isize;
        let mut cy = (p0.1 as f64 + (p1.1 - p0.1) as f64 * t + perp_y * bend).round() as isize;
        for &(tx, ty, tr) in towns {
            let dx = cx - tx;
            let dy = cy - ty;
            let d = ((dx * dx + dy * dy) as f64).sqrt();
            let safe_r = (tr + radius + 1) as f64;
            if d < safe_r {
                let ux = if d > 0.1 { dx as f64 / d } else { 1.0 };
                let uy = if d > 0.1 { dy as f64 / d } else { 0.0 };
                cx = tx + (ux * safe_r).round() as isize;
                cy = ty + (uy * safe_r).round() as isize;
            }
        }
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx * dx + dy * dy <= radius * radius + 1 {
                    let gx = cx + dx;
                    let gy = cy + dy;
                    let inside_inner_wall = towns.iter().any(|&(tx, ty, tr)| {
                        let keep_r = (tr - 2).max(2);
                        let tdx = gx - tx;
                        let tdy = gy - ty;
                        tdx * tdx + tdy * tdy <= keep_r * keep_r
                    });
                    if inside_inner_wall {
                        continue;
                    }
                    if let Some(pos) = idx_of(gx, gy)
                        && terrain[pos] != TERRAIN_SEA
                    {
                        terrain[pos] = TERRAIN_RIVER;
                    }
                }
            }
        }
    }
}

fn ensure_all_rivers_reach_sea(terrain: &mut [u8]) {
    let total = GRID_W * GRID_H;
    let mut visited = vec![false; total];
    let mut q = VecDeque::new();
    for start in 0..total {
        if terrain[start] != TERRAIN_RIVER || visited[start] {
            continue;
        }
        visited[start] = true;
        q.push_back(start);
        let mut comp = Vec::new();
        let mut touches_sea = false;
        while let Some(cur) = q.pop_front() {
            comp.push(cur);
            let cx = cur % GRID_W;
            let cy = cur / GRID_W;
            for nopt in [
                if cx > 0 { Some(cur - 1) } else { None },
                if cx + 1 < GRID_W { Some(cur + 1) } else { None },
                if cy > 0 { Some(cur - GRID_W) } else { None },
                if cy + 1 < GRID_H {
                    Some(cur + GRID_W)
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            {
                if terrain[nopt] == TERRAIN_SEA {
                    touches_sea = true;
                } else if terrain[nopt] == TERRAIN_RIVER && !visited[nopt] {
                    visited[nopt] = true;
                    q.push_back(nopt);
                }
            }
        }
        if !touches_sea {
            for pos in comp {
                terrain[pos] = TERRAIN_LAND;
            }
        }
    }
}

fn compile_rivers_to_sea(
    doc: &WorldStaticDoc,
    cell_owner: &[u16],
    terrain: &mut [u8],
    towns: &[(isize, isize, isize)],
) {
    let mut sources: Vec<&VoronoiCellDef> = doc
        .cells
        .values()
        .filter(|c| {
            (!c.is_coast && c.flux >= 28) || (!c.is_coast && c.kind == "wilderness" && c.flux >= 24)
        })
        .collect();
    sources.sort_by_key(|c| c.cell_id);

    for src in sources {
        let mut cur = src;
        let mut seen = BTreeSet::new();
        seen.insert(cur.cell_id);
        let mut chain: Vec<&VoronoiCellDef> = vec![cur];

        for _ in 0..10 {
            if cur.is_coast {
                break;
            }
            let best_next = cur
                .neighbors_f6
                .iter()
                .filter_map(|nb| doc.cells.get(&nb.to_string()))
                .filter(|nb| !seen.contains(&nb.cell_id))
                .max_by(|a, b| {
                    let sa =
                        (if a.is_coast { 1000 } else { 0 }) + (cur.height - a.height) * 40 + a.flux;
                    let sb =
                        (if b.is_coast { 1000 } else { 0 }) + (cur.height - b.height) * 40 + b.flux;
                    sa.cmp(&sb)
                });
            let Some(next_cell) = best_next else {
                break;
            };
            seen.insert(next_cell.cell_id);
            chain.push(next_cell);
            cur = next_cell;
        }

        if !cur.is_coast {
            continue;
        }

        for pair in chain.windows(2) {
            let a = world_to_grid(pair[0].center[0], pair[0].center[1]);
            let b = world_to_grid(pair[1].center[0], pair[1].center[1]);
            carve_river_segment(a, b, 1, terrain, towns);
        }

        let c_pt = world_to_grid(cur.center[0], cur.center[1]);
        let start_pt = if near_town_plaza(c_pt.0, c_pt.1, towns) && chain.len() >= 2 {
            let p_pt = world_to_grid(
                chain[chain.len() - 2].center[0],
                chain[chain.len() - 2].center[1],
            );
            ((p_pt.0 + c_pt.0) / 2, (p_pt.1 + c_pt.1) / 2)
        } else {
            c_pt
        };

        if let Some(sea_pt) = find_nearest_sea_point(start_pt, cell_owner) {
            carve_river_segment(start_pt, sea_pt, 1, terrain, towns);
        }
    }
    ensure_all_rivers_reach_sea(terrain);
}

fn find_nearest_sea_point(start: (isize, isize), cell_owner: &[u16]) -> Option<(isize, isize)> {
    let mut best: Option<(isize, (isize, isize))> = None;
    let max_r: isize = 55;
    for gy in (start.1 - max_r).max(0)..=(start.1 + max_r).min(GRID_H as isize - 1) {
        for gx in (start.0 - max_r).max(0)..=(start.0 + max_r).min(GRID_W as isize - 1) {
            let pos = (gy as usize) * GRID_W + (gx as usize);
            if cell_owner[pos] == u16::MAX {
                let d2 = (gx - start.0).pow(2) + (gy - start.1).pow(2);
                if best.is_none_or(|(bd, _)| d2 < bd) {
                    best = Some((d2, (gx, gy)));
                }
            }
        }
    }
    best.map(|(_, pt)| pt)
}

fn compile_mountains(
    doc: &WorldStaticDoc,
    cell_ids: &[u32],
    cell_owner: &[u16],
    terrain: &mut [u8],
    towns: &[(isize, isize, isize)],
) {
    let mut ridge_by_idx = vec![false; cell_ids.len()];
    for (idx, &cid) in cell_ids.iter().enumerate() {
        if let Some(cell) = doc.cells.get(&cid.to_string()) {
            ridge_by_idx[idx] = cell.height >= 24
                || (cell.height == 23
                    && matches!(cell.kind.as_str(), "wilderness" | "road_corridor"));
        }
    }

    for gy in 2..(GRID_H as isize - 2) {
        for gx in 2..(GRID_W as isize - 2) {
            let pos = (gy as usize) * GRID_W + (gx as usize);
            if terrain[pos] != TERRAIN_LAND {
                continue;
            }
            let owner = cell_owner[pos];
            if owner == u16::MAX || !ridge_by_idx[owner as usize] || near_town_plaza(gx, gy, towns)
            {
                continue;
            }
            let Some(cell) = doc.cells.get(&cell_ids[owner as usize].to_string()) else {
                continue;
            };
            let (cgx, cgy) = world_to_grid(cell.center[0], cell.center[1]);
            let d = (((gx - cgx).pow(2) + (gy - cgy).pow(2)) as f64).sqrt();
            let wob = noise01(gx / 2, gy / 2, 0x4D54_4E01) * 4.0;
            let limit = if cell.height >= 25 {
                13.5
            } else if cell.height == 24 {
                9.5
            } else {
                5.8
            };
            if d + wob <= limit {
                terrain[pos] = TERRAIN_MOUNTAIN;
            }
        }
    }
}

fn draw_road_segment(
    p0: (isize, isize),
    p1: (isize, isize),
    cell_owner: &[u16],
    terrain: &[u8],
    road: &mut [u8],
    bldg: &[u8],
) {
    let steps = (p1.0 - p0.0).abs().max((p1.1 - p0.1).abs()).max(1);
    for s in 0..=steps {
        let t = s as f64 / steps as f64;
        let gx = (p0.0 as f64 + (p1.0 - p0.0) as f64 * t).round() as isize;
        let gy = (p0.1 as f64 + (p1.1 - p0.1) as f64 * t).round() as isize;
        let Some(pos) = idx_of(gx, gy) else {
            continue;
        };
        if cell_owner[pos] == u16::MAX || terrain[pos] == TERRAIN_SEA || bldg[pos] != BLDG_NONE {
            continue;
        }
        if terrain[pos] == TERRAIN_RIVER {
            road[pos] = ROAD_BRIDGE_OR_PIER;
        } else if road[pos] == ROAD_NONE {
            road[pos] = ROAD_COUNTRY_TRACK;
        }
    }
}

fn compile_highways_and_exits(
    doc: &WorldStaticDoc,
    cell_owner: &[u16],
    terrain: &[u8],
    road: &mut [u8],
    bldg: &[u8],
    node_anchors: &BTreeMap<String, [f32; 2]>,
) {
    for hwy in &doc.highways {
        for pair in hwy.points.windows(2) {
            let p0 = world_to_grid(pair[0][0], pair[0][1]);
            let p1 = world_to_grid(pair[1][0], pair[1][1]);
            draw_road_segment(p0, p1, cell_owner, terrain, road, bldg);
        }
    }
    for (nid, nd) in &doc.nodes {
        if nd.dungeon
            || nd.indoor
            || !matches!(nd.kind.as_str(), "gate" | "village_gate" | "road" | "plaza")
        {
            continue;
        }
        let Some(&a0) = node_anchors.get(nid) else {
            continue;
        };
        let p0 = (a0[0].floor() as isize, a0[1].floor() as isize);
        for ex in &nd.exits {
            if let Some(dst) = doc.nodes.get(&ex.to)
                && !dst.dungeon
                && !dst.indoor
                && dst.kind != "dock"
                && let Some(&a1) = node_anchors.get(&ex.to)
            {
                let p1 = (a1[0].floor() as isize, a1[1].floor() as isize);
                let dist2 = (p1.0 - p0.0).pow(2) + (p1.1 - p0.1).pow(2);
                if dist2 <= 900 {
                    draw_road_segment(p0, p1, cell_owner, terrain, road, bldg);
                }
            }
        }
    }
}

fn compile_farms_and_forests(
    doc: &WorldStaticDoc,
    cell_ids: &[u32],
    cell_owner: &[u16],
    sea_dist: &[u8],
    terrain: &mut [u8],
    road: &[u8],
    bldg: &[u8],
) {
    let mut cell_kind_tag = vec![0u8; cell_ids.len()];
    for (idx, &cid) in cell_ids.iter().enumerate() {
        let Some(c) = doc.cells.get(&cid.to_string()) else {
            continue;
        };
        let is_farm = matches!(c.resource_slug.as_str(), "wheat_grain" | "dairy_cheese");
        let is_forest = c.biome == 6
            || c.biome == 9
            || matches!(
                c.resource_slug.as_str(),
                "timber_log" | "charcoal_sack" | "herb_bundle" | "beast_tallow"
            )
            || cid == 4525;
        cell_kind_tag[idx] = if is_farm {
            2
        } else if is_forest {
            1
        } else if c.kind == "plague_ruin" {
            3
        } else {
            0
        };
    }

    let near_bldg = |gx: isize, gy: isize| -> bool {
        for dy in -2..=2 {
            for dx in -2..=2 {
                if let Some(pos) = idx_of(gx + dx, gy + dy)
                    && bldg[pos] != BLDG_NONE
                {
                    return true;
                }
            }
        }
        false
    };

    for gy in 2..(GRID_H as isize - 2) {
        for gx in 2..(GRID_W as isize - 2) {
            let pos = (gy as usize) * GRID_W + (gx as usize);
            if terrain[pos] != TERRAIN_LAND || road[pos] != ROAD_NONE || bldg[pos] != BLDG_NONE {
                continue;
            }
            if sea_dist[pos] <= 1 || near_bldg(gx, gy) {
                continue;
            }
            let owner = cell_owner[pos];
            if owner == u16::MAX {
                continue;
            }
            let tag = cell_kind_tag[owner as usize];
            let block_n = noise01(gx / 6, gy / 6, 0xFA01_0001);
            let fine_n = noise01(gx, gy, 0xFA02_0002);
            if tag == 2 {
                if block_n > 0.34 && fine_n > 0.14 && sea_dist[pos] >= 3 {
                    terrain[pos] = TERRAIN_FARM;
                } else if fine_n > 0.93 {
                    terrain[pos] = TERRAIN_TREE;
                }
            } else if tag == 1 {
                let ridge_wob = (gx as f64 * 0.22 + gy as f64 * 0.19).sin() * 0.12;
                if block_n + ridge_wob > 0.32 && fine_n > 0.12 {
                    terrain[pos] = TERRAIN_TREE;
                }
            } else if tag != 3 && ((block_n > 0.70 && fine_n > 0.28) || fine_n > 0.955) {
                terrain[pos] = TERRAIN_TREE;
            }
        }
    }
}

//! 全島靜態道路 A* 編織與室內／地城門戶格定位輔助模組。

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, VecDeque};

use super::island_substrate::{
    BLDG_NONE, GRID_H, GRID_W, ROAD_BRIDGE_OR_PIER, ROAD_COUNTRY_TRACK, ROAD_NONE, ROAD_STONE_WALL,
    ROAD_TOWN_COBBLE, TERRAIN_LAND, TERRAIN_MOUNTAIN, TERRAIN_REEF_SAND, TERRAIN_RIVER,
    TERRAIN_SEA, TERRAIN_TREE, idx_of, world_to_grid,
};
use super::types::WorldStaticDoc;

/// 判斷靜態格點是否可步行（含碼頭木橋、河橋與陸路；排除海、無橋河、山地、建物與城牆）。
#[inline]
pub fn is_static_walkable(
    cell_owner: &[u16],
    terrain: &[u8],
    road: &[u8],
    bldg: &[u8],
    pos: usize,
) -> bool {
    if bldg[pos] != BLDG_NONE || road[pos] == ROAD_STONE_WALL {
        return false;
    }
    if road[pos] == ROAD_BRIDGE_OR_PIER {
        return true;
    }
    if cell_owner[pos] == u16::MAX {
        return false;
    }
    !matches!(terrain[pos], TERRAIN_SEA | TERRAIN_RIVER | TERRAIN_MOUNTAIN)
}

/// 將全數室外節點（`!dungeon && !indoor`）之錨點對齊到最近可步格點。
pub(super) fn snap_outdoor_node_anchors(
    doc: &WorldStaticDoc,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &[u8],
    bldg: &[u8],
    node_anchors: &mut BTreeMap<String, [f32; 2]>,
) {
    for (nid, nd) in &doc.nodes {
        if nd.dungeon || nd.indoor {
            continue;
        }
        let Some(&[ax, ay]) = node_anchors.get(nid) else {
            continue;
        };
        let gx0 = (ax.floor() as isize).clamp(0, GRID_W as isize - 1);
        let gy0 = (ay.floor() as isize).clamp(0, GRID_H as isize - 1);
        if let Some(pos) = idx_of(gx0, gy0)
            && is_static_walkable(cell_owner, terrain, road, bldg, pos)
        {
            continue;
        }
        let mut best: Option<(isize, usize, isize, isize)> = None;
        for r in 1..=32isize {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let Some(pos) = idx_of(gx0 + dx, gy0 + dy) else {
                        continue;
                    };
                    if !is_static_walkable(cell_owner, terrain, road, bldg, pos) {
                        continue;
                    }
                    let road_bias = if road[pos] != ROAD_NONE { 0 } else { 1 };
                    let key = (dx * dx + dy * dy) * 2 + road_bias;
                    if best.is_none_or(|(bk, _, _, _)| key < bk) {
                        best = Some((key, pos, gx0 + dx, gy0 + dy));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        if let Some((_, _, bx, by)) = best {
            node_anchors.insert(nid.clone(), [bx as f32 + 0.5, by as f32 + 0.5]);
        } else if let Some(pos) = idx_of(gx0, gy0)
            && cell_owner[pos] != u16::MAX
            && bldg[pos] == BLDG_NONE
            && terrain[pos] == TERRAIN_MOUNTAIN
        {
            terrain[pos] = TERRAIN_LAND;
            node_anchors.insert(nid.clone(), [gx0 as f32 + 0.5, gy0 as f32 + 0.5]);
        }
    }
}

struct AStarScratch {
    stamp: Vec<u32>,
    closed: Vec<u32>,
    g_score: Vec<u32>,
    parent: Vec<u32>,
    epoch: u32,
}

impl AStarScratch {
    fn new(total: usize) -> Self {
        Self {
            stamp: vec![0; total],
            closed: vec![0; total],
            g_score: vec![u32::MAX; total],
            parent: vec![u32::MAX; total],
            epoch: 0,
        }
    }

    fn begin_search(&mut self) -> u32 {
        self.epoch = self.epoch.saturating_add(1);
        if self.epoch == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.epoch = 1;
        }
        self.epoch
    }
}

fn draw_raw_segment(
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
        if road[pos] == ROAD_STONE_WALL {
            continue;
        }
        if terrain[pos] == TERRAIN_RIVER {
            road[pos] = ROAD_BRIDGE_OR_PIER;
        } else if road[pos] == ROAD_NONE {
            road[pos] = ROAD_COUNTRY_TRACK;
        }
    }
}

fn route_edge_astar(
    scratch: &mut AStarScratch,
    start: (isize, isize),
    goal: (isize, isize),
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &[u8],
) {
    let Some(start_pos) = idx_of(start.0, start.1) else {
        return;
    };
    let Some(goal_pos) = idx_of(goal.0, goal.1) else {
        return;
    };
    if start_pos == goal_pos {
        return;
    }

    let epoch = scratch.begin_search();
    scratch.stamp[start_pos] = epoch;
    scratch.g_score[start_pos] = 0;
    scratch.parent[start_pos] = u32::MAX;

    let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
    let h0 = ((goal.0 - start.0).abs() + (goal.1 - start.1).abs()) as u32 * 2;
    heap.push(Reverse((h0, start_pos)));

    const DIRS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    let mut found = false;

    while let Some(Reverse((_, cur))) = heap.pop() {
        if scratch.closed[cur] == epoch {
            continue;
        }
        scratch.closed[cur] = epoch;
        if cur == goal_pos {
            found = true;
            break;
        }
        let cx = (cur % GRID_W) as isize;
        let cy = (cur / GRID_W) as isize;
        let cur_g = scratch.g_score[cur];

        for (dx, dy) in DIRS {
            let nx = cx + dx;
            let ny = cy + dy;
            let Some(npos) = idx_of(nx, ny) else {
                continue;
            };
            if scratch.closed[npos] == epoch {
                continue;
            }
            if npos != goal_pos
                && (cell_owner[npos] == u16::MAX
                    || bldg[npos] != BLDG_NONE
                    || road[npos] == ROAD_STONE_WALL
                    || (terrain[npos] == TERRAIN_SEA && road[npos] != ROAD_BRIDGE_OR_PIER))
            {
                continue;
            }
            let step_cost: u32 = if matches!(road[npos], ROAD_TOWN_COBBLE | ROAD_BRIDGE_OR_PIER) {
                2
            } else if road[npos] == ROAD_COUNTRY_TRACK {
                3
            } else {
                match terrain[npos] {
                    TERRAIN_LAND | TERRAIN_TREE => {
                        if terrain[npos] == TERRAIN_TREE {
                            14
                        } else {
                            10
                        }
                    }
                    TERRAIN_RIVER => 35,
                    TERRAIN_MOUNTAIN => 45,
                    TERRAIN_REEF_SAND => 65,
                    _ => 20,
                }
            };
            let ng = cur_g.saturating_add(step_cost);
            let seen = scratch.stamp[npos] == epoch;
            if !seen || ng < scratch.g_score[npos] {
                scratch.stamp[npos] = epoch;
                scratch.g_score[npos] = ng;
                scratch.parent[npos] = cur as u32;
                let h = ((goal.0 - nx).abs() + (goal.1 - ny).abs()) as u32 * 2;
                heap.push(Reverse((ng.saturating_add(h), npos)));
            }
        }
    }

    if !found {
        return;
    }

    let mut cur = goal_pos;
    loop {
        if cell_owner[cur] != u16::MAX && bldg[cur] == BLDG_NONE && road[cur] != ROAD_STONE_WALL {
            if terrain[cur] == TERRAIN_RIVER {
                road[cur] = ROAD_BRIDGE_OR_PIER;
            } else {
                if terrain[cur] == TERRAIN_MOUNTAIN {
                    terrain[cur] = TERRAIN_LAND;
                }
                if road[cur] == ROAD_NONE {
                    road[cur] = ROAD_COUNTRY_TRACK;
                }
            }
        }
        let p = scratch.parent[cur];
        if p == u32::MAX || cur == start_pos {
            break;
        }
        cur = p as usize;
    }
}

/// 沿公路網與全部 462 座非海非潮汐室外邊線以加權 A* 編織連續 4 連通道路與河橋。
pub(super) fn compile_all_trails_with_astar(
    doc: &WorldStaticDoc,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &[u8],
    node_anchors: &BTreeMap<String, [f32; 2]>,
) {
    for hwy in &doc.highways {
        for pair in hwy.points.windows(2) {
            let p0 = world_to_grid(pair[0][0], pair[0][1]);
            let p1 = world_to_grid(pair[1][0], pair[1][1]);
            draw_raw_segment(p0, p1, cell_owner, terrain, road, bldg);
        }
    }

    let mut edges: Vec<(isize, String, String)> = Vec::new();
    for (nid, nd) in &doc.nodes {
        if nd.dungeon || nd.indoor {
            continue;
        }
        let Some(&a0) = node_anchors.get(nid) else {
            continue;
        };
        let p0 = (a0[0].floor() as isize, a0[1].floor() as isize);
        for ex in &nd.exits {
            if ex.kind == "sea" || ex.env_gate == "low_tide" {
                continue;
            }
            let Some(dst) = doc.nodes.get(&ex.to) else {
                continue;
            };
            if dst.dungeon || dst.indoor || nid.as_str() >= ex.to.as_str() {
                continue;
            }
            let Some(&a1) = node_anchors.get(&ex.to) else {
                continue;
            };
            let p1 = (a1[0].floor() as isize, a1[1].floor() as isize);
            let dist2 = (p1.0 - p0.0).pow(2) + (p1.1 - p0.1).pow(2);
            edges.push((dist2, nid.clone(), ex.to.clone()));
        }
    }
    edges.sort();

    let mut scratch = AStarScratch::new(GRID_W * GRID_H);
    for (_, u, v) in edges {
        let Some(&a0) = node_anchors.get(&u) else {
            continue;
        };
        let Some(&a1) = node_anchors.get(&v) else {
            continue;
        };
        let p0 = (a0[0].floor() as isize, a0[1].floor() as isize);
        let p1 = (a1[0].floor() as isize, a1[1].floor() as isize);
        route_edge_astar(&mut scratch, p0, p1, cell_owner, terrain, road, bldg);
    }
}

fn nearest_outdoor_anchor(
    doc: &WorldStaticDoc,
    start_nid: &str,
    node_anchors: &BTreeMap<String, [f32; 2]>,
) -> Option<[f32; 2]> {
    let mut visited = BTreeMap::<String, ()>::new();
    let mut queue = VecDeque::new();
    visited.insert(start_nid.to_string(), ());
    queue.push_back(start_nid.to_string());
    while let Some(cur) = queue.pop_front() {
        let Some(nd) = doc.nodes.get(&cur) else {
            continue;
        };
        if !nd.indoor
            && !nd.dungeon
            && let Some(&anch) = node_anchors.get(&cur)
        {
            return Some(anch);
        }
        for ex in &nd.exits {
            if !visited.contains_key(&ex.to) {
                visited.insert(ex.to.clone(), ());
                queue.push_back(ex.to.clone());
            }
        }
    }
    None
}

fn has_open_orth_walkable(
    cell_owner: &[u16],
    terrain: &[u8],
    road: &[u8],
    bldg: &[u8],
    gx: isize,
    gy: isize,
) -> bool {
    [(1, 0), (-1, 0), (0, 1), (0, -1)]
        .iter()
        .any(|&(ox2, oy2)| {
            idx_of(gx + ox2, gy + oy2)
                .is_some_and(|op| is_static_walkable(cell_owner, terrain, road, bldg, op))
        })
}

/// 為所有室內節點與 `oi:dng_pit:entry` 計算對應門外可步道路格 `door_tiles`。
pub(super) fn compute_door_tiles(
    doc: &WorldStaticDoc,
    cell_owner: &[u16],
    terrain: &[u8],
    road: &[u8],
    bldg: &[u8],
    node_anchors: &BTreeMap<String, [f32; 2]>,
    house_footprint_cells: &BTreeMap<String, Vec<usize>>,
) -> BTreeMap<String, [u16; 2]> {
    let mut doors = BTreeMap::new();
    for (nid, nd) in &doc.nodes {
        if !nd.indoor && nid != "oi:dng_pit:entry" {
            continue;
        }
        let outdoor_ref = nearest_outdoor_anchor(doc, nid, node_anchors).or_else(|| {
            node_anchors
                .get(nid)
                .copied()
                .or(Some(super::island_substrate::world_to_grid_f32(
                    nd.pos[0], nd.pos[1],
                )))
        });
        let Some([ox, oy]) = outdoor_ref else {
            continue;
        };

        let mut best: Option<(f32, u16, u16)> = None;
        if let Some(fp) = house_footprint_cells.get(nid) {
            for &pos in fp {
                let fx = (pos % GRID_W) as isize;
                let fy = (pos / GRID_W) as isize;
                for dy in -1..=1isize {
                    for dx in -1..=1isize {
                        if dx == 0 && dy == 0 {
                            continue;
                        }
                        let Some(npos) = idx_of(fx + dx, fy + dy) else {
                            continue;
                        };
                        if !is_static_walkable(cell_owner, terrain, road, bldg, npos) {
                            continue;
                        }
                        if !has_open_orth_walkable(
                            cell_owner,
                            terrain,
                            road,
                            bldg,
                            fx + dx,
                            fy + dy,
                        ) {
                            continue;
                        }
                        let nx = (fx + dx) as f32 + 0.5;
                        let ny = (fy + dy) as f32 + 0.5;
                        let dist = (nx - ox).powi(2) + (ny - oy).powi(2);
                        let road_penalty = match road[npos] {
                            ROAD_TOWN_COBBLE | ROAD_BRIDGE_OR_PIER => 0.0,
                            ROAD_COUNTRY_TRACK => 2.0,
                            _ => 8.0,
                        };
                        let diag_penalty = if dx != 0 && dy != 0 { 0.8 } else { 0.0 };
                        let score = dist + road_penalty + diag_penalty;
                        if best.is_none_or(|(bs, _, _)| score < bs) {
                            best = Some((score, (fx + dx) as u16, (fy + dy) as u16));
                        }
                    }
                }
            }
        }

        if best.is_none() {
            let [ax, ay] = node_anchors.get(nid).copied().unwrap_or([ox, oy]);
            let gx0 = (ax.floor() as isize).clamp(0, GRID_W as isize - 1);
            let gy0 = (ay.floor() as isize).clamp(0, GRID_H as isize - 1);
            for r in 0..=24isize {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs().max(dy.abs()) != r {
                            continue;
                        }
                        let Some(npos) = idx_of(gx0 + dx, gy0 + dy) else {
                            continue;
                        };
                        if !is_static_walkable(cell_owner, terrain, road, bldg, npos) {
                            continue;
                        }
                        if !has_open_orth_walkable(
                            cell_owner,
                            terrain,
                            road,
                            bldg,
                            gx0 + dx,
                            gy0 + dy,
                        ) {
                            continue;
                        }
                        let nx = (gx0 + dx) as f32 + 0.5;
                        let ny = (gy0 + dy) as f32 + 0.5;
                        let dist = (nx - ox).powi(2) + (ny - oy).powi(2);
                        let road_penalty = if road[npos] != ROAD_NONE { 0.0 } else { 6.0 };
                        let score = (dx * dx + dy * dy) as f32 + dist * 0.25 + road_penalty;
                        if best.is_none_or(|(bs, _, _)| score < bs) {
                            best = Some((score, (gx0 + dx) as u16, (gy0 + dy) as u16));
                        }
                    }
                }
                if best.is_some() {
                    break;
                }
            }
        }

        if let Some((_, dx, dy)) = best {
            doors.insert(nid.clone(), [dx, dy]);
        }
    }
    doors
}

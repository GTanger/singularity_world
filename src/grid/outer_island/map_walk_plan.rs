//! 全島 `860×930` 地格步行可通性判斷、無雜湊初次播種與 8 向防斜切 A* 尋路。

use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::rc::Rc;

use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    BLDG_NONE, CompiledIslandSubstrate, GRID_H, GRID_W, ROAD_BRIDGE_OR_PIER, ROAD_COUNTRY_TRACK,
    ROAD_NONE, ROAD_STONE_WALL, ROAD_TOWN_COBBLE, TERRAIN_MOUNTAIN, TERRAIN_REEF_SAND,
    TERRAIN_RIVER, TERRAIN_SEA, TERRAIN_TREE, compiled_island_substrate, idx_of, world_to_grid,
};
use super::live_land::is_bridge_washed_out;
use super::live_land_tiles::effective_tile_state;
use super::live_tide::derive_tide_clock_details;
use super::map_memory::ActorMapMemory;
use super::mesh_expedition::is_high_tide_slot;
use super::types::{IslandWorldDelta, WorldStaticDoc};

type PathCacheKey = (u32, u32, bool, u128);

struct WalkScratch {
    stamp: Vec<u32>,
    closed: Vec<u32>,
    g_score: Vec<u32>,
    parent: Vec<u32>,
    epoch: u32,
    seed_hash: String,
    path_cache: HashMap<PathCacheKey, Option<Rc<[[u16; 2]]>>>,
}

impl WalkScratch {
    fn new(total: usize) -> Self {
        Self {
            stamp: vec![0; total],
            closed: vec![0; total],
            g_score: vec![u32::MAX; total],
            parent: vec![u32::MAX; total],
            epoch: 0,
            seed_hash: String::new(),
            path_cache: HashMap::new(),
        }
    }

    fn begin(&mut self) -> u32 {
        self.epoch = self.epoch.saturating_add(1);
        if self.epoch == 0 {
            self.stamp.fill(0);
            self.closed.fill(0);
            self.epoch = 1;
        }
        self.epoch
    }
}

thread_local! {
    static WALK_SCRATCH: RefCell<WalkScratch> = RefCell::new(WalkScratch::new(GRID_W * GRID_H));
}

#[inline]
pub fn cell_id_at_tile(sub: &CompiledIslandSubstrate, gx: isize, gy: isize) -> Option<u32> {
    let pos = idx_of(gx, gy)?;
    let owner = sub.cell_owner[pos];
    if owner == u16::MAX {
        None
    } else {
        Some(sub.cell_ids[owner as usize])
    }
}

/// 判斷格點 `(gx, gy)` 當下是否可步行（支援遠端未見斷橋之樂觀規劃切換）。
#[expect(
    clippy::too_many_arguments,
    reason = "explicit tile walkability context"
)]
pub fn is_tile_walkable(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    gx: isize,
    gy: isize,
    allow_unwitnessed_washed_bridge: bool,
    mem_opt: Option<&ActorMapMemory>,
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    let (t, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
    if bg != BLDG_NONE || rd == ROAD_STONE_WALL || t == TERRAIN_MOUNTAIN {
        return false;
    }
    if t == TERRAIN_SEA {
        return rd == ROAD_BRIDGE_OR_PIER;
    }
    if t == TERRAIN_RIVER {
        if rd != ROAD_BRIDGE_OR_PIER {
            return false;
        }
        let owner = sub.cell_owner[pos];
        if owner != u16::MAX {
            let cid = sub.cell_ids[owner as usize];
            if is_bridge_washed_out(delta, cid) {
                if !allow_unwitnessed_washed_bridge {
                    return false;
                }
                if mem_opt
                    .and_then(|m| m.witnessed_cells.get(&cid))
                    .is_some_and(|s| s.bridge_down)
                {
                    return false;
                }
            }
        }
        return true;
    }
    if t == TERRAIN_REEF_SAND && rd == ROAD_NONE {
        let td = derive_tide_clock_details(clock);
        if is_high_tide_slot(td.tide_slot) {
            return false;
        }
    }
    if rd == ROAD_NONE && sub.cell_owner[pos] == u16::MAX {
        return false;
    }
    let _ = doc;
    true
}

fn is_static_walkable_tile(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    pos: usize,
) -> bool {
    let (t, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
    if bg != BLDG_NONE || rd == ROAD_STONE_WALL || t == TERRAIN_MOUNTAIN {
        return false;
    }
    if t == TERRAIN_SEA {
        return rd == ROAD_BRIDGE_OR_PIER;
    }
    if t == TERRAIN_RIVER {
        return rd == ROAD_BRIDGE_OR_PIER;
    }
    rd != ROAD_NONE || sub.cell_owner[pos] != u16::MAX
}

fn find_unoccupied_walkable_near(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    bx: isize,
    by: isize,
    occupied: &HashSet<usize>,
    max_r: isize,
) -> Option<(isize, isize, usize)> {
    let mut fallback: Option<(isize, isize, usize)> = None;
    for r in 0..=max_r {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let nx = bx + dx;
                let ny = by + dy;
                let Some(pos) = idx_of(nx, ny) else {
                    continue;
                };
                if !is_static_walkable_tile(sub, delta, pos) {
                    continue;
                }
                if !occupied.contains(&pos) {
                    return Some((nx, ny, pos));
                }
                if fallback.is_none() {
                    fallback = Some((nx, ny, pos));
                }
            }
        }
    }
    fallback
}

/// 以 $O(1)$ 為單一角色播種格網座標（僅查目標同格／門前節點既有佔用者，避免掃描全服千人）。
pub fn ensure_single_actor_grid_seeded(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    aid: &str,
) {
    let Some(nid) = delta
        .actors
        .get(aid)
        .filter(|a| !a.map_pos_seeded)
        .map(|a| a.node_id.clone())
    else {
        return;
    };
    let is_door_bound = doc.nodes.get(&nid).is_some_and(|n| n.indoor || n.dungeon);
    let base = if is_door_bound {
        sub.door_tiles
            .get(&nid)
            .map(|&[dx, dy]| (dx as isize, dy as isize))
    } else {
        sub.node_anchors
            .get(&nid)
            .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
    }
    .unwrap_or_else(|| {
        doc.nodes
            .get(&nid)
            .map_or((430, 465), |nd| world_to_grid(nd.pos[0], nd.pos[1]))
    });
    let mut occupied = HashSet::new();
    if !is_door_bound && let Some(occ_ids) = delta.node_occupants.get(&nid) {
        for oid in occ_ids {
            if oid != aid
                && let Some(o) = delta.actors.get(oid)
                && o.map_pos_seeded
                && let Some(pos) = idx_of(o.map_gx.floor() as isize, o.map_gy.floor() as isize)
            {
                occupied.insert(pos);
            }
        }
    }
    let picked = find_unoccupied_walkable_near(sub, delta, base.0, base.1, &occupied, 10)
        .unwrap_or((base.0, base.1, idx_of(base.0, base.1).unwrap_or(0)));
    if let Some(a) = delta.actors.get_mut(aid) {
        a.map_gx = picked.0 as f32 + 0.5;
        a.map_gy = picked.1 as f32 + 0.5;
        a.map_pos_seeded = true;
        a.dirty = true;
    }
}

/// 一次性為未播種格網座標的角色配發真實可步鄰近格（無雜湊散射，保證物理站位不重疊）。
pub fn seed_missing_actor_grid_positions_once(doc: &WorldStaticDoc, delta: &mut IslandWorldDelta) {
    if delta.actors.values().all(|a| a.map_pos_seeded) {
        return;
    }
    let sub = compiled_island_substrate(doc);
    let mut occupied: HashSet<usize> = HashSet::with_capacity(delta.actors.len());
    for a in delta.actors.values() {
        if a.map_pos_seeded
            && doc
                .nodes
                .get(&a.node_id)
                .is_none_or(|n| !n.indoor && !n.dungeon)
            && let Some(pos) = idx_of(a.map_gx.floor() as isize, a.map_gy.floor() as isize)
        {
            occupied.insert(pos);
        }
    }
    let pending_ids: Vec<String> = delta
        .actors
        .iter()
        .filter(|(_, a)| !a.map_pos_seeded)
        .map(|(id, _)| id.clone())
        .collect();
    for aid in pending_ids {
        let Some(nid) = delta.actors.get(&aid).map(|a| a.node_id.clone()) else {
            continue;
        };
        let is_door_bound = doc.nodes.get(&nid).is_some_and(|n| n.indoor || n.dungeon);
        let base = if is_door_bound {
            sub.door_tiles
                .get(&nid)
                .map(|&[dx, dy]| (dx as isize, dy as isize))
        } else {
            sub.node_anchors
                .get(&nid)
                .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
        }
        .unwrap_or_else(|| {
            doc.nodes
                .get(&nid)
                .map_or((430, 465), |nd| world_to_grid(nd.pos[0], nd.pos[1]))
        });
        let picked = find_unoccupied_walkable_near(sub, delta, base.0, base.1, &occupied, 10)
            .unwrap_or((base.0, base.1, idx_of(base.0, base.1).unwrap_or(0)));
        if !is_door_bound {
            occupied.insert(picked.2);
        }
        if let Some(a) = delta.actors.get_mut(&aid) {
            a.map_gx = picked.0 as f32 + 0.5;
            a.map_gy = picked.1 as f32 + 0.5;
            a.map_pos_seeded = true;
            a.dirty = true;
        }
    }
}

#[expect(clippy::too_many_arguments, reason = "nearest walkable helper")]
fn find_nearest_walkable_tile(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    gx: isize,
    gy: isize,
    max_r: isize,
    allow_unwitnessed_washed_bridge: bool,
    mem_opt: Option<&ActorMapMemory>,
) -> Option<(isize, isize)> {
    for r in 0..=max_r {
        let mut best: Option<(isize, isize, isize)> = None;
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let nx = gx + dx;
                let ny = gy + dy;
                if is_tile_walkable(
                    doc,
                    sub,
                    delta,
                    clock,
                    nx,
                    ny,
                    allow_unwitnessed_washed_bridge,
                    mem_opt,
                ) {
                    let d2 = dx * dx + dy * dy;
                    if best.is_none_or(|(bd, _, _)| d2 < bd) {
                        best = Some((d2, nx, ny));
                    }
                }
            }
        }
        if let Some((_, nx, ny)) = best {
            return Some((nx, ny));
        }
    }
    None
}

/// 玩家與 NPC 共用之 8 向防斜切加權 A* 地格尋路（支援終點避讓已佔用格）。
#[expect(
    clippy::too_many_arguments,
    reason = "explicit 8-dir weighted grid pathfinder"
)]
pub fn plan_grid_walk_path(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    mem_opt: Option<&ActorMapMemory>,
    actor_id: &str,
    start: (isize, isize),
    goal: (isize, isize),
) -> Option<Vec<[u16; 2]>> {
    plan_grid_walk_path_ext(
        doc, sub, delta, clock, mem_opt, actor_id, start, goal, true, 90_000,
    )
}

/// 單跳尋路與門檻直抵尋路（支持可控展開上限與門檻防推離）。
#[expect(
    clippy::too_many_arguments,
    reason = "explicit 8-dir bounded doorway/hop pathfinder"
)]
pub fn plan_grid_walk_hop(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    mem_opt: Option<&ActorMapMemory>,
    actor_id: &str,
    start: (isize, isize),
    goal: (isize, isize),
    dodge_occupants: bool,
) -> Option<Vec<[u16; 2]>> {
    plan_grid_walk_path_ext(
        doc,
        sub,
        delta,
        clock,
        mem_opt,
        actor_id,
        start,
        goal,
        dodge_occupants,
        90_000,
    )
}

#[expect(
    clippy::too_many_arguments,
    reason = "explicit 8-dir weighted grid pathfinder impl"
)]
fn plan_grid_walk_path_ext(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    mem_opt: Option<&ActorMapMemory>,
    actor_id: &str,
    start: (isize, isize),
    goal: (isize, isize),
    dodge_occupants: bool,
    max_cap: usize,
) -> Option<Vec<[u16; 2]>> {
    let start_ok = if is_tile_walkable(doc, sub, delta, clock, start.0, start.1, true, mem_opt) {
        start
    } else {
        find_nearest_walkable_tile(doc, sub, delta, clock, start.0, start.1, 8, true, mem_opt)?
    };
    let goal_cand = if is_tile_walkable(doc, sub, delta, clock, goal.0, goal.1, true, mem_opt) {
        goal
    } else {
        find_nearest_walkable_tile(doc, sub, delta, clock, goal.0, goal.1, 8, true, mem_opt)?
    };
    let goal_ok = if dodge_occupants {
        let mut occupied = HashSet::new();
        if let Some(goal_pos) = idx_of(goal_cand.0, goal_cand.1) {
            let owner = sub.cell_owner[goal_pos];
            if owner != u16::MAX
                && let Some(nodes) = sub.outdoor_nodes_by_cell.get(owner as usize)
            {
                for (nid, _) in nodes {
                    if let Some(occ_ids) = delta.node_occupants.get(nid) {
                        for oid in occ_ids {
                            if oid == actor_id {
                                continue;
                            }
                            if let Some(o) = delta.actors.get(oid)
                                && o.alive
                                && o.setup_done
                                && let Some(p) =
                                    idx_of(o.map_gx.floor() as isize, o.map_gy.floor() as isize)
                            {
                                occupied.insert(p);
                            }
                        }
                    }
                }
            }
        }
        if idx_of(goal_cand.0, goal_cand.1).is_some_and(|p| occupied.contains(&p)) {
            find_unoccupied_walkable_near(sub, delta, goal_cand.0, goal_cand.1, &occupied, 3)
                .map(|(x, y, _)| (x, y))
                .unwrap_or(goal_cand)
        } else {
            goal_cand
        }
    } else {
        goal_cand
    };
    if start_ok == goal_ok {
        return Some(Vec::new());
    }
    let start_pos = idx_of(start_ok.0, start_ok.1)?;
    let goal_pos = idx_of(goal_ok.0, goal_ok.1)?;
    let high_tide = is_high_tide_slot(derive_tide_clock_details(clock).tide_slot);
    let has_diffs = !delta.tile_diffs.is_empty();
    let washed_by_owner: Vec<bool> = sub
        .cell_ids
        .iter()
        .map(|&cid| {
            is_bridge_washed_out(delta, cid)
                && mem_opt
                    .and_then(|m| m.witnessed_cells.get(&cid))
                    .is_some_and(|s| s.bridge_down)
        })
        .collect();
    let cacheable = sub.cell_ids.len() <= 128;
    let mut washed_mask = 0u128;
    if cacheable {
        for (i, &w) in washed_by_owner.iter().enumerate() {
            if w {
                washed_mask |= 1u128 << i;
            }
        }
    }
    let cache_key: PathCacheKey = (start_pos as u32, goal_pos as u32, high_tide, washed_mask);
    let fast_walk = |nx: isize, ny: isize| -> Option<(usize, u8, u8)> {
        let pos = idx_of(nx, ny)?;
        let (t, rd, bg) = if has_diffs {
            effective_tile_state(sub, &delta.tile_diffs, pos)
        } else {
            (
                sub.base_terrain[pos],
                sub.base_road[pos],
                sub.base_bldg[pos],
            )
        };
        if bg != BLDG_NONE || rd == ROAD_STONE_WALL || t == TERRAIN_MOUNTAIN {
            return None;
        }
        if t == TERRAIN_SEA {
            return (rd == ROAD_BRIDGE_OR_PIER).then_some((pos, t, rd));
        }
        let owner = sub.cell_owner[pos];
        if t == TERRAIN_RIVER {
            if rd != ROAD_BRIDGE_OR_PIER {
                return None;
            }
            if owner != u16::MAX && washed_by_owner[owner as usize] {
                return None;
            }
            return Some((pos, t, rd));
        }
        if high_tide && t == TERRAIN_REEF_SAND && rd == ROAD_NONE {
            return None;
        }
        if rd == ROAD_NONE && owner == u16::MAX {
            return None;
        }
        Some((pos, t, rd))
    };

    WALK_SCRATCH.with(|cell| {
        let mut s = cell.borrow_mut();
        if s.seed_hash != sub.seed_hash {
            s.seed_hash = sub.seed_hash.clone();
            s.path_cache.clear();
        }
        if cacheable && let Some(hit) = s.path_cache.get(&cache_key) {
            return hit.as_ref().map(|rc| rc.to_vec());
        }
        let epoch = s.begin();
        s.stamp[start_pos] = epoch;
        s.g_score[start_pos] = 0;
        s.parent[start_pos] = u32::MAX;

        let mut heap: BinaryHeap<Reverse<(u32, usize)>> = BinaryHeap::new();
        let adx = (goal_ok.0 - start_ok.0).unsigned_abs() as u32;
        let ady = (goal_ok.1 - start_ok.1).unsigned_abs() as u32;
        let h0 = adx.min(ady) * 14 + adx.abs_diff(ady) * 10;
        heap.push(Reverse((h0, start_pos)));

        const DIRS: [(isize, isize, bool); 8] = [
            (1, 0, false),
            (-1, 0, false),
            (0, 1, false),
            (0, -1, false),
            (1, 1, true),
            (1, -1, true),
            (-1, 1, true),
            (-1, -1, true),
        ];

        let expand_cap = max_cap;
        let mut expanded = 0usize;
        let mut found = false;
        while let Some(Reverse((_, cur))) = heap.pop() {
            if s.closed[cur] == epoch {
                continue;
            }
            s.closed[cur] = epoch;
            if cur == goal_pos {
                found = true;
                break;
            }
            expanded += 1;
            if expanded > expand_cap {
                break;
            }
            let cx = (cur % GRID_W) as isize;
            let cy = (cur / GRID_W) as isize;
            let cur_g = s.g_score[cur];

            for (dx, dy, diag) in DIRS {
                let nx = cx + dx;
                let ny = cy + dy;
                let Some((npos, t, rd)) = fast_walk(nx, ny) else {
                    continue;
                };
                if diag && (fast_walk(cx + dx, cy).is_none() || fast_walk(cx, cy + dy).is_none()) {
                    continue;
                }
                if s.closed[npos] == epoch {
                    continue;
                }
                let factor: u32 = if matches!(rd, ROAD_TOWN_COBBLE | ROAD_BRIDGE_OR_PIER) {
                    10
                } else if rd == ROAD_COUNTRY_TRACK {
                    12
                } else if t == TERRAIN_TREE {
                    26
                } else {
                    18
                };
                let step_cost = if diag { (factor * 14) / 10 } else { factor };
                let ng = cur_g.saturating_add(step_cost);
                if s.stamp[npos] != epoch || ng < s.g_score[npos] {
                    s.stamp[npos] = epoch;
                    s.g_score[npos] = ng;
                    s.parent[npos] = cur as u32;
                    let hx = (goal_ok.0 - nx).unsigned_abs() as u32;
                    let hy = (goal_ok.1 - ny).unsigned_abs() as u32;
                    let h = ((hx.min(hy) * 14 + hx.abs_diff(hy) * 10) * 13) / 10;
                    heap.push(Reverse((ng.saturating_add(h), npos)));
                }
            }
        }

        if !found {
            if cacheable {
                if s.path_cache.len() >= 16_384 {
                    s.path_cache.clear();
                }
                s.path_cache.insert(cache_key, None);
            }
            return None;
        }
        let mut rev = Vec::new();
        let mut cur = goal_pos;
        while cur != start_pos {
            rev.push([(cur % GRID_W) as u16, (cur / GRID_W) as u16]);
            let p = s.parent[cur];
            if p == u32::MAX {
                return None;
            }
            cur = p as usize;
        }
        rev.reverse();
        if cacheable {
            if s.path_cache.len() >= 16_384 {
                s.path_cache.clear();
            }
            s.path_cache
                .insert(cache_key, Some(Rc::from(rev.as_slice())));
        }
        Some(rev)
    })
}

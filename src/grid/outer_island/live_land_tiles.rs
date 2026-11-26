//! 地格層動態覆寫輔助：A* 過河橋樑綁定、實體伐木去樹格、就近水土開墾農田與休耕／再生消退。

use std::collections::{BTreeMap, BTreeSet};

use super::island_substrate::{
    BLDG_NONE, CompiledIslandSubstrate, GRID_H, GRID_W, ROAD_BRIDGE_OR_PIER, ROAD_NONE,
    TERRAIN_FARM, TERRAIN_LAND, TERRAIN_RIVER, TERRAIN_TREE, idx_of,
};
use super::types::{IslandWorldDelta, TileDiffEntry, WorldStaticDoc};

/// 讀取特定格點疊加 `delta.tile_diffs` 後之實際 `(terrain, road, bldg)`。
#[inline]
pub fn effective_tile_state(
    sub: &CompiledIslandSubstrate,
    tile_diffs: &BTreeMap<u32, TileDiffEntry>,
    pos: usize,
) -> (u8, u8, u8) {
    if let Some(d) = tile_diffs.get(&(pos as u32)) {
        (d.terrain, d.road, d.bldg)
    } else {
        (
            sub.base_terrain[pos],
            sub.base_road[pos],
            sub.base_bldg[pos],
        )
    }
}

/// 掃描編譯底盤上非聚落區域具 `TERRAIN_RIVER + ROAD_BRIDGE_OR_PIER` 之格，
/// 確保每座實際過河木橋在 `live_land.cells` 皆標記 `has_river_crossing = true`。
pub fn sync_river_crossings_with_substrate(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
) {
    let mut crossed_owners = BTreeSet::new();
    for (pos, (&t, &r)) in sub
        .base_terrain
        .iter()
        .zip(sub.base_road.iter())
        .enumerate()
    {
        if t == TERRAIN_RIVER && r == ROAD_BRIDGE_OR_PIER {
            let owner = sub.cell_owner[pos];
            if owner != u16::MAX {
                crossed_owners.insert(owner);
            }
        }
    }
    let mut mutated = false;
    for o in crossed_owners {
        let cid = sub.cell_ids[o as usize];
        let is_non_settlement = doc
            .cells
            .get(&cid.to_string())
            .is_some_and(|c| c.kind != "settlement" && c.kind != "ruin_dungeon");
        if !is_non_settlement {
            continue;
        }
        if let Some(cl) = delta.live_land.cells.get_mut(&cid)
            && !cl.has_river_crossing
        {
            cl.has_river_crossing = true;
            if cl.bridge_integrity == 0 && cl.bridge_down_days == 0 {
                cl.bridge_integrity = (88 + (cid % 13)) as u8;
            }
            mutated = true;
        }
    }
    if mutated {
        delta.extras_dirty = true;
    }
}

/// 僅依角色真實站立座標 `(map_gx, map_gy)` 回傳整數格點（禁止退回節點錨點代勞）。
pub fn resolve_actor_standing_xy(delta: &IslandWorldDelta, aid: &str) -> Option<(isize, isize)> {
    let a = delta.actors.get(aid).filter(|a| a.map_pos_seeded)?;
    Some((
        (a.map_gx.floor() as isize).clamp(0, GRID_W as isize - 1),
        (a.map_gy.floor() as isize).clamp(0, GRID_H as isize - 1),
    ))
}

fn is_forest_edge_tile(
    sub: &CompiledIslandSubstrate,
    tile_diffs: &BTreeMap<u32, TileDiffEntry>,
    gx: isize,
    gy: isize,
) -> bool {
    for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let Some(npos) = idx_of(gx + dx, gy + dy) else {
            return true;
        };
        let (nt, nrd, _) = effective_tile_state(sub, tile_diffs, npos);
        if nt != TERRAIN_TREE || nrd != ROAD_NONE {
            return true;
        }
    }
    false
}

/// 在角色實地 `(gx, gy)` 周遭 `14` 格工作半徑內，優先挑林緣 `TERRAIN_TREE` 真實伐除為 `TERRAIN_LAND`（最多 `2` 格）。
pub fn carve_felled_tree_tile_at_actor(
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    gx: isize,
    gy: isize,
    game_day: i64,
) -> usize {
    let mut edge_picks: Vec<usize> = Vec::new();
    let mut interior_picks: Vec<usize> = Vec::new();
    for r in 0..=14isize {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let nx = gx + dx;
                let ny = gy + dy;
                let Some(pos) = idx_of(nx, ny) else {
                    continue;
                };
                if sub.cell_owner[pos] == u16::MAX {
                    continue;
                }
                let (t, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
                if t == TERRAIN_TREE && rd == ROAD_NONE && bg == BLDG_NONE {
                    if is_forest_edge_tile(sub, &delta.tile_diffs, nx, ny) {
                        edge_picks.push(pos);
                        if edge_picks.len() >= 2 {
                            break;
                        }
                    } else if interior_picks.len() < 2 {
                        interior_picks.push(pos);
                    }
                }
            }
            if edge_picks.len() >= 2 {
                break;
            }
        }
        if edge_picks.len() >= 2 {
            break;
        }
    }
    let mut picked = edge_picks;
    if picked.len() < 2 {
        for pos in interior_picks {
            if !picked.contains(&pos) {
                picked.push(pos);
                if picked.len() >= 2 {
                    break;
                }
            }
        }
    }
    let count = picked.len();
    for pos in picked {
        let (_, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
        delta.tile_diffs.insert(
            pos as u32,
            TileDiffEntry {
                terrain: TERRAIN_LAND,
                road: rd,
                bldg: bg,
                changed_day: game_day,
            },
        );
    }
    if count > 0 {
        delta.extras_dirty = true;
    }
    count
}

fn has_neighbor_terrain(
    sub: &CompiledIslandSubstrate,
    tile_diffs: &BTreeMap<u32, TileDiffEntry>,
    gx: isize,
    gy: isize,
    target: u8,
) -> bool {
    for dy in -1..=1isize {
        for dx in -1..=1isize {
            if dx == 0 && dy == 0 {
                continue;
            }
            if let Some(npos) = idx_of(gx + dx, gy + dy) {
                let (nt, _, _) = effective_tile_state(sub, tile_diffs, npos);
                if nt == target {
                    return true;
                }
            }
        }
    }
    false
}

/// 在角色實地 `(gx, gy)` 周遭 `14` 格工作半徑內開墾最多 `2` 格可耕地 `TERRAIN_LAND`（優先貼近河流與既有農田，半徑內至多累積 `16` 格）。
pub fn reclaim_arable_tiles_at_actor(
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    gx: isize,
    gy: isize,
    game_day: i64,
) -> usize {
    let local_reclaimed = delta
        .tile_diffs
        .iter()
        .filter(|(pos_u32, e)| {
            let pos = **pos_u32 as usize;
            let px = (pos % GRID_W) as isize;
            let py = (pos / GRID_W) as isize;
            e.terrain == TERRAIN_FARM && (px - gx).abs().max((py - gy).abs()) <= 14
        })
        .count();
    if local_reclaimed >= 16 {
        return 0;
    }
    let mut cands: Vec<(i32, usize)> = Vec::new();
    for r in 1..=14isize {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let nx = gx + dx;
                let ny = gy + dy;
                let Some(pos) = idx_of(nx, ny) else {
                    continue;
                };
                if sub.cell_owner[pos] == u16::MAX || sub.sea_dist_by_cell_edge[pos] < 2 {
                    continue;
                }
                let (t, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
                if t != TERRAIN_LAND || rd != ROAD_NONE || bg != BLDG_NONE {
                    continue;
                }
                let near_river =
                    has_neighbor_terrain(sub, &delta.tile_diffs, nx, ny, TERRAIN_RIVER);
                let near_farm = has_neighbor_terrain(sub, &delta.tile_diffs, nx, ny, TERRAIN_FARM);
                let mut score = (dx * dx + dy * dy) as i32 * 4;
                if near_farm {
                    score -= 36;
                }
                if near_river {
                    score -= 24;
                }
                cands.push((score, pos));
            }
        }
        if cands.len() >= 16 {
            break;
        }
    }
    cands.sort_unstable();
    let picked: Vec<usize> = cands.into_iter().take(2).map(|(_, p)| p).collect();
    let count = picked.len();
    for pos in picked {
        let (_, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
        delta.tile_diffs.insert(
            pos as u32,
            TileDiffEntry {
                terrain: TERRAIN_FARM,
                road: rd,
                bldg: bg,
                changed_day: game_day,
            },
        );
    }
    if count > 0 {
        delta.extras_dirty = true;
    }
    count
}

/// 每日隨自然森林再生或休耕退化清理對應地格的過期 `tile_diffs`。
pub(super) fn prune_tile_diffs_for_cell_day(
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    cid: u32,
    game_day: i64,
    restore_forest: bool,
    decay_farm: bool,
) {
    if (!restore_forest && !decay_farm) || delta.tile_diffs.is_empty() {
        return;
    }
    let Some(&cidx) = sub.cell_index_by_id.get(&cid) else {
        return;
    };
    let mut drop_forest: Option<u32> = None;
    let mut drop_farm: Vec<u32> = Vec::new();
    for (&pos_u32, entry) in &delta.tile_diffs {
        let pos = pos_u32 as usize;
        if pos >= sub.cell_owner.len() || sub.cell_owner[pos] != cidx {
            continue;
        }
        if restore_forest
            && drop_forest.is_none()
            && entry.terrain == TERRAIN_LAND
            && sub.base_terrain[pos] == TERRAIN_TREE
            && game_day.saturating_sub(entry.changed_day) >= 14
        {
            drop_forest = Some(pos_u32);
        }
        if decay_farm
            && drop_farm.len() < 2
            && entry.terrain == TERRAIN_FARM
            && sub.base_terrain[pos] != TERRAIN_FARM
            && game_day.saturating_sub(entry.changed_day) >= 18
        {
            drop_farm.push(pos_u32);
        }
        if (!restore_forest || drop_forest.is_some()) && (!decay_farm || drop_farm.len() >= 2) {
            break;
        }
    }
    if let Some(p) = drop_forest {
        delta.tile_diffs.remove(&p);
        delta.extras_dirty = true;
    }
    for p in drop_farm {
        delta.tile_diffs.remove(&p);
        delta.extras_dirty = true;
    }
}

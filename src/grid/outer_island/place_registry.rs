//! 只增不減之動態聚落建築清冊（`place_registry`）：路邊雙格融合屋基配地、滿額自動延伸 3..=5 格支巷，以及永不刪 ID 之廢墟標記。

use serde::{Deserialize, Serialize};

use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_RUIN_WALL, CompiledIslandSubstrate, GRID_W, ROAD_COUNTRY_TRACK,
    ROAD_NONE, ROAD_TOWN_COBBLE, TERRAIN_FARM, TERRAIN_LAND, TERRAIN_TREE,
    compiled_island_substrate, idx_of,
};
use super::live_land_tiles::effective_tile_state;
use super::types::{IslandWorldDelta, TileDiffEntry, WorldStaticDoc};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DynamicPlaceRecord {
    pub place_id: String,
    pub owner_actor_id: String,
    pub anchor_node_id: String,
    pub cell_id: u32,
    pub label_zh: String,
    pub door_tile: [u16; 2],
    pub footprint_tiles: Vec<[u16; 2]>,
    #[serde(default)]
    pub extended_road_tiles: Vec<[u16; 2]>,
    pub built_day: i64,
    #[serde(default)]
    pub ruined: bool,
}

/// 計算動態建築足跡之中心格網座標，供輿圖名牌精確落點。
pub fn footprint_center_xy(rec: &DynamicPlaceRecord) -> [f32; 2] {
    if rec.footprint_tiles.is_empty() {
        return [rec.door_tile[0] as f32 + 0.5, rec.door_tile[1] as f32 + 0.5];
    }
    let n = rec.footprint_tiles.len() as f32;
    let sx: f32 = rec.footprint_tiles.iter().map(|t| t[0] as f32 + 0.5).sum();
    let sy: f32 = rec.footprint_tiles.iter().map(|t| t[1] as f32 + 0.5).sum();
    [sx / n, sy / n]
}

fn is_walkable_town_or_country_road(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    cidx: u16,
    gx: isize,
    gy: isize,
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    if sub.cell_owner[pos] != cidx {
        return false;
    }
    let (t, r, b) = effective_tile_state(sub, &delta.tile_diffs, pos);
    t == TERRAIN_LAND && b == BLDG_NONE && matches!(r, ROAD_COUNTRY_TRACK | ROAD_TOWN_COBBLE)
}

fn is_open_building_lot(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    cidx: u16,
    gx: isize,
    gy: isize,
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    if sub.cell_owner[pos] != cidx {
        return false;
    }
    let (t, r, b) = effective_tile_state(sub, &delta.tile_diffs, pos);
    matches!(t, TERRAIN_LAND | TERRAIN_FARM) && r == ROAD_NONE && b == BLDG_NONE
}

fn is_alley_clearable_tile(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    cidx: u16,
    gx: isize,
    gy: isize,
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    if sub.cell_owner[pos] != cidx {
        return false;
    }
    let (t, r, b) = effective_tile_state(sub, &delta.tile_diffs, pos);
    matches!(t, TERRAIN_LAND | TERRAIN_FARM | TERRAIN_TREE) && r == ROAD_NONE && b == BLDG_NONE
}

fn find_paired_lot_near_road(
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    cidx: u16,
    rx: isize,
    ry: isize,
) -> Option<([u16; 2], Vec<[u16; 2]>)> {
    const DIRS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    for (ux, uy) in DIRS {
        let hx1 = rx + ux;
        let hy1 = ry + uy;
        if !is_open_building_lot(sub, delta, cidx, hx1, hy1) {
            continue;
        }
        // 第二格優先沿與道路平行方向延伸形成緊貼道路的 2×1 連體屋頂，其次向深處 1×2 延伸
        let perp_a = (-uy, ux);
        let perp_b = (uy, -ux);
        for (sx, sy) in [perp_a, perp_b, (ux, uy)] {
            let hx2 = hx1 + sx;
            let hy2 = hy1 + sy;
            if is_open_building_lot(sub, delta, cidx, hx2, hy2) {
                return Some((
                    [rx as u16, ry as u16],
                    vec![[hx1 as u16, hy1 as u16], [hx2 as u16, hy2 as u16]],
                ));
            }
        }
    }
    None
}

/// 優先從起建者腳下與錨點周邊的道路旁尋找 2 格連棟空屋基；若路邊已滿，自動從路尾向空地延伸 `3..=5` 格支巷 (`ROAD_COUNTRY_TRACK`) 再立屋。
pub fn allocate_and_register_roadside_homestead(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    game_day: i64,
    owner_actor_id: &str,
    place_id: &str,
    anchor_node_id: &str,
    label_zh: &str,
) -> Option<DynamicPlaceRecord> {
    let cell_id = doc.nodes.get(anchor_node_id)?.cell_id;
    let sub = compiled_island_substrate(doc);
    let cidx = *sub.cell_index_by_id.get(&cell_id)?;

    let (origin_x, origin_y) = delta
        .actors
        .get(owner_actor_id)
        .filter(|a| a.map_pos_seeded)
        .and_then(|a| {
            let gx = a.map_gx.floor() as isize;
            let gy = a.map_gy.floor() as isize;
            idx_of(gx, gy)
                .filter(|&p| sub.cell_owner[p] == cidx)
                .map(|_| (gx, gy))
        })
        .or_else(|| {
            sub.node_anchors
                .get(anchor_node_id)
                .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
        })?;

    // 第一輪：掃描距離 `0..=4` 格內現有可行走道路兩旁，找 2 格緊鄰空地融合成雙格連脊屋頂；鄰街滿額即自然延伸新支巷
    let mut picked: Option<([u16; 2], Vec<[u16; 2]>, Vec<[u16; 2]>)> = None;
    'pass1: for r in 0..=4isize {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs().max(dy.abs()) != r {
                    continue;
                }
                let rx = origin_x + dx;
                let ry = origin_y + dy;
                if is_walkable_town_or_country_road(sub, delta, cidx, rx, ry)
                    && let Some((door, fp)) = find_paired_lot_near_road(sub, delta, cidx, rx, ry)
                {
                    picked = Some((door, fp, Vec::new()));
                    break 'pass1;
                }
            }
        }
    }

    // 第二輪：路旁已飽和時，從現有道路延伸 3..=5 格 `ROAD_COUNTRY_TRACK` 新支巷並在巷旁配 2 格屋基
    if picked.is_none() {
        const DIRS: [(isize, isize); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        'pass2: for r in 0..=32isize {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let rx = origin_x + dx;
                    let ry = origin_y + dy;
                    if !is_walkable_town_or_country_road(sub, delta, cidx, rx, ry) {
                        continue;
                    }
                    for (ux, uy) in DIRS {
                        for alley_len in [4isize, 3, 5] {
                            let mut alley: Vec<[u16; 2]> = Vec::new();
                            let mut clear = true;
                            for step in 1..=alley_len {
                                let tx = rx + ux * step;
                                let ty = ry + uy * step;
                                if !is_alley_clearable_tile(sub, delta, cidx, tx, ty) {
                                    clear = false;
                                    break;
                                }
                                alley.push([tx as u16, ty as u16]);
                            }
                            if !clear {
                                continue;
                            }
                            let tip_x = rx + ux * alley_len;
                            let tip_y = ry + uy * alley_len;
                            let alley_contains = |px: isize, py: isize| {
                                alley
                                    .iter()
                                    .any(|t| (t[0] as isize) == px && (t[1] as isize) == py)
                            };
                            let perp = [(-uy, ux), (uy, -ux)];
                            for (sx, sy) in perp {
                                let hx1 = tip_x + sx;
                                let hy1 = tip_y + sy;
                                if !alley_contains(hx1, hy1)
                                    && is_alley_clearable_tile(sub, delta, cidx, hx1, hy1)
                                {
                                    for (ox, oy) in [(-ux, -uy), (ux, uy), (sx, sy)] {
                                        let hx2 = hx1 + ox;
                                        let hy2 = hy1 + oy;
                                        if !alley_contains(hx2, hy2)
                                            && is_alley_clearable_tile(sub, delta, cidx, hx2, hy2)
                                        {
                                            picked = Some((
                                                [tip_x as u16, tip_y as u16],
                                                vec![
                                                    [hx1 as u16, hy1 as u16],
                                                    [hx2 as u16, hy2 as u16],
                                                ],
                                                alley,
                                            ));
                                            break 'pass2;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 保底單格屋基（極狹窄山谷仍保證能落地不退單）
    if picked.is_none() {
        'pass3: for r in 0..=32isize {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs().max(dy.abs()) != r {
                        continue;
                    }
                    let rx = origin_x + dx;
                    let ry = origin_y + dy;
                    if !is_walkable_town_or_country_road(sub, delta, cidx, rx, ry) {
                        continue;
                    }
                    for (ux, uy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        if is_open_building_lot(sub, delta, cidx, rx + ux, ry + uy) {
                            picked = Some((
                                [rx as u16, ry as u16],
                                vec![[(rx + ux) as u16, (ry + uy) as u16]],
                                Vec::new(),
                            ));
                            break 'pass3;
                        }
                    }
                }
            }
        }
    }

    let (door_tile, footprint_tiles, extended_road_tiles) = picked?;
    for &[rx, ry] in &extended_road_tiles {
        let pos = usize::from(ry) * GRID_W + usize::from(rx);
        delta.tile_diffs.insert(
            pos as u32,
            TileDiffEntry {
                terrain: TERRAIN_LAND,
                road: ROAD_COUNTRY_TRACK,
                bldg: BLDG_NONE,
                changed_day: game_day,
            },
        );
    }
    for &[hx, hy] in &footprint_tiles {
        let pos = usize::from(hy) * GRID_W + usize::from(hx);
        delta.tile_diffs.insert(
            pos as u32,
            TileDiffEntry {
                terrain: TERRAIN_LAND,
                road: ROAD_NONE,
                bldg: BLDG_ROOF,
                changed_day: game_day,
            },
        );
    }

    let rec = DynamicPlaceRecord {
        place_id: place_id.to_string(),
        owner_actor_id: owner_actor_id.to_string(),
        anchor_node_id: anchor_node_id.to_string(),
        cell_id,
        label_zh: label_zh.to_string(),
        door_tile,
        footprint_tiles,
        extended_road_tiles,
        built_day: game_day,
        ruined: false,
    };
    delta
        .place_registry
        .insert(place_id.to_string(), rec.clone());
    delta.extras_dirty = true;
    Some(rec)
}

/// 廢墟化動態宅第：**嚴守只增不減**，絕不移除 `place_registry` 條目與 `place_id`，僅設 `ruined = true` 並將瓦頂地格改為 `BLDG_RUIN_WALL`。
pub fn ruin_dynamic_homestead(delta: &mut IslandWorldDelta, place_id: &str, game_day: i64) -> bool {
    let Some(rec) = delta.place_registry.get_mut(place_id) else {
        return false;
    };
    if rec.ruined {
        return false;
    }
    rec.ruined = true;
    for &[hx, hy] in &rec.footprint_tiles {
        let pos = usize::from(hy) * GRID_W + usize::from(hx);
        delta.tile_diffs.insert(
            pos as u32,
            TileDiffEntry {
                terrain: TERRAIN_LAND,
                road: ROAD_NONE,
                bldg: BLDG_RUIN_WALL,
                changed_day: game_day,
            },
        );
    }
    delta.extras_dirty = true;
    true
}

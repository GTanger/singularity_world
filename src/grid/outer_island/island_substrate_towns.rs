//! 全島底盤之 34 座城鎮聚落（5 核心港鎮＋29 鄉野聚落）無縫街廓、城牆圓塔與百戶屋頂布局。

use std::collections::BTreeMap;

use serde_json::Value;

use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_TOWER, BLDG_VACANT_ROOF, ROAD_BRIDGE_OR_PIER, ROAD_NONE,
    ROAD_STONE_WALL, ROAD_TOWN_COBBLE, SubstratePoiSeed, TERRAIN_LAND, TERRAIN_SEA, idx_of,
    world_to_grid,
};
use super::types::WorldStaticDoc;

fn measure_cell_clearance(
    cidx: u16,
    cx: isize,
    cy: isize,
    cell_owner: &[u16],
    terrain: &[u8],
) -> (isize, isize, isize, isize) {
    let scan = |dx: isize, dy: isize, max_step: isize| -> isize {
        let mut last = 0;
        for step in 1..=max_step {
            let gx = cx + dx * step;
            let gy = cy + dy * step;
            let Some(pos) = idx_of(gx, gy) else {
                break;
            };
            if cell_owner[pos] != cidx || terrain[pos] == TERRAIN_SEA {
                break;
            }
            last = step;
        }
        last
    };
    (
        scan(-1, 0, 36),
        scan(1, 0, 36),
        scan(0, -1, 28),
        scan(0, 1, 28),
    )
}

/// 將全島 34 座聚落之街道、石牆、圓塔、碼頭棧橋與不粘連獨立民居寫入連續底盤。
#[expect(clippy::too_many_arguments, reason = "settlement grid synthesis stage")]
pub fn compile_settlements(
    doc: &WorldStaticDoc,
    cell_index_by_id: &BTreeMap<u32, u16>,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &mut [u8],
    node_anchors: &mut BTreeMap<String, [f32; 2]>,
    house_footprint_cells: &mut BTreeMap<String, Vec<usize>>,
    poi_base_entries: &mut Vec<SubstratePoiSeed>,
) {
    let mut entries: Vec<(&String, &Value)> = doc.town_micro_maps.iter().collect();
    entries.sort_by_key(|(slug, _)| slug.as_str());

    for (slug, tval) in entries {
        let Some(cid) = tval
            .get("cell_id")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
        else {
            continue;
        };
        let Some(&cidx) = cell_index_by_id.get(&cid) else {
            continue;
        };
        let Some(cell) = doc.cells.get(&cid.to_string()) else {
            continue;
        };
        let name_str = tval
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&cell.title);
        let is_core = tval.get("tier").is_none();
        let walled = tval
            .get("walled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let has_port = tval
            .get("has_port")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let coast_dir = tval
            .get("coast_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("none");

        let (cgx, cgy) = world_to_grid(cell.center[0], cell.center[1]);
        let (cl_l, cl_r, cl_u, cl_d) = measure_cell_clearance(cidx, cgx, cgy, cell_owner, terrain);

        let safe_l = (cl_l - 2).max(4);
        let safe_r = (cl_r - 2).max(4);
        let safe_u = (cl_u - 2).max(4);
        let safe_d = (cl_d - 2).max(4);

        poi_base_entries.push(SubstratePoiSeed {
            id: format!("town:{slug}"),
            cell_id: cid,
            label: name_str.to_string(),
            tier: if is_core { "core_town" } else { "rural_town" },
            gx: cgx as f32 + 0.5,
            gy: cgy as f32 + 0.5,
        });

        let spine = tval.get("spine");
        let nid_of = |key: &str| -> Option<String> {
            spine
                .and_then(|sp| sp.get(key))
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };

        if !is_core {
            super::island_substrate_rural::compile_bespoke_rural_settlement(
                doc,
                slug,
                cid,
                cidx,
                cgx,
                cgy,
                safe_l.min(7),
                safe_r.min(7),
                safe_u.min(7),
                safe_d.min(7),
                tval,
                cell_owner,
                terrain,
                road,
                bldg,
                node_anchors,
                house_footprint_cells,
            );
            continue;
        }

        let span_u = safe_u.min(18);
        let span_d = safe_d.min(18);
        let top_y = cgy - span_u + 2;
        let bot_y = cgy + span_d - 2;

        for gy in (top_y - 1)..=(bot_y + 1) {
            for gx in (cgx - safe_l + 1)..=(cgx + safe_r - 1) {
                if let Some(pos) = idx_of(gx, gy)
                    && cell_owner[pos] == cidx
                    && terrain[pos] != TERRAIN_SEA
                {
                    terrain[pos] = TERRAIN_LAND;
                }
            }
        }
        for dy in -1..=1 {
            for dx in -1..=1 {
                if let Some(pos) = idx_of(cgx + dx, cgy + dy)
                    && terrain[pos] != TERRAIN_SEA
                {
                    road[pos] = ROAD_TOWN_COBBLE;
                }
            }
        }
        if let Some(nid) = nid_of("plaza") {
            node_anchors.insert(nid, [cgx as f32 + 0.5, cgy as f32 + 0.5]);
        }

        for gy in top_y..=bot_y {
            if let Some(pos) = idx_of(cgx, gy)
                && terrain[pos] != TERRAIN_SEA
            {
                road[pos] = ROAD_TOWN_COBBLE;
            }
        }
        if let Some(nid) = nid_of("north_gate") {
            let lbl = if walled {
                "北城門"
            } else if slug == "north_cape_beacon" {
                "長明燈塔"
            } else {
                "北哨口"
            };
            node_anchors.insert(nid.clone(), [cgx as f32 + 0.5, top_y as f32 + 0.5]);
            poi_base_entries.push(SubstratePoiSeed {
                id: nid,
                cell_id: cid,
                label: lbl.to_string(),
                tier: "gate",
                gx: cgx as f32 + 0.5,
                gy: top_y as f32 + 0.5,
            });
        }
        if let Some(nid) = nid_of("south_gate") {
            node_anchors.insert(nid.clone(), [cgx as f32 + 0.5, bot_y as f32 + 0.5]);
            poi_base_entries.push(SubstratePoiSeed {
                id: nid,
                cell_id: cid,
                label: if walled { "南城門" } else { "南哨口" }.to_string(),
                tier: "gate",
                gx: cgx as f32 + 0.5,
                gy: bot_y as f32 + 0.5,
            });
        }
        if let Some(nid) = nid_of("smithy_st") {
            let sy = ((top_y + cgy) / 2).min(cgy - 2);
            node_anchors.insert(nid, [cgx as f32 + 0.5, sy as f32 + 0.5]);
        }
        if let Some(nid) = nid_of("tavern_st") {
            let ty = ((cgy + bot_y) / 2).max(cgy + 2);
            node_anchors.insert(nid, [cgx as f32 + 0.5, ty as f32 + 0.5]);
        }

        if has_port {
            let (p_dx, p_dy) = match coast_dir {
                "west" => (-1isize, 0isize),
                "south" => (0, 1),
                "north" => (0, -1),
                _ => (1, 0),
            };
            let mut dock_anchor = [
                cgx as f32 + p_dx as f32 * 4.0,
                cgy as f32 + p_dy as f32 * 4.0,
            ];
            let mut step = 1isize;
            let mut sea_steps = 0;
            while step <= 40 && sea_steps < 5 {
                let gx = cgx + p_dx * step;
                let gy = cgy + p_dy * step;
                let Some(pos) = idx_of(gx, gy) else {
                    break;
                };
                if terrain[pos] == TERRAIN_SEA {
                    sea_steps += 1;
                    road[pos] = ROAD_BRIDGE_OR_PIER;
                    if sea_steps == 3 {
                        dock_anchor = [gx as f32 + 0.5, gy as f32 + 0.5];
                    }
                } else {
                    road[pos] = ROAD_TOWN_COBBLE;
                }
                step += 1;
            }
            if let Some(nid) = nid_of("dock") {
                node_anchors.insert(nid.clone(), dock_anchor);
                poi_base_entries.push(SubstratePoiSeed {
                    id: nid,
                    cell_id: cid,
                    label: "港務碼頭".to_string(),
                    tier: "gate",
                    gx: dock_anchor[0],
                    gy: dock_anchor[1],
                });
            }
        }

        if slug == "north_cape_beacon" {
            for dy in -1..=0 {
                for dx in 1..=2 {
                    if let Some(pos) = idx_of(cgx + dx, top_y + dy)
                        && cell_owner[pos] == cidx
                        && terrain[pos] != TERRAIN_SEA
                    {
                        bldg[pos] = BLDG_TOWER;
                    }
                }
            }
        }

        place_town_landmarks_and_alleys(
            slug,
            cid,
            cidx,
            cgx,
            cgy,
            safe_l,
            safe_r,
            top_y,
            bot_y,
            walled,
            has_port,
            coast_dir,
            tval,
            cell_owner,
            terrain,
            road,
            bldg,
            node_anchors,
            house_footprint_cells,
            poi_base_entries,
        );
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "town landmarks, alleys, walls helper"
)]
fn place_town_landmarks_and_alleys(
    slug: &str,
    cid: u32,
    cidx: u16,
    cgx: isize,
    cgy: isize,
    safe_l: isize,
    safe_r: isize,
    top_y: isize,
    bot_y: isize,
    walled: bool,
    has_port: bool,
    coast_dir: &str,
    tval: &Value,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &mut [u8],
    node_anchors: &mut BTreeMap<String, [f32; 2]>,
    house_footprint_cells: &mut BTreeMap<String, Vec<usize>>,
    poi_base_entries: &mut Vec<SubstratePoiSeed>,
) {
    let lms = tval
        .get("landmarks")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let lm_offsets: [(isize, isize, u8); 4] = [(-3, -3, 1), (2, -3, 1), (-5, 2, 2), (2, 2, 1)];
    for (lidx, lm) in lms.iter().enumerate() {
        let Some((mut ox, oy, shape)) = lm_offsets.get(lidx).copied() else {
            continue;
        };
        let use_courtyard = shape == 2 && safe_l >= 8 && bot_y - cgy >= 6;
        if shape == 2 && !use_courtyard {
            ox = -4;
        }
        let lx = cgx + ox;
        let ly = cgy + oy;
        let nid = lm.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
        let lbl = lm.get("label").and_then(|v| v.as_str()).unwrap_or("");
        let courtyard_coords: [(isize, isize); 8] = [
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ];
        let coords: &[(isize, isize)] = if use_courtyard {
            &courtyard_coords
        } else if shape == 2 {
            &[(0, 0), (1, 0), (0, 1)]
        } else {
            &[(0, 0), (1, 0)]
        };
        for &(dx, dy) in coords {
            if let Some(pos) = idx_of(lx + dx, ly + dy)
                && cell_owner[pos] == cidx
                && terrain[pos] != TERRAIN_SEA
            {
                bldg[pos] = BLDG_ROOF;
                if !nid.is_empty() {
                    house_footprint_cells
                        .entry(nid.to_string())
                        .or_default()
                        .push(pos);
                    if lidx == 2 {
                        house_footprint_cells
                            .entry(format!("oi:{slug}:inn_upstairs"))
                            .or_default()
                            .push(pos);
                    }
                }
            }
        }
        if use_courtyard
            && let Some(pos) = idx_of(lx + 1, ly + 1)
            && cell_owner[pos] == cidx
            && terrain[pos] != TERRAIN_SEA
        {
            terrain[pos] = TERRAIN_LAND;
            bldg[pos] = BLDG_NONE;
            road[pos] = ROAD_NONE;
        }
        if !nid.is_empty() {
            let ax = lx as f32 + if use_courtyard { 1.5 } else { 0.8 };
            let ay_anchor = ly as f32 + 0.5;
            node_anchors.insert(nid.to_string(), [ax, ay_anchor]);
            if lidx == 2 {
                node_anchors.insert(format!("oi:{slug}:inn_upstairs"), [ax, ay_anchor]);
            }
            poi_base_entries.push(SubstratePoiSeed {
                id: nid.to_string(),
                cell_id: cid,
                label: lbl.to_string(),
                tier: "shop",
                gx: ax,
                gy: ay_anchor,
            });
        }
    }

    let alleys = tval
        .get("alleys")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let roofs = tval
        .get("roofs")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let row_cnt = alleys.len().div_ceil(2).max(1) as isize;
    let mut wall_bounds = (cgx - 6, cgx + 6, top_y, bot_y);
    let mut used_west_ays: Vec<isize> = Vec::new();
    let mut used_east_ays: Vec<isize> = Vec::new();

    for (aidx, al) in alleys.iter().enumerate() {
        let side_east = al.get("side").and_then(|v| v.as_str()) == Some("east");
        let r_row = (aidx / 2) as isize;
        let avail_top = top_y + 2;
        let avail_bot = bot_y - 2;
        let raw_y = if row_cnt <= 1 {
            cgy - 5
        } else {
            let t = r_row as f64 / (row_cnt - 1) as f64;
            (avail_top as f64 + t * (avail_bot - avail_top) as f64).round() as isize
        };
        let mut ay = if row_cnt <= 1 {
            cgy - 5
        } else if (raw_y - (cgy - 3)).abs() <= 1 {
            cgy - 6
        } else if raw_y >= cgy + 1 && raw_y <= cgy + 5 {
            (cgy + 6).min(avail_bot)
        } else if (raw_y - cgy).abs() <= 1 {
            cgy - 5
        } else {
            raw_y
        };
        let used_list = if side_east {
            &mut used_east_ays
        } else {
            &mut used_west_ays
        };
        if used_list.iter().any(|&py| (ay - py).abs() < 3) {
            let mut cands: Vec<isize> = (avail_top..=avail_bot)
                .filter(|&cand| {
                    !(cand >= cgy - 4 && cand <= cgy + 5)
                        && used_list.iter().all(|&py| (cand - py).abs() >= 3)
                })
                .collect();
            cands.sort_by_key(|&cand| (cand - raw_y).abs());
            if let Some(&best_ay) = cands.first() {
                ay = best_ay;
            }
        }
        used_list.push(ay);

        let my_roofs: Vec<&Value> = roofs
            .iter()
            .filter(|r| r.get("alley_idx").and_then(|v| v.as_u64()) == Some(aidx as u64))
            .collect();
        let half_houses = my_roofs.len().div_ceil(2).max(1) as isize;
        let dir_x: isize = if side_east { 1 } else { -1 };
        let max_x_span = if side_east { safe_r - 3 } else { safe_l - 3 };
        let alley_len = (half_houses * 2 + 2).min(max_x_span.max(6));
        let x_start = cgx + dir_x * 2;
        let x_end = cgx + dir_x * (1 + alley_len);

        if side_east {
            wall_bounds.1 = wall_bounds.1.max(x_end + 1);
        } else {
            wall_bounds.0 = wall_bounds.0.min(x_end - 1);
        }

        for step in 1..=alley_len {
            let gx = cgx + dir_x * step;
            if let Some(pos) = idx_of(gx, ay)
                && cell_owner[pos] == cidx
                && terrain[pos] != TERRAIN_SEA
            {
                road[pos] = ROAD_TOWN_COBBLE;
            }
        }
        if let Some(a_nid) = al.get("node_id").and_then(|v| v.as_str()) {
            let mx = (x_start + x_end) as f32 * 0.5 + 0.5;
            node_anchors.insert(a_nid.to_string(), [mx, ay as f32 + 0.5]);
            let al_name = al.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if !al_name.is_empty() {
                poi_base_entries.push(SubstratePoiSeed {
                    id: a_nid.to_string(),
                    cell_id: cid,
                    label: al_name.to_string(),
                    tier: "alley",
                    gx: mx,
                    gy: ay as f32 + 0.5,
                });
            }
        }

        for (h_ord, rf) in my_roofs.iter().enumerate() {
            let row_off: isize = if h_ord % 2 == 0 { -1 } else { 1 };
            let col_slot = (h_ord / 2) as isize;
            let hx = cgx + dir_x * (3 + col_slot * 2);
            let hy = ay + row_off;
            let vacant = rf.get("vacant").and_then(|v| v.as_bool()).unwrap_or(false);
            let roof_val = if vacant { BLDG_VACANT_ROOF } else { BLDG_ROOF };
            if let Some(pos) = idx_of(hx, hy)
                && cell_owner[pos] == cidx
                && terrain[pos] != TERRAIN_SEA
                && road[pos] == ROAD_NONE
                && bldg[pos] == BLDG_NONE
            {
                bldg[pos] = roof_val;
                if let Some(rnid) = rf.get("node_id").and_then(|v| v.as_str()) {
                    node_anchors.insert(rnid.to_string(), [hx as f32 + 0.5, hy as f32 + 0.5]);
                    house_footprint_cells
                        .entry(rnid.to_string())
                        .or_default()
                        .push(pos);
                }
            }
        }
    }

    if walled {
        let (wx0, wx1, wy0, wy1) = (
            wall_bounds.0.max(cgx - safe_l + 1),
            wall_bounds.1.min(cgx + safe_r - 1),
            wall_bounds.2.max(top_y),
            wall_bounds.3.min(bot_y),
        );
        let mut place_wall_tile = |gx: isize, gy: isize, is_tower: bool| {
            if (gx - cgx).abs() <= 1 && ((gy - wy0).abs() <= 1 || (gy - wy1).abs() <= 1) {
                return;
            }
            if has_port && coast_dir == "east" && gx == wx1 && (gy - cgy).abs() <= 1 {
                return;
            }
            if let Some(pos) = idx_of(gx, gy)
                && cell_owner[pos] == cidx
                && terrain[pos] != TERRAIN_SEA
                && bldg[pos] == BLDG_NONE
            {
                if is_tower {
                    bldg[pos] = BLDG_TOWER;
                } else if road[pos] == ROAD_NONE {
                    road[pos] = ROAD_STONE_WALL;
                }
            }
        };
        for gx in wx0..=wx1 {
            place_wall_tile(gx, wy0, gx == wx0 || gx == wx1);
            place_wall_tile(gx, wy1, gx == wx0 || gx == wx1);
        }
        for gy in wy0..=wy1 {
            place_wall_tile(wx0, gy, gy == wy0 || gy == wy1);
            place_wall_tile(wx1, gy, gy == wy0 || gy == wy1);
        }
        for &(tx, ty) in &[
            (cgx - 2, wy0),
            (cgx + 2, wy0),
            (cgx - 2, wy1),
            (cgx + 2, wy1),
            (wx1, cgy - 2),
            (wx1, cgy + 2),
        ] {
            place_wall_tile(tx, ty, true);
        }
    }
}

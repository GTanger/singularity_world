//! 29 處鄉村聚落地理專屬編譯器：依河口橋哨、高地石寨、半月漁港、林道Ｙ字炭寮與倖存梯田坡莊五大原型各自落格，禁止複製貼上一字長蛇陣。

use std::collections::BTreeMap;

use serde_json::Value;

use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_TOWER, BLDG_VACANT_ROOF, ROAD_BRIDGE_OR_PIER, ROAD_COUNTRY_TRACK,
    ROAD_NONE, ROAD_STONE_WALL, ROAD_TOWN_COBBLE, TERRAIN_FARM, TERRAIN_LAND, TERRAIN_RIVER,
    TERRAIN_SEA, idx_of,
};
use super::types::WorldStaticDoc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuralArchetype {
    RiverMarketBridge,
    HighlandBastion,
    CoastalFishingCove,
    ForestYForkCamp,
    SteadOrTerracedPasture,
}

fn classify_rural_archetype(
    slug: &str,
    tier: &str,
    is_coast: bool,
    flux: i32,
    height: i32,
    res_slug: &str,
) -> RuralArchetype {
    if matches!(tier, "market_town" | "quarantine_post")
        || flux >= 26
        || matches!(slug, "burg_4194" | "burg_4200" | "burg_4853" | "burg_4695")
    {
        return RuralArchetype::RiverMarketBridge;
    }
    if height >= 24
        || res_slug == "quarry_stone"
        || matches!(slug, "burg_4198" | "burg_4429" | "burg_4069")
    {
        return RuralArchetype::HighlandBastion;
    }
    if is_coast
        && (res_slug == "salt_fish_barrel"
            || matches!(
                slug,
                "burg_4196" | "burg_4204" | "burg_4206" | "burg_4433" | "burg_4083" | "burg_4926"
            ))
    {
        return RuralArchetype::CoastalFishingCove;
    }
    if matches!(res_slug, "charcoal_sack" | "black_coal" | "timber_log") {
        return RuralArchetype::ForestYForkCamp;
    }
    RuralArchetype::SteadOrTerracedPasture
}

#[expect(
    clippy::too_many_arguments,
    reason = "direct grid canvas mutation for rural settlement archetypes"
)]
fn paint_road_tile(
    cidx: u16,
    gx: isize,
    gy: isize,
    road_kind: u8,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &mut [u8],
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    if cell_owner[pos] != cidx {
        return false;
    }
    if terrain[pos] == TERRAIN_SEA && road_kind != ROAD_BRIDGE_OR_PIER {
        return false;
    }
    if !(terrain[pos] == TERRAIN_RIVER && road_kind == ROAD_BRIDGE_OR_PIER)
        && terrain[pos] != TERRAIN_SEA
    {
        terrain[pos] = TERRAIN_LAND;
    }
    road[pos] = road_kind;
    bldg[pos] = BLDG_NONE;
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "roof placement along bespoke rural geometry"
)]
fn place_roof_slot(
    cidx: u16,
    gx: isize,
    gy: isize,
    rf: &Value,
    allow_double_span: Option<(isize, isize)>,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &mut [u8],
    node_anchors: &mut BTreeMap<String, [f32; 2]>,
    house_footprint_cells: &mut BTreeMap<String, Vec<usize>>,
) -> bool {
    let Some(pos) = idx_of(gx, gy) else {
        return false;
    };
    if cell_owner[pos] != cidx
        || terrain[pos] == TERRAIN_SEA
        || road[pos] != ROAD_NONE
        || bldg[pos] != BLDG_NONE
    {
        return false;
    }
    let vacant = rf.get("vacant").and_then(|v| v.as_bool()).unwrap_or(false);
    let roof_val = if vacant { BLDG_VACANT_ROOF } else { BLDG_ROOF };
    terrain[pos] = TERRAIN_LAND;
    bldg[pos] = roof_val;
    if let Some(rnid) = rf.get("node_id").and_then(|v| v.as_str()) {
        node_anchors.insert(rnid.to_string(), [gx as f32 + 0.5, gy as f32 + 0.5]);
        let fp = house_footprint_cells.entry(rnid.to_string()).or_default();
        fp.push(pos);
        if let Some((ox, oy)) = allow_double_span
            && let Some(pos2) = idx_of(gx + ox, gy + oy)
            && cell_owner[pos2] == cidx
            && terrain[pos2] == TERRAIN_LAND
            && road[pos2] == ROAD_NONE
            && bldg[pos2] == BLDG_NONE
        {
            bldg[pos2] = roof_val;
            fp.push(pos2);
        }
    }
    true
}

#[expect(
    clippy::too_many_arguments,
    reason = "tower/wall accent placement preserving open exits"
)]
fn place_structure_accent(
    cidx: u16,
    gx: isize,
    gy: isize,
    is_tower: bool,
    cell_owner: &[u16],
    terrain: &[u8],
    road: &mut [u8],
    bldg: &mut [u8],
) {
    let Some(pos) = idx_of(gx, gy) else {
        return;
    };
    if cell_owner[pos] != cidx
        || terrain[pos] == TERRAIN_SEA
        || terrain[pos] == TERRAIN_RIVER
        || road[pos] != ROAD_NONE
        || bldg[pos] != BLDG_NONE
    {
        return;
    }
    if is_tower {
        bldg[pos] = BLDG_TOWER;
    } else {
        road[pos] = ROAD_STONE_WALL;
    }
}

/// 依 29 鄉村聚落自身地形與資源特性繪製專屬骨架、雙塔／棧橋／丫字林道／梯田牧坡並掛回精確節點錨點。
#[expect(
    clippy::too_many_arguments,
    reason = "bespoke compilation across 29 rural settlements"
)]
pub(super) fn compile_bespoke_rural_settlement(
    doc: &WorldStaticDoc,
    slug: &str,
    cid: u32,
    cidx: u16,
    cgx: isize,
    cgy: isize,
    safe_l: isize,
    safe_r: isize,
    safe_u: isize,
    safe_d: isize,
    tval: &Value,
    cell_owner: &[u16],
    terrain: &mut [u8],
    road: &mut [u8],
    bldg: &mut [u8],
    node_anchors: &mut BTreeMap<String, [f32; 2]>,
    house_footprint_cells: &mut BTreeMap<String, Vec<usize>>,
) {
    let roofs = tval
        .get("roofs")
        .and_then(|v| v.as_array())
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let tier = tval
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap_or("micro_hub");
    let coast_dir = tval
        .get("coast_dir")
        .and_then(|v| v.as_str())
        .unwrap_or("none");
    let cell_opt = doc.cells.get(&cid.to_string());
    let is_coast = cell_opt.is_some_and(|c| c.is_coast);
    let flux = cell_opt.map_or(10, |c| c.flux);
    let height = cell_opt.map_or(20, |c| c.height);
    let res_slug = cell_opt.map_or("", |c| c.resource_slug.as_str());

    let arch = classify_rural_archetype(slug, tier, is_coast, flux, height, res_slug);
    let sl = safe_l.clamp(3, 7);
    let sr = safe_r.clamp(3, 7);
    let su = safe_u.clamp(3, 7);
    let sd = safe_d.clamp(3, 7);

    // 核心廣場與院落：保證所有鄰格可達
    let core_road = if matches!(
        arch,
        RuralArchetype::RiverMarketBridge | RuralArchetype::HighlandBastion
    ) {
        ROAD_TOWN_COBBLE
    } else {
        ROAD_COUNTRY_TRACK
    };
    paint_road_tile(cidx, cgx, cgy, core_road, cell_owner, terrain, road, bldg);
    node_anchors.insert(
        format!("oi:{slug}:yard"),
        [cgx as f32 + 0.5, cgy as f32 + 0.5],
    );
    node_anchors.insert(
        format!("oi:{slug}:plaza"),
        [cgx as f32 + 0.5, cgy as f32 + 0.5],
    );

    let candidate_slots: Vec<(isize, isize, Option<(isize, isize)>)>;
    let gate_xy: (isize, isize);

    match arch {
        RuralArchetype::RiverMarketBridge => {
            // 雙向石砌集市十字軸＋橋道延伸＋哨塔雙門柱
            for dx in -(sl.min(4))..=(sr.min(4)) {
                let on_river = idx_of(cgx + dx, cgy).is_some_and(|p| terrain[p] == TERRAIN_RIVER);
                let rkind = if on_river {
                    ROAD_BRIDGE_OR_PIER
                } else {
                    ROAD_TOWN_COBBLE
                };
                paint_road_tile(cidx, cgx + dx, cgy, rkind, cell_owner, terrain, road, bldg);
            }
            for dy in -(su.min(3))..=(sd.min(3)) {
                paint_road_tile(
                    cidx,
                    cgx,
                    cgy + dy,
                    ROAD_TOWN_COBBLE,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            gate_xy = (cgx - sl.min(4), cgy);
            place_structure_accent(
                cidx,
                cgx - 2,
                cgy - 1,
                true,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            place_structure_accent(
                cidx,
                cgx - 2,
                cgy + 1,
                true,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            if tier == "quarantine_post" {
                place_structure_accent(
                    cidx,
                    cgx + 2,
                    cgy - 1,
                    true,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                place_structure_accent(
                    cidx,
                    cgx + 2,
                    cgy + 1,
                    false,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            candidate_slots = vec![
                (cgx + 1, cgy - 2, Some((1, 0))),
                (cgx + 1, cgy + 2, Some((1, 0))),
                (cgx - 1, cgy - 2, Some((0, -1))),
                (cgx - 1, cgy + 2, Some((0, 1))),
                (cgx + 3, cgy - 2, None),
                (cgx + 3, cgy + 2, None),
                (cgx - 3, cgy - 2, None),
                (cgx - 3, cgy + 2, None),
                (cgx + 2, cgy - 3, None),
                (cgx + 2, cgy + 3, None),
            ];
        }
        RuralArchetype::HighlandBastion => {
            // 石砌迴廊與四角箭塔，南北與西門保留敞開通路
            for dx in -2isize..=2 {
                for dy in -2isize..=2 {
                    if dx.abs() <= 1 && dy.abs() <= 1 {
                        paint_road_tile(
                            cidx,
                            cgx + dx,
                            cgy + dy,
                            ROAD_TOWN_COBBLE,
                            cell_owner,
                            terrain,
                            road,
                            bldg,
                        );
                    }
                }
            }
            for step in 2..=sl.min(4) {
                paint_road_tile(
                    cidx,
                    cgx - step,
                    cgy,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            paint_road_tile(
                cidx,
                cgx + 2,
                cgy,
                ROAD_TOWN_COBBLE,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx,
                cgy - 2,
                ROAD_TOWN_COBBLE,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx,
                cgy + 2,
                ROAD_TOWN_COBBLE,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            gate_xy = (cgx - sl.min(3), cgy);

            for &(tx, ty) in &[
                (cgx - 3, cgy - 3),
                (cgx + 3, cgy - 3),
                (cgx - 3, cgy + 3),
                (cgx + 3, cgy + 3),
            ] {
                place_structure_accent(cidx, tx, ty, true, cell_owner, terrain, road, bldg);
            }
            for &(wx, wy) in &[
                (cgx - 2, cgy - 3),
                (cgx + 2, cgy - 3),
                (cgx - 2, cgy + 3),
                (cgx + 2, cgy + 3),
            ] {
                place_structure_accent(cidx, wx, wy, false, cell_owner, terrain, road, bldg);
            }
            candidate_slots = vec![
                (cgx + 1, cgy - 2, Some((1, 0))),
                (cgx - 1, cgy - 2, None),
                (cgx + 1, cgy + 2, Some((1, 0))),
                (cgx - 1, cgy + 2, None),
                (cgx + 2, cgy - 1, Some((0, -1))),
                (cgx + 2, cgy + 1, Some((0, 1))),
                (cgx - 2, cgy - 1, None),
                (cgx - 2, cgy + 1, None),
            ];
        }
        RuralArchetype::CoastalFishingCove => {
            // 弧形濱海魚街＋伸海木棧橋碼頭與燈塔屋
            let (px, py): (isize, isize) = match coast_dir {
                "north" => (0, -1),
                "south" => (0, 1),
                "west" => (-1, 0),
                _ => (1, 0),
            };
            let (tx, ty) = (-py, px); // 垂直海岸線之岸街方向
            for s in -2..=2 {
                paint_road_tile(
                    cidx,
                    cgx + tx * s,
                    cgy + ty * s,
                    ROAD_TOWN_COBBLE,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            for step in 1..=4 {
                let on_water = idx_of(cgx + px * step, cgy + py * step)
                    .is_some_and(|p| matches!(terrain[p], TERRAIN_SEA | TERRAIN_RIVER));
                paint_road_tile(
                    cidx,
                    cgx + px * step,
                    cgy + py * step,
                    if on_water {
                        ROAD_BRIDGE_OR_PIER
                    } else {
                        ROAD_COUNTRY_TRACK
                    },
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            paint_road_tile(
                cidx,
                cgx - px,
                cgy - ty,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            gate_xy = (cgx + tx * 2, cgy + ty * 2);
            place_structure_accent(
                cidx,
                cgx + px + tx * 2,
                cgy + py + ty * 2,
                true,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            candidate_slots = vec![
                (cgx - px + tx, cgy - py + ty, None),
                (cgx - px - tx, cgy - py - ty, None),
                (cgx - px * 2, cgy - py * 2, Some((tx, ty))),
                (cgx + tx * 2 - px, cgy + ty * 2 - py, None),
                (cgx - tx * 2 - px, cgy - ty * 2 - py, None),
                (cgx + tx * 3, cgy + ty * 3, None),
                (cgx - tx * 3, cgy - ty * 3, None),
            ];
        }
        RuralArchetype::ForestYForkCamp => {
            // Ｙ字岔路口炭寮伐木場（4-鄰接連貫不跳格）
            for dx in -(sl.min(3))..=0 {
                paint_road_tile(
                    cidx,
                    cgx + dx,
                    cgy,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
            }
            // 上杈 (1,0)->(1,-1)->(2,-1)->(3,-1)
            paint_road_tile(
                cidx,
                cgx + 1,
                cgy,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 1,
                cgy - 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 2,
                cgy - 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 3,
                cgy - 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            // 下杈 (0,1)->(1,1)->(2,1)->(3,1)
            paint_road_tile(
                cidx,
                cgx,
                cgy + 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 1,
                cgy + 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 2,
                cgy + 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            paint_road_tile(
                cidx,
                cgx + 3,
                cgy + 1,
                ROAD_COUNTRY_TRACK,
                cell_owner,
                terrain,
                road,
                bldg,
            );
            gate_xy = (cgx - sl.min(3), cgy);
            candidate_slots = vec![
                (cgx + 1, cgy - 2, Some((1, 0))),
                (cgx + 2, cgy + 2, Some((1, 0))),
                (cgx - 1, cgy - 1, None),
                (cgx - 2, cgy + 1, None),
                (cgx + 3, cgy - 2, None),
                (cgx + 3, cgy + 2, None),
                (cgx - 2, cgy - 2, None),
                (cgx, cgy - 2, None),
            ];
        }
        RuralArchetype::SteadOrTerracedPasture => {
            if tier == "survivor_stead" {
                // 倖存林莊：木柵瞭望哨＋Ｌ形內庭
                for dx in -1..=2 {
                    paint_road_tile(
                        cidx,
                        cgx + dx,
                        cgy,
                        ROAD_COUNTRY_TRACK,
                        cell_owner,
                        terrain,
                        road,
                        bldg,
                    );
                }
                for dy in -1..=2 {
                    paint_road_tile(
                        cidx,
                        cgx,
                        cgy + dy,
                        ROAD_COUNTRY_TRACK,
                        cell_owner,
                        terrain,
                        road,
                        bldg,
                    );
                }
                place_structure_accent(
                    cidx,
                    cgx + 2,
                    cgy - 2,
                    true,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                place_structure_accent(
                    cidx,
                    cgx - 2,
                    cgy - 1,
                    false,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                place_structure_accent(
                    cidx,
                    cgx - 2,
                    cgy + 1,
                    false,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                gate_xy = (cgx, cgy + 2);
                candidate_slots = vec![
                    (cgx + 1, cgy - 1, Some((0, -1))),
                    (cgx - 1, cgy - 1, None),
                    (cgx + 1, cgy + 1, None),
                    (cgx - 1, cgy + 1, None),
                    (cgx + 2, cgy + 1, None),
                ];
            } else {
                // 梯田牧坡與錯落村道：彎折石階小道＋鄰坡耕地
                paint_road_tile(
                    cidx,
                    cgx - 2,
                    cgy,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                paint_road_tile(
                    cidx,
                    cgx - 1,
                    cgy,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                paint_road_tile(
                    cidx,
                    cgx + 1,
                    cgy,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                paint_road_tile(
                    cidx,
                    cgx + 1,
                    cgy + 1,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                paint_road_tile(
                    cidx,
                    cgx + 2,
                    cgy + 1,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                paint_road_tile(
                    cidx,
                    cgx,
                    cgy - 1,
                    ROAD_COUNTRY_TRACK,
                    cell_owner,
                    terrain,
                    road,
                    bldg,
                );
                for &(fx, fy) in &[
                    (cgx - 2, cgy + 2),
                    (cgx - 1, cgy + 2),
                    (cgx, cgy + 2),
                    (cgx - 2, cgy - 2),
                ] {
                    if let Some(fpos) = idx_of(fx, fy)
                        && cell_owner[fpos] == cidx
                        && terrain[fpos] == TERRAIN_LAND
                        && road[fpos] == ROAD_NONE
                        && bldg[fpos] == BLDG_NONE
                    {
                        terrain[fpos] = TERRAIN_FARM;
                    }
                }
                gate_xy = (cgx - 2, cgy);
                candidate_slots = vec![
                    (cgx + 1, cgy - 1, Some((1, 0))),
                    (cgx - 1, cgy - 1, None),
                    (cgx + 2, cgy + 2, None),
                    (cgx - 2, cgy + 1, None),
                    (cgx + 3, cgy, None),
                    (cgx + 3, cgy + 2, None),
                    (cgx, cgy - 2, Some((1, 0))),
                    (cgx - 3, cgy - 1, None),
                ];
            }
        }
    }

    node_anchors.insert(
        format!("oi:{slug}:village_gate"),
        [gate_xy.0 as f32 + 0.5, gate_xy.1 as f32 + 0.5],
    );
    for special_suffix in ["tea_shed", "quarantine_post", "survivor_palisade"] {
        let nid = format!("oi:{slug}:{special_suffix}");
        if doc.nodes.contains_key(&nid) {
            node_anchors.insert(nid, [cgx as f32 + 0.5, cgy as f32 + 0.5]);
        }
    }

    // 把此聚落每間房舍依專屬幾何槽位落定；若前幾個槽位受水域地形阻礙則向外環安全格掃描，絕不漏屋或堵路
    for (idx, rf) in roofs.iter().enumerate() {
        let mut placed = false;
        if let Some(&(sx, sy, span)) = candidate_slots.get(idx) {
            placed = place_roof_slot(
                cidx,
                sx,
                sy,
                rf,
                span,
                cell_owner,
                terrain,
                road,
                bldg,
                node_anchors,
                house_footprint_cells,
            );
        }
        if !placed {
            'outer: for r in 1..=6isize {
                for dy in -r..=r {
                    for dx in -r..=r {
                        if dx.abs().max(dy.abs()) != r {
                            continue;
                        }
                        if place_roof_slot(
                            cidx,
                            cgx + dx,
                            cgy + dy,
                            rf,
                            None,
                            cell_owner,
                            terrain,
                            road,
                            bldg,
                            node_anchors,
                            house_footprint_cells,
                        ) {
                            placed = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
        if !placed && let Some(rnid) = rf.get("node_id").and_then(|v| v.as_str()) {
            node_anchors.insert(rnid.to_string(), [cgx as f32 + 0.5, cgy as f32 + 0.5]);
        }
    }
}

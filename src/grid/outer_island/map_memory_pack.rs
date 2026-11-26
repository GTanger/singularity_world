//! 玩家端無黑霧個人親見記憶輿圖封包編碼（`18` 格局部實況、`<= 14` 格戶外人物、`<= 9.5` 格近身膠囊）。

use std::collections::BTreeSet;
use std::hash::{Hash, Hasher};

use serde_json::{Value, json};

use super::cell_resources::is_low_tide_window;
use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    BLDG_ROOF, BLDG_RUIN_WALL, GRID_H, GRID_SCALE, GRID_W, WX0, WY0, compiled_island_substrate,
    world_to_grid_f32,
};
use super::island_substrate_pack::{
    apply_tile_hydrology_and_tide, base64_encode, place_built_house_roof, rle_encode_bytes,
};
use super::live_land::house_is_ruined;
use super::live_land_tiles::effective_tile_state;
use super::map_memory::{WitnessedCellSnap, actor_has_core_cell_access, live_cell_snap};
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

const SIGHT_RADIUS_SQ: isize = 18 * 18;
const SIGHT_RADIUS_F32_SQ: f32 = 18.0 * 18.0;
const NEARBY_ACTOR_RADIUS_SQ: f32 = 14.0 * 14.0;
const CAPSULE_RADIUS_SQ: f32 = 9.5 * 9.5;

fn resolve_poi_nav_node(
    doc: &WorldStaticDoc,
    seed_id: &str,
    cid: u32,
    known_nodes: &BTreeSet<String>,
) -> Option<String> {
    if let Some(slug) = seed_id.strip_prefix("town:") {
        let candidates = [
            format!("oi:{slug}:plaza"),
            format!("oi:{slug}:yard"),
            format!("oi:{slug}:north_gate"),
            format!("oi:{slug}:south_gate"),
            format!("oi:{slug}:dock"),
            format!("oi:cell{cid}:0"),
        ];
        return candidates
            .into_iter()
            .find(|nid| known_nodes.contains(nid) && doc.nodes.contains_key(nid));
    }
    if seed_id.starts_with("cell:") {
        let candidates = [format!("oi:cell{cid}:0"), format!("oi:{seed_id}:ruin_hall")];
        return candidates
            .into_iter()
            .find(|nid| known_nodes.contains(nid) && doc.nodes.contains_key(nid));
    }
    if known_nodes.contains(seed_id) && doc.nodes.contains_key(seed_id) {
        Some(seed_id.to_string())
    } else {
        None
    }
}

fn short_node_label(name: &str) -> &str {
    name.rsplit('・')
        .next()
        .unwrap_or(name)
        .split('（')
        .next()
        .unwrap_or(name)
}

fn viewer_standing_xy(
    doc: &WorldStaticDoc,
    sub: &super::island_substrate::CompiledIslandSubstrate,
    viewer: &UnifiedMindBody,
) -> [f32; 2] {
    let cur_node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node));
    if viewer.map_pos_seeded {
        return [viewer.map_gx, viewer.map_gy];
    }
    cur_node
        .and_then(|nd| {
            if nd.dungeon {
                sub.node_anchors.get("oi:cell4529:0").copied()
            } else if nd.indoor {
                sub.door_tiles
                    .get(&nd.id)
                    .map(|&[dx, dy]| [dx as f32 + 0.5, dy as f32 + 0.5])
                    .or_else(|| sub.node_anchors.get(&nd.id).copied())
            } else {
                sub.node_anchors.get(&nd.id).copied()
            }
            .or_else(|| Some(world_to_grid_f32(nd.pos[0], nd.pos[1])))
        })
        .unwrap_or([430.0, 465.0])
}

/// 計算旅人當前親見與近距地貌之穩定 64 位元摘要（供行進 `map_sync` 免重編碼 `799,800` 格 RLE）。
pub fn compute_viewer_terrain_digest(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> u64 {
    let sub = compiled_island_substrate(doc);
    let low_tide = is_low_tide_window(clock);
    let cur_node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node));
    let in_dungeon = cur_node.is_some_and(|nd| nd.dungeon);
    let cur_outdoor_cell = cur_node.filter(|nd| !nd.dungeon).map(|nd| nd.cell_id);
    let [vx, vy] = viewer_standing_xy(doc, sub, viewer);
    let mem = &viewer.mind.ltm.map_memory;

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    low_tide.hash(&mut hasher);
    in_dungeon.hash(&mut hasher);
    if in_dungeon {
        viewer.node_id.hash(&mut hasher);
        viewer.mind.ltm.known_nodes.len().hash(&mut hasher);
    }

    let mut eff_diffs = mem.witnessed_tile_diffs.clone();
    let mut in_sight_owner = vec![false; sub.cell_ids.len()];
    if !in_dungeon {
        let ivx = vx.floor() as isize;
        let ivy = vy.floor() as isize;
        eff_diffs.retain(|&pos_u32, _| {
            let pos = pos_u32 as usize;
            let dx = (pos % GRID_W) as isize - ivx;
            let dy = (pos / GRID_W) as isize - ivy;
            dx * dx + dy * dy > SIGHT_RADIUS_SQ || delta.tile_diffs.contains_key(&pos_u32)
        });
        for (&pos_u32, &e) in &delta.tile_diffs {
            let pos = pos_u32 as usize;
            let dx = (pos % GRID_W) as isize - ivx;
            let dy = (pos / GRID_W) as isize - ivy;
            if dx * dx + dy * dy <= SIGHT_RADIUS_SQ {
                eff_diffs.insert(pos_u32, e);
            }
        }
        for dy in -18isize..=18 {
            for dx in -18isize..=18 {
                if dx * dx + dy * dy <= SIGHT_RADIUS_SQ
                    && let Some(pos) = super::island_substrate::idx_of(ivx + dx, ivy + dy)
                {
                    let owner = sub.cell_owner[pos];
                    if owner != u16::MAX {
                        in_sight_owner[owner as usize] = true;
                    }
                }
            }
        }
    }
    eff_diffs.len().hash(&mut hasher);
    for (pos_u32, e) in &eff_diffs {
        pos_u32.hash(&mut hasher);
        e.terrain.hash(&mut hasher);
        e.road.hash(&mut hasher);
        e.bldg.hash(&mut hasher);
    }

    for (idx, &cid) in sub.cell_ids.iter().enumerate() {
        let (mem_flood, mem_bridge) = mem
            .witnessed_cells
            .get(&cid)
            .map_or((0u8, false), |s| (s.flood_level, s.bridge_down));
        let (live_flood, live_bridge) =
            delta.live_land.cells.get(&cid).map_or((0u8, false), |cl| {
                (
                    cl.flood_level,
                    cl.has_river_crossing && cl.bridge_integrity == 0,
                )
            });
        let (eff_flood, eff_bridge) = if !in_dungeon && in_sight_owner[idx] {
            (live_flood, live_bridge)
        } else {
            (mem_flood, mem_bridge)
        };
        if eff_flood > 0 || eff_bridge {
            cid.hash(&mut hasher);
            eff_flood.hash(&mut hasher);
            eff_bridge.hash(&mut hasher);
        }
    }

    for hnid in sub.house_footprint_cells.keys() {
        let near_house = !in_dungeon
            && sub.node_anchors.get(hnid).is_some_and(|&[ax, ay]| {
                (ax - vx).powi(2) + (ay - vy).powi(2) <= SIGHT_RADIUS_F32_SQ
            });
        let ruined = if near_house {
            house_is_ruined(delta, hnid)
        } else {
            mem.witnessed_ruined_houses.contains(hnid)
        };
        let sold = if near_house {
            delta.sold_houses.contains_key(hnid)
        } else {
            mem.witnessed_sold_houses.contains(hnid)
        };
        if ruined || sold {
            hnid.hash(&mut hasher);
            ruined.hash(&mut hasher);
            sold.hash(&mut hasher);
        }
    }

    for bh in delta.built_houses.values() {
        if delta.place_registry.contains_key(&bh.house_id) {
            continue;
        }
        let Some(and) = doc.nodes.get(&bh.anchor_node_id) else {
            continue;
        };
        let show = cur_outdoor_cell == Some(and.cell_id)
            || mem.witnessed_built_houses.contains_key(&bh.house_id);
        if show && actor_has_core_cell_access(doc, viewer, and.cell_id) {
            bh.house_id.hash(&mut hasher);
            bh.anchor_node_id.hash(&mut hasher);
        }
    }
    hasher.finish()
}

struct ViewerOverlayBundle {
    pois: Vec<Value>,
    nearby_actors: Vec<Value>,
    proximity_capsules: Vec<Value>,
    rumor_pins: Vec<Value>,
    building_hits: Vec<Value>,
}

fn collect_viewer_map_overlays(
    doc: &WorldStaticDoc,
    sub: &super::island_substrate::CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
    vx: f32,
    vy: f32,
    in_dungeon: bool,
    cur_outdoor_cell: Option<u32>,
    core_by_idx: &[bool],
) -> ViewerOverlayBundle {
    let mem = &viewer.mind.ltm.map_memory;
    let known_nodes = &viewer.mind.ltm.known_nodes;
    let mut pois: Vec<Value> = Vec::new();
    for p in &sub.poi_base_entries {
        if let Some(nav_id) = resolve_poi_nav_node(doc, &p.id, p.cell_id, known_nodes) {
            pois.push(json!({
                "id": p.id,
                "nav_id": nav_id,
                "label": p.label,
                "tier": p.tier,
                "gx": p.gx,
                "gy": p.gy,
            }));
        }
    }
    for bh in delta.built_houses.values() {
        if delta.place_registry.contains_key(&bh.house_id) {
            continue;
        }
        let Some(and) = doc.nodes.get(&bh.anchor_node_id) else {
            continue;
        };
        let show = cur_outdoor_cell == Some(and.cell_id)
            || mem.witnessed_built_houses.contains_key(&bh.house_id);
        if !show {
            continue;
        }
        let cidx = sub
            .cell_index_by_id
            .get(&and.cell_id)
            .copied()
            .unwrap_or(u16::MAX) as usize;
        if !core_by_idx.get(cidx).copied().unwrap_or(false) {
            continue;
        }
        if let Some(&[gx, gy]) = sub.node_anchors.get(&bh.anchor_node_id) {
            pois.push(json!({
                "id": bh.house_id,
                "nav_id": bh.anchor_node_id,
                "label": bh.label_zh,
                "tier": "built",
                "gx": gx,
                "gy": gy,
            }));
        }
    }
    for rec in delta.place_registry.values() {
        if rec.ruined {
            continue;
        }
        let [gx, gy] = super::place_registry::footprint_center_xy(rec);
        let near = !in_dungeon && (gx - vx).powi(2) + (gy - vy).powi(2) <= SIGHT_RADIUS_F32_SQ;
        let show = near
            || cur_outdoor_cell == Some(rec.cell_id)
            || mem.witnessed_built_houses.contains_key(&rec.place_id);
        let cidx = sub
            .cell_index_by_id
            .get(&rec.cell_id)
            .copied()
            .unwrap_or(u16::MAX) as usize;
        if show && core_by_idx.get(cidx).copied().unwrap_or(false) {
            pois.push(json!({
                "id": rec.place_id,
                "nav_id": rec.anchor_node_id,
                "label": rec.label_zh,
                "tier": "built",
                "gx": gx,
                "gy": gy,
            }));
        }
    }
    for band in delta.live_wilds.bands.values() {
        let in_near_band = !in_dungeon
            && sub
                .node_anchors
                .get(&band.camp_node_id)
                .is_some_and(|&[ax, ay]| {
                    (ax - vx).powi(2) + (ay - vy).powi(2) <= SIGHT_RADIUS_F32_SQ
                });
        if (cur_outdoor_cell == Some(band.cell_id) || in_near_band)
            && clock.game_days >= band.suppressed_until_day
            && (band.raid_pressure >= 60 || band.hoard_copper >= 8)
            && let Some(&[gx, gy]) = sub.node_anchors.get(&band.camp_node_id)
        {
            pois.push(json!({
                "id": format!("band:{}", band.cell_id),
                "nav_id": band.camp_node_id,
                "label": band.band_name_zh,
                "tier": "ruin",
                "gx": gx,
                "gy": gy,
            }));
        }
    }
    for wb in mem.witnessed_bands.values() {
        if cur_outdoor_cell != Some(wb.cell_id)
            && let Some(&[gx, gy]) = sub.node_anchors.get(&wb.camp_node_id)
        {
            pois.push(json!({
                "id": format!("band:{}", wb.cell_id),
                "nav_id": wb.camp_node_id,
                "label": wb.band_name_zh,
                "tier": "ruin",
                "gx": gx,
                "gy": gy,
            }));
        }
    }

    let mut nearby_actors: Vec<(f32, Value)> = Vec::new();
    if !in_dungeon {
        for a in delta.actors.values() {
            if !a.alive
                || !a.setup_done
                || a.actor_id == viewer.actor_id
                || a.active_transit.is_some()
                || !a.map_pos_seeded
            {
                continue;
            }
            if doc
                .nodes
                .get(&a.node_id)
                .is_some_and(|nd| nd.indoor || nd.dungeon)
            {
                continue;
            }
            let d2 = (a.map_gx - vx).powi(2) + (a.map_gy - vy).powi(2);
            if d2 <= NEARBY_ACTOR_RADIUS_SQ {
                let shown = if a.controller_id.is_some() && !viewer.is_acquainted_with(&a.actor_id)
                {
                    format!("旅人・{}", a.role)
                } else {
                    a.display_name.clone()
                };
                nearby_actors.push((
                    d2,
                    json!({
                        "id": a.public_actor_token(),
                        "name": shown,
                        "role": a.role,
                        "gx": a.map_gx,
                        "gy": a.map_gy,
                        "walking": !a.walk_waypoints.is_empty(),
                    }),
                ));
            }
        }
        nearby_actors.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    let nearby_actors_json: Vec<Value> = nearby_actors.into_iter().map(|(_, v)| v).collect();

    let mut proximity_capsules: Vec<(u8, f32, Value)> = Vec::new();
    if !in_dungeon {
        let mut seen_ids = BTreeSet::new();
        for (nid, nd) in &doc.nodes {
            if nd.dungeon && nid != "oi:dng_pit:entry" {
                continue;
            }
            if nid == "oi:dng_pit:entry" && !viewer.unlocked_secrets.contains("oi:cell4529:0") {
                continue;
            }
            if !nd.indoor
                && !matches!(
                    nd.kind.as_str(),
                    "gate" | "village_gate" | "dock" | "plaza" | "wild" | "camp" | "ruin"
                )
            {
                continue;
            }
            let anchor = sub.node_anchors.get(nid).copied();
            let door = sub
                .door_tiles
                .get(nid)
                .map(|&[dx, dy]| [dx as f32 + 0.5, dy as f32 + 0.5]);
            let Some([gx, gy]) = anchor.or(door) else {
                continue;
            };
            let [dgx, dgy] = door.unwrap_or([gx, gy]);
            let d2 = ((gx - vx).powi(2) + (gy - vy).powi(2))
                .min((dgx - vx).powi(2) + (dgy - vy).powi(2));
            if d2 <= CAPSULE_RADIUS_SQ {
                let is_res_house = nid
                    .rsplit(':')
                    .next()
                    .and_then(|tail| tail.strip_prefix('h'))
                    .is_some_and(|num| !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()));
                if is_res_house {
                    let is_vacant_sale = nd.kind == "vacant_house"
                        && !delta.sold_houses.contains_key(nid)
                        && !house_is_ruined(delta, nid);
                    let max_res_d2 = if is_vacant_sale { 20.25 } else { 6.25 };
                    if d2 > max_res_d2 {
                        continue;
                    }
                }
                if seen_ids.insert(nid.clone()) {
                    proximity_capsules.push((
                        u8::from(is_res_house),
                        d2,
                        json!({
                            "id": nid,
                            "label": short_node_label(&nd.name),
                            "kind": nd.kind,
                            "indoor": nd.indoor,
                            "gx": gx,
                            "gy": gy,
                            "door_gx": dgx,
                            "door_gy": dgy,
                            "dist": d2.sqrt(),
                        }),
                    ));
                }
            }
        }
        for bh in delta.built_houses.values() {
            if let Some(&[gx, gy]) = sub.node_anchors.get(&bh.anchor_node_id) {
                let d2 = (gx - vx).powi(2) + (gy - vy).powi(2);
                if d2 <= CAPSULE_RADIUS_SQ && seen_ids.insert(bh.house_id.clone()) {
                    proximity_capsules.push((
                        0,
                        d2,
                        json!({
                            "id": bh.house_id,
                            "label": bh.label_zh,
                            "kind": "built",
                            "indoor": true,
                            "gx": gx,
                            "gy": gy,
                            "door_gx": gx,
                            "door_gy": gy,
                            "dist": d2.sqrt(),
                        }),
                    ));
                }
            }
        }
        proximity_capsules.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.total_cmp(&b.1)));
    }
    let proximity_capsules_json: Vec<Value> =
        proximity_capsules.into_iter().map(|(_, _, v)| v).collect();

    let mut building_hits: Vec<Value> = Vec::new();
    for (nid, cells) in &sub.house_footprint_cells {
        if cells.is_empty() {
            continue;
        }
        let Some(nd) = doc.nodes.get(nid) else {
            continue;
        };
        let cidx = sub
            .cell_index_by_id
            .get(&nd.cell_id)
            .copied()
            .unwrap_or(u16::MAX) as usize;
        let core_ok = core_by_idx.get(cidx).copied().unwrap_or(false);
        let near_hit = !in_dungeon
            && sub.node_anchors.get(nid).is_some_and(|&[ax, ay]| {
                (ax - vx).powi(2) + (ay - vy).powi(2) <= CAPSULE_RADIUS_SQ
            });
        if !known_nodes.contains(nid) && !(core_ok && near_hit) {
            continue;
        }
        let (mut gx0, mut gy0, mut gx1, mut gy1) = (GRID_W, GRID_H, 0usize, 0usize);
        for &pos in cells {
            let x = pos % GRID_W;
            let y = pos / GRID_W;
            gx0 = gx0.min(x);
            gy0 = gy0.min(y);
            gx1 = gx1.max(x + 1);
            gy1 = gy1.max(y + 1);
        }
        let door_badge = nid
            .rsplit(':')
            .next()
            .and_then(|tail| tail.strip_prefix('h'))
            .filter(|num| !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()))
            .map(|num| format!("{num}號"));
        building_hits.push(json!({
            "nid": nid,
            "label": short_node_label(&nd.name),
            "door_badge": door_badge,
            "gx0": gx0,
            "gy0": gy0,
            "gx1": gx1,
            "gy1": gy1,
        }));
    }

    let mut rumor_pins_json: Vec<Value> = mem
        .rumor_pins
        .values()
        .map(|p| json!({ "id": p.pin_id, "label": p.label_zh, "gx": p.gx, "gy": p.gy }))
        .collect();
    if viewer.known_rumors.contains("pit_rumor")
        && !viewer.unlocked_secrets.contains("oi:cell4529:0")
        && !mem.rumor_pins.contains_key("pit_rumor")
        && let Some(&[gx, gy]) = sub.node_anchors.get("oi:cell4529:0")
    {
        rumor_pins_json.push(json!({
            "id": "pit_rumor",
            "label": "？傳聞：灰松斷橋石縫暗門",
            "gx": gx,
            "gy": gy,
        }));
    }

    ViewerOverlayBundle {
        pois,
        nearby_actors: nearby_actors_json,
        proximity_capsules: proximity_capsules_json,
        rumor_pins: rumor_pins_json,
        building_hits,
    }
}

/// 產出步行途中極輕量 `<3KB` 同步封包，僅於地貌見證實變時置 `terrain_dirty = true` 觸發全圖重拉。
pub fn build_walk_map_sync_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
    prev_digest: Option<u64>,
) -> Value {
    let sub = compiled_island_substrate(doc);
    let cur_node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node));
    let in_dungeon = cur_node.is_some_and(|nd| nd.dungeon);
    let in_indoor = cur_node.is_some_and(|nd| nd.indoor);
    let cur_outdoor_cell = cur_node.filter(|nd| !nd.dungeon).map(|nd| nd.cell_id);
    let [vx, vy] = viewer_standing_xy(doc, sub, viewer);
    let core_by_idx: Vec<bool> = sub
        .cell_ids
        .iter()
        .map(|&cid| actor_has_core_cell_access(doc, viewer, cid))
        .collect();
    let overlays = collect_viewer_map_overlays(
        doc,
        sub,
        delta,
        viewer,
        clock,
        vx,
        vy,
        in_dungeon,
        cur_outdoor_cell,
        &core_by_idx,
    );
    let digest = compute_viewer_terrain_digest(doc, delta, viewer, clock);
    let terrain_dirty = prev_digest.is_some_and(|p| p != digest);
    let walk_waypoints_json: Vec<[u16; 2]> = viewer.walk_waypoints.iter().copied().collect();
    json!({
        "current_cell_id": cur_node.map_or(0, |n| n.cell_id),
        "current_node_id": cur_node.map_or(doc.spawn_node.as_str(), |n| n.id.as_str()),
        "current_node_name": cur_node.map_or("", |n| n.name.as_str()),
        "in_dungeon": in_dungeon,
        "in_indoor": in_indoor,
        "indoor_node_id": if in_indoor { json!(viewer.node_id) } else { Value::Null },
        "indoor_label": if in_indoor { json!(cur_node.map_or("", |n| short_node_label(&n.name))) } else { Value::Null },
        "terrain_digest": digest.to_string(),
        "terrain_dirty": terrain_dirty,
        "player_pos": [vx, vy],
        "walk_waypoints": walk_waypoints_json,
        "nearby_actors": overlays.nearby_actors,
        "proximity_capsules": overlays.proximity_capsules,
        "pois": overlays.pois,
        "rumor_pins": overlays.rumor_pins,
        "building_hits": overlays.building_hits,
    })
}

/// 依旅人個人親見記憶與近身物理距離合成全島零黑霧底盤、近距實時地況、`nearby_actors` 與 `proximity_capsules`。
pub fn build_viewer_memory_substrate_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> Value {
    let sub = compiled_island_substrate(doc);
    let total = GRID_W * GRID_H;
    let low_tide = is_low_tide_window(clock);
    let cur_node = doc
        .nodes
        .get(&viewer.node_id)
        .or_else(|| doc.nodes.get(&doc.spawn_node));
    let in_dungeon = cur_node.is_some_and(|nd| nd.dungeon);
    let in_indoor = cur_node.is_some_and(|nd| nd.indoor);
    let cur_outdoor_cell = cur_node.filter(|nd| !nd.dungeon).map(|nd| nd.cell_id);

    let mem = &viewer.mind.ltm.map_memory;
    let [vx, vy] = viewer_standing_xy(doc, sub, viewer);
    let ivx = vx.floor() as isize;
    let ivy = vy.floor() as isize;

    let mut core_by_idx = vec![false; sub.cell_ids.len()];
    let mut cell_live_snap: Vec<WitnessedCellSnap> = Vec::with_capacity(sub.cell_ids.len());
    let mut cell_mem_snap: Vec<WitnessedCellSnap> = Vec::with_capacity(sub.cell_ids.len());
    let mut cell_reef = vec![false; sub.cell_ids.len()];

    for (idx, &cid) in sub.cell_ids.iter().enumerate() {
        let core_ok = actor_has_core_cell_access(doc, viewer, cid);
        core_by_idx[idx] = core_ok;
        let mut plague_base = false;
        if let Some(c) = doc.cells.get(&cid.to_string()) {
            cell_reef[idx] = c.is_coast && c.resource_slug == "shellfish_meat";
            plague_base = c.plague_sealed || c.kind == "plague_ruin";
        }
        cell_live_snap.push(live_cell_snap(doc, delta, cid, core_ok, clock.game_days));
        let mem_s = if let Some(saved) = mem.witnessed_cells.get(&cid) {
            saved.clone()
        } else {
            WitnessedCellSnap {
                flood_level: 0,
                bridge_down: false,
                forest_pct: 100,
                stock_pct: 100,
                reclaimed_plots: 0,
                fallow_days: 0,
                plague_active: plague_base,
                core_visited: core_ok,
                witnessed_day: clock.game_days,
            }
        };
        cell_mem_snap.push(mem_s);
    }

    let mut terrain = vec![0u8; total];
    let mut overlay = vec![0u8; total];
    for y in 0..GRID_H {
        let row = y * GRID_W;
        let dy = y as isize - ivy;
        let dy2 = dy * dy;
        for x in 0..GRID_W {
            let pos = row + x;
            let dx = x as isize - ivx;
            let in_sight = !in_dungeon && dx * dx + dy2 <= SIGHT_RADIUS_SQ;
            let diffs_map = if in_sight {
                &delta.tile_diffs
            } else {
                &mem.witnessed_tile_diffs
            };
            let (bt, br, bb) = effective_tile_state(sub, diffs_map, pos);
            let owner = sub.cell_owner[pos];
            if owner == u16::MAX {
                terrain[pos] = bt;
                overlay[pos] = br | (bb << 3);
                continue;
            }
            let oidx = owner as usize;
            let snap = if in_sight {
                &cell_live_snap[oidx]
            } else {
                &cell_mem_snap[oidx]
            };
            let (t, r, b) = apply_tile_hydrology_and_tide(
                sub,
                pos,
                x,
                y,
                bt,
                br,
                bb,
                snap.flood_level,
                snap.bridge_down,
                cell_reef[oidx],
                low_tide,
            );
            terrain[pos] = t;
            overlay[pos] = r | (b << 3);
        }
    }

    for (hnid, cells) in &sub.house_footprint_cells {
        let near_house = !in_dungeon
            && sub.node_anchors.get(hnid).is_some_and(|&[ax, ay]| {
                (ax - vx).powi(2) + (ay - vy).powi(2) <= SIGHT_RADIUS_F32_SQ
            });
        let ruined = if near_house {
            house_is_ruined(delta, hnid)
        } else {
            mem.witnessed_ruined_houses.contains(hnid)
        };
        let sold = if near_house {
            delta.sold_houses.contains_key(hnid)
        } else {
            mem.witnessed_sold_houses.contains(hnid)
        };
        let next_bldg = if ruined {
            Some(BLDG_RUIN_WALL)
        } else if sold {
            Some(BLDG_ROOF)
        } else {
            None
        };
        if let Some(nb) = next_bldg {
            for &pos in cells {
                overlay[pos] = (overlay[pos] & 0x07) | (nb << 3);
            }
        }
    }

    for bh in delta.built_houses.values() {
        if delta.place_registry.contains_key(&bh.house_id) {
            continue;
        }
        let Some(and) = doc.nodes.get(&bh.anchor_node_id) else {
            continue;
        };
        let show = cur_outdoor_cell == Some(and.cell_id)
            || mem.witnessed_built_houses.contains_key(&bh.house_id);
        let cidx = sub
            .cell_index_by_id
            .get(&and.cell_id)
            .copied()
            .unwrap_or(u16::MAX) as usize;
        if show && core_by_idx.get(cidx).copied().unwrap_or(false) {
            let _ = place_built_house_roof(sub, &terrain, &mut overlay, &bh.anchor_node_id);
        }
    }

    let overlays = collect_viewer_map_overlays(
        doc,
        sub,
        delta,
        viewer,
        clock,
        vx,
        vy,
        in_dungeon,
        cur_outdoor_cell,
        &core_by_idx,
    );
    let digest = compute_viewer_terrain_digest(doc, delta, viewer, clock);
    let walk_waypoints_json: Vec<[u16; 2]> = viewer.walk_waypoints.iter().copied().collect();
    json!({
        "grid_w": GRID_W,
        "grid_h": GRID_H,
        "wx0": WX0,
        "wy0": WY0,
        "scale": GRID_SCALE,
        "low_tide": low_tide,
        "in_indoor": in_indoor,
        "indoor_node_id": if in_indoor { json!(viewer.node_id) } else { Value::Null },
        "indoor_label": if in_indoor { json!(cur_node.map_or("", |n| short_node_label(&n.name))) } else { Value::Null },
        "terrain_digest": digest.to_string(),
        "terrain_rle_b64": base64_encode(&rle_encode_bytes(&terrain)),
        "overlay_rle_b64": base64_encode(&rle_encode_bytes(&overlay)),
        "player_pos": [vx, vy],
        "walk_waypoints": walk_waypoints_json,
        "nearby_actors": overlays.nearby_actors,
        "proximity_capsules": overlays.proximity_capsules,
        "pois": overlays.pois,
        "rumor_pins": overlays.rumor_pins,
        "building_hits": overlays.building_hits,
        "adjacent_exits": [],
    })
}

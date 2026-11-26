//! 旅人個人記憶輿圖封包合成：局部節點 Chamfer 揭霧、遠端災變隔離與屋頂／傳聞點圖互動資料組裝。

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::cell_resources::is_low_tide_window;
use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_RUIN_WALL, CompiledIslandSubstrate, GRID_H, GRID_SCALE, GRID_W,
    ROAD_BRIDGE_OR_PIER, ROAD_NONE, ROAD_STONE_WALL, ROAD_TOWN_COBBLE, TERRAIN_FARM, TERRAIN_FOG,
    TERRAIN_LAND, TERRAIN_REEF_SAND, TERRAIN_RIVER, TERRAIN_TREE, WX0, WY0,
    compiled_island_substrate, idx_of, noise01, world_to_grid_f32,
};
use super::island_substrate_pack::{base64_encode, rle_encode_bytes};
use super::live_land::house_is_ruined;
use super::map_memory::{WitnessedCellSnap, actor_has_core_cell_access, live_cell_snap};
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

const SKIRT_REVEAL_CHAMFER: u16 = 115;
const CORE_REVEAL_CHAMFER: u16 = 560;

fn compute_node_chamfer_dists(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    viewer: &UnifiedMindBody,
    core_by_idx: &[bool],
) -> (Vec<u16>, Vec<u16>) {
    let total = GRID_W * GRID_H;
    let mut core_d = vec![u16::MAX / 2; total];
    let mut skirt_d = vec![u16::MAX / 2; total];

    for nid in &viewer.mind.ltm.known_nodes {
        let Some(nd) = doc.nodes.get(nid) else {
            continue;
        };
        if nd.dungeon {
            continue;
        }
        let Some(&[ax, ay]) = sub.node_anchors.get(nid) else {
            continue;
        };
        let gx = (ax.floor() as isize).clamp(0, GRID_W as isize - 1) as usize;
        let gy = (ay.floor() as isize).clamp(0, GRID_H as isize - 1) as usize;
        let pos = gy * GRID_W + gx;
        let owner = sub.cell_owner[pos];
        let is_core_cell =
            owner != u16::MAX && core_by_idx.get(owner as usize).copied().unwrap_or(false);
        if is_core_cell {
            core_d[pos] = 0;
        } else {
            skirt_d[pos] = 0;
        }
    }

    for dist in [&mut core_d, &mut skirt_d] {
        for y in 0..GRID_H {
            let row = y * GRID_W;
            for x in 0..GRID_W {
                let pos = row + x;
                let mut d = dist[pos];
                if d == 0 {
                    continue;
                }
                if x > 0 {
                    d = d.min(dist[pos - 1] + 10);
                }
                if y > 0 {
                    let up = pos - GRID_W;
                    d = d.min(dist[up] + 10);
                    if x > 0 {
                        d = d.min(dist[up - 1] + 14);
                    }
                    if x + 1 < GRID_W {
                        d = d.min(dist[up + 1] + 14);
                    }
                }
                dist[pos] = d;
            }
        }
        for y in (0..GRID_H).rev() {
            let row = y * GRID_W;
            for x in (0..GRID_W).rev() {
                let pos = row + x;
                let mut d = dist[pos];
                if d == 0 {
                    continue;
                }
                if x + 1 < GRID_W {
                    d = d.min(dist[pos + 1] + 10);
                }
                if y + 1 < GRID_H {
                    let dn = pos + GRID_W;
                    d = d.min(dist[dn] + 10);
                    if x + 1 < GRID_W {
                        d = d.min(dist[dn + 1] + 14);
                    }
                    if x > 0 {
                        d = d.min(dist[dn - 1] + 14);
                    }
                }
                dist[pos] = d;
            }
        }
    }
    (core_d, skirt_d)
}

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

/// 依旅人個人親見記憶、局部節點視距揭霧與 `？傳聞` 釘圖合成玩家端無縫輿圖封包。
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
    let cur_outdoor_cell = cur_node.filter(|nd| !nd.dungeon).map(|nd| nd.cell_id);

    let mem = &viewer.mind.ltm.map_memory;
    let known_cells = &viewer.mind.ltm.known_cells;
    let known_nodes = &viewer.mind.ltm.known_nodes;

    let mut core_by_idx = vec![false; sub.cell_ids.len()];
    let mut cell_snap: Vec<WitnessedCellSnap> = Vec::with_capacity(sub.cell_ids.len());
    let mut cell_reef = vec![false; sub.cell_ids.len()];

    for (idx, &cid) in sub.cell_ids.iter().enumerate() {
        let core_ok = actor_has_core_cell_access(doc, viewer, cid);
        core_by_idx[idx] = core_ok;
        if let Some(c) = doc.cells.get(&cid.to_string()) {
            cell_reef[idx] = c.is_coast && c.resource_slug == "shellfish_meat";
        }
        let snap = if cur_outdoor_cell == Some(cid) {
            live_cell_snap(doc, delta, cid, core_ok, clock.game_days)
        } else if let Some(saved) = mem.witnessed_cells.get(&cid) {
            saved.clone()
        } else {
            let plague_base = doc
                .cells
                .get(&cid.to_string())
                .is_some_and(|c| c.kind == "plague_ruin");
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
        cell_snap.push(snap);
    }

    let (core_d, skirt_d) = compute_node_chamfer_dists(doc, sub, viewer, &core_by_idx);
    let mut revealed = vec![false; total];
    for pos in 0..total {
        let owner = sub.cell_owner[pos];
        if owner == u16::MAX {
            if core_d[pos] <= 290 || skirt_d[pos] <= 190 {
                revealed[pos] = true;
            }
            continue;
        }
        let oidx = owner as usize;
        let cid = sub.cell_ids[oidx];
        if !known_cells.contains(&cid) {
            continue;
        }
        if core_by_idx[oidx] {
            if core_d[pos] <= CORE_REVEAL_CHAMFER {
                revealed[pos] = true;
            }
        } else if skirt_d[pos] <= SKIRT_REVEAL_CHAMFER {
            revealed[pos] = true;
        }
    }

    let mut terrain = vec![TERRAIN_FOG; total];
    let mut overlay = vec![0u8; total];
    for y in 0..GRID_H {
        let row = y * GRID_W;
        for x in 0..GRID_W {
            let pos = row + x;
            if !revealed[pos] {
                continue;
            }
            let owner = sub.cell_owner[pos];
            if owner == u16::MAX {
                terrain[pos] = sub.base_terrain[pos];
                overlay[pos] = sub.base_road[pos] | (sub.base_bldg[pos] << 3);
                continue;
            }
            let oidx = owner as usize;
            let snap = &cell_snap[oidx];
            let mut t = sub.base_terrain[pos];
            let mut r = sub.base_road[pos];
            let mut b = sub.base_bldg[pos];

            if !core_by_idx[oidx] {
                if b != BLDG_NONE {
                    b = BLDG_NONE;
                    t = TERRAIN_LAND;
                }
                if r == ROAD_STONE_WALL || r == ROAD_TOWN_COBBLE {
                    r = ROAD_NONE;
                }
            }

            if snap.flood_level >= 1
                && t == TERRAIN_LAND
                && b == BLDG_NONE
                && r != ROAD_STONE_WALL
                && r != ROAD_TOWN_COBBLE
            {
                let rad = isize::from(snap.flood_level).min(2);
                let mut near_riv = false;
                'flood_chk: for dy in -rad..=rad {
                    for dx in -rad..=rad {
                        if dx * dx + dy * dy <= rad * rad
                            && let Some(npos) = idx_of(x as isize + dx, y as isize + dy)
                            && sub.base_terrain[npos] == TERRAIN_RIVER
                        {
                            near_riv = true;
                            break 'flood_chk;
                        }
                    }
                }
                if near_riv {
                    t = TERRAIN_RIVER;
                }
            }
            if snap.bridge_down && t == TERRAIN_RIVER && r == ROAD_BRIDGE_OR_PIER {
                r = ROAD_NONE;
            }
            if low_tide
                && t == TERRAIN_LAND
                && cell_reef[oidx]
                && sub.sea_dist_by_cell_edge[pos] <= 2
                && b == BLDG_NONE
                && r == ROAD_NONE
            {
                t = TERRAIN_REEF_SAND;
            }
            let pct = if t == TERRAIN_TREE {
                (u32::from(snap.stock_pct) * u32::from(snap.forest_pct)) / 100
            } else {
                u32::from(snap.stock_pct)
            };
            if (t == TERRAIN_TREE || t == TERRAIN_FARM) && pct < 95 {
                let h = ((x as u32).wrapping_mul(73) ^ (y as u32).wrapping_mul(179)) % 100;
                if h >= pct {
                    t = TERRAIN_LAND;
                }
            }
            if core_by_idx[oidx]
                && t == TERRAIN_LAND
                && snap.reclaimed_plots > 0
                && b == BLDG_NONE
                && r == ROAD_NONE
                && sub.sea_dist_by_cell_edge[pos] >= 2
            {
                let patch_slot = ((x / 5).wrapping_mul(31) ^ (y / 5).wrapping_mul(97)) % 12;
                if patch_slot < usize::from(snap.reclaimed_plots) {
                    t = TERRAIN_FARM;
                }
            } else if t == TERRAIN_FARM && snap.fallow_days >= 18 {
                let reg_pct = ((snap.fallow_days - 15) * 8).min(80) as usize;
                let rh = ((x as usize).wrapping_mul(53) ^ (y as usize).wrapping_mul(131)) % 100;
                if rh < reg_pct {
                    t = TERRAIN_LAND;
                }
            }
            if snap.plague_active && b == BLDG_NONE && r == ROAD_NONE && t == TERRAIN_LAND {
                let h = noise01(x as isize, y as isize, 0xDEAD_BEEF);
                if h > 0.94 {
                    b = BLDG_RUIN_WALL;
                }
            }
            terrain[pos] = t;
            overlay[pos] = r | (b << 3);
        }
    }

    for (hnid, cells) in &sub.house_footprint_cells {
        let Some(hnd) = doc.nodes.get(hnid) else {
            continue;
        };
        let at_cur = cur_outdoor_cell == Some(hnd.cell_id);
        let ruined = if at_cur {
            house_is_ruined(delta, hnid)
        } else {
            mem.witnessed_ruined_houses.contains(hnid)
        };
        let sold = if at_cur {
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
                if revealed[pos] {
                    overlay[pos] = (overlay[pos] & 0x07) | (nb << 3);
                }
            }
        }
    }

    let mut visible_built: Vec<(String, String, String, f32, f32)> = Vec::new();
    let mut push_built = |house_id: &str, anchor_nid: &str, bh_cid: u32, label_zh: &str| {
        if !core_by_idx
            .get(
                sub.cell_index_by_id
                    .get(&bh_cid)
                    .copied()
                    .unwrap_or(u16::MAX) as usize,
            )
            .copied()
            .unwrap_or(false)
        {
            return;
        }
        let Some(&[ax, ay]) = sub.node_anchors.get(anchor_nid) else {
            return;
        };
        let gx0 = ax.floor() as isize;
        let gy0 = ay.floor() as isize;
        let mut placed_xy = None;
        for dy in -2..=2 {
            for dx in -2..=2 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if let Some(pos) = idx_of(gx0 + dx, gy0 + dy)
                    && revealed[pos]
                    && terrain[pos] == TERRAIN_LAND
                    && (overlay[pos] & 0x07) == ROAD_NONE
                    && (overlay[pos] >> 3) == BLDG_NONE
                {
                    overlay[pos] = (overlay[pos] & 0x07) | (BLDG_ROOF << 3);
                    placed_xy = Some([(gx0 + dx) as f32 + 0.5, (gy0 + dy) as f32 + 0.5]);
                    break;
                }
            }
            if placed_xy.is_some() {
                break;
            }
        }
        if let Some([px, py]) = placed_xy {
            visible_built.push((
                house_id.to_string(),
                anchor_nid.to_string(),
                label_zh.to_string(),
                px,
                py,
            ));
        }
    };
    if let Some(cur_cid) = cur_outdoor_cell {
        for bh in delta.built_houses.values() {
            if doc
                .nodes
                .get(&bh.anchor_node_id)
                .is_some_and(|and| and.cell_id == cur_cid)
            {
                push_built(&bh.house_id, &bh.anchor_node_id, cur_cid, &bh.label_zh);
            }
        }
    }
    for bh in mem.witnessed_built_houses.values() {
        if cur_outdoor_cell != Some(bh.cell_id) {
            push_built(&bh.house_id, &bh.anchor_node_id, bh.cell_id, &bh.label_zh);
        }
    }

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
    for (id, nav_id, label, gx, gy) in visible_built {
        pois.push(json!({
            "id": id,
            "nav_id": nav_id,
            "label": label,
            "tier": "built",
            "gx": gx,
            "gy": gy,
        }));
    }
    for band in delta.live_wilds.bands.values() {
        if cur_outdoor_cell == Some(band.cell_id)
            && core_by_idx
                .get(
                    sub.cell_index_by_id
                        .get(&band.cell_id)
                        .copied()
                        .unwrap_or(u16::MAX) as usize,
                )
                .copied()
                .unwrap_or(false)
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

    let mut adj_targets: BTreeSet<String> = BTreeSet::new();
    let mut adj_exits: Vec<Value> = Vec::new();
    let in_alley = cur_node.is_some_and(|nd| nd.kind == "alley");
    if let Some(nd) = cur_node
        && !nd.dungeon
    {
        for ex in &nd.exits {
            if let Some(dst) = doc.nodes.get(&ex.to)
                && !dst.dungeon
                && (known_cells.contains(&dst.cell_id) || ex.kind != "secret")
            {
                adj_targets.insert(dst.id.clone());
                if in_alley && matches!(dst.kind.as_str(), "house" | "house_vacant") {
                    continue;
                }
                if let Some(&[ax, ay]) = sub.node_anchors.get(&dst.id) {
                    let short_lbl = dst
                        .name
                        .rsplit('・')
                        .next()
                        .unwrap_or(&dst.name)
                        .split('（')
                        .next()
                        .unwrap_or(&dst.name);
                    adj_exits.push(json!({
                        "id": dst.id,
                        "label": short_lbl,
                        "gx": ax,
                        "gy": ay,
                    }));
                }
            }
        }
    }

    let mut building_hits: Vec<Value> = Vec::new();
    for (nid, cells) in &sub.house_footprint_cells {
        if cells.is_empty() || (!known_nodes.contains(nid) && !adj_targets.contains(nid)) {
            continue;
        }
        let Some(nd) = doc.nodes.get(nid) else {
            continue;
        };
        if !cells.iter().any(|&pos| revealed[pos]) {
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
        let short_lbl = nd
            .name
            .rsplit('・')
            .next()
            .unwrap_or(&nd.name)
            .split('（')
            .next()
            .unwrap_or(&nd.name);
        let door_badge = nid
            .rsplit(':')
            .next()
            .and_then(|tail| tail.strip_prefix('h'))
            .filter(|num| !num.is_empty() && num.chars().all(|c| c.is_ascii_digit()))
            .map(|num| format!("{num}號"));
        building_hits.push(json!({
            "nid": nid,
            "label": short_lbl,
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
        .map(|p| {
            json!({
                "id": p.pin_id,
                "label": p.label_zh,
                "gx": p.gx,
                "gy": p.gy,
            })
        })
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

    let cur_anchor = cur_node.and_then(|nd| {
        if nd.dungeon {
            doc.nodes
                .get("oi:cell4529:0")
                .and_then(|ent| sub.node_anchors.get(&ent.id))
                .copied()
        } else {
            sub.node_anchors
                .get(&nd.id)
                .copied()
                .or_else(|| {
                    delta
                        .built_houses
                        .get(&nd.id)
                        .and_then(|bh| sub.node_anchors.get(&bh.anchor_node_id))
                        .copied()
                })
                .or_else(|| Some(world_to_grid_f32(nd.pos[0], nd.pos[1])))
        }
    });

    json!({
        "grid_w": GRID_W,
        "grid_h": GRID_H,
        "wx0": WX0,
        "wy0": WY0,
        "scale": GRID_SCALE,
        "low_tide": low_tide,
        "terrain_rle_b64": base64_encode(&rle_encode_bytes(&terrain)),
        "overlay_rle_b64": base64_encode(&rle_encode_bytes(&overlay)),
        "player_pos": cur_anchor,
        "pois": pois,
        "rumor_pins": rumor_pins_json,
        "building_hits": building_hits,
        "adjacent_exits": adj_exits,
    })
}

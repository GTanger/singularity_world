//! 全島單一無縫底盤之即時迷霧濾鏡、動態狀態疊加（潮汐／新屋／採伐／售屋）與 RLE Base64 壓縮封包。

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
use super::slow_think::mask_player_names_in_text;
use super::types::{IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

pub(crate) fn rle_encode_bytes(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() / 16);
    let mut i = 0;
    while i < raw.len() {
        let val = raw[i];
        let mut run = 1usize;
        while i + run < raw.len() && raw[i + run] == val && run < 255 {
            run += 1;
        }
        out.push(run as u8);
        out.push(val);
        i += run;
    }
    out
}

pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const TBL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let n =
            (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8) | u32::from(bytes[i + 2]);
        out.push(TBL[((n >> 18) & 63) as usize] as char);
        out.push(TBL[((n >> 12) & 63) as usize] as char);
        out.push(TBL[((n >> 6) & 63) as usize] as char);
        out.push(TBL[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = bytes.len() - i;
    if rem == 1 {
        let n = u32::from(bytes[i]) << 16;
        out.push(TBL[((n >> 18) & 63) as usize] as char);
        out.push(TBL[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = (u32::from(bytes[i]) << 16) | (u32::from(bytes[i + 1]) << 8);
        out.push(TBL[((n >> 18) & 63) as usize] as char);
        out.push(TBL[((n >> 12) & 63) as usize] as char);
        out.push(TBL[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}

fn synthesize_live_substrate_buffers(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    known_cells_filter: Option<&BTreeSet<u32>>,
) -> (Vec<u8>, Vec<u8>, Vec<(String, u32, String, f32, f32)>, bool) {
    let total = GRID_W * GRID_H;
    let low_tide = is_low_tide_window(clock);

    let mut cell_known = vec![known_cells_filter.is_none(); sub.cell_ids.len()];
    let mut cell_reef = vec![false; sub.cell_ids.len()];
    let mut cell_stock_pct = vec![100u8; sub.cell_ids.len()];
    let mut cell_plague = vec![false; sub.cell_ids.len()];
    let mut cell_flood = vec![0u8; sub.cell_ids.len()];
    let mut cell_bridge_down = vec![false; sub.cell_ids.len()];
    let mut cell_forest_pct = vec![100u8; sub.cell_ids.len()];
    let mut cell_reclaimed = vec![0u8; sub.cell_ids.len()];
    let mut cell_fallow = vec![0u16; sub.cell_ids.len()];

    for (idx, &cid) in sub.cell_ids.iter().enumerate() {
        if let Some(known) = known_cells_filter {
            cell_known[idx] = known.contains(&cid);
        }
        if let Some(c) = doc.cells.get(&cid.to_string()) {
            cell_reef[idx] = c.is_coast && c.resource_slug == "shellfish_meat";
            cell_plague[idx] = c.kind == "plague_ruin";
            if c.stock_cap > 0 {
                let cur = delta.cell_stocks.get(&cid).copied().unwrap_or(c.stock_cap);
                cell_stock_pct[idx] =
                    ((u32::from(cur) * 100) / u32::from(c.stock_cap)).min(100) as u8;
            }
        }
        if let Some(cl) = delta.live_land.cells.get(&cid) {
            cell_flood[idx] = cl.flood_level;
            cell_bridge_down[idx] = cl.has_river_crossing && cl.bridge_integrity == 0;
            cell_forest_pct[idx] = ((u32::from(cl.forest_density) * 100) / 85).min(100) as u8;
            cell_reclaimed[idx] = cl.reclaimed_plots;
            cell_fallow[idx] = cl.fallow_days;
            if cl.plague_miasmas >= 30
                && doc
                    .cells
                    .get(&cid.to_string())
                    .is_some_and(|c| c.kind == "plague_ruin")
            {
                cell_plague[idx] = true;
            } else if cl.plague_miasmas < 30 && delta.live_land.seeded {
                cell_plague[idx] = false;
            }
        }
    }

    let sea_dist = if known_cells_filter.is_some() {
        let mut sd = vec![u16::MAX / 2; total];
        for y in 0..GRID_H {
            let row = y * GRID_W;
            for x in 0..GRID_W {
                let pos = row + x;
                let owner = sub.cell_owner[pos];
                if owner != u16::MAX && cell_known[owner as usize] {
                    sd[pos] = 0;
                }
            }
        }
        // Rosenfeld-Pfaltz 雙趟 Chamfer 距離場（正交 10、對角 14）：確保外海與迷霧沿海岸自然圓弧暈開，絕非直角矩形
        for y in 0..GRID_H {
            let row = y * GRID_W;
            for x in 0..GRID_W {
                let pos = row + x;
                let mut d = sd[pos];
                if d == 0 {
                    continue;
                }
                if x > 0 {
                    d = d.min(sd[pos - 1] + 10);
                }
                if y > 0 {
                    let up = pos - GRID_W;
                    d = d.min(sd[up] + 10);
                    if x > 0 {
                        d = d.min(sd[up - 1] + 14);
                    }
                    if x + 1 < GRID_W {
                        d = d.min(sd[up + 1] + 14);
                    }
                }
                sd[pos] = d;
            }
        }
        for y in (0..GRID_H).rev() {
            let row = y * GRID_W;
            for x in (0..GRID_W).rev() {
                let pos = row + x;
                let mut d = sd[pos];
                if d == 0 {
                    continue;
                }
                if x + 1 < GRID_W {
                    d = d.min(sd[pos + 1] + 10);
                }
                if y + 1 < GRID_H {
                    let dn = pos + GRID_W;
                    d = d.min(sd[dn] + 10);
                    if x + 1 < GRID_W {
                        d = d.min(sd[dn + 1] + 14);
                    }
                    if x > 0 {
                        d = d.min(sd[dn - 1] + 14);
                    }
                }
                sd[pos] = d;
            }
        }
        sd
    } else {
        vec![0u16; total]
    };

    let mut terrain = vec![TERRAIN_FOG; total];
    let mut overlay = vec![0u8; total];

    for y in 0..GRID_H {
        let row = y * GRID_W;
        for x in 0..GRID_W {
            let pos = row + x;
            let owner = sub.cell_owner[pos];
            if owner == u16::MAX {
                if known_cells_filter.is_none() || sea_dist[pos] <= 290 {
                    terrain[pos] = sub.base_terrain[pos];
                    overlay[pos] = sub.base_road[pos] | (sub.base_bldg[pos] << 3);
                }
                continue;
            }
            let oidx = owner as usize;
            if !cell_known[oidx] {
                continue;
            }
            let mut t = sub.base_terrain[pos];
            let mut r = sub.base_road[pos];
            let mut b = sub.base_bldg[pos];

            if cell_flood[oidx] >= 1
                && t == TERRAIN_LAND
                && b == BLDG_NONE
                && r != ROAD_STONE_WALL
                && r != ROAD_TOWN_COBBLE
            {
                let rad = isize::from(cell_flood[oidx]).min(2);
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
            if cell_bridge_down[oidx] && t == TERRAIN_RIVER && r == ROAD_BRIDGE_OR_PIER {
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
                (u32::from(cell_stock_pct[oidx]) * u32::from(cell_forest_pct[oidx])) / 100
            } else {
                u32::from(cell_stock_pct[oidx])
            };
            if (t == TERRAIN_TREE || t == TERRAIN_FARM) && pct < 95 {
                let h = ((x as u32).wrapping_mul(73) ^ (y as u32).wrapping_mul(179)) % 100;
                if h >= pct {
                    t = TERRAIN_LAND;
                }
            }
            if t == TERRAIN_LAND
                && cell_reclaimed[oidx] > 0
                && b == BLDG_NONE
                && r == ROAD_NONE
                && sub.sea_dist_by_cell_edge[pos] >= 2
            {
                let patch_slot = ((x / 5).wrapping_mul(31) ^ (y / 5).wrapping_mul(97)) % 12;
                if patch_slot < usize::from(cell_reclaimed[oidx]) {
                    t = TERRAIN_FARM;
                }
            } else if t == TERRAIN_FARM && cell_fallow[oidx] >= 18 {
                let reg_pct = ((cell_fallow[oidx] - 15) * 8).min(80) as usize;
                let rh = ((x as usize).wrapping_mul(53) ^ (y as usize).wrapping_mul(131)) % 100;
                if rh < reg_pct {
                    t = TERRAIN_LAND;
                }
            }
            if cell_plague[oidx] && b == BLDG_NONE && r == ROAD_NONE && t == TERRAIN_LAND {
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
        if super::live_land::house_is_ruined(delta, hnid) {
            for &pos in cells {
                let owner = sub.cell_owner[pos];
                if owner != u16::MAX && cell_known[owner as usize] {
                    let r = overlay[pos] & 0x07;
                    overlay[pos] = r | (BLDG_RUIN_WALL << 3);
                }
            }
        }
    }

    for sold_nid in delta.sold_houses.keys() {
        if let Some(cells) = sub.house_footprint_cells.get(sold_nid) {
            for &pos in cells {
                let owner = sub.cell_owner[pos];
                if owner != u16::MAX && cell_known[owner as usize] {
                    let r = overlay[pos] & 0x07;
                    overlay[pos] = r | (BLDG_ROOF << 3);
                }
            }
        }
    }

    let mut built_pois = Vec::new();
    for bh in delta.built_houses.values() {
        let Some(anchor_nd) = doc.nodes.get(&bh.anchor_node_id) else {
            continue;
        };
        if known_cells_filter.is_some_and(|known| !known.contains(&anchor_nd.cell_id)) {
            continue;
        }
        let Some(&[ax, ay]) = sub.node_anchors.get(&bh.anchor_node_id) else {
            continue;
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
            let pub_id = if known_cells_filter.is_none() && bh.house_id.contains("pl:") {
                format!("homestead:traveler:{}", bh.built_day)
            } else {
                bh.house_id.clone()
            };
            let label = if known_cells_filter.is_none() {
                mask_player_names_in_text(delta, &bh.label_zh)
            } else {
                bh.label_zh.clone()
            };
            built_pois.push((pub_id, anchor_nd.cell_id, label, px, py));
        }
    }

    (terrain, overlay, built_pois, low_tide)
}

/// 依旅人目前親見記憶輿圖、傳聞問號批註與潮汐，產生全島單一無縫底盤序列化 JSON。
pub fn build_viewer_substrate_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    viewer: &UnifiedMindBody,
    clock: &SolarClockSnapshot,
) -> Value {
    super::map_memory::build_viewer_memory_substrate_json(doc, delta, viewer, clock)
}

/// 產生全島 100% 無迷霧（零 `TERRAIN_FOG`）即時水彩活態底盤封包（專供 `/world_atlas` 設計者全開大地圖）。
pub fn build_full_revealed_substrate_json(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
) -> Value {
    let sub = compiled_island_substrate(doc);
    let (terrain, overlay, built_pois_raw, low_tide) =
        synthesize_live_substrate_buffers(doc, sub, delta, clock, None);

    let terrain_b64 = base64_encode(&rle_encode_bytes(&terrain));
    let overlay_b64 = base64_encode(&rle_encode_bytes(&overlay));

    let mut pois: Vec<Value> = Vec::with_capacity(
        sub.poi_base_entries.len() + built_pois_raw.len() + delta.live_wilds.bands.len(),
    );
    for p in &sub.poi_base_entries {
        pois.push(json!({
            "id": p.id,
            "cell_id": p.cell_id,
            "label": p.label,
            "tier": p.tier,
            "gx": p.gx,
            "gy": p.gy,
        }));
    }

    let mut node_anchors =
        serde_json::Map::with_capacity(sub.node_anchors.len() + built_pois_raw.len());
    for (nid, &[gx, gy]) in &sub.node_anchors {
        node_anchors.insert(nid.clone(), json!([gx, gy]));
    }
    for (pub_id, cell_id, label, gx, gy) in built_pois_raw {
        pois.push(json!({
            "id": pub_id,
            "cell_id": cell_id,
            "label": label,
            "tier": "built",
            "gx": gx,
            "gy": gy,
        }));
        node_anchors.insert(pub_id, json!([gx, gy]));
    }

    for band in delta.live_wilds.bands.values() {
        let [gx, gy] = sub
            .node_anchors
            .get(&band.camp_node_id)
            .copied()
            .or_else(|| {
                doc.cells
                    .get(&band.cell_id.to_string())
                    .map(|c| world_to_grid_f32(c.center[0], c.center[1]))
            })
            .unwrap_or([370.0, 410.0]);
        pois.push(json!({
            "id": band.camp_node_id,
            "cell_id": band.cell_id,
            "label": band.band_name_zh,
            "tier": "band",
            "gx": gx,
            "gy": gy,
        }));
        node_anchors
            .entry(band.camp_node_id.clone())
            .or_insert_with(|| json!([gx, gy]));
    }

    let mut cell_grid_centers = serde_json::Map::with_capacity(doc.cells.len());
    for c in doc.cells.values() {
        let [gx, gy] = world_to_grid_f32(c.center[0], c.center[1]);
        cell_grid_centers.insert(c.cell_id.to_string(), json!([gx, gy]));
    }

    json!({
        "grid_w": GRID_W,
        "grid_h": GRID_H,
        "wx0": WX0,
        "wy0": WY0,
        "scale": GRID_SCALE,
        "low_tide": low_tide,
        "camera_bounds": [1360.0, 628.0, 1506.0, 792.0],
        "terrain_rle_b64": terrain_b64,
        "overlay_rle_b64": overlay_b64,
        "player_pos": Value::Null,
        "pois": pois,
        "adjacent_exits": [],
        "node_anchors": node_anchors,
        "cell_grid_centers": cell_grid_centers,
    })
}

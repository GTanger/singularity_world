//! 全島單一無縫底盤之實時地貌疊加（`tile_diffs`／汛期斷橋／潮汐／新屋／售屋／斷牆）與 RLE Base64 壓縮封包。

use serde_json::{Value, json};

use super::cell_resources::is_low_tide_window;
use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    BLDG_NONE, BLDG_ROOF, BLDG_RUIN_WALL, CompiledIslandSubstrate, GRID_H, GRID_SCALE, GRID_W,
    ROAD_BRIDGE_OR_PIER, ROAD_NONE, ROAD_STONE_WALL, ROAD_TOWN_COBBLE, TERRAIN_LAND,
    TERRAIN_REEF_SAND, TERRAIN_RIVER, WX0, WY0, compiled_island_substrate, idx_of,
    world_to_grid_f32,
};
use super::live_land::house_is_ruined;
use super::live_land_tiles::effective_tile_state;
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

/// 計算全島地理外接矩形（與玩家所踏足區段無關，恆框住整座銀鷗外島）。
pub(crate) fn full_island_camera_bounds(doc: &WorldStaticDoc) -> [f64; 4] {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for cell in doc.cells.values() {
        for &[x, y] in &cell.polygon {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if min_x.is_finite() {
        [min_x - 3.0, min_y - 3.0, max_x + 3.0, max_y + 3.0]
    } else {
        [1360.0, 628.0, 1506.0, 792.0]
    }
}

/// 單一地格疊加水文漫溢、斷橋拆板與退潮露礁，並回傳寫入後之 `(terrain, road, bldg)`。
#[inline]
#[expect(
    clippy::too_many_arguments,
    reason = "hot-loop per-tile hydrology helper"
)]
pub(crate) fn apply_tile_hydrology_and_tide(
    sub: &CompiledIslandSubstrate,
    pos: usize,
    x: usize,
    y: usize,
    mut t: u8,
    mut r: u8,
    b: u8,
    flood_level: u8,
    bridge_down: bool,
    cell_reef: bool,
    low_tide: bool,
) -> (u8, u8, u8) {
    if flood_level >= 1
        && t == TERRAIN_LAND
        && b == BLDG_NONE
        && r != ROAD_STONE_WALL
        && r != ROAD_TOWN_COBBLE
    {
        let rad = isize::from(flood_level).min(2);
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
    if bridge_down && t == TERRAIN_RIVER && r == ROAD_BRIDGE_OR_PIER {
        r = ROAD_NONE;
    }
    if low_tide
        && t == TERRAIN_LAND
        && cell_reef
        && sub.sea_dist_by_cell_edge[pos] <= 2
        && b == BLDG_NONE
        && r == ROAD_NONE
    {
        t = TERRAIN_REEF_SAND;
    }
    (t, r, b)
}

/// 在已配置或未占用路旁空地落位自建屋宅屋頂並產生座標。
pub(crate) fn place_built_house_roof(
    sub: &CompiledIslandSubstrate,
    terrain: &[u8],
    overlay: &mut [u8],
    anchor_nid: &str,
) -> Option<[f32; 2]> {
    let &[ax, ay] = sub.node_anchors.get(anchor_nid)?;
    let gx0 = ax.floor() as isize;
    let gy0 = ay.floor() as isize;
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
                return Some([(gx0 + dx) as f32 + 0.5, (gy0 + dy) as f32 + 0.5]);
            }
        }
    }
    None
}

fn synthesize_live_substrate_buffers(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
) -> (Vec<u8>, Vec<u8>, Vec<(String, u32, String, f32, f32)>, bool) {
    let total = GRID_W * GRID_H;
    let low_tide = is_low_tide_window(clock);

    let mut cell_reef = vec![false; sub.cell_ids.len()];
    let mut cell_flood = vec![0u8; sub.cell_ids.len()];
    let mut cell_bridge_down = vec![false; sub.cell_ids.len()];

    for (idx, &cid) in sub.cell_ids.iter().enumerate() {
        if let Some(c) = doc.cells.get(&cid.to_string()) {
            cell_reef[idx] = c.is_coast && c.resource_slug == "shellfish_meat";
        }
        if let Some(cl) = delta.live_land.cells.get(&cid) {
            cell_flood[idx] = cl.flood_level;
            cell_bridge_down[idx] = cl.has_river_crossing && cl.bridge_integrity == 0;
        }
    }

    let mut terrain = vec![0u8; total];
    let mut overlay = vec![0u8; total];

    for y in 0..GRID_H {
        let row = y * GRID_W;
        for x in 0..GRID_W {
            let pos = row + x;
            let (bt, br, bb) = effective_tile_state(sub, &delta.tile_diffs, pos);
            let owner = sub.cell_owner[pos];
            if owner == u16::MAX {
                terrain[pos] = bt;
                overlay[pos] = br | (bb << 3);
                continue;
            }
            let oidx = owner as usize;
            let (t, r, b) = apply_tile_hydrology_and_tide(
                sub,
                pos,
                x,
                y,
                bt,
                br,
                bb,
                cell_flood[oidx],
                cell_bridge_down[oidx],
                cell_reef[oidx],
                low_tide,
            );
            terrain[pos] = t;
            overlay[pos] = r | (b << 3);
        }
    }

    for (hnid, cells) in &sub.house_footprint_cells {
        if house_is_ruined(delta, hnid) {
            for &pos in cells {
                let r = overlay[pos] & 0x07;
                overlay[pos] = r | (BLDG_RUIN_WALL << 3);
            }
        }
    }

    for sold_nid in delta.sold_houses.keys() {
        if let Some(cells) = sub.house_footprint_cells.get(sold_nid) {
            for &pos in cells {
                let r = overlay[pos] & 0x07;
                overlay[pos] = r | (BLDG_ROOF << 3);
            }
        }
    }

    let mut built_pois = Vec::new();
    for bh in delta.built_houses.values() {
        let Some(anchor_nd) = doc.nodes.get(&bh.anchor_node_id) else {
            continue;
        };
        if let Some([px, py]) =
            place_built_house_roof(sub, &terrain, &mut overlay, &bh.anchor_node_id)
        {
            let pub_id = if bh.house_id.contains("pl:") {
                format!("homestead:traveler:{}", bh.built_day)
            } else {
                bh.house_id.clone()
            };
            let label = mask_player_names_in_text(delta, &bh.label_zh);
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
        synthesize_live_substrate_buffers(doc, sub, delta, clock);

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
        "camera_bounds": full_island_camera_bounds(doc),
        "terrain_rle_b64": terrain_b64,
        "overlay_rle_b64": overlay_b64,
        "player_pos": Value::Null,
        "pois": pois,
        "adjacent_exits": [],
        "node_anchors": node_anchors,
        "cell_grid_centers": cell_grid_centers,
    })
}

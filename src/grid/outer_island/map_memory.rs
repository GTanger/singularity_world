//! 旅人親見記憶輿圖、傳聞問號批註、局部視距揭霧與沿已知節點逐跳記憶導航。
//!
//! 嚴守設計定案：
//! - 玩家端輿圖絕非全知衛星雷達：僅在親身抵達地塊時更新現場實況快照，其餘已探地塊只呈現個人親見記憶；
//! - 道聽塗說（`TavernHearsay`／`ClaimedBySpeaker`／秘聞）僅於地圖疊加獨立 `？` 傳聞批註，絕不偷改底圖地形；
//! - 擦邊路過 `:skirt_*`／`:approach_*` 僅揭開局部路徑迷霧並遮罩未入核心之聚落建築，踏入核心始揭開聚落；
//! - 點圖導航嚴格沿本人 `known_nodes` 逐跳邁步，遇高潮淹礁、疫氣封路、現場新斷橋或遭遇戰立即停步。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    CompiledIslandSubstrate, GRID_W, ROAD_BRIDGE_OR_PIER, TERRAIN_RIVER, compiled_island_substrate,
    idx_of, world_to_grid_f32,
};
use super::live_land::{house_is_ruined, is_bridge_washed_out, is_quarantine_restricted};
use super::movement::{apply_move_step, push_breath, shortest_walk_path};
use super::types::{IslandWorldDelta, TileDiffEntry, UnifiedMindBody, WorldStaticDoc};

pub use super::map_memory_pack::build_viewer_memory_substrate_json;

pub const MAX_RUMOR_PINS: usize = 16;
const MAX_MEMORY_TRAVEL_HOPS: usize = 18;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WitnessedCellSnap {
    pub flood_level: u8,
    pub bridge_down: bool,
    pub forest_pct: u8,
    pub stock_pct: u8,
    pub reclaimed_plots: u8,
    pub fallow_days: u16,
    pub plague_active: bool,
    pub core_visited: bool,
    pub witnessed_day: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessedHomesteadSnap {
    pub house_id: String,
    pub anchor_node_id: String,
    pub cell_id: u32,
    pub label_zh: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WitnessedBandSnap {
    pub cell_id: u32,
    pub camp_node_id: String,
    pub band_name_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MapRumorPin {
    pub pin_id: String,
    pub cell_id: u32,
    pub node_id: String,
    pub label_zh: String,
    pub gx: f32,
    pub gy: f32,
    pub heard_day: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ActorMapMemory {
    pub witnessed_cells: BTreeMap<u32, WitnessedCellSnap>,
    pub witnessed_ruined_houses: BTreeSet<String>,
    pub witnessed_sold_houses: BTreeSet<String>,
    pub witnessed_built_houses: BTreeMap<String, WitnessedHomesteadSnap>,
    pub witnessed_bands: BTreeMap<u32, WitnessedBandSnap>,
    pub rumor_pins: BTreeMap<String, MapRumorPin>,
    #[serde(default)]
    pub witnessed_tile_diffs: BTreeMap<u32, TileDiffEntry>,
    #[serde(default)]
    pub witnessed_cleared_tiles: BTreeSet<u32>,
}

/// 區分外圍擦邊節點（`:skirt_*`／`:approach_*`）與真正踏入地塊核心（城鎮廣場、門戶、聚落院落、主道、營寨、遺址）。
pub fn is_core_cell_node(doc: &WorldStaticDoc, node_id: &str) -> bool {
    if node_id.contains(":skirt_") || node_id.contains(":approach_") {
        return false;
    }
    doc.nodes.get(node_id).is_none_or(|nd| !nd.dungeon)
}

fn default_unvisited_cell_snap(
    doc: &WorldStaticDoc,
    cid: u32,
    core_visited: bool,
    game_day: i64,
) -> WitnessedCellSnap {
    let plague_base = doc
        .cells
        .get(&cid.to_string())
        .is_some_and(|c| c.plague_sealed || c.kind == "plague_ruin");
    WitnessedCellSnap {
        flood_level: 0,
        bridge_down: false,
        forest_pct: 100,
        stock_pct: 100,
        reclaimed_plots: 0,
        fallow_days: 0,
        plague_active: plague_base,
        core_visited,
        witnessed_day: game_day,
    }
}

fn core_anchor_for_cell(doc: &WorldStaticDoc, sub: &CompiledIslandSubstrate, cid: u32) -> [f32; 2] {
    for suffix in [":plaza", ":yard", ":market_plaza", ":0"] {
        let prefix = format!("oi:cell{cid}{suffix}");
        if let Some(&pos) = sub.node_anchors.get(&prefix) {
            return pos;
        }
    }
    if let Some(c) = doc.cells.get(&cid.to_string()) {
        for (nid, nd) in &doc.nodes {
            if nd.cell_id == cid
                && !nd.dungeon
                && is_core_cell_node(doc, nid)
                && matches!(nd.kind.as_str(), "plaza" | "yard" | "gate" | "village_gate")
                && let Some(&pos) = sub.node_anchors.get(nid)
            {
                return pos;
            }
        }
        return world_to_grid_f32(c.center[0], c.center[1]);
    }
    [430.0, 465.0]
}

pub(crate) fn live_cell_snap(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cid: u32,
    core_visited: bool,
    game_day: i64,
) -> WitnessedCellSnap {
    let mut stock_pct = 100u8;
    let mut plague_base = false;
    if let Some(c) = doc.cells.get(&cid.to_string()) {
        plague_base = c.plague_sealed || c.kind == "plague_ruin";
        if c.stock_cap > 0 {
            let cur = delta.cell_stocks.get(&cid).copied().unwrap_or(c.stock_cap);
            stock_pct = ((u32::from(cur) * 100) / u32::from(c.stock_cap)).min(100) as u8;
        }
    }
    if let Some(cl) = delta.live_land.cells.get(&cid) {
        let plague_active = if delta.live_land.seeded {
            cl.plague_miasmas >= 30
        } else {
            plague_base
        };
        WitnessedCellSnap {
            flood_level: cl.flood_level,
            bridge_down: cl.has_river_crossing && cl.bridge_integrity == 0,
            forest_pct: ((u32::from(cl.forest_density) * 100) / 85).min(100) as u8,
            stock_pct,
            reclaimed_plots: cl.reclaimed_plots,
            fallow_days: cl.fallow_days,
            plague_active,
            core_visited,
            witnessed_day: game_day,
        }
    } else {
        WitnessedCellSnap {
            flood_level: 0,
            bridge_down: false,
            forest_pct: 100,
            stock_pct,
            reclaimed_plots: 0,
            fallow_days: 0,
            plague_active: plague_base,
            core_visited,
            witnessed_day: game_day,
        }
    }
}

impl WitnessedCellSnap {
    pub(crate) fn same_terrain_state(&self, other: &Self) -> bool {
        self.flood_level == other.flood_level
            && self.bridge_down == other.bridge_down
            && self.forest_pct == other.forest_pct
            && self.stock_pct == other.stock_pct
            && self.reclaimed_plots == other.reclaimed_plots
            && self.fallow_days == other.fallow_days
            && self.plague_active == other.plague_active
            && self.core_visited == other.core_visited
    }
}

/// 在場親見寫入：僅於踏入非地城核心節點時更新該地塊核況；若為外圍擦邊節點則僅依實地視距 `18` 格局部見證。
pub fn record_local_witness_at_cur_node(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    actor_id: &str,
    game_day: i64,
) {
    let (cur_nid, is_player, prev_snap, has_cid_rumor, has_band_snap, sight_xy, pos_seeded) = {
        let Some(a) = delta.actors.get(actor_id) else {
            return;
        };
        let Some(cur_nd) = doc.nodes.get(&a.node_id) else {
            return;
        };
        if cur_nd.dungeon {
            return;
        }
        let cid = cur_nd.cell_id;
        let prev = a.mind.ltm.map_memory.witnessed_cells.get(&cid).cloned();
        let has_r = a
            .mind
            .ltm
            .map_memory
            .rumor_pins
            .values()
            .any(|p| p.cell_id == cid);
        let has_b = a.mind.ltm.map_memory.witnessed_bands.contains_key(&cid);
        let sub = compiled_island_substrate(doc);
        let sxy = if a.map_pos_seeded {
            [a.map_gx, a.map_gy]
        } else {
            sub.node_anchors
                .get(&a.node_id)
                .copied()
                .unwrap_or_else(|| world_to_grid_f32(cur_nd.pos[0], cur_nd.pos[1]))
        };
        (
            a.node_id.clone(),
            a.controller_id.is_some() || actor_id.starts_with("pl:"),
            prev,
            has_r,
            has_b,
            sxy,
            a.map_pos_seeded,
        )
    };
    let Some(cur_nd) = doc.nodes.get(&cur_nid) else {
        return;
    };
    let cid = cur_nd.cell_id;
    let was_core = prev_snap.as_ref().is_some_and(|s| s.core_visited);
    let at_core = is_core_cell_node(doc, &cur_nid);
    let core_visited = was_core || at_core;
    let sub = compiled_island_substrate(doc);
    let [cax, cay] = core_anchor_for_cell(doc, sub, cid);
    let far_from_core =
        pos_seeded && ((cax - sight_xy[0]).powi(2) + (cay - sight_xy[1]).powi(2) > 324.0);
    if !at_core || far_from_core {
        record_local_witness_at_tile(doc, delta, actor_id, sight_xy[0], sight_xy[1], game_day);
        return;
    }
    let mut saw_bridge_near = false;
    let mut saw_river_near = false;
    let cx = sight_xy[0].floor() as isize;
    let cy = sight_xy[1].floor() as isize;
    for dy in -18isize..=18 {
        for dx in -18isize..=18 {
            if dx * dx + dy * dy > 324 {
                continue;
            }
            if let Some(pos) = idx_of(cx + dx, cy + dy) {
                let owner = sub.cell_owner[pos];
                if owner != u16::MAX && sub.cell_ids[owner as usize] == cid {
                    if sub.base_terrain[pos] == TERRAIN_RIVER {
                        saw_river_near = true;
                        if sub.base_road[pos] == ROAD_BRIDGE_OR_PIER {
                            saw_bridge_near = true;
                        }
                    }
                }
            }
        }
    }
    let mut snap = live_cell_snap(doc, delta, cid, core_visited, game_day);
    if !saw_bridge_near {
        snap.bridge_down = prev_snap.as_ref().is_some_and(|p| p.bridge_down);
    }
    if !saw_river_near {
        snap.flood_level = prev_snap.as_ref().map_or(0, |p| p.flood_level);
    }
    let band_now_active = core_visited
        && delta.live_wilds.bands.get(&cid).is_some_and(|band| {
            game_day >= band.suppressed_until_day
                && (band.raid_pressure >= 60 || band.hoard_copper >= 8)
        });

    if !has_cid_rumor
        && has_band_snap == band_now_active
        && prev_snap
            .as_ref()
            .is_some_and(|p| p.same_terrain_state(&snap))
        && !is_player
    {
        return;
    }

    let mut ruined_in_cell = Vec::new();
    let mut clear_ruined_in_cell = Vec::new();
    let mut sold_in_cell = Vec::new();
    let mut clear_sold_in_cell = Vec::new();
    let mut built_in_cell = Vec::new();
    let mut active_band: Option<WitnessedBandSnap> = None;

    if core_visited {
        let sub = compiled_island_substrate(doc);
        if (is_player || !was_core)
            && doc
                .cells
                .get(&cid.to_string())
                .is_some_and(|c| matches!(c.kind.as_str(), "settlement" | "rural_settlement"))
        {
            for hnid in sub.house_footprint_cells.keys() {
                if doc.nodes.get(hnid).is_some_and(|h| h.cell_id == cid) {
                    if house_is_ruined(delta, hnid) {
                        ruined_in_cell.push(hnid.clone());
                    } else {
                        clear_ruined_in_cell.push(hnid.clone());
                    }
                    if delta.sold_houses.contains_key(hnid) {
                        sold_in_cell.push(hnid.clone());
                    } else {
                        clear_sold_in_cell.push(hnid.clone());
                    }
                }
            }
            for bh in delta.built_houses.values() {
                if doc
                    .nodes
                    .get(&bh.anchor_node_id)
                    .is_some_and(|and| and.cell_id == cid)
                {
                    built_in_cell.push(WitnessedHomesteadSnap {
                        house_id: bh.house_id.clone(),
                        anchor_node_id: bh.anchor_node_id.clone(),
                        cell_id: cid,
                        label_zh: bh.label_zh.clone(),
                    });
                }
            }
        }
        if let Some(band) = delta.live_wilds.bands.get(&cid)
            && game_day >= band.suppressed_until_day
            && (band.raid_pressure >= 60 || band.hoard_copper >= 8)
        {
            active_band = Some(WitnessedBandSnap {
                cell_id: cid,
                camp_node_id: band.camp_node_id.clone(),
                band_name_zh: band.band_name_zh.clone(),
            });
        }
    }

    let update_houses = core_visited && (is_player || !was_core);
    let Some(a) = delta.actors.get_mut(actor_id) else {
        return;
    };
    let secret_unlocked = a.unlocked_secrets.contains("oi:cell4529:0");
    let mem = &mut a.mind.ltm.map_memory;
    mem.witnessed_cells.insert(cid, snap);
    if update_houses {
        for h in ruined_in_cell {
            mem.witnessed_ruined_houses.insert(h);
        }
        for h in clear_ruined_in_cell {
            mem.witnessed_ruined_houses.remove(&h);
        }
        for h in sold_in_cell {
            mem.witnessed_sold_houses.insert(h);
        }
        for h in clear_sold_in_cell {
            mem.witnessed_sold_houses.remove(&h);
        }
        mem.witnessed_built_houses.retain(|_, v| v.cell_id != cid);
        for bh in built_in_cell {
            mem.witnessed_built_houses.insert(bh.house_id.clone(), bh);
        }
    }
    if core_visited {
        if let Some(wb) = active_band {
            mem.witnessed_bands.insert(cid, wb);
        } else {
            mem.witnessed_bands.remove(&cid);
        }
    }
    mem.rumor_pins.retain(|k, pin| {
        if pin.cell_id != cid {
            return true;
        }
        if k == "pit_rumor" {
            return !secret_unlocked;
        }
        !saw_bridge_near && pin.node_id != cur_nid
    });
    a.mark_mutated();
}

/// 依角色實地站立座標 `(gx, gy)` 半徑 `18` 格精準親見寫入：
/// 嚴禁僅因擦過 Voronoi 地塊邊界 `1` 格便將整塊未見之斷橋或洪災寫入記憶；
/// 唯獨實際在 `18` 格內目睹河格才記水勢、目睹橋格才記橋況、近抵聚落核心才記核況與屋宅，並同步 `witnessed_tile_diffs`。
pub fn record_local_witness_at_tile(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    actor_id: &str,
    gx: f32,
    gy: f32,
    game_day: i64,
) {
    let Some((cur_nid, is_player, secret_unlocked)) = delta.actors.get(actor_id).map(|a| {
        (
            a.node_id.clone(),
            a.controller_id.is_some() || actor_id.starts_with("pl:"),
            a.unlocked_secrets.contains("oi:cell4529:0"),
        )
    }) else {
        return;
    };
    if doc.nodes.get(&cur_nid).is_some_and(|n| n.dungeon) {
        return;
    }
    let sub = compiled_island_substrate(doc);
    let cx = gx.floor() as isize;
    let cy = gy.floor() as isize;
    let mut touched_cids = BTreeSet::new();
    let mut saw_bridge_cids = BTreeSet::new();
    let mut saw_river_cids = BTreeSet::new();
    for dy in -18isize..=18 {
        for dx in -18isize..=18 {
            if dx * dx + dy * dy > 324 {
                continue;
            }
            if let Some(pos) = idx_of(cx + dx, cy + dy) {
                let owner = sub.cell_owner[pos];
                if owner != u16::MAX {
                    let cid = sub.cell_ids[owner as usize];
                    touched_cids.insert(cid);
                    if sub.base_terrain[pos] == TERRAIN_RIVER {
                        saw_river_cids.insert(cid);
                        if sub.base_road[pos] == ROAD_BRIDGE_OR_PIER {
                            saw_bridge_cids.insert(cid);
                        }
                    }
                }
            }
        }
    }

    let cur_cid = doc.nodes.get(&cur_nid).map(|n| n.cell_id);
    let at_core = is_core_cell_node(doc, &cur_nid);
    let mut saw_core_cids = BTreeSet::new();
    for &cid in &touched_cids {
        if Some(cid) == cur_cid && !at_core {
            continue;
        }
        let [cax, cay] = core_anchor_for_cell(doc, sub, cid);
        if (cax - gx).powi(2) + (cay - gy).powi(2) <= 324.0 {
            saw_core_cids.insert(cid);
        }
    }

    let mut cell_snaps = Vec::new();
    let mut band_updates: Vec<(u32, Option<WitnessedBandSnap>)> = Vec::new();
    for &cid in &touched_cids {
        let saw_b = saw_bridge_cids.contains(&cid);
        let saw_r = saw_river_cids.contains(&cid);
        let saw_c = saw_core_cids.contains(&cid);
        if !saw_b && !saw_r && !saw_c {
            continue;
        }
        let prev_opt = delta
            .actors
            .get(actor_id)
            .and_then(|a| a.mind.ltm.map_memory.witnessed_cells.get(&cid).cloned());
        let core_visited = saw_c || prev_opt.as_ref().is_some_and(|s| s.core_visited);
        let live = live_cell_snap(doc, delta, cid, core_visited, game_day);
        let mut merged = prev_opt
            .unwrap_or_else(|| default_unvisited_cell_snap(doc, cid, core_visited, game_day));
        if saw_c {
            let keep_bridge = merged.bridge_down;
            let keep_flood = merged.flood_level;
            merged = live.clone();
            if !saw_b {
                merged.bridge_down = keep_bridge;
            }
            if !saw_r {
                merged.flood_level = keep_flood;
            }
        }
        if saw_r {
            merged.flood_level = live.flood_level;
            merged.witnessed_day = game_day;
        }
        if saw_b {
            merged.bridge_down = live.bridge_down;
            merged.witnessed_day = game_day;
        }
        cell_snaps.push((cid, merged));
        if core_visited && saw_c {
            let wb = delta
                .live_wilds
                .bands
                .get(&cid)
                .filter(|b| {
                    game_day >= b.suppressed_until_day
                        && (b.raid_pressure >= 60 || b.hoard_copper >= 8)
                })
                .map(|b| WitnessedBandSnap {
                    cell_id: cid,
                    camp_node_id: b.camp_node_id.clone(),
                    band_name_zh: b.band_name_zh.clone(),
                });
            band_updates.push((cid, wb));
        }
    }

    let mut add_diffs: Vec<(u32, TileDiffEntry)> = Vec::new();
    for (&pos_u32, &entry) in &delta.tile_diffs {
        let pos = pos_u32 as usize;
        let tx = (pos % GRID_W) as isize;
        let ty = (pos / GRID_W) as isize;
        if (tx - cx).pow(2) + (ty - cy).pow(2) <= 324 {
            add_diffs.push((pos_u32, entry));
        }
    }

    let mut changed = false;
    let Some(a) = delta.actors.get_mut(actor_id) else {
        return;
    };
    let mem = &mut a.mind.ltm.map_memory;
    for (cid, snap) in cell_snaps {
        let same = mem
            .witnessed_cells
            .get(&cid)
            .is_some_and(|p| p.same_terrain_state(&snap));
        if !same || is_player {
            mem.witnessed_cells.insert(cid, snap);
            changed = true;
        }
    }
    for (cid, wb_opt) in band_updates {
        if let Some(wb) = wb_opt {
            if mem.witnessed_bands.get(&cid) != Some(&wb) {
                mem.witnessed_bands.insert(cid, wb);
                changed = true;
            }
        } else if mem.witnessed_bands.remove(&cid).is_some() {
            changed = true;
        }
    }
    mem.witnessed_tile_diffs.retain(|&pos_u32, _| {
        let pos = pos_u32 as usize;
        let tx = (pos % GRID_W) as isize;
        let ty = (pos / GRID_W) as isize;
        let in_sight = (tx - cx).pow(2) + (ty - cy).pow(2) <= 324;
        if in_sight && !delta.tile_diffs.contains_key(&pos_u32) {
            changed = true;
            false
        } else {
            true
        }
    });
    for (pos_u32, entry) in add_diffs {
        if mem.witnessed_tile_diffs.get(&pos_u32) != Some(&entry) {
            mem.witnessed_tile_diffs.insert(pos_u32, entry);
            mem.witnessed_cleared_tiles.remove(&pos_u32);
            changed = true;
        }
    }
    let before_pins = mem.rumor_pins.len();
    mem.rumor_pins.retain(|k, pin| {
        if k == "pit_rumor" && !secret_unlocked {
            return true;
        }
        let d2 = (pin.gx - gx).powi(2) + (pin.gy - gy).powi(2);
        d2 > 144.0
    });
    if mem.rumor_pins.len() != before_pins {
        changed = true;
    }
    if changed {
        a.mark_mutated();
    }
}

/// 出生／遷入預設地景記憶播種：依據本人已知核心地塊播種初始親見快照，絕不偷開未訪區域。
pub fn seed_actor_initial_map_witness(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    actor_id: &str,
    game_day: i64,
) {
    let Some(home_cell) = delta
        .actors
        .get(actor_id)
        .and_then(|a| doc.nodes.get(&a.node_id).map(|n| n.cell_id))
    else {
        return;
    };
    let snap = live_cell_snap(doc, delta, home_cell, true, game_day);
    if let Some(a) = delta.actors.get_mut(actor_id) {
        a.mind
            .ltm
            .map_memory
            .witnessed_cells
            .insert(home_cell, snap);
    }
    record_local_witness_at_cur_node(doc, delta, actor_id, game_day);
}

fn infer_rumor_target(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    sub: &CompiledIslandSubstrate,
    event_id: &str,
    raw_text: &str,
) -> Option<(String, u32, String, String, [f32; 2])> {
    if event_id == "pit_rumor"
        || (raw_text.contains("灰松") && raw_text.contains("暗門"))
        || (raw_text.contains("試煉坑") && raw_text.contains("斷橋"))
    {
        let nid = "oi:cell4529:0".to_string();
        let pos = sub.node_anchors.get(&nid).copied()?;
        return Some((
            "pit_rumor".to_string(),
            4529,
            nid,
            "？傳聞：灰松斷橋石縫暗門".to_string(),
            pos,
        ));
    }

    let is_disaster_or_wild = raw_text.contains("斷橋")
        || raw_text.contains("橋毀")
        || raw_text.contains("淹水")
        || raw_text.contains("春汛")
        || raw_text.contains("洪水")
        || raw_text.contains("疫")
        || raw_text.contains("灰耳")
        || raw_text.contains("營寨")
        || raw_text.contains("劫")
        || raw_text.contains("流寇")
        || raw_text.contains("坍塌");
    if !is_disaster_or_wild {
        return None;
    }

    for band in delta.live_wilds.bands.values() {
        let short_band = band
            .band_name_zh
            .split('（')
            .next()
            .unwrap_or(&band.band_name_zh);
        if raw_text.contains(short_band)
            && let Some(&pos) = sub.node_anchors.get(&band.camp_node_id)
        {
            return Some((
                format!("rumor:band:{}", band.cell_id),
                band.cell_id,
                band.camp_node_id.clone(),
                format!("？傳聞：{}", band.band_name_zh),
                pos,
            ));
        }
    }

    let mut matched_cell: Option<u32> = None;
    if let Some(pos) = raw_text.find('#') {
        let digits: String = raw_text[pos + 1..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(cid) = digits.parse::<u32>()
            && doc.cells.contains_key(&cid.to_string())
        {
            matched_cell = Some(cid);
        }
    }
    if matched_cell.is_none() {
        for c in doc.cells.values() {
            let title_core = c.title.split('（').next().unwrap_or(&c.title);
            if title_core.chars().count() >= 2 && raw_text.contains(title_core) {
                matched_cell = Some(c.cell_id);
                break;
            }
        }
    }
    let cid = matched_cell?;
    let c = doc.cells.get(&cid.to_string())?;
    let title_core = c.title.split('（').next().unwrap_or(&c.title);
    let tag = if raw_text.contains("斷橋") || raw_text.contains("橋毀") {
        format!("？傳聞：{title_core}木橋沖斷")
    } else if raw_text.contains("疫") {
        format!("？傳聞：{title_core}疫氣封鎖")
    } else if raw_text.contains("流寇") || raw_text.contains("坍塌") {
        format!("？傳聞：{title_core}屋宅異變")
    } else {
        format!("？傳聞：{title_core}水勢異常")
    };
    let nid = format!("oi:cell{cid}:0");
    let anchor = sub
        .node_anchors
        .get(&nid)
        .copied()
        .unwrap_or_else(|| world_to_grid_f32(c.center[0], c.center[1]));
    Some((format!("rumor:cell:{cid}"), cid, nid, tag, anchor))
}

/// 寫入遠方聽聞之 `？傳聞` 地圖批註；聽者身處事發現場時不寫傳聞，且絕不更動底圖地形與親見快照。
pub fn ingest_hearsay_rumor_pin(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    listener_id: &str,
    event_id: &str,
    raw_text: &str,
    game_day: i64,
) {
    let sub = compiled_island_substrate(doc);
    let Some((pin_id, cid, nid, label_zh, pos)) =
        infer_rumor_target(doc, delta, sub, event_id, raw_text)
    else {
        return;
    };
    let Some(listener) = delta.actors.get_mut(listener_id) else {
        return;
    };
    let listener_cell = doc.nodes.get(&listener.node_id).map(|n| n.cell_id);
    if listener_cell == Some(cid) && pin_id != "pit_rumor" {
        return;
    }
    if pin_id == "pit_rumor" && listener.unlocked_secrets.contains("oi:cell4529:0") {
        return;
    }
    let pins = &mut listener.mind.ltm.map_memory.rumor_pins;
    if pins.len() >= MAX_RUMOR_PINS
        && !pins.contains_key(&pin_id)
        && let Some(oldest_k) = pins
            .iter()
            .min_by_key(|(_, p)| p.heard_day)
            .map(|(k, _)| k.clone())
    {
        pins.remove(&oldest_k);
    }
    pins.insert(
        pin_id.clone(),
        MapRumorPin {
            pin_id,
            cell_id: cid,
            node_id: nid,
            label_zh,
            gx: pos[0],
            gy: pos[1],
            heard_day: game_day,
        },
    );
    listener.mark_mutated();
}

pub fn actor_has_core_cell_access(
    doc: &WorldStaticDoc,
    viewer: &UnifiedMindBody,
    cid: u32,
) -> bool {
    if viewer
        .mind
        .ltm
        .map_memory
        .witnessed_cells
        .get(&cid)
        .is_some_and(|s| s.core_visited)
    {
        return true;
    }
    viewer.mind.ltm.known_nodes.iter().any(|nid| {
        doc.nodes
            .get(nid)
            .is_some_and(|nd| nd.cell_id == cid && !nd.dungeon && is_core_cell_node(doc, nid))
    })
}

/// 沿本人 `known_nodes` 逐跳推進之點圖記憶導航：單步鄰接直走，跨步僅經已親見節點並遇險／遇阻／見變即停。
pub fn apply_memory_map_travel(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    target_node: &str,
    now_ms: u64,
) -> Result<String, String> {
    let Some(aid) = delta.resolve_actor_id(actor_key) else {
        return Err("請先向港務官核對通行身分。".to_string());
    };
    let (cur_node_id, known_nodes, mem_snap, last_ms) = {
        let Some(a) = delta.actors.get(&aid) else {
            return Err("請先向港務官核對通行身分。".to_string());
        };
        if !a.setup_done {
            return Err("請先在港務登岸木冊留下你的名號與來歷。".to_string());
        }
        if a.active_skirmish.is_some() {
            return Err("眼前正與敵手短兵對峙，請先出招應對或虛晃退開！".to_string());
        }
        if a.active_transit.is_some() {
            return Err("你正坐在車船座艙內趕路，若要改走山徑請先拍板中途下車。".to_string());
        }
        (
            a.node_id.clone(),
            a.mind.ltm.known_nodes.clone(),
            a.mind.ltm.map_memory.clone(),
            a.last_step_unix_ms,
        )
    };
    if cur_node_id == target_node {
        return Ok("你正站在此處，四下風物已在眼前。".to_string());
    }
    let cur_nd = doc
        .nodes
        .get(&cur_node_id)
        .ok_or("四周霧氣迷濛，暫時辨不清方位。")?;
    let is_direct = cur_nd.exits.iter().any(|e| e.to == target_node)
        || doc
            .hidden_entrances
            .get(&cur_node_id)
            .is_some_and(|h| h.target == target_node);
    if is_direct {
        return apply_move_step(doc, delta, clock, actor_key, target_node, now_ms);
    }

    if !known_nodes.contains(target_node) {
        return Err("前方尚未親身踏足，只能一步一步摸索前進。".to_string());
    }
    if now_ms > 0 && last_ms > 0 && now_ms.saturating_sub(last_ms) < 180 {
        return Err("腳步太急，先穩住氣息再邁步。".to_string());
    }
    let path = shortest_walk_path(doc, &cur_node_id, target_node, &known_nodes)
        .ok_or("依目前記憶中的熟路找不到可直抵彼處之連通道。".to_string())?;
    let hops = &path[1..];
    if hops.is_empty() {
        return Ok("你正站在此處。".to_string());
    }
    if hops.len() > MAX_MEMORY_TRAVEL_HOPS {
        return Err("路途遙遠，請先點選中繼熟路再繼續趕往。".to_string());
    }

    let mut walked = 0usize;
    let mut stop_reason: Option<String> = None;
    for next_nid in hops {
        let next_cid = doc.nodes.get(next_nid).map_or(0, |n| n.cell_id);
        let prev_mem = mem_snap.witnessed_cells.get(&next_cid).cloned();
        let was_ruined = mem_snap.witnessed_ruined_houses.contains(next_nid);

        match apply_move_step(doc, delta, clock, actor_key, next_nid, 0) {
            Ok(_) => walked += 1,
            Err(block_err) => {
                if walked == 0 {
                    return Err(block_err);
                }
                if let Some(a) = delta.actors.get_mut(&aid) {
                    push_breath(a, format!("中途去路受阻，只好就地停步：{block_err}"));
                }
                stop_reason = Some(format!("沿熟路趕行 {walked} 段後受阻停步：{block_err}"));
                break;
            }
        }

        if delta
            .actors
            .get(&aid)
            .is_some_and(|a| a.active_skirmish.is_some())
        {
            stop_reason = Some("半途驟生變故，你立即收住腳步警戒應敵！".to_string());
            break;
        }
        let bridge_surprise = prev_mem.as_ref().is_some_and(|m| !m.bridge_down)
            && is_bridge_washed_out(delta, next_cid);
        let miasma_surprise = prev_mem.as_ref().is_some_and(|m| !m.plague_active)
            && is_quarantine_restricted(doc, delta, next_cid);
        let house_surprise = !was_ruined && house_is_ruined(delta, next_nid);
        if bridge_surprise || miasma_surprise || house_surprise {
            let cue = if bridge_surprise {
                "抵達岸口方見木橋已被暴水沖斷，你連忙在橋頭勒住腳步！"
            } else if miasma_surprise {
                "臨近路段才察覺腐濁疫氣驟起，你當即停步不敢輕入！"
            } else {
                "來到門前卻見梁柱早已傾頹成斷牆，你駐足仔細端詳！"
            };
            if let Some(a) = delta.actors.get_mut(&aid) {
                push_breath(a, cue);
            }
            stop_reason = Some(cue.to_string());
            break;
        }
        if delta.actors.get(&aid).is_some_and(|a| a.fatigue >= 88) {
            stop_reason = Some("氣息已近力竭，你在半途路旁駐足歇息。".to_string());
            break;
        }
    }

    if let Some(a) = delta.actors.get_mut(&aid) {
        if now_ms > 0 {
            a.last_step_unix_ms = now_ms;
            if a.controller_id.is_some() {
                a.last_active_unix = clock.real_unix;
            }
        }
        a.mark_mutated();
    }

    if let Some(reason) = stop_reason {
        Ok(reason)
    } else {
        let dest_name = doc
            .nodes
            .get(target_node)
            .map_or(target_node, |n| n.name.as_str());
        Ok(format!(
            "你沿記在心裡的熟路穩步前行，已抵達【{dest_name}】。"
        ))
    }
}

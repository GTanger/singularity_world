//! 全島 `860×930` 門前出入、NPC 逐格漫步推進與玩家點擊／節拍移動控制器。

use super::clock_and_lang::SolarClockSnapshot;
use super::island_substrate::{
    BLDG_NONE, CompiledIslandSubstrate, GRID_H, GRID_W, ROAD_BRIDGE_OR_PIER, ROAD_NONE,
    ROAD_STONE_WALL, TERRAIN_MOUNTAIN, TERRAIN_REEF_SAND, TERRAIN_RIVER, TERRAIN_SEA,
    compiled_island_substrate, idx_of, world_to_grid,
};
use super::live_land::{is_bridge_washed_out, is_quarantine_restricted};
use super::live_land_tiles::effective_tile_state;
use super::live_tide::{derive_tide_clock_details, relocate_actor_at_xy_with_local_witness};
use super::map_memory::{record_local_witness_at_cur_node, record_local_witness_at_tile};
pub use super::map_walk_plan::{
    cell_id_at_tile, ensure_single_actor_grid_seeded, is_tile_walkable, plan_grid_walk_hop,
    plan_grid_walk_path, seed_missing_actor_grid_positions_once,
};
use super::mesh_expedition::is_high_tide_slot;
use super::movement::{apply_move_step, push_breath};
use super::types::{IslandWorldDelta, SpatialNodeDef, WorldStaticDoc};

fn nearest_outdoor_node_in_tile(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    gx: isize,
    gy: isize,
    fallback_nid: &str,
) -> String {
    let px = gx as f32 + 0.5;
    let py = gy as f32 + 0.5;
    if let Some(pos) = idx_of(gx, gy) {
        let owner = sub.cell_owner[pos];
        if owner != u16::MAX
            && let Some(cands) = sub.outdoor_nodes_by_cell.get(owner as usize)
            && !cands.is_empty()
        {
            let mut best: Option<(f32, &String)> = None;
            for (nid, [ax, ay]) in cands {
                let d2 = (*ax - px).powi(2) + (*ay - py).powi(2);
                let hazard_or_skirt = nid.ends_with(":camp")
                    || nid.contains(":skirt_")
                    || nid.contains(":approach_")
                    || nid.contains(":ruin");
                if hazard_or_skirt && d2 > 12.25 {
                    continue;
                }
                if best.is_none_or(|(bd, _)| d2 < bd) {
                    best = Some((d2, nid));
                }
            }
            if best.is_none() {
                for (nid, [ax, ay]) in cands {
                    let d2 = (*ax - px).powi(2) + (*ay - py).powi(2);
                    if best.is_none_or(|(bd, _)| d2 < bd) {
                        best = Some((d2, nid));
                    }
                }
            }
            if let Some((_, nid)) = best {
                return nid.clone();
            }
        }
    }
    let cid = cell_id_at_tile(sub, gx, gy);
    let mut best_in_cell: Option<(f32, &String)> = None;
    let mut best_any: Option<(f32, &String)> = None;
    for (nid, nd) in &doc.nodes {
        if nd.dungeon || nd.indoor {
            continue;
        }
        let Some(&[ax, ay]) = sub.node_anchors.get(nid) else {
            continue;
        };
        let d2 = (ax - px).powi(2) + (ay - py).powi(2);
        if Some(nd.cell_id) == cid && best_in_cell.is_none_or(|(bd, _)| d2 < bd) {
            best_in_cell = Some((d2, nid));
        }
        if best_any.is_none_or(|(bd, _)| d2 < bd) {
            best_any = Some((d2, nid));
        }
    }
    best_in_cell
        .or(best_any)
        .map_or_else(|| fallback_nid.to_string(), |(_, id)| id.clone())
}

fn exit_indoor_to_door(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) {
    let Some(cur_nid) = delta.actors.get(aid).map(|a| a.node_id.clone()) else {
        return;
    };
    if !doc.nodes.get(&cur_nid).is_some_and(|n| n.indoor) {
        return;
    }
    let Some(&[dx, dy]) = sub.door_tiles.get(&cur_nid) else {
        return;
    };
    let out_nid = nearest_outdoor_node_in_tile(doc, sub, dx as isize, dy as isize, &cur_nid);
    let out_cid = doc.nodes.get(&out_nid).map_or(0, |n| n.cell_id);
    let door_xy = [dx as f32 + 0.5, dy as f32 + 0.5];
    relocate_actor_at_xy_with_local_witness(
        doc,
        delta,
        clock,
        aid,
        &cur_nid,
        &out_nid,
        out_cid,
        false,
        Some(door_xy),
    );
    record_local_witness_at_tile(doc, delta, aid, door_xy[0], door_xy[1], clock.game_days);
}

#[expect(
    clippy::too_many_arguments,
    reason = "single-pass step validation returning next node id"
)]
fn check_step_blocked_reason(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    high_tide: bool,
    aid: &str,
    nx: isize,
    ny: isize,
) -> Result<String, String> {
    let Some(pos) = idx_of(nx, ny) else {
        return Err("前方已是海領界限，無法再邁出半步。".to_string());
    };
    let (t, rd, bg) = effective_tile_state(sub, &delta.tile_diffs, pos);
    if bg != BLDG_NONE || rd == ROAD_STONE_WALL || t == TERRAIN_MOUNTAIN {
        return Err("前方山壁水勢或牆垣阻隔，無從通行。".to_string());
    }
    if t == TERRAIN_SEA && rd != ROAD_BRIDGE_OR_PIER {
        return Err("前方山壁水勢或牆垣阻隔，無從通行。".to_string());
    }
    if t == TERRAIN_RIVER {
        if rd != ROAD_BRIDGE_OR_PIER {
            return Err("前方山壁水勢或牆垣阻隔，無從通行。".to_string());
        }
        if let Some(cid) = cell_id_at_tile(sub, nx, ny)
            && is_bridge_washed_out(delta, cid)
        {
            return Err("抵達岸口方見木橋已被暴水沖斷，你連忙在橋頭勒住腳步！".to_string());
        }
    }
    if t == TERRAIN_REEF_SAND && rd == ROAD_NONE && high_tide {
        return Err("潮浪湧上淺灘，礁路已被淹沒，只好先退回高岸！".to_string());
    }
    if rd == ROAD_NONE && sub.cell_owner[pos] == u16::MAX {
        return Err("前方已是海領界限，無法再邁出半步。".to_string());
    }
    let cur_nid = delta.actors.get(aid).map_or("", |a| a.node_id.as_str());
    let next_nid = nearest_outdoor_node_in_tile(doc, sub, nx, ny, cur_nid);
    let mut miasma_surprise_cid: Option<u32> = None;
    let mut cargo_err_opt: Option<String> = None;
    let actor_snapshot_ok = match (delta.actors.get(aid), doc.nodes.get(&next_nid)) {
        (Some(a), Some(dst_nd)) => {
            if let Some(src_nd) = doc.nodes.get(&a.node_id)
                && src_nd.cell_id != dst_nd.cell_id
            {
                if a.mind
                    .ltm
                    .map_memory
                    .witnessed_cells
                    .get(&dst_nd.cell_id)
                    .is_some_and(|m| !m.plague_active)
                    && is_quarantine_restricted(doc, delta, dst_nd.cell_id)
                {
                    miasma_surprise_cid = Some(dst_nd.cell_id);
                } else if let Err(cargo_err) = super::transit_and_cargo::check_bulk_overland_move(
                    doc, delta, a, src_nd, dst_nd,
                ) {
                    cargo_err_opt = Some(cargo_err);
                }
            }
            true
        }
        _ => false,
    };
    if let Some(mcid) = miasma_surprise_cid {
        if let Some(a) = delta.actors.get_mut(aid) {
            a.mind
                .ltm
                .map_memory
                .witnessed_cells
                .entry(mcid)
                .or_default()
                .plague_active = true;
            a.mark_mutated();
        }
        return Err("臨近路段才察覺腐濁疫氣驟起，你當即停步不敢輕入！".to_string());
    }
    if let Some(cerr) = cargo_err_opt {
        return Err(cerr);
    }
    if !actor_snapshot_ok {
        return Err("前方路徑未明，暫且收步。".to_string());
    }
    Ok(next_nid)
}

fn advance_one_grid_waypoint(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    high_tide: bool,
    aid: &str,
) -> Result<bool, String> {
    let Some([nx_u16, ny_u16]) = delta
        .actors
        .get_mut(aid)
        .and_then(|a| a.walk_waypoints.pop_front())
    else {
        return Ok(false);
    };
    let nx = nx_u16 as isize;
    let ny = ny_u16 as isize;
    let next_nid = match check_step_blocked_reason(doc, sub, delta, high_tide, aid, nx, ny) {
        Ok(nid) => nid,
        Err(reason) => {
            let (gx, gy) = delta
                .actors
                .get(aid)
                .map_or((nx as f32 + 0.5, ny as f32 + 0.5), |a| (a.map_gx, a.map_gy));
            if let Some(a) = delta.actors.get_mut(aid) {
                a.walk_waypoints.clear();
                a.walk_goal_enter_node = None;
                a.route_plan.clear();
                push_breath(a, reason.clone());
                a.mark_mutated();
            }
            record_local_witness_at_tile(doc, delta, aid, gx, gy, clock.game_days);
            return Err(reason);
        }
    };

    let prev_nid = delta
        .actors
        .get(aid)
        .map(|a| a.node_id.clone())
        .unwrap_or_default();
    let new_gx = nx as f32 + 0.5;
    let new_gy = ny as f32 + 0.5;
    if next_nid != prev_nid
        && let Some(nd) = doc.nodes.get(&next_nid)
    {
        let knows_state = matches!(nd.kind.as_str(), "plaza" | "gate" | "village_gate");
        relocate_actor_at_xy_with_local_witness(
            doc,
            delta,
            clock,
            aid,
            &prev_nid,
            &next_nid,
            nd.cell_id,
            knows_state,
            Some([new_gx, new_gy]),
        );
    }
    let mut is_player = aid.starts_with("pl:");
    if let Some(a) = delta.actors.get_mut(aid) {
        is_player = is_player || a.controller_id.is_some();
        a.map_gx = new_gx;
        a.map_gy = new_gy;
        a.map_pos_seeded = true;
        a.mark_mutated();
    }
    if is_player {
        record_local_witness_at_tile(doc, delta, aid, new_gx, new_gy, clock.game_days);
    }
    Ok(true)
}

fn try_enter_goal_if_arrived(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Option<Result<String, String>> {
    let (enter_nid, cur_nid, gx, gy, remaining) = {
        let a = delta.actors.get(aid)?;
        (
            a.walk_goal_enter_node.clone()?,
            a.node_id.clone(),
            a.map_gx,
            a.map_gy,
            a.walk_waypoints.len(),
        )
    };
    if remaining > 0 {
        return None;
    }
    if cur_nid == enter_nid {
        if let Some(a) = delta.actors.get_mut(aid) {
            a.walk_goal_enter_node = None;
        }
        return None;
    }
    let door = sub
        .door_tiles
        .get(&enter_nid)
        .map(|&[dx, dy]| [dx as f32 + 0.5, dy as f32 + 0.5])
        .or_else(|| sub.node_anchors.get(&enter_nid).copied())?;
    let d2 = (door[0] - gx).powi(2) + (door[1] - gy).powi(2);
    if d2 > 25.0 {
        return None;
    }
    if let Some(a) = delta.actors.get_mut(aid) {
        a.walk_goal_enter_node = None;
    }
    let dst_nd = doc.nodes.get(&enter_nid)?;
    if dst_nd.dungeon || doc.hidden_entrances.contains_key(&cur_nid) {
        Some(apply_move_step(doc, delta, clock, aid, &enter_nid, 0))
    } else if dst_nd.indoor {
        let (cur_gx, cur_gy) = delta
            .actors
            .get(aid)
            .map_or((door[0], door[1]), |a| (a.map_gx, a.map_gy));
        relocate_actor_at_xy_with_local_witness(
            doc,
            delta,
            clock,
            aid,
            &cur_nid,
            &enter_nid,
            dst_nd.cell_id,
            true,
            Some([cur_gx, cur_gy]),
        );
        let name = dst_nd.name.clone();
        Some(Ok(format!("你推門走入【{name}】。")))
    } else {
        let (cur_gx, cur_gy) = delta
            .actors
            .get(aid)
            .map_or((door[0], door[1]), |a| (a.map_gx, a.map_gy));
        let knows_state = matches!(dst_nd.kind.as_str(), "plaza" | "gate" | "village_gate");
        relocate_actor_at_xy_with_local_witness(
            doc,
            delta,
            clock,
            aid,
            &cur_nid,
            &enter_nid,
            dst_nd.cell_id,
            knows_state,
            Some([cur_gx, cur_gy]),
        );
        let name = dst_nd.name.clone();
        Some(Ok(format!("你沿路穩步前行，已抵達【{name}】。")))
    }
}

/// NPC 全員共構逐格推進：將 `route_plan` 逐跳轉化為連續 `walk_waypoints` 並每拍推進至多 `max_tiles` 格。
pub fn step_npc_grid_walk_tick(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
    max_tiles: usize,
) {
    let sub = compiled_island_substrate(doc);
    ensure_single_actor_grid_seeded(doc, sub, delta, aid);
    if delta.actors.get(aid).is_none_or(|a| {
        a.active_skirmish.is_some()
            || a.active_transit.is_some()
            || super::live_tide::is_actor_online_protected(a, clock.real_unix)
    }) {
        return;
    }

    let high_tide = is_high_tide_slot(derive_tide_clock_details(clock).tide_slot);
    let mut budget = max_tiles;
    let mut hop_guard = 0usize;
    while budget > 0 && hop_guard < 16 {
        hop_guard += 1;
        let (cur_nid, has_wp, next_route_opt) = {
            let Some(a) = delta.actors.get(aid) else {
                return;
            };
            (
                a.node_id.clone(),
                !a.walk_waypoints.is_empty(),
                a.route_plan.front().cloned(),
            )
        };
        if !has_wp {
            let Some(next_nid) = next_route_opt else {
                break;
            };
            let cur_nd = doc.nodes.get(&cur_nid);
            let dst_nd = doc.nodes.get(&next_nid);
            let dungeon_step = cur_nd.is_some_and(|n| n.dungeon)
                || dst_nd.is_some_and(|n| n.dungeon)
                || doc.hidden_entrances.contains_key(&cur_nid);
            let both_indoor = cur_nd.is_some_and(|n| n.indoor) && dst_nd.is_some_and(|n| n.indoor);
            if dungeon_step || both_indoor {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.route_plan.pop_front();
                }
                let _ = apply_move_step(doc, delta, clock, aid, &next_nid, 0);
                budget = budget.saturating_sub(1);
                continue;
            }
            if let (Some(a_ro), Some(cnd), Some(fnd)) = (delta.actors.get(aid), cur_nd, dst_nd)
                && super::transit_and_cargo::check_bulk_overland_move(doc, delta, a_ro, cnd, fnd)
                    .is_err()
            {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.route_plan.clear();
                    a.mark_mutated();
                }
                break;
            }
            exit_indoor_to_door(doc, sub, delta, clock, aid);
            let enter_door = dst_nd.is_some_and(|n| n.indoor || n.id == "oi:dng_pit:entry");
            if let Some(a) = delta.actors.get_mut(aid) {
                a.walk_goal_enter_node = Some(next_nid.clone());
            }
            let goal_xy = if enter_door {
                sub.door_tiles
                    .get(&next_nid)
                    .map(|&[dx, dy]| (dx as isize, dy as isize))
                    .or_else(|| {
                        sub.node_anchors
                            .get(&next_nid)
                            .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
                    })
            } else {
                sub.node_anchors
                    .get(&next_nid)
                    .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
            };
            let is_final_hop = delta
                .actors
                .get(aid)
                .is_some_and(|a| a.route_plan.len() <= 1);
            let planned = if let (Some(gxy), Some(a_ro)) = (goal_xy, delta.actors.get(aid)) {
                let start = (a_ro.map_gx.floor() as isize, a_ro.map_gy.floor() as isize);
                plan_grid_walk_hop(
                    doc,
                    sub,
                    delta,
                    clock,
                    Some(&a_ro.mind.ltm.map_memory),
                    aid,
                    start,
                    gxy,
                    !enter_door && is_final_hop,
                )
            } else {
                None
            };
            if let Some(wps) = planned {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.route_plan.pop_front();
                    a.walk_waypoints.clear();
                    a.walk_waypoints.extend(wps);
                    a.mark_mutated();
                }
                if delta
                    .actors
                    .get(aid)
                    .is_some_and(|a| a.walk_waypoints.is_empty())
                {
                    let _ = try_enter_goal_if_arrived(doc, sub, delta, clock, aid);
                    continue;
                }
            } else {
                if let Some(a) = delta.actors.get_mut(aid) {
                    a.route_plan.clear();
                    a.walk_goal_enter_node = None;
                }
                break;
            }
        }

        while budget > 0 {
            match advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, aid) {
                Ok(true) => budget -= 1,
                Ok(false) => break,
                Err(_) => {
                    budget = 0;
                    break;
                }
            }
        }
        let _ = try_enter_goal_if_arrived(doc, sub, delta, clock, aid);
    }
}

fn find_clicked_building_goal(
    doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    delta: &IslandWorldDelta,
    gx: f32,
    gy: f32,
) -> Option<(String, (isize, isize))> {
    let igx = gx.floor() as isize;
    let igy = gy.floor() as isize;
    if igx >= 0 && igy >= 0 {
        let uxy = [igx as u16, igy as u16];
        for rec in delta.place_registry.values() {
            if !rec.ruined && rec.footprint_tiles.contains(&uxy) {
                return Some((
                    rec.anchor_node_id.clone(),
                    (rec.door_tile[0] as isize, rec.door_tile[1] as isize),
                ));
            }
        }
    }
    if let Some(pos) = idx_of(igx, igy) {
        for (nid, fp) in &sub.house_footprint_cells {
            if fp.contains(&pos)
                && let Some(&[dx, dy]) = sub.door_tiles.get(nid)
            {
                return Some((nid.clone(), (dx as isize, dy as isize)));
            }
        }
    }
    let mut best: Option<(f32, String, (isize, isize))> = None;
    for (nid, &[dx, dy]) in &sub.door_tiles {
        if !doc
            .nodes
            .get(nid)
            .is_some_and(|n| n.indoor || n.id == "oi:dng_pit:entry")
        {
            continue;
        }
        let dist2 = (dx as f32 + 0.5 - gx).powi(2) + (dy as f32 + 0.5 - gy).powi(2);
        if dist2 <= 2.6 && best.as_ref().is_none_or(|(bd, _, _)| dist2 < *bd) {
            best = Some((dist2, nid.clone(), (dx as isize, dy as isize)));
        }
    }
    best.map(|(_, nid, dxy)| (nid, dxy))
}

fn node_target_xy(
    _doc: &WorldStaticDoc,
    sub: &CompiledIslandSubstrate,
    nd: &SpatialNodeDef,
) -> (isize, isize) {
    if (nd.indoor || nd.id == "oi:dng_pit:entry")
        && let Some(&[dx, dy]) = sub.door_tiles.get(&nd.id)
    {
        return (dx as isize, dy as isize);
    }
    sub.node_anchors
        .get(&nd.id)
        .map(|&[ax, ay]| (ax.floor() as isize, ay.floor() as isize))
        .unwrap_or_else(|| world_to_grid(nd.pos[0], nd.pos[1]))
}

/// 處理玩家移動請求：支援畫布點擊連續規劃（`plan`）、單飛節拍推進（`step`）、立即停步（`stop`）與舊測試相容逐格走完。
pub fn handle_player_move_request(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    target_node: &str,
    gx_opt: Option<f32>,
    gy_opt: Option<f32>,
    mode: &str,
    steps: Option<u8>,
    now_ms: u64,
) -> Result<String, String> {
    seed_missing_actor_grid_positions_once(doc, delta);
    let Some(aid) = delta.resolve_actor_id(actor_key) else {
        return Err("請先向港務官核對通行身分。".to_string());
    };
    {
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
    }

    if mode == "stop" {
        if let Some(a) = delta.actors.get_mut(&aid) {
            a.walk_waypoints.clear();
            a.walk_goal_enter_node = None;
            a.route_plan.clear();
            a.mark_mutated();
        }
        return Ok("你收住腳步，就地停在路旁。".to_string());
    }

    let high_tide = is_high_tide_slot(derive_tide_clock_details(clock).tide_slot);
    let sub = compiled_island_substrate(doc);
    if mode == "step" {
        let count = usize::from(steps.unwrap_or(2).clamp(1, 4));
        for _ in 0..count {
            match advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, &aid) {
                Ok(true) => {}
                Ok(false) => break,
                Err(block_msg) => return Err(block_msg),
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
        if let Some(enter_res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
            return enter_res;
        }
        let rem = delta.actors.get(&aid).map_or(0, |a| a.walk_waypoints.len());
        return Ok(if rem == 0 {
            "你穩步走抵目的地。".to_string()
        } else {
            "你沿著路跡邁步前行。".to_string()
        });
    }

    let cur_nid = delta
        .actors
        .get(&aid)
        .map(|a| a.node_id.clone())
        .unwrap_or_default();
    let cur_nd = doc
        .nodes
        .get(&cur_nid)
        .ok_or("四周霧氣迷濛，暫時辨不清方位。")?;

    if let (Some(cgx), Some(cgy)) = (gx_opt, gy_opt) {
        let (ax, ay, walking) = delta.actors.get(&aid).map_or((cgx, cgy, false), |a| {
            (a.map_gx, a.map_gy, !a.walk_waypoints.is_empty())
        });
        if walking && (ax - cgx).powi(2) + (ay - cgy).powi(2) < 0.81 {
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.walk_waypoints.clear();
                a.walk_goal_enter_node = None;
                a.route_plan.clear();
                a.mark_mutated();
            }
            return Ok("你收住腳步，停在原地。".to_string());
        }
        let bldg_goal = find_clicked_building_goal(doc, sub, delta, cgx, cgy);
        if let Some((enter_nid, door_xy)) = bldg_goal {
            exit_indoor_to_door(doc, sub, delta, clock, &aid);
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.walk_goal_enter_node = Some(enter_nid);
            }
            let (sx, sy) = {
                let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤。")?;
                (a.map_gx.floor() as isize, a.map_gy.floor() as isize)
            };
            let wps = plan_grid_walk_path(
                doc,
                sub,
                delta,
                clock,
                delta.actors.get(&aid).map(|a| &a.mind.ltm.map_memory),
                &aid,
                (sx, sy),
                door_xy,
            )
            .ok_or("前往該屋門前的道路受海峽或峭壁阻隔，無法步行抵達。")?;
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.walk_waypoints.clear();
                a.walk_waypoints.extend(wps);
                a.route_plan.clear();
            }
            if let Some(res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
                return res;
            }
            let _ = advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, &aid)?;
            if let Some(res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
                return res;
            }
            return Ok("你邁步朝該處屋門走去。".to_string());
        }

        let goal_pt = (
            (cgx.floor() as isize).clamp(0, GRID_W as isize - 1),
            (cgy.floor() as isize).clamp(0, GRID_H as isize - 1),
        );
        if let Some(pos) = idx_of(goal_pt.0, goal_pt.1)
            && effective_tile_state(sub, &delta.tile_diffs, pos).0 == TERRAIN_SEA
            && effective_tile_state(sub, &delta.tile_diffs, pos).1 != ROAD_BRIDGE_OR_PIER
        {
            return Err("眼前是深藍海浪，沒有船隻或棧橋難以涉水過去。".to_string());
        }
        exit_indoor_to_door(doc, sub, delta, clock, &aid);
        let (sx, sy) = {
            let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤。")?;
            (a.map_gx.floor() as isize, a.map_gy.floor() as isize)
        };
        let wps = plan_grid_walk_path(
            doc,
            sub,
            delta,
            clock,
            delta.actors.get(&aid).map(|a| &a.mind.ltm.map_memory),
            &aid,
            (sx, sy),
            goal_pt,
        )
        .ok_or("該處與腳下之間被海灣、絕壁或城牆完全隔開，步行無法抵達。")?;
        if let Some(a) = delta.actors.get_mut(&aid) {
            a.walk_waypoints.clear();
            a.walk_waypoints.extend(wps);
            a.walk_goal_enter_node = None;
            a.route_plan.clear();
            if now_ms > 0 {
                a.last_step_unix_ms = now_ms;
                if a.controller_id.is_some() {
                    a.last_active_unix = clock.real_unix;
                }
            }
            a.mark_mutated();
        }
        let _ = advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, &aid)?;
        return Ok("你順著地勢邁出腳步，朝所指方位走去。".to_string());
    }

    let dst_nd = doc
        .nodes
        .get(target_node)
        .ok_or("前方霧氣深鎖，無路可通。")?;
    if cur_nd.dungeon
        || dst_nd.dungeon
        || (cur_nd.indoor && dst_nd.indoor)
        || doc
            .hidden_entrances
            .get(&cur_nid)
            .is_some_and(|h| h.target == target_node)
    {
        let msg = apply_move_step(doc, delta, clock, &aid, target_node, now_ms)?;
        record_local_witness_at_cur_node(doc, delta, &aid, clock.game_days);
        return Ok(msg);
    }

    let sync_legacy = mode.is_empty();
    exit_indoor_to_door(doc, sub, delta, clock, &aid);
    let goal_xy = node_target_xy(doc, sub, dst_nd);
    let (sx, sy) = {
        let a = delta.actors.get(&aid).ok_or("找不到旅人行蹤。")?;
        (a.map_gx.floor() as isize, a.map_gy.floor() as isize)
    };
    let wps = plan_grid_walk_path(
        doc,
        sub,
        delta,
        clock,
        delta.actors.get(&aid).map(|a| &a.mind.ltm.map_memory),
        &aid,
        (sx, sy),
        goal_xy,
    )
    .ok_or("兩處之間受海灣、峭壁或城牆阻斷，暫時沒有可行的陸路。")?;

    if let Some(a) = delta.actors.get_mut(&aid) {
        a.walk_waypoints.clear();
        a.walk_waypoints.extend(wps);
        a.walk_goal_enter_node = Some(target_node.to_string());
        a.route_plan.clear();
        a.mark_mutated();
    }

    if sync_legacy {
        let mut walked = 0usize;
        for _ in 0..2000 {
            match advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, &aid) {
                Ok(true) => walked += 1,
                Ok(false) => break,
                Err(block_msg) => {
                    if walked > 0 {
                        return Ok(block_msg);
                    }
                    return Err(block_msg);
                }
            }
        }
        if let Some(res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
            return res;
        }
        return Ok(format!("你沿路穩步前行，已抵達【{}】。", dst_nd.name));
    }

    if let Some(res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
        return res;
    }
    let _ = advance_one_grid_waypoint(doc, sub, delta, clock, high_tide, &aid)?;
    if let Some(res) = try_enter_goal_if_arrived(doc, sub, delta, clock, &aid) {
        return res;
    }
    Ok(format!("你動身朝【{}】方向走去。", dst_nd.name))
}

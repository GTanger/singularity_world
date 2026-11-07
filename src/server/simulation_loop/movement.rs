//! 移動節拍：travel tick（觀測圈＋腦驅動）與方格 NPC tick。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::Server;
use crate::db::{
    get_npc_person_display_name, get_npc_title_from_assignments, get_schedule_target,
    with_room_graph,
};
use crate::gametext;
use crate::npc::{
    GridNpcManager, NpcStep, TravelerManager, UNOBSERVED_MAX_NPCS_PER_TICK,
    apply_brain_arrival_effects, get_shift_flavor, run_unobserved_world_tick,
};
use crate::npcnpc::push_room_event;

use super::super::SessionStore;
use super::super::broadcast::{refresh_room_views_for_room, send_narrate_to_room};
use super::MainLoopTickState;
use super::helpers::{effective_travel_tick_interval, periodic_fire, sprintf_s};

const M3_027_NORTH_BANK_ROOM_ID: &str = "grid:0:0";
const M3_027_NORTH_BANK_ACTOR_PERIOD_SEC: u64 = 12;

/// 觀測圈：有玩家的房＋鄰房。
fn build_active_room_ids(sessions: &SessionStore, g: &crate::db::RoomGraph) -> HashSet<String> {
    let mut out = HashSet::new();
    for rid in sessions.player_room_ids() {
        out.insert(rid.clone());
        for nb in g.neighbors(&rid) {
            out.insert(nb);
        }
    }
    out
}

fn move_direction_zh(dx: i32, dy: i32) -> &'static str {
    match (dx.signum(), dy.signum()) {
        (0, 1) => "北面",
        (0, -1) => "南面",
        (1, 0) => "東面",
        (-1, 0) => "西面",
        _ => {
            if dy.abs() > dx.abs() {
                if dy > 0 {
                    "北面"
                } else {
                    "南面"
                }
            } else if dx > 0 {
                "東面"
            } else if dx < 0 {
                "西面"
            } else if dy > 0 {
                "北面"
            } else if dy < 0 {
                "南面"
            } else {
                "前方"
            }
        }
    }
}

fn resolve_grid_cell_display_name(room_id: &str) -> String {
    if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
        && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
    {
        if !cell.name.trim().is_empty() {
            return cell.name.clone();
        }
        return crate::grid::terrain_name_zh(cell.terrain).to_string();
    }
    crate::db::get_room_name(room_id).unwrap_or_else(|_| "街道".to_string())
}

fn resolve_npc_name_for_viewer(viewer_pid: &str, npc_id: &str, display_title: &str) -> String {
    let named = crate::db::get_acquaintance(viewer_pid, npc_id)
        .ok()
        .flatten()
        .map(|a| a.named)
        .unwrap_or(false);
    if named {
        crate::store::resolve_npc_person_name(npc_id, display_title, None)
    } else {
        let brief = crate::store::npc_brief_appearance_tag(npc_id, display_title);
        let slot = crate::store::wenyao_household_slot_for(npc_id);
        let role_src = format!(
            "{} {} {}",
            crate::npc::causal_core::strip_trailing_person_name_from_role(display_title),
            slot.brief_tag,
            slot.work_name
        );
        let role = if role_src.contains("鐵") || role_src.contains("鍛造") || role_src.contains("木作") || role_src.contains("工坊") {
            "匠人"
        } else if role_src.contains("商") || role_src.contains("鋪") || role_src.contains("市集") || role_src.contains("行") {
            "行商"
        } else if role_src.contains("醫") || role_src.contains("藥") || role_src.contains("診") {
            "醫工"
        } else if role_src.contains("學") || role_src.contains("書") || role_src.contains("講堂") || role_src.contains("觀星") {
            "學者"
        } else {
            "街坊"
        };
        if !brief.is_empty() {
            if matches!(brief.as_str(), "合社獵團" | "入山百工" | "古道行旅") {
                format!("{brief}的{role}")
            } else {
                format!("一名{brief}的{role}")
            }
        } else {
            format!("一名{role}")
        }
    }
}

pub(crate) fn resolve_npc_name_for_viewer_in_room(
    viewer_pid: &str,
    npc_id: &str,
    display_title: &str,
    room_name: &str,
    activity: &str,
    equipment_slots: &str,
) -> String {
    let named = !viewer_pid.is_empty()
        && crate::db::get_acquaintance(viewer_pid, npc_id)
            .ok()
            .flatten()
            .map(|a| a.named)
            .unwrap_or(false);
    if named {
        crate::store::resolve_npc_person_name(npc_id, display_title, None)
    } else {
        let (is_staff, has_uniform, in_counter_zone) =
            crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
                room_name,
                npc_id,
                display_title,
                activity,
                equipment_slots,
            );
        if is_staff {
            let host_title = crate::grid::resolve_any_room_venue_object(room_name, display_title)
                .map(|v| v.default_host_title)
                .unwrap_or("當值主事");
            if in_counter_zone && has_uniform {
                format!("身著制服站於櫃檯內側的{host_title}")
            } else if in_counter_zone {
                format!("站在櫃檯內側的{host_title}")
            } else if has_uniform {
                format!("身著工作制服的{host_title}")
            } else {
                host_title.to_string()
            }
        } else {
            resolve_npc_name_for_viewer(viewer_pid, npc_id, display_title)
        }
    }
}

pub(crate) fn resolve_unnamed_arrival_action_detail(
    room_name: &str,
    npc_id: &str,
    display_title: &str,
    activity: &str,
    equipment_slots: &str,
) -> String {
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            room_name,
            npc_id,
            display_title,
            activity,
            equipment_slots,
        );
    if is_staff {
        if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, display_title) {
            if in_counter_zone {
                return format!(
                    "邁步進入{}當值櫃檯內側，{}",
                    venue.venue_kind, venue.on_duty_fixture_activity
                );
            } else if has_uniform {
                return format!(
                    "身著工作制服步入{}，{}",
                    venue.venue_kind, venue.on_duty_fixture_activity
                );
            }
        }
    } else if let Some(ambient) = crate::grid::resolve_room_ambient_visitor_activity(
        room_name,
        npc_id.bytes().fold(0usize, |acc, b| acc.wrapping_add(b as usize)),
    ) {
        return format!("步入此處，{}", ambient);
    }
    "步履從容地沿街前行".to_string()
}

/// 發佈逐步移動敘事、房間事件與視野刷新。
fn apply_travel_steps(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    now_unix: i64,
    steps: Vec<NpcStep>,
    g: &crate::db::RoomGraph,
) {
    if steps.is_empty() {
        return;
    }
    let rf = gametext::runtime_fmt();
    for step in steps {
        if crate::store::is_npc_paused_for_interaction(&step.entity_id) {
            continue;
        }
        let old_name = g.room_name(&step.old_room);
        let new_name = g.room_name(&step.new_room);
        let want_old = crate::db::canonical_location_key(&step.old_room);
        let want_new = crate::db::canonical_location_key(&step.new_room);

        for s in sessions.all_sessions() {
            let loc = crate::server::session::player_location_key(&s.player_id);
            if loc == want_old {
                let viewer_name = resolve_npc_name_for_viewer(&s.player_id, &step.entity_id, &step.npc_name);
                let leave_text = sprintf_s(&rf.travel_leave_fmt, &[viewer_name.as_str(), new_name.as_str()]);
                crate::server::broadcast::send_narrate_to_player(&s, &leave_text);
                if !step.decision_narrative.is_empty() {
                    let d_text = if crate::store::player_knows_npc(&s.player_id, &step.entity_id) {
                        step.decision_narrative.clone()
                    } else {
                        step.decision_narrative.replace(&step.npc_name, &viewer_name)
                    };
                    crate::server::broadcast::send_narrate_to_player(&s, &d_text);
                }
            } else if loc == want_new {
                let viewer_name = resolve_npc_name_for_viewer(&s.player_id, &step.entity_id, &step.npc_name);
                let text = if let Some(target) = get_schedule_target(&step.entity_id, hour)
                    && target.room == step.new_room
                {
                    if target.is_work {
                        let mut occ = get_npc_title_from_assignments(&step.entity_id);
                        if occ.is_empty() {
                            occ = gametext::occupation_clerk();
                        }
                        let person = get_npc_person_display_name(&step.entity_id).unwrap_or_default();
                        let fl = get_shift_flavor(&occ, &person, true);
                        if !fl.is_empty() {
                            if crate::store::player_knows_npc(&s.player_id, &step.entity_id) {
                                fl
                            } else {
                                fl.replace(&person, &viewer_name)
                            }
                        } else {
                            sprintf_s(&rf.travel_arrive_fmt, &[viewer_name.as_str(), old_name.as_str()])
                        }
                    } else {
                        sprintf_s(&rf.wander_arrive_home, &[viewer_name.as_str()])
                    }
                } else {
                    sprintf_s(&rf.travel_arrive_fmt, &[viewer_name.as_str(), old_name.as_str()])
                };
                crate::server::broadcast::send_narrate_to_player(&s, &text);
                if let Some(ai) = step.arrival_intent {
                    let arrival_line = if crate::store::player_knows_npc(&s.player_id, &step.entity_id) {
                        gametext::brain_arrival(ai.as_key(), &step.npc_name)
                    } else {
                        gametext::brain_arrival(ai.as_key(), &viewer_name)
                    };
                    if !arrival_line.is_empty() {
                        crate::server::broadcast::send_narrate_to_player(&s, &arrival_line);
                    }
                }
            }
        }

        push_room_event(
            &step.old_room,
            "leave",
            &step.npc_name,
            &sprintf_s(&rf.push_leave_fmt, &[new_name.as_str()]),
        );
        push_room_event(
            &step.new_room,
            "arrive",
            &step.npc_name,
            &sprintf_s(&rf.push_arrive_fmt, &[old_name.as_str()]),
        );
        if let Some(ai) = step.arrival_intent {
            let (_, _, _, game_day) =
                crate::game::game_time_now(now_unix, cfg.game_time_epoch_unix, cfg.game_time_scale);
            apply_brain_arrival_effects(now_unix, game_day, &step.entity_id, &step.new_room, ai);
        }
        refresh_room_views_for_room(sessions, cfg, &step.old_room);
        refresh_room_views_for_room(sessions, cfg, &step.new_room);
    }
}

/// 每 `travel_tick_interval` 次 tick：無觀測則 `RunUnobservedWorldTick`；有觀測則 `TravelerManager.tick`。
pub(super) fn run_travel_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    // 白塔 runtime 啟用後，舊 RoomGraph traveler 不得再改寫 entity_rooms。
    if crate::store::get_store().is_some_and(|store| {
        store
            .read()
            .place_graph
            .places()
            .any(|place| place.source_kind == "azgaar_runtime" && place.status == "active")
    }) {
        return;
    }
    let fire = {
        let Ok(mut st) = state.lock() else {
            return;
        };
        st.travel_tick_count += 1;
        if st.travel_tick_count >= effective_travel_tick_interval() {
            st.travel_tick_count = 0;
            true
        } else {
            false
        }
    };
    if !fire {
        return;
    }
    with_room_graph(|g| {
        let active = build_active_room_ids(sessions, g);
        if active.is_empty() {
            run_unobserved_world_tick(Some(g), hour, UNOBSERVED_MAX_NPCS_PER_TICK, now_unix);
            return;
        }
        let steps = match traveler_mgr.lock() {
            Ok(mut guard) => guard.tick(g, hour, Some(&active)),
            Err(_) => vec![],
        };
        apply_travel_steps(sessions, cfg, hour, now_unix, steps, g);
    });
}

/// 居民場所行為 tick：玩家與 NPC 共用 world_routes，不再各算一套鄰接。
pub(super) fn run_grid_npc_tick(
    sessions: &SessionStore,
    cfg: &Server,
    now_unix: i64,
    hour: i32,
    game_day: i32,
    _state: &Arc<Mutex<MainLoopTickState>>,
    grid_npc_mgr: &Arc<Mutex<GridNpcManager>>,
) {
    let Ok(mut mgr) = grid_npc_mgr.lock() else {
        return;
    };
    let events = mgr.tick(now_unix, hour, game_day);
    crate::npc::causal_core::ensure_circular_economy_normalized_if_needed();
    let scheduled_moves = crate::server::grid_manager::advance_scheduled_residents_one_step();
    let household_moves = crate::store::get_store()
        .map(|s| s.read().advance_household_npcs_one_step())
        .unwrap_or_default();
    let _ = crate::npc::causal_core::drain_pending_arrival_effects();
    let _ = crate::npc::causal_core::step_offline_players_autonomy(crate::game::now_unix());
    if events.is_empty() && scheduled_moves.is_empty() && household_moves.is_empty() {
        return;
    }
    let mut player_rooms = sessions.player_room_ids();
    for pid in sessions.all_player_ids() {
        if let Ok(Some(ent)) = crate::db::get_entity(&pid) {
            if let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y) {
                player_rooms.insert(crate::grid::grid_room_id_from_coord(gx, gy));
            }
        }
    }

    let mut seen_moved = HashSet::new();
    let mut all_moves: Vec<(String, String, String)> = Vec::new();
    for (id, from_room, to_room) in household_moves {
        if seen_moved.insert(id.clone()) {
            all_moves.push((id, from_room, to_room));
        }
    }
    for (id, from_room, to_room) in scheduled_moves {
        if seen_moved.insert(id.clone()) {
            all_moves.push((id, from_room, to_room));
        }
    }
    for event in events {
        if seen_moved.insert(event.entity_id.clone()) {
            all_moves.push((event.entity_id, event.from_room_id, event.to_room_id));
        }
    }
    let mut refresh = HashSet::new();
    let mut narrate_counts: HashMap<String, usize> = HashMap::new();
    static LAST_TRANSIT_NARRATE_UNIX: std::sync::OnceLock<Mutex<HashMap<String, i64>>> =
        std::sync::OnceLock::new();
    let transit_gate = LAST_TRANSIT_NARRATE_UNIX.get_or_init(|| Mutex::new(HashMap::new()));
    let now_ts = crate::game::now_unix();
    for (npc_id, from_room_id, to_room_id) in all_moves {
        if from_room_id == to_room_id {
            continue;
        }
        let from_in_player = player_rooms.contains(&from_room_id);
        let to_in_player = player_rooms.contains(&to_room_id);
        if !from_in_player && !to_in_player {
            continue;
        }

        if from_in_player {
            refresh.insert(from_room_id.clone());
        }
        if to_in_player {
            refresh.insert(to_room_id.clone());
        }

        let from_coord = crate::grid::parse_grid_room_id(&from_room_id);
        let to_coord = crate::grid::parse_grid_room_id(&to_room_id);
        let (dx, dy) = match (from_coord, to_coord) {
            (Some(fc), Some(tc)) => (tc.x - fc.x, tc.y - fc.y),
            _ => (0, 0),
        };
        let dir_forward = move_direction_zh(dx, dy);
        let dir_backward = move_direction_zh(-dx, -dy);
        let from_name_raw = resolve_grid_cell_display_name(&from_room_id);
        let to_name_raw = resolve_grid_cell_display_name(&to_room_id);
        let clean_room_name = |name: &str| -> String {
            let s = name.trim_start_matches("文曜・");
            if s == "中心樞紐石橋" {
                "三水匯流大石橋".to_string()
            } else {
                s.to_string()
            }
        };
        let from_name_clean = clean_room_name(&from_name_raw);
        let to_name_clean = clean_room_name(&to_name_raw);

        let ent_opt = crate::db::get_entity(&npc_id).ok().flatten();
        let display_title = ent_opt.as_ref().map(|e| e.display_title.as_str()).unwrap_or("");
        let activity = ent_opt.as_ref().map(|e| e.current_activity.as_str()).unwrap_or("");
        let equipment_slots = ent_opt.as_ref().map(|e| e.equipment_slots.as_str()).unwrap_or("");
        // 離開現場敘事（每間房每 25 秒最多推播 1 條路過訊息，避免洗掉現場交談與叫賣）
        if from_in_player {
            let count = narrate_counts.entry(from_room_id.clone()).or_insert(0);
            let allow_transit = transit_gate
                .lock()
                .ok()
                .is_some_and(|g| now_ts - g.get(&from_room_id).copied().unwrap_or(0) >= 25);
            if *count < 1 && allow_transit {
                *count += 1;
                if let Ok(mut g) = transit_gate.lock() {
                    g.insert(from_room_id.clone(), now_ts);
                }
                let want = crate::db::canonical_location_key(&from_room_id);
                for s in sessions.all_sessions() {
                    if crate::server::session::player_location_key(&s.player_id) == want {
                        let named = crate::db::get_acquaintance(&s.player_id, &npc_id)
                            .ok()
                            .flatten()
                            .map(|a| a.named)
                            .unwrap_or(false);
                        let caller_name = resolve_npc_name_for_viewer_in_room(
                            &s.player_id,
                            &npc_id,
                            display_title,
                            &from_name_raw,
                            activity,
                            equipment_slots,
                        );
                        let leave_text = if !named || to_name_clean.is_empty() {
                            if dir_forward.is_empty() {
                                format!("{caller_name}邁步離去。")
                            } else {
                                format!("{caller_name}邁步往{dir_forward}一側行去。")
                            }
                        } else {
                            format!("{caller_name}邁步往{dir_forward}【{to_name_clean}】一側行去。")
                        };
                        crate::server::send_narrate_to_player(&s, &leave_text);
                    }
                }
            }
        }

        // 進入現場敘事（每間房每 25 秒最多推播 1 條路過訊息，避免洗掉現場交談與叫賣）
        if to_in_player {
            let count = narrate_counts.entry(to_room_id.clone()).or_insert(0);
            let allow_transit = transit_gate
                .lock()
                .ok()
                .is_some_and(|g| now_ts - g.get(&to_room_id).copied().unwrap_or(0) >= 25);
            if *count < 1 && allow_transit {
                *count += 1;
                if let Ok(mut g) = transit_gate.lock() {
                    g.insert(to_room_id.clone(), now_ts);
                }
                let cleaned_activity = if let Some(idx) = activity.find("沿街前往") {
                    // 若前面有「正從【...】」或「自【...】收工，」，只保留「沿街前往」及其前置修飾（如「才收工，正」）
                    let prefix = if activity.contains("收工") {
                        "收工後正"
                    } else {
                        "正"
                    };
                    format!("{}{}", prefix, &activity[idx..])
                } else {
                    activity.to_string()
                };
                let action_detail = if !cleaned_activity.is_empty() {
                    cleaned_activity
                } else {
                    "腳步從容，正沿街前行".to_string()
                };
                let want = crate::db::canonical_location_key(&to_room_id);
                for s in sessions.all_sessions() {
                    if crate::server::session::player_location_key(&s.player_id) == want {
                        let named = crate::db::get_acquaintance(&s.player_id, &npc_id).ok().flatten().map(|a| a.named).unwrap_or(false);
                        let caller_name = resolve_npc_name_for_viewer_in_room(
                            &s.player_id,
                            &npc_id,
                            display_title,
                            &to_name_raw,
                            activity,
                            equipment_slots,
                        );
                        let unnamed_detail = if !named {
                            resolve_unnamed_arrival_action_detail(
                                &to_name_raw,
                                &npc_id,
                                display_title,
                                activity,
                                equipment_slots,
                            )
                        } else {
                            String::new()
                        };
                        let effective_action_detail = if named {
                            action_detail.as_str()
                        } else {
                            unnamed_detail.as_str()
                        };
                        let act_clause = effective_action_detail.trim().trim_end_matches(['。', '！', '？', '，', ' ']);
                        let arrive_text = if dir_backward.is_empty() {
                            format!("{caller_name}自【{from_name_clean}】走來，{act_clause}。")
                        } else {
                            format!("{caller_name}自{dir_backward}【{from_name_clean}】一側走來，{act_clause}。")
                        };
                        crate::server::send_narrate_to_player(&s, &arrive_text);
                    }
                }
            }
        }

        if crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            &to_name_raw,
            &npc_id,
            display_title,
            activity,
            equipment_slots,
        )
        .0
        {
            if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &to_name_raw,
                None,
                now_ts as _,
            ) {
                if let Some(first_key) = ev.key_objects.split('、').next() {
                    let _ = crate::npc::brain_arrival::advance_room_event_pack_on_npc_arrival_action(
                        &to_name_raw,
                        first_key,
                    );
                }
            }
        }
    }

    for room_id in refresh {
        refresh_room_views_for_room(sessions, cfg, &room_id);
    }
}

/// M3-029 局部經濟行動者：北岸/市集有人觀測時，正式 NPC 個體推進同一張需求板。
pub(super) fn run_m3_027_north_bank_actor_tick(
    sessions: &SessionStore,
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    if !sessions.player_room_ids().iter().any(|room_id| {
        room_id == M3_027_NORTH_BANK_ROOM_ID || room_id == "grid:1:0" || room_id == "grid:1:-2"
    }) {
        return;
    }
    let fire = {
        let Ok(mut st) = state.lock() else {
            return;
        };
        periodic_fire(
            &mut st.last_m3_027_north_bank_actor,
            Duration::from_secs(M3_027_NORTH_BANK_ACTOR_PERIOD_SEC),
        )
    };
    if !fire {
        return;
    }
    let Some(tick) = crate::server::m3_029::run_pack_runner_tick(now_unix) else {
        return;
    };
    let narrative = format!("{}{}", tick.actor, tick.detail);
    push_room_event(tick.room_id, "work", tick.actor, &tick.detail);
    send_narrate_to_room(sessions, tick.room_id, &narrative);
    refresh_room_views_for_room(sessions, cfg, tick.room_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_npc_name_for_viewer_in_room_uniform_and_counter() {
        let name_both = resolve_npc_name_for_viewer_in_room(
            "",
            "test_npc_clerk",
            "店員",
            "半島百草診所",
            "",
            "",
        );
        assert_eq!(name_both, "身著制服站於櫃檯內側的當值醫師");

        let name_counter = resolve_npc_name_for_viewer_in_room(
            "",
            "test_npc_mgr",
            "掌事",
            "半島百草診所",
            "",
            "",
        );
        assert_eq!(name_counter, "站在櫃檯內側的當值醫師");

        let name_uniform = resolve_npc_name_for_viewer_in_room(
            "",
            "test_npc_cust",
            "穿制服的顧客",
            "半島百草診所",
            "",
            "",
        );
        assert_eq!(name_uniform, "身著工作制服的當值醫師");

        let name_passerby = resolve_npc_name_for_viewer_in_room(
            "",
            "test_npc_guest",
            "路人顧客",
            "",
            "",
            "",
        );
        assert!(name_passerby.contains("街坊"));
    }

    #[test]
    fn test_resolve_unnamed_arrival_action_detail() {
        let staff_arrival = resolve_unnamed_arrival_action_detail(
            "半島百草診所",
            "test_npc_clerk",
            "店員",
            "",
            "",
        );
        assert_eq!(
            staff_arrival,
            "邁步進入醫館藥局當值櫃檯內側，正於青石藥案前調配青鹽薄荷潔牙粉與水苔敷藥"
        );

        // 2. 穿著制服但非管制櫃檯區（以蒙學學舍為例）
        let uniform_arrival = resolve_unnamed_arrival_action_detail(
            "文曜蒙學",
            "test_npc_cust",
            "穿制服的顧客",
            "",
            "",
        );
        assert_eq!(
            uniform_arrival,
            "身著工作制服步入蒙學學舍，正敲響響板帶領學生朗讀基礎教材"
        );

        // 3. 一般訪客抵達（環境氛圍活動）
        let visitor_arrival = resolve_unnamed_arrival_action_detail(
            "尋常林間小道",
            "test_npc_guest",
            "路人顧客",
            "",
            "",
        );
        assert_eq!(
            visitor_arrival,
            "步入此處，沿著平整的路道與相熟的街坊點頭寒暄"
        );
    }
}

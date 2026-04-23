//! 移動節拍：travel tick（觀測圈＋腦驅動）與方格 NPC tick。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::config::Server;
use crate::db::{get_npc_person_display_name, get_npc_title_from_assignments, get_schedule_target, with_room_graph};
use crate::gametext;
use crate::npc::{
    apply_brain_arrival_effects, get_shift_flavor, run_unobserved_world_tick, GridNpcManager,
    NpcStep, TravelerManager, UNOBSERVED_MAX_NPCS_PER_TICK,
};
use crate::npcnpc::push_room_event;

use super::super::broadcast::{refresh_room_views_for_room, send_narrate_to_room};
use super::super::SessionStore;
use super::helpers::{effective_travel_tick_interval, sprintf_s};
use super::MainLoopTickState;

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
        let old_name = g.room_name(&step.old_room);
        let new_name = g.room_name(&step.new_room);
        let leave_text = sprintf_s(&rf.travel_leave_fmt, &[step.npc_name.as_str(), new_name.as_str()]);
        let mut arrive_text = sprintf_s(&rf.travel_arrive_fmt, &[step.npc_name.as_str(), old_name.as_str()]);
        if let Some(target) = get_schedule_target(&step.entity_id, hour)
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
                    arrive_text = fl;
                }
            } else {
                arrive_text = sprintf_s(&rf.wander_arrive_home, &[step.npc_name.as_str()]);
            }
        }
        send_narrate_to_room(sessions, &step.old_room, &leave_text);
        send_narrate_to_room(sessions, &step.new_room, &arrive_text);
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
        if !step.decision_narrative.is_empty() {
            send_narrate_to_room(sessions, &step.old_room, &step.decision_narrative);
        }
        if let Some(ai) = step.arrival_intent {
            let arrival_line = gametext::brain_arrival(ai.as_key(), &step.npc_name);
            if !arrival_line.is_empty() {
                send_narrate_to_room(sessions, &step.new_room, &arrival_line);
            }
            apply_brain_arrival_effects(now_unix, &step.entity_id, &step.new_room, ai);
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
    let fire = {
        let Ok(mut st) = state.lock() else { return; };
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
            run_unobserved_world_tick(Some(g), hour, UNOBSERVED_MAX_NPCS_PER_TICK);
            return;
        }
        let steps = match traveler_mgr.lock() {
            Ok(mut guard) => guard.tick(g, hour, Some(&active)),
            Err(_) => vec![],
        };
        apply_travel_steps(sessions, cfg, hour, now_unix, steps, g);
    });
}

/// 正方格 NPC 行為 tick：GridNpcManager.tick 本身很輕，無 NPC 即 early return。
pub(super) fn run_grid_npc_tick(
    now_unix: i64,
    _state: &Arc<Mutex<MainLoopTickState>>,
    grid_npc_mgr: &Arc<Mutex<GridNpcManager>>,
) {
    let Ok(mut mgr) = grid_npc_mgr.lock() else { return };
    super::super::grid_manager::with_square_grid_mut(|grid| {
        let _events = mgr.tick(grid, now_unix);
    });
}

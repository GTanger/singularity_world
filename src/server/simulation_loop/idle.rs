//! 閒置節拍：NPC↔NPC AI、微互動、在職巡邏與閒置動作。

use std::sync::{Arc, Mutex};

use rand::Rng;

use crate::config::{sim, Server};
use crate::db::{
    canonical_location_key, get_all_schedules, get_disposition, get_entity_room,
    get_npc_person_display_name, get_npc_title_from_assignments, get_room_name, set_entity_room,
};
use crate::gametext;
use crate::npc::{
    get_time_period, get_wander_flavor, get_wander_rooms, pick_idle_emote, pick_micro_interaction,
    pick_random_npc_npc_topic_by_mask,
};
use crate::npcnpc::{push_room_event, topic_mask_for_room, try_trigger_npc_npc_in_room};

use super::super::broadcast::{refresh_room_views_for_room, send_narrate_to_room};
use super::super::SessionStore;
use super::helpers::{roll_next_idle_interval, sprintf_s};
use super::MainLoopTickState;

pub(super) fn run_idle_wander_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let run = {
        let Ok(mut st) = state.lock() else { return; };
        st.idle_tick_count += 1;
        if st.idle_tick_count >= st.next_idle_trigger {
            st.idle_tick_count = 0;
            st.next_idle_trigger = roll_next_idle_interval();
            true
        } else {
            false
        }
    };
    if !run {
        return;
    }
    let period = get_time_period(hour);
    let player_room_set = sessions.player_room_ids();
    let ollama_ready = !cfg.ollama_base_url.is_empty() && !cfg.ollama_model.is_empty();
    let mut dialogue_done = false;
    if ollama_ready {
        for room_id in &player_room_set {
            let topic_hint = pick_random_npc_npc_topic_by_mask(&topic_mask_for_room(room_id, hour), "")
                .map(|t| t.hint)
                .unwrap_or_default();
            if try_trigger_npc_npc_in_room(sessions, cfg, room_id, &topic_hint, hour) {
                dialogue_done = true;
                break;
            }
        }
    }
    if !dialogue_done {
        let mut micro_pct = sim().micro_interaction_chance_percent;
        if micro_pct <= 0 {
            micro_pct = 15;
        }
        for room_id in &player_room_set {
            let line = pick_micro_interaction(room_id, micro_pct, hour);
            if !line.is_empty() {
                send_narrate_to_room(sessions, room_id, &line);
                break;
            }
        }
    }
    let Ok(schedules) = get_all_schedules() else {
        return;
    };
    let rf = gametext::runtime_fmt();
    let mut wmax = sim().wander_roll_max;
    if wmax <= 0 {
        wmax = 10;
    }
    let mut rng = rand::rng();
    for sch in schedules {
        if !sch.is_on_duty(hour) {
            continue;
        }
        let Ok(npc_room) = get_entity_room(&sch.entity_id) else {
            continue;
        };
        let mut title = get_npc_title_from_assignments(&sch.entity_id);
        if title.is_empty() {
            title = gametext::default_manager_title();
        }
        let person = get_npc_person_display_name(&sch.entity_id).unwrap_or_default();
        let wander_rooms = get_wander_rooms(&title);
        if wander_rooms.len() > 1 && rng.random_range(0..wmax) == 0 {
            let npc_key = canonical_location_key(&npc_room);
            let candidates: Vec<String> = wander_rooms
                .into_iter()
                .filter(|wr| canonical_location_key(wr) != npc_key)
                .collect();
            if !candidates.is_empty() {
                let dest = candidates[rng.random_range(0..candidates.len())].clone();
                let dest_name = get_room_name(&dest).unwrap_or_else(|_| String::new());
                let src_name = get_room_name(&npc_room).unwrap_or_else(|_| String::new());
                let leave_text = get_wander_flavor(&title, &person, &dest_name, true);
                let arrive_text = get_wander_flavor(&title, &person, &src_name, false);
                if !leave_text.is_empty() {
                    send_narrate_to_room(sessions, &npc_room, &leave_text);
                }
                if set_entity_room(&sch.entity_id, &dest).is_ok() && !arrive_text.is_empty() {
                    send_narrate_to_room(sessions, &dest, &arrive_text);
                }
                push_room_event(
                    &npc_room,
                    "leave",
                    &person,
                    &sprintf_s(&rf.push_leave_fmt, &[dest_name.as_str()]),
                );
                push_room_event(
                    &dest,
                    "arrive",
                    &person,
                    &sprintf_s(&rf.push_arrive_fmt, &[src_name.as_str()]),
                );
                refresh_room_views_for_room(sessions, cfg, &npc_room);
                refresh_room_views_for_room(sessions, cfg, &dest);
                continue;
            }
        }
        if !player_room_set.contains(&canonical_location_key(&npc_room)) {
            continue;
        }
        let disp = get_disposition(&sch.entity_id);
        let emote = pick_idle_emote(&title, &period, &person, disp);
        if !emote.is_empty() {
            send_narrate_to_room(sessions, &npc_room, &emote);
            break;
        }
    }
}

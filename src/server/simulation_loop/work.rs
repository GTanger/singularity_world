//! 工作節拍：求職撮合 tick、排班換時敘事與視野刷新。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::{sim, Server};
use crate::db::{
    adjust_disposition, apply_schedules, get_npc_ids_with_room, get_npc_person_display_name,
    get_npc_title_from_assignments, get_room_name, get_schedule_for_entity, get_schedule_target,
    upsert_npc_rumor, with_room_graph,
};
use crate::gametext;
use crate::npc::{
    get_npc_npc_topic_by_id, get_shift_flavor, movement_speed_for_title,
    pick_random_npc_npc_topic_by_mask, run_job_matching_tick, JobMatchParams, MovementDef,
    MovementType, TravelerManager, SEEK_JOB_MG_THRESHOLD_DEFAULT,
};
use crate::npcnpc::{push_room_event, topic_mask_for_room, try_trigger_npc_npc_in_room};
use crate::store::NpcRumor;

use super::super::broadcast::{broadcast_room_views, send_narrate_to_room};
use super::super::SessionStore;
use super::helpers::{periodic_fire, sprintf_d, sprintf_s};
use super::MainLoopTickState;

/// 求職撮合 tick（＋傳聞）；新入職且有排班者註冊排班型 Traveler。
pub(super) fn run_job_matching_section(
    sessions: &SessionStore,
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    let mut jm_sec = sim().job_matching_interval_sec;
    if jm_sec <= 0 {
        jm_sec = 30;
    }
    let fire = {
        let Ok(mut st) = state.lock() else { return; };
        periodic_fire(&mut st.last_job_matching, Duration::from_secs(jm_sec as u64))
    };
    if !fire {
        return;
    }
    let mut mg_th = cfg.seek_job_mg_threshold;
    if mg_th <= 0 {
        mg_th = SEEK_JOB_MG_THRESHOLD_DEFAULT;
    }
    let added = with_room_graph(|g| {
        run_job_matching_tick(
            Some(g),
            JobMatchParams {
                max_per_venue: sim().max_assignments_per_venue,
                mg_threshold: mg_th,
                job_match_when_stable: cfg.job_match_when_stable,
            },
        )
    });
    if added.is_empty() {
        return;
    }
    for eid in &added {
        if get_schedule_for_entity(eid).is_some() {
            let mut title = get_npc_title_from_assignments(eid);
            if title.is_empty() {
                title = gametext::default_manager_title();
            }
            let speed = movement_speed_for_title(&title);
            if let Ok(mut g) = traveler_mgr.lock() {
                g.register(
                    eid.clone(),
                    MovementDef {
                        kind: MovementType::Schedule,
                        speed,
                    },
                );
            }
        }
    }
    broadcast_room_views(sessions, cfg);
    let mut job_ttl = sim().job_match_rumor_ttl_sec;
    if job_ttl <= 0 {
        job_ttl = 7200;
    }
    let fmt = gametext::npc_social().job_match_rumor_fmt;
    let text = sprintf_d(&fmt, added.len() as i32);
    let rumor = NpcRumor {
        id: format!("job|{}", now_unix / 1800),
        text,
        room_id: String::new(),
        zone: String::new(),
        source: "job".into(),
        source_score: 4,
        weight: 2,
        mention_count: 0,
        last_used_at: 0,
        blocked_until: 0,
        penalty_count: 0,
        last_penalty_at: 0,
        last_penalty_reason: String::new(),
        updated_at: now_unix,
        expires_at: now_unix + job_ttl,
    };
    if let Err(e) = upsert_npc_rumor(rumor) {
        tracing::warn!("job match upsert_npc_rumor: {e}");
    }
}

/// 排班換時：心境時段、`ApplySchedules` 敘事、廣播視野、嘗試交班主題 NPC↔NPC。
pub(super) fn run_schedule_hour_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let run = {
        let Ok(mut st) = state.lock() else { return; };
        if hour != st.last_schedule_hour {
            st.last_schedule_hour = hour;
            true
        } else {
            false
        }
    };
    if !run {
        return;
    }

    let tod = &sim().disposition_time_of_day;
    let (mut ds, mut de) = (tod.dawn_hour_start, tod.dawn_hour_end);
    if ds == 0 && de == 0 {
        ds = 6;
        de = 9;
    }
    let mut disp_delta = 0;
    if hour >= ds && hour <= de {
        disp_delta = tod.dawn_delta;
    } else if (0..=tod.late_night_hour_end).contains(&hour) {
        disp_delta = tod.late_night_delta;
    }
    if disp_delta != 0 {
        for nid in get_npc_ids_with_room() {
            let _ = adjust_disposition(&nid, disp_delta);
        }
    }

    let Ok(moves) = apply_schedules(hour) else {
        return;
    };
    let rf = gametext::runtime_fmt();
    for m in &moves {
        let mut person = get_npc_person_display_name(&m.entity_id).unwrap_or_default();
        if person.is_empty() {
            person.clone_from(&m.entity_id);
        }
        let occ = get_npc_title_from_assignments(&m.entity_id);
        let leave_text = if let Some(target) = get_schedule_target(&m.entity_id, hour) {
            if target.room == m.new_room && target.is_work {
                sprintf_s(&rf.shift_leave_to_shop_fmt, &[&person])
            } else if !occ.is_empty() {
                let flavor = get_shift_flavor(&occ, &person, false);
                if !flavor.is_empty() {
                    flavor
                } else {
                    sprintf_s(&rf.shift_leave_fmt, &[&person])
                }
            } else {
                sprintf_s(&rf.shift_leave_fmt, &[&person])
            }
        } else {
            sprintf_s(&rf.shift_leave_fmt, &[&person])
        };
        if !m.old_room.is_empty() && !leave_text.is_empty() {
            send_narrate_to_room(sessions, &m.old_room, &leave_text);
        }
        let mut new_room_name = get_room_name(&m.new_room).unwrap_or_default();
        if new_room_name.is_empty() {
            new_room_name.clone_from(&m.new_room);
        }
        if !m.old_room.is_empty() {
            push_room_event(
                &m.old_room,
                "shift_leave",
                &person,
                &sprintf_s(&rf.push_shift_leave_detail_fmt, &[&new_room_name]),
            );
        }
        if !m.new_room.is_empty() {
            push_room_event(&m.new_room, "shift_arrive", &person, &rf.shift_arrive);
        }
    }

    if moves.is_empty() {
        return;
    }
    broadcast_room_views(sessions, cfg);

    let topic_hint = get_npc_npc_topic_by_id(&gametext::topic_id("shift_handover"))
        .map(|t| t.hint)
        .unwrap_or_default();
    let mut rooms_active: HashSet<String> = HashSet::new();
    for m in &moves {
        if !m.old_room.is_empty() {
            rooms_active.insert(m.old_room.clone());
        }
        if !m.new_room.is_empty() {
            rooms_active.insert(m.new_room.clone());
        }
    }
    for room_id in rooms_active {
        let hint = if topic_hint.is_empty() {
            pick_random_npc_npc_topic_by_mask(&topic_mask_for_room(&room_id, hour), "")
                .map(|t| t.hint)
                .unwrap_or_default()
        } else {
            topic_hint.clone()
        };
        if try_trigger_npc_npc_in_room(sessions, cfg, &room_id, &hint, hour) {
            break;
        }
    }
}

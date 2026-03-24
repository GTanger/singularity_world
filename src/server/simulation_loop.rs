//! 主迴圈背景 tick（對齊 Go `simulation_main_loop.go`：視野、傳聞、隨機 NPC↔NPC、遊戲日、NPC 池、求職撮合、排班敘事、`TravelerManager`、未觀測 tick、閒置／微互動／巡邏）。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::config::{sim, Server};
use crate::db::{
    adjust_disposition, apply_schedules, build_npc_rumor_digest, decay_npc_rumors, deduct_daily_expense,
    get_all_schedules, get_disposition, get_entity_room, get_npc_ids_with_room, get_npc_person_display_name,
    get_npc_title_from_assignments, get_player_ids_with_room, get_room, get_room_name, get_schedule_for_entity,
    get_schedule_target, get_spawn_room_id, set_entity_room, spawn_one_npc_from_pool, upsert_npc_rumor,
    with_room_graph,
};
use crate::gametext;
use crate::game::{game_time_now, run_view_simulation, Pos};
use crate::npc::{
    get_npc_npc_topic_by_id, get_shift_flavor, get_time_period, get_wander_flavor, get_wander_rooms,
    movement_speed_for_title, pick_idle_emote, pick_micro_interaction, pick_random_npc_npc_topic_by_mask,
    apply_brain_arrival_effects, run_job_matching_tick, run_unobserved_world_tick, seed_traveler_manager,
    JobMatchParams, MovementDef, MovementType, NpcStep, TravelerManager,
    SEEK_JOB_MG_THRESHOLD_DEFAULT, UNOBSERVED_MAX_NPCS_PER_TICK,
};
use crate::npcnpc::{push_room_event, topic_mask_for_room, try_trigger_npc_npc_in_room};
use crate::store::NpcRumor;

use super::broadcast::{broadcast_room_views, refresh_room_views_for_room, send_narrate_to_room};
use super::SessionStore;

/// 跨 tick 的計時與狀態（對齊 Go 閉包內變數）。
struct MainLoopTickState {
    random_dialogue_ticks: i32,
    last_rumor_decay: Option<Instant>,
    last_rumor_digest: Option<Instant>,
    last_expense_game_day: i32,
    last_spawn_check: Instant,
    last_job_matching: Option<Instant>,
    last_schedule_hour: i32,
    travel_tick_count: i32,
    idle_tick_count: i32,
    /// 累積到此 tick 數即觸發閒置／巡邏區塊。
    next_idle_trigger: i32,
}

/// 間隔到期則更新錨點並回傳 true；`last` 為 `None` 時第一次立即觸發（對齊 Go `time.Time` 零值）。
fn periodic_fire(last: &mut Option<Instant>, period: Duration) -> bool {
    let now = Instant::now();
    let fire = match *last {
        None => true,
        Some(t) => now.duration_since(t) >= period,
    };
    if fire {
        *last = Some(now);
    }
    fire
}

fn sprintf_s(template: &str, args: &[&str]) -> String {
    let mut s = template.to_string();
    for a in args {
        if let Some(pos) = s.find("%s") {
            s.replace_range(pos..pos + 2, a);
        }
    }
    s
}

fn sprintf_d(template: &str, n: i32) -> String {
    template.replacen("%d", &n.to_string(), 1)
}

/// 依 `simulation.json` 的 `random_npc_dialogue_ticks` 產生初始倒數。
/// `simulation.json` 的 `travel_tick_interval`；無效時 30。
fn effective_travel_tick_interval() -> i32 {
    let v = sim().travel_tick_interval;
    if v <= 0 {
        30
    } else {
        v
    }
}

/// 首次閒置觸發門檻（對齊 Go `nextIdleTrigger` 初值）。
fn initial_next_idle_trigger() -> i32 {
    let idle = &sim().idle;
    let mut span = idle.first_trigger_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    idle.first_trigger_min + rng.random_range(0..span)
}

/// 之後每次閒置觸發後重骰的間隔 tick。
fn roll_next_idle_interval() -> i32 {
    let idle = &sim().idle;
    let mut span = idle.interval_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    idle.interval_min + rng.random_range(0..span)
}

fn initial_random_dialogue_ticks() -> i32 {
    let rdt = &sim().random_npc_dialogue_ticks;
    let mut span = rdt.initial_span;
    if span <= 0 {
        span = 1;
    }
    let mut rng = rand::rng();
    rdt.initial_min + rng.random_range(0..span)
}

/// 無玩家在線或無法觸發 AI 時，用 `server_defaults` 的間隔重設倒數（對齊 Go `NpcNpcSocialTickMinNoPlayer` 等）。
fn reset_ticks_no_player(cfg: &Server) -> i32 {
    let mut min_e = cfg.npc_npc_social_tick_min_no_player;
    if min_e <= 0 {
        min_e = 80;
    }
    let mut extra = cfg.npc_npc_social_tick_extra_no_player;
    if extra <= 0 {
        extra = 40;
    }
    let span = extra.max(1);
    let mut rng = rand::rng();
    min_e + rng.random_range(0..span)
}

/// 遊戲日換日時扣鎂並可寫經濟 pulse 傳聞（對齊 Go `lastExpenseDay` 區塊）。
fn run_game_day_economy(now_unix: i64, game_day: i32, state: &Arc<Mutex<MainLoopTickState>>) {
    let should = {
        let mut st = state.lock().expect("main loop state poisoned");
        if game_day != st.last_expense_game_day {
            st.last_expense_game_day = game_day;
            true
        } else {
            false
        }
    };
    if !should {
        return;
    }
    if let Err(e) = deduct_daily_expense(now_unix) {
        tracing::warn!("deduct_daily_expense: {e}");
    }
    let eco = &sim().economy_pulse;
    let mut eco_mod = eco.game_day_modulo;
    if eco_mod <= 0 {
        eco_mod = 3;
    }
    if eco_mod > 0 && game_day.rem_euclid(eco_mod) == 0 {
        let lines = gametext::economy_rumor_lines();
        if lines.is_empty() {
            return;
        }
        let mut rng = rand::rng();
        let eco_text = lines[rng.random_range(0..lines.len())].clone();
        let mut ttl = eco.rumor_ttl_sec;
        if ttl <= 0 {
            ttl = 5400;
        }
        let pulse_id = game_day.div_euclid(eco_mod);
        let rumor = NpcRumor {
            id: format!("eco|pulse|{pulse_id}"),
            text: eco_text,
            room_id: String::new(),
            zone: String::new(),
            source: "economy".into(),
            source_score: 1,
            weight: 1,
            mention_count: 0,
            last_used_at: 0,
            blocked_until: 0,
            penalty_count: 0,
            last_penalty_at: 0,
            last_penalty_reason: String::new(),
            updated_at: now_unix,
            expires_at: now_unix + ttl,
        };
        if let Err(e) = upsert_npc_rumor(rumor) {
            tracing::warn!("economy pulse upsert_npc_rumor: {e}");
        }
    }
}

/// NPC 池補滿（對齊 Go）；新生成 NPC 註冊為腦驅動 Traveler。
fn run_npc_pool_tick(
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    let pool = cfg.npc_pool_size;
    let spawn_sec = cfg.npc_spawn_interval_sec;
    if pool <= 0 || spawn_sec <= 0 {
        return;
    }
    let fire = {
        let mut st = state.lock().expect("main loop state poisoned");
        if st.last_spawn_check.elapsed() >= Duration::from_secs(spawn_sec as u64) {
            st.last_spawn_check = Instant::now();
            true
        } else {
            false
        }
    };
    if !fire {
        return;
    }
    let n_npcs = get_npc_ids_with_room().len();
    let n_players = get_player_ids_with_room().len();
    if n_npcs + n_players >= pool as usize {
        return;
    }
    let spawn_room = get_spawn_room_id();
    let new_id = match spawn_one_npc_from_pool(&spawn_room) {
        Ok(id) if !id.is_empty() => id,
        Ok(_) => return,
        Err(e) => {
            tracing::warn!("spawn_one_npc_from_pool: {e}");
            return;
        }
    };
    let zone = get_room(&spawn_room)
        .ok()
        .flatten()
        .map(|r| r.zone)
        .unwrap_or_default();
    let mut ttl = sim().spawn_rumor_ttl_sec;
    if ttl <= 0 {
        ttl = 3600;
    }
    let rumor = NpcRumor {
        id: format!("spawn|{spawn_room}|{new_id}"),
        text: gametext::npc_social().newcomer_rumor,
        room_id: spawn_room.clone(),
        zone,
        source: "spawn".into(),
        source_score: 1,
        weight: 1,
        mention_count: 0,
        last_used_at: 0,
        blocked_until: 0,
        penalty_count: 0,
        last_penalty_at: 0,
        last_penalty_reason: String::new(),
        updated_at: now_unix,
        expires_at: now_unix + ttl,
    };
    if let Err(e) = upsert_npc_rumor(rumor) {
        tracing::warn!("spawn upsert_npc_rumor: {e}");
    }
    if let Ok(mut g) = traveler_mgr.lock() {
        g.register(
            new_id,
            MovementDef {
                kind: MovementType::Brain,
                speed: 1,
            },
        );
    }
}

/// 求職撮合 tick（對齊 Go `RunJobMatchingTick` ＋傳聞）；新入職且有排班者註冊排班型 Traveler。
fn run_job_matching_section(
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
        let mut st = state.lock().expect("main loop state poisoned");
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

/// 排班換時：心境時段、`ApplySchedules` 敘事、廣播視野、嘗試交班主題 NPC↔NPC（對齊 Go；逐步位移由 `TravelerManager` 處理）。
fn run_schedule_hour_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let run = {
        let mut st = state.lock().expect("main loop state poisoned");
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

/// 觀測圈：有玩家的房＋鄰房（對齊 Go `buildActiveRoomIDs`）。
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

/// 發佈逐步移動敘事、房間事件與視野刷新（對齊 Go `travelSteps` 迴圈）。
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
fn run_travel_section(
    sessions: &SessionStore,
    cfg: &Server,
    hour: i32,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    let fire = {
        let mut st = state.lock().expect("main loop state poisoned");
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
        let steps = traveler_mgr
            .lock()
            .expect("traveler mgr poisoned")
            .tick(g, hour, Some(&active));
        apply_travel_steps(sessions, cfg, hour, now_unix, steps, g);
    });
}

/// 閒置 tick：NPC↔NPC AI、微互動、在職巡邏與閒置動作（對齊 Go 主迴圈末段）。
fn run_idle_wander_section(sessions: &SessionStore, cfg: &Server, hour: i32, state: &Arc<Mutex<MainLoopTickState>>) {
    let run = {
        let mut st = state.lock().expect("main loop state poisoned");
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
            let candidates: Vec<String> = wander_rooms
                .into_iter()
                .filter(|wr| wr != &npc_room)
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
        if !player_room_set.contains(&npc_room) {
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

/// 單次 tick（對齊 Go `game.Loop` 回呼內順序）。
fn run_simulation_tick(
    sessions: &SessionStore,
    cfg: &Server,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    run_view_simulation(Vec::<Pos>::new, None);

    let now_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let (_sec, hour, _min, game_day) =
        game_time_now(now_unix, cfg.game_time_epoch_unix, cfg.game_time_scale);

    let sim_c = sim();
    let mut rd_sec = sim_c.rumor_decay_interval_sec;
    if rd_sec <= 0 {
        rd_sec = 30;
    }
    let mut rd_min = sim_c.rumor_digest_interval_min;
    if rd_min <= 0 {
        rd_min = 2;
    }
    let decay_dur = Duration::from_secs(rd_sec as u64);
    let digest_dur = Duration::from_secs((rd_min as u64).saturating_mul(60));

    let (fire_decay, fire_digest) = {
        let mut st = state.lock().expect("main loop state poisoned");
        let fd = periodic_fire(&mut st.last_rumor_decay, decay_dur);
        let fdi = periodic_fire(&mut st.last_rumor_digest, digest_dur);
        (fd, fdi)
    };

    if fire_decay && let Err(e) = decay_npc_rumors(now_unix) {
        tracing::warn!("decay_npc_rumors: {e}");
    }
    if fire_digest && let Err(e) = build_npc_rumor_digest(now_unix) {
        tracing::warn!("build_npc_rumor_digest: {e}");
    }

    let ollama_ready = !cfg.ollama_base_url.is_empty() && !cfg.ollama_model.is_empty();
    let mut do_trigger: Option<(String, i32)> = None;
    {
        let mut st = state.lock().expect("main loop state poisoned");
        st.random_dialogue_ticks -= 1;
        if st.random_dialogue_ticks <= 0 && ollama_ready {
            let room_set = sessions.player_room_ids();
            if room_set.is_empty() {
                st.random_dialogue_ticks = reset_ticks_no_player(cfg);
            } else {
                let mut rooms: Vec<String> = room_set.into_iter().collect();
                let mut rng = rand::rng();
                let idx = rng.random_range(0..rooms.len());
                let room_id = rooms.swap_remove(idx);
                let rdt = &sim().random_npc_dialogue_ticks;
                let mut rsp = rdt.reset_span_with_player;
                if rsp <= 0 {
                    rsp = 1;
                }
                st.random_dialogue_ticks = rdt.reset_min_with_player + rng.random_range(0..rsp);
                do_trigger = Some((room_id, hour));
            }
        } else if st.random_dialogue_ticks <= 0 && !ollama_ready {
            st.random_dialogue_ticks = reset_ticks_no_player(cfg);
        }
    }

    if let Some((room_id, h)) = do_trigger {
        let mask = topic_mask_for_room(&room_id, h);
        let topic_hint = pick_random_npc_npc_topic_by_mask(&mask, "")
            .map(|t| t.hint)
            .unwrap_or_default();
        let _ = try_trigger_npc_npc_in_room(sessions, cfg, &room_id, &topic_hint, h);
    }

    run_game_day_economy(now_unix, game_day, state);
    run_npc_pool_tick(cfg, now_unix, state, traveler_mgr);
    run_job_matching_section(sessions, cfg, now_unix, state, traveler_mgr);
    run_schedule_hour_section(sessions, cfg, hour, state);
    run_travel_section(sessions, cfg, hour, now_unix, state, traveler_mgr);
    run_idle_wander_section(sessions, cfg, hour, state);
}

/// 在 Tokio runtime 上依 `cfg.tick_interval_ms` 週期執行主迴圈片段（與 Axum 並行）。
pub fn spawn_simulation_main_loop(sessions: Arc<SessionStore>, cfg: Server) {
    let traveler_mgr = Arc::new(Mutex::new(TravelerManager::new()));
    if let Err(e) = seed_traveler_manager(&traveler_mgr) {
        tracing::warn!("seed_traveler_manager: {e}");
    }
    let tick_state = Arc::new(Mutex::new(MainLoopTickState {
        random_dialogue_ticks: initial_random_dialogue_ticks(),
        last_rumor_decay: None,
        last_rumor_digest: None,
        last_expense_game_day: -1,
        last_spawn_check: Instant::now(),
        last_job_matching: None,
        last_schedule_hour: -1,
        travel_tick_count: 0,
        idle_tick_count: 0,
        next_idle_trigger: initial_next_idle_trigger(),
    }));
    let tick_ms = cfg.tick_interval_ms.max(1);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(tick_ms));
        loop {
            interval.tick().await;
            let sessions = Arc::clone(&sessions);
            let cfg = cfg.clone();
            let tick_state = Arc::clone(&tick_state);
            let tm = Arc::clone(&traveler_mgr);
            let res = tokio::task::spawn_blocking(move || {
                run_simulation_tick(&sessions, &cfg, &tick_state, &tm);
            })
            .await;
            if res.is_err() {
                tracing::warn!("simulation tick task join failed");
            }
        }
    });
}

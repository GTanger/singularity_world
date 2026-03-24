//! 主迴圈背景 tick（對齊 Go `simulation_main_loop.go`：視野、傳聞、隨機 NPC↔NPC、遊戲日、NPC 池、求職撮合、排班敘事）。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::config::{sim, Server};
use crate::db::{
    adjust_disposition, apply_schedules, build_npc_rumor_digest, decay_npc_rumors, deduct_daily_expense,
    get_npc_ids_with_room, get_npc_person_display_name, get_npc_title_from_assignments, get_player_ids_with_room,
    get_room, get_room_name, get_schedule_target, get_spawn_room_id, spawn_one_npc_from_pool, upsert_npc_rumor,
    with_room_graph,
};
use crate::gametext;
use crate::game::{game_time_now, run_view_simulation, Pos};
use crate::npc::{
    get_npc_npc_topic_by_id, get_shift_flavor, pick_random_npc_npc_topic_by_mask, run_job_matching_tick,
    JobMatchParams, SEEK_JOB_MG_THRESHOLD_DEFAULT,
};
use crate::npcnpc::{push_room_event, topic_mask_for_room, try_trigger_npc_npc_in_room};
use crate::store::NpcRumor;

use super::broadcast::{broadcast_room_views, send_narrate_to_room};
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

/// NPC 池補滿（對齊 Go；Traveler 註冊待日後遷移）。
fn run_npc_pool_tick(cfg: &Server, now_unix: i64, state: &Arc<Mutex<MainLoopTickState>>) {
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
}

/// 求職撮合 tick（對齊 Go `RunJobMatchingTick` ＋傳聞）。
fn run_job_matching_section(sessions: &SessionStore, cfg: &Server, now_unix: i64, state: &Arc<Mutex<MainLoopTickState>>) {
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

/// 排班換時：心境時段、`ApplySchedules` 敘事、廣播視野、嘗試交班主題 NPC↔NPC（對齊 Go；實際走路仍待 Traveler）。
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

/// 單次 tick（對齊 Go `game.Loop` 回呼內順序）。
fn run_simulation_tick(sessions: &SessionStore, cfg: &Server, state: &Arc<Mutex<MainLoopTickState>>) {
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
    run_npc_pool_tick(cfg, now_unix, state);
    run_job_matching_section(sessions, cfg, now_unix, state);
    run_schedule_hour_section(sessions, cfg, hour, state);
}

/// 在 Tokio runtime 上依 `cfg.tick_interval_ms` 週期執行主迴圈片段（與 Axum 並行）。
pub fn spawn_simulation_main_loop(sessions: Arc<SessionStore>, cfg: Server) {
    let tick_state = Arc::new(Mutex::new(MainLoopTickState {
        random_dialogue_ticks: initial_random_dialogue_ticks(),
        last_rumor_decay: None,
        last_rumor_digest: None,
        last_expense_game_day: -1,
        last_spawn_check: Instant::now(),
        last_job_matching: None,
        last_schedule_hour: -1,
    }));
    let tick_ms = cfg.tick_interval_ms.max(1);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(tick_ms));
        loop {
            interval.tick().await;
            let sessions = Arc::clone(&sessions);
            let cfg = cfg.clone();
            let tick_state = Arc::clone(&tick_state);
            let res = tokio::task::spawn_blocking(move || {
                run_simulation_tick(&sessions, &cfg, &tick_state);
            })
            .await;
            if res.is_err() {
                tracing::warn!("simulation tick task join failed");
            }
        }
    });
}

//! 主迴圈背景 tick（對齊既有 `simulation_main_loop`：視野、傳聞、隨機 NPC↔NPC、遊戲日、NPC 池、求職撮合、排班敘事、`TravelerManager`、未觀測 tick、閒置／微互動／巡邏）。
//!
//! 分層：節拍子模組（world/work/movement/idle）＋共用小工具（helpers）。

mod helpers;
mod idle;
mod movement;
mod work;
mod world;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::config::{sim, Server};
use crate::db::{build_npc_rumor_digest, decay_lexicon, decay_npc_rumors, promote_lexicon_candidates};
use crate::game::{game_time_now, run_view_simulation, Pos};
use crate::npc::{seed_traveler_manager, GridNpcManager, TravelerManager};
use crate::npcnpc::{topic_mask_for_room, try_trigger_npc_npc_in_room};

use super::SessionStore;

use helpers::{initial_next_idle_trigger, initial_random_dialogue_ticks, periodic_fire, reset_ticks_no_player};
use idle::run_idle_wander_section;
use movement::{run_grid_npc_tick, run_travel_section};
use work::{run_job_matching_section, run_schedule_hour_section};
use world::{inject_event_seed, run_game_day_economy, run_npc_pool_tick};

/// 跨 tick 的計時與狀態（對齊既有 閉包內變數）。欄位以 `pub(super)` 開放供子模組存取。
pub(super) struct MainLoopTickState {
    pub(super) random_dialogue_ticks: i32,
    pub(super) last_rumor_decay: Option<Instant>,
    pub(super) last_rumor_digest: Option<Instant>,
    pub(super) last_expense_game_day: i32,
    pub(super) last_spawn_check: Instant,
    pub(super) last_job_matching: Option<Instant>,
    pub(super) last_schedule_hour: i32,
    pub(super) travel_tick_count: i32,
    pub(super) idle_tick_count: i32,
    /// 累積到此 tick 數即觸發閒置／巡邏區塊。
    pub(super) next_idle_trigger: i32,
    /// 事件種子注入計時器
    pub(super) last_event_seed: Option<Instant>,
    /// 世界詞典維護計時器
    pub(super) last_lexicon_maintenance: Option<Instant>,
}

/// 單次 tick（對齊既有 `game.Loop` 回呼內順序）。
fn run_simulation_tick(
    sessions: &SessionStore,
    cfg: &Server,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
    grid_npc_mgr: &Arc<Mutex<GridNpcManager>>,
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
        let Ok(mut st) = state.lock() else { return; };
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

    // 事件種子注入（每 10 分鐘一次）
    let fire_seed = {
        let Ok(mut st) = state.lock() else { return };
        periodic_fire(&mut st.last_event_seed, Duration::from_secs(600))
    };
    if fire_seed {
        inject_event_seed(now_unix);
    }

    // 世界詞典維護（每 5 分鐘一次）
    let fire_lexicon = {
        let Ok(mut st) = state.lock() else { return };
        periodic_fire(&mut st.last_lexicon_maintenance, Duration::from_secs(300))
    };
    if fire_lexicon {
        promote_lexicon_candidates();
        decay_lexicon();
    }

    let ollama_ready = !cfg.ollama_base_url.is_empty() && !cfg.ollama_model.is_empty();
    let mut do_trigger: Option<(String, i32)> = None;
    {
        let Ok(mut st) = state.lock() else { return; };
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
        let topic_hint = crate::npc::pick_random_npc_npc_topic_by_mask(&mask, "")
            .map(|t| t.hint)
            .unwrap_or_default();
        let _ = try_trigger_npc_npc_in_room(sessions, cfg, &room_id, &topic_hint, h);
    }

    run_game_day_economy(now_unix, game_day, state);
    run_npc_pool_tick(cfg, now_unix, state, traveler_mgr);
    run_job_matching_section(sessions, cfg, now_unix, state, traveler_mgr);
    run_schedule_hour_section(sessions, cfg, hour, state);
    run_travel_section(sessions, cfg, hour, now_unix, state, traveler_mgr);
    run_grid_npc_tick(now_unix, state, grid_npc_mgr);
    run_idle_wander_section(sessions, cfg, hour, state);
}

/// 在 Tokio runtime 上依 `cfg.tick_interval_ms` 週期執行主迴圈片段（與 Axum 並行）。
pub fn spawn_simulation_main_loop(sessions: Arc<SessionStore>, cfg: Server) {
    let traveler_mgr = Arc::new(Mutex::new(TravelerManager::new()));
    if let Err(e) = seed_traveler_manager(&traveler_mgr) {
        tracing::warn!("seed_traveler_manager: {e}");
    }
    let grid_npc_mgr = Arc::new(Mutex::new(GridNpcManager::new()));
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
        last_event_seed: None,
        last_lexicon_maintenance: None,
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
            let gm = Arc::clone(&grid_npc_mgr);
            let res = tokio::task::spawn_blocking(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_simulation_tick(&sessions, &cfg, &tick_state, &tm, &gm);
                }));
                if let Err(e) = result {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "unknown panic".to_string()
                    };
                    tracing::error!("simulation tick PANIC: {msg}");
                }
            })
            .await;
            if res.is_err() {
                tracing::warn!("simulation tick task join failed");
            }
        }
    });
}

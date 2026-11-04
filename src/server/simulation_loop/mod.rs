//! 主迴圈背景 tick（對齊既有 `simulation_main_loop`：視野、傳聞、隨機 NPC↔NPC、遊戲日、NPC 池、求職撮合、排班敘事、`TravelerManager`、未觀測 tick、閒置／微互動／巡邏）。
//!
//! 分層：節拍子模組（world/work/movement/idle）＋共用小工具（helpers）。

mod helpers;
mod idle;
mod market_economy;
mod movement;
mod work;
mod world;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::config::{Server, sim};
use crate::db::{
    build_npc_rumor_digest, decay_lexicon, decay_npc_rumors, promote_lexicon_candidates,
};
use crate::game::{Pos, game_time_now, run_view_simulation};
use crate::npc::{GridNpcManager, TravelerManager, seed_traveler_manager};
use crate::npcnpc::{topic_mask_for_room, try_trigger_npc_npc_in_room};

use super::SessionStore;
use super::hub::Hub;

use helpers::{
    initial_next_idle_trigger, initial_random_dialogue_ticks, periodic_fire, reset_ticks_no_player,
};
use idle::run_idle_wander_section;
use market_economy::run_market_economy_section;
use movement::{run_grid_npc_tick, run_m3_027_north_bank_actor_tick, run_travel_section};
use work::run_job_matching_section;
use world::{inject_event_seed, run_climate_tick, run_game_day_economy, run_npc_pool_tick};

/// 跨 tick 的計時與狀態（對齊既有 閉包內變數）。欄位以 `pub(super)` 開放供子模組存取。
pub(super) struct MainLoopTickState {
    pub(super) random_dialogue_ticks: i32,
    pub(super) last_rumor_decay: Option<Instant>,
    pub(super) last_rumor_digest: Option<Instant>,
    pub(super) last_expense_game_day: i32,
    pub(super) last_spawn_check: Instant,
    pub(super) last_job_matching: Option<Instant>,
    pub(super) travel_tick_count: i32,
    pub(super) idle_tick_count: i32,
    /// 累積到此 tick 數即觸發閒置／巡邏區塊。
    pub(super) next_idle_trigger: i32,
    /// 事件種子注入計時器
    pub(super) last_event_seed: Option<Instant>,
    /// 世界詞典維護計時器
    pub(super) last_lexicon_maintenance: Option<Instant>,
    /// freshness 回復計時器（每 N tick 觸發一次）
    pub(super) last_freshness_reset: Option<Instant>,
    /// 氣候 tick 計時器（每 60 現實秒觸發一次）
    pub(super) last_climate_tick: Option<Instant>,
    /// M3-027 半嶺水岸渡口最小同規則行動者 tick。
    pub(super) last_m3_027_north_bank_actor: Option<Instant>,
    /// 白塔市場需求、供應與物流的短節拍；無玩家時仍會運轉。
    pub(super) last_market_economy: Option<Instant>,
    /// 上一次推送 world_tick 的遊戲時辰（-1 代表尚未推過）
    pub(super) last_broadcast_hour: i32,
}

/// 單次 tick（對齊既有 `game.Loop` 回呼內順序）。
fn run_simulation_tick(
    sessions: &SessionStore,
    hub: &Arc<Hub>,
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

    // 時辰改變時推 world_tick time（推送後更新快取，不每 tick 推）
    {
        let should_broadcast = {
            let Ok(mut st) = state.lock() else { return };
            if st.last_broadcast_hour != hour {
                st.last_broadcast_hour = hour;
                true
            } else {
                false
            }
        };
        if should_broadcast {
            let hour_str = format!("{:02}:00", hour);
            let msg = serde_json::json!({
                "type": "world_tick",
                "time": { "hour_str": hour_str, "hour": hour }
            });
            if let Ok(bytes) = serde_json::to_vec(&msg) {
                hub.broadcast(bytes);
            }
            crate::server::grid_manager::sync_canonical_white_tower_npcs(hour);
            let mut active_rooms = sessions.player_room_ids();
            for pid in sessions.all_player_ids() {
                if let Ok(Some(ent)) = crate::db::get_entity(&pid)
                    && let (Some(gx), Some(gy)) = (ent.grid_x, ent.grid_y)
                {
                    active_rooms.insert(crate::grid::grid_room_id_from_coord(gx, gy));
                }
            }
            for rid in active_rooms {
                super::broadcast::refresh_room_views_for_room(sessions, cfg, &rid);
            }
        }
    }

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
        let Ok(mut st) = state.lock() else {
            return;
        };
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

    let ai_ready = !cfg.ollama_disable;
    let mut do_trigger: Option<(String, i32)> = None;
    {
        let Ok(mut st) = state.lock() else {
            return;
        };
        st.random_dialogue_ticks -= 1;
        if st.random_dialogue_ticks <= 0 && ai_ready {
            let player_room_set = sessions.player_room_ids();
            let offscreen_rooms: Vec<String> = if let Some(store) = crate::store::get_store() {
                let st_guard = store.read();
                let mut counts: HashMap<String, usize> = HashMap::new();
                for e in st_guard.entities.values() {
                    if e.kind == "npc" && e.vit > 0 {
                        let rid = st_guard.get_entity_room(&e.id);
                        if !rid.is_empty() {
                            *counts.entry(rid).or_insert(0) += 1;
                        }
                    }
                }
                counts
                    .into_iter()
                    .filter(|(rid, count)| *count >= 2 && !player_room_set.contains(rid))
                    .map(|(rid, _)| rid)
                    .collect()
            } else {
                Vec::new()
            };

            let mut rng = rand::rng();
            if !player_room_set.is_empty() {
                let pick_player_room = offscreen_rooms.is_empty() || rng.random_range(0..100) < 70;
                let chosen_room = if pick_player_room {
                    let mut rooms: Vec<String> = player_room_set.into_iter().collect();
                    let idx = rng.random_range(0..rooms.len());
                    rooms.swap_remove(idx)
                } else {
                    let idx = rng.random_range(0..offscreen_rooms.len());
                    offscreen_rooms[idx].clone()
                };

                let rdt = &sim().random_npc_dialogue_ticks;
                let mut rsp = rdt.reset_span_with_player;
                if rsp <= 0 {
                    rsp = 1;
                }
                st.random_dialogue_ticks = rdt.reset_min_with_player + rng.random_range(0..rsp);
                do_trigger = Some((chosen_room, hour));
            } else if !offscreen_rooms.is_empty() {
                let idx = rng.random_range(0..offscreen_rooms.len());
                let chosen_room = offscreen_rooms[idx].clone();
                st.random_dialogue_ticks = reset_ticks_no_player(cfg);
                do_trigger = Some((chosen_room, hour));
            } else {
                st.random_dialogue_ticks = reset_ticks_no_player(cfg);
            }
        } else if st.random_dialogue_ticks <= 0 && !ai_ready {
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

    // freshness 回復（每 10 秒觸發，離格玩家對應格回 fresh）
    let fire_freshness = {
        let Ok(mut st) = state.lock() else { return };
        periodic_fire(&mut st.last_freshness_reset, Duration::from_secs(10))
    };
    if fire_freshness {
        use crate::db::reset_freshness_for_vacant_cells;
        // 取所有玩家當前的方格座標作為「有人」座標
        let occupied: Vec<(i32, i32)> = sessions
            .player_room_ids()
            .iter()
            .filter_map(|rid| crate::grid::parse_grid_room_id(rid).map(|c| (c.x, c.y)))
            .collect();
        if occupied.is_empty() {
            // 正式白塔世界不再讓退役格網的 freshness 形成背景寫入。
        } else {
            if let Err(e) = reset_freshness_for_vacant_cells(&occupied) {
                tracing::warn!("reset_freshness_for_vacant_cells: {e}");
            }
            // 同步更新記憶體格網（讓記憶體與 PG 一致）
            crate::server::grid_manager::with_square_grid_mut_in_memory(|grid| {
                for cell in grid.cells_mut() {
                    let coord = (cell.coord.x, cell.coord.y);
                    if cell.freshness != crate::grid::Freshness::Fresh && !occupied.contains(&coord)
                    {
                        cell.freshness = crate::grid::Freshness::Fresh;
                    }
                }
            });
        }
    }

    // 氣候 tick（每 60 秒觸發一次）
    let fire_climate = {
        let Ok(mut st) = state.lock() else { return };
        periodic_fire(&mut st.last_climate_tick, Duration::from_secs(60))
    };
    if fire_climate {
        let climate_changes = run_climate_tick(now_unix);
        // 氣候事件變化時推 world_tick weather（每次變化獨立推，不合批）
        for (region_id, event_type, prose_seed) in climate_changes {
            let msg = serde_json::json!({
                "type": "world_tick",
                "weather": {
                    "region_id": region_id,
                    "event_type": event_type,
                    "prose_seed": prose_seed
                }
            });
            if let Ok(bytes) = serde_json::to_vec(&msg) {
                hub.broadcast(bytes);
            }
        }
    }

    run_game_day_economy(now_unix, game_day, state);
    run_market_economy_section(sessions, cfg, now_unix, state);
    run_m3_027_north_bank_actor_tick(sessions, cfg, now_unix, state);
    run_npc_pool_tick(cfg, now_unix, state, traveler_mgr);
    run_job_matching_section(sessions, cfg, now_unix, state, traveler_mgr);
    run_travel_section(sessions, cfg, hour, now_unix, state, traveler_mgr);
    run_grid_npc_tick(sessions, cfg, now_unix, hour, game_day, state, grid_npc_mgr);
    run_idle_wander_section(sessions, cfg, hour, state);
}

/// 在 Tokio runtime 上依 `cfg.tick_interval_ms` 週期執行主迴圈片段（與 Axum 並行）。
pub fn spawn_simulation_main_loop(sessions: Arc<SessionStore>, hub: Arc<Hub>, cfg: Server) {
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
        travel_tick_count: 0,
        idle_tick_count: 0,
        next_idle_trigger: initial_next_idle_trigger(),
        last_event_seed: None,
        last_lexicon_maintenance: None,
        last_freshness_reset: None,
        last_climate_tick: None,
        last_m3_027_north_bank_actor: None,
        last_market_economy: None,
        last_broadcast_hour: -1,
    }));
    let tick_ms = cfg.tick_interval_ms.max(1);
    let hub = Arc::clone(&hub);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(tick_ms));
        loop {
            interval.tick().await;
            let sessions = Arc::clone(&sessions);
            let hub = Arc::clone(&hub);
            let cfg = cfg.clone();
            let tick_state = Arc::clone(&tick_state);
            let tm = Arc::clone(&traveler_mgr);
            let gm = Arc::clone(&grid_npc_mgr);
            let res = tokio::task::spawn_blocking(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_simulation_tick(&sessions, &hub, &cfg, &tick_state, &tm, &gm);
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

//! 世界節拍：每日經濟結算、事件種子注入、NPC 池補滿。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::config::{sim, Server};
use crate::db::{
    deduct_daily_expense, get_npc_ids_with_room, get_player_ids_with_room, get_room,
    get_room_count, get_spawn_room_id, spawn_one_npc_from_pool, upsert_npc_rumor,
};
use crate::gametext;
use crate::npc::{MovementDef, MovementType, TravelerManager};
use crate::store::NpcRumor;

use super::MainLoopTickState;

/// 遊戲日換日時扣鎂並可寫經濟 pulse 傳聞。
pub(super) fn run_game_day_economy(
    now_unix: i64,
    game_day: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let should = {
        let Ok(mut st) = state.lock() else { return; };
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

/// 事件種子注入：從 event_seeds.json 隨機挑選一條注入 rumor 系統，防止語義坍縮。
pub(super) fn inject_event_seed(now_unix: i64) {
    use std::sync::OnceLock;

    #[derive(serde::Deserialize)]
    struct SeedFile {
        seeds: Vec<Seed>,
        #[allow(dead_code)]
        inject_interval_game_hours: Option<i32>,
    }
    #[derive(serde::Deserialize, Clone)]
    struct Seed {
        #[allow(dead_code)]
        category: String,
        text: String,
    }

    static SEEDS: OnceLock<Vec<Seed>> = OnceLock::new();
    static USED: Mutex<Vec<i64>> = Mutex::new(Vec::new());

    let seeds = SEEDS.get_or_init(|| {
        let path = std::path::Path::new("data/config/event_seeds.json");
        match std::fs::read_to_string(path) {
            Ok(text) => match serde_json::from_str::<SeedFile>(&text) {
                Ok(f) => f.seeds,
                Err(e) => {
                    tracing::warn!("event_seeds.json 解析失敗: {e}");
                    vec![]
                }
            },
            Err(_) => vec![],
        }
    });

    if seeds.is_empty() {
        return;
    }

    let cooldown = 48 * 3600i64;
    let Ok(mut used) = USED.lock() else { return };
    if used.len() != seeds.len() {
        used.resize(seeds.len(), 0);
    }

    let available: Vec<usize> = (0..seeds.len())
        .filter(|&i| now_unix - used[i] >= cooldown)
        .collect();

    if available.is_empty() {
        return;
    }

    let mut rng = rand::rng();
    let pick = available[rng.random_range(0..available.len())];
    used[pick] = now_unix;

    let seed = &seeds[pick];
    let rumor = NpcRumor {
        id: format!("seed|{}|{pick}", now_unix / 600),
        text: seed.text.clone(),
        room_id: String::new(),
        zone: String::new(),
        source: "event_seed".into(),
        source_score: 2,
        weight: 3,
        mention_count: 0,
        last_used_at: 0,
        blocked_until: 0,
        penalty_count: 0,
        last_penalty_at: 0,
        last_penalty_reason: String::new(),
        updated_at: now_unix,
        expires_at: now_unix + 3600,
    };
    if let Err(e) = upsert_npc_rumor(rumor) {
        tracing::warn!("event seed upsert_npc_rumor: {e}");
    } else {
        tracing::info!("注入事件種子: {}", seed.text);
    }
}

/// NPC 池補滿；新生成 NPC 註冊為腦驅動 Traveler。
pub(super) fn run_npc_pool_tick(
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    traveler_mgr: &Arc<Mutex<TravelerManager>>,
) {
    let mut pool = cfg.npc_pool_size;
    if pool == 0 {
        pool = (get_room_count() / 2) as i32;
    }
    let spawn_sec = cfg.npc_spawn_interval_sec;
    if pool <= 0 || spawn_sec <= 0 {
        return;
    }
    let fire = {
        let Ok(mut st) = state.lock() else { return; };
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

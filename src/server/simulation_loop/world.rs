//! 世界節拍：每日經濟結算、事件種子注入、NPC 池補滿。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::config::{Server, sim};
use crate::db::{
    deduct_daily_expense, get_npc_ids_with_room, get_player_ids_with_room, get_room_count,
    get_spawn_room_id, spawn_one_npc_from_pool, upsert_npc_rumor,
};
use crate::gametext;
use crate::npc::TravelerManager;
use crate::store::NpcRumor;

use super::MainLoopTickState;

/// 遊戲日換日時扣鎂並可寫經濟 pulse 傳聞。
pub(super) fn run_game_day_economy(
    now_unix: i64,
    game_day: i32,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let should = {
        let Ok(mut st) = state.lock() else {
            return;
        };
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
    if let Some(pool) = crate::pg::pool() {
        match crate::pg::world_resource::regenerate_all(&pool, game_day, now_unix) {
            Ok(report) if report.quantity_created > 0 => tracing::info!(
                "white tower resources: {} sources regenerated, +{} units",
                report.sources_changed,
                report.quantity_created
            ),
            Ok(_) => {}
            Err(error) => tracing::warn!("white tower resource regeneration: {error}"),
        }
    }
    if let Err(e) = deduct_daily_expense(game_day, now_unix) {
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

/// 事件種子注入：結合區域氣候與居民義務，合成活體世界傳聞，防止語義坍縮。
pub(super) fn inject_event_seed(now_unix: i64) {
    let mut rumor_text = String::new();
    let mut rumor_zone = String::new();

    // 1. 優先檢查開立中的居民義務
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut conn) = pool.get()
    {
            let ob_row = conn.query_opt(
                "SELECT entity_id, obligation_kind, subject_ref, source_ref
                 FROM resident_obligations
                 WHERE status IN ('active', 'open')
                   AND obligation_kind IN ('repair_tool', 'stranded_cargo', 'repay_debt', 'treat_injury')
                 ORDER BY updated_at DESC LIMIT 1",
                &[],
            ).unwrap_or(None);
            if let Some(row) = ob_row {
                let entity_id: String = row.get(0);
                let kind: String = row.get(1);
                let subject: String = row.get(2);

                let fallback_room = crate::db::get_entity_room(&entity_id).unwrap_or_default();
                let (place_name, resolved_zone) = crate::store::get_store()
                    .map(|s| {
                        let st = s.read();
                        if let Some(p) = st.get_place(&subject) {
                            (p.name.clone(), p.id.clone())
                        } else if let Some(p) = st.get_place(&fallback_room) {
                            (p.name.clone(), p.id.clone())
                        } else {
                            ("附近聚落".to_string(), fallback_room.clone())
                        }
                    })
                    .unwrap_or_else(|| ("附近聚落".to_string(), fallback_room));

                let entity_name = crate::store::get_store()
                    .and_then(|s| {
                        s.read().entities.get(&entity_id).map(|e| {
                            let name = crate::store::resolve_npc_person_name(&e.id, &e.display_title, None);
                            if !name.is_empty() && name != "陌生人" {
                                name
                            } else if !e.display_title.trim().is_empty() {
                                e.display_title.rsplit(['|', '｜']).next().unwrap_or(&e.display_title).trim().to_string()
                            } else {
                                e.display_char.clone()
                            }
                        })
                    })
                    .unwrap_or_else(|| entity_id.clone());

                rumor_text = match kind.as_str() {
                    "repair_tool" => format!("聽聞{}在{}忙於整修受損器物，正尋求可用資材。", entity_name, place_name),
                    "stranded_cargo" => format!("聽說{}的貨物因路況受阻滯留在{}，急需設法轉運。", entity_name, place_name),
                    "repay_debt" => format!("聽聞{}正在{}四處張羅，設法籌措欠款。", entity_name, place_name),
                    "treat_injury" => format!("據說{}在{}休養療傷，暫緩了手頭事務。", entity_name, place_name),
                    _ => format!("有人看見{}在{}為手頭事務奔波探詢。", entity_name, place_name),
                };
                rumor_zone = resolved_zone;
            }
    }

    // 2. 若無義務，則檢視區域氣候合成路況/天候傳聞
    if rumor_text.is_empty()
        && let Ok(climates) = crate::db::list_region_climates()
        && let Some(c) = climates
            .iter()
            .find(|c| matches!(c.event_type.as_str(), "rain" | "plum_rain" | "snow" | "wind"))
    {
                let weather_desc = match c.event_type.as_str() {
                    "rain" => "連綿陰雨、路面泥濘濕滑",
                    "plum_rain" => "梅雨連日不歇、行路受阻",
                    "snow" => "風雪交加、路徑難行",
                    "wind" => "狂風肆虐、途經者皆需謹慎",
                    _ => "天候有變",
                };
                let region_label = crate::store::get_store()
                    .and_then(|s| {
                        let st = s.read();
                        st.place_graph
                            .places()
                            .find(|p| p.zone == c.region_id)
                            .map(|p| p.player_title().to_string())
                    })
                    .unwrap_or_else(|| c.region_id.clone());
                rumor_text = format!("往來旅人回報，{}周邊{}，行商車隊宜多加留意。", region_label, weather_desc);
                rumor_zone = c.region_id.clone();
    }

    if rumor_text.is_empty() {
        rumor_text = "據聞半島各處近來商旅行人往來漸多，市面動靜頻繁。".to_string();
    }

    let rumor = NpcRumor {
        id: format!("world_seed|{}", now_unix / 600),
        text: rumor_text.clone(),
        room_id: String::new(),
        zone: rumor_zone,
        source: "living_world".into(),
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
        tracing::warn!("world seed upsert_npc_rumor: {e}");
    } else {
        tracing::info!("注入活體世界傳聞: {}", rumor_text);
    }
}
/// NPC 池補滿；新人物立即進入白塔半島，之後由 PlaceGraph 居民迴圈接手。
pub(super) fn run_npc_pool_tick(
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
    _traveler_mgr: &Arc<Mutex<TravelerManager>>,
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
        let Ok(mut st) = state.lock() else {
            return;
        };
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
    let runtime_spawn = match crate::store::ensure_entity_in_white_tower(&new_id) {
        Ok(place_id) => place_id,
        Err(error) => {
            tracing::warn!("new npc white tower placement: {error}");
            return;
        }
    };
    let zone = crate::store::get_store()
        .and_then(|store| {
            store
                .read()
                .get_place(&runtime_spawn)
                .map(|place| place.zone.clone())
        })
        .unwrap_or_default();
    let mut ttl = sim().spawn_rumor_ttl_sec;
    if ttl <= 0 {
        ttl = 3600;
    }
    let rumor = NpcRumor {
        id: format!("spawn|{runtime_spawn}|{new_id}"),
        text: gametext::npc_social().newcomer_rumor,
        room_id: runtime_spawn,
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

/// 荒野自然資源回復與野地營火衰減
pub(super) fn run_wilderness_ecology_and_campfire_tick(now_unix: i64) {
    let modified_cells = crate::server::grid_manager::with_square_grid_mut(|g| {
        let mut modified = Vec::new();
        for cell in g.cells_mut() {
            let mut changed = false;
            for obj in &mut cell.objects {
                if obj.quantity < obj.max_quantity && obj.regrowth_per_tick > 0 {
                    obj.quantity = (obj.quantity + obj.regrowth_per_tick).min(obj.max_quantity);
                    changed = true;
                }
            }
            if changed {
                modified.push((
                    cell.coord.x,
                    cell.coord.y,
                    cell.explored,
                    cell.freshness,
                    cell.objects.clone(),
                ));
            }
        }
        modified
    })
    .unwrap_or_default();

    for (x, y, explored, freshness, objects) in modified_cells {
        if let Err(e) = crate::db::update_cell_state(x, y, explored, freshness, &objects) {
            tracing::warn!("update_cell_state failed: {e}");
        }
    }

    {
        let mut ledger = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .write()
            .unwrap_or_else(|e| e.into_inner());
        ledger.active_campfires.retain(|_room, state| {
            if now_unix.saturating_sub(state.updated_at) >= 180 {
                state.fuel_level = state.fuel_level.saturating_sub(1);
                state.updated_at = now_unix;
                if state.fuel_level == 1 {
                    state.is_charcoal = true;
                }
            }
            state.fuel_level > 0
        });
    }
}

/// 氣候 tick：每 60 秒觸發，對每個區域隨機決定是否轉換氣候。
/// 氣候事件類型：`clear` / `rain` / `wind` / `plum_rain` / `snow`
/// 回傳：有氣候變化的 `(region_id, event_type, prose_seed)` 清單（空代表本輪無變化）。
pub(super) fn run_climate_tick(now_unix: i64) -> Vec<(String, String, i64)> {
    run_wilderness_ecology_and_campfire_tick(now_unix);
    use crate::db::{RegionClimate, list_region_climates, upsert_region_climate};
    use rand::Rng;

    const CLIMATE_EVENTS: &[&str] = &["clear", "rain", "wind", "plum_rain", "snow"];
    /// 每次 tick 氣候轉換概率（可調）
    const CHANGE_PROB: f64 = 0.05;

    let mut climates = match list_region_climates() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("run_climate_tick list: {e}");
            return vec![];
        }
    };

    // 若 region_climate 尚未初始化種子，從 PlaceGraph 自動建立各區域初始天候
    if climates.is_empty()
        && let Some(store) = crate::store::get_store()
    {
        let region_ids: Vec<String> = {
            let st = store.read();
            let mut ids: Vec<String> = st
                .place_graph
                .places()
                .map(|p| p.zone.clone())
                .filter(|r| !r.trim().is_empty())
                .collect();
            ids.sort();
            ids.dedup();
            ids
        };
        for (idx, region_id) in region_ids.into_iter().enumerate() {
            let initial_event = if idx % 5 == 1 { "rain" } else { "clear" };
            let seeded = RegionClimate {
                region_id,
                event_type: initial_event.to_string(),
                started_at_unix: now_unix,
                prose_seed: (now_unix ^ (idx as i64 * 997)) & 0x7fff_ffff,
            };
            if upsert_region_climate(&seeded).is_ok() {
                climates.push(seeded);
            }
        }
    }

    let mut changed = Vec::new();
    let mut rng = rand::rng();
    for climate in climates {
        let roll: f64 = rng.random();
        if roll < CHANGE_PROB {
            let new_event = CLIMATE_EVENTS[rng.random_range(0..CLIMATE_EVENTS.len())];
            let prose_seed: i64 = rng.random();
            let updated = RegionClimate {
                region_id: climate.region_id.clone(),
                event_type: new_event.to_string(),
                started_at_unix: now_unix,
                prose_seed,
            };
            if let Err(e) = upsert_region_climate(&updated) {
                tracing::warn!("run_climate_tick upsert {}: {e}", climate.region_id);
            } else {
                tracing::info!(
                    "[climate] 區域 {} 氣候轉換 {} → {}",
                    climate.region_id,
                    climate.event_type,
                    new_event
                );
                changed.push((climate.region_id.clone(), new_event.to_string(), prose_seed));
                if matches!(new_event, "rain" | "plum_rain" | "snow") {
                    let weather_desc = match new_event {
                        "rain" => "開始降下陣雨，道路變得濕滑泥濘",
                        "plum_rain" => "連綿梅雨籠罩，天候潮濕低迷",
                        "snow" => "飄起陣陣飛雪，寒氣侵襲路途",
                        _ => "天候生變",
                    };
                    let region_label = match climate.region_id.as_str() {
                        "wt-n-sacred-mountain" => "神山",
                        "wt-n-north-crown-sea" => "北冕海",
                        "wt-w-windship-island" => "風舶嶼",
                        "wt-w-natural-harbor-channel" => "天然港灣道",
                        "wt-w-windship-cape" => "風舶角",
                        "wt-c-wenyao" => "文曜",
                        "wt-c-sacred-tower" => "白塔山麓",
                        "wt-e-green-ridge" => "青脊山",
                        "wt-e-east-sea" => "東海",
                        "wt-c-southwest-plain" => "西南原",
                        "wt-s-south-sentinel" => "南哨",
                        "wt-s-black-marsh" => "黑沼林",
                        "wt-s-qinglong-pass" => "青籠關",
                        other => other,
                    };
                    let rumor = NpcRumor {
                        id: format!("weather|{}|{}", climate.region_id, now_unix / 300),
                        text: format!("{}一帶{}，提醒行經該處者提早因應。", region_label, weather_desc),
                        room_id: String::new(),
                        zone: climate.region_id.clone(),
                        source: "climate".into(),
                        source_score: 3,
                        weight: 4,
                        mention_count: 0,
                        last_used_at: 0,
                        blocked_until: 0,
                        penalty_count: 0,
                        last_penalty_at: 0,
                        last_penalty_reason: String::new(),
                        updated_at: now_unix,
                        expires_at: now_unix + 1800,
                    };
                    if let Err(e) = upsert_npc_rumor(rumor) {
                        tracing::warn!("weather change upsert_npc_rumor: {e}");
                    }
                }
            }
        }
    }
    changed
}

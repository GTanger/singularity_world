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

/// 依場域名稱與時間嘗試合成當前活躍事件包之世界傳聞
pub(crate) fn synthesize_active_event_pack_world_rumor(
    room_name: &str,
    now_unix: i64,
) -> Option<(String, String)> {
    let (ev, sp) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
        room_name.trim(),
        None,
        now_unix,
    )?;
    let host_title = crate::grid::resolve_any_room_venue_object(room_name.trim(), "")
        .map(|v| v.default_host_title)
        .unwrap_or("當值主事");
    let first_key = ev
        .key_objects
        .split('、')
        .next()
        .unwrap_or(ev.key_objects)
        .trim();
    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        room_name.trim(),
        first_key,
    );
    let text = format!(
        "聽聞{}的{}正圍繞【{}】忙著{}，往來街坊議論紛紛。",
        room_name.trim(), host_title, first_key, sp.title
    );
    Some((format!("event_pack|{}", ev.id), text))
}

/// 事件種子注入：結合區域氣候與居民義務，合成活體世界傳聞，防止語義坍縮。
pub(super) fn inject_event_seed(now_unix: i64) {
    let mut seed_rumor_id: Option<String> = None;
    let mut rumor_text = String::new();
    let mut rumor_zone = String::new();

    // 1. 優先檢查開立中的居民義務
    if let Some(pool) = crate::pg::pool()
        && let Ok(mut conn) = pool.get()
    {
        if let Ok(expired_rows) = conn.query(
            "UPDATE resident_obligations
             SET status = 'failed', updated_at = $1
             WHERE status IN ('active', 'open')
               AND obligation_kind != 'employment'
               AND (entity_id LIKE 'npc_%' OR entity_id LIKE 'wt_npc_%')
               AND ((due_at IS NOT NULL AND due_at < $1) OR (due_at IS NULL AND updated_at < $1 - 14400))
             RETURNING entity_id, obligation_kind",
            &[&now_unix],
        ) {
            if !expired_rows.is_empty() {
                let mut map = crate::store::LIVE_NPC_OBLIGATIONS.write();
                for r in expired_rows {
                    let eid: String = r.get(0);
                    let okind: String = r.get(1);
                    if okind == "repay_debt" || okind == "stranded_cargo" {
                        continue;
                    }
                    if let Some(list) = map.get_mut(&eid) {
                        list.retain(|(k, _)| k != &okind);
                        if list.is_empty() {
                            map.remove(&eid);
                        }
                    }
                }
            }
        }
        let _ = conn.execute(
            "DELETE FROM resident_obligations WHERE entity_id LIKE '%test%'",
            &[],
        );
        let stale_cutoff = now_unix - 300;
        let _ = conn.execute(
            "DELETE FROM resident_obligations WHERE status IN ('completed', 'failed') AND obligation_kind != 'employment' AND updated_at < $1",
            &[&stale_cutoff],
        );
        let _ = conn.execute(
            "DELETE FROM resident_obligations WHERE obligation_id ~ '^ob:[^:]+:[^:]+:[0-9]+$'",
            &[],
        );
        let ob_row = conn.query_opt(
                "SELECT entity_id, obligation_kind, subject_ref, source_ref
                 FROM (
                     SELECT DISTINCT ON (obligation_kind) entity_id, obligation_kind, subject_ref, source_ref
                     FROM resident_obligations
                    WHERE status IN ('active', 'open')
                      AND entity_id NOT LIKE '%test%'
                      AND EXISTS (SELECT 1 FROM entities e WHERE e.id = resident_obligations.entity_id)
                      AND obligation_kind IN ('repair_tool', 'stranded_cargo', 'repay_debt', 'treat_injury', 'restock_goods', 'market_delivery', 'supply_shortage')
                    ORDER BY obligation_kind, RANDOM()
                 ) sub
                 ORDER BY RANDOM() LIMIT 1",
                &[],
            ).unwrap_or(None);
            if let Some(row) = ob_row {
                let entity_id: String = row.get(0);
                let kind: String = row.get(1);
                let subject: String = row.get(2);

                let fallback_room = crate::db::get_entity_room(&entity_id).unwrap_or_default();
                let extracted_subj_place = subject
                    .strip_prefix("受阻於")
                    .and_then(|s| {
                        s.strip_suffix("的急貨待援")
                            .or_else(|| s.strip_suffix("的隨行貨擔"))
                    })
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());
                let fallback_room_name = crate::db::get_room_name(&fallback_room)
                    .ok()
                    .filter(|n| !n.is_empty() && !n.starts_with("grid:") && !n.starts_with("wt-physical-"));
                let (mut place_name, mut resolved_zone) = crate::store::get_store()
                    .map(|s| {
                        let st = s.read();
                        if let Some(p) = st.get_place(&subject) {
                            (p.player_title().to_string(), p.zone.clone())
                        } else if let Some(p) = st.get_place(&fallback_room) {
                            (p.player_title().to_string(), p.zone.clone())
                        } else if let Some(sp) = extracted_subj_place.clone() {
                            (sp, fallback_room.clone())
                        } else if let Some(rn) = fallback_room_name.clone() {
                            (rn, fallback_room.clone())
                        } else {
                            ("白塔驛道".to_string(), fallback_room.clone())
                        }
                    })
                    .unwrap_or_else(|| {
                        (
                            extracted_subj_place
                                .or(fallback_room_name)
                                .unwrap_or_else(|| "白塔驛道".to_string()),
                            fallback_room,
                        )
                    });
                let entity_name = crate::store::get_store()
                    .and_then(|s| {
                        let guard = s.read();
                        guard.entities.get(&entity_id).map(|e| {
                            crate::store::resolve_npc_person_name(&e.id, &e.display_title, Some(&e.gender))
                        })
                    })
                    .unwrap_or_default();
                if entity_name.is_empty()
                    || entity_name == "陌生人"
                    || entity_name.chars().any(|c| c.is_ascii_alphabetic())
                {
                    return;
                }
                if resolved_zone.starts_with("grid:") || resolved_zone.is_empty() {
                    if let Some((prefix, _)) = place_name.split_once('・') {
                        resolved_zone = prefix.trim().to_string();
                    } else {
                        resolved_zone = "wt-c-wenyao".to_string();
                    }
                }

                place_name = crate::store::clean_wilderness_place_label(&place_name);
                for road_suffix in &["・道路", "・山道", "・林道", "・石徑", "・步道"] {
                    if place_name.ends_with(road_suffix) {
                        place_name = place_name.strip_suffix(road_suffix).unwrap_or(&place_name).to_string();
                        break;
                    }
                }

                let raw_settlement = place_name.split('・').next().unwrap_or("文曜").trim().to_string();
                let settlement = crate::pg::living_world::peninsula_market_settlement_name(&raw_settlement).to_string();
                let canonical_settlements = [
                    "文曜", "半嶺鎮", "風舶角", "南哨", "大圳頭", "紅土崁", "北嶺腳", "風櫃尾",
                    "下營", "青竹坪", "牛埔", "烏石鼻", "霜林口", "三號坑口", "水尾寮", "老堰頭",
                    "三水口", "風舶嶼",
                ];
                let canonical_settlement = if canonical_settlements.contains(&settlement.as_str()) {
                    settlement.as_str()
                } else {
                    "文曜"
                };
                if place_name == raw_settlement && settlement != raw_settlement {
                    place_name = format!("{settlement}外道");
                } else if settlement != raw_settlement
                    && let Some(rest) = place_name.strip_prefix(&format!("{raw_settlement}・"))
                {
                    place_name = format!("{settlement}・{rest}");
                }
                let venue_leaf = place_name.rsplit('・').next().unwrap_or(&place_name).trim();
                let is_settlement_or_venue = canonical_settlements.contains(&raw_settlement.as_str())
                    || [
                        "工坊", "鐵器", "船塢", "商行", "鋪", "店", "館", "堂", "市集", "居",
                        "宅", "舍", "齋", "院", "閣", "所", "屋", "寮", "房", "棚",
                    ]
                    .iter()
                    .any(|k| place_name.contains(k));
                let is_wilderness_or_road = !is_settlement_or_venue
                    && [
                        "林", "谷", "嶺", "山", "溪", "澤", "沼", "灘", "坡", "道", "徑", "橋",
                        "路", "草甸", "岩岸", "峭壁", "原", "岬", "灣", "關", "麓", "洲", "嶼",
                        "坑",
                    ]
                    .iter()
                    .any(|k| place_name.contains(k));
                let has_indoor_suffix = [
                    "鋪", "店", "工坊", "商行", "食堂", "茶屋", "酒坊", "醫館", "藥鋪", "診所", "書閣", "印行", "船塢", "灰窯", "鹽場", "市集",
                ]
                .iter()
                .any(|s| venue_leaf.ends_with(s));
                let is_outdoor_street_or_square = !has_indoor_suffix
                    && [
                        "街", "巷", "弄", "道", "路", "段", "口", "橋", "門", "坪", "廣場", "岸", "階", "亭", "井", "哨",
                    ]
                    .iter()
                    .any(|k| venue_leaf.contains(k));
                let venue_opt = crate::grid::resolve_any_room_venue_object(&place_name, "");
                let mut is_workshop = !is_outdoor_street_or_square
                    && [
                        "工坊", "鐵器", "鍛", "木工", "石工", "船塢", "窯", "廠", "匠", "工具", "農具",
                    ]
                    .iter()
                    .any(|k| venue_leaf.contains(k));
                let mut is_shop_or_commercial = !is_outdoor_street_or_square
                    && [
                        "鋪", "店", "行", "坊", "館", "堂", "閣", "市集", "塢", "倉", "庫", "局", "社", "攤", "窯", "廠",
                    ]
                    .iter()
                    .any(|k| venue_leaf.contains(k));
                if let Some(v) = &venue_opt {
                    let tags = v.semantic_tags();
                    is_workshop =
                        is_workshop || tags.contains(&"workshop") || tags.contains(&"blacksmith");
                    is_shop_or_commercial = is_shop_or_commercial
                        || v.has_counter_restricted_zone
                        || tags.contains(&"market")
                        || tags.contains(&"shop");
                }
                let is_residential = [
                    "居", "宅", "舍", "宿", "值房", "別院", "內室", "客房",
                ]
                .iter()
                .any(|k| venue_leaf.contains(k));
                let has_building_noun = has_indoor_suffix
                    || is_residential
                    || is_workshop
                    || is_shop_or_commercial
                    || ["屋", "居", "宅", "舍", "齋", "院", "閣", "所", "寮", "房", "棚", "亭", "哨"]
                        .iter()
                        .any(|k| venue_leaf.contains(k));
                let is_pure_water_or_mud = !has_building_noun
                    && ["海灣", "泥口", "海面", "水道", "淺灘", "外海", "河口", "潮池", "潮間帶", "淺灣", "礫灣", "岩岸", "海蝕", "泥澤", "深潭", "沙洲", "白茫洲"]
                        .iter()
                        .any(|k| venue_leaf.contains(k));
                let is_wilderness_or_road = is_wilderness_or_road || is_pure_water_or_mud;
                let sanitized_place = if is_pure_water_or_mud && matches!(kind.as_str(), "repair_tool" | "stranded_cargo" | "restock_goods" | "supply_shortage" | "repay_debt" | "market_delivery") {
                    format!("{canonical_settlement}外山道")
                } else if kind == "repair_tool" && !is_wilderness_or_road && !is_workshop {
                    format!("{canonical_settlement}・街坊工坊")
                } else if kind == "supply_shortage" && !is_wilderness_or_road && (!is_shop_or_commercial || is_residential) {
                    format!("{canonical_settlement}・市集鋪面")
                } else if kind == "restock_goods" && !is_wilderness_or_road && (!is_shop_or_commercial || is_residential) {
                    format!("{canonical_settlement}・城內鋪面")
                } else if kind == "stranded_cargo" && is_settlement_or_venue {
                    format!("{canonical_settlement}外山道")
                } else if kind == "treat_injury" && (is_wilderness_or_road || is_outdoor_street_or_square || is_residential) {
                    format!("{canonical_settlement}・街坊醫館")
                } else if kind == "market_delivery" && (is_residential || is_outdoor_street_or_square) {
                    format!("{canonical_settlement}・市集鋪面")
                } else {
                    place_name.clone()
                };
                rumor_text = match kind.as_str() {
                    "repair_tool" => {
                        if is_wilderness_or_road {
                            format!("聽聞{}行經{}時隨身器物受損，正尋求可用資材修整。", entity_name, sanitized_place)
                        } else {
                            format!("聽聞{}在{}忙於整修受損器物，正尋求可用資材。", entity_name, sanitized_place)
                        }
                    }
                    "stranded_cargo" => format!("聽說{}的貨物因路況受阻滯留在{}，急需設法轉運。", entity_name, sanitized_place),
                    "repay_debt" => {
                        if is_wilderness_or_road {
                            format!("有人瞧見{entity_name}行經{sanitized_place}奔波趕路，說是正設法張羅籌措欠款。")
                        } else {
                            format!("聽聞{entity_name}正在{sanitized_place}四處張羅，設法籌措欠款。")
                        }
                    }
                    "treat_injury" => format!("據說{}在{}休養療傷，暫緩了手頭事務。", entity_name, sanitized_place),
                    "restock_goods" => {
                        if is_wilderness_or_road {
                            format!("有人瞧見{entity_name}行經{sanitized_place}趕路，說是鋪內存貨見底，正趕往鄰鎮市集採辦補倉。")
                        } else {
                            format!("聽聞{entity_name}在{sanitized_place}盤點鋪內存貨，正四處打聽補倉的貨源。")
                        }
                    }
                    "market_delivery" => {
                        if is_wilderness_or_road {
                            let market_settlement =
                                crate::game::object_ecology::normalize_canonical_settlement(canonical_settlement);
                            format!("有人瞧見{entity_name}押著貨擔行經{sanitized_place}趕路，說是正趕往{market_settlement}市集交割物產。")
                        } else {
                            format!("有人瞧見{entity_name}押著貨擔往{sanitized_place}一帶趕路，說是趕著交割物產。")
                        }
                    }
                    "supply_shortage" => {
                        if is_wilderness_or_road {
                            format!("有人瞧見{entity_name}行經{sanitized_place}趕路，說是為調度緊缺物資正趕往鄰鎮市集。")
                        } else {
                            format!("聽聞{entity_name}在{sanitized_place}四處調度緊缺物資，市集上往來詢問的人不少。")
                        }
                    }
                    _ => format!("有人看見{}在{}為手頭事務奔波探詢。", entity_name, sanitized_place),
                };

                rumor_zone = resolved_zone;
                seed_rumor_id = Some(format!("world_seed|{}", kind));
            }
    }
    if rumor_text.is_empty() {
        let candidate_rooms = [
            "老堰頭・水輪石磨工坊",
            "三水大食堂",
            "文曜百鍛鐵坊",
            "公議堂房地契書局",
            "泉香溫水浴場",
            "東橋旅社茶屋",
            "半島百草診所",
            "風舶早市鮮魚拍賣場",
        ];
        let candidate_room = candidate_rooms[(now_unix.unsigned_abs() as usize / 600) % 8];
        if now_unix.rem_euclid(2) == 0 {
            if let Some((rid, rtext)) = synthesize_active_event_pack_world_rumor(candidate_room, now_unix) {
                seed_rumor_id = Some(rid);
                rumor_text = rtext;
            }
        }
    }


    // 2. 若無義務，則檢視區域氣候合成路況/天候傳聞
    let mut is_weather_seed = false;
    if rumor_text.is_empty()
        && let Ok(climates) = crate::db::list_region_climates()
    {
        let active_climates: Vec<_> = climates
            .iter()
            .filter(|c| matches!(c.event_type.as_str(), "rain" | "plum_rain" | "snow" | "wind"))
            .collect();
        if !active_climates.is_empty() {
            let c = active_climates[(now_unix.unsigned_abs() as usize / 600) % active_climates.len()];
                let weather_desc = match c.event_type.as_str() {
                    "rain" => "連綿陰雨、路面泥濘濕滑",
                    "plum_rain" => "梅雨連日不歇、行路受阻",
                    "snow" => "風雪交加、路徑難行",
                    "wind" => "狂風肆虐、途經者皆需謹慎",
                    _ => "天候有變",
                };
                let raw_region_label = crate::store::get_store()
                    .and_then(|s| {
                        let st = s.read();
                        st.place_graph
                            .places()
                            .find(|p| p.zone == c.region_id)
                            .map(|p| p.player_title().to_string())
                    })
                    .unwrap_or_else(|| c.region_id.clone());
                let region_label = crate::pg::living_world::peninsula_market_settlement_name(&raw_region_label);
                rumor_text = format!("往來行旅傳言，{}周邊{}，過路商隊多半放慢了腳程。", region_label, weather_desc);
                rumor_zone = c.region_id.clone();
                is_weather_seed = true;
        }
    }

    if rumor_text.is_empty() {
        rumor_text = "據聞半島各處近來商旅行人往來漸多，市面動靜頻繁。".to_string();
    }

    let rumor = NpcRumor {
        id: if let Some(id) = seed_rumor_id {
            id
        } else if is_weather_seed {
            format!("weather|{}", rumor_zone)
        } else {
            format!("world_seed|{}", now_unix / 600)
        },
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
    let modified_cells = crate::server::grid_manager::with_square_grid_mut_in_memory(|g| {
        let mut modified = Vec::new();
        for cell in g.cells_mut() {
            if crate::grid::zhulin::is_bamboo_slot_cell(cell) {
                continue;
            }
            let mut changed = false;
            for obj in &mut cell.objects {
                if obj.quantity < obj.max_quantity && obj.regrowth_per_tick > 0 {
                    obj.quantity = (obj.quantity + obj.regrowth_per_tick).min(obj.max_quantity);
                    changed = true;
                }
            }
            if changed {
                crate::grid::zhulin::refresh_bamboo_cell_after_gather(cell);
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
            if !ids.iter().any(|r| r == "default") {
                ids.push("default".to_string());
            }
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
        if now_unix.saturating_sub(climate.started_at_unix) < 1200 {
            continue;
        }
        let roll: f64 = rng.random();
        if roll < CHANGE_PROB {
            let new_event = if !climate.region_id.starts_with("wt-") {
                let gh = crate::store::current_game_hour() as u32;
                crate::grid::zhulin::weather::next_zhulin_weather_in_cycle(&climate.event_type, gh)
                    .legacy_key
            } else {
                let can_snow = matches!(
                    climate.region_id.as_str(),
                    "wt-n-sacred-mountain"
                        | "wt-n-north-crown-sea"
                        | "wt-c-sacred-tower"
                        | "wt-e-green-ridge"
                );
                let allowed_events: &[&str] = if can_snow {
                    CLIMATE_EVENTS
                } else {
                    &["clear", "rain", "wind", "plum_rain"]
                };
                allowed_events[rng.random_range(0..allowed_events.len())]
            };
            if new_event == climate.event_type {
                continue;
            }
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
                if matches!(new_event, "clear" | "wind" | "rain" | "plum_rain" | "snow") {
                    let raw_region_label = match climate.region_id.as_str() {
                        "wt-n-sacred-mountain" => "神山",
                        "wt-n-north-crown-sea" => "北冕海",
                        "wt-w-windship-island" => "風舶嶼",
                        "wt-w-natural-harbor-channel" => "天然港灣道",
                        "wt-w-windship-cape" => "風舶角",
                        "wt-c-wenyao" => "文曜",
                        "wt-c-sacred-tower" => "半嶺鎮",
                        "wt-e-green-ridge" => "青脊山",
                        "wt-e-east-sea" => "東海",
                        "wt-c-southwest-plain" => "大圳頭",
                        "wt-s-south-sentinel" => "南哨",
                        "wt-s-black-marsh" => "水尾寮",
                        "wt-s-qinglong-pass" => "青籠關",
                        other => other,
                    };
                    let region_label = crate::pg::living_world::peninsula_market_settlement_name(raw_region_label);
                    let rumor_text = match new_event {
                        "clear" => {
                            let clear_lead = match climate.event_type.as_str() {
                                "snow" => "風雪漸歇、天色放晴",
                                "wind" => "強風漸息、天光開朗",
                                _ => "雨勢漸歇、天色放晴",
                            };
                            format!(
                                "聽聞{}一帶{}，道路已好走許多，往來商旅也跟著加快了腳程。",
                                region_label, clear_lead
                            )
                        }
                        "wind" => format!(
                            "聽聞{}一帶風勢轉勁，過路行旅多半收緊了行裝、留神腳下。",
                            region_label
                        ),
                        "rain" => format!(
                            "聽聞{}一帶開始降下陣雨，道路變得濕滑泥濘，往來行旅得多留神腳下。",
                            region_label
                        ),
                        "plum_rain" => format!(
                            "聽聞{}一帶連綿梅雨籠罩，天候潮濕低迷，往來行旅得多留神腳下。",
                            region_label
                        ),
                        "snow" => format!(
                            "聽聞{}一帶飄起陣陣飛雪，寒氣侵襲路途，往來行旅得多留神腳下。",
                            region_label
                        ),
                        _ => format!("聽聞{}一帶天候生變，往來行旅得多留神腳下。", region_label),
                    };
                    let rumor = NpcRumor {
                        id: format!("weather|{}", climate.region_id),
                        text: rumor_text,
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesize_active_event_pack_world_rumor() {
        let res = synthesize_active_event_pack_world_rumor("老堰頭・水輪石磨工坊", 1_700_000_000);
        assert!(res.is_some());
        let (rid, text) = res.unwrap();
        assert!(rid.starts_with("event_pack|"));
        assert!(text.contains("老堰頭・水輪石磨工坊"));
        assert!(text.contains("街坊議論紛紛"));
    }
}

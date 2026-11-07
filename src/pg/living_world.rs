//! 人物連續性、共同行動收據與來源層級的 PostgreSQL 權威資料。

use std::collections::HashMap;

use r2d2_postgres::postgres::{GenericClient, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::living_world::{ActorKind, SourceRecord, builtin_contract, shared_policy_key};
use crate::store::sql::DbPool;
use crate::world::place::WHITE_TOWER_WORLD_ID;


include!("living_world/schema_and_continuity.rs");
include!("living_world/obligations_and_actions.rs");
include!("living_world/afflictions_and_aux.rs");
include!("living_world/somatic_and_habits.rs");
include!("living_world/residents_seed.rs");
include!("living_world/life_choices_and_factors.rs");
include!("living_world/human_life_loop.rs");
include!("living_world/tests.rs");
/// 一次性建世種子：為全島工坊與住戶匠人初始化器物磨損與吐納基底（所有人皆為自己的主角）
pub fn seed_flagship_specimen_qin_shuisheng_once(now_unix: i64) {
    static ALL_RESIDENTS_SEEDED: std::sync::Once = std::sync::Once::new();
    static SEEDED_ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    ensure_aux_pg_tables();
    let npc_id = "秦水生";

    record_entity_studied_manual(npc_id, "堰渠水輪分動考工譜");
    record_entity_studied_manual(npc_id, "石磨齒距微調與水磨澄漿手札");
    {
        let mut guard = ANATOMY_FAMILIARITY_CACHE.write().unwrap();
        let map = guard.entry(npc_id.to_string()).or_default();
        map.entry("石磨鑿紋與水輪木齒".to_string()).or_insert(28);
        map.entry("在來米與北山藥材水磨澄漿".to_string()).or_insert(36);
    }
    {
        let mut guard = BREATH_HABIT_CACHE.write().unwrap();
        guard.entry(npc_id.to_string()).or_insert_with(|| BreathHabitRecord {
            entity_id: npc_id.to_string(),
            dominant: BreathHabitKind::TideContinuous,
            forge_score: 14,
            forest_score: 6,
            tide_score: 48,
            chamber_score: 8,
            battle_score: 2,
            updated_at: now_unix,
        });
    }
    {
        let mut guard = LINEAGE_IMPRINT_CACHE.write().unwrap();
        let list = guard.entry(npc_id.to_string()).or_default();
        if !list.iter().any(|imp| imp.imprint_title == "老堰頭水輪分動與雙疊石磨聽音辨距法") {
            list.push(LineageImprintRecord {
                entity_id: npc_id.to_string(),
                mentor_name: "秦老堰主".to_string(),
                imprint_title: "老堰頭水輪分動與雙疊石磨聽音辨距法".to_string(),
                imprint_prose: "順水輪潮汐綿長吐納，聽上下磨盤咬合之音以拿捏三毫齒距，分篩純漿而不傷米性".to_string(),
                interaction_count: 12,
                manual_gifted: true,
                updated_at: now_unix,
            });
        }
    }
    {
        let mut guard = VESSEL_STATE_CACHE.write().unwrap();
        let sieve_entry = guard
            .entry((npc_id.to_string(), "wt_cat_0973".to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: npc_id.to_string(),
                item_id: "wt_cat_0973".to_string(),
                use_count: 68,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
        if sieve_entry.wear_points > 0 || sieve_entry.condition != VesselCondition::Sharp {
            sieve_entry.wear_points = 0;
            sieve_entry.condition = VesselCondition::Sharp;
            sieve_entry.updated_at = now_unix;
        }
        guard
            .entry((npc_id.to_string(), "wt_cat_0984".to_string()))
            .or_insert_with(|| VesselStateRecord {
                entity_id: npc_id.to_string(),
                item_id: "wt_cat_0984".to_string(),
                use_count: 42,
                wear_points: 0,
                condition: VesselCondition::Sharp,
                updated_at: now_unix,
            });
    }


    ALL_RESIDENTS_SEEDED.call_once(|| {
        seed_all_residents_somatic_records(now_unix);
    });

    if SEEDED_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        return;
    }

    seed_qin_shuisheng_sql_venues_and_schedules();
}

/// 辨識現場人員：透過裝備槽位可見制服／圍裙／工務裝束或站在管制櫃檯區（非本號夥計禁止入內區）精準辨識值守店員或掌事
pub fn is_recognizable_room_staff_or_counter_holder(
    room_name: &str,
    npc_id: &str,
    display_title: &str,
    current_activity: &str,
    equipment_slots: &str,
) -> (bool, bool, bool) {
    let mut has_visible_uniform = false;
    if let Ok(slots) =
        serde_json::from_str::<std::collections::HashMap<String, String>>(equipment_slots)
    {
        for slot in &["body", "waist", "wrist", "hold_r", "hold_l"] {
            if let Some(item_id) = slots.get(*slot).filter(|s| !s.trim().is_empty()) {
                if crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(item_id) {
                    has_visible_uniform = true;
                    break;
                }
            }
        }
    }
    if !has_visible_uniform {
        if display_title.contains("穿制服")
            || display_title.contains("店員")
            || display_title.contains("制服")
            || display_title.contains("圍裙")
            || display_title.contains("工長")
            || display_title.contains("司秤")
            || display_title.contains("主廚")
        {
            has_visible_uniform = true;
        }
    }
    if !has_visible_uniform
        && equipment_slots.trim().is_empty()
        && !npc_id.is_empty()
        && !display_title.contains("便服")
    {
        if let Some((_, sched_work)) = crate::server::grid_manager::resident_schedule_home_and_work(npc_id) {
            if !sched_work.is_empty()
                && (room_name.trim().is_empty()
                    || room_name.contains(sched_work)
                    || sched_work.contains(room_name.trim()))
            {
                has_visible_uniform = true;
            }
        }
        if !has_visible_uniform {
            let slot = crate::store::wenyao_household_slot_for(npc_id);
            let is_matched_slot = npc_id == slot.name()
                || display_title.contains(slot.name())
                || (!slot.work_name.is_empty() && current_activity.contains(slot.work_name));
            let work_matches_room = room_name.trim().is_empty()
                || (!slot.work_name.is_empty()
                    && (room_name.contains(slot.work_name) || slot.work_name.contains(room_name)));
            if is_matched_slot
                && work_matches_room
                && crate::game::peninsula_catalog::is_work_uniform_item_id_or_name(slot.attire_line)
            {
                has_visible_uniform = true;
            }
        }
    }

    let is_passerby_customer = display_title.contains("顧客")
        || display_title.contains("路人")
        || display_title.contains("茶客")
        || display_title.contains("食客")
        || display_title.contains("香客")
        || display_title.contains("過客")
        || display_title.contains("外地商旅");

    let mut has_counter_zone_marker = display_title.contains("櫃檯")
        || display_title.contains("櫃台")
        || display_title.contains("非夥計禁止入內")
        || display_title.contains("非本店人員禁止進入區")
        || display_title.contains("非本號夥計禁止入內區")
        || display_title.contains("內場")
        || display_title.contains("掌事")
        || display_title.contains("主事")
        || display_title.contains("當值")
        || display_title.contains("首座");

    if !has_counter_zone_marker {
        let mut eff_title = display_title.to_string();
        let mut eff_act = current_activity.to_string();
        if eff_title.is_empty() && eff_act.is_empty() && equipment_slots.is_empty() && !npc_id.is_empty() {
            if let Some(arc) = crate::store::get_store() {
                let s = arc.read();
                if let Some(e) = s.entities.get(npc_id) {
                    eff_title = e.display_title.clone();
                    eff_act = e.current_activity.clone();
                }
            }
        }

        if let Some(v) = crate::grid::resolve_any_room_venue_object(room_name, &eff_title) {
            if v.has_counter_restricted_zone
                && v.venue_kind != "街巷橋樑公共空地"
                && (eff_act.contains("櫃")
                    || eff_act.contains("爐")
                    || eff_act.contains("釜")
                    || eff_act.contains("砧")
                    || eff_act.contains("磨")
                    || eff_act.contains("藥")
                    || eff_act.contains("算")
                    || eff_act.contains("帳")
                    || eff_act.contains("攤"))
            {
                has_counter_zone_marker = true;
            }
        }
    }

    let in_staff_only_counter_zone = if is_passerby_customer && !has_counter_zone_marker {
        false
    } else if has_counter_zone_marker {
        true
    } else if has_visible_uniform
        && !is_passerby_customer
        && crate::grid::resolve_any_room_venue_object(room_name, display_title)
            .is_some_and(|v| v.has_counter_restricted_zone)
    {
        true
    } else {
        false
    };

    let is_staff = has_visible_uniform || in_staff_only_counter_zone;
    (is_staff, has_visible_uniform, in_staff_only_counter_zone)
}

/// 依原點座標 (origin.0, origin.1) 與標籤清單，於方格地圖中搜尋半徑內最近之房間設施
pub fn resolve_nearest_room_by_tag(
    origin: (i32, i32),
    wanted_tags: &[&str],
    grid_opt: Option<&crate::grid::SquareGrid>,
    fallback_coord: (i32, i32),
    fallback_bldg: &str,
) -> ((i32, i32), String) {
    let runtime_grid_holder;
    let grid_ref = match grid_opt {
        Some(g) => Some(g),
        None => {
            runtime_grid_holder = crate::server::grid_manager::get_square_grid();
            runtime_grid_holder.as_ref()
        }
    };

    let Some(grid) = grid_ref else {
        return (fallback_coord, fallback_bldg.to_string());
    };

    let mut best: Option<((i32, i32), String, i32)> = None;
    for cell in grid.cells() {
        let rcoord = (cell.coord.x, cell.coord.y);
        let dist = (rcoord.0 - origin.0).abs() + (rcoord.1 - origin.1).abs();
        if dist > 45 {
            continue;
        }
        let tag_match = wanted_tags.iter().any(|&tag| !tag.is_empty() && cell.has_room_tag(tag));
        if !tag_match {
            continue;
        }

        let is_better = match &best {
            None => true,
            Some((best_coord, _, best_dist)) => {
                if dist < *best_dist {
                    true
                } else if dist == *best_dist {
                    rcoord < *best_coord
                } else {
                    false
                }
            }
        };

        if is_better {
            let bldg_name = if cell.name.trim().is_empty() {
                if !cell.zone.is_empty() {
                    cell.zone.clone()
                } else {
                    fallback_bldg.to_string()
                }
            } else if cell.name.contains('・') || cell.zone.is_empty() {
                cell.name.clone()
            } else {
                format!("{}・{}", cell.zone, cell.name)
            };
            best = Some((rcoord, bldg_name, dist));
        }
    }

    if let Some((b_coord, b_bldg, b_dist)) = best {
        let fb_dist = (fallback_coord.0 - origin.0).abs() + (fallback_coord.1 - origin.1).abs();
        if (fallback_coord == (-1, 124) || fallback_coord == (1, 124)) && fb_dist <= b_dist {
            (fallback_coord, fallback_bldg.to_string())
        } else {
            (b_coord, b_bldg)
        }
    } else {
        (fallback_coord, fallback_bldg.to_string())
    }
}


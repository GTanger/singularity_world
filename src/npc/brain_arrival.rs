//! 腦驅動 NPC 抵達房間後的數值效果（對齊既有 `brain_arrival` 之 `applyBrainArrivalEffects`）。

use crate::config::sim;
use crate::db::{
    EVT_BEG, EVT_GATHER, EVT_HIRED, EVT_TALK, EVT_TRADE, EVT_WORK, ThoughtSource, add_thought,
    add_to_inventory, get_assignment_count_by_venue, get_assignments_for_entity, get_room_name,
    get_runtime_occupation_for_venue, get_venue_ids_for_room, get_venue_max_staff,
    insert_assignment, log_npc_event, npc_street_sell_one,
};
use crate::gametext;
use crate::world::materials;

use super::decision::IntentType;

fn fmt_two_s(fmt: &str, a: &str, b: &str) -> String {
    fmt.replacen("%s", a, 1).replacen("%s", b, 1)
}

/// 依抵達意圖套用鎂／背包／指派／事件／心境（對齊既有）。
pub fn apply_brain_arrival_effects(
    at: i64,
    game_day: i32,
    entity_id: &str,
    room_id: &str,
    intent: IntentType,
) -> Option<String> {
    if intent == IntentType::Work
        && let Some(activity) = try_complete_market_delivery(at, entity_id, room_id)
    {
        return Some(activity);
    }
    let be = gametext::brain_effects();
    match intent {
        IntentType::Beg => {
            // 贈與尚未接到人物共用帳本前，求助只能是可觀測行為，不能憑空生鎂。
            let payload = format!("在{room_id}向身邊的人求助；沒有可核實的贈與成交");
            log_npc_event(at, entity_id, EVT_BEG, &payload);
            let _ = add_thought(entity_id, ThoughtSource::BegSuccess);
        }
        IntentType::Gather => {
            if try_gather_runtime_resource(at, game_day, entity_id, room_id) {
                return None;
            }
            // 白塔舊 P0 資源仍留在資料庫供回查，但不再從相容素材表補生。
            if room_id.starts_with("wt-physical-") {
                return None;
            }
            if let Some(gathered) =
                materials::try_gather_material_for_room_preserving(room_id, at, 1)
            {
                let _ = add_to_inventory(entity_id, &gathered.item_id, 1);
                let room_name = get_room_name(room_id)
                    .ok()
                    .filter(|name| !name.is_empty())
                    .unwrap_or(gathered.source_room_name);
                let payload = format!(
                    "在{room_name}取得{}，{}少了一份。",
                    gathered.name, gathered.origin_label
                );
                log_npc_event(at, entity_id, EVT_GATHER, &payload);
                let _ = add_thought(entity_id, ThoughtSource::Gather);
            }
        }
        IntentType::SeekJob => {
            let Ok(list) = get_assignments_for_entity(entity_id) else {
                return None;
            };
            if !list.is_empty() {
                return None;
            }
            let Ok(venue_ids) = get_venue_ids_for_room(room_id) else {
                return None;
            };
            let fallback_occ = gametext::occupation_waiter();
            let fallback_cap = sim().max_assignments_per_venue.max(0);
            for vid in venue_ids {
                let Ok(n) = get_assignment_count_by_venue(&vid) else {
                    continue;
                };
                let runtime_occ = get_runtime_occupation_for_venue(&vid);
                let occ = runtime_occ.as_deref().unwrap_or(&fallback_occ);
                let cap = get_venue_max_staff(&vid, fallback_cap).max(0) as usize;
                let assigned_by = runtime_occ
                    .as_ref()
                    .map(|_| "runtime:author_economy")
                    .unwrap_or("");
                if n < cap && insert_assignment(entity_id, occ, &vid, assigned_by).is_ok() {
                    let payload = fmt_two_s(&be.hired_event_fmt, room_id, occ);
                    log_npc_event(at, entity_id, EVT_HIRED, &payload);
                    let _ = add_thought(entity_id, ThoughtSource::Hired);
                    break;
                }
            }
        }
        IntentType::Socialize => {
            log_npc_event(at, entity_id, EVT_TALK, &be.talk_social);
            let _ = add_thought(entity_id, ThoughtSource::Talked);
        }
        IntentType::Trade => {
            if !npc_street_sell_one(entity_id, room_id, at) {
                log_npc_event(at, entity_id, EVT_TRADE, &be.trade_no_stock);
            }
        }
        IntentType::Work => try_perform_runtime_work(at, entity_id),
        IntentType::Wander | IntentType::Idle => {}
    }
    None
}

fn try_complete_market_delivery(at: i64, entity_id: &str, place_id: &str) -> Option<String> {
    let pool = crate::pg::pool()?;
    let delivered =
        match crate::pg::world_logistics::complete_if_arrived(&pool, entity_id, place_id, at) {
            Ok(Some(delivered)) => delivered,
            Ok(None) => return None,
            Err(error) => {
                tracing::warn!("market delivery completion failed {entity_id}: {error}");
                return None;
            }
        };
    let payload = format!(
        "把{}份{}送抵{}，貨物正式進入目的市場",
        delivered.quantity, delivered.display_name, delivered.destination_place_id
    );
    log_npc_event(at, entity_id, EVT_WORK, &payload);
    let _ = add_thought(entity_id, ThoughtSource::WorkDone);
    Some(format!(
        "剛把{}份{}送到市場",
        delivered.quantity, delivered.display_name
    ))
}

fn try_perform_runtime_work(at: i64, entity_id: &str) {
    let Some(pool) = crate::pg::pool() else {
        return;
    };
    let result =
        match crate::pg::world_work::perform_shift(&pool, entity_id, at, "npc_assigned_work") {
            Ok(Some(result)) => result,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!("npc runtime work failed {entity_id}: {error}");
                return;
            }
        };
    if let Some(store) = crate::store::get_store()
        && let Err(error) = store.write().mirror_economy_from_authority(
            entity_id,
            result.entity_magnesium,
            &result.inventory,
            &result.current_activity,
        )
    {
        tracing::warn!("npc work cache mirror failed {entity_id}: {error}");
    }
    let payload = if result.wage_mg > 0 {
        if let Some(supply) = &result.supply {
            if supply.delivery_kind == "shipment" {
                format!(
                    "在{}完成{}工作，領到{}鎂，並把{}份{}裝成前往目的市場的實體貨運",
                    result.place_id,
                    result.occupation_id,
                    result.wage_mg,
                    supply.quantity,
                    supply.display_name
                )
            } else {
                format!(
                    "在{}完成{}工作，領到{}鎂，並把{}份{}交入同地市場",
                    result.place_id,
                    result.occupation_id,
                    result.wage_mg,
                    supply.quantity,
                    supply.display_name
                )
            }
        } else {
            format!(
                "在{}完成{}工作，從工作場所財庫領到{}鎂",
                result.place_id, result.occupation_id, result.wage_mg
            )
        }
    } else {
        format!(
            "在{}的{}工作暫停：{}",
            result.place_id, result.occupation_id, result.current_activity
        )
    };
    log_npc_event(at, entity_id, EVT_WORK, &payload);
    if result.wage_mg > 0 {
        let _ = add_thought(entity_id, ThoughtSource::WorkDone);
    }
}

fn try_gather_runtime_resource(at: i64, game_day: i32, entity_id: &str, place_id: &str) -> bool {
    let Some(pool) = crate::pg::pool() else {
        return false;
    };
    let gathered = match crate::pg::world_resource::gather(
        &pool,
        entity_id,
        place_id,
        1,
        game_day,
        at,
        "npc_gather",
    ) {
        Ok(Some(gathered)) => gathered,
        Ok(None) => return place_id.starts_with("wt-physical-"),
        Err(error) => {
            tracing::warn!("npc runtime gather failed {entity_id}: {error}");
            return place_id.starts_with("wt-physical-");
        }
    };
    if let Some(store) = crate::store::get_store()
        && let Err(error) = store
            .write()
            .mirror_inventory_from_authority(entity_id, &gathered.inventory)
    {
        tracing::warn!("npc gather cache mirror failed {entity_id}: {error}");
    }
    log_npc_event(at, entity_id, EVT_GATHER, &gathered.narrative);
    let _ = add_thought(entity_id, ThoughtSource::Gather);
    true
}

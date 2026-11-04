//! 腦驅動 NPC 抵達房間後的數值效果（對齊既有 `brain_arrival` 之 `applyBrainArrivalEffects`）。

use crate::config::sim;
use crate::db::{
    EVT_BEG, EVT_GATHER, EVT_HIRED, EVT_WORK, ThoughtSource, add_thought,
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

trait KeyObjectsExt {
    fn first(&self) -> Option<&str>;
}

impl KeyObjectsExt for str {
    fn first(&self) -> Option<&str> {
        self.split('、').map(str::trim).find(|s| !s.is_empty())
    }
}

pub fn advance_room_event_pack_on_npc_arrival_action(
    room_name: &str,
    item_or_clue: &str,
) -> Option<String> {
    if room_name.trim().is_empty() || item_or_clue.trim().is_empty() {
        return None;
    }
    crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        room_name,
        item_or_clue,
    )
    .map(|(_ev, _sp, note)| note)
}

pub fn format_venue_market_delivery_activity(
    dest_name: &str,
    dest_market_prefix: &str,
    quantity: i32,
    display_name: &str,
) -> String {
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(dest_name, display_name) {
        let venue_kind = if venue.venue_kind == "大食堂" {
            "聚落大食堂"
        } else {
            venue.venue_kind
        };
        if venue.has_counter_restricted_zone {
            return format!(
                "已將{}份{}送達{}（{}當值櫃檯點收）",
                quantity, display_name, dest_market_prefix, venue_kind
            );
        }
        return format!(
            "已將{}份{}送達{}（{}點收）",
            quantity, display_name, dest_market_prefix, venue_kind
        );
    }
    format!(
        "已將{}份{}送達{}市集",
        quantity, display_name, dest_market_prefix
    )
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
            let room_name = get_room_name(room_id)
                .ok()
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "街坊".to_string());
            let payload = format!("在{room_name}向身邊的人求助；沒有可核實的贈與成交");
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
                let _ = advance_room_event_pack_on_npc_arrival_action(&room_name, &gathered.name);
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
                    let room_name = get_room_name(room_id)
                        .ok()
                        .filter(|name| !name.is_empty())
                        .unwrap_or_else(|| "街坊".to_string());
                    let payload = fmt_two_s(&be.hired_event_fmt, &room_name, occ);
                    log_npc_event(at, entity_id, EVT_HIRED, &payload);
                    let _ = add_thought(entity_id, ThoughtSource::Hired);
                    break;
                }
            }
        }
        IntentType::Socialize => {
            let _ = add_thought(entity_id, ThoughtSource::Talked);
        }
        IntentType::Trade => {
            let _ = npc_street_sell_one(entity_id, room_id, at);
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
    let dest_name = get_room_name(&delivered.destination_place_id)
        .ok()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "當地市集".to_string());
    let dest_market_prefix =
        crate::pg::living_world::peninsula_market_settlement_name(&dest_name);
    let display_name =
        crate::pg::living_world::peninsula_good_display_name(&delivered.display_name);
    let _ = advance_room_event_pack_on_npc_arrival_action(&dest_name, &display_name);
    let payload = format!(
        "把{}份{}送抵{}市集",
        delivered.quantity, display_name, dest_market_prefix
    );
    log_npc_event(at, entity_id, EVT_WORK, &payload);
    let _ = add_thought(entity_id, ThoughtSource::WorkDone);
    Some(format_venue_market_delivery_activity(
        &dest_name,
        &dest_market_prefix,
        delivered.quantity,
        &display_name,
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
    if result.wage_mg > 0 {
        let raw_place_name = get_room_name(&result.place_id)
            .ok()
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "當地工坊".to_string());
        let place_settlement =
            crate::pg::living_world::peninsula_market_settlement_name(&raw_place_name);
        let _ = crate::npc::causal_core::advance_causal_state_on_arrival(
            entity_id,
            &result.occupation_id,
            &place_settlement,
            at.unsigned_abs(),
            &raw_place_name,
            1,
            false,
            false,
            result.entity_magnesium,
            None,
        );
        let place_name = if raw_place_name.ends_with("市集") {
            format!("{place_settlement}市集")
        } else if place_settlement != raw_place_name {
            place_settlement.to_string()
        } else {
            raw_place_name.clone()
        };
        let occ_label = match result.occupation_id.as_str() {
            "市場供應調度員" => "市集調度差事",
            "市場採購調度員" => "市集採買差事",
            "地方供應人員" => "物產供應差事",
            other => other,
        };
        let payload = if let Some(supply) = &result.supply {
            let good_display =
                crate::pg::living_world::peninsula_good_display_name(&supply.display_name);
            let _ = advance_room_event_pack_on_npc_arrival_action(&raw_place_name, &good_display);
            let good_name = good_display;
            if supply.delivery_kind == "shipment" {
                let raw_dest_name = get_room_name(&supply.destination_place_id)
                    .ok()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "鄰近市集".to_string());
                let dest_settlement =
                    crate::pg::living_world::peninsula_market_settlement_name(&raw_dest_name);
                let dest_name = if raw_dest_name.ends_with("市集") || dest_settlement == "鄰近" {
                    format!("{dest_settlement}市集")
                } else if dest_settlement != raw_dest_name {
                    dest_settlement.to_string()
                } else {
                    raw_dest_name
                };
                if dest_name.is_empty() || dest_name == place_name {
                    format!(
                        "在{}完成物產供應差事，領得{}鎂，並將{}份{}交入當地市集鋪面",
                        place_name, result.wage_mg, supply.quantity, good_name
                    )
                } else {
                    format!(
                        "在{}完成{}，領得{}鎂，並將{}份{}裝運發往{}",
                        place_name,
                        occ_label,
                        result.wage_mg,
                        supply.quantity,
                        good_name,
                        dest_name
                    )
                }
            } else {
                format!(
                    "在{}完成{}，領得{}鎂，並將{}份{}交入當地市集",
                    place_name,
                    occ_label,
                    result.wage_mg,
                    supply.quantity,
                    good_display
                )
            }
        } else {
            if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &raw_place_name,
                None,
                at.unsigned_abs() as _,
            ) {
                if let Some(first_key) = ev.key_objects.first() {
                    let _ = advance_room_event_pack_on_npc_arrival_action(&raw_place_name, first_key);
                }
            }
            format!(
                "在{}完成{}，領得工錢{}鎂",
                place_name, occ_label, result.wage_mg
            )
        };
        log_npc_event(at, entity_id, EVT_WORK, &payload);
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
    let room_name = get_room_name(place_id).unwrap_or_else(|_| place_id.to_string());
    let _ = advance_room_event_pack_on_npc_arrival_action(&room_name, &gathered.narrative);
    log_npc_event(at, entity_id, EVT_GATHER, &gathered.narrative);
    let _ = add_thought(entity_id, ThoughtSource::Gather);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_venue_market_delivery_activity_uses_counter_zone() {
        let activity = format_venue_market_delivery_activity(
            "三水大食堂",
            "文曜",
            3,
            "現磨溫米漿",
        );
        assert_eq!(
            activity,
            "已將3份現磨溫米漿送達文曜（聚落大食堂當值櫃檯點收）"
        );
    }

    #[test]
    fn test_advance_room_event_pack_on_npc_arrival_action() {
        let note_opt = advance_room_event_pack_on_npc_arrival_action(
            "老堰頭・水輪石磨工坊",
            "現磨溫米漿",
        );
        assert!(
            note_opt.is_some(),
            "delivering or producing 現磨溫米漿 must advance room event pack"
        );
        let note = note_opt.unwrap();
        assert!(
            note.contains("【事件包與劇本包聯動・"),
            "advanced note must contain linkage header: {note}"
        );
    }
}

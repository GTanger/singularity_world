//! 腦驅動 NPC 抵達房間後的數值效果（對齊既有 `brain_arrival` 之 `applyBrainArrivalEffects`）。

use rand::Rng;

use crate::config::sim;
use crate::db::{
    add_magnesium, add_to_inventory, adjust_disposition, get_assignment_count_by_venue, get_assignments_for_entity,
    get_venue_ids_for_room, insert_assignment, log_npc_event, npc_street_sell_one, DISP_BEG_SUCCESS, DISP_GATHER,
    DISP_HIRED, DISP_TALKED, EVT_BEG, EVT_GATHER, EVT_HIRED, EVT_TALK, EVT_TRADE,
};
use crate::gametext;

use super::decision::IntentType;

fn fmt_beg(fmt: &str, room: &str, amount: i32) -> String {
    fmt.replacen("%s", room, 1).replacen("%d", &amount.to_string(), 1)
}

fn fmt_one_s(fmt: &str, a: &str) -> String {
    fmt.replacen("%s", a, 1)
}

fn fmt_two_s(fmt: &str, a: &str, b: &str) -> String {
    fmt.replacen("%s", a, 1).replacen("%s", b, 1)
}

/// 依抵達意圖套用鎂／背包／指派／事件／心境（對齊既有）。
pub fn apply_brain_arrival_effects(at: i64, entity_id: &str, room_id: &str, intent: IntentType) {
    let be = gametext::brain_effects();
    match intent {
        IntentType::Beg => {
            let bb = &sim().brain_beg;
            let mut extra = bb.magnesium_rand_exclusive;
            if extra <= 0 {
                extra = 8;
            }
            let mut min_mg = bb.magnesium_min;
            if min_mg < 0 {
                min_mg = 0;
            }
            let mut rng = rand::rng();
            let amount = min_mg + rng.random_range(0..extra);
            let _ = add_magnesium(entity_id, amount);
            let payload = fmt_beg(&be.beg_event_fmt, room_id, amount);
            log_npc_event(at, entity_id, EVT_BEG, &payload);
            let _ = adjust_disposition(entity_id, DISP_BEG_SUCCESS);
        }
        IntentType::Gather => {
            let _ = add_to_inventory(entity_id, "wild_herb", 1);
            let payload = fmt_one_s(&be.gather_event_fmt, room_id);
            log_npc_event(at, entity_id, EVT_GATHER, &payload);
            let _ = adjust_disposition(entity_id, DISP_GATHER);
        }
        IntentType::SeekJob => {
            let Ok(list) = get_assignments_for_entity(entity_id) else {
                return;
            };
            if !list.is_empty() {
                return;
            }
            let Ok(venue_ids) = get_venue_ids_for_room(room_id) else {
                return;
            };
            let occ = gametext::occupation_waiter();
            let cap = sim().max_assignments_per_venue.max(0) as usize;
            for vid in venue_ids {
                let Ok(n) = get_assignment_count_by_venue(&vid) else {
                    continue;
                };
                if n < cap && insert_assignment(entity_id, &occ, &vid, "").is_ok() {
                    let payload = fmt_two_s(&be.hired_event_fmt, room_id, &occ);
                    log_npc_event(at, entity_id, EVT_HIRED, &payload);
                    let _ = adjust_disposition(entity_id, DISP_HIRED);
                    break;
                }
            }
        }
        IntentType::Socialize => {
            log_npc_event(at, entity_id, EVT_TALK, &be.talk_social);
            let _ = adjust_disposition(entity_id, DISP_TALKED);
        }
        IntentType::Trade => {
            if !npc_street_sell_one(entity_id, room_id, at) {
                log_npc_event(at, entity_id, EVT_TRADE, &be.trade_no_stock);
            }
        }
        IntentType::Wander | IntentType::Work | IntentType::Idle => {}
    }
}

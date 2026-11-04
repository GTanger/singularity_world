//! 白塔市場的短節拍：需求成交、貨運指派與中斷復原，不依賴在線玩家。

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::config::Server;
use crate::db::{
    EVT_TRADE, ThoughtSource, add_thought, log_npc_event, mirror_market_trade_settlement,
};

use super::super::SessionStore;
use super::super::broadcast::refresh_room_views_for_room;
use super::MainLoopTickState;
use super::helpers::periodic_fire;
pub(crate) fn format_market_consumption_event_payload_and_advance_pack(
    place_name: &str,
    price_mg: i32,
    good_display: &str,
) -> String {
    let _ = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
        place_name.trim(),
        good_display.trim(),
    );
    let market_prefix = crate::pg::living_world::peninsula_market_settlement_name(place_name);
    if let Some(venue) = crate::grid::resolve_any_room_venue_object(place_name.trim(), "") {
        let zone_tag = if venue.has_counter_restricted_zone {
            "當值櫃檯"
        } else {
            "當值攤位"
        };
        format!(
            "在{}（{}{}）花{}鎂買下一份{}",
            market_prefix, venue.venue_kind, zone_tag, price_mg, good_display
        )
    } else {
        format!("在{}市集花{}鎂買下一份{}", market_prefix, price_mg, good_display)
    }
}


pub(super) fn run_market_economy_section(
    sessions: &SessionStore,
    cfg: &Server,
    now_unix: i64,
    state: &Arc<Mutex<MainLoopTickState>>,
) {
    let fire = {
        let Ok(mut state) = state.lock() else {
            return;
        };
        periodic_fire(&mut state.last_market_economy, Duration::from_secs(15))
    };
    if !fire {
        return;
    }
    let Some(pool) = crate::pg::pool() else {
        return;
    };
    let mut changed_places = HashSet::new();
    if let Err(error) = crate::pg::world_logistics::reconcile_one(&pool, now_unix) {
        tracing::warn!("market logistics reconciliation failed: {error}");
    }
    match crate::pg::world_logistics::dispatch_one(&pool, now_unix) {
        Ok(Some(dispatch)) => {
            changed_places.insert(dispatch.origin_place_id);
            changed_places.insert(dispatch.destination_place_id);
        }
        Ok(None) => {}
        Err(error) => tracing::warn!("market shipment dispatch failed: {error}"),
    }
    for _ in 0..3 {
        match crate::pg::world_trade::settle_one_npc_consumption(&pool, now_unix) {
            Ok(Some(settlement)) => {
                mirror_market_trade_settlement(&settlement);
                let place_name = crate::db::get_room_name(&settlement.place_id)
                    .ok()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| "當地".to_string());
                let good_display =
                    crate::pg::living_world::peninsula_good_display_name(&settlement.display_name);
                let payload = format_market_consumption_event_payload_and_advance_pack(
                    &place_name,
                    settlement.price_mg,
                    &good_display,
                );
                log_npc_event(now_unix, &settlement.buyer.entity_id, EVT_TRADE, &payload);
                let _ = add_thought(&settlement.buyer.entity_id, ThoughtSource::Trade);
                changed_places.insert(settlement.place_id);
            }
            Ok(None) => break,
            Err(error) => {
                tracing::warn!("npc market consumption failed: {error}");
                break;
            }
        }
    }
    for place_id in changed_places {
        refresh_room_views_for_room(sessions, cfg, &place_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_market_consumption_event_payload_and_advance_pack() {
        let payload_venue_counter = format_market_consumption_event_payload_and_advance_pack(
            "老堰頭・水輪石磨工坊",
            3,
            "現磨溫米漿",
        );
        assert_eq!(
            payload_venue_counter,
            "在老堰頭・水輪石磨工坊（水輪石磨工坊當值櫃檯）花3鎂買下一份現磨溫米漿"
        );

        let payload_venue_non_counter = format_market_consumption_event_payload_and_advance_pack(
            "蒙學講舍",
            5,
            "蒙學基礎識字課本",
        );
        assert_eq!(
            payload_venue_non_counter,
            "在蒙學講舍（蒙學學舍當值攤位）花5鎂買下一份蒙學基礎識字課本"
        );

        let payload_fallback = format_market_consumption_event_payload_and_advance_pack(
            "荒蕪野地集散點",
            10,
            "粗麻布卷",
        );
        assert_eq!(
            payload_fallback,
            "在荒蕪野地集散點市集花10鎂買下一份粗麻布卷"
        );
    }
}

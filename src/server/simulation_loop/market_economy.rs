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
                let payload = format!(
                    "在{}以{}鎂買下並使用一份{}",
                    settlement.place_id, settlement.price_mg, settlement.display_name
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

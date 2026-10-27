//! NPC 街頭兜售（對齊既有 `db/npc_trade`）。

use super::{
    EVT_TRADE, ThoughtSource, add_thought, default_trade_ask_mg, get_entity, get_item_def,
    log_npc_event, pick_npc_trade_offer, settle_npc_inventory_sale,
};

/// 把背包中一份可交易物品賣給同一市場內的真實 NPC；沒有買家就不成交。
pub fn npc_street_sell_one(entity_id: &str, room_id: &str, at: i64) -> bool {
    let Ok(Some(ch)) = get_entity(entity_id) else {
        return false;
    };
    if ch.inventory.is_empty() || ch.inventory == "[]" {
        return false;
    }
    let Some(item_id) = pick_npc_trade_offer(&ch.inventory) else {
        return false;
    };
    let mag = default_trade_ask_mg(&item_id);
    let settlement = match settle_npc_inventory_sale(entity_id, &item_id, 1, mag, at) {
        Ok(Some(settlement)) => settlement,
        Ok(None) => return false,
        Err(error) => {
            tracing::warn!("npc trade failed {entity_id}: {error}");
            return false;
        }
    };
    if settlement.place_id != room_id {
        tracing::warn!(
            "npc trade place mirror differed {}: {} / {}",
            settlement.trade_id,
            settlement.place_id,
            room_id
        );
    }
    let item_name = get_item_def(&item_id)
        .map(|d| d.name)
        .filter(|name| !name.is_empty())
        .unwrap_or(item_id);
    let payload = format!(
        "在{}把{item_name}賣給{}，實收{mag}鎂（交易 {}）",
        settlement.place_id, settlement.buyer.entity_id, settlement.trade_id
    );
    log_npc_event(at, entity_id, EVT_TRADE, &payload);
    let _ = add_thought(entity_id, ThoughtSource::Trade);
    let _ = add_thought(&settlement.buyer.entity_id, ThoughtSource::Trade);
    true
}

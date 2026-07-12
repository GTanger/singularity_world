//! NPC 街頭兜售（對齊既有 `db/npc_trade`）。

use super::{
    EVT_TRADE, ThoughtSource, add_magnesium, add_thought, default_trade_ask_mg, get_entity,
    get_item_def, log_npc_event, pick_npc_trade_offer, remove_from_inventory,
};

/// 賣出背包中第一筆 `qty≥1` 的物品 1 件換鎂；成功回 `true`（對齊既有 `NpcStreetSellOne`）。
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
    if remove_from_inventory(entity_id, &item_id, 1).is_err() {
        return false;
    }
    let mag = default_trade_ask_mg(&item_id);
    let _ = add_magnesium(entity_id, mag);
    let item_name = get_item_def(&item_id)
        .map(|d| d.name)
        .filter(|name| !name.is_empty())
        .unwrap_or(item_id);
    let payload = format!("在{room_id}兜售賣出{item_name}，得{mag}鎂");
    log_npc_event(at, entity_id, EVT_TRADE, &payload);
    let _ = add_thought(entity_id, ThoughtSource::Trade);
    true
}

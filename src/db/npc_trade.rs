//! NPC 街頭兜售（對齊 Go `db/npc_trade.go`）。

use rand::Rng;

use super::{
    add_magnesium, adjust_disposition, get_entity, log_npc_event, remove_from_inventory, DISP_TRADE, EVT_TRADE,
};

/// 賣出背包中第一筆 `qty≥1` 的物品 1 件換鎂；成功回 `true`（對齊 Go `NpcStreetSellOne`）。
pub fn npc_street_sell_one(entity_id: &str, room_id: &str, at: i64) -> bool {
    let Ok(Some(ch)) = get_entity(entity_id) else {
        return false;
    };
    if ch.inventory.is_empty() || ch.inventory == "[]" {
        return false;
    }
    #[derive(serde::Deserialize)]
    struct InvRow {
        #[serde(rename = "item_id")]
        item_id: String,
        qty: i32,
    }
    let entries: Vec<InvRow> = serde_json::from_str(&ch.inventory).unwrap_or_default();
    for e in &entries {
        if e.qty <= 0 || e.item_id.is_empty() {
            continue;
        }
        if remove_from_inventory(entity_id, &e.item_id, 1).is_err() {
            continue;
        }
        let mut rng = rand::rng();
        let mag = 4 + rng.random_range(0..10);
        let _ = add_magnesium(entity_id, mag);
        let payload = format!("在{room_id}兜售賣出{}，得{mag}鎂", e.item_id);
        log_npc_event(at, entity_id, EVT_TRADE, &payload);
        let _ = adjust_disposition(entity_id, DISP_TRADE);
        return true;
    }
    false
}

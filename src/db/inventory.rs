//! 背包寫入（對齊 Go `db/inventory.go` 的 `AddToInventory`）。

use serde::{Deserialize, Serialize};

use crate::store;

use super::ErrNoStore;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventoryEntry {
    #[serde(rename = "item_id")]
    item_id: String,
    qty: i32,
}

/// 將物品堆疊寫入實體的 `inventory` JSON；`qty` 為 0 或 `item_id` 空則 no-op。成功時觸發 `persist_entities`。
pub fn add_to_inventory(entity_id: &str, item_id: &str, qty: i32) -> anyhow::Result<()> {
    if qty == 0 || item_id.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        let mut raw = e.inventory.clone();
        if raw.is_empty() {
            raw = "[]".to_string();
        }
        let mut entries: Vec<InventoryEntry> = serde_json::from_str(&raw).unwrap_or_default();
        let mut found = false;
        for ent in &mut entries {
            if ent.item_id == item_id {
                ent.qty += qty;
                found = true;
                break;
            }
        }
        if !found {
            entries.push(InventoryEntry {
                item_id: item_id.to_string(),
                qty,
            });
        }
        if let Ok(b) = serde_json::to_string(&entries) {
            e.inventory = b;
        }
    })
}

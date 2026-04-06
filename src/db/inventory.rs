//! 背包讀寫與裝備槽（對齊既有 `db/inventory`）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::store;

use super::ErrNoStore;

/// 每點體質可負重（對齊既有 `weightPerVit`）。
const WEIGHT_PER_VIT: f64 = 10.0;

/// 單列背包顯示（對齊既有 `InventoryDisplayItem`）。
#[derive(Debug, Clone)]
pub struct InventoryLine {
    pub item_id: String,
    pub name: String,
    pub item_type: String,
    pub qty: i32,
    pub weight: f64,
    pub sub_total: f64,
    pub description: String,
    pub slot: String,
}

/// 背包查詢結果（對齊既有 `InventoryResult`）。
#[derive(Debug, Clone)]
pub struct InventorySnapshot {
    pub items: Vec<InventoryLine>,
    pub current_weight: f64,
    pub max_weight: f64,
}

/// 單一物品定義摘要（對齊既有 `GetItemInfo` 回傳欄位）。
#[derive(Debug, Clone)]
pub struct ItemDefBrief {
    pub name: String,
    pub item_type: String,
    pub slot: String,
    pub description: String,
    pub weight: f64,
    pub vit_bonus: i32,
    pub dex_bonus: i32,
    pub atk_bonus: i32,
}

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

/// 自背包 JSON 扣減數量；堆疊歸零則移除該列（對齊既有 `RemoveFromInventory` 語意）。
pub fn remove_from_inventory(entity_id: &str, item_id: &str, qty: i32) -> anyhow::Result<()> {
    if qty <= 0 || item_id.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        let mut raw = e.inventory.clone();
        if raw.is_empty() {
            raw = "[]".to_string();
        }
        let mut entries: Vec<InventoryEntry> = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return,
        };
        for ent in &mut entries {
            if ent.item_id == item_id {
                ent.qty -= qty;
                break;
            }
        }
        entries.retain(|e| e.qty > 0 && !e.item_id.is_empty());
        if let Ok(b) = serde_json::to_string(&entries) {
            e.inventory = b;
        }
    })
}

/// 背包內所有物品數量加總（僅 `qty > 0`；對齊既有 `DecisionState.InventoryTotalQty`）。
#[must_use]
pub fn inventory_total_qty(inventory_json: &str) -> i32 {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return 0;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    entries.iter().filter(|e| e.qty > 0 && !e.item_id.is_empty()).map(|e| e.qty).sum()
}

/// 自 `inventory` JSON 與 `items` 表組裝前端用清單與負重（對齊既有 `GetInventory`）。
#[must_use]
pub fn get_inventory(inventory_json: &str, vit: i32) -> InventorySnapshot {
    let max_weight = vit as f64 * WEIGHT_PER_VIT;
    if inventory_json.is_empty() || inventory_json == "[]" {
        return InventorySnapshot {
            items: Vec::new(),
            current_weight: 0.0,
            max_weight,
        };
    }
    let entries: Vec<InventoryEntry> = match serde_json::from_str(inventory_json) {
        Ok(v) => v,
        Err(_) => {
            return InventorySnapshot {
                items: Vec::new(),
                current_weight: 0.0,
                max_weight,
            };
        }
    };
    let Some(arc) = store::get_store() else {
        return InventorySnapshot {
            items: Vec::new(),
            current_weight: 0.0,
            max_weight,
        };
    };
    let s = arc.read().unwrap();
    let mut items: Vec<InventoryLine> = Vec::new();
    let mut total_weight = 0.0;
    for e in entries {
        if e.qty <= 0 || e.item_id.is_empty() {
            continue;
        }
        let (name, item_type, description, slot, weight) = if let Some(it) = s.get_item(&e.item_id) {
            (
                it.name.clone(),
                it.item_type.clone(),
                it.description.clone(),
                it.slot.clone(),
                it.weight,
            )
        } else {
            (e.item_id.clone(), "misc".into(), String::new(), String::new(), 0.0)
        };
        let sub = weight * f64::from(e.qty);
        items.push(InventoryLine {
            item_id: e.item_id,
            name,
            item_type,
            qty: e.qty,
            weight,
            sub_total: sub,
            description,
            slot,
        });
        total_weight += sub;
    }
    InventorySnapshot {
        items,
        current_weight: total_weight,
        max_weight,
    }
}

/// 查單一物品定義；無 store 或無此 id 則 `None`（對齊既有 `GetItemInfo` 之失敗路徑）。
#[must_use]
pub fn get_item_def(item_id: &str) -> Option<ItemDefBrief> {
    let arc = store::get_store()?;
    let s = arc.read().unwrap();
    let it = s.get_item(item_id)?;
    Some(ItemDefBrief {
        name: it.name.clone(),
        item_type: it.item_type.clone(),
        slot: it.slot.clone(),
        description: it.description.clone(),
        weight: it.weight,
        vit_bonus: it.vit_bonus,
        dex_bonus: it.dex_bonus,
        atk_bonus: it.atk_bonus,
    })
}

/// 背包目前總重量（對齊既有 `InventoryWeight`）。
#[must_use]
pub fn inventory_weight(inventory_json: &str) -> f64 {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return 0.0;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    let Some(arc) = store::get_store() else {
        return 0.0;
    };
    let s = arc.read().unwrap();
    let mut total = 0.0;
    for e in entries {
        if e.qty <= 0 {
            continue;
        }
        let w = s.get_item(&e.item_id).map(|it| it.weight).unwrap_or(0.0);
        total += w * f64::from(e.qty);
    }
    total
}

/// 設定單一裝備槽之 `item_id`（對齊既有 `UpdateEquipmentSlot`）。
pub fn update_equipment_slot(entity_id: &str, slot: &str, item_id: &str) -> anyhow::Result<()> {
    if slot.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        let mut map: HashMap<String, String> = if e.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&e.equipment_slots).unwrap_or_default()
        };
        map.insert(slot.to_string(), item_id.to_string());
        if let Ok(b) = serde_json::to_string(&map) {
            e.equipment_slots = b;
        }
    })
}

/// 清空單一裝備槽（對齊既有 `ClearEquipmentSlot`）。
pub fn clear_equipment_slot(entity_id: &str, slot: &str) -> anyhow::Result<()> {
    if slot.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write().unwrap();
    s.update_entity(entity_id, |e| {
        let mut map: HashMap<String, String> = if e.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&e.equipment_slots).unwrap_or_default()
        };
        map.remove(slot);
        if let Ok(b) = serde_json::to_string(&map) {
            e.equipment_slots = b;
        }
    })
}

/// 背包 JSON 中是否至少有一件指定 `item_id` 且 `qty > 0`。
#[must_use]
pub fn inventory_has_item(inventory_json: &str, item_id: &str) -> bool {
    if inventory_json.is_empty() || inventory_json == "[]" || item_id.is_empty() {
        return false;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    entries
        .iter()
        .any(|e| e.item_id == item_id && e.qty > 0)
}

/// 從 NPC 背包 JSON 取第一個數量 > 0 的物品作為本輪賣品（對齊既有 `PickNPCTradeOffer`）。
#[must_use]
pub fn pick_npc_trade_offer(inventory_json: &str) -> Option<String> {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return None;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    for e in entries {
        if !e.item_id.is_empty() && e.qty > 0 {
            return Some(e.item_id);
        }
    }
    None
}

/// 第一版預設喊價（對齊既有 `DefaultTradeAskMg`）。
#[must_use]
pub fn default_trade_ask_mg(item_id: &str) -> i32 {
    match item_id {
        "wild_herb" => 5,
        _ => 8,
    }
}

/// 議價底價＝喊價七成（至少 1）（對齊既有 `TradeFloorFromAsk`）。
#[must_use]
pub fn trade_floor_from_ask(ask: i32) -> i32 {
    if ask <= 0 {
        return 1;
    }
    let f = ask * 7 / 10;
    if f < 1 {
        1
    } else {
        f
    }
}

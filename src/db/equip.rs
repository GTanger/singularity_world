// 初始裝備與裸奔判定，對齊 Go db/equip.go。

use std::collections::HashMap;

use serde_json::Value;

use crate::store;

/// 依性別回傳初始裝備 JSON（對齊 Go `StarterEquipment`）。
#[must_use]
pub fn starter_equipment(gender: &str) -> String {
    if gender == "F" {
        r#"{"body":"starter_body_f","legs":"starter_legs_f","feet":"starter_feet_f"}"#.to_string()
    } else {
        r#"{"body":"starter_body_m","legs":"starter_legs_m","feet":"starter_feet_m"}"#.to_string()
    }
}

/// `body` 或 `legs` 任一為空即「衣不蔽體」（對齊 Go `IsNaked`）。
#[must_use]
pub fn is_naked(equipment_slots: &str) -> bool {
    if equipment_slots.is_empty() {
        return true;
    }
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(equipment_slots) else {
        return true;
    };
    let body = map.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let legs = map.get("legs").and_then(|v| v.as_str()).unwrap_or("");
    body.is_empty() || legs.is_empty()
}

/// 依 `equipment_slots` 查槽位→物品名稱（對齊 Go `GetItemNames`）。
#[must_use]
pub fn get_item_names(equipment_slots: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if equipment_slots.is_empty() {
        return result;
    }
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(equipment_slots) else {
        return result;
    };
    let Some(st) = store::get_store() else {
        return result;
    };
    let s = st.read().unwrap();
    for (slot, v) in map {
        let Some(item_id) = v.as_str() else { continue };
        if item_id.is_empty() {
            continue;
        }
        if let Some(it) = s.get_item(item_id) {
            result.insert(slot, it.name.clone());
        }
    }
    result
}

/// 依 `equipment_slots` 查槽位→物品描述（對齊 Go `GetItemDescs`）。
#[must_use]
pub fn get_item_descs(equipment_slots: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if equipment_slots.is_empty() {
        return result;
    }
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(equipment_slots) else {
        return result;
    };
    let Some(st) = store::get_store() else {
        return result;
    };
    let s = st.read().unwrap();
    for (slot, v) in map {
        let Some(item_id) = v.as_str() else { continue };
        if item_id.is_empty() {
            continue;
        }
        if let Some(it) = s.get_item(item_id) {
            result.insert(slot, it.description.clone());
        }
    }
    result
}

/// 物品種子由 `items.json` 載入；no-op（對齊 Go `SeedItems`）。
pub fn seed_items() -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_f_differs_from_m() {
        assert_ne!(starter_equipment("F"), starter_equipment("M"));
        assert!(starter_equipment("M").contains("starter_body_m"));
    }

    #[test]
    fn naked_empty_slots() {
        assert!(is_naked(""));
        assert!(is_naked("{}"));
        assert!(!is_naked(r#"{"body":"x","legs":"y"}"#));
        assert!(is_naked(r#"{"body":"","legs":"y"}"#));
    }
}

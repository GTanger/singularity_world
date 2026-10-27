// 初始裝備與裸奔判定，對齊既有 db/equip。

use std::collections::HashMap;

use serde_json::Value;

use crate::store;

/// 依性別回傳初始裝備 JSON（對齊既有 `StarterEquipment`）。
#[must_use]
pub fn starter_equipment(gender: &str) -> String {
    if gender == "F" {
        r#"{"undershirt":"","inner_armor":"","body":"starter_body_f","legs":"starter_legs_f","leg_armor":"","feet":"starter_feet_f"}"#.to_string()
    } else {
        r#"{"undershirt":"","inner_armor":"","body":"starter_body_m","legs":"starter_legs_m","leg_armor":"","feet":"starter_feet_m"}"#.to_string()
    }
}

/// `body` 或 `legs` 任一為空即「衣不蔽體」（對齊既有 `IsNaked`）。
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

/// 依 `equipment_slots` 查槽位→物品名稱（對齊既有 `GetItemNames`）。
#[must_use]
pub fn get_item_names(equipment_slots: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if equipment_slots.is_empty() {
        return result;
    }
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(equipment_slots) else {
        return result;
    };
    let st = store::get_store();
    let s = st.as_ref().map(|arc| arc.read());
    for (slot, v) in map {
        let Some(item_id) = v.as_str() else { continue };
        if item_id.is_empty() {
            continue;
        }
        if let Some(it) = s.as_ref().and_then(|store| store.get_item(item_id)) {
            let name = match it.name.as_str() {
                "粗布短褂" => "棉麻日常上衣",
                "粗布長褲" => "耐磨休閒長褲",
                "草鞋" => "輕便布鞋",
                _ => &it.name,
            };
            result.insert(slot, name.to_string());
        } else if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
            result.insert(slot, mat.name.clone());
        } else if let Some(name) = crate::db::inventory::fallback_item_name(item_id) {
            result.insert(slot, name.to_string());
        } else if let Some(ecol) = crate::game::object_ecology::lookup_ecology_item_def(item_id) {
            result.insert(slot, ecol.name);
        } else {
            let norm_name = match item_id {
                "粗布短褂" | "starter_body_m" | "starter_body_f" => "棉麻日常上衣",
                "粗布長褲" | "starter_legs_m" | "starter_legs_f" => "耐磨休閒長褲",
                "草鞋" | "starter_feet_m" | "starter_feet_f" => "輕便布鞋",
                _ => item_id,
            };
            result.insert(slot, norm_name.to_string());
        }
    }
    result
}

/// 依 `equipment_slots` 查槽位→物品描述（對齊既有 `GetItemDescs`）。
#[must_use]
pub fn get_item_descs(equipment_slots: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    if equipment_slots.is_empty() {
        return result;
    }
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(equipment_slots) else {
        return result;
    };
    let st = store::get_store();
    let s = st.as_ref().map(|arc| arc.read());
    for (slot, v) in map {
        let Some(item_id) = v.as_str() else { continue };
        if item_id.is_empty() {
            continue;
        }
        if let Some(it) = s.as_ref().and_then(|store| store.get_item(item_id)) {
            result.insert(slot, it.description.clone());
        } else if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
            result.insert(slot, mat.description.clone());
        } else if let Some(_name) = crate::db::inventory::fallback_item_name(item_id) {
            let desc = match item_id {
                "starter_body_m" | "starter_body_f" | "粗布短褂" => "質樸舒適的棉麻日常上衣。",
                "starter_legs_m" | "starter_legs_f" | "粗布長褲" => "耐磨且便於活動的日常休閒長褲。",
                "starter_feet_m" | "starter_feet_f" | "草鞋" => "輕便透氣的平底布鞋。",
                _ => "採集或探索獲得的物產。",
            };
            result.insert(slot, desc.to_string());
        }
    }
    result
}

/// 物品種子由 `items.json` 載入；no-op（對齊既有 `SeedItems`）。
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
        assert!(starter_equipment("M").contains("\"inner_armor\":\"\""));
        assert!(starter_equipment("M").contains("\"undershirt\":\"\""));
        assert!(starter_equipment("M").contains("\"leg_armor\":\"\""));
    }

    #[test]
    fn naked_empty_slots() {
        assert!(is_naked(""));
        assert!(is_naked("{}"));
        assert!(!is_naked(r#"{"body":"x","legs":"y"}"#));
        assert!(is_naked(r#"{"body":"","legs":"y"}"#));
    }

    #[test]
    fn starter_equipment_resolves_modern_names() {
        let slots_m = starter_equipment("M");
        let names_m = get_item_names(&slots_m);
        assert_eq!(names_m.get("body").map(|s| s.as_str()), Some("棉麻日常上衣"));
        assert_eq!(names_m.get("legs").map(|s| s.as_str()), Some("耐磨休閒長褲"));
        assert_eq!(names_m.get("feet").map(|s| s.as_str()), Some("輕便布鞋"));

        let slots_f = starter_equipment("F");
        let names_f = get_item_names(&slots_f);
        assert_eq!(names_f.get("body").map(|s| s.as_str()), Some("棉麻日常上衣"));
        assert_eq!(names_f.get("legs").map(|s| s.as_str()), Some("耐磨休閒長褲"));
        assert_eq!(names_f.get("feet").map(|s| s.as_str()), Some("輕便布鞋"));
    }
}

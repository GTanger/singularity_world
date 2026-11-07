//! store 實體 — entities + entity_rooms 的讀寫、座標綁定、轉帳。
//! 寫入路徑走 writer queue（PG 權威）+ 必要時 in-memory mirror；entity_rooms 不再寫 JSON。

use super::{Entity, Store};

include!("entity/part_01_state.rs");
include!("entity/part_02_household.rs");
include!("entity/part_03_navigation.rs");
include!("entity/part_04_store_crud.rs");
include!("entity/part_05_store_simulation.rs");
include!("entity/part_06_store_movement.rs");

/// 將家戶與常駐 NPC 之 `equipment_slots` 與 `inventory` 對齊其家戶職司單一真相源：
/// - 衣著槽（`body`/`legs`/`feet` 等）僅保留空值或 `starter_*`，防止非本職斗篷、皮靴覆寫家戶常服；
/// - 持物槽（`hold_r`/`hold_l`/`weapon`/`tool`/`hand`）若家戶 `attire_line` 已寫明手持器物則清空（防三隻手衝突），否則僅保留與自身職司本命器物一致之 ID（清除誤發之獵刀、雨傘、十字鎬等）；
/// - 背包僅保留守恆市集商品（`wt-good-*`）與單件本職器物，清除歷史自主生產誤塞之原材與雨傘斗篷。
pub fn sanitize_npc_equipment_and_inventory(e: &mut super::Entity) -> bool {
    if !(e.kind == "npc" || e.id.starts_with("npc_") || e.id.starts_with("wt_")) {
        return false;
    }
    let is_household_npc = e.id.starts_with("npc_");
    let (cat_id, vspec, attire_has_held_item) = if is_household_npc {
        let slot = wenyao_household_slot_for(&e.id);
        let (cid, _) = held_vessel(slot);
        let vs = if !slot.work_name.is_empty() {
            let s1 = crate::npc::causal_core::resolve_vocational_vessel(slot.work_name);
            if s1.item_id != "craft_woven_basket" || slot.work_purpose.is_empty() {
                s1
            } else {
                crate::npc::causal_core::resolve_vocational_vessel(slot.work_purpose)
            }
        } else {
            crate::npc::causal_core::resolve_vocational_vessel(slot.work_purpose)
        };
        let has_held = ["手持", "手握", "手執", "手捧", "手提"]
            .iter()
            .any(|kw| slot.attire_line.contains(kw));
        (cid, vs, has_held)
    } else {
        let role_hint = if !e.display_title.trim().is_empty() {
            e.display_title.as_str()
        } else {
            e.id.as_str()
        };
        let vs = crate::npc::causal_core::resolve_vocational_vessel(role_hint);
        (vs.catalog_id, vs, false)
    };
    let mut changed = false;

    if !e.equipment_slots.trim().is_empty()
        && let Ok(mut map) =
            serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&e.equipment_slots)
    {
        let mut eq_modified = false;
        for (k, v) in map.iter_mut() {
            let val_str = v.as_str().unwrap_or("").trim();
            if val_str.is_empty() {
                continue;
            }
            let is_wear_slot = matches!(
                k.as_str(),
                "body" | "legs" | "feet" | "head" | "inner_armor" | "undershirt" | "leg_armor"
            );
            if is_wear_slot {
                let allow_wear = if is_household_npc {
                    val_str.starts_with("starter_")
                } else {
                    (val_str.starts_with("starter_") || val_str.starts_with("wt_cat_"))
                        && !val_str.contains("umbrella")
                        && !val_str.contains("cloak")
                };
                if !allow_wear {
                    *v = serde_json::Value::String(String::new());
                    eq_modified = true;
                }
            } else {
                let is_allowed_vessel = !attire_has_held_item
                    && (val_str == vspec.item_id
                        || val_str == vspec.catalog_id
                        || val_str == cat_id)
                    && !val_str.contains("umbrella")
                    && !val_str.contains("cloak");
                if !is_allowed_vessel {
                    *v = serde_json::Value::String(String::new());
                    eq_modified = true;
                }
            }
        }
        if eq_modified
            && let Ok(new_eq) = serde_json::to_string(&map)
        {
            e.equipment_slots = new_eq;
            changed = true;
        }
    }

    if !e.inventory.trim().is_empty()
        && let Ok(mut entries) =
            serde_json::from_str::<Vec<crate::db::inventory::InventoryEntry>>(&e.inventory)
    {
        let orig_len = entries.len();
        let mut inv_modified = false;
        entries.retain_mut(|entry| {
            if entry.qty <= 0 || entry.item_id.is_empty() {
                inv_modified = true;
                return false;
            }
            if crate::world::author_economy::is_runtime_good_item(&entry.item_id) {
                return true;
            }
            if (entry.item_id == vspec.item_id
                || entry.item_id == vspec.catalog_id
                || entry.item_id == cat_id)
                && !entry.item_id.contains("umbrella")
                && !entry.item_id.contains("cloak")
            {
                if entry.qty > 1 {
                    entry.qty = 1;
                    inv_modified = true;
                }
                return true;
            }
            inv_modified = true;
            false
        });
        if (inv_modified || entries.len() != orig_len)
            && let Ok(new_inv) = serde_json::to_string(&entries)
        {
            e.inventory = new_inv;
            changed = true;
        }
    }

    let home_name = if is_household_npc {
        wenyao_household_slot_for(&e.id).home_name
    } else {
        ""
    };
    let work_name = if is_household_npc {
        let w = wenyao_household_slot_for(&e.id).work_name;
        if w.is_empty() { "文曜・百工修繕坊" } else { w }
    } else {
        "文曜・百工修繕坊"
    };
    let mut thoughts_modified = false;
    for t in &mut e.thoughts {
        if let Some(flavor) = t.flavor.as_mut() {
            let orig = flavor.clone();
            if flavor.contains("陳氏庖廚居") && !home_name.contains("陳氏庖廚居") {
                *flavor = flavor
                    .replace("文曜・三水口・陳氏庖廚居", "文曜・三水大食堂")
                    .replace("三水口・陳氏庖廚居", "文曜・三水大食堂")
                    .replace("陳氏庖廚居", "文曜・三水大食堂");
            }
            if flavor.contains("海風麵點鋪居") && !home_name.contains("海風麵點鋪居") {
                *flavor = flavor
                    .replace("風舶角・海風麵點鋪居", "風舶角・海風熱食攤")
                    .replace("海風麵點鋪居", "風舶角・海風熱食攤");
            }
            if flavor.contains("修耙補犁鋪居") && !home_name.contains("修耙補犁鋪居") {
                *flavor = flavor
                    .replace("大圳頭・修耙補犁鋪居", "大圳頭・農具百工坊")
                    .replace("修耙補犁鋪居", "大圳頭・農具百工坊");
            }
            if flavor.contains("白氏湯屋居") && !home_name.contains("白氏湯屋居") {
                *flavor = flavor
                    .replace("文曜・三水口・白氏湯屋居", "文曜・泉香溫水浴場")
                    .replace("三水口・白氏湯屋居", "文曜・泉香溫水浴場")
                    .replace("白氏湯屋居", "文曜・泉香溫水浴場");
            }
            if flavor.contains("石板弄堂天井") {
                if flavor.contains("清冽甘水") || flavor.contains("竹水筒") {
                    *flavor = flavor
                        .replace("文曜・三水口・石板弄堂天井", "文曜・三水口水岸公井")
                        .replace("三水口・石板弄堂天井", "文曜・三水口水岸公井")
                        .replace("石板弄堂天井", "文曜・三水口水岸公井");
                } else if !home_name.contains("石板弄堂天井")
                    || flavor.contains("重新紮緊")
                    || flavor.contains("修整校準")
                {
                    *flavor = flavor
                        .replace("文曜・三水口・石板弄堂天井", work_name)
                        .replace("三水口・石板弄堂天井", work_name)
                        .replace("石板弄堂天井", work_name);
                }
            }
            if flavor.contains("洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了")
                && !["籃", "簍", "袋", "桶", "網"].iter().any(|k| flavor.contains(k))
            {
                *flavor = flavor.replace(
                    "洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了",
                    "細細修整校準，器況煥然一新，幹起活來順手多了",
                );
            }
            if *flavor != orig {
                thoughts_modified = true;
            }
        }
    }
    if thoughts_modified {
        changed = true;
    }

    changed
}

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

    let work_ctx = if is_household_npc {
        let s = wenyao_household_slot_for(&e.id);
        format!("{} {}", s.work_name, s.work_purpose)
    } else {
        e.display_title.clone()
    };
    let (canon_supply_id, _) =
        crate::game::object_ecology::ItemPhysicalClass::canonical_vocation_supply_item(&work_ctx);

    if !e.inventory.trim().is_empty()
        || is_household_npc
    {
        let mut entries: Vec<crate::db::inventory::InventoryEntry> = if e.inventory.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&e.inventory).unwrap_or_default()
        };
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
            if crate::game::object_ecology::ItemPhysicalClass::is_item_coherent_with_vocation(
                &entry.item_id,
                &work_ctx,
            ) {
                if !entry.item_id.starts_with("wt-good-") && entry.qty > 2 {
                    entry.qty = 2;
                    inv_modified = true;
                }
                return true;
            }
            inv_modified = true;
            false
        });
        if is_household_npc && !entries.iter().any(|it| it.item_id == canon_supply_id) {
            entries.push(crate::db::inventory::InventoryEntry {
                item_id: canon_supply_id.to_string(),
                qty: 2,
            });
            inv_modified = true;
        }
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
            *flavor = sanitize_thought_flavor_by_grid_and_class(flavor, home_name, work_name, "文曜");
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

/// 通用念頭文本場所與器物整飭校驗：
/// 1. 將非自家之私人住宅或在街邊執行的就餐/飲水/沐浴/整飭場所校正為公共設施或工作場所；
/// 2. 依實際器物分類對齊其維護動態詞句。
pub fn sanitize_thought_flavor_by_grid_and_class(
    flavor: &str,
    home_name: &str,
    work_name: &str,
    _settlement: &str,
) -> String {
    let mut result = flavor.to_string();
    let action_verbs = [
        "完成",
        "用過",
        "飲過",
        "把隨身的",
        "泡過",
        "歇息",
        "坐街邊",
    ];

    if let Some(start_idx) = result.find("在") {
        let after_at = &result[start_idx + "在".len()..];
        let mut best_verb: Option<(&str, usize)> = None;
        for &verb in &action_verbs {
            if let Some(pos) = after_at.find(verb) {
                match best_verb {
                    None => best_verb = Some((verb, pos)),
                    Some((_, existing_pos)) if pos < existing_pos => {
                        best_verb = Some((verb, pos));
                    }
                    _ => {}
                }
            }
        }
        if let Some((verb, verb_pos)) = best_verb {
            let raw_venue = after_at[..verb_pos].trim();
            let clean_venue = raw_venue.rsplit('・').next().unwrap_or(raw_venue).trim();
            let is_eating = verb == "用過"
                || result.contains("飯食")
                || result.contains("熱湯")
                || result.contains("麵")
                || result.contains("饃")
                || result.contains("乾糧");
            let is_drinking = verb == "飲過"
                || result.contains("清冽甘水")
                || result.contains("冷水")
                || result.contains("冷開水")
                || result.contains("竹水筒")
                || result.contains("公井");
            let is_bathing = verb == "泡過" || result.contains("溫水") || result.contains("熱湯洗塵");
            let is_maintaining = verb == "把隨身的"
                || result.contains("重新紮緊")
                || result.contains("修整校準")
                || result.contains("砥礪刃口");

            let is_residence = (crate::grid::is_private_residence_room_name(clean_venue)
                || crate::grid::is_private_residence_room_name(raw_venue))
                && (!home_name.contains(clean_venue) && !home_name.contains(raw_venue));

            let is_street_for_indoor_activity = (crate::grid::is_street_or_gate_room_name(clean_venue)
                || crate::grid::is_street_or_gate_room_name(raw_venue)
                || clean_venue.contains("天井")
                || raw_venue.contains("天井"))
                && (is_drinking || is_eating || is_bathing || is_maintaining);

            if is_residence || is_street_for_indoor_activity {
                let target_venue = if is_drinking {
                    "文曜・三水口水岸公井"
                } else if is_eating {
                    "文曜・三水大食堂"
                } else if is_bathing {
                    "文曜・泉香溫水浴場"
                } else if is_maintaining {
                    if !work_name.is_empty() {
                        work_name
                    } else {
                        "文曜・百工修繕坊"
                    }
                } else if !home_name.is_empty() {
                    home_name
                } else {
                    "文曜・三水大食堂"
                };
                result = result.replace(raw_venue, target_venue);
            }
        }
    }

    if result.contains("把隨身的「") && result.contains("」") {
        if let Some(start_v) = result.find("把隨身的「") {
            let after_open = &result[start_v + "把隨身的「".len()..];
            if let Some(end_rel) = after_open.find('」') {
                let vessel_name = &after_open[..end_rel];
                let phys = crate::game::object_ecology::ItemPhysicalClass::classify("", vessel_name);
                let action_phrase = phys.maintenance_action_phrase();
                let target_replacement = format!("{action_phrase}，器況煥然一新，幹起活來順手多了");
                if result.contains("洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了") {
                    result = result.replace(
                        "洗淨擦拭、重新紮緊，器況煥然一新，用起來順手多了",
                        &target_replacement,
                    );
                } else if result.contains("細細修整校準，器況煥然一新，幹起活來順手多了") {
                    result = result.replace(
                        "細細修整校準，器況煥然一新，幹起活來順手多了",
                        &target_replacement,
                    );
                }
            }
        }
    }

    result
}

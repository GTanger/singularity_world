//! store 物品 — items 的讀寫。寫入走 writer queue + JSON 備份。

use super::{Item, Store};

impl Store {
    pub fn get_item(&self, id: &str) -> Option<&Item> {
        self.items.get(id)
    }

    pub fn put_item(&mut self, it: Item) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertItem(it.clone()));
        self.items.insert(it.id.clone(), it);
        self.persist_items()
    }

    /// 將 P0 世界物質補進啟動期快取。PG-first 時 items 表可能仍為空；
    /// 這裡只補缺，不覆蓋既有資料。
    pub fn ensure_p0_material_items(&mut self) -> usize {
        let mut inserted = 0;
        for item in crate::world::materials::material_item_defs() {
            if let Some(existing) = self.items.get_mut(&item.id) {
                if existing.origin.is_empty()
                    || existing.origin.contains("北岸")
                    || existing.origin.contains("中鎮")
                    || existing.origin.contains("短市")
                    || existing.origin.contains("小市")
                    || existing.origin.contains("水尾寮渡頭小市")
                    || existing.origin.contains("補給小市")
                    || existing.origin.contains("邊市")
                {
                    existing.origin = item.origin;
                    existing.description = item.description;
                }
            } else {
                self.items.insert(item.id.clone(), item);
                inserted += 1;
            }
        }
        inserted
    }

    /// 將舊版採集占位物升級為 P0 來源材料，並清理舊版 crystal_* 項目，避免交易仍露出舊物。
    pub fn normalize_legacy_material_inventories(&mut self) -> usize {
        let test_ids: Vec<String> = self
            .entities
            .iter()
            .filter(|(id, e)| {
                id.starts_with("test_npc_")
                    || id.ends_with("_check2")
                    || (e.kind == "player" && id.as_str() != "葉卅")
            })
            .map(|(id, _)| id.clone())
            .collect();
        for tid in test_ids {
            self.entities.remove(&tid);
            self.entity_rooms.remove(&tid);
        }
        let bad_mut_items: Vec<String> = self
            .items
            .keys()
            .filter(|id| {
                id.starts_with("mut_craft_meridian_salve")
                    || id.starts_with("mut_craft_bamboo_oil_umbrella")
                    || id.starts_with("mut_craft_camphor_storage_box")
                    || id.starts_with("mut_craft_wooden_tool_handle")
                    || id.starts_with("mut_craft_pine_soot_ink")
                    || id.starts_with("mut_craft_pine_torch")
                    || id.starts_with("mut_blank_manual")
                    || id.starts_with("mut_wt_cat_0793")
            })
            .cloned()
            .collect();
        let mut items_dirty = !bad_mut_items.is_empty();
        for bid in bad_mut_items {
            self.items.remove(&bid);
        }
        for non_equip_id in [
            "craft_camphor_storage_box",
            "craft_pine_soot_ink",
            "craft_wooden_tool_handle",
        ] {
            if let Some(it) = self.items.get_mut(non_equip_id)
                && !it.slot.is_empty()
            {
                it.slot.clear();
                items_dirty = true;
            }
        }
        if items_dirty {
            let _ = self.persist_items();
        }
        let mut changed = 0;
        for entity in self.entities.values_mut() {
            let mut inv_changed = false;
            if let Some(normalized) =
                crate::world::materials::normalize_legacy_inventory_json(&entity.inventory)
            {
                entity.inventory = normalized;
                inv_changed = true;
            }
            let is_npc = entity.kind == "npc"
                || entity.id.starts_with("npc_")
                || entity.id.starts_with("wt_");
            if let Ok(mut entries) =
                serde_json::from_str::<Vec<crate::db::inventory::InventoryEntry>>(&entity.inventory)
            {
                let before_retain = entries.len();
                entries.retain(|it| it.item_id != "manual_854b8035e7134512");
                if entries.len() != before_retain {
                    inv_changed = true;
                }
                if is_npc && crate::db::inventory::normalize_npc_inventory_entries(&mut entries) {
                    inv_changed = true;
                }
                if inv_changed {
                    if let Ok(serialized) = serde_json::to_string(&entries) {
                        entity.inventory = serialized;
                        entity.inventory_seq = entity.inventory_seq.saturating_add(1);
                    }
                }
            }
            if inv_changed {
                crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityEconomySnapshot {
                    entity_id: entity.id.clone(),
                    magnesium: entity.magnesium,
                    inventory: entity.inventory.clone(),
                    inventory_seq: entity.inventory_seq,
                    current_activity: entity.current_activity.clone(),
                });
                changed += 1;
            }
        }
        changed
    }
}

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
            if !self.items.contains_key(&item.id) {
                self.items.insert(item.id.clone(), item);
                inserted += 1;
            }
        }
        inserted
    }

    /// 將舊版採集占位物升級為 P0 來源材料，避免交易仍露出 `wild_herb`。
    pub fn normalize_legacy_material_inventories(&mut self) -> usize {
        let mut changed = 0;
        for entity in self.entities.values_mut() {
            if let Some(normalized) =
                crate::world::materials::normalize_legacy_inventory_json(&entity.inventory)
            {
                entity.inventory = normalized;
                changed += 1;
            }
        }
        changed
    }
}

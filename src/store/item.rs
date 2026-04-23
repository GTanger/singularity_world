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
}

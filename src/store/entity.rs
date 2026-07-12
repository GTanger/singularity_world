//! store 實體 — entities + entity_rooms 的讀寫、座標綁定、轉帳。
//! 寫入路徑走 writer queue（PG 權威）+ 必要時 in-memory mirror；entity_rooms 不再寫 JSON。

use super::{Entity, Store};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TradeInventoryEntry {
    #[serde(rename = "item_id")]
    item_id: String,
    qty: i32,
}

fn parse_trade_inventory(raw: &str) -> anyhow::Result<Vec<TradeInventoryEntry>> {
    if raw.trim().is_empty() || raw.trim() == "[]" {
        return Ok(Vec::new());
    }
    Ok(serde_json::from_str(raw)?)
}

fn add_trade_inventory_item(entries: &mut Vec<TradeInventoryEntry>, item_id: &str, qty: i32) {
    for ent in entries.iter_mut() {
        if ent.item_id == item_id {
            ent.qty += qty;
            return;
        }
    }
    entries.push(TradeInventoryEntry {
        item_id: item_id.to_string(),
        qty,
    });
}

fn remove_trade_inventory_item(
    entries: &mut Vec<TradeInventoryEntry>,
    item_id: &str,
    qty: i32,
) -> anyhow::Result<()> {
    let Some(idx) = entries
        .iter()
        .position(|ent| ent.item_id == item_id && !ent.item_id.is_empty())
    else {
        anyhow::bail!("賣方沒有這件商品");
    };
    if entries[idx].qty < qty {
        anyhow::bail!("賣方商品不足");
    }
    entries[idx].qty -= qty;
    entries.retain(|ent| ent.qty > 0 && !ent.item_id.is_empty());
    Ok(())
}

impl Store {
    // ── entity_rooms ──

    pub fn get_entity_room(&self, entity_id: &str) -> String {
        self.entity_rooms
            .get(entity_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_entity_room(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.entity_rooms
            .insert(entity_id.to_string(), room_id.to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityRoom {
            entity_id: entity_id.to_string(),
            room_id: room_id.to_string(),
        });
        Ok(())
    }

    /// 設定實體的表面可觀測行為。
    pub fn set_entity_activity(&mut self, entity_id: &str, activity: &str) -> anyhow::Result<()> {
        if let Some(e) = self.entities.get_mut(entity_id) {
            e.current_activity = activity.to_string();
        }
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityActivity {
            entity_id: entity_id.to_string(),
            activity: activity.to_string(),
        });
        Ok(())
    }

    pub fn entity_ids_in_room(&self, room_id: &str) -> Vec<String> {
        self.entity_rooms
            .iter()
            .filter(|(_, rid)| rid.as_str() == room_id)
            .map(|(eid, _)| eid.clone())
            .collect()
    }

    pub fn all_entity_ids(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    pub fn get_npc_ids_with_room(&self) -> Vec<String> {
        self.entity_rooms
            .keys()
            .filter(|eid| {
                self.entities
                    .get(eid.as_str())
                    .is_some_and(|e| e.kind == "npc" && e.vit > 0)
            })
            .cloned()
            .collect()
    }

    pub fn get_player_ids_with_room(&self) -> Vec<String> {
        self.entity_rooms
            .keys()
            .filter(|eid| {
                self.entities
                    .get(eid.as_str())
                    .is_some_and(|e| e.kind == "player")
            })
            .cloned()
            .collect()
    }

    // ── entities ──

    pub fn get_entity(&self, id: &str) -> Option<Entity> {
        self.entities.get(id).cloned()
    }

    pub fn put_entity(&mut self, e: Entity) -> anyhow::Result<()> {
        let mut e = e;
        if e.activated_nodes.is_empty() {
            e.activated_nodes = r#"["N000"]"#.to_string();
        }
        if e.inventory.is_empty() {
            e.inventory = "[]".to_string();
        }
        self.entities.insert(e.id.clone(), e.clone());
        crate::pg::entity::upsert_async(e);
        Ok(())
    }

    pub fn update_entity(&mut self, id: &str, f: impl FnOnce(&mut Entity)) -> anyhow::Result<()> {
        if let Some(e) = self.entities.get_mut(id) {
            f(e);
            let e_clone = e.clone();
            crate::pg::entity::upsert_async(e_clone);
        }
        Ok(())
    }

    /// 設定實體在正方格世界上的權威座標；寫入 PG 與快取。
    pub fn set_entity_grid(&mut self, entity_id: &str, x: i32, y: i32) -> anyhow::Result<()> {
        self.update_entity(entity_id, |e| {
            e.grid_x = Some(x);
            e.grid_y = Some(y);
        })?;
        let rid = crate::grid::grid_room_id_from_coord(x, y);
        self.set_entity_room(entity_id, &rid)
    }

    /// 與指定正方格座標重合的實體 id（依 `grid_x` / `grid_y`）。
    pub fn entity_ids_at_grid(&self, x: i32, y: i32) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.grid_x == Some(x) && e.grid_y == Some(y))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 有正方格座標的存活 NPC id 列表。
    pub fn get_npc_ids_with_grid(&self) -> Vec<String> {
        self.entities
            .values()
            .filter(|e| e.kind == "npc" && e.vit > 0 && e.grid_x.is_some() && e.grid_y.is_some())
            .map(|e| e.id.clone())
            .collect()
    }

    pub fn transfer_magnesium(
        &mut self,
        from_id: &str,
        to_id: &str,
        amount: i32,
    ) -> anyhow::Result<()> {
        if amount <= 0 {
            anyhow::bail!("轉帳鎂數須為正");
        }
        let from_mg = self
            .entities
            .get(from_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉出實體"))?
            .magnesium;
        let _to = self
            .entities
            .get(to_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉入實體"))?;
        if from_mg < amount {
            anyhow::bail!("鎂不足");
        }
        self.entities.get_mut(from_id).unwrap().magnesium = from_mg - amount;
        let to_mg = self.entities.get(to_id).unwrap().magnesium;
        self.entities.get_mut(to_id).unwrap().magnesium = to_mg + amount;
        if let Some(from_e) = self.entities.get(from_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(from_e.clone()));
        }
        if let Some(to_e) = self.entities.get(to_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(to_e.clone()));
        }
        self.persist_entities()
    }

    pub fn settle_inventory_trade(
        &mut self,
        buyer_id: &str,
        seller_id: &str,
        item_id: &str,
        qty: i32,
        price_mg: i32,
    ) -> anyhow::Result<()> {
        if buyer_id == seller_id {
            anyhow::bail!("交易雙方不能是同一實體");
        }
        if item_id.is_empty() || qty <= 0 {
            anyhow::bail!("交易物品無效");
        }
        if price_mg <= 0 {
            anyhow::bail!("交易鎂數須為正");
        }

        let (buyer_mg, buyer_inventory, seller_mg, seller_inventory) = {
            let buyer = self
                .entities
                .get(buyer_id)
                .ok_or_else(|| anyhow::anyhow!("找不到買方實體"))?;
            let seller = self
                .entities
                .get(seller_id)
                .ok_or_else(|| anyhow::anyhow!("找不到賣方實體"))?;
            (
                buyer.magnesium,
                buyer.inventory.clone(),
                seller.magnesium,
                seller.inventory.clone(),
            )
        };
        if buyer_mg < price_mg {
            anyhow::bail!("買方鎂不足");
        }

        let mut seller_entries = parse_trade_inventory(&seller_inventory)?;
        remove_trade_inventory_item(&mut seller_entries, item_id, qty)?;
        let mut buyer_entries = parse_trade_inventory(&buyer_inventory)?;
        add_trade_inventory_item(&mut buyer_entries, item_id, qty);

        let seller_inventory = serde_json::to_string(&seller_entries)?;
        let buyer_inventory = serde_json::to_string(&buyer_entries)?;

        if let Some(buyer) = self.entities.get_mut(buyer_id) {
            buyer.magnesium = buyer_mg - price_mg;
            buyer.inventory = buyer_inventory;
        }
        if let Some(seller) = self.entities.get_mut(seller_id) {
            seller.magnesium = seller_mg + price_mg;
            seller.inventory = seller_inventory;
        }

        if let Some(buyer) = self.entities.get(buyer_id) {
            crate::pg::entity::upsert_async(buyer.clone());
        }
        if let Some(seller) = self.entities.get(seller_id) {
            crate::pg::entity::upsert_async(seller.clone());
        }
        self.persist_entities()
    }

    pub fn get_entities_in_box(
        &self,
        x_min: i32,
        x_max: i32,
        y_min: i32,
        y_max: i32,
        kind: &str,
    ) -> Vec<Entity> {
        self.entities
            .values()
            .filter(|e| {
                (kind.is_empty() || e.kind == kind)
                    && e.x >= x_min
                    && e.x <= x_max
                    && e.y >= y_min
                    && e.y <= y_max
            })
            .cloned()
            .collect()
    }

    pub fn get_moving_entity_ids(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.move_state == "moving")
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn npc_ids_with_missing_soul_seed(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && e.soul_seed.is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 回傳缺少方格座標（`grid_x` 或 `grid_y` 為 None）的 NPC id 列表。
    pub fn npc_ids_missing_grid(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && (e.grid_x.is_none() || e.grid_y.is_none()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn clear_all_entities(&mut self) -> anyhow::Result<()> {
        self.entities.clear();
        self.entity_rooms.clear();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute("DELETE FROM entities", &[]);
            let _ = conn.execute("DELETE FROM entity_rooms", &[]);
        }
        self.persist_entities()?;
        self.persist_entity_rooms()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_store() -> Store {
        Store {
            rooms: HashMap::new(),
            exits: HashMap::new(),
            entity_rooms: HashMap::new(),
            venues: HashMap::new(),
            assignments: HashMap::new(),
            schedules: HashMap::new(),
            entities: HashMap::new(),
            items: HashMap::new(),
            npc_threads: HashMap::new(),
            npc_dyads: HashMap::new(),
            npc_rumors: HashMap::new(),
            npc_rumor_digest: None,
            db_pool: None,
            rooms_path: String::new(),
            runtime_dir: String::new(),
            entity_rooms_path: PathBuf::new(),
            venues_path: PathBuf::new(),
            assignments_path: PathBuf::new(),
            schedules_path: PathBuf::new(),
            entities_path: PathBuf::new(),
            items_path: PathBuf::new(),
            npc_thread_path: PathBuf::new(),
            npc_dyad_path: PathBuf::new(),
            npc_rumor_path: PathBuf::new(),
            npc_rumor_digest_path: PathBuf::new(),
        }
    }

    fn test_entity(id: &str, magnesium: i32, inventory: &str) -> Entity {
        Entity {
            id: id.to_string(),
            kind: "npc".to_string(),
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: String::new(),
            target_x: None,
            target_y: None,
            walk_or_run: String::new(),
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium,
            last_observed_at: None,
            created_at: 0,
            gender: String::new(),
            soul_seed: None,
            display_title: String::new(),
            activated_nodes: String::new(),
            equipment_slots: String::new(),
            inventory: inventory.to_string(),
            disposition: 0,
            current_activity: String::new(),
            grid_x: None,
            grid_y: None,
            thoughts: Vec::new(),
        }
    }

    #[test]
    fn settle_inventory_trade_moves_money_and_item_once() {
        let mut store = test_store();
        store
            .entities
            .insert("buyer".to_string(), test_entity("buyer", 10, "[]"));
        store.entities.insert(
            "seller".to_string(),
            test_entity("seller", 1, r#"[{"item_id":"wild_herb","qty":2}]"#),
        );

        store
            .settle_inventory_trade("buyer", "seller", "wild_herb", 1, 5)
            .unwrap();

        let buyer = store.entities.get("buyer").unwrap();
        let seller = store.entities.get("seller").unwrap();
        assert_eq!(buyer.magnesium, 5);
        assert_eq!(seller.magnesium, 6);
        let buyer_inventory = parse_trade_inventory(&buyer.inventory).unwrap();
        let seller_inventory = parse_trade_inventory(&seller.inventory).unwrap();
        assert_eq!(buyer_inventory[0].item_id, "wild_herb");
        assert_eq!(buyer_inventory[0].qty, 1);
        assert_eq!(seller_inventory[0].item_id, "wild_herb");
        assert_eq!(seller_inventory[0].qty, 1);
    }

    #[test]
    fn settle_inventory_trade_rejects_missing_item_without_moving_money() {
        let mut store = test_store();
        store
            .entities
            .insert("buyer".to_string(), test_entity("buyer", 10, "[]"));
        store
            .entities
            .insert("seller".to_string(), test_entity("seller", 1, "[]"));

        let result = store.settle_inventory_trade("buyer", "seller", "wild_herb", 1, 5);

        assert!(result.is_err());
        assert_eq!(store.entities.get("buyer").unwrap().magnesium, 10);
        assert_eq!(store.entities.get("seller").unwrap().magnesium, 1);
        assert_eq!(store.entities.get("buyer").unwrap().inventory, "[]");
        assert_eq!(store.entities.get("seller").unwrap().inventory, "[]");
    }
}

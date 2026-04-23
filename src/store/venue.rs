//! store 場所 — venues 的查詢。純讀取，純記憶體快取。
//! 場所結構變更由 `sync_all_to_postgresql` 統一處理，不在此模組寫入。

use super::{Store, Venue};

impl Store {
    pub fn get_venue(&self, id: &str) -> Option<&Venue> {
        self.venues.get(id)
    }

    pub fn is_room_in_venue(&self, room_id: &str, venue_id: &str) -> bool {
        self.venues
            .get(venue_id)
            .is_some_and(|v| v.room_ids.iter().any(|r| r == room_id))
    }

    pub fn get_venue_ids_for_room(&self, room_id: &str) -> Vec<String> {
        self.venues
            .iter()
            .filter(|(_, v)| v.room_ids.iter().any(|r| r == room_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_venue_max_staff(&self, venue_id: &str, default_max: i32) -> i32 {
        self.venues
            .get(venue_id)
            .map(|v| {
                if v.max_staff > 0 {
                    v.max_staff
                } else {
                    default_max
                }
            })
            .unwrap_or(default_max)
    }

    pub fn get_all_venue_ids(&self) -> Vec<String> {
        self.venues.keys().cloned().collect()
    }

    pub fn get_all_venue_room_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self
            .venues
            .values()
            .flat_map(|v| v.room_ids.clone())
            .collect();
        ids.sort();
        ids.dedup();
        ids
    }
}

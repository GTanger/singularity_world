//! store 房間 — rooms + exits 的讀寫、重命名、重新載入。
//! JSON（editor/*.json）+ PG rooms/exits 雙寫；runtime 以 in-memory HashMap 為快取。

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::model;

use super::{ExitOut, RoomFileOne, Store};

impl Store {
    /// 房間 JSON 根目錄（如 `data/rooms`），供城市模組掃描 editor 中繼資料。
    pub fn rooms_path(&self) -> &str {
        self.rooms_path.as_str()
    }

    pub fn room_ids(&self) -> Vec<String> {
        self.rooms.keys().cloned().collect()
    }

    pub fn get_room(&self, id: &str) -> Option<model::Room> {
        self.rooms.get(id).cloned()
    }

    pub fn get_room_name(&self, id: &str) -> String {
        self.rooms
            .get(id)
            .map(|r| r.name.clone())
            .unwrap_or_default()
    }

    pub fn get_room_id_by_name(&self, name: &str) -> String {
        self.rooms
            .values()
            .find(|r| r.name == name)
            .map(|r| r.id.clone())
            .unwrap_or_default()
    }

    pub fn get_rooms_by_tag(&self, tag: &str) -> Vec<String> {
        let lt = tag.trim().to_lowercase();
        self.rooms
            .iter()
            .filter(|(_, r)| r.tags.iter().any(|t| t.trim().to_lowercase() == lt))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_rooms_by_zone(&self, zone: &str) -> Vec<String> {
        let lz = zone.trim().to_lowercase();
        self.rooms
            .iter()
            .filter(|(_, r)| r.zone.trim().to_lowercase() == lz)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn get_exits_for_room(&self, from_room_id: &str) -> Vec<model::Exit> {
        self.exits.get(from_room_id).cloned().unwrap_or_default()
    }

    /// 僅寫入記憶體房間表（不寫 editor JSON、不寫 PG）；供啟動時注入城市 GeoJSON 解析結果。
    pub fn inject_ephemeral_rooms(
        &mut self,
        rooms: HashMap<String, model::Room>,
        exits: HashMap<String, Vec<model::Exit>>,
    ) {
        for (id, room) in rooms {
            self.rooms.insert(id, room);
        }
        for (id, mut ex_list) in exits {
            for ex in &mut ex_list {
                if ex.to_room_name.is_empty()
                    && let Some(r) = self.rooms.get(&ex.to_room_id)
                {
                    ex.to_room_name = r.name.clone();
                }
            }
            self.exits.insert(id, ex_list);
        }
    }

    /// 在既有房間的出口列表末尾加一筆（同 direction + 目標房不重複）。
    pub fn append_exit_unique(&mut self, room_id: &str, mut exit: model::Exit) {
        if room_id.is_empty() || exit.direction.is_empty() || exit.to_room_id.is_empty() {
            return;
        }
        if exit.to_room_name.is_empty()
            && let Some(r) = self.rooms.get(&exit.to_room_id)
        {
            exit.to_room_name = r.name.clone();
        }
        let list = self.exits.entry(room_id.to_string()).or_default();
        if list
            .iter()
            .any(|e| e.direction == exit.direction && e.to_room_id == exit.to_room_id)
        {
            return;
        }
        list.push(exit);
    }

    /// 更新房間資料並寫回 JSON 與 PostgreSQL。
    pub fn upsert_room_data(&mut self, room: model::Room, exits: Option<Vec<model::Exit>>) {
        self.upsert_room_data_internal(room, exits, true);
    }

    pub(super) fn upsert_room_data_internal(
        &mut self,
        room: model::Room,
        exits: Option<Vec<model::Exit>>,
        sync_graph: bool,
    ) {
        if room.id.is_empty() {
            return;
        }
        let id = room.id.clone();

        self.rooms.insert(id.clone(), room.clone());
        if let Some(exits_val) = exits {
            let enriched: Vec<model::Exit> = exits_val
                .into_iter()
                .filter(|ex| !ex.direction.is_empty() && !ex.to_room_id.is_empty())
                .map(|mut ex| {
                    if ex.to_room_name.is_empty()
                        && let Some(r) = self.rooms.get(&ex.to_room_id)
                    {
                        ex.to_room_name = r.name.clone();
                    }
                    ex
                })
                .collect();
            self.exits.insert(id.clone(), enriched);
        }

        let room_json_path = PathBuf::from(&self.rooms_path)
            .join("editor")
            .join(format!("{}.json", id));
        let exits_for_json = self.exits.get(&id).cloned().unwrap_or_default();
        let file_data = RoomFileOne {
            id: id.clone(),
            name: room.name.clone(),
            description: room.description.clone(),
            tags: room.tags.clone(),
            zone: room.zone.clone(),
            exits: exits_for_json
                .iter()
                .map(|e| ExitOut {
                    direction: e.direction.clone(),
                    to: e.to_room_id.clone(),
                })
                .collect(),
            objects: room.objects.clone(),
        };
        if let Ok(json) = serde_json::to_string_pretty(&file_data)
            && let Err(e) = std::fs::write(&room_json_path, json)
        {
            tracing::error!(
                "[store] Failed to write room JSON to {:?}: {}",
                room_json_path,
                e
            );
        }

        if let Some(pool) = &self.db_pool {
            let mut conn = match pool.get() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("[store] Failed to get DB connection: {}", e);
                    return;
                }
            };

            let tags = &room.tags;
            let objects_json =
                serde_json::to_string(&room.objects).unwrap_or_else(|_| "[]".to_string());
            let res = conn.execute(
                "INSERT INTO rooms (id, name, description, zone, tags, objects)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name,
                    description = EXCLUDED.description,
                    zone = EXCLUDED.zone,
                    tags = EXCLUDED.tags,
                    objects = EXCLUDED.objects",
                &[
                    &room.id,
                    &room.name,
                    &room.description,
                    &room.zone,
                    tags,
                    &objects_json,
                ],
            );
            if let Err(e) = res {
                tracing::error!("[store] Failed to upsert room {} to DB: {}", room.id, e);
            }

            let _ = conn.execute("DELETE FROM exits WHERE from_room_id = $1", &[&room.id]);
            for ex in self.exits.get(&id).cloned().unwrap_or_default() {
                let _ = conn.execute(
                    "INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ($1, $2, $3)
                     ON CONFLICT DO NOTHING",
                    &[&room.id, &ex.direction, &ex.to_room_id],
                );
            }
        }

        if sync_graph {
            crate::db::sync_room_graph_with_store(self);
        }
    }

    pub fn delete_room_data(&mut self, room_id: &str) {
        if room_id.is_empty() {
            return;
        }
        let id = room_id.to_string();
        self.rooms.remove(&id);
        self.exits.remove(&id);
        for exits in self.exits.values_mut() {
            exits.retain(|ex| ex.to_room_id != id);
        }

        let room_json_path = PathBuf::from(&self.rooms_path)
            .join("editor")
            .join(format!("{}.json", id));
        if room_json_path.exists() {
            let _ = std::fs::remove_file(room_json_path);
        }

        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute("DELETE FROM rooms WHERE id = $1", &[&id]);
            let _ = conn.execute(
                "DELETE FROM exits WHERE from_room_id = $1 OR to_room_id = $1",
                &[&id],
            );
        }

        crate::db::sync_room_graph_with_store(self);
    }

    /// 重新命名房間 ID，並級聯更新所有引用（出口、物件）。
    pub fn rename_room(&mut self, old_id: &str, new_id: &str) -> anyhow::Result<()> {
        if self.rooms.contains_key(new_id) {
            anyhow::bail!("new id already exists");
        }
        let Some(room) = self.rooms.get(old_id).cloned() else {
            anyhow::bail!("room not found");
        };
        let exits = self.get_exits_for_room(old_id);

        let old_path = PathBuf::from(&self.rooms_path)
            .join("editor")
            .join(format!("{}.json", old_id));
        let new_path = PathBuf::from(&self.rooms_path)
            .join("editor")
            .join(format!("{}.json", new_id));
        if old_path.exists() {
            fs::rename(&old_path, &new_path)?;
        }

        self.rooms.remove(old_id);
        self.exits.remove(old_id);

        let mut new_room = room;
        new_room.id = new_id.to_string();
        self.upsert_room_data_internal(new_room, Some(exits), false);

        let mut to_update = Vec::new();

        for (rid, r) in self.rooms.iter_mut() {
            let mut changed = false;
            for obj in &mut r.objects {
                if obj.move_to_room_id == old_id {
                    obj.move_to_room_id = new_id.to_string();
                    changed = true;
                }
                if obj.id == old_id {
                    obj.id = new_id.to_string();
                    changed = true;
                }
            }
            if changed {
                to_update.push(rid.clone());
            }
        }

        for (rid, ex_list) in self.exits.iter_mut() {
            let mut changed = false;
            for ex in ex_list.iter_mut() {
                if ex.to_room_id == old_id {
                    ex.to_room_id = new_id.to_string();
                    changed = true;
                }
            }
            if changed && !to_update.contains(rid) {
                to_update.push(rid.clone());
            }
        }

        for rid in to_update {
            if let Some(r) = self.rooms.get(&rid).cloned() {
                let exs = self.exits.get(&rid).cloned();
                self.upsert_room_data_internal(r, exs, false);
            }
        }

        for room_id in self.entity_rooms.values_mut() {
            if room_id == old_id {
                *room_id = new_id.to_string();
            }
        }

        if let Some(pool) = &self.db_pool {
            let mut conn = pool.get()?;
            conn.execute(
                "UPDATE npc_rumors SET room_id = $1 WHERE room_id = $2",
                &[&new_id, &old_id],
            )?;
            conn.execute("DELETE FROM rooms WHERE id = $1", &[&old_id])?;
            conn.execute("DELETE FROM exits WHERE from_room_id = $1", &[&old_id])?;
        }

        crate::db::sync_room_graph_with_store(self);
        Ok(())
    }

    pub fn reload_rooms(&mut self) -> anyhow::Result<()> {
        self.rooms.clear();
        self.exits.clear();
        let path = self.rooms_path.clone();
        self.load_rooms(&path)?;

        if let Err(e) = self.sync_all_to_postgresql() {
            tracing::error!(
                "[store] Failed to sync rooms to PostgreSQL during reload: {}",
                e
            );
        }

        crate::db::sync_room_graph_with_store(self);
        Ok(())
    }
}

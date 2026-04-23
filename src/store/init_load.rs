//! JSON／檔案系統載入：Store 初始化階段專用。執行期寫回走 `persist` 子模組。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::model;

use super::{
    dyad_key, AssignmentsFile, EntitiesFile, EntityRoomsFile, ItemsFile, NpcDyadsFile, NpcRumor,
    NpcRumorDigest, NpcRumorDigestFile, NpcRumorsFile, NpcThreadsFile, RoomFileOne, RoomsFile,
    SchedulesFile, Store, VenuesFile,
};

impl Store {
    pub(super) fn assignments_count(&self) -> usize {
        self.assignments.values().map(|v| v.len()).sum()
    }

    // ── 房間載入 ──

    pub(super) fn load_rooms(&mut self, path: &str) -> anyhow::Result<()> {
        let p = Path::new(path);
        if p.is_dir() {
            self.load_rooms_from_dir(p)?;
        } else {
            self.load_rooms_from_file(p)?;
        }
        Ok(())
    }

    fn load_rooms_from_file(&mut self, path: &Path) -> anyhow::Result<()> {
        let raw = fs::read_to_string(path)?;
        let f: RoomsFile = serde_json::from_str(&raw)?;
        let mut name_by_id: HashMap<String, String> = HashMap::new();
        for r in &f.rooms {
            self.rooms.insert(r.id.clone(), model::Room {
                id: r.id.clone(),
                name: r.name.clone(),
                tags: r.tags.clone(),
                zone: r.zone.clone(),
                description: r.description.clone(),
                objects: Vec::new(),
            });
            name_by_id.insert(r.id.clone(), r.name.clone());
        }
        for e in &f.exits {
            let to_name = name_by_id.get(&e.to).cloned().unwrap_or_default();
            self.exits.entry(e.from.clone()).or_default().push(model::Exit {
                direction: e.direction.clone(),
                to_room_id: e.to.clone(),
                to_room_name: to_name,
            });
        }
        self.prune_non_street_rooms(None);
        Ok(())
    }

    fn load_rooms_from_dir(&mut self, dir: &Path) -> anyhow::Result<()> {
        struct FileEntry {
            room: RoomFileOne,
            is_editor: bool,
        }
        let mut list: Vec<FileEntry> = Vec::new();
        Self::walk_json_dir(dir, &mut |path, rel_path| {
            let raw = fs::read_to_string(path)?;
            if let Ok(one) = serde_json::from_str::<RoomFileOne>(&raw) {
                let is_editor = rel_path.starts_with("editor/");
                list.push(FileEntry { room: one, is_editor });
            }
            Ok(())
        })?;

        let mut editor_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut name_by_id: HashMap<String, String> = HashMap::new();

        for entry in &list {
            let one = &entry.room;
            self.rooms.insert(one.id.clone(), model::Room {
                id: one.id.clone(),
                name: one.name.clone(),
                tags: one.tags.clone(),
                zone: one.zone.clone(),
                description: one.description.clone(),
                objects: one.objects.clone(),
            });
            name_by_id.insert(one.id.clone(), one.name.clone());
            if entry.is_editor && !one.id.is_empty() {
                editor_ids.insert(one.id.clone());
            }
        }
        for entry in &list {
            let one = &entry.room;
            for ex in &one.exits {
                let to_name = name_by_id.get(&ex.to).cloned().unwrap_or_default();
                self.exits.entry(one.id.clone()).or_default().push(model::Exit {
                    direction: ex.direction.clone(),
                    to_room_id: ex.to.clone(),
                    to_room_name: to_name,
                });
            }
        }
        self.prune_non_street_rooms(Some(&editor_ids));
        Ok(())
    }

    /// 遞迴掃描 dir 下所有 .json（跳過底線開頭），呼叫 f(path, rel_path)。
    fn walk_json_dir(dir: &Path, f: &mut dyn FnMut(&Path, &str) -> anyhow::Result<()>) -> anyhow::Result<()> {
        fn walk_inner(base: &Path, current: &Path, f: &mut dyn FnMut(&Path, &str) -> anyhow::Result<()>) -> anyhow::Result<()> {
            let entries = match fs::read_dir(current) {
                Ok(e) => e,
                Err(_) => return Ok(()),
            };
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    walk_inner(base, &path, f)?;
                } else if path.extension().is_some_and(|e| e == "json") {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy();
                    if name.starts_with('_') {
                        continue;
                    }
                    let rel = path.strip_prefix(base)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    f(&path, &rel)?;
                }
            }
            Ok(())
        }
        walk_inner(dir, dir, f)
    }

    fn is_street_or_alley(room: &model::Room) -> bool {
        if room.name.contains("大街") || room.name.contains("巷") {
            return true;
        }
        room.tags.iter().any(|t| {
            let lt = t.trim().to_lowercase();
            lt == "street" || lt == "alley"
        })
    }

    fn prune_non_street_rooms(&mut self, exempt_ids: Option<&std::collections::HashSet<String>>) {
        let keep: std::collections::HashSet<String> = self.rooms.iter()
            .filter(|(id, r)| {
                exempt_ids.is_some_and(|e| e.contains(id.as_str()))
                    || Self::is_street_or_alley(r)
            })
            .map(|(id, _)| id.clone())
            .collect();

        self.rooms.retain(|id, _| keep.contains(id));
        self.exits.retain(|id, _| keep.contains(id));
        for exits in self.exits.values_mut() {
            exits.retain(|ex| keep.contains(&ex.to_room_id));
        }
    }

    pub(super) fn first_room_id_sorted(&self) -> String {
        let mut ids: Vec<&String> = self.rooms.keys().collect();
        ids.sort();
        ids.first().map(|s| s.to_string()).unwrap_or_default()
    }

    // ── entity_rooms ──

    pub(super) fn load_entity_rooms(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.entity_rooms_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: EntityRoomsFile = serde_json::from_str(&raw)?;
        let fallback = self.first_room_id_sorted();
        for e in f.entries {
            if self.rooms.contains_key(&e.room_id) {
                self.entity_rooms.insert(e.entity_id, e.room_id);
            } else if !fallback.is_empty() {
                self.entity_rooms.insert(e.entity_id, fallback.clone());
            }
        }
        Ok(())
    }

    // ── venues / assignments / schedules ──

    pub(super) fn load_venues(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.venues_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: VenuesFile = serde_json::from_str(&raw)?;
        for v in f.venues {
            self.venues.insert(v.id.clone(), v);
        }
        Ok(())
    }

    pub(super) fn load_assignments(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.assignments_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: AssignmentsFile = serde_json::from_str(&raw)?;
        for a in f.entries {
            self.assignments.entry(a.entity_id.clone()).or_default().push(a);
        }
        Ok(())
    }

    pub(super) fn load_schedules(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.schedules_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: SchedulesFile = serde_json::from_str(&raw)?;
        for s in f.entries {
            self.schedules.insert(s.entity_id.clone(), s);
        }
        Ok(())
    }

    // ── entities / items ──

    pub(super) fn load_entities(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.entities_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: EntitiesFile = serde_json::from_str(&raw)?;
        for e in f.entities {
            self.entities.insert(e.id.clone(), e);
        }
        Ok(())
    }

    pub(super) fn load_items(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.items_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: ItemsFile = serde_json::from_str(&raw)?;
        for it in f.items {
            self.items.insert(it.id.clone(), it);
        }
        Ok(())
    }

    // ── npc threads / dyads / rumors ──

    pub(super) fn load_npc_threads(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_thread_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcThreadsFile = serde_json::from_str(&raw)?;
        for t in f.entries {
            self.npc_threads.insert(t.thread_key.clone(), t);
        }
        Ok(())
    }

    pub(super) fn load_npc_dyads(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_dyad_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcDyadsFile = serde_json::from_str(&raw)?;
        for d in f.entries {
            let key = dyad_key(&d.a_id, &d.b_id);
            self.npc_dyads.insert(key, d);
        }
        Ok(())
    }

    pub(super) fn load_npc_rumors(&mut self) -> anyhow::Result<()> {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get() {
                if let Ok(rows) = conn.query(
                    "SELECT id, text, room_id, zone, source, source_score, weight, mention_count,
                            last_used_at, blocked_until, penalty_count, last_penalty_at,
                            last_penalty_reason, updated_at, expires_at FROM npc_rumors",
                    &[]
                ) {
                    for row in rows {
                        let r = NpcRumor {
                            id: row.get(0),
                            text: row.get(1),
                            room_id: row.get(2),
                            zone: row.get(3),
                            source: row.get(4),
                            source_score: row.get(5),
                            weight: row.get(6),
                            mention_count: row.get(7),
                            last_used_at: row.get(8),
                            blocked_until: row.get(9),
                            penalty_count: row.get(10),
                            last_penalty_at: row.get(11),
                            last_penalty_reason: row.get(12),
                            updated_at: row.get(13),
                            expires_at: row.get(14),
                        };
                        self.npc_rumors.insert(r.id.clone(), r);
                    }
                    return Ok(());
                }
            }

        // Fallback to JSON
        let raw = match fs::read_to_string(&self.npc_rumor_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        let f: NpcRumorsFile = serde_json::from_str(&raw)?;
        for r in f.entries {
            self.npc_rumors.insert(r.id.clone(), r);
        }
        Ok(())
    }

    pub(super) fn load_npc_rumor_digest(&mut self) -> anyhow::Result<()> {
        let raw = match fs::read_to_string(&self.npc_rumor_digest_path) {
            Ok(r) => r,
            Err(_) => return Ok(()),
        };
        if let Ok(w) = serde_json::from_str::<NpcRumorDigestFile>(&raw) {
            self.npc_rumor_digest = Some(w.digest);
            return Ok(());
        }
        if let Ok(d) = serde_json::from_str::<NpcRumorDigest>(&raw) {
            self.npc_rumor_digest = Some(d);
        }
        Ok(())
    }
}

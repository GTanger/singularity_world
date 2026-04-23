//! PostgreSQL 同步與全量載入：啟動時雙向同步。執行期單筆 upsert 走 `persist`。

use super::{dyad_key, Assignment, Entity, Item, NpcDyad, NpcThread, Schedule, Store, Venue};

impl Store {
    /// 只同步房間與出口到 PostgreSQL（每次啟動時呼叫，因房間從檔案系統載入）。
    pub(super) fn sync_rooms_to_postgresql(&self) -> anyhow::Result<()> {
        let Some(pool) = &self.db_pool else { return Ok(()) };
        let mut conn = pool.get()?;
        let mut trans = conn.transaction()?;
        for room in self.rooms.values() {
            let tags = &room.tags;
            let objects_json = serde_json::to_string(&room.objects).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO rooms (id, name, description, zone, tags, objects)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name, description = EXCLUDED.description,
                    zone = EXCLUDED.zone, tags = EXCLUDED.tags, objects = EXCLUDED.objects",
                &[&room.id, &room.name, &room.description, &room.zone, tags, &objects_json],
            )?;
            trans.execute("DELETE FROM exits WHERE from_room_id = $1", &[&room.id])?;
            if let Some(exs) = self.exits.get(&room.id) {
                for ex in exs {
                    trans.execute(
                        "INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ($1, $2, $3)
                         ON CONFLICT DO NOTHING",
                        &[&room.id, &ex.direction, &ex.to_room_id],
                    )?;
                }
            }
        }
        trans.commit()?;
        tracing::info!("[store] {} 個房間已同步至 PostgreSQL", self.rooms.len());
        Ok(())
    }

    /// 將目前內存中所有資料同步到 PostgreSQL（全量 upsert）。
    /// 用途：啟動時把 JSON 種子灌入 DB（如果 DB 是空的或有差異）。
    pub fn sync_all_to_postgresql(&self) -> anyhow::Result<()> {
        let Some(pool) = &self.db_pool else { return Ok(()) };
        let mut conn = pool.get()?;
        let mut trans = conn.transaction()?;

        // ── 房間 + 出口 ──
        tracing::info!("[store] Syncing {} rooms to PostgreSQL...", self.rooms.len());
        for room in self.rooms.values() {
            let tags = &room.tags;
            let objects_json = serde_json::to_string(&room.objects).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO rooms (id, name, description, zone, tags, objects)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (id) DO UPDATE SET
                    name = EXCLUDED.name, description = EXCLUDED.description,
                    zone = EXCLUDED.zone, tags = EXCLUDED.tags, objects = EXCLUDED.objects",
                &[&room.id, &room.name, &room.description, &room.zone, tags, &objects_json],
            )?;
            trans.execute("DELETE FROM exits WHERE from_room_id = $1", &[&room.id])?;
            if let Some(exs) = self.exits.get(&room.id) {
                for ex in exs {
                    trans.execute(
                        "INSERT INTO exits (from_room_id, direction, to_room_id) VALUES ($1, $2, $3)
                         ON CONFLICT DO NOTHING",
                        &[&room.id, &ex.direction, &ex.to_room_id],
                    )?;
                }
            }
        }

        // ── 實體 ──
        tracing::info!("[store] Syncing {} entities to PostgreSQL...", self.entities.len());
        for e in self.entities.values() {
            trans.execute(
                "INSERT INTO entities (id, kind, display_char, x, y, move_state, target_x, target_y,
                    walk_or_run, move_started_at, vit, qi, dex, magnesium, last_observed_at,
                    created_at, gender, soul_seed, display_title, activated_nodes,
                    equipment_slots, inventory, disposition, current_activity, hex_q, hex_r,
                    grid_x, grid_y)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26,$27,$28)
                 ON CONFLICT (id) DO UPDATE SET
                    kind=EXCLUDED.kind, display_char=EXCLUDED.display_char, x=EXCLUDED.x, y=EXCLUDED.y,
                    move_state=EXCLUDED.move_state, target_x=EXCLUDED.target_x, target_y=EXCLUDED.target_y,
                    walk_or_run=EXCLUDED.walk_or_run, move_started_at=EXCLUDED.move_started_at,
                    vit=EXCLUDED.vit, qi=EXCLUDED.qi, dex=EXCLUDED.dex, magnesium=EXCLUDED.magnesium,
                    last_observed_at=EXCLUDED.last_observed_at, created_at=EXCLUDED.created_at,
                    gender=EXCLUDED.gender, soul_seed=EXCLUDED.soul_seed, display_title=EXCLUDED.display_title,
                    activated_nodes=EXCLUDED.activated_nodes, equipment_slots=EXCLUDED.equipment_slots,
                    inventory=EXCLUDED.inventory, disposition=EXCLUDED.disposition,
                    current_activity=EXCLUDED.current_activity, hex_q=EXCLUDED.hex_q, hex_r=EXCLUDED.hex_r,
                    grid_x=EXCLUDED.grid_x, grid_y=EXCLUDED.grid_y",
                &[&e.id, &e.kind, &e.display_char, &e.x, &e.y, &e.move_state,
                  &e.target_x, &e.target_y, &e.walk_or_run, &e.move_started_at,
                  &e.vit, &e.qi, &e.dex, &e.magnesium, &e.last_observed_at,
                  &e.created_at, &e.gender, &e.soul_seed, &e.display_title,
                  &e.activated_nodes, &e.equipment_slots, &e.inventory, &e.disposition,
                  &e.current_activity, &e.hex_q, &e.hex_r, &e.grid_x, &e.grid_y],
            )?;
        }

        // ── entity_rooms ──
        tracing::info!("[store] Syncing {} entity_rooms to PostgreSQL...", self.entity_rooms.len());
        for (eid, rid) in &self.entity_rooms {
            trans.execute(
                "INSERT INTO entity_rooms (entity_id, room_id) VALUES ($1, $2)
                 ON CONFLICT (entity_id) DO UPDATE SET room_id = EXCLUDED.room_id",
                &[eid, rid],
            )?;
        }

        // ── 場所 ──
        tracing::info!("[store] Syncing {} venues to PostgreSQL...", self.venues.len());
        for v in self.venues.values() {
            let room_ids_json = serde_json::to_string(&v.room_ids).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO venues (id, name, room_ids, max_staff) VALUES ($1, $2, $3, $4)
                 ON CONFLICT (id) DO UPDATE SET
                    name=EXCLUDED.name, room_ids=EXCLUDED.room_ids, max_staff=EXCLUDED.max_staff",
                &[&v.id, &v.name, &room_ids_json, &v.max_staff],
            )?;
        }

        // ── 指派 ──
        let asgn_count: usize = self.assignments.values().map(|v| v.len()).sum();
        tracing::info!("[store] Syncing {} assignments to PostgreSQL...", asgn_count);
        // 全量替換：先清後寫
        trans.execute("DELETE FROM assignments", &[])?;
        for list in self.assignments.values() {
            for a in list {
                trans.execute(
                    "INSERT INTO assignments (entity_id, occupation_id, venue_id, assigned_by)
                     VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
                    &[&a.entity_id, &a.occupation_id, &a.venue_id, &a.assigned_by],
                )?;
            }
        }

        // ── 排班 ──
        tracing::info!("[store] Syncing {} schedules to PostgreSQL...", self.schedules.len());
        for sc in self.schedules.values() {
            trans.execute(
                "INSERT INTO schedules (entity_id, work_room, rest_room, shift_start, shift_end)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT (entity_id) DO UPDATE SET
                    work_room=EXCLUDED.work_room, rest_room=EXCLUDED.rest_room,
                    shift_start=EXCLUDED.shift_start, shift_end=EXCLUDED.shift_end",
                &[&sc.entity_id, &sc.work_room, &sc.rest_room, &sc.shift_start, &sc.shift_end],
            )?;
        }

        // ── 物品定義 ──
        tracing::info!("[store] Syncing {} items to PostgreSQL...", self.items.len());
        for it in self.items.values() {
            trans.execute(
                "INSERT INTO items (id, name, slot, item_type, weight, stackable, denomination, description, vit_bonus, dex_bonus, atk_bonus)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
                 ON CONFLICT (id) DO UPDATE SET
                    name=EXCLUDED.name, slot=EXCLUDED.slot, item_type=EXCLUDED.item_type,
                    weight=EXCLUDED.weight, stackable=EXCLUDED.stackable,
                    denomination=EXCLUDED.denomination, description=EXCLUDED.description,
                    vit_bonus=EXCLUDED.vit_bonus, dex_bonus=EXCLUDED.dex_bonus, atk_bonus=EXCLUDED.atk_bonus",
                &[&it.id, &it.name, &it.slot, &it.item_type, &it.weight, &it.stackable, &it.denomination, &it.description, &it.vit_bonus, &it.dex_bonus, &it.atk_bonus],
            )?;
        }

        // ── NPC threads ──
        tracing::info!("[store] Syncing {} npc_threads to PostgreSQL...", self.npc_threads.len());
        for t in self.npc_threads.values() {
            let anchors_json = serde_json::to_string(&t.anchors).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO npc_threads (thread_key, topic_type, phase, anchors, turn_count, cooldown_until, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (thread_key) DO UPDATE SET
                    topic_type=EXCLUDED.topic_type, phase=EXCLUDED.phase, anchors=EXCLUDED.anchors,
                    turn_count=EXCLUDED.turn_count, cooldown_until=EXCLUDED.cooldown_until, updated_at=EXCLUDED.updated_at",
                &[&t.thread_key, &t.topic_type, &t.phase, &anchors_json, &t.turn_count, &t.cooldown_until, &t.updated_at],
            )?;
        }

        // ── NPC dyads ──
        tracing::info!("[store] Syncing {} npc_dyads to PostgreSQL...", self.npc_dyads.len());
        for d in self.npc_dyads.values() {
            let dkey = dyad_key(&d.a_id, &d.b_id);
            let tags_json = serde_json::to_string(&d.tags).unwrap_or_else(|_| "[]".to_string());
            trans.execute(
                "INSERT INTO npc_dyads (dyad_key, a_id, b_id, familiarity, sentiment, tags, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dyad_key) DO UPDATE SET
                    familiarity=EXCLUDED.familiarity, sentiment=EXCLUDED.sentiment,
                    tags=EXCLUDED.tags, updated_at=EXCLUDED.updated_at",
                &[&dkey, &d.a_id, &d.b_id, &d.familiarity, &d.sentiment, &tags_json, &d.updated_at],
            )?;
        }

        // ── NPC rumors ──
        tracing::info!("[store] Syncing {} npc_rumors to PostgreSQL...", self.npc_rumors.len());
        for r in self.npc_rumors.values() {
            trans.execute(
                "INSERT INTO npc_rumors (id, text, room_id, zone, source, source_score, weight, mention_count,
                    last_used_at, blocked_until, penalty_count, last_penalty_at,
                    last_penalty_reason, updated_at, expires_at)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)
                 ON CONFLICT(id) DO UPDATE SET
                    text=EXCLUDED.text, room_id=EXCLUDED.room_id, zone=EXCLUDED.zone, source=EXCLUDED.source,
                    source_score=EXCLUDED.source_score, weight=EXCLUDED.weight, mention_count=EXCLUDED.mention_count,
                    last_used_at=EXCLUDED.last_used_at, blocked_until=EXCLUDED.blocked_until,
                    penalty_count=EXCLUDED.penalty_count, last_penalty_at=EXCLUDED.last_penalty_at,
                    last_penalty_reason=EXCLUDED.last_penalty_reason, updated_at=EXCLUDED.updated_at,
                    expires_at=EXCLUDED.expires_at",
                &[&r.id, &r.text, &r.room_id, &r.zone, &r.source, &r.source_score, &r.weight, &r.mention_count,
                  &r.last_used_at, &r.blocked_until, &r.penalty_count, &r.last_penalty_at,
                  &r.last_penalty_reason, &r.updated_at, &r.expires_at],
            )?;
        }

        trans.commit()?;
        tracing::info!("[store] PostgreSQL full sync completed.");
        Ok(())
    }

    /// 從 PostgreSQL 載入實體到記憶體快取。如果 DB 有資料則回傳 true。
    pub fn load_entities_from_pg(&mut self) -> bool {
        let Some(pool) = &self.db_pool else { return false };
        let Ok(mut conn) = pool.get() else { return false };
        let Ok(rows) = conn.query("SELECT COUNT(*) FROM entities", &[]) else { return false };
        let count: i64 = rows[0].get(0);
        if count == 0 { return false; }

        let Ok(rows) = conn.query(
            "SELECT id, kind, display_char, x, y, move_state, target_x, target_y,
                    walk_or_run, move_started_at, vit, qi, dex, magnesium, last_observed_at,
                    created_at, gender, soul_seed, display_title, activated_nodes,
                    equipment_slots, inventory, disposition, current_activity, hex_q, hex_r,
                    grid_x, grid_y
             FROM entities", &[]
        ) else { return false };

        self.entities.clear();
        for row in &rows {
            let e = Entity {
                id: row.get(0),
                kind: row.get(1),
                display_char: row.get(2),
                x: row.get(3),
                y: row.get(4),
                move_state: row.get(5),
                target_x: row.get(6),
                target_y: row.get(7),
                walk_or_run: row.get(8),
                move_started_at: row.get(9),
                vit: row.get(10),
                qi: row.get(11),
                dex: row.get(12),
                magnesium: row.get(13),
                last_observed_at: row.get(14),
                created_at: row.get(15),
                gender: row.get(16),
                soul_seed: row.get(17),
                display_title: row.get(18),
                activated_nodes: row.get(19),
                equipment_slots: row.get(20),
                inventory: row.get(21),
                disposition: row.get(22),
                current_activity: row.get(23),
                hex_q: row.get(24),
                hex_r: row.get(25),
                grid_x: row.get(26),
                grid_y: row.get(27),
            };
            self.entities.insert(e.id.clone(), e);
        }
        tracing::info!("[store] Loaded {} entities from PostgreSQL", self.entities.len());

        // entity_rooms
        if let Ok(rows) = conn.query("SELECT entity_id, room_id FROM entity_rooms", &[]) {
            if !rows.is_empty() {
                self.entity_rooms.clear();
                for row in &rows {
                    let eid: String = row.get(0);
                    let rid: String = row.get(1);
                    self.entity_rooms.insert(eid, rid);
                }
                tracing::info!("[store] Loaded {} entity_rooms from PostgreSQL", self.entity_rooms.len());
            }
        }

        // assignments
        if let Ok(rows) = conn.query("SELECT entity_id, occupation_id, venue_id, assigned_by FROM assignments", &[]) {
            if !rows.is_empty() {
                self.assignments.clear();
                for row in &rows {
                    let a = Assignment {
                        entity_id: row.get(0),
                        occupation_id: row.get(1),
                        venue_id: row.get(2),
                        assigned_by: row.get(3),
                    };
                    self.assignments.entry(a.entity_id.clone()).or_default().push(a);
                }
                let count: usize = self.assignments.values().map(|v| v.len()).sum();
                tracing::info!("[store] Loaded {} assignments from PostgreSQL", count);
            }
        }

        // schedules
        if let Ok(rows) = conn.query("SELECT entity_id, work_room, rest_room, shift_start, shift_end FROM schedules", &[]) {
            if !rows.is_empty() {
                self.schedules.clear();
                for row in &rows {
                    let sc = Schedule {
                        entity_id: row.get(0),
                        work_room: row.get(1),
                        rest_room: row.get(2),
                        shift_start: row.get(3),
                        shift_end: row.get(4),
                    };
                    self.schedules.insert(sc.entity_id.clone(), sc);
                }
                tracing::info!("[store] Loaded {} schedules from PostgreSQL", self.schedules.len());
            }
        }

        // venues
        if let Ok(rows) = conn.query("SELECT id, name, room_ids, max_staff FROM venues", &[]) {
            if !rows.is_empty() {
                self.venues.clear();
                for row in &rows {
                    let room_ids_raw: String = row.get(2);
                    let room_ids: Vec<String> = serde_json::from_str(&room_ids_raw).unwrap_or_default();
                    let v = Venue {
                        id: row.get(0),
                        name: row.get(1),
                        room_ids,
                        max_staff: row.get(3),
                    };
                    self.venues.insert(v.id.clone(), v);
                }
                tracing::info!("[store] Loaded {} venues from PostgreSQL", self.venues.len());
            }
        }

        // items
        if let Ok(rows) = conn.query("SELECT id, name, slot, item_type, weight, stackable, denomination, description, vit_bonus, dex_bonus, atk_bonus FROM items", &[]) {
            if !rows.is_empty() {
                self.items.clear();
                for row in &rows {
                    let it = Item {
                        id: row.get(0),
                        name: row.get(1),
                        slot: row.get(2),
                        item_type: row.get(3),
                        weight: row.get(4),
                        stackable: row.get(5),
                        denomination: row.get(6),
                        description: row.get(7),
                        vit_bonus: row.get(8),
                        dex_bonus: row.get(9),
                        atk_bonus: row.get(10),
                    };
                    self.items.insert(it.id.clone(), it);
                }
                tracing::info!("[store] Loaded {} items from PostgreSQL", self.items.len());
            }
        }

        // npc_threads
        if let Ok(rows) = conn.query(
            "SELECT thread_key, topic_type, phase, anchors, turn_count, cooldown_until, updated_at FROM npc_threads", &[]
        ) {
            if !rows.is_empty() {
                self.npc_threads.clear();
                for row in &rows {
                    let anchors_raw: String = row.get(3);
                    let anchors: Vec<String> = serde_json::from_str(&anchors_raw).unwrap_or_default();
                    let t = NpcThread {
                        thread_key: row.get(0),
                        topic_type: row.get(1),
                        phase: row.get(2),
                        anchors,
                        turn_count: row.get(4),
                        cooldown_until: row.get(5),
                        updated_at: row.get(6),
                    };
                    self.npc_threads.insert(t.thread_key.clone(), t);
                }
                tracing::info!("[store] Loaded {} npc_threads from PostgreSQL", self.npc_threads.len());
            }
        }

        // npc_dyads
        if let Ok(rows) = conn.query(
            "SELECT dyad_key, a_id, b_id, familiarity, sentiment, tags, updated_at FROM npc_dyads", &[]
        ) {
            if !rows.is_empty() {
                self.npc_dyads.clear();
                for row in &rows {
                    let tags_raw: String = row.get(5);
                    let tags: Vec<String> = serde_json::from_str(&tags_raw).unwrap_or_default();
                    let d = NpcDyad {
                        a_id: row.get(1),
                        b_id: row.get(2),
                        familiarity: row.get(3),
                        sentiment: row.get(4),
                        tags,
                        updated_at: row.get(6),
                    };
                    let key: String = row.get(0);
                    self.npc_dyads.insert(key, d);
                }
                tracing::info!("[store] Loaded {} npc_dyads from PostgreSQL", self.npc_dyads.len());
            }
        }

        true
    }
}

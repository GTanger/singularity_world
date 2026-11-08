impl Store {
    // ── entity_rooms ──

    pub fn get_entity_room(&self, entity_id: &str) -> String {
        if let Some(e) = self.entities.get(entity_id)
            && let Some((x, y)) = self.effective_entity_grid(e)
        {
            return crate::grid::grid_room_id_from_coord(x, y);
        }
        if let Some(place_id) = self.get_entity_place_id(entity_id)
            && let Some(place) = self.get_place(&place_id)
        {
            return place.compatibility_room_id();
        }
        self.entity_rooms
            .get(entity_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_entity_room(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
            let place_id = crate::world::place::legacy_place_id(coord);
            if self.get_place(&place_id).is_some() {
                return self.set_entity_place(entity_id, &place_id);
            }
        }
        self.entity_rooms
            .insert(entity_id.to_string(), room_id.to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityRoom {
            entity_id: entity_id.to_string(),
            room_id: room_id.to_string(),
        });
        Ok(())
    }
    /// 直接更新實體所在房間或虛擬格（如 `grid:x:y`），不更動場所圖狀態。
    pub fn move_entity(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.entity_rooms
            .insert(entity_id.to_string(), room_id.to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityRoom {
            entity_id: entity_id.to_string(),
            room_id: room_id.to_string(),
        });
        Ok(())
    }

    /// 寫入或更新完整實體資料並同步快取與非同步持久化。
    pub fn upsert_entity_full(&mut self, e: Entity) -> anyhow::Result<()> {
        self.put_entity(e)
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

    /// 讀取 NPC 實體的即時表面行為（若在文曜定居居民狀態中則取用記憶體 LIVE_NPC_GRID_STATES，否則回傳 e.current_activity）。
    pub fn effective_npc_activity(&self, e: &Entity) -> String {
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(&e.id) {
            return state.current_activity();
        }
        if !e.current_activity.is_empty() {
            return e.current_activity.clone();
        }
        String::new()
    }

    pub fn ensure_live_npcs_seeded(&self) {
        static LIVE_SEEDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LIVE_SEEDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            self.ensure_household_assignments();
            for e in self.entities.values() {
                if e.kind == "npc" && e.vit > 0 && e.grid_x.is_none() {
                    let _ = self.effective_entity_grid(e);
                }
            }
        }
    }

    pub fn entity_ids_in_room(&self, room_id: &str) -> Vec<String> {
        if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
            self.ensure_live_npcs_seeded();
            let mut ids = Vec::new();
            let mut seen = std::collections::HashSet::new();

            let live_states = LIVE_NPC_GRID_STATES.read();
            // 1. LIVE_NPC_GRID_STATES whose state.current == (coord.x, coord.y)
            for (id, state) in live_states.iter() {
                if state.current == (coord.x, coord.y) {
                    if seen.insert(id.clone()) {
                        ids.push(id.clone());
                    }
                }
            }

            // 2. Canonical NPCs directly from self.entities at (coord.x, coord.y) not tracked in LIVE_NPC_GRID_STATES
            for (id, e) in &self.entities {
                if e.kind == "npc"
                    && e.vit > 0
                    && e.grid_x == Some(coord.x)
                    && e.grid_y == Some(coord.y)
                {
                    if seen.insert(id.clone()) {
                        ids.push(id.clone());
                    }
                }
            }

            // 3. Players or non-grid entities in self.entity_rooms at room_id
            for (eid, rid) in &self.entity_rooms {
                if rid == room_id {
                    let is_tracked_live_npc = live_states.contains_key(eid);
                    let is_tracked_canon_npc = self
                        .entities
                        .get(eid)
                        .map(|e| e.kind == "npc" && e.grid_x.is_some() && e.grid_y.is_some())
                        .unwrap_or(false);
                    if !is_tracked_live_npc && !is_tracked_canon_npc {
                        if seen.insert(eid.clone()) {
                            ids.push(eid.clone());
                        }
                    }
                }
            }

            return ids;
        }
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
        let mut ids: Vec<String> = self.entity_rooms
            .keys()
            .filter(|eid| {
                self.entities
                    .get(eid.as_str())
                    .is_some_and(|e| e.kind == "npc" && e.vit > 0)
            })
            .cloned()
            .collect();
        let live = LIVE_NPC_GRID_STATES.read();
        for id in live.keys() {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
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
        if is_deprecated_npc_id(id) {
            return None;
        }
        let mut e = self.entities.get(id).cloned()?;
        if e.kind == "npc" && (e.grid_x.is_none() || e.grid_y.is_none()) {
            let _ = self.effective_entity_grid(&e);
        }
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(id) {
            let act = state.current_activity();
            if !act.is_empty() {
                e.current_activity = act;
            }
            if e.kind == "npc" {
                e.grid_x = Some(state.current.0);
                e.grid_y = Some(state.current.1);
            }
        }
        Some(e)
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

    /// PostgreSQL 已先提交的新人物只鏡射進快取，不再排一筆可能覆蓋後續位置的舊快照。
    pub fn mirror_new_entity_from_authority(&mut self, mut entity: Entity) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.entities.contains_key(&entity.id),
            "實體已存在：{}",
            entity.id
        );
        if entity.activated_nodes.is_empty() {
            entity.activated_nodes = r#"["N000"]"#.to_string();
        }
        if entity.inventory.is_empty() {
            entity.inventory = "[]".to_string();
        }
        self.entities.insert(entity.id.clone(), entity);
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

    pub fn append_thought(
        &mut self,
        entity_id: &str,
        source: crate::db::ThoughtSource,
        flavor: String,
        now_unix: i64,
    ) {
        let _ = self.update_entity(entity_id, |ent| {
            crate::db::add_thought_to_entity_with_delta(ent, source, 8, now_unix, flavor);
            crate::db::clean_and_recalc_entity_disposition(ent, now_unix);
        });
    }

    /// PostgreSQL 已在同一交易完成背包變動時，只校正讀取快取，不再排第二次寫入。
    pub fn mirror_inventory_from_authority(
        &mut self,
        entity_id: &str,
        inventory: &str,
    ) -> anyhow::Result<()> {
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.inventory = inventory.to_string();
        Ok(())
    }

    /// PostgreSQL 已完成共用交易後，校正快取並以同一快照壓在較早排隊的寫入之後。
    pub fn mirror_economy_from_authority(
        &mut self,
        entity_id: &str,
        magnesium: i32,
        inventory: &str,
        current_activity: &str,
    ) -> anyhow::Result<()> {
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.magnesium = magnesium;
        entity.inventory = inventory.to_string();
        entity.current_activity = current_activity.to_string();
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityEconomySnapshot {
            entity_id: entity_id.to_string(),
            magnesium,
            inventory: inventory.to_string(),
            current_activity: current_activity.to_string(),
        });
        Ok(())
    }

    /// 舊座標入口；立即轉為正式 place_id，座標只作相容鏡像。
    pub fn set_entity_grid(&mut self, entity_id: &str, x: i32, y: i32) -> anyhow::Result<()> {
        let mut states = LIVE_NPC_GRID_STATES.write();
        if let Some(state) = states.get_mut(entity_id) {
            state.current = (x, y);
        } else {
            let slot = wenyao_household_slot_for(entity_id);
            let hour = current_game_hour();
            let (dest, dest_name, purpose, stay_activity, stay_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, slot.home_name);
            states.insert(
                entity_id.to_string(),
                NpcGridState {
                    current: (x, y),
                    dest,
                    dest_name,
                    origin_name: slot.home_name.to_string(),
                    origin_purpose: "居家整頓備工".to_string(),
                    stay_activity,
                    purpose,
                    stay_ticks_remaining: stay_ticks,
                    work_errand_toggle: false,
                    consecutive_work_streaks: 0,
                },
            );
        }
        drop(states);
        self.set_entity_grid_compat(entity_id, x, y)
    }

    /// 計算實體在正方格地圖上的有效座標：
    /// - 若已有明確 `(grid_x, grid_y)` 則直接採用；
    /// - 若為存活 NPC，先查執行期連續座標表；
    /// - 若尚未初始化，依所屬區域決定性選取初始可行走方格，並登錄執行期座標表。
    /// 確保全島所有戶口與山海獵場／古道遠征槽位（`total_household_slots_len()`）皆有對應的真實持久化 `Entity` 實體：
    /// 若資料庫既有無固定座標存活 NPC 數不足槽位總數，即自動補齊具備真實 `soul_seed`、`magnesium`、`thoughts` 與職銜之持久化 `Entity`。
    pub fn ensure_all_household_entities_seeded(&mut self) {
        let needed = total_household_slots_len();
        let existing_eligible = self
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.vit > 0
                    && e.grid_x.is_none()
                    && !is_deprecated_npc_id(&e.id)
            })
            .count();
        if existing_eligible < needed {
            let now_ts = crate::game::now_unix();
            let mut created = 0usize;
            let mut seq = 0usize;
            while existing_eligible + created < needed {
                seq += 1;
                let id = format!("npc_exp_{seq:04}");
                if self.entities.contains_key(&id) {
                    continue;
                }
                let slot_idx = existing_eligible + created;
                let slot = household_slot_at(slot_idx);
                let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (slot_idx as u64);
                for b in id.as_bytes() {
                    h ^= u64::from(*b);
                    h = h.wrapping_mul(0x0100_0000_01b3);
                }
                let gender = if slot_idx % 3 == 0 { "F" } else { "M" };
                let person_name = deterministic_wenyao_full_name_with_gender(&id, Some(gender));
                let display_char = person_name
                    .chars()
                    .next()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "人".to_string());
                let init_mg = (h % 38) as i32;
                let mut entity = Entity {
                    id: id.clone(),
                    kind: "npc".to_string(),
                    display_char,
                    x: 0,
                    y: 0,
                    move_state: "idle".to_string(),
                    target_x: None,
                    target_y: None,
                    walk_or_run: String::new(),
                    move_started_at: None,
                    vit: 100,
                    qi: 100,
                    dex: 58,
                    magnesium: init_mg,
                    last_observed_at: None,
                    created_at: now_ts,
                    gender: gender.to_string(),
                    soul_seed: Some(h as i64),
                    display_title: person_name,
                    activated_nodes: r#"["N000"]"#.to_string(),
                    equipment_slots: "{}".to_string(),
                    inventory: "[]".to_string(),
                    disposition: 8,
                    current_activity: slot.work_stay_activity.to_string(),
                    place_id: None,
                    grid_x: None,
                    grid_y: None,
                    thoughts: Vec::new(),
                };
                crate::db::add_thought_to_entity_with_delta(
                    &mut entity,
                    crate::db::ThoughtSource::WorkDone,
                    6,
                    now_ts,
                    format!(
                        "盤算著今日在【{}】「{}」的進度，收妥隨身器物準備推進",
                        slot.work_name, slot.work_purpose
                    ),
                );
                crate::db::clean_and_recalc_entity_disposition(&mut entity, now_ts);
                let ent_sync = entity.clone();
                self.entities.insert(id, entity);
                crate::pg::entity::upsert_async(ent_sync);
                created += 1;
            }
            HOUSEHOLD_MEMBER_ASSIGNMENTS.write().clear();
        }
        self.ensure_household_assignments();
        let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
        for e in self.entities.values() {
            if e.kind == "npc" && e.vit > 0 && e.grid_x.is_none() {
                let _ = self.effective_entity_grid_with_grid(e, grid_opt.as_ref());
            }
        }
        if let Some(ref grid) = grid_opt {
            self.sync_and_advance_hunting_parties(grid);
        }
    }

    pub fn ensure_household_assignments(&self) {
        let total_slots = total_household_slots_len();
        if HOUSEHOLD_MEMBER_ASSIGNMENTS.read().len() >= total_slots && self.place_graph.places().next().is_some() {
            return;
        }
        let slot_gender_compatible = |slot: &WenyaoHouseholdSlot, is_female: bool| -> bool {
            let is_feminine_slot = [
                "繡花圍裙", "繡花小圍裙", "碎花", "髮簪", "木簪", "銀簪", "布裙", "長裙", "百褶裙", "羅裙", "婦人", "姑娘", "婆婆", "娘子",
            ]
            .iter()
            .any(|kw| slot.attire_line.contains(kw) || slot.brief_tag.contains(kw));
            let is_masculine_slot = ["短髭", "絡腮鬍", "長鬚", "鬍鬚", "老漢", "壯漢"]
                .iter()
                .any(|kw| slot.attire_line.contains(kw) || slot.brief_tag.contains(kw));
            if !is_female && is_feminine_slot {
                return false;
            }
            if is_female && is_masculine_slot {
                return false;
            }
            true
        };
        let fallback_settlement_group = |settlement: &str| -> &'static [&'static str] {
            match settlement {
                "霜林口" => &["霜林口", "北嶺腳", "老堰頭", "風舶嶼"],
                "三號坑口" => &["三號坑口", "風櫃尾", "牛埔", "紅土崁"],
                "大圳頭" => &["大圳頭", "紅土崁", "牛埔", "烏石鼻"],
                "水尾寮" => &["水尾寮", "烏石鼻", "下營", "風櫃尾"],
                "下營" => &["下營", "烏石鼻", "風櫃尾", "牛埔"],
                "南哨" => &["南哨", "風舶角", "風舶嶼", "烏石鼻"],
                _ => &[],
            }
        };
        let mut eligible_entities: Vec<(String, &'static str, bool)> = self
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.vit > 0
                    && e.id.starts_with("npc_")
                    && !is_deprecated_npc_id(&e.id)
            })
            .map(|e| {
                let is_female = npc_effective_is_female(&e.id, Some(&e.gender));
                let settlement = e
                    .place_id
                    .as_deref()
                    .and_then(|pid| self.get_place(pid))
                    .map(|pl| {
                        if pl.zone == "wt-c-wenyao" || pl.name.starts_with("文曜") {
                            "文曜"
                        } else {
                            let mapped =
                                crate::pg::living_world::peninsula_market_settlement_name(&pl.name);
                            normalize_household_home_settlement(mapped)
                        }
                    })
                    .unwrap_or_else(|| {
                        if e.x.abs() <= 20 && e.y.abs() <= 20 {
                            "文曜"
                        } else {
                            resident_macro_settlement(&e.id, &e.display_title)
                                .unwrap_or("文曜")
                        }
                    });
                (e.id.clone(), settlement, is_female)
            })
            .collect();
        eligible_entities.sort_by(|a, b| a.0.cmp(&b.0));
        let mut map = HOUSEHOLD_MEMBER_ASSIGNMENTS.write();
        if map.len() < eligible_entities.len().min(total_slots) {
            map.clear();
            let mut used_slots = vec![false; total_slots];
            let mut unassigned_entities = Vec::new();
            for (id, settlement, is_female) in eligible_entities.into_iter().take(total_slots) {
                let exact_gender_match = (0..total_slots).find(|&idx| {
                    let s = household_slot_at(idx);
                    !used_slots[idx]
                        && normalize_household_home_settlement(s.home_name) == settlement
                        && slot_gender_compatible(s, is_female)
                });
                let group = fallback_settlement_group(settlement);
                let group_gender_match = exact_gender_match.or_else(|| {
                    (0..total_slots).find(|&idx| {
                        let s = household_slot_at(idx);
                        let norm = normalize_household_home_settlement(s.home_name);
                        !used_slots[idx]
                            && group.contains(&norm)
                            && slot_gender_compatible(s, is_female)
                    })
                });
                if let Some(slot_idx) = group_gender_match {
                    used_slots[slot_idx] = true;
                    map.insert(id, slot_idx);
                } else {
                    unassigned_entities.push((id, is_female));
                }
            }
            for (id, is_female) in unassigned_entities {
                let slot_idx_opt = (0..total_slots)
                    .find(|&idx| !used_slots[idx] && slot_gender_compatible(household_slot_at(idx), is_female))
                    .or_else(|| (0..total_slots).find(|&idx| !used_slots[idx]));
                if let Some(slot_idx) = slot_idx_opt {
                    used_slots[slot_idx] = true;
                    map.insert(id, slot_idx);
                }
            }
        }
    }

    fn resolve_venue_for_requirement(
        req: crate::npc::causal_core::VenueRequirement,
        slot: &WenyaoHouseholdSlot,
        grid: Option<&crate::grid::SquareGrid>,
        current: (i32, i32),
        is_bladed: bool,
        seed: u64,
    ) -> ((i32, i32), String) {
    let is_wenyao = slot.home_x.abs() <= 20 && slot.home_y.abs() <= 20;
    let settlement = slot.home_name.split('・').next().unwrap_or("文曜");
    let ref_pt = if !is_wenyao && current == (0, 0) {
        (slot.home_x, slot.home_y)
    } else {
        current
    };
    let is_public_cell = |c: &crate::grid::GridCell| {
        !c.is_private_residence() && !c.is_pure_street_or_courtyard()
    };
    match req {
        crate::npc::causal_core::VenueRequirement::HomeOrKitchen => {
            ((slot.home_x, slot.home_y), slot.home_name.to_string())
        }
        crate::npc::causal_core::VenueRequirement::OwnWorkplace => {
            let hour = crate::game::current_game_hour();
            let is_night_duty = slot.work_name.contains("守更")
                || slot.work_name.contains("哨")
                || slot.work_name.contains("鐘樓")
                || slot.work_name.contains("燈塔")
                || slot.brief_tag.contains("守更")
                || slot.brief_tag.contains("夜巡");
            let in_shift = if slot.evening_hour > slot.morning_hour {
                hour >= i32::from(slot.morning_hour) && hour < i32::from(slot.evening_hour)
            } else {
                hour >= i32::from(slot.morning_hour) || hour < i32::from(slot.evening_hour)
            };
            if !in_shift && !is_night_duty {
                return ((slot.home_x, slot.home_y), slot.home_name.to_string());
            }
            if slot.brief_tag.starts_with("入山百工・") {
                let zone_prefix = slot.work_name.split('・').next().unwrap_or(slot.work_name);
                let (pt, actual_zone, sub) =
                    crate::game::pick_zone_artisan_waypoint_with_zone(zone_prefix, seed, current);
                (pt, clean_wilderness_place_label(&format!("{actual_zone}・{sub}")))
            } else if slot.brief_tag.starts_with("古道行旅・") {
                let zone_prefix = slot.work_name.split('・').next().unwrap_or(slot.work_name);
                let (pt, rzone) = crate::game::pick_island_trail_waypoint(
                    zone_prefix,
                    (slot.home_x, slot.home_y),
                    (slot.errand_x, slot.errand_y),
                    seed,
                    current,
                );
                (pt, format!("{rzone}・古道驛路"))
            } else {
                ((slot.work_x, slot.work_y), slot.work_name.to_string())
            }
        }
        crate::npc::causal_core::VenueRequirement::CanteenOrStall => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("canteen")
                                || c.name.contains("食堂")
                                || c.name.contains("飯")
                                || c.name.contains("麵")
                                || c.name.contains("廚"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("三水大食堂", (-1, 2)),
                    "文曜・三水大食堂".to_string(),
                )
            } else {
                (
                    (slot.errand_x, slot.errand_y),
                    format!("{settlement}・街角熱食攤"),
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::TeahouseOrTavern => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("teahouse")
                                || c.has_room_tag("inn")
                                || c.name.contains("茶")
                                || c.name.contains("旅社"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("東橋旅社與山風茶屋", (2, -1)),
                    "文曜・東橋旅社與山風茶屋".to_string(),
                )
            } else {
                (
                    (slot.errand_x, slot.errand_y),
                    format!("{settlement}・路邊茶寮"),
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::WellOrStream => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("well")
                                || c.name.contains('井')
                                || c.name.contains("山泉")
                                || c.name.contains("清泉"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                ((1, 0), "文曜・三水口水岸公井".to_string())
            } else {
                (
                    (slot.home_x, slot.home_y),
                    format!("{settlement}・街角山泉公井"),
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::Bathhouse => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("bathhouse")
                                || c.name.contains("浴")
                                || c.name.contains("湯"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("泉香溫水浴場", (-4, 1)),
                    "文曜・泉香溫水浴場".to_string(),
                )
            } else {
                (
                    (slot.home_x, slot.home_y),
                    format!("{settlement}・溫水石湯屋"),
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::InnOrHome => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("inn")
                                || c.name.contains("旅社")
                                || c.name.contains("歇舍"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("東橋旅社與山風茶屋", (2, -1)),
                    "文曜・東橋旅社與山風茶屋".to_string(),
                )
            } else {
                (
                    (slot.home_x, slot.home_y),
                    if !slot.home_name.is_empty() {
                        slot.home_name.to_string()
                    } else {
                        format!("{settlement}・居所")
                    },
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::Clinic => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 28
                            && is_public_cell(c)
                            && (c.has_room_tag("herb") || c.has_room_tag("medicine"))
                            && (c.name.contains("診")
                                || c.name.contains("醫")
                                || c.name.contains("藥鋪")
                                || c.name.contains("藥局"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("溫氏脈絡診所", (1, -4)),
                    "文曜・溫氏脈絡診所".to_string(),
                )
            } else if slot.home_y < -220 {
                ((0, -320), "南哨・南哨脈絡解毒診所".to_string())
            } else if slot.home_x < -200 {
                ((-263, 158), "風舶角・港務脈絡診所".to_string())
            } else {
                (
                    (slot.errand_x, slot.errand_y),
                    format!("{settlement}・草木藥鋪"),
                )
            }
        }
        crate::npc::causal_core::VenueRequirement::ForgeOrWorkshop => {
            if is_bladed {
                let found = grid.and_then(|g| {
                    g.cells()
                        .filter(|c| {
                            let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                            d <= 28
                                && is_public_cell(c)
                                && (c.has_room_tag("blacksmith")
                                    || c.name.contains("鍛造")
                                    || c.name.contains("鐵器"))
                        })
                        .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
                });
                if let Some(c) = found {
                    ((c.coord.x, c.coord.y), c.name.clone())
                } else if is_wenyao {
                    (
                        resolve_bldg_coord_or_fallback("岑氏第一鍛造工坊", (2, -4)),
                        "文曜・岑氏第一鍛造工坊".to_string(),
                    )
                } else if slot.home_x > 40 {
                    ((95, 95), "半嶺鎮・百工鐵器鍛造坊".to_string())
                } else {
                    ((slot.work_x, slot.work_y), slot.work_name.to_string())
                }
            } else {
                ((slot.work_x, slot.work_y), slot.work_name.to_string())
            }
        }
        crate::npc::causal_core::VenueRequirement::DockOrStation => {
            if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("西門車馬聯運總站", (-5, 0)),
                    "文曜・西門車馬聯運總站".to_string(),
                )
            } else {
                ((slot.errand_x, slot.errand_y), slot.errand_name.to_string())
            }
        }
        crate::npc::causal_core::VenueRequirement::Market => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && is_public_cell(c)
                            && (c.has_room_tag("market")
                                || c.has_room_tag("shop")
                                || c.name.contains("市集")
                                || c.name.contains("商行")
                                || c.name.contains("市")
                                || c.name.contains("鋪"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                (
                    resolve_bldg_coord_or_fallback("三水總商行", (-1, 1)),
                    "文曜・三水總商行".to_string(),
                )
            } else {
                ((slot.errand_x, slot.errand_y), slot.errand_name.to_string())
            }
        }
        crate::npc::causal_core::VenueRequirement::WildOrOutskirts => {
            let zone = if slot.home_y > 140 {
                "清泉溪谷"
            } else if slot.home_x > 50 {
                "西嶺礦谷"
            } else if slot.home_x < -220 {
                "驚濤岩岸"
            } else if slot.home_y < -150 {
                "長風草甸"
            } else {
                "西麓林道"
            };
            let (c, actual_zone, sub) =
                crate::game::pick_zone_artisan_waypoint_with_zone(zone, seed, current);
            (c, clean_wilderness_place_label(&format!("{actual_zone}・{sub}")))
        }
        crate::npc::causal_core::VenueRequirement::BridgeOrPlaza => {
            let found = grid.and_then(|g| {
                g.cells()
                    .filter(|c| {
                        let d = (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs();
                        d <= 24
                            && (c.has_room_tag("bridge")
                                || c.has_room_tag("plaza")
                                || c.name.contains('橋')
                                || c.name.contains("廣場")
                                || c.name.contains("石坪"))
                    })
                    .min_by_key(|c| (c.coord.x - ref_pt.0).abs() + (c.coord.y - ref_pt.1).abs())
            });
            if let Some(c) = found {
                ((c.coord.x, c.coord.y), c.name.clone())
            } else if is_wenyao {
                ((0, 0), "文曜・三水匯流大石橋".to_string())
            } else {
                (
                    (slot.errand_x, slot.errand_y),
                    format!("{settlement}・街心石坪"),
                )
            }
        }
    }
}

    pub fn effective_entity_grid(&self, e: &Entity) -> Option<(i32, i32)> {
        self.effective_entity_grid_with_grid(e, None)
    }

    pub fn effective_entity_grid_with_grid(
        &self,
        e: &Entity,
        grid: Option<&crate::grid::SquareGrid>,
    ) -> Option<(i32, i32)> {
        if is_deprecated_npc_id(&e.id) {
            return None;
        }
        if let (Some(x), Some(y)) = (e.grid_x, e.grid_y) {
            return Some((x, y));
        }
        if e.kind != "npc" || e.vit <= 0 {
            return None;
        }
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(&e.id) {
            return Some(state.current);
        }

        self.ensure_household_assignments();
        let slot_idx = *HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(&e.id)?;
        let slot = household_slot_at(slot_idx);

        let hour = current_game_hour();
        let is_expedition_slot = slot.brief_tag.starts_with("合社獵團・")
            || slot.brief_tag.starts_with("入山百工・")
            || slot.brief_tag.starts_with("古道行旅・");
        let init_toggle = if is_expedition_slot {
            slot_idx % 6 == 0
        } else {
            slot_idx % 2 == 1
        };
        let is_daytime = (5..=19).contains(&hour);
        let sched_hour = if is_expedition_slot && is_daytime { 10 } else { hour };
        let (dest, _dest_name, _purpose, _stay_activity, stay_ticks) =
            resolve_wenyao_npc_schedule(slot, sched_hour, init_toggle, slot.home_name);
        let mut init_coord = dest;
        let zone_prefix = slot.work_name.split('・').next().unwrap_or(slot.work_name);
        if is_daytime && slot.brief_tag.starts_with("入山百工・") && !init_toggle {
            let (start_pt, _start_sub) = crate::game::pick_zone_artisan_waypoint(
                zone_prefix,
                (slot_idx as u64).wrapping_mul(131),
                (slot.work_x, slot.work_y),
            );
            init_coord = start_pt;
        } else if is_daytime && slot.brief_tag.starts_with("古道行旅・") && !init_toggle {
            let (start_pt, _start_zone) = crate::game::pick_island_trail_waypoint(
                zone_prefix,
                (slot.home_x, slot.home_y),
                (slot.errand_x, slot.errand_y),
                (slot_idx as u64).wrapping_mul(97),
                (slot.home_x, slot.home_y),
            );
            init_coord = start_pt;
        } else if is_daytime && (slot.home_x.abs() > 24 || slot.home_y.abs() > 24) && !is_expedition_slot && (slot_idx % 4 == 0) {
            let (near_zone, _, _) =
                crate::grid::r#gen::classify_wilderness_cell(slot.home_x, slot.home_y);
            let (out_pt, _out_sub) = if slot_idx % 8 == 0 {
                crate::game::pick_island_trail_waypoint(
                    near_zone,
                    (slot.home_x, slot.home_y),
                    (slot.errand_x, slot.errand_y),
                    (slot_idx as u64).wrapping_mul(113),
                    (slot.home_x, slot.home_y),
                )
            } else {
                crate::game::pick_zone_artisan_waypoint(
                    near_zone,
                    (slot_idx as u64).wrapping_mul(113),
                    (slot.home_x, slot.home_y),
                )
            };
            init_coord = out_pt;
        }
        let origin_name = slot.home_name.to_string();
        let stay_ticks_remaining = if is_expedition_slot {
            1 + ((slot_idx % 3) as u8)
        } else {
            (stay_ticks / 2).max(3)
        };
        let settlement_name = normalize_household_home_settlement(slot.home_name);
        let actual_scarce_name =
            crate::game::object_ecology::scarcest_item_for_settlement(settlement_name)
                .map(|(_id, nm, _cur, _max, _pr)| nm);
        let res_seed = crate::npc::causal_core::deterministic_resident_seed(&e.id, e.soul_seed);
        let clean_title = crate::npc::causal_core::strip_trailing_person_name_from_role(&e.display_title);
        let role_combined = format!("{} {}", slot.brief_tag, clean_title);
        let (causal, chosen) = crate::npc::causal_core::plan_next_causal_action(
            &e.id,
            &role_combined,
            settlement_name,
            res_seed,
            "",
            1,
            false,
            false,
            e.magnesium,
            hour.max(0) as u32,
            actual_scarce_name.as_deref(),
        );
        let (dest_coord, dest_name) = Self::resolve_venue_for_requirement(
            chosen.venue_req,
            slot,
            grid,
            init_coord,
            false,
            res_seed,
        );
        let stay_activity = crate::npc::causal_core::format_predefined_stay_activity(
            &dest_name,
            &causal,
            e.magnesium,
        );
        LIVE_NPC_GRID_STATES.write().insert(
            e.id.clone(),
            NpcGridState {
                current: init_coord,
                dest: dest_coord,
                dest_name,
                origin_name,
                origin_purpose: "居家整頓備工".to_string(),
                stay_activity,
                purpose: chosen.action_title.to_string(),
                stay_ticks_remaining,
                work_errand_toggle: init_toggle,
                consecutive_work_streaks: (slot_idx % 2) as u8,
            },
        );
        Some(init_coord)
    }
}

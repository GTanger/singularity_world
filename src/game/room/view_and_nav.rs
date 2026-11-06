/// 當前房間視野。
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub exits: Vec<Exit>,
    pub entities: Vec<Character>,
    pub objects: Vec<RoomObject>,
}

/// 過濾視野中不合適顯示的實體：
/// 1. 氣血歸零（無生命）
/// 2. 測試殘留（含底線命名或測試標記）
/// 3. 超過 180 秒無活動觀測的離線殭屍角色
pub fn filter_room_entities(entities: Vec<Character>) -> Vec<Character> {
    let now = crate::game::now_unix();
    entities
        .into_iter()
        .filter(|e| {
            if e.vit <= 0 {
                return false;
            }
            if e.display_char.contains('_') {
                return false;
            }
            match e.kind {
                crate::entity::EntityKind::Player => {
                    if e.id.contains('_') {
                        return false;
                    }
                    let last_active = e.last_observed_at.unwrap_or(e.created_at);
                    if last_active > 0 && now.saturating_sub(last_active) > 180 {
                        return false;
                    }
                }
                crate::entity::EntityKind::Npc => {
                    let lower_id = e.id.to_ascii_lowercase();
                    if lower_id.starts_with("test")
                        || lower_id.contains("test_")
                        || lower_id.contains("_test")
                        || lower_id.starts_with("stub")
                        || lower_id.starts_with("dummy")
                    {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}

/// 依房間 id 載入視野；`game_hour` 0–23 供 NPC 職稱，`-1` 表示不套用收工休歇規則。
pub fn get_room_view(room_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(room) = db::get_room(room_id)? else {
        return Ok(None);
    };
    let exits = db::get_exits_for_room(room_id)?;
    let entities = filter_room_entities(db::get_entities_in_room(room_id, game_hour)?);
    let objects = db::get_objects_in_room(room_id)?;
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

/// 以正式 `world_places` / `world_routes` 組裝玩家當前所見。
pub fn get_place_room_view(place_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(store) = store::get_store() else {
        return Ok(None);
    };
    let (place, exits) = {
        let guard = store.read();
        let Some(place) = guard.get_place(place_id).cloned() else {
            return Ok(None);
        };
        if !place.traversable || place.status != "active" {
            return Ok(None);
        }
        let graph = guard.place_graph_snapshot();
        let exits = graph
            .routes_from(place_id)
            .iter()
            .filter(|route| route.status == "active")
            .filter_map(|route| {
                let target = graph.place(&route.to_place_id)?;
                Some(Exit {
                    direction: route.direction.clone(),
                    to_room_id: target.id.clone(),
                    to_room_name: target.name.clone(),
                    route_id: route.id.clone(),
                    mode: route.mode.clone(),
                    travel_cost: route.travel_cost,
                })
            })
            .collect::<Vec<_>>();
        (place, exits)
    };
    let mut objects = Vec::new();
    if let Some(pool) = crate::pg::pool()
        && let Some(source) = crate::pg::world_resource::get_at_place(&pool, place_id)?
    {
        let sockets = vec!["Look".to_string(), "Gather".to_string()];
        let state = if source.current_stock > 0 {
            format!("目前仍可取得約{}份。", source.current_stock)
        } else {
            "眼前能取用的部分已空，必須等它慢慢恢復。".to_string()
        };
        objects.push(RoomObject {
            id: format!("resource:{}:{}", source.place_id, source.item_id),
            name: format!("可取用的{}", source.name),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([(
                "Look".to_string(),
                format!("{}；{}", source.origin_label, state),
            )]),
            move_to_room_id: String::new(),
        });
    }
    let entities = filter_room_entities(db::get_entities_in_room(place_id, game_hour)?);
    Ok(Some(RoomView {
        room: Room {
            id: place.id,
            name: place.name,
            tags: place.tags,
            zone: place.zone,
            description: place.description,
            objects: objects.clone(),
        },
        exits,
        entities,
        objects,
    }))
}

/// 若實體尚無房間則設為預設房間，並回傳房間 id。
pub fn ensure_entity_in_room(entity_id: &str, default_room_id: &str) -> anyhow::Result<String> {
    let mut room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        db::set_entity_room(entity_id, default_room_id)?;
        room_id = default_room_id.to_string();
    }
    Ok(room_id)
}

/// 依出口方向移動；回傳 `(新房間 id, 是否成功)`。
pub fn move_by_exit(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        return Ok((String::new(), false));
    }
    let exits = db::get_exits_for_room(&room_id)?;
    for ex in &exits {
        if ex.direction.trim() == dir {
            db::set_entity_room(entity_id, &ex.to_room_id)?;
            return Ok((ex.to_room_id.clone(), true));
        }
    }
    let objs = db::get_objects_in_room(&room_id)?;
    for o in objs {
        if o.name.trim() != dir {
            continue;
        }
        if o.move_to_room_id.is_empty() || !object_has_socket(&o, "Move") {
            continue;
        }
        db::set_entity_room(entity_id, &o.move_to_room_id)?;
        return Ok((o.move_to_room_id.clone(), true));
    }
    Ok((String::new(), false))
}

// ─── 正方格地圖 ───────────────────────────────────────────────────

fn grid_objects_to_room_objects(objs: &[crate::grid::GridObject]) -> Vec<RoomObject> {
    objs.iter()
        .map(|o| RoomObject {
            id: o.id.clone(),
            name: o.name.clone(),
            owner: String::new(),
            sockets: if o.quantity > 0 {
                vec!["Look".into(), "Gather".into()]
            } else {
                vec!["Look".into()]
            },
            responses: Default::default(),
            move_to_room_id: String::new(),
        })
        .collect()
}

/// 依方位字在正方格上走一步。成功時已寫入 DB。
pub fn move_by_grid_direction(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    move_by_place_route(entity_id, direction, None)
}

/// 沿確切相鄰場所移動；未指定目標時才取該方向排序後的第一條路。
pub fn move_by_place_route(
    entity_id: &str,
    direction: &str,
    target_place_id: Option<&str>,
) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if !matches!(dir, "東" | "西" | "南" | "北" | "上" | "下" | "進" | "出") {
        return Ok((String::new(), false));
    }
    let arc = store::get_store().ok_or_else(|| anyhow::anyhow!("no store"))?;
    let current_room = arc.read().get_entity_room(entity_id);
    let first_move =
        if let Some(target) = target_place_id.filter(|target| !target.trim().is_empty()) {
            let movement = arc
                .write()
                .move_entity_to_adjacent_place(entity_id, target.trim())?;
            if let Some(ref movement) = movement {
                anyhow::ensure!(movement.route.direction == dir, "目標場所與方向不相符");
            }
            movement
        } else {
            arc.write().move_entity_direction(entity_id, dir)?
        };
    if let Some(place_move) = first_move {
        record_command_move(entity_id, &place_move);
        let new_room = {
            let s = arc.read();
            let tz = s
                .get_place(&place_move.to_place_id)
                .map(|p| p.zone.clone())
                .unwrap_or_else(|| "wt-c-wenyao".to_string());
            let _ = s.step_npc_grid_coord(
                entity_id,
                &place_move.route.direction,
                &tz,
                super::now_unix() as u64,
            );
            s.get_entity_room(entity_id)
        };
        return Ok((new_room, true));
    }
    if let Some(gdir) = Direction::from_zh(dir)
        && reveal_missing_grid_route(entity_id, gdir)?
    {
        let second_move = { arc.write().move_entity_direction(entity_id, dir)? };
        if let Some(place_move) = second_move {
            record_command_move(entity_id, &place_move);
            let new_room = {
                let s = arc.read();
                let tz = s
                    .get_place(&place_move.to_place_id)
                    .map(|p| p.zone.clone())
                    .unwrap_or_else(|| "wt-c-wenyao".to_string());
                let _ = s.step_npc_grid_coord(
                    entity_id,
                    &place_move.route.direction,
                    &tz,
                    super::now_unix() as u64,
                );
                s.get_entity_room(entity_id)
            };
            return Ok((new_room, true));
        }
    }
    Ok((current_room, false))
}

fn record_command_move(entity_id: &str, movement: &store::PlaceMove) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let from_label = crate::db::get_room_name(&movement.from_place_id)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "原處".to_string());
    let to_label = crate::db::get_room_name(&movement.to_place_id)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "前方地界".to_string());
    db::set_entity_journey(crate::world::journey::EntityJourney {
        entity_id: entity_id.to_string(),
        reason: "player_command".to_string(),
        destination_place_id: Some(movement.to_place_id.clone()),
        planned_path: Vec::new(),
        consequence: format!(
            "自「{}」向{}行至「{}」",
            from_label,
            movement.route.direction,
            to_label
        ),
        updated_at: now,
    });
}


fn reveal_missing_grid_route(entity_id: &str, direction: Direction) -> anyhow::Result<bool> {
    let Some(store) = store::get_store() else {
        return Ok(false);
    };
    let Some(entity) = store.read().get_entity(entity_id) else {
        return Ok(false);
    };
    let (Some(x), Some(y)) = (entity.grid_x, entity.grid_y) else {
        return Ok(false);
    };
    let from = SquareCoord::new(x, y);
    let changed = crate::server::grid_manager::with_square_grid_mut(|grid| {
        let Some(cell) = grid.get(from) else {
            return false;
        };
        if !cell.explicit_exits.is_empty() {
            return false;
        }
        let target = from.neighbor(direction);
        let before = grid.len();
        crate::grid::reveal_grid_disk(grid, target, 2);
        grid.can_walk(from, target) && grid.len() > before
    })
    .unwrap_or(false);
    if changed {
        let grid = crate::server::grid_manager::get_runtime_square_grid()
            .ok_or_else(|| anyhow::anyhow!("grid not loaded"))?;
        store.write().sync_places_from_grid(&grid)?;
    }
    Ok(changed)
}

/// 組裝正方格 RoomView（當前格的實體、出口、物件）。
pub fn get_grid_room_view(
    _player_id: &str,
    x: i32,
    y: i32,
    game_hour: i32,
) -> anyhow::Result<Option<RoomView>> {
    let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
    let coord = SquareCoord::new(x, y);
    let cell = if let Some(grid) = &grid_opt {
        let Some(c) = grid.get(coord) else {
            return Ok(None);
        };
        c.clone()
    } else {
        let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
        let mut c = crate::grid::GridCell::new(coord, terrain, format!("{zone}・{sub_geo}"));
        c.zone = zone.to_string();
        c.description = format!("{zone}的{sub_geo}地帶");
        c
    };
    let cell_objects = if cell.objects.is_empty() && !is_inside_settlement_urban_core(x, y) {
        crate::grid::r#gen::generate_cell_objects(coord, cell.terrain)
    } else {
        cell.objects.clone()
    };
    let mut room_objs = grid_objects_to_room_objects(&cell_objects);
    room_objs.extend(super::player_scene::player_layer_scene_objects_for_cell(
        &cell,
    ));
    let mut room = Room {
        id: grid_room_id_from_coord(x, y),
        name: if cell.name.is_empty() {
            crate::grid::terrain_name_zh(cell.terrain).to_string()
        } else {
            cell.name.clone()
        },
        tags: cell.tags.clone(),
        zone: cell.zone.clone(),
        description: cell.description.clone(),
        objects: room_objs.clone(),
    };
    let mut exits: Vec<Exit> = Vec::new();
    if let Some(grid) = &grid_opt {
        if !cell.explicit_exits.is_empty() {
            for ex in &cell.explicit_exits {
                let target_name = grid
                    .get(ex.target)
                    .map(|tc| {
                        if tc.name.is_empty() {
                            crate::grid::terrain_name_zh(tc.terrain).to_string()
                        } else {
                            tc.name.clone()
                        }
                    })
                    .unwrap_or_else(|| "▒▒▒".to_string());
                exits.push(Exit {
                    direction: ex.dir.label_zh().to_string(),
                    to_room_id: grid_room_id_from_coord(ex.target.x, ex.target.y),
                    to_room_name: target_name,
                    route_id: String::new(),
                    mode: String::new(),
                    travel_cost: 0,
                });
            }
        } else {
            for n_coord in coord.neighbors() {
                let Some(nc) = grid.get(n_coord) else {
                    continue;
                };
                if !grid.can_walk(coord, n_coord) {
                    continue;
                }
                let Some(dir) = coord.direction_to(n_coord) else {
                    continue;
                };
                let target_name = if nc.name.is_empty() {
                    crate::grid::terrain_name_zh(nc.terrain).to_string()
                } else {
                    nc.name.clone()
                };
                exits.push(Exit {
                    direction: dir.label_zh().to_string(),
                    to_room_id: grid_room_id_from_coord(n_coord.x, n_coord.y),
                    to_room_name: target_name,
                    route_id: String::new(),
                    mode: String::new(),
                    travel_cost: 0,
                });
            }
        }
    }
    let mut entities = db::get_entities_at_grid(x, y, game_hour)
        .map(filter_room_entities)
        .unwrap_or_default();

    let is_road = room.name.contains("道路")
        || room.name.contains("木橋")
        || cell.terrain == crate::grid::Terrain::Road
        || cell.terrain == crate::grid::Terrain::Bridge;

    let time_bucket = (crate::game::now_unix() / 180) as i64;
    let mut matched_live_slot = false;

    if !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
    {
        for z_idx in 0..CANONICAL_HUNTING_ZONES.len() {
            let slots = live_zone_shanhai_slots(z_idx, time_bucket);
            for sl in slots.iter().filter(|s| s.x == x && s.y == y) {
                if sl.tier == 1 && !is_road && !is_apex_beast_tracked_at(x, y) {
                    if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                        room_objs.push(RoomObject {
                            id: "shanhai_apex_trace".to_string(),
                            name: "新鮮霸主獸跡".to_string(),
                            owner: String::new(),
                            sockets: vec!["Look".to_string()],
                            responses: std::collections::HashMap::new(),
                            move_to_room_id: String::new(),
                        });
                    }
                } else if sl.tier == 1 && is_road {
                    // 霸主獸跡不留於人工道路上
                } else if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, sl.tier_tag) {
                    amb.id = format!("shanhai_{}_{}_{}_{}", z_idx, sl.slot_idx, x, y);
                    if !entities.iter().any(|e| e.display_title == amb.display_title) {
                        entities.push(amb);
                    }
                }
                matched_live_slot = true;
            }
        }
    }

    if !matched_live_slot
        && !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
        && (x.abs() > 18 || y.abs() > 18)
        && !entities.iter().any(|e| e.kind == crate::entity::EntityKind::Npc)
    {
        let z_idx = nearest_hunting_zone_idx(x, y);
        let st = get_zone_trophic_state(z_idx);
        let apex_present = st.as_ref().map(|s| s.apex_presence >= 0.95).unwrap_or(true);
        let t4_scale = st
            .as_ref()
            .map(|s| (s.n_tier4 / s.k_tier4.max(1.0)).clamp(0.20, 1.25))
            .unwrap_or(1.0);
        let roam_threshold = (72.0 * t4_scale) as u64;

        // 霸主威壓屏息格（約 14% 獵場腹地）：尋常小怪驚散匿跡，地上留有【新鮮霸主獸跡】，查探後方會破霧顯形
        let silence_hash = ((x as i64 * 31337) ^ (y as i64 * 65521)).unsigned_abs() % 7;
        if !is_road && is_apex_beast_tracked_at(x, y) && apex_present {
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, "__apex__") {
                amb.id = format!("apex_{z_idx}_{x}_{y}");
                entities.push(amb);
            }
        } else if !is_road && silence_hash == 0 && apex_present {
            if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                room_objs.push(RoomObject {
                    id: "shanhai_apex_trace".to_string(),
                    name: "新鮮霸主獸跡".to_string(),
                    owner: String::new(),
                    sockets: vec!["Look".to_string()],
                    responses: std::collections::HashMap::new(),
                    move_to_room_id: String::new(),
                });
            }
        } else {
            // 尋常物種與山海小怪：隨時間片在格內動態游走顯形
            let roam_gate = ((x as i64).wrapping_mul(1103515245)
                ^ (y as i64).wrapping_mul(12345)
                ^ time_bucket.wrapping_mul(2654435761))
                .unsigned_abs()
                % 100;
            if is_road || roam_gate < roam_threshold {
                if let Some(amb) = ensure_ambient_npc_at_grid(x, y, &room.name) {
                    entities.push(amb);
                }
                // 獸群／小怪群聚地塊（約 28% 野外地塊有結群伴生小怪或同群獸類）
                if !is_road && roam_gate < 28 {
                    let flock_tag = if roam_gate % 2 == 0 { "__tier4a__" } else { "__tier4b__" };
                    if let Some(mut companion) = ensure_ambient_npc_at_grid(x, y, flock_tag) {
                        companion.id = format!("ambient_flock_{x}_{y}");
                        if !entities.iter().any(|e| e.display_title == companion.display_title) {
                            entities.push(companion);
                        }
                    }
                }
            }
        }
    }

    if let Some(site) = peek_recent_hunt_kill_site(x, y, crate::game::now_unix()) {
        room_objs.push(RoomObject {
            id: format!("hunt_kill_site_{x}_{y}"),
            name: format!("{}・圍獵遺骸", site.prey_name),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Harvest".to_string()],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }
    if !is_inside_settlement_urban_core(x, y)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
        && !is_road
    {
        let z_idx = nearest_hunting_zone_idx(x, y);
        if let Some(st) = &get_zone_trophic_state(z_idx) {
            let (weather, _) = current_zone_weather_and_tide(z_idx, time_bucket);
            let fungi_threshold = if weather == ZoneWeatherPhase::RainMist {
                105.0
            } else {
                120.0
            };
            if st.detritus_s >= fungi_threshold
                && ((x as i64 * 17389) ^ (y as i64 * 49157)).unsigned_abs() % 8 == 0
            {
                if !room_objs.iter().any(|o| o.id == format!("detritus_fungi_{x}_{y}")) {
                    room_objs.push(RoomObject {
                        id: format!("detritus_fungi_{x}_{y}"),
                        name: "腐木牛樟靈菇叢".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "Harvest".to_string()],
                        responses: std::collections::HashMap::new(),
                        move_to_room_id: String::new(),
                    });
                }
            }
        }
    }
     let vendor_objs = super::player_scene::dynamic_vendor_objects_for_entities(&entities);
     for v_obj in vendor_objs {
         if !room_objs.iter().any(|o| o.id == v_obj.id) {
             room_objs.push(v_obj);
         }
     }
     room.objects = room_objs.clone();
     let objects = room_objs;
     Ok(Some(RoomView {
         room,
         exits,
         entities,
         objects,
     }))
 }
 

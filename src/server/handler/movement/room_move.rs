static IN_PRIVATE_ROOM: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 查詢玩家是否處於私人客房狀態
pub fn is_player_in_private_room(player_id: &str) -> bool {
    IN_PRIVATE_ROOM.read().contains(player_id)
}

/// 設定玩家私人客房狀態
pub fn set_player_private_room(player_id: &str, in_room: bool) {
    let mut lock = IN_PRIVATE_ROOM.write();
    if in_room {
        lock.insert(player_id.to_string());
    } else {
        lock.remove(player_id);
    }
}

pub(crate) fn player_holds_property_deed_or_key(player_id: &str) -> bool {
    let Ok(Some(ent)) = db::get_entity(player_id) else {
        return false;
    };
    let inv = db::get_inventory(&ent.inventory, ent.vit);
    inv.items.iter().any(|item| {
        let id = item.item_id.as_str();
        let name = item.name.as_str();
        matches!(
            id,
            "wt_red_seal_property_deed"
                | "wt_cat_0793"
                | "wt_brass_pin_tumbler_key"
                | "key_brass_home_lock"
        ) || name.contains("紅印屋契")
            || name.contains("門鎖鑰匙")
    })
}

pub(crate) fn is_residential_building(name: &str) -> bool {
    if name.contains("書院")
        || name.contains("學院")
        || name.contains("講院")
        || name.contains("醫館")
        || name.contains("工坊")
        || name.contains("茶屋")
        || name.contains("書屋")
    {
        return false;
    }
    name.contains('宅')
        || name.contains('居')
        || name.contains('屋')
        || name.contains('舍')
        || name.contains("宅院")
        || name.contains("院落")
        || name.contains("小院")
        || name.contains('家')
}

/// 探索冷卻計時：(player_id, x, y) -> 過期 unix timestamp

pub(super) fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let mut direction = msg.direction.clone();
    let ent = db::get_entity(&player_id).ok().flatten();
    let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));

    if direction.is_empty()
        && (msg.x.is_some() || msg.y.is_some())
        && let (Some(tx), Some(ty)) = (msg.x, msg.y)
    {
        match (tx - gx, ty - gy) {
            (0, 1) => direction = "北".to_string(),
            (0, -1) => direction = "南".to_string(),
            (1, 0) => direction = "東".to_string(),
            (-1, 0) => direction = "西".to_string(),
            (dx, dy) if dx != 0 || dy != 0 => direction = "行旅".to_string(),
            _ => {}
        }
    }
    if direction.is_empty()
        && !msg.target_place_id.is_empty()
        && let Some(store) = crate::store::get_store()
    {
        let guard = store.read();
        if let Some(from_place) = guard.get_entity_place_id(&player_id)
            && let Some(route) = guard.place_graph.route_to(&from_place, &msg.target_place_id)
        {
            direction = route.direction.clone();
        }
    }
    if direction.is_empty() {
        conn.send_error("direction required");
        return;
    }

    // 私人客房「進／出」分流（支援中英文別名：進/in/enter, 出/out/leave）
    let dir_trimmed = direction.trim();
    let dir_lower = dir_trimmed.to_ascii_lowercase();
    let is_dir_in = dir_trimmed == "進" || dir_lower == "in" || dir_lower == "enter";
    let is_dir_out = dir_trimmed == "出" || dir_lower == "out" || dir_lower == "leave";

    if is_dir_in {
        let gh = current_game_hour(&conn.cfg);
        let cell_info = crate::server::grid_manager::get_runtime_grid_cell(SquareCoord::new(gx, gy))
            .map(|c| {
                let name = c.name.trim().to_string();
                let is_lodging = name.contains("旅社")
                    || name.contains("茶屋")
                    || (name.contains("行館") && !name.contains("印行館"))
                    || name.contains("歇舍")
                    || name.contains("驛館");
                let is_residential = is_residential_building(&name);
                let can_enter = is_lodging || (is_residential && player_holds_property_deed_or_key(&player_id));
                (can_enter, is_lodging)
            });
        let (can_enter, is_lodging) = cell_info.unwrap_or((false, false));

        if can_enter {
            set_player_private_room(&player_id, true);
            send_grid_view(conn, &player_id, gx, gy, gh);
            let narrate_text = if is_lodging {
                "你推開客房木門邁步入內，反手扣上黃銅插銷，將外頭街巷的喧囂隔絕在門外。"
            } else {
                "你穿過門廊步入宅院內室，掩上房門，屋內恬靜安寧。"
            };
            conn.send_json(&super::super::protocol::NarrateMsg {
                msg_type: "narrate".into(),
                text: narrate_text.into(),
            });
        } else {
            let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
            conn.send_json(&BlockedMsg {
                msg_type: "blocked".into(),
                direction: direction.clone(),
            });
        }
        return;
    } else if is_dir_out {
        let is_lodging = crate::server::grid_manager::get_runtime_grid_cell(SquareCoord::new(gx, gy))
            .map(|c| {
                let name = c.name.trim();
                name.contains("旅社")
                    || name.contains("茶屋")
                    || (name.contains("行館") && !name.contains("印行館"))
                    || name.contains("歇舍")
                    || name.contains("驛館")
            })
            .unwrap_or(false);
        set_player_private_room(&player_id, false);
        let gh = current_game_hour(&conn.cfg);
        send_grid_view(conn, &player_id, gx, gy, gh);
        let narrate_text = if is_lodging {
            "你拉開黃銅插銷，推開房門走出客房，重新回到堂前。"
        } else {
            "你推開內室房門邁步而出，重新回到外間。"
        };
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: narrate_text.into(),
        });
        return;
    } else {
        // 移動至其他方向時自動脫離私人客房
        if is_player_in_private_room(&player_id) {
            set_player_private_room(&player_id, false);
        }
    }
    if matches!(direction.as_str(), "東北" | "西北" | "東南" | "西南") {
        let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.clone(),
        });
        return;
    }
    // 優先在 SquareGrid（5,040 格白塔半島）上進行移動
    let delta_opt = match direction.as_str() {
        "北" => Some((0, 1)),
        "南" => Some((0, -1)),
        "東" => Some((1, 0)),
        "西" => Some((-1, 0)),
        "行旅" => msg.x.zip(msg.y).map(|(tx, ty)| (tx - gx, ty - gy)),
        _ => None,
    };

    if let Some((dx, dy)) = delta_opt {
        let (nx, ny) = (gx + dx, gy + dy);
        let target_coord = SquareCoord::new(nx, ny);
        let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
        let target_cell = grid_snap.as_ref().and_then(|g| g.get(target_coord).cloned());

        if let Some(cell) = target_cell {
            let travel_path = if direction == "行旅" {
                grid_snap
                    .as_ref()
                    .and_then(|g| g.find_path(SquareCoord::new(gx, gy), target_coord))
            } else {
                None
            };
            let can_move = if direction == "行旅" {
                cell.terrain.walkable() && travel_path.is_some()
            } else {
                grid_snap
                    .as_ref()
                    .map(|g| can_step_in_nine_grid(g, SquareCoord::new(gx, gy), target_coord))
                    .unwrap_or_else(|| cell.terrain.walkable())
            };
            if can_move {
                let old_room = crate::grid::grid_room_id_from_coord(gx, gy);
                let (gx, gy) = (nx, ny);
                let is_water_or_coastal = matches!(
                    cell.terrain,
                    Terrain::Water | Terrain::WaterDeep | Terrain::Beach | Terrain::Dock
                ) || cell.name.contains('海')
                    || cell.name.contains('水')
                    || cell.name.contains('灘')
                    || cell.name.contains('渚')
                    || cell.name.contains('渡')
                    || cell.name.contains('港');
                let is_outdoor_wild = matches!(
                    cell.terrain,
                    Terrain::Plain
                        | Terrain::Grassland
                        | Terrain::Desert
                        | Terrain::Tundra
                        | Terrain::Forest
                        | Terrain::ForestHeavy
                        | Terrain::ForestLight
                        | Terrain::Jungle
                        | Terrain::Mountain
                        | Terrain::Hills
                        | Terrain::MinePass
                        | Terrain::Swamp
                        | Terrain::Beach
                        | Terrain::EmptyFurrow
                        | Terrain::DryField
                        | Terrain::FarmField
                );
                let is_outdoor_step = is_outdoor_wild || is_water_or_coastal;
                let mut is_rugged = matches!(
                    cell.terrain,
                    Terrain::Mountain
                        | Terrain::Hills
                        | Terrain::MinePass
                        | Terrain::ForestHeavy
                        | Terrain::Forest
                        | Terrain::ForestLight
                        | Terrain::Jungle
                        | Terrain::Swamp
                        | Terrain::Desert
                        | Terrain::Tundra
                ) || cell.terrain.move_cost() >= 1.5;
                let is_bamboo_cleared = crate::grid::zhulin::is_bamboo_trail_cleared(&cell);
                if is_bamboo_cleared {
                    is_rugged = false;
                    if (gx + gy).abs() % 4 == 0 {
                        if let Ok(Some(current_ent)) = db::get_entity(&player_id) {
                            let new_vit = (current_ent.vit + 1).min(100);
                            if new_vit != current_ent.vit {
                                let _ = db::update_vit(&player_id, new_vit);
                            }
                        }
                    }
                }

                let now = crate::game::now_unix();
                if is_outdoor_step && (gx + gy).abs() % 6 == 0 {
                    let habit_kind = if is_water_or_coastal {
                        crate::pg::living_world::BreathHabitKind::TideContinuous
                    } else {
                        crate::pg::living_world::BreathHabitKind::ForestLong
                    };
                    crate::pg::living_world::record_breath_activity(&player_id, habit_kind, 1, now);
                }

                let craft_dims = crate::server::handler::entity_total_craft_dims(&player_id);
                let seed = ent.as_ref().and_then(|e| e.soul_seed).unwrap_or(0);
                let breath = crate::pg::living_world::get_or_derive_breath_habit(&player_id, "", seed);
                let has_leg_sprain = crate::pg::living_world::get_entity_afflictions(&player_id)
                    .contains(&crate::pg::living_world::AfflictionKind::LegSprain);

                if has_leg_sprain {
                    if let Ok(Some(current_ent)) = db::get_entity(&player_id) {
                        let new_vit = (current_ent.vit - 1).max(5);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                        }
                    }
                    if is_rugged && (gx + gy).abs() % 4 == 0 {
                        conn.send_json(&super::super::protocol::NarrateMsg {
                            msg_type: "narrate".into(),
                            text: "【下盤扭傷】跋涉間足踝經絡隱隱作痛，步履略顯沉重，宜尋醫館或敷用舒筋藥膏。".into(),
                        });
                    }
                } else if (craft_dims.wayfare >= 0.8
                    || matches!(
                        breath.dominant,
                        crate::pg::living_world::BreathHabitKind::ForestLong
                            | crate::pg::living_world::BreathHabitKind::TideContinuous
                    ))
                    && is_rugged
                    && (gx + gy).abs() % 9 == 0
                {
                    if let Ok(Some(current_ent)) = db::get_entity(&player_id) {
                        let new_vit = (current_ent.vit + 1).min(100);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                        }
                    }
                    conn.send_json(&super::super::protocol::NarrateMsg {
                        msg_type: "narrate".into(),
                        text: "【行旅涉險・長息輕身】你順著山海地勢勻長換氣，腳下借力輕省，跋涉不覺疲累。".into(),
                    });
                }
                if let Err(e) = db::set_entity_grid(&player_id, nx, ny) {
                    tracing::error!("set_entity_grid failed for {player_id} to ({nx}, {ny}): {e}");
                    conn.send_error(gametext::client("move_failed"));
                    return;
                }
                let new_room = crate::grid::grid_room_id_from_coord(nx, ny);
                let cell_name = if cell.name.trim().is_empty() {
                    crate::grid::terrain_name_zh(cell.terrain).to_string()
                } else {
                    cell.name.clone()
                };

                let moved = MovedMsg {
                    msg_type: "moved".into(),
                    player_id: player_id.to_string(),
                    room_id: new_room.clone(),
                    room_name: cell_name,
                };
                let bytes = serde_json::to_vec(&moved).unwrap_or_default();
                conn.hub.broadcast(bytes);
                super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &old_room, Some(&player_id));
                super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &new_room, Some(&player_id));

                let gh = current_game_hour(&conn.cfg);
                let claimed = crate::pg::living_world::claim_ground_dropped_items(nx, ny);
                if !claimed.is_empty() {
                    for (item_id, _name, qty) in &claimed {
                        let _ = db::add_to_inventory(&player_id, item_id, *qty);
                    }
                    conn.send_json(&super::super::protocol::NarrateMsg {
                        msg_type: "narrate".into(),
                        text: "你在地上尋回了先前遺落的隨身物資。".into(),
                    });
                    super::push_refresh(conn, &player_id);
                }
                send_grid_view(conn, &player_id, nx, ny, gh);
                if let Some(path) = &travel_path {
                    if path.len() > 2 {
                        let mut trail_cnt = 0usize;
                        let mut campfire_cnt = 0usize;
                        let mut grove_cnt = 0usize;
                        let mut water_cnt = 0usize;
                        if let Some(g) = &grid_snap {
                            for step_coord in &path[1..path.len() - 1] {
                                if let Some(sc) = g.get(*step_coord) {
                                    if sc.has_room_tag("zl_campfire:active")
                                        || sc.objects.iter().any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
                                    {
                                        campfire_cnt += 1;
                                    } else if crate::grid::zhulin::is_bamboo_trail_cleared(sc)
                                        || crate::grid::zhulin::has_bamboo_trail_mark(sc)
                                    {
                                        trail_cnt += 1;
                                    } else if sc.has_room_tag("waterside") || sc.name.contains("水") || sc.name.contains("泉") || sc.name.contains("灘") {
                                        water_cnt += 1;
                                    } else {
                                        grove_cnt += 1;
                                    }
                                }
                            }
                        }
                        let mut parts = Vec::new();
                        if trail_cnt > 0 {
                            parts.push(format!("循著闢開的 {} 處疏竹小徑輕快穿行", trail_cnt));
                        }
                        if campfire_cnt > 0 {
                            parts.push(format!("掠過 {} 處竹林營火的微溫炭香", campfire_cnt));
                        }
                        if grove_cnt > 0 {
                            parts.push(format!("撥開 {} 處幽深竹蔭", grove_cnt));
                        }
                        if water_cnt > 0 {
                            parts.push(format!("踏過 {} 處沁涼水岸", water_cnt));
                        }
                        let mid_clause = if parts.is_empty() {
                            format!("跨越 {} 處林地", path.len() - 1)
                        } else {
                            parts.join("、")
                        };
                        let dest_label = if cell.name.trim().is_empty() {
                            crate::grid::terrain_name_zh(cell.terrain)
                        } else {
                            cell.name.as_str()
                        };
                        conn.send_json(&super::super::protocol::NarrateMsg {
                            msg_type: "narrate".into(),
                            text: format!("你步入【{}】——沿途{}。", dest_label, mid_clause),
                        });
                    }
                }
                emit_presence_reflex_on_arrival(conn, &player_id, &new_room);
                return;
            } else {
                let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
                conn.send_json(&BlockedMsg {
                    msg_type: "blocked".into(),
                    direction: direction.clone(),
                });
                return;
            }
        } else {
            let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
            conn.send_json(&BlockedMsg {
                msg_type: "blocked".into(),
                direction: direction.clone(),
            });
            return;
        }
    }

    if db::get_entity_place_id(&player_id).is_none() {
        conn.send_error(gametext::client("move_failed"));
        return;
    }
    handle_move_place(
        conn,
        &player_id,
        &direction,
        (!msg.target_place_id.is_empty()).then_some(msg.target_place_id.as_str()),
    );
}
fn handle_move_place(
    conn: &WsConnection,
    player_id: &str,
    direction: &str,
    target_place_id: Option<&str>,
) {
    let old_room = db::get_entity_room(player_id).unwrap_or_default();
    let (new_room, ok) = match game::move_by_place_route(player_id, direction, target_place_id) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("player move failed {player_id}: {e}");
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    if !ok {
        let _ = event::append(game::now_unix(), player_id, BLOCKED, direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.to_string(),
        });
        return;
    }
    if crate::pg::living_world::get_entity_afflictions(player_id)
        .contains(&crate::pg::living_world::AfflictionKind::LegSprain)
    {
        if let Ok(Some(current_ent)) = db::get_entity(player_id) {
            let new_vit = (current_ent.vit - 1).max(5);
            if new_vit != current_ent.vit {
                let _ = db::update_vit(player_id, new_vit);
            }
        }
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: "【下盤扭傷】跋涉間足踝經絡隱隱作痛，步履略顯沉重，宜尋醫館或敷用舒筋藥膏。".into(),
        });
    }
    let gh = current_game_hour(&conn.cfg);
    let room_name = game::get_place_room_view(&new_room, gh)
        .ok()
        .flatten()
        .map(|v| v.room.name)
        .unwrap_or_else(|| new_room.clone());
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.to_string(),
        room_id: new_room.clone(),
        room_name,
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
    super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &old_room, Some(player_id));
    super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &new_room, Some(player_id));
    let ent = db::get_entity(player_id).ok().flatten();
    let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
    send_grid_view(conn, player_id, gx, gy, gh);
    emit_presence_reflex_on_arrival(conn, player_id, &new_room);
}


// ── 昏倒輔助 ──────────────────────────────────────────────────────────────

/// 找曼哈頓距離最近的脈絡診所（+）或 landmark 格作為聚落傳送目的地。
/// 找不到時回傳 None（呼叫端 fallback 到 (0, 0, "脈絡診所")）。
pub fn find_nearest_clinic_for_combat(from_x: i32, from_y: i32) -> (i32, i32, String) {
    find_nearest_clinic_or_landmark(from_x, from_y)
        .unwrap_or_else(|| (210, 510, "脈絡診所".to_string()))
}

fn find_nearest_clinic_or_landmark(from_x: i32, from_y: i32) -> Option<(i32, i32, String)> {
    let grid = crate::server::grid_manager::get_runtime_square_grid()?;
    // 優先找脈絡診所（+）
    let clinic = grid
        .cells()
        .filter(|c| c.terrain == crate::grid::Terrain::Clinic || c.terrain.display_char() == "+")
        .min_by_key(|c| (c.coord.x - from_x).abs() + (c.coord.y - from_y).abs());
    if let Some(c) = clinic {
        let name = if c.name.is_empty() {
            "脈絡診所".to_string()
        } else {
            c.name.clone()
        };
        return Some((c.coord.x, c.coord.y, name));
    }

    // 次選其他聚落地標
    grid.cells()
        .filter(|c| c.terrain.category() == "landmark" && !c.name.is_empty())
        .min_by_key(|c| (c.coord.x - from_x).abs() + (c.coord.y - from_y).abs())
        .map(|c| (c.coord.x, c.coord.y, c.name.clone()))
}

#[allow(dead_code)]
fn find_nearest_landmark(from_x: i32, from_y: i32) -> Option<(i32, i32, String)> {
    find_nearest_clinic_or_landmark(from_x, from_y)
}

/// 從背包隨機移除 1-3 件（整堆移除一個 item_id），寫入地面遺落資料表與快取，回傳被移除物品名稱列表。
pub fn faint_drop_items_to_ground(
    player_id: &str,
    inventory_json: &str,
    gx: i32,
    gy: i32,
    now: i64,
) -> Vec<String> {
    let dropped_tuples = drop_random_items(player_id, inventory_json);
    if !dropped_tuples.is_empty() {
        crate::pg::living_world::record_ground_dropped_items(
            gx,
            gy,
            player_id,
            &dropped_tuples,
            now,
        );
    }
    dropped_tuples.into_iter().map(|(_, name, _)| name).collect()
}

/// 從背包隨機移除 1-3 件（整堆移除一個 item_id），回傳被移除物品的 (item_id, item_name, qty) 列表。
fn drop_random_items(entity_id: &str, inventory_json: &str) -> Vec<(String, String, i32)> {
    #[derive(serde::Deserialize)]
    struct Entry {
        item_id: String,
        qty: i32,
    }

    if inventory_json.is_empty() || inventory_json == "[]" {
        return vec![];
    }
    let entries: Vec<Entry> = match serde_json::from_str(inventory_json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    if entries.is_empty() {
        return vec![];
    }

    let droppable: Vec<&Entry> = entries
        .iter()
        .filter(|e| {
            !e.item_id.starts_with("manual_")
                && !e.item_id.starts_with("corpse_")
                && !e.item_id.starts_with("crystal_")
                && !e.item_id.starts_with("key_")
                && !e.item_id.starts_with("wt_cat_0793")
        })
        .collect();

    if droppable.is_empty() {
        return vec![];
    }

    let mut rng = rand::rng();
    let drop_count = rng.random_range(1_usize..=3_usize).min(droppable.len());

    // Fisher-Yates 取前 drop_count 個隨機索引
    let mut indices: Vec<usize> = (0..droppable.len()).collect();
    for i in 0..drop_count {
        let j = rng.random_range(i..droppable.len());
        indices.swap(i, j);
    }

    let mut dropped = Vec::new();
    for &idx in &indices[..drop_count] {
        let e = droppable[idx];
        // 移除整堆
        let _ = db::remove_from_inventory(entity_id, &e.item_id, e.qty);
        dropped.push((e.item_id.clone(), db::get_item_name(&e.item_id), e.qty));
    }
    dropped
}

/// scout 探路：揭露指定方向的單一鄰格，回傳該格名稱供前端更新路口顯示。
/// WS 訊息格式：`{ type: "scout", direction: "北"|"南"|"東"|"西" }`
/// 回傳 `{ type: "scout_result", direction, name, explored, walkable }`
pub(super) fn handle_scout(conn: &WsConnection, msg: &super::super::protocol::ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let ent = match crate::db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            conn.send_error("entity not found");
            return;
        }
    };
    let (gx, gy) = match (ent.grid_x, ent.grid_y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            conn.send_error("尚未置身於城鎮地圖中");
            return;
        }
    };

    let dir_zh = msg.direction.as_str();
    let dir = match crate::grid::Direction::from_zh(dir_zh) {
        Some(d) => d,
        None => {
            conn.send_error(format!("unknown direction: {dir_zh}"));
            return;
        }
    };

    let target = crate::grid::SquareCoord::new(gx, gy).neighbor(dir);

    // Reveal only cells that already exist in the authoritative grid.
    // Missing cells are map boundaries until an explicit map/import pass adds them.
    let (name, walkable) = {
        let result = crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(target) {
                cell.explored = true;
                let data = (cell.name.clone(), cell.terrain.walkable());
                grid.mark_dirty();
                data
            } else {
                ("邊界未開通".to_string(), false)
            }
        });
        result.unwrap_or_else(|| ("邊界未開通".to_string(), false))
    };

    let resp = serde_json::json!({
        "type": "scout_result",
        "direction": dir_zh,
        "name": name,
        "explored": true,
        "walkable": walkable
    });
    if let Ok(bytes) = serde_json::to_vec(&resp) {
        let _ = conn.tx.try_send(bytes);
    }
}

/// 地形改造處理函式：支援動態地形變更
/// 開山 ⛰︎->△、伐林 Φ->𐡚、開墾 𐡚->≡、播種 ≡->▦/▤/☷、造橋 ≈->≍
pub(super) fn handle_mutate_terrain(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let ent = match db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    // 目標座標：優先使用 msg.x / msg.y，否則使用玩家當前位置
    let (tx, ty) = match (msg.x, msg.y) {
        (Some(x), Some(y)) => (x, y),
        _ => match (ent.grid_x, ent.grid_y) {
            (Some(x), Some(y)) => (x, y),
            _ => {
                conn.send_error("尚未置身於城鎮地圖中");
                return;
            }
        },
    };

    let current_terrain = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(SquareCoord::new(tx, ty)).map(|c| c.terrain));

    let Some(current_t) = current_terrain else {
        conn.send_error("目標格子不存在");
        return;
    };

    let action_str = msg.action.trim();
    let target_str = msg.target_terrain.trim();
    let input_str = msg.player_input.trim();

    // 判斷想要進行的地形改造
    let new_terrain = match (current_t, action_str, target_str) {
        // 開山：Mountain (⛰︎) -> Hills (△)
        (Terrain::Mountain, "mine" | "開山" | "quarry", _)
        | (Terrain::Mountain, _, "hills" | "△" | "丘陵") => Some(Terrain::Hills),

        // 伐林：Φ (Forest/ForestHeavy/ForestLight/Jungle) -> Plain (𐡚)
        (
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
            "chop" | "伐林" | "clearcut" | "伐木",
            _,
        )
        | (
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
            _,
            "plain" | "𐡚" | "平原",
        ) => Some(Terrain::Plain),

        // 開墾：𐡚 (Plain/Grassland/Desert/Tundra) -> EmptyFurrow (≡)
        (
            Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra,
            "till" | "開墾" | "plow" | "耕作",
            _,
        )
        | (
            Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra,
            _,
            "empty_furrow" | "≡" | "荒壟" | "壟溝",
        ) => Some(Terrain::EmptyFurrow),

        // 播種：EmptyFurrow (≡) -> ▦/▤/☷ (FarmField / DryField / HerbGarden)
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", "dry_field" | "▤" | "旱田") => {
            Some(Terrain::DryField)
        }
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", "herb_garden" | "☷" | "藥園" | "草藥園") => {
            Some(Terrain::HerbGarden)
        }
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", _) => {
            Some(Terrain::FarmField)
        }
        (Terrain::EmptyFurrow, _, "farm_field" | "▦" | "農田" | "水田") => {
            Some(Terrain::FarmField)
        }
        (Terrain::EmptyFurrow, _, "dry_field" | "▤" | "旱田") => Some(Terrain::DryField),
        (Terrain::EmptyFurrow, _, "herb_garden" | "☷" | "藥園" | "草藥園") => {
            Some(Terrain::HerbGarden)
        }

        // 造橋：Water (≈) -> Bridge (≍)
        (Terrain::Water, "bridge" | "造橋" | "build_bridge", _)
        | (Terrain::Water, _, "bridge" | "≍" | "橋樑") => Some(Terrain::Bridge),

        // 亦可相容由 player_input 觸發
        _ if input_str.contains("開山") && current_t == Terrain::Mountain => Some(Terrain::Hills),
        _ if (input_str.contains("伐林") || input_str.contains("伐木"))
            && matches!(
                current_t,
                Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle
            ) =>
        {
            Some(Terrain::Plain)
        }
        _ if input_str.contains("開墾")
            && matches!(
                current_t,
                Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra
            ) =>
        {
            Some(Terrain::EmptyFurrow)
        }
        _ if input_str.contains("播種") && current_t == Terrain::EmptyFurrow => {
            if input_str.contains("藥") {
                Some(Terrain::HerbGarden)
            } else if input_str.contains("旱") {
                Some(Terrain::DryField)
            } else {
                Some(Terrain::FarmField)
            }
        }
        _ if input_str.contains("造橋") && current_t == Terrain::Water => Some(Terrain::Bridge),

        _ => None,
    };

    let Some(target_terrain) = new_terrain else {
        conn.send_error("此處地形無法執行該項改造或指令不符。");
        return;
    };

    // 使用 with_square_grid_mut 變更地形
    let mutated = crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(SquareCoord::new(tx, ty)) {
            cell.terrain = target_terrain;
            let suffix = match target_terrain {
                Terrain::Hills => "開鑿石坡",
                Terrain::Plain => "開闊平地",
                Terrain::EmptyFurrow => "整地空壟",
                Terrain::FarmField => "阡陌水田",
                Terrain::DryField => "平整旱田",
                Terrain::HerbGarden => "分畦藥圃",
                Terrain::Bridge => "木石便橋",
                other => crate::grid::terrain_name_zh(other),
            };
            if cell.name.contains('・') {
                let prefix = cell.name.split('・').next().unwrap_or("").trim();
                cell.name = format!("{prefix}・{suffix}");
            } else if cell.name.is_empty() || cell.name == crate::grid::terrain_name_zh(current_t) {
                cell.name = suffix.to_string();
            }
            grid.mark_dirty();
            true
        } else {
            false
        }
    })
    .unwrap_or(false);

    if !mutated {
        conn.send_error("地形改造失敗");
        return;
    }
    // 若改造為水田、旱田或藥圃，立即向作物生長核心註冊新播種農田
    match target_terrain {
        Terrain::FarmField => {
            crate::npc::causal_core::register_crop_plot_sown(tx, ty, "water_field", crate::game::now_unix());
        }
        Terrain::DryField => {
            crate::npc::causal_core::register_crop_plot_sown(tx, ty, "dry_field", crate::game::now_unix());
        }
        Terrain::HerbGarden => {
            crate::npc::causal_core::register_crop_plot_sown(tx, ty, "herb_garden", crate::game::now_unix());
        }
        _ => {}
    }

    let narrative = match target_terrain {
        Terrain::EmptyFurrow => {
            "【百工開拓・墾荒闢壟】你揮動鋤具翻開荒草土石，順著地勢理出整齊通水的田壟溝渠。".to_string()
        }
        Terrain::FarmField => {
            "【百工開拓・播種育苗】你順著整好的田壟引水播下稻種，新苗在微風與活水滋潤下吐露青翠生機。".to_string()
        }
        Terrain::DryField => {
            "【百工開拓・整飭旱田】你耙平土塊、修整畦道，將田地整理得疏鬆透氣，便利各色雜糧與耐旱作物深紮根脈。".to_string()
        }
        Terrain::HerbGarden => {
            "【百工開拓・分畦藥圃】你細分苗床、鋪設腐殖沃土與疏水碎石，闢出一壟壟清氣撲鼻、適宜草藥生長的分畦苗圃。".to_string()
        }
        Terrain::Hills => {
            "【百工開拓・開鑿礦道】你順著岩層裂隙鑿開險峻山壁，闢出可供通行的丘陵石道與採石工作面。".to_string()
        }
        Terrain::Plain => {
            "【百工開拓・伐木取材】你揮斧伐開密林枝幹，清理出開闊平坦的林間空地。".to_string()
        }
        Terrain::Bridge => {
            "【百工開拓・架木為橋】你打下固若金湯的石木橋樁，平鋪厚實行水踏板，於湍急水脈上方築起便利往來的通途。".to_string()
        }
        _ => {
            "【百工開拓・地脈變更】你運用百工巧藝順應地勢梳理此方水土，讓地表煥然一新。".to_string()
        }
    };

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "MutateTerrain".into(),
        target_id: "__terrain_pioneer__".into(),
        target_name: "百工開拓".into(),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });

    // 立即推播最新地圖與視野
    let gh = current_game_hour(&conn.cfg);
    if let (Some(px), Some(py)) = (ent.grid_x, ent.grid_y) {
        send_grid_view(conn, &player_id, px, py, gh);
    } else {
        send_grid_view(conn, &player_id, tx, ty, gh);
    }
    send_place_view(conn, &player_id);
}

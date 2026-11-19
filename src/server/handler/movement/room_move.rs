fn current_substrate_field() -> crate::grid::substrate::slots::FieldState {
    let (w_def, _) = crate::grid::zhulin::weather::current_world_zhulin_weather();
    let wetting = matches!(
        w_def.legacy_key,
        "rain" | "plum_rain" | "storm" | "light_rain" | "downpour"
    );
    crate::grid::substrate::slots::FieldState {
        weather_wetting: wetting,
        ..crate::grid::substrate::slots::FieldState::default()
    }
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
    let current_z = ent.as_ref().map(|e| e.grid_z).unwrap_or(0);

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

    // 立體樓層移動（進 / 出 / 上 / 下，含英文別名）：100% 透過 entities.grid_z (0..=3) 持久化與三段房號廣播
    let dir_trimmed = direction.trim();
    let dir_lower = dir_trimmed.to_ascii_lowercase();
    let vertical_dir = match dir_trimmed {
        "進" => Some("進"),
        "出" => Some("出"),
        "上" => Some("上"),
        "下" => Some("下"),
        _ => match dir_lower.as_str() {
            "in" | "enter" => Some("進"),
            "out" | "leave" => Some("出"),
            "up" => Some("上"),
            "down" => Some("下"),
            _ => None,
        },
    };

    let pool = crate::pg::pool();
    let field = current_substrate_field();
    let has_substrate = crate::grid::substrate::runtime::global().is_some();

    if let Some(vdir) = vertical_dir {
        let gh = current_game_hour(&conn.cfg);
        let snap = crate::grid::substrate::facade::CellFacade::snapshot_cached(
            pool.as_deref(),
            gx,
            gy,
            field,
        );
        match crate::grid::substrate::arcade_street::resolve_floor_move(
            &snap,
            current_z,
            vdir,
            &player_id,
        ) {
            Ok((next_z, narrate_text)) => {
                let old_room = crate::grid::grid_room_id_from_coord_z(gx, gy, current_z);
                if let Err(e) = db::set_entity_grid_z(&player_id, gx, gy, next_z) {
                    tracing::error!(
                        "set_entity_grid_z failed for {player_id} to ({gx},{gy},{next_z}): {e}"
                    );
                    conn.send_error(gametext::client("move_failed"));
                    return;
                }
                let new_room = crate::grid::grid_room_id_from_coord_z(gx, gy, next_z);
                let room_name = if next_z == 0 {
                    snap.synth.micro_name.to_string()
                } else {
                    format!(
                        "{}・{}",
                        snap.synth.micro_name,
                        crate::grid::substrate::arcade_street::floor_label_zh(next_z)
                    )
                };
                let moved = MovedMsg {
                    msg_type: "moved".into(),
                    player_id: player_id.to_string(),
                    room_id: new_room.clone(),
                    room_name,
                };
                let bytes = serde_json::to_vec(&moved).unwrap_or_default();
                conn.hub.broadcast(bytes);
                super::super::broadcast::refresh_room_views_for_room_except(
                    &conn.sessions,
                    &conn.cfg,
                    &old_room,
                    Some(&player_id),
                );
                super::super::broadcast::refresh_room_views_for_room_except(
                    &conn.sessions,
                    &conn.cfg,
                    &new_room,
                    Some(&player_id),
                );
                send_grid_view(conn, &player_id, gx, gy, gh);
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: narrate_text.into(),
                });
            }
            Err(reason) => {
                let _ = event::append(game::now_unix(), &player_id, BLOCKED, vdir);
                conn.send_json(&BlockedMsg {
                    msg_type: "blocked".into(),
                    direction: vdir.to_string(),
                });
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: reason.into(),
                });
            }
        }
        return;
    }

    if matches!(direction.as_str(), "東北" | "西北" | "東南" | "西南") {
        let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.clone(),
        });
        return;
    }

    // 身處室內／樓上（grid_z > 0）不可水平穿牆，須先「出」或「下」回到地面層
    if current_z > 0 {
        let hint = if current_z == 1 {
            "身處內室之中，四壁阻隔，須先步出前堂再往外行。"
        } else {
            "身處樓上空間，須先順著木梯下樓再往外行。"
        };
        let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.clone(),
        });
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: hint.into(),
        });
        return;
    }

    // 優先以 CellFacade（20m 台灣底板＋西麓騎樓防穿牆）進行單步與行旅尋路；底板未載入時回退 SquareGrid
    let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
    let mut auto_walk_meta: Option<(i32, i32, String, usize, Vec<String>)> = None;
    let delta_opt = match direction.as_str() {
        "北" => Some((0, 1)),
        "南" => Some((0, -1)),
        "東" => Some((1, 0)),
        "西" => Some((-1, 0)),
        "行旅" => {
            let Some((tx, ty)) = msg.x.zip(msg.y) else {
                let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
                conn.send_json(&BlockedMsg {
                    msg_type: "blocked".into(),
                    direction: direction.clone(),
                });
                return;
            };
            if tx == gx && ty == gy {
                let gh = current_game_hour(&conn.cfg);
                send_grid_view(conn, &player_id, gx, gy, gh);
                return;
            }
            let plan_opt = if has_substrate {
                let live_clock =
                    crate::grid::substrate::hydrology::WorldClockSnapshot::current_live();
                crate::grid::substrate::facade::CellFacade::plan_path_live_clock(
                    pool.as_deref(),
                    (gx, gy),
                    (tx, ty),
                    live_clock,
                    64,
                )
                .and_then(|path| {
                    if path.len() < 2 {
                        return None;
                    }
                    let mut steps = Vec::with_capacity(path.len() - 1);
                    for pair in path.windows(2) {
                        let d = match (pair[1].0 - pair[0].0, pair[1].1 - pair[0].1) {
                            (0, 1) => "北",
                            (0, -1) => "南",
                            (1, 0) => "東",
                            (-1, 0) => "西",
                            _ => return None,
                        };
                        steps.push(d.to_string());
                    }
                    let dest_snap = crate::grid::substrate::facade::CellFacade::snapshot_live_clock(
                        pool.as_deref(),
                        tx,
                        ty,
                        live_clock,
                    );
                    Some((
                        path[1].0 - gx,
                        path[1].1 - gy,
                        dest_snap.synth.micro_name.to_string(),
                        steps,
                    ))
                })
            } else {
                let start_coord = SquareCoord::new(gx, gy);
                let dest_coord = SquareCoord::new(tx, ty);
                grid_snap.as_ref().and_then(|g| {
                    let dest_cell = g.get(dest_coord)?;
                    if !dest_cell.terrain.walkable() {
                        return None;
                    }
                    let path = g.find_travel_path(start_coord, dest_coord)?;
                    if path.len() < 2 {
                        return None;
                    }
                    let mut steps = Vec::with_capacity(path.len() - 1);
                    for pair in path.windows(2) {
                        let from_c = pair[0];
                        let to_c = pair[1];
                        let dir_str = from_c
                            .direction_to(to_c)
                            .map(|d| d.label_zh().to_string())?;
                        steps.push(dir_str);
                    }
                    let dest_label = if !dest_cell.name.trim().is_empty() {
                        dest_cell.name.clone()
                    } else {
                        crate::grid::terrain_name_zh(dest_cell.terrain).to_string()
                    };
                    Some((path[1].x - gx, path[1].y - gy, dest_label, steps))
                })
            };
            if let Some((sdx, sdy, dest_label, steps)) = plan_opt {
                let total_steps = steps.len();
                let rem_steps = steps[1..].to_vec();
                auto_walk_meta = Some((tx, ty, dest_label, total_steps, rem_steps));
                Some((sdx, sdy))
            } else {
                let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
                conn.send_json(&BlockedMsg {
                    msg_type: "blocked".into(),
                    direction: direction.clone(),
                });
                return;
            }
        }
        _ => None,
    };

    if let Some((dx, dy)) = delta_opt {
        let (nx, ny) = (gx + dx, gy + dy);
        let target_coord = SquareCoord::new(nx, ny);
        let target_cell = grid_snap.as_ref().and_then(|g| g.get(target_coord).cloned());

        let (can_move, cell_name, is_substrate_rugged, is_substrate_water) = if has_substrate {
            let live_clock = crate::grid::substrate::hydrology::WorldClockSnapshot::current_live();
            let from_snap = crate::grid::substrate::facade::CellFacade::snapshot_live_clock(
                pool.as_deref(),
                gx,
                gy,
                live_clock,
            );
            let to_snap = crate::grid::substrate::facade::CellFacade::snapshot_live_clock(
                pool.as_deref(),
                nx,
                ny,
                live_clock,
            );
            let ok = crate::grid::substrate::arcade_street::can_step_horizontal(
                &from_snap, &to_snap,
            );
            let rugged = to_snap.step_ms >= 400
                || to_snap.slope.is_some_and(|s| s.grade_ratio >= 0.35);
            let water = to_snap.l0.surface_water()
                != crate::grid::substrate::l0_cell::SurfaceWaterKind::None;
            (ok, to_snap.synth.micro_name.to_string(), rugged, water)
        } else if let Some(ref cell) = target_cell {
            let ok = grid_snap
                .as_ref()
                .map(|g| can_step_in_nine_grid(g, SquareCoord::new(gx, gy), target_coord))
                .unwrap_or_else(|| cell.terrain.walkable());
            let name = if cell.name.trim().is_empty() {
                crate::grid::terrain_name_zh(cell.terrain).to_string()
            } else {
                cell.name.clone()
            };
            (ok, name, cell.terrain.move_cost() >= 1.5, false)
        } else {
            (false, String::new(), false, false)
        };

        if can_move {
            let old_room = crate::grid::grid_room_id_from_coord_z(gx, gy, 0);
            let (gx, gy) = (nx, ny);
            let mut vit_changed = false;
            let is_rugged = is_substrate_rugged;
            if let Some(ref cell) = target_cell {
                let is_bamboo_cleared = crate::grid::zhulin::is_bamboo_trail_cleared(cell);
                if is_bamboo_cleared {
                    if (gx + gy).abs() % 3 == 0
                        && let Ok(Some(current_ent)) = db::get_entity(&player_id)
                    {
                        let new_vit = (current_ent.vit + 1).min(100);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                            vit_changed = true;
                        }
                    }
                } else if crate::grid::zhulin::is_bamboo_slot_cell(cell)
                    && let Ok(Some(current_ent)) = db::get_entity(&player_id)
                {
                    let inv = &current_ent.inventory;
                    let has_parting_tool = inv.contains("zl_item_fire_hardened_bamboo_knife")
                        || inv.contains("zl_item_sharp_stone_flake")
                        || inv.contains("zl_item_bamboo_stone_axe")
                        || inv.contains("zl_item_rhizome_club");
                    let stride_mod = if has_parting_tool { 4 } else { 2 };
                    if (gx + gy).abs() % stride_mod == 0 && current_ent.vit > 15 {
                        let new_vit = (current_ent.vit - 1).max(15);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                            vit_changed = true;
                        }
                    }
                }
            }

            let now = crate::game::now_unix();
            if (gx + gy).abs() % 6 == 0 {
                let habit_kind = if is_substrate_water {
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

            if let Err(e) = db::set_entity_grid_z(&player_id, nx, ny, 0) {
                tracing::error!("set_entity_grid_z failed for {player_id} to ({nx}, {ny}, 0): {e}");
                conn.send_error(gametext::client("move_failed"));
                return;
            }
            // 僅於活人實際踏入時刷新 pave_heat 合併寫入緩衝與個人探索迷霧點陣圖
            if has_substrate {
                crate::grid::substrate::facade::CellFacade::record_footstep(
                    pool.as_deref(),
                    nx,
                    ny,
                );
            }
            crate::grid::substrate::fog::mark_player_explored(
                pool.as_deref(),
                &player_id,
                nx,
                ny,
                2,
            );

            let new_room = crate::grid::grid_room_id_from_coord_z(nx, ny, 0);
            let moved = MovedMsg {
                msg_type: "moved".into(),
                player_id: player_id.to_string(),
                room_id: new_room.clone(),
                room_name: cell_name,
            };
            let bytes = serde_json::to_vec(&moved).unwrap_or_default();
            conn.hub.broadcast(bytes);
            super::super::broadcast::refresh_room_views_for_room_except(
                &conn.sessions,
                &conn.cfg,
                &old_room,
                Some(&player_id),
            );
            super::super::broadcast::refresh_room_views_for_room_except(
                &conn.sessions,
                &conn.cfg,
                &new_room,
                Some(&player_id),
            );

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
            } else if vit_changed {
                super::push_refresh(conn, &player_id);
            }
            let mut post_arrival_travel_note: Option<String> = None;
            if let Some((dest_x, dest_y, dest_label, total_steps, rem_steps)) = auto_walk_meta {
                if !rem_steps.is_empty() {
                    conn.send_json(&super::super::protocol::AutoWalkPlanMsg {
                        msg_type: "auto_walk_plan".into(),
                        steps: rem_steps,
                        dest_x,
                        dest_y,
                        dest_name: dest_label.clone(),
                        total_steps,
                    });
                    post_arrival_travel_note =
                        Some(format!("你辨明路徑，朝【{}】邁步而行……", dest_label));
                } else {
                    post_arrival_travel_note = Some(format!("你循路步入【{}】。", dest_label));
                }
            }
            send_grid_view(conn, &player_id, nx, ny, gh);
            emit_presence_reflex_on_arrival(conn, &player_id, &new_room);
            if let Some(travel_text) = post_arrival_travel_note {
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: travel_text,
                });
            }
            return;
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

fn resolve_runtime_resource(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let pool = crate::pg::pool()?;
    let source = crate::pg::world_resource::get_at_place(&pool, room_id)
        .ok()
        .flatten()?;
    let id = format!("resource:{}:{}", source.place_id, source.item_id);
    if target_id != id {
        return None;
    }
    let state = if source.current_stock > 0 {
        format!("目前仍可取得約{}份。", source.current_stock)
    } else {
        "眼前能取用的部分已空，必須等它慢慢恢復。".to_string()
    };
    Some(crate::model::RoomObject {
        id,
        name: format!("可取用的{}", source.name),
        owner: String::new(),
        sockets: vec!["Look".to_string(), "Gather".to_string()],
        responses: std::collections::HashMap::from([(
            "Look".to_string(),
            format!("{}；{}", source.origin_label, state),
        )]),
        move_to_room_id: String::new(),
    })
}

fn append_somatic_awareness_note(pid: &str, now: i64, narrative: &mut String) {
    let habit = crate::pg::living_world::record_breath_activity(
        pid,
        crate::pg::living_world::BreathHabitKind::ChamberClear,
        2,
        now,
    );
    let soul_seed = db::get_entity(pid).ok().flatten().and_then(|e| e.soul_seed).unwrap_or(0);
    let meridian_prose = crate::combat::wordplate::somatic_meridian_awareness(soul_seed);
    let somatic_desc = habit.dominant.somatic_description();
    let somatic_trimmed = somatic_desc.trim_end_matches('。');
    let meridian_prose = meridian_prose.replace("其人", "你");
    narrative.push_str(&format!(
        "\n此際斂氣澄心，體內「{}」隨呼吸自如流轉——{}。\n內觀周天經絡：{}",
        habit.dominant.label(),
        somatic_trimmed,
        meridian_prose
    ));
}

const PLAYER_GATHER_BATCH: i32 = 3;

fn format_runtime_gather_batch_narrative(
    gathered: &crate::pg::world_resource::GatherResult,
) -> String {
    if gathered.quantity <= 1 {
        gathered.narrative.clone()
    } else {
        format!(
            "{}\n你順著手勢一氣呵成連取了 {} 份「{}」，直到手頭分量紮實才收手。",
            gathered.narrative, gathered.quantity, gathered.name
        )
    }
}

fn try_grid_cell_object_action(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    let Some(coord) = parse_grid_room_id(player_room) else {
        return false;
    };
    if action != "Gather" && action != "Harvest" && action != "Take" {
        return false;
    }

    let has_target_obj = crate::server::grid_manager::get_runtime_grid_cell(coord)
        .map(|c| c.objects.iter().any(|o| o.id == obj.id || o.name == obj.name))
        .unwrap_or(false);
    if !has_target_obj {
        return false;
    }

    let mut updated_state = None;
    let mut zero_quantity = false;
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(coord) {
            if let Some(target_obj) = cell.objects.iter_mut().find(|o| o.id == obj.id || o.name == obj.name) {
                if target_obj.quantity > 0 {
                    target_obj.quantity -= 1;
                    let remaining_qty = target_obj.quantity;
                    let obj_name = target_obj.name.clone();
                    let obj_id = target_obj.id.clone();
                    updated_state = Some((
                        cell.explored,
                        cell.freshness,
                        cell.objects.clone(),
                        remaining_qty,
                        obj_name,
                        obj_id,
                    ));
                } else {
                    zero_quantity = true;
                }
            }
        }
    });

    if zero_quantity {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: format!("眼前的「{}」已被採擷殆盡，需待時序流轉自然復生。", obj.name),
            success: false,
            actions: vec!["Look".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

    let Some((explored, freshness, updated_objects, remaining_qty, obj_name, obj_id)) = updated_state else {
        return false;
    };

    let _ = db::update_cell_state(coord.x, coord.y, explored, freshness, &updated_objects);

    let item_id = if obj_id.starts_with("wild_fruit")
        || obj_name.contains("漿果")
        || obj_name.contains("果")
        || obj_name.contains("山茶籽")
        || obj_name.contains("燕窩")
    {
        "harvest_berries"
    } else if obj_id.starts_with("wood")
        || obj_name.contains("木材")
        || obj_name.contains("原木")
        || obj_name.contains("枯木")
        || obj_name.contains("倒木")
        || obj_name.contains("心材")
        || obj_name.contains("木材")
        || obj_name.contains("良材")
        || obj_name.contains("雲杉")
        || obj_name.contains("烏木")
        || obj_name.contains("韌竹")
        || obj_name.contains("鐵杉")
        || obj_name.contains("穀木")
        || obj_name.contains("樹脂")
    {
        if obj_name.contains("溫木") || obj_name.contains("烏木") || obj_name.contains("心材") || obj_name.contains("樹脂") {
            "harvest_rare_wood"
        } else if obj_name.contains("雲杉") || obj_name.contains("原木") {
            "harvest_wood"
        } else {
            "harvest_deadwood"
        }
    } else if obj_id.starts_with("herb")
        || obj_name.contains("草")
        || obj_name.contains("苔")
        || obj_name.contains("蓮")
        || obj_name.contains("藤")
        || obj_name.contains("葦")
        || obj_name.contains("百草")
    {
        if obj_name.contains("藤") {
            "harvest_vine"
        } else {
            "harvest_herb"
        }
    } else if obj_id.starts_with("ore")
        || obj_name.contains("礦")
        || obj_name.contains("玉")
        || obj_name.contains("石")
        || obj_name.contains("岩")
        || obj_name.contains("晶")
        || obj_name.contains("髓")
        || obj_name.contains("貝")
        || obj_name.contains("砂")
        || obj_name.contains("黏土")
        || obj_name.contains("鹽")
    {
        if obj_name.contains("玉") || obj_name.contains("晶") || obj_name.contains("髓") {
            "harvest_crystal"
        } else if obj_name.contains("鐵") || obj_name.contains("銅") || obj_name.contains("礦") || obj_name.contains("煤") {
            "harvest_ore"
        } else {
            "harvest_hardstone"
        }
    } else if obj_id.starts_with("water_source") || obj_name.contains("水") {
        "fresh_water"
    } else if obj_id.starts_with("grain") || obj_name.contains("稻") {
        "grain_rice"
    } else if obj_id.starts_with("wheat") || obj_name.contains("麥") {
        "grain_wheat"
    } else {
        "harvest_herb"
    };
    let _ = db::add_to_inventory(pid, item_id, 1);
    let now_unix = crate::game::now_unix();
    let (_extra_qty, harvest_note) = crate::game::object_ecology::harvest_from_world_object(
        pid,
        &obj.id,
        &obj.name,
        item_id,
        now_unix,
    );

    let actions = if remaining_qty > 0 {
        vec!["Look".into(), "Gather".into()]
    } else {
        vec!["Look".into()]
    };

    let mut narrative = format!("你俯身細心採擷，將一份「{}」收入行囊之中。", obj.name);
    if !harvest_note.is_empty() {
        narrative.push(' ');
        narrative.push_str(&harvest_note);
    }

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions,
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });

    push_refresh(conn, pid);
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
    true
}

fn try_runtime_resource_action(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now_unix: i64,
) -> bool {
    if !obj.id.starts_with("resource:") || action != "Gather" {
        return false;
    }
    let (_, _, _, game_day) = game::game_time_now(
        now_unix,
        conn.cfg.game_time_epoch_unix,
        conn.cfg.game_time_scale,
    );
    let Some(pool) = crate::pg::pool() else {
        conn.send_error("世界物質主資料暫時不可用");
        return true;
    };
    match crate::pg::world_resource::gather(
        &pool,
        pid,
        player_room,
        PLAYER_GATHER_BATCH,
        game_day,
        now_unix,
        "player_gather",
    ) {
        Ok(Some(gathered)) => {
            if let Some(store) = crate::store::get_store()
                && let Err(e) = store
                    .write()
                    .mirror_inventory_from_authority(pid, &gathered.inventory)
            {
                tracing::warn!("resource gather cache mirror: {e}");
            }
            let _ = event::append(
                now_unix,
                pid,
                types::RESOURCE_GATHER,
                &format!("{}|{}|{}", player_room, gathered.item_id, gathered.quantity),
            );
            let narrative = format_runtime_gather_batch_narrative(&gathered);
            let actions = if gathered.source_stock_after <= 0 {
                vec!["Look".into()]
            } else {
                vec!["Look".into(), "Gather".into()]
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions,
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        }
        Ok(None) => {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative: "眼前能取用的部分已空；它不會憑空再出現。".into(),
                success: false,
                actions: vec!["Look".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(e) => {
            tracing::warn!("runtime resource gather failed: {e}");
            conn.send_error("這次沒有取到東西");
        }
    }
    true
}

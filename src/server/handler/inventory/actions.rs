pub(crate) fn push_refresh(conn: &WsConnection, player_id: &str) {
    let Ok(Some(ent)) = db::get_entity(player_id) else {
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
    conn.send_json(&entity_status_from_character(&ent, true));
}

pub(super) fn handle_get_entity_status(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let mut entity_id = msg.entity_id.clone();
    if entity_id.is_empty() {
        entity_id.clone_from(&pid);
    }
    let Ok(Some(ent)) = db::get_entity(&entity_id) else {
        conn.send_error(gametext::client("status_entity_not_found"));
        return;
    };
    let is_self = entity_id == pid;
    conn.send_json(&entity_status_from_character(&ent, is_self));
}

pub(super) fn handle_get_inventory(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    conn.send_json(&inventory_msg_for_character(&ent));
}

pub(super) fn handle_equip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Some(def) = get_item_def(&msg.item_id) else {
        conn.send_error(gametext::client("inv_item_not_found"));
        return;
    };
    if def.slot.is_empty()
        || msg.item_id.contains("storage_box")
        || msg.item_id.contains("pine_soot_ink")
        || msg.item_id.contains("tool_handle")
        || def.name.contains("隨身匣")
        || def.name.contains("收納匣")
        || def.name.contains("墨錠")
        || def.name.contains("工具長柄")
    {
        conn.send_error(gametext::client("inv_cannot_equip"));
        return;
    }
    let mut target_slot = match def.slot.as_str() {
        "hand" | "weapon" => "hold_r".to_string(),
        "shield" | "tool" => "hold_l".to_string(),
        "accessory" => "trinket".to_string(),
        "helmet" => "head".to_string(),
        "boots" => "feet".to_string(),
        "armor" => "body".to_string(),
        "hold" => {
            if msg.target_slot == "hold_l" || msg.target_slot == "hold_r" {
                msg.target_slot.clone()
            } else {
                "hold_r".to_string()
            }
        }
        other => other.to_string(),
    };
    if (target_slot == "hold_r" || target_slot == "hold_l")
        && (msg.target_slot == "hold_l" || msg.target_slot == "hold_r")
        && (msg.target_slot == "hold_l"
            || crate::pg::living_world::is_weapon_or_tool_item(&msg.item_id, &def.name))
    {
        target_slot.clone_from(&msg.target_slot);
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    let old_item_id = current_slots.get(&target_slot).cloned().unwrap_or_default();
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    if update_equipment_slot(&pid, &target_slot, &msg.item_id).is_err() {
        conn.send_error(gametext::client("inv_equip_fail"));
        return;
    }
    if !old_item_id.is_empty() && add_to_inventory(&pid, &old_item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_unequip_old_fail"));
        return;
    }
    if (target_slot == "hold_r" || target_slot == "hold_l")
        && crate::pg::living_world::is_weapon_or_tool_item(&msg.item_id, &def.name)
    {
        let vstate = crate::pg::living_world::get_vessel_state(&pid, &msg.item_id);
        if vstate.use_count == 0 {
            let _ = crate::pg::living_world::record_vessel_use(
                &pid,
                &msg.item_id,
                0,
                1,
                crate::game::now_unix(),
            );
        }
    }
    push_refresh(conn, &pid);
    let item_name = format_entity_item_with_inscription(&pid, &msg.item_id, &def.name);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "equip_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name.clone(),
        narrative: format!("你穿戴並佩掛起【{item_name}】。"),
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_unequip_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.slot.is_empty() {
        conn.send_error(gametext::client("unequip_no_slot"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let current_slots: HashMap<String, String> = if ent.equipment_slots.is_empty() {
        HashMap::new()
    } else {
        serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
    };
    if current_slots.is_empty() {
        conn.send_error(gametext::client("unequip_nothing"));
        return;
    }
    let item_id = current_slots.get(&msg.slot).cloned().unwrap_or_default();
    if item_id.is_empty() {
        conn.send_error(gametext::client("unequip_slot_empty"));
        return;
    }
    let weight = get_item_def(&item_id).map(|d| d.weight).unwrap_or(0.0);
    let cur_w = inventory_weight(&ent.inventory);
    let max_w = ent.vit as f64 * 10.0;
    if cur_w + weight > max_w {
        conn.send_error(gametext::client("unequip_bag_full"));
        return;
    }
    if clear_equipment_slot(&pid, &msg.slot).is_err() {
        conn.send_error(gametext::client("unequip_slot_fail"));
        return;
    }
    if add_to_inventory(&pid, &item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    push_refresh(conn, &pid);
}

pub(super) fn handle_drop_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let inv = get_inventory(&ent.inventory, ent.vit);
    let owned_qty = inv
        .items
        .iter()
        .find(|it| it.item_id == msg.item_id)
        .map(|it| it.qty)
        .unwrap_or(1)
        .max(1);
    let drop_qty = if msg.action == "all" {
        owned_qty
    } else {
        1
    };
    let item_name = resolve_inv_item_name(&msg.item_id);
    if remove_from_inventory(&pid, &msg.item_id, drop_qty).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    push_refresh(conn, &pid);
    let narrative = if drop_qty > 1 {
        format!("你從行囊中取出 {drop_qty} 份【{item_name}】，隨手將其捨棄在路旁。")
    } else {
        format!("你從行囊中取出一份【{item_name}】，隨手將其捨棄在路旁。")
    };
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "drop_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}


pub(super) fn handle_sell_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if !inventory_has_item(&ent.inventory, &msg.item_id) {
        conn.send_error(gametext::client("inv_bag_missing"));
        return;
    }
    let inv = get_inventory(&ent.inventory, ent.vit);
    let owned_qty = inv
        .items
        .iter()
        .find(|it| it.item_id == msg.item_id)
        .map(|it| it.qty)
        .unwrap_or(1)
        .max(1);
    let sell_qty = if msg.action == "all" {
        owned_qty
    } else {
        1
    };

    let item_name = resolve_inv_item_name(&msg.item_id);

    let base_unit_price = crate::combat::wordplate::parse_crystal_item_id(&msg.item_id)
        .map(|(_, q)| (q.magnesium_value() + 5).max(3))
        .or_else(|| crate::world::materials::trade_price_anchor_mg(&msg.item_id))
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(&msg.item_id)
                .map(crate::game::peninsula_catalog::catalog_entry_price_mg)
        })
        .or_else(|| {
            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                if fg.id == msg.item_id {
                    return Some(fg.base_price);
                }
            }
            get_item_def(&msg.item_id).map(|d| (d.weight * 15.0).round() as i32)
        })
        .unwrap_or(3)
        .max(3);

    let raw_room = db::get_entity_room(&pid).unwrap_or_default();
    let player_room = if raw_room.starts_with("grid:") {
        let inf = crate::world::materials::infer_origin_settlement(&raw_room);
        if !inf.is_empty() {
            inf.to_string()
        } else {
            db::get_room_name(&raw_room).unwrap_or_else(|_| raw_room.clone())
        }
    } else {
        db::get_room_name(&raw_room).unwrap_or_else(|_| {
            let inf = crate::world::materials::infer_origin_settlement(&raw_room);
            if !inf.is_empty() {
                inf.to_string()
            } else if !raw_room.is_empty() {
                raw_room.clone()
            } else {
                "文曜".to_string()
            }
        })
    };
    let player_room = if player_room.trim().is_empty() {
        "文曜".to_string()
    } else {
        player_room
    };
    let initial_quote = crate::game::object_ecology::dynamic_settlement_market_quote(
        &player_room,
        &msg.item_id,
        base_unit_price,
    );
    if remove_from_inventory(&pid, &msg.item_id, sell_qty).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    let mut payout = 0i32;
    for _ in 0..sell_qty {
        let cur = crate::game::object_ecology::dynamic_settlement_market_quote(
            &player_room,
            &msg.item_id,
            base_unit_price,
        );
        payout = payout.saturating_add(cur.unit_price);
        let _ = crate::game::object_ecology::record_settlement_market_trade(
            &player_room,
            &msg.item_id,
            base_unit_price,
            1,
            "玩家售出",
        );
    }
    if db::add_magnesium(&pid, payout).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }

    push_refresh(conn, &pid);

    let narrative = format!(
        "你將 {sell_qty} 份【{item_name}】交予本地市集商鋪驗收（{}，實得 {payout} 鎂，款貨兩訖）。",
        initial_quote.market_narrative,
    );
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "sell_item".into(),
        target_id: msg.item_id.clone(),
        target_name: item_name,
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

pub(super) fn handle_craft_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    if let Some(zhulin_res) = crate::grid::zhulin::try_craft_zhulin_item(&pid, &msg.item_id) {
        match zhulin_res {
            Ok((out_id, out_name, narrative)) => {
                push_refresh(conn, &pid);
                let cur_room = db::get_entity_room(&pid).unwrap_or_default();
                if !cur_room.is_empty() {
                    crate::server::broadcast::refresh_room_views_for_room(&conn.sessions, &conn.cfg, &cur_room);
                }
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: "craft_item".into(),
                    target_id: out_id,
                    target_name: out_name,
                    narrative,
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
            }
            Err(err) => {
                conn.send_error(err);
            }
        }
        return;
    }
    let (_, room_name) = entity_location_label(&ent);
    let now = crate::game::now_unix();

    match crate::game::object_ecology::craft_from_raw_material(&pid, &msg.item_id, &room_name, now) {
        Ok((item_id, item_name, _is_mutated, mut narrative)) => {
            let nat_skills = synthesize_entity_natural_skills(&ent);
            if let Some(forge_sk) = nat_skills.iter().find(|s| s.domain == "production" && s.category_label.contains("百工")) {
                let is_culinary = item_id.contains("shrimp")
                    || item_id.contains("fish")
                    || item_id.contains("jerky")
                    || item_id.contains("grain")
                    || item_id.contains("tea");
                if is_culinary {
                    narrative.push_str(&format!(
                        " 你順著平日熟稔的《{}》手法掌控火候收汁，滋味愈發醇厚鮮美。",
                        forge_sk.name
                    ));
                } else {
                    narrative.push_str(&format!(
                        " 你順著平日熟稔的《{}》手法落手收勢，器物肌理更顯勻稱。",
                        forge_sk.name
                    ));
                }
            }
            push_refresh(conn, &pid);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "craft_item".into(),
                target_id: item_id,
                target_name: item_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(err) => {
            conn.send_error(err);
        }
    }
}

pub(super) fn handle_mutate_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let (_, room_name) = entity_location_label(&ent);
    let now = crate::game::now_unix();

    match crate::game::object_ecology::mutate_finished_item(&pid, &msg.item_id, &room_name, now) {
        Ok((mut_id, mut_name, mut narrative)) => {
            let nat_skills = synthesize_entity_natural_skills(&ent);
            if let Some(forge_sk) = nat_skills.iter().find(|s| s.domain == "production" && s.category_label.contains("百工")) {
                narrative.push_str(&format!(
                    " 你循著《{}》的呼吸節奏導引火候，器表靈韻隱隱流轉。",
                    forge_sk.name
                ));
            }
            push_refresh(conn, &pid);
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: "mutate_item".into(),
                target_id: mut_id,
                target_name: mut_name,
                narrative,
                success: true,
                actions: vec![],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
        }
        Err(err) => {
            conn.send_error(err);
        }
    }
}

pub(super) fn handle_print_topology_debug(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let Ok(Some(ent)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("inv_entity_not_found"));
        return;
    };
    let seed = match ent.soul_seed {
        Some(s) if s != 0 => s,
        _ => {
            tracing::info!(target: "topology_debug", "角色無 soul_seed（可能為舊資料）");
            conn.send_error(gametext::client("seed_no_soul"));
            return;
        }
    };
    let costs = expand_soul_seed_to_topology_costs(seed);
    let sum: f64 = costs.iter().sum();
    tracing::info!(target: "topology_debug", "========== 361 拓撲除錯（當前角色） ==========");
    tracing::info!(target: "topology_debug", soul_seed = seed, "SoulSeed (int64)");
    tracing::info!(target: "topology_debug", "N000（生之本源）→ 前三條主星軌阻力：");
    if costs.len() >= 3 {
        tracing::info!(target: "topology_debug", c0 = costs[0], "N000 → N001");
        tracing::info!(target: "topology_debug", c1 = costs[1], "N000 → N002");
        tracing::info!(target: "topology_debug", c2 = costs[2], "N000 → N003");
    }
    tracing::info!(target: "topology_debug", sum, "全 760 條連線 Cost 總和（規格常數應為 10000）");
    tracing::info!(target: "topology_debug", "=============================================");
    conn.send_json(&TopologyDebugAckMsg {
        msg_type: "topology_debug".into(),
        message: "已於伺服器終端印出".into(),
    });
}

// ─── 詞匣與四戰術序位系統 ─────────────────────────────────────────────


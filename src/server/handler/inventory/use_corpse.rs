fn handle_use_corpse_action(conn: &WsConnection, msg: &ClientMsg, pid: &str) -> bool {
    // ─── 肢體分解（ButcherCorpse 或殘軀之 StripNameWhole）────────────────────
    if msg.action == "ButcherCorpse" || msg.action == "StripNameWhole" {
        if !msg.item_id.starts_with("corpse_") {
            conn.send_error("此物已非完整殘軀，不可再次分解。");
            return true;
        }

        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return true;
        };

        if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
            conn.send_error(gametext::client("inv_bag_fail"));
            return true;
        }

        let corpse_rec = crate::pg::living_world::get_beast_corpse(&msg.item_id);
        crate::pg::living_world::consume_beast_corpse(&msg.item_id);

        let primary_loot_id = corpse_rec
            .as_ref()
            .map(|r| r.primary_loot_id.as_str())
            .unwrap_or("mat_leather_strip");
        let primary_loot_name = corpse_rec
            .as_ref()
            .map(|r| r.primary_loot_name.as_str())
            .unwrap_or("異獸毛皮");
        let creature_title = corpse_rec
            .as_ref()
            .map(|r| r.creature_title.as_str())
            .unwrap_or("異獸");

        let _ = db::add_to_inventory(&pid, primary_loot_id, 1);
        let _ = db::add_to_inventory(&pid, "wt_beast_bone", 1);
        push_refresh(conn, &pid);
        crate::pg::living_world::record_entity_anatomy_familiarity(&pid, creature_title, 1);

        let now = crate::game::now_unix();
        let habit_rec = crate::pg::living_world::record_breath_activity(
            &pid,
            crate::pg::living_world::BreathHabitKind::ForestLong,
            2,
            now,
        );
        let equipment_map: HashMap<String, String> = if ent.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&ent.equipment_slots).unwrap_or_default()
        };
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut bladed_tool: Option<(String, String)> = None;
        for key in &held_keys {
            if let Some(held_id) = equipment_map.get(*key).filter(|id| !id.is_empty()) {
                let held_name = resolve_inv_item_name(held_id);
                if crate::pg::living_world::is_bladed_vessel_item(held_id, &held_name) {
                    bladed_tool = Some((held_id.clone(), held_name));
                    break;
                }
            }
        }
        if bladed_tool.is_none() {
            let inv_check = get_inventory(&ent.inventory, ent.vit);
            for it in inv_check.items {
                if crate::pg::living_world::is_bladed_vessel_item(&it.item_id, &it.name) {
                    bladed_tool = Some((it.item_id, it.name));
                    break;
                }
            }
        }
        let vstate = bladed_tool.as_ref().map(|(kid, _)| {
            crate::pg::living_world::record_vessel_use(&pid, kid, 1, 2, now)
        });

        let mut narrative = if let (Some((_, kname)), Some(vs)) = (&bladed_tool, &vstate) {
            format!(
                "你操持【{}】（{}），順著肌理筋絡細細剖解【{}殘軀】。{}。手中刃鋒俐落剔下品相完好的「{}」，並收納下一批堅實的獸骨。",
                kname,
                vs.status_badge(),
                creature_title,
                vs.familiarity().feel_prose(),
                primary_loot_name
            )
        } else {
            format!(
                "你徒手順著肌理筋絡細細分理【{}殘軀】。氣勁沉穩，俐落剔下品相完好的「{}」，並收納下一批堅實的獸骨。",
                creature_title,
                primary_loot_name
            )
        };
        let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
        let anatomy_fam = crate::pg::living_world::get_entity_anatomy_familiarity_total(&pid);
        let seam = crate::combat::human_phase::evaluate_seam_threading(
            &bone,
            CarrierAffinity::Blade,
            ent.dex as f64,
            45.0,
            anatomy_fam,
        );
        if seam.xi_index >= 0.40 {
            narrative.push_str("你順著生靈骨節肌理之天然空隙游刃穿入，刀鋒避開堅硬外皮，毫不費力便將其完整剖解，器物鋒刃幾無損耗。");
        }

        let familiar_enough = vstate.as_ref().map(|vs| matches!(
            vs.familiarity(),
            crate::pg::living_world::VesselFamiliarity::WellBalanced
                | crate::pg::living_world::VesselFamiliarity::LikeOwnArm
        )).unwrap_or(false);

        if familiar_enough {
            let actor_name = if !ent.display_title.trim().is_empty() {
                ent.display_title.trim().to_string()
            } else {
                "獵手".to_string()
            };
            let tool_name = bladed_tool.as_ref().map(|(_, n)| n.clone()).unwrap_or_else(|| "徒手".to_string());
            let tool_familiarity = vstate.as_ref().map(|vs| vs.familiarity().label().to_string()).unwrap_or_else(|| "得心應手".to_string());
            let packet = crate::ai::CraftActionFactPacket {
                actor_name,
                tool_name,
                tool_familiarity,
                breath_habit: habit_rec.dominant.label().to_string(),
                material_names: vec![format!("{}殘軀", creature_title)],
                output_name: primary_loot_name.to_string(),
                highlight_kind: "殘軀分解".to_string(),
                environment_hint: "荒澤獵場".to_string(),
            };
            let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
            if !highlight.is_empty() {
                narrative.push_str("\n\n");
                narrative.push_str(&highlight);
            }
        }

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "ButcherCorpse".into(),
            target_id: msg.item_id.clone(),
            target_name: format!("{}殘軀", creature_title),
            narrative,
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }


    let Ok(Some(ent)) = db::get_entity(pid) else {
        return false;
    };
    if matches!(
        msg.action.as_str(),
        "MakeFire" | "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle" | "AddFuel" | "RepairStructure"
    ) || (msg.action.is_empty()
        && matches!(
            msg.item_id.as_str(),
            "mat_kindling_bundle" | "mat_timber_log" | "harvest_wood" | "mat_charcoal_sack"
        ))
    {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();

        let res = match msg.action.as_str() {
            "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle" => {
                crate::game::object_ecology::refine_wood_multi_socket(&pid, &msg.action, &ent_room, now)
                    .map(|(_id, _name, narr)| narr)
            }
            "AddFuel" => {
                crate::game::object_ecology::stoke_workshop_furnace(&pid, &ent_room, now)
            }
            "RepairStructure" => {
                let target_name = resolve_inv_item_name(&msg.item_id);
                crate::game::object_ecology::repair_room_structure(&pid, &ent_room, &target_name, now)
            }
            _ => {
                if crate::game::object_ecology::get_active_campfire(&ent_room).is_none() {
                    let result = crate::game::object_ecology::ignite_or_stoke_campfire(
                        &pid,
                        &ent_room,
                        Some(&msg.item_id),
                        now,
                    );
                    if result.is_ok() && msg.item_id == "mat_timber_log" {
                        let _ = db::add_to_inventory(&pid, "mat_kindling_bundle", 1);
                    }
                    result.map(|(narr, _is_new)| narr)
                } else {
                    let mode = if msg.item_id == "mat_timber_log" {
                        "RefinePlank"
                    } else {
                        "CraftTorch"
                    };
                    crate::game::object_ecology::refine_wood_multi_socket(&pid, mode, &ent_room, now)
                        .map(|(_id, _name, narr)| narr)
                }
            }
        };

        match res {
            Ok(narrative) => {
                push_refresh(conn, &pid);
                crate::server::broadcast::refresh_room_views_for_room(
                    &conn.sessions,
                    &conn.cfg,
                    &ent_room,
                );
                let item_name = resolve_inv_item_name(&msg.item_id);

                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { "use_item".into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "MakeFire".into(),
                        "SplitWood".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "RepairStructure".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "WashOre" | "SmeltBloom" | "ForgeRepairStrip" | "SharpenBlade" | "BuildFieldAnvil"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "WashOre" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_mineral_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                if act == "BuildFieldAnvil" {
                    crate::server::broadcast::refresh_room_views_for_room(
                        &conn.sessions,
                        &conn.cfg,
                        &ent_room,
                    );
                }
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "WashOre".into(),
                        "SmeltBloom".into(),
                        "ForgeRepairStrip".into(),
                        "SharpenBlade".into(),
                        "BuildFieldAnvil".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "WashHerb" | "DryHerb" | "CrushHerb" | "BoilDecoction" | "SetupHerbStove"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "WashHerb" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_herb_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                if act == "SetupHerbStove" {
                    crate::server::broadcast::refresh_room_views_for_room(
                        &conn.sessions,
                        &conn.cfg,
                        &ent_room,
                    );
                }
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "WashHerb".into(),
                        "DryHerb".into(),
                        "CrushHerb".into(),
                        "BoilDecoction".into(),
                        "SetupHerbStove".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "TwistRope" | "WeaveSackcloth" | "OilCanvas" | "PressPaper"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "TwistRope" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_fiber_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "TwistRope".into(),
                        "WeaveSackcloth".into(),
                        "OilCanvas".into(),
                        "PressPaper".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    } else if matches!(
        msg.action.as_str(),
        "ScrapeHide" | "BoilFishGlue" | "CalcifyShell" | "CobbleBoots"
    ) {
        let (room_id, room_name) = entity_location_label(&ent);
        let ent_room = if !room_name.is_empty() { room_name } else { room_id };
        let now = crate::game::now_unix();
        let act = if msg.action.is_empty() { "ScrapeHide" } else { msg.action.as_str() };
        let res = crate::game::object_ecology::refine_hide_fish_multi_socket(&pid, act, &ent_room, now);
        match res {
            Ok((_id, _name, narrative)) => {
                push_refresh(conn, &pid);
                let item_name = resolve_inv_item_name(&msg.item_id);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: if msg.action.is_empty() { act.into() } else { msg.action.clone() },
                    target_id: msg.item_id.clone(),
                    target_name: item_name,
                    narrative,
                    success: true,
                    actions: vec![
                        "ScrapeHide".into(),
                        "BoilFishGlue".into(),
                        "CalcifyShell".into(),
                        "CobbleBoots".into(),
                    ],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    }
    false
}

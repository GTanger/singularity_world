fn try_handle_special_object_action(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
    now: i64,
) -> bool {
    if action == "MakeFire" {
        match crate::game::object_ecology::ignite_or_stoke_campfire(pid, player_room, None, now) {
            Ok((msg, _is_new)) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative: msg,
                    success: true,
                    actions: vec![
                        "WarmHands".into(),
                        "RoastFood".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "CollectCharcoal".into(),
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
    if obj.id == "dyn_campfire" && matches!(action, "WarmHands" | "RoastFood" | "CollectCharcoal") {
        match crate::game::object_ecology::interact_with_campfire(pid, player_room, action, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                let actions = if action == "CollectCharcoal" {
                    vec!["Look".into(), "MakeFire".into(), "Gather".into()]
                } else {
                    vec![
                        "WarmHands".into(),
                        "RoastFood".into(),
                        "CraftTorch".into(),
                        "AddFuel".into(),
                        "CollectCharcoal".into(),
                    ]
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
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    }
    if obj.id == "dyn_field_anvil" || obj.id == "dyn_herb_stove" {
        match crate::game::object_ecology::interact_with_field_facility(pid, player_room, &obj.id, action, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                let actions = if obj.id == "dyn_field_anvil" {
                    vec!["Look".into(), "ForgeRepairStrip".into(), "SharpenBlade".into()]
                } else {
                    vec!["Look".into(), "BoilDecoction".into(), "DryHerb".into(), "CrushHerb".into()]
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
                return true;
            }
            Err(e) => {
                conn.send_error(e);
                return true;
            }
        }
    }
    if obj.id == "wt_multi_firewood_pile" && action == "Gather" {
        let _ = db::add_to_inventory(pid, "mat_timber_log", 1);
        let _ = db::add_to_inventory(pid, "mat_kindling_bundle", 2);
        push_refresh(conn, pid);
        let narrative = "你走至簷角木料垛前細緻翻檢，從中挑選並抽取出一根質地堅實的「伐採原木」與兩束乾燥易燃的「乾燥柴枝束」妥善收入行囊。".to_string();
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success: true,
            actions: vec![
                "MakeFire".into(),
                "SplitWood".into(),
                "RefinePlank".into(),
                "BurnCharcoal".into(),
                "CraftTorch".into(),
                "RepairStructure".into(),
            ],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }
    if action == "AddFuel" {
        if obj.id == "dyn_campfire" || obj.name.contains("營火") {
            match crate::game::object_ecology::interact_with_campfire(pid, player_room, "AddFuel", now) {
                Ok(narrative) => {
                    push_refresh(conn, pid);
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: action.into(),
                        target_id: obj.id.clone(),
                        target_name: obj.name.clone(),
                        narrative,
                        success: true,
                        actions: vec![
                            "WarmHands".into(),
                            "RoastFood".into(),
                            "CraftTorch".into(),
                            "AddFuel".into(),
                            "CollectCharcoal".into(),
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
        } else {
            match crate::game::object_ecology::stoke_workshop_furnace(pid, player_room, now) {
                Ok(narrative) => {
                    push_refresh(conn, pid);
                    conn.send_json(&ActionResultMsg {
                        msg_type: "action_result".into(),
                        action: action.into(),
                        target_id: obj.id.clone(),
                        target_name: obj.name.clone(),
                        narrative,
                        success: true,
                        actions: vec![
                            "Operate".into(),
                            "AddFuel".into(),
                            "RefinePlank".into(),
                            "BurnCharcoal".into(),
                            "CraftHandle".into(),
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
        }
    }
    if action == "RepairStructure" {
        match crate::game::object_ecology::repair_room_structure(pid, player_room, &obj.name, now) {
            Ok(narrative) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "Look".into(),
                        "MakeFire".into(),
                        "SplitWood".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
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
    }
    if matches!(
        action,
        "SplitWood" | "RefinePlank" | "BurnCharcoal" | "CraftTorch" | "CraftHandle"
    ) {
        let res = if obj.id == "dyn_campfire" && action == "CraftTorch" {
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
            crate::game::object_ecology::interact_with_campfire(pid, player_room, "CraftTorch", now)
                .map(|narr| ("craft_pine_torch".to_string(), "浸脂松明火把".to_string(), narr))
        } else {
            crate::game::object_ecology::refine_wood_multi_socket(pid, action, player_room, now)
        };
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![
                        "MakeFire".into(),
                        "AddFuel".into(),
                        "RefinePlank".into(),
                        "BurnCharcoal".into(),
                        "CraftTorch".into(),
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
    }
    if matches!(
        action,
        "WashOre" | "SmeltBloom" | "ForgeRepairStrip" | "SharpenBlade" | "BuildFieldAnvil"
    ) {
        let res = crate::game::object_ecology::refine_mineral_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                if action == "BuildFieldAnvil" {
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                }
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
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
    }
    if matches!(
        action,
        "WashHerb" | "DryHerb" | "CrushHerb" | "BoilDecoction" | "SetupHerbStove"
    ) {
        let res = crate::game::object_ecology::refine_herb_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                if action == "SetupHerbStove" {
                    refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                }
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
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
    }
    if matches!(
        action,
        "TwistRope" | "WeaveSackcloth" | "OilCanvas" | "PressPaper"
    ) {
        let res = crate::game::object_ecology::refine_fiber_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
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
    }
    if matches!(
        action,
        "ScrapeHide" | "BoilFishGlue" | "CalcifyShell" | "CobbleBoots"
    ) {
        let res = crate::game::object_ecology::refine_hide_fish_multi_socket(pid, action, player_room, now);
        match res {
            Ok((_prod_id, _prod_name, narrative)) => {
                push_refresh(conn, pid);
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.into(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
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

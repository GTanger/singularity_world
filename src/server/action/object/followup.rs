fn recent_scene_object_action_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_ACTION)
        .map(|row| row.payload)
        .collect()
}

fn recent_scene_object_state_payloads(pid: &str, now: i64) -> Vec<String> {
    event::events_in_range(pid, now.saturating_sub(6 * 60 * 60), now)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.event_type == types::SCENE_OBJECT_STATE)
        .map(|row| row.payload)
        .collect()
}

fn object_followup_buttons(
    obj: &crate::model::RoomObject,
    player_room: &str,
    action: &str,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) -> (Vec<String>, String) {
    let mut others: Vec<String> = obj
        .sockets
        .iter()
        .filter(|s| *s != action)
        .cloned()
        .collect();
    m3_020_filter_scene_object_buttons(
        &obj.id,
        &mut others,
        prior_scene_actions,
        prior_scene_states,
    );
    m3_027_filter_scene_object_buttons(&obj.id, &mut others, prior_scene_states);
    if !obj.id.starts_with("m3_")
        && (obj.id == "wt_guild_commission_board"
            || obj.id == "wt_hall_bulletin_board"
            || obj.id == "wt_cat_0794"
            || obj.name.contains("委託")
            || obj.name.contains("告示")
            || obj.name.contains("公佈欄"))
    {
        let mut custom = vec![
            "Look".to_string(),
            "Read".to_string(),
            "Use".to_string(),
            "Deliver".to_string(),
        ];
        if obj.id == "wt_hall_bulletin_board" || obj.name.contains("民務告示") {
            custom.push("DeliberateCivicPact".to_string());
        }
        let now_unix = crate::game::now_unix();
        let active_orders = crate::npc::causal_core::list_active_stranded_cargo_orders(now_unix);
        if !active_orders.is_empty() {
            custom.push("ClaimStrandedCargo".to_string());
            custom.push("DeliverStrandedCargo".to_string());
        }
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_harbor_insurance_desk" || obj.name.contains("海事承保") || obj.name.contains("海事保單") {
        let custom = vec!["Look".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_harbor_clockwise_ship" || obj.name.contains("順時針環島海船") || obj.name.contains("順時針環島海運商船") || obj.name.contains("遠洋順流商船") {
        let custom = vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_field_crop_plot" || obj.name.contains("田壟作物") || obj.name.contains("當季田壟") {
        let custom = vec!["Look".to_string(), "Harvest".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_home_bamboo_mailbox" || obj.name.contains("竹編信箱") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "Open".to_string(), "DispatchCourier".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_courier_post_desk" || obj.name.contains("信使櫃檯") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "DispatchCourier".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_cat_0790" || obj.id == "wt_bank_ledger_drawer" || obj.name.contains("分格存摺櫃") || obj.name.contains("存摺") {
        let custom = vec!["Look".to_string(), "Read".to_string(), "DepositBank".to_string(), "WithdrawBank".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_home_jade_cabinet" || obj.name.contains("冰鑑櫃") || obj.name.contains("寒水玉保鮮石櫃") || obj.name.contains("寒水玉雙層保鮮石櫃") {
        let custom = vec!["Look".to_string(), "Open".to_string(), "StoreStash".to_string(), "RetrieveStash".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_coastal_angling_spot" || obj.id == "wt_cat_1006" || obj.name.contains("垂釣") || obj.name.contains("釣竿") {
        let custom = vec!["Look".to_string(), "AngleFish".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_lookout_telescope" || obj.id == "wt_cat_0733" || obj.name.contains("望遠鏡") {
        let custom = vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_field_camp_stove" || obj.id == "wt_cat_1010" || obj.name.contains("便攜爐") {
        let custom = vec!["Look".to_string(), "Cook".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_mill_") || obj.name.contains("水輪玄武磨") || obj.name.contains("篩粉斗") || obj.name.contains("儲糧斗") {
        let custom = vec!["Look".to_string(), "MillGrain".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_sawmill_") || obj.name.contains("排鋸架") || obj.name.contains("檢寸台") {
        let custom = vec!["Look".to_string(), "RefinePlank".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_stonecut_") || obj.name.contains("拉鋸") || obj.name.contains("寒水玉料架") || obj.name.contains("琢石盆") {
        let custom = vec!["Look".to_string(), "CutJade".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_cheese_") || obj.id == "wt_wool_spinning_wheel" || obj.id == "wt_farm_tool_iron_plow" || obj.name.contains("壓酪架") || obj.name.contains("熟成架") || obj.name.contains("精鐵犁") || obj.name.contains("手搖木紡車") {
        let custom = vec!["Look".to_string(), "PressCheese".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_mine_") || obj.id.starts_with("wt_trolley_") || obj.name.contains("洗礦槽") || obj.name.contains("重鐵篩") || obj.name.contains("重台車") || obj.name.contains("絞盤") {
        let custom = vec!["Look".to_string(), "HaulMineCart".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_smokehouse_") || obj.name.contains("紅陶灶") || obj.name.contains("冷燻") {
        let custom = vec!["Look".to_string(), "SmokeFish".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_lime_") || obj.id.starts_with("wt_kelp_") || obj.id == "wt_shipyard_dry_dock" || obj.name.contains("灰窯") || obj.name.contains("曬藻棚") || obj.name.contains("搓繩輪") || obj.name.contains("龍骨架") {
        let custom = vec!["Look".to_string(), "BurnLime".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id.starts_with("wt_hunt_") || obj.id == "wt_clinic_herb_cabinet" || obj.name.contains("破甲弩") || obj.name.contains("鋼獸夾") || obj.name.contains("百草丸藥櫃") {
        let custom = vec!["Look".to_string(), "PrepareMiasmaGear".to_string(), "Use".to_string(), "Operate".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_dock_riverbank_raft" || obj.name.contains("平底竹筏") {
        let custom = vec!["Look".to_string(), "RowSwampRaft".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_dining_iron_cauldron" || obj.id == "wt_reg_plains_specialty" || obj.id == "wt_reg_mining_specialty" || obj.name.contains("燉肉釜") || obj.name.contains("陳年米酒甕") || obj.name.contains("烤番薯砂鍋") {
        let custom = vec!["Look".to_string(), "SeasonalBanquet".to_string(), "Taste".to_string(), "Use".to_string(), "AssistKitchen".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if obj.id == "wt_inn_ebony_shell_chess" || obj.name.contains("烏木棋盤") {
        let custom = vec!["Look".to_string(), "PlayChess".to_string(), "Operate".to_string(), "Use".to_string()];
        others = custom.into_iter().filter(|s| s != "Look").collect();
    } else if matches!(
        action,
        "ChartAstrolabe"
            | "TestEngraving"
            | "SparPillar"
            | "SoakHotSpring"
            | "GroomAppearance"
            | "MendTravelGear"
            | "ReadWindVane"
            | "ApplyMoxibustion"
            | "HangVotivePlaque"
            | "TranscribeRareTome"
            | "AttendLecture"
            | "CalibrateClock"
            | "FitMortiseTenon"
            | "QuenchBlade"
            | "DyeIndigoCloth"
            | "FeedDraftBeast"
            | "BoardCoastalShip"
            | "InspectLandRegistry"
            | "AssistKitchen"
            | "RoastMountainTea"
            | "SunBooks"
            | "RoastHerbs"
            | "CollateInscriptions"
            | "GrindOpticLens"
            | "SelectDriedSeafood"
            | "DeliberateCivicPact"
            | "DrawSpringWell"
    ) {
        others = obj.sockets.iter().filter(|s| s.as_str() != "Look").cloned().collect();
    }
    let mut move_target_id = String::new();
    if action == "Look" && others.is_empty() && object_response(obj, "Look").is_empty() {
        let Ok(objs) = db::get_objects_in_room(player_room) else {
            return (others, move_target_id);
        };
        let idx = objs.iter().position(|o| o.id == obj.id);
        if let Some(i) = idx {
            for range in [(i + 1..objs.len()), (0..i)] {
                for j in range {
                    let o = &objs[j];
                    if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                        move_target_id.clone_from(&o.id);
                        break;
                    }
                }
                if !move_target_id.is_empty() {
                    break;
                }
            }
        } else {
            for o in &objs {
                if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                    move_target_id.clone_from(&o.id);
                    break;
                }
            }
        }
        if !move_target_id.is_empty() {
            others = vec!["Move".into()];
        }
    }
    (others, move_target_id)
}

fn m3_020_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_actions: &[String],
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_020_mint_half_open_door" => {
            let can_queue = m3_020_waiting_line_has_been_used(prior_scene_actions)
                && !m3_020_mint_queue_state_active(prior_scene_states);
            if !can_queue {
                actions.retain(|action| action != "Queue");
            }
        }
        "m3_020_hire_short_work_boards" => {
            let can_accept_job = m3_020_short_work_board_has_been_read(prior_scene_actions)
                && !m3_020_short_work_state_active(prior_scene_states);
            if !can_accept_job {
                actions.retain(|action| action != "AcceptJob");
            }
        }
        "m3_020_stone_gate_pass_tags" => {
            let can_join_check = m3_020_pass_tags_have_been_read(prior_scene_actions)
                && !m3_020_pass_check_state_active(prior_scene_states);
            if !can_join_check {
                actions.retain(|action| action != "JoinCheck");
            }
        }
        "m3_020_council_notice_strip" => {
            let can_register_complaint = m3_020_council_notice_has_been_read(prior_scene_actions)
                && !m3_020_council_complaint_state_active(prior_scene_states);
            if !can_register_complaint {
                actions.retain(|action| action != "RegisterComplaint");
            }
        }
        _ => {}
    }
}

fn m3_027_filter_scene_object_buttons(
    object_id: &str,
    actions: &mut Vec<String>,
    prior_scene_states: &[String],
) {
    match object_id {
        "m3_027_wet_bench" if m3_027_bench_rest_state_active(prior_scene_states) => {
            actions.retain(|action| action != "Rest");
        }
        "m3_027_hot_soup_stall" if m3_027_soup_taken_state_active(prior_scene_states) => {
            actions.retain(|action| action != "BuySoup" && action != "QueueBroth");
        }
        "m3_027_cargo_pole" if m3_027_cargo_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "CarryCargo");
        }
        "m3_027_boat_rope" if m3_027_rope_help_state_active(prior_scene_states) => {
            actions.retain(|action| action != "HelpRope");
        }
        "m3_027_river_awning" if m3_027_awning_dry_state_active(prior_scene_states) => {
            actions.retain(|action| action != "DryOff");
        }
        "m3_027_charcoal_brazier" if m3_027_brazier_warm_state_active(prior_scene_states) => {
            actions.retain(|action| action != "WarmHands");
        }
        "m3_027_waiting_line_chat" => {
            if m3_027_queue_talk_state_active(prior_scene_states) {
                actions.retain(|action| action != "TalkQueue");
            }
            if m3_027_queue_ferry_state_active(prior_scene_states) {
                actions.retain(|action| action != "QueueFerry");
            }
        }
        "m3_027_market_reed_buyer" => {
            if m3_027_reed_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellReeds");
            }
            if m3_027_sackcloth_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellSackcloth");
            }
            if m3_027_rope_demand_filled_state_active(prior_scene_states) {
                actions.retain(|action| action != "SellRope");
            }
        }
        "m3_027_plank_boarding_marker" => {
            let can_board = m3_027_queue_ferry_state_active(prior_scene_states)
                && !m3_027_board_ferry_state_active(prior_scene_states);
            if !can_board {
                actions.retain(|action| action != "BoardFerry");
            }
        }
        "m3_027_south_landing_rope" => {
            let can_step_ashore = m3_027_board_ferry_state_active(prior_scene_states)
                && !m3_027_landed_south_state_active(prior_scene_states);
            if !can_step_ashore {
                actions.retain(|action| action != "StepAshore");
            }
        }
        "m3_027_warehouse_tally_board" => {
            let can_check_cargo = m3_027_cargo_help_state_active(prior_scene_states)
                && !m3_027_warehouse_tally_state_active(prior_scene_states);
            if !can_check_cargo {
                actions.retain(|action| action != "CheckCargo");
            }
        }
        _ => {}
    }
}

fn apply_object_move(
    conn: &WsConnection,
    pid: &str,
    _player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    if action != "Move" || obj.move_to_room_id.is_empty() {
        return true;
    }
    let Some(coord) = parse_grid_room_id(&obj.move_to_room_id) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let gh = current_game_hour(&conn.cfg);
    let Ok(view_opt) = game::get_grid_room_view(pid, coord.x, coord.y, gh) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let Some(view) = view_opt else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    if set_entity_grid(pid, coord.x, coord.y).is_err() {
        conn.send_error(gametext::client("move_failed"));
        return false;
    }
    let Some(session) = conn.sessions.get(pid) else {
        conn.send_error(gametext::client("move_failed"));
        return false;
    };
    send_room_view_to_session(&session, &view, pid, &conn.cfg);
    let rid = crate::grid::grid_room_id_from_coord(coord.x, coord.y);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: pid.to_string(),
        room_id: rid.clone(),
        room_name: view.room.name.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&moved) {
        conn.hub.broadcast(bytes);
    }
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, &rid);
    crate::server::handler::emit_presence_reflex_on_arrival(conn, pid, &rid);
    true
}


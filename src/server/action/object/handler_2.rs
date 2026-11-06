pub(super) fn do_object_branch(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
) {
    let Some(obj) = resolve_room_object(player_room, target_id) else {
        conn.send_error(gametext::client("target_not_found"));
        return;
    };
    let allow_rest_meditate = (action == "Rest" || action == "Meditate")
        && (obj.name.contains("床")
            || obj.name.contains("榻")
            || obj.name.contains("席")
            || obj.name.contains("凳")
            || obj.name.contains("椅")
            || obj.name.contains("案")
            || obj.name.contains("蒲團")
            || obj.name.contains("坐墊")
            || obj.name.contains("石階")
            || obj.name.contains("埠階")
            || obj.id.contains("bed")
            || obj.id.contains("bench")
            || obj.id.contains("chair")
            || obj.id.contains("tea")
            || obj.id.contains("clinic"));
    if !object_has_socket(&obj, action) && !allow_rest_meditate {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return;
    }
    let now = game::now_unix();
    if try_handle_special_object_action(conn, pid, player_room, &obj, action, now) {
        return;
    }
    if try_grid_cell_object_action(conn, pid, player_room, &obj, action) {
        return;
    }
    if try_runtime_resource_action(conn, pid, player_room, &obj, action, now) {
        return;
    }
    if try_m3_027_material_gather(conn, pid, player_room, &obj, action, now) {
        return;
    }
    let needs_scene_history = m3_020_scene_object_uses_history(&obj.id, action)
        || m3_027_scene_object_uses_history(&obj.id, action);
    let mut prior_scene_actions = if needs_scene_history {
        recent_scene_object_action_payloads(pid, now)
    } else {
        Vec::new()
    };
    let mut prior_scene_states = if needs_scene_history {
        recent_scene_object_state_payloads(pid, now)
    } else {
        Vec::new()
    };
    if needs_scene_history {
        m3_029::merge_shared_market_history(&mut prior_scene_actions, &mut prior_scene_states, now);
    }
    if matches!(
        (obj.id.as_str(), action),
        ("m3_020_hire_short_work_boards", "AcceptJob")
            | ("m3_020_stone_gate_pass_tags", "JoinCheck")
            | ("m3_020_council_notice_strip", "RegisterComplaint")
    ) {
        let read_action = format!("{}|Read", obj.id);
        if !prior_scene_actions.iter().any(|a| a == &read_action) {
            let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &read_action);
            prior_scene_actions.push(read_action);
        }
    }
    if let Some(denial) = m3_027_scene_object_action_denial(
        pid,
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    ) {
        let (others, move_target_id) = object_followup_buttons(
            &obj,
            player_room,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        );
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: denial.to_string(),
            success: false,
            actions: others,
            move_target_id,
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if obj.id == "m3_027_market_reed_buyer" && action == "SellReeds" {
        let available = player_inventory_item_quantity(pid, M3_027_REED_ITEM_ID);
        let batch_count =
            m3_027_reed_batch_sale_count(available, &prior_scene_actions, &prior_scene_states);
        if batch_count >= 2 {
            for _ in 0..batch_count {
                let _ = event::append(
                    now,
                    pid,
                    types::SCENE_OBJECT_ACTION,
                    "m3_027_market_reed_buyer|SellReeds",
                );
                prior_scene_actions.push("m3_027_market_reed_buyer|SellReeds".to_string());
            }
            let _ = event::append(now, pid, types::SCENE_OBJECT_STATE, M3_027_REED_SOLD_STATE);
            let _ = event::append(
                now,
                pid,
                types::SCENE_OBJECT_STATE,
                M3_027_REED_DEMAND_FILLED_STATE,
            );
            prior_scene_states.push(M3_027_REED_SOLD_STATE.to_string());
            prior_scene_states.push(M3_027_REED_DEMAND_FILLED_STATE.to_string());

            let _ = db::remove_from_inventory(pid, M3_027_REED_ITEM_ID, batch_count as i32);
            let total_pay = (batch_count as i32) * M3_027_REED_SELL_PAY;
            let _ = crate::npc::causal_core::settle_player_with_settlement_treasury(pid, player_room, total_pay);
            push_refresh(conn, pid);

            let (others, move_target_id) = object_followup_buttons(
                &obj,
                player_room,
                action,
                &prior_scene_actions,
                &prior_scene_states,
            );
            let narrative = format!(
                "你將收割整齊的 {} 束蘆草一次性齊齊交到秤邊，秤手熟練撥動秤錘：「正合分量！這是結算給你的 {} 鎂工錢。」",
                batch_count, total_pay
            );
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: others,
                move_target_id,
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }
    }
    let mut narrative = m3_027_scene_object_state_action_response(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_020_scene_object_state_action_response(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    })
    .map(str::to_string)
    .unwrap_or_else(|| object_response(&obj, action));

    let is_street_cart = obj.id == "wt_street_roast_snack_cart"
        || obj.name.contains("炭烤")
        || obj.name.contains("豆花攤")
        || obj.name.contains("茶水攤");
    let gh = current_game_hour(&conn.cfg);
    let is_cart_night_closed = !(6..22).contains(&gh);

    if is_street_cart && is_cart_night_closed {
        match action {
            "Look" => {
                narrative = "夜深休市，街角小推車的炭火早已熄滅收攏，鐵網擦拭乾淨並蓋上了防露帆布。".to_string();
            }
            "Taste" => {
                narrative = "攤車已在深夜收攤熄火，待天亮開市後再來品嚐熱食吧。".to_string();
            }
            "Smell" => {
                narrative = "夜風微涼，熄火的推車旁僅餘一絲淡淡的木炭餘味。".to_string();
            }
            _ => {}
        }
    }

    if narrative.is_empty() {
        let act_label = action_label_zh(action);
        narrative = format!(
            "你對「{}」執行了「{act_label}」，但似乎沒有什麼特別的反應。",
            obj.name
        );
    }
    if action == "Look"
        && let Some(echo) = m3_027_scene_object_look_echo(
            player_room,
            &obj.id,
            now,
            &prior_scene_actions,
            &prior_scene_states,
        )
        .or_else(|| {
            m3_020_scene_object_look_echo(&obj.id, &prior_scene_actions, &prior_scene_states)
                .map(str::to_string)
        })
    {
        narrative.push('\n');
        narrative.push_str(&echo);
    }
    if action == "Look" {
        if let Some(read_text) = obj.responses.get("Read") {
            let read_clean = read_text.trim();
            if !read_clean.is_empty() && !narrative.contains(read_clean) {
                let mut cleaned = read_clean;
                const PREFIXES: &[&str] = &[
                    "你走近告示欄細讀通告，",
                    "你走近公佈欄細閱，",
                    "你細讀櫃檯告示：",
                    "你細讀櫃頂張貼的三水總商行通兌章程，",
                    "你走近石柱研讀鐫刻的方位路引：",
                    "你走近立柱端讀鐫刻的商貿契稅規章與每日各港口船班、車隊發貨時序。",
                    "你細讀帳冊上的公道行情：",
                    "你細閱屋契條款，",
                    "你逐條細讀告示內容，",
                    "你逐行研讀告示板上的急件委託，",
                ];
                for &pfx in PREFIXES {
                    if let Some(rest) = cleaned.strip_prefix(pfx) {
                        cleaned = rest.trim();
                        break;
                    }
                }
                if let Some(rest) = cleaned.strip_prefix("你走近") {
                    if let Some(pos) = rest.find("細閱，") {
                        cleaned = rest[pos + "細閱，".len()..].trim();
                    } else if let Some(pos) = rest.find("細讀，") {
                        cleaned = rest[pos + "細讀，".len()..].trim();
                    } else if let Some(pos) = rest.find("細閱：") {
                        cleaned = rest[pos + "細閱：".len()..].trim();
                    } else if let Some(pos) = rest.find("細讀：") {
                        cleaned = rest[pos + "細讀：".len()..].trim();
                    }
                } else if let Some(rest) = cleaned.strip_prefix("你細讀") {
                    if let Some(pos) = rest.find('：') {
                        cleaned = rest[pos + '：'.len_utf8()..].trim();
                    }
                } else if let Some(rest) = cleaned.strip_prefix("你細閱") {
                    if let Some(pos) = rest.find('：') {
                        cleaned = rest[pos + '：'.len_utf8()..].trim();
                    }
                }
                if !narrative.contains(cleaned) {
                    if !narrative.ends_with('。') && !narrative.ends_with('！') && !narrative.ends_with('？') {
                        narrative.push('。');
                    }
                    narrative.push_str(cleaned);
                }
            }
        }
    }
    if obj.id.starts_with("hunt_kill_site_") {
        let (gx, gy) = if let Some(rest) = obj.id.strip_prefix("hunt_kill_site_") {
            let mut parts = rest.split('_');
            if let (Some(xs), Some(ys)) = (parts.next(), parts.next())
                && let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>())
            {
                (x, y)
            } else {
                db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
            }
        } else {
            db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
        };

        if action == "Look" {
            let site_opt = crate::game::peek_recent_hunt_kill_site(gx, gy, now);
            let narrative = if let Some(site) = site_opt {
                format!(
                    "領隊【{}】率合社獵團於此處四面合圍搏殺【{}】後留下的新鮮現場，草地上尚留有餘溫獸跡與散落骨角，可執行【採集 (Harvest)】拾取遺留獸材【{}】。",
                    site.hunter_label, site.prey_name, site.loot_name
                )
            } else {
                "此處圍獵痕跡已漸次風化冷卻，獵團成員早已分鹿割肉、各自返程，僅餘些許雜亂足印。".to_string()
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.to_string(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: vec!["Harvest".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }

        if matches!(action, "Harvest" | "Gather" | "Take") {
            if let Some(site) = crate::game::take_recent_hunt_kill_site(gx, gy, now) {
                let _ = db::add_to_inventory(pid, &site.loot_id, 1);
                let narrative = format!(
            "你俯身清理合社圍獵【{}】後留下的現場，細心從殘骨與草叢間剝取收納了新鮮獸材【{}】放入行囊妥善收整，現場餘溫漸漸散去。",
                    site.prey_name, site.loot_name
                );
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                push_refresh(conn, pid);
                return;
            } else {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative: "此處遺留的獸材殘軀已被取走或已隨風化散，未尋得更多材料。".to_string(),
                    success: false,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
        }
    }
    if obj.id.starts_with("detritus_fungi_") {
        let (gx, gy) = if let Some(rest) = obj.id.strip_prefix("detritus_fungi_") {
            let mut parts = rest.split('_');
            if let (Some(xs), Some(ys)) = (parts.next(), parts.next())
                && let (Ok(x), Ok(y)) = (xs.parse::<i32>(), ys.parse::<i32>())
            {
                (x, y)
            } else {
                db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
            }
        } else {
            db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0))
        };

        if action == "Look" {
            let narrative = "此處曾有獸類殘骸與倒伏古木深埋於腐殖土壤中，歷經天地清氣與地底菌絲長久分解滋養，如今已催生出一簇生機盎然的【腐木牛樟靈菇叢】。赤紅如珊瑚般的靈芝菇蓋微吐芬芳，可執行【採集 (Harvest)】採摘收納。".to_string();
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.to_string(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: vec!["Harvest".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return;
        }

        if matches!(action, "Harvest" | "Gather" | "Take") {
            if let Some((item_id, item_name)) = crate::game::harvest_zone_detritus_fungi(gx, gy) {
                let _ = db::add_to_inventory(pid, item_id, 1);
                let narrative = format!(
                    "你蹲下身軀小心撥開腐土與厚實苔蘚，運勁將生長於朽木根部的【{}】整株完整採摘起，收入隨身藥簍——泥土中殘留的腐殖清氣漸隨風散入山林土壤。",
                    item_name
                );
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative,
                    success: true,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                push_refresh(conn, pid);
                return;
            } else {
                conn.send_json(&ActionResultMsg {
                    msg_type: "action_result".into(),
                    action: action.to_string(),
                    target_id: obj.id.clone(),
                    target_name: obj.name.clone(),
                    narrative: "此處靈菇已被採摘一空，僅餘濕潤腐土與散落菌絲，需待山林水土再度滋養才能重新萌發。".to_string(),
                    success: false,
                    actions: vec![],
                    move_target_id: String::new(),
                    open_trade: false,
                    suggested_action: String::new(),
                    tactical_choices: vec![],
                });
                return;
            }
        }
    }
    if action == "Look" {
        if (obj.id == "shanhai_apex_trace"
            || obj.name.contains("霸主獸跡")
            || obj.name.contains("獸徑足印")
            || obj.name.contains("避風岩龕")
            || obj.name.contains("山澗清泉眼")
            || obj.name.contains("巡山木樁")
            || obj.id == "dyn_campfire")
            && !narrative.contains("原野清氣")
        {
            let room_label = resolve_room_name(player_room);
            if let Some(omen) = crate::game::dynamic_hunting_zone_ecology_omen(&room_label) {
                narrative.push('\n');
                narrative.push_str(&omen);
            }
            if (obj.id == "shanhai_apex_trace"
                || obj.name.contains("霸主獸跡")
                || obj.name.contains("獸徑足印"))
                && let Some((gx, gy)) = db::get_entity(pid).ok().flatten().and_then(|e| e.grid_x.zip(e.grid_y))
            {
                crate::game::mark_apex_beast_tracked_at(gx, gy);
                narrative.push_str(
                    "\n順著地上深陷的爪印與殘留氣息撥開深林草莽，暗處一雙威凜獸瞳驟然亮起——潛伏此地的山海巨獸已現身！",
                );
                refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
                push_refresh(conn, pid);
            }
            if obj.name.contains("山澗清泉眼") || obj.name.contains("避風岩龕") {
                append_somatic_awareness_note(pid, now, &mut narrative);
            }
        }
    }
    if !apply_object_move(conn, pid, player_room, &obj, action) {
        return;
    }
    if let Some(payload) = m3_020_scene_object_followup_event_payload(&obj.id, action)
        .or_else(|| m3_027_scene_object_followup_event_payload(&obj.id, action))
    {
        let _ = event::append(now, pid, types::SCENE_OBJECT_ACTION, &payload);
    }
    let state_payload = m3_020_scene_object_state_event_payload(
        &obj.id,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    )
    .or_else(|| {
        m3_027_scene_object_state_event_payload(
            &obj.id,
            action,
            &prior_scene_actions,
            &prior_scene_states,
        )
    });
    let state_written = if let Some(payload) = state_payload {
        event::append(now, pid, types::SCENE_OBJECT_STATE, payload).is_ok()
    } else {
        false
    };
    if state_written && m3_027_scene_object_apply_side_effects(pid, player_room, &obj.id, action, state_payload)
    {
        push_refresh(conn, pid);
    }
    if (!is_street_cart || !is_cart_night_closed)
        && (action != "Look"
            || obj.id.starts_with("wt_stall_")
            || obj.id == "wt_street_roast_snack_cart"
            || obj.id == "wt_bridge_sundial_pillar"
            || obj.name.contains("日晷")
            || obj.name.contains("石柱")
            || obj.id == "wt_street_news_board"
            || obj.id == "wt_cat_0786"
            || obj.name.contains("閱報")
            || obj.name.contains("旬報")
            || obj.id == "wt_home_bamboo_mailbox"
            || obj.name.contains("竹編信箱")
            || obj.id == "wt_courier_post_desk"
            || obj.name.contains("信使櫃檯")
            || obj.id == "wt_cat_0790"
            || obj.id == "wt_bank_ledger_drawer"
            || obj.name.contains("存摺")
            || obj.name.contains("票號")
            || obj.id == "wt_guild_commission_board"
            || obj.id == "wt_hall_bulletin_board"
            || obj.id == "wt_cat_0794"
            || obj.name.contains("委託")
            || obj.name.contains("告示")
            || obj.name.contains("公佈欄")
            || obj.id == "wt_harbor_insurance_desk"
            || obj.name.contains("海事承保")
            || obj.name.contains("海事保單")
            || obj.id == "wt_harbor_clockwise_ship"
            || obj.name.contains("順時針環島海船")
            || obj.name.contains("順時針環島海運商船")
            || obj.name.contains("遠洋順流商船")
            || obj.id == "wt_home_jade_cabinet"
            || obj.name.contains("冰鑑櫃")
            || obj.name.contains("寒水玉保鮮石櫃")
            || obj.name.contains("寒水玉雙層保鮮石櫃")
            || obj.name.contains("松炭溫風烘衣櫥")
            || obj.id == "wt_coastal_angling_spot"
            || obj.id == "wt_cat_1006"
            || obj.name.contains("釣竿")
            || obj.name.contains("垂釣")
            || obj.id == "wt_lookout_telescope"
            || obj.id == "wt_cat_0733"
            || obj.name.contains("望遠鏡")
            || obj.id == "wt_field_camp_stove"
            || obj.id == "wt_cat_1010"
            || obj.name.contains("便攜爐")
            || obj.id == "wt_field_crop_plot"
            || obj.name.contains("田壟作物")
            || obj.name.contains("當季田壟"))
    {
        apply_peninsula_world_object_effects(
            conn,
            pid,
            player_room,
            &obj,
            action,
            now,
            &mut narrative,
        );
        if !narrative.is_empty()
            && !narrative.contains("一同落實手頭要務")
            && obj.id != "wt_venue_counter"
            && !obj.id.starts_with("wt_event_obj_")
        {
            let room_name = resolve_room_name(player_room);
            let full_advance_opt =
                crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object_full(
                    &room_name,
                    &obj.id,
                    &obj.name,
                    action,
                    now,
                );
            if let Some((_ev, _sp, pack_note)) = full_advance_opt {
                narrative.push_str(&pack_note);
            }
        }
    }
    let (others, move_target_id) = object_followup_buttons(
        &obj,
        player_room,
        action,
        &prior_scene_actions,
        &prior_scene_states,
    );
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions: others,
        move_target_id,
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}

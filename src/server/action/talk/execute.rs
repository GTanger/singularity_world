
fn build_one_hop_spatial_grounding_note(
    room_id: &str,
    gx: Option<i32>,
    gy: Option<i32>,
) -> String {
    let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
    let coord_opt = gx.zip(gy).or_else(|| {
        crate::grid::parse_grid_room_id(room_id).map(|c| (c.x, c.y)).or_else(|| {
            if let Some(rest) = room_id.strip_prefix("grid_") {
                let mut parts = rest.split('_');
                if let (Some(x_str), Some(y_str)) = (parts.next(), parts.next()) {
                    if let (Ok(x), Ok(y)) = (x_str.parse::<i32>(), y_str.parse::<i32>()) {
                        return Some((x, y));
                    }
                }
            }
            grid_opt.as_ref().and_then(|g| {
                crate::server::grid_manager::resolve_settlement_room_coord(g, room_id)
                    .map(|c| (c.coord.x, c.coord.y))
            })
        })
    });

    let is_peninsula = grid_opt.as_ref().is_some_and(|g| g.has_peninsula_wilderness());
    let (cell_desc, room_canon_name, scene_objs) = if let (Some(grid), Some((cx, cy))) = (&grid_opt, coord_opt) {
        if let Some(cell) = grid.get(crate::grid::SquareCoord::new(cx, cy)) {
            let desc = if !cell.description.trim().is_empty() {
                cell.description.trim().to_string()
            } else if is_peninsula {
                format!("白塔半島街坊（{}）", cell.name)
            } else {
                cell.name.clone()
            };
            let objs = if is_peninsula {
                let mut o = crate::game::player_layer_scene_objects_for_cell(cell);
                if o.is_empty() {
                    o = crate::game::player_layer_scene_objects_for_room(&cell.name);
                }
                o
            } else {
                Vec::new()
            };
            (desc, cell.name.clone(), objs)
        } else {
            let objs = if is_peninsula {
                crate::game::player_layer_scene_objects_for_room(room_id)
            } else {
                Vec::new()
            };
            let fallback_desc = if is_peninsula { "白塔半島市井街區" } else { "野外地帶" };
            (fallback_desc.to_string(), room_id.to_string(), objs)
        }
    } else {
        let objs = if is_peninsula {
            crate::game::player_layer_scene_objects_for_room(room_id)
        } else {
            Vec::new()
        };
        let fallback_desc = if is_peninsula { "白塔半島市井街區" } else { "野外地帶" };
        (fallback_desc.to_string(), room_id.to_string(), objs)
    };

    let mut obj_descs = Vec::new();
    for obj in scene_objs.iter().take(6) {
        let look = obj.responses.get("Look").or_else(|| obj.responses.get("look"));
        if let Some(l) = look {
            let l_trimmed = l.trim();
            if !l_trimmed.is_empty() {
                obj_descs.push(format!("{}（{}）", obj.name, l_trimmed));
            } else {
                obj_descs.push(obj.name.clone());
            }
        } else {
            obj_descs.push(obj.name.clone());
        }
    }
    let objs_str = if obj_descs.is_empty() {
        "街巷石板路、沿街屋舍與旋蓋夜光貝母防風燈".to_string()
    } else {
        obj_descs.join("、")
    };

    let neighbors_str = if let (Some(grid), Some((cx, cy))) = (&grid_opt, coord_opt) {
        let east = grid
            .get(crate::grid::SquareCoord::new(cx + 1, cy))
            .map(|c| c.name.as_str())
            .unwrap_or("東側街區");
        let west = grid
            .get(crate::grid::SquareCoord::new(cx - 1, cy))
            .map(|c| c.name.as_str())
            .unwrap_or("西側街區");
        let south = grid
            .get(crate::grid::SquareCoord::new(cx, cy - 1))
            .map(|c| c.name.as_str())
            .unwrap_or("南側街區");
        let north = grid
            .get(crate::grid::SquareCoord::new(cx, cy + 1))
            .map(|c| c.name.as_str())
            .unwrap_or("北側街區");
        format!("東鄰：【{east}】、西鄰：【{west}】、南鄰：【{south}】、北鄰：【{north}】")
    } else {
        "東鄰：【東向街坊】、西鄰：【西向街坊】、南鄰：【南向街坊】、北鄰：【北向街坊】".to_string()
    };

    format!(
        "\n【現場實景與東西南北四鄰真況（閉世界空間硬約束）】當前格位：【{}】（{}）；現場可見器物：{}；四向緊鄰場所：{}。對話時提及周邊方位、店鋪或街景器物，必須 100% 符合上述真實地名與器物材質（半島街燈與防風提燈皆嵌「夜光貝母薄片／黃銅燈罩」，絕無「牛角燈」）；凡未列於當前場所或四鄰之店鋪（如燈籠鋪、燈具店等），嚴禁宣稱「就在這條街上」或「就在前頭鋪子」！",
        room_canon_name, cell_desc, objs_str, neighbors_str
    )
}

fn format_relative_grid_direction(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> String {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    if dx == 0 && dy == 0 {
        return format!("就在當前原地（座標 {to_x}, {to_y}）");
    }
    let ew = if dx > 0 {
        format!("往東 {} 步", dx)
    } else if dx < 0 {
        format!("往西 {} 步", -dx)
    } else {
        String::new()
    };
    let ns = if dy > 0 {
        format!("往北 {} 步", dy)
    } else if dy < 0 {
        format!("往南 {} 步", -dy)
    } else {
        String::new()
    };
    let compass = match (dx.signum(), dy.signum()) {
        (1, 0) => "正東向",
        (-1, 0) => "正西向",
        (0, 1) => "正北向",
        (0, -1) => "正南向",
        (1, 1) => "東北向",
        (1, -1) => "東南向",
        (-1, 1) => "西北向",
        (-1, -1) => "西南向",
        _ => "原地",
    };
    let steps_desc = match (!ew.is_empty(), !ns.is_empty()) {
        (true, true) => format!("{ew}、{ns}"),
        (true, false) => ew,
        (false, true) => ns,
        (false, false) => "0 步".to_string(),
    };
    format!("{compass}（{steps_desc}，座標 {to_x}, {to_y}）")
}

fn build_spatial_navigation_guide(
    target: &Character,
    gx: Option<i32>,
    gy: Option<i32>,
    player_input: &str,
) -> String {
    let (Some(cx), Some(cy)) = (gx, gy) else {
        return String::new();
    };
    let slot = crate::store::wenyao_household_slot_for(&target.id);
    let work_dir = format_relative_grid_direction(cx, cy, slot.work_x, slot.work_y);
    let home_dir = format_relative_grid_direction(cx, cy, slot.home_x, slot.home_y);
    let errand_dir = format_relative_grid_direction(cx, cy, slot.errand_x, slot.errand_y);
    let mut landmark_parts = vec![
        format!("你的任事場所【{}】：{}", slot.work_name, work_dir),
        format!("你的住處【{}】：{}", slot.home_name, home_dir),
        format!("常去差事地點【{}】：{}", slot.errand_name, errand_dir),
    ];
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let landmarks = grid.query_settlement_landmarks_from((cx, cy), 24, player_input);
        for ((lx, ly), lname) in landmarks {
            landmark_parts.push(format!(
                "【{lname}】：{}",
                format_relative_grid_direction(cx, cy, lx, ly)
            ));
        }
    }
    format!(
        "\n【精確指路與地標方位表（當前所在座標：{cx}, {cy}）】{}。若來客詢問聚落內客舍、食堂、浴場、醫館、工坊或商行位置，必須嚴格依據上方【鄰近真實場所方位表】給出準確方位與步數，不得憑空捏造路徑。",
        landmark_parts.join("；")
    )
}

fn build_temporal_and_skepticism_grounding_note(
    room_display_name: &str,
    gx: Option<i32>,
    gy: Option<i32>,
) -> String {
    let (hour, _minute) = crate::game::current_game_hour_and_min();
    let shichen = match hour {
        23 | 0 => "子時（23:00–00:59，三更深夜）",
        1 | 2 => "丑時（01:00–02:59，四更殘夜）",
        3 | 4 => "寅時（03:00–04:59，五更黎明前）",
        5 | 6 => "卯時（05:00–06:59，晨光破曉）",
        7 | 8 => "辰時（07:00–08:59，早市開張）",
        9 | 10 => "巳時（09:00–10:59，白日正忙）",
        11 | 12 => "午時（11:00–12:59，日正當中）",
        13 | 14 => "未時（13:00–14:59，午後當值）",
        15 | 16 => "申時（15:00–16:59，晡時將晚）",
        17 | 18 => "酉時（17:00–18:59，日落收工）",
        19 | 20 => "戌時（19:00–20:59，入夜初更）",
        _ => "亥時（21:00–22:59，二更夜深）",
    };
    let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
    let cell_opt = match (grid_opt.as_ref(), gx, gy) {
        (Some(grid), Some(x), Some(y)) => grid.get(crate::grid::SquareCoord::new(x, y)),
        _ => None,
    };
    let is_street = cell_opt.map(|c| c.is_pure_street_or_courtyard()).unwrap_or_else(|| {
        crate::grid::is_street_or_gate_room_name(room_display_name)
    });
    let is_open = cell_opt.map(|c| c.is_open_at_hour(hour.max(0) as u32, false)).unwrap_or((5..=19).contains(&hour));

    let mut note = String::new();
    if !(5..=19).contains(&hour) {
        if is_street {
            note.push_str(&format!(
                "\n【時辰與公共通道夜段硬約束】當前真實時刻為 {hour:02} 點（屬{shichen}，必須如實使用該時辰名稱，不得跨時辰誤報）。現在為夜段收市時分，公共街巷與橋樑通道僅作通行、巡防或返家路徑，並無營業商鋪，居民均在收工返家或回家休息途中。"
            ));
        } else if !is_open {
            note.push_str(&format!(
                "\n【時辰與場所夜間打烊硬約束】當前真實時刻為 {hour:02} 點（屬{shichen}，必須如實使用該時辰名稱，不得跨時辰誤報）。當前場所【{room_display_name}】於夜段已打烊閉門，商號與工坊均已收工歇業。"
            ));
        } else {
            note.push_str(&format!(
                "\n【時辰與夜段硬約束】當前真實時刻為 {hour:02} 點（屬{shichen}，必須如實使用該時辰名稱，不得跨時辰誤報）。現在為夜段，全城日間商號工坊均已打烊，除值更巡防與夜間接待場所外，居民皆在休息安頓，言行須符合深夜情境。"
            ));
        }
    } else if (5..=7).contains(&hour) {
        note.push_str(&format!(
            "\n【時辰與拂曉晨起硬約束】當前真實時刻為 {hour:02} 點（屬{shichen}，拂曉晨間）。此時為清晨早起上工、晨汲洗漱或早市開張時分，絕非「剛收工準備返家睡覺」；言談應符合晨起準備活計或早間當值情境。"
        ));
    }

    if is_street {
        note.push_str(&format!(
            "\n【空間機能與時序誠實校準】當前場所【{room_display_name}】為公共通道，僅供穿行、駐足或巡防，並非固定營業店鋪；面對來客詢問所在位置或時段時，必須依據此處真實場域性質與當前時辰如實應答。"
        ));
    } else if !is_open {
        note.push_str(&format!(
            "\n【空間機能與時序誠實校準】當前場所【{room_display_name}】目前已打烊收工；面對來客詢問所在位置或時段時，應坦誠自己正料理收尾、準備回家或在此過夜休息，不得虛構營業狀態。"
        ));
    } else {
        note.push_str(&format!(
            "\n【空間機能與時序誠實校準】面對來客詢問所在位置或時段時，必須依據當前場所【{room_display_name}】之現場器物與當前時辰如實應答，言行與現場環境保持一致。"
        ));
    }
    note.push_str("\n台詞請一律使用自然生活化的台灣正體中文日常白話口語，勿用古裝武俠文言腔調或北方兒化用語。");
    note
}

pub(super) fn do_entity_talk(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player_room: &str,
    target_id: &str,
    target: &Character,
    now: i64,
) {
    crate::store::pause_npc_for_interaction(target_id, 180);
    let mut player_input = msg.player_input.clone();
    if player_input.trim().is_empty() {
        player_input = "（搭話）".into();
    }
    let was_named = if target.kind == EntityKind::Npc {
        db::get_acquaintance(pid, target_id)
            .ok()
            .flatten()
            .map(|a| a.named)
            .unwrap_or(false)
    } else {
        true
    };
    let mut talk_name = super::player_visible_target_name(pid, target);
    let hour = crate::store::current_game_hour();
    let is_night = hour >= 22 || hour < 5;
    let is_sleeping = target.current_activity.contains("安歇")
        || target.current_activity.contains("熟睡")
        || target.current_activity.contains("就寢")
        || target.current_activity.contains("熄燈");
    if is_night && is_sleeping {
        let narrative = "夜色已深，對方已在內室熄燈安歇，不便驚擾。".to_string();
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Talk".into(),
            target_id: target.id.clone(),
            target_name: talk_name,
            narrative,
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if is_open_trade_input(&player_input) && target_can_open_trade(target, target_id, player_room) {
        let narrative = if player_input.trim() != "（搭話）" && !player_input.trim().is_empty() {
            format!("【{talk_name}】示意你可以先看貨、再出價。")
        } else {
            format!("你向【{talk_name}】問起交易。{talk_name}示意你可以先看貨、再出價。")
        };
        let npc_reply = "可以先看貨、再出價。".to_string();
        let _ = event::append(now, pid, "talk", target_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "Talk".into(),
            target_id: target.id.clone(),
            target_name: talk_name.clone(),
            narrative,
            success: true,
            actions: vec!["Trade".into()],
            move_target_id: String::new(),
            open_trade: true,
            suggested_action: "Trade".into(),
            tactical_choices: vec![],
        });
        conn.sessions
            .record_player_talk_for_room_echo(pid, player_room, &msg.player_input);
        flush_conversation_and_append(pid, target_id, &player_input, &npc_reply, now);
        let _ = adjust_favorability(target_id, pid, FAV_TALK);
        return;
    }
    let is_quick_chip_talk = false;
    let (backstory, style_examples, mut sensitivity) = if is_quick_chip_talk {
        (String::new(), Vec::new(), String::new())
    } else {
        let raw_backstory = db::build_identity(target_id);
        let bs = if target.kind == EntityKind::Npc {
            let real_name = resolve_npc_real_name(target);
            let patched = raw_backstory.replace(target_id, &real_name);
            if !patched.contains(&real_name) {
                format!("你的真實身份與姓名是：{real_name}。{patched}")
            } else {
                patched
            }
        } else {
            raw_backstory
        };
        let mut se = pick_style_examples(target_id, 3);
        se.retain(|s| {
            !s.contains("錢袋")
                && !s.contains("再誇")
                && !s.contains("花錢")
                && !s.contains("有啥")
                && !s.contains("啥事")
                && !s.contains("\u{5ba2}\u{5b98}")
                && !s.contains("客倌")
                && !s.contains("小二")
                && !s.contains("銀兩")
                && !s.contains("銅錢")
                && !s.contains("\u{638c}\u{6ac3}")
        });
        let mut sens = talk_sensitivity_hint(target);
        sens.push_str(&talk_disposition_hint(target_id, pid));
        (bs, se, sens)
    };
    let personality = crate::db::get_personality_for_entity(target_id)
        .or_else(|| target.soul_seed.map(crate::db::expand_soul_seed_to_personality))
        .unwrap_or(crate::db::Personality {
            boldness: 0.5,
            sensitivity: 0.5,
            orderliness: 0.5,
        });
    let pool_opt = crate::pg::pool();
    let (rel, obligations) = if let Some(pool) = &pool_opt {
        let r = crate::pg::living_world::get_relationship_summary(pool, target_id, pid)
            .unwrap_or_default();
        let o = crate::pg::living_world::list_open_obligations_for_entity(pool, target_id)
            .unwrap_or_default();
        (r, o)
    } else {
        (crate::pg::living_world::RelationshipSummary::default(), Vec::new())
    };
    let mut obligation_descs: Vec<String> = obligations
        .iter()
        .filter(|o| o.obligation_kind != "employment")
        .filter_map(|o| format_obligation_subject_zh(&o.obligation_kind, &o.subject_ref))
        .collect();
    if let Some((kind, subj)) = crate::store::get_active_npc_obligation(target_id) {
        if let Some(desc) = format_obligation_subject_zh(&kind, &subj) {
            if !obligation_descs.contains(&desc) {
                obligation_descs.push(desc);
            }
        }
    }
    let live_activity = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.get_entity(target_id)
            .map(|e| s.effective_npc_activity(&e))
            .unwrap_or_default()
    } else {
        String::new()
    };
    let current_activity = if !live_activity.is_empty() {
        live_activity.as_str()
    } else if target.current_activity.is_empty() {
        "佇立觀察四周"
    } else {
        &target.current_activity
    };

    let session_turns_pre = get_recent_turns(pid, target_id, 16);
    let talk_intent = classify_player_talk_intent(&player_input);
    let is_trade_topic = talk_intent == PlayerTalkIntent::TradeOrErrand;
    let already_mentioned_barter =
        has_recent_barter_or_obligation_mention(&session_turns_pre, &obligation_descs);
    let effective_obligations_for_stance: &[String] = if is_trade_topic {
        &obligation_descs
    } else {
        &[]
    };

    // Step 1: 拍板 NPC 互動立場
    let decision_input = NpcDecisionInput {
        npc_name: &talk_name,
        boldness: personality.boldness,
        sensitivity: personality.sensitivity,
        orderliness: personality.orderliness,
        familiarity_score: rel.familiarity_score,
        balance_score: rel.balance_score,
        open_obligations: effective_obligations_for_stance,
        current_activity,
        action_kind: "Talk",
        player_utterance: &player_input,
    };
    let mut decision = score_npc_interaction_stance(&decision_input);
    if !is_trade_topic && decision.choice == NpcStanceChoice::Bargain {
        decision.choice = NpcStanceChoice::Deflect;
        decision.prompt_directive =
            "【已拍板決策：C 劃定邊界／有條件往來】請切合當前時刻與自身手頭差事回應。若非交易話題，不可盲從應允，應講究情理邊界或提出對等互惠條件（如要求出示信物、打聽特定消息或先協助手頭差事）；若來客邀你離開現場，請以差事在身為由婉拒；台詞限 35 字內。".to_string();
    }
    let resolved_room_for_stance = db::get_room_name(player_room).unwrap_or_else(|_| player_room.to_string());
    let (eval_stance, eval_stance_desc) = if let Ok(Some(player_ent)) = db::get_entity(pid) {
        let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
        evaluate_npc_negotiation_stance(pid, target_id, &resolved_room_for_stance, &inv)
    } else {
        let empty_inv = crate::db::inventory::InventorySnapshot {
            items: vec![],
            current_weight: 0.0,
            max_weight: 50.0,
        };
        evaluate_npc_negotiation_stance(pid, target_id, &resolved_room_for_stance, &empty_inv)
    };
    if eval_stance == NegotiationStance::Refuse {
        decision.choice = NpcStanceChoice::Refuse;
    } else if eval_stance == NegotiationStance::Deflect {
        decision.choice = NpcStanceChoice::Deflect;
    } else if eval_stance == NegotiationStance::Bargain && decision.choice == NpcStanceChoice::Accept {
        decision.choice = NpcStanceChoice::Bargain;
    }
    let mentorship_note = handle_three_baton_mentorship_and_obligation(
        pid,
        target,
        &player_input,
        eval_stance,
        now,
    );
    if mentorship_note.is_some() {
        push_refresh(conn, pid);
    }

    // Step 2.5: 實行對話世界效果（義務交割、立約交換、同道相助）
    let world_effect = apply_talk_world_effects(
        conn,
        pid,
        target_id,
        &talk_name,
        current_activity,
        &player_input,
        decision.choice,
        &obligation_descs,
        rel.familiarity_score,
        now,
        pool_opt.as_deref(),
    );
    let hunting_pledge_note = if decision.choice != NpcStanceChoice::Refuse {
        let party_opt = crate::store::LIVE_HUNTING_PARTIES
            .read()
            .values()
            .find(|p| p.member_ids.contains(&target.id))
            .cloned();
        if let Some(party) = party_opt {
            crate::store::pledge_player_to_hunting_party(pid, &party.zone_name);
            crate::pg::living_world::record_breath_activity(
                pid,
                crate::pg::living_world::BreathHabitKind::ForestLong,
                1,
                now,
            );
            let (px, py) = party.prey_coord;
            let prey_place = crate::game::CANONICAL_HUNTING_ZONES
                .get(crate::game::nearest_hunting_zone_idx(px, py))
                .map(|z| z.0.to_string())
                .unwrap_or_else(|| {
                    let (z, s, _) = crate::grid::r#gen::classify_wilderness_cell(px, py);
                    format!("{z}・{s}")
                });
            Some(format!(
                "\n【合社入團・見者有份】你已應募加入領隊【{}】率領之【{}】合社獵團（現共 {} 名獵手）！當前鎖定獵物為【{prey_place}】一帶之【{}】——無論由你親手擊殺或隨獵團合圍收網，皆可依半島古規「見者有份」分得【{}】與交割鎂幣紅利！",
                party.leader_name,
                party.zone_name,
                party.member_ids.len(),
                party.prey_name,
                party.prey_loot_name
            ))
        } else {
            None
        }
    } else {
        None
    };

    // Step 3: 將拍板指令注入 prompt
    if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
        sensitivity.push('\n');
    }
    sensitivity.push_str(&decision.prompt_directive);
    if is_trade_topic {
        if !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(&eval_stance_desc);
        if let Some((kind, subj)) = crate::store::get_active_npc_obligation(target_id) {
            if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
                sensitivity.push('\n');
            }
            sensitivity.push_str(&format!(
                "【當前待辦義務】NPC目前身負待辦事務（{}: {}），處境緊迫或分心，請在對話神態與回應中自然體現。",
                kind, subj
            ));
        }
    }
    if let Some(prompt_hint) = &world_effect.prompt_hint {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(prompt_hint);
    }
    if mentorship_note.is_some() {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str("【行家指點・師承授受】你正在就百工修行與身法要訣指點來客，請在回覆中具體流露行家指點與傳授奧妙的長者風範。");
    }
    let now_stamp = crate::store::format_game_calendar_stamp(now);
    let (first_met_at, last_prior_at, prior_turn_count) =
        db::player_npc_temporal_acquaintance_stats(target_id, pid, now);
    let effective_first_met = match (first_met_at, (rel.first_interaction_at > 0).then_some(rel.first_interaction_at)) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let session_span = crate::server::conversation::get_current_session_span(pid, target_id, now);
    let session_desc = if let Some((first_turn_at, last_turn_at, turn_cnt)) = session_span {
        format!(
            "本場交談自{}前開始（已連續聊了 {} 輪，上一句在{}前）",
            crate::store::format_elapsed_duration_zh(now - first_turn_at),
            turn_cnt,
            crate::store::format_elapsed_duration_zh(now - last_turn_at)
        )
    } else {
        "本句為本場交談的開場首句".to_string()
    };
    if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
        sensitivity.push('\n');
    }
    if prior_turn_count == 0 && (rel.first_interaction_at == 0 || (now - rel.first_interaction_at) < 1800) {
        let met_desc = effective_first_met
            .map(|t| {
                format!(
                    "於{}（{}）",
                    crate::store::format_elapsed_duration_zh(now - t),
                    crate::store::format_game_calendar_stamp(t)
                )
            })
            .unwrap_or_else(|| "於此刻".to_string());
        sensitivity.push_str(&format!(
            "【時間感與相識關係事實】當前為{now_stamp}。你與眼前這位來客是今日這場交談才初次結識搭話（初識{met_desc}，{session_desc}，先前素昧平生並無往日舊交）。若對方問起「我們認識多久了」或過往，請根據上述真實經過秒數／分鐘數如實回答今天才頭一回聊上，切勿把本場對話前幾句聊過的話當成以前發生的往日舊事！"
        ));
    } else {
        let first_desc = effective_first_met
            .map(|t| crate::store::format_memory_temporal_anchor(t, now))
            .unwrap_or_else(|| "往昔".to_string());
        let last_prior_desc = last_prior_at
            .map(|t| crate::store::format_memory_temporal_anchor(t, now))
            .unwrap_or_else(|| "先前".to_string());
        let mut fact_line = format!(
            "【時間感與相識關係事實】當前為{now_stamp}。你與這位來客初次相識於【{first_desc}】，上次交談於【{last_prior_desc}】（歷史累計交談 {} 回，熟稔度 {}），{}。",
            prior_turn_count, rel.familiarity_score, session_desc
        );
        let prior_summary = crate::store::get_store()
            .map(|arc| arc.read().get_npc_summary(target_id))
            .unwrap_or_default();
        if !prior_summary.trim().is_empty() {
            fact_line.push_str(&format!(" 過往交談整體印象：{}", prior_summary.trim()));
        }
        sensitivity.push_str(&fact_line);
    }
    let has_prior_session_talk = prior_turn_count > 0;
    let trimmed_input = player_input.trim();
    let is_asking_shift_hours = (trimmed_input.contains("幾點")
        || trimmed_input.contains("何時")
        || trimmed_input.contains("什麼時候")
        || trimmed_input.contains("到幾點"))
        && ["下班", "收工", "上工", "開工", "當值", "休息", "打烊", "歇業", "工作"]
            .iter()
            .any(|kw| trimmed_input.contains(kw));
    if is_asking_shift_hours {
        let slot = crate::store::wenyao_household_slot_for(target_id);
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(&format!(
            "【上下班作息時刻直答指令】來客正在詢問你的上下班（開工／收工）時刻！你每日固定 {:02}:00 上工、傍晚 {:02}:00 收工（下班）。請直接以自然口語明確回答「我每天早上 {} 點上工、傍晚 {} 點收工」（或直接報出 {} 點收工），嚴禁答非所問去報路程步數或只說『才收工』卻不報幾點！",
            slot.morning_hour,
            slot.evening_hour,
            slot.morning_hour,
            slot.evening_hour,
            slot.evening_hour
        ));
    }
    let is_asking_where_going = ["去哪", "往哪", "做什麼", "幹嘛", "在忙什麼", "去哪裡", "從哪", "還要走多久", "幾步"]
        .iter()
        .any(|kw| trimmed_input.contains(kw));
    let already_mentioned_transit = session_turns_pre.iter().any(|(_, n_rep)| {
        ["正趕著", "正往", "才從", "動身", "步就到", "歇腳呢"]
            .iter()
            .any(|kw| n_rep.contains(kw))
    });
    if already_mentioned_transit && !is_asking_where_going {
        if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
            sensitivity.push('\n');
        }
        sensitivity.push_str(&format!(
            "【多輪對話推進與防鬼打牆複讀鐵律】你在本場前幾句對話中已經向來客交代過自己從何處過來、正趕往何處；本句請 100% 針對來客的新提問（「{trimmed_input}」）直接正面回答，嚴禁在句首或句尾再次重複背誦「才從…出來」、「正趕著回…歇腳」或剩餘步數！"
        ));
    } else if current_activity.starts_with("正動身前往") {
        if let Some(spatial_sense) = crate::store::describe_npc_spatial_temporal_state(target_id) {
            if !sensitivity.is_empty() && !sensitivity.ends_with('\n') {
                sensitivity.push('\n');
            }
            sensitivity.push_str(&spatial_sense);
        } else {
            sensitivity.push_str(
                "\n【在途位置事實】你目前人正在半路上趕路（尚未抵達目的地鋪面或場館，手頭也還沒開始挑貨或辦事）；若來客問「在哪裡」或聊起手頭物件，請如實回答目前正走在當前場所路上、準備前往目的地，切勿說自己已經在鋪子裡挑貨！",
            );
        }
    }

    let recent_turns = get_recent_turns(pid, target_id, 5);
    let effective_user_prompt = if trimmed_input.is_empty() || trimmed_input == "（搭話）" {
        if !recent_turns.is_empty() {
            let last_topic = recent_turns.last().map(|t| t.0.as_str()).unwrap_or("");
            if !last_topic.is_empty() && last_topic != "（搭話）" {
                format!(
                    "（來客再次向你開口搭話，請延續你們先前的話題「{}」或根據你手頭正在做的事自然回應）",
                    last_topic
                )
            } else {
                "（來客再次向你開口搭話，請延續你們先前交談的氣氛或根據你手頭正在做的事自然回應）".to_string()
            }
        } else if was_named || rel.familiarity_score >= 2 {
            "（熟識的來客走近向你打招呼，請以熟絡口吻結合你手頭的活計、心情或近期市集供需動靜主動開口）".to_string()
        } else {
            "（一名來客走近向你搭話，請展現你正在忙碌的動作神態，並以符合你職業與性格的半島自然口語開口回應）".to_string()
        }
    } else {
        trimmed_input.to_string()
    };

    let is_quick_chip_talk = false;
    let mut snippets = Vec::new();
    if !is_quick_chip_talk {
        let session_turns_timed = crate::server::conversation::get_recent_turns_with_time(pid, target_id, 16);
        let mut seen_replies = std::collections::HashSet::new();
        for (p_in, n_rep, turn_at) in &session_turns_timed {
            let p_in = p_in.trim();
            let clean_rep = crate::ai::sanitize::sanitize_npc_dialogue_line(n_rep.trim());
            let n_rep = clean_rep.trim();
            if !p_in.is_empty() && !n_rep.is_empty() && seen_replies.insert(n_rep.to_string()) {
                let anchor = crate::store::format_memory_temporal_anchor(*turn_at, now);
                snippets.push(format!("【先前對話】[{anchor}] 來客說：「{p_in}」 → 你已回答：「{n_rep}」"));
            }
        }
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            let archs = s.get_archival_by_entity(target_id);
            let mut seen_arch_content = std::collections::HashSet::new();
            for a in archs.into_iter().rev() {
                let already_in_session = session_turns_timed.iter().any(|(p_in, n_rep, _)| {
                    let clean_rep = n_rep.trim().trim_matches(|c| c == '「' || c == '」');
                    (!clean_rep.is_empty() && a.content.contains(clean_rep))
                        || (!p_in.trim().is_empty() && a.content.contains(p_in.trim()))
                });
                if !already_in_session
                    && (now - a.created_at) >= 1800
                    && seen_arch_content.insert(a.content.clone())
                    && (a.tag == "player_talk" || a.content.contains(pid) || a.content.contains("來客"))
                    && !a.content.contains("青籠關")
                    && !a.content.contains("林緣聚地")
                {
                    let anchor = crate::store::format_memory_temporal_anchor(a.created_at, now);
                    snippets.push(format!("【往日記憶・{anchor}】{}", a.content));
                    if snippets.len() >= 8 {
                        break;
                    }
                }
            }
        }
        if has_prior_session_talk {
            let extra_archival =
                db::search_archival_for_player_talk_with_time(target_id, &effective_user_prompt, 4);
            for (s, at) in extra_archival {
                let in_current_session = session_turns_timed.iter().any(|(p_in, n_rep, _)| {
                    let clean_rep = n_rep.trim().trim_matches(|c| c == '「' || c == '」');
                    (!clean_rep.is_empty() && s.contains(clean_rep))
                        || (!p_in.trim().is_empty() && s.contains(p_in.trim()))
                });
                if !in_current_session
                    && (now - at) >= 1800
                    && !s.contains("青籠關")
                    && !s.contains("林緣聚地")
                    && !s.contains("草原聚地")
                    && !s.contains("錢袋")
                    && !s.contains("有啥")
                    && !snippets.iter().any(|existing| existing.contains(&s))
                {
                    let anchor = crate::store::format_memory_temporal_anchor(at, now);
                    snippets.push(format!("【往日記憶・{anchor}】{s}"));
                }
            }
        }
    }

    let mut room_display_name = super::resolve_action_room_name(pid, target);
    if room_display_name.trim().is_empty() {
        room_display_name = get_room_name(player_room).unwrap_or_default();
    }
    if room_display_name.trim().is_empty() {
        room_display_name = player_room.to_string();
    }
    let backstory = if is_quick_chip_talk {
        String::new()
    } else {
        let cleaned_identity_bs = if !is_trade_topic && already_mentioned_barter {
            backstory
                .replace("青籠關", "文曜")
                .lines()
                .filter(|line| !line.contains("你當前手頭有待辦的義務或牽掛"))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            backstory.replace("青籠關", "文曜")
        };
        let mut bs = format!(
            "你目前所在的當前地點是：【{room_display_name}】（對話時請嚴格以【{room_display_name}】與你當前的具體活動為準，切勿提及不在場的其他關隘或異地）。{}",
            cleaned_identity_bs
        );
        let player_coords = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|p| (p.grid_x, p.grid_y))
            .unwrap_or((None, None));
        let (gx, gy) = match player_coords {
            (Some(x), Some(y)) => (Some(x), Some(y)),
            _ => crate::store::LIVE_NPC_GRID_STATES
                .read()
                .get(&target.id)
                .map(|s| (Some(s.current.0), Some(s.current.1)))
                .unwrap_or((target.grid_x, target.grid_y)),
        };
        let spatial_note = build_one_hop_spatial_grounding_note(player_room, gx, gy);
        if !spatial_note.is_empty() {
            bs.push_str(&spatial_note);
        }
        let nav_guide = build_spatial_navigation_guide(target, gx, gy, &player_input);
        if !nav_guide.is_empty() {
            bs.push_str(&nav_guide);
        }
        let temporal_skepticism_note = build_temporal_and_skepticism_grounding_note(&room_display_name, gx, gy);
        if !temporal_skepticism_note.is_empty() {
            bs.push_str(&temporal_skepticism_note);
        }
        let venue_ctx = extract_venue_and_role_talk_context(target, &room_display_name);
        if !venue_ctx.is_empty() {
            bs.push_str(&format!("\n【坊市職責與攤位現況】：{}", venue_ctx));
        }
        let live_talk_ctx = build_universal_entity_live_talk_context(target);
        if !live_talk_ctx.is_empty() {
            bs.push_str(&live_talk_ctx);
        }
        let knowledge_facts = crate::pg::living_world::recent_resident_knowledge_facts(&target.id, 3);
        if !knowledge_facts.is_empty() {
            bs.push_str(&format!("\n【近期親歷見聞】：{}（若來客問及近況、物資或鄰里，可自然帶出這些親歷見聞）", knowledge_facts.join("；")));
        }
        if is_trade_topic {
            if let Ok(Some(player_ent)) = db::get_entity(pid) {
                let inv = db::get_inventory(&player_ent.inventory, player_ent.vit);
                let mat_ctx = build_player_material_context(&inv, &room_display_name);
                if !mat_ctx.is_empty() {
                    bs.push_str(&mat_ctx);
                }
            }
        }
        bs
    };
    let instant_cognitive_pair: Option<(String, String)> = None;
    let (narrative, npc_reply, ai_succeeded) = if let Some((n, r)) = instant_cognitive_pair {
        (n, r, true)
    } else {
        let sanitize_instant_time_words = |text: String| -> String {
            let cleaned = crate::ai::sanitize::sanitize_npc_dialogue_line(&text);
            cleaned
                .replace("\u{525b}裝好", "才裝好")
                .replace("\u{525b}從", "才從")
                .replace("\u{525b}聽", "才聽")
                .replace("\u{525b}到", "才到")
                .replace("\u{525b}\u{624d}", "前面")
                .replace("\u{525b}\u{525b}", "前面")
                .replace("\u{65b9}\u{624d}", "前面")
        };
        let player_or_key = crate::ai::get_player_openrouter_key(pid);
        let ai_result = if player_or_key.is_some() || conn.cfg.player_talk_uses_web_api() {
            let past_only_snippets: Vec<String> = snippets
                .iter()
                .filter(|s| !s.starts_with("【先前對話】") && !s.starts_with("【當場對話】"))
                .cloned()
                .collect();
            let (mut system_prompt, first_turn_user_msg) = player_npc_talk_build_prompts(
                &effective_user_prompt,
                &backstory,
                &past_only_snippets,
                &style_examples,
                &sensitivity,
                &room_display_name,
            );
            if !session_turns_pre.is_empty() && !past_only_snippets.is_empty() {
                system_prompt.push_str("\n【往日相關記憶（皆附帶具體時間座標，切勿與本場當下對話混淆）】\n");
                system_prompt.push_str(&past_only_snippets.join("\n"));
                system_prompt.push('\n');
            }
            let timed_history_for_api: Vec<(String, String)> =
                crate::server::conversation::get_recent_turns_with_time(pid, target_id, 16)
                    .into_iter()
                    .map(|(p_in, n_rep, turn_at)| {
                        let elapsed = crate::store::format_elapsed_duration_zh(now - turn_at);
                        (format!("（本場對話・{elapsed}前）{p_in}"), n_rep)
                    })
                    .collect();
            let final_user_msg = if timed_history_for_api.is_empty() {
                first_turn_user_msg
            } else {
                format!("（本場對話・當下本句）{effective_user_prompt}")
            };
            let primary_res = if let Some(user_key) = &player_or_key {
                match call_openai_compatible_chat_with_history(
                    crate::ai::DEFAULT_OPENROUTER_BASE_URL,
                    user_key,
                    crate::ai::DEFAULT_OPENROUTER_TALK_MODEL,
                    &system_prompt,
                    &timed_history_for_api,
                    &final_user_msg,
                    120,
                ) {
                    Ok(reply) => Ok(reply),
                    Err(e) => {
                        tracing::warn!(
                            "player openrouter key chat error for {pid}: {e}, falling back to server default / local fast GPU"
                        );
                        call_openai_compatible_chat_with_history(
                            &conn.cfg.player_talk_api_base_url,
                            &conn.cfg.player_talk_api_key,
                            &conn.cfg.player_talk_api_model,
                            &system_prompt,
                            &timed_history_for_api,
                            &final_user_msg,
                            110,
                        )
                    }
                }
            } else {
                call_openai_compatible_chat_with_history(
                    &conn.cfg.player_talk_api_base_url,
                    &conn.cfg.player_talk_api_key,
                    &conn.cfg.player_talk_api_model,
                    &system_prompt,
                    &timed_history_for_api,
                    &final_user_msg,
                    110,
                )
            };
            match primary_res {
                Ok(reply) => Ok(sanitize_instant_time_words(reply)),
                Err(e) => {
                    tracing::warn!("web api chat error: {e}, falling back to local fast GPU / call_ai_talk");
                    match call_openai_compatible_chat_with_history(
                        crate::ai::LOCAL_FAST_GPU_URL,
                        "",
                        crate::ai::LOCAL_FAST_GPU_MODEL,
                        &system_prompt,
                        &timed_history_for_api,
                        &final_user_msg,
                        350,
                    ) {
                        Ok(reply) => Ok(sanitize_instant_time_words(reply)),
                        Err(e2) => {
                            tracing::warn!("local fast GPU chat error: {e2}, falling back to call_ai_talk");
                            call_ai_talk(
                                &conn.cfg.ollama_base_url,
                                &conn.cfg.ollama_model,
                                &effective_user_prompt,
                                &backstory,
                                &snippets,
                                &style_examples,
                                &sensitivity,
                                &room_display_name,
                            )
                            .map(sanitize_instant_time_words)
                        }
                    }
                }
            }
        } else {
            call_ai_talk(
                &conn.cfg.ollama_base_url,
                &conn.cfg.ollama_model,
                &effective_user_prompt,
                &backstory,
                &snippets,
                &style_examples,
                &sensitivity,
                &room_display_name,
            )
            .map(sanitize_instant_time_words)
        };

        match ai_result {
            Ok(raw_reply) => {
                let reply = if (!is_trade_topic && already_mentioned_barter)
                    || (already_mentioned_transit && !is_asking_where_going)
                {
                    strip_repeated_barter_tail(&raw_reply, &session_turns_pre, &obligation_descs)
                } else {
                    raw_reply
                };
                let reply = enforce_strict_third_person_and_no_parens(&reply);
                let narr = format_npc_talk_line(target, &talk_name, &reply, &player_input, pid);
                (narr, reply, true)
            }
            Err(err) => {
                tracing::warn!("all AI calls failed ({err}), falling back to cognitive / canonical or rule reply");
                let (n, r) = if let Some(cog_pair) = build_cognitive_talk_reply(
                    target,
                    &talk_name,
                    &room_display_name,
                    &player_input,
                    current_activity,
                    decision.choice,
                    &obligation_descs,
                ) {
                    cog_pair
                } else if let Some(canon_pair) = canonical_or_venue_talk_reply(
                    target,
                    &talk_name,
                    &room_display_name,
                    &player_input,
                    decision.choice,
                    false,
                ) {
                    canon_pair
                } else {
                    build_talk_fallback(
                        target,
                        &talk_name,
                        &room_display_name,
                        &player_input,
                        decision.choice,
                    )
                };
                let n = enforce_strict_third_person_and_no_parens(&n);
                let r = enforce_strict_third_person_and_no_parens(&r);
                (n, r, false)
            }
        }
    };
    let gender_str = match target.gender {
        Some(crate::entity::Gender::F) => Some("F"),
        Some(crate::entity::Gender::M) => Some("M"),
        None => None,
    };
    let short_real_name =
        crate::store::resolve_npc_person_name(&target.id, &target.display_title, gender_str);
    let real_name = resolve_npc_real_name(target);
    let asking_name = is_asking_name(&player_input);
    let self_intro = npc_reply.contains("在下")
        || npc_reply.contains("我叫")
        || npc_reply.contains("我是")
        || npc_reply.contains("叫我")
        || npc_reply.contains("老夫")
        || npc_reply.contains("老身")
        || npc_reply.contains("免貴姓")
        || npc_reply.contains("賤名");
    let name_revealed_in_text = (!short_real_name.is_empty()
        && (narrative.contains(&short_real_name) || npc_reply.contains(&short_real_name)))
        || (!real_name.is_empty()
            && (narrative.contains(&real_name) || npc_reply.contains(&real_name)));
    let relationship_met = rel.familiarity_score >= 2
        || rel.balance_score >= 1
        || world_effect.narrative_suffix.is_some();
    let quick_greeting_met = matches!(trimmed_input, "寒暄問候" | "打聽近況")
        && decision.choice != NpcStanceChoice::Refuse;
    let should_unlock_name = target.kind == EntityKind::Npc
        && !was_named
        && (asking_name || self_intro || relationship_met || quick_greeting_met || name_revealed_in_text);
    let mut final_narrative = narrative;
    if should_unlock_name {
        talk_name = short_real_name.clone();
        let already_mentioned = npc_reply.contains(&real_name) || npc_reply.contains(&short_real_name);
        final_narrative = final_narrative
            .replace("【陌生人】", &format!("【{talk_name}】"))
            .replace("陌生人說道：", &format!("【{talk_name}】說道："));
        conn.send_json(&serde_json::json!({
            "type": "interact_result",
            "observation_update": {
                "target_id": target.id,
                "new_state": "named",
                "tag": short_real_name
            }
        }));
        let role_raw = crate::store::npc_persistent_role_label(&target.id, &target.display_title);
        let role_desc = role_raw
            .split(['|', '｜'])
            .next()
            .unwrap_or(&role_raw)
            .trim()
            .to_string();
        if !already_mentioned {
            final_narrative.push_str(&format!(" 交談間，你得知對方名叫【{talk_name}】（{role_desc}）。"));
        }
    }
    if let Some(suffix) = &world_effect.narrative_suffix {
        final_narrative.push_str(suffix);
    }
    if let Some(m_note) = &mentorship_note {
        final_narrative.push_str(m_note);
    }
    if let Some(h_note) = &hunting_pledge_note {
        final_narrative.push_str(h_note);
    }
    if decision.choice != NpcStanceChoice::Refuse
        && !room_display_name.trim().is_empty()
        && (is_trade_topic || is_delivery_intent_utterance(trimmed_input))
    {
        let ep_res = if let Some((ev, _sp)) =
            crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                &room_display_name,
                Some(&real_name),
                now,
            ) {
            let mut candidate_keys: Vec<String> = Vec::new();
            for seg in ev.key_objects.split('、') {
                let s = seg.trim();
                if s.chars().count() >= 2 {
                    candidate_keys.push(s.to_string());
                }
            }
            for clean in clean_catalog_codes(ev.key_objects).split('、') {
                let s = clean.trim();
                if s.chars().count() >= 2 && !candidate_keys.iter().any(|k| k == s) {
                    candidate_keys.push(s.to_string());
                }
            }
            let mut matched_res = None;
            for k in &candidate_keys {
                if trimmed_input.contains(k.as_str()) {
                    if let Some(res) =
                        crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(
                            &room_display_name,
                            k,
                        )
                    {
                        matched_res = Some(res);
                        break;
                    }
                }
            }
            matched_res
        } else {
            None
        };
        if let Some((_pack_id, _step, ep_line)) = ep_res {
            final_narrative.push_str(&format!("\n{ep_line}"));
        }
    }
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "Talk".into(),
        target_id: target.id.clone(),
        target_name: talk_name.clone(),
        narrative: final_narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
    if should_unlock_name {
        let _ = db::set_named(pid, target_id);
        let _ = db::set_observed(pid, target_id);
        super::super::handler::send_place_view(conn, pid);
    }
    if let Some(pool) = &pool_opt {
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            target_id,
            pid,
            "talked",
            1,
            "player_talk",
            now,
        );
        let _ = crate::pg::living_world::record_relationship_delta(
            pool,
            pid,
            target_id,
            "talked",
            1,
            "player_talk",
            now,
        );
        let stance_str = match decision.choice {
            NpcStanceChoice::Accept => "Accept",
            NpcStanceChoice::Refuse => "Refuse",
            NpcStanceChoice::Deflect => "Deflect",
            NpcStanceChoice::Bargain => "Bargain",
        };
        let effects = serde_json::json!({
            "stance": stance_str,
            "confidence": decision.confidence,
            "latency_ms": decision.latency_ms,
        });
        let _ = crate::pg::living_world::record_player_world_action(
            pool,
            pid,
            "talk",
            target_id,
            player_room,
            "success",
            effects,
            now,
        );
    }
    let player_name = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.display_title)
        .unwrap_or_else(|| "遊歷客友".to_string());
    crate::npc::causal_core::mutate_npc_from_player_action(
        target_id,
        pid,
        &player_name,
        player_room,
        "talk_exchange",
        player_room,
        0,
        now,
    );
    let _ = event::append(now, pid, "talk", target_id);
    conn.sessions
        .record_player_talk_for_room_echo(pid, player_room, &msg.player_input);
    if ai_succeeded {
        flush_conversation_and_append(pid, target_id, &player_input, &npc_reply, now);
    }
    let _ = adjust_favorability(target_id, pid, FAV_TALK);
}

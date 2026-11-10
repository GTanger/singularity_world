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

const ZHULIN_COOKED_FOOD_IDS: &[&str] = &[
    "zl_item_bamboo_tube_soup",
    "zl_item_roasted_bamboo_shoot",
    "zl_item_roasted_weevil",
];

fn campfire_socket_actions_for_inventory(inv_json: &str) -> Vec<String> {
    let mut acts = vec!["Look".to_string(), "WarmHands".to_string()];
    if inv_json.contains("zl_item_bamboo_canteen_full") {
        acts.push("CampfireDrinkWater".to_string());
    }
    if ZHULIN_COOKED_FOOD_IDS.iter().any(|id| inv_json.contains(id)) {
        acts.push("CampfireEatFood".to_string());
    }
    if inv_json.contains("zl_item_bamboo_canteen")
        || (inv_json.contains("zl_item_bamboo_culm") && !inv_json.contains("zl_item_bamboo_canteen_full"))
    {
        acts.push("FillBambooCanteen".to_string());
    }
    if inv_json.contains("zl_item_aged_bamboo_slat") {
        acts.push("CampfireRoastKnife".to_string());
    }
    if inv_json.contains("zl_item_bamboo_shoot") && inv_json.contains("zl_item_bamboo_sheath") {
        acts.push("CampfireRoastShoot".to_string());
    }
    if inv_json.contains("zl_item_bamboo_weevil")
        || inv_json.contains("zl_item_bamboo_fungus")
        || inv_json.contains("zl_item_termite_mushroom")
    {
        acts.push("CampfireRoastFungus".to_string());
    }
    if inv_json.contains("zl_item_bamboo_canteen_full")
        && (inv_json.contains("zl_item_bamboo_shoot")
            || inv_json.contains("zl_item_bamboo_fungus")
            || inv_json.contains("zl_item_termite_mushroom"))
    {
        acts.push("CampfireBoilSoup".to_string());
    }
    if inv_json.contains("zl_item_bamboo_culm") {
        acts.push("CampfirePopCulm".to_string());
    }
    acts
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
    let is_campfire_socket = matches!(
        action,
        "WarmHands"
            | "CampfireDrinkWater"
            | "CampfireEatFood"
            | "FillBambooCanteen"
            | "CampfireRoastKnife"
            | "CampfireRoastShoot"
            | "CampfireRoastFungus"
            | "CampfireBoilSoup"
            | "CampfirePopCulm"
    );
    let is_bamboo_look = action == "Look" && obj.id == "zl_bamboo_campfire";
    if action != "Gather"
        && action != "Harvest"
        && action != "Take"
        && action != "CarveMark"
        && action != "ClearBambooTrail"
        && action != "IgniteCampfire"
        && !is_campfire_socket
        && !is_bamboo_look
    {
        return false;
    }

    let has_target_obj = crate::server::grid_manager::get_runtime_grid_cell(coord)
        .map(|c| c.objects.iter().any(|o| o.id == obj.id || o.name == obj.name))
        .unwrap_or(false);
    if !has_target_obj {
        return false;
    }

    let player_inv_json = db::get_entity(pid)
        .ok()
        .flatten()
        .map(|e| e.inventory)
        .unwrap_or_default();

    if is_bamboo_look {
        let narrative = obj
            .responses
            .get("Look")
            .cloned()
            .unwrap_or_else(|| "林下燃著一簇劈啪作響的竹葉營火，紅熱竹炭正散發著乾燥暖意。".to_string());
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success: true,
            actions: campfire_socket_actions_for_inventory(&player_inv_json),
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

    if is_campfire_socket {
        if action == "WarmHands" {
            let mut warmed_low_vit = false;
            if let Ok(Some(ent)) = db::get_entity(pid) {
                if ent.vit < 45 {
                    let _ = db::update_vit(pid, (ent.vit + 10).min(45));
                    warmed_low_vit = true;
                }
            }
            let narrative = if warmed_low_vit {
                "你湊近「竹葉營火」伸出雙手烘火，竹葉與竹炭散發的乾燥暖流驅散了林間寒濕，略微回升了受寒消耗的氣力。".to_string()
            } else {
                "你湊近「竹葉營火」伸手取暖，掌心與四肢暖融舒暢；惟腹中飢渴仍須飲水進食方能補足充沛氣力。".to_string()
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success: true,
                actions: campfire_socket_actions_for_inventory(&player_inv_json),
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            return true;
        }
        if action == "CampfireDrinkWater" {
            let (success, narrative) = if db::remove_from_inventory(pid, "zl_item_bamboo_canteen_full", 1).is_ok() {
                let _ = db::add_to_inventory(pid, "zl_item_bamboo_canteen", 1);
                if let Some(arc) = crate::store::get_store() {
                    let _ = arc.write().update_entity(pid, |e| {
                        e.vit = (e.vit + 12).min(100);
                        e.qi = (e.qi + 5).min(100);
                    });
                }
                (
                    true,
                    "你坐在竹葉營火旁拔開竹葉塞，仰頭暢飲「盛水青竹筒」中的甘冽泉露，唇齒生津、精神一振，並將空出的「青竹水筒」收回行囊備用。".to_string(),
                )
            } else {
                (false, "行囊中尚無「盛水青竹筒」可供飲用。".to_string())
            };
            let new_inv = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.inventory)
                .unwrap_or_default();
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success,
                actions: campfire_socket_actions_for_inventory(&new_inv),
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
            return true;
        }
        if action == "CampfireEatFood" {
            let chosen_id = ZHULIN_COOKED_FOOD_IDS
                .iter()
                .copied()
                .find(|id| player_inv_json.contains(id));
            let (success, narrative) = if let Some(food_id) = chosen_id {
                if db::remove_from_inventory(pid, food_id, 1).is_ok() {
                    let (dvit, dqi, text) = match food_id {
                        "zl_item_bamboo_tube_soup" => {
                            let _ = db::add_to_inventory(pid, "zl_item_bamboo_canteen", 1);
                            (
                                25,
                                15,
                                "你坐在竹葉營火旁捧起溫熱竹筒，仰頭飲盡「竹筒山珍湯」，山泉、竹膜與野菌的鮮甜精華順喉而下，渾身寒氣與疲憊一掃而空！（空出的「青竹水筒」已收回行囊）".to_string(),
                            )
                        }
                        "zl_item_roasted_bamboo_shoot" => (
                            18,
                            8,
                            "你坐在竹葉營火旁剝開焦香微裂的竹籜，趁熱咬下清甜脆嫩的「籜裹煨竹筍」，四肢百骸頓時湧起充沛氣力！".to_string(),
                        ),
                        _ => (
                            15,
                            6,
                            "你坐在竹葉營火旁嚼碎金紅酥脆的「焦香烤筍龜」，濃郁脂香在口中化開，迅速補充了山林跋涉消耗的體能！".to_string(),
                        ),
                    };
                    if let Some(arc) = crate::store::get_store() {
                        let _ = arc.write().update_entity(pid, move |e| {
                            e.vit = (e.vit + dvit).min(100);
                            e.qi = (e.qi + dqi).min(100);
                        });
                    }
                    (true, text)
                } else {
                    (false, "行囊中尚無可享用的野炊熟食。".to_string())
                }
            } else {
                (false, "行囊中尚無可享用的野炊熟食。".to_string())
            };
            let new_inv = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.inventory)
                .unwrap_or_default();
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success,
                actions: campfire_socket_actions_for_inventory(&new_inv),
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
            return true;
        }
        if action == "FillBambooCanteen" {
            let mut jit_prefix = String::new();
            if !player_inv_json.contains("zl_item_bamboo_canteen")
                && player_inv_json.contains("zl_item_bamboo_culm")
            {
                if let Some(Ok(_)) = crate::grid::zhulin::try_craft_zhulin_item(pid, "zl_item_bamboo_culm:canteen_spade") {
                    jit_prefix = "你就地將一節「刺竹稈」裁出「青竹水筒」與「掘土竹鏟」，隨即".to_string();
                }
            }
            let (success, narrative) = match crate::grid::zhulin::try_craft_zhulin_item(pid, "zl_item_bamboo_canteen") {
                Some(Ok((_act, _title, msg))) => (true, format!("{}{}", jit_prefix, msg)),
                Some(Err(err_msg)) => (false, err_msg),
                None => (false, "手頭尚缺「青竹水筒」或「刺竹稈」，無法盛水。".to_string()),
            };
            let new_inv = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.inventory)
                .unwrap_or_default();
            let post_actions = if obj.id == "zl_bamboo_campfire" {
                campfire_socket_actions_for_inventory(&new_inv)
            } else {
                vec![
                    "Look".into(),
                    "Gather".into(),
                    "ClearBambooTrail".into(),
                    "CarveMark".into(),
                    "FillBambooCanteen".into(),
                ]
            };
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative,
                success,
                actions: post_actions,
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            push_refresh(conn, pid);
            refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
            return true;
        }
        let recipe_key = match action {
            "CampfireRoastKnife" => "zl_item_aged_bamboo_slat",
            "CampfireRoastShoot" => "zl_item_bamboo_shoot:roast",
            "CampfireRoastFungus" => {
                if player_inv_json.contains("zl_item_bamboo_weevil") {
                    "zl_item_bamboo_weevil"
                } else if player_inv_json.contains("zl_item_termite_mushroom") {
                    "zl_item_termite_mushroom:roast"
                } else {
                    "zl_item_bamboo_fungus:roast"
                }
            }
            "CampfireBoilSoup" => {
                if player_inv_json.contains("zl_item_termite_mushroom") {
                    "zl_item_termite_mushroom:soup"
                } else if player_inv_json.contains("zl_item_bamboo_fungus") {
                    "zl_item_bamboo_fungus:soup"
                } else {
                    "zl_item_bamboo_shoot:soup"
                }
            }
            "CampfirePopCulm" => "zl_item_bamboo_culm:fire_pop",
            _ => "",
        };
        let (success, narrative) = match crate::grid::zhulin::try_craft_zhulin_item(pid, recipe_key) {
            Some(Ok((_act, _title, msg))) => (true, msg),
            Some(Err(err_msg)) => (false, err_msg),
            None => (false, "手頭尚缺對應食材或器物，無法在竹葉營火上施作。".to_string()),
        };
        let new_inv = db::get_entity(pid)
            .ok()
            .flatten()
            .map(|e| e.inventory)
            .unwrap_or_default();
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success,
            actions: campfire_socket_actions_for_inventory(&new_inv),
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        return true;
    }

    if action == "IgniteCampfire" {
        let mut ignite_res: Result<String, String> = Err("此處無乾竹葉可供引火。".to_string());
        let mut cell_snapshot = None;
        crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(coord) {
                let (w_def, _) = crate::grid::zhulin::weather::current_world_zhulin_weather();
                let leaves_qty = cell
                    .objects
                    .iter()
                    .find(|o| o.id == "zl_dry_leaves" || o.name == "乾竹葉層")
                    .map(|o| o.quantity as i32)
                    .unwrap_or(0);
                match crate::grid::zhulin::evaluate_fire_weather_shelter(
                    w_def.tags.attribute_tags,
                    &cell.tags,
                    false,
                    leaves_qty.max(2),
                ) {
                    Err(msg) => {
                        ignite_res = Err(msg);
                    }
                    Ok(shelter_note) => {
                        if let Some(leaves_obj) = cell
                            .objects
                            .iter_mut()
                            .find(|o| o.id == "zl_dry_leaves" || o.name == "乾竹葉層")
                        {
                            let cost = leaves_obj.quantity.min(2);
                            leaves_obj.quantity = leaves_obj.quantity.saturating_sub(cost);
                        }
                        if !cell.has_room_tag("zl_campfire:active") {
                            cell.tags.push("zl_campfire:active".to_string());
                        }
                        if let Some(cf) = cell
                            .objects
                            .iter_mut()
                            .find(|o| o.id == crate::grid::zhulin::campfire::ID)
                        {
                            cf.quantity = cf.quantity.max(3);
                        } else {
                            cell.objects
                                .push(crate::grid::zhulin::campfire::build_grid_object(3));
                        }
                        crate::grid::zhulin::refresh_bamboo_cell_after_gather(cell);
                        cell_snapshot = Some((cell.explored, cell.freshness, cell.objects.clone()));
                        let prefix = shelter_note.unwrap_or("");
                        ignite_res = Ok(format!(
                            "{}你就地攏起乾燥竹葉與枯枝引火，火苗劈啪竄起，在林下燃起一簇溫暖的「竹葉營火」！點擊主畫面上的【竹葉營火】即可直接烘火、烤刀、煨筍或煲湯。",
                            prefix
                        ));
                    }
                }
            }
        });
        if let Some((exp, fresh, objs)) = cell_snapshot {
            let _ = db::update_cell_state(coord.x, coord.y, exp, fresh, &objs);
            if let Ok(Some(ent)) = db::get_entity(pid) {
                let _ = db::update_vit(pid, (ent.vit + 12).min(100));
            }
        }
        let (success, narrative) = match ignite_res {
            Ok(s) => (true, s),
            Err(e) => (false, e),
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success,
            actions: vec!["Look".into(), "Gather".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        return true;
    }

    if action == "ClearBambooTrail" {
        let mut player_inv_json = player_inv_json;
        let mut jit_prefix = String::new();
        if !crate::grid::zhulin::inventory_has_capability_tag(&player_inv_json, "heavy_chop") {
            if player_inv_json.contains("zl_item_bamboo_rhizome") {
                if let Some(Ok((_id, crafted_name, _msg))) =
                    crate::grid::zhulin::try_craft_zhulin_item(pid, "zl_item_bamboo_rhizome")
                {
                    jit_prefix = format!("你就地取行囊材料紮妥一把「{}」，隨即", crafted_name);
                    player_inv_json = db::get_entity(pid)
                        .ok()
                        .flatten()
                        .map(|e| e.inventory)
                        .unwrap_or_default();
                }
            }
        }
        if !crate::grid::zhulin::inventory_has_capability_tag(&player_inv_json, "heavy_chop") {
            conn.send_json(&ActionResultMsg {
                msg_type: "action_result".into(),
                action: action.into(),
                target_id: obj.id.clone(),
                target_name: obj.name.clone(),
                narrative: "合抱刺竹叢竹竿粗硬如鐵、刺枝交錯，徒手或單憑薄刃難以開路；需持有「竹鞭根槌」或「籜繩竹鞭石斧」方能一舉劈開竹障！".to_string(),
                success: false,
                actions: vec!["Look".into(), "Gather".into(), "ClearBambooTrail".into(), "CarveMark".into()],
                move_target_id: String::new(),
                open_trade: false,
                suggested_action: String::new(),
                tactical_choices: vec![],
            });
            return true;
        }
        let has_stone_axe = player_inv_json.contains("zl_item_bamboo_stone_axe");
        let harvest_culms = if has_stone_axe { 3 } else { 2 };
        let mut cell_snapshot = None;
        crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(coord) {
                if let Some(target_obj) = cell
                    .objects
                    .iter_mut()
                    .find(|o| o.id == "zl_bamboo_culm" || o.name == "合抱刺竹叢")
                {
                    target_obj.quantity = 0;
                }
                crate::grid::zhulin::record_bamboo_gather_pressure(cell);
                crate::grid::zhulin::refresh_bamboo_cell_after_gather(cell);
                cell_snapshot = Some((cell.explored, cell.freshness, cell.objects.clone()));
            }
        });
        if let Some((exp, fresh, objs)) = cell_snapshot {
            let _ = db::update_cell_state(coord.x, coord.y, exp, fresh, &objs);
            let _ = db::add_to_inventory(pid, "zl_item_bamboo_culm", harvest_culms);
        }
        let tool_desc = if has_stone_axe {
            "揮動「籜繩竹鞭石斧」順著竹節一氣呵成連番斜劈"
        } else {
            "掄起「竹鞭根槌」順著竹節猛力砸斷交錯刺枝"
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative: format!(
                "{}你{}，將擋路的「合抱刺竹叢」連根伐開，闢出一條透光通風的【疏竹小徑】，並順手收起 {} 根筆直的「刺竹稈」入袋，林下也隨之落滿厚實乾竹葉！",
                jit_prefix, tool_desc, harvest_culms
            ),
            success: true,
            actions: vec!["Look".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        return true;
    }

    if action == "CarveMark" {
        let mut carve_res = Err("無法在此處刻記號".to_string());
        let mut updated = false;
        let mut cell_snapshot = None;
        crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(coord) {
                carve_res = crate::grid::zhulin::carve_bamboo_trail_mark(cell, &player_inv_json);
                if carve_res.is_ok() {
                    updated = true;
                    cell_snapshot = Some((cell.explored, cell.freshness, cell.objects.clone()));
                }
            }
        });
        if updated {
            if let Some((exp, fresh, objs)) = cell_snapshot {
                let _ = db::update_cell_state(coord.x, coord.y, exp, fresh, &objs);
            }
        }
        let (success, narrative) = match carve_res {
            Ok(s) => (true, s),
            Err(e) => (false, e),
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success,
            actions: vec!["Look".into(), "Gather".into(), "ClearBambooTrail".into(), "CarveMark".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        push_refresh(conn, pid);
        refresh_room_views_for_room(&conn.sessions, &conn.cfg, player_room);
        return true;
    }

    let mut player_inv_json = player_inv_json;
    let mut jit_gather_prefix = String::new();
    if obj.id == "zl_bamboo_culm"
        && !crate::grid::zhulin::inventory_has_capability_tag(&player_inv_json, "heavy_chop")
        && player_inv_json.contains("zl_item_bamboo_rhizome")
    {
        if let Some(Ok((_id, crafted_name, _msg))) =
            crate::grid::zhulin::try_craft_zhulin_item(pid, "zl_item_bamboo_rhizome")
        {
            jit_gather_prefix = format!("你就地取行囊材料紮妥一把「{}」，隨即", crafted_name);
            player_inv_json = db::get_entity(pid)
                .ok()
                .flatten()
                .map(|e| e.inventory)
                .unwrap_or_default();
        }
    }

    let mut updated_state = None;
    let mut zero_quantity = false;
    let mut blocked_narrative: Option<String> = None;
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(coord) {
            if let Some(target_obj) = cell.objects.iter_mut().find(|o| o.id == obj.id || o.name == obj.name) {
                if target_obj.quantity > 0 {
                    match crate::grid::zhulin::evaluate_zhulin_gather(
                        &target_obj.id,
                        target_obj.quantity,
                        &player_inv_json,
                    ) {
                        crate::grid::zhulin::GatherCheckResult::Blocked { narrative } => {
                            blocked_narrative = Some(narrative);
                        }
                        crate::grid::zhulin::GatherCheckResult::Allowed {
                            stock_cost,
                            yield_qty,
                            custom_narrative,
                        } => {
                            let is_batch_hand_gather = (target_obj.id == "zl_dry_leaves"
                                || target_obj.id == "zl_bamboo_sheath")
                                && target_obj.quantity >= 2;
                            let eff_cost = if is_batch_hand_gather { 2 } else { stock_cost };
                            let eff_yield = if is_batch_hand_gather { 2 } else { yield_qty };
                            target_obj.quantity = target_obj.quantity.saturating_sub(eff_cost);
                            let remaining_qty = target_obj.quantity;
                            let obj_name = target_obj.name.clone();
                            let obj_id = target_obj.id.clone();
                            crate::grid::zhulin::record_bamboo_gather_pressure(cell);
                            updated_state = Some((
                                cell.explored,
                                cell.freshness,
                                cell.objects.clone(),
                                remaining_qty,
                                obj_name,
                                obj_id,
                                eff_yield,
                                custom_narrative,
                            ));
                        }
                    }
                } else {
                    zero_quantity = true;
                }
            }
        }
    });

    if let Some(narrative) = blocked_narrative {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: action.into(),
            target_id: obj.id.clone(),
            target_name: obj.name.clone(),
            narrative,
            success: false,
            actions: vec!["Look".into(), "Gather".into()],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

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

    let Some((
        explored,
        freshness,
        updated_objects,
        remaining_qty,
        _obj_name,
        obj_id,
        actual_harvested,
        custom_narrative,
    )) = updated_state
    else {
        return false;
    };

    let _ = db::update_cell_state(coord.x, coord.y, explored, freshness, &updated_objects);

    let Some(item_id) = crate::grid::zhulin::yield_item_id_for_object(&obj_id) else {
        conn.send_error("此地景物件無法直接採集");
        return true;
    };

    let final_remaining_qty = remaining_qty;
    let _ = db::add_to_inventory(pid, item_id, actual_harvested as i32);

    let actions = if final_remaining_qty > 0 {
        vec!["Look".into(), "Gather".into()]
    } else {
        vec!["Look".into()]
    };

    let mut narrative = if let Some(custom) = custom_narrative {
        custom
    } else if let Some(custom) = crate::grid::zhulin::gather_narrative_for_object(
        &obj_id,
        actual_harvested,
        final_remaining_qty,
    ) {
        custom
    } else {
        let clean_item = crate::game::object_ecology::get_clean_item_name(item_id);
        format!(
            "你自「{}」處細心採擷，將 {} 份「{}」收入行囊之中。",
            obj.name, actual_harvested, clean_item
        )
    };
    if !jit_gather_prefix.is_empty() {
        narrative = format!("{}{}", jit_gather_prefix, narrative);
    }
    let has_stone_axe = player_inv_json.contains("zl_item_bamboo_stone_axe");
    let has_rhizome_club = player_inv_json.contains("zl_item_rhizome_club");
    let has_bamboo_spade = player_inv_json.contains("zl_item_bamboo_spade");
    if obj_id == "zl_bamboo_culm" {
        if has_stone_axe {
            let _ = db::add_to_inventory(pid, item_id, 1);
            narrative.push_str("（手中「籜繩竹鞭石斧」石刃鋒利，順著竹節斜劈而下，一舉多伐得 1 根刺竹稈！）");
        } else if has_rhizome_club {
            narrative.push_str("（以純竹根槌砸節砍伐頗費氣力；若點選【劈竹開路】可一舉伐開竹障，或配銳邊石片紮成「籜繩竹鞭石斧」倍增產出）");
        }
    } else if obj_id == "zl_bamboo_rhizome" && has_bamboo_spade {
        let _ = db::add_to_inventory(pid, item_id, 1);
        narrative.push_str("（厚壁「掘土竹鏟」順著紅土縫隙深鏟斷根，額外多掘出 1 截完整刺竹鞭！）");
    }
    let weevil_mod = if has_bamboo_spade { 2 } else { 3 };
    if obj_id == "zl_bamboo_shoot" && final_remaining_qty > 0 && (coord.x + coord.y + final_remaining_qty as i32).abs() % weevil_mod == 0 {
        let _ = db::add_to_inventory(pid, "zl_item_bamboo_weevil", 1);
        narrative.push_str("翻挖筍殼間，你還順手抓到一隻肥美的「筍龜」收入行囊！");
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

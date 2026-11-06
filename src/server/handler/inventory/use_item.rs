pub fn is_item_edible(item_id: &str, item_name: &str, item_type: &str) -> bool {
    if item_id == "raw_meat"
        || item_id == "wt_down_feather"
        || item_id == "wt_beast_bone"
        || item_name.contains("生獸肉")
        || item_name.contains("絨羽")
        || item_name.contains("獸骨")
    {
        return false;
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        if cat.socket == "Taste"
            || cat.attribute_tags.contains(&"food")
            || cat.attribute_tags.contains(&"drink")
            || cat.attribute_tags.contains(&"edible")
            || cat.category_name.contains('食')
            || cat.category_name.contains('飲')
            || cat.category_name.contains('茶')
            || cat.category_name.contains('膳')
        {
            return true;
        }
    }
    item_type == "food"
        || item_type == "consumable"
        || item_id.starts_with("food_")
        || item_id.starts_with("meal_")
        || item_id.starts_with("tea_")
        || item_id.starts_with("drink_")
        || matches!(
            item_id,
            "mat_fresh_shrimp" | "mat_salted_fish" | "mat_wild_berries"
        )
        || item_name.contains("茶")
        || item_name.contains("湯")
        || item_name.contains("麵")
        || item_name.contains("飯")
        || item_name.contains("餅")
        || item_name.contains("粥")
        || item_name.contains("肉脯")
        || item_name.contains("魚乾")
        || item_name.contains("蝦串")
}

/// 依千物總典類別碼與百工資料源，結構化判定行囊物品之階段（`mutated` / `raw` / `finished`）、可否手作與手作按鈕標籤。
pub fn classify_inventory_item_capabilities(
    item_id: &str,
    item_name: &str,
    item_type: &str,
    slot: &str,
) -> (String, bool, String) {
    let is_mutated = item_id.starts_with("mut_")
        || item_name.contains("【異變")
        || crate::game::object_ecology::classify_item_stage(item_id) == "mutated";
    if is_mutated {
        return ("mutated".to_string(), false, String::new());
    }
    let is_crafted_finished = item_id.starts_with("craft_")
        || item_id.starts_with("manual_")
        || item_id.starts_with("item_")
        || item_id == "blank_manual"
        || item_id == "wt_blank_manual"
        || item_id.starts_with("corpse_")
        || !slot.is_empty()
        || item_type == "tool"
        || item_type == "equipment"
        || item_type == "manual"
        || item_type == "corpse"
        || item_name.contains("精鍛");
    let is_catalog_raw = crate::game::peninsula_catalog::find_catalog_entry(item_id).is_some_and(|cat| {
        matches!(
            cat.category_num.trim(),
            "01" | "1" | "02" | "2" | "03" | "3" | "04" | "4" | "05" | "5" | "06" | "6" | "07" | "7"
        )
    });
    let is_raw = !is_crafted_finished
        && (is_catalog_raw
            || item_type == "material"
            || item_type == "raw_material"
            || crate::game::object_ecology::classify_item_stage(item_id) == "raw");
    if !is_raw {
        return ("finished".to_string(), false, String::new());
    }

    let is_non_culinary_part = [
        "魚骨", "魚鱗", "魚皮", "釣魚線", "魚線", "漁線", "魚網", "漁網", "螺殼", "貝殼", "茶籽",
    ]
    .iter()
    .any(|kw| item_name.contains(kw))
        || ["fishing_line", "fish_bone", "fish_scale", "shell_sack"]
            .iter()
            .any(|kw| item_id.contains(kw));
    let is_culinary_raw = !is_non_culinary_part
        && (crate::game::peninsula_catalog::find_catalog_entry(item_id).is_some_and(|cat| {
            matches!(cat.category_num.trim(), "05" | "5" | "06" | "6" | "07" | "7")
                || cat.attribute_tags.contains(&"food")
                || cat.attribute_tags.contains(&"edible")
        }) || ["shrimp", "fish", "meat", "grain", "berries"]
            .iter()
            .any(|kw| item_id.contains(kw))
            || [
                "蝦", "魚", "蟹", "鮑", "蛤", "蜆", "螺", "肉", "熊掌", "鹿腱", "牛蛙", "老鱉",
                "稻穀", "穀糧", "漿果", "鮮果", "茶", "水芹",
            ]
            .iter()
            .any(|kw| item_name.contains(kw)));
    let craft_label = if is_culinary_raw {
        "烹調炙烤".to_string()
    } else {
        "加工煉製".to_string()
    };
    ("raw".to_string(), true, craft_label)
}
pub(super) fn handle_use_item(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    if msg.item_id.is_empty() {
        conn.send_error(gametext::client("inv_no_item"));
        return;
    }

    if handle_use_manual_action(conn, msg, &pid) {
        return;
    }

    // ─── 舊法攔截（InscribeItem, StackWord, MintWord, StripName）───────────
    if matches!(
        msg.action.as_str(),
        "InscribeItem" | "StackWord" | "MintWord" | "ExtractWord"
    ) {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
        return;
    }
    if msg.action == "StripName" {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
        return;
    }
    if msg.action == "StripNameWhole" && !msg.item_id.starts_with("corpse_") {
        conn.send_error("此項舊法已不再通行，請直接運用器物或執行百工工序。");
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
    // ─── 舊版晶石使用清理 ───────────────────────────────────────────────────
    if crate::combat::wordplate::parse_crystal_item_id(&msg.item_id).is_some() {
        let _ = remove_from_inventory(&pid, &msg.item_id, 1);
        push_refresh(conn, &pid);
        return;
    }

    if handle_use_corpse_action(conn, msg, &pid) {
        return;
    }

    if let Some((tag, vit_gain, qi_gain, consume, effect_narr)) = crate::game::shanhai_item_use_effect(&msg.item_id) {
        if consume && remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
            conn.send_error(gametext::client("inv_bag_fail"));
            return;
        }
        if let Some(arc) = crate::store::get_store() {
            let _ = arc.write().update_entity(&pid, |e| {
                e.vit = (e.vit + vit_gain).min(100);
                e.qi = (e.qi + qi_gain).min(100);
            });
        }
        crate::pg::living_world::clear_entity_afflictions(&pid, true, crate::game::now_unix());
        push_refresh(conn, &pid);
        let item_name = resolve_inv_item_name(&msg.item_id);
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
            target_id: msg.item_id.clone(),
            target_name: item_name,
            narrative: format!("【{tag}】{effect_narr}（山海靈韻化入肺腑，氣血隨之充盈）"),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    let item_def = get_item_def(&msg.item_id);
    let item_name = format_entity_item_with_inscription(&pid, &msg.item_id, &resolve_inv_item_name(&msg.item_id));
    let is_non_edible_gear_or_mat = item_def
        .as_ref()
        .is_some_and(|d| !d.slot.is_empty() || d.item_type == "equipment" || d.item_type == "misc" || d.item_type == "tool")
        || matches!(
            msg.item_id.as_str(),
            "mat_fishing_line_bundle"
                | "mat_fish_scale_sack"
                | "mat_fish_bone_sack"
                | "mat_sealing_pitch"
                | "item_ebony_mortise_tea_tray"
        )
        || [
            "魚骨", "魚鱗", "魚皮", "釣魚線", "魚線", "漁線", "魚網", "漁網", "魚簍", "魚刀", "魚鰾",
            "螺殼", "貝殼", "蟹殼", "封縫膏", "灰膏", "茶盤", "茶壺", "茶盅", "茶篩", "茶托", "茶籽",
            "湯鍋", "湯碗", "果盤", "螺鈿",
        ]
        .iter()
        .any(|kw| item_name.contains(kw));
    let is_edible = !is_non_edible_gear_or_mat
        && (item_def.as_ref().is_some_and(|d| d.item_type == "consumable" || d.item_type == "food")
            || matches!(
                msg.item_id.as_str(),
                "mat_fresh_shrimp"
                    | "shanhai_fresh_fish"
                    | "shanhai_beast_meat"
                    | "mat_salted_fish"
                    | "harvest_fish"
                    | "harvest_berries"
                    | "craft_roasted_shrimp_skewer"
                    | "craft_smoked_fish_fillet"
                    | "craft_spiced_beast_jerky"
                    | "craft_ration_grain_cake"
                    | "craft_meridian_salve"
                    | "craft_calming_tea_pack"
            )
            || ["蝦", "魚", "肉", "蟹", "鮑", "蛤", "蜆", "螺", "果", "茶", "餅", "脯", "串", "湯", "粥", "飲", "露", "膏", "散", "丹", "丸", "水芹", "芝"]
                .iter()
                .any(|kw| item_name.contains(kw)));
    if !is_edible {
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
            target_id: msg.item_id.clone(),
            target_name: item_name.clone(),
            narrative: format!("【{item_name}】屬於百工原材或隨身器物，無法直接食用；請於行囊點擊「加工煉製」或「穿戴」。"),
            success: false,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return;
    }
    if remove_from_inventory(&pid, &msg.item_id, 1).is_err() {
        conn.send_error(gametext::client("inv_bag_fail"));
        return;
    }
    let is_meridian_salve = msg.item_id == "craft_meridian_salve" || msg.item_id.contains("meridian_salve");
    let is_calming_tea = msg.item_id == "craft_calming_tea_pack" || msg.item_id.contains("calming_tea");
    let is_culinary = matches!(
        msg.item_id.as_str(),
        "craft_roasted_shrimp_skewer"
            | "craft_smoked_fish_fillet"
            | "craft_spiced_beast_jerky"
            | "craft_ration_grain_cake"
            | "mat_salted_fish"
    );
    let is_mutated = msg.item_id.starts_with("mut_") || crate::game::object_ecology::classify_item_stage(&msg.item_id) == "mutated";
    if let Some(arc) = crate::store::get_store() {
        let _ = arc.write().update_entity(&pid, |e| {
            if is_meridian_salve {
                e.vit = (e.vit + 25).min(100);
                e.qi = (e.qi + 20).min(100);
            } else if is_calming_tea {
                e.qi = (e.qi + 30).min(100);
            } else if is_culinary {
                e.vit = (e.vit + 20).min(100);
                e.qi = (e.qi + 18).min(100);
            } else if is_mutated {
                e.vit = (e.vit + 15).min(100);
                e.qi = (e.qi + 20).min(100);
            } else {
                e.vit = (e.vit + 8).min(100);
                e.qi = (e.qi + 10).min(100);
            }
        });
    }
    if is_meridian_salve || is_calming_tea || is_mutated {
        crate::pg::living_world::clear_entity_afflictions(&pid, true, crate::game::now_unix());
    }
    push_refresh(conn, &pid);

    // item_name already bound above
    let bracketed_name = if item_name.starts_with('【') {
        item_name.clone()
    } else {
        format!("【{item_name}】")
    };

    let is_food = ["餅", "麵", "飯", "肉", "薯", "凍", "花", "菜", "蝦", "蛤", "魚", "丸", "糖", "湯", "粥", "粿"]
        .iter()
        .any(|k| item_name.contains(k));
    let is_drink = ["茶", "漿", "飲", "水", "酒", "汁", "露"]
        .iter()
        .any(|k| item_name.contains(k));
    let is_herb = ["草", "葉", "根", "果", "藥", "丹", "膏", "散"]
        .iter()
        .any(|k| item_name.contains(k));

    let narrative = if is_meridian_salve {
        format!("你取出一份{bracketed_name}塗抹於周身經絡要穴。溫潤的藥力順著經脈化開，氣血大暢，肌肉酸痛盡消（周身氣血與心神皆獲豐沛滋養）！")
    } else if is_calming_tea {
        format!("你沏開這份{bracketed_name}細細品飲。茶湯清洌甘甜，清氣透入眉宇，心神澄澈通達（心神頓時澄澈充盈）！")
    } else if is_mutated {
        format!("你凝神運用手中神異的{bracketed_name}。微芒順著指尖湧入經脈，天地共鳴之氣充盈四肢百骸，周身氣脈頓時大振（體魄與元氣漸次平順舒暢）！")
    } else if is_food {
        format!("你取出一份熱騰騰的{bracketed_name}，大口趁熱品嚐。外皮酥香、內餡鮮美多汁，溫潤的香氣自舌尖散開，頓時腹中一暖，整個人精神大振！")
    } else if is_drink {
        format!("你仰頭啜飲這份{bracketed_name}。甘潤清甜的湯水順喉而下，齒頰留香，周身燥熱頓消，氣息隨之舒暢通達。")
    } else if is_herb {
        format!("你細細研服這份{bracketed_name}。草木清香在口中化開，微苦回甘，藥力順著經脈緩緩化開，氣血漸漸充盈。")
    } else {
        format!("你取出手中的{bracketed_name}仔細檢視並加以使用。觸感紮實細膩，工藝純熟，片刻調理後整個人氣定神閒。")
    };

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: if msg.action.is_empty() { "Use".into() } else { msg.action.clone() },
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


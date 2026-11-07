/// 玩家或工匠以原料煉製成品，並依條件機率性觸發異變。
pub fn craft_from_raw_material(
    actor_id: &str,
    raw_item_id: &str,
    room_name: &str,
    now_unix: i64,
) -> Result<(String, String, bool, String), String> {
    let Ok(Some(ent)) = db::get_entity(actor_id) else {
        return Err("角色不存在".to_string());
    };
    let inv = db::get_inventory(&ent.inventory, ent.vit);

    let chosen_raw_id = if raw_item_id == "auto" || raw_item_id.trim().is_empty() {
        inv.items
            .iter()
            .find(|it| {
                classify_item_stage(&it.item_id) == "raw"
                    && !matches!(
                        map_raw_to_finished(&it.item_id).id,
                        "craft_roasted_shrimp_skewer"
                            | "craft_smoked_fish_fillet"
                            | "craft_spiced_beast_jerky"
                            | "craft_ration_grain_cake"
                    )
            })
            .or_else(|| inv.items.iter().find(|it| classify_item_stage(&it.item_id) == "raw"))
            .map(|it| it.item_id.clone())
            .ok_or_else(|| {
                "行囊中尚無可用於煉製的原料（可前往河岸、藥圃、碼頭採集，或向攤商收購原料）。".to_string()
            })?
    } else {
        if !db::inventory_has_item(&ent.inventory, raw_item_id) {
            return Err("行囊中未持有指定的原料。".to_string());
        }
        raw_item_id.to_string()
    };

    let mut raw_name = get_clean_item_name(&chosen_raw_id);
    if raw_name.is_ascii() {
        raw_name = crate::world::materials::material_by_item_id(&chosen_raw_id)
            .map(|m| m.name.clone())
            .or_else(|| crate::game::peninsula_catalog::find_catalog_entry(&chosen_raw_id).map(|c| c.name.to_string()))
            .unwrap_or_else(|| "行囊物產".to_string());
        if raw_name.is_ascii() {
            raw_name = "行囊物產".to_string();
        }
    }

    if db::remove_from_inventory(actor_id, &chosen_raw_id, 1).is_err() {
        return Err("扣減原料失敗".to_string());
    }

    let finished_def = map_raw_to_finished(&chosen_raw_id);

    let has_furnace_heat = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.get(room_name).copied().unwrap_or(0) > 0
    };
    let has_charcoal = db::inventory_has_item(&ent.inventory, "mat_charcoal_sack");

    // 檢查異變機率：基礎 35%，工坊或高元氣時提升至 55%，高溫工爐或持有白炭時大幅躍升至 78%
    let is_workshop = room_name.contains("鐵坊")
        || room_name.contains("木作")
        || room_name.contains("印行")
        || room_name.contains("藥房")
        || room_name.contains("織造")
        || room_name.contains("觀星")
        || room_name.contains("工坊")
        || room_name.contains("作坊");
    let threshold = if has_furnace_heat || has_charcoal { 14 } else if is_workshop { 10 } else { 4 };

    let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);
    let is_culinary_craft = finished_def.item_type == "food"
        || matches!(
            finished_def.id,
            "craft_roasted_shrimp_skewer"
                | "craft_smoked_fish_fillet"
                | "craft_spiced_beast_jerky"
                | "craft_ration_grain_cake"
                | "craft_calming_tea_pack"
        )
        || finished_def.id.contains("tea")
        || finished_def.id.contains("fish")
        || finished_def.id.contains("shrimp")
        || finished_def.id.contains("jerky")
        || finished_def.id.contains("grain")
        || finished_def.id.contains("dish")
        || finished_def.id.contains("soup")
        || room_name.contains("廚")
        || room_name.contains("灶")
        || room_name.contains("食")
        || chosen_raw_id.contains("shrimp")
        || chosen_raw_id.contains("fish")
        || chosen_raw_id.contains("meat")
        || chosen_raw_id.contains("grain")
        || chosen_raw_id.contains("tea");

    let relevant_craft = if is_culinary_craft {
        craft_dims.culinary
    } else {
        craft_dims.forge
    };

    let venue_for_mentor = if !room_name.is_empty() {
        crate::grid::resolve_any_room_venue_object(room_name, "")
    } else {
        None
    };

    // 預先研判工藝分類以記錄吐納動態並取得內息加成
    let (habit_kind, mentor_name, imprint_title, imprint_prose, slot_label, carrier_code, formula_word_ids, formula_word_chars, station_tool_name) = if !is_culinary_craft && (finished_def.id.contains("steel")
        || finished_def.id.contains("iron")
        || finished_def.id.contains("metal")
        || finished_def.id.contains("pick")
        || finished_def.id.contains("knife")
        || finished_def.slot == "weapon"
        || room_name.contains("鐵")
        || chosen_raw_id.contains("iron")
        || chosen_raw_id.contains("metal")
        || chosen_raw_id.contains("stone")
        || chosen_raw_id.contains("礦"))
    {
        let mentor_name = if room_name.contains("岑") || room_name.is_empty() {
            "岑鐵匠"
        } else if let Some(v) = &venue_for_mentor {
            v.default_host_title
        } else {
            "岑鐵匠"
        };
        (
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            mentor_name,
            "岑氏沉腕落錘法",
            "鐵砧火花明滅間沉腕落錘，將剛猛勁力勻入器胚，剛柔兼濟",
            "重手",
            "blunt",
            vec!["gang".to_string(), "chen_tai".to_string(), "zhong".to_string()],
            vec!["\u{525b}".to_string(), "沉".to_string(), "重".to_string()],
            "鍛鐵重錘",
        )
    } else if finished_def.item_type == "food"
        || matches!(
            finished_def.id,
            "craft_roasted_shrimp_skewer"
                | "craft_smoked_fish_fillet"
                | "craft_spiced_beast_jerky"
                | "craft_ration_grain_cake"
                | "craft_calming_tea_pack"
        )
        || finished_def.id.contains("tea")
        || finished_def.id.contains("fish")
        || finished_def.id.contains("shrimp")
        || finished_def.id.contains("jerky")
        || finished_def.id.contains("grain")
        || finished_def.id.contains("dish")
        || finished_def.id.contains("soup")
        || room_name.contains("廚")
        || room_name.contains("灶")
        || room_name.contains("食")
        || chosen_raw_id.contains("shrimp")
        || chosen_raw_id.contains("fish")
        || chosen_raw_id.contains("meat")
        || chosen_raw_id.contains("grain")
        || chosen_raw_id.contains("tea")
    {
        let mentor_name = if room_name.contains("三水") || room_name.contains("溫") || room_name.is_empty() {
            "溫大娘"
        } else if let Some(v) = &venue_for_mentor {
            v.default_host_title
        } else {
            "溫大娘"
        };
        (
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            mentor_name,
            "三水火候收汁訣",
            "三水調和火候收汁，氣脈如潮汐連綿不絕，滋養臟腑",
            "庖廚百工",
            "tool",
            vec!["shui".to_string(), "huo_shou".to_string(), "he_tai".to_string()],
            vec!["水".to_string(), "火".to_string(), "合".to_string()],
            "庖廚湯杓",
        )
    } else if finished_def.id == "craft_meridian_salve"
        || finished_def.id.contains("salve")
        || chosen_raw_id.contains("herb")
        || raw_name.contains("草藥")
        || raw_name.contains("藥草")
    {
        let mentor_name = if room_name.contains("脈絡") || room_name.contains("溫") || room_name.is_empty() {
            "溫藥師"
        } else if let Some(v) = &venue_for_mentor {
            v.default_host_title
        } else {
            "溫藥師"
        };
        (
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            mentor_name,
            "百草九蒸提純訣",
            "文火慢熬百草、凝神提純藥力，使草木菁華溫潤入脈",
            "藥理百工",
            "tool",
            vec!["shui".to_string(), "huo_shou".to_string(), "he_tai".to_string()],
            vec!["水".to_string(), "火".to_string(), "合".to_string()],
            "紫銅藥碾",
        )
    } else {
        let mentor_name = if room_name.contains("柯") || room_name.is_empty() {
            "柯木匠"
        } else if let Some(v) = &venue_for_mentor {
            v.default_host_title
        } else {
            "柯木匠"
        };
        (
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            mentor_name,
            "柯氏順紋卯合訣",
            "順應木石竹絲紋理卯榫相合，意守方寸凝神聚氣，分毫不差",
            "木作百工",
            "tool",
            vec!["qiao_shu".to_string(), "mu".to_string(), "he_tai".to_string()],
            vec!["巧".to_string(), "木".to_string(), "合".to_string()],
            "百工刻刀",
        )
    };

    let habit_rec = crate::pg::living_world::record_breath_activity(actor_id, habit_kind, 2, now_unix);
    let breath_mut_bonus = ((habit_rec.dominant.craft_mutation_bonus() * 40.0).round() as i32).min(4);
    let craft_mutation_bonus = ((relevant_craft * 0.02).min(0.06) * 100.0).round() as i32;
    let is_utility_or_doc = finished_def.id.contains("torch")
        || finished_def.id.contains("umbrella")
        || finished_def.id.contains("ink")
        || finished_def.id.contains("manual")
        || finished_def.id.contains("almanac")
        || finished_def.id.contains("storage_box")
        || finished_def.id.contains("tool_handle")
        || finished_def.name.contains("火把")
        || finished_def.name.contains("晴雨傘")
        || finished_def.name.contains("墨錠")
        || finished_def.name.contains("手札")
        || finished_def.name.contains("曆書")
        || finished_def.name.contains("屋契")
        || finished_def.name.contains("租約")
        || finished_def.name.contains("隨身匣")
        || finished_def.name.contains("收納匣")
        || finished_def.name.contains("工具長柄");
    let can_mutate_def = finished_def.item_type != "consumable"
        && finished_def.item_type != "food"
        && !finished_def.slot.is_empty()
        && !is_utility_or_doc;
    let threshold = if can_mutate_def {
        (threshold + craft_mutation_bonus + breath_mut_bonus).min(22)
    } else {
        0
    };

    let roll = ((now_unix.wrapping_mul(1103515245).wrapping_add(12345) / 65536) % 100).abs() as i32;
    let is_mutated = can_mutate_def && roll < threshold;
    let (prod_id, prod_name, mut narrative) = if is_mutated {
        let arch_idx = ((now_unix as usize) ^ (chosen_raw_id.len())) % MUTATION_ARCHETYPES.len();
        let arch = &MUTATION_ARCHETYPES[arch_idx];
        let mut_id = format!("mut_{}_{}", finished_def.id, arch.key);
        let mut_name = format!("{}・{}", arch.name_prefix, finished_def.name);
        let desc = format!(
            "{}（{}，器物成色與質理俱臻上乘。）",
            finished_def.description, arch.desc_note
        );

        let item = Item {
            id: mut_id.clone(),
            name: mut_name.clone(),
            slot: finished_def.slot.to_string(),
            item_type: finished_def.item_type.to_string(),
            weight: finished_def.weight,
            stackable: 1,
            denomination: (finished_def.base_price as f64 * arch.price_mult).round() as i32,
            description: desc,
            vit_bonus: finished_def.vit_bonus + arch.vit_bonus,
            dex_bonus: finished_def.dex_bonus + arch.dex_bonus,
            atk_bonus: finished_def.atk_bonus + arch.atk_bonus,
            origin: finished_def.origin.to_string(),
        };
        ensure_item_in_store(item);
        let _ = db::add_to_inventory(actor_id, &mut_id, 1);

        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let ev = if mut_name.starts_with('【') {
                format!("百工煉製之際天地交感，成功淬煉出神異之寶{mut_name}！")
            } else {
                format!("百工煉製之際天地交感，成功淬煉出神異之寶【{mut_name}】！")
            };
            ledger.recent_events.push_back(ev);
            if ledger.recent_events.len() > 30 {
                ledger.recent_events.pop_front();
            }
        }
        db::log_npc_event(now_unix, actor_id, "craft_mutation", &mut_name);

        let formatted_mut_name = if mut_name.starts_with('【') {
            mut_name.clone()
        } else {
            format!("【{mut_name}】")
        };
        let narr = format!(
            "你將【{raw_name}】置於工爐中全神貫注煉製。爐火明滅流轉之際，周遭氣脈忽起劇烈共鳴，只見器身神芒驟顯，淬火出世之物赫然引發異變，蛻變為稀世罕見的{formatted_mut_name}！"
        );
        (mut_id, mut_name, narr)
    } else {
        let item = Item {
            id: finished_def.id.to_string(),
            name: finished_def.name.to_string(),
            slot: finished_def.slot.to_string(),
            item_type: finished_def.item_type.to_string(),
            weight: finished_def.weight,
            stackable: if finished_def.slot.is_empty() { 20 } else { 1 },
            denomination: finished_def.base_price,
            description: finished_def.description.to_string(),
            vit_bonus: finished_def.vit_bonus,
            dex_bonus: finished_def.dex_bonus,
            atk_bonus: finished_def.atk_bonus,
            origin: finished_def.origin.to_string(),
        };
        ensure_item_in_store(item);
        let _ = db::add_to_inventory(actor_id, finished_def.id, 1);

        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                entry.stock = (entry.stock + 1).min((entry.target_stock * 4).max(1));
            }
        }

        let narr = if is_culinary_craft {
            format!(
                "你起火架起炙架與鐵釜，將新鮮的【{raw_name}】佐以海鹽與山胡椒馬告細心翻炙收汁，陣陣鮮香撲鼻而來，成功烹製出一份香氣四溢的【{}】收入行囊。",
                finished_def.name
            )
        } else if finished_def.id.contains("salve") || finished_def.name.contains("膏") || finished_def.name.contains("藥") {
            format!(
                "你引火溫爐、細研百草，將【{raw_name}】慢火熬製提純收膏，藥香清幽沁脾，成功煉製出一份成色溫潤的【{}】收入行囊。",
                finished_def.name
            )
        } else if finished_def.id.contains("cloak") || finished_def.id.contains("boots") || finished_def.name.contains("斗篷") || finished_def.name.contains("皮靴") {
            format!(
                "你操持骨針與浸油麻線，將【{raw_name}】順著紋理裁切縫合、浸油加固，針腳緊密勻稱，成功縫製出一件結實耐用的【{}】收入行囊。",
                finished_def.name
            )
        } else if finished_def.id.contains("ink") || finished_def.id.contains("almanac") || finished_def.name.contains("墨") || finished_def.name.contains("手札") {
            format!(
                "你在案前凝神運杵、細心裁整【{raw_name}】，墨韻與紙理相得益彰，成功製成一件雅致精良的【{}】收入行囊。",
                finished_def.name
            )
        } else if finished_def.id.contains("umbrella") || finished_def.id.contains("box") || finished_def.id.contains("torch") || finished_def.id.contains("handle") {
            format!(
                "你順應木竹肌理運使刨刀，將【{raw_name}】細心裁切打磨、卯合上油，成功製成一件做工講究的【{}】收入行囊。",
                finished_def.name
            )
        } else {
            format!(
                "你細心運轉工藝，將【{raw_name}】反覆鍛打成型、細細打磨淬火，成功鍛造出一件質地純良厚重的【{}】收入行囊。",
                finished_def.name
            )
        };
        (finished_def.id.to_string(), finished_def.name.to_string(), narr)
    };

    if craft_dims.forge >= 0.8 || craft_dims.culinary >= 0.8 {
        narrative.push_str(" 【百工熟稔】你呼吸沉穩、指掌施力勻稱，火候與力道拿捏得恰到好處。");
    }

    let vstate = if crate::pg::living_world::is_weapon_or_tool_item(&prod_id, &prod_name) {
        Some(crate::pg::living_world::record_vessel_use(actor_id, &prod_id, 0, 2, now_unix))
    } else {
        None
    };

    let mentor_present = is_workshop
        || room_name.contains("廚")
        || room_name.contains("灶")
        || room_name.contains("食堂")
        || room_name.contains(mentor_name);
    if mentor_present {
        let (_imprint_rec, newly_gifted) = crate::pg::living_world::record_mentor_imprint_progress(
            actor_id,
            mentor_name,
            imprint_title,
            imprint_prose,
            now_unix,
        );

        if newly_gifted && !db::inventory::is_npc_entity_id(actor_id) {
            let _ = db::add_to_inventory(actor_id, "blank_manual", 1);
        }
        if newly_gifted {
            let mut hasher = 0xcbf29ce484222325u64;
            for b in format!("{}_{}", mentor_name, imprint_title).bytes() {
                hasher ^= b as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
            let manual_hash = format!("{:016x}", hasher);
            let manual_rec = crate::pg::living_world::TranscribedManualRecord {
                manual_hash: manual_hash.clone(),
                author_id: mentor_name.to_string(),
                author_name: mentor_name.to_string(),
                formal_name: imprint_title.to_string(),
                slot_label: slot_label.to_string(),
                carrier_code: carrier_code.to_string(),
                word_ids: formula_word_ids.clone(),
                word_chars: formula_word_chars.clone(),
                origin_basis: imprint_prose.to_string(),
                created_at: now_unix,
            };
            crate::pg::living_world::save_transcribed_manual(&manual_rec);
            let manual_item_id = format!("manual_{manual_hash}");
            let _ = db::add_to_inventory(actor_id, &manual_item_id, 1);

            let word_id_refs: Vec<&str> = formula_word_ids.iter().map(|s| s.as_str()).collect();
            crate::server::handler::inventory::grant_words_to_player_casket(actor_id, &word_id_refs);

            narrative.push_str(&format!(
                " {}見你手法沉穩老練，眼中滿是讚許之色，微微頷首道：「此道你已深得精髓。」說罷贈予你一卷親筆手書的《{}》手札與空白手札，並傳下「{}」諸字真意，以供日後參悟傳承。",
                mentor_name,
                imprint_title,
                formula_word_chars.join("・")
            ));
        }
    }

    let is_high_tier = is_mutated || finished_def.base_price >= 100 || !finished_def.slot.is_empty();
    if is_high_tier {
        let actor_name = if !ent.display_title.trim().is_empty() {
            ent.display_title.trim().to_string()
        } else {
            "百工匠人".to_string()
        };
        let tool_familiarity = vstate.as_ref().map(|vs| vs.familiarity().label().to_string()).unwrap_or_else(|| "得心應手".to_string());
        let packet = crate::ai::CraftActionFactPacket {
            actor_name,
            tool_name: station_tool_name.to_string(),
            tool_familiarity,
            breath_habit: habit_rec.dominant.label().to_string(),
            material_names: vec![raw_name.clone()],
            output_name: prod_name.clone(),
            highlight_kind: if is_mutated { "器物異變" } else { "精工淬煉" }.to_string(),
            environment_hint: if room_name.is_empty() { "工坊作坊" } else { room_name }.to_string(),
        };
        let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
        if !highlight.is_empty() {
            narrative.push_str("\n\n");
            narrative.push_str(&highlight);
        }
    }

    if !room_name.trim().is_empty() {
        if let Some((_pack_id, _step, ep_line)) = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, &prod_name)
            .or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(room_name, &raw_name))
        {
            narrative.push('\n');
            narrative.push_str(&ep_line);
        }
    }

    Ok((prod_id, prod_name, is_mutated, narrative))
}

/// 以氣脈淬煉已成之器物，引導其發生質理異變。
pub fn mutate_finished_item(
    actor_id: &str,
    item_id: &str,
    _room_name: &str,
    now_unix: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(actor_id) else {
        return Err("角色不存在".to_string());
    };
    if ent.qi < 10 {
        return Err("元氣不足（需內息充沛才能催動器物淬鍊共鳴）。".to_string());
    }

    let inv = db::get_inventory(&ent.inventory, ent.vit);

    let can_mutate_item = |id: &str| -> bool {
        if id == "blank_manual"
            || id == "wt_blank_manual"
            || id.starts_with("manual_")
            || id.starts_with("corpse_")
            || id.starts_with("crystal_")
            || id.contains("torch")
            || id.contains("umbrella")
            || id.contains("ink")
            || id.contains("almanac")
            || id.contains("storage_box")
            || id.contains("tool_handle")
        {
            return false;
        }
        if is_raw_material(id) || id.starts_with("mut_") || classify_item_stage(id) == "mutated" {
            return false;
        }
        if id.contains("salve") || id.contains("tea") || id.contains("decoction") {
            return false;
        }
        if let Some(def) = db::get_item_def(id) {
            let non_gear_types = [
                "equipment",
                "weapon",
                "armor",
                "tool",
                "clothing_textiles_footwear",
                "weapons_armor_patrol",
                "craft_machinery_tools",
            ];
            if def.slot.is_empty() || !non_gear_types.contains(&def.item_type.as_str()) {
                return false;
            }
            let rejected_names = [
                "手札", "課本", "殘軀", "活血膏", "茶包", "仙草凍", "豆花", "胡椒餅", "切仔麵",
                "牛肉", "番薯", "飯糰", "炊飯", "米漿", "油條", "火把", "晴雨傘", "墨錠", "曆書",
                "屋契", "租約", "隨身匣", "收納匣", "木匣", "工具箱", "工具長柄", "長柄",
            ];
            if rejected_names.iter().any(|k| def.name.contains(k)) {
                return false;
            }
        }
        true
    };

    let chosen_item_id = if item_id == "auto" || item_id.trim().is_empty() {
        inv.items
            .iter()
            .filter(|it| can_mutate_item(&it.item_id))
            .find(|it| classify_item_stage(&it.item_id) == "finished")
            .map(|it| it.item_id.clone())
            .ok_or_else(|| "行囊中尚無可用於氣脈異變的工造成品。".to_string())?
    } else {
        if !db::inventory_has_item(&ent.inventory, item_id) {
            return Err("行囊中未持有指定的成品物件。".to_string());
        }
        if !can_mutate_item(item_id) {
            return Err("此類器物非百工器具或防身兵甲，無法引導氣脈共鳴淬鍊。".to_string());
        }
        item_id.to_string()
    };

    let has_furnace_heat = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.get(_room_name).copied().unwrap_or(0) > 0
    };
    let has_charcoal = db::inventory_has_item(&ent.inventory, "mat_charcoal_sack");

    let is_workshop = _room_name.contains("鐵坊")
        || _room_name.contains("木作")
        || _room_name.contains("印行")
        || _room_name.contains("藥房")
        || _room_name.contains("織造")
        || _room_name.contains("觀星")
        || _room_name.contains("工坊")
        || _room_name.contains("作坊");
    let threshold = if has_furnace_heat || has_charcoal { 35 } else if is_workshop { 24 } else { 12 };

    let roll = ((now_unix.wrapping_mul(1103515245).wrapping_add(12345) / 65536) % 100).abs() as i32;
    if roll >= threshold {
        return Err("氣脈共鳴未臻極致，未能引發質理異變（可藉由工爐添柴增熱、攜帶硬木白炭或至工坊重試）。".to_string());
    }
    // 扣除 10 點元氣
    if let Some(arc) = store::get_store() {
        let _ = arc.write().update_entity(actor_id, |e| {
            e.qi = (e.qi - 10).max(0);
        });
    }

    if db::remove_from_inventory(actor_id, &chosen_item_id, 1).is_err() {
        return Err("扣減物件失敗".to_string());
    }

    // 取得基礎成品資訊
    let base_def = db::get_item_def(&chosen_item_id);
    let mut base_clean_id = chosen_item_id.trim_start_matches("mut_").to_string();
    for a in MUTATION_ARCHETYPES {
        let suf = format!("_{}", a.key);
        if let Some(stripped) = base_clean_id.strip_suffix(&suf) {
            base_clean_id = stripped.to_string();
        }
    }
    let resolved_base_name = base_def
        .as_ref()
        .map(|d| d.name.clone())
        .or_else(|| crate::world::materials::material_by_item_id(&chosen_item_id).map(|m| m.name.clone()))
        .or_else(|| crate::game::peninsula_catalog::find_catalog_entry(&chosen_item_id).map(|c| c.name.to_string()))
        .unwrap_or_else(|| {
            if chosen_item_id.is_ascii() {
                "隨身器物".to_string()
            } else {
                chosen_item_id.clone()
            }
        });
    let base_name = if resolved_base_name.is_ascii() {
        "隨身器物".to_string()
    } else {
        resolved_base_name
    };
    let base_clean_name = crate::game::object_ecology::ItemPhysicalClass::strip_mutation_affixes(&base_name);
    // 選擇異變原型（避免與原先重複）
    let mut arch_idx = ((now_unix as usize) ^ (chosen_item_id.len())) % MUTATION_ARCHETYPES.len();
    if chosen_item_id.contains(MUTATION_ARCHETYPES[arch_idx].key) {
        arch_idx = (arch_idx + 1) % MUTATION_ARCHETYPES.len();
    }
    let arch = &MUTATION_ARCHETYPES[arch_idx];

    let mut_id = format!("mut_{}_{}", base_clean_id, arch.key);
    let mut_name = format!("{}・{}", arch.name_prefix, base_clean_name.trim());

    let base_desc = base_def.as_ref().map(|d| d.description.as_str()).unwrap_or("做工精湛的工造之物。");
    let desc = format!(
        "{}（{}，器物成色與質理俱臻上乘。）",
        base_desc, arch.desc_note
    );
    let real_base_price = item_base_price_mg(&chosen_item_id).max(15);
    let (slot, item_type, weight, base_price, base_vit, base_dex, base_atk) = if let Some(d) = &base_def {
        (d.slot.clone(), d.item_type.clone(), d.weight, real_base_price, d.vit_bonus, d.dex_bonus, d.atk_bonus)
    } else {
        ("hold_r".to_string(), "equipment".to_string(), 1.0, real_base_price, 0, 0, 0)
    };

    let item = Item {
        id: mut_id.clone(),
        name: mut_name.clone(),
        slot,
        item_type,
        weight,
        stackable: 1,
        denomination: (base_price as f64 * arch.price_mult).round() as i32,
        description: desc,
        vit_bonus: base_vit + arch.vit_bonus,
        dex_bonus: base_dex + arch.dex_bonus,
        atk_bonus: base_atk + arch.atk_bonus,
        origin: base_def.as_ref().map(|d| d.origin.clone()).unwrap_or_else(|| lookup_item_origin(&base_clean_id)),
    };
    ensure_item_in_store(item);
    let _ = db::add_to_inventory(actor_id, &mut_id, 1);

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let ev = if mut_name.starts_with('【') {
            format!("{mut_name}在氣脈共鳴中破繭出世！")
        } else {
            format!("【{mut_name}】在氣脈共鳴中破繭出世！")
        };
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }
    db::log_npc_event(now_unix, actor_id, "item_mutation", &mut_name);

    let mut narrative = format!(
        "你凝神調息，將沛然內息徐徐注入【{base_clean_name}】中。隨行氣勢激盪，天地與器物深層質理產生極強共振，微芒自器表寸寸漫溢，赫然蛻變為神異斐然的{mut_name}！"
    );
    if !_room_name.trim().is_empty() {
        if let Some((_pack_id, _step, ep_line)) = crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(_room_name, &mut_name)
            .or_else(|| crate::game::peninsula_scripts::try_advance_room_event_and_script_pack_via_object(_room_name, &base_clean_name))
        {
            narrative.push('\n');
            narrative.push_str(&ep_line);
        }
    }
    Ok((mut_id, mut_name, narrative))
}


fn handle_use_manual_action(conn: &WsConnection, msg: &ClientMsg, pid: &str) -> bool {
    let pid = pid.to_string();
    // ─── 著譜立說（TranscribeManual）───────────────────────────────────────
    if msg.action == "TranscribeManual" {
        ensure_player_initialized(&pid);
        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return true;
        };

        let blank_id = if inventory_has_item(&ent.inventory, "blank_manual") {
            "blank_manual"
        } else if inventory_has_item(&ent.inventory, "wt_blank_manual") {
            "wt_blank_manual"
        } else {
            conn.send_error("行囊中尚無「空白手札」，無法著譜立說。");
            return true;
        };

        let slot_str = msg.item_id.trim();
        let (slot_enum, slot_label) = match slot_str {
            "opening" | "起手" => (TacticalSlot::Opening, "起手"),
            "midgame" | "連攜" => (TacticalSlot::Midgame, "連攜"),
            "finisher" | "破綻" => (TacticalSlot::Finisher, "破綻"),
            "defense" | "守勢" => (TacticalSlot::Defense, "守勢"),
            _ => (TacticalSlot::Opening, "起手"),
        };

        let registry = WordRegistry::global();
        ensure_player_initialized(&pid);
        let casket = get_player_word_casket(&pid);
        let loadout = {
            let guard = PLAYER_LOADOUTS.read();
            guard.get(&pid).cloned().unwrap_or_else(|| {
                let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
                crate::combat::wordplate::build_loadout_from_soul_seed(seed, Some(&casket), registry)
            })
        };

        let sent_opt = match slot_enum {
            TacticalSlot::Opening => loadout.opening.clone(),
            TacticalSlot::Midgame => loadout.midgame.clone(),
            TacticalSlot::Finisher => loadout.finisher.clone(),
            TacticalSlot::Defense => loadout.defense.clone(),
        };

        let Some(sent) = sent_opt else {
            conn.send_error("此路數尚未純熟，無法著譜立說。");
            return true;
        };

        let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut held_item_id = String::new();
        let mut held_item_display = String::new();
        for k in &held_keys {
            if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
                held_item_id = id.clone();
                let base = equip_names.get(*k).cloned().unwrap_or_else(|| resolve_inv_item_name(id));
                held_item_display = format_entity_item_with_inscription(&ent.id, id, &base);
                break;
            }
        }
        let held_carrier = if !held_item_id.is_empty() {
            crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
        } else {
            None
        };
        let martial_carrier = held_carrier.unwrap_or(CarrierAffinity::Unarmed);
        let vessel_name = if held_item_display.is_empty() {
            "隨身套路".to_string()
        } else {
            held_item_display.clone()
        };
        let studied_manuals = crate::pg::living_world::get_entity_studied_manuals(&ent.id);
        let learned_bonus = (studied_manuals.len() as f64 * 0.25).min(1.0);
        let vessel_familiarity_count = if !held_item_id.is_empty() {
            crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id).use_count as u32
        } else {
            0
        };

        let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
        let personal_routine = crate::combat::human_phase::derive_personal_martial_routine(
            &bone,
            slot_enum,
            martial_carrier,
            &vessel_name,
            learned_bonus,
            vessel_familiarity_count,
            registry,
        );

        let (formal_name, origin_basis, carrier, word_ids, word_chars) = if let Some(custom_name) = &sent.custom_name {
            let ids: Vec<String> = sent.inlays.iter().map(|i| i.word_id.clone()).collect();
            let chars: Vec<String> = sent
                .inlays
                .iter()
                .map(|inlay| {
                    registry
                        .get_by_id(&inlay.word_id)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| inlay.word_id.clone())
                })
                .collect();
            (
                custom_name.clone(),
                format!("體察自身發力習慣與呼吸節奏，將隨身演練純熟之《{}》著錄於冊", custom_name),
                martial_carrier,
                ids,
                chars,
            )
        } else {
            let mut ids = vec![personal_routine.omp_word_ids.0.clone(), personal_routine.omp_word_ids.1.clone()];
            for inlay in &sent.inlays {
                if !ids.contains(&inlay.word_id) {
                    ids.push(inlay.word_id.clone());
                }
            }
            while ids.len() < 3 {
                ids.push(personal_routine.omp_word_ids.1.clone());
            }
            let chars: Vec<String> = ids
                .iter()
                .map(|wid| {
                    registry
                        .get_by_id(wid)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| wid.clone())
                })
                .collect();
            (
                personal_routine.formal_name.clone(),
                personal_routine.origin_basis_zh.clone(),
                personal_routine.carrier,
                ids,
                chars,
            )
        };

        let mut hasher: u64 = 0xcbf29ce484222325;
        for b in pid.bytes() {
            hasher ^= b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        for wid in &word_ids {
            for b in wid.bytes() {
                hasher ^= b as u64;
                hasher = hasher.wrapping_mul(0x100000001b3);
            }
        }
        let manual_hash = format!("{:016x}", hasher);

        if remove_from_inventory(&pid, blank_id, 1).is_err() {
            conn.send_error("扣除空白手札失敗");
            return true;
        }

        let author_name = if !ent.display_title.trim().is_empty() {
            ent.display_title.trim().to_string()
        } else {
            "山海行旅者".to_string()
        };

        let now = crate::game::now_unix();
        let record = crate::pg::living_world::TranscribedManualRecord {
            manual_hash: manual_hash.clone(),
            author_id: pid.clone(),
            author_name,
            formal_name: formal_name.clone(),
            slot_label: slot_label.to_string(),
            carrier_code: carrier.code().to_string(),
            word_ids,
            word_chars: word_chars.clone(),
            origin_basis: origin_basis.clone(),
            created_at: now,
        };
        crate::pg::living_world::save_transcribed_manual(&record);

        let manual_item_id = format!("manual_{manual_hash}");
        let _ = db::add_to_inventory(&pid, &manual_item_id, 1);
        push_refresh(conn, &pid);

        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "TranscribeManual".into(),
            target_id: manual_item_id,
            target_name: format!("《{}》手札", formal_name),
            narrative: format!(
                "你鋪開空白手札，體察自身發力習慣與招式流轉，將隨身演練純熟之招法著錄於冊：以「{}」之勢與「{}」諸字真意逐筆錄下，裝訂成一冊《{}》手札收入行囊。",
                origin_basis,
                word_chars.join("・"),
                formal_name
            ),
            success: true,
            actions: vec![],
            move_target_id: String::new(),
            open_trade: false,
            suggested_action: String::new(),
            tactical_choices: vec![],
        });
        return true;
    }

    // ─── 研讀手札（StudyManual / Use manual_*）─────────────────────────────
    if msg.action == "StudyManual"
        || (msg.item_id.starts_with("manual_") && (msg.action.is_empty() || msg.action == "Use"))
    {
        ensure_player_initialized(&pid);
        let Ok(Some(ent)) = db::get_entity(&pid) else {
            conn.send_error(gametext::client("inv_entity_not_found"));
            return true;
        };

        let Some(manual) = crate::pg::living_world::get_transcribed_manual(&msg.item_id) else {
            conn.send_error("此卷手札字跡殘損模糊，其中要訣已無從辨識。");
            return true;
        };
        if manual.word_ids.len() < 3 {
            conn.send_error("此冊手札篇章殘缺，難以連貫參悟。");
            return true;
        }
        let registry = WordRegistry::global();
        let learner_seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
        let bone = crate::combat::human_phase::InnateBonePhase::from_seed(learner_seed);

        let is_craft_manual = manual.carrier_code == "tool"
            || manual.slot_label.contains("百工")
            || manual.slot_label.contains("庖廚")
            || manual.slot_label.contains("採獵")
            || manual.slot_label.contains("行旅")
            || manual.slot_label == "craft"
            || manual.slot_label == "production"
            || manual.formal_name.contains("卯合")
            || manual.formal_name.contains("火候")
            || manual.formal_name.contains("收汁")
            || manual.formal_name.contains("衡準");
        let (slot_enum, slot_label_zh) = match manual.slot_label.as_str() {
            "起手" | "opening" => (TacticalSlot::Opening, "起手"),
            "連攜" | "midgame" => (TacticalSlot::Midgame, "連攜"),
            "破綻" | "finisher" | "重手" => (TacticalSlot::Finisher, "破綻"),
            "守勢" | "defense" => (TacticalSlot::Defense, "守勢"),
            _ => (TacticalSlot::Opening, "起手"),
        };

        let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
        let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
        let mut held_item_id = String::new();
        let mut held_item_display = String::new();
        for k in &held_keys {
            if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
                let base = equip_names.get(*k).cloned().unwrap_or_else(|| resolve_inv_item_name(id));
                if crate::pg::living_world::is_weapon_or_tool_item(id, &base) {
                    held_item_id = id.clone();
                    held_item_display = format_entity_item_with_inscription(&ent.id, id, &base);
                    break;
                }
            }
        }
        let carrier = if !held_item_id.is_empty() {
            crate::combat::wordplate::infer_item_carrier_affinity(&held_item_id, &held_item_display)
                .unwrap_or(CarrierAffinity::Blade)
        } else {
            crate::combat::wordplate::CarrierAffinity::from_code(&manual.carrier_code)
                .unwrap_or(CarrierAffinity::Unarmed)
        };

        let (routine, adaptation_prose) = crate::combat::human_phase::adapt_manual_to_bone_phase(
            &bone,
            slot_enum,
            carrier,
            &manual.formal_name,
            &manual.word_chars,
            registry,
        );

        crate::pg::living_world::record_entity_studied_manual(&pid, &manual.formal_name);

        let word_count = if manual.word_ids.len() >= 7 {
            7
        } else if manual.word_ids.len() >= 5 {
            5
        } else {
            3
        };
        let inlays: Vec<StarInlay> = (0..word_count)
            .map(|i| {
                let node_id = format!("N{:03}", i + 1);
                let wid = if i == 0 {
                    routine.omp_word_ids.0.clone()
                } else if i == 1 {
                    routine.omp_word_ids.1.clone()
                } else {
                    manual.word_ids.get(i).cloned().unwrap_or_else(|| routine.omp_word_ids.1.clone())
                };
                let fluency = if routine.continuity_c >= 1.25 {
                    OrbitFluency::ChangLiu
                } else if routine.continuity_c >= 0.95 {
                    OrbitFluency::ShunTong
                } else if routine.continuity_c >= 0.70 {
                    OrbitFluency::PingWen
                } else {
                    OrbitFluency::ZhiSe
                };
                StarInlay::with_fluency(
                    node_id,
                    wid,
                    fluency,
                )
            })
            .collect();
        let mut star_sentence = match StarSentence::new(inlays) {
            Ok(s) => s.with_custom_name(&routine.formal_name),
            Err(e) => {
                conn.send_error(format!("招式推演受阻：{e}"));
                return true;
            }
        };
        star_sentence.set_soul_seed(learner_seed);

        if !is_craft_manual {
            let mut loadouts = PLAYER_LOADOUTS.write();
            if let Some(loadout) = loadouts.get_mut(&pid) {
                loadout.set_slot(slot_enum, star_sentence);
            } else {
                let mut loadout = create_default_loadout(registry);
                loadout.set_slot(slot_enum, star_sentence);
                loadouts.insert(pid.clone(), loadout);
            }
            save_player_wordplates();
        }
        push_refresh(conn, &pid);

        let motions_str = routine.main_motions.join(" → ");
        let narrative = if is_craft_manual {
            format!(
                "你靜心參讀《{}》手札，細細揣摩其中百工造物與行氣法度，隨自身發力習慣心領神會！\n{}\n手札所載之「{}」要訣已融會貫通並納入【{}】與百工手藝之中！",
                manual.formal_name,
                adaptation_prose,
                motions_str,
                manual.slot_label
            )
        } else {
            format!(
                "你靜心參讀《{}》手札，細細揣摩其中發力法度，隨自身體魄節奏自然化為「{}」！\n{}\n招式循序運化為「{}」，已自然融入{}身手！",
                manual.formal_name,
                routine.formal_name,
                adaptation_prose,
                motions_str,
                slot_label_zh
            )
        };
        conn.send_json(&ActionResultMsg {
            msg_type: "action_result".into(),
            action: "StudyManual".into(),
            target_id: msg.item_id.clone(),
            target_name: format!("《{}》手札", manual.formal_name),
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
    false
}

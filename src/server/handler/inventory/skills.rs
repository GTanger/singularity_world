pub fn entity_total_craft_dims(pid: &str) -> CraftDims4D {
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let loadouts = PLAYER_LOADOUTS.read();
    let mut craft_totals = loadouts
        .get(pid)
        .map(|l| l.total_craft_dims(registry))
        .unwrap_or_default();
    drop(loadouts);

    let casket = get_player_word_casket(pid);
    if let Ok(Some(ent)) = crate::db::get_entity(pid)
        && let Ok(map) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&ent.equipment_slots)
    {
        for key in ["weapon", "hold_r", "hand", "tool", "hold_l"] {
            if let Some(weapon_id) = map.get(key).and_then(|v| v.as_str())
                && !weapon_id.is_empty()
            {
                let bonus = casket.inscribed_craft_bonus(weapon_id, registry);
                craft_totals.forge += bonus.forge;
                craft_totals.culinary += bonus.culinary;
                craft_totals.harvest += bonus.harvest;
                craft_totals.wayfare += bonus.wayfare;
            }
        }
    }
    craft_totals
}

fn sentence_to_view(
    slot_name: &str,
    sentence: &StarSentence,
    registry: &WordRegistry,
) -> TacticalSentenceView {
    let name = sentence
        .custom_name
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| match sentence.synthesize(registry) {
            Ok(syn) => syn.effective_name,
            Err(_) => "無名招式".to_string(),
        });

    let words: Vec<String> = sentence
        .inlays
        .iter()
        .map(|inlay| {
            registry
                .get_by_id(&inlay.word_id)
                .map(|elem| elem.ch.clone())
                .unwrap_or_else(|| inlay.word_id.clone())
        })
        .collect();

    let (fluency_label, primary_carrier, primary_target) = match sentence.synthesize(registry) {
        Ok(syn) => (
            syn.fluency_summary.label().to_string(),
            syn.primary_carrier.label().to_string(),
            syn.primary_target.label().to_string(),
        ),
        Err(_) => (
            OrbitFluency::PingWen.label().to_string(),
            CarrierAffinity::Unarmed.label().to_string(),
            TargetPart::Torso.label().to_string(),
        ),
    };

    TacticalSentenceView {
        slot: slot_name.to_string(),
        name,
        words,
        nodes: sentence.inlays.iter().map(|inlay| inlay.node_id.clone()).collect(),
        fluency_label,
        primary_carrier,
        primary_target,
    }
}

fn loadout_to_view(loadout: &TacticalLoadout, registry: &WordRegistry) -> TacticalLoadoutView {
    TacticalLoadoutView {
        opening: loadout
            .opening
            .as_ref()
            .map(|s| sentence_to_view("opening", s, registry)),
        midgame: loadout
            .midgame
            .as_ref()
            .map(|s| sentence_to_view("midgame", s, registry)),
        finisher: loadout
            .finisher
            .as_ref()
            .map(|s| sentence_to_view("finisher", s, registry)),
        defense: loadout
            .defense
            .as_ref()
            .map(|s| sentence_to_view("defense", s, registry)),
    }
}

fn casket_to_entries(casket: &WordCasket, registry: &WordRegistry) -> Vec<CasketWordEntry> {
    let mut entries = Vec::new();
    for (word_id, count) in casket.list() {
        if let Some(elem) = registry.get_by_id(word_id) {
            entries.push(CasketWordEntry {
                id: word_id.to_string(),
                ch: elem.ch.clone(),
                count,
                carrier_affinity: elem.carrier_affinity.label().to_string(),
                target_part: elem.target_part.label().to_string(),
                quality_label: casket.quality_of(word_id).label_zh().to_string(),
                charge_pct: casket.charge_of(word_id),
                extraction_ratio: casket.ratio_of(word_id),
                semantic: elem.semantic.clone(),
                craft_label: elem.craft_dims.primary_aspect().label().to_string(),
            });
        }
    }
    entries
}

fn build_tactical_state_msg(pid: &str) -> TacticalStateMsg {
    ensure_player_initialized(pid);
    let registry = WordRegistry::global();
    let casket_guard = PLAYER_CASKETS.read();
    let loadout_guard = PLAYER_LOADOUTS.read();

    let casket = casket_guard
        .get(pid)
        .map(|c| casket_to_entries(c, registry))
        .unwrap_or_default();

    let loadout = loadout_guard
        .get(pid)
        .map(|l| loadout_to_view(l, registry))
        .unwrap_or_default();

    TacticalStateMsg {
        msg_type: "tactical_state".to_string(),
        casket,
        loadout,
    }
}

pub(super) fn handle_get_tactical_state(conn: &WsConnection) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let msg = build_tactical_state_msg(&pid);
    conn.send_json(&msg);
}

pub(super) fn handle_update_tactics(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };

    let slot = match msg.slot.to_lowercase().as_str() {
        "opening" => TacticalSlot::Opening,
        "midgame" => TacticalSlot::Midgame,
        "finisher" => TacticalSlot::Finisher,
        "defense" => TacticalSlot::Defense,
        _ => {
            conn.send_error("未知的演練路數槽位");
            return;
        }
    };

    let raw_inlays: Vec<(String, String)> = if !msg.inlays.is_empty() {
        msg.inlays.clone()
    } else if !msg.words.is_empty() {
        let fallback_node = msg.nodes.first().cloned().unwrap_or_else(|| "N001".to_string());
        let nodes: Vec<String> = if msg.nodes.len() == msg.words.len() {
            msg.nodes.clone()
        } else {
            (0..msg.words.len()).map(|_| fallback_node.clone()).collect()
        };
        nodes.into_iter().zip(msg.words.iter().cloned()).collect()
    } else {
        conn.send_error("未提供星盤節點與鑲嵌字韻");
        return;
    };

    let len = raw_inlays.len();
    if len != 3 && len != 5 && len != 7 {
        conn.send_error(format!(
            "造句字數不符規則：長度 {}（僅允許三言、五言、七言）",
            len
        ));
        return;
    }

    let registry = WordRegistry::global();
    let mut resolved_inlays = Vec::with_capacity(len);
    for (node_id, word_spec) in &raw_inlays {
        if let Err(e) = crate::combat::wordplate::parse_star_node(node_id) {
            conn.send_error(format!("星盤節點校驗失敗：{e}"));
            return;
        }
        let resolved_id = if registry.get_by_id(word_spec).is_some() {
            word_spec.clone()
        } else if let Some(id) = resolve_word_id(registry, word_spec) {
            id
        } else {
            conn.send_error(format!("字韻不存在：{word_spec}"));
            return;
        };
        resolved_inlays.push(StarInlay::new(node_id.clone(), resolved_id));
    }

    let sentence = match StarSentence::new(resolved_inlays) {
        Ok(s) => s,
        Err(e) => {
            conn.send_error(format!("造句校驗失敗：{e}"));
            return;
        }
    };

    ensure_player_initialized(&pid);

    let mut word_counts: HashMap<String, u32> = HashMap::new();
    for inlay in &sentence.inlays {
        *word_counts.entry(inlay.word_id.clone()).or_insert(0) += 1;
    }

    {
        let caskets = PLAYER_CASKETS.read();
        if let Some(casket) = caskets.get(&pid) {
            for (word_id, needed) in &word_counts {
                let owned = casket.count(word_id);
                if owned < *needed {
                    let ch = registry
                        .get_by_id(word_id)
                        .map(|w| w.ch.as_str())
                        .unwrap_or(word_id.as_str());
                    conn.send_error(format!(
                        "字匣中字韻數量不足：{}（現有 {}，需求 {}）",
                        ch, owned, needed
                    ));
                    return;
                }
            }
        }
    }

    {
        let mut loadouts = PLAYER_LOADOUTS.write();
        if let Some(loadout) = loadouts.get_mut(&pid) {
            loadout.set_slot(slot, sentence);
        } else {
            let mut loadout = create_default_loadout(registry);
            loadout.set_slot(slot, sentence);
            loadouts.insert(pid.clone(), loadout);
        }
    }
    save_player_wordplates();
    push_refresh(conn, &pid);
    let state_msg = build_tactical_state_msg(&pid);
    conn.send_json(&state_msg);

    let effective_name = state_msg
        .loadout
        .opening
        .as_ref()
        .filter(|_| matches!(slot, TacticalSlot::Opening))
        .or(state_msg.loadout.midgame.as_ref().filter(|_| matches!(slot, TacticalSlot::Midgame)))
        .or(state_msg.loadout.finisher.as_ref().filter(|_| matches!(slot, TacticalSlot::Finisher)))
        .or(state_msg.loadout.defense.as_ref().filter(|_| matches!(slot, TacticalSlot::Defense)))
        .map(|v| v.name.clone())
        .unwrap_or_else(|| msg.words.join(""));
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "UpdateTactics".into(),
        target_id: msg.slot.clone(),
        target_name: effective_name.clone(),
        narrative: format!("你凝神引字入樞，氣隨星位流轉，凝成一式「{}」。", effective_name),
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });
}


/// 後台根據人物先天骨相、內息底色、所學手札、所熟悉器物與山海歷練自然生成技能列表與經歷紀錄。
/// 這些技能並非玩家手動自訂，而是在戰鬥交鋒與百工生產中由後台自動施展。
pub fn synthesize_entity_natural_skills(ent: &Character) -> Vec<NaturalSkillView> {
    ensure_player_initialized(&ent.id);
    let registry = WordRegistry::global();

    let (equip_slots, equip_names, _) = parse_equipment_triple(&ent.equipment_slots);
    let held_keys = ["hold_r", "hold_l", "weapon", "hand", "tool"];
    let mut held_item_id = String::new();
    let mut held_item_display = String::new();
    let mut martial_item_id = String::new();
    let mut martial_item_display = String::new();
    for k in &held_keys {
        if let Some(id) = equip_slots.get(*k).filter(|s| !s.is_empty()) {
            let base = equip_names
                .get(*k)
                .cloned()
                .unwrap_or_else(|| resolve_inv_item_name(id));
            let display = format_entity_item_with_inscription(&ent.id, id, &base);
            if held_item_id.is_empty() {
                held_item_id = id.clone();
                held_item_display = display.clone();
            }
            if martial_item_id.is_empty()
                && crate::pg::living_world::is_weapon_or_tool_item(id, &base)
            {
                martial_item_id = id.clone();
                martial_item_display = display;
            }
        }
    }

    let held_carrier = if !martial_item_id.is_empty() {
        crate::combat::wordplate::infer_item_carrier_affinity(&martial_item_id, &martial_item_display)
    } else {
        None
    };

    let raw_vocation = {
        let t = ent.display_title.trim();
        if t.is_empty() || t == "山海行旅者" {
            ""
        } else {
            t
        }
    };
    let seed = crate::combat::human_phase::resolve_entity_soul_seed(&ent.id, ent.soul_seed);
    let bone = crate::combat::human_phase::InnateBonePhase::from_seed(seed);
    let habit = crate::pg::living_world::get_or_derive_breath_habit(&ent.id, raw_vocation, seed);
    let imprints = crate::pg::living_world::get_entity_lineage_imprints(&ent.id);
    let studied_manuals = crate::pg::living_world::get_entity_studied_manuals(&ent.id);
    let anatomy_total = crate::pg::living_world::get_entity_anatomy_familiarity_total(&ent.id);
    let active_afflictions = crate::pg::living_world::get_entity_afflictions(&ent.id);

    let mut breath_origin = habit.dominant.somatic_description().to_string();
    if !active_afflictions.is_empty() {
        let aff_labels: Vec<_> = active_afflictions.iter().map(|a| a.label()).collect();
        breath_origin.push_str(&format!("（身帶創傷：{}）", aff_labels.join("、")));
    }

    let vessel_name = if martial_item_display.is_empty() {
        "徒手".to_string()
    } else {
        martial_item_display.clone()
    };
    let vessel_familiarity_count = if !martial_item_id.is_empty() {
        crate::pg::living_world::get_vessel_state(&ent.id, &martial_item_id).use_count as u32
    } else {
        0
    };
    let learned_bonus = (studied_manuals.len() as f64 * 0.25).min(1.0);

    let mut skills = Vec::with_capacity(14);

    // ─── 領域一：骨相與氣息底色（bone_phase）─────────────────────────────
    skills.push(NaturalSkillView {
        domain: "bone_phase".to_string(),
        category: "innate_bone".to_string(),
        category_label: "天生體魄".to_string(),
        name: bone.bone_archetype_title(),
        origin_basis: bone.describe_bone_phase_zh(),
        learned_and_familiar: bone.auto_trigger_summary_zh(),
        auto_trigger_desc: String::new(),
    });

    skills.push(NaturalSkillView {
        domain: "bone_phase".to_string(),
        category: "inner_breath".to_string(),
        category_label: "內息底色".to_string(),
        name: habit.dominant.label().to_string(),
        origin_basis: breath_origin,
        learned_and_familiar: "周天吐納（隨日常打鐵、涉山、涉水、靜修與交鋒自然演進，行氣節奏漸次沉入筋骨）".to_string(),
        auto_trigger_desc: format!("{}。{}", habit.dominant.combat_rhythm_phrase(), habit.dominant.effect_summary()),
    });

    // ─── 領域二：臨陣武相（martial，4 項常駐）─────────────────────────────
    let martial_slots = [
        ("opening", TacticalSlot::Opening),
        ("midgame", TacticalSlot::Midgame),
        ("finisher", TacticalSlot::Finisher),
        ("defense", TacticalSlot::Defense),
    ];

    let martial_carrier = held_carrier.unwrap_or(CarrierAffinity::Unarmed);
    let player_loadout = PLAYER_LOADOUTS.read().get(&ent.id).cloned();

    for (cat, slot) in martial_slots {
        let studied_sent_opt = player_loadout.as_ref().and_then(|ld| match slot {
            TacticalSlot::Opening => ld.opening.as_ref(),
            TacticalSlot::Midgame => ld.midgame.as_ref(),
            TacticalSlot::Finisher => ld.finisher.as_ref(),
            TacticalSlot::Defense => ld.defense.as_ref(),
        });

        let routine = if let Some(sent) = studied_sent_opt.filter(|s| {
            !studied_manuals.is_empty()
                && s.custom_name
                    .as_deref()
                    .is_some_and(|n| {
                        !n.contains("順紋卯合")
                            && !n.contains("火候收汁")
                            && studied_manuals.iter().any(|m| n.contains(m) || m.contains(n))
                    })
        }) {
            let manual_name = sent.custom_name.as_deref().unwrap_or("承襲手札");
            let manual_word_chars: Vec<String> = sent
                .inlays
                .iter()
                .map(|inlay| {
                    registry
                        .get_by_id(&inlay.word_id)
                        .map(|e| e.ch.clone())
                        .unwrap_or_else(|| inlay.word_id.clone())
                })
                .collect();
            let (adapted, _) = crate::combat::human_phase::adapt_manual_to_bone_phase(
                &bone,
                slot,
                martial_carrier,
                manual_name,
                &manual_word_chars,
                registry,
            );
            adapted
        } else {
            crate::combat::human_phase::derive_personal_martial_routine(
                &bone,
                slot,
                martial_carrier,
                &vessel_name,
                learned_bonus,
                vessel_familiarity_count,
                registry,
            )
        };

        let mut affliction_tags = Vec::new();
        for aff in &active_afflictions {
            match aff {
                crate::pg::living_world::AfflictionKind::ArmSprain => {
                    if martial_carrier != CarrierAffinity::Footwork {
                        affliction_tags.push("【帶傷：腕肘挫傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::LegSprain => {
                    if slot == TacticalSlot::Defense || martial_carrier == CarrierAffinity::Footwork {
                        affliction_tags.push("【帶傷：下盤扭傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::TorsoInternal => {
                    affliction_tags.push("【帶傷：胸脅內傷】");
                }
                crate::pg::living_world::AfflictionKind::HeadConcussion => {
                    affliction_tags.push("【帶傷：頭面震盪】");
                }
                crate::pg::living_world::AfflictionKind::Miasma => {
                    affliction_tags.push("【帶傷：瘴氣入體】");
                }
            }
        }
        let affliction_suffix = if !affliction_tags.is_empty() {
            format!("；{}", affliction_tags.join("、"))
        } else {
            String::new()
        };

        skills.push(NaturalSkillView {
            domain: "martial".to_string(),
            category: cat.to_string(),
            category_label: format!("{}・{}", routine.slot_label_zh, routine.phase_specialty_zh),
            name: routine.formal_name,
            origin_basis: routine.origin_basis_zh,
            learned_and_familiar: format!("{}{}", routine.learned_and_familiar_zh, affliction_suffix),
            auto_trigger_desc: routine.auto_trigger_desc_zh,
        });
    }

    // ─── 領域三：百工藝相（production，4 項常駐）─────────────────────────
    let craft_specs = [
        ("harvest", held_carrier.unwrap_or(CarrierAffinity::Blade)),
        ("forge", CarrierAffinity::Blunt),
        ("culinary", CarrierAffinity::Blade),
        ("wayfare", CarrierAffinity::Footwork),
    ];

    for (craft_code, carrier) in craft_specs {
        let craft_tool_name = match craft_code {
            "harvest" => {
                if !martial_item_display.is_empty() {
                    martial_item_display.as_str()
                } else {
                    "採獵刀索與隨身器具"
                }
            }
            "forge" => "鍛錘、刻刀與百工器具",
            "culinary" => "庖廚刀勺與灶火器皿",
            _ => "行囊步履與隨身行具",
        };
        let routine = crate::combat::human_phase::derive_personal_craft_routine(
            &bone,
            craft_code,
            carrier,
            craft_tool_name,
            vessel_familiarity_count,
            anatomy_total,
            registry,
        );

        let mut craft_afflictions = Vec::new();
        for aff in &active_afflictions {
            match aff {
                crate::pg::living_world::AfflictionKind::ArmSprain => {
                    if carrier != CarrierAffinity::Footwork {
                        craft_afflictions.push("【帶傷：腕肘挫傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::LegSprain => {
                    if craft_code == "wayfare" {
                        craft_afflictions.push("【帶傷：下盤扭傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::TorsoInternal => {
                    if craft_code == "forge" || craft_code == "wayfare" {
                        craft_afflictions.push("【帶傷：胸脅內傷】");
                    }
                }
                crate::pg::living_world::AfflictionKind::HeadConcussion => {
                    if craft_code == "harvest" {
                        craft_afflictions.push("【帶傷：頭面震盪】");
                    }
                }
                crate::pg::living_world::AfflictionKind::Miasma => {
                    craft_afflictions.push("【帶傷：瘴氣入體】");
                }
            }
        }
        let craft_aff_suffix = if !craft_afflictions.is_empty() {
            format!("；{}", craft_afflictions.join("、"))
        } else {
            String::new()
        };

        skills.push(NaturalSkillView {
            domain: "production".to_string(),
            category: craft_code.to_string(),
            category_label: format!("{}・{}", routine.slot_label_zh, routine.phase_specialty_zh),
            name: routine.formal_name,
            origin_basis: routine.origin_basis_zh,
            learned_and_familiar: format!("{}{}", routine.learned_and_familiar_zh, craft_aff_suffix),
            auto_trigger_desc: routine.auto_trigger_desc_zh,
        });
    }

    // ─── 領域四：所學手札與器物歷練（mastery_ledger，3 項抽屜卡片）─────────
    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "learned_manuals".to_string(),
        category_label: "所學・師承與手札".to_string(),
        name: "師承與手札印記".to_string(),
        origin_basis: if studied_manuals.is_empty() {
            "尚未展卷研讀各家手札，全憑日常交手與勞作摸索門道。".to_string()
        } else {
            let titles = studied_manuals
                .iter()
                .map(|m| format!("《{}》", m))
                .collect::<Vec<_>>()
                .join("、");
            format!("平日展卷參詳{}等手札經卷，揣摩前人行氣與百工造物之法度。", titles)
        },
        learned_and_familiar: if imprints.is_empty() {
            "平日多憑自身揣摩習練，尚未得前輩親口指點。".to_string()
        } else {
            let s = imprints
                .iter()
                .map(|imp| format!("曾蒙{}親授「{}」心法", imp.mentor_name, imp.imprint_title))
                .collect::<Vec<_>>()
                .join("、");
            format!("{}。", s)
        },
        auto_trigger_desc: "所習手札與前輩指點已融入日常吐納與出手習慣，臨陣及勞作時自然應變。".to_string(),
    });

    let is_bladed_held = !held_item_id.is_empty()
        && crate::pg::living_world::is_bladed_vessel_item(&held_item_id, &held_item_display);
    let is_weapon_held = !held_item_id.is_empty()
        && crate::pg::living_world::is_weapon_or_tool_item(&held_item_id, &held_item_display);

    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "familiar_vessels".to_string(),
        category_label: "所熟・隨身器物磨合".to_string(),
        name: if held_item_display.is_empty() {
            "隨身持物：徒手".to_string()
        } else {
            format!("隨身持物：{}", held_item_display)
        },
        origin_basis: if held_item_display.is_empty() {
            "以血肉筋骨為器，長久修持鍛打。".to_string()
        } else if is_bladed_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!(
                "器物現況：{}（磨合度：{}）",
                vstate.status_badge(),
                vstate.familiarity().label()
            )
        } else if is_weapon_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!(
                "器物現況：器身完好（磨合度：{}）",
                vstate.familiarity().label()
            )
        } else {
            "器物現況：隨身攜行（日常器物）".to_string()
        },
        learned_and_familiar: if held_item_display.is_empty() {
            "掌拳合一，收發自如。".to_string()
        } else if is_weapon_held {
            let vstate = crate::pg::living_world::get_vessel_state(&ent.id, &held_item_id);
            format!("{}。", vstate.familiarity().feel_prose())
        } else {
            "日常隨身攜行之物，取用順手。".to_string()
        },
        auto_trigger_desc: if is_bladed_held {
            "手持兵刃經反覆磨合，發力順應刀刃重心與硬度，減免鋒口磨損並提升破綻穿透。".to_string()
        } else if is_weapon_held {
            "手持器物經反覆磨合，發力順應器身重心與分量，運使愈發沉穩順手。".to_string()
        } else {
            "供平日行旅或起居取用；臨陣交鋒時則憑拳腳身法或隨身兵刃應敵。".to_string()
        },
    });

    skills.push(NaturalSkillView {
        domain: "mastery_ledger".to_string(),
        category: "material_anatomy".to_string(),
        category_label: "所歷・山海肌理與百工".to_string(),
        name: "山海肌理與百工體察".to_string(),
        origin_basis: crate::pg::living_world::get_entity_anatomy_familiarity_summary(&ent.id),
        learned_and_familiar: if anatomy_total >= 10 {
            "長年於山海之間採獵、庖解與鍛冶，熟知各類走獸異木之關節空隙與紋理走向。".to_string()
        } else if anatomy_total > 0 {
            "於日常採集與處理獵物時留心觀察皮肉筋節，漸能順著自然紋理落手。".to_string()
        } else {
            "平日多依常理下刀運錘，尚在摸索各類物材之肌理走向。".to_string()
        },
        auto_trigger_desc: "面對堅硬外皮或礦石結構時，能順著紋理縫隙落刃下錘，避開硬處、省力耐用。".to_string(),
    });

    skills
}


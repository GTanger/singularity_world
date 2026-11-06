pub fn resolve_entity_signature_vessel(
    entity_id: &str,
    display_title: &str,
) -> Option<(&'static str, &'static str)> {
    let clue = if !display_title.trim().is_empty() {
        display_title
    } else if !entity_id.trim().is_empty() {
        entity_id
    } else {
        return None;
    };
    let vessel = crate::npc::causal_core::resolve_vocational_vessel(clue);
    Some((vessel.item_id, vessel.display_name))
}

pub fn get_or_init_entity_combat_word_casket(entity_id: &str, display_title: &str) -> WordCasket {
    crate::server::handler::inventory::ensure_player_initialized(entity_id);
    let mut casket = get_player_word_casket(entity_id);
    if casket.item_inscriptions.is_empty() {
        if let Some((vessel_id, _vessel_name)) = resolve_entity_signature_vessel(entity_id, display_title) {
            let registry = WordRegistry::global();
            crate::server::handler::inventory::populate_resident_vocational_casket(entity_id, &mut casket, registry);
            if !display_title.trim().is_empty() {
                crate::server::handler::inventory::populate_resident_vocational_casket(display_title, &mut casket, registry);
            }
            if casket.inscribed_words_of(vessel_id).is_empty() {
                for ch in ["巧", "堅"] {
                    if let Some(wids) = registry.get_by_char(ch) {
                        if let Some(wid) = wids.first() {
                            casket.add(wid.as_str(), 1);
                            let _ = casket.inscribe_item(vessel_id, wid.as_str());
                        }
                    }
                }
            }
        }
    }
    casket
}

pub fn calculate_entity_inscribed_combat_bonus(
    entity_id: &str,
    display_title: &str,
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (f64, f64, f64) {
    let (atk, pen, def) = calculate_inscribed_combat_bonus(equipment_slots, casket, registry);
    if atk > 0.0 || pen > 0.0 || def > 0.0 {
        return (atk, pen, def);
    }
    if let Some((vessel_id, _)) = resolve_entity_signature_vessel(entity_id, display_title) {
        return casket.inscribed_combat_bonus(vessel_id, registry);
    }
    (0.0, 0.0, 0.0)
}

pub fn resolve_entity_weapon_affinity_with_inscriptions(
    entity_id: &str,
    display_title: &str,
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (CarrierAffinity, Option<String>) {
    let (aff, name_opt) = resolve_equipped_weapon_affinity_with_inscriptions(equipment_slots, casket, registry);
    if name_opt.is_some() {
        return (aff, name_opt);
    }
    if let Some((vessel_id, vessel_name)) = resolve_entity_signature_vessel(entity_id, display_title) {
        let suffix = casket.format_inscribed_suffix(vessel_id, registry);
        let full_name = if suffix.is_empty() {
            format!("{vessel_name} [巧][堅]")
        } else {
            format!("{vessel_name}{suffix}")
        };
        let mut affinity = infer_item_carrier_affinity(vessel_id, vessel_name)
            .unwrap_or_else(|| classify_weapon_affinity(vessel_name, "", ""));
        if affinity == CarrierAffinity::Unarmed {
            let clue = if !display_title.trim().is_empty() {
                display_title
            } else {
                entity_id
            };
            let vessel = crate::npc::causal_core::resolve_vocational_vessel(clue);
            affinity = if vessel.is_bladed {
                CarrierAffinity::Blade
            } else if vessel_name.contains('竿')
                || vessel_name.contains('篙')
                || vessel_name.contains('杖')
                || vessel_name.contains("扁擔")
            {
                CarrierAffinity::Pole
            } else {
                CarrierAffinity::Blunt
            };
        }
        return (affinity, Some(full_name));
    }
    (CarrierAffinity::Unarmed, None)
}

fn calculate_inscribed_combat_bonus(
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (f64, f64, f64) {
    if equipment_slots.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(equipment_slots) else {
        return (0.0, 0.0, 0.0);
    };
    let mut total_atk = 0.0;
    let mut total_pen = 0.0;
    let mut total_def = 0.0;
    for (_slot, item_id) in slots {
        if !item_id.is_empty() {
            let (atk, pen, def) = casket.inscribed_combat_bonus(&item_id, registry);
            total_atk += atk;
            total_pen += pen;
            total_def += def;
        }
    }
    (total_atk, total_pen, total_def)
}

fn resolve_equipped_weapon_affinity_with_inscriptions(
    equipment_slots: &str,
    casket: &WordCasket,
    registry: &WordRegistry,
) -> (CarrierAffinity, Option<String>) {
    if equipment_slots.is_empty() {
        return (CarrierAffinity::Unarmed, None);
    }
    let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(equipment_slots) else {
        return (CarrierAffinity::Unarmed, None);
    };
    let check_slots = ["hold_r", "weapon", "main_hand", "hold", "hold_l", "tool"];
    for slot in check_slots {
        if let Some(item_id) = slots.get(slot).filter(|id| !id.is_empty()) {
            let def = db::get_item_def(item_id);
            let raw_name = def.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| {
                if item_id.is_ascii() || item_id.trim().is_empty() {
                    "隨身器具".to_string()
                } else {
                    item_id.clone()
                }
            });
            if !crate::pg::living_world::is_weapon_or_tool_item(item_id, &raw_name) {
                continue;
            }
            let suffix = casket.format_inscribed_suffix(item_id, registry);
            let full_item_name = format!("{}{}", raw_name, suffix);

            let affinity = infer_item_carrier_affinity(item_id, &raw_name)
                .unwrap_or_else(|| classify_weapon_affinity(&raw_name, "", ""));
            return (affinity, Some(full_item_name));
        }
    }
    (CarrierAffinity::Unarmed, None)
}
fn execute_tianji_wordplate_combat_internal(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32, bool) {
    let registry = WordRegistry::global();

    // 1. 取得攻方戰術配置，若有缺漏自動補齊
    let a_seed = resolve_character_seed(attacker);
    let mut a_loadout = get_player_tactical_loadout(pid);
    ensure_loadout_complete(&mut a_loadout, registry, true, a_seed);

    // 2. 構建守方適配戰術配置（真正萬人萬相：soul_seed + 760 條拓撲星軌阻力）
    let d_seed = resolve_character_seed(defender);
    let mut d_loadout = build_defender_tactical_loadout(defender, registry);
    inject_real_entity_postnatal_experience(pid, &mut a_loadout);
    inject_real_entity_postnatal_experience(&defender.id, &mut d_loadout);
    // 3. 取得攻守雙方詞匣以查詢外盤器物銘文刻字
    let a_casket = get_or_init_entity_combat_word_casket(pid, &attacker.display_title);
    let d_casket = get_or_init_entity_combat_word_casket(&defender.id, &defender.display_title);

    // 4. 計算持物裝備之銘文戰鬥加成
    let (extra_a_atk, extra_a_pen, extra_a_def) =
        calculate_entity_inscribed_combat_bonus(pid, &attacker.display_title, &attacker.equipment_slots, &a_casket, registry);
    let (extra_d_atk, extra_d_pen, extra_d_def) =
        calculate_entity_inscribed_combat_bonus(&defender.id, &defender.display_title, &defender.equipment_slots, &d_casket, registry);

    // 5. 檢查攻守裝備載體親和度共鳴（器物無分工具與武器：手持即載體），並在名稱顯化刻字後綴
    let (a_weapon_carrier, a_weapon_name) = resolve_entity_weapon_affinity_with_inscriptions(
        pid,
        &attacker.display_title,
        &attacker.equipment_slots,
        &a_casket,
        registry,
    );
    let (d_weapon_carrier, d_weapon_name) = resolve_entity_weapon_affinity_with_inscriptions(
        &defender.id,
        &defender.display_title,
        &defender.equipment_slots,
        &d_casket,
        registry,
    );
    let resonance_note = inspect_carrier_resonance(
        &a_loadout,
        a_weapon_carrier,
        a_weapon_name.as_deref(),
        registry,
    );
    let d_resonance_note = inspect_carrier_resonance(
        &d_loadout,
        d_weapon_carrier,
        d_weapon_name.as_deref(),
        registry,
    );
    // 6. 計算有效體魄與攻擊屬性（結合 a_dex, a_atk, d_dex, d_atk 與載體共鳴增益）
    let (raw_a_vit, _a_qi, raw_a_dex, a_atk) = db::effective_stats(attacker);
    let (raw_d_vit, _d_qi, raw_d_dex, d_atk) = db::effective_stats(defender);
    let a_vit = if attacker.kind == EntityKind::Player && raw_a_vit <= 30 {
        raw_a_vit * 10
    } else {
        raw_a_vit
    };
    let a_dex = if attacker.kind == EntityKind::Player && raw_a_dex <= 30 {
        raw_a_dex * 5
    } else {
        raw_a_dex
    };
    let d_vit = if defender.kind == EntityKind::Player && raw_d_vit <= 30 {
        raw_d_vit * 10
    } else {
        raw_d_vit
    };
    let d_dex = if defender.kind == EntityKind::Player && raw_d_dex <= 30 {
        raw_d_dex * 5
    } else {
        raw_d_dex
    };
    let a_res_bonus = if resonance_note.is_some() { 1.15 } else { 1.0 };
    let d_res_bonus = if d_resonance_note.is_some() { 1.15 } else { 1.0 };
    let a_breath = crate::pg::living_world::get_or_derive_breath_habit(
        pid,
        &attacker.display_title,
        a_seed,
    );
    let d_breath = crate::pg::living_world::get_or_derive_breath_habit(
        &defender.id,
        &defender.display_title,
        d_seed,
    );
    let (a_b_atk, a_b_def, a_b_pen, a_b_line) = a_breath.dominant.combat_modifiers();
    let (d_b_atk, d_b_def, d_b_pen, _) = d_breath.dominant.combat_modifiers();

    let _effective_a_atk = ((a_atk as f64) + extra_a_atk + (a_dex as f64 * 0.25)) * a_res_bonus * a_b_atk;
    let _effective_d_atk = ((d_atk as f64) + extra_d_atk + (d_dex as f64 * 0.25)) * d_res_bonus * d_b_atk;
    let effective_a_pen = extra_a_pen + a_b_pen;
    let effective_d_pen = extra_d_pen + d_b_pen;

    let cur_a_hp = ((a_vit as f64 + extra_a_def) * a_b_def).max(1.0).round() as i32;
    let cur_d_hp = ((d_vit as f64 + extra_d_def) * d_b_def).max(1.0).round() as i32;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let a_afflictions = crate::pg::living_world::get_entity_afflictions(pid);
    let d_afflictions = crate::pg::living_world::get_entity_afflictions(&defender.id);

    let calc_affliction_penalty = |afflictions: &[AfflictionKind]| -> f64 {
        let count = afflictions
            .iter()
            .filter(|a| {
                matches!(
                    a,
                    AfflictionKind::ArmSprain
                        | AfflictionKind::TorsoInternal
                        | AfflictionKind::HeadConcussion
                        | AfflictionKind::LegSprain
                        | AfflictionKind::Miasma
                )
            })
            .count();
        if count == 0 {
            1.0
        } else {
            (0.88f64).powi(count as i32).max(0.65)
        }
    };
    let penalty_a = calc_affliction_penalty(&a_afflictions);
    let penalty_d = calc_affliction_penalty(&d_afflictions);

    // 7. 呼叫田忌賽馬時序對決結算（融入量級權重與專克／一力降十會裁決）
    let weight_a =
        (((a_atk as f64 + extra_a_atk) * 0.55) + (cur_a_hp as f64) * 0.45 + effective_a_pen * 0.35) * penalty_a;
    let weight_b =
        (((d_atk as f64 + extra_d_atk) * 0.55) + (cur_d_hp as f64) * 0.45 + effective_d_pen * 0.35) * penalty_d;
    let matchup_result = resolve_tianji_matchup_with_weights(
        &a_loadout,
        cur_a_hp,
        &d_loadout,
        cur_d_hp,
        weight_a,
        weight_b,
        registry,
    );

    let (raw_winner, mut rounds, side_a_final, side_b_final) = match matchup_result {
        Ok(report) => (
            report.winner,
            report.rounds,
            report.side_a_final_hp,
            report.side_b_final_hp,
        ),
        Err(_) => {
            return (
                "雙方氣機交錯，各自收勢退開。".to_string(),
                "draw".to_string(),
                cur_a_hp,
                cur_d_hp,
                false,
            );
        }
    };

    let fight_winner = match raw_winner.as_str() {
        "side_a" => "attacker".to_string(),
        "side_b" => "defender".to_string(),
        _ => "draw".to_string(),
    };

    // 8. 計算血量結算（Subdue 切磋留人絕不致死，Slay 決一死戰可奪命）
    let (raw_final_a, raw_final_d) = if subdue {
        (side_a_final.max(1), side_b_final.max(1))
    } else if fight_winner == "attacker" {
        let hp_d = if defender.id.starts_with("ambient_") || defender.id.starts_with("shanhai_") || side_b_final <= 5 {
            0
        } else {
            crate::pg::living_world::record_entity_affliction(
                &defender.id,
                AfflictionKind::TorsoInternal,
                "combat_scuffle",
                now,
            );
            side_b_final.max(1)
        };
        (side_a_final.max(1), hp_d)
    } else {
        (side_a_final.max(1), side_b_final.max(1))
    };
    let final_a_hp = raw_final_a.min(attacker.vit);
    let final_d_hp = raw_final_d.min(defender.vit);

    // 9. 生成 100% 完全去數值化武俠感官散文
    let a_name = attacker
        .display_title
        .rsplit(['|', '｜'])
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("你")
        .to_string();
    let mut d_name = super::player_visible_target_name(pid, defender);
    if d_name.is_empty() || d_name.is_ascii() {
        d_name = if defender.id.is_ascii() || defender.id.trim().is_empty() {
            "陌生人".to_string()
        } else {
            defender.id.clone()
        };
    }
    let d_is_beast = is_wild_creature_entity(defender);
    let effective_d_weapon_label = if d_is_beast {
        let anatomy = crate::combat::human_phase::BeastAnatomy::from_species_title(&defender.display_title);
        for r in &mut rounds {
            let (beast_skill, beast_motions) =
                crate::combat::human_phase::natural_beast_moves(anatomy, r.slot, (d_seed.unsigned_abs()).wrapping_add(r.phase as u64));
            r.b_technique_name = beast_skill.to_string();
            r.b_motions = beast_motions;
            if matches!(r.a_target_part, TargetPart::Weapon | TargetPart::Arm) {
                r.a_target_part = TargetPart::Torso;
            }
        }
        anatomy.carrier_label()
    } else {
        d_weapon_name.as_deref().unwrap_or("徒手")
    };

    let mut prose_log = render_dequantified_combat_prose(
        pid,
        &defender.id,
        now,
        &a_name,
        &d_name,
        subdue,
        &fight_winner,
        &rounds,
        resonance_note.as_deref(),
        defender.soul_seed,
        a_weapon_name.as_deref().unwrap_or("徒手"),
        effective_d_weapon_label,
        a_weapon_carrier,
        d_weapon_carrier,
    );
    prose_log.push_str(&format!("\n{a_b_line}"));

    crate::pg::living_world::record_breath_activity(
        pid,
        crate::pg::living_world::BreathHabitKind::BattleFierce,
        2,
        now,
    );

    let weapon_id_opt: Option<String> = if !attacker.equipment_slots.is_empty() {
        if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&attacker.equipment_slots) {
            let check_slots = ["weapon", "hold_r", "main_hand", "hold", "tool", "hold_l"];
            check_slots.iter().find_map(|s| slots.get(*s).cloned().filter(|id| !id.is_empty()))
        } else {
            None
        }
    } else {
        None
    };

    if let Some(weapon_id) = weapon_id_opt {
        let w_name = a_weapon_name.as_deref().unwrap_or("");
        if crate::pg::living_world::is_bladed_vessel_item(&weapon_id, w_name) {
            let mut vstate = crate::pg::living_world::record_vessel_use(pid, &weapon_id, 1, 1, now);
            let weapon_clashed = rounds.iter().any(|r| {
                r.b_target_part == TargetPart::Weapon
                    || (matches!(r.a_carrier, CarrierAffinity::Blade | CarrierAffinity::Pole) && matches!(r.b_carrier, CarrierAffinity::Blunt | CarrierAffinity::Shield | CarrierAffinity::Pole))
            });
            if weapon_clashed {
                vstate = crate::pg::living_world::record_vessel_use(pid, &weapon_id, 2, 1, now);
            }
            match vstate.condition {
                crate::pg::living_world::VesselCondition::SlightlyDull => {
                    prose_log.push_str("\n【器況體察】連番交鋒硬撼，手中兵刃已顯「刃口微鈍」，戰後宜尋街邊青石磨刀槽推磨開鋒。");
                }
                crate::pg::living_world::VesselCondition::RolledEdge => {
                    prose_log.push_str("\n【器況體察】連番交鋒硬撼，手中兵刃已現「捲刃待修」，宜赴鍛造工坊回火修整。");
                }
                crate::pg::living_world::VesselCondition::Sharp => {}
            }
        }
    }
    let a_posture_broken = rounds
        .last()
        .map(|r| r.a_posture_after == PostureState::Broken)
        .unwrap_or(false);

    (prose_log, fight_winner, final_a_hp, final_d_hp, a_posture_broken)
}


#[cfg(test)]
pub fn execute_tianji_wordplate_combat(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32) {
    let (prose, winner, a_hp, d_hp, _) =
        execute_tianji_wordplate_combat_internal(pid, attacker, defender, subdue);
    (prose, winner, a_hp, d_hp)
}


fn build_attack_narrative_with_broken(
    pid: &str,
    attacker: &Character,
    defender: &Character,
    subdue: bool,
) -> (String, String, i32, i32, bool) {
    execute_tianji_wordplate_combat_internal(pid, attacker, defender, subdue)
}

fn resolve_character_seed(c: &Character) -> i64 {
    crate::combat::human_phase::resolve_entity_soul_seed(&c.id, c.soul_seed)
}

fn is_wild_creature_id_and_title(id: &str, title: &str) -> bool {
    if id.starts_with("shanhai_") || id.starts_with("ambient_flock_") {
        return true;
    }
    if id.starts_with("ambient_") {
        return crate::game::ambient_creature_loot(title).is_some();
    }
    false
}

fn is_wild_creature_entity(c: &Character) -> bool {
    is_wild_creature_id_and_title(&c.id, &c.display_title)
}

fn build_defender_tactical_loadout(
    defender: &Character,
    registry: &WordRegistry,
) -> TacticalLoadout {
    // 1. 若守方已有自訂／持久化之戰術配置，優先使用
    let existing_loadout = get_player_tactical_loadout(&defender.id);
    let casket = get_player_word_casket(&defender.id);
    let d_seed = resolve_character_seed(defender);

    if existing_loadout.opening.is_some()
        || existing_loadout.midgame.is_some()
        || existing_loadout.finisher.is_some()
    {
        let mut loadout = existing_loadout;
        ensure_loadout_complete(&mut loadout, registry, false, d_seed);
        return loadout;
    }

    // 2. 徹底移除舊版硬編碼模板，依據生命種子（soul_seed）＋ 760 條星軌拓撲展開真正的萬人萬相
    crate::combat::wordplate::build_loadout_from_soul_seed(d_seed, Some(&casket), registry)
}

fn inject_real_entity_postnatal_experience(entity_id: &str, loadout: &mut TacticalLoadout) {
    let studied_manuals = crate::pg::living_world::get_entity_studied_manuals(entity_id);
    let learned_delta = (studied_manuals.len() as f64 * 0.25).min(1.0);
    let anatomy_fam = crate::pg::living_world::get_entity_anatomy_familiarity_total(entity_id);

    let max_vessel_use = crate::pg::living_world::get_entity_max_vessel_use_count(entity_id);
    let practice_count = max_vessel_use.max(25);

    let apply_to_sentence = |sentence_opt: &mut Option<StarSentence>| {
        if let Some(s) = sentence_opt.as_mut() {
            s.learned_delta = Some(learned_delta);
            s.anatomy_familiarity = Some(anatomy_fam);
            s.practice_count = Some(practice_count);
        }
    };

    apply_to_sentence(&mut loadout.opening);
    apply_to_sentence(&mut loadout.midgame);
    apply_to_sentence(&mut loadout.finisher);
    apply_to_sentence(&mut loadout.defense);
}

fn make_3word_sentence(
    nodes: [&str; 3],
    chars: [&str; 3],
    name: &str,
    registry: &WordRegistry,
    fallback_offset: usize,
) -> StarSentence {
    let get_id = |ch: &str, offset: usize| -> String {
        if let Some(ids) = registry.get_by_char(ch)
            && let Some(first) = ids.first()
        {
            first.clone()
        } else if let Some(elem) = registry.ordered_elements().nth(offset) {
            elem.id.clone()
        } else {
            "wu".to_string()
        }
    };
    let inlays = vec![
        StarInlay::new(nodes[0], get_id(chars[0], fallback_offset)),
        StarInlay::new(nodes[1], get_id(chars[1], fallback_offset + 1)),
        StarInlay::new(nodes[2], get_id(chars[2], fallback_offset + 2)),
    ];
    StarSentence::new(inlays)
        .unwrap_or_else(|_| {
            let elems: Vec<String> = registry
                .ordered_elements()
                .take(3)
                .map(|e| e.id.clone())
                .collect();
            StarSentence::new(vec![
                StarInlay::new(nodes[0], elems.first().cloned().unwrap_or_default()),
                StarInlay::new(nodes[1], elems.get(1).cloned().unwrap_or_default()),
                StarInlay::new(nodes[2], elems.get(2).cloned().unwrap_or_default()),
            ])
            .expect("fallback sentence must be valid")
        })
        .with_custom_name(name)
}

fn ensure_loadout_complete(
    loadout: &mut TacticalLoadout,
    registry: &WordRegistry,
    is_attacker: bool,
    seed: i64,
) {
    let seed_loadout = crate::combat::wordplate::build_loadout_from_soul_seed(seed, None, registry);
    let offset_base = if is_attacker { 1 } else { 13 };
    let is_legacy_template = |s: &Option<StarSentence>| -> bool {
        match s.as_ref().and_then(|st| st.custom_name.as_deref()) {
            Some("疾刺斬" | "流破斬" | "吾震破" | "凝流步") => true,
            _ => false,
        }
    };
    if loadout.opening.is_none() || is_legacy_template(&loadout.opening) {
        if let Some(s) = seed_loadout.opening {
            loadout.set_slot(TacticalSlot::Opening, s);
        } else {
            let n1 = format!("N{:03}", offset_base);
            let n2 = format!("N{:03}", offset_base + 1);
            let n3 = format!("N{:03}", offset_base + 2);
            let mut sentence = make_3word_sentence([&n1, &n2, &n3], ["疾", "刺", "斬"], "迎風破隙勢", registry, 0);
            sentence.set_soul_seed(seed);
            loadout.set_slot(TacticalSlot::Opening, sentence);
        }
    } else if let Some(s) = loadout.opening.as_mut() {
        if s.soul_seed.is_none() {
            s.set_soul_seed(seed);
        }
    }
    if loadout.midgame.is_none() || is_legacy_template(&loadout.midgame) {
        if let Some(s) = seed_loadout.midgame {
            loadout.set_slot(TacticalSlot::Midgame, s);
        } else {
            let n4 = format!("N{:03}", offset_base + 3);
            let n5 = format!("N{:03}", offset_base + 4);
            let n6 = format!("N{:03}", offset_base + 5);
            let mut sentence = make_3word_sentence([&n4, &n5, &n6], ["流", "破", "斬"], "行雲連環擊", registry, 3);
            sentence.set_soul_seed(seed);
            loadout.set_slot(TacticalSlot::Midgame, sentence);
        }
    } else if let Some(s) = loadout.midgame.as_mut() {
        if s.soul_seed.is_none() {
            s.set_soul_seed(seed);
        }
    }
    if loadout.finisher.is_none() || is_legacy_template(&loadout.finisher) {
        if let Some(s) = seed_loadout.finisher {
            loadout.set_slot(TacticalSlot::Finisher, s);
        } else {
            let n7 = format!("N{:03}", offset_base + 6);
            let n8 = format!("N{:03}", offset_base + 7);
            let n9 = format!("N{:03}", offset_base + 8);
            let mut sentence = make_3word_sentence([&n7, &n8, &n9], ["吾", "震", "破"], "雷霆貫骨破", registry, 6);
            sentence.set_soul_seed(seed);
            loadout.set_slot(TacticalSlot::Finisher, sentence);
        }
    } else if let Some(s) = loadout.finisher.as_mut() {
        if s.soul_seed.is_none() {
            s.set_soul_seed(seed);
        }
    }
    if loadout.defense.is_none() || is_legacy_template(&loadout.defense) {
        if let Some(s) = seed_loadout.defense {
            loadout.set_slot(TacticalSlot::Defense, s);
        } else {
            let n10 = format!("N{:03}", offset_base + 9);
            let n11 = format!("N{:03}", offset_base + 10);
            let n12 = format!("N{:03}", offset_base + 11);
            let mut sentence = make_3word_sentence([&n10, &n11, &n12], ["凝", "流", "步"], "沉步卸勁身", registry, 9);
            sentence.set_soul_seed(seed);
            loadout.set_slot(TacticalSlot::Defense, sentence);
        }
    } else if let Some(s) = loadout.defense.as_mut() {
        if s.soul_seed.is_none() {
            s.set_soul_seed(seed);
        }
    }
}


fn classify_weapon_affinity(name: &str, desc: &str, item_type: &str) -> CarrierAffinity {
    let lower = format!("{name} {desc} {item_type}").to_lowercase();
    if lower.contains('劍')
        || lower.contains('刀')
        || lower.contains('刃')
        || lower.contains('匕')
        || lower.contains("blade")
        || lower.contains("sword")
        || lower.contains("dagger")
    {
        CarrierAffinity::Blade
    } else if lower.contains('槍')
        || lower.contains('矛')
        || lower.contains('戟')
        || lower.contains('棍')
        || lower.contains('杖')
        || lower.contains("pole")
        || lower.contains("spear")
        || lower.contains("lance")
        || lower.contains("staff")
    {
        CarrierAffinity::Pole
    } else if lower.contains('錘')
        || lower.contains('斧')
        || lower.contains('鎬')
        || lower.contains('鈍')
        || lower.contains("blunt")
        || lower.contains("hammer")
        || lower.contains("mace")
        || lower.contains("axe")
    {
        CarrierAffinity::Blunt
    } else if lower.contains('盾') || lower.contains("shield") {
        CarrierAffinity::Shield
    } else {
        CarrierAffinity::Unarmed
    }
}

fn inspect_carrier_resonance(
    loadout: &TacticalLoadout,
    weapon_affinity: CarrierAffinity,
    weapon_name: Option<&str>,
    registry: &WordRegistry,
) -> Option<String> {
    let resonates = [&loadout.opening, &loadout.midgame, &loadout.finisher]
        .iter()
        .filter_map(|s| s.as_ref())
        .any(|sentence| {
            if let Ok(tech) = sentence.synthesize(registry) {
                tech.primary_carrier == weapon_affinity
            } else {
                false
            }
        });

    if resonates {
        let clean_w = weapon_name.map(crate::ai::narrative::clean_vessel_name);
        let w_ref = clean_w.as_deref().filter(|s| !s.is_empty()).or(weapon_name);
        match weapon_affinity {
            CarrierAffinity::Blade => {
                if let Some(w) = w_ref {
                    Some(format!(
                        "手握【{w}】，刀劍鋒芒與招式吞吐相通，森寒劍意直逼肺腑！"
                    ))
                } else {
                    Some("劍氣縱橫破空，式意與刀劍鋒芒渾然天成！".to_string())
                }
            }
            CarrierAffinity::Pole => {
                if let Some(w) = w_ref {
                    Some(format!(
                        "緊握【{w}】，長兵揮灑生風，槍勢勢大力沉貫穿長空！"
                    ))
                } else {
                    Some("長槍破空長嘶，氣芒大開大闔，銳不可當！".to_string())
                }
            }
            CarrierAffinity::Blunt => {
                if let Some(w) = w_ref {
                    Some(format!(
                        "揮舞【{w}】，重錘剛猛無匹，威沉如泰山崩岳！"
                    ))
                } else {
                    Some("剛猛重擊破風而落，勢若奔雷下凡！".to_string())
                }
            }
            CarrierAffinity::Shield => {
                if let Some(w) = w_ref {
                    Some(format!(
                        "挽起【{w}】，盾隨身轉，攻守圓融固若金湯！"
                    ))
                } else {
                    Some("堅盾化勁護體，守勢滴水不漏！".to_string())
                }
            }
            CarrierAffinity::Unarmed => {
                Some("身形空靈，雙掌氣韻流轉，勁力透骨化有形為無形！".to_string())
            }
            CarrierAffinity::Footwork => {
                Some("步法神妙飄逸，踏風而行，身影虛實交錯！".to_string())
            }
        }
    } else {
        None
    }
}

fn render_dequantified_combat_prose(
    a_id: &str,
    d_id: &str,
    now: i64,
    a_name: &str,
    d_name: &str,
    subdue: bool,
    fight_winner: &str,
    rounds: &[crate::combat::wordplate::PhaseClashRecord],
    resonance_note: Option<&str>,
    defender_seed: Option<i64>,
    a_weapon_name: &str,
    d_weapon_name: &str,
    a_weapon_carrier: CarrierAffinity,
    d_weapon_carrier: CarrierAffinity,
) -> String {
    let mut lines = Vec::new();

    // 意圖起手
    let intent_line = if subdue {
        format!("你凝神收住力道，向【{d_name}】展開制伏攻擊！")
    } else {
        format!("你目光如炬，向【{d_name}】全力出擊，意在致死！")
    };
    lines.push(intent_line);

    // 載體共鳴提示
    if let Some(res) = resonance_note {
        lines.push(format!("（{res}）"));
    }

    let d_is_beast = is_wild_creature_id_and_title(d_id, d_name);

    // 身形感知
    if d_is_beast {
        lines.push(format!(
            "（【{d_name}】雙目機警、野性勃發，周身繃緊蓄勢待撲）"
        ));
    } else if let Some(seed) = defender_seed.filter(|&s| s != 0) {
        let style = combat_style_flavor(seed);
        lines.push(format!("（【{d_name}】氣息沉凝，身法動靜間盡顯{style}之象）"));
    }

    lines.push(String::new());

    let mut packets = Vec::new();
    for record in rounds {
        if record.slot == TacticalSlot::Defense {
            continue;
        }

        let a_motions = if record.a_motions.is_empty() || record.a_carrier != a_weapon_carrier {
            derive_elementary_motions_from_chars(&[], a_weapon_carrier, record.a_target_part)
        } else {
            record.a_motions.clone()
        };

        let b_motions = if d_is_beast {
            if record.b_motions.is_empty() {
                let anatomy = crate::combat::human_phase::BeastAnatomy::from_species_title(d_name);
                crate::combat::human_phase::natural_beast_moves(anatomy, record.slot, record.phase as u64).1
            } else {
                record.b_motions.clone()
            }
        } else if record.b_motions.is_empty() || record.b_carrier != d_weapon_carrier {
            derive_elementary_motions_from_chars(&[], d_weapon_carrier, record.b_target_part)
        } else {
            record.b_motions.clone()
        };

        let winner_side = if record.a_manifested_as_skill && !record.b_manifested_as_skill {
            "attacker"
        } else if record.b_manifested_as_skill && !record.a_manifested_as_skill {
            "defender"
        } else if record.damage_to_b > record.damage_to_a {
            "attacker"
        } else if record.damage_to_a > record.damage_to_b {
            "defender"
        } else {
            "draw"
        };

        let active_residue = record.active_residue_before.map(|r| r.label().to_string());

        let (residue_interaction, loser_posture_after, inflicted_injury) = match winner_side {
            "attacker" => {
                let res_inter = record.a_residue_interaction.map(|i| i.label().to_string());
                let posture = if record.b_posture_after != PostureState::Steady {
                    Some(record.b_posture_after.label().to_string())
                } else {
                    None
                };
                let inj = if !subdue && (record.damage_to_b >= 18 || record.b_posture_after == PostureState::Broken) {
                    let (label, kind_opt) = match record.a_target_part {
                        TargetPart::Head => (Some("頭面震盪"), Some(AfflictionKind::HeadConcussion)),
                        TargetPart::Torso => (Some("胸脅內傷"), Some(AfflictionKind::TorsoInternal)),
                        TargetPart::Arm => (Some("腕肘挫傷"), Some(AfflictionKind::ArmSprain)),
                        TargetPart::Leg => (Some("下盤扭傷"), Some(AfflictionKind::LegSprain)),
                        TargetPart::Weapon => (Some("兵刃相交硬磕之下，虎口劇震、刃口隱現磨損微鈍之跡"), None),
                        TargetPart::SelfTarget => (None, None),
                    };
                    if let Some(kind) = kind_opt {
                        crate::pg::living_world::record_entity_affliction(d_id, kind, "combat", now);
                    }
                    label.map(str::to_string)
                } else {
                    None
                };
                (res_inter, posture, inj)
            }
            "defender" => {
                let res_inter = record.b_residue_interaction.map(|i| i.label().to_string());
                let posture = if record.a_posture_after != PostureState::Steady {
                    Some(record.a_posture_after.label().to_string())
                } else {
                    None
                };
                let inj = if !subdue && (record.damage_to_a >= 18 || record.a_posture_after == PostureState::Broken) {
                    let (label, kind_opt) = match record.b_target_part {
                        TargetPart::Head => (Some("頭面震盪"), Some(AfflictionKind::HeadConcussion)),
                        TargetPart::Torso => (Some("胸脅內傷"), Some(AfflictionKind::TorsoInternal)),
                        TargetPart::Arm => (Some("腕肘挫傷"), Some(AfflictionKind::ArmSprain)),
                        TargetPart::Leg => (Some("下盤扭傷"), Some(AfflictionKind::LegSprain)),
                        TargetPart::Weapon => (Some("兵刃相交硬磕之下，虎口劇震、刃口隱現磨損微鈍之跡"), None),
                        TargetPart::SelfTarget => (None, None),
                    };
                    if let Some(kind) = kind_opt {
                        crate::pg::living_world::record_entity_affliction(a_id, kind, "combat", now);
                    }
                    label.map(str::to_string)
                } else {
                    None
                };
                (res_inter, posture, inj)
            }
            _ => (None, None, None),
        };

        let packet = match winner_side {
            "attacker" => {
                let severity_label = if record.damage_to_b >= 25 {
                    "重創失據".to_string()
                } else if record.damage_to_b >= 12 {
                    "氣血翻湧".to_string()
                } else {
                    "輕微擦震".to_string()
                };
                CombatBoutFactPacket {
                    slot: record.slot,
                    attacker_name: a_name.to_string(),
                    defender_name: d_name.to_string(),
                    attacker_weapon: a_weapon_name.to_string(),
                    defender_weapon: d_weapon_name.to_string(),
                    attacker_carrier: a_weapon_carrier,
                    defender_carrier: d_weapon_carrier,
                    winner_side: "attacker".to_string(),
                    winner_skill_name: record.a_technique_name.clone(),
                    loser_skill_name: record.b_technique_name.clone(),
                    winner_motions: a_motions,
                    loser_attempted_motions: b_motions
                        .into_iter()
                        .take(record.b_completed_motions.max(1))
                        .collect(),
                    counter_reason: record.counter_reason.clone(),
                    hit_part: record.a_target_part,
                    severity_label,
                    active_residue,
                    residue_interaction,
                    loser_posture_after,
                    inflicted_injury,
                }
            }
            "defender" => {
                let severity_label = if record.damage_to_a >= 25 {
                    "重創失據".to_string()
                } else if record.damage_to_a >= 12 {
                    "氣血翻湧".to_string()
                } else {
                    "輕微擦震".to_string()
                };
                CombatBoutFactPacket {
                    slot: record.slot,
                    attacker_name: a_name.to_string(),
                    defender_name: d_name.to_string(),
                    attacker_weapon: a_weapon_name.to_string(),
                    defender_weapon: d_weapon_name.to_string(),
                    attacker_carrier: a_weapon_carrier,
                    defender_carrier: d_weapon_carrier,
                    winner_side: "defender".to_string(),
                    winner_skill_name: record.b_technique_name.clone(),
                    loser_skill_name: record.a_technique_name.clone(),
                    winner_motions: b_motions,
                    loser_attempted_motions: a_motions
                        .into_iter()
                        .take(record.a_completed_motions.max(1))
                        .collect(),
                    counter_reason: record.counter_reason.clone(),
                    hit_part: record.b_target_part,
                    severity_label,
                    active_residue,
                    residue_interaction,
                    loser_posture_after,
                    inflicted_injury,
                }
            }
            _ => CombatBoutFactPacket {
                slot: record.slot,
                attacker_name: a_name.to_string(),
                defender_name: d_name.to_string(),
                attacker_weapon: a_weapon_name.to_string(),
                defender_weapon: d_weapon_name.to_string(),
                attacker_carrier: a_weapon_carrier,
                defender_carrier: d_weapon_carrier,
                winner_side: "draw".to_string(),
                winner_skill_name: String::new(),
                loser_skill_name: String::new(),
                winner_motions: vec![],
                loser_attempted_motions: a_motions
                    .into_iter()
                    .take(record.a_completed_motions.max(1))
                    .collect(),
                counter_reason: record.counter_reason.clone(),
                hit_part: record.a_target_part,
                severity_label: "輕微擦震".to_string(),
                active_residue,
                residue_interaction: None,
                loser_posture_after: None,
                inflicted_injury: None,
            },
        };
        packets.push(packet);
    }

    let bout_lines = compose_combat_bouts_concurrent(&packets);
    lines.extend(bout_lines);

    lines.push(String::new());

    let summary = if d_is_beast {
        match (subdue, fight_winner) {
            (true, "attacker") => format!("雙方身形驟分。你收住最後一分勁道，看準破綻將【{d_name}】生擒制伏！"),
            (true, "defender") => format!("【{d_name}】振身掙脫束縛、借勢竄入林隙草叢，你未能將其制伏。"),
            (true, _) => format!("數合追逐交鋒，【{d_name}】警覺退入暗影與你拉開距離，暫時僵持。"),
            (false, "attacker") => format!("塵埃落定。你一鼓作氣瓦解【{d_name}】凶勢，奪下勝局！"),
            (false, "defender") => format!("狂瀾散去。【{d_name}】兇性大發反撲得手，你力有不逮，暫退避其鋒芒。"),
            (false, _) => format!("漫天塵土消散，你與【{d_name}】隔空對峙，彼此伺機而動。"),
        }
    } else {
        match (subdue, fight_winner) {
            (true, "attacker") => {
                "雙方身形驟分。你收住最後一分力道，將對方徹底制伏。".to_string()
            }
            (true, "defender") => {
                "雙方收招罷手。對方身法沉穩老練，你未能破開其守勢，暫落下風。".to_string()
            }
            (true, _) => "數合交手告終，兩人氣息相若，各自點頭收勢。".to_string(),
            (false, "attacker") => "塵埃落定。你一鼓作氣瓦解敵勢，奪下勝局！".to_string(),
            (false, "defender") => "狂瀾散去。對方招式老辣，你力有不逮，敗下陣來。".to_string(),
            (false, _) => "漫天塵土消散，雙方互不相讓，戰局陷入對峙。".to_string(),
        }
    };
    lines.push(summary);

    lines.join("\n")
}

fn combat_style_flavor(seed: i64) -> &'static str {
    let (alpha, beta, gamma) = db::expand_soul_seed_to_combat_axes(seed);
    let norm_amp = (alpha - 0.5) / 1.5;
    let norm_freq = (beta - 0.5) / 1.0;
    let norm_phase = (gamma - 0.05) / 0.15;
    if norm_amp >= norm_freq && norm_amp >= norm_phase {
        "剛猛"
    } else if norm_freq >= norm_amp && norm_freq >= norm_phase {
        "疾促"
    } else {
        "沉穩"
    }
}

fn broadcast_npc_death(
    conn: &WsConnection,
    death_room: &str,
    npc_id: &str,
    name: &str,
    now: i64,
) {
    let payload = format!("在{death_room}倒下");
    log_npc_event(now, npc_id, EVT_DEATH, &payload);
    let safe_name = if name.is_ascii() || name.trim().is_empty() {
        "陌生人"
    } else {
        name
    };
    let narrate_text = format!("傳來消息：【{safe_name}】倒下了。");
    let mut rooms: HashSet<String> = HashSet::new();
    rooms.insert(death_room.to_string());
    db::with_room_graph(|g| {
        for nb in g.neighbors(death_room) {
            rooms.insert(nb);
        }
    });
    if let Ok(assignments) = get_assignments_for_entity(npc_id) {
        for a in assignments {
            if let Ok(Some(vr)) = get_room_ids_for_venue(&a.venue_id) {
                for rid in vr {
                    rooms.insert(rid);
                }
            }
        }
    }
    let store_ref: &SessionStore = &conn.sessions;
    for rid in rooms {
        send_narrate_to_room(store_ref, &rid, &narrate_text);
    }
}


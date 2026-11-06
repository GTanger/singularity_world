
/// 單序位交鋒判定（融合相同量級專克拆招、跨量級一力降十會、戰場殘留氣機與架勢判定）
pub fn resolve_bout(
    slot: TacticalSlot,
    phase: usize,
    tech_a: Option<&SynthesizedTechnique>,
    tech_b: Option<&SynthesizedTechnique>,
    momentum_a: f64,
    momentum_b: f64,
    weight_a: f64,
    weight_b: f64,
    def_mit_a: f64,
    def_mit_b: f64,
    a_flawed_prior: bool,
    b_flawed_prior: bool,
) -> BoutClashOutcome {
    resolve_bout_with_state(
        slot,
        phase,
        tech_a,
        tech_b,
        momentum_a,
        momentum_b,
        weight_a,
        weight_b,
        def_mit_a,
        def_mit_b,
        a_flawed_prior,
        b_flawed_prior,
        PostureState::Steady,
        PostureState::Steady,
        None,
    )
}

/// 帶入架勢狀態與殘留氣場之單序位完整交鋒判定
pub fn resolve_bout_with_state(
    slot: TacticalSlot,
    _phase: usize,
    tech_a: Option<&SynthesizedTechnique>,
    tech_b: Option<&SynthesizedTechnique>,
    momentum_a: f64,
    momentum_b: f64,
    weight_a: f64,
    weight_b: f64,
    def_mit_a: f64,
    def_mit_b: f64,
    a_flawed_prior: bool,
    b_flawed_prior: bool,
    a_prior_posture: PostureState,
    b_prior_posture: PostureState,
    active_residue: Option<BattlefieldResidue>,
) -> BoutClashOutcome {
    let name_a = tech_a.map(|t| t.effective_name.clone()).unwrap_or_else(|| "隨風拂袖".to_string());
    let name_b = tech_b.map(|t| t.effective_name.clone()).unwrap_or_else(|| "凝神自守".to_string());

    let (raw_score_a, pen_a, shock_a) = if let Some(t) = tech_a {
        let sc = match slot {
            TacticalSlot::Opening => t.vector_12d.tempo_early * 1.6 + t.vector_12d.focus * 1.0,
            TacticalSlot::Midgame => t.vector_12d.tempo_mid * 1.5 + t.vector_12d.flow * 1.2,
            TacticalSlot::Finisher => t.vector_12d.tempo_late * 1.8 + t.vector_12d.shock * 1.5,
            TacticalSlot::Defense => t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2,
        };
        (sc, t.vector_12d.penetration, t.vector_12d.shock)
    } else {
        (0.25, 0.1, 0.1)
    };

    let (raw_score_b, pen_b, shock_b) = if let Some(t) = tech_b {
        let sc = match slot {
            TacticalSlot::Opening => t.vector_12d.tempo_early * 1.6 + t.vector_12d.focus * 1.0,
            TacticalSlot::Midgame => t.vector_12d.tempo_mid * 1.5 + t.vector_12d.flow * 1.2,
            TacticalSlot::Finisher => t.vector_12d.tempo_late * 1.8 + t.vector_12d.shock * 1.5,
            TacticalSlot::Defense => t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2,
        };
        (sc, t.vector_12d.penetration, t.vector_12d.shock)
    } else {
        (0.25, 0.1, 0.1)
    };

    let res_tier_a = tech_a.map(|t| t.resonance_tier()).unwrap_or(1);
    let res_tier_b = tech_b.map(|t| t.resonance_tier()).unwrap_or(1);
    let res_mult_a = tech_a.map(|t| t.resonance_multiplier()).unwrap_or(1.0);
    let res_mult_b = tech_b.map(|t| t.resonance_multiplier()).unwrap_or(1.0);

    let mut score_a = raw_score_a * momentum_a * res_mult_a;
    let mut score_b = raw_score_b * momentum_b * res_mult_b;

    // 殘留氣機交互作用判定
    let a_residue_interaction = active_residue.and_then(|res| tech_a.and_then(|t| t.check_residue_interaction(res)));
    let b_residue_interaction = active_residue.and_then(|res| tech_b.and_then(|t| t.check_residue_interaction(res)));

    if let Some(ResidueInteraction::Resonance) = a_residue_interaction {
        score_a *= 1.15;
    } else if let Some(ResidueInteraction::Backlash) = a_residue_interaction {
        score_a *= 0.88;
    }

    if let Some(ResidueInteraction::Resonance) = b_residue_interaction {
        score_b *= 1.15;
    } else if let Some(ResidueInteraction::Backlash) = b_residue_interaction {
        score_b *= 0.88;
    }

    // 1. 跨量級碾壓（為什麼近身競技要分級，因為「一力降十會」！）
    let ratio_a = weight_a / weight_b.max(0.1);
    let ratio_b = weight_b / weight_a.max(0.1);
    let mut counter_reason = None;

    let a_evades_heavy_b = if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;
        (v_a.penetration + v_a.flow >= 2.6)
            && (v_a.tempo_early >= v_b.tempo_early + 0.3)
            && (v_b.sustain < 0.5)
    } else {
        false
    };
    let b_evades_heavy_a = if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;
        (v_b.penetration + v_b.flow >= 2.6)
            && (v_b.tempo_early >= v_a.tempo_early + 0.3)
            && (v_a.sustain < 0.5)
    } else {
        false
    };

    if ratio_a >= 1.45 && !b_evades_heavy_a {
        score_a += (ratio_a - 1.0) * 1.8;
        counter_reason = Some("重勢硬壓：挾千鈞重勢正面硬壓，沛然巨力直接震散對方巧勁架勢".to_string());
    } else if ratio_b >= 1.45 && !a_evades_heavy_b {
        score_b += (ratio_b - 1.0) * 1.8;
        counter_reason = Some("重勢硬壓：挾千鈞重勢正面硬壓，沛然巨力直接震散對方巧勁架勢".to_string());
    } else if ratio_b >= 1.45 && a_evades_heavy_b {
        score_a += 0.75;
        counter_reason = Some("入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string());
    } else if ratio_a >= 1.45 && b_evades_heavy_a {
        score_b += 0.75;
        counter_reason = Some("入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string());
    } else if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        // 2. 同量級拆招（「玉女劍法專克全真劍法：以巧破剛」）
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;

        // A 克制 B 判定
        if (v_b.shock + v_b.sustain >= 0.9) && (v_a.penetration + v_a.flow >= 0.9) && (v_a.tempo_early >= v_b.tempo_early - 0.1) {
            score_a += 0.60;
            counter_reason = Some("以巧破剛：身形微側，鋒芒恰恰切入對方換氣收勢的空隙".to_string());
        } else if (v_b.flow >= 0.9 && v_b.shock < 0.6) && (v_a.shock + v_a.reach >= 1.1) {
            score_a += 0.50;
            counter_reason = Some("以重破巧：大開大闔，以沉渾重勁強行封死對方騰挪退路".to_string());
        } else if (v_b.tempo_late >= 0.8 && v_b.qi_cost >= 0.7) && (v_a.tempo_early + v_a.focus >= 1.0) {
            score_a += 0.50;
            counter_reason = Some("以守待竭：在對方後勁尚未蓄滿之際搶先欺身，半途截斷其氣機".to_string());
        } else if (v_a.shock + v_a.sustain >= 0.9) && (v_b.penetration + v_b.flow >= 0.9) && (v_b.tempo_early >= v_a.tempo_early - 0.1) {
            // B 克制 A 判定
            score_b += 0.60;
            counter_reason = Some("以巧破剛：身形微側，鋒芒恰恰切入對方換氣收勢的空隙".to_string());
        } else if (v_a.flow >= 0.9 && v_a.shock < 0.6) && (v_b.shock + v_b.reach >= 1.1) {
            score_b += 0.50;
            counter_reason = Some("以重破巧：大開大闔，以沉渾重勁強行封死對方騰挪退路".to_string());
        } else if (v_a.tempo_late >= 0.8 && v_a.qi_cost >= 0.7) && (v_b.tempo_early + v_b.focus >= 1.0) {
            score_b += 0.50;
            counter_reason = Some("以守待竭：在對方後勁尚未蓄滿之際搶先欺身，半途截斷其氣機".to_string());
        }
    }

    let a_carrier = tech_a
        .map(|t| t.primary_carrier)
        .unwrap_or(CarrierAffinity::Unarmed);
    let b_carrier = tech_b
        .map(|t| t.primary_carrier)
        .unwrap_or(CarrierAffinity::Unarmed);
    let a_target_part = tech_a
        .map(|t| t.primary_target)
        .unwrap_or(TargetPart::Torso);
    let b_target_part = tech_b
        .map(|t| t.primary_target)
        .unwrap_or(TargetPart::Torso);
    let a_motions = tech_a
        .map(|t| t.derive_elementary_motions_for_carrier(None))
        .unwrap_or_else(|| {
            derive_elementary_motions_from_chars(&[], a_carrier, a_target_part)
        });
    let b_motions = tech_b
        .map(|t| t.derive_elementary_motions_for_carrier(None))
        .unwrap_or_else(|| {
            derive_elementary_motions_from_chars(&[], b_carrier, b_target_part)
        });
    let seed_a = tech_a
        .and_then(|t| t.soul_seed)
        .unwrap_or_else(|| {
            let key = tech_a
                .map(|t| format!("{}:{slot:?}", t.effective_name))
                .unwrap_or_else(|| format!("neutral_empty_slot:{slot:?}"));
            crate::combat::human_phase::resolve_entity_soul_seed(&key, None)
        });
    let seed_b = tech_b
        .and_then(|t| t.soul_seed)
        .unwrap_or_else(|| {
            let key = tech_b
                .map(|t| format!("{}:{slot:?}", t.effective_name))
                .unwrap_or_else(|| format!("neutral_empty_slot:{slot:?}"));
            crate::combat::human_phase::resolve_entity_soul_seed(&key, None)
        });
    let bone_a = crate::combat::human_phase::InnateBonePhase::from_seed(seed_a);
    let bone_b = crate::combat::human_phase::InnateBonePhase::from_seed(seed_b);

    let practice_a = tech_a.map(|t| t.practice_count).unwrap_or(25);
    let practice_b = tech_b.map(|t| t.practice_count).unwrap_or(25);
    let anatomy_a = tech_a.map(|t| t.anatomy_familiarity).unwrap_or(20);
    let anatomy_b = tech_b.map(|t| t.anatomy_familiarity).unwrap_or(20);
    let learned_a = tech_a.map(|t| t.learned_delta).unwrap_or(0.0);
    let learned_b = tech_b.map(|t| t.learned_delta).unwrap_or(0.0);

    let three_phase_res = crate::combat::human_phase::evaluate_three_phase_bout_ext(
        slot,
        &bone_a,
        &bone_b,
        weight_a,
        weight_b,
        a_carrier,
        b_carrier,
        a_prior_posture,
        b_prior_posture,
        a_motions.len(),
        b_motions.len(),
        _phase,
        practice_a,
        practice_b,
        anatomy_a,
        anatomy_b,
        learned_a,
        learned_b,
        if a_flawed_prior { 0.25 } else { 0.0 },
        if b_flawed_prior { 0.25 } else { 0.0 },
    );

    if counter_reason.is_none() {
        score_a += three_phase_res.score_delta_a;
        score_b += three_phase_res.score_delta_b;
        if let Some(cr) = &three_phase_res.counter_reason {
            counter_reason = Some(cr.clone());
        }
    }
    if let Some(cr) = counter_reason.as_deref() {
        if cr.starts_with("重勢硬壓") || cr.starts_with("重勢硬撼") {
            if (score_a > score_b && weight_a < weight_b)
                || (score_b > score_a && weight_b < weight_a)
            {
                counter_reason = Some(
                    "入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string(),
                );
            }
        }
    }

    let mut notes = Vec::new();
    if let Some(r) = counter_reason.as_deref() {
        notes.push(r.to_string());
    }
    if let Some(res_int) = a_residue_interaction {
        notes.push(format!("甲方{}", res_int.label()));
    }
    if let Some(res_int) = b_residue_interaction {
        notes.push(format!("乙方{}", res_int.label()));
    }
    let counter_note = if notes.is_empty() {
        String::new()
    } else {
        format!("（{}）", notes.join("；"))
    };

    let is_draw = (score_a - score_b).abs() < 1e-6;

    if is_draw {
        let a_completed_motions = if !three_phase_res.a_manifested_as_skill && three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange {
            three_phase_res.a_completed_motions.clamp(1, a_motions.len().max(1))
        } else {
            a_motions.len().max(1)
        };
        let b_completed_motions = if !three_phase_res.b_manifested_as_skill && three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange {
            three_phase_res.b_completed_motions.clamp(1, b_motions.len().max(1))
        } else {
            b_motions.len().max(1)
        };
        let a_chain_str = a_motions.join("、");
        let b_chain_str = b_motions.join("、");
        let narr = format!(
            "雙方同時出招，甲方之「{name_a}」（{a_chain_str}）與乙方之「{name_b}」（{b_chain_str}）於半空激烈碰撞！氣勁互相抵消激盪，雙方各自震退，勢均力敵，難分高下！{counter_note}"
        );
        let a_post = if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_prior_posture.degrade()
        } else {
            a_prior_posture
        };
        let b_post = if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_prior_posture.degrade()
        } else {
            b_prior_posture
        };
        let residue_left_after = tech_a
            .and_then(|t| t.emitted_residue())
            .or_else(|| tech_b.and_then(|t| t.emitted_residue()));

        BoutClashOutcome {
            winner_side: "draw".to_string(),
            a_manifested_as_skill: false,
            b_manifested_as_skill: false,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: 0,
            damage_to_b: 0,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    } else if score_a > score_b {
        let base_dmg = match slot {
            TacticalSlot::Opening => (score_a - score_b) * 6.0 + pen_a * 8.0 + 3.0,
            TacticalSlot::Midgame => (score_a - score_b) * 7.0 + pen_a * 10.0 + 4.0,
            TacticalSlot::Finisher => {
                let flaw_mult = if b_flawed_prior { 1.5 } else { 1.0 };
                ((score_a - score_b) * 9.0 + shock_a * 12.0 + 6.0) * flaw_mult
            }
            TacticalSlot::Defense => (score_a - score_b) * 5.0 + 2.0,
        };
        let final_dmg = (base_dmg - def_mit_b).max(2.0).round() as i32;
        let a_completed_motions = a_motions.len().max(1);
        let b_completed_motions = if three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange && !three_phase_res.b_manifested_as_skill {
            three_phase_res.b_completed_motions.clamp(1, b_motions.len().max(1))
        } else if score_a - score_b >= 0.8 {
            1
        } else {
            2.min(b_motions.len().saturating_sub(1).max(1))
        };
        let a_chain_str = a_motions.join("、");
        let b_frag_str = b_motions
            .iter()
            .take(b_completed_motions)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let target_label = a_target_part.label();

        let narr = match slot {
            TacticalSlot::Opening => format!(
                "甲方將「{a_chain_str}」順暢連貫，起承轉合一氣呵成，顯化成招為技能「{name_a}」！乙方甫自起手做出「{b_frag_str}」，便在半途被截斷氣機，套路當場潰散為零散碎招，{target_label}受創！{counter_note}"
            ),
            TacticalSlot::Midgame => format!(
                "甲方乘勝將「{a_chain_str}」融會貫通，成招顯化為技能「{name_a}」！乙方勉強遞出「{b_frag_str}」苦撐，氣息浮動間被硬生截斷，只餘零星碎招便被破開防線，{target_label}受震！{counter_note}"
            ),
            TacticalSlot::Finisher => format!(
                "甲方周身勁力合一，「{a_chain_str}」連環貫徹，悍然顯化為絕殺技能「{name_a}」！乙方意圖運轉「{b_frag_str}」抵禦，終因招式受阻散為零碎招架，{target_label}遭重創失據！{counter_note}"
            ),
            TacticalSlot::Defense => format!(
                "甲方守勢「{a_chain_str}」圓融貫通，顯化為技能「{name_a}」！乙方僅遞出「{b_frag_str}」便散為碎招，未能突破防線，反受震退！{counter_note}"
            ),
        };

        let mut a_post = a_prior_posture;
        if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_post = a_post.degrade();
        }

        let mut b_post = if final_dmg >= 22 || b_prior_posture == PostureState::Shaken || b_prior_posture == PostureState::Broken {
            PostureState::Broken
        } else if final_dmg >= 11 || three_phase_res.b_posture_after == PostureState::Shaken {
            PostureState::Shaken
        } else {
            PostureState::Steady
        };
        if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_post = b_post.degrade();
        }

        let residue_left_after = tech_a.and_then(|t| t.emitted_residue());

        BoutClashOutcome {
            winner_side: "side_a".to_string(),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: 0,
            damage_to_b: final_dmg,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    } else {
        let base_dmg = match slot {
            TacticalSlot::Opening => (score_b - score_a) * 6.0 + pen_b * 8.0 + 3.0,
            TacticalSlot::Midgame => (score_b - score_a) * 7.0 + pen_b * 10.0 + 4.0,
            TacticalSlot::Finisher => {
                let flaw_mult = if a_flawed_prior { 1.5 } else { 1.0 };
                ((score_b - score_a) * 9.0 + shock_b * 12.0 + 6.0) * flaw_mult
            }
            TacticalSlot::Defense => (score_b - score_a) * 5.0 + 2.0,
        };
        let final_dmg = (base_dmg - def_mit_a).max(2.0).round() as i32;
        let b_completed_motions = b_motions.len().max(1);
        let a_completed_motions = if three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange && !three_phase_res.a_manifested_as_skill {
            three_phase_res.a_completed_motions.clamp(1, a_motions.len().max(1))
        } else if score_b - score_a >= 0.8 {
            1
        } else {
            2.min(a_motions.len().saturating_sub(1).max(1))
        };
        let b_chain_str = b_motions.join("、");
        let a_frag_str = a_motions
            .iter()
            .take(a_completed_motions)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let target_label = b_target_part.label();

        let narr = match slot {
            TacticalSlot::Opening => format!(
                "乙方神速搶制，「{b_chain_str}」貫徹到底，顯化成招為技能「{name_b}」！甲方甫自做出「{a_frag_str}」，便在半途遭強勢截斷，套路當場潰散為零散碎招，{target_label}受創！{counter_note}"
            ),
            TacticalSlot::Midgame => format!(
                "乙方「{b_chain_str}」起承轉合無懈可擊，成招顯化為技能「{name_b}」！甲方遞出「{a_frag_str}」後攻角遭全面壓制散為碎招，退避不及致{target_label}受震！{counter_note}"
            ),
            TacticalSlot::Finisher => format!(
                "乙方後勁爆發，「{b_chain_str}」一氣呵成顯化為技能「{name_b}」！甲方僅做出「{a_frag_str}」便被打散為碎招，防線瓦解，{target_label}遭重創！{counter_note}"
            ),
            TacticalSlot::Defense => format!(
                "乙方守勢「{b_chain_str}」化境成招，顯化為技能「{name_b}」！甲方「{a_frag_str}」散為零碎招架，無功而返，反受震退！{counter_note}"
            ),
        };

        let mut b_post = b_prior_posture;
        if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_post = b_post.degrade();
        }

        let mut a_post = if final_dmg >= 22 || a_prior_posture == PostureState::Shaken || a_prior_posture == PostureState::Broken {
            PostureState::Broken
        } else if final_dmg >= 11 || three_phase_res.a_posture_after == PostureState::Shaken {
            PostureState::Shaken
        } else {
            PostureState::Steady
        };
        if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_post = a_post.degrade();
        }

        let residue_left_after = tech_b.and_then(|t| t.emitted_residue());

        BoutClashOutcome {
            winner_side: "side_b".to_string(),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: final_dmg,
            damage_to_b: 0,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    }
}

/// 帶入雙方攻防權重之田忌賽馬時序對決判定
pub fn resolve_tianji_matchup_with_weights(
    side_a: &TacticalLoadout,
    side_a_hp: i32,
    side_b: &TacticalLoadout,
    side_b_hp: i32,
    weight_a: f64,
    weight_b: f64,
    registry: &WordRegistry,
) -> Result<TacticalClashReport, WordPlateError> {
    let mut current_hp_a = side_a_hp.max(1);
    let mut current_hp_b = side_b_hp.max(1);
    let mut rounds = Vec::with_capacity(3);
    let mut posture_a = PostureState::Steady;
    let mut posture_b = PostureState::Steady;
    let mut active_residue = None;

    // 守勢身法預備
    let def_a_tech = side_a.defense.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let def_b_tech = side_b.defense.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let def_a_mit = def_a_tech.as_ref().map(|t| t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2).unwrap_or(0.0);
    let def_b_mit = def_b_tech.as_ref().map(|t| t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2).unwrap_or(0.0);

    // 一、起手式（Opening）交鋒
    let op_a = side_a.opening.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let op_b = side_b.opening.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let bout_1 = resolve_bout_with_state(
        TacticalSlot::Opening,
        1,
        op_a.as_ref(),
        op_b.as_ref(),
        1.0,
        1.0,
        weight_a,
        weight_b,
        def_a_mit,
        def_b_mit,
        false,
        false,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_1.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_1.damage_to_b).max(0);
    posture_a = bout_1.a_posture_after;
    posture_b = bout_1.b_posture_after;
    active_residue = bout_1.residue_left_after;

    let (momentum_a, momentum_b) = if bout_1.winner_side == "side_a" {
        (1.2, 0.85)
    } else if bout_1.winner_side == "side_b" {
        (0.85, 1.2)
    } else {
        (1.0, 1.0)
    };

    rounds.push(PhaseClashRecord {
        phase: 1,
        slot: TacticalSlot::Opening,
        a_technique_name: bout_1.a_technique_name,
        b_technique_name: bout_1.b_technique_name,
        a_initiative_score: bout_1.a_initiative_score,
        b_initiative_score: bout_1.b_initiative_score,
        damage_to_a: bout_1.damage_to_a,
        damage_to_b: bout_1.damage_to_b,
        narrative: bout_1.narrative,
        a_manifested_as_skill: bout_1.a_manifested_as_skill,
        b_manifested_as_skill: bout_1.b_manifested_as_skill,
        counter_reason: bout_1.counter_reason,
        a_motions: bout_1.a_motions,
        b_motions: bout_1.b_motions,
        a_completed_motions: bout_1.a_completed_motions,
        b_completed_motions: bout_1.b_completed_motions,
        a_target_part: bout_1.a_target_part,
        b_target_part: bout_1.b_target_part,
        a_carrier: bout_1.a_carrier,
        b_carrier: bout_1.b_carrier,
        a_posture_after: bout_1.a_posture_after,
        b_posture_after: bout_1.b_posture_after,
        active_residue_before: bout_1.active_residue_before,
        residue_left_after: bout_1.residue_left_after,
        a_residue_interaction: bout_1.a_residue_interaction,
        b_residue_interaction: bout_1.b_residue_interaction,
        a_resonance_tier: bout_1.a_resonance_tier,
        b_resonance_tier: bout_1.b_resonance_tier,
    });

    // 二、常態連攜（Midgame）交鋒
    let mid_a = side_a.midgame.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let mid_b = side_b.midgame.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let a_flawed = (bout_1.winner_side == "side_b" && (bout_1.b_initiative_score - bout_1.a_initiative_score > 0.4)) || bout_1.a_posture_after == PostureState::Broken;
    let b_flawed = (bout_1.winner_side == "side_a" && (bout_1.a_initiative_score - bout_1.b_initiative_score > 0.4)) || bout_1.b_posture_after == PostureState::Broken;

    let bout_2 = resolve_bout_with_state(
        TacticalSlot::Midgame,
        2,
        mid_a.as_ref(),
        mid_b.as_ref(),
        momentum_a,
        momentum_b,
        weight_a,
        weight_b,
        def_a_mit * 0.7,
        def_b_mit * 0.7,
        a_flawed,
        b_flawed,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_2.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_2.damage_to_b).max(0);
    posture_a = bout_2.a_posture_after;
    posture_b = bout_2.b_posture_after;
    active_residue = bout_2.residue_left_after;

    let (momentum_a_fin, momentum_b_fin) = if bout_2.winner_side == "side_a" {
        (1.2, 0.85)
    } else if bout_2.winner_side == "side_b" {
        (0.85, 1.2)
    } else {
        (1.0, 1.0)
    };

    rounds.push(PhaseClashRecord {
        phase: 2,
        slot: TacticalSlot::Midgame,
        a_technique_name: bout_2.a_technique_name,
        b_technique_name: bout_2.b_technique_name,
        a_initiative_score: bout_2.a_initiative_score,
        b_initiative_score: bout_2.b_initiative_score,
        damage_to_a: bout_2.damage_to_a,
        damage_to_b: bout_2.damage_to_b,
        narrative: bout_2.narrative,
        a_manifested_as_skill: bout_2.a_manifested_as_skill,
        b_manifested_as_skill: bout_2.b_manifested_as_skill,
        counter_reason: bout_2.counter_reason,
        a_motions: bout_2.a_motions,
        b_motions: bout_2.b_motions,
        a_completed_motions: bout_2.a_completed_motions,
        b_completed_motions: bout_2.b_completed_motions,
        a_target_part: bout_2.a_target_part,
        b_target_part: bout_2.b_target_part,
        a_carrier: bout_2.a_carrier,
        b_carrier: bout_2.b_carrier,
        a_posture_after: bout_2.a_posture_after,
        b_posture_after: bout_2.b_posture_after,
        active_residue_before: bout_2.active_residue_before,
        residue_left_after: bout_2.residue_left_after,
        a_residue_interaction: bout_2.a_residue_interaction,
        b_residue_interaction: bout_2.b_residue_interaction,
        a_resonance_tier: bout_2.a_resonance_tier,
        b_resonance_tier: bout_2.b_resonance_tier,
    });

    // 三、破綻殺招（Finisher）交鋒
    let fin_a = side_a.finisher.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let fin_b = side_b.finisher.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let a_flawed_fin = (bout_2.winner_side == "side_b" && (bout_2.b_initiative_score - bout_2.a_initiative_score > 0.5)) || bout_2.a_posture_after == PostureState::Broken;
    let b_flawed_fin = (bout_2.winner_side == "side_a" && (bout_2.a_initiative_score - bout_2.b_initiative_score > 0.5)) || bout_2.b_posture_after == PostureState::Broken;

    let bout_3 = resolve_bout_with_state(
        TacticalSlot::Finisher,
        3,
        fin_a.as_ref(),
        fin_b.as_ref(),
        momentum_a_fin,
        momentum_b_fin,
        weight_a,
        weight_b,
        def_a_mit * 0.5,
        def_b_mit * 0.5,
        a_flawed_fin,
        b_flawed_fin,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_3.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_3.damage_to_b).max(0);

    rounds.push(PhaseClashRecord {
        phase: 3,
        slot: TacticalSlot::Finisher,
        a_technique_name: bout_3.a_technique_name,
        b_technique_name: bout_3.b_technique_name,
        a_initiative_score: bout_3.a_initiative_score,
        b_initiative_score: bout_3.b_initiative_score,
        damage_to_a: bout_3.damage_to_a,
        damage_to_b: bout_3.damage_to_b,
        narrative: bout_3.narrative,
        a_manifested_as_skill: bout_3.a_manifested_as_skill,
        b_manifested_as_skill: bout_3.b_manifested_as_skill,
        counter_reason: bout_3.counter_reason,
        a_motions: bout_3.a_motions,
        b_motions: bout_3.b_motions,
        a_completed_motions: bout_3.a_completed_motions,
        b_completed_motions: bout_3.b_completed_motions,
        a_target_part: bout_3.a_target_part,
        b_target_part: bout_3.b_target_part,
        a_carrier: bout_3.a_carrier,
        b_carrier: bout_3.b_carrier,
        a_posture_after: bout_3.a_posture_after,
        b_posture_after: bout_3.b_posture_after,
        active_residue_before: bout_3.active_residue_before,
        residue_left_after: bout_3.residue_left_after,
        a_residue_interaction: bout_3.a_residue_interaction,
        b_residue_interaction: bout_3.b_residue_interaction,
        a_resonance_tier: bout_3.a_resonance_tier,
        b_resonance_tier: bout_3.b_resonance_tier,
    });

    let winner = if current_hp_a > current_hp_b {
        "side_a".to_string()
    } else if current_hp_b > current_hp_a {
        "side_b".to_string()
    } else {
        "draw".to_string()
    };

    let summary = match winner.as_str() {
        "side_a" => "數合交手告終，甲方憑藉精妙路數掌握節奏，氣機悠長沉穩，贏得此役！".to_string(),
        "side_b" => "數合交手告終，乙方後發制人底蘊深厚，守中帶攻，奪下勝局！".to_string(),
        _ => "數合交手勢均力敵，雙方身形氣機並駕齊驅，各自收勢！".to_string(),
    };

    Ok(TacticalClashReport {
        winner,
        rounds,
        side_a_final_hp: current_hp_a,
        side_b_final_hp: current_hp_b,
        summary,
    })
}

/// 執行田忌賽馬時序對決判定（保證零自殺判定、傷害單向承載與受控反衝）
pub fn resolve_tianji_matchup(
    side_a: &TacticalLoadout,
    side_a_hp: i32,
    side_b: &TacticalLoadout,
    side_b_hp: i32,
    registry: &WordRegistry,
) -> Result<TacticalClashReport, WordPlateError> {
    resolve_tianji_matchup_with_weights(
        side_a,
        side_a_hp,
        side_b,
        side_b_hp,
        1.0,
        1.0,
        registry,
    )
}


// ==========================================
// 模組四：事件分派之三相動力學判準
// ==========================================

/// 載體預備時間基底
fn carrier_pre_base(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Footwork => 1.0,
        CarrierAffinity::Blade | CarrierAffinity::Unarmed => 1.5,
        CarrierAffinity::Pole => 2.0,
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 2.5,
    }
}

/// 載體肢段幾何係數
pub fn carrier_limb_factor(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Unarmed | CarrierAffinity::Footwork => 0.85,
        CarrierAffinity::Blade => 1.0,
        CarrierAffinity::Pole => 1.15,
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.30,
    }
}

/// 載體基準器物配重（公斤）
pub fn carrier_base_tool_mass(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Blunt => 14.0,
        CarrierAffinity::Shield => 10.0,
        CarrierAffinity::Pole => 5.0,
        CarrierAffinity::Blade => 3.5,
        CarrierAffinity::Unarmed | CarrierAffinity::Footwork => 0.0,
    }
}

/// 載體截擊係數 carrier_int
pub fn carrier_intercept_factor(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Footwork => 1.30,
        CarrierAffinity::Blade => 1.15,
        CarrierAffinity::Unarmed => 1.05,
        CarrierAffinity::Pole => 0.95,
        CarrierAffinity::Shield => 0.85,
        CarrierAffinity::Blunt => 0.75,
    }
}

/// 人獸對稱有效質量 M_eff：肢段有效質量與載體基準配重
pub fn effective_carrier_mass(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    _external_weight: f64,
) -> f64 {
    let m_limb = (0.12 * bone.vit_0 + 0.06 * bone.qi_0)
        * bone.alpha.sqrt()
        * carrier_limb_factor(carrier);
    let m_tool = carrier_base_tool_mass(carrier);
    m_limb + m_tool
}

/// 連續浮點時序計算：預備時間 tau_pre 與換招空檔破綻窗 tau_gap
pub fn compute_continuous_timings(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    single_move_depth_m: f64,
    continuity_c: f64,
    scar_severity: f64,
) -> (f64, f64) {
    let pre_base = carrier_pre_base(carrier);
    let tau_pre = 1.0 + ((pre_base - 0.6).max(0.2))
        / (bone.beta.sqrt() * (1.0 + 0.35 * single_move_depth_m.max(0.0)));

    let tau_gap = (1.0
        + (pre_base * 1.40)
            / (bone.beta.sqrt()
                * (1.0 + 0.15 * bone.gamma.max(0.0))
                * (1.0 + 0.22 * continuity_c.max(0.0))))
        * (1.0 + 0.50 * scar_severity.max(0.0));

    (tau_pre, tau_gap)
}

/// 多局識破適應因子 R_read in [0.0, 0.75]
pub fn read_adaptation_factor(bout_index: usize, defender_v: f64, attacker_v: f64) -> f64 {
    (0.22
        * (bout_index as f64)
        * defender_v.clamp(0.1, 1.0)
        * (1.15 - 0.65 * attacker_v.clamp(0.1, 1.0)))
    .clamp(0.0, 0.75)
}

/// 第三相入微穿隙（庖丁解牛）評估：支援無上限熟練練習深度
pub fn evaluate_seam_threading_with_depth(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    edex: f64,
    target_tightness_kappa: f64,
    anatomy_familiarity: u32,
    single_move_depth_m: f64,
) -> SeamThreadingEval {
    let p_tool = match carrier {
        CarrierAffinity::Blade => 1.80,
        CarrierAffinity::Pole => 1.40,
        CarrierAffinity::Unarmed => 1.00,
        _ => 0.50,
    };

    let dex_beta = edex * bone.beta;
    let match_term = dex_beta / (dex_beta + target_tightness_kappa.max(1.0));

    let fam_term = 1.0
        + 0.25 * ((anatomy_familiarity as f64) / 20.0).powf(P_PRACTICE)
        + 0.12 * single_move_depth_m.max(0.0);

    let raw_xi = p_tool * match_term * fam_term;
    let xi_index = raw_xi.clamp(0.0, 2.30);

    let hardness_bypass = 1.0 - (-xi_index).exp();
    let wear_multiplier = 0.05f64.max((-xi_index).exp());

    SeamThreadingEval {
        xi_index,
        hardness_bypass,
        wear_multiplier,
    }
}

/// 第三相入微穿隙（庖丁解牛）評估（相容原簽名）
pub fn evaluate_seam_threading(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    edex: f64,
    target_seam_tightness: f64,
    anatomy_familiarity: u32,
) -> SeamThreadingEval {
    evaluate_seam_threading_with_depth(
        bone,
        carrier,
        edex,
        target_seam_tightness,
        anatomy_familiarity,
        0.0,
    )
}

/// 事件分派之三相交鋒評定（擴充版：支援多局識破、練習深度與傷殘係數）
pub fn evaluate_three_phase_bout_ext(
    slot: TacticalSlot,
    bone_a: &InnateBonePhase,
    bone_b: &InnateBonePhase,
    weight_a: f64,
    weight_b: f64,
    carrier_a: CarrierAffinity,
    carrier_b: CarrierAffinity,
    prior_posture_a: PostureState,
    prior_posture_b: PostureState,
    a_motions_len: usize,
    b_motions_len: usize,
    bout_index: usize,
    practice_a: u32,
    practice_b: u32,
    anatomy_a: u32,
    anatomy_b: u32,
    learned_a: f64,
    learned_b: f64,
    scar_a: f64,
    scar_b: f64,
) -> ThreePhaseEvalResult {
    let learned_delta_a = learned_a.clamp(0.0, 1.0);
    let learned_delta_b = learned_b.clamp(0.0, 1.0);
    let (cont_a, var_v_a) = compute_viterbi_continuity_and_variation(
        bone_a,
        slot,
        carrier_a,
        learned_delta_a,
        practice_a,
    );
    let (cont_b, var_v_b) = compute_viterbi_continuity_and_variation(
        bone_b,
        slot,
        carrier_b,
        learned_delta_b,
        practice_b,
    );
    let depth_a = practice_depth(practice_a as f64);
    let depth_b = practice_depth(practice_b as f64);
    let (tau_pre_a, tau_gap_a) =
        compute_continuous_timings(bone_a, carrier_a, depth_a, cont_a, scar_a);
    let (tau_pre_b, tau_gap_b) =
        compute_continuous_timings(bone_b, carrier_b, depth_b, cont_b, scar_b);

    // 多局識破適應：以真實轉位變化性 (var_v_b, var_v_a) 相互推演，收窄破綻窗
    let r_read_a = read_adaptation_factor(bout_index, var_v_a, var_v_b);
    let r_read_b = read_adaptation_factor(bout_index, var_v_b, var_v_a);

    let tau_gap_eff_a = 1.0 + (tau_gap_a - 1.0).max(0.0) * (1.0 - r_read_a);
    let tau_gap_eff_b = 1.0 + (tau_gap_b - 1.0).max(0.0) * (1.0 - r_read_b);

    // 計算角度對齊因子 Omega in [0.4, 1.35]
    let omega_factor = match slot {
        TacticalSlot::Opening => 1.25,
        TacticalSlot::Midgame => 1.0,
        TacticalSlot::Finisher => 0.85,
        TacticalSlot::Defense => 1.35,
    };

    // 計算雙方有效質量與衝量
    let kappa_a = match carrier_a {
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
        CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
        _ => 0.8,
    };
    let kappa_b = match carrier_b {
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
        CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
        _ => 0.8,
    };

    let weight_scale_a = if weight_a > 0.0 && weight_b > 0.0 {
        (weight_a / 50.0).clamp(0.35, 2.5)
    } else {
        1.0
    };
    let weight_scale_b = if weight_a > 0.0 && weight_b > 0.0 {
        (weight_b / 50.0).clamp(0.35, 2.5)
    } else {
        1.0
    };

    let m_eff_a = effective_carrier_mass(bone_a, carrier_a, weight_a) * weight_scale_a;
    let m_eff_b = effective_carrier_mass(bone_b, carrier_b, weight_b) * weight_scale_b;

    let i_a = kappa_a * m_eff_a.max(4.0) * bone_a.vit_0 * bone_a.impact_factor();
    let i_b = kappa_b * m_eff_b.max(4.0) * bone_b.vit_0 * bone_b.impact_factor();

    let post_mult_a = match prior_posture_a {
        PostureState::Steady => 1.0,
        PostureState::Shaken => 0.7,
        PostureState::Broken => 0.3,
    };
    let post_mult_b = match prior_posture_b {
        PostureState::Steady => 1.0,
        PostureState::Shaken => 0.7,
        PostureState::Broken => 0.3,
    };

    let s_dex_a = bone_a.cell_smoothness(2, 2);
    let s_dex_b = bone_b.cell_smoothness(2, 2);

    let i_hold_a = post_mult_a * m_eff_a.max(4.0) * bone_a.vit_0 * (1.0 + 0.35 * (-bone_a.gamma).max(0.0) * s_dex_a);
    let i_hold_b = post_mult_b * m_eff_b.max(4.0) * bone_b.vit_0 * (1.0 + 0.35 * (-bone_b.gamma).max(0.0) * s_dex_b);

    // 1. 第一相【一力降十會（正面硬撼）】：Lambda >= 1.5 且實際量級高於對手至少 15%
    let lambda_a = i_a / i_hold_b.max(1.0);
    let lambda_b = i_b / i_hold_a.max(1.0);

    let a_can_crush_b = lambda_a >= 1.5
        && weight_a >= weight_b * 1.15
        && !(carrier_b == CarrierAffinity::Footwork && slot == TacticalSlot::Defense);
    let b_can_crush_a = lambda_b >= 1.5
        && weight_b >= weight_a * 1.15
        && !(carrier_a == CarrierAffinity::Footwork && slot == TacticalSlot::Defense);
    if a_can_crush_b && (!b_can_crush_a || lambda_a > lambda_b + 1e-6) {
        let bounded_delta = 1.8 * (lambda_a - 1.0).min(1.20);
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase1ImpulseCrush {
                winner_is_a: true,
                lambda_ratio: lambda_a,
            },
            score_delta_a: bounded_delta,
            score_delta_b: -1.2,
            counter_reason: Some("重勢硬撼：千鈞重勢硬撼而下，沛然巨力震散敵方招式架勢！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_completed_motions: a_motions_len.max(3),
            b_completed_motions: 1,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if b_can_crush_a && (!a_can_crush_b || lambda_b > lambda_a + 1e-6) {
        let bounded_delta = 1.8 * (lambda_b - 1.0).min(1.20);
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase1ImpulseCrush {
                winner_is_a: false,
                lambda_ratio: lambda_b,
            },
            score_delta_a: -1.2,
            score_delta_b: bounded_delta,
            counter_reason: Some("重勢硬撼：千鈞重勢硬撼而下，沛然巨力震散敵方招式架勢！".to_string()),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_completed_motions: 1,
            b_completed_motions: b_motions_len.max(3),
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 2. 第二相【專克拆招（玉女克全真）】：連續相對餘裕截擊
    let margin_ab = ((tau_gap_eff_b - tau_pre_a) / tau_gap_eff_b.max(1e-6)).max(0.0);
    let margin_ba = ((tau_gap_eff_a - tau_pre_b) / tau_gap_eff_a.max(1e-6)).max(0.0);

    let int_a = if i_a / i_hold_b.max(1.0) >= 0.55 && margin_ab > 0.0 {
        margin_ab
            * carrier_intercept_factor(carrier_a)
            * (1.0 + 0.20 * bone_a.gamma.max(0.0))
            * (1.0 + 0.15 * cont_a)
            * omega_factor
    } else {
        -1.0
    };

    let int_b = if i_b / i_hold_a.max(1.0) >= 0.55 && margin_ba > 0.0 {
        margin_ba
            * carrier_intercept_factor(carrier_b)
            * (1.0 + 0.20 * bone_b.gamma.max(0.0))
            * (1.0 + 0.15 * cont_b)
            * omega_factor
    } else {
        -1.0
    };

    if int_a >= 0.20 && int_a > int_b + 0.05 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: true,
                intercept_score: int_a,
                gap_ticks: tau_gap_eff_b,
            },
            score_delta_a: 1.2,
            score_delta_b: -0.8,
            counter_reason: Some("借勢拆招：身形微側切入換招死角，當場截斷其後續連段！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_completed_motions: a_motions_len,
            b_completed_motions: 2,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if int_b >= 0.20 && int_b > int_a + 0.05 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: false,
                intercept_score: int_b,
                gap_ticks: tau_gap_eff_a,
            },
            score_delta_a: -0.8,
            score_delta_b: 1.2,
            counter_reason: Some("借勢拆招：身形微側切入換招死角，當場截斷其後續連段！".to_string()),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_completed_motions: 2,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 3. 第三相【入微穿隙（庖丁解牛）】：依架勢與時序差計算動態緊密度
    let posture_tightness = |p: PostureState| -> f64 {
        match p {
            PostureState::Steady => 100.0,
            PostureState::Shaken => 70.0,
            PostureState::Broken => 40.0,
        }
    };

    let kappa_target_b = posture_tightness(prior_posture_b)
        / (1.0 + 0.30 * (tau_gap_eff_b - tau_pre_a).max(0.0));
    let kappa_target_a = posture_tightness(prior_posture_a)
        / (1.0 + 0.30 * (tau_gap_eff_a - tau_pre_b).max(0.0));

    let seam_a = evaluate_seam_threading_with_depth(
        bone_a,
        carrier_a,
        bone_a.dex_0,
        kappa_target_b,
        anatomy_a,
        depth_a,
    );
    let seam_b = evaluate_seam_threading_with_depth(
        bone_b,
        carrier_b,
        bone_b.dex_0,
        kappa_target_a,
        anatomy_b,
        depth_b,
    );

    if seam_a.xi_index >= 1.50 && seam_a.xi_index > seam_b.xi_index + 0.30 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: true,
                xi_index: seam_a.xi_index,
                bypass_pct: seam_a.hardness_bypass * 100.0,
            },
            score_delta_a: 0.85 * seam_a.xi_index,
            score_delta_b: -0.5,
            counter_reason: Some("尋隙游刃：游刃順應骨縫筋節穿入，避實擊虛直透要害！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: true,
            a_completed_motions: a_motions_len,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if seam_b.xi_index >= 1.50 && seam_b.xi_index > seam_a.xi_index + 0.30 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: false,
                xi_index: seam_b.xi_index,
                bypass_pct: seam_b.hardness_bypass * 100.0,
            },
            score_delta_a: -0.5,
            score_delta_b: 0.85 * seam_b.xi_index,
            counter_reason: Some("尋隙游刃：游刃順應骨縫筋節穿入，避實擊虛直透要害！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: true,
            a_completed_motions: 2,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 均勢交鋒
    ThreePhaseEvalResult {
        verdict: ThreePhaseVerdict::StandardExchange,
        score_delta_a: 0.0,
        score_delta_b: 0.0,
        counter_reason: None,
        a_manifested_as_skill: true,
        b_manifested_as_skill: true,
        a_completed_motions: a_motions_len,
        b_completed_motions: b_motions_len,
        a_posture_after: prior_posture_a,
        b_posture_after: prior_posture_b,
    }
}

/// 事件分派之三相交鋒評定（維持原簽名 100% 相容）
pub fn evaluate_three_phase_bout(
    slot: TacticalSlot,
    bone_a: &InnateBonePhase,
    bone_b: &InnateBonePhase,
    weight_a: f64,
    weight_b: f64,
    carrier_a: CarrierAffinity,
    carrier_b: CarrierAffinity,
    prior_posture_a: PostureState,
    prior_posture_b: PostureState,
    a_motions_len: usize,
    b_motions_len: usize,
) -> ThreePhaseEvalResult {
    evaluate_three_phase_bout_ext(
        slot,
        bone_a,
        bone_b,
        weight_a,
        weight_b,
        carrier_a,
        carrier_b,
        prior_posture_a,
        prior_posture_b,
        a_motions_len,
        b_motions_len,
        0,
        25,
        25,
        20,
        20,
        0.5,
        0.5,
        0.0,
        0.0,
    )
}

// 模組五：2-Step Residual OMP 殘差正交匹配定名
// ==========================================

const CLASSICAL_PARTICLES: &[&str] = &[
    "夫", "蓋", "焉", "哉", "乎", "也", "矣", "兮", "耳", "爾", "歟", "耶", "云", "之", "其", "於", "以", "而", "則", "乃", "且", "若", "所", "者"
];

fn dot_product_16(a: &[f64; 16], b: &[f64; 16]) -> f64 {
    let mut sum = 0.0;
    for i in 0..16 {
        sum += a[i] * b[i];
    }
    sum
}

fn norm_l2_16(a: &[f64; 16]) -> f64 {
    let d = dot_product_16(a, a);
    if d <= 0.0 {
        0.0
    } else {
        d.sqrt()
    }
}

/// 2 步殘差正交匹配追蹤定名
pub fn residual_omp_select_roots(
    feature_16d: &[f64; 16],
    registry: &WordRegistry,
) -> ((String, String), (String, String)) {
    // 1. 正規化目標向量
    let feat_norm = norm_l2_16(feature_16d);
    let mut phi_hat = [0.0f64; 16];
    if feat_norm > 1e-9 {
        for i in 0..16 {
            phi_hat[i] = feature_16d[i] / feat_norm;
        }
    } else {
        phi_hat[0] = 1.0;
    }

    // 2. 候選字根庫：過濾虛字並將 (12D + 4D) 轉為 16D 單位向量
    let mut candidates: Vec<(&WordElement, [f64; 16])> = Vec::with_capacity(registry.len());

    for elem in registry.ordered_elements() {
        if CLASSICAL_PARTICLES.contains(&elem.ch.as_str()) || elem.semantic == "助" {
            continue;
        }
        let mut v16 = [0.0f64; 16];
        let arr12 = elem.vector_12d.to_array();
        for i in 0..12 {
            v16[i] = arr12[i];
        }
        v16[12] = elem.craft_dims.harvest;
        v16[13] = elem.craft_dims.forge;
        v16[14] = elem.craft_dims.culinary;
        v16[15] = elem.craft_dims.wayfare;

        let n = norm_l2_16(&v16);
        if n > 1e-9 {
            for i in 0..16 {
                v16[i] /= n;
            }
            candidates.push((elem, v16));
        }
    }

    if candidates.is_empty() {
        return (
            ("w_default_1".to_string(), "w_default_2".to_string()),
            ("開".to_string(), "闔".to_string()),
        );
    }

    // 步驟 1：找內積最大之第一字根
    let mut best_score1 = -1.0f64;
    let mut best_idx1 = 0;

    for (idx, (_, u)) in candidates.iter().enumerate() {
        let score = dot_product_16(&phi_hat, u);
        if score > best_score1 {
            best_score1 = score;
            best_idx1 = idx;
        }
    }

    let (elem1, u1) = &candidates[best_idx1];
    let proj1 = dot_product_16(&phi_hat, u1);

    // 步驟 2：計算正交殘差 r1 = phi_hat - <phi_hat, u1> u1
    let mut r1 = [0.0f64; 16];
    for i in 0..16 {
        r1[i] = phi_hat[i] - proj1 * u1[i];
    }
    let r1_norm = norm_l2_16(&r1);
    let mut r1_hat = [0.0f64; 16];
    if r1_norm > 1e-9 {
        for i in 0..16 {
            r1_hat[i] = r1[i] / r1_norm;
        }
    } else {
        // 若殘差極小，以正交擾動維度代入
        r1_hat[(best_idx1 + 1) % 16] = 1.0;
    }

    // 步驟 3：從殘差中挑選第二字根 (w != w1*)
    let mut best_score2 = -1.0f64;
    let mut best_idx2 = 0;

    for (idx, (elem, u)) in candidates.iter().enumerate() {
        if elem.id == elem1.id || elem.ch == elem1.ch {
            continue;
        }
        let score = dot_product_16(&r1_hat, u);
        if score > best_score2 {
            best_score2 = score;
            best_idx2 = idx;
        }
    }

    let (elem2, _) = &candidates[best_idx2];

    (
        (elem1.id.clone(), elem2.id.clone()),
        (elem1.ch.clone(), elem2.ch.clone()),
    )
}

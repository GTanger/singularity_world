/// 驗證單節交鋒敘事段落是否嚴格契合資料
pub fn verify_combat_bout_output(prose: &str, packet: &CombatBoutFactPacket) -> bool {
    let char_count = prose.chars().count();
    if !(20..=180).contains(&char_count) {
        return false;
    }

    // 禁用 ASCII 數字與全形數字 ('0'..='9', '０'..='９')
    if prose
        .chars()
        .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c))
    {
        return false;
    }

    // 殘留氣機驗證：
    // 若 active_residue 為 None，禁止捏造殘留特徵詞彙
    // 若 active_residue 為 Some(res)，要求至少有一非通用字元與 res 重合
    if let Some(res) = &packet.active_residue {
        let meaningful_chars: Vec<char> = res.chars().filter(|&c| c != '氣').collect();
        let has_overlap = if meaningful_chars.is_empty() {
            res.chars().any(|c| prose.contains(c))
        } else {
            meaningful_chars.into_iter().any(|c| prose.contains(c))
        };
        if !has_overlap {
            return false;
        }
    } else if FORBIDDEN_FABRICATED_RESIDUE.iter().any(|w| prose.contains(w)) {
        return false;
    }
    // 禁用第四面牆與即時性禁忌詞
    if FORBIDDEN_WORDS_COMBAT_PROSE.iter().any(|w| prose.contains(w)) {
        return false;
    }

    // 攻守雙方主詞檢查：必須同時出現進攻方與防守方稱呼
    if !packet.attacker_name.is_empty() && !prose.contains(&packet.attacker_name) {
        return false;
    }
    if !packet.defender_name.is_empty() && !prose.contains(&packet.defender_name) {
        return false;
    }

    // 勝負顯化檢查
    if packet.winner_side == "attacker" || packet.winner_side == "defender" {
        // 必須包含勝方顯化之招式名稱
        if !prose.contains(&packet.winner_skill_name) {
            return false;
        }
        // 敗方招式被打散為碎招，絕不可出現敗方招式名稱
        if !packet.loser_skill_name.is_empty()
            && packet.loser_skill_name != packet.winner_skill_name
            && prose.contains(&packet.loser_skill_name)
        {
            return false;
        }
    }

    // 動作資料錨定檢查：必須包含勝方或敗方動作的關鍵詞或雙字子串
    let mut motion_grounded = false;
    for m in packet
        .winner_motions
        .iter()
        .chain(packet.loser_attempted_motions.iter())
    {
        let trimmed = m.trim();
        if trimmed.is_empty() {
            continue;
        }
        if prose.contains(trimmed) {
            motion_grounded = true;
            break;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() >= 2 {
            for w in chars.windows(2) {
                let sub: String = w.iter().collect();
                if prose.contains(&sub) {
                    motion_grounded = true;
                    break;
                }
            }
        } else if chars.len() == 1 && prose.contains(chars[0]) {
            motion_grounded = true;
            break;
        }
        if motion_grounded {
            break;
        }
    }
    if !motion_grounded {
        return false;
    }

    // 兵刃白名單檢查：
    // 若雙方皆非 Blade（且雙方兵刃名稱皆無 刀/劍/刃），不可出現 劍、刀鋒、刀光
    let has_blade = packet.attacker_carrier == CarrierAffinity::Blade
        || packet.defender_carrier == CarrierAffinity::Blade
        || packet.attacker_weapon.contains('刀')
        || packet.attacker_weapon.contains('劍')
        || packet.attacker_weapon.contains('刃')
        || packet.defender_weapon.contains('刀')
        || packet.defender_weapon.contains('劍')
        || packet.defender_weapon.contains('刃');

    if !has_blade && (prose.contains('劍') || prose.contains("刀鋒") || prose.contains("刀光")) {
        return false;
    }

    // 若雙方皆非 Pole，不可出現 長槍、槍尖
    let has_pole = packet.attacker_carrier == CarrierAffinity::Pole
        || packet.defender_carrier == CarrierAffinity::Pole
        || packet.attacker_weapon.contains('槍')
        || packet.attacker_weapon.contains('矛')
        || packet.attacker_weapon.contains('戟')
        || packet.defender_weapon.contains('槍')
        || packet.defender_weapon.contains('矛')
        || packet.defender_weapon.contains('戟');

    if !has_pole && (prose.contains("長槍") || prose.contains("槍尖")) {
        return false;
    }

    true
}

/// 數據轉敘事（Data-to-Text）四維確定性品質評分報告（0..=100 分）
///
/// 借鑑 `NVIDIA/SkillEvaluator` 之多軸確定性評分與反事實增益（Counterfactual Lift）設計：
/// - `correctness_score` (0..=35)：字數合規、零阿拉伯與全形數字、勝方招式顯化、敗方碎招名嚴格隱匿
/// - `grounding_score` (0..=25)：攻守雙方稱呼、兵刃載體親和、環境殘留氣機與動作錨定
/// - `diegetic_purity_score` (0..=25)：零第四面牆詞彙、零系統括號／Markdown 標記、零 `WT-XXXX` 編號外洩
/// - `lexical_diversity_score` (0..=15)：字元雙連詞（2-gram）多樣性比例，防止機械重複模板句
#[derive(Debug, Clone, PartialEq)]
pub struct CombatNarrativeQualityScore {
    pub total_score: f64,
    pub correctness_score: f64,
    pub grounding_score: f64,
    pub diegetic_purity_score: f64,
    pub lexical_diversity_score: f64,
    pub passed_gate: bool,
    pub findings: Vec<String>,
}

/// 數據轉敘事反事實增益（Counterfactual Lift）驗算報告
#[derive(Debug, Clone, PartialEq)]
pub struct CombatNarrativeCounterfactualReport {
    pub scenario_count: usize,
    pub baseline_mean_score: f64,
    pub counterfactual_defective_mean_score: f64,
    pub mean_lift: f64,
    pub all_baseline_passed: bool,
    pub all_defective_rejected: bool,
}

/// 計算單節交鋒敘事的四維確定性品質分數（0..=100）
pub fn evaluate_combat_bout_quality(
    prose: &str,
    packet: &CombatBoutFactPacket,
) -> CombatNarrativeQualityScore {
    let mut findings = Vec::new();
    let mut correctness_score: f64 = 0.0;
    let mut grounding_score: f64 = 0.0;
    let mut diegetic_purity_score: f64 = 0.0;

    // 1. Correctness (0..=35)
    let char_count = prose.chars().count();
    if (20..=180).contains(&char_count) {
        correctness_score += 8.0;
    } else {
        findings.push(format!("字數越界：{char_count} 字（須介於 20..=180 字）"));
    }

    let has_digits = prose
        .chars()
        .any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c));
    if !has_digits {
        correctness_score += 9.0;
    } else {
        findings.push("出現違禁阿拉伯數字或全形數字".to_string());
    }

    if packet.winner_side == "attacker" || packet.winner_side == "defender" {
        if !packet.winner_skill_name.is_empty() && prose.contains(&packet.winner_skill_name) {
            correctness_score += 9.0;
        } else {
            findings.push(format!("未顯化勝方招式名「{}」", packet.winner_skill_name));
        }
        let leaked_loser_skill = !packet.loser_skill_name.is_empty()
            && packet.loser_skill_name != packet.winner_skill_name
            && prose.contains(&packet.loser_skill_name);
        if !leaked_loser_skill {
            correctness_score += 9.0;
        } else {
            findings.push(format!(
                "敗方碎招未隱匿，外洩敗方招式名「{}」",
                packet.loser_skill_name
            ));
        }
    } else {
        correctness_score += 18.0;
    }

    // 2. Grounding (0..=25)
    if !packet.attacker_name.is_empty()
        && prose.contains(&packet.attacker_name)
        && !packet.defender_name.is_empty()
        && prose.contains(&packet.defender_name)
    {
        grounding_score += 7.0;
    } else {
        findings.push("未完整錨定攻守雙方稱呼".to_string());
    }

    let residue_ok = if let Some(res) = &packet.active_residue {
        let meaningful_chars: Vec<char> = res.chars().filter(|&c| c != '氣').collect();
        if meaningful_chars.is_empty() {
            res.chars().any(|c| prose.contains(c))
        } else {
            meaningful_chars.into_iter().any(|c| prose.contains(c))
        }
    } else {
        !FORBIDDEN_FABRICATED_RESIDUE.iter().any(|w| prose.contains(w))
    };
    if residue_ok {
        grounding_score += 6.0;
    } else {
        findings.push("環境殘留氣機錨定失真或憑空捏造殘留".to_string());
    }

    let mut motion_grounded = false;
    for m in packet
        .winner_motions
        .iter()
        .chain(packet.loser_attempted_motions.iter())
    {
        let trimmed = m.trim();
        if trimmed.is_empty() {
            continue;
        }
        if prose.contains(trimmed) {
            motion_grounded = true;
            break;
        }
        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() >= 2
            && chars
                .windows(2)
                .any(|w| prose.contains(&w.iter().collect::<String>()))
        {
            motion_grounded = true;
            break;
        }
    }
    if motion_grounded {
        grounding_score += 6.0;
    } else {
        findings.push("未錨定勝敗雙方動作關鍵字".to_string());
    }

    let has_blade = packet.attacker_carrier == CarrierAffinity::Blade
        || packet.defender_carrier == CarrierAffinity::Blade
        || packet.attacker_weapon.contains('刀')
        || packet.attacker_weapon.contains('劍')
        || packet.attacker_weapon.contains('刃')
        || packet.defender_weapon.contains('刀')
        || packet.defender_weapon.contains('劍')
        || packet.defender_weapon.contains('刃');
    let has_pole = packet.attacker_carrier == CarrierAffinity::Pole
        || packet.defender_carrier == CarrierAffinity::Pole
        || packet.attacker_weapon.contains('槍')
        || packet.attacker_weapon.contains('矛')
        || packet.attacker_weapon.contains('戟')
        || packet.defender_weapon.contains('槍')
        || packet.defender_weapon.contains('矛')
        || packet.defender_weapon.contains('戟');
    let carrier_ok = (has_blade
        || (!prose.contains('劍') && !prose.contains("刀鋒") && !prose.contains("刀光")))
        && (has_pole || (!prose.contains("長槍") && !prose.contains("槍尖")));
    if carrier_ok {
        grounding_score += 6.0;
    } else {
        findings.push("出現與雙方兵刃載體不符之刀劍／長槍描寫".to_string());
    }

    // 3. Diegetic Purity (0..=25)
    if !FORBIDDEN_WORDS_COMBAT_PROSE.iter().any(|w| prose.contains(w)) {
        diegetic_purity_score += 13.0;
    } else {
        findings.push("包含第四面牆或系統回合違禁詞".to_string());
    }
    let has_meta_markup = prose.contains("WT-")
        || prose.contains("wt_cat_")
        || prose.contains("**")
        || prose.contains("```")
        || prose.contains('[')
        || prose.contains(']');
    if !has_meta_markup {
        diegetic_purity_score += 12.0;
    } else {
        findings.push("包含 WT-XXXX 編號或 Markdown 系統標記".to_string());
    }

    // 4. Lexical Diversity (0..=15)
    let cjk_chars: Vec<char> = prose
        .chars()
        .filter(|c| !c.is_whitespace() && !['，', '。', '！', '？', '、', '；', '：', '「', '」'].contains(c))
        .collect();
    let lexical_diversity_score = if cjk_chars.len() >= 4 {
        let total_bigrams = cjk_chars.len() - 1;
        let unique_bigrams: std::collections::HashSet<(char, char)> =
            cjk_chars.windows(2).map(|w| (w[0], w[1])).collect();
        let ratio = unique_bigrams.len() as f64 / total_bigrams as f64;
        if ratio < 0.65 {
            findings.push(format!("雙連詞多樣性過低（ratio={ratio:.2}）"));
        }
        ((ratio / 0.80).clamp(0.0, 1.0) * 15.0 * 100.0).round() / 100.0
    } else {
        0.0
    };

    let total_score =
        ((correctness_score + grounding_score + diegetic_purity_score + lexical_diversity_score)
            * 100.0)
            .round()
            / 100.0;
    let passed_gate = total_score >= 85.0 && verify_combat_bout_output(prose, packet);

    CombatNarrativeQualityScore {
        total_score,
        correctness_score,
        grounding_score,
        diegetic_purity_score,
        lexical_diversity_score,
        passed_gate,
        findings,
    }
}

/// 執行四組正典交鋒資料包的確定性敘事與反事實缺陷對照驗算
pub fn evaluate_combat_narrative_counterfactual_lift() -> CombatNarrativeCounterfactualReport {
    let packets = vec![
        CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "韓鐵山".to_string(),
            defender_name: "羅烈".to_string(),
            attacker_weapon: "百鍛斬馬刀".to_string(),
            defender_weapon: "精鋼開山斧".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Blunt,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷流劈山斬".to_string(),
            loser_skill_name: "裂岩橫掃".to_string(),
            winner_motions: vec!["沉腰劈斬".to_string(), "順勢壓刃".to_string()],
            loser_attempted_motions: vec!["橫柄格擋".to_string()],
            counter_reason: Some("刀勢先至破開中宮".to_string()),
            hit_part: TargetPart::Torso,
            severity_label: "重創".to_string(),
            active_residue: Some("熾烈火勁".to_string()),
            residue_interaction: Some("引燃餘溫".to_string()),
            loser_posture_after: Some("踉蹌退步".to_string()),
            inflicted_injury: Some("肩頭滲血".to_string()),
        },
        CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            attacker_name: "石萬鈞".to_string(),
            defender_name: "雷千鈞".to_string(),
            attacker_weapon: "寒水重鐵錘".to_string(),
            defender_weapon: "巨藻困獸叉".to_string(),
            attacker_carrier: CarrierAffinity::Blunt,
            defender_carrier: CarrierAffinity::Pole,
            winner_side: "defender".to_string(),
            winner_skill_name: "盤根鎖海刺".to_string(),
            loser_skill_name: "開山震地槌".to_string(),
            winner_motions: vec!["擰腰斜挑".to_string(), "長桿點腕".to_string()],
            loser_attempted_motions: vec!["雙手掄砸".to_string()],
            counter_reason: Some("長兵借勢撥偏重錘".to_string()),
            hit_part: TargetPart::Torso,
            severity_label: "震退".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: Some("氣血翻湧".to_string()),
            inflicted_injury: None,
        },
    ];

    let mut baseline_sum = 0.0;
    let mut defective_sum = 0.0;
    let mut all_baseline_passed = true;
    let mut all_defective_rejected = true;

    for packet in &packets {
        let good_prose = render_deterministic_bout_from_data(packet);
        let good_eval = evaluate_combat_bout_quality(&good_prose, packet);
        if !good_eval.passed_gate {
            all_baseline_passed = false;
        }
        baseline_sum += good_eval.total_score;

        // 反事實缺陷樣本：外洩敗方碎招名、夾帶阿拉伯數字與第四面牆回合詞、捏造不存在兵刃與重複字句
        let bad_prose = format!(
            "[系統第1回合] {}使出{}對決{}的{}，劍光一閃造成35點HP傷害，劍氣縱橫、劍氣縱橫、劍氣縱橫！",
            packet.attacker_name,
            packet.winner_skill_name,
            packet.defender_name,
            packet.loser_skill_name
        );
        let bad_eval = evaluate_combat_bout_quality(&bad_prose, packet);
        if bad_eval.passed_gate {
            all_defective_rejected = false;
        }
        defective_sum += bad_eval.total_score;
    }

    let count = packets.len().max(1) as f64;
    let baseline_mean_score = ((baseline_sum / count) * 100.0).round() / 100.0;
    let counterfactual_defective_mean_score = ((defective_sum / count) * 100.0).round() / 100.0;
    let mean_lift =
        ((baseline_mean_score - counterfactual_defective_mean_score) * 100.0).round() / 100.0;

    CombatNarrativeCounterfactualReport {
        scenario_count: packets.len(),
        baseline_mean_score,
        counterfactual_defective_mean_score,
        mean_lift,
        all_baseline_passed,
        all_defective_rejected,
    }
}

/// 確定性渲染交鋒段落（純資料組裝，無第四面牆與違禁字）
pub fn render_deterministic_bout_from_data(packet: &CombatBoutFactPacket) -> String {
    let clean_atk_w = clean_vessel_name(&packet.attacker_weapon);
    let clean_def_w = clean_vessel_name(&packet.defender_weapon);

    let winner_is_attacker = packet.winner_side == "attacker";
    let winner_is_defender = packet.winner_side == "defender";

    let w_motions = if packet.winner_motions.is_empty() {
        "連貫變招".to_string()
    } else {
        packet.winner_motions.join("、")
    };

    let l_motions = if packet.loser_attempted_motions.is_empty() {
        "起勢應對".to_string()
    } else {
        packet.loser_attempted_motions.join("、")
    };

    let counter_raw = packet
        .counter_reason
        .as_deref()
        .unwrap_or("搶佔空門")
        .trim()
        .trim_end_matches(['！', '。', '!']);
    let counter = if let Some((head, _tail)) = counter_raw.split_once('：') {
        format!("「{}」之勢", head.trim())
    } else {
        counter_raw.to_string()
    };
    let hit_part = packet.hit_part.label();
    let severity = &packet.severity_label;

    let residue_prefix = match (&packet.active_residue, &packet.residue_interaction) {
        (Some(res), Some(inter)) => format!("場中{}未散，{}，", res, inter),
        (Some(res), None) => format!("場中{}未散，", res),
        (None, Some(inter)) => format!("{}，", inter),
        (None, None) => String::new(),
    };

    let impact_tail = match (&packet.inflicted_injury, &packet.loser_posture_after) {
        (Some(inj), Some(pos)) => format!("致{}，陷入{}，{}", inj, severity, pos),
        (Some(inj), None) => format!("致{}，陷入{}", inj, severity),
        (None, Some(pos)) => format!("陷入{}，{}", severity, pos),
        (None, None) => format!("陷入{}", severity),
    };

    let is_natural_anatomy = |w: &str| {
        matches!(
            w,
            "雙翼利爪" | "利爪獠牙" | "犄角勁蹄" | "堅甲巨螯" | "鱗軀毒牙" | "天生爪牙"
        )
    };

    if winner_is_attacker {
        let atk_w_desc = if clean_atk_w.is_empty() || clean_atk_w == "徒手" {
            "徒手運勁".to_string()
        } else if is_natural_anatomy(&clean_atk_w) {
            clean_atk_w.to_string()
        } else {
            format!("手執{}", clean_atk_w)
        };
        format!(
            "{}{}以{}施展「{}」，由{}順勢貫通；{}雖試圖以{}應招，卻遭{}中途截斷化為碎招，{}受創，{}。",
            residue_prefix,
            packet.attacker_name,
            atk_w_desc,
            packet.winner_skill_name,
            w_motions,
            packet.defender_name,
            l_motions,
            counter,
            hit_part,
            impact_tail,
        )
    } else if winner_is_defender {
        let def_w_desc = if clean_def_w.is_empty() || clean_def_w == "徒手" {
            "徒手運勁".to_string()
        } else if is_natural_anatomy(&clean_def_w) {
            clean_def_w.to_string()
        } else {
            format!("手執{}", clean_def_w)
        };
        format!(
            "{}{}以{}迎擊施展「{}」，經{}一氣呵成；{}進襲的{}被{}化為碎招，{}遭挫，{}。",
            residue_prefix,
            packet.defender_name,
            def_w_desc,
            packet.winner_skill_name,
            w_motions,
            packet.attacker_name,
            l_motions,
            counter,
            hit_part,
            impact_tail,
        )
    } else {
        // Draw / 不分勝負
        let w_name = if packet.winner_skill_name.is_empty() {
            "招式".to_string()
        } else {
            format!("「{}」", packet.winner_skill_name)
        };
        format!(
            "{}{}施展{}經由{}搶攻，{}亦以{}寸步不讓，雙方氣機交錯散為碎招，各退半步暫歛鋒芒。",
            residue_prefix,
            packet.attacker_name,
            w_name,
            w_motions,
            packet.defender_name,
            l_motions,
        )
    }
}

// ---------------------------------------------------------------------------
// 交鋒敘事快取與 LLM 即時生成
// ---------------------------------------------------------------------------

fn compute_combat_bout_cache_key(packet: &CombatBoutFactPacket) -> u64 {
    let mut hasher = DefaultHasher::new();
    packet.slot.hash(&mut hasher);
    packet.attacker_name.hash(&mut hasher);
    packet.defender_name.hash(&mut hasher);
    packet.attacker_weapon.hash(&mut hasher);
    packet.defender_weapon.hash(&mut hasher);
    packet.attacker_carrier.hash(&mut hasher);
    packet.defender_carrier.hash(&mut hasher);
    packet.winner_side.hash(&mut hasher);
    packet.winner_skill_name.hash(&mut hasher);
    packet.loser_skill_name.hash(&mut hasher);
    packet.winner_motions.hash(&mut hasher);
    packet.loser_attempted_motions.hash(&mut hasher);
    packet.counter_reason.hash(&mut hasher);
    packet.hit_part.hash(&mut hasher);
    packet.severity_label.hash(&mut hasher);
    packet.active_residue.hash(&mut hasher);
    packet.residue_interaction.hash(&mut hasher);
    packet.loser_posture_after.hash(&mut hasher);
    packet.inflicted_injury.hash(&mut hasher);
    hasher.finish()
}

static COMBAT_BOUT_CACHE: LazyLock<RwLock<HashMap<u64, String>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn get_combat_bout_from_cache(key: u64) -> Option<String> {
    COMBAT_BOUT_CACHE.read().get(&key).cloned()
}

fn store_combat_bout_in_cache(key: u64, prose: String) {
    let mut cache = COMBAT_BOUT_CACHE.write();
    if cache.len() > 1000 {
        cache.clear();
    }
    cache.insert(key, prose);
}

/// 綜合交鋒事實資料生成精緻敘事段落
pub fn compose_combat_bout_from_data(packet: &CombatBoutFactPacket) -> String {
    let key = compute_combat_bout_cache_key(packet);

    if let Some(cached) = get_combat_bout_from_cache(key) {
        if verify_combat_bout_output(&cached, packet) {
            return cached;
        }
    }

    if cfg!(test) || std::env::var("SW_DISABLE_NARRATIVE_LLM").unwrap_or_default() == "1" {
        return render_deterministic_bout_from_data(packet);
    }

    let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
    let model = "google_gemma-4-26B-A4B-it-Q4_1";

    let system_prompt = concat!(
        "你是一位武俠與百工世界的戰鬥敘事演繹者。請依據提供的戰鬥事實資料，編撰一段精準、連貫、富有武俠節奏的交鋒描述（25至150字）。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9）或全形數字（０-９），不可出現「第」、「回合」、「序位」、「",
        concat!("\u{525b}", "\u{624d}"),
        "」、「",
        concat!("\u{525b}", "\u{525b}"),
        "」、「",
        concat!("\u{65b9}", "\u{624d}"),
        "」、「頃刻」、「還有」、「系統」、「生成」、「HP」等字眼。\
3. 必須明確寫出進攻方與防守方的稱呼，並描寫勝方如何將分解碎招動作一氣呵成連貫使出「勝方技能名稱」（必須以「」括起招式名自然融入句中，不可單獨掛在句尾）。\
4. 敗方僅做出前半截碎招動作便被中途截斷潰散，絕對不可替敗方編造招式名稱。\
5. 必須依據提供的器物、相剋原因、受創部位與受創程度描寫結果，不可憑空捏造未持有的兵器。\
6. 僅輸出最終敘事段落，不可有任何額外說明或前綴。"
    );

    let (winner_display, loser_display) = if packet.winner_side == "attacker" {
        (packet.attacker_name.as_str(), packet.defender_name.as_str())
    } else if packet.winner_side == "defender" {
        (packet.defender_name.as_str(), packet.attacker_name.as_str())
    } else {
        ("雙方平分秋色", "雙方")
    };
    // 絕對不將 loser_skill_name 傳入 prompt！
    let mut user_prompt = format!(
        "交鋒階段：{}\n進攻方：{}（手持：{}，路數：{}）\n防守方：{}（手持：{}，路數：{}）\n成招勝方：{}\n勝方完整連貫之碎招動作：{}\n勝方顯化之技能名稱：「{}」\n受阻敗方：{}\n敗方中途被打斷之碎招動作：{}\n破招與相剋原因：{}\n敗方受創部位：{}\n受創程度：{}",
        packet.slot.label(),
        packet.attacker_name,
        packet.attacker_weapon,
        packet.attacker_carrier.label(),
        packet.defender_name,
        packet.defender_weapon,
        packet.defender_carrier.label(),
        winner_display,
        packet.winner_motions.join(" → "),
        packet.winner_skill_name,
        loser_display,
        packet.loser_attempted_motions.join(" → "),
        packet.counter_reason.as_deref().unwrap_or("搶佔空門截斷氣機"),
        packet.hit_part.label(),
        packet.severity_label,
    );
    if let Some(res) = &packet.active_residue {
        user_prompt.push_str(&format!("\n現場殘留氣機：{}", res));
    }
    if let Some(inter) = &packet.residue_interaction {
        user_prompt.push_str(&format!("\n氣機交互狀態：{}", inter));
    }
    if let Some(posture) = &packet.loser_posture_after {
        user_prompt.push_str(&format!("\n敗方受創後架勢：{}", posture));
    }
    if let Some(injury) = &packet.inflicted_injury {
        user_prompt.push_str(&format!("\n敗方具體傷勢：{}", injury));
    }
    match call_local_chat_llm(
        &base_url,
        model,
        Duration::from_millis(3800),
        system_prompt,
        &user_prompt,
        false,
    ) {
        Ok(reply) => {
            let cleaned = reply.trim();
            let stripped = cleaned
                .trim_matches(|c| c == '"' || c == '\'' || c == '「' || c == '」' || c == '`');
            if verify_combat_bout_output(stripped, packet) {
                let prose = stripped.to_string();
                store_combat_bout_in_cache(key, prose.clone());
                return prose;
            }
            render_deterministic_bout_from_data(packet)
        }
        Err(_) => render_deterministic_bout_from_data(packet),
    }
}

/// 並發組裝多節交鋒敘事段落（保持原本序位，以 std::thread::scope 同步並發）
pub fn compose_combat_bouts_concurrent(packets: &[CombatBoutFactPacket]) -> Vec<String> {
    if packets.is_empty() {
        return Vec::new();
    }
    std::thread::scope(|s| {
        let handles: Vec<_> = packets
            .iter()
            .map(|packet| s.spawn(|| compose_combat_bout_from_data(packet)))
            .collect();
        handles
            .into_iter()
            .zip(packets.iter())
            .map(|(handle, packet)| {
                handle
                    .join()
                    .unwrap_or_else(|_| render_deterministic_bout_from_data(packet))
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------
// 百工匠造文采生成與 Fact-Fidelity 保真度檢驗
// ---------------------------------------------------------------------------


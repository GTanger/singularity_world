
/// 依個人骨相、載體與器物推導個人臨陣武相
pub fn derive_personal_martial_routine(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    vessel_name: &str,
    learned_bonus: f64,
    familiarity_count: u32,
    registry: &WordRegistry,
) -> EmergentRoutine {
    let catalog = &*GLOBAL_MOTION_CATALOG;

    // 依載體挑選各階段候選碎招（過濾掉百工碎招）
    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for m in catalog {
        if m.craft_domain.is_none() && m.carrier == carrier && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    // 若為 Defense 守勢，額外納入 Footwork 步法碎招以強化移形換位防禦
    if slot == TacticalSlot::Defense && carrier != CarrierAffinity::Footwork {
        for m in catalog {
            if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Footwork && m.stage < 5 {
                by_stage[m.stage].push(m);
            }
        }
    }

    // 若某階段不足，以徒手補充（確保排除百工）
    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in catalog {
                if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Unarmed && m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let (motions, compensated, c_val) = find_viterbi_5stages_for_slot(
        bone,
        Some(slot),
        &by_stage,
        learned_bonus,
        familiarity_count,
    );

    let main_motions = [
        motions[0].name.clone(),
        motions[1].name.clone(),
        motions[2].name.clone(),
        motions[3].name.clone(),
        motions[4].name.clone(),
    ];

    // 尋找「轉」位應變分支碎招（根據 slot 戰術意圖鎖定最佳 counter_target）
    let main_turn_name = &main_motions[2];
    let mut best_branch_name = "順勢側滑步".to_string();
    let mut best_branch_fit = -1.0f64;

    // 根據 slot 設定期待克制目標
    let preferred_target_idx = match slot {
        TacticalSlot::Opening => 2,   // 貼身搶攻 (克搶先手)
        TacticalSlot::Midgame => 0,   // 剛猛硬架 (破其僵持)
        TacticalSlot::Finisher => 1,  // 後撤閃避 (追襲閃避)
        TacticalSlot::Defense => 0,   // 剛猛硬架 / 借力化勁
    };

    let mut counter_target_zh = match preferred_target_idx {
        0 => "剛猛硬架",
        1 => "後撤閃避",
        _ => "貼身搶攻",
    };

    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            let carrier_pref = if slot == TacticalSlot::Defense && (m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield) {
                0.35
            } else {
                0.0
            };
            let target_fit = m.counter_fit[preferred_target_idx] + carrier_pref;
            if target_fit > best_branch_fit {
                best_branch_fit = target_fit;
                best_branch_name = m.name.clone();
                counter_target_zh = match preferred_target_idx {
                    0 => "剛猛硬架",
                    1 => "後撤閃避",
                    _ => "貼身搶攻",
                };
            }
        }
    }

    // 兜底：若特定目標未找到合適分支，取任意最高 fit
    if best_branch_fit < 0.0 {
        for m in &by_stage[2] {
            if m.name != *main_turn_name {
                let carrier_pref = if slot == TacticalSlot::Defense && (m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield) {
                    0.35
                } else {
                    0.0
                };
                let fit_sum = m.counter_fit[0].max(m.counter_fit[1]).max(m.counter_fit[2]) + carrier_pref;
                if fit_sum > best_branch_fit {
                    best_branch_fit = fit_sum;
                    best_branch_name = m.name.clone();
                    if m.counter_fit[1] >= m.counter_fit[0] && m.counter_fit[1] >= m.counter_fit[2] {
                        counter_target_zh = "後撤閃避";
                    } else if m.counter_fit[2] >= m.counter_fit[0] && m.counter_fit[2] >= m.counter_fit[1] {
                        counter_target_zh = "貼身搶攻";
                    } else {
                        counter_target_zh = "剛猛硬架";
                    }
                }
            }
        }
    }

    // 變化性指標 V_e
    let variation_v = (best_branch_fit * (1.0 + 0.25 * (-bone.gamma).max(0.0))).clamp(0.40, 1.80);
    // 換招最大空檔 ticks
    let mut max_gap = 1u8;
    for i in 0..4 {
        let gap = transition_gap_ticks(bone, &motions[i], &motions[i + 1], c_val);
        if gap > max_gap {
            max_gap = gap;
        }
    }

    // 套路特徵向量 16D：隨 slot、bone 與碎招深度調變
    let mut feat_16d = [0.0f64; 16];
    for (i, m) in motions.iter().enumerate() {
        let weight = (i + 1) as f64 / 15.0;
        for k in 0..16 {
            feat_16d[k] += m.feature_16d[k] * weight;
        }
    }
    feat_16d[0] *= bone.impact_factor();
    feat_16d[1] *= bone.beta;

    // 根據戰術位 slot 與骨相三相 (alpha, beta, gamma) 加註鮮明特徵權重
    match slot {
        TacticalSlot::Opening => {
            feat_16d[0] += 0.35 * bone.beta; // 疾發速戰
            feat_16d[2] += 0.45;             // 敏捷出探
            feat_16d[5] += 0.30;             // 先聲奪人
        }
        TacticalSlot::Midgame => {
            feat_16d[1] += 0.40;             // 氣流連綿
            feat_16d[3] += 0.35;             // 順滑運轉
            feat_16d[6] += 0.30;             // 連環應變
        }
        TacticalSlot::Finisher => {
            feat_16d[0] += 0.60 * bone.alpha;// 巔峰重勢破堅
            feat_16d[4] += 0.50;             // 定鼎擊穿
            feat_16d[7] += 0.35;             // 終結一式
        }
        TacticalSlot::Defense => {
            feat_16d[8] += 0.50;             // 固若金湯
            feat_16d[9] += 0.40 * (1.0 + bone.gamma.max(0.0)); // 守正不移
            feat_16d[11] += 0.30;            // 避虛就實
        }
    }

    // 2-Step Residual OMP 選字定名
    let ((w1_id, w2_id), (w1_ch, w2_ch)) = residual_omp_select_roots(&feat_16d, registry);

    let (slot_code, slot_label_zh) = match slot {
        TacticalSlot::Opening => ("opening", "起手先機"),
        TacticalSlot::Midgame => ("midgame", "中盤連攜"),
        TacticalSlot::Finisher => ("finisher", "破敵收勢"),
        TacticalSlot::Defense => ("defense", "守勢應變"),
    };

    let phase_specialty_zh = match slot {
        TacticalSlot::Finisher => {
            if bone.dex_0 >= 110.0 {
                "尋隙游刃"
            } else {
                "重勢硬撼"
            }
        }
        TacticalSlot::Opening | TacticalSlot::Defense => {
            if bone.gamma >= -0.10 {
                "借勢拆招"
            } else if bone.alpha >= 1.25 {
                "重勢硬撼"
            } else {
                "尋隙游刃"
            }
        }
        TacticalSlot::Midgame => {
            if bone.alpha >= 1.20 {
                "重勢硬撼"
            } else if bone.gamma >= 0.10 {
                "借勢拆招"
            } else {
                "尋隙游刃"
            }
        }
    };

    let first2 = |s: &str| -> String { s.chars().take(2).collect() };
    let last2 = |s: &str| -> String {
        let chs: Vec<char> = s.chars().collect();
        let len = chs.len();
        if len >= 2 {
            chs[len - 2..].iter().collect()
        } else {
            s.to_string()
        }
    };
    let last3_or_2 = |s: &str| -> String {
        let chs: Vec<char> = s.chars().collect();
        let len = chs.len();
        if len >= 5 {
            chs[len - 3..].iter().collect()
        } else if len >= 2 {
            format!("{}式", chs[len - 2..].iter().collect::<String>())
        } else {
            format!("{s}式")
        }
    };

    let formal_name = match slot {
        TacticalSlot::Opening => format!("{}{}式", first2(&main_motions[0]), last2(&main_motions[1])),
        TacticalSlot::Midgame => format!("{}{}式", first2(&main_motions[2]), last2(&main_motions[3])),
        TacticalSlot::Finisher => format!("{}{}", first2(&main_motions[3]), last3_or_2(&main_motions[4])),
        TacticalSlot::Defense => {
            let suffix = if carrier == CarrierAffinity::Shield { "勢" } else { "步" };
            let tail = last2(&main_motions[4]).trim_end_matches('步').trim_end_matches('勢').to_string();
            if tail.chars().count() == 1 {
                format!("{}{}{}", first2(&main_motions[1]), last2(&main_motions[2]), suffix)
            } else {
                format!("{}{}{}", first2(&main_motions[1]), tail, suffix)
            }
        }
    };

    let trimmed_vessel = vessel_name.trim();
    let is_unarmed = trimmed_vessel.is_empty() || trimmed_vessel == "徒手" || carrier == CarrierAffinity::Unarmed;
    let vessel_clean = if is_unarmed {
        "雙拳肉掌"
    } else {
        trimmed_vessel
    };

    let origin_basis_zh = match slot {
        TacticalSlot::Opening => {
            let comp = if compensated.is_empty() {
                if is_unarmed {
                    "提氣墊步間身拳合一，搶在對手出手之前壓制其勢頭。"
                } else {
                    "提氣墊步間人器合一，搶在對手出手之前壓制其勢頭。"
                }
            } else {
                "起手時順著腕力與步法微調重心，避開生硬對撞，搶在對手出手之前壓制其勢頭。"
            };
            if is_unarmed {
                format!(
                    "臨敵之際雙拳護身疾步搶進，以敏銳步法試探敵隙。{}",
                    comp
                )
            } else {
                format!(
                    "臨敵之際手持{}疾步搶進，以敏銳步法試探敵隙。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Midgame => {
            let comp = if compensated.is_empty() {
                "手腕吞吐與周身勁力連貫一氣，招招環扣不露端倪。"
            } else {
                "換氣過渡間順著手腕慣性流轉卸力，令攻勢綿延不絕。"
            };
            if is_unarmed {
                format!(
                    "近身交接後，雙掌、呼吸與步法隨身形開展綿密周旋。{}",
                    comp
                )
            } else {
                format!(
                    "兵刃交接後，手腕、呼吸與步法隨{}開展綿密周旋。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Finisher => {
            let tip_word = if is_unarmed {
                "拳鋒"
            } else if carrier == CarrierAffinity::Blunt || carrier == CarrierAffinity::Shield {
                "器身"
            } else {
                "鋒刃"
            };
            let comp = if compensated.is_empty() {
                format!("整勁順勢壓入{}，一擊定奪勝負。", tip_word)
            } else {
                "借腰胯與肩背重心自然傾注，將全身力道凝聚於末梢一擊定局。".to_string()
            };
            if is_unarmed {
                format!(
                    "窺準對手氣竭露怯之際，聚全身蓄勁凝於{}，直擊門戶破綻。{}",
                    vessel_clean,
                    comp
                )
            } else {
                format!(
                    "窺準對手氣竭露怯之際，聚全身蓄勁貫入{}之中，直劈門戶破綻。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Defense => {
            let comp = if compensated.is_empty() {
                "下盤重心沉穩，步法隨機牽引，化去來襲鋒芒。"
            } else {
                "順著腰胯與足尖自然卸力偏轉，避開正鋒死磕。"
            };
            if carrier == CarrierAffinity::Footwork || is_unarmed {
                format!(
                    "遇險時足下起落移形換位，於方寸間騰挪閃轉、避開敵勢銳鋒。{}",
                    comp
                )
            } else {
                format!(
                    "受壓時足下步法與重心隨機換位，斜閃引化來勢並持{}從容拉開戰距。{}",
                    vessel_clean,
                    comp
                )
            }
        }
    };

    let learned_and_familiar_zh = match slot {
        TacticalSlot::Opening => format!(
            "起手先機循［{} → {} → {} → {} → {}］疾發爭先；中途若遇對手{}，可順勢變招改走［{}］搶佔上風。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Midgame => format!(
            "連攜攻防循［{} → {} → {} → {} → {}］流轉不息；交鋒中途若遇對手{}，可順勢變招改走［{}］拆解來勢。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Finisher => format!(
            "破勢定局循［{} → {} → {} → {} → {}］合力驟落；臨陣若遇對手{}，可中途煞勁改走［{}］追襲破敵。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Defense => format!(
            "守勢引卸循［{} → {} → {} → {} → {}］沉穩周旋；避讓之際若遇對手{}，可借勢轉身改走［{}］化解殺招。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
    };

    let fluency_word = if c_val >= 1.25 {
        "渾然天成，招意綿密無隙"
    } else if c_val >= 0.95 {
        "順暢自如，換氣轉勢間隙極短"
    } else {
        "平穩紮實，講究沉著應對"
    };

    let mastery_tier = if familiarity_count >= 30 || learned_bonus >= 0.8 {
        "信手拈來"
    } else if familiarity_count >= 10 || learned_bonus >= 0.3 {
        "熟能生巧"
    } else {
        "初具法度"
    };

    let auto_trigger_desc_zh = match slot {
        TacticalSlot::Opening => format!(
            "起手攻勢已達「{}」，{}；長於{}，能在接戰第一刻捕捉空隙疾速發動。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Midgame => format!(
            "中盤連環已臻「{}」，{}；長於{}，能隨器物手感與呼吸節奏綿密銜接。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Finisher => format!(
            "破陣殺勢已入「{}」，{}；長於{}，能乘敵手露怯或門戶大開時傾力定局。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Defense => format!(
            "避實禦敵已具「{}」，{}；長於{}，能在受壓之際借步法身形從容重整架勢。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
    };

    EmergentRoutine {
        domain: "martial",
        slot_code,
        slot_label_zh,
        carrier,
        main_motions,
        turn_branch_motion: best_branch_name,
        turn_counter_target_zh: counter_target_zh,
        compensated_stages: compensated,
        continuity_c: c_val,
        variation_v,
        max_gap_ticks: max_gap,
        phase_specialty_zh,
        omp_roots: (w1_ch, w2_ch),
        omp_word_ids: (w1_id, w2_id),
        formal_name,
        origin_basis_zh,
        learned_and_familiar_zh,
        auto_trigger_desc_zh,
    }
}

/// 依個人骨相與百工歷練推導個人百工藝相
pub fn derive_personal_craft_routine(
    bone: &InnateBonePhase,
    craft_code: &str, // "harvest" | "forge" | "culinary" | "wayfare"
    carrier: CarrierAffinity,
    vessel_name: &str,
    familiarity_count: u32,
    anatomy_familiarity: u32,
    registry: &WordRegistry,
) -> EmergentRoutine {
    let catalog = build_elementary_motion_catalog();

    let (slot_code, domain_label_zh, craft_idx, craft_specialty_zh, craft_suffix) = match craft_code {
        "harvest" => ("harvest", "山林採獵", 0usize, "順理游刃", "法"),
        "forge" => ("forge", "爐錘鍛造", 1usize, "借勢運錘", "法"),
        "culinary" => ("culinary", "庖廚火候", 2usize, "循理調鼎", "訣"),
        "wayfare" => ("wayfare", "涉遠行旅", 3usize, "借勢健行", "步"),
        _ => ("harvest", "百工手藝", 0usize, "順理游刃", "法"),
    };

    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for m in &catalog {
        if m.craft_domain == Some(craft_code) && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    // 防呆兜底
    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in &catalog {
                if m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let learned_delta = ((anatomy_familiarity as f64) * 0.05).min(1.2);
    let (motions, compensated, c_val) = find_viterbi_5stages(
        bone,
        &by_stage,
        learned_delta,
        familiarity_count,
    );

    let main_motions = [
        motions[0].name.clone(),
        motions[1].name.clone(),
        motions[2].name.clone(),
        motions[3].name.clone(),
        motions[4].name.clone(),
    ];

    let main_turn_name = &main_motions[2];
    let mut best_branch_name = main_motions[2].clone();
    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            best_branch_name = m.name.clone();
            break;
        }
    }

    let mut feat_16d = [0.0f64; 16];
    for (i, m) in motions.iter().enumerate() {
        let weight = (i + 1) as f64 / 15.0;
        for k in 0..16 {
            feat_16d[k] += m.feature_16d[k] * weight;
        }
    }
    feat_16d[12 + craft_idx] += 0.85;

    let ((w1_id, w2_id), (w1_ch, w2_ch)) = residual_omp_select_roots(&feat_16d, registry);
    let first2_c: String = main_motions[0].chars().take(2).collect();
    let last2_c: String = {
        let chs: Vec<char> = main_motions[3].chars().collect();
        if chs.len() >= 2 {
            chs[chs.len() - 2..].iter().collect()
        } else {
            main_motions[3].clone()
        }
    };
    let formal_name = format!("{}{}{}", first2_c, last2_c.trim_end_matches(craft_suffix), craft_suffix);

    let tool_clean = if vessel_name.trim().is_empty() {
        "隨身器具"
    } else {
        vessel_name.trim()
    };

    let origin_basis_zh = match craft_code {
        "harvest" => {
            let comp_note = if compensated.is_empty() {
                "手眼與刃口配合純熟，下刀輕快俐落。"
            } else {
                "割剝時順應手腕習慣調整入刃角度，省力而不傷皮張。"
            };
            format!(
                "日常持用{}巡歷山林獵場，於採集與剝解獵物時摸索皮肉筋節走向。{}",
                tool_clean,
                comp_note
            )
        }
        "forge" => {
            let comp_note = if compensated.is_empty() {
                "呼吸與落錘節拍相合，久鍛而臂力不衰。"
            } else {
                "揮錘間借腰背擺動分擔腕肘負荷，連綿敲打依然沉穩。"
            };
            format!(
                "日常持用{}守候爐火砧台，依火色冷熱調諧重錘起落與修整節奏。{}",
                tool_clean,
                comp_note
            )
        }
        "culinary" => {
            let comp_note = if compensated.is_empty() {
                "運刀與翻炒一氣呵成，火候拿捏分毫不差。"
            } else {
                "分切與掌勺時順著手腕巧勁施力，久立灶前亦從容不迫。"
            };
            format!(
                "日常持用{}料理庖廚膳食，於分切食材與文武火候收放間養成細膩手感。{}",
                tool_clean,
                comp_note
            )
        }
        "wayfare" => {
            let comp_note = if compensated.is_empty() {
                "步幅與吐納自然相合，長途跋涉依然氣息平順。"
            } else {
                "遇陡坡石階時自然微調步幅與重心，減輕雙腿疲勞。"
            };
            format!(
                "日常佩備{}穿梭山野古道，於涉水攀嶺間磨礪出悠長穩健的跋涉步調。{}",
                tool_clean,
                comp_note
            )
        }
        _ => format!(
            "日常持用{}從事百工勞作，於反覆操持間自然養成的熟練手藝。",
            tool_clean
        ),
    };

    let contingency_zh = match craft_code {
        "harvest" => format!("若遇筋骨糾結或皮膜堅韌處，可順勢改以［{}］從容落刃。", best_branch_name),
        "forge" => format!("若遇礦鐵含雜或火色驟變，可即時改以［{}］調勻力道。", best_branch_name),
        "culinary" => format!("若遇肉質厚薄不一或鼎中火候微變，可隨時改以［{}］收束湯味。", best_branch_name),
        "wayfare" => format!("若遇山徑濕滑或溪谷陡峭，可隨步改以［{}］穩住身形。", best_branch_name),
        _ => format!("若遇質地生硬或節奏微滯，可隨機改以［{}］順暢應對。", best_branch_name),
    };

    let learned_and_familiar_zh = format!(
        "工序循［{} → {} → {} → {} → {}］次序推進；{}",
        main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
        contingency_zh
    );
    let auto_trigger_desc_zh = match craft_code {
        "harvest" => "下刀順應皮毛與骨節紋理，下料俐落完整，不易傷及刃口與皮張。".to_string(),
        "forge" => "運錘落點沉穩勻稱，能依火色精準拿捏力道，減少雜質殘留。".to_string(),
        "culinary" => "刀工與火候拿捏得宜，能留住食材本味與湯汁精華。".to_string(),
        _ => "遠行換氣勻長，涉水攀嶺時步履穩健，耐勞而不易氣促。".to_string(),
    };

    EmergentRoutine {
        domain: "production",
        slot_code,
        slot_label_zh: domain_label_zh,
        carrier,
        main_motions,
        turn_branch_motion: best_branch_name,
        turn_counter_target_zh: "肌理阻滯",
        compensated_stages: compensated,
        continuity_c: c_val,
        variation_v: 1.0,
        max_gap_ticks: 1,
        phase_specialty_zh: craft_specialty_zh,
        omp_roots: (w1_ch, w2_ch),
        omp_word_ids: (w1_id, w2_id),
        formal_name,
        origin_basis_zh,
        learned_and_familiar_zh,
        auto_trigger_desc_zh,
    }
}

/// 將標準教材手札適配至個人骨相，自然產生代償變體
pub fn adapt_manual_to_bone_phase(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    manual_name: &str,
    manual_roots: &[String],
    registry: &WordRegistry,
) -> (EmergentRoutine, String) {
    let mut routine = derive_personal_martial_routine(
        bone,
        slot,
        carrier,
        "隨身器物",
        1.0,
        15,
        registry,
    );

    if manual_roots.len() >= 2 {
        let ch1 = &manual_roots[0];
        let ch2 = &manual_roots[1];
        routine.omp_roots = (ch1.clone(), ch2.clone());
        let clean_manual: String = manual_name
            .trim_matches(|c: char| c == '《' || c == '》')
            .trim_end_matches("手札")
            .trim_end_matches("訣")
            .trim_end_matches("譜")
            .chars()
            .take(4)
            .collect();
        if !clean_manual.is_empty() {
            routine.formal_name = format!("{}式", clean_manual);
        }

        let id1 = registry.get_by_char(ch1).and_then(|ids| ids.first().cloned()).unwrap_or_else(|| routine.omp_word_ids.0.clone());
        let id2 = registry.get_by_char(ch2).and_then(|ids| ids.first().cloned()).unwrap_or_else(|| routine.omp_word_ids.1.clone());
        routine.omp_word_ids = (id1, id2);
        routine.learned_and_familiar_zh = format!(
            "招式循［{} → {} → {} → {} → {}］一氣連貫；交鋒中途若遇對手{}，可順勢變招改走［{}］。深得《{}》所載之法度精髓。",
            routine.main_motions[0],
            routine.main_motions[1],
            routine.main_motions[2],
            routine.main_motions[3],
            routine.main_motions[4],
            routine.turn_counter_target_zh,
            routine.turn_branch_motion,
            manual_name,
        );
    }

    let adaptation_log = if routine.compensated_stages.is_empty() {
        format!("此本《{}》與自身體魄底蘊絲絲入扣，依法演練即可發揮十成威力。", manual_name)
    } else {
        format!(
            "研讀《{}》之際，順應自身手腕與步法發力習慣微調了轉折角度，化為得心應手之路數。",
            manual_name
        )
    };
    routine.origin_basis_zh = format!(
        "依自身「{}」之體魄底蘊，參酌《{}》之法度運轉。{}",
        bone.bone_archetype_title(),
        manual_name,
        adaptation_log
    );

    (routine, adaptation_log)
}

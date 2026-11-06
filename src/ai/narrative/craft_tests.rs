/// 驗證百工產出文采是否嚴守無數值化與事實錨定
pub fn verify_craft_output(text: &str, packet: &CraftActionFactPacket) -> Result<(), String> {
    let trimmed = text.trim();
    let char_count = trimmed.chars().count();
    if char_count < 25 {
        return Err("百工文采長度不足二十五字".to_string());
    }
    if char_count > 260 {
        return Err("百工文采長度超過二百六十字".to_string());
    }

    if trimmed.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)) {
        return Err("百工文采嚴禁包含阿拉伯數字或全形數字".to_string());
    }
    if trimmed.contains('%') || trimmed.contains('％') || trimmed.contains('+') || trimmed.contains('＋') {
        return Err("百工文采嚴禁包含百分比或加號符號".to_string());
    }

    const FORBIDDEN_WORDS_CRAFT: &[&str] = &[
        "系統", "玩家", "生成", concat!("\u{525b}", "\u{624d}"), concat!("\u{525b}", "\u{525b}"), concat!("\u{65b9}", "\u{624d}"), "頃刻", "還有", "資料庫", "database", "grid", "debug",
        "scene_stub", concat!("聖", "塔"), concat!("110", "v"), concat!("念", "場"), concat!("富", "態"), concat!("小", "市"), concat!("短", "市"), concat!("青籠", "關"), "wt_", "mat_", "npc_", "hp", "數值", "點數",
    ];
    let lower = trimmed.to_lowercase();
    for &fw in FORBIDDEN_WORDS_CRAFT {
        if lower.contains(fw) {
            return Err(format!("百工文采包含禁用詞語：{fw}"));
        }
    }

    // 必須提及產出物名稱（或其二字字尾）
    let clean_output = clean_vessel_name(&packet.output_name);
    let output_chars: Vec<char> = clean_output.chars().collect();
    let output_suffix_2: Option<String> = if output_chars.len() >= 2 {
        Some(output_chars[output_chars.len() - 2..].iter().collect())
    } else {
        None
    };

    let output_matched = (!packet.output_name.is_empty() && trimmed.contains(&packet.output_name))
        || (!clean_output.is_empty() && trimmed.contains(&clean_output))
        || output_suffix_2.as_deref().map(|s| trimmed.contains(s)).unwrap_or(false);

    if !output_matched {
        return Err("百工文采未提及產出物名稱或其字尾".to_string());
    }

    // 必須提及至少一種所用材料或工器具
    let mut tool_or_mat_matched = false;
    let clean_tool = clean_vessel_name(&packet.tool_name);
    if !packet.tool_name.is_empty() && trimmed.contains(&packet.tool_name) {
        tool_or_mat_matched = true;
    } else if !clean_tool.is_empty() && trimmed.contains(&clean_tool) {
        tool_or_mat_matched = true;
    } else {
        let tool_chars: Vec<char> = clean_tool.chars().collect();
        if tool_chars.len() >= 2 {
            let suffix: String = tool_chars[tool_chars.len() - 2..].iter().collect();
            if trimmed.contains(&suffix) {
                tool_or_mat_matched = true;
            }
        }
    }

    if !tool_or_mat_matched {
        for mat in &packet.material_names {
            let clean_mat = clean_vessel_name(mat);
            if !mat.is_empty() && trimmed.contains(mat) {
                tool_or_mat_matched = true;
                break;
            }
            if !clean_mat.is_empty() && trimmed.contains(&clean_mat) {
                tool_or_mat_matched = true;
                break;
            }
            let mat_chars: Vec<char> = clean_mat.chars().collect();
            if mat_chars.len() >= 2 {
                let suffix: String = mat_chars[mat_chars.len() - 2..].iter().collect();
                if trimmed.contains(&suffix) {
                    tool_or_mat_matched = true;
                    break;
                }
            }
        }
    }

    if !tool_or_mat_matched {
        return Err("百工文采未提及任何所用材料或工器具".to_string());
    }

    Ok(())
}

/// 確定性百工匠造文采保底（無 LLM 或驗證失敗時之純文字兜底）
pub fn deterministic_craft_highlight_fallback(packet: &CraftActionFactPacket) -> String {
    let actor = if packet.actor_name.trim().is_empty() {
        "工匠"
    } else {
        packet.actor_name.trim()
    };
    let tool = if packet.tool_name.trim().is_empty() {
        "器具"
    } else {
        packet.tool_name.trim()
    };
    let mat = packet
        .material_names
        .first()
        .map(|s| s.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("素材");
    let out = if packet.output_name.trim().is_empty() {
        "器物"
    } else {
        packet.output_name.trim()
    };
    let habit = if packet.breath_habit.trim().is_empty() {
        "沉凝調息"
    } else {
        packet.breath_habit.trim()
    };
    let env = if packet.environment_hint.trim().is_empty() {
        "案前工坊"
    } else {
        packet.environment_hint.trim()
    };
    let fam = if packet.tool_familiarity.trim().is_empty() {
        "熟稔"
    } else {
        packet.tool_familiarity.trim()
    };
    let highlight = if packet.highlight_kind.trim().is_empty() {
        "火候純熟"
    } else {
        packet.highlight_kind.trim()
    };

    format!(
        "身處{}，{}運轉{}之息，手持{}更顯{}，凝神調理{}，處處顯露{}，終成{}，鋒芒神彩沉凝內斂。",
        env, actor, habit, tool, fam, mat, highlight, out
    )
}

/// 呼叫本地 LLM 或回退確定性管線生成百工匠造高光文采
pub fn compose_craft_highlight_from_data(packet: &CraftActionFactPacket) -> String {
    let enable_llm = std::env::var("SW_ENABLE_CRAFT_LLM").unwrap_or_else(|_| {
        if cfg!(test) {
            "0".to_string()
        } else {
            "1".to_string()
        }
    });

    if enable_llm == "1" {
        let base_url = std::env::var("SW_NARRATIVE_LLM_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434/v1".to_string());
        let model = "google_gemma-4-26B-A4B-it-Q4_1";

        let system_prompt = concat!(
            "你是一位百工匠造與山林採擷的文采演繹者。請依據工匠身手、持器心境、內息節奏與所用材料，描摹一段百工產出過程的光彩體感。\
嚴格要求：\
1. 嚴格使用台灣正體中文。\
2. 絕對不可出現任何阿拉伯數字（0-9）或全形數字（０-９），不可出現百分比符號或加號。\
3. 絕對不可出現「系統」、「玩家」、「生成」、「",
            concat!("\u{525b}", "\u{624d}"),
            "」、「",
            concat!("\u{525b}", "\u{525b}"),
            "」、「",
            concat!("\u{65b9}", "\u{624d}"),
            "」、「頃刻」、「還有」、「資料庫」、「database」、「HP」、「數值」等字眼。\
4. 描述長度必須在三十至二百二十字之間。\
5. 必須明確提及最終產出物名稱與所用材料或工具器具。\
6. 直接輸出這段純文字描述，切勿包含任何前後引言、標籤或額外格式。"
        );

        let materials_str = if packet.material_names.is_empty() {
            "各色素材".to_string()
        } else {
            packet.material_names.join("、")
        };

        let user_prompt = format!(
            "工匠姓名：{}\n手持工具：{}（熟悉度：{}）\n內息底色：{}\n所用材料：{}\n產出成品：{}\n工藝特徵：{}\n環境氛圍：{}",
            packet.actor_name,
            packet.tool_name,
            packet.tool_familiarity,
            packet.breath_habit,
            materials_str,
            packet.output_name,
            packet.highlight_kind,
            packet.environment_hint,
        );

        if let Ok(raw_reply) = call_local_chat_llm(
            &base_url,
            model,
            Duration::from_secs(6),
            system_prompt,
            &user_prompt,
            false,
        ) {
            let candidate = raw_reply.trim().to_string();
            if verify_craft_output(&candidate, packet).is_ok() {
                return candidate;
            }
        }
    }

    deterministic_craft_highlight_fallback(packet)
}

// ---------------------------------------------------------------------------
// 單元測試
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_deterministic_skill_martial_blade() {
        let packet = SkillDataPacket {
            entity_id: "test_entity".to_string(),
            role_title: "劍客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "太陰肺樞".to_string(),
            word_chars: vec!["斷".to_string(), "浪".to_string()],
            vessel_name: "分骨彎刀 [銳]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["弓步前突".to_string(), "旋腕斜撩".to_string(), "回身橫削".to_string()],
            fallback_name: "斷浪".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "斷浪刀式");
        assert!(prose.origin_basis.contains("分骨彎刀"));
        assert!(prose.origin_basis.contains("弓步前突"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_skill_unarmed() {
        let packet = SkillDataPacket {
            entity_id: "test_entity".to_string(),
            role_title: "行者".to_string(),
            domain: "martial".to_string(),
            slot_label: "連攜".to_string(),
            hub_or_tier: "少陽膽樞".to_string(),
            word_chars: vec!["崩".to_string(), "拳".to_string()],
            vessel_name: "徒手".to_string(),
            carrier: CarrierAffinity::Unarmed,
            motion_sequence: vec!["踏步進身".to_string(), "側身擺肘".to_string(), "直拳破空".to_string()],
            fallback_name: "崩拳".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "崩拳拳勢");
        assert!(prose.origin_basis.contains("徒手運勁"));
        assert!(!prose.origin_basis.contains("刀"));
        assert!(!prose.origin_basis.contains("劍"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_skill_production() {
        let packet = SkillDataPacket {
            entity_id: "artisan_1".to_string(),
            role_title: "工匠".to_string(),
            domain: "production".to_string(),
            slot_label: "百工".to_string(),
            hub_or_tier: "熟稔".to_string(),
            word_chars: vec!["巧".to_string(), "工".to_string()],
            vessel_name: "雕花刻刀 [良]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["審度紋理".to_string(), "斜刃修邊".to_string(), "細磨拋光".to_string()],
            fallback_name: "細琢巧工".to_string(),
        };

        let prose = render_deterministic_skill_from_data(&packet);
        assert_eq!(prose.name, "細琢巧工");
        assert!(prose.origin_basis.contains("雕花刻刀"));
        assert!(verify_skill_output(&prose.name, &prose.origin_basis, &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_forbidden_words() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        // 禁忌詞：系統
        assert!(!verify_skill_output("系統刀式", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 禁忌詞：即時時間詞
        assert!(!verify_skill_output("斬風刀式", concat!(concat!("\u{525b}", "\u{624d}"), "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。"), &packet));
        // 禁忌詞：第
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，第一序位由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 禁忌詞：資料庫
        assert!(!verify_skill_output("斬風刀式", "資料庫記載手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_ascii_digits() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        assert!(!verify_skill_output("斬風刀1", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，由1「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        // 全形數字
        assert!(!verify_skill_output("斬風刀１", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀式", "手執鋼刀，由２「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_punctuation_in_name() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "鋼刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["提刃起步".to_string(), "橫揮斷水".to_string(), "收鋒定步".to_string()],
            fallback_name: "斬風".to_string(),
        };

        assert!(!verify_skill_output("[斬風刀]", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風·刀", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
        assert!(!verify_skill_output("斬風刀！", "手執鋼刀，由「提刃起步」起勢，順接「橫揮斷水」，再以「收鋒定步」一氣呵成。", &packet));
    }

    #[test]
    fn test_verify_skill_output_reject_weapon_mismatch() {
        // 徒手不能包含兵刃名
        let unarmed_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "拳手".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["拳".to_string()],
            vessel_name: "徒手".to_string(),
            carrier: CarrierAffinity::Unarmed,
            motion_sequence: vec!["推掌向前".to_string(), "抱元守一".to_string(), "收步凝神".to_string()],
            fallback_name: "推掌".to_string(),
        };
        assert!(!verify_skill_output("推掌劍式", "徒手運勁，由「推掌向前」起勢，順接「抱元守一」，再以「收步凝神」連貫成招。", &unarmed_packet));
        assert!(!verify_skill_output("推掌拳勢", "手握寶劍，由「推掌向前」起勢，順接「抱元守一」，再以「收步凝神」連貫成招。", &unarmed_packet));

        // 鈍器 Blunt 不可含 劍、槍、矛
        let blunt_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "力士".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["破".to_string()],
            vessel_name: "玄鐵重錘".to_string(),
            carrier: CarrierAffinity::Blunt,
            motion_sequence: vec!["引錘蓄力".to_string(), "砸地碎石".to_string(), "回步收力".to_string()],
            fallback_name: "破岩".to_string(),
        };
        assert!(!verify_skill_output("破岩劍式", "手執玄鐵重錘，由「引錘蓄力」起勢，順接「砸地碎石」，再以「回步收力」一氣呵成。", &blunt_packet));
        assert!(!verify_skill_output("破岩重式", "如長槍直刺，由「引錘蓄力」起勢，順接「砸地碎石」，再以「回步收力」一氣呵成。", &blunt_packet));

        // 刀刃 Blade 不可含 錘、盾、槍、矛
        let blade_packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "刀客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["斬".to_string()],
            vessel_name: "青鋼長刀".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string(), "納刀回鞘".to_string()],
            fallback_name: "破空".to_string(),
        };
        assert!(!verify_skill_output("破空錘法", "手執青鋼長刀，由「拔刀出鞘」起勢，順接「斜斬空門」，再以「納刀回鞘」一氣呵成。", &blade_packet));
        assert!(!verify_skill_output("破空刀式", "如堅盾立防，由「拔刀出鞘」起勢，順接「斜斬空門」，再以「納刀回鞘」一氣呵成。", &blade_packet));
    }

    #[test]
    fn test_verify_skill_output_reject_ungrounded_motion() {
        let packet = SkillDataPacket {
            entity_id: "test".to_string(),
            role_title: "劍客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "樞位".to_string(),
            word_chars: vec!["刺".to_string()],
            vessel_name: "龍泉劍".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["點星尋隙".to_string(), "穿雲刺喉".to_string(), "飄身落定".to_string()],
            fallback_name: "追星".to_string(),
        };

        // origin_basis 完全未包含動作、兵器或銘文
        let ungrounded_basis = "天地玄黃宇宙洪荒日月盈昃辰宿列張寒來暑往秋收冬藏。";
        assert!(!verify_skill_output("追星劍式", ungrounded_basis, &packet));
    }

    #[test]
    fn test_render_deterministic_bout_and_verify() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀 [銳]".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷浪刀式".to_string(),
            loser_skill_name: "破甲拳".to_string(),
            winner_motions: vec!["弓步前突".to_string(), "旋腕斜撩".to_string()],
            loser_attempted_motions: vec!["沉身立肘".to_string()],
            counter_reason: Some("搶佔先機".to_string()),
            hit_part: TargetPart::Arm,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let prose = render_deterministic_bout_from_data(&packet);
        assert!(prose.contains("斷浪刀式"));
        assert!(!prose.contains("破甲拳")); // 敗方招式名稱絕不可出現
        assert!(prose.contains("碎招"));
        assert!(prose.contains("雙臂關節"));
        assert!(prose.contains("氣血翻湧"));
        assert!(verify_combat_bout_output(&prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_leaked_loser_skill() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Finisher,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "精鋼鐵棍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Pole,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "千鈞覆地".to_string(),
            winner_motions: vec!["欺身進步".to_string(), "反手揮刃".to_string()],
            loser_attempted_motions: vec!["提棍蓄力".to_string()],
            counter_reason: Some("截其破綻".to_string()),
            hit_part: TargetPart::Torso,
            severity_label: "重創失據".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        // 包含敗方招式名稱 "千鈞覆地"，必須被拒絕！
        let invalid_prose = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客以「千鈞覆地」應對遭化為碎招，軀幹心脈受創，重創失據。";
        assert!(!verify_combat_bout_output(invalid_prose, &packet));

        // 僅包含敗方動作 "提棍蓄力" 且無敗方招式名，驗證通過
        let valid_prose = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客嘗試提棍蓄力遭化為碎招，軀幹心脈受創，陷入重創失據。";
        assert!(verify_combat_bout_output(valid_prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_forbidden_words_and_digits() {
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "碎星拳".to_string(),
            winner_motions: vec!["欺身進步".to_string()],
            loser_attempted_motions: vec!["起手格擋".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        // 出現數字
        let digit_prose = "李俠以分骨彎刀施展「斷空絕斬」，欺身進步命中，扣除30點傷害，王客陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(digit_prose, &packet));

        // 出現全形數字
        let fullwidth_digit_prose = "李俠以分骨彎刀施展「斷空絕斬」，欺身進步命中，扣除１２點傷害，王客陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fullwidth_digit_prose, &packet));

        // 出現「第」、「回合」
        let round_prose = "第一回合交手，李俠以分骨彎刀施展「斷空絕斬」，欺身進步破開起手格擋，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(round_prose, &packet));

        // 出現「即時時間詞」
        let instant_prose = concat!(concat!("\u{525b}", "\u{624d}"), "李俠以分骨彎刀施展「斷空絕斬」，欺身進步破開起手格擋，王客受創散為碎招，陷入氣血翻湧。");
        assert!(!verify_combat_bout_output(instant_prose, &packet));
    }

    #[test]
    fn test_verify_combat_bout_reject_weapon_mismatch() {
        // 雙方皆為徒手，不可出現 劍、刀鋒、刀光、長槍、槍尖
        let packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            attacker_name: "拳師甲".to_string(),
            defender_name: "拳師乙".to_string(),
            attacker_weapon: "徒手".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Unarmed,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "大伏魔拳".to_string(),
            loser_skill_name: "八卦掌".to_string(),
            winner_motions: vec!["進步衝拳".to_string()],
            loser_attempted_motions: vec!["側閃游走".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let sword_leak = "拳師甲以雙拳貫勁施展「大伏魔拳」，劍氣森森由進步衝拳貫通，側閃游走化為碎招，軀幹心脈受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(sword_leak, &packet));

        let spear_leak = "拳師甲以雙拳貫勁施展「大伏魔拳」，身如長槍由進步衝拳貫通，側閃游走化為碎招，軀幹心脈受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(spear_leak, &packet));
    }

    #[test]
    fn test_render_deterministic_bout_defender_win_and_draw() {
        let def_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Defense,
            attacker_name: "刀客".to_string(),
            defender_name: "槍客".to_string(),
            attacker_weapon: "短刀".to_string(),
            defender_weapon: "精鐵長槍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Pole,
            winner_side: "defender".to_string(),
            winner_skill_name: "回馬突刺".to_string(),
            loser_skill_name: "撲面橫斬".to_string(),
            winner_motions: vec!["撤步引鋒".to_string(), "旋桿探心".to_string()],
            loser_attempted_motions: vec!["提刃猛撲".to_string()],
            counter_reason: Some("以長制短".to_string()),
            hit_part: TargetPart::Head,
            severity_label: "重創失據".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let def_prose = render_deterministic_bout_from_data(&def_packet);
        assert!(def_prose.contains("回馬突刺"));
        assert!(!def_prose.contains("撲面橫斬"));
        assert!(def_prose.contains("頭顱要害"));
        assert!(verify_combat_bout_output(&def_prose, &def_packet));

        let draw_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            attacker_name: "甲".to_string(),
            defender_name: "乙".to_string(),
            attacker_weapon: "青鋼長刀".to_string(),
            defender_weapon: "精鋼短劍".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Blade,
            winner_side: "draw".to_string(),
            winner_skill_name: "平分秋色".to_string(),
            loser_skill_name: String::new(),
            winner_motions: vec!["起步截擊".to_string()],
            loser_attempted_motions: vec!["橫刃格拒".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "輕微擦震".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let draw_prose = render_deterministic_bout_from_data(&draw_packet);
        assert!(verify_combat_bout_output(&draw_prose, &draw_packet));
    }

    #[test]
    fn test_synthesize_skill_and_compose_bout_fallbacks() {
        let skill_packet = SkillDataPacket {
            entity_id: "hero".to_string(),
            role_title: "行客".to_string(),
            domain: "martial".to_string(),
            slot_label: "起手".to_string(),
            hub_or_tier: "少陽膽樞".to_string(),
            word_chars: vec!["追".to_string(), "風".to_string()],
            vessel_name: "青鋼長刀 [銳]".to_string(),
            carrier: CarrierAffinity::Blade,
            motion_sequence: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string(), "納刀回鞘".to_string()],
            fallback_name: "追風".to_string(),
        };

        let synthesized = synthesize_skill_from_data(&skill_packet);
        assert!(verify_skill_output(&synthesized.name, &synthesized.origin_basis, &skill_packet));

        let bout_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "行客".to_string(),
            defender_name: "強梁".to_string(),
            attacker_weapon: "青鋼長刀".to_string(),
            defender_weapon: "粗木棒".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Blunt,
            winner_side: "attacker".to_string(),
            winner_skill_name: synthesized.name,
            loser_skill_name: "亂披風棒".to_string(),
            winner_motions: vec!["拔刀出鞘".to_string(), "斜斬空門".to_string()],
            loser_attempted_motions: vec!["掄棒下砸".to_string()],
            counter_reason: Some("刀勢先至".to_string()),
            hit_part: TargetPart::Arm,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };
        let bout_prose = compose_combat_bout_from_data(&bout_packet);
        assert!(verify_combat_bout_output(&bout_prose, &bout_packet));
    }

    #[test]
    fn test_verify_combat_bout_residue_and_concurrent() {
        // 1. active_residue 為 None 時，禁止捏造殘留特徵詞彙
        let base_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Opening,
            attacker_name: "李俠".to_string(),
            defender_name: "王客".to_string(),
            attacker_weapon: "分骨彎刀".to_string(),
            defender_weapon: "徒手".to_string(),
            attacker_carrier: CarrierAffinity::Blade,
            defender_carrier: CarrierAffinity::Unarmed,
            winner_side: "attacker".to_string(),
            winner_skill_name: "斷空絕斬".to_string(),
            loser_skill_name: "碎星拳".to_string(),
            winner_motions: vec!["欺身進步".to_string()],
            loser_attempted_motions: vec!["起手格擋".to_string()],
            counter_reason: None,
            hit_part: TargetPart::Torso,
            severity_label: "氣血翻湧".to_string(),
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: None,
            inflicted_injury: None,
        };

        let valid_clean = "李俠手執分骨彎刀施展「斷空絕斬」，由欺身進步順勢貫通；王客雖試圖以起手格擋應招卻散為碎招，軀幹心脈受創，陷入氣血翻湧。";
        assert!(verify_combat_bout_output(valid_clean, &base_packet));

        let fabricated_heat = "李俠手執分骨彎刀施展「斷空絕斬」，場中燥氣四溢，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_heat, &base_packet));

        let fabricated_frost = "李俠手執分骨彎刀施展「斷空絕斬」，霜氣凝結，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_frost, &base_packet));

        let fabricated_tremor = "李俠手執分骨彎刀施展「斷空絕斬」，暗震湧動，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_tremor, &base_packet));

        let fabricated_vortex = "李俠手執分骨彎刀施展「斷空絕斬」，旋流不散，由欺身進步順勢貫通，王客受創陷入氣血翻湧。";
        assert!(!verify_combat_bout_output(fabricated_vortex, &base_packet));

        // 2. active_residue 為 Some("熾烈燥氣") 時，要求重合關鍵字（例如 燥/熾/烈）
        let heat_packet = CombatBoutFactPacket {
            active_residue: Some("熾烈燥氣".to_string()),
            residue_interaction: Some("借勢共鳴".to_string()),
            loser_posture_after: Some("架勢潰散".to_string()),
            inflicted_injury: Some("雙肋挫傷".to_string()),
            ..base_packet.clone()
        };

        // 未提及殘留氣機字元之段落被拒絕
        assert!(!verify_combat_bout_output(valid_clean, &heat_packet));

        // 包含 "燥" 之段落驗證通過
        let heat_prose = render_deterministic_bout_from_data(&heat_packet);
        assert!(heat_prose.contains("熾烈燥氣"));
        assert!(heat_prose.contains("借勢共鳴"));
        assert!(heat_prose.contains("雙肋挫傷"));
        assert!(heat_prose.contains("架勢潰散"));
        assert!(verify_combat_bout_output(&heat_prose, &heat_packet));

        // 3. 測試並發組裝 compose_combat_bouts_concurrent
        let mid_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Midgame,
            active_residue: Some("凝寒霜氣".to_string()),
            residue_interaction: Some("氣脈受激".to_string()),
            ..base_packet.clone()
        };
        let fin_packet = CombatBoutFactPacket {
            slot: TacticalSlot::Finisher,
            active_residue: None,
            residue_interaction: None,
            loser_posture_after: Some("步履微晃".to_string()),
            ..base_packet
        };

        let packets = vec![heat_packet, mid_packet, fin_packet];
        let results = compose_combat_bouts_concurrent(&packets);
        assert_eq!(results.len(), 3);
        assert!(verify_combat_bout_output(&results[0], &packets[0]));
        assert!(verify_combat_bout_output(&results[1], &packets[1]));
        assert!(verify_combat_bout_output(&results[2], &packets[2]));
        assert!(results[0].contains("熾烈燥氣"));
        assert!(results[1].contains("凝寒霜氣"));
        assert!(results[2].contains("步履微晃"));
    }

    #[test]
    fn test_craft_output_verification_and_deterministic_fallback() {
        let packet = CraftActionFactPacket {
            actor_name: "鐵匠阿明".to_string(),
            tool_name: "淬火鐵錘 [銳]".to_string(),
            tool_familiarity: "得心應手".to_string(),
            breath_habit: "爐火沉重".to_string(),
            material_names: vec!["青剛赤鐵".to_string(), "松木炭".to_string()],
            output_name: "百煉青鋼匕首".to_string(),
            highlight_kind: "火候純熟".to_string(),
            environment_hint: "鐵匠鋪火爐旁".to_string(),
        };

        // 1. 確定性兜底必然通過驗證
        let fallback = deterministic_craft_highlight_fallback(&packet);
        assert!(
            verify_craft_output(&fallback, &packet).is_ok(),
            "確定性兜底必須通過驗證：{:?}",
            verify_craft_output(&fallback, &packet)
        );
        let char_count = fallback.chars().count();
        assert!((25..=260).contains(&char_count));
        assert!(!fallback.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)));
        assert!(!fallback.contains('%') && !fallback.contains('+'));
        assert!(fallback.contains("百煉青鋼匕首") || fallback.contains("匕首"));
        assert!(fallback.contains("淬火鐵錘") || fallback.contains("鐵錘") || fallback.contains("青剛赤鐵"));

        // 2. 測試 compose_craft_highlight_from_data 離線回退
        let composed = compose_craft_highlight_from_data(&packet);
        assert!(verify_craft_output(&composed, &packet).is_ok());

        // 3. 測試驗證器邊界條件
        // 長度不足 25 字
        assert!(verify_craft_output("太短的淬火鐵錘匕首", &packet).is_err());

        // 包含阿拉伯數字
        let with_digit = format!("{} 帶有1把新器物", fallback);
        assert!(verify_craft_output(&with_digit, &packet).is_err());

        // 包含全形數字
        let with_full_digit = format!("{} 耗時３炷香", fallback);
        assert!(verify_craft_output(&with_full_digit, &packet).is_err());

        // 包含百分比或加號
        let with_percent = format!("{} 提升10%純度", fallback);
        assert!(verify_craft_output(&with_percent, &packet).is_err());
        let with_plus = format!("{} 威力+5", fallback);
        assert!(verify_craft_output(&with_plus, &packet).is_err());

        // 包含禁用詞彙（系統、玩家、即時時間詞、等等）
        let with_forbidden1 = format!("{} 系統提示完成", fallback);
        assert!(verify_craft_output(&with_forbidden1, &packet).is_err());
        let with_forbidden2 = format!("{} 玩家凝神細看", fallback);
        assert!(verify_craft_output(&with_forbidden2, &packet).is_err());
        let with_forbidden3 = format!("{} {}火勢熾烈", fallback, concat!("\u{65b9}", "\u{624d}"));
        assert!(verify_craft_output(&with_forbidden3, &packet).is_err());
        let with_forbidden4 = format!("{} {}震盪不休", fallback, concat!("念", "場"));
        assert!(verify_craft_output(&with_forbidden4, &packet).is_err());
        let with_forbidden5 = format!("{} 還有多餘雜質", fallback);
        assert!(verify_craft_output(&with_forbidden5, &packet).is_err());

        // 未提及產出物
        let no_output = "身處鐵匠鋪火爐旁，鐵匠阿明運轉爐火沉重之息，手持淬火鐵錘更顯得心應手，凝神調理青剛赤鐵，處處顯露火候純熟，神彩沉凝內斂。";
        assert!(verify_craft_output(no_output, &packet).is_err());

        // 未提及任何材料或工具
        let no_tool_or_mat = "身處鐵匠鋪火爐旁，鐵匠阿明運轉爐火沉重之息，得心應手，處處顯露火候純熟，終成百煉青鋼匕首，鋒芒神彩沉凝內斂。";
        assert!(verify_craft_output(no_tool_or_mat, &packet).is_err());
    }
}

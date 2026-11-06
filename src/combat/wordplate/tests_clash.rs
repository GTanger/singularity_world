
    #[test]
    fn test_resolve_bout_style_countering_and_weight_class_domination() {
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(6).collect();

        // 構建巧勁流技法（高先發、高流轉、高破甲）
        let inlay_light = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N121", &words[2].id, OrbitFluency::ChangLiu),
        ];
        let sent_light = StarSentence::new(inlay_light).unwrap().with_custom_name("輕靈巧解式");
        let mut tech_light = sent_light.synthesize(registry).unwrap();
        tech_light.vector_12d.tempo_early = 1.5;
        tech_light.vector_12d.flow = 1.2;
        tech_light.vector_12d.penetration = 1.2;
        tech_light.vector_12d.shock = 0.2;
        tech_light.vector_12d.sustain = 0.2;

        // 構建剛猛厚重技法（高震盪、高持恆、厚重如全真剛硬）
        let inlay_heavy = vec![
            StarInlay::with_fluency("N002", &words[3].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[4].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N122", &words[5].id, OrbitFluency::PingWen),
        ];
        let sent_heavy = StarSentence::new(inlay_heavy).unwrap().with_custom_name("沉嶽劈山勢");
        let mut tech_heavy = sent_heavy.synthesize(registry).unwrap();
        tech_heavy.vector_12d.tempo_early = 0.5;
        tech_heavy.vector_12d.shock = 1.6;
        tech_heavy.vector_12d.sustain = 1.4;
        tech_heavy.vector_12d.penetration = 0.3;
        tech_heavy.vector_12d.flow = 0.3;

        // 情況一：同量級拆招（玉女克全真：以巧破剛）
        // 雙方量級均為 100.0（ratio 1.0 < 1.45）
        let bout_finesse = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_light),
            Some(&tech_heavy),
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_finesse.winner_side, "side_a");
        assert!(bout_finesse.a_manifested_as_skill, "巧勁勝出方應正式成招顯化為技能");
        assert!(!bout_finesse.b_manifested_as_skill, "受克制厚重方套路應崩散為碎招");
        assert!(bout_finesse.narrative.contains("碎招"), "敘事應明確體現散為碎招");
        assert!(bout_finesse.counter_reason.as_ref().unwrap().contains("以巧破剛"));

        // 情況二：跨量級碾壓（一力降十會）
        // 厚重方體魄量級提升至 250.0，巧勁方為 100.0（ratio 2.5 >= 1.45）
        let bout_crush = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_light),
            Some(&tech_heavy),
            1.0,
            1.0,
            100.0,
            250.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_crush.winner_side, "side_b");
        assert!(bout_crush.b_manifested_as_skill, "量級碾壓方成招顯化為技能");
        assert!(!bout_crush.a_manifested_as_skill, "巧勁方套路被絕對力量轟散為碎招");
        assert!(bout_crush.narrative.contains("碎招"));
        assert!(bout_crush.counter_reason.as_ref().unwrap().contains("重勢硬壓"));

        // 情況三：十會解一力（以無厚入有間：當重擊方一味蠻幹毫無守勢根底 sustain < 0.5，而庖丁入微者具備極致穿透與流轉 >= 2.6 時，可避開正面蠻力切入骨縫反拆）
        let mut tech_master_paoding = tech_light.clone();
        tech_master_paoding.vector_12d.penetration = 1.5;
        tech_master_paoding.vector_12d.flow = 1.4;
        let mut tech_clumsy_brute = tech_heavy.clone();
        tech_clumsy_brute.vector_12d.sustain = 0.2;
        let bout_paoding = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_master_paoding),
            Some(&tech_clumsy_brute),
            1.0,
            1.0,
            100.0,
            180.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_paoding.winner_side, "side_a");
        assert!(bout_paoding.counter_reason.as_ref().unwrap().contains("入微破勢"));
    }

    #[test]
    fn test_posture_residue_and_fluency_integration() {
        // 1. PostureState 狀態與衰減
        assert_eq!(PostureState::default(), PostureState::Steady);
        assert_eq!(PostureState::Steady.label(), "架勢沉穩");
        assert_eq!(PostureState::Shaken.label(), "步履微晃");
        assert_eq!(PostureState::Broken.label(), "架勢潰散");
        assert_eq!(PostureState::Steady.degrade(), PostureState::Shaken);
        assert_eq!(PostureState::Shaken.degrade(), PostureState::Broken);
        assert_eq!(PostureState::Broken.degrade(), PostureState::Broken);

        // 2. BattlefieldResidue 與 ResidueInteraction 意象與標籤
        assert_eq!(BattlefieldResidue::ScorchingHeat.label(), "熾烈燥氣");
        assert_eq!(BattlefieldResidue::ScorchingHeat.keyword(), "燥");
        assert_eq!(BattlefieldResidue::FreezingFrost.label(), "凝寒霜氣");
        assert_eq!(BattlefieldResidue::FreezingFrost.keyword(), "寒");
        assert_eq!(BattlefieldResidue::DeepTremor.label(), "沉雄暗震");
        assert_eq!(BattlefieldResidue::DeepTremor.keyword(), "震");
        assert_eq!(BattlefieldResidue::GaleEdge.label(), "疾風銳勢");
        assert_eq!(BattlefieldResidue::GaleEdge.keyword(), "疾風");
        assert_eq!(BattlefieldResidue::VortexPull.label(), "牽引旋流");
        assert_eq!(BattlefieldResidue::VortexPull.keyword(), "旋流");

        assert!(ResidueInteraction::Resonance.label().contains("借勢共鳴"));
        assert!(ResidueInteraction::Backlash.label().contains("逆沖"));

        // 3. 同法不同相：相同字印在不同星軌通暢度（OrbitFluency）下推導出不同碎招序列
        let sample_chars = vec!["分".to_string(), "骨".to_string(), "斬".to_string()];
        let motions_smooth = derive_elementary_motions_with_fluency(
            &sample_chars,
            CarrierAffinity::Blade,
            TargetPart::Torso,
            OrbitFluency::ChangLiu,
        );
        let motions_hindered = derive_elementary_motions_with_fluency(
            &sample_chars,
            CarrierAffinity::Blade,
            TargetPart::Torso,
            OrbitFluency::ZhiSe,
        );
        assert_eq!(motions_smooth.len(), 3);
        assert_eq!(motions_hindered.len(), 3);
        assert_ne!(
            motions_smooth, motions_hindered,
            "同法不同相：暢流與滯澀之星軌通暢度應推導出完全不同的動作風格"
        );

        // 4. 三階共鳴倍率與階層
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(6).collect();

        // 5言暢流 -> tier 3 (1.55)
        let inlays_tier3 = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N121", &words[2].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N221", &words[3].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N321", &words[4].id, OrbitFluency::ChangLiu),
        ];
        let sent_tier3 = StarSentence::new(inlays_tier3).unwrap();
        let tech_tier3 = sent_tier3.synthesize(registry).unwrap();
        assert_eq!(tech_tier3.resonance_tier(), 3);
        assert!((tech_tier3.resonance_multiplier() - 1.55).abs() < 1e-6);

        // 3言平穩 -> tier 2 (1.25)
        let inlays_tier2 = vec![
            StarInlay::with_fluency("N002", &words[0].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[1].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N122", &words[2].id, OrbitFluency::PingWen),
        ];
        let sent_tier2 = StarSentence::new(inlays_tier2).unwrap();
        let tech_tier2 = sent_tier2.synthesize(registry).unwrap();
        assert_eq!(tech_tier2.resonance_tier(), 2);
        assert!((tech_tier2.resonance_multiplier() - 1.25).abs() < 1e-6);

        // 3言險阻 -> tier 1 (1.0)
        let inlays_tier1 = vec![
            StarInlay::with_fluency("N003", &words[0].id, OrbitFluency::XianZu),
            StarInlay::with_fluency("N023", &words[1].id, OrbitFluency::XianZu),
            StarInlay::with_fluency("N123", &words[2].id, OrbitFluency::XianZu),
        ];
        let sent_tier1 = StarSentence::new(inlays_tier1).unwrap();
        let tech_tier1 = sent_tier1.synthesize(registry).unwrap();
        assert_eq!(tech_tier1.resonance_tier(), 1);
        assert!((tech_tier1.resonance_multiplier() - 1.0).abs() < 1e-6);

        // 5. resolve_bout_with_state：勢均力敵平局
        let bout_draw = resolve_bout_with_state(
            TacticalSlot::Opening,
            1,
            None,
            None,
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
            PostureState::Steady,
            PostureState::Steady,
            None,
        );
        assert_eq!(bout_draw.winner_side, "draw");
        assert_eq!(bout_draw.damage_to_a, 0);
        assert_eq!(bout_draw.damage_to_b, 0);
        assert_eq!(bout_draw.a_posture_after, PostureState::Steady);
        assert_eq!(bout_draw.b_posture_after, PostureState::Steady);

        // 6. resolve_bout_with_state：殘留反沖（Backlash）導致架勢受阻下墜，但絕無自殘扣血（零自傷）
        let mut tech_flame = tech_tier2.clone();
        tech_flame.word_chars = vec!["烈".to_string(), "火".to_string()];
        tech_flame.vector_12d.tempo_early = 1.2;

        let mut tech_frost = tech_tier2.clone();
        tech_frost.word_chars = vec!["凝".to_string(), "霜".to_string()];
        tech_frost.vector_12d.tempo_early = 0.3;
        tech_frost.vector_12d[4] = 0.0;

        assert_eq!(tech_flame.emitted_residue(), Some(BattlefieldResidue::ScorchingHeat));
        assert_eq!(tech_frost.emitted_residue(), Some(BattlefieldResidue::FreezingFrost));

        // 戰場殘留「熾烈燥氣」，甲方出燥氣（順應共鳴），乙方出霜氣（煞氣逆沖）
        let bout_residue = resolve_bout_with_state(
            TacticalSlot::Opening,
            1,
            Some(&tech_flame),
            Some(&tech_frost),
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
            PostureState::Steady,
            PostureState::Steady,
            Some(BattlefieldResidue::ScorchingHeat),
        );
        assert_eq!(bout_residue.winner_side, "side_a");
        assert_eq!(bout_residue.a_residue_interaction, Some(ResidueInteraction::Resonance));
        assert_eq!(bout_residue.b_residue_interaction, Some(ResidueInteraction::Backlash));
        // 甲方無自傷，乙方傷害僅來自甲方的單向輸出（無自我反噬自傷）
        assert_eq!(bout_residue.damage_to_a, 0);
        assert!(bout_residue.damage_to_b > 0);
        // 乙方遭受反沖且受到傷害，架勢必然衰減為步履微晃或潰散
        assert_ne!(bout_residue.b_posture_after, PostureState::Steady);

        // 7. TacticalClashReport.summary 敘事嚴禁出現任何半形或阿拉伯數字
        let mut loadout_a = TacticalLoadout::new();
        let mut loadout_b = TacticalLoadout::new();
        loadout_a.set_slot(TacticalSlot::Opening, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Midgame, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Finisher, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Defense, sent_tier2.clone());

        loadout_b.set_slot(TacticalSlot::Opening, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Midgame, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Finisher, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Defense, sent_tier1.clone());

        let report = resolve_tianji_matchup(&loadout_a, 100, &loadout_b, 100, registry).expect("田忌對決成功");
        assert!(
            !report.summary.chars().any(|c| c.is_ascii_digit()),
            "戰報總結必須是純文字話語，嚴格禁止出現任何阿拉伯數字：{}",
            report.summary
        );
    }

    #[test]
    fn test_auto_detour_meridian_note_and_somatic_awareness() {
        let registry = WordRegistry::global();
        let seeds = [100_003i64, 42i64, 999_999i64, -12_345i64, 0i64];

        for &seed in &seeds {
            let awareness = somatic_meridian_awareness(seed);
            assert!(
                !awareness.is_empty(),
                "體感覺察不可為空"
            );
            assert!(
                !awareness.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "體感覺察嚴禁包含任何阿拉伯數字或全形數字：{}",
                awareness
            );
            assert!(
                !awareness.contains("G0") && !awareness.contains("g0"),
                "體感覺察嚴禁外洩內部群組代號 G0：{}",
                awareness
            );
            assert!(
                awareness.contains("交手發力之際") || awareness.contains("暢流"),
                "體感覺察必須包含關鍵體察字詞：{}",
                awareness
            );

            // 測試詞元造句之自動繞行體感
            let sample_words: Vec<&WordEntry> = ["wu", "zhan", "su", "pi", "dun_dong"]
                .iter()
                .filter_map(|id| registry.get_by_id(id))
                .collect();

            let note = compute_auto_detour_meridian_note(&sample_words, seed);
            assert!(
                !note.is_empty(),
                "經脈繞行描述不可為空"
            );
            assert!(
                !note.chars().any(|c| c.is_ascii_digit() || ('０'..='傷').contains(&c) && c.is_numeric()),
                "經脈繞行描述嚴禁包含任何數字：{}",
                note
            );
            assert!(
                !note.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "經脈繞行描述嚴禁包含數字：{}",
                note
            );
            assert!(
                !note.contains("G0") && !note.contains("g0"),
                "經脈繞行描述嚴禁外洩代號 G0：{}",
                note
            );

            // 驗證合成招式中包含經脈繞行資訊與 practice_feel
            let inlays = vec![
                StarInlay::new("N001", "wu"),
                StarInlay::new("N002", "zhan"),
                StarInlay::new("N003", "su"),
            ];
            let sentence = StarSentence::new(inlays)
                .expect("造句成功")
                .with_soul_seed(seed);
            let tech = sentence.synthesize(registry).expect("合成成功");
            assert_eq!(tech.meridian_detour_note, note);
            assert!(tech.practice_feel.contains(&note));
            assert!(
                !tech.practice_feel.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "演練體感嚴禁包含任何數字：{}",
                tech.practice_feel
            );
            assert!(
                !tech.practice_feel.contains("G0") && !tech.practice_feel.contains("g0"),
                "演練體感嚴禁包含代號 G0：{}",
                tech.practice_feel
            );
        }
    }

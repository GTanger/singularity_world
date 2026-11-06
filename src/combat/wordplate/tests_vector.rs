
    #[test]
    fn test_all_500_words_load_with_nonzero_12d_vectors() {
        let registry = WordRegistry::global();
        assert_eq!(registry.len(), 500, "詞元總數必須為 500");

        for word in registry.iter() {
            assert!(
                word.vector_12d.is_non_zero(),
                "詞元 [{}] {} 的 12D 向量全為零！",
                word.id,
                word.ch
            );
            // 驗證四維能量形態非負
            assert!(word.vector_12d.focus >= 0.0);
            assert!(word.vector_12d.shock >= 0.0);
            assert!(word.vector_12d.flow >= 0.0);
            assert!(word.vector_12d.sustain >= 0.0);

            // 驗證三維時序非負
            assert!(word.vector_12d.tempo_early >= 0.0);
            assert!(word.vector_12d.tempo_mid >= 0.0);
            assert!(word.vector_12d.tempo_late >= 0.0);

            // 驗證五維負載非負
            assert!(word.vector_12d.penetration >= 0.0);
            assert!(word.vector_12d.reach >= 0.0);
            assert!(word.vector_12d.vitality >= 0.0);
            assert!(word.vector_12d.qi_cost >= 0.0);
            assert!(word.vector_12d.recoil >= 0.0);
        }
    }

    #[test]
    fn test_all_360_star_nodes_inlayable_and_sentences() {
        let registry = WordRegistry::global();

        // 1. 驗證 N000 不可鑲字
        assert_eq!(
            parse_star_node("N000"),
            Err(WordPlateError::NodeNotInlayable("N000（生之樞紐為本源核心，不可鑲嵌字訣）".into()))
        );

        // 2. 驗證 N001..N360 全數合法且可解析
        for i in 1..=360 {
            let node_id = format!("N{i:03}");
            assert_eq!(parse_star_node(&node_id), Ok(i));
        }

        // 3. 驗證非法編號拒絕
        assert!(parse_star_node("N361").is_err());
        assert!(parse_star_node("N999").is_err());
        assert!(parse_star_node("X001").is_err());
        assert!(parse_star_node("").is_err());

        // 4. 驗證全 360 個星位皆可造句鑲字（以 3 言、5 言、7 言覆蓋）
        // 選取前幾顆詞元
        let words: Vec<&WordElement> = registry.ordered_elements().take(7).collect();
        assert_eq!(words.len(), 7);

        // 測試 3 言句式（N001..N003）
        let inlays_3 = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N002", &words[1].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N003", &words[2].id, OrbitFluency::PingWen),
        ];
        let sent_3 = StarSentence::new(inlays_3).expect("三言句式建立成功");
        let tech_3 = sent_3.synthesize(registry).expect("三言句式合成成功");
        assert_eq!(tech_3.length, 3);
        assert!(tech_3.vector_12d.is_non_zero());
        assert!(!tech_3.canonical_name.is_empty());

        // 測試 5 言句式（N004..N008）
        let inlays_5 = (4..=8)
            .enumerate()
            .map(|(idx, n)| {
                StarInlay::with_fluency(format!("N{n:03}"), &words[idx % words.len()].id, OrbitFluency::ShunTong)
            })
            .collect();
        let sent_5 = StarSentence::new(inlays_5).expect("五言句式建立成功");
        let tech_5 = sent_5.synthesize(registry).expect("五言句式合成成功");
        assert_eq!(tech_5.length, 5);
        assert!(tech_5.vector_12d.is_non_zero());

        // 測試 7 言句式（N009..N015）
        let inlays_7 = (9..=15)
            .enumerate()
            .map(|(idx, n)| {
                StarInlay::with_fluency(format!("N{n:03}"), &words[idx % words.len()].id, OrbitFluency::PingWen)
            })
            .collect();
        let sent_7 = StarSentence::new(inlays_7).expect("七言句式建立成功");
        let tech_7 = sent_7.synthesize(registry).expect("七言句式合成成功");
        assert_eq!(tech_7.length, 7);
        assert!(tech_7.vector_12d.is_non_zero());

        // 驗證遍歷所有 360 個節點皆可正確造句成相
        for chunk_start in (1..=360).step_by(3) {
            let chunk_end = (chunk_start + 2).min(360);
            if chunk_end - chunk_start + 1 == 3 {
                let chunk_inlays: Vec<StarInlay> = (chunk_start..=chunk_end)
                    .map(|n| StarInlay::new(format!("N{n:03}"), &words[0].id))
                    .collect();
                let sent = StarSentence::new(chunk_inlays).expect("全節點覆蓋三言造句合法");
                let synth = sent.synthesize(registry).expect("全節點合成合法");
                assert_eq!(synth.length, 3);
            }
        }

        // 驗證字數限制拒絕（4 言與 6 言非法）
        let invalid_inlays_4 = (1..=4).map(|n| StarInlay::new(format!("N{n:03}"), &words[0].id)).collect();
        assert_eq!(
            StarSentence::new(invalid_inlays_4).err(),
            Some(WordPlateError::InvalidSentenceLength(4))
        );

        // 驗證節點重複鑲嵌拒絕
        let dup_inlays = vec![
            StarInlay::new("N010", &words[0].id),
            StarInlay::new("N010", &words[1].id),
            StarInlay::new("N011", &words[2].id),
        ];
        assert_eq!(
            StarSentence::new(dup_inlays).err(),
            Some(WordPlateError::DuplicateNodeInlay("N010".into()))
        );
    }

    #[test]
    fn test_tactical_slots_and_tianji_matchup_cleanly() {
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(15).collect();

        // 甲方：起手快劍風格
        let mut loadout_a = TacticalLoadout::new();
        let sent_a_open = StarSentence::new(vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N002", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N003", &words[2].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("流星絕影劍");
        let sent_a_mid = StarSentence::new(vec![
            StarInlay::with_fluency("N004", &words[3].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N005", &words[4].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N006", &words[5].id, OrbitFluency::ShunTong),
        ]).unwrap().with_custom_name("驚雷連環擊");
        let sent_a_fin = StarSentence::new(vec![
            StarInlay::with_fluency("N007", &words[6].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N008", &words[7].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N009", &words[8].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("萬象天樞破");
        let sent_a_def = StarSentence::new(vec![
            StarInlay::with_fluency("N010", &words[9].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N011", &words[10].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N012", &words[11].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("凌波游雲步");

        loadout_a.set_slot(TacticalSlot::Opening, sent_a_open);
        loadout_a.set_slot(TacticalSlot::Midgame, sent_a_mid);
        loadout_a.set_slot(TacticalSlot::Finisher, sent_a_fin);
        loadout_a.set_slot(TacticalSlot::Defense, sent_a_def);

        // 乙方：穩健持重重甲風格
        let mut loadout_b = TacticalLoadout::new();
        let sent_b_open = StarSentence::new(vec![
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[2].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N023", &words[3].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("泰山沉岳勢");
        let sent_b_mid = StarSentence::new(vec![
            StarInlay::with_fluency("N024", &words[4].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N025", &words[5].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N026", &words[6].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("金鐘伏魔罩");
        let sent_b_fin = StarSentence::new(vec![
            StarInlay::with_fluency("N027", &words[7].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N028", &words[8].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N029", &words[9].id, OrbitFluency::ShunTong),
        ]).unwrap().with_custom_name("千鈞開山斧");
        let sent_b_def = StarSentence::new(vec![
            StarInlay::with_fluency("N030", &words[10].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N031", &words[11].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N032", &words[12].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("鐵壁無縫身");

        loadout_b.set_slot(TacticalSlot::Opening, sent_b_open);
        loadout_b.set_slot(TacticalSlot::Midgame, sent_b_mid);
        loadout_b.set_slot(TacticalSlot::Finisher, sent_b_fin);
        loadout_b.set_slot(TacticalSlot::Defense, sent_b_def);

        let report = resolve_tianji_matchup(&loadout_a, 100, &loadout_b, 100, registry).expect("田忌賽馬對決順暢無誤");

        // 驗證三階段完整
        assert_eq!(report.rounds.len(), 3);
        assert_eq!(report.rounds[0].slot, TacticalSlot::Opening);
        assert_eq!(report.rounds[1].slot, TacticalSlot::Midgame);
        assert_eq!(report.rounds[2].slot, TacticalSlot::Finisher);

        // 驗證零自殺判定：受傷者絕非自殘，氣血扣除受控
        assert!(report.side_a_final_hp >= 0);
        assert!(report.side_b_final_hp >= 0);

        // 驗證定名正確帶入
        assert_eq!(report.rounds[0].a_technique_name, "流星絕影劍");
        assert_eq!(report.rounds[0].b_technique_name, "泰山沉岳勢");

        // 驗證勝負有明確判定
        assert!(["side_a", "side_b", "draw"].contains(&report.winner.as_str()));
    }

    #[test]
    fn test_word_casket_operations() {
        let mut casket = WordCasket::new();
        assert_eq!(casket.total_count(), 0);
        assert_eq!(casket.total_distinct(), 0);

        casket.add("wu", 3);
        casket.add("zhan", 1);
        assert_eq!(casket.count("wu"), 3);
        assert_eq!(casket.count("zhan"), 1);
        assert_eq!(casket.total_count(), 4);
        assert_eq!(casket.total_distinct(), 2);
        assert!(casket.has("wu"));

        // 移除一部分
        assert!(casket.remove("wu", 2).is_ok());
        assert_eq!(casket.count("wu"), 1);
        assert_eq!(casket.total_count(), 2);

        // 溢出移除應報錯
        assert_eq!(
            casket.remove("wu", 5),
            Err(WordPlateError::InsufficientWordCount("wu".into(), 1, 5))
        );

        // 扣光時完全移除
        assert!(casket.remove("wu", 1).is_ok());
        assert_eq!(casket.count("wu"), 0);
        assert!(!casket.has("wu"));
        assert_eq!(casket.total_distinct(), 1);
    }

    #[test]
    fn test_positional_word_order_breaks_commutativity() {
        let registry = WordRegistry::global();

        // 使用三顆不同的詞元："su", "zhan", "qiang"
        assert!(registry.get_by_id("su").is_some());
        assert!(registry.get_by_id("zhan").is_some());
        assert!(registry.get_by_id("qiang").is_some());

        // 正序造句：[su, zhan, qiang]
        let inlays_abc = vec![
            StarInlay::new("N001", "su"),
            StarInlay::new("N002", "zhan"),
            StarInlay::new("N003", "qiang"),
        ];
        let sent_abc = StarSentence::new(inlays_abc).expect("正序三言句式建立成功");
        let tech_abc = sent_abc.synthesize(registry).expect("正序三言句式合成成功");

        // 逆序造句：[qiang, zhan, su]
        let inlays_cba = vec![
            StarInlay::new("N001", "qiang"),
            StarInlay::new("N002", "zhan"),
            StarInlay::new("N003", "su"),
        ];
        let sent_cba = StarSentence::new(inlays_cba).expect("逆序三言句式建立成功");
        let tech_cba = sent_cba.synthesize(registry).expect("逆序三言句式合成成功");

        // 斷言起承轉合語序權重打破交換律：[A, B, C] != [C, B, A]
        assert_ne!(
            tech_abc.vector_12d.tempo_early,
            tech_cba.vector_12d.tempo_early,
            "起手先發時序受字序影響不相等"
        );
        assert_ne!(
            tech_abc.vector_12d.shock,
            tech_cba.vector_12d.shock,
            "震盪能量受字序影響不相等"
        );
    }

    #[test]
    fn test_semantic_contrast_and_dual_context_hundred_crafts() {
        let registry = WordRegistry::global();

        let w_qiang = registry.get_by_id("qiang").expect("強 詞元存在");
        let w_ruo = registry.get_by_id("ruo").expect("弱 詞元存在");
        let w_su = registry.get_by_id("su").expect("速 詞元存在");
        let w_huan = registry.get_by_id("huan").expect("緩 詞元存在");

        // 驗證語意對比：強的 shock 嚴格大於 弱，速的 tempo_early 嚴格大於 緩
        assert!(
            w_qiang.vector_12d.shock > w_ruo.vector_12d.shock,
            "強 之震盪衝擊 ({}) 應嚴格大於 弱 ({})",
            w_qiang.vector_12d.shock,
            w_ruo.vector_12d.shock
        );
        assert!(
            w_su.vector_12d.tempo_early > w_huan.vector_12d.tempo_early,
            "速 之起手先發 ({}) 應嚴格大於 緩 ({})",
            w_su.vector_12d.tempo_early,
            w_huan.vector_12d.tempo_early
        );

        // 以「鑄」「熾」「器」造句，驗證主修百工為鍛鑄百工且一字雙境描述完備
        assert!(registry.get_by_id("zhu").is_some());
        assert!(registry.get_by_id("chi").is_some());
        assert!(registry.get_by_id("qi_shou").is_some());

        let inlays = vec![
            StarInlay::new("N001", "zhu"),
            StarInlay::new("N002", "chi"),
            StarInlay::new("N003", "qi_shou"),
        ];
        let sent = StarSentence::new(inlays).expect("鑄熾器三言造句成功");
        let tech = sent.synthesize(registry).expect("鑄熾器合成成功");

        assert_eq!(
            tech.primary_craft,
            CraftAspect::Forge,
            "鑄熾器組合之主修藝相應為鍛鑄百工"
        );
        assert!(
            tech.craft_dims.is_non_zero(),
            "鍛鑄百工向量非零"
        );

        let summary = tech.dual_context_summary();
        assert!(!summary.is_empty(), "一字雙境摘要不可為空");
        assert!(summary.contains("【交鋒武相】"), "摘要應包含交鋒武相");
        assert!(summary.contains("【百工藝相】"), "摘要應包含百工藝相");
        assert!(summary.contains("鍛鑄百工"), "摘要應提及鍛鑄百工");
    }

    #[test]
    fn test_five_sources_coverage_and_roll_word_from_source() {
        let registry = WordRegistry::global();

        let beast = registry.words_by_source(WordSource::Beast);
        let plant = registry.words_by_source(WordSource::Plant);
        let mineral = registry.words_by_source(WordSource::Mineral);
        let water = registry.words_by_source(WordSource::Water);
        let qi = registry.words_by_source(WordSource::Qi);

        assert_eq!(beast.len(), 193, "獸源詞元應為 193 個");
        assert_eq!(plant.len(), 153, "植源詞元應為 153 個");
        assert_eq!(mineral.len(), 205, "礦源詞元應為 205 個");
        assert_eq!(water.len(), 161, "水源詞元應為 161 個");
        assert_eq!(qi.len(), 212, "氣源詞元應為 212 個");

        // 驗證抽取結果均包含對應源質標籤
        for (source, expected_tag) in [
            (WordSource::Beast, "獸"),
            (WordSource::Plant, "植"),
            (WordSource::Mineral, "礦"),
            (WordSource::Water, "水"),
            (WordSource::Qi, "氣"),
        ] {
            let elem = registry.roll_word_from_source(source, WordQuality::Bing, 42);
            assert!(
                elem.sources.iter().any(|s| s == expected_tag),
                "抽出的詞元 {:?} 應包含源質標籤 {}",
                elem.id,
                expected_tag
            );
        }
    }

    #[test]
    fn test_crystal_id_and_parsing_and_strip_ratio() {
        let id = crystal_item_id(WordSource::Beast, WordQuality::Jia);
        assert_eq!(id, "crystal_beast_jia");
        let parsed = parse_crystal_item_id(&id);
        assert_eq!(parsed, Some((WordSource::Beast, WordQuality::Jia)));

        let q = roll_strip_quality(0.5, 2, true, 12345);
        assert!(matches!(q, WordQuality::Bing | WordQuality::Yi | WordQuality::Jia));

        let ratio_civil = compute_extraction_ratio(0.8, false);
        assert!((3..=9).contains(&ratio_civil));
        let ratio_official = compute_extraction_ratio(0.2, true);
        assert!((7..=9).contains(&ratio_official));
    }

    #[test]
    fn test_casket_stack_refine_and_mint_to_magnesium() {
        let mut casket = WordCasket::new();

        // 未擁有時鎔鑄失敗
        assert!(casket.mint_to_magnesium("huo").is_err());

        // 存入 3 枚「火」
        casket.add_extracted("huo", 3, WordQuality::Bing, 7);
        assert_eq!(casket.count("huo"), 3);
        assert_eq!(casket.quality_of("huo"), WordQuality::Bing);
        assert_eq!(casket.ratio_of("huo"), 7);
        assert_eq!(casket.charge_of("huo"), 0);

        // 鎔鑄化鎂 1 枚
        let (mag_val, q, narrative) = casket.mint_to_magnesium("huo").expect("鎔鑄成功");
        assert_eq!(mag_val, 12);
        assert_eq!(q, WordQuality::Bing);
        assert!(narrative.contains("12 鎂"));
        assert_eq!(casket.count("huo"), 2);

        // 同名疊煉（目前有 2 枚，足夠消耗 1 枚）
        let (new_q, new_charge, _ascended, _refine_narrative) =
            casket.stack_refine("huo", 0.1, 7777).expect("疊煉成功");
        assert_eq!(casket.count("huo"), 1);
        assert!(new_charge > 0 || matches!(new_q, WordQuality::Yi | WordQuality::Jia));

        // 只剩 1 枚時再次疊煉應回傳不足錯誤
        let err = casket.stack_refine("huo", 0.1, 8888);
        assert!(matches!(err, Err(WordPlateError::InsufficientWordCount(_, 1, 2))));

        // 鎔鑄最後 1 枚，確認完全清理
        casket.mint_to_magnesium("huo").expect("最後一枚鎔鑄成功");
        assert_eq!(casket.count("huo"), 0);
        assert_eq!(casket.quality_of("huo"), WordQuality::Bing);
        assert_eq!(casket.charge_of("huo"), 0);
        assert_eq!(casket.ratio_of("huo"), 0);
    }

    #[test]
    fn test_tactical_loadout_craft_dims_and_xuan_luck_bonus() {
        let registry = WordRegistry::global();
        let mut casket = WordCasket::new();

        // 初始玄運加成 0
        let mut loadout = TacticalLoadout::new();
        assert_eq!(loadout.xuan_luck_bonus(&casket, registry), 0.0);

        // 放入「幸」「運」兩種玄意詞元
        casket.add("xing_xuan", 2);
        casket.add("yun", 1);
        let bonus = loadout.xuan_luck_bonus(&casket, registry);
        assert!((bonus - 0.05).abs() < 1e-4);

        // 配置開首招式
        let inlays = vec![
            StarInlay::new("N001", "zhu"),
            StarInlay::new("N002", "chi"),
            StarInlay::new("N003", "qi_shou"),
        ];
        let sent = StarSentence::new(inlays).expect("造句成功");
        loadout.set_slot(TacticalSlot::Opening, sent);

        let total_craft = loadout.total_craft_dims(registry);
        assert!(total_craft.forge > 0.0, "開首招式應有鍛鑄維度貢獻");
    }

    #[test]
    fn test_inscribe_item_capacity_and_returns() {
        let registry = WordRegistry::global();
        let mut casket = WordCasket::new();

        // 獲取前 4 個詞元 id
        let words: Vec<String> = registry.ordered_elements().take(4).map(|e| e.id.clone()).collect();
        assert_eq!(words.len(), 4);

        // 給予詞匣各 1 枚
        for w in &words {
            casket.add(w, 1);
        }

        // 銘刻前 3 字
        let res1 = casket.inscribe_item("item_blade_01", &words[0]);
        assert!(res1.is_ok());
        assert_eq!(casket.count(&words[0]), 0);

        let res2 = casket.inscribe_item("item_blade_01", &words[1]);
        assert!(res2.is_ok());
        assert_eq!(casket.count(&words[1]), 0);

        let res3 = casket.inscribe_item("item_blade_01", &words[2]);
        assert!(res3.is_ok());
        assert_eq!(casket.count(&words[2]), 0);
        assert_eq!(casket.inscribed_words_of("item_blade_01"), &[words[0].clone(), words[1].clone(), words[2].clone()]);

        // 銘刻第 4 字：最舊的 words[0] 必須被擠出並返還詞匣！
        let res4 = casket.inscribe_item("item_blade_01", &words[3]);
        assert!(res4.is_ok());
        assert_eq!(casket.count(&words[0]), 1, "最舊詞元應被歸還至詞匣");
        assert_eq!(casket.count(&words[3]), 0);
        assert_eq!(
            casket.inscribed_words_of("item_blade_01"),
            &[words[1].clone(), words[2].clone(), words[3].clone()]
        );

        // 銘文後綴展示
        let suffix = casket.format_inscribed_suffix("item_blade_01", registry);
        assert!(suffix.starts_with(" ["));
        assert!(suffix.ends_with(']'));

        // 銘文百工與戰鬥增益
        let craft_bonus = casket.inscribed_craft_bonus("item_blade_01", registry);
        assert!(craft_bonus.is_non_zero());
        let (atk, pen, def) = casket.inscribed_combat_bonus("item_blade_01", registry);
        assert!(atk > 0.0 || pen > 0.0 || def > 0.0);
    }

    #[test]
    fn test_infer_item_carrier_affinity_tools_as_weapons() {
        // 刀劍鋒芒：菜刀、柴刀
        assert_eq!(
            infer_item_carrier_affinity("tool_kitchen_knife", "圓弧雙柄削皮彎刀"),
            Some(CarrierAffinity::Blade)
        );
        assert_eq!(
            infer_item_carrier_affinity("woodcutting_machete", "精鋼柴刀"),
            Some(CarrierAffinity::Blade)
        );

        // 重錘鈍器：木棒/球棒、十字鎬、撬棍
        assert_eq!(
            infer_item_carrier_affinity("wooden_club", "結實木棒"),
            Some(CarrierAffinity::Blunt)
        );
        assert_eq!(
            infer_item_carrier_affinity("mining_pick", "青銅鶴嘴鎬"),
            Some(CarrierAffinity::Blunt)
        );
        assert_eq!(
            infer_item_carrier_affinity("iron_crowbar", "鍛鐵撬棍"),
            Some(CarrierAffinity::Blunt)
        );

        // 堅盾重禦：油紙傘、鍋蓋、木砧板
        assert_eq!(
            infer_item_carrier_affinity("rain_umbrella", "青竹油紙傘"),
            Some(CarrierAffinity::Shield)
        );
        assert_eq!(
            infer_item_carrier_affinity("pot_lid", "生鐵鍋蓋"),
            Some(CarrierAffinity::Shield)
        );

        // 長槍大戟：鐵鏟、長柄鋤頭
        assert_eq!(
            infer_item_carrier_affinity("farm_shovel", "掘地鐵鏟"),
            Some(CarrierAffinity::Pole)
        );

        // 步法身形：草鞋
        assert_eq!(
            infer_item_carrier_affinity("woven_shoes", "行路草鞋"),
            Some(CarrierAffinity::Footwork)
        );

        // 徒手拳掌：皮手套
        assert_eq!(
            infer_item_carrier_affinity("leather_gloves", "粗皮護腕手套"),
            Some(CarrierAffinity::Unarmed)
        );
    }

    #[test]
    fn test_ten_soul_seeds_produce_distinct_loadouts() {
        let registry = WordRegistry::global();
        let seeds = [
            1001i64, 2002i64, 3003i64, 4004i64, 5005i64,
            6006i64, 7007i64, 8008i64, 9009i64, 10010i64,
        ];

        let mut distinct_openings = HashSet::new();
        let mut distinct_names = HashSet::new();

        for &s in &seeds {
            let loadout = build_loadout_from_soul_seed(s, None, registry);
            assert!(loadout.opening.is_some(), "起手式不可為空");
            assert!(loadout.midgame.is_some(), "中盤式不可為空");
            assert!(loadout.finisher.is_some(), "終末式不可為空");
            assert!(loadout.defense.is_some(), "守勢不可為空");

            let op = loadout.opening.as_ref().unwrap();
            let word_seq: Vec<String> = op.inlays.iter().map(|i| i.word_id.clone()).collect();
            distinct_openings.insert(word_seq);

            if let Some(name) = &op.custom_name {
                distinct_names.insert(name.clone());
            }
        }

        assert_eq!(distinct_openings.len(), 10, "10 種不同的靈魂種子必須產出 10 種互異的起手式詞譜");
        assert_eq!(distinct_names.len(), 10, "10 種不同的靈魂種子必須產出 10 種互異的動態招式名號");
    }

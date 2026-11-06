#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_weapon_affinity_keywords() {
        assert_eq!(
            classify_weapon_affinity("青鋼長劍", "三尺青鋒", "weapon"),
            CarrierAffinity::Blade
        );
        assert_eq!(
            classify_weapon_affinity("點鋼槍", "長丈二", "weapon"),
            CarrierAffinity::Pole
        );
        assert_eq!(
            classify_weapon_affinity("八角重錘", "沉重無匹", "weapon"),
            CarrierAffinity::Blunt
        );
        assert_eq!(
            classify_weapon_affinity("精鐵大盾", "厚實重禦", "shield"),
            CarrierAffinity::Shield
        );
        assert_eq!(
            classify_weapon_affinity("草鞋", "普通草鞋", "armor"),
            CarrierAffinity::Unarmed
        );
    }

    #[test]
    fn test_dequantified_prose_contains_no_digits() {
        let rounds = vec![
            crate::combat::wordplate::PhaseClashRecord {
                phase: 1,
                slot: TacticalSlot::Opening,
                a_technique_name: "疾刺斬".into(),
                b_technique_name: "震天裂石勢".into(),
                a_initiative_score: 1.5,
                b_initiative_score: 1.0,
                damage_to_a: 0,
                damage_to_b: 10,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: Some("專克拆招・以巧破剛：疾速巧勁穿透重擊硬直破防！".into()),
                ..Default::default()
            },
            crate::combat::wordplate::PhaseClashRecord {
                phase: 2,
                slot: TacticalSlot::Midgame,
                a_technique_name: "流破斬".into(),
                b_technique_name: "摧山覆岳擊".into(),
                a_initiative_score: 2.0,
                b_initiative_score: 1.2,
                damage_to_a: 0,
                damage_to_b: 15,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: None,
                ..Default::default()
            },
            crate::combat::wordplate::PhaseClashRecord {
                phase: 3,
                slot: TacticalSlot::Finisher,
                a_technique_name: "吾震破".into(),
                b_technique_name: "九霄神殛破".into(),
                a_initiative_score: 2.5,
                b_initiative_score: 1.5,
                damage_to_a: 0,
                damage_to_b: 25,
                narrative: "此處不管原字串".into(),
                a_manifested_as_skill: true,
                b_manifested_as_skill: false,
                counter_reason: None,
                ..Default::default()
            },
        ];

        let prose = render_dequantified_combat_prose(
            "hero_1",
            "npc_blacksmith",
            0,
            "你",
            "王鐵匠",
            true,
            "attacker",
            &rounds,
            Some("手握【青鋼長劍】，刀劍鋒芒與招式吞吐相通，森寒劍意直逼肺腑！"),
            Some(123456789),
            "青鋼長劍",
            "鍛造鐵錘",
            CarrierAffinity::Blade,
            CarrierAffinity::Blunt,
        );

        // 嚴格校驗：向玩家輸出的感官散文絕無任何量化阿拉伯數字
        for ch in prose.chars() {
            assert!(
                !ch.is_ascii_digit(),
                "戰鬥散文不得包含任何阿拉伯數字：發現 '{ch}' 在散文中：\n{prose}"
            );
        }
        assert!(prose.contains("「"));
        assert!(prose.contains("」"));
    }

    #[test]
    fn test_execute_tianji_wordplate_combat_with_inscribed_tool() {
        let registry = WordRegistry::global();
        let w1 = registry.ordered_elements().next().unwrap().id.clone();
        let w2 = registry.ordered_elements().nth(1).unwrap().id.clone();
        let mut casket = WordCasket::new();
        // 銘刻削皮彎刀
        casket.add(&w1, 5);
        casket.add(&w2, 5);
        let _ = casket.inscribe_item("curved_peeling_knife", &w1);
        let _ = casket.inscribe_item("curved_peeling_knife", &w2);
        crate::server::handler::set_player_word_casket("test_hero_combat", casket);

        let attacker: Character = serde_json::from_str(
            r#"{"id":"test_hero_combat","kind":"player","display_title":"名刀俠客","vit":120,"dex":20,"qi":15,"equipment_slots":"{\"hold_r\":\"curved_peeling_knife\"}"}"#,
        ).unwrap();

        let defender: Character = serde_json::from_str(
            r#"{"id":"npc_target_combat","kind":"npc","display_title":"黑風山寇","vit":90,"dex":12,"qi":8,"soul_seed":987654321}"#,
        ).unwrap();

        let (prose, winner, final_a, final_d) =
            execute_tianji_wordplate_combat("test_hero_combat", &attacker, &defender, true);

        assert!(!prose.is_empty());
        assert!(final_a >= 1);
        assert!(final_d >= 1);
        assert!(winner == "attacker" || winner == "defender" || winner == "draw");
        for ch in prose.chars() {
            assert!(!ch.is_ascii_digit(), "Prose must not contain digits: {ch}");
        }
        assert!(prose.contains("「"));
    }

    #[test]
    fn test_wild_beast_shanhai_bamboo_partridge_anatomy_and_mass_physics() {
        let attacker: Character = serde_json::from_str(
            r#"{"id":"test_player_blackbone","kind":"player","display_title":"黑骨","vit":10,"dex":10,"qi":10,"equipment_slots":"{}"}"#,
        )
        .unwrap();

        let defender: Character = serde_json::from_str(
            r#"{"id":"shanhai_10_40_-35_12","kind":"npc","display_title":"山林竹雞","vit":18,"dex":28,"qi":12}"#,
        )
        .unwrap();

        let (prose, winner, _final_a, _final_d) =
            execute_tianji_wordplate_combat("test_player_blackbone", &attacker, &defender, true);

        assert_eq!(winner, "attacker", "成人武者(BASE_STAT=10)對決山林竹雞應憑量級與氣血勝出，戰報：\n{prose}");

        let muntjac: Character = serde_json::from_str(
            r#"{"id":"ambient_-24_1","kind":"npc","display_title":"林道山羌","vit":30,"dex":20,"qi":15}"#,
        )
        .unwrap();
        let (prose_m, winner_m, _, _) =
            execute_tianji_wordplate_combat("test_player_blackbone", &attacker, &muntjac, true);
        assert_eq!(winner_m, "attacker", "成人武者(BASE_STAT=10)對決林道山羌應憑量級與氣血勝出，戰報：\n{prose_m}");
        assert!(
            !prose.contains("山林竹雞以徒手運勁"),
            "野禽不得出現『以徒手運勁』：\n{prose}"
        );
        assert!(
            !prose.contains("流破斬") && !prose.contains("疾刺斬") && !prose.contains("吾震破"),
            "不得出現露西時代三字硬編碼招式名：\n{prose}"
        );
        assert!(
            !prose.contains("對方身法沉穩老練") && !prose.contains("點頭致意收手"),
            "野獸對決不得出現武林同道切磋客套話：\n{prose}"
        );
        assert!(
            prose.contains("振翅") || prose.contains("撲") || prose.contains("啄") || prose.contains("生擒制伏"),
            "野禽戰報應呈現飛禽解剖動作或生擒敘事：\n{prose}"
        );
    }

    #[test]
    fn test_universal_resident_signature_vessel_and_inscribed_combat_bonus() {
        let registry = WordRegistry::global();
        let entity_id = "outer_household_0001";
        let display_title = "當值石磨匠師";
        let equipment_slots = "";

        let casket = get_or_init_entity_combat_word_casket(entity_id, display_title);
        assert!(
            !casket.item_inscriptions.is_empty(),
            "住戶匠人詞匣初始化後應具備器物銘文刻印"
        );

        let (bonus_atk, bonus_pen, bonus_def) = calculate_entity_inscribed_combat_bonus(
            entity_id,
            display_title,
            equipment_slots,
            &casket,
            registry,
        );
        assert!(
            bonus_atk > 0.0 || bonus_pen > 0.0 || bonus_def > 0.0,
            "住戶即便未穿戴外部裝備，其隨身器物銘文亦應提供正值戰鬥加成：({bonus_atk}, {bonus_pen}, {bonus_def})"
        );

        let (affinity, weapon_name_opt) = resolve_entity_weapon_affinity_with_inscriptions(
            entity_id,
            display_title,
            equipment_slots,
            &casket,
            registry,
        );
        assert_ne!(
            affinity,
            CarrierAffinity::Unarmed,
            "當值石磨匠師隨身器物（篩架/石磨工具）應具備專屬載體親和度"
        );
        assert!(
            weapon_name_opt.is_some(),
            "住戶應成功顯化其專屬隨身器物名稱"
        );
        let weapon_name = weapon_name_opt.unwrap();
        assert!(
            weapon_name.contains("石磨") || weapon_name.contains("篩架"),
            "隨身器物名稱應相符，實際為：{weapon_name}"
        );
        assert!(
            weapon_name.contains('[') && weapon_name.contains(']'),
            "隨身器物應具備銘文刻字後綴，實際為：{weapon_name}"
        );
    }
}

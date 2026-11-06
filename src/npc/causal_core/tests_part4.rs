    #[test]
    fn test_plural_personality_weights_and_cornerstone_interaction() {
        // 1. 驗證先天單純形總和恆等於 600 且各向在 45..=165
        let w_neutral = compute_plural_personality_weights("路人", 12345, 20, 10);
        let innate = innate_personality_simplex(12345);
        assert_eq!(innate.iter().sum::<i32>(), 600);
        for &v in &innate {
            assert!((45..=165).contains(&v));
        }

        // 2. 驗證職業偏置
        let w_craftsman = compute_plural_personality_weights("鐵工坊鐵匠", 12345, 20, 10);
        assert!(w_craftsman.craft > w_neutral.craft);

        let w_merchant = compute_plural_personality_weights("商行主事", 12345, 20, 10);
        assert!(w_merchant.profit > w_neutral.profit);

        let w_guard = compute_plural_personality_weights("城哨巡守", 12345, 20, 10);
        assert!(w_guard.honor > w_neutral.honor);

        let w_family = compute_plural_personality_weights("持家內當家", 12345, 20, 10);
        assert!(w_family.care > w_neutral.care);

        // 3. 驗證情境調節
        let w_stressed = compute_plural_personality_weights("路人", 12345, 75, 10);
        assert!(w_stressed.pride > w_neutral.pride);
        assert!(w_stressed.care > w_neutral.care);
        assert!(w_stressed.profit > w_neutral.profit);

        let w_poor = compute_plural_personality_weights("路人", 12345, 20, 2);
        assert!(w_poor.profit > w_neutral.profit);
        assert!(w_poor.care > w_neutral.care);

        let w_wealthy = compute_plural_personality_weights("路人", 12345, 20, 30);
        assert!(w_wealthy.honor > w_neutral.honor);
        assert!(w_wealthy.craft > w_neutral.craft);
        assert!(w_wealthy.pride > w_neutral.pride);

        // 4. 驗證 weight_for 提取
        assert_eq!(w_craftsman.weight_for(Cornerstone::Craft), w_craftsman.craft);
        assert_eq!(w_merchant.weight_for(Cornerstone::Profit), w_merchant.profit);
        assert_eq!(w_guard.weight_for(Cornerstone::Honor), w_guard.honor);
    }

    #[test]
    fn test_innate_personality_simplex_conservation() {
        for seed in 0..200 {
            let simplex = innate_personality_simplex(seed);
            let sum: i32 = simplex.iter().sum();
            assert_eq!(sum, 600, "Seed {seed} sum was {sum}, expected 600");
            for (idx, &v) in simplex.iter().enumerate() {
                assert!(
                    (45..=165).contains(&v),
                    "Seed {seed} facet {idx} was {v}, expected in 45..=165"
                );
            }
        }
    }

    #[test]
    fn test_situational_if_then_offset_zero_sum() {
        let slots = [
            PsychSituationSlot::ScarcitySurvival,
            PsychSituationSlot::VocationalDuty,
            PsychSituationSlot::RelationalEncounter,
            PsychSituationSlot::HighStressCrisis,
        ];
        for seed in 0..100 {
            for slot in slots {
                let off = situational_if_then_offset(seed, slot);
                let sum: i32 = off.iter().sum();
                assert_eq!(sum, 0, "Seed {seed}, slot {:?} sum was {sum}, expected 0", slot);
                for &v in &off {
                    assert!((-32..=32).contains(&v), "Offset {v} out of bounds [-32, 32]");
                }
            }
        }
    }

    #[test]
    fn test_occupation_cornerstone_diversity_no_stereotypes() {
        use std::collections::HashSet;
        let mut merchant_cornerstones = HashSet::new();
        let mut smith_cornerstones = HashSet::new();
        for seed in 0..100 {
            merchant_cornerstones.insert(Cornerstone::from_role_and_seed("行商", seed));
            smith_cornerstones.insert(Cornerstone::from_role_and_seed("鐵匠", seed));
        }
        assert!(
            merchant_cornerstones.len() >= 4,
            "Expected at least 4 merchant cornerstone variants, got {:?}",
            merchant_cornerstones
        );
        assert!(merchant_cornerstones.contains(&Cornerstone::Honor), "Should include Honor merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Care), "Should include Care merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Pride), "Should include Pride merchant");
        assert!(merchant_cornerstones.contains(&Cornerstone::Profit), "Should include Profit merchant");
        assert!(smith_cornerstones.len() >= 4, "Expected diverse smith cornerstones, got {:?}", smith_cornerstones);
    }

    #[test]
    fn test_experiential_personality_shift_scars_and_gratitude() {
        let seed = 42u64;
        let hook_gratitude = RelationalHook {
            kind_label: "守諾償清".to_string(),
            target_name: "林掌櫃".to_string(),
            summary: "當面守諾償清積欠銀錢".to_string(),
            intensity: 80,
            target_id: "npc_lin".to_string(),
            updated_at_unix: 1720500000,
        };
        let hook_grudge = RelationalHook {
            kind_label: "求借遭拒".to_string(),
            target_name: "趙管事".to_string(),
            summary: "向其借貸求助卻求借遭拒".to_string(),
            intensity: 80,
            target_id: "npc_zhao".to_string(),
            updated_at_unix: 1720500000,
        };

        let w_gratitude = compute_plural_personality_weights_with_experience(
            "居民", "文曜", seed, 20, 10, 1, &[hook_gratitude], None,
        );
        let w_grudge = compute_plural_personality_weights_with_experience(
            "居民", "文曜", seed, 20, 10, 1, &[hook_grudge], None,
        );

        assert!(
            w_gratitude.honor > w_grudge.honor,
            "Gratitude experience should elevate Honor (got gratitude={}, grudge={})",
            w_gratitude.honor, w_grudge.honor
        );
        assert!(
            w_grudge.profit > w_gratitude.profit || w_grudge.pride > w_gratitude.pride,
            "Grudge/refusal experience should elevate Profit or Pride"
        );
    }

    #[test]
    fn test_dyadic_kotodama_loan_and_repayment_chain() {
        let now = 1720500000;
        let a_id = "test_npc_kotodama_borrower";
        let a_name = "沈玉";
        let a_role = "樵夫";
        let b_id = "test_npc_kotodama_lender";
        let b_name = "林員外";
        let b_role = "富商";
        let room = "文曜・市集石坪";

        // 第一階段：匱乏與充裕碰撞 -> 觸發借貸 (lend_magnesium)
        let plan_loan = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 2, None,
            b_id, b_name, b_role, 20, None,
            room, 20, 15, now,
        ).expect("Borrower in scarcity and lender in surplus with high sentiment should trigger loan plan");

        assert_eq!(plan_loan.plan_code, "lend_magnesium");
        assert_eq!(plan_loan.mg_delta_a, 4);
        assert!(plan_loan.keep_thread_open);
        assert_eq!(plan_loan.open_obligation_a, Some(("repay_debt".to_string(), b_id.to_string())));

        // 執行借貸方案
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_loan, now);

        // 驗證借貸義務已開立
        let a_ob = crate::store::get_active_npc_obligation(a_id);
        assert_eq!(a_ob, Some(("repay_debt".to_string(), b_id.to_string())));

        // 驗證人情鉤子
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A should have hook toward B");
        assert_eq!(hook_a.kind_label, "立契借貸");
        let hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B should have hook toward A");
        assert_eq!(hook_b.kind_label, "解囊借貸");

        // 驗證 A 的下步意圖
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let st_a = states.get(a_id).expect("A state must exist");
            assert!(st_a.plan_steps[0].contains("四枚鎂幣"));
        }

        // 第二階段：A 籌措到足額鎂幣 (a_mg = 6 >= 4) 再度遭遇 B -> 觸發結清 (settle_debt)
        let plan_settle = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 6, a_ob,
            b_id, b_name, b_role, 16, None,
            room, 25, 20, now + 600,
        ).expect("Repaying debt when magnesium >= 4 should trigger settle_debt");

        assert_eq!(plan_settle.plan_code, "settle_debt");
        assert_eq!(plan_settle.mg_delta_a, -4);
        assert!(!plan_settle.keep_thread_open);
        assert_eq!(plan_settle.clear_obligation_a, Some("repay_debt".to_string()));

        // 執行結清方案
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_settle, now + 600);

        // 驗證義務已解除
        assert!(crate::store::get_active_npc_obligation(a_id).is_none());

        // 驗證關係鉤子更新為守諾償清
        let updated_hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(updated_hook_a.kind_label, "守諾償清");
        let updated_hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B hook must exist");
        assert_eq!(updated_hook_b.kind_label, "守諾償清");
    }

    #[test]
    fn test_dyadic_kotodama_loan_and_default_extension_chain() {
        let now = 1720600000;
        let a_id = "test_npc_defaulter_a";
        let a_name = "阿七";
        let a_role = "散工";
        let b_id = "test_npc_creditor_b";
        let b_name = "周巡長";
        let b_role = "城哨巡守";
        let room = "南哨・望樓步道";

        // 第一階段：借貸
        let plan_loan = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 1, None,
            b_id, b_name, b_role, 15, None,
            room, 10, 5, now,
        ).expect("Scarcity vs surplus loan trigger");
        assert_eq!(plan_loan.plan_code, "lend_magnesium");
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_loan, now);

        let a_ob = crate::store::get_active_npc_obligation(a_id);
        assert_eq!(a_ob, Some(("repay_debt".to_string(), b_id.to_string())));

        // 第二階段：A 依然匱乏 (a_mg = 1 < 4)，再度相遇 -> 觸發債務展期 (default_extend_debt)
        let plan_extend = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 1, a_ob,
            b_id, b_name, b_role, 11, None,
            room, 15, 5, now + 300,
        ).expect("Default when magnesium < 4 should trigger default_extend_debt");

        assert_eq!(plan_extend.plan_code, "default_extend_debt");
        assert_eq!(plan_extend.mg_delta_a, 0);
        assert!(plan_extend.keep_thread_open);

        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_extend, now + 300);

        // 驗證義務未被清除
        assert_eq!(crate::store::get_active_npc_obligation(a_id), Some(("repay_debt".to_string(), b_id.to_string())));

        // 驗證關係鉤子更新為展期
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(hook_a.kind_label, "展期欠債");

        // 驗證 A 推進之意圖包含償還欠款
        {
            let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
            let st_a = states.get(a_id).expect("A state must exist");
            assert!(st_a.plan_steps[0].contains("周巡長之欠款"));
        }
    }

    #[test]
    fn test_dyadic_kotodama_subcontract_task_obligation() {
        let now = 1720700000;
        let a_id = "test_npc_subcontractor_a";
        let a_name = "老張";
        let a_role = "獵戶";
        let b_id = "test_npc_worker_b";
        let b_name = "小李";
        let b_role = "鐵工坊學徒";
        let room = "文曜・百鍛鐵工坊";

        // A 手頭有 repair_tool 義務，且身懷 6 鎂幣，B 手頭清空
        let a_ob = Some(("repair_tool".to_string(), "開山短斧".to_string()));
        let plan_sub = plan_dyadic_kotodama_encounter(
            a_id, a_name, a_role, 6, a_ob.clone(),
            b_id, b_name, b_role, 5, None,
            room, 15, 10, now,
        ).expect("Task subcontracting plan should trigger");

        assert_eq!(plan_sub.plan_code, "subcontract_obligation");
        assert_eq!(plan_sub.mg_delta_a, -2);
        assert!(plan_sub.keep_thread_open);
        assert_eq!(plan_sub.clear_obligation_a, Some("repair_tool".to_string()));
        assert_eq!(plan_sub.open_obligation_b, Some(("repair_tool".to_string(), "開山短斧".to_string())));

        // 預先掛載 A 的義務至 store
        crate::store::open_active_npc_obligation(a_id, "repair_tool", "開山短斧");

        // 執行託付
        commit_dyadic_kotodama_plan(a_id, a_name, b_id, b_name, &plan_sub, now);

        // 驗證 A 的義務已轉移清除，B 承接新義務
        assert!(crate::store::get_active_npc_obligation(a_id).is_none());
        assert_eq!(
            crate::store::get_active_npc_obligation(b_id),
            Some(("repair_tool".to_string(), "開山短斧".to_string()))
        );

        // 驗證鉤子
        let hook_a = find_relational_hook_for_target(a_id, b_id, b_name).expect("A hook must exist");
        assert_eq!(hook_a.kind_label, "出資託付");
        let hook_b = find_relational_hook_for_target(b_id, a_id, a_name).expect("B hook must exist");
        assert_eq!(hook_b.kind_label, "受託代辦");
    }

    #[test]
    fn test_round50_resident_names_and_catalog_room_tags() {
        assert_eq!(
            crate::store::resolve_npc_person_name("秦水生", "老堰頭水輪石磨首座|秦水生", Some("M")),
            "秦水生"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("陳萬福", "三水食堂當家大廚|陳萬福", Some("M")),
            "陳萬福"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("蘇錦心", "織造服飾坊主理人|蘇錦心", Some("F")),
            "蘇錦心"
        );
        assert_eq!(
            crate::store::resolve_npc_person_name("wt_vendor_noodle_soup", "現煮切仔麵攤女東家|林罔市", Some("F")),
            "林罔市"
        );
        assert!(
            crate::store::wenyao_household_slot_for("秦水生").home_name.starts_with("老堰頭"),
            "Qin Shuisheng slot home_name must start with 老堰頭"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("秦水生").brief_tag,
            "水輪石磨匠"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("秦水生").work_name,
            "老堰頭・水輪石磨工坊"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("韓鐵山").home_name,
            "西門・韓氏總站宅院"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("韓鐵山").work_name,
            "西門車馬聯運總站"
        );
        assert_eq!(
            crate::store::wenyao_household_slot_for("陳萬福").home_name,
            "三水口・陳氏庖廚居"
        );

        let resident_settlement_cases = [
            ("秦水生", "老堰頭"),
            ("陳萬福", "文曜"),
            ("陸秉衡", "文曜"),
            ("岑守岳", "文曜"),
            ("許萬川", "風舶角"),
            ("許海音", "風舶角"),
            ("林正雄", "風舶角"),
            ("顧海平", "風舶角"),
            ("林山", "老堰頭"),
            ("馬山行", "北嶺腳"),
            ("石萬鈞", "半嶺鎮"),
            ("葉青", "南哨"),
            ("田大犁", "大圳頭"),
            ("牧野", "牛埔"),
            ("wt_npc_xu_agan", "文曜"),
            ("wt_vendor_noodle_soup", "文曜"),
        ];
        for (id, expected_settlement) in resident_settlement_cases {
            assert_eq!(
                crate::store::resident_macro_settlement(id, ""),
                Some(expected_settlement),
                "resident_macro_settlement for {id} must be Some({expected_settlement})"
            );
            let slot = crate::store::wenyao_household_slot_for(id);
            assert_eq!(
                crate::store::normalize_household_home_settlement(slot.home_name),
                expected_settlement,
                "normalize_household_home_settlement for {id} home_name ({}) must be {expected_settlement}",
                slot.home_name
            );
        }
        let cat_0150 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0150").unwrap();
        assert_eq!(cat_0150.room_tag, "canteen");
        let cat_0212 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0212").unwrap();
        assert_eq!(cat_0212.room_tag, "hunting");
        let cat_0745 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0745").unwrap();
        assert_eq!(cat_0745.room_tag, "lighthouse");
        let cat_0774 = crate::game::peninsula_catalog::find_catalog_entry("wt_cat_0774").unwrap();
        assert_eq!(cat_0774.room_tag, "lighthouse");
    }

    #[test]
    fn test_dyadic_kotodama_co_enacts_room_event_and_script_pack() {
        let plan = plan_dyadic_kotodama_encounter(
            "wt_npc_test_miller_a",
            "秦小匠",
            "水輪匠",
            20,
            None,
            "wt_npc_test_helper_b",
            "周幫工",
            "幫工",
            20,
            None,
            "老堰頭・水輪石磨工坊",
            10,
            10,
            1_775_000_000,
        )
        .expect("should co-enact active room EventPack and ScriptPack");
        assert_eq!(plan.plan_code, "co_enact_script_pack");
        assert!(
            plan.topic_override.contains("秦小匠") && plan.topic_override.contains("周幫工"),
            "unexpected topic_override: {}",
            plan.topic_override
        );
        assert!(
            plan.prompt_directive.contains("雙人協作差事") && !plan.prompt_directive.contains("事件包") && !plan.prompt_directive.contains("劇本包"),
            "unexpected prompt_directive: {}",
            plan.prompt_directive
        );
    }

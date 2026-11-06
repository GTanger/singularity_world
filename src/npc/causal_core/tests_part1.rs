    #[test]
    fn vocational_vessel_maps_distinct_occupations_not_only_hunting_knife() {
        let scholar = resolve_vocational_vessel("文曜・書院講席學者");
        assert_eq!(scholar.item_id, "craft_peninsula_almanac");
        assert_eq!(scholar.catalog_id, "wt_cat_0781");
        assert!(!scholar.is_bladed);

        let cook = resolve_vocational_vessel("南哨・街角食堂掌事");
        assert_eq!(cook.item_id, "craft_kitchen_cleaver");
        assert_eq!(cook.catalog_id, "wt_cat_0564");
        assert!(cook.is_bladed);

        let merchant = resolve_vocational_vessel("風舶角・商行帳房管事");
        assert_eq!(merchant.item_id, "craft_copper_abacus");
        assert_eq!(merchant.catalog_id, "wt_cat_0834");

        let hunter = resolve_vocational_vessel("紅土崁・山林獵戶");
        assert_eq!(hunter.item_id, "craft_steel_hunting_knife");
        assert_eq!(hunter.catalog_id, "wt_cat_1021");
        assert!(hunter.is_bladed);

        let apprentice = resolve_vocational_vessel("風舶角・船塢年輕學徒");
        assert_eq!(apprentice.item_id, "craft_wooden_tool_handle");
        assert_eq!(apprentice.catalog_id, "wt_cat_0954");
        assert_eq!(apprentice.display_name, "百工學徒手用小木尺");

        let bamboo_weaver = resolve_vocational_vessel("分家巷・竹器巧編院匠人");
        assert_eq!(bamboo_weaver.item_id, "craft_wooden_tool_handle");
        assert_eq!(bamboo_weaver.catalog_id, "wt_cat_0954");
        assert_eq!(bamboo_weaver.display_name, "魯班平推刨刀");

        let stone_miller = resolve_vocational_vessel("老堰頭水輪石磨首座|秦水生");
        assert_eq!(stone_miller.item_id, "wt_cat_0973");
        assert_eq!(stone_miller.catalog_id, "wt_cat_0973");
        assert_eq!(stone_miller.display_name, "平底石磨分粉篩架");
        assert!(!stone_miller.is_bladed);
    }

    #[test]
    fn belief_surprise_and_stress_break_pivot_3step_plan() {
        let state = advance_causal_state_on_arrival(
            "test_npc_causal_01",
            "文曜・鐵匠工坊匠人",
            "文曜",
            42,
            "文曜・西門大市集",
            1,
            false,
            false,
            30,
            Some("特級精煉煤"),
        );
        assert_eq!(state.belief_scarcity_item, "特級精煉煤");
        assert!(
            state
                .last_surprise_delta
                .as_deref()
                .unwrap_or("")
                .contains("轉缺「特級精煉煤」")
        );
        assert!(state.plan_steps[1].contains("特級精煉煤"));

        // Trigger high stress break
        let stressed = advance_causal_state_on_arrival(
            "test_npc_causal_01",
            "文曜・鐵匠工坊匠人",
            "文曜",
            42,
            "文曜・百鍛鐵工坊",
            6,
            true,
            true,
            5,
            Some("特級精煉煤"),
        );
        assert!(stressed.stress >= 72);
        assert!(stressed.stress_break_active);
    }

    #[test]
    fn somatic_renderer_incorporates_vessel_plan_and_hook() {
        let text = render_somatic_10layer_thought(
            "test_npc_somatic_02",
            "大圳頭・書院講席",
            "大圳頭",
            77,
            "正在講堂整理卷宗",
            "白塔風物考工手札（微有磨損）",
            "靜室清息",
            &[],
            "",
            45,
        );
        assert!(text.contains("白塔風物考工手札"));
        assert!(text.contains("下一步") || text.contains("承接") || text.contains("盤算"));
    }

    #[test]
    fn life_line_is_monotonic_and_waking_up_does_not_reset_to_zero() {
        let entity_id = "test_npc_linear_life_01";
        let role = "文曜・工坊木匠";
        let settlement = "文曜";
        let seed = 108;

        // 1. 晨間工坊上工：初始時間刻度與同業累計
        let s1 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・百工木器坊",
            1,
            false,
            false,
            30,
            None,
        );
        let tick1 = s1.life_step_tick;
        let rep1 = s1.same_task_repetitions;
        assert!(tick1 >= 1);
        assert!(!s1.carryover_from_yesterday.is_empty());

        // 2. 市集交割差事：時間刻度單調推進
        let s2 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・西門大市集",
            2,
            false,
            false,
            35,
            None,
        );
        assert!(s2.life_step_tick > tick1);
        assert!(s2.same_task_repetitions >= rep1);

        // 3. 夜間就寢安歇：起床不是歸零重整，生命是一條直線
        let s3 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・分家巷宅居",
            0,
            false,
            false,
            35,
            None,
        );
        assert!(s3.life_step_tick > s2.life_step_tick);
        assert!(!s3.carryover_from_yesterday.is_empty());
        assert!(s3.carryover_from_yesterday.contains("延續至今晨"));

        // 4. 次日晨起開工：時間刻度與同業累計持續遞增，延續昨日底蘊
        let s4 = advance_causal_state_on_arrival(
            entity_id,
            role,
            settlement,
            seed,
            "文曜・百工木器坊",
            1,
            false,
            false,
            35,
            None,
        );
        assert!(s4.life_step_tick > s3.life_step_tick);
        assert!(s4.same_task_repetitions > rep1);
        assert!(!s4.carryover_from_yesterday.is_empty());
        assert!(!s4.active_need.is_empty());
        assert!(!s4.image_decision.is_empty());
        assert!(!s4.situated_action.is_empty());

        let mark = format_time_scale_mark(&s4, 1720000000);
        assert!(mark.contains("同業累計"));
        assert!(mark.contains("回"));
    }

    #[test]
    fn abundance_unlocks_more_action_options_than_scarcity_across_all_decision_images() {
        let all_images = [
            DecisionImageKind::Eat,
            DecisionImageKind::Drink,
            DecisionImageKind::SleepOrRest,
            DecisionImageKind::HealOrTend,
            DecisionImageKind::MendOrEquip,
            DecisionImageKind::WorkOrTrade,
            DecisionImageKind::InteractOrUnwind,
        ];

        for img in all_images {
            let scarcity_opts = eligible_actions_for_image(img, 0);
            let subsistence_opts = eligible_actions_for_image(img, 12);
            let abundance_opts = eligible_actions_for_image(img, 40);

            assert_eq!(
                scarcity_opts.len(),
                2,
                "匱乏（沒錢 0 鎂）時，決策意象 {:?} 應僅有 2 個求生窄選項",
                img
            );
            assert_eq!(
                subsistence_opts.len(),
                3,
                "溫飽（有小錢 12 鎂）時，決策意象 {:?} 應有 3 個務實選項",
                img
            );
            assert!(
                abundance_opts.len() >= 5,
                "有餘裕（40 鎂）時，決策意象 {:?} 選項應大幅展開（>=5），實際為 {}",
                img,
                abundance_opts.len()
            );
            assert!(
                abundance_opts.len() > subsistence_opts.len()
                    && subsistence_opts.len() > scarcity_opts.len(),
                "必須嚴格滿足「有餘裕的選擇多，匱乏的選擇少」"
            );
        }

        // 驗證同一人在「食堂」面對「餓了 ➔ 吃」時，沒錢（2 鎂）與有餘裕（50 鎂）做出的選擇截然不同
        let poor_eat = advance_causal_state_on_arrival(
            "test_npc_margin_eat_poor",
            "文曜・工坊學徒",
            "文曜",
            19,
            "文曜・街角食堂",
            1,
            false,
            false,
            2,
            None,
        );
        let rich_eat = advance_causal_state_on_arrival(
            "test_npc_margin_eat_rich",
            "文曜・商行主事",
            "文曜",
            19,
            "文曜・街角食堂",
            1,
            false,
            false,
            50,
            None,
        );
        assert_eq!(poor_eat.decision_image_code, "eat");
        assert_eq!(rich_eat.decision_image_code, "eat");
        assert!(rich_eat.eligible_option_count > poor_eat.eligible_option_count);
        assert_ne!(poor_eat.chosen_action_code, rich_eat.chosen_action_code);
    }

    static TEST_ECONOMY_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn total_conserved_magnesium_remains_strictly_invariant_across_50_actions() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }

        let store_arc = crate::store::get_store().expect("store available");
        let mut store = store_arc.write();

        normalize_and_seed_circular_economy_once(&mut store);

        let test_actor_id = "test_npc_conserved_actor";
        let test_peer_id = "test_npc_conserved_peer";
        let test_wealthy_id = "test_npc_conserved_wealthy";

        let actor: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":30,"display_title":"文曜・百鍛鐵工匠"}}"#,
            test_actor_id
        )).unwrap();
        let peer: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":5,"display_title":"文曜・工坊學徒"}}"#,
            test_peer_id
        )).unwrap();
        let wealthy: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":100,"display_title":"文曜・大商行大主事"}}"#,
            test_wealthy_id
        )).unwrap();

        store.entities.insert(test_actor_id.to_string(), actor);
        store.entities.insert(test_peer_id.to_string(), peer);
        store.entities.insert(test_wealthy_id.to_string(), wealthy);

        let initial_total = total_conserved_magnesium(&store);
        assert!(initial_total > 0, "Initial conserved magnesium must be positive");

        let action_codes = [
            "eat_cheap_coarse_noodles",
            "work_perform_steady_daily_vocation",
            "eat_treat_peers_at_table",
            "work_peddle_gathered_goods_at_market",
            "drink_roadside_coarse_tea_bowl",
            "work_craft_masterpiece_at_own_pace",
            "heal_clinic_basic_clean_and_bandage",
            "social_bail_out_and_sponsor_struggling_peer",
            "rest_rent_cheap_common_bunk",
            "work_scavenge_wild_firewood_and_scraps",
        ];

        let now = 1720000000i64;
        for i in 0..50 {
            let code = action_codes[i % action_codes.len()];
            let _net = execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・百鍛鐵工坊",
                code,
                now + (i as i64) * 60,
            );
            let current_total = total_conserved_magnesium(&store);
            assert_eq!(
                current_total,
                initial_total,
                "Total conserved magnesium drifted on step {}: before={}, after={}",
                i,
                initial_total,
                current_total
            );
        }
    }

    #[test]
    fn format_predefined_stay_activity_contains_zero_debug_tags() {
        let causal = NpcCausalState {
            cornerstone: Cornerstone::Craft,
            stress: 20,
            stress_break_active: false,
            hunger: 30,
            thirst: 25,
            fatigue: 20,
            plan_steps: ["".to_string(), "".to_string(), "".to_string()],
            plan_cursor: 0,
            belief_scarcity_item: "特級精煉煤".to_string(),
            last_surprise_delta: None,
            hooks: Vec::new(),
            life_step_tick: 150,
            same_task_repetitions: 45,
            carryover_from_yesterday: "昨日手感延續".to_string(),
            decision_image_code: "work_or_trade".to_string(),
            eligible_option_count: 6,
            chosen_action_code: "work_craft_masterpiece_at_own_pace".to_string(),
            active_need: "".to_string(),
            image_decision: "".to_string(),
            situated_action: "".to_string(),
            belief_summary: String::new(),
            active_ripples: Vec::new(),
            immune_fact_ids: Vec::new(),
        };

        let result = format_predefined_stay_activity("文曜・百鍛鐵工坊", &causal, 45);
        assert!(!result.contains("項符合條件選項中擇"), "Must not contain option selection debug tag");
        assert!(!result.contains('【'), "Must not contain opening bracket debug tag");
        assert!(!result.contains('】'), "Must not contain closing bracket debug tag");
    }

    #[test]
    fn plan_next_causal_action_advances_somatic_needs_and_derives_forward_causality() {
        let (state, chosen) = plan_next_causal_action(
            "test_npc_causal_plan_01",
            "文曜・百鍛鐵匠",
            "文曜",
            42,
            "文曜・分家巷宅居",
            0,
            false,
            false,
            28,
            12,
            None,
        );
        assert!(state.hunger >= 12);
        assert!(state.thirst >= 14);
        assert!(state.fatigue >= 10);
        assert_eq!(state.decision_image_code, "eat");
        assert_eq!(chosen.image, DecisionImageKind::Eat);
        assert!(chosen.cost_mg <= 28);
    }

    #[test]
    fn commodity_stock_and_dynamic_price_respond_to_npc_consumption_and_supply() {
        let _guard = TEST_ECONOMY_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }

        let store_arc = crate::store::get_store().expect("store available");
        let mut store = store_arc.write();

        normalize_and_seed_circular_economy_once(&mut store);

        let test_actor_id = "test_npc_dynamic_actor";
        let actor: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":50,"display_title":"文曜・常駐住民"}}"#,
            test_actor_id
        )).unwrap();
        store.entities.insert(test_actor_id.to_string(), actor);

        let initial_total = total_conserved_magnesium(&store);

        let opt_eat = lookup_action_option("eat_feast_mountain_sea_stew").unwrap();
        let opt_forage = lookup_action_option("work_scavenge_wild_firewood_and_scraps").unwrap();

        let initial_cost = dynamic_action_cost_mg("文曜", opt_eat);
        let (_, food_item_id, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category("文曜", "food");
        let stock_before = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), food_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);

        // 1. 住民真實消費飲食：扣減聚落現貨並推升動態物價
        for i in 0..3 {
            execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・公營食堂",
                "eat_feast_mountain_sea_stew",
                1720000000 + i * 100,
            );
        }

        let stock_after = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), food_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);
        let post_eat_cost = dynamic_action_cost_mg("文曜", opt_eat);

        assert!(stock_after <= stock_before, "Stock must decrease after consumption");
        assert!(post_eat_cost >= initial_cost, "Dynamic cost must scale with consumption");

        // 2. 住民執行勞作採集補貨：物資庫存增加，並獲得動態工資
        let (_, forage_item_id, _) = crate::game::object_ecology::settlement_scarcity_multiplier_for_category("文曜", "forage");
        let forage_stock_before = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), forage_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);

        for i in 0..3 {
            execute_conserved_action_settlement(
                &mut store,
                test_actor_id,
                "文曜",
                "文曜・西麓林道",
                "work_scavenge_wild_firewood_and_scraps",
                1720000500 + i * 100,
            );
        }

        let forage_stock_after = crate::game::object_ecology::LIVE_ECONOMY_LEDGER
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .settlement_markets
            .get(&("文曜".to_string(), forage_item_id.to_string()))
            .map(|c| c.stock)
            .unwrap_or(0);
        let earn_mg = dynamic_action_earn_mg("文曜", opt_forage);

        assert!(forage_stock_after >= forage_stock_before, "Stock must increase after work/forage");
        assert!(earn_mg >= 2, "Dynamic earn must be at least baseline");

        // 3. 全島總守恆鎂幣量維持恆定
        let current_total = total_conserved_magnesium(&store);
        assert_eq!(
            current_total,
            initial_total,
            "Total conserved magnesium must remain strictly invariant! initial={}, current={}",
            initial_total,
            current_total
        );
        store.entities.remove(test_actor_id);
    }

    #[test]
    fn no_design_jargon_in_frontend_strings() {
        const FORBIDDEN: &[&str] = &[
            "ΔE",
            "預擬選項",
            "決策意象",
            "觸發條件",
            "時間刻度 #",
            "壓負荷",
            "市價約",
            "項符合條件選項",
            "起床非歸零重整",
            "生命是一條直線",
            "選擇少",
            "選擇多",
            "➔",
        ];

        let sample_roles = [
            ("npc_jargon_01", "文曜・百鍛鐵工坊匠人", "文曜", "文曜・百鍛鐵工坊", 2, false, false),
            ("npc_jargon_02", "風舶角・商行帳房管事", "風舶角", "風舶角・港埠大市集", 18, false, true),
            ("npc_jargon_03", "紅土崁・山林獵戶", "紅土崁", "紅土崁・赤石坡", 45, true, false),
        ];

        for (id, role, settlement, dest, mg, aff, worn) in sample_roles {
            let (st1, _) = plan_next_causal_action(
                id,
                role,
                settlement,
                42,
                dest,
                2,
                aff,
                worn,
                mg,
                12,
                Some("特級精煉煤"),
            );
            let st2 = advance_causal_state_on_arrival(
                id,
                role,
                settlement,
                42,
                dest,
                6,
                aff,
                worn,
                mg,
                Some("山野鮮藥草"),
            );
            let aff_vec = if aff {
                vec!["左肩舊傷".to_string()]
            } else {
                Vec::new()
            };
            let thought = render_somatic_10layer_thought(
                id,
                role,
                settlement,
                42,
                "正在工坊忙活",
                "百鍛長柄鐵錘（稱手）",
                "爐火深息",
                &aff_vec,
                "",
                mg,
            );
            let act = format_predefined_stay_activity(dest, &st2, mg);
            let mark = format_time_scale_mark(&st2, 1720000000);

            for field in [
                &st1.active_need,
                &st1.image_decision,
                &st1.situated_action,
                &st1.carryover_from_yesterday,
                &st1.belief_summary,
                &st2.active_need,
                &st2.image_decision,
                &st2.situated_action,
                &st2.carryover_from_yesterday,
                &st2.belief_summary,
                &thought,
                &act,
                &mark,
            ] {
                for &bad in FORBIDDEN {
                    assert!(
                        !field.contains(bad),
                        "NPC frontend string leaked forbidden design term '{bad}': {field}"
                    );
                }
            }
        }

        let sample_ripple = WorldRippleFact {
            fact_id: "rip_sample_01".to_string(),
            kind: RippleFactKind::StrandedCargo,
            subject_zone_or_settlement: "落日岬".to_string(),
            item_or_actor: "特級精煉煤".to_string(),
            order_id: None,
            severity: 0.9,
            origin_tick: 1720000000,
            hop_count: 0,
            max_hops: 3,
            prose_summary: "行商【林雅筑】運送「特級精煉煤」行經落日岬受阻，貨單受滯急待接應".to_string(),
        };
        let sample_cargo = StrandedCargoOrder {
            order_id: "cargo_sample_01".to_string(),
            carrier_id: "carrier_01".to_string(),
            carrier_name: "林雅筑".to_string(),
            item_id: "coal_item".to_string(),
            item_name: "特級精煉煤".to_string(),
            qty: 10,
            origin_settlement: "文曜".to_string(),
            dest_settlement: "風舶角".to_string(),
            waystation_name: "落日岬".to_string(),
            waystation_coord: (10, 20),
            escrow_bounty_mg: 8,
            hazard_prose: "山道遭遇異獸盤踞，風雪阻道難行，行商【林雅筑】受困於落日岬".to_string(),
            claimed_by: None,
            created_at_unix: 1720000000,
            expires_at_unix: 1720003600,
        };

        const INSTANT_TIME_WORDS: &[&str] = &["剛才", "剛剛", "方才"];
        for field in [
            &sample_ripple.prose_summary,
            &sample_cargo.hazard_prose,
        ] {
            for &bad in FORBIDDEN {
                assert!(
                    !field.contains(bad),
                    "NPC frontend string leaked forbidden design term '{bad}': {field}"
                );
            }
            for &bad in INSTANT_TIME_WORDS {
                assert!(
                    !field.contains(bad),
                    "NPC frontend string leaked forbidden instant word '{bad}': {field}"
                );
            }
        }
    }


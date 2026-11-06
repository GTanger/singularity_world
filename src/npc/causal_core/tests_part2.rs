    #[test]
    fn test_sir_ripple_propagation_and_dag_loop_immunity() {
        let now = 1720000000;
        let fact = WorldRippleFact {
            fact_id: "fact_eco_predator_01".to_string(),
            kind: RippleFactKind::PredatorSpillover,
            subject_zone_or_settlement: "西麓林道".to_string(),
            item_or_actor: "裂谷霜豹".to_string(),
            order_id: None,
            severity: 0.9,
            origin_tick: now,
            hop_count: 0,
            max_hops: 2,
            prose_summary: "西麓林道風雪劇盛，裂谷霜豹現身擾道".to_string(),
        };

        let npc_a = "test_propagator_a";
        let npc_b = "test_propagator_b";
        let npc_c = "test_propagator_c";

        // Step 1: Inject fact into NPC A
        assert!(inject_ripple_fact_into_npc(npc_a, fact.clone(), now));
        // Re-injecting the same fact into NPC A fails due to immunity
        assert!(!inject_ripple_fact_into_npc(npc_a, fact.clone(), now));

        // NPC A's top ripple should be this fact
        let top_a = top_active_ripple_for_npc(npc_a, now).expect("top active ripple for A");
        assert_eq!(top_a.fact_id, "fact_eco_predator_01");
        assert_eq!(top_a.hop_count, 0);

        // Step 2: Propagate from A to B (hop 0 -> 1)
        let prop_b = propagate_ripple_between_npcs(npc_a, "張哲銘", npc_b, "李佳穎", now);
        assert!(prop_b.is_some());
        let fact_b = prop_b.unwrap();
        assert_eq!(fact_b.hop_count, 1);
        assert!(fact_b.prose_summary.contains("聽張哲銘提起"));

        // Check B has relational hook toward A
        let hook_b = find_relational_hook_for_target(npc_b, npc_a, "張哲銘");
        assert!(hook_b.is_some());
        assert_eq!(hook_b.unwrap().kind_label, "互通風聲");

        // Step 3: DAG loop immunity: B tries to propagate back to A -> should fail (A is immune)
        let back_prop = propagate_ripple_between_npcs(npc_b, "李佳穎", npc_a, "張哲銘", now);
        assert!(back_prop.is_none(), "DAG loop must be prevented by immunity list");

        // Step 4: Propagate from B to C (hop 1 -> 2)
        let prop_c = propagate_ripple_between_npcs(npc_b, "李佳穎", npc_c, "王俊傑", now);
        assert!(prop_c.is_some());
        let fact_c = prop_c.unwrap();
        assert_eq!(fact_c.hop_count, 2);

        // Step 5: C tries to propagate to D, but hop_count (2) >= max_hops (2), so must fail
        let npc_d = "test_propagator_d";
        let prop_d = propagate_ripple_between_npcs(npc_c, "王俊傑", npc_d, "陳冠宇", now);
        assert!(prop_d.is_none(), "Propagation must stop when hop_count reaches max_hops");
    }

    #[test]
    fn test_stranded_cargo_escrow_sfc_conservation_and_delivery_ripple() {
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

        let test_carrier_id = "test_npc_cargo_carrier";
        let test_rescuer_id = "test_npc_cargo_rescuer";

        let carrier: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":40,"display_title":"風舶角・行商掌事"}}"#,
            test_carrier_id
        )).unwrap();
        let rescuer: crate::store::Entity = serde_json::from_str(&format!(
            r#"{{"id":"{}","kind":"npc","magnesium":25,"display_title":"風舶角・健行巡客"}}"#,
            test_rescuer_id
        )).unwrap();

        store.entities.insert(test_carrier_id.to_string(), carrier);
        store.entities.insert(test_rescuer_id.to_string(), rescuer);

        let sfc_baseline = total_conserved_magnesium(&store);
        let now = crate::game::now_unix();

        // 1. Create a stranded cargo order with dest "風舶角"
        let order = create_stranded_cargo_order(
            test_carrier_id,
            "行商阿明",
            "item_heavy_copper",
            "紫銅錠",
            20,
            "文曜",
            "風舶角",
            "落日岬驛站",
            (15, 25),
            "落日岬風急雪大，商隊車軸受損困於驛站",
            now,
        ).expect("Order should be created");

        assert_eq!(order.escrow_bounty_mg, 8);
        // Strict SFC Invariance check:
        assert_eq!(
            total_conserved_magnesium(&store),
            sfc_baseline,
            "Total magnesium must remain strictly invariant after escrow lock"
        );

        // Carrier should receive StrandedCargo ripple fact
        let carrier_rip = top_active_ripple_for_npc(test_carrier_id, now).expect("carrier ripple");
        assert_eq!(carrier_rip.kind, RippleFactKind::StrandedCargo);

        // 2. Deliver/claim the cargo by rescuer
        let (delivered, bounty) = claim_or_deliver_stranded_cargo_for_npc(
            test_rescuer_id,
            "健行客阿華",
            "落日岬驛站",
            (15, 26),
            now + 120,
        ).expect("Rescuer should be able to claim/deliver near waystation");

        assert_eq!(delivered.order_id, order.order_id);
        assert_eq!(bounty, 8);

        // Rescuer magnesium credited via credit_magnesium_bounty
        credit_magnesium_bounty(&mut store, test_rescuer_id, bounty);

        // Strict SFC Invariance check after delivery:
        assert_eq!(
            total_conserved_magnesium(&store),
            sfc_baseline,
            "Total magnesium must remain strictly invariant after cargo delivery and payout"
        );

        // Verify relational hook formed on carrier toward rescuer
        let hook = find_relational_hook_for_target(test_carrier_id, test_rescuer_id, "健行客阿華")
            .expect("carrier should have hook for rescuer");
        assert_eq!(hook.kind_label, "山道解圍");
        assert!(hook.intensity >= 4);

        // Verify rescuer received ApexSlainOrRescued ripple
        let rescuer_rip = top_active_ripple_for_npc(test_rescuer_id, now + 120).expect("rescuer ripple");
        assert_eq!(rescuer_rip.kind, RippleFactKind::ApexSlainOrRescued);

        // 3. Test escrow refund on expiration
        let order2 = create_stranded_cargo_order(
            test_carrier_id,
            "行商阿明",
            "item_herbs",
            "冷杉藥草",
            10,
            "文曜",
            "風舶角",
            "赤石坡",
            (10, 10),
            "赤石坡巨石滾落阻斷棧道",
            now + 200,
        ).expect("Order 2 should be created");

        assert_eq!(order2.escrow_bounty_mg, 8);
        assert_eq!(total_conserved_magnesium(&store), sfc_baseline);

        // Force expiration of order2
        {
            let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
            if let Some(o) = orders.get_mut(&order2.order_id) {
                o.expires_at_unix = 0;
            }
        }

        // When total_conserved_magnesium runs, it will sweep expired orders and refund escrow
        let sfc_after_sweep = total_conserved_magnesium(&store);
        assert_eq!(
            sfc_after_sweep,
            sfc_baseline,
            "Total magnesium must remain strictly invariant after escrow expiration refund"
        );

        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        let o2 = orders.get(&order2.order_id).expect("order2 should exist");
        assert_eq!(o2.escrow_bounty_mg, 0, "Escrow should be zeroed after refund");
    }

    #[test]
    fn test_dynamic_relational_hook_and_player_action_mutation() {
        let now = 1720100000;
        let npc_id = "test_npc_rel_mut_01";
        let player_id = "player_hero_01";
        let player_name = "青衣遊俠";

        // Initial state
        get_or_init_causal_state(npc_id, "文曜・百鍛鐵工坊匠人", "文曜", 42);

        // Test player rescue action
        mutate_npc_from_player_action(
            npc_id,
            player_id,
            player_name,
            "文曜",
            "rescue",
            "百鍛長柄鐵錘",
            0,
            now,
        );

        let hook = find_relational_hook_for_target(npc_id, player_id, player_name)
            .expect("Hook should be recorded after rescue");
        assert_eq!(hook.kind_label, "救命之恩");
        assert!(hook.intensity >= 6);
        assert!(hook.summary.contains("百鍛長柄鐵錘"));

        // Check ripple fact injected
        let rip = top_active_ripple_for_npc(npc_id, now).expect("Should have ripple fact");
        assert_eq!(rip.kind, RippleFactKind::PlayerDeed);
        assert!(rip.prose_summary.contains(player_name));

        // Test trade action updates hook
        mutate_npc_from_player_action(
            npc_id,
            player_id,
            player_name,
            "文曜",
            "trade",
            "特級精煉煤",
            10,
            now + 60,
        );

        let updated_hook = find_relational_hook_for_target(npc_id, player_id, player_name)
            .expect("Hook should be updated");
        assert_eq!(updated_hook.kind_label, "商貿往來");
        assert!(updated_hook.intensity > hook.intensity);

        // Test manual upsert_relational_hook cap at 100
        upsert_relational_hook(
            npc_id,
            player_id,
            player_name,
            "莫逆之交",
            "歷經患難，信諾千金".to_string(),
            99,
            now + 120,
        );
        let max_hook = find_relational_hook_for_target(npc_id, player_id, player_name).unwrap();
        assert_eq!(max_hook.intensity, 100);
    }
    #[test]
    fn test_settle_player_with_settlement_treasury_sfc() {
        let pid = "test_player_sfc_01";
        let res = settle_player_with_settlement_treasury(pid, "文曜", 10);
        assert!(res >= 0);
        let res_deduct = settle_player_with_settlement_treasury(pid, "文曜", -10);
        assert!(res_deduct <= 0);
    }

    #[test]
    fn test_claim_and_deliver_stranded_cargo_for_player() {
        let now = 1000000;
        let order = StrandedCargoOrder {
            order_id: "cargo_test_claim_01".to_string(),
            carrier_id: "carrier_npc_01".to_string(),
            carrier_name: "林主事".to_string(),
            item_id: "mat_grain_sack".to_string(),
            item_name: "大圳頭飽滿新米".to_string(),
            qty: 2,
            origin_settlement: "大圳頭".to_string(),
            dest_settlement: "文曜".to_string(),
            waystation_name: "落日岬驛站".to_string(),
            waystation_coord: (15, 26),
            escrow_bounty_mg: 12,
            hazard_prose: "路經落日岬遇風浪受阻".to_string(),
            claimed_by: None,
            created_at_unix: now,
            expires_at_unix: now + 3600,
        };
        {
            let mut orders = LIVE_STRANDED_CARGO_ORDERS.write().unwrap_or_else(|e| e.into_inner());
            orders.insert(order.order_id.clone(), order);
        }

        let pid = "player_hero_01";
        let pname = "義士阿強";

        // Claim by player
        let claimed = claim_stranded_cargo_for_player(pid, pname, "cargo_test_claim_01", "文曜", now + 10)
            .expect("Should claim order");
        assert_eq!(claimed.claimed_by.as_deref(), Some(pid));

        // Another NPC should not claim this order
        let npc_claim = claim_or_deliver_stranded_cargo_for_npc(
            "npc_other_01",
            "路人NPC",
            "落日岬驛站",
            (15, 26),
            now + 20,
        );
        assert!(npc_claim.is_none(), "NPC should skip player-claimed order");

        // Player delivers cargo
        let (delivered, bounty) = deliver_stranded_cargo_for_player(
            pid,
            pname,
            "cargo_test_claim_01",
            "文曜",
            now + 30,
        ).expect("Player delivery should succeed");
        assert_eq!(delivered.order_id, "cargo_test_claim_01");
        assert_eq!(bounty, 12);
    }

    #[test]
    fn test_offline_autonomy_directives_and_recaps() {
        let pid = "player_offline_test_01";
        assert_eq!(get_player_offline_directive(pid), OfflineAutonomyDirective::InnRestAndHeal);

        set_player_offline_directive(pid, OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(get_player_offline_directive(pid), OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(OfflineAutonomyDirective::from_code("workshop_craft"), OfflineAutonomyDirective::WorkshopCraftAndAssist);
        assert_eq!(OfflineAutonomyDirective::from_code("field_tend"), OfflineAutonomyDirective::FieldTendAndForage);
        assert_eq!(OfflineAutonomyDirective::from_code("road_escort"), OfflineAutonomyDirective::RoadEscortAndPatrol);
        assert_eq!(OfflineAutonomyDirective::from_code("inn_rest"), OfflineAutonomyDirective::InnRestAndHeal);

        let now = 2000000;
        let step_res = step_offline_player_once(pid, now);
        assert!(step_res.is_some());

        let recaps = drain_player_offline_recap(pid, now);
        assert!(!recaps.is_empty());
        assert_eq!(recaps[0].0, "social");
        assert!(recaps[0].1.contains("駐坊"));

        // Drained second time returns empty when no new offline step occurred
        let recaps2 = drain_player_offline_recap(pid, now);
        assert!(recaps2.is_empty());
    }

    #[test]
    fn test_offline_autonomy_title_heuristics_and_physics() {
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

        if let Some(arc) = crate::store::get_store() {
            let mut s = arc.write();
            let mut e_craft = crate::store::Entity::default();
            e_craft.id = "test_offline_crafter".to_string();
            e_craft.display_title = "水輪石磨工匠".to_string();
            e_craft.equipment_slots = r#"{"hold_r":"wt_cat_0984"}"#.to_string();
            e_craft.vit = 80;
            e_craft.qi = 60;
            e_craft.inventory = r#"[{"item_id":"wt-good-009","qty":5}]"#.to_string();
            s.entities.insert(e_craft.id.clone(), e_craft);

            let mut e_farm = crate::store::Entity::default();
            e_farm.id = "test_offline_farmer".to_string();
            e_farm.display_title = "靈田採藥圃農".to_string();
            e_farm.vit = 80;
            e_farm.qi = 60;
            s.entities.insert(e_farm.id.clone(), e_farm);

            let mut e_guard = crate::store::Entity::default();
            e_guard.id = "test_offline_guard".to_string();
            e_guard.display_title = "古道行商哨衛".to_string();
            e_guard.vit = 80;
            e_guard.qi = 60;
            s.entities.insert(e_guard.id.clone(), e_guard);

            let mut e_tired = crate::store::Entity::default();
            e_tired.id = "test_offline_tired".to_string();
            e_tired.display_title = "百工鐵匠".to_string();
            e_tired.vit = 15;
            e_tired.qi = 10;
            s.entities.insert(e_tired.id.clone(), e_tired);
        }

        assert_eq!(
            get_player_offline_directive("test_offline_crafter"),
            OfflineAutonomyDirective::WorkshopCraftAndAssist
        );
        assert_eq!(
            get_player_offline_directive("test_offline_farmer"),
            OfflineAutonomyDirective::FieldTendAndForage
        );
        assert_eq!(
            get_player_offline_directive("test_offline_guard"),
            OfflineAutonomyDirective::RoadEscortAndPatrol
        );

        let now = 2005000;
        let step_tired = step_offline_player_once("test_offline_tired", now);
        assert!(step_tired.is_some());
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get("test_offline_tired") {
                assert!(e.vit >= 25);
                assert_eq!(e.current_activity, "於客舍內閉門靜養、調勻氣血");
            }
        }

        let _ = step_offline_player_once("test_offline_crafter", now);
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            if let Some(e) = s.entities.get("test_offline_crafter") {
                assert_eq!(e.vit, 77);
                assert_eq!(e.qi, 58);
                assert!(e.current_activity.contains("工坊"));
            }
        }
        let vstate = crate::pg::living_world::get_vessel_state("test_offline_crafter", "wt_cat_0984");
        assert!(vstate.use_count >= 1);
    }


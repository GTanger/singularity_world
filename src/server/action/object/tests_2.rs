#[cfg(test)]
mod tests_2 {
    use super::*;

    #[test]
    fn m3_020_queue_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_mint_half_open_door".to_string(),
            name: "半開鑄幣門".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Open".to_string(), "Queue".to_string()],
            responses: [("Look".to_string(), "半開的門縫。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_waiting, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_waiting, vec!["Open".to_string()]);

        let (with_waiting, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[],
        );
        assert_eq!(with_waiting, vec!["Open".to_string(), "Queue".to_string()]);

        let (already_queued, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        );
        assert_eq!(already_queued, vec!["Open".to_string()]);
    }

    #[test]
    fn m3_020_accept_job_button_is_contextual() {
        let obj = crate::model::RoomObject {
            id: "m3_020_hire_short_work_boards".to_string(),
            name: "短工牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "AcceptJob".to_string(),
            ],
            responses: [("Look".to_string(), "短工牌掛在棚柱外側。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };

        let (without_read, _) = object_followup_buttons(&obj, "", "Look", &[], &[]);
        assert_eq!(without_read, vec!["Read".to_string()]);

        let (with_read, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[],
        );
        assert_eq!(with_read, vec!["Read".to_string(), "AcceptJob".to_string()]);

        let (already_accepted, _) = object_followup_buttons(
            &obj,
            "",
            "Look",
            &["m3_020_hire_short_work_boards|Read".to_string()],
            &[M3_020_SHORT_WORK_STATE.to_string()],
        );
        assert_eq!(already_accepted, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_pass_and_council_buttons_are_contextual() {
        let pass_obj = crate::model::RoomObject {
            id: "m3_020_stone_gate_pass_tags".to_string(),
            name: "通行牌".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "JoinCheck".to_string(),
            ],
            responses: [("Look".to_string(), "通行牌壓在石案邊。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_pass_read, _) = object_followup_buttons(&pass_obj, "", "Look", &[], &[]);
        assert_eq!(without_pass_read, vec!["Read".to_string()]);
        let (with_pass_read, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_pass_read,
            vec!["Read".to_string(), "JoinCheck".to_string()]
        );
        let (pass_already_checked, _) = object_followup_buttons(
            &pass_obj,
            "",
            "Look",
            &["m3_020_stone_gate_pass_tags|Read".to_string()],
            &[M3_020_PASS_CHECK_STATE.to_string()],
        );
        assert_eq!(pass_already_checked, vec!["Read".to_string()]);

        let notice_obj = crate::model::RoomObject {
            id: "m3_020_council_notice_strip".to_string(),
            name: "議棚告示".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "RegisterComplaint".to_string(),
            ],
            responses: [("Look".to_string(), "告示貼在階邊木板上。".to_string())]
                .into_iter()
                .collect(),
            move_to_room_id: String::new(),
        };
        let (without_notice_read, _) = object_followup_buttons(&notice_obj, "", "Look", &[], &[]);
        assert_eq!(without_notice_read, vec!["Read".to_string()]);
        let (with_notice_read, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[],
        );
        assert_eq!(
            with_notice_read,
            vec!["Read".to_string(), "RegisterComplaint".to_string()]
        );
        let (notice_already_registered, _) = object_followup_buttons(
            &notice_obj,
            "",
            "Look",
            &["m3_020_council_notice_strip|Read".to_string()],
            &[M3_020_COUNCIL_COMPLAINT_STATE.to_string()],
        );
        assert_eq!(notice_already_registered, vec!["Read".to_string()]);
    }

    #[test]
    fn m3_020_queue_state_echo_takes_priority() {
        let echo = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[M3_020_MINT_QUEUE_STATE.to_string()],
        )
        .unwrap();
        assert!(echo.contains("側線已被兌牌人接住"));

        let flagstone_echo = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_echo.contains("通往棚口"));

        let mint_echo_after_short_work = m3_020_scene_object_look_echo(
            "m3_020_mint_half_open_door",
            &["m3_020_mint_half_open_door|Open".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(mint_echo_after_short_work.contains("側線已被兌牌人接住"));

        let flagstone_after_council = m3_020_scene_object_look_echo(
            "m3_020_flagstone_waiting_lines",
            &["m3_020_flagstone_waiting_lines|Use".to_string()],
            &[
                M3_020_MINT_QUEUE_STATE.to_string(),
                M3_020_SHORT_WORK_STATE.to_string(),
                M3_020_PASS_CHECK_STATE.to_string(),
                M3_020_COUNCIL_COMPLAINT_STATE.to_string(),
            ],
        )
        .unwrap();
        assert!(flagstone_after_council.contains("往坡街那側"));
    }

    #[test]
    fn m3_020_followup_echo_requires_matching_prior_payload() {
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_mint_half_open_door|Open".to_string()]
            )
            .unwrap()
            .contains("門縫比先前更窄")
        );
        assert!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["m3_020_flagstone_waiting_lines|Use".to_string()]
            )
            .unwrap()
            .contains("側線延伸到巷口")
        );
        assert_eq!(
            m3_020_scene_object_followup_echo(
                "m3_020_mint_half_open_door",
                &["bad-payload".to_string()]
            ),
            None
        );
    }

    #[test]
    fn m3_020_followup_echo_uses_latest_relevant_event() {
        let waiting_then_mint = vec![
            "m3_020_flagstone_waiting_lines|Use".to_string(),
            "m3_020_mint_half_open_door|Open".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &waiting_then_mint)
                .unwrap()
                .contains("門縫比先前更窄")
        );

        let mint_then_waiting = vec![
            "m3_020_mint_half_open_door|Open".to_string(),
            "m3_020_flagstone_waiting_lines|Use".to_string(),
        ];
        assert!(
            m3_020_scene_object_followup_echo("m3_020_mint_half_open_door", &mint_then_waiting)
                .unwrap()
                .contains("側線延伸到巷口")
        );
    }

    #[test]
    fn m3_027_material_gather_batch_success_text_formatting() {
        let single_reed = materials::GatheredMaterial {
            item_id: M3_027_REED_ITEM_ID.to_string(),
            name: "河霧蘆草".to_string(),
            source_node_id: "north_bank_reeds".to_string(),
            source_room_name: "半嶺水岸渡口".to_string(),
            origin_label: "河霧蘆叢".to_string(),
            stock_remaining: 3,
        };
        let single_text = m3_027_material_gather_batch_success_text(
            "m3_027_north_bank_reeds",
            &single_reed,
            1,
        );
        assert!(single_text.contains("割下一束「河霧蘆草」"));

        let batch_reed = materials::GatheredMaterial {
            item_id: M3_027_REED_ITEM_ID.to_string(),
            name: "河霧蘆草".to_string(),
            source_node_id: "north_bank_reeds".to_string(),
            source_room_name: "半嶺水岸渡口".to_string(),
            origin_label: "河霧蘆叢".to_string(),
            stock_remaining: 0,
        };
        let batch_text = m3_027_material_gather_batch_success_text(
            "m3_027_north_bank_reeds",
            &batch_reed,
            3,
        );
        assert!(batch_text.contains("紮成 3 束「河霧蘆草」"));
        assert!(batch_text.contains("整片靠近水線的蘆叢都被割低了"));

        let batch_sackcloth = materials::GatheredMaterial {
            item_id: M3_027_SACKCLOTH_ITEM_ID.to_string(),
            name: "粗麻袋布".to_string(),
            source_node_id: "market_sackcloth_pile".to_string(),
            source_room_name: "水尾寮渡頭市集".to_string(),
            origin_label: "棚腳麻袋堆".to_string(),
            stock_remaining: 2,
        };
        let sackcloth_text = m3_027_material_gather_batch_success_text(
            "m3_027_market_sackcloth_pile",
            &batch_sackcloth,
            2,
        );
        assert!(sackcloth_text.contains("捲成 2 卷「粗麻袋布」"));

        let batch_rope = materials::GatheredMaterial {
            item_id: M3_027_ROPE_ITEM_ID.to_string(),
            name: "浸水麻繩".to_string(),
            source_node_id: "plank_wet_rope_coil".to_string(),
            source_room_name: "渡船踏板".to_string(),
            origin_label: "踏板木樁繩圈".to_string(),
            stock_remaining: 1,
        };
        let rope_text = m3_027_material_gather_batch_success_text(
            "m3_027_plank_wet_rope_coil",
            &batch_rope,
            2,
        );
        assert!(rope_text.contains("解下並盤好 2 卷韌度結實的「浸水麻繩」"));
    }

    #[test]
    fn m3_027_reed_batch_sale_count_calculation() {
        // 當手頭有 2 束蘆草且市場無人交貨時，一次交齊 2 束
        assert_eq!(m3_027_reed_batch_sale_count(2, &[], &[]), 2);
        // 當手頭有 5 束蘆草，受限於每輪需求上限 2 束，一次交齊 2 束
        assert_eq!(m3_027_reed_batch_sale_count(5, &[], &[]), 2);
        // 當手頭僅有 1 束，交出 1 束
        assert_eq!(m3_027_reed_batch_sale_count(1, &[], &[]), 1);
        // 當市場已有人交了 1 束，手頭有 2 束，只交出剩餘的 1 束
        let one_prior_sale = vec!["m3_027_market_reed_buyer|SellReeds".to_string()];
        assert_eq!(m3_027_reed_batch_sale_count(2, &one_prior_sale, &[]), 1);
        // 當市場需求已滿（2 束），交出 0 束
        let two_prior_sales = vec![
            "m3_027_market_reed_buyer|SellReeds".to_string(),
            "m3_027_market_reed_buyer|SellReeds".to_string(),
        ];
        assert_eq!(m3_027_reed_batch_sale_count(2, &two_prior_sales, &[]), 0);
        // 當市場需求狀態已標記填滿，交出 0 束
        let filled_state = vec![M3_027_REED_DEMAND_FILLED_STATE.to_string()];
        assert_eq!(m3_027_reed_batch_sale_count(2, &[], &filled_state), 0);
        // 手頭無蘆草時交出 0 束
        assert_eq!(m3_027_reed_batch_sale_count(0, &[], &[]), 0);
    }

    #[test]
    fn format_runtime_gather_batch_narrative_test() {
        let single = crate::pg::world_resource::GatherResult {
            action_id: "act_1".into(),
            place_id: "place_1".into(),
            entity_id: "ent_1".into(),
            item_id: "mat_ore".into(),
            name: "粗礦石".into(),
            quantity: 1,
            source_stock_after: 5,
            entity_quantity_after: 1,
            inventory: "[]".into(),
            narrative: "你敲下了一塊粗礦石。".into(),
        };
        assert_eq!(
            format_runtime_gather_batch_narrative(&single),
            "你敲下了一塊粗礦石。"
        );

        let batch = crate::pg::world_resource::GatherResult {
            quantity: 3,
            narrative: "你敲下了一塊粗礦石。".into(),
            name: "粗礦石".into(),
            ..single
        };
        let batch_text = format_runtime_gather_batch_narrative(&batch);
        assert!(batch_text.contains("你順著手勢一氣呵成連取了 3 份「粗礦石」"));
    }

    #[test]
    fn resolve_room_object_handles_catalog_and_peninsula_entries() {
        let obj = resolve_room_object("nonexistent_room", "wt_cat_0001");
        assert!(obj.is_some());
        let obj = obj.unwrap();
        assert_eq!(obj.name, "赤鐵重礦");
        assert!(obj.sockets.contains(&"Look".to_string()));

        let by_name = resolve_room_object("nonexistent_room", "三號坑赤鐵重礦");
        assert!(by_name.is_some());
        let by_short_name = resolve_room_object("nonexistent_room", "赤鐵重礦");
        assert!(by_short_name.is_some());
    }

    #[test]
    fn resolve_room_object_handles_fountain_pen() {
        let by_id = resolve_room_object("文曜・文曜學院", "wt_academy_fountain_pen");
        assert!(by_id.is_some());
        let obj = by_id.unwrap();
        assert_eq!(obj.id, "wt_academy_fountain_pen");
        assert_eq!(obj.name, "黃銅儲墨筆");
        assert!(obj.sockets.contains(&"Look".to_string()));
        assert!(obj.sockets.contains(&"Use".to_string()));
        assert!(obj.sockets.contains(&"Operate".to_string()));
        assert!(obj.responses.get("Look").unwrap().contains("黃銅桿儲墨硬筆"));
        assert!(obj.responses.get("Use").unwrap().contains("銥金筆尖觸及竹綿紙面"));
        assert!(obj.responses.get("Operate").unwrap().contains("輕旋黃銅筆桿檢視儲墨囊"));

        let by_name = resolve_room_object("any_room", "教案上的黃銅硬筆");
        assert!(by_name.is_some());
    }

    #[test]
    fn resolve_room_name_handles_grid_and_plain_names() {
        assert_eq!(resolve_room_name("白塔公議堂"), "白塔公議堂");
    }

    #[test]
    fn test_v03852_living_world_expansion_object_resolution_and_followup() {
        let mill = resolve_room_object("老堰頭水磨坊", "wt_mill_laoyan_flour_mill")
            .expect("wt_mill_laoyan_flour_mill must resolve");
        assert!(mill.sockets.contains(&"MillGrain".to_string()));

        let chess = resolve_room_object("雙星鎮茶寮客堂", "wt_inn_ebony_shell_chess")
            .expect("wt_inn_ebony_shell_chess must resolve");
        assert!(chess.sockets.contains(&"PlayChess".to_string()));

        let raft = resolve_room_object("水尾寮蘆葦碼頭", "wt_dock_riverbank_raft")
            .expect("wt_dock_riverbank_raft must resolve");
        assert!(raft.sockets.contains(&"RowSwampRaft".to_string()));

        // Verify object_followup_buttons keeps repeatable actions when clicked
        let (buttons_after_mill, _) = object_followup_buttons(&mill, "老堰頭水磨坊", "MillGrain", &[], &[]);
        assert!(buttons_after_mill.contains(&"MillGrain".to_string()), "Repeatable MillGrain action must not disappear after clicking");

        let angling_obj = crate::model::RoomObject {
            id: "wt_coastal_angling_spot".to_string(),
            name: "岸邊垂釣位".to_string(),
            sockets: vec!["Look".into(), "AngleFish".into(), "Use".into()],
            responses: std::collections::HashMap::new(),
            owner: String::new(),
            move_to_room_id: String::new(),
        };
        let (buttons_after_angle, _) = object_followup_buttons(&angling_obj, "黑石灣小碼頭", "AngleFish", &[], &[]);
        assert!(buttons_after_angle.contains(&"AngleFish".to_string()), "Repeatable AngleFish must not disappear after clicking");
    }

    #[test]
    fn test_v03853_three_pillars_actions_and_labels() {
        use super::action_label_zh;

        // 1. Verify action_label_zh mappings
        assert_eq!(action_label_zh("ChartAstrolabe"), "拓印星盤");
        assert_eq!(action_label_zh("TestEngraving"), "測紋校準");
        assert_eq!(action_label_zh("SparPillar"), "試勁拆招");
        assert_eq!(action_label_zh("SoakHotSpring"), "祛寒湯浴");
        assert_eq!(action_label_zh("GroomAppearance"), "修面整容");
        assert_eq!(action_label_zh("MendTravelGear"), "補綴行裝");
        assert_eq!(action_label_zh("ReadWindVane"), "觀測風候");
        assert_eq!(action_label_zh("ApplyMoxibustion"), "百草溫灸");
        assert_eq!(action_label_zh("HangVotivePlaque"), "懸掛銘牌");

        // 2. Verify resolve_room_object for all 9 objects
        let astrolabe = resolve_room_object("半嶺觀星台", "wt_cat_0734")
            .expect("wt_cat_0734 must resolve");
        assert_eq!(astrolabe.name, "渾天星軌銅儀");
        assert!(astrolabe.sockets.contains(&"ChartAstrolabe".to_string()));

        let engraving = resolve_room_object("烏石鼻匠坊", "wt_martial_force_stone")
            .expect("wt_martial_force_stone must resolve");
        assert_eq!(engraving.name, "測勁玄武碑");
        assert!(engraving.sockets.contains(&"TestEngraving".to_string()));

        let pillar = resolve_room_object("烏石鼻試武場", "wt_arena_testing_pillar")
            .expect("wt_arena_testing_pillar must resolve");
        assert_eq!(pillar.name, "玄武試勁柱");
        assert!(pillar.sockets.contains(&"SparPillar".to_string()));

        let hotspring = resolve_room_object("半嶺溫泉池", "wt_bath_jade_pool")
            .expect("wt_bath_jade_pool must resolve");
        assert_eq!(hotspring.name, "恆溫熱湯池");
        assert!(hotspring.sockets.contains(&"SoakHotSpring".to_string()));

        let barber = resolve_room_object("雙星鎮修面鋪", "wt_barber_folding_razor")
            .expect("wt_barber_folding_razor must resolve");
        assert_eq!(barber.name, "摺疊剃刀架");
        assert!(barber.sockets.contains(&"GroomAppearance".to_string()));

        let repair = resolve_room_object("霜林口街角", "wt_repair_umbrella_bench")
            .expect("wt_repair_umbrella_bench must resolve");
        assert_eq!(repair.name, "修傘木砧檯");
        assert!(repair.sockets.contains(&"MendTravelGear".to_string()));

        let wind_vane = resolve_room_object("半嶺測候所", "wt_observatory_anemometer")
            .expect("wt_observatory_anemometer must resolve");
        assert_eq!(wind_vane.name, "黃銅風向儀");
        assert!(wind_vane.sockets.contains(&"ReadWindVane".to_string()));

        let clinic = resolve_room_object("溫氏百草堂", "wt_clinic_acupoint_bed")
            .expect("wt_clinic_acupoint_bed must resolve");
        assert_eq!(clinic.name, "竹席診脈榻");
        assert!(clinic.sockets.contains(&"ApplyMoxibustion".to_string()));

        let votive = resolve_room_object("北嶺山道古杉前", "wt_obj_mountain_milestone")
            .expect("wt_obj_mountain_milestone must resolve");
        assert_eq!(votive.name, "山道指路碑");
        assert!(votive.sockets.contains(&"HangVotivePlaque".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (buttons_astrolabe, _) = object_followup_buttons(&astrolabe, "半嶺觀星台", "ChartAstrolabe", &[], &[]);
        assert!(buttons_astrolabe.contains(&"ChartAstrolabe".to_string()));

        let (buttons_engraving, _) = object_followup_buttons(&engraving, "烏石鼻匠坊", "TestEngraving", &[], &[]);
        assert!(buttons_engraving.contains(&"TestEngraving".to_string()));

        let (buttons_pillar, _) = object_followup_buttons(&pillar, "烏石鼻試武場", "SparPillar", &[], &[]);
        assert!(buttons_pillar.contains(&"SparPillar".to_string()));

        let (buttons_hotspring, _) = object_followup_buttons(&hotspring, "半嶺溫泉池", "SoakHotSpring", &[], &[]);
        assert!(buttons_hotspring.contains(&"SoakHotSpring".to_string()));

        let (buttons_barber, _) = object_followup_buttons(&barber, "雙星鎮修面鋪", "GroomAppearance", &[], &[]);
        assert!(buttons_barber.contains(&"GroomAppearance".to_string()));

        let (buttons_repair, _) = object_followup_buttons(&repair, "霜林口街角", "MendTravelGear", &[], &[]);
        assert!(buttons_repair.contains(&"MendTravelGear".to_string()));

        let (buttons_wind_vane, _) = object_followup_buttons(&wind_vane, "半嶺測候所", "ReadWindVane", &[], &[]);
        assert!(buttons_wind_vane.contains(&"ReadWindVane".to_string()));

        let (buttons_clinic, _) = object_followup_buttons(&clinic, "溫氏百草堂", "ApplyMoxibustion", &[], &[]);
        assert!(buttons_clinic.contains(&"ApplyMoxibustion".to_string()));

        let (buttons_votive, _) = object_followup_buttons(&votive, "北嶺山道古杉前", "HangVotivePlaque", &[], &[]);
        assert!(buttons_votive.contains(&"HangVotivePlaque".to_string()));
    }
    #[test]
    fn test_v03854_three_pillar_actions_and_labels() {
        use super::{action_label_zh, normalize_action_key};

        // 1. Verify action_label_zh and normalize_action_key mappings
        let actions = [
            ("TranscribeRareTome", "鈔錄孤本"),
            ("AttendLecture", "旁聽策論"),
            ("CalibrateClock", "校時報刻"),
            ("FitMortiseTenon", "榫卯校修"),
            ("QuenchBlade", "精鋼回火"),
            ("DyeIndigoCloth", "藍染浸布"),
            ("FeedDraftBeast", "飼餵草獸"),
            ("BoardCoastalShip", "登船遠航"),
            ("InspectLandRegistry", "查閱地籍"),
        ];
        for (key, zh) in actions {
            assert_eq!(action_label_zh(key), zh);
            assert_eq!(action_label_zh(zh), zh);
            assert_eq!(normalize_action_key(key), key);
            assert_eq!(normalize_action_key(zh), key);
        }

        // 2. Verify resolve_room_object for all 8 fallback objects
        let shelf = resolve_room_object("藏書閣", "wt_book_printing_shelf")
            .expect("wt_book_printing_shelf must resolve");
        assert_eq!(shelf.name, "風物誌書架");
        assert!(shelf.sockets.contains(&"TranscribeRareTome".to_string()));

        let pen = resolve_room_object("講堂", "wt_academy_fountain_pen")
            .expect("wt_academy_fountain_pen must resolve");
        assert_eq!(pen.name, "黃銅儲墨筆");
        assert!(pen.sockets.contains(&"AttendLecture".to_string()));

        let bell = resolve_room_object("鐘樓", "wt_clocktower_bronze_bell")
            .expect("wt_clocktower_bronze_bell must resolve");
        assert_eq!(bell.name, "報時青銅鐘");
        assert!(bell.sockets.contains(&"CalibrateClock".to_string()));

        let bench = resolve_room_object("工坊", "wt_wood_planing_bench")
            .expect("wt_wood_planing_bench must resolve");
        assert_eq!(bench.name, "刨木工作台");
        assert!(bench.sockets.contains(&"FitMortiseTenon".to_string()));

        let trough = resolve_room_object("鐵匠鋪", "wt_forge_quench_trough")
            .expect("wt_forge_quench_trough must resolve");
        assert_eq!(trough.name, "精鋼淬火槽");
        assert!(trough.sockets.contains(&"QuenchBlade".to_string()));

        let rack = resolve_room_object("布莊", "wt_cloth_garment_rack")
            .expect("wt_cloth_garment_rack must resolve");
        assert_eq!(rack.name, "防雨長衫架");
        assert!(rack.sockets.contains(&"DyeIndigoCloth".to_string()));

        let water = resolve_room_object("車馬行", "wt_caravan_water_trough")
            .expect("wt_caravan_water_trough must resolve");
        assert_eq!(water.name, "青石飲馬槽");
        assert!(water.sockets.contains(&"FeedDraftBeast".to_string()));

        let cadastral = resolve_room_object("房地契局", "wt_deed_cadastral_map_desk")
            .expect("wt_deed_cadastral_map_desk must resolve");
        assert_eq!(cadastral.name, "四坊地籍案");
        assert!(cadastral.sockets.contains(&"InspectLandRegistry".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (b_shelf, _) = object_followup_buttons(&shelf, "藏書閣", "TranscribeRareTome", &[], &[]);
        assert!(b_shelf.contains(&"TranscribeRareTome".to_string()));

        let (b_pen, _) = object_followup_buttons(&pen, "講堂", "AttendLecture", &[], &[]);
        assert!(b_pen.contains(&"AttendLecture".to_string()));

        let (b_bell, _) = object_followup_buttons(&bell, "鐘樓", "CalibrateClock", &[], &[]);
        assert!(b_bell.contains(&"CalibrateClock".to_string()));

        let (b_bench, _) = object_followup_buttons(&bench, "工坊", "FitMortiseTenon", &[], &[]);
        assert!(b_bench.contains(&"FitMortiseTenon".to_string()));

        let (b_trough, _) = object_followup_buttons(&trough, "鐵匠鋪", "QuenchBlade", &[], &[]);
        assert!(b_trough.contains(&"QuenchBlade".to_string()));

        let (b_rack, _) = object_followup_buttons(&rack, "布莊", "DyeIndigoCloth", &[], &[]);
        assert!(b_rack.contains(&"DyeIndigoCloth".to_string()));

        let (b_water, _) = object_followup_buttons(&water, "車馬行", "FeedDraftBeast", &[], &[]);
        assert!(b_water.contains(&"FeedDraftBeast".to_string()));

        let (b_cadastral, _) = object_followup_buttons(&cadastral, "房地契局", "InspectLandRegistry", &[], &[]);
        assert!(b_cadastral.contains(&"InspectLandRegistry".to_string()));

        let ship = resolve_room_object("風舶角碼頭", "wt_harbor_clockwise_ship")
            .expect("wt_harbor_clockwise_ship must resolve");
        assert!(ship.sockets.contains(&"BoardCoastalShip".to_string()));
        let (b_ship, _) = object_followup_buttons(&ship, "風舶角碼頭", "BoardCoastalShip", &[], &[]);
        assert!(b_ship.contains(&"BoardCoastalShip".to_string()));
        let gangplank = resolve_room_object("風舶角碼頭", "wt_dock_cargo_gangplank")
            .expect("wt_dock_cargo_gangplank must resolve");
        assert!(gangplank.sockets.contains(&"BoardCoastalShip".to_string()));
    }

    #[test]
    fn test_v03855_three_pillar_actions_and_contracts() {
        use super::{action_label_zh, normalize_action_key};

        // 1. Verify action_label_zh and normalize_action_key mappings for all 9 v0.38.55 actions
        let actions = [
            ("AssistKitchen", "幫廚備膳"),
            ("RoastMountainTea", "焙茶品茗"),
            ("SunBooks", "曝書防蠹"),
            ("RoastHerbs", "炮製藥材"),
            ("CollateInscriptions", "校勘碑帖"),
            ("GrindOpticLens", "研磨鏡片"),
            ("SelectDriedSeafood", "揀選海味"),
            ("DeliberateCivicPact", "參議鄉約"),
            ("DrawSpringWell", "汲水潤嗓"),
        ];
        for (key, zh) in actions {
            assert_eq!(action_label_zh(key), zh);
            assert_eq!(action_label_zh(zh), zh);
            assert_eq!(normalize_action_key(key), key);
            assert_eq!(normalize_action_key(zh), key);
        }

        // 2. Verify resolve_room_object for fallback objects
        let cauldron = resolve_room_object("食堂", "wt_dining_iron_cauldron")
            .expect("wt_dining_iron_cauldron must resolve");
        assert!(cauldron.sockets.contains(&"AssistKitchen".to_string()));

        let milk_stall = resolve_room_object("食堂", "wt_dining_rice_milk_stall")
            .expect("wt_dining_rice_milk_stall must resolve");
        assert!(milk_stall.sockets.contains(&"AssistKitchen".to_string()));

        let tea_stove = resolve_room_object("旅社茶屋", "wt_inn_milk_tea_stove")
            .expect("wt_inn_milk_tea_stove must resolve");
        assert!(tea_stove.sockets.contains(&"RoastMountainTea".to_string()));

        let book_shelf = resolve_room_object("藏書閣", "wt_book_printing_shelf")
            .expect("wt_book_printing_shelf must resolve");
        assert!(book_shelf.sockets.contains(&"SunBooks".to_string()));

        let herb_kiln = resolve_room_object("百草烘藥房", "wt_herb_drying_kiln")
            .expect("wt_herb_drying_kiln must resolve");
        assert!(herb_kiln.sockets.contains(&"RoastHerbs".to_string()));

        let herb_sieve = resolve_room_object("百草烘藥房", "wt_herb_bamboo_sieve")
            .expect("wt_herb_bamboo_sieve must resolve");
        assert!(herb_sieve.sockets.contains(&"RoastHerbs".to_string()));

        let desk_ink = resolve_room_object("校勘齋", "wt_study_desk_inkstone")
            .expect("wt_study_desk_inkstone must resolve");
        assert!(desk_ink.sockets.contains(&"CollateInscriptions".to_string()));
        assert!(desk_ink.sockets.contains(&"SunBooks".to_string()));

        let stele_cab = resolve_room_object("校勘齋", "wt_obj_stone_rubbing_cabinet")
            .expect("wt_obj_stone_rubbing_cabinet must resolve");
        assert!(stele_cab.sockets.contains(&"CollateInscriptions".to_string()));

        let tele_rack = resolve_room_object("觀星台", "wt_obs_brass_telescope_rack")
            .expect("wt_obs_brass_telescope_rack must resolve");
        assert!(tele_rack.sockets.contains(&"GrindOpticLens".to_string()));

        let map_table = resolve_room_object("閱覽室", "wt_book_map_reading_table")
            .expect("wt_book_map_reading_table must resolve");
        assert!(map_table.sockets.contains(&"GrindOpticLens".to_string()));

        let market_stand = resolve_room_object("海味行", "wt_market_produce_stand")
            .expect("wt_market_produce_stand must resolve");
        assert!(market_stand.sockets.contains(&"SelectDriedSeafood".to_string()));

        let market_scale = resolve_room_object("海味行", "wt_market_scale_counter")
            .expect("wt_market_scale_counter must resolve");
        assert!(market_scale.sockets.contains(&"SelectDriedSeafood".to_string()));

        let council_table = resolve_room_object("公議堂", "wt_hall_council_table")
            .expect("wt_hall_council_table must resolve");
        assert!(council_table.sockets.contains(&"DeliberateCivicPact".to_string()));

        let bulletin = resolve_room_object("公議堂", "wt_hall_bulletin_board")
            .expect("wt_hall_bulletin_board must resolve");
        assert!(bulletin.sockets.contains(&"DeliberateCivicPact".to_string()));

        let river_steps = resolve_room_object("水岸北街", "wt_river_stone_steps")
            .expect("wt_river_stone_steps must resolve");
        assert!(river_steps.sockets.contains(&"DrawSpringWell".to_string()));

        let stone_well = resolve_room_object("巷弄天井", "wt_alley_stone_well")
            .expect("wt_alley_stone_well must resolve");
        assert!(stone_well.sockets.contains(&"DrawSpringWell".to_string()));

        // 3. Verify object_followup_buttons keeps repeatable action buttons for all 9 actions
        let (b_cauldron, _) = object_followup_buttons(&cauldron, "食堂", "AssistKitchen", &[], &[]);
        assert!(b_cauldron.contains(&"AssistKitchen".to_string()));

        let (b_tea, _) = object_followup_buttons(&tea_stove, "旅社茶屋", "RoastMountainTea", &[], &[]);
        assert!(b_tea.contains(&"RoastMountainTea".to_string()));

        let (b_shelf, _) = object_followup_buttons(&book_shelf, "藏書閣", "SunBooks", &[], &[]);
        assert!(b_shelf.contains(&"SunBooks".to_string()));

        let (b_herb, _) = object_followup_buttons(&herb_kiln, "百草烘藥房", "RoastHerbs", &[], &[]);
        assert!(b_herb.contains(&"RoastHerbs".to_string()));

        let (b_desk, _) = object_followup_buttons(&desk_ink, "校勘齋", "CollateInscriptions", &[], &[]);
        assert!(b_desk.contains(&"CollateInscriptions".to_string()));

        let (b_tele, _) = object_followup_buttons(&tele_rack, "觀星台", "GrindOpticLens", &[], &[]);
        assert!(b_tele.contains(&"GrindOpticLens".to_string()));

        let (b_market, _) = object_followup_buttons(&market_stand, "海味行", "SelectDriedSeafood", &[], &[]);
        assert!(b_market.contains(&"SelectDriedSeafood".to_string()));

        let (b_council, _) = object_followup_buttons(&council_table, "公議堂", "DeliberateCivicPact", &[], &[]);
        assert!(b_council.contains(&"DeliberateCivicPact".to_string()));

        let (b_river, _) = object_followup_buttons(&river_steps, "水岸北街", "DrawSpringWell", &[], &[]);
        assert!(b_river.contains(&"DrawSpringWell".to_string()));

        // Also verify bulletin board followup contains DeliberateCivicPact
        let (b_bulletin, _) = object_followup_buttons(&bulletin, "公議堂", "DeliberateCivicPact", &[], &[]);
        assert!(b_bulletin.contains(&"DeliberateCivicPact".to_string()));

        // 4. Verify no design jargon in strings
        let prohibited_terms = ["SFC", "Stock-Flow", "RelationalHook", "WorldRippleFact", "Chebyshev", "\u{525b}\u{624d}", "\u{525b}\u{525b}", "\u{65b9}\u{624d}"];
        for &(key, zh) in &actions {
            for term in &prohibited_terms {
                assert!(!key.contains(term), "Key {} contains jargon {}", key, term);
                assert!(!zh.contains(term), "Label {} contains jargon {}", zh, term);
            }
        }
    }

    #[test]
    fn test_resolve_grid_scene_object_venue_counter_and_event_obj() {
        let coord_canteen = crate::grid::SquareCoord::new(990, 992);
        let canteen_room_id = format!("grid:{}:{}", coord_canteen.x, coord_canteen.y);
        let cell_canteen = crate::grid::GridCell::new(coord_canteen, crate::grid::Terrain::Workshop, "三水大食堂");

        let coord_mill = crate::grid::SquareCoord::new(991, 992);
        let mill_room_id = format!("grid:{}:{}", coord_mill.x, coord_mill.y);
        let mut cell_mill = crate::grid::GridCell::new(coord_mill, crate::grid::Terrain::Workshop, "老堰頭・水輪石磨工坊");
        cell_mill.tags.push("石磨".to_string());
        cell_mill.tags.push("水輪".to_string());

        let mut test_grid = crate::grid::SquareGrid::new();
        test_grid.insert(cell_canteen);
        test_grid.insert(cell_mill);
        crate::server::grid_manager::set_runtime_square_grid_for_testing(test_grid);

        // 1. Resolve wt_venue_counter on 三水大食堂 (has_counter_restricted_zone == true)
        let counter_obj = resolve_grid_scene_object(&canteen_room_id, "wt_venue_counter")
            .expect("wt_venue_counter must resolve for venue with counter restricted zone");
        assert_eq!(counter_obj.id, "wt_venue_counter");
        assert!(counter_obj.name.contains("當值櫃檯"));
        assert!(counter_obj.sockets.contains(&"Look".to_string()));
        assert!(counter_obj.sockets.contains(&"Inspect".to_string()));
        assert!(counter_obj.sockets.contains(&"Operate".to_string()));

        let inspect_res = counter_obj.responses.get("Inspect").cloned().unwrap_or_default();
        let operate_res = counter_obj.responses.get("Operate").cloned().unwrap_or_default();
        assert!(!inspect_res.contains("事件包") && !inspect_res.contains("劇本包"));
        assert!(inspect_res.contains("待辦記注"));
        assert!(operate_res.contains("示意協辦"));
        // 2. Resolve wt_event_obj_ws-001 by ID directly on 老堰頭・水輪石磨工坊
        let event_obj = resolve_grid_scene_object(&mill_room_id, "wt_event_obj_ws-001")
            .expect("wt_event_obj_ws-001 must resolve by event object id");
        assert!(event_obj.id.starts_with("wt_event_obj_ws-"));
        assert!(!event_obj.name.is_empty());
        assert!(event_obj.sockets.contains(&"Look".to_string()));
        assert!(event_obj.sockets.contains(&"Inspect".to_string()));
        assert!(event_obj.sockets.contains(&"Operate".to_string()));
        assert!(event_obj.sockets.contains(&"Use".to_string()));

        let ev_look = event_obj.responses.get("Look").cloned().unwrap_or_default();
        assert!(!ev_look.contains("事件包") && !ev_look.contains("劇本包"));
        assert!(ev_look.contains("場內顯眼處擺著「"));
    }
}

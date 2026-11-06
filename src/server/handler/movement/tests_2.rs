    #[test]
    fn m3_032_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("水尾寮水網口", 7);
        let from_field = route_reentry_log_for_room("水尾寮水網口", "秧苗田邊", 7)
            .expect("水尾寮水網口應有從秧苗田邊進入的路徑句");
        let from_warehouse = route_reentry_log_for_room("魚貨分揀場", "水倉前場", 7)
            .expect("魚貨分揀場應有從水倉返回的路徑句");
        assert_ne!(generic, from_field);
        assert!(from_field.contains("從秧苗田邊走到水尾寮水網口"));
        assert!(from_field.contains("田面蛙聲"));
        assert!(from_warehouse.contains("從水倉前場回到魚貨分揀場"));
        assert!(!from_field.contains('7'));
        assert!(!from_warehouse.contains('7'));
    }

    #[test]
    fn m3_032_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "水尾寮水網口"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "魚貨分揀場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "水倉前場"), "陌生人");
    }

    #[test]
    fn m3_045_entry_logs_have_granary_presence_not_counts() {
        let scale = player_layer_entry_log("糧倉外秤", 9);
        let gate = player_layer_entry_log("倉門前場", 9);

        assert!(scale.contains("糧倉外秤"));
        assert!(scale.contains("秤手"));
        assert!(gate.contains("倉門前場"));
        assert!(gate.contains("守倉人"));
        assert_eq!(crowd_label_for_scene("糧車泊位"), "糧車人群");
        assert!(!scale.contains('9'));
        assert!(!gate.contains('9'));
    }

    #[test]
    fn m3_045_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("糧倉外秤", 9);
        let from_downriver = route_reentry_log_for_room("糧倉外秤", "下游田埂路", 9)
            .expect("糧倉外秤應有從下游田埂路進入的路徑句");
        let from_cart = route_reentry_log_for_room("糧車泊位", "車行口", 9)
            .expect("糧車泊位應有從車行口進入的路徑句");

        assert_ne!(generic, from_downriver);
        assert!(from_downriver.contains("從下游田埂路走到糧倉外秤"));
        assert!(from_downriver.contains("秤盤"));
        assert!(from_cart.contains("從車行口走到糧車泊位"));
        assert!(!from_downriver.contains('9'));
        assert!(!from_cart.contains('9'));
    }

    #[test]
    fn m3_045_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "糧倉外秤"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "倉門前場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "糧車泊位"), "陌生人");
    }

    #[test]
    fn m3_048_entry_logs_have_estuary_port_presence_not_counts() {
        let dock = player_layer_entry_log("河口潮泥碼頭", 11);
        let tax = player_layer_entry_log("稅站棚", 11);

        assert!(dock.contains("河口潮泥碼頭"));
        assert!(dock.contains("碼頭工"));
        assert!(tax.contains("稅站棚"));
        assert!(tax.contains("稅吏"));
        assert_eq!(crowd_label_for_scene("商船候泊線"), "候泊人群");
        assert!(!dock.contains("11"));
        assert!(!tax.contains("11"));
    }

    #[test]
    fn m3_048_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("河口潮泥碼頭", 11);
        let from_granary = route_reentry_log_for_room("河口潮泥碼頭", "守倉棚", 11)
            .expect("河口潮泥碼頭應有從守倉棚進入的路徑句");
        let from_cargo = route_reentry_log_for_room("商船候泊線", "潮線貨場", 11)
            .expect("商船候泊線應有從潮線貨場進入的路徑句");

        assert_ne!(generic, from_granary);
        assert!(from_granary.contains("從守倉棚走到河口潮泥碼頭"));
        assert!(from_granary.contains("海口"));
        assert!(from_cargo.contains("從潮線貨場走到商船候泊線"));
        assert!(!from_granary.contains("11"));
        assert!(!from_cargo.contains("11"));
    }

    #[test]
    fn m3_048_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "河口潮泥碼頭"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "稅站棚"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "商船候泊線"), "陌生人");
    }

    #[test]
    fn m3_050_entry_logs_have_shipyard_presence_not_counts() {
        let gate = player_layer_entry_log("東南船塢口", 12);
        let repair = player_layer_entry_log("修船架", 12);
        let sailcloth = player_layer_entry_log("帆布曬場", 12);

        assert!(gate.contains("東南船塢口"));
        assert!(gate.contains("船匠"));
        assert!(repair.contains("修船架"));
        assert!(repair.contains("修船工"));
        assert!(sailcloth.contains("帆布曬場"));
        assert!(sailcloth.contains("補帆人"));
        assert!(!sailcloth.contains("翻穀人"));
        assert!(!sailcloth.contains("穀粒"));
        assert_eq!(crowd_label_for_scene("碼頭木階"), "木階人群");
        assert_eq!(crowd_label_for_scene("帆布曬場"), "補帆人群");
        assert!(!gate.contains("12"));
        assert!(!repair.contains("12"));
    }

    #[test]
    fn m3_050_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("東南船塢口", 12);
        let from_warehouse = route_reentry_log_for_room("東南船塢口", "風舶角港倉門", 12)
            .expect("東南船塢口應有從風舶角港倉門進入的路徑句");
        let from_repair = route_reentry_log_for_room("帆布曬場", "修船架", 12)
            .expect("帆布曬場應有從修船架進入的路徑句");
        let from_steps = route_reentry_log_for_room("船戶街口", "碼頭木階", 12)
            .expect("船戶街口應有從碼頭木階進入的路徑句");

        assert_ne!(generic, from_warehouse);
        assert!(from_warehouse.contains("從風舶角港倉門走到東南船塢口"));
        assert!(from_warehouse.contains("船港"));
        assert!(from_repair.contains("從修船架走到帆布曬場"));
        assert!(from_repair.contains("舊帆"));
        assert!(!from_repair.contains("翻穀"));
        assert!(from_steps.contains("從碼頭木階走到船戶街口"));
        assert!(!from_warehouse.contains("12"));
        assert!(!from_repair.contains("12"));
        assert!(!from_steps.contains("12"));
    }

    #[test]
    fn m3_050_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "東南船塢口"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "修船架"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "帆布曬場"), "陌生人");
    }

    #[test]
    fn m3_053_entry_logs_have_trade_port_presence_not_counts() {
        let dock = player_layer_entry_log("外貿碼頭", 9);
        let customs = player_layer_entry_log("舶司前棚", 9);
        let exchange = player_layer_entry_log("匯兌小屋", 9);

        assert!(dock.contains("外貿碼頭"));
        assert!(dock.contains("船員"));
        assert!(customs.contains("舶司前棚"));
        assert!(customs.contains("舶司吏"));
        assert!(exchange.contains("匯兌小屋"));
        assert!(exchange.contains("兌票人"));
        assert_eq!(crowd_label_for_scene("外貿碼頭"), "外貿人群");
        assert_eq!(crowd_label_for_scene("翻譯桌"), "翻譯人群");
        assert!(!dock.contains('9'));
        assert!(!customs.contains('9'));
        assert!(!exchange.contains('9'));
    }

    #[test]
    fn m3_053_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("外貿碼頭", 9);
        let from_sailcloth = route_reentry_log_for_room("外貿碼頭", "帆布曬場", 9)
            .expect("外貿碼頭應有從帆布曬場進入的路徑句");
        let from_customs = route_reentry_log_for_room("翻譯桌", "舶司前棚", 9)
            .expect("翻譯桌應有從舶司前棚進入的路徑句");
        let from_warehouse = route_reentry_log_for_room("匯兌小屋", "風舶角外洋棧倉", 9)
            .expect("匯兌小屋應有從風舶角外洋棧倉進入的路徑句");

        assert_ne!(generic, from_sailcloth);
        assert!(from_sailcloth.contains("從帆布曬場走到外貿碼頭"));
        assert!(from_sailcloth.contains("港旗"));
        assert!(from_customs.contains("從舶司前棚走到翻譯桌"));
        assert!(from_warehouse.contains("從風舶角外洋棧倉走到匯兌小屋"));
        assert!(!from_sailcloth.contains("9"));
        assert!(!from_customs.contains("9"));
        assert!(!from_warehouse.contains("9"));
    }

    #[test]
    fn m3_053_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "外貿碼頭"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "舶司前棚"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "翻譯桌"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "匯兌小屋"), "陌生人");
    }

    #[test]
    fn m3_006_entry_logs_have_north_harbor_presence_not_counts() {
        let dock = player_layer_entry_log("風舶角避風港", 8);
        let shed = player_layer_entry_log("港灣貨棚", 8);
        let watch = player_layer_entry_log("烏石鼻守望石階", 8);

        assert!(dock.contains("風舶角避風港"));
        assert!(dock.contains("船戶"));
        assert!(shed.contains("港灣貨棚"));
        assert!(shed.contains("搬魚人"));
        assert!(watch.contains("烏石鼻守望石階"));
        assert!(watch.contains("守望人"));
        assert_eq!(crowd_label_for_scene("冷霧魚市"), "魚市人群");
        assert_eq!(crowd_label_for_scene("漁網曬場"), "補網人群");
        assert!(!dock.contains('8'));
        assert!(!shed.contains('8'));
        assert!(!watch.contains('8'));
    }

    #[test]
    fn m3_006_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("風舶角避風港", 8);
        let from_farm = route_reentry_log_for_room("風舶角避風港", "農舍前坪", 8)
            .expect("風舶角避風港應有從農舍前坪進入的路徑句");
        let from_dock = route_reentry_log_for_room("港灣貨棚", "風舶角避風港", 8)
            .expect("港灣貨棚應有從風舶角避風港進入的路徑句");
        let from_market = route_reentry_log_for_room("烏石鼻守望石階", "冷霧魚市", 8)
            .expect("烏石鼻守望石階應有從冷霧魚市進入的路徑句");

        assert_ne!(generic, from_farm);
        assert!(from_farm.contains("從農舍前坪走到風舶角避風港"));
        assert!(from_farm.contains("冷海水"));
        assert!(from_dock.contains("從風舶角避風港走進港灣貨棚"));
        assert!(from_market.contains("從冷霧魚市踏上烏石鼻守望石階"));
        assert!(!from_farm.contains("8"));
        assert!(!from_dock.contains("8"));
        assert!(!from_market.contains("8"));
    }

    #[test]
    fn m3_006_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "港灣貨棚"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "漁網曬場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "冷霧魚市"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "烏石鼻守望石階"), "陌生人");
    }

    #[test]
    fn m3_009_entry_logs_have_mine_presence_not_counts() {
        let pit = player_layer_entry_log("礦坑外棚", 8);
        let wash = player_layer_entry_log("洗礦溝", 8);
        let load = player_layer_entry_log("紅土崁選礦車場", 8);

        assert!(pit.contains("礦坑外棚"));
        assert!(pit.contains("礦工"));
        assert!(wash.contains("洗礦溝"));
        assert!(wash.contains("洗礦工"));
        assert!(load.contains("紅土崁選礦車場"));
        assert!(load.contains("車夫"));
        assert_eq!(crowd_label_for_scene("粗料倉"), "粗料倉人群");
        assert_eq!(crowd_label_for_scene("礦渣坡"), "礦渣坡人群");
        assert!(!pit.contains('8'));
        assert!(!wash.contains('8'));
        assert!(!load.contains('8'));
    }

    #[test]
    fn m3_009_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("礦坑外棚", 8);
        let from_field = route_reentry_log_for_room("礦坑外棚", "田埂深處", 8)
            .expect("礦坑外棚應有從田埂深處進入的路徑句");
        let from_pit = route_reentry_log_for_room("洗礦溝", "礦坑外棚", 8)
            .expect("洗礦溝應有從礦坑外棚進入的路徑句");
        let from_border = route_reentry_log_for_room("紅土崁選礦車場", "邊城外門", 8)
            .expect("紅土崁選礦車場應有從邊城外門進入的路徑句");

        assert_ne!(generic, from_field);
        assert!(from_field.contains("從田埂深處走到礦坑外棚"));
        assert!(from_field.contains("吊燈光"));
        assert!(from_pit.contains("從礦坑外棚走到洗礦溝"));
        assert!(from_border.contains("從邊城外門走回紅土崁選礦車場"));
        assert!(!from_field.contains("8"));
        assert!(!from_pit.contains("8"));
        assert!(!from_border.contains("8"));
    }

    #[test]
    fn m3_009_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "礦坑外棚"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "洗礦溝"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "工具修棚"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "礦渣坡"), "陌生人");
        assert_eq!(observed_label_for_scene("錢七", "紅土崁選礦車場"), "陌生人");
    }

    #[test]
    fn final_map_entry_logs_have_scene_presence_not_counts() {
        for (room, role, crowd) in [
            ("邊城外門", "守門人", "邊門人群"),
            ("糧倉前場", "倉管", "糧倉人群"),
            ("城邦外坡道", "牽獸人", "坡道人群"),
            ("石壘門", "門衛", "門前人群"),
            ("門內券道", "引路人", "券道人群"),
            ("旗石廣場", "旗手", "廣場人群"),
            ("西門坡道街", "坡街販", "坡街人群"),
            ("議棚前階", "記牌人", "議棚人群"),
            ("鑄幣巷口", "兌牌人", "巷口人群"),
            ("傭工棚邊", "棚頭人", "傭工人群"),
            ("工坊街口", "學徒", "工坊人群"),
            ("牧地柵門", "牧人", "牧地人群"),
            ("牧場門牌", "牧場管事", "牧場人群"),
            ("邊鎮井口", "取水人", "井口人群"),
            ("車隊場", "車夫", "車隊人群"),
        ] {
            let entry = player_layer_entry_log(room, 8);
            let reentry = player_layer_reentry_log(room, 8);

            assert!(entry.contains(room), "{room}");
            assert!(entry.contains(role), "{room}");
            assert!(reentry.contains(room), "{room}");
            assert_eq!(crowd_label_for_scene(room), crowd, "{room}");
            assert!(!entry.contains('8'), "{room}");
            assert!(!reentry.contains('8'), "{room}");
        }
    }

    #[test]
    fn m3_020_public_presence_cards_are_stable_without_counts() {
        let mut cards = Vec::new();
        append_m3_020_public_presence_cards("旗石廣場", &mut cards);
        let labels: Vec<&str> = cards
            .iter()
            .filter_map(|card| card.get("display_name").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(labels, vec!["旗手", "廣場人群"]);
        for card in &cards {
            assert_eq!(
                card.get("kind").and_then(serde_json::Value::as_str),
                Some("crowd")
            );
            let label = card
                .get("display_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            assert!(!label.contains("還有"));
            assert!(!label.contains('8'));
        }

        append_m3_020_public_presence_cards("旗石廣場", &mut cards);
        assert_eq!(cards.len(), 2);

        let mut other_cards = Vec::new();
        append_m3_020_public_presence_cards("半嶺水岸渡口", &mut other_cards);
        assert!(other_cards.is_empty());
    }

    #[test]
    fn final_map_observed_labels_fall_back_to_public_roles() {
        for (room, _label) in [
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("青籠關邊貿坪", "商旅"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西門坡道街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("鍛造棚", "鐵匠"),
            ("油布庫", "看庫人"),
            ("畜欄口", "牲口販"),
            ("獸醫棚", "獸醫"),
            ("商隊泊位", "商隊人"),
            ("旅社前廊", "店伙"),
            ("補給市集", "小販"),
            ("巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_scene("張三", room), "陌生人", "{room}");
        }
    }

    #[test]
    fn resident_narrative_subjects_match_roles_and_attire() {
        let chen = VisibleNpcCard {
            id: "wt_npc_chen_wanfu",
            display_name: "陌生人".to_string(),
            card_state: "stranger",
            current_activity: "熬煮濃湯",
        };
        let chen_subj = narrative_person_subject(&chen, 0);
        assert!(chen_subj.contains("主廚"), "{chen_subj} should contain 主廚");
        assert!(chen_subj.contains("潔白雙排扣主廚服與厚布圍裙"), "{chen_subj} should contain 主廚服");
        assert!(!chen_subj.contains("外地商旅"), "{chen_subj} must not be 外地商旅");

        let lu = VisibleNpcCard {
            id: "wt_npc_lu_chengli",
            display_name: "陌生人".to_string(),
            card_state: "stranger",
            current_activity: "審理戶籍",
        };
        let lu_subj = narrative_person_subject(&lu, 0);
        assert!(lu_subj.contains("主事"), "{lu_subj} should contain 主事");
        assert!(!lu_subj.contains("匠人"), "{lu_subj} must not be 匠人");
    }

    #[test]
    fn test_weave_scene_fixtures_empty_desc_or_objects() {
        assert_eq!(weave_scene_fixtures_into_desc("", "茶屋", &[]), "");
        assert_eq!(weave_scene_fixtures_into_desc("  ", "茶屋", &[]), "  ");
        assert_eq!(
            weave_scene_fixtures_into_desc("室內安靜。", "茶屋", &[]),
            "室內安靜。"
        );
    }

    #[test]
    fn test_weave_scene_fixtures_skips_multiplication_and_empty() {
        let objs = vec![
            crate::server::protocol::ViewObject {
                id: "obj1".into(),
                name: "".into(),
                object_type: "".into(),
                actions: vec!["Operate".into()],
                tag: "".into(),
                category_tag: "mechanism".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
            crate::server::protocol::ViewObject {
                id: "obj2".into(),
                name: "粗麻布×3".into(),
                object_type: "".into(),
                actions: vec!["Operate".into()],
                tag: "".into(),
                category_tag: "furnishing".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
        ];
        assert_eq!(
            weave_scene_fixtures_into_desc("室內清幽。", "茶室", &objs),
            "室內清幽。"
        );
    }

    #[test]
    fn test_weave_scene_fixtures_indoor_single_and_multiple() {
        let single = vec![crate::server::protocol::ViewObject {
            id: "obj_tea_table".into(),
            name: "老杉木茶案".into(),
            object_type: "".into(),
            actions: vec!["Look".into()],
            tag: "".into(),
            category_tag: "furnishing".into(),
            attribute_tags: vec![],
            tags: vec![],
        }];
        assert_eq!(
            weave_scene_fixtures_into_desc("屋內整潔。", "茶行客堂", &single),
            "屋內整潔。隅處設有一方老杉木茶案。"
        );

        let double = vec![
            crate::server::protocol::ViewObject {
                id: "obj_table".into(),
                name: "老杉木茶案".into(),
                object_type: "".into(),
                actions: vec!["Look".into()],
                tag: "".into(),
                category_tag: "furnishing".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
            crate::server::protocol::ViewObject {
                id: "obj_pot".into(),
                name: "紫砂茶海".into(),
                object_type: "".into(),
                actions: vec!["Look".into()],
                tag: "".into(),
                category_tag: "vessel_container".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
        ];
        assert_eq!(
            weave_scene_fixtures_into_desc("屋內整潔。", "茶行客堂", &double),
            "屋內整潔。近旁設有老杉木茶案與紫砂茶海。"
        );
    }

    #[test]
    fn test_weave_scene_fixtures_waterfront_and_open_ground() {
        let waterfront_objs = vec![
            crate::server::protocol::ViewObject {
                id: "post".into(),
                name: "青石繫船樁".into(),
                object_type: "".into(),
                actions: vec!["Operate".into()],
                tag: "".into(),
                category_tag: "architecture".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
            crate::server::protocol::ViewObject {
                id: "flume".into(),
                name: "引水木槽".into(),
                object_type: "".into(),
                actions: vec!["Look".into()],
                tag: "".into(),
                category_tag: "mechanism".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
            crate::server::protocol::ViewObject {
                id: "rack".into(),
                name: "長竹篙架".into(),
                object_type: "".into(),
                actions: vec!["Look".into()],
                tag: "".into(),
                category_tag: "craft_tool".into(),
                attribute_tags: vec![],
                tags: vec![],
            },
        ];
        assert_eq!(
            weave_scene_fixtures_into_desc("渡口水闊。", "古渡頭", &waterfront_objs),
            "渡口水闊。岸畔置有青石繫船樁、引水木槽與長竹篙架。"
        );

        let street_objs = vec![crate::server::protocol::ViewObject {
            id: "mill".into(),
            name: "石碾".into(),
            object_type: "".into(),
            actions: vec!["MillGrain".into()],
            tag: "".into(),
            category_tag: "mechanism".into(),
            attribute_tags: vec![],
            tags: vec![],
        }];
        assert_eq!(
            weave_scene_fixtures_into_desc("街面安靜", "十字街口", &street_objs),
            "街面安靜。近旁立有石碾。"
        );
    }

    #[test]
    fn test_weave_scene_fixtures_already_mentioned_and_ground_loot() {
        let objs = vec![crate::server::protocol::ViewObject {
            id: "obj1".into(),
            name: "老杉木茶案".into(),
            object_type: "".into(),
            actions: vec!["Look".into()],
            tag: "".into(),
            category_tag: "furnishing".into(),
            attribute_tags: vec![],
            tags: vec![],
        }];
        assert_eq!(
            weave_scene_fixtures_into_desc("屋內擺有一方老杉木茶案。", "茶行客堂", &objs),
            "屋內擺有一方老杉木茶案。"
        );

        let suffix_obj = vec![crate::server::protocol::ViewObject {
            id: "obj2".into(),
            name: "雙環大石碾".into(),
            object_type: "".into(),
            actions: vec!["Operate".into()],
            tag: "".into(),
            category_tag: "mechanism".into(),
            attribute_tags: vec![],
            tags: vec![],
        }];
        assert_eq!(
            weave_scene_fixtures_into_desc("門前置有幾只石碾。", "水磨坊", &suffix_obj),
            "門前置有幾只石碾。"
        );

        let loot_obj = vec![crate::server::protocol::ViewObject {
            id: "knife".into(),
            name: "生鏽短刀".into(),
            object_type: "".into(),
            actions: vec!["Look".into(), "Move".into()],
            tag: "".into(),
            category_tag: "craft_tool".into(),
            attribute_tags: vec![],
            tags: vec![],
        }];
        assert_eq!(
            weave_scene_fixtures_into_desc("街面冷清。", "十字街口", &loot_obj),
            "街面冷清。"
        );
    }

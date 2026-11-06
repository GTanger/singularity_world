#[cfg(test)]
fn crowd_label_for_scene(room_name: &str) -> &'static str {
    scene_crowd_label(room_name).unwrap_or("人群")
}

#[cfg(test)]
fn m3_020_public_presence_for_room(room_name: &str) -> Option<(&'static str, &'static str)> {
    const M3_020_ROOM_NAMES: [&str; 8] = [
        "城邦外坡道",
        "石壘門",
        "門內券道",
        "旗石廣場",
        "西門坡道街",
        "議棚前階",
        "鑄幣巷口",
        "傭工棚邊",
    ];
    if !M3_020_ROOM_NAMES
        .iter()
        .any(|room| room_name.contains(room))
    {
        return None;
    }
    let role = game::final_map_public_role_for_room(room_name)?;
    let crowd = game::final_map_crowd_label_for_room(room_name)?;
    Some((role, crowd))
}

#[cfg(test)]
fn append_m3_020_public_presence_cards(room_name: &str, cards: &mut Vec<serde_json::Value>) {
    let Some((role, crowd)) = m3_020_public_presence_for_room(room_name) else {
        return;
    };
    push_public_presence_card(cards, role);
    push_public_presence_card(cards, crowd);
}

#[cfg(test)]
fn push_public_presence_card(cards: &mut Vec<serde_json::Value>, label: &'static str) {
    if cards
        .iter()
        .any(|card| card.get("display_name").and_then(serde_json::Value::as_str) == Some(label))
    {
        return;
    }
    cards.push(serde_json::json!({
        "kind": "crowd",
        "display_name": label,
        "card_state": "crowd"
    }));
}

#[cfg(test)]
fn player_layer_entry_log(room_name: &str, stranger_total: usize) -> String {
    player_layer_entry_log_for_scene(room_name, stranger_total, 0, None)
}


#[cfg(test)]
fn route_reentry_log_for_room(
    room_name: &str,
    from_room_name: &str,
    stranger_total: usize,
) -> Option<&'static str> {
    if stranger_total == 0 {
        return None;
    }
    scene_route_reentry_logs(room_name)
        .into_iter()
        .find(|(from, _)| *from == from_room_name)
        .map(|(_, line)| line)
}

#[cfg(test)]
fn observed_label_for_scene(_raw_name: &str, _room_name: &str) -> String {
    "陌生人".to_string()
}

#[cfg(test)]
fn scene_crowd_label(room_name: &str) -> Option<&'static str> {
    if let Some(label) = game::final_map_crowd_label_for_room(room_name) {
        Some(label)
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        Some("候渡人群")
    } else if room_name.contains("水網入口") {
        Some("水網人群")
    } else if room_name.contains("湖港木棧") {
        Some("湖港人群")
    } else if room_name.contains("短船泊位") {
        Some("短船人群")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some("分揀人群")
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some("蘆棚人群")
    } else if room_name.contains("水倉") {
        Some("水倉人群")
    } else if room_name.contains("糧倉外秤") {
        Some("外秤人群")
    } else if room_name.contains("倉門前場") {
        Some("倉門人群")
    } else if room_name.contains("曬場邊") {
        Some("曬場人群")
    } else if room_name.contains("車行口") {
        Some("車行人群")
    } else if room_name.contains("守倉棚") {
        Some("守倉人群")
    } else if room_name.contains("糧車泊位") {
        Some("糧車人群")
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some("河口人群")
    } else if room_name.contains("稅站棚") {
        Some("稅站人群")
    } else if room_name.contains("船行門面") {
        Some("船行人群")
    } else if room_name.contains("潮線貨場") {
        Some("貨場人群")
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some("港倉門人群")
    } else if room_name.contains("商船候泊線") {
        Some("候泊人群")
    } else if room_name.contains("東南船塢口") {
        Some("船塢人群")
    } else if room_name.contains("修船架") {
        Some("修船人群")
    } else if room_name.contains("補給倉") {
        Some("補給人群")
    } else if room_name.contains("碼頭木階") {
        Some("木階人群")
    } else if room_name.contains("帆布曬場") {
        Some("補帆人群")
    } else if room_name.contains("船戶街口") {
        Some("船戶人群")
    } else if room_name.contains("外貿碼頭") {
        Some("外貿人群")
    } else if room_name.contains("舶司前棚") {
        Some("舶司人群")
    } else if room_name.contains("商館街口") {
        Some("商館人群")
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some("外洋棧倉人群")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯人群")
    } else if room_name.contains("匯兌小屋") {
        Some("匯兌人群")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some("避風港人群")
    } else if room_name.contains("港灣貨棚") {
        Some("貨棚人群")
    } else if room_name.contains("漁網曬場") {
        Some("補網人群")
    } else if room_name.contains("冷霧魚市") {
        Some("魚市人群")
    } else if room_name.contains("船戶屋排") {
        Some("船戶屋排人群")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("守望人群")
    } else if room_name.contains("礦坑外棚") {
        Some("礦坑人群")
    } else if room_name.contains("洗礦溝") {
        Some("洗礦人群")
    } else if room_name.contains("粗料倉") {
        Some("粗料倉人群")
    } else if room_name.contains("工具修棚") {
        Some("修具人群")
    } else if room_name.contains("礦渣坡") {
        Some("礦渣坡人群")
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("裝車人群")
    } else if room_name.contains("谷地路口")
        || room_name.contains("北堤")
        || room_name.contains("外谷地路")
        || room_name.contains("南側谷地路")
        || room_name.contains("渡口貨路")
        || room_name.contains("貨路外彎")
    {
        Some("行路人群")
    } else if room_name.contains("田埂") || room_name.contains("上田") {
        Some("田埂人群")
    } else if room_name.contains("挑擔小徑") {
        Some("挑擔人群")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") || room_name.contains("市集") {
        Some("市集人群")
    } else if room_name.contains("歇腳") {
        Some("歇腳人群")
    } else if room_name.contains("踏板") {
        Some("候船人群")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("上岸人群")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("船邊人群")
    } else if room_name.contains("倉棚") {
        Some("倉棚人群")
    } else {
        None
    }
}



    fn npc_card(id: &'static str, label: &str, card_state: &'static str) -> NpcCard<'static> {
        NpcCard {
            id,
            label: label.to_string(),
            card_state,
            current_activity: "",
            interaction_priority: false,
        }
    }

    #[test]
    fn folds_unnamed_people_without_hiding_named_cards() {
        let cards = vec![
            npc_card("o1", "候渡者", "observed"),
            npc_card("o2", "候渡者", "observed"),
            npc_card("o3", "候渡者", "observed"),
            npc_card("o4", "候渡者", "observed"),
            npc_card("o5", "候渡者", "observed"),
            npc_card("s1", "行人", "stranger"),
            npc_card("s2", "行人", "stranger"),
            npc_card("s3", "行人", "stranger"),
            npc_card("n1", "渡工江季良", "named"),
        ];

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(!has_hidden_unnamed);
        assert_eq!(visible.len(), 9);
        assert!(visible.iter().any(|card| card.id == "n1"));
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.display_name.starts_with("候渡者"))
                .count(),
            5
        );
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.display_name.starts_with("行人"))
                .count(),
            3
        );
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.card_state != "named")
                .count(),
            8
        );
    }

    #[test]
    fn does_not_fold_small_unnamed_groups() {
        let cards = vec![
            npc_card("o1", "候渡者", "observed"),
            npc_card("o2", "候渡者", "observed"),
            npc_card("s1", "行人", "stranger"),
        ];

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(!has_hidden_unnamed);
        let names: Vec<&str> = visible
            .iter()
            .map(|card| card.display_name.as_str())
            .collect();
        assert_eq!(names, vec!["候渡者1", "候渡者2", "行人"]);
    }

    #[test]
    fn keeps_available_market_operator_out_of_folded_crowd() {
        let mut cards = vec![
            npc_card("s1", "行人", "stranger"),
            npc_card("s2", "行人", "stranger"),
            npc_card("s3", "行人", "stranger"),
            npc_card("s4", "行人", "stranger"),
            npc_card("s5", "行人", "stranger"),
            npc_card("market", "行人", "stranger"),
        ];
        cards[5].interaction_priority = true;

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(!has_hidden_unnamed);
        assert!(visible.iter().any(|card| card.id == "market"));
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.card_state != "named")
                .count(),
            6
        );
    }

    #[test]
    fn north_bank_scene_changes_with_time_without_losing_place_anchor() {
        let empty_events: Vec<String> = Vec::new();
        let dawn = north_bank_waiting_scene_desc(6 * 60, Some(("clear", 0)), 8, &empty_events);
        let night = north_bank_waiting_scene_desc(22 * 60, Some(("clear", 0)), 8, &empty_events);

        assert_ne!(dawn, night);
        assert!(dawn.contains("半嶺水岸渡口"));
        assert!(night.contains("半嶺水岸渡口"));
        assert!(dawn.contains("晨霧") || dawn.contains("天光"));
        assert!(night.contains("夜水") || night.contains("夜風"));
    }

    #[test]
    fn north_bank_scene_projects_climate_and_crowd_without_exact_count() {
        let empty_events: Vec<String> = Vec::new();
        let rainy_crowd =
            north_bank_waiting_scene_desc(13 * 60, Some(("rain", 2)), 14, &empty_events);

        assert!(rainy_crowd.contains("雨後") || rainy_crowd.contains("濕氣"));
        assert!(rainy_crowd.contains("候渡的人群") || rainy_crowd.contains("市集邊"));
        assert!(!rainy_crowd.contains("14"));
    }

    #[test]
    fn north_bank_scene_projects_object_surfaces_as_current_state() {
        let empty_events: Vec<String> = Vec::new();
        let desc = north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 8, &empty_events);

        assert!(desc.contains("長凳"));
        assert!(desc.contains("湯"));
        assert!(desc.contains("貨擔"));
        assert!(desc.contains("船繩") || desc.contains("踏板"));
        for forbidden in ["登入後", "資料庫", "debug", "還有"] {
            assert!(!desc.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn north_bank_scene_projects_ferry_phase() {
        let empty_events: Vec<String> = Vec::new();
        let north_docked =
            north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 8, &empty_events);
        let south_docked =
            north_bank_waiting_scene_desc(8 * 60 + 20, Some(("clear", 0)), 8, &empty_events);

        assert_ne!(north_docked, south_docked);
        assert!(north_docked.contains("船腹碰岸") || north_docked.contains("濕繩繃"));
        assert!(south_docked.contains("南岸那頭") || south_docked.contains("對岸船邊"));
    }

    #[test]
    fn north_bank_scene_casts_recent_events_as_surface_traces_without_names() {
        let events = vec!["王小明，往南岸離開".to_string()];
        let desc = north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 4, &events);

        assert!(desc.contains("濕腳印") || desc.contains("空出"));
        assert!(!desc.contains("王小明"));
    }

    #[test]
    fn north_bank_entry_log_is_multiline_scene_flow_without_self_projection() {
        let entry = player_layer_entry_log("半嶺水岸渡口", 8);
        let lines: Vec<&str> = entry
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        assert!(lines.len() >= 3, "北岸第一屏入場動態應是可閱讀的多行場景流");
        assert!(entry.contains("船繩"));
        assert!(entry.contains("市集"));
        assert!(entry.contains("等船人群") || entry.contains("候渡"));
        for forbidden in ["你", "現場動態", "登入後", "debug", "資料庫", "8"] {
            assert!(!entry.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn north_bank_entry_scene_flow_changes_with_time_weather_and_ferry_phase() {
        let fog_dawn = player_layer_entry_log_for_scene("半嶺水岸渡口", 8, 6 * 60, Some(("fog", 11)));
        let clear_night =
            player_layer_entry_log_for_scene("半嶺水岸渡口", 8, 23 * 60 + 40, Some(("clear", 11)));

        assert_ne!(fog_dawn, clear_night);
        assert_eq!(
            fog_dawn
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            3
        );
        assert_eq!(
            clear_night
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            3
        );
        assert!(fog_dawn.contains('霧'));
        assert!(clear_night.contains('夜') || clear_night.contains("燈影"));
        for text in [fog_dawn, clear_night] {
            for forbidden in ["你", "現場動態", "debug", "資料庫"] {
                assert!(!text.contains(forbidden), "{forbidden}");
            }
        }
    }

    #[test]
    fn north_bank_reentry_log_is_continuous_not_first_entry_replay() {
        let first = player_layer_entry_log("半嶺水岸渡口", 8);
        let again = player_layer_reentry_log("半嶺水岸渡口", 8);

        assert_ne!(first, again);
        assert!(again.contains("回到半嶺水岸渡口"));
        assert!(!again.contains("站定"));
        assert!(!again.contains('8'));
    }

    #[test]
    fn north_bank_reentry_log_can_reflect_previous_room() {
        let generic = player_layer_reentry_log("半嶺水岸渡口", 8);
        let from_market = route_reentry_log_for_room("半嶺水岸渡口", "水尾寮渡頭市集", 8)
            .expect("北岸應有從市集返回的路徑句");

        assert_ne!(generic, from_market);
        assert!(from_market.contains("從水尾寮渡頭市集那頭回到半嶺水岸渡口"));
        assert!(from_market.contains("價牌"));
        assert!(!from_market.contains('8'));
    }

    #[test]
    fn outer_road_entry_logs_have_route_presence_not_counts() {
        let road = player_layer_entry_log("北堤泥路", 1);
        let field = player_layer_entry_log("田埂水口", 1);

        assert!(road.contains("北堤泥路"));
        assert!(road.contains("挑擔腳印"));
        assert!(field.contains("田埂水口"));
        assert!(field.contains("細水"));
        assert!(!road.contains('1'));
        assert!(!field.contains('1'));
    }

    #[test]
    fn outer_road_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("北堤泥路", 1);
        let from_crossroad = route_reentry_log_for_room("北堤泥路", "中河谷地路口", 1)
            .expect("北堤泥路應有從谷地路口進入的路徑句");

        assert_ne!(generic, from_crossroad);
        assert!(from_crossroad.contains("從中河谷地路口踏上北堤泥路"));
        assert!(from_crossroad.contains("渡口人聲"));
        assert!(!from_crossroad.contains('1'));
    }

    #[test]
    fn market_outer_road_does_not_use_market_body_log() {
        let entry = player_layer_entry_log("水尾寮外谷地路", 1);

        assert!(entry.contains("水尾寮外谷地路"));
        assert!(entry.contains("挑擔"));
        assert!(!entry.contains("走進水尾寮渡頭市集"));
    }

    #[test]
    fn guard_shed_scene_keeps_own_description_not_ferry_warehouse() {
        let desc = player_layer_scene_desc(
            "守倉棚半遮著冊簿和茶壺，棚柱旁掛著值牌。",
            "守倉棚",
            18 * 60,
            Some(("clear", 0)),
            3,
            &[],
        );

        assert!(desc.contains("守倉棚"));
        assert!(desc.contains("冊簿"));
        assert!(!desc.contains("倉棚前場"));
        assert!(!desc.contains("下一趟船"));
    }

    #[test]
    fn boathouse_street_scene_keeps_own_description_not_ferry_work_point() {
        let desc = player_layer_scene_desc(
            "船戶街口擠著招工牌、飯攤和晾著鹽味衣物的繩線。",
            "船戶街口",
            6 * 60,
            Some(("clear", 0)),
            3,
            &[],
        );

        assert!(desc.contains("船戶街口"));
        assert!(desc.contains("招工牌"));
        assert!(!desc.contains("船戶作業點"));
        assert!(!desc.contains("撐篙"));
    }

    #[test]
    fn m3_032_entry_logs_have_waterwork_presence_not_counts() {
        let water_entry = player_layer_entry_log("水尾寮水網口", 7);
        let fish_sorting = player_layer_entry_log("魚貨分揀場", 7);

        assert!(water_entry.contains("水尾寮水網口"));
        assert!(water_entry.contains("引船人"));
        assert!(fish_sorting.contains("魚貨分揀場"));
        assert!(fish_sorting.contains("分揀工"));
        assert_eq!(crowd_label_for_scene("水倉前場"), "水倉人群");
        assert!(!water_entry.contains('7'));
        assert!(!fish_sorting.contains('7'));
    }


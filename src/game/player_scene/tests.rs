#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_current_calendar_and_season() {
        let cal = current_calendar();
        assert!(["春融", "夏炎", "秋狩", "冬凝"].contains(&cal.season_name));
        assert!(cal.year >= 1);
        assert!(cal.day_in_season >= 1 && cal.day_in_season <= 92);
    }
    #[test]
    fn public_labels_cover_m3_027_scenes() {
        assert_eq!(public_person_label_for_room("半嶺水岸渡口"), Some("候渡者"));
        assert_eq!(public_person_label_for_room("水尾寮渡頭市集"), Some("看貨人"));
        assert_eq!(public_person_label_for_room("船戶作業點"), Some("渡工"));
        assert_eq!(public_person_label_for_room("倉棚前場"), Some("搬貨人"));
    }

    #[test]
    fn public_labels_cover_m3_032_scenes() {
        assert_eq!(public_person_label_for_room("水尾寮水網口"), Some("引船人"));
        assert_eq!(public_person_label_for_room("水網入口"), Some("引船人"));
        assert_eq!(public_person_label_for_room("湖港木棧"), Some("候船者"));
        assert_eq!(public_person_label_for_room("短船泊位"), Some("船戶"));
        assert_eq!(public_person_label_for_room("魚貨分揀場"), Some("分揀工"));
        assert_eq!(public_person_label_for_room("蘆草貨棚"), Some("採集者"));
        assert_eq!(public_person_label_for_room("水倉前場"), Some("倉管"));
    }

    #[test]
    fn public_labels_cover_m3_045_scenes() {
        assert_eq!(public_person_label_for_room("糧倉外秤"), Some("秤手"));
        assert_eq!(public_person_label_for_room("倉門前場"), Some("守倉人"));
        assert_eq!(public_person_label_for_room("曬場邊"), Some("翻穀人"));
        assert_eq!(public_person_label_for_room("車行口"), Some("車夫"));
        assert_eq!(public_person_label_for_room("守倉棚"), Some("倉管"));
        assert_eq!(public_person_label_for_room("糧車泊位"), Some("搬糧人"));
    }

    #[test]
    fn public_labels_cover_m3_048_scenes() {
        assert_eq!(public_person_label_for_room("河口潮泥碼頭"), Some("碼頭工"));
        assert_eq!(public_person_label_for_room("稅站棚"), Some("稅吏"));
        assert_eq!(public_person_label_for_room("船行門面"), Some("船行人"));
        assert_eq!(public_person_label_for_room("潮線貨場"), Some("搬貨人"));
        assert_eq!(public_person_label_for_room("風舶角港倉門"), Some("倉管"));
        assert_eq!(public_person_label_for_room("港倉門"), Some("倉管"));
        assert_eq!(public_person_label_for_room("商船候泊線"), Some("船員"));
    }

    #[test]
    fn public_labels_cover_m3_050_scenes() {
        assert_eq!(public_person_label_for_room("東南船塢口"), Some("船匠"));
        assert_eq!(public_person_label_for_room("修船架"), Some("修船工"));
        assert_eq!(public_person_label_for_room("補給倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("碼頭木階"), Some("船客"));
        assert_eq!(public_person_label_for_room("帆布曬場"), Some("補帆人"));
        assert_eq!(public_person_label_for_room("船戶街口"), Some("船戶"));
    }

    #[test]
    fn public_labels_cover_m3_053_scenes() {
        assert_eq!(public_person_label_for_room("外貿碼頭"), Some("船員"));
        assert_eq!(public_person_label_for_room("舶司前棚"), Some("舶司吏"));
        assert_eq!(public_person_label_for_room("商館街口"), Some("商館人"));
        assert_eq!(public_person_label_for_room("風舶角外洋棧倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("外洋棧倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("翻譯桌"), Some("翻譯"));
        assert_eq!(public_person_label_for_room("匯兌小屋"), Some("兌票人"));
    }

    #[test]
    fn public_labels_cover_m3_006_scenes() {
        assert_eq!(public_person_label_for_room("風舶角避風港"), Some("船戶"));
        assert_eq!(public_person_label_for_room("避風港"), Some("船戶"));
        assert_eq!(public_person_label_for_room("港灣貨棚"), Some("搬魚人"));
        assert_eq!(public_person_label_for_room("漁網曬場"), Some("補網人"));
        assert_eq!(public_person_label_for_room("冷霧魚市"), Some("魚販"));
        assert_eq!(public_person_label_for_room("船戶屋排"), Some("船戶家屬"));
        assert_eq!(public_person_label_for_room("烏石鼻守望石階"), Some("守望人"));
        assert_eq!(public_person_label_for_room("守望石階"), Some("守望人"));
    }

    #[test]
    fn public_labels_cover_m3_009_scenes() {
        assert_eq!(public_person_label_for_room("礦坑外棚"), Some("礦工"));
        assert_eq!(public_person_label_for_room("洗礦溝"), Some("洗礦工"));
        assert_eq!(public_person_label_for_room("粗料倉"), Some("倉管"));
        assert_eq!(public_person_label_for_room("工具修棚"), Some("修具工"));
        assert_eq!(public_person_label_for_room("礦渣坡"), Some("推車人"));
        assert_eq!(public_person_label_for_room("紅土崁選礦車場"), Some("車夫"));
        assert_eq!(public_person_label_for_room("選礦車場"), Some("車夫"));
    }

    #[test]
    fn public_labels_cover_final_map_stub_replacement_scenes() {
        for (room, label) in [
            ("文曜鎮口", "守門人"),
            ("文曜東市街", "看貨人"),
            ("文曜旅社前廊", "接待員"),
            ("文曜講堂前坪", "傳話人"),
            ("文曜井邊巷", "取水人"),
            ("文曜百工坊", "學徒"),
            ("文曜車馬行", "車夫"),
            ("文曜倉場門口", "倉管"),
            ("南城外驛道", "驛夫"),
            ("南城甕門", "門卒"),
            ("門內集散廣場", "引路人"),
            ("中軸大街南段", "攤主"),
            ("中軸大街中段", "巡街人"),
            ("外區坊門", "坊門看守"),
            ("三水總商行長廊", "商會書手"),
            ("大十字通衢", "指路人"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西門坡道街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("下營邊貿坪", "商旅"),
            ("糧倉前場", "倉管"),
            ("乾草棚", "馬夫"),
            ("巡邏院", "巡哨"),
            ("工坊街口", "學徒"),
            ("鍛造棚", "鐵匠"),
            ("料場秤台", "秤手"),
            ("修車棚", "修車工"),
            ("石粉堆", "搬料人"),
            ("油布庫", "看庫人"),
            ("牧地柵門", "牧人"),
            ("水槽棚", "牧童"),
            ("乾草堆", "搬草人"),
            ("畜欄口", "牲口販"),
            ("獸醫棚", "獸醫"),
            ("季節草場", "牧人"),
            ("牧場門牌", "牧場管事"),
            ("馬欄前場", "馬夫"),
            ("草料棚", "草料工"),
            ("獸醫點", "獸醫"),
            ("馱具修棚", "修具工"),
            ("商隊泊位", "商隊人"),
            ("邊鎮井口", "取水人"),
            ("低牆門洞", "守門人"),
            ("旅社前廊", "接待員"),
            ("補給市集", "小販"),
            ("巡哨棚", "巡哨"),
            ("車隊場", "車夫"),
        ] {
            assert_eq!(public_person_label_for_room(room), Some(label), "{room}");
            assert!(is_final_map_public_role(label), "{label}");
        }
    }

    #[test]
    fn m3_020_scene_objects_are_clickable_player_facing_surface() {
        for (room, expected_name, expected_fragment, expected_action) in [
            ("石壘門", "通行牌", "商隊", "Read"),
            ("旗石廣場", "等候線", "西門坡道街", "Use"),
            ("鑄幣巷口", "半開鑄幣門", "銅秤", "Open"),
            ("傭工棚邊", "短工牌", "活計", "Read"),
            ("議棚前階", "議棚告示", "欠糧", "Read"),
        ] {
            let objects = player_layer_scene_objects_for_room(room);
            assert_eq!(objects.len(), 1, "{room}");
            let obj = &objects[0];
            assert_eq!(obj.name, expected_name, "{room}");
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{room}");
            assert!(
                obj.sockets.iter().any(|socket| socket == expected_action),
                "{room}: {expected_action}"
            );
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(expected_fragment), "{room}: {look}");
            let followup = obj
                .responses
                .get(expected_action)
                .cloned()
                .unwrap_or_default();
            assert!(!followup.is_empty(), "{room}: {expected_action}");
            for text in [look, followup] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{room}: {forbidden}");
                }
            }
        }
        assert!(player_layer_scene_objects_for_room("城邦外坡道").is_empty());
    }

    #[test]
    fn north_bank_objects_are_playable_state_surfaces() {
        let objects = player_layer_scene_objects_for_room("半嶺水岸渡口");
        let expected = [
            ("河霧蘆叢", "Gather", "蘆叢"),
            ("濕長凳", "Rest", "包袱"),
            ("熱湯攤", "BuySoup", "熱氣"),
            ("熱湯攤", "QueueBroth", "熱氣"),
            ("貨擔", "CarryCargo", "踏板"),
            ("船繩", "HelpRope", "木樁"),
            ("避風棚", "DryOff", "舊帆布"),
            ("炭火盆", "WarmHands", "煙氣"),
            ("候渡人龍", "TalkQueue", "船期"),
            ("候渡人龍", "QueueFerry", "木牌"),
        ];

        for (name, action, look_fragment) in expected {
            let obj = objects
                .iter()
                .find(|obj| obj.name == name)
                .unwrap_or_else(|| panic!("missing north bank object: {name}"));
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{name}");
            assert!(obj.sockets.iter().any(|socket| socket == action), "{name}");
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(look_fragment), "{name}: {look}");
            let action_text = obj.responses.get(action).cloned().unwrap_or_default();
            assert!(!action_text.is_empty(), "{name}: {action}");
            for text in [look, action_text] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{name}: {forbidden}");
                }
            }
        }
    }

    #[test]
    fn north_bank_first_screen_objects_cover_m3_028_start_paths() {
        let objects = player_layer_scene_objects_for_room("半嶺水岸渡口");
        let has_action = |name: &str, action: &str| {
            objects
                .iter()
                .any(|obj| obj.name == name && obj.sockets.iter().any(|socket| socket == action))
        };

        assert!(has_action("貨擔", "CarryCargo")); // 賺錢 / 接活
        assert!(has_action("候渡人龍", "QueueFerry")); // 通行
        assert!(has_action("河霧蘆叢", "Gather")); // 材料 / 交易準備
        assert!(has_action("船繩", "HelpRope")); // 工作手勢 / 熟練線索
        assert!(has_action("熱湯攤", "QueueBroth") || has_action("濕長凳", "Rest")); // 歇腳處理

        let rope = objects
            .iter()
            .find(|obj| obj.name == "船繩")
            .expect("船繩應在北岸第一屏");
        let rope_look = rope.responses.get("Look").cloned().unwrap_or_default();
        let rope_action = rope.responses.get("HelpRope").cloned().unwrap_or_default();
        assert!(rope_look.contains("手勢"));
        assert!(rope_action.contains("記住"));
    }

    #[test]
    fn m3_027_cross_scene_objects_are_playable_state_surfaces() {
        for (room, expected_name, expected_action, look_fragment) in [
            ("水尾寮渡頭市集", "舊麻袋堆", "Gather", "包貨"),
            ("水尾寮渡頭市集", "市集包貨單", "SellReeds", "蘆草"),
            ("水尾寮渡頭市集", "市集包貨單", "SellSackcloth", "麻袋布"),
            ("水尾寮渡頭市集", "市集包貨單", "SellRope", "麻繩"),
            ("渡船踏板", "濕繩圈", "Gather", "捆貨"),
            ("渡船踏板", "踏板木牌", "BoardFerry", "排到牌內側"),
            ("南岸泥坡", "上岸繩", "StepAshore", "泥坡木樁"),
            ("倉棚前場", "倉棚點貨牌", "CheckCargo", "已送到踏板邊"),
        ] {
            let objects = player_layer_scene_objects_for_room(room);
            let obj = objects
                .iter()
                .find(|obj| obj.name == expected_name)
                .unwrap_or_else(|| panic!("missing M3-027 object: {room} / {expected_name}"));
            assert!(obj.sockets.iter().any(|socket| socket == "Look"), "{room}");
            assert!(
                obj.sockets.iter().any(|socket| socket == expected_action),
                "{room}: {expected_action}"
            );
            let look = obj.responses.get("Look").cloned().unwrap_or_default();
            assert!(look.contains(look_fragment), "{room}: {look}");
            let action_text = obj
                .responses
                .get(expected_action)
                .cloned()
                .unwrap_or_default();
            assert!(!action_text.is_empty(), "{room}: {expected_action}");
            for text in [look, action_text] {
                for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"]
                {
                    assert!(!text.contains(forbidden), "{room}: {forbidden}");
                }
            }
        }
    }

    #[test]
    fn micro_lines_do_not_use_private_names() {
        for room in [
            "半嶺水岸渡口",
            "文曜鎮口",
            "文曜東市街",
            "文曜旅社前廊",
            "文曜講堂前坪",
            "文曜井邊巷",
            "文曜百工坊",
            "文曜車馬行",
            "文曜倉場門口",
            "南城外驛道",
            "南城甕門",
            "門內集散廣場",
            "中軸大街南段",
            "中軸大街中段",
            "外區坊門",
            "三水總商行長廊",
            "大十字通衢",
            "城邦外坡道",
            "石壘門",
            "門內券道",
            "旗石廣場",
            "西門坡道街",
            "議棚前階",
            "鑄幣巷口",
            "傭工棚邊",
            "水尾寮水網口",
            "魚貨分揀場",
            "水倉前場",
            "糧倉外秤",
            "守倉棚",
            "糧車泊位",
            "河口潮泥碼頭",
            "稅站棚",
            "商船候泊線",
            "東南船塢口",
            "修船架",
            "帆布曬場",
            "外貿碼頭",
            "舶司前棚",
            "商館街口",
            "翻譯桌",
            "匯兌小屋",
            "風舶角外洋棧倉",
            "風舶角港倉門",
            "風舶角避風港",
            "港灣貨棚",
            "漁網曬場",
            "冷霧魚市",
            "船戶屋排",
            "烏石鼻守望石階",
            "礦坑外棚",
            "洗礦溝",
            "粗料倉",
            "工具修棚",
            "礦渣坡",
            "紅土崁選礦車場",
            "邊城外門",
            "哨樓梯",
            "下營邊貿坪",
            "糧倉前場",
            "乾草棚",
            "巡邏院",
            "工坊街口",
            "鍛造棚",
            "料場秤台",
            "修車棚",
            "石粉堆",
            "油布庫",
            "牧地柵門",
            "水槽棚",
            "乾草堆",
            "畜欄口",
            "獸醫棚",
            "季節草場",
            "牧場門牌",
            "馬欄前場",
            "草料棚",
            "獸醫點",
            "馱具修棚",
            "商隊泊位",
            "邊鎮井口",
            "低牆門洞",
            "旅社前廊",
            "補給市集",
            "巡哨棚",
            "車隊場",
        ] {
            let lines = micro_interaction_lines_for_room(room);
            assert!(lines.iter().all(|line| !line.contains('【')), "{room}");
        }
    }
    #[test]
    fn white_tower_peninsula_rooms_have_rich_playable_objects() {
        let sample_rooms = [
            "文曜・三水大食堂",
            "文曜・泉香溫水浴場",
            "文曜・東橋旅社",
            "文曜・三水口石砌住宅",
            "文曜・雲錦織造與服飾坊",
            "文曜・巧木居所傢俱坊",
            "文曜・岑氏第一鍛造工坊",
            "文曜・藏書閣與風物誌編纂處",
            "文曜・講堂街書局",
            "文曜・學院主校區",
            "文曜・溫氏脈絡診所",
            "老堰頭・水輪石磨工坊",
            "霜林口・原木水車鋸木坊",
            "半嶺鎮・寒水玉切石工坊",
            "半嶺鎮・半嶺氣象觀測所",
            "半嶺鎮・半嶺試招場",
            "大圳頭・大圳農具與百工坊",
            "牛埔・羊毛與乳酪精製坊",
            "紅土崁・紅土崁選礦場",
            "三號坑口・坑口台車維修棚",
            "風櫃尾・風櫃尾燈塔",
            "風櫃尾・水產冷燻工坊",
            "風櫃尾・風櫃尾巨藻晾曬場",
            "烏石鼻・烏石鼻海貝灰窯",
            "南哨・重型獵具工坊",
            "水尾寮・蘆葦與竹編工棚",
            "風舶角・林氏大船塢",
            "文曜・三水總商行",
            "文曜・西門車馬聯運總站",
            "文曜・傳訊鐘樓與信使總站",
            "風舶角・深水泊位與卸貨碼頭",
            "大圳頭・農產與糧油市集",
            "文曜・學院評議會廳",
            "老堰頭・農產公議所",
            "文曜・市街石道",
            "文曜・中心樞紐橋",
            "風舶嶼・木柵圍欄",
            "老堰頭・水渠畔石砌農舍",
            "烏石鼻・玄武岩堆砌漁舍",
            "水尾寮・水尾寮高腳木屋",
            "大圳頭・周家莊園",
            "半嶺鎮・岑家二房別院",
            "文曜・三水口・陸家宅邸",
        ];

        for room in sample_rooms {
            let objs = player_layer_scene_objects_for_room(room);
            assert!(
                objs.len() >= 3,
                "Room {room} should have at least 3 playable objects, got {}",
                objs.len()
            );
            for obj in &objs {
                assert!(obj.sockets.contains(&"Look".to_string()), "Object {} missing Look socket", obj.id);
                assert!(!obj.sockets.is_empty(), "Object {} should have at least Look socket", obj.id);
                for socket in &obj.sockets {
                    let resp = obj.responses.get(socket);
                    assert!(resp.is_some(), "Object {} missing response for socket {socket}", obj.id);
                    let resp_text = resp.unwrap();
                    assert!(!resp_text.is_empty(), "Response for socket {socket} in {} is empty", obj.id);
                    for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"] {
                        assert!(
                            !resp_text.contains(forbidden),
                            "Object {} response contains forbidden word {forbidden}",
                            obj.id
                        );
                    }
                }
            }
        }

        // Verify non-peninsula room remains empty to preserve existing test invariants
        assert!(player_layer_scene_objects_for_room("城邦外坡道").is_empty());
        assert!(
            crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS.len() >= 1100,
            "Peninsula world catalog should have >= 1100 objects, got {}",
            crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS.len()
        );
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            for forbidden in ["debug", "資料庫", "database", "grid", "scene_stub", "還有"] {
                assert!(!entry.look_text.contains(forbidden));
                assert!(!entry.action_text.contains(forbidden));
            }
        }
    }

    #[test]
    fn wenyao_outer_ward_objects_and_residential_classification() {
        // 1. Residential classification
        let residences = [
            "文曜・講堂外坊・修竹讀書山房",
            "講堂外坊・刻書雕版匠居",
            "文曜・三水外坊・蘆洲漁家小築",
            "三水外坊・老舵手石砌院",
            "文曜・百工外坊・司爐老鐵匠院",
            "百工外坊・天工安居大院",
            "文曜・西門外坊・老護隊四合院",
            "西門外坊・駝隊領隊家宅",
        ];
        for r in residences {
            assert!(
                is_wenyao_residential_or_study(r),
                "Should classify {} as residential",
                r
            );
        }

        // 2. Commercial / Academic / Craft exclusions
        let commercial = [
            "文曜・講堂外坊・格物輿圖外院",
            "講堂外坊・活字排版印書局",
            "文曜・三水外坊・北灣貨船大埠",
            "三水外坊・早市鮮魚拍賣場",
            "文曜・百工外坊・重鐵水錘大鍛坊",
            "百工外坊・五金農具總商行",
            "文曜・西門外坊・萬里駝隊大驛站",
            "西門外坊・西門皮毛大市",
        ];
        for c in commercial {
            assert!(
                !is_wenyao_residential_or_study(c),
                "Should NOT classify {} as residential",
                c
            );
        }

        // 3. Inner Gate Plazas authentic objects and NO bath amenities
        let gate_plazas = ["文曜・北內門坊", "文曜・南內門坊", "文曜・東內門坊", "文曜・西內門坊"];
        for gate in gate_plazas {
            let objs = player_layer_scene_objects_for_room(gate);
            assert!(
                objs.iter().any(|o| o.id == "wt_gate_stone_archway_board"),
                "Gate plaza {} must have stone archway board",
                gate
            );
            assert!(
                !objs.iter().any(|o| o.id == "wt_reg_wenyao_amenities"),
                "Gate plaza {} must NEVER have indoor bath sink",
                gate
            );
            let reg_obj = regional_specialty_object_for_room(gate).expect("gate plaza must have regional obj");
            assert_eq!(reg_obj.id, "wt_reg_wenyao_gate_plaza");
        }

        // 4. Outer landmarks have specialized objects
        let nw_wharf = player_layer_scene_objects_for_room("文曜・三水外坊・北灣貨船大埠");
        assert!(nw_wharf.iter().any(|o| o.id == "wt_obj_wharf_bollard"));

        let ne_academy = player_layer_scene_objects_for_room("文曜・講堂外坊・格物輿圖外院");
        assert!(ne_academy.iter().any(|o| o.id == "wt_obj_cartography_table"));

        let se_forge = player_layer_scene_objects_for_room("文曜・百工外坊・重鐵水錘大鍛坊");
        assert!(se_forge.iter().any(|o| o.id == "wt_obj_trip_hammer_trough"));

        let sw_inn = player_layer_scene_objects_for_room("文曜・西門外坊・萬里駝隊大驛站");
        assert!(sw_inn.iter().any(|o| o.id == "wt_obj_caravan_hitch_trough"));
    }

    #[test]
    fn test_dynamic_vendor_objects_for_entities() {
        use crate::entity::{Character, EntityKind, Gender, MoveState, WalkOrRun};

        let mut vendor = Character {
            id: "wt_vendor_roast_beef".to_string(),
            kind: EntityKind::Npc,
            display_char: String::new(),
            x: 0,
            y: 0,
            move_state: MoveState::Idle,
            target_x: None,
            target_y: None,
            walk_or_run: WalkOrRun::Walk,
            move_started_at: None,
            vit: 10,
            qi: 10,
            dex: 10,
            magnesium: 0,
            last_observed_at: None,
            created_at: 0,
            gender: Some(Gender::M),
            soul_seed: None,
            display_title: "炭烤牛肉番薯攤東家|鄭火旺".to_string(),
            activated_nodes: "[]".to_string(),
            equipment_slots: String::new(),
            inventory: "[]".to_string(),
            disposition: 0,
            thoughts: vec![],
            current_activity: "正在炭烤牛肉番薯攤前翻動原牛肉串，高聲叫賣".to_string(),
            grid_x: None,
            grid_y: None,
        };

        // 1. 營業中的攤販應動態生成攤位物件
        let objects = dynamic_vendor_objects_for_entities(&[vendor.clone()]);
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].id, "wt_stall_roast_beef");
        assert_eq!(objects[0].name, "炭烤牛肉攤");
        assert!(objects[0].sockets.contains(&"Look".to_string()));
        assert!(objects[0].sockets.contains(&"Taste".to_string()));
        assert!(objects[0].sockets.contains(&"Smell".to_string()));

        // 2. 移動中或睡眠中的攤販不應生成攤位物件
        vendor.current_activity = "正在前往三水口南街".to_string();
        assert!(dynamic_vendor_objects_for_entities(&[vendor.clone()]).is_empty());

        vendor.current_activity = "在屋內安歇".to_string();
        assert!(dynamic_vendor_objects_for_entities(&[vendor]).is_empty());

        // 3. 一般街道不應含有固定的炭烤牛肉番薯攤
        let street_objs = player_layer_scene_objects_for_room("三水口南街");
        assert!(!street_objs.iter().any(|o| o.id == "wt_street_roast_snack_cart" || o.name == "炭烤牛肉攤"));
    }

    #[test]
    fn test_v03852_living_world_expansion_scene_sockets() {
        use super::*;
        let cal = current_calendar();
        assert!(["春融", "夏炎", "秋狩", "冬凝"].contains(&cal.season_name));

        let laoyan_objs = player_layer_scene_objects_for_room("老堰頭・水輪石磨工坊");
        assert!(laoyan_objs.iter().any(|o| o.id == "wt_mill_waterwheel_stone" && o.sockets.contains(&"MillGrain".to_string())));

        let shuanglin_objs = player_layer_scene_objects_for_room("霜林口・原木水車鋸木坊");
        assert!(shuanglin_objs.iter().any(|o| o.id == "wt_sawmill_hydraulic_saw" && o.sockets.contains(&"RefinePlank".to_string())));

        let banling_objs = player_layer_scene_objects_for_room("半嶺鎮・切石工坊");
        assert!(banling_objs.iter().any(|o| o.id == "wt_stonecut_wire_saw" && o.sockets.contains(&"CutJade".to_string())));

        let niupu_objs = player_layer_scene_objects_for_room("牛埔・乳酪精製坊");
        assert!(niupu_objs.iter().any(|o| o.id == "wt_cheese_curd_press" && o.sockets.contains(&"PressCheese".to_string())));

        let mine_objs = player_layer_scene_objects_for_room("紅土崁・選礦場與磚窯");
        assert!(mine_objs.iter().any(|o| o.id == "wt_mine_wash_chute" && o.sockets.contains(&"HaulMineCart".to_string())));

        let smoke_objs = player_layer_scene_objects_for_room("風櫃尾・冷燻工坊");
        assert!(smoke_objs.iter().any(|o| o.id == "wt_smokehouse_kiln" && o.sockets.contains(&"SmokeFish".to_string())));

        let lime_objs = player_layer_scene_objects_for_room("烏石鼻・海貝灰窯");
        assert!(lime_objs.iter().any(|o| o.id == "wt_lime_kiln_chamber" && o.sockets.contains(&"BurnLime".to_string())));

        let swamp_objs = player_layer_scene_objects_for_room("水尾寮・蘆葦碼頭");
        assert!(swamp_objs.iter().any(|o| o.id == "wt_dock_riverbank_raft" && o.sockets.contains(&"RowSwampRaft".to_string())));

        let nanshao_objs = player_layer_scene_objects_for_room("南哨・重型獵具工坊");
        assert!(nanshao_objs.iter().any(|o| o.id == "wt_hunt_heavy_crossbow" && o.sockets.contains(&"PrepareMiasmaGear".to_string())));
        let feast_objs = player_layer_scene_objects_for_room("豐收原大公堂食堂");
        assert!(feast_objs.iter().any(|o| o.id == "wt_dining_iron_cauldron" && o.sockets.contains(&"SeasonalBanquet".to_string())));

        let chess_objs = player_layer_scene_objects_for_room("雙星鎮茶寮客堂");
        assert!(chess_objs.iter().any(|o| o.id == "wt_inn_ebony_shell_chess" && o.sockets.contains(&"PlayChess".to_string())));
    }

    #[test]
    fn test_v03853_living_world_expansion_scene_sockets() {
        use super::*;
        // Pillar 1: ChartAstrolabe, TestEngraving, SparPillar
        let astro_objs = player_layer_scene_objects_for_room("文曜・觀星台");
        assert!(astro_objs.iter().any(|o| o.id == "wt_cat_0734" && o.sockets.contains(&"ChartAstrolabe".to_string())));

        let force_objs = player_layer_scene_objects_for_room("文曜・演武場試招院");
        assert!(force_objs.iter().any(|o| o.id == "wt_martial_force_stone" && o.sockets.contains(&"TestEngraving".to_string())));
        assert!(force_objs.iter().any(|o| o.id == "wt_martial_padded_wooden_blade" && o.sockets.contains(&"SparPillar".to_string())));

        let arena_objs = player_layer_scene_objects_for_room("半嶺試招場");
        assert!(arena_objs.iter().any(|o| o.id == "wt_arena_testing_pillar" && o.sockets.contains(&"SparPillar".to_string())));
        assert!(arena_objs.iter().any(|o| o.id == "wt_arena_wooden_weapon_rack" && o.sockets.contains(&"SparPillar".to_string())));

        // Pillar 2: SoakHotSpring, GroomAppearance, MendTravelGear
        let bath_objs = player_layer_scene_objects_for_room("溫泉公共大浴場");
        assert!(bath_objs.iter().any(|o| o.id == "wt_bath_jade_pool" && o.sockets.contains(&"SoakHotSpring".to_string())));
        assert!(bath_objs.iter().any(|o| o.id == "wt_bath_mirror_washstand" && o.sockets.contains(&"GroomAppearance".to_string())));

        let barber_objs = player_layer_scene_objects_for_room("文曜・剪剃理髮鋪");
        assert!(barber_objs.iter().any(|o| o.id == "wt_barber_folding_razor" && o.sockets.contains(&"GroomAppearance".to_string())));
        assert!(barber_objs.iter().any(|o| o.id == "wt_barber_copper_chair" && o.sockets.contains(&"GroomAppearance".to_string())));

        let repair_objs = player_layer_scene_objects_for_room("文曜・修傘補靴鋪");
        assert!(repair_objs.iter().any(|o| o.id == "wt_repair_umbrella_bench" && o.sockets.contains(&"MendTravelGear".to_string())));
        assert!(repair_objs.iter().any(|o| o.id == "wt_repair_boots_last" && o.sockets.contains(&"MendTravelGear".to_string())));

        let tailor_objs = player_layer_scene_objects_for_room("文曜・織造服飾補給行");
        assert!(tailor_objs.iter().any(|o| o.id == "wt_cloth_tailor_sewing_bench" && o.sockets.contains(&"MendTravelGear".to_string())));

        // Pillar 3: ReadWindVane, ApplyMoxibustion, HangVotivePlaque
        let obs_objs = player_layer_scene_objects_for_room("文曜・海天觀測台");
        assert!(obs_objs.iter().any(|o| o.id == "wt_observatory_anemometer" && o.sockets.contains(&"ReadWindVane".to_string())));

        let clinic_objs = player_layer_scene_objects_for_room("半嶺診所");
        assert!(clinic_objs.iter().any(|o| o.id == "wt_clinic_acupoint_bed" && o.sockets.contains(&"ApplyMoxibustion".to_string())));
        assert!(clinic_objs.iter().any(|o| o.id == "wt_clinic_decoction_stove" && o.sockets.contains(&"ApplyMoxibustion".to_string())));

        let pulse_objs = player_layer_scene_objects_for_room("文曜・杏林醫館");
        assert!(pulse_objs.iter().any(|o| o.id == "wt_obj_acupuncture_pulse_pillow" && o.sockets.contains(&"ApplyMoxibustion".to_string())));

        let mountain_objs = player_layer_scene_objects_for_room("青脊山・觀雲石台");
        assert!(mountain_objs.iter().any(|o| o.id == "wt_obj_mountain_milestone" && o.sockets.contains(&"HangVotivePlaque".to_string())));
    }

    #[test]
    fn test_v03854_living_world_expansion_scene_sockets() {
        use super::*;
        // Pillar 1: TranscribeRareTome, AttendLecture, CalibrateClock
        let book_objs = player_layer_scene_objects_for_room("文曜・藏書閣與印行館");
        assert!(book_objs.iter().any(|o| o.id == "wt_book_printing_shelf" && o.sockets.contains(&"TranscribeRareTome".to_string())));
        assert!(book_objs.iter().any(|o| o.id == "wt_book_pen_notebook_desk" && o.sockets.contains(&"TranscribeRareTome".to_string())));

        let academy_objs = player_layer_scene_objects_for_room("文曜・學院主校區");
        assert!(academy_objs.iter().any(|o| o.id == "wt_academy_fountain_pen" && o.sockets.contains(&"AttendLecture".to_string())));
        assert!(academy_objs.iter().any(|o| o.id == "wt_book_pen_notebook_desk" && o.sockets.contains(&"AttendLecture".to_string())));

        let clock_objs = player_layer_scene_objects_for_room("文曜・傳訊鐘樓與信使總站");
        assert!(clock_objs.iter().any(|o| o.id == "wt_clocktower_bronze_bell" && o.sockets.contains(&"CalibrateClock".to_string())));
        assert!(clock_objs.iter().any(|o| o.id == "wt_messenger_lookout_perch" && o.sockets.contains(&"CalibrateClock".to_string())));

        // Pillar 2: FitMortiseTenon, QuenchBlade, DyeIndigoCloth
        let wood_objs = player_layer_scene_objects_for_room("文曜・巧木居所傢俱坊");
        assert!(wood_objs.iter().any(|o| o.id == "wt_wood_planing_bench" && o.sockets.contains(&"FitMortiseTenon".to_string())));

        let forge_objs = player_layer_scene_objects_for_room("文曜・岑氏第一鍛造工坊");
        assert!(forge_objs.iter().any(|o| o.id == "wt_forge_furnace_anvil" && o.sockets.contains(&"QuenchBlade".to_string())));
        assert!(forge_objs.iter().any(|o| o.id == "wt_forge_quench_trough" && o.sockets.contains(&"QuenchBlade".to_string())));

        let cloth_objs = player_layer_scene_objects_for_room("文曜・雲錦織造與服飾坊");
        assert!(cloth_objs.iter().any(|o| o.id == "wt_cloth_garment_rack" && o.sockets.contains(&"DyeIndigoCloth".to_string())));

        let loom_objs = player_layer_scene_objects_for_room("大染坊");
        assert!(loom_objs.iter().any(|o| o.id == "wt_obj_loom_dye_vats" && o.sockets.contains(&"DyeIndigoCloth".to_string())));

        // Pillar 3: FeedDraftBeast, BoardCoastalShip, InspectLandRegistry
        let caravan_objs = player_layer_scene_objects_for_room("文曜・西門車馬聯運總站");
        assert!(caravan_objs.iter().any(|o| o.id == "wt_caravan_water_trough" && o.sockets.contains(&"FeedDraftBeast".to_string())));

        let ship_objs = player_layer_scene_objects_for_room("文曜・三水口・碼頭行商道");
        assert!(ship_objs.iter().any(|o| o.id == "wt_harbor_clockwise_ship" && o.sockets.contains(&"BoardCoastalShip".to_string())));

        let deed_objs = player_layer_scene_objects_for_room("文曜・公議堂房地契書局");
        assert!(deed_objs.iter().any(|o| o.id == "wt_deed_cadastral_map_desk" && o.sockets.contains(&"InspectLandRegistry".to_string())));
        assert!(deed_objs.iter().any(|o| o.id == "wt_deed_household_registry" && o.sockets.contains(&"InspectLandRegistry".to_string())));
    }

    #[test]
    fn test_v03855_three_pillar_scene_objects() {
        use super::*;
        // Pillar 1: AssistKitchen, RoastMountainTea, SunBooks
        let dining_objs = player_layer_scene_objects_for_room("文曜・三水大食堂");
        assert!(dining_objs.iter().any(|o| o.id == "wt_dining_rice_milk_stall" && o.sockets.contains(&"AssistKitchen".to_string())));
        assert!(dining_objs.iter().any(|o| o.id == "wt_dining_iron_cauldron" && o.sockets.contains(&"AssistKitchen".to_string())));

        let tea_objs = player_layer_scene_objects_for_room("文曜・東橋旅社與山風茶屋");
        assert!(tea_objs.iter().any(|o| o.id == "wt_inn_milk_tea_stove" && o.sockets.contains(&"RoastMountainTea".to_string())));

        let sun_objs = player_layer_scene_objects_for_room("文曜・藏書閣旁曬書石坪");
        assert!(sun_objs.iter().any(|o| o.id == "wt_book_printing_shelf" && o.sockets.contains(&"SunBooks".to_string())));

        // Pillar 2: RoastHerbs, CollateInscriptions, GrindOpticLens
        let herb_objs = player_layer_scene_objects_for_room("文曜・溫氏百草烘藥房");
        assert!(herb_objs.iter().any(|o| o.id == "wt_herb_drying_kiln" && o.sockets.contains(&"RoastHerbs".to_string())));
        assert!(herb_objs.iter().any(|o| o.id == "wt_herb_bamboo_sieve" && o.sockets.contains(&"RoastHerbs".to_string())));

        let stone_rubbing_objs = player_layer_scene_objects_for_room("文曜・藏書閣東側石巷");
        assert!(stone_rubbing_objs.iter().any(|o| o.id == "wt_obj_stone_rubbing_cabinet" && o.sockets.contains(&"CollateInscriptions".to_string())));

        let obs_objs = player_layer_scene_objects_for_room("文曜・觀星台");
        assert!(obs_objs.iter().any(|o| o.id == "wt_obs_brass_telescope_rack" && o.sockets.contains(&"GrindOpticLens".to_string())));

        // Pillar 3: SelectDriedSeafood, DeliberateCivicPact, DrawSpringWell
        let market_objs = player_layer_scene_objects_for_room("文曜・三水口乾貨海味行");
        assert!(market_objs.iter().any(|o| o.id == "wt_market_produce_stand" && o.sockets.contains(&"SelectDriedSeafood".to_string())));
        assert!(market_objs.iter().any(|o| o.id == "wt_market_scale_counter" && o.sockets.contains(&"SelectDriedSeafood".to_string())));

        let civic_objs = player_layer_scene_objects_for_room("文曜・公議堂");
        assert!(civic_objs.iter().any(|o| o.id == "wt_hall_council_table" && o.sockets.contains(&"DeliberateCivicPact".to_string())));
        assert!(civic_objs.iter().any(|o| o.id == "wt_hall_bulletin_board" && o.sockets.contains(&"DeliberateCivicPact".to_string())));

        let well_objs = player_layer_scene_objects_for_room("文曜・三水北街與水岸公井");
        assert!(well_objs.iter().any(|o| o.id == "wt_river_stone_steps" && o.sockets.contains(&"DrawSpringWell".to_string())));
    }

    #[test]
    fn test_public_person_label_and_scene_observation_line_use_room_venue_object() {
        let label = public_person_label_for_room("老堰頭・水輪石磨工坊");
        assert_eq!(label, Some("當值石磨匠師"));

        let mill_obs = scene_observation_line("老堰頭・水輪石磨工坊", "他");
        assert!(mill_obs.is_some());
        let mill_str = mill_obs.unwrap();
        assert!(mill_str.starts_with("他正立於水輪石磨工坊當值櫃檯內側"));
        assert!(mill_str.contains("玄武岩大石磨"));

        let canteen_obs = scene_observation_line("三水大食堂", "她");
        assert!(canteen_obs.is_some());
        let canteen_str = canteen_obs.unwrap();
        assert!(canteen_str.starts_with("她正立於大食堂當值櫃檯內側"));
    }

    #[test]
    fn test_sanshui_rooms_no_longer_spam_fishing_rod_or_street_mailbox() {
        let ferry_home_objs = player_layer_scene_objects_for_room("文曜・三水口・擺渡老爹宿所");
        assert!(
            ferry_home_objs.iter().any(|o| o.name == "床榻"),
            "文曜・三水口・擺渡老爹宿所 應具備床榻"
        );
        assert!(
            !ferry_home_objs.iter().any(|o| o.id == "wt_coastal_angling_spot" || o.name == "釣竿"),
            "文曜・三水口・擺渡老爹宿所 不應包含釣竿"
        );
        assert!(
            !ferry_home_objs.iter().any(|o| o.id == "wt_river_stone_steps" || o.name == "汲水青石階"),
            "文曜・三水口・擺渡老爹宿所 不應包含汲水青石階"
        );

        let outer_ring_objs = player_layer_scene_objects_for_room("文曜・三水外環北遠段");
        assert!(
            !outer_ring_objs.iter().any(|o| o.id == "wt_coastal_angling_spot" || o.name == "釣竿"),
            "文曜・三水外環北遠段 不應包含釣竿"
        );
        assert!(
            !outer_ring_objs.iter().any(|o| o.id == "wt_home_bamboo_mailbox" || o.name == "信箱"),
            "文曜・三水外環北遠段 不應包含信箱"
        );

        let lawn_objs = player_layer_scene_objects_for_room("文曜・三水外坊・綠草平坪");
        assert!(
            !lawn_objs.iter().any(|o| o.id == "wt_coastal_angling_spot" || o.name == "釣竿"),
            "文曜・三水外坊・綠草平坪 不應包含釣竿"
        );
        assert!(
            !lawn_objs.iter().any(|o| o.id == "wt_home_bamboo_mailbox" || o.name == "信箱"),
            "文曜・三水外坊・綠草平坪 不應包含信箱"
        );

        let ferry_cell = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(0, 0),
            crate::grid::Terrain::House,
            "文曜・三水口・擺渡老爹宿所",
        );
        let cell_objs = player_layer_scene_objects_for_cell(&ferry_cell);
        assert!(cell_objs.iter().any(|o| o.name == "床榻"));
        assert!(!cell_objs.iter().any(|o| o.id == "wt_coastal_angling_spot" || o.name == "釣竿"));
        assert!(!cell_objs.iter().any(|o| o.id == "wt_river_stone_steps" || o.name == "汲水青石階"));

        let ring_cell = crate::grid::GridCell::new(
            crate::grid::SquareCoord::new(0, 1),
            crate::grid::Terrain::Road,
            "文曜・三水外環北遠段",
        );
        let ring_cell_objs = player_layer_scene_objects_for_cell(&ring_cell);
        assert!(!ring_cell_objs.iter().any(|o| o.id == "wt_coastal_angling_spot" || o.name == "釣竿"));
        assert!(!ring_cell_objs.iter().any(|o| o.id == "wt_home_bamboo_mailbox" || o.name == "信箱"));
    }
}

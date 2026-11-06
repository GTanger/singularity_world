/// 根據在場人物狀態動態產出攤販攤位物件。
///
/// 排除移動中（`正在前往` / `正步行前往` / `邁步離開`）與睡眠中（`安歇` / `熟睡` / `就寢`）的角色；
/// 僅當攤販在場且處於擺攤營業狀態時，才動態生成對應攤位物件。
pub fn dynamic_vendor_objects_for_entities(entities: &[crate::entity::Character]) -> Vec<RoomObject> {
    let mut objects = Vec::new();
    for ent in entities {
        let act = ent.current_activity.as_str();
        let title = ent.display_title.as_str();
        let id = ent.id.as_str();

        // 排除移動中或睡眠中的角色
        if act.contains("正在前往")
            || act.contains("動身前往")
            || act.contains("返回【")
            || act.contains("正步行前往")
            || act.contains("趕往")
            || act.contains("邁步離開")
            || act.contains("安歇")
            || act.contains("熟睡")
            || act.contains("就寢")
        {
            continue;
        }

        // 僅當處於擺攤營業或工作活動時生效
        let is_vending = act.contains('攤')
            || act.contains("叫賣")
            || act.contains("吆喝")
            || act.contains("招呼")
            || act.contains("楦頭")
            || act.contains("擺攤")
            || act.contains("晴雨傘");
        if !is_vending {
            continue;
        }

        // 1. wt_vendor_roast_beef (炭烤牛肉番薯攤)
        if id == "wt_vendor_roast_beef" || title.contains("炭烤牛肉番薯攤") {
            objects.push(make_peninsula_object(
                "wt_stall_roast_beef",
                "炭烤牛肉攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "街角木輪推車上的炭火正旺，鐵網上烤著滋滋冒油的原牛肉串與焦蜜流淌的紅心烤番薯，煙火氣撲鼻。"),
                    ("Taste", "你接過一串炙烤得滋滋作響的焦香原牛肉串咬下一口，肉質勁道多汁，炭烤椒鹽香氣瞬間充盈口腔。"),
                    ("Use", "你走向炭烤牛肉番薯攤，準備向攤主買下一份炭烤牛肉串。"),
                    ("Smell", "你站在下風處細聞烤番薯散發出的甘甜焦香與烤肉煙火氣，頓時食指大動。"),
                ],
            ));
        }
        // 2. wt_vendor_noodle_soup (現煮大骨切仔麵攤)
        else if id == "wt_vendor_noodle_soup" || title.contains("切仔麵攤") {
            objects.push(make_peninsula_object(
                "wt_stall_noodle_soup",
                "大骨切仔麵攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "木製麵攤前熱氣氤氳，大銅鍋裡翻滾著乳白濃郁的大骨高湯，竹笊籬整齊掛在木架旁，油蔥酥與芹菜珠香氣四溢。"),
                    ("Taste", "你接過一碗熱騰騰的現煮切仔麵啜飲熱湯，大骨湯頭醇厚回甘，麵條彈牙吸汁，暖胃舒暢。"),
                    ("Use", "你走向切仔麵攤，準備向攤主買下一碗切仔湯麵。"),
                    ("Smell", "濃白大骨高湯與酥炸紅蔥頭的誘人鹹香隨熱氣蒸騰撲面而來，引人垂涎。"),
                ],
            ));
        }
        // 3. wt_vendor_pepper_bun (貼爐蔥香胡椒餅攤)
        else if id == "wt_vendor_pepper_bun" || title.contains("胡椒餅攤") {
            objects.push(make_peninsula_object(
                "wt_stall_pepper_bun",
                "炭烤胡椒餅攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "高大的炭火深鐵桶正冒著滾燙熱浪，桶壁內圈整齊貼滿烤得金黃微焦的厚實胡椒餅，芝麻粒粒飽滿。"),
                    ("Taste", "你咬下一口熱氣騰騰的胡椒餅，酥脆外皮應聲碎開，滾燙濃郁的蔥香肉汁與辛香黑胡椒瞬間在舌尖爆發。"),
                    ("Use", "你走向胡椒餅攤，準備向攤主買下一份酥皮鮮肉胡椒餅。"),
                    ("Smell", "烘烤麵皮的麥香、焙烤黑胡椒的辛香與炭火焦香交織在一起，濃烈撲鼻。"),
                ],
            ));
        }
        // 4. wt_vendor_herbal_tea (寒水玉冰鎮仙草凍攤)
        else if id == "wt_vendor_herbal_tea" || title.contains("仙草凍攤") || title.contains("烏梅湯攤") || title.contains("酸梅湯攤") {
            objects.push(make_peninsula_object(
                "wt_stall_herbal_tea",
                "冰鎮仙草凍攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "青瓷大陶甕外凝著細密水珠，浸在深井冷泉木盆中，旁邊懸著兩枚光亮的小銅盞，青瓷缽裡盛著晶瑩透亮的仙草凍與清涼百草飲。"),
                    ("Taste", "你接過一盞寒水玉冰鎮黑糖仙草凍舀起一口，滑嫩仙草沁涼透心，手炒黑糖醇厚回甘，滿口清香頓時消暑解膩。"),
                    ("Use", "你走向仙草凍攤，準備向攤主買下一份寒水玉冰鎮黑糖仙草凍。"),
                    ("Smell", "文火慢熬仙草的醇厚草本清香伴著手炒黑糖的焦甜香氣撲鼻而來，令人神清氣爽。"),
                ],
            ));
        }
        // 5. wt_vendor_tofu_pudding (古早味黑糖粉圓豆花攤)
        else if id == "wt_vendor_tofu_pudding" || title.contains("豆花攤") {
            objects.push(make_peninsula_object(
                "wt_stall_tofu_pudding",
                "黑糖豆花攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "原木手推攤車上立著光滑溫潤的白鐵豆花大圓桶，旁置一小陶缽晶瑩剔透、裹滿琥珀黑糖漿的慢火粉圓。"),
                    ("Taste", "你舀起一匙溫潤豆花入口，嫩滑如脂的豆花入口即化，伴著Q彈黑糖粉圓與微辛老薑糖水，甜而不膩。"),
                    ("Use", "你走向豆花攤，準備向攤主買下一份黑糖薑汁熱豆花。"),
                    ("Smell", "天然黃豆的醇濃豆香與現炒黑糖的焦甜香氣溫暖撲鼻，令人回味無窮。"),
                ],
            ));
        }
        // 6. wt_vendor_rice_roll (現包紫米飯糰早點攤)
        else if id == "wt_vendor_rice_roll" || title.contains("飯糰早點攤") {
            objects.push(make_peninsula_object(
                "wt_stall_rice_roll",
                "紫米飯糰攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "厚實的木蒸桶掀開棉布冒出騰騰白氣，旁邊排著酥脆老油條、炒香酸菜、香脆菜脯與現切滷蛋等豐富料餡。"),
                    ("Taste", "你大口咬下溫熱捏妥的紫米飯糰與荷葉炊飯，糯米粒粒香糯彈牙，內餡老油條香脆無比，鹹香酸脆層次豐富極其飽腹。"),
                    ("Use", "你走向早點攤，準備向攤主買下一份荷葉糯米炊飯。"),
                    ("Smell", "現蒸紫糯米特有的溫潤米香混著炒酸菜與老油條的鹹香，是清晨最踏實的煙火氣息。"),
                ],
            ));
        }
        // 7. wt_vendor_soy_milk (現磨米漿杏仁茶攤)
        else if id == "wt_vendor_soy_milk" || title.contains("米漿杏仁茶攤") {
            objects.push(make_peninsula_object(
                "wt_stall_soy_milk",
                "現磨米漿攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "攤前溫著兩口長嘴大銅壺，壺嘴不斷吐出溫熱蒸氣，一旁擺著竹盤盛裝的金黃酥脆現炸油條。"),
                    ("Taste", "你端起熱碗啜飲濃稠醇厚的花生糙米漿，焙烤花生的濃香滑入喉嚨，再蘸上一口酥脆油條，甘醇滿口。"),
                    ("Use", "你走向米漿攤，準備向攤主買下一份老堰頭現磨溫米漿。"),
                    ("Smell", "現磨烘烤花生糙米的濃郁焦香與南杏仁的溫潤甜香隨熱霧四溢，暖人心脾。"),
                ],
            ));
        }
        // 8. wt_vendor_morning_produce (清晨帶露時令蔬果攤)
        else if id == "wt_vendor_morning_produce" || title.contains("蔬果攤") {
            objects.push(make_peninsula_object(
                "wt_stall_morning_produce",
                "時令蔬果攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "竹編菜簍裡整齊碼放著清晨自田畦採摘的新鮮高麗菜、帶泥嫩薑、翠綠青蔥與鮮紅多汁的水果，葉片上猶掛著晶亮晨露。"),
                    ("Taste", "你挑選了一份鮮脆水芹菜咀嚼，清甜脆嫩、滿口鮮綠生機。"),
                    ("Use", "你走向蔬果攤，準備向攤主挑選買下一捆水芹菜。"),
                    ("Smell", "水靈靈的新鮮瓜果與帶泥時蔬散發出泥土與青草的純淨生機芬芳。"),
                ],
            ));
        }
        // 9. wt_vendor_morning_fish (現撈三水河鮮蛤蜊攤)
        else if id == "wt_vendor_morning_fish" || title.contains("河鮮蛤蜊攤") {
            objects.push(make_peninsula_object(
                "wt_stall_morning_fish",
                "三水河鮮攤",
                &["Look", "Taste", "Use", "Smell"],
                &[
                    ("Look", "淺木盆注滿清澄河水，幾尾鮮活溪魚正在盆底輕快擺尾，粗麻布上鋪滿吐淨泥沙的肥美文蛤與青背河蝦。"),
                    ("Taste", "你嚐了嚐攤上鮮燙熟透的現撈河蝦與文蛤，肉質鮮甜彈嫩，滿口清冽河水鮮味。"),
                    ("Use", "你走向河鮮攤，準備向攤主稱上一斤鮮活青殼河蝦。"),
                    ("Smell", "淺木盆與碎冰中散發出三水川清澈活水特有的清涼河鮮水氣，毫無半點腥濁。"),
                ],
            ));
        }
        // 10. wt_npc_ma_buxue (老匠晴雨傘與皮靴修補攤)
        else if id == "wt_npc_ma_buxue"
            || (title.contains("修傘補靴") && (act.contains("楦頭") || act.contains("晴雨傘") || act.contains("修補攤")))
        {
            objects.push(make_peninsula_object(
                "wt_stall_umbrella_boot_repair",
                "修傘補靴攤",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "街角鋪著防潮厚油布，擺著一副磨損鋥亮的硬木鞋楦、鐵砧與各式皮針錐子，旁邊架著幾柄新刷過防水熟桐油的堅韌竹骨晴雨傘。"),
                    ("Use", "你走向晴雨傘與修補攤，挑選了一柄堅韌的竹骨晴雨摺疊傘買下。"),
                    ("Operate", "你請老匠檢視自己的行囊與行靴，老匠熟練地接過查看針腳皮面，俐落地替你緊固了帶扣。"),
                ],
            ));
        }
    }
    objects
}

/// 根據 ViewEntity 清單動態產出攤位對應的 ViewObject
pub fn dynamic_vendor_view_objects_for_entities(
    entities: &[crate::server::ViewEntity],
) -> Vec<crate::server::ViewObject> {
    let mut view_objects = Vec::new();
    for ent in entities {
        let act = ent.current_activity.as_str();
        let title = ent.display_name.as_str();
        let id = ent.id.as_str();

        if act.contains("正在前往")
            || act.contains("動身前往")
            || act.contains("返回【")
            || act.contains("正步行前往")
            || act.contains("趕往")
            || act.contains("邁步離開")
            || act.contains("安歇")
            || act.contains("熟睡")
            || act.contains("就寢")
        {
            continue;
        }

        let is_vending = act.contains('攤')
            || act.contains("叫賣")
            || act.contains("吆喝")
            || act.contains("招呼")
            || act.contains("楦頭")
            || act.contains("擺攤")
            || act.contains("晴雨傘");
        if !is_vending {
            continue;
        }

        let stall_info: Option<(&str, &str, &[&str])> = if id == "wt_vendor_roast_beef" || title.contains("炭烤牛肉番薯攤") {
            Some(("wt_stall_roast_beef", "炭烤牛肉攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_noodle_soup" || title.contains("切仔麵攤") {
            Some(("wt_stall_noodle_soup", "大骨切仔麵攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_pepper_bun" || title.contains("胡椒餅攤") {
            Some(("wt_stall_pepper_bun", "炭烤胡椒餅攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_herbal_tea" || title.contains("仙草凍攤") || title.contains("烏梅湯攤") || title.contains("酸梅湯攤") {
            Some(("wt_stall_herbal_tea", "冰鎮仙草凍攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_tofu_pudding" || title.contains("豆花攤") {
            Some(("wt_stall_tofu_pudding", "黑糖豆花攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_rice_roll" || title.contains("飯糰早點攤") {
            Some(("wt_stall_rice_roll", "紫米飯糰攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_soy_milk" || title.contains("米漿杏仁茶攤") {
            Some(("wt_stall_soy_milk", "現磨米漿攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_morning_produce" || title.contains("蔬果攤") {
            Some(("wt_stall_morning_produce", "時令蔬果攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_vendor_morning_fish" || title.contains("河鮮蛤蜊攤") {
            Some(("wt_stall_morning_fish", "三水河鮮攤", &["Look", "Taste", "Use", "Smell"]))
        } else if id == "wt_npc_ma_buxue"
            || (title.contains("修傘補靴") && (act.contains("楦頭") || act.contains("晴雨傘") || act.contains("修補攤")))
        {
            Some(("wt_stall_umbrella_boot_repair", "修傘補靴攤", &["Look", "Use", "Operate"]))
        } else {
            None
        };

        if let Some((s_id, s_name, actions)) = stall_info {
            let act_vec: Vec<String> = actions.iter().map(|s| s.to_string()).collect();
            let obj_tag = crate::model::canonical_object_tag(s_id, s_name);
            let obj_tags = crate::model::object_semantic_tags(s_id, s_name, &act_vec);
            view_objects.push(crate::server::ViewObject {
                id: s_id.to_string(),
                name: s_name.to_string(),
                object_type: "object".to_string(),
                actions: act_vec,
                tag: obj_tag,
                category_tag: crate::model::object_category_tag(s_id, s_name).to_string(),
                attribute_tags: crate::model::object_attribute_tags(s_id, s_name).into_iter().map(str::to_string).collect(),
                tags: obj_tags,
            });
        }
    }
    view_objects
}

#[derive(Debug, Clone)]
pub struct CalendarInfo {
    pub season_name: &'static str,
    pub year: i32,
    pub day_in_season: i32,
}

pub fn current_calendar() -> CalendarInfo {
    let (days, _, _, _, _) = crate::game::current_game_clock_full();
    let (year, season_name, day_in_season) = crate::game::kaipu_calendar_parts(days);
    CalendarInfo {
        season_name,
        year,
        day_in_season,
    }
}

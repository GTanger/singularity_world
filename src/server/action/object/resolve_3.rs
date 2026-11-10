fn resolve_grid_scene_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let target = target_id.trim();
    if target.is_empty() {
        return None;
    }
    let cell = if let Some(coord) = parse_grid_room_id(room_id) {
        crate::server::grid_manager::get_runtime_grid_cell(coord)?
    } else {
        crate::server::grid_manager::get_runtime_grid_cell_by_name(room_id)?
    };
    let coord = cell.coord;

    // 0.8) 檢查動態野地設施（dyn_field_anvil, dyn_herb_stove 等）
    let now_fac = crate::game::now_unix();
    if let Some(fac) = crate::game::object_ecology::active_field_facility_objects(room_id, now_fac)
        .into_iter()
        .find(|obj| {
            obj.id == target
                || obj.name == target
                || obj.name.contains(target)
                || target.contains(&obj.name)
        })
    {
        return Some(fac);
    }
    // 0.9) 檢查私人內室／客房五大陳設器物（wt_reg_wenyao_home, wt_inn_bath_Preview, wt_home_shell_lamp, wt_home_jade_cabinet, wt_home_charcoal_wardrobe）
    let indoor_home_objs = [
        crate::model::RoomObject {
            id: "wt_reg_wenyao_home".to_string(),
            name: "床榻".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Rest".to_string(), "Sleep".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "寬大厚實的松香木大床上鋪著潔白乾爽的棉麻床單與蓬鬆的羽絨暖被，枕邊散發著淡淡的山野松香與陽光曝曬後的暖意。".to_string()),
                ("Rest".to_string(), "你在松香木大床邊坐下歇息，卸下隨身行囊，緊繃的肩背與小腿肌肉漸漸放鬆下來。".to_string()),
                ("Sleep".to_string(), "你吹熄床頭小燈，鑽入溫暖蓬鬆的羽絨暖被中沉沉睡去，一覺到天明，周身疲憊盡消。".to_string()),
                ("Use".to_string(), "你躺在鬆軟的羽絨暖被上閉目養神，呼吸著淡淡的松木清香，心神安寧。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "沐浴桶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Wash".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚實的紅杉木浴桶內盛滿了冒著騰騰熱氣的清澈山泉溫水，水面上漂浮著幾片乾香草葉，散發著舒緩筋骨的溫潤水汽。".to_string()),
                ("Wash".to_string(), "你踏入溫熱的杉木浴桶中浸泡洗沐，溫潤泉水洗去滿身風塵與汗漬，四肢百骸說不出的舒暢通透。".to_string()),
                ("Use".to_string(), "你掬起溫熱的山泉水洗淨面龐與雙手，水汽氤氳間精神為之一振。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "檯燈".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "書案上擺著一盞鑲嵌打磨夜光貝母薄片的黃銅旋蓋檯燈，柔和的珠光透過貝母散射開來，映照得室內溫馨靜謐。".to_string()),
                ("Operate".to_string(), "你輕輕旋動黃銅燈蓋調節光隙，室內的貝母柔光隨之流轉明滅，恰到好處地照亮案頭而不刺眼。".to_string()),
                ("Use".to_string(), "你湊近夜光貝母檯燈溫潤的光暈下靜心凝神，思緒格外澄澈清明。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_jade_cabinet".to_string(),
            name: "冰鑑櫃".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Open".to_string(),
                "StoreStash".to_string(),
                "RetrieveStash".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "屋角立著一座由柯氏木作坊榫接外框、內嵌北山寒水玉薄板的雙層保鮮石櫃。櫃門縫隙間隱隱透出天然清冽的涼意，存於其中的鮮食與獵獲經宿不腐。".to_string()),
                ("Open".to_string(), "你拉開寒水玉雙層保鮮石櫃的黃銅插銷，一股沁人心脾的天然玉石涼氣撲面而來，裡頭分格擺放的茶點、鮮果與肉乾皆維持著初入櫃時的鮮潤色澤。".to_string()),
                ("StoreStash".to_string(), "你將隨身物資妥善安放於寒水玉石板上，玉面寒氣瞬時凝固鮮氣，封存妥當。".to_string()),
                ("RetrieveStash".to_string(), "你拉開石櫃底層玉匣，將封存保鮮的物資取回隨身行囊之中。".to_string()),
                ("Use".to_string(), "你將隨身乾糧與果品整理妥當，置入寒水玉保鮮石櫃的冰涼玉板上鎖住鮮氣，並取出一盞冰鎮泉露飲下，暑氣與焦渴頓時全消。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_bamboo_mailbox".to_string(),
            name: "信箱".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "Open".to_string(),
                "DispatchCourier".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "掛在門扉旁以青竹細編、外塗熟桐油防雨的立體竹筒信箱，開口設有銅扣翻蓋，專供半島旬報投遞與鄰里熟人託遞家書。".to_string()),
                ("Read".to_string(), "你掀開信箱防雨翻蓋，取出摺疊整齊的最新一期《半島旬報》細細閱覽。".to_string()),
                ("Open".to_string(), "你打開信箱銅扣，探手檢視內底，查閱是否有街坊鄰里或行腳客商留下的書信與心意回禮。".to_string()),
                ("DispatchCourier".to_string(), "你將備好的書信封妥投入信箱附設的快騎取件格，交付三枚鎂幣委請疾行草獸信使飛騎送出。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
        crate::model::RoomObject {
            id: "wt_home_charcoal_wardrobe".to_string(),
            name: "衣櫥".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "WarmHands".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "以耐潮樟木打造的立式衣櫥，底層設有岑氏工坊鍛造的百葉抽屜與白瓷炭缽，點燃無煙松炭後溫風徐徐上升，能徹底烘乾沾染山霧海風的濕重衣物。".to_string()),
                ("WarmHands".to_string(), "你旋開烘衣櫥底部的百葉氣孔，將微涼的雙手湊近溫潤帶香的松炭暖風旁烘烤，暖意順著指尖流遍全身。".to_string()),
                ("Use".to_string(), "你將略帶潮氣的外袍掛入松炭溫風烘衣櫥中烘烤片刻，再穿上時已變得蓬鬆乾爽、帶著淡淡的松木暖香，周身濕寒一掃而空。".to_string()),
            ]),
            move_to_room_id: String::new(),
        },
    ];
    if let Some(obj) = indoor_home_objs.into_iter().find(|obj| {
        obj.id == target
            || obj.name == target
            || obj.name.contains(target)
            || target.contains(&obj.name)
    }) {
        return Some(obj);
    }
    // 1) 先查 game::player_layer_scene_objects_for_cell
    if let Some(obj) = game::player_layer_scene_objects_for_cell(&cell)
        .into_iter()
        .find(|obj| {
            obj.id == target
                || obj.name == target
                || obj.name.contains(target)
                || target.contains(&obj.name)
        })
    {
        return Some(obj);
    }

    // 1.5) 查詢同格 NPC 並匹配動態攤位物件
    let gh = crate::store::current_game_hour();
    if let Ok(entities) = crate::db::get_entities_in_room(room_id, gh)
        .or_else(|_| crate::db::get_entities_at_grid(coord.x, coord.y, gh))
    {
        if let Some(obj) = crate::game::player_scene::dynamic_vendor_objects_for_entities(&entities)
            .into_iter()
            .find(|obj| {
                obj.id == target
                    || obj.name == target
                    || obj.name.contains(target)
                    || target.contains(&obj.name)
            })
        {
            return Some(obj);
        }
    }
    // 2) 檢查 cell.objects（GridObject）
    if let Some(obj) = cell.objects.iter().find(|o| {
        o.id == target
            || o.name == target
            || o.name.contains(target)
            || target.contains(&o.name)
    }) {
        if obj.has_tag("beast_tracks") {
            let sockets = vec!["Look".to_string(), "Inspect".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "泥地石隙間留有一串清晰而深陷的野獸足印，蹄爪痕跡新鮮，周遭草葉微伏，顯然不久前曾有龐然巨物或敏捷靈獸由此踏過。".to_string()),
                ("Inspect".to_string(), "你伏身湊近仔細審視足印的爪痕開合、受力深淺與間距步幅，辨認出蹄印周邊殘留的泥痕與氣息，荒野行獸的蹤跡去向瞭然於胸。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("rock_niche") {
            let sockets = vec!["Look".to_string(), "Rest".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "厚實背風的山體天然凹陷處，石壁乾燥內斂，形成一座足以容身遮蔽風雨的山石岩龕，地面被往來旅人打掃得平整乾燥。".to_string()),
                ("Rest".to_string(), "你移步退入避風岩龕之內背石而坐，四周凜冽山風頓時被厚壁阻隔，呼吸沉緩調勻，渾身百骸漸次溫熱舒暢。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("mountain_spring") {
            let sockets = vec!["Look".to_string(), "Drink".to_string(), "Wash".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "清冽山泉自岩壁石隙間汩汩湧出，積聚成一泓澄澈見底的冷玉水窪，泉水清甘微涼，映著天光泛出粼粼水波。".to_string()),
                ("Drink".to_string(), "你俯身以掌心掬起一捧甘美冰涼的清泉痛飲，泉水清冽入喉直透臟腑，滿腹焦渴頓消，神清氣爽。".to_string()),
                ("Wash".to_string(), "你探手入冰涼的泉水中揉搓洗漱淨手，滌去一路攀跋的塵垢汗漬，指節肌理清涼微麻，神采倍感澄明。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("hunter_post") {
            let sockets = vec!["Look".to_string(), "Inspect".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "一截半人高的厚實硬木樁深釘於地，樁頭削成斜面，刻有防風雨的溝槽與巡山記號，周遭雜草已被過往獵手清整乾淨。".to_string()),
                ("Inspect".to_string(), "你湊近仔細審視木樁側面的刻痕與刮擦印記，上面留有獵團互通獵況的新舊刀痕與風雨標記，此方獵場的物候動靜一目了然。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("stepping_logs") {
            let sockets = vec!["Look".to_string(), "Rest".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "數截粗壯平整的硬木墩穩穩嵌在泥濘濕地中，表面斧鑿防滑刻痕，可供行人墊步落足、避開深沼。".to_string()),
                ("Rest".to_string(), "你踏上乾燥平穩的木墩坐下稍歇，甩去履底沾附的濕泥，迎著野徑清風調勻呼吸，腳力漸趨恢復。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("way_stele") {
            let sockets = vec!["Look".to_string(), "Read".to_string()];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "大道旁矗立著風化古樸的四稜指路石碑，基座是一方厚重石礎，碑面經受風雨洗刷依然字跡深邃，指引著前路方隅。".to_string()),
                ("Read".to_string(), "你俯身撫平碑面石紋辨讀刻字，通往周遭市鎮、要隘與主幹古道的方向里數工整分明，路途去向瞭然於心。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        } else if obj.has_tag("campfire") && obj.id != "zl_bamboo_campfire" {
            let sockets = vec![
                "Look".to_string(),
                "WarmHands".to_string(),
                "RoastFood".to_string(),
                "CraftTorch".to_string(),
                "AddFuel".to_string(),
                "CollectCharcoal".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                ("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string()),
                ("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕。".to_string()),
                ("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實。".to_string()),
                ("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string()),
                ("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string()),
                ("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string()),
            ]);
            return Some(crate::model::RoomObject {
                id: obj.id.clone(),
                name: obj.name.clone(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }
        if let Some(zl_slot_obj) =
            crate::grid::zhulin::resolve_bamboo_slot_object_with_qty(&obj.id, obj.quantity)
                .or_else(|| {
                    crate::grid::zhulin::resolve_bamboo_slot_object_with_qty(
                        &obj.name,
                        obj.quantity,
                    )
                })
        {
            let mut zl_slot_obj = zl_slot_obj;
            if (obj.id == "zl_bamboo_culm" || obj.name == "合抱刺竹叢") && obj.quantity > 0 {
                if !zl_slot_obj.sockets.iter().any(|s| s == "ClearBambooTrail") {
                    zl_slot_obj.sockets.push("ClearBambooTrail".to_string());
                }
                if !zl_slot_obj.sockets.iter().any(|s| s == "CarveMark") {
                    zl_slot_obj.sockets.push("CarveMark".to_string());
                }
                if !zl_slot_obj.sockets.iter().any(|s| s == "FillBambooCanteen") {
                    zl_slot_obj.sockets.push("FillBambooCanteen".to_string());
                }
            } else if (obj.id == "zl_dry_leaves" || obj.name == "乾竹葉層") && obj.quantity > 0 {
                let has_cf = cell
                    .objects
                    .iter()
                    .any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0);
                if !has_cf && !zl_slot_obj.sockets.iter().any(|s| s == "IgniteCampfire") {
                    zl_slot_obj.sockets.push("IgniteCampfire".to_string());
                }
            }
            return Some(zl_slot_obj);
        }
        if let Some(mut zl_obj) = crate::grid::resolve_shalu_zhulin_object(&obj.id)
            .or_else(|| crate::grid::resolve_shalu_zhulin_object(&obj.name))
        {
            if obj.quantity > 0 && !zl_obj.sockets.iter().any(|s| s == "Gather") {
                zl_obj.sockets.push("Gather".to_string());
            } else if obj.quantity == 0 {
                zl_obj.sockets.retain(|s| s != "Gather");
            }
            if obj.max_quantity > 0 {
                if let Some(look) = zl_obj.responses.get_mut("Look") {
                    crate::grid::zhulin::append_diegetic_stock_clause(
                        &obj.id,
                        &obj.name,
                        look,
                        obj.quantity,
                        obj.max_quantity,
                    );
                }
            }
            return Some(zl_obj);
        }

        let sockets = vec!["Look".to_string(), "Gather".to_string()];
        let look_desc = if obj.quantity > 0 {
            format!("你湊近端詳「{}」，此處物產尚存、生機完好，可順手採集取用。", obj.name)
        } else {
            format!("眼前的「{}」已被採擷一空，尚待時序更迭自然復生。", obj.name)
        };
        return Some(crate::model::RoomObject {
            id: obj.id.clone(),
            name: obj.name.clone(),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([("Look".to_string(), look_desc)]),
            move_to_room_id: String::new(),
        });
    }

    // 2.5) 房間一等物件（RoomVenueObject）之櫃檯禁區與外借器物插座，以及活躍事件包（EventPack）關鍵器物插座
    if let Some(venue) = cell.venue_object() {
        if venue.has_counter_restricted_zone
            && (target == "櫃檯"
                || target == "櫃台"
                || target == "內場"
                || target == "wt_venue_counter"
                || target.contains("櫃檯")
                || target.contains("櫃台")
                || target.contains("非本店人員禁止進入區"))
        {
            let offers_str = if venue.counter_trade_offers.is_empty() {
                "各色常備器物與號內差事簿冊".to_string()
            } else {
                venue
                    .counter_trade_offers
                    .iter()
                    .map(|(_, name, price)| format!("{}（{}鎂）", name, price))
                    .collect::<Vec<_>>()
                    .join("、")
            };
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Operate".to_string(),
            ];
            let active_pack_opt = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&cell.name, None, now_fac);
            let (inspect_desc, operate_desc) = if let Some((ev_pack, _sc_pack)) = &active_pack_opt {
                let primary_obj = ev_pack.key_objects.split('、').next().unwrap_or(ev_pack.key_objects).trim();
                (
                    format!(
                        "你隔著櫃檯端詳內場，案頭攤著一冊「{}」待辦記注，旁側備有{}。",
                        ev_pack.event_title, primary_obj
                    ),
                    format!(
                        "櫃檯內側為當值管制區，你在櫃檯外禮貌止步，向內場{}示意協辦「{}」。",
                        venue.default_host_title, ev_pack.event_title
                    ),
                )
            } else {
                (
                    format!(
                        "你隔著櫃檯端詳內場陳設，{}正於案前當值，若有採買或洽公可直接開口。",
                        venue.default_host_title
                    ),
                    format!(
                        "櫃檯內側屬於【{}】當值作業管制區，你禮貌地在櫃檯外止步候詢。",
                        venue.venue_kind
                    ),
                )
            };
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!(
                         "【{}】當值櫃檯，立有「非本店當值人員止步」木牌。櫃上陳列著：{}。",
                         cell.name, offers_str
                     ),
                 ),
                 ("Inspect".to_string(), inspect_desc),
                 ("Operate".to_string(), operate_desc),
            ]);
            return Some(crate::model::RoomObject {
                id: "wt_venue_counter".to_string(),
                name: format!("{}・當值櫃檯", venue.venue_kind),
                owner: venue.default_host_title.to_string(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }

        if let Some((tool_id, tool_name, lend_line)) = venue.lendable_tool
            && (target == tool_id
                || target == tool_name
                || (target.chars().count() >= 2 && (tool_name.contains(target) || target.contains(tool_name))))
        {
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Use".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!(
                         "【{}】常備於號內的「{}」，保養得宜、隨時可供{}與往來熟客借用。",
                         cell.name, tool_name, venue.default_host_title
                     ),
                 ),
                 (
                     "Inspect".to_string(),
                     format!(
                         "你細看這件「{}」，器身烙有號內印記。若需借用，可向{}開口（{}）。",
                         tool_name, venue.default_host_title, lend_line
                     ),
                 ),
                 (
                     "Use".to_string(),
                     format!(
                         "你順手試了試號內備置的「{}」，手感沉穩趁手（亦可透過借物向{}正式借取隨身攜行）。",
                         tool_name, venue.default_host_title
                     ),
                 ),
            ]);
            return Some(crate::model::RoomObject {
                id: tool_id.to_string(),
                name: tool_name.to_string(),
                owner: venue.default_host_title.to_string(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }
    }

    let resolved_pack_opt = if let Some(raw_script_id) = target.strip_prefix("wt_event_obj_") {
        crate::game::peninsula_scripts::find_script_by_id(raw_script_id)
            .map(|sc| crate::game::peninsula_scripts::decompose_into_event_and_script_pack(sc, &cell.name, None, None))
            .or_else(|| crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&cell.name, None, now_fac))
    } else {
        crate::game::peninsula_scripts::active_event_and_script_pack_for_room(&cell.name, None, now_fac)
    };
    if let Some((ev_pack, sc_pack)) = resolved_pack_opt {
        let expected_event_obj_id = format!("wt_event_obj_{}", ev_pack.id.to_lowercase());
        let key_candidates: Vec<&str> = ev_pack
            .key_objects
            .split('、')
            .map(|seg| {
                seg.split('（')
                    .next()
                    .unwrap_or(seg)
                    .split('(')
                    .next()
                    .unwrap_or(seg)
                    .trim()
            })
            .filter(|k| !k.is_empty())
            .collect();
        let matched_key = if target.eq_ignore_ascii_case(&expected_event_obj_id) || target.starts_with("wt_event_obj_") {
            key_candidates.first().copied()
        } else {
            key_candidates
                .into_iter()
                .find(|k| *k == target || (target.chars().count() >= 2 && (k.contains(target) || target.contains(*k))))
        };
        if let Some(matched_key) = matched_key {
            let sockets = vec![
                "Look".to_string(),
                "Inspect".to_string(),
                "Operate".to_string(),
                "Use".to_string(),
            ];
            let responses = std::collections::HashMap::from([
                (
                     "Look".to_string(),
                     format!("場內顯眼處擺著「{}」，正供「{}」差事取用。", matched_key, ev_pack.event_title),
                 ),
                 (
                     "Inspect".to_string(),
                     {
                         let short_chain = ev_pack.trigger_and_causal_chain.split(['。', '；']).next().unwrap_or(ev_pack.trigger_and_causal_chain.as_str()).trim_end_matches(['。', '；']);
                         format!("你湊近端詳「{}」——{}。", matched_key, short_chain)
                     },
                 ),
                 (
                     "Operate".to_string(),
                     {
                         let short_act = sc_pack.player_actions.split(['。', '；']).next().unwrap_or(sc_pack.player_actions.as_str()).trim_end_matches(['。', '；']);
                         format!("你動手打理「{}」，依差事吩咐{}。", matched_key, short_act)
                     },
                 ),
                 (
                     "Use".to_string(),
                     {
                         let short_outcome = sc_pack.outcome_and_world_change.split(['。', '；']).next().unwrap_or(sc_pack.outcome_and_world_change.as_str()).trim_end_matches(['。', '；']);
                         format!("你取用「{}」襄助差事，促成{}。", matched_key, short_outcome)
                     },
                 ),
            ]);
            return Some(crate::model::RoomObject {
                id: format!("wt_event_obj_{}", ev_pack.id.to_lowercase()),
                name: matched_key.to_string(),
                owner: sc_pack.host_slot_actor,
                sockets,
                responses,
                move_to_room_id: String::new(),
            });
        }
    }

    // 3) 檢查是否為環境地景名稱或沉浸式兜底
    let scenery_desc = match target {
        "平整道石" => Some("青灰道石經年累月受車馬行旅踏磨，表面溫潤平實，石縫間嵌著細碎砂土與微濕青苔。".to_string()),
        "沿途界碑" => Some("半人高的風化石碑立於道旁，碑面刻著辨識方位的古舊字跡，底座覆著一層淺淺塵土。".to_string()),
        "白塔盤旋石階" => Some("素白石階循著塔身緩緩盤旋而上，階面潔淨微涼，隱約可見歷代巡塔人留下的足跡。".to_string()),
        "引靈拱券石基" => Some("厚重的拱券石基承托著高聳廊道，石紋間似有微弱的溫熱氣息緩緩流轉。".to_string()),
        "素白玉壁" => Some("壁面由整塊素白石材打磨砌成，觸手生涼，映著天光泛起柔和的光澤。".to_string()),
        "潮打青石階" => Some("石階下半截常年受潮水拍打，布滿深色水痕與細小貝殼，踏上去略帶濕滑。".to_string()),
        "泊舟木樁" => Some("粗壯的硬木樁深釘入岸邊泥石，樁身纏繞著磨出毛邊的粗麻纜繩，散發淡淡鹹腥水氣。".to_string()),
        "水畔蘆葦叢" => Some("密實的蘆葦隨風起伏搖曳，根部泥水清淺，偶有水蟲與飛鳥掠過。".to_string()),
        "青苔怪石" => Some("嶙峋山石表面覆滿厚實翠綠的青苔，背陰處凝著冰涼露珠。".to_string()),
        "盤根古木" => Some("蒼老古木的粗壯根系如虯龍般盤繞地表，枝葉繁茂，遮蔽天日。".to_string()),
        "林間落葉" => Some("層層疊疊的枯葉鋪滿林地，踩踏其上發出細碎聲響，散發著泥土與草木腐殖的清香。".to_string()),
        _ => {
            if !target.is_empty()
                && target.chars().count() <= 32
                && !target.chars().any(|c| c.is_control() || c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                Some(format!(
                    "你駐足細看{}一帶的「{}」，其肌理與周遭地勢自然相融，別有一番風物韻致。",
                    if cell.name.is_empty() { "此處" } else { &cell.name },
                    target
                ))
            } else {
                None
            }
        }
    };

    if let Some(desc) = scenery_desc {
        return Some(crate::model::RoomObject {
            id: target.to_string(),
            name: target.to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string()],
            responses: std::collections::HashMap::from([("Look".to_string(), desc)]),
            move_to_room_id: String::new(),
        });
    }

    None
}

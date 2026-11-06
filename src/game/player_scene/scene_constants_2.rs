fn final_map_scene_for_room(room_name: &str) -> Option<&'static PolishedSceneText> {
    FINAL_MAP_SCENE_TEXTS
        .iter()
        .find(|scene| room_name.contains(scene.name))
}

pub fn final_map_public_role_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.role)
}

pub fn final_map_crowd_label_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.crowd)
}

pub fn final_map_entry_log_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.entry)
}

pub fn final_map_reentry_log_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.reentry)
}

pub fn final_map_observation_template_for_room(room_name: &str) -> Option<&'static str> {
    final_map_scene_for_room(room_name).map(|scene| scene.observation)
}

pub fn player_layer_scene_objects_for_cell(cell: &crate::grid::GridCell) -> Vec<RoomObject> {
    player_layer_scene_objects_inner(&cell.name, Some(cell.terrain), Some(&cell.effective_tags()))
}

pub fn player_layer_scene_objects_for_room(room_name: &str) -> Vec<RoomObject> {
    let inferred_terrain = if crate::grid::grid::is_private_residence_room_name(room_name)
        || is_wenyao_residential_or_study(room_name)
    {
        Some(crate::grid::Terrain::House)
    } else {
        None
    };
    let derived = crate::grid::derive_room_tags(
        inferred_terrain.unwrap_or(crate::grid::Terrain::Road),
        room_name,
        "",
    );
    player_layer_scene_objects_inner(room_name, inferred_terrain, Some(&derived))
}

fn player_layer_scene_objects_inner(
    room_name: &str,
    terrain_opt: Option<crate::grid::Terrain>,
    tags_opt: Option<&[String]>,
) -> Vec<RoomObject> {
    let clean_buf = room_name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
    let room_name = clean_buf.as_str();
    let has_tag = |wanted: &str| -> bool {
        let canon_wanted = crate::grid::canonical_room_tag(wanted);
        tags_opt.map_or(false, |tags| {
            tags.iter().any(|t| {
                t == wanted
                    || (!canon_wanted.is_empty()
                        && crate::grid::canonical_room_tag(t) == canon_wanted)
            })
        })
    };
    let is_m3_020 = M3_020_SCENE_OBJECTS.iter().any(|obj| room_name.contains(obj.room));
    let mut objects: Vec<RoomObject> = M3_020_SCENE_OBJECTS
        .iter()
        .filter(|obj| room_name.contains(obj.room))
        .map(|obj| {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), obj.look.to_string());
            responses.insert(obj.followup_action.to_string(), obj.followup.to_string());
            let mut sockets = vec!["Look".to_string(), obj.followup_action.to_string()];
            if obj.id == "m3_020_stone_gate_pass_tags" {
                sockets.push("JoinCheck".to_string());
            }
            if obj.id == "m3_020_mint_half_open_door" {
                sockets.push("Queue".to_string());
            }
            if obj.id == "m3_020_hire_short_work_boards" {
                sockets.push("AcceptJob".to_string());
            }
            if obj.id == "m3_020_council_notice_strip" {
                sockets.push("RegisterComplaint".to_string());
            }
            RoomObject {
                id: obj.id.to_string(),
                name: obj.name.to_string(),
                owner: String::new(),
                sockets,
                responses,
                move_to_room_id: String::new(),
            }
        })
        .collect();

    if is_m3_020 {
        return objects;
    }

    objects.extend(
        M3_027_LIFE_OBJECTS
            .iter()
            .filter(|obj| {
                room_name.contains(obj.room)
                    || (obj.room == "半嶺水岸渡口"
                        && (room_name.contains("水岸渡口")
                            || room_name.contains("水尾寮渡口")
                            || room_name.contains("水尾寮渡頭")))
                    || (obj.room == "水尾寮渡頭市集" && (room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集")))
            })
            .map(|obj| {
                let mut responses = HashMap::new();
                responses.insert("Look".to_string(), obj.look.to_string());
                let mut sockets = vec!["Look".to_string()];
                for (action, response) in obj.actions {
                    sockets.push((*action).to_string());
                    responses.insert((*action).to_string(), (*response).to_string());
                }
                RoomObject {
                    id: obj.id.to_string(),
                    name: obj.name.to_string(),
                    owner: String::new(),
                    sockets,
                    responses,
                    move_to_room_id: String::new(),
                }
            }),
    );
    if objects.is_empty() {
        objects = white_tower_peninsula_objects_for_room(room_name);
        objects.truncate(6);
    }
    if objects.is_empty() && !is_white_tower_peninsula_room(room_name) {
        return objects;
    }

    let is_firewood_venue = room_name.contains("鋸木")
        || room_name.contains("木作")
        || room_name.contains("巧木")
        || room_name.contains("炭窯")
        || room_name.contains("竹木場")
        || room_name.contains("林工")
        || room_name.contains("柴");
    if is_firewood_venue && !objects.iter().any(|o| o.id == "wt_multi_firewood_pile") {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "整齊碼放於簷下或角隅的風乾柴木與硬木料垛，木質乾燥密實，既是用以生火禦寒之良材，亦能順紋劈柴、裁鋸木板、進爐悶燒精炭或車削長柄。".to_string());
        responses.insert("Gather".to_string(), "你自木料垛中細細翻揀，抱出一根紋理密實的伐採原木與兩束乾燥柴枝收整妥當。".to_string());
        responses.insert("MakeFire".to_string(), "你取出乾燥柴枝引燃火星，在空地中央架起通風柴堆，迅速生起一堆溫暖明亮的野地營火。".to_string());
        responses.insert("SplitWood".to_string(), "你運力順著木紋利索剖劈，將原木細細剖分劈成易於引火的乾燥柴枝束。".to_string());
        responses.insert("RefinePlank".to_string(), "你推動長鋸順紋切削，將沉實原木裁鋸打磨為平整規整的樟松木板料。".to_string());
        responses.insert("BurnCharcoal".to_string(), "你將截切硬木置入坑窯泥封悶燒，凝煉出無煙透亮的硬木白炭。".to_string());
        responses.insert("CraftTorch".to_string(), "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的浸脂松明火把。".to_string());
        responses.insert("RepairStructure".to_string(), "你挑選合適的木板料與工具長柄，加固修整破損的欄杆踏板與設施構件。".to_string());

        objects.push(RoomObject {
            id: "wt_multi_firewood_pile".to_string(),
            name: "風乾柴木垛".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Gather".to_string(),
                "MakeFire".to_string(),
                "SplitWood".to_string(),
                "RefinePlank".to_string(),
                "BurnCharcoal".to_string(),
                "CraftTorch".to_string(),
                "RepairStructure".to_string(),
            ],
            responses,
            move_to_room_id: String::new(),
        });
    }

    if let Some(_cf) = crate::game::object_ecology::get_active_campfire(room_name) {
        if !objects.iter().any(|o| o.id == "dyn_campfire") {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string());
            responses.insert("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕。".to_string());
            responses.insert("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實。".to_string());
            responses.insert("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string());
            responses.insert("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string());
            responses.insert("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string());

            objects.push(RoomObject {
                id: "dyn_campfire".to_string(),
                name: "野地營火堆".to_string(),
                owner: String::new(),
                sockets: vec![
                    "Look".to_string(),
                    "WarmHands".to_string(),
                    "RoastFood".to_string(),
                    "CraftTorch".to_string(),
                    "AddFuel".to_string(),
                    "CollectCharcoal".to_string(),
                ],
                responses,
                move_to_room_id: String::new(),
            });
        }
    }
    let field_objs = crate::game::object_ecology::active_field_facility_objects(room_name, crate::game::now_unix());
    for f in field_objs {
        if !objects.iter().any(|o| o.id == f.id) {
            objects.push(f);
        }
    }

    // Coastal harbor / dock injection for 風舶角, 風舶嶼, 烏石鼻, 風櫃尾, 南哨
    let is_coastal_port_zone = room_name.contains("風舶角")
        || room_name.contains("風舶嶼")
        || room_name.contains("烏石鼻")
        || room_name.contains("風櫃尾")
        || room_name.contains("水尾寮")
        || room_name.contains("南哨")
        || room_name.contains("碼頭行商道")
        || room_name.contains("船埠");
    let is_harbor_room = !room_name.contains("內室")
        && (room_name.contains("港")
            || room_name.contains("渡")
            || room_name.contains("村")
            || room_name.contains("道")
            || room_name.contains("徑")
            || room_name.contains("船")
            || room_name.contains("澳")
            || room_name.contains("海事")
            || room_name.contains("守望石階")
            || room_name.contains("漁")
            || room_name.contains("堤")
            || room_name.contains("棧")
            || room_name.contains("街")
            || room_name.contains("橋")
            || room_name.contains("岬")
            || room_name.contains("哨")
            || room_name.contains("廣場")
            || room_name.contains("埠"));
    if is_coastal_port_zone && is_harbor_room {
        if !objects.iter().any(|o| o.id == "wt_harbor_clockwise_ship") {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), "泊靠在石砌突堤旁的順時針環島海運商船，船體以厚實樟木與防水桐油膠漆拼接，帆檣高聳，隨暖流與黑潮在各港口間順時針巡航聯運。".to_string());
            responses.insert("Read".to_string(), "你走近船舷看清船身木牌：此船順黑潮洋流依序循行「烏石鼻、風櫃尾、風舶嶼、風舶角、水尾寮、南哨」，單程船資 8 鎂，風浪顛簸不誤客期。".to_string());
            responses.insert("Use".to_string(), "你登上海運商船甲板，交付 8 鎂船資，準備搭乘順流班次隨潮航向下一座海港。".to_string());
            responses.insert("BoardCoastalShip".to_string(), "你踏上順時針環島海運商船甲板，隨老舵手核對黑潮洋流與潮汐海圖，揚帆出海巡航。".to_string());
            objects.push(RoomObject {
                id: "wt_harbor_clockwise_ship".to_string(),
                name: "順時針環島海運商船".to_string(),
                owner: String::new(),
                sockets: vec!["Look".to_string(), "Read".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()],
                responses,
                move_to_room_id: String::new(),
            });
        }
        if !objects.iter().any(|o| o.id == "wt_harbor_insurance_desk") {
            let mut responses = HashMap::new();
            responses.insert("Look".to_string(), "碼頭棧橋旁的築港頭海事承保櫃檯，桌案整齊陳列著防水油麻布海事保單與氣象水文籌板，由當值海事掌事主持水險承保。".to_string());
            responses.insert("Read".to_string(), "櫃檯案頭告示：凡出海客商皆可在此投保築港頭分級海事保單（每份 6 鎂）。若航程中突遇驟雨狂風海損，抵埠憑單可自風舶角公庫領取 12 鎂理賠撫卹。".to_string());
            responses.insert("Use".to_string(), "你走近海事承保櫃檯，繳納 6 鎂契費，在防水油麻布保單上簽押投保海事水險。".to_string());
            objects.push(RoomObject {
                id: "wt_harbor_insurance_desk".to_string(),
                name: "築港頭海事承保櫃檯".to_string(),
                owner: String::new(),
                sockets: vec!["Look".to_string(), "Read".to_string(), "Use".to_string()],
                responses,
                move_to_room_id: String::new(),
            });
        }
    }
    let is_caravan_hub = !room_name.contains("內室")
        && (room_name.contains("聯運")
            || room_name.contains("貨運")
            || room_name.contains("舊堰石橋")
            || room_name == "老堰頭・村中泥徑"
            || (room_name.contains("半嶺鎮") && (room_name.contains("街") || room_name.contains("坪") || room_name.contains("道")))
            || (room_name.contains("大圳頭") && (room_name.contains("街") || room_name.contains("渠") || room_name.contains("道")))
            || (room_name.contains("南哨") && (room_name.contains("街") || room_name.contains("哨") || room_name.contains("道")))
            || (room_name.contains("風舶角") && (room_name.contains("街") || room_name.contains("港") || room_name.contains("碼頭") || room_name.contains("棧"))));
    if is_caravan_hub && !objects.iter().any(|o| o.id == "wt_caravan_ticket_counter") {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "木牌案上每日更換木質班次籤牌，寫明發往大圳頭、風舶角與南哨的車馬出發時刻。".to_string());
        responses.insert("Read".to_string(), "你細看班次牌，往各重鎮的大車正依序整備發班，回程車隊則已進棚卸貨。".to_string());
        responses.insert("Use".to_string(), "你向車馬行主事交付車資，登車準備前往下一處聯運重鎮。".to_string());
        objects.push(RoomObject {
            id: "wt_caravan_ticket_counter".to_string(),
            name: "班次木牌案".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Read".to_string(), "Use".to_string()],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // Agricultural field rooms injection
    let is_crop_field = room_name.contains("水田")
        || room_name.contains("旱田")
        || room_name.contains("藥圃")
        || room_name.contains("空壟")
        || room_name.contains("田壟")
        || room_name.contains("阡陌")
        || room_name.contains("農");
    if is_crop_field && !objects.iter().any(|o| o.id == "wt_field_crop_plot") {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "引清泉灌溉、壟溝分明的當季田壟苗畦，泥土深厚鬆軟，壟間生機盎然，正依時序沐浴甘霖與陽光靜靜孕育穀蔬。".to_string());
        responses.insert("Use".to_string(), "你俯身巡視田壟，順手疏理溝渠清泉，照料禾苗生息。".to_string());
        responses.insert("Harvest".to_string(), "你順著壟溝採收飽滿成熟的當季作物，泥香與稻菽之氣撲鼻而來。".to_string());
        objects.push(RoomObject {
            id: "wt_field_crop_plot".to_string(),
            name: "當季田壟苗畦".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Harvest".to_string()],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // Ensure all commission / bulletin boards support full action list
    for obj in &mut objects {
        if obj.id == "wt_guild_commission_board" || obj.id == "wt_hall_bulletin_board" {
            for act in ["Look", "Read", "Use", "Deliver", "ClaimStrandedCargo", "DeliverStrandedCargo"] {
                if !obj.sockets.iter().any(|s| s == act) {
                    obj.sockets.push(act.to_string());
                }
            }
        }
    }

    // 1) 防雨竹編信箱：僅當實際住宅（terrain == Terrain::House 或 tags 包含住宅/民居 或 is_wenyao_residential_or_study 或包含宿所/客寮）
    let is_mailbox_room = (terrain_opt == Some(crate::grid::Terrain::House)
        || has_tag("residence")
        || is_wenyao_residential_or_study(room_name)
        || room_name.contains("宿所")
        || room_name.contains("客寮")
        || room_name.contains("內室")
        || room_name.contains("居所")
        || room_name.contains("民宅")
        || room_name.contains("宅院")
        || room_name.contains("民舍")
        || room_name.contains("住宅")
        || room_name.contains("客房"))
        && !room_name.contains("外環")
        && !room_name.contains("內環")
        && !room_name.contains("平坪")
        && !room_name.contains("草坪")
        && !room_name.contains("步道")
        && !has_tag("street")
        && !has_tag("road")
        && !crate::grid::grid::is_street_or_gate_room_name(room_name);
    if is_mailbox_room && !objects.iter().any(|o| o.id == "wt_home_bamboo_mailbox") {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "掛在門扉旁以青竹細編、外塗熟桐油防雨的立體竹筒信箱，開口設有銅扣翻蓋，專供半島旬報投遞與鄰里熟人託遞家書。".to_string());
        responses.insert("Read".to_string(), "你掀開信箱防雨翻蓋，取出摺疊整齊的最新一期《半島旬報》細細閱覽。".to_string());
        responses.insert("Open".to_string(), "你打開信箱銅扣，探手檢視內底，查閱是否有街坊鄰里或行腳客商留下的書信與心意回禮。".to_string());
        responses.insert("DispatchCourier".to_string(), "你將備好的書信封妥投入信箱附設的快騎取件格，交付三枚鎂幣委請疾行草獸信使飛騎送出。".to_string());
        objects.push(RoomObject {
            id: "wt_home_bamboo_mailbox".to_string(),
            name: "信箱".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "Open".to_string(),
                "DispatchCourier".to_string(),
            ],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // 2) 疾行草獸信使櫃檯：傳訊鐘樓、信使、車馬聯運、公議堂
    let is_courier_room = !room_name.contains("內室")
        && (has_tag("courier")
            || room_name.contains("傳訊鐘樓")
            || room_name.contains("信使")
            || room_name.contains("車馬聯運")
            || room_name.contains("公議堂"));
    if is_courier_room && !objects.iter().any(|o| o.id == "wt_courier_post_desk" || o.has_tag("courier_post_desk")) {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "疾行草獸信使櫃檯前陳列著往返四鎮的驛道封冊、火漆印章與加急竹筒，案旁栓著數頭蓄勢待發的健碩草獸。".to_string());
        responses.insert("Read".to_string(), "你細讀櫃檯告示：草獸信使日夜兼程往來四鎮，承辦跨聚落加急包裹與重金文卷託運，每件資費 4 鎂。".to_string());
        responses.insert("DispatchCourier".to_string(), "你向信使櫃檯交付 4 鎂託運規費，將重要急件交付草獸信使快騎飛遞。".to_string());
        responses.insert("Use".to_string(), "你走近信使櫃檯，向值守掌事探詢各鎮驛道最新路況與班次安排。".to_string());
        objects.push(RoomObject {
            id: "wt_courier_post_desk".to_string(),
            name: "信使櫃檯".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "DispatchCourier".to_string(),
            ],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // 3) 分格存摺櫃：商行、商盟、票號
    let is_bank_room = !room_name.contains("內室")
        && (has_tag("bank")
            || room_name.contains("商行")
            || room_name.contains("商盟")
            || room_name.contains("票號"));
    if is_bank_room && !objects.iter().any(|o| o.has_tag("ledger_cabinet")) {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "厚重鐵木所造的分格存摺櫃，抽屜銅扣皆刻有三水口流水徽印，收存著各大商隊與鎮民的通兌簿冊與印鑑。".to_string());
        responses.insert("Read".to_string(), "你細讀櫃頂張貼的三水總商行通兌章程，載明各港口通兌匯水與信用票號兌領條款。".to_string());
        responses.insert("DepositBank".to_string(), "你在帳房案前出示存摺與信印，將隨身鎂幣妥善存入商行票號帳頭。".to_string());
        responses.insert("WithdrawBank".to_string(), "你憑分格存摺防偽硃印與帳房核對銀數，自票號結存中支取現鎂。".to_string());
        responses.insert("Use".to_string(), "你走近分格存摺櫃，向值守執事出示存摺辦理商行通兌手續。".to_string());
        objects.push(RoomObject {
            id: "wt_cat_0790".to_string(),
            name: "分格存摺櫃".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "DepositBank".to_string(),
                "WithdrawBank".to_string(),
                "Use".to_string(),
            ],
            responses,
            move_to_room_id: String::new(),
        });
    }

    let is_indoor_building = room_name.contains("食堂")
        || room_name.contains("旅社")
        || room_name.contains("茶屋")
        || room_name.contains("茶樓")
        || room_name.contains("商行")
        || room_name.contains("工坊")
        || room_name.contains("鐵坊")
        || room_name.contains("木作")
        || room_name.contains("診所")
        || room_name.contains("藥房")
        || room_name.contains("書閣")
        || room_name.contains("講堂")
        || room_name.contains("蒙學")
        || room_name.contains("學院")
        || room_name.contains("民宅")
        || room_name.contains("居")
        || room_name.contains("齋")
        || room_name.contains("客房")
        || room_name.contains("內室")
        || room_name.contains("宿所")
        || room_name.contains("客寮")
        || room_name.contains("工舍")
        || room_name.contains("住宅")
        || room_name.contains("民居")
        || room_name.contains("小院")
        || room_name.contains("家宅")
        || room_name.contains("宅舍")
        || is_wenyao_residential_or_study(room_name);

    let is_residential_zone = terrain_opt == Some(crate::grid::Terrain::House)
        || has_tag("residence")
        || room_name.contains("住宅")
        || room_name.contains("民居")
        || room_name.contains("宿所")
        || room_name.contains("客寮")
        || room_name.contains("工舍")
        || is_wenyao_residential_or_study(room_name);

    let is_street_or_walkway = has_tag("street")
        || has_tag("road")
        || room_name.contains("外環")
        || room_name.contains("內環")
        || room_name.contains("平坪")
        || room_name.contains("草坪")
        || room_name.contains("步道")
        || room_name.contains("街")
        || room_name.contains("巷");

    let is_waterfront_foundation = terrain_opt.is_some_and(|t| {
        matches!(
            t,
            crate::grid::Terrain::Dock
                | crate::grid::Terrain::Beach
                | crate::grid::Terrain::Water
                | crate::grid::Terrain::WaterDeep
        )
    }) || has_tag("waterfront")
        || has_tag("dock")
        || has_tag("fishing_port")
        || has_tag("beach");

    // 4) 老青竹黃銅輪捲線釣竿：僅當格地基為水岸/碼頭/沙灘，或具備明確漁港/渡埠/釣魚地標，嚴禁於住宅、宿所、外環/內環、平坪、步道、街巷生成
    let is_angling_room = !is_indoor_building
        && !is_residential_zone
        && !is_street_or_walkway
        && (is_waterfront_foundation
            || room_name.contains("釣")
            || room_name.contains("渡埠")
            || room_name.contains("貨船大埠")
            || room_name.contains("泊船")
            || room_name.contains("魚苗")
            || room_name.contains("碼頭")
            || room_name.contains("渡口")
            || room_name.contains("海灣")
            || room_name.contains("淺灣")
            || room_name.contains("礫灣"));
    if is_angling_room && !objects.iter().any(|o| o.has_tag("fishing_rod")) {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "架設在水岸青石旁的精工釣竿，竿身採老山青竹精製，堅韌修長；尾端配有岑氏工坊手工打磨的黃銅滾輪與牛角旋柄，絲線入水隨波蕩漾。".to_string());
        responses.insert("AngleFish".to_string(), "你手持老青竹釣竿揚竿甩線，誘餌破水而入，隨波靜候水底大物咬鉤。".to_string());
        responses.insert("Use".to_string(), "你檢視青竹釣竿的黃銅線輪與釣線張力，整理浮標隨時準備垂釣。".to_string());
        objects.push(RoomObject {
            id: "wt_coastal_angling_spot".to_string(),
            name: "釣竿".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "AngleFish".to_string()],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // 5) 紅土光學黃銅單筒望遠鏡：僅限燈塔、瞭望樓、觀星/觀測台、哨所、海崖、山峰與海岬，嚴禁於一般橋樑或街坊生成
    let is_lookout_room = terrain_opt == Some(crate::grid::Terrain::GuardPost)
        || room_name.contains("瞭望")
        || room_name.contains("觀星")
        || room_name.contains("觀測")
        || room_name.contains("燈塔")
        || room_name.contains("哨")
        || room_name.contains("崖")
        || room_name.contains("峰")
        || room_name.contains("岬");
    if is_lookout_room && !objects.iter().any(|o| o.has_tag("telescope")) {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "架設在堅實石座上的黃銅單筒望遠鏡，筒身銘刻光學工坊刻度，配備雙層紅土消色差透鏡，能極目遠眺數里外的荒野動向。".to_string());
        responses.insert("Operate".to_string(), "你手扶黃銅鏡身旋動對焦螺環，鏡中視野漸漸清澄，將遠方山川風物與生靈行跡盡收眼底。".to_string());
        responses.insert("Use".to_string(), "你湊近望遠鏡目鏡凝神端詳，穿透遠方山嵐薄霧，探查四方地貌與周遭行蹤。".to_string());
        objects.push(RoomObject {
            id: "wt_lookout_telescope".to_string(),
            name: "望遠鏡".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string()],
            responses,
            move_to_room_id: String::new(),
        });
    }

    // 6) 卡式導能便攜爐：僅限獵場、荒野、野外營地，嚴禁於城鎮平坪、街坊或室內生成
    let is_camp_room = !is_indoor_building
        && !is_residential_zone
        && !is_street_or_walkway
        && !room_name.contains("三水")
        && (room_name.contains("獵")
            || room_name.contains("荒野")
            || room_name.contains("營地")
            || room_name.contains("野營"));
    if is_camp_room && !objects.iter().any(|o| o.has_tag("camp_stove")) {
        let mut responses = HashMap::new();
        responses.insert("Look".to_string(), "緊湊輕便的行旅便攜導能爐，配備防風熟鐵環形爐圈與精準出火的紫銅火嘴，可隨時在野外安營起火烹煮。".to_string());
        responses.insert("Cook".to_string(), "你旋開導能便攜爐的銅旋鈕點燃青焰，架起野炊銅鍋，投入食材烹煮熱氣騰騰的野地熱羹。".to_string());
        responses.insert("Use".to_string(), "你調節導能爐盤的火焰大小，煮沸一壺清冽山泉暖手調息。".to_string());
        objects.push(RoomObject {
            id: "wt_field_camp_stove".to_string(),
            name: "便攜爐".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Cook".to_string()],
            responses,
            move_to_room_id: String::new(),
        });
    }
    // 7) 巧工木作檯：工坊、鍛造、鐵匠、木工
    let is_craft_workshop = has_tag("workshop")
        || has_tag("forge")
        || has_tag("blacksmith")
        || has_tag("carpentry");
    if is_craft_workshop
        && !objects.iter().any(|o| o.has_tag("craft_machinery_tools") || o.has_tag("anvil"))
    {
        objects.push(make_peninsula_object(
            "wt_obj_craft_workbench",
            "百工木作檯",
            &["Look", "Use", "Craft"],
            &[
                ("Look", "厚重結實的硬木作檯，檯面上整齊排列著各式鑿子、刨刀、木鋸與標尺，透著工匠嚴謹耐心的手藝氣息。"),
                ("Use", "你俯身整理工作檯上的器皿與工具，檢視工件榫卯嵌合之處，手感精確妥帖。"),
                ("Craft", "你在木作檯前挑選合適的料件與工具細心修琢，打磨邊角，體會巧手工藝的專注樂趣。"),
            ],
        ));
    }

    // 8) 垂柳石欄：橋樑、石橋
    let is_bridge_room = has_tag("bridge");
    if is_bridge_room
        && !objects.iter().any(|o| o.has_tag("railing"))
    {
        objects.push(make_peninsula_object(
            "wt_obj_willow_stone_railing",
            "垂柳石欄",
            &["Look", "Rest"],
            &[
                ("Look", "石橋兩側立著雕刻簡樸的平整青石護欄，橋畔垂柳依依，枝條隨風輕拂石欄。"),
                ("Rest", "你倚靠在微涼的橋石欄旁歇息片刻，看著橋下清波徐徐流淌，水波粼粼。"),
            ],
        ));
    }

    // 9) 青石歇腳凳：園林、草坪
    let is_garden_room = has_tag("garden");
    if is_garden_room
        && !objects.iter().any(|o| o.has_tag("bench"))
    {
        objects.push(make_peninsula_object(
            "wt_street_stone_bench",
            "青石歇腳凳",
            &["Look", "Rest", "Use"],
            &[
                ("Look", "平坪草地旁設有一列平整打磨的青石條凳，石面平滑微涼，供漫步遊人安坐小憩。"),
                ("Rest", "你在青石長凳上舒然落座歇息，迎著江風呼吸帶著草木清香的空氣，心曠神怡。"),
                ("Use", "你拂去石凳上的落葉整肅衣襟，靜坐觀看四周水岸生機，悠閒自在。"),
            ],
        ));
    }

    // 10) 汲水青石階：水井、公井、水步
    let is_well_room = has_tag("well");
    if is_well_room
        && !objects.iter().any(|o| o.has_tag("river_stone_steps") || o.has_tag("water_tap"))
    {
        objects.push(make_peninsula_object(
            "wt_river_stone_steps",
            "汲水青石階",
            &["Look", "Use", "Wash", "Rest", "DrawSpringWell"],
            &[
                ("Look", "平整青石台階直通向清澈水面，階旁繫著兩只浸在水中的竹編魚簍，活水潺潺流淌。"),
                ("Use", "你拾級而下至石階盡頭，俯身掬起一捧清泉洗濯面容，水質甘冽沁骨，整個人神清氣爽。"),
                ("Wash", "你舀起清澈川水洗去雙手與面龐上的微塵，水氣清涼，暑氣與疲倦全消。"),
                ("Rest", "你在石階最高處席地而坐，望著川流碧波隨風泛起漣漪，片刻安寧。"),
                ("DrawSpringWell", "你在水岸公井青石階前汲起清澈甘冽的活水潤嗓，水氣沁涼，與汲水街坊寒暄問好。"),
            ],
        ));
    }

    objects
}
pub fn resolve_player_scene_objects(room_name: &str) -> Vec<RoomObject> {
    player_layer_scene_objects_for_room(room_name)
}
pub fn is_wenyao_residential_or_study(room_name: &str) -> bool {
    let mut clean_name = room_name.trim();
    const SETTLEMENT_PREFIXES: &[&str] = &[
        "文曜・", "文曜",
        "風舶角・", "風舶角",
        "風舶嶼・", "風舶嶼",
        "大圳頭・", "大圳頭",
        "半嶺鎮・", "半嶺鎮",
        "南哨・", "南哨",
        "老堰頭・", "老堰頭",
        "北嶺腳・", "北嶺腳",
        "霜林口・", "霜林口",
        "風櫃尾・", "風櫃尾",
        "烏石鼻・", "烏石鼻",
        "牛埔・", "牛埔",
        "紅土崁・", "紅土崁",
        "三號坑口・", "三號坑口",
        "水尾寮・", "水尾寮",
    ];
    let mut changed = true;
    while changed {
        changed = false;
        for p in SETTLEMENT_PREFIXES {
            if let Some(rest) = clean_name.strip_prefix(p) {
                clean_name = rest.trim();
                changed = true;
                break;
            }
        }
    }
    const WARD_PREFIXES: &[&str] = &[
        "講堂外坊・", "講堂外坊", "三水外坊・", "三水外坊",
        "百工外坊・", "百工外坊", "西門外坊・", "西門外坊",
        "講堂街・", "講堂街", "三水口・", "三水口",
        "分家巷・", "分家巷", "西門・", "西門",
    ];
    for p in WARD_PREFIXES {
        if let Some(rest) = clean_name.strip_prefix(p) {
            clean_name = rest.trim();
        }
    }

    const EXCLUDE: &[&str] = &[
        "旅社", "驛館", "茶屋", "茶棚", "浴場", "商行", "學舍", "試招院", "公議", "工坊", "作坊",
        "書院", "外院", "醫館", "藥棧", "大埠", "拍賣", "大鍛坊", "鋸木坊", "精煉坊", "木炭窯",
        "磚瓦窯", "工具行", "織造坊", "傢俱坊", "服飾坊", "烘藥房", "五金鋪", "造船塢", "竹木場", "大貨棧", "中轉站", "拍賣行",
        "鹽鐵庫", "修造場", "獸醫館", "武備鋪", "酒坊", "印書局", "裝幀鋪", "文具行", "觀測台",
        "傳習所", "哨所", "公所", "診所",
        "鐘樓", "望野樓", "哨樓", "礦務所", "觀測所", "瞭望所", "中繼站", "調配站", "調撥處",
        "集散行", "收購行", "特產行", "物資行", "總社", "公會",
        "乾貨", "海味", "大食堂", "食堂", "曬書", "公井", "觀星台", "觀星",
    ];
    if EXCLUDE.iter().any(|ex| clean_name.contains(ex)) {
        return false;
    }

    const CONTAIN: &[&str] = &[
        "住宅", "民宅", "居所", "家屋", "宅邸", "宅院", "民舍", "人家", "小築", "宿所",
        "學寮", "員寮", "山房", "養馬莊", "莊園",
    ];
    if CONTAIN.iter().any(|kw| clean_name.contains(kw)) {
        return true;
    }

    const END_CHARS: &[char] = &['居', '宅', '齋', '廬', '院', '舍', '家', '築', '所', '寮', '房', '莊', '屋', '樓'];
    if let Some(last_ch) = clean_name.chars().last() {
        if END_CHARS.contains(&last_ch) {
            return true;
        }
    }
    false
}

fn normalize_peninsula_object_name(name: &str) -> String {
    const MATERIAL_PREFIXES: &[&str] = &[
        "黃銅鑲邊", "黃銅包邊", "黃銅桿", "黃銅", "雕花青石", "青石", "石雕", "生鐵", "熟鐵",
        "精鋼", "防蛀樟木", "樟木", "松香木", "松木", "烏木", "紅木", "鐵木", "楠木",
        "粗陶", "白瓷", "防雨竹筒", "防雨竹編", "竹筒", "竹編", "寒水玉", "夜光貝母",
        "夜光貝", "貝母", "單筒", "捲線", "老山青竹", "老青竹", "青竹",
    ];

    let mut cur = name.trim();
    loop {
        let mut stripped = false;
        for &pfx in MATERIAL_PREFIXES {
            if let Some(rest) = cur.strip_prefix(pfx) {
                let rest_trimmed = rest.trim();
                if rest_trimmed.chars().count() >= 2 {
                    cur = rest_trimmed;
                    stripped = true;
                    break;
                }
            }
        }
        if !stripped {
            break;
        }
    }

    // Specific noun mappings if any
    match cur {
        "告示欄" | "公佈欄" | "告示板" => "告示欄".to_string(),
        "青石欄" | "雕花石欄" | "橋欄杆" | "護欄" => "橋欄".to_string(),
        "指路柱" | "日晷柱" => "日晷".to_string(),
        "暖床" | "大床" | "彈力床" => "床榻".to_string(),
        "活水栓" | "自來水栓" => "水栓".to_string(),
        "冰櫃" | "保鮮石櫃" | "石櫃" | "儲食石櫃" => "冰鑑櫃".to_string(),
        other => other.to_string(),
    }
}

fn clean_read_narrative(read_text: &str) -> String {
    let trimmed = read_text.trim();
    const PREFIXES: &[&str] = &[
        "你走近告示欄細讀通告，",
        "你走近公佈欄細閱，",
        "你細讀櫃檯告示：",
        "你細讀櫃頂張貼的三水總商行通兌章程，",
        "你走近石柱研讀鐫刻的方位路引：",
        "你走近立柱端讀鐫刻的商貿契稅規章與每日各港口船班、車隊發貨時序。",
        "你細讀帳冊上的公道行情：",
        "你細閱屋契條款，",
        "你逐條細讀告示內容，",
        "你逐行研讀告示板上的急件委託，",
    ];
    for &pfx in PREFIXES {
        if let Some(rest) = trimmed.strip_prefix(pfx) {
            return rest.trim().to_string();
        }
    }

    // Generic prefixes like "你走近……細閱，", "你細讀……："
    if let Some(rest) = trimmed.strip_prefix("你走近") {
        if let Some(pos) = rest.find("細閱，") {
            return rest[pos + "細閱，".len()..].trim().to_string();
        }
        if let Some(pos) = rest.find("細讀，") {
            return rest[pos + "細讀，".len()..].trim().to_string();
        }
        if let Some(pos) = rest.find("細閱：") {
            return rest[pos + "細閱：".len()..].trim().to_string();
        }
        if let Some(pos) = rest.find("細讀：") {
            return rest[pos + "細讀：".len()..].trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("你細讀") {
        if let Some(pos) = rest.find('：') {
            return rest[pos + '：'.len_utf8()..].trim().to_string();
        }
    }
    if let Some(rest) = trimmed.strip_prefix("你細閱") {
        if let Some(pos) = rest.find('：') {
            return rest[pos + '：'.len_utf8()..].trim().to_string();
        }
    }

    trimmed.to_string()
}

fn make_peninsula_object(
    id: &str,
    name: &str,
    sockets: &[&str],
    responses: &[(&str, &str)],
) -> RoomObject {
    let clean_name = normalize_peninsula_object_name(name);
    let mut resp_map = std::collections::HashMap::new();
    for (k, v) in responses {
        resp_map.insert((*k).to_string(), (*v).to_string());
    }

    if let (Some(look_text), Some(read_text)) = (resp_map.get("Look").cloned(), resp_map.get("Read").cloned()) {
        if !look_text.contains(&read_text) {
            let cleaned_read = clean_read_narrative(&read_text);
            if !look_text.contains(&cleaned_read) {
                let combined_look = format!("{}{}", look_text.trim(), cleaned_read);
                resp_map.insert("Look".to_string(), combined_look);
            }
        }
    }

    let mut filtered_sockets: Vec<String> = sockets
        .iter()
        .filter(|&&s| s != "Use" && s != "Operate" && s != "Read")
        .map(|s| (*s).to_string())
        .collect();

    if !filtered_sockets.iter().any(|s| s == "Look") {
        filtered_sockets.insert(0, "Look".to_string());
    }

    RoomObject {
        id: id.to_string(),
        name: clean_name,
        owner: String::new(),
        sockets: filtered_sockets,
        responses: resp_map,
        move_to_room_id: String::new(),
    }
}

pub fn is_white_tower_peninsula_room(room_name: &str) -> bool {
    const PENINSULA_PLACES: &[&str] = &[
        "文曜", "風舶", "大圳頭", "半嶺", "南哨", "老堰頭", "北嶺",
        "霜林", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮",
        "白塔",
    ];
    PENINSULA_PLACES.iter().any(|p| room_name.contains(p))
}


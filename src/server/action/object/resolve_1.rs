fn resolve_special_room_object_part1(room_id: &str, target: &str) -> Option<crate::model::RoomObject> {
    if target == "room_interior"
        || target == "wt_reg_wenyao_home"
        || target.contains("大床")
        || target.contains("羽絨暖被")
        || target.contains("臥榻")
    {
        return Some(crate::model::RoomObject {
            id: "wt_reg_wenyao_home".to_string(),
            name: "床榻".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Use".to_string(),
                "Rest".to_string(),
                "Sleep".to_string(),
                "Operate".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，床旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。".to_string()),
                ("Use".to_string(), "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞與肌肉酸脹頓時消散大半，身心漸漸平復放鬆。".to_string()),
                ("Rest".to_string(), "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。".to_string()),
                ("Sleep".to_string(), "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母檯燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。".to_string()),
                ("Operate".to_string(), "你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_inn_bath_Preview" || target.contains("自來活水黃銅水栓") || target.contains("溫石熱水槽") {
        return Some(crate::model::RoomObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "沐浴桶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Wash".to_string(), "Use".to_string(), "Bath".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "打磨光滑的黃銅水栓連接著暗渠活水管道，下方是紅土耐火磚砌成的溫石蓄熱水槽，槽邊備有山茶松脂皂與棉長絨擦巾。".to_string()),
                ("Wash".to_string(), "你踏入溫熱的杉木浴桶中浸泡洗沐，溫潤泉水洗去滿身風塵與汗漬，四肢百骸說不出的舒暢通透。".to_string()),
                ("Use".to_string(), "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，草木皂香格外清新。".to_string()),
                ("Bath".to_string(), "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }

    if target == "wt_home_shell_lamp" || target.contains("旋蓋夜光貝母") || target.contains("樟木衣箱") || target.contains("貝母檯燈") || target.contains("貝母台燈") {
        return Some(crate::model::RoomObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "檯燈".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "Open".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "松木几案上立著旋蓋夜光貝母調光檯燈，旁側設有一只包銅角的防蛀樟木衣箱，箱蓋微敞散發幽淡樟腦木香。".to_string()),
                ("Use".to_string(), "你旋亮夜光貝母檯燈，珠白光暈柔和灑滿几案與床鋪，室內一片溫馨安寧。".to_string()),
                ("Operate".to_string(), "你旋轉烏木外罩調節進光弧度，光影隨心明暗，極具生活雅趣。".to_string()),
                ("Open".to_string(), "你揭開樟木衣箱銅扣，箱內分層鋪著吸濕防潮的竹綿細布，可妥善存放換洗衣物與貴重行囊。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_academy_fountain_pen"
        || target.contains("硬筆")
        || target.contains("黃銅硬筆")
        || target.contains("黃銅儲墨筆")
    {
        return Some(crate::model::RoomObject {
            id: "wt_academy_fountain_pen".to_string(),
            name: "黃銅儲墨筆".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "AttendLecture".to_string()],
            responses: std::collections::HashMap::from([
                (
                    "Look".to_string(),
                    "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨硬筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。".to_string(),
                ),
                (
                    "Use".to_string(),
                    "你執起教案上的黃銅硬筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。".to_string(),
                ),
                (
                    "Operate".to_string(),
                    "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。".to_string(),
                ),
                (
                    "AttendLecture".to_string(),
                    "你端坐於講席側畔，執筆旁聽教習策論，凝神研討時務經世之道。".to_string(),
                ),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "dyn_campfire" || target.contains("野地營火堆") || target.contains("營火") {
        return Some(crate::model::RoomObject {
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
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "粗壯硬柴交錯架起的野地營火堆正熊熊燃燒，金紅火苗隨風輕躍，散發著令人心神安定的融融暖意與松木焦香。".to_string()),
                ("WarmHands".to_string(), "你湊近營火伸出雙手，融融暖流自掌心順著經脈蔓延至周身四肢，驅散了荒野寒濕。".to_string()),
                ("RoastFood".to_string(), "你取出行囊中的乾糧肉脯架於熾熱柴炭上方炙烤，油脂滋滋作響、肉香四溢，趁熱嚼食腹中溫暖充實。".to_string()),
                ("CraftTorch".to_string(), "你自營火堆中抽出一根引燃的松脂硬木，細心纏裹封固，製成一柄劈啪作響的照明松明火把。".to_string()),
                ("AddFuel".to_string(), "你拾起乾柴添入火堆中央，劈啪脆響中火勢陡然騰起，騰躍的金紅火舌將營地周遭照耀得一片透亮。".to_string()),
                ("CollectCharcoal".to_string(), "你以長枝撥開外層浮灰，將底層悶燒透紅的硬木火炭小心夾出、封土隔氧冷卻，收整出成色極佳的無煙白炭。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_home_bamboo_mailbox" || target.contains("竹編信箱") || target.contains("防雨信箱") {
        return Some(crate::model::RoomObject {
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
        });
    }
    if target == "wt_courier_post_desk" || target.contains("信使櫃檯") || target.contains("信使總站") || target.contains("草獸信使") {
        return Some(crate::model::RoomObject {
            id: "wt_courier_post_desk".to_string(),
            name: "疾行草獸信使櫃檯".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Read".to_string(),
                "DispatchCourier".to_string(),
                "Use".to_string(),
            ],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "疾行草獸信使櫃檯前陳列著往返四鎮的驛道封冊、火漆印章與加急竹筒，案旁栓著數頭蓄勢待發的健碩草獸。".to_string()),
                ("Read".to_string(), "你細讀櫃檯告示：草獸信使日夜兼程往來四鎮，承辦跨聚落加急包裹與重金文卷託運，每件資費 4 鎂。".to_string()),
                ("DispatchCourier".to_string(), "你向信使櫃檯交付 4 鎂託運規費，將重要急件交付草獸信使快騎飛遞。".to_string()),
                ("Use".to_string(), "你走近信使櫃檯，向值守掌事探詢各鎮驛道最新路況與班次安排。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_cat_0790" || target == "wt_bank_ledger_drawer" || target.contains("存摺櫃") || target.contains("分格存摺") {
        return Some(crate::model::RoomObject {
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
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚重鐵木所造的分格存摺櫃，抽屜銅扣皆刻有三水口流水徽印，收存著各大商隊與鎮民的通兌簿冊與印鑑。".to_string()),
                ("Read".to_string(), "你細讀櫃頂張貼的三水總商行通兌章程，載明各港口通兌匯水與信用票號兌領條款。".to_string()),
                ("DepositBank".to_string(), "你在掌事案前出示存摺與信印，將隨身鎂幣妥善存入商行票號帳頭。".to_string()),
                ("WithdrawBank".to_string(), "你憑分格存摺防偽硃印與掌事核對數目，自票號結存中支取現鎂。".to_string()),
                ("Use".to_string(), "你走近分格存摺櫃，向值守執事出示存摺辦理商行通兌手續。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_home_jade_cabinet" || target.contains("寒水玉保鮮石櫃") || target.contains("寒水玉雙層保鮮石櫃") || target.contains("保鮮石櫃") {
        return Some(crate::model::RoomObject {
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
        });
    }
    if target == "wt_coastal_angling_spot" || target == "wt_cat_1006" || target.contains("老青竹黃銅輪捲線釣竿") || target.contains("老青竹釣竿") || target.contains("捲線釣竿") {
        return Some(crate::model::RoomObject {
            id: "wt_coastal_angling_spot".to_string(),
            name: "釣竿".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "AngleFish".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "架設在水岸青石旁的精工釣竿，竿身採老山青竹精製，堅韌修長；尾端配有岑氏工坊手工打磨的黃銅滾輪與牛角旋柄，絲線入水隨波蕩漾。".to_string()),
                ("AngleFish".to_string(), "你手持老青竹釣竿揚竿甩線，誘餌破水而入，隨波靜候水底大物咬鉤。".to_string()),
                ("Use".to_string(), "你檢視青竹釣竿的黃銅線輪與釣線張力，整理浮標隨時準備垂釣。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_lookout_telescope" || target == "wt_cat_0733" || target.contains("紅土光學黃銅單筒望遠鏡") || target.contains("黃銅單筒望遠鏡") || target.contains("單筒望遠鏡") {
        return Some(crate::model::RoomObject {
            id: "wt_lookout_telescope".to_string(),
            name: "望遠鏡".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "架設在堅實石座上的黃銅單筒望遠鏡，筒身銘刻光學工坊刻度，配備雙層紅土消色差透鏡，能極目遠眺數里外的荒野動向。".to_string()),
                ("Operate".to_string(), "你手扶黃銅鏡身旋動對焦螺環，鏡中視野漸漸清澄，將遠方山川風物與生靈行跡盡收眼底。".to_string()),
                ("Use".to_string(), "你湊近望遠鏡目鏡凝神端詳，穿透遠方山嵐薄霧，探查四方地貌與周遭行蹤。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_field_camp_stove" || target == "wt_cat_1010" || target.contains("卡式導能便攜爐") || target.contains("導能便攜爐") || target.contains("便攜爐") {
        return Some(crate::model::RoomObject {
            id: "wt_field_camp_stove".to_string(),
            name: "卡式導能便攜爐".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Cook".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "緊湊輕便的行旅便攜導能爐，配備防風耐熱精鋼環形爐圈與精準出火的紫銅火嘴，可隨時在野外安營起火烹煮。".to_string()),
                ("Cook".to_string(), "你旋開導能便攜爐的銅旋鈕點燃青焰，架起野炊銅鍋，投入食材烹煮熱氣騰騰的野地熱羹。".to_string()),
                ("Use".to_string(), "你調節導能爐盤的火焰大小，煮沸一壺清冽山泉暖手調息。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
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
    if target == "wt_multi_firewood_pile" || target.contains("風乾柴木與良木料垛") || target.contains("木料垛") || target.contains("柴木垛") {
        return Some(crate::model::RoomObject {
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
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "整齊碼放於簷下或角隅的風乾柴木與硬木料垛，木質乾燥密實，既是用以生火禦寒之良材，亦能順紋劈柴、裁鋸木板、進爐悶燒精炭或車削長柄。".to_string()),
                ("Gather".to_string(), "你自木料垛中細細翻揀，抱出一根紋理密實的伐採原木與兩束乾燥柴枝收整妥當。".to_string()),
                ("MakeFire".to_string(), "你取出乾燥柴枝引燃火星，在空地中央架起通風柴堆，迅速生起一堆溫暖明亮的野地營火。".to_string()),
                ("SplitWood".to_string(), "你運力順著木紋利索剖劈，將原木細細剖分劈成易於引火的乾燥柴枝束。".to_string()),
                ("RefinePlank".to_string(), "你推動長鋸順紋切削，將沉實原木裁鋸打磨為平整規整的樟松木板料。".to_string()),
                ("BurnCharcoal".to_string(), "你將截切硬木置入坑窯泥封悶燒，凝煉出無煙透亮的硬木白炭。".to_string()),
                ("CraftTorch".to_string(), "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的浸脂松明火把。".to_string()),
                ("RepairStructure".to_string(), "你挑選合適的木板料與工具長柄，加固修整破損的欄杆踏板與設施構件。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }


    if target.starts_with("hunt_kill_site_") || target.ends_with("・圍獵遺骸") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("hunt_kill_site_") {
                target.to_string()
            } else {
                "hunt_kill_site_0_0".to_string()
            },
            name: if target.ends_with("・圍獵遺骸") {
                target.to_string()
            } else {
                "圍獵遺骸".to_string()
            },
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Harvest".to_string(),
                "Gather".to_string(),
                "Take".to_string(),
            ],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }

    if target.starts_with("detritus_fungi_") || target.contains("腐木牛樟靈菇叢") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("detritus_fungi_") {
                target.to_string()
            } else {
                "detritus_fungi_0_0".to_string()
            },
            name: "腐木牛樟靈菇叢".to_string(),
            owner: String::new(),
            sockets: vec![
                "Look".to_string(),
                "Harvest".to_string(),
                "Gather".to_string(),
                "Take".to_string(),
            ],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }

    if target == "shanhai_apex_trace" || target.contains("新鮮霸主獸跡") {
        return Some(crate::model::RoomObject {
            id: "shanhai_apex_trace".to_string(),
            name: "新鮮霸主獸跡".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string()],
            responses: std::collections::HashMap::new(),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dock_riverbank_raft" || target.contains("平底竹筏") {
        return Some(crate::model::RoomObject {
            id: "wt_dock_riverbank_raft".to_string(),
            name: "平底竹筏".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string(), "RowSwampRaft".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "水岸石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著數艘雙層防浸平底竹筏，筏上置有推水長竹篙與青竹魚簍。".to_string()),
                ("Operate".to_string(), "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。".to_string()),
                ("Use".to_string(), "你手持長竹篙在石階輕撐借力，竹筏滑入川流清澈碧流，沿岸水草豐美、水鳥掠波。".to_string()),
                ("RowSwampRaft".to_string(), "你解開纜繩點篙撐起竹筏，輕舟破開水尾寮蘆葦與黑沼迷霧，順流探尋澤中沉水烏木與止血水苔。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_inn_ebony_shell_chess" || target.contains("烏木棋盤") {
        return Some(crate::model::RoomObject {
            id: "wt_inn_ebony_shell_chess".to_string(),
            name: "烏木棋盤".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "PlayChess".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "黑沼沉水烏木雕琢的棋盤線道深沉分明，兩只棋罐各盛著溫潤厚重的烏木黑子與烏石鼻厚海貝白子，茶客對弈殺局難解難分。".to_string()),
                ("Operate".to_string(), "你拈起一枚沉甸甸的白貝棋子扣在棋盤交叉處，發出清脆沉實的篤響，盤中局勢頓時生出新意。".to_string()),
                ("PlayChess".to_string(), "你端坐於烏木棋盤前拈子佈局，黑白交錯推演天地之數，心神凝定，雜念俱泯。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dining_iron_cauldron" || target.contains("紅鐵燉肉釜") {
        return Some(crate::model::RoomObject {
            id: "wt_dining_iron_cauldron".to_string(),
            name: "紅鐵燉肉釜".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "WarmHands".to_string(), "SeasonalBanquet".to_string(), "AssistKitchen".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅鴛鴦鍋滾水翻騰，備有切仔湯麵配滷蛋與大圳頭溫米酒。".to_string()),
                ("Taste".to_string(), "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。".to_string()),
                ("WarmHands".to_string(), "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。".to_string()),
                ("SeasonalBanquet".to_string(), "你在紅鐵釜前添薪圍坐，鍋內歲時時令熱氣翻騰，同席之人舉杯共飲，暖意直透肺腑。".to_string()),
                ("AssistKitchen".to_string(), "你持長柄木杓在紅鐵燉肉釜前幫廚添薪撇沫，慢火細燉大角原牛肉湯。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_mill_") || target.contains("水輪玄武磨") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_mill_waterwheel_stone".to_string() },
            name: "水輪玄武磨".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "MillGrain".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "老堰頭引水渠衝擊巨木水輪帶動齒輪轉動，兩合巨大玄武岩磨盤正飛速研磨新熟米穀。".to_string()),
                ("Operate".to_string(), "你扳動水流分水閘柄調節水勢，水輪轉速隨之放緩，磨盤研磨出的白米粉更顯細密勻淨。".to_string()),
                ("MillGrain".to_string(), "你合力推開水磨料門，巨輪轉動帶動石磨隆隆轟鳴，白米碎成細漿，研磨出老堰頭溫熱糙米漿。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_sawmill_") || target.contains("水動排鋸架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_sawmill_hydraulic_saw".to_string() },
            name: "水動排鋸架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "RefinePlank".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "水流帶動的長排精鋼鋸條轟鳴起伏，平穩將整根百年雲杉原木剖切成均勻厚板。".to_string()),
                ("Operate".to_string(), "你推動送木台車向前咬緊鋸齒，鋼鋸飛速下切，鋸路筆直順暢，木屑紛飛。".to_string()),
                ("RefinePlank".to_string(), "你推動水動排鋸咬合雲杉巨木，精鋼鋸齒飛速剖切，裁出筆直平整的樟松木板料。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_stonecut_") || target.contains("金剛砂拉鋸") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_stonecut_wire_saw".to_string() },
            name: "金剛砂拉鋸".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "CutJade".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工匠引細密合金絲拉鋸帶著濕潤金剛砂緩緩剖開巨大的北山寒水玉原石，石皮下透出溫潤玉光。".to_string()),
                ("Operate".to_string(), "你輕轉注水輪維持金剛砂泥漿順暢流入切割線，金屬絲勻速拉動，石料緩緩剖為兩半。".to_string()),
                ("CutJade".to_string(), "你拉動金剛砂鋼絲鋸緊貼寒水玉原石，冷泉注淋鋸槽，細細切剖出溫潤剔透的寒水玉料。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_cheese_") || target.contains("螺旋壓酪架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cheese_curd_press".to_string() },
            name: "螺旋壓酪架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Taste".to_string(), "PressCheese".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "大木架上設有黃銅螺旋加壓手柄，正將粗濾布中的凝乳塊瀝出金黃乳清，壓製硬乳酪輪。".to_string()),
                ("Operate".to_string(), "你握住螺旋桿加力旋轉半圈，木板沉重下壓，澄澈的乳清順著凹槽汩汩流出。".to_string()),
                ("Taste".to_string(), "你嚐了指尖沾上的一滴新鮮乳清，滋味微酸清冽，帶著濃醇的天然奶香。".to_string()),
                ("PressCheese".to_string(), "你旋緊螺旋木桿壓榨酪包，黃亮乳清緩緩瀝出，壓製出緊實濃郁的熟成硬乳酪。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_mine_") || target.starts_with("wt_trolley_") || target.contains("雙軸重台車") || target.contains("木製洗礦槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_trolley_rail_carriage".to_string() },
            name: "雙軸重台車".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "HaulMineCart".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。".to_string()),
                ("Operate".to_string(), "你握住車身旁的機械制動閘手柄扳動，鐵閘緊緊咬住輪緣，發出沉實摩擦聲。".to_string()),
                ("HaulMineCart".to_string(), "你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴，將滿車赤鐵重礦運出坑道。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_smokehouse_") || target.contains("慢燻紅陶灶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_smokehouse_kiln".to_string() },
            name: "慢燻紅陶灶".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Smell".to_string(), "SmokeFish".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "地面砌出長排低溫煙道陶灶，正以受控的乾燥松木鋸屑慢火逼出濃郁木香冷煙。".to_string()),
                ("Smell".to_string(), "你深吸一口自陶灶溢出的煙氣，醇厚的松木煙燻香氣純淨甘醇，全無嗆鼻焦味。".to_string()),
                ("SmokeFish".to_string(), "你添入濕柏木與香草慢燻，灶內青煙繚繞，低溫透骨冷燻海鹽魚乾。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_lime_") || target.contains("海貝燃灰窯") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_lime_kiln_chamber".to_string() },
            name: "海貝燃灰窯".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "WarmHands".to_string(), "Operate".to_string(), "BurnLime".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "烏石鼻以玄武岩壘砌的圓形灰窯，烈火正將堆積的海貝殼高溫煅燒成雪白的建材灰粉。".to_string()),
                ("WarmHands".to_string(), "你將身子貼近灰窯厚實的外牆，石壁蓄積的渾厚餘熱烘散了海邊帶來的陰冷濕氣。".to_string()),
                ("Operate".to_string(), "你使用長柄鐵扒撥動窯底通風孔讓空氣流通，爐膛火勢更加旺盛純青。".to_string()),
                ("BurnLime".to_string(), "你添入焦炭鼓風引燃灰窯，高溫煅燒海貝碎殼，出窯白灰熱氣騰騰，細密凝實。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_hunt_") || target.contains("牛角破甲弩") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_hunt_heavy_crossbow".to_string() },
            name: "牛角破甲弩".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "PrepareMiasmaGear".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工坊以大角原牛角片與堅木多層膠合的守林重弩，弩弦以原牛筋絞就，張力沉雄。".to_string()),
                ("Operate".to_string(), "你扳動腳蹬絞盤為重弩上弦，機括發出清脆卡響，鎖定弩機扣板，蓄勢待發。".to_string()),
                ("PrepareMiasmaGear".to_string(), "你校準破甲弩機，配齊紅磷引路信號筒與避瘴藥劑，整飭行囊以防黑沼瘴毒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    None
}

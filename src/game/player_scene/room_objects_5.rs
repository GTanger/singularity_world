#[allow(unused_variables)]
fn peninsula_objects_chunk_5(room_name: &str) -> Option<Vec<RoomObject>> {
    let is_peninsula = is_white_tower_peninsula_room(room_name);
    let is_outdoor_road = room_name.contains('街')
        || room_name.contains('道')
        || room_name.contains('巷')
        || room_name.contains('橋')
        || room_name.contains("石階")
        || room_name.contains("石坪")
        || room_name.contains("曬場")
        || room_name.contains("檢修坪")
        || room_name.contains("十字口")
        || room_name.contains("北口")
        || room_name.contains("南口")
        || room_name.contains("東口")
        || room_name.contains("西口");
    let is_residential_house = room_name.ends_with('宅')
        || room_name.ends_with('居')
        || room_name.contains("民宅")
        || room_name.contains("家宅")
        || room_name.contains("老宅");

    let objs = if room_name.contains("北門") {
        if room_name.contains("船埠路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_wharf_bollard",
                    "生鐵繫纜樁",
                    &["Look", "Tie", "LookShip"],
                    &[
                        ("Look", "條石駁岸邊深埋著粗壯的生鐵繫纜樁，樁身受粗麻纜繩摩挲得油黑發亮，駁岸沿線排滿厚重防撞松木垛。"),
                        ("Tie", "你將隨身繫繩熟練地在生鐵繫纜樁上打了個牢固的雙套結，纜結吃勁穩當。"),
                        ("LookShip", "你放眼望向泊位，數艘雙桅載重貨船隨著江波起伏，船工正忙碌裝卸貨包。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_wharf_pulley_hoist",
                    "滑輪吊架",
                    &["Look", "Operate"],
                    &[
                        ("Look", "埠頭路口架設著以粗大雲杉木與生鐵滑輪組裝的起重吊架，垂掛著多股絞緊的粗麻吊繩。"),
                        ("Operate", "你拉了拉吊架垂落的麻繩，滑輪轉動平順，承重吊鉤牢固可靠。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("學院分院路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_south_road_milestone",
                    "指路石碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "路口立著一座四稜青石指路短碑，四面分別刻有通往學院各分院與城北碼頭的指引。"),
                        ("Read", "你辨讀指路石碑上的箭頭與工整刻字，分院方位一目了然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_news_board",
                    "旬報閱報欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及蒙學堂告示。"),
                        ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("林蔭") {
            vec![
                make_peninsula_object(
                    "wt_obj_chinaberry_stone_bench",
                    "苦楝樹石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "林蔭路旁高大苦楝樹下，整齊安放著平整的條石長凳，樹影婆娑，涼風習習。"),
                        ("Rest", "你在苦楝樹下的石凳上歇息，細碎陽光透落石面，步調安詳寧靜。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_reg_wenyao_street",
                    "告示欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "平整石板街邊立著雕工細緻的黃銅包邊木質告示欄，張貼著文曜各工坊、商行與學院評議會的最新公議告示。"),
                        ("Read", "你走近告示欄細讀通告，字跡工整端麗，上載各城鎮商船班次、工坊學徒招募與物資公道定價。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_obj_drainage_stone_ditch",
                    "排水青石渠",
                    &["Look", "Check"],
                    &[
                        ("Look", "街石兩側鋪設著規整開敞的青石排水明渠，活水自暗涵穿流而過，渠水清澈無濁。"),
                        ("Check", "你俯身查看石渠流水，渠壁平滑無淤，水流暢達排向外城水門。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_hitching_stone_post",
                    "拴馬石樁",
                    &["Look", "Use"],
                    &[
                        ("Look", "道旁埋設著鑿有穿繩圓孔的厚重青石短樁，頂部打磨成圓鼓形，石孔已被麻繩摩得油光鋥亮。"),
                        ("Use", "你伸手試了試拴馬石樁的穩固程度，石樁深植地底，紋絲不動。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        }
    } else if room_name.starts_with("文曜") && (room_name.contains("三水") || room_name.contains("商號街")) {
        if room_name.contains("商號街") {
            vec![
                make_peninsula_object(
                    "wt_abacus_fair_price_ledger",
                    "行商價目冊",
                    &["Look", "Read"],
                    &[
                        ("Look", "厚實的松木帳案上攤開著《各地行商貨價公道帳冊》，墨跡清晰，記載著半島各坊各埠即時市價。"),
                        ("Read", "你細讀價目冊上的物產行市，生鐵、宣紙、海鹽與寒水玉的公道兌換比例一目了然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_market_scale_counter",
                    "黃銅雙盤秤",
                    &["Look", "Operate"],
                    &[
                        ("Look", "懸在木架上的精準黃銅雙盤大吊秤，配備一套由小至大的青銅砝碼，用於買賣過磅。"),
                        ("Operate", "你輕撥雙盤吊秤的黃銅提紐，兩端精準平衡，指針端直無偏。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("外環北道") || room_name.contains("外環西道") {
            vec![
                make_peninsula_object(
                    "wt_obj_flood_dike_pile",
                    "防汛石堤樁",
                    &["Look", "Check"],
                    &[
                        ("Look", "外環石堤外緣深打著粗壯整齊的花崗岩防汛石樁，頂部套有生鐵箍，能迎抗大潮衝擊。"),
                        ("Check", "你拍了拍冰冷沉實的石堤樁，樁身受巨浪沖刷多年依舊嚴絲合縫。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_drying_nets_trough",
                    "苧麻曬網架",
                    &["Look", "Inspect", "Open"],
                    &[
                        ("Look", "露天院落中橫架著數十丈長的曬網木架，晾曬著浸染柿漆防腐的深褐色苧麻大網，廊下整齊排列著加蓋重石的鹽漬大陶缸。"),
                        ("Inspect", "你走近翻看懸掛的苧麻大網，網線以熟桐油與柿漆浸透，柔韌緊密，網目結實無損。"),
                        ("Open", "你揭開廊下一隻鹽漬陶缸的木蓋，一股濃郁鮮鹹的醃漬海味混合著粗海鹽氣息迎面撲來。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_bridge_harbor_causeway",
                    "防風航標燈",
                    &["Look", "Rest", "Operate"],
                    &[
                        ("Look", "跨海石橋橫臥於碧波海灣之上，厚實防風石欄抵禦著長風與浪沫，石欄轉折處設有旋蓋夜光貝母航標燈，指引往來舟楫與行旅。"),
                        ("Rest", "你在石橋避風凹處倚欄小憩，海浪拍擊橋墩白浪翻滾，海風拂衣。"),
                        ("Operate", "你轉動夜光貝母燈罩調準射角，柔和珠白強光穿透夜霧，指引遠方船影。"),
                    ],
                ),
            ]
        } else if room_name.contains("內環") {
            vec![
                make_peninsula_object(
                    "wt_obj_sanshui_stone_culvert",
                    "引水青石渠",
                    &["Look", "Check"],
                    &[
                        ("Look", "沿內環街道鋪展的石砌水渠引流三水清波，渠底鋪有細礫，活水長流不息。"),
                        ("Check", "你注視著渠中清澈活水順暢奔流，渠畔水生草葉隨波輕擺。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_willow_stone_seat",
                    "垂柳石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "臨渠垂柳樹蔭下安放著青條石座，石面被流水汽息浸染得溫潤生涼。"),
                        ("Rest", "你在垂柳旁的石凳上落座，看著清渠落葉漂流，愜意從容。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else {
            let is_residence = room_name.contains("宿所")
                || room_name.contains("客寮")
                || room_name.contains("工舍")
                || room_name.contains("家宅")
                || room_name.contains("宅舍")
                || room_name.contains("小院")
                || room_name.contains("聚義舍")
                || room_name.contains("哨居")
                || room_name.contains("安居宅")
                || room_name.contains("纜樁居")
                || room_name.contains("信鴿棲籠宅")
                || room_name.contains("住宅")
                || room_name.contains("宅邸")
                || room_name.contains("民宅")
                || room_name.contains("民居")
                || room_name.contains("居所")
                || is_wenyao_residential_or_study(room_name);
            let derived_room_tags = crate::grid::derive_room_tags(crate::grid::Terrain::Road, room_name, "");
            let has_derived_tag = |wanted: &str| -> bool {
                let canon = crate::grid::canonical_room_tag(wanted);
                derived_room_tags.iter().any(|t| {
                    t == wanted || (!canon.is_empty() && crate::grid::canonical_room_tag(t) == canon)
                })
            };
            let is_craft_workshop = has_derived_tag("workshop")
                || has_derived_tag("forge")
                || has_derived_tag("blacksmith")
                || has_derived_tag("carpentry")
                || room_name.contains("修網結繩工坊")
                || room_name.contains("船用桐油作坊")
                || room_name.contains("浮標竹笛作坊")
                || room_name.contains("水產乾貨儲庫")
                || room_name.contains("工坊")
                || room_name.contains("作坊")
                || room_name.contains("儲庫");
            let is_water_steps = has_derived_tag("well")
                || room_name.contains("河階渡埠")
                || room_name.contains("鮮魚早市石階")
                || room_name.contains("沿河洗滌石磯")
                || room_name.contains("水井")
                || room_name.contains("公井")
                || room_name.contains("汲水");
            let is_bridge = has_derived_tag("bridge")
                || room_name.contains("石橋")
                || room_name.contains("跳石橋")
                || room_name.contains("石平橋")
                || room_name.contains("拱橋")
                || room_name.contains("外環西北隅")
                || room_name.contains('橋');
            let is_garden_lawn = has_derived_tag("garden")
                || room_name.contains("水岸垂柳苑")
                || room_name.contains("水邊白鷺草坪")
                || room_name.contains("芳草清水池")
                || room_name.contains("綠草平坪")
                || room_name.contains("草坪")
                || room_name.contains("花圃")
                || room_name.contains("苑")
                || room_name.contains("草池");

            if is_residence {
                vec![
                    make_peninsula_object(
                        "wt_home_bed",
                        "床榻",
                        &["Look", "Use", "Rest", "Sleep"],
                        &[
                            ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。"),
                            ("Use", "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞頓時消散大半，氣血與內息漸漸調和平復。"),
                            ("Rest", "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。"),
                            ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，在靜謐祥和的屋內安然入眠，體魄與心神盡皆恢復。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_home_wardrobe",
                        "衣櫥",
                        &["Look", "Use", "Open"],
                        &[
                            ("Look", "依牆而設的樟木雙門防潮衣櫥，四角包著打磨鋥亮的黃銅包邊，散發著防蟲辟穢的淡淡天然樟木清香。"),
                            ("Use", "你拉開衣櫥門扇整理懸掛的外袍與換洗衣物，取出一套乾爽整潔的衣衫換上，神清氣爽。"),
                            ("Open", "你輕輕拉開樟木衣櫥銅環，櫥格分層井然，整齊疊放著防寒衣物與軟絨棉被。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_home_water_jar",
                        "儲水陶缸",
                        &["Look", "Use", "Wash"],
                        &[
                            ("Look", "厚實堅固的粗陶儲水缸，缸口蓋著洗刷乾淨的杉木圓蓋，缸內注滿清晨自公井打來的甘冽活水，水質澄澈清涼。"),
                            ("Use", "你揭開儲水缸的杉木蓋，以長柄竹杓舀起一杓清涼甘冽的井水飲用，沁人心脾。"),
                            ("Wash", "你自陶缸中舀起一盆清澈涼水洗去雙手與面龐上的微塵，清爽舒適。"),
                        ],
                    ),
                ]
            } else if is_craft_workshop {
                vec![
                    make_peninsula_object(
                        "wt_obj_craft_workbench",
                        "百工木作檯",
                        &["Look", "Use", "Craft"],
                        &[
                            ("Look", "厚重結實的硬木作檯，檯面上整齊排列著各式鑿子、刨刀、木鋸與標尺，透著工匠嚴謹耐心的手藝氣息。"),
                            ("Use", "你俯身整理工作檯上的器皿與工具，檢視工件榫卯嵌合之處，手感精確妥帖。"),
                            ("Craft", "你在木作檯前挑選合適的料件與工具細心修琢，打磨邊角，體會巧手工藝的專注樂趣。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_obj_drying_nets_trough",
                        "晾網木架",
                        &["Look", "Inspect"],
                        &[
                            ("Look", "工坊旁架設著數丈長的結實杉木架，上面晾掛著浸過防腐熟桐油的深色麻網與繩索，通風透氣。"),
                            ("Inspect", "你走近端詳木架上晾曬的網繩，纖維緊實柔韌，散發著天然桐油與草木清香。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_obj_storage_crate",
                        "儲物貨箱",
                        &["Look", "Open", "Check"],
                        &[
                            ("Look", "以鐵箍加固的厚實樟木儲物貨箱，表面刷有防雨瀝青漆，分門別類存放著工坊備料與物產。"),
                            ("Open", "你掀起貨箱厚重頂蓋檢視內存物資，各類原料碼放整齊有序，乾燥避潮。"),
                            ("Check", "你核對貨箱外的貨牌標籤，物料進出登記清楚明白。"),
                        ],
                    ),
                ]
            } else if is_water_steps {
                vec![
                    make_peninsula_object(
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
                    ),
                    make_peninsula_object(
                        "wt_obj_willow_stone_railing",
                        "垂柳石欄",
                        &["Look", "Rest"],
                        &[
                            ("Look", "街側水渠旁立著雕刻簡樸的平整青石護欄，岸邊垂柳依依，枝條拂過平滑的石欄頂部。"),
                            ("Rest", "你倚靠在微涼的石欄旁歇息片刻，柳風拂面，水波粼粼。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_street_lamp_signpost",
                        "貝母街燈",
                        &["Look", "Read", "Operate"],
                        &[
                            ("Look", "石階口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                            ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                            ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                        ],
                    ),
                ]
            } else if is_bridge {
                vec![
                    make_peninsula_object(
                        "wt_obj_willow_stone_railing",
                        "垂柳石欄",
                        &["Look", "Rest"],
                        &[
                            ("Look", "石橋兩側立著雕刻簡樸的平整青石護欄，橋畔垂柳依依，枝條隨風輕拂石欄。"),
                            ("Rest", "你倚靠在微涼的橋石欄旁歇息片刻，看著橋下清波徐徐流淌，水波粼粼。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_obj_flood_gauge_stele",
                        "防汛水尺碑",
                        &["Look", "Check", "Read"],
                        &[
                            ("Look", "石橋墩旁嵌立著一塊刻劃分明水位刻度的花崗岩水尺碑，用以常年監測三水匯流處的水位漲落。"),
                            ("Check", "你探身俯看水尺刻度，當前水位平穩清澈，標尺石面上的警戒紅線清晰可辨。"),
                            ("Read", "你細讀水尺碑旁附刻的防汛條例，載明了歷年最高水位與各防洪閘啟閉規程。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_street_lamp_signpost",
                        "貝母街燈",
                        &["Look", "Read", "Operate"],
                        &[
                            ("Look", "橋頭立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                            ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                            ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板橋面。"),
                        ],
                    ),
                ]
            } else if is_garden_lawn {
                vec![
                    make_peninsula_object(
                        "wt_obj_willow_stone_railing",
                        "垂柳石欄",
                        &["Look", "Rest"],
                        &[
                            ("Look", "苑旁水渠立著雕刻簡樸的平整青石護欄，岸邊垂柳依依，枝條拂過平滑的石欄頂部。"),
                            ("Rest", "你倚靠在微涼的石欄旁歇息片刻，柳風拂面，水波粼粼。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_street_stone_bench",
                        "青石歇腳凳",
                        &["Look", "Rest", "Use"],
                        &[
                            ("Look", "平坪草地旁設有一列平整打磨的青石條凳，石面平滑微涼，供漫步遊人安坐小憩。"),
                            ("Rest", "你在青石長凳上舒然落座歇息，迎著江風呼吸帶著草木清香的空氣，心曠神怡。"),
                            ("Use", "你拂去石凳上的落葉整肅衣襟，靜坐觀看四周水岸生機，悠閒自在。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_street_lamp_signpost",
                        "貝母街燈",
                        &["Look", "Read", "Operate"],
                        &[
                            ("Look", "草坪路口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                            ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                            ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了四周草徑。"),
                        ],
                    ),
                ]
            } else {
                vec![
                    make_peninsula_object(
                        "wt_obj_wharf_bollard",
                        "繫船石柱",
                        &["Look", "Use", "Check"],
                        &[
                            ("Look", "河岸道旁深固著打磨圓潤的厚重花崗岩繫船石柱，柱首雕有簡樸水紋，常年被粗麻纜繩磨得發亮。"),
                            ("Use", "你伸手拉了拉繫在石柱上的粗麻纜繩，纜繩繃緊結實，泊靠的排筏隨波微微晃動。"),
                            ("Check", "你拍了拍冰冷堅實的花崗岩石柱，柱身深植地底，穩如磐石。"),
                        ],
                    ),
                    make_peninsula_object(
                        "wt_street_lamp_signpost",
                        "貝母街燈",
                        &["Look", "Read", "Operate"],
                        &[
                            ("Look", "道旁立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                            ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                            ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                        ],
                    ),
                ]
            }
        }
    } else if room_name.starts_with("文曜") && (room_name.contains("分家巷") || room_name.contains("百工")) {
        if room_name.contains("百工內環") {
            vec![
                make_peninsula_object(
                    "wt_obj_fire_water_tank",
                    "引水消防槽",
                    &["Look", "Check"],
                    &[
                        ("Look", "百工街口設有一座以青石大板拼砌的蓄水槽，連通水渠暗涵保持常滿，備有取水長柄銅斗。"),
                        ("Check", "你俯看水槽內部，槽水清涼充盈，槽緣擱置的汲水桶擺放井然。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_rest_wood_pavilion",
                    "歇腳木亭",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街心設有四柱尖頂的厚重雲杉涼亭，亭內置有環形靠背木椅，專供匠人勞作間隙歇涼。"),
                        ("Rest", "你在木亭長椅上落座歇息，四面穿堂風吹散周身悶熱，頓感舒暢。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else if room_name.contains("百工外環") {
            vec![
                make_peninsula_object(
                    "wt_obj_stone_anvil_platform",
                    "石料砧檯",
                    &["Look", "Operate"],
                    &[
                        ("Look", "外環石道邊置有一座沉重平坦的玄武岩砧檯，檯面留有鐵鑿與石錘敲擊的斑駁凹痕。"),
                        ("Operate", "你伸手撫過粗礪沉重的石砧檯面，堅實穩固，足以承受千鈞重錘。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_raw_log_stack",
                    "原木排垛",
                    &["Look", "Check"],
                    &[
                        ("Look", "道路空坪上整齊滾排著以鐵索鎖固的原木大垛，根根直徑及抱，木皮粗厚乾燥。"),
                        ("Check", "你巡視木垛兩端的固定楔木，鐵索吃緊牢固，堆置安全平穩。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_street_grindstone_trough",
                    "磨刀青石槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "工坊門口砌著一堵長條青石水槽，槽緣擱著幾塊粗細不一的天然油磨石與金剛切石泥。"),
                        ("Use", "你舀了勺水淋在油磨石上，順手將隨身小刃貼著石面輕輕推磨幾下，鋒口立即恢復清亮。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_obj_hardwood_square_timber",
                    "硬木方料垛",
                    &["Look", "Check"],
                    &[
                        ("Look", "工坊門側整齊架起離地的硬木方料垛，墊木通風，木料棱角分明，散發濃郁樟松氣味。"),
                        ("Check", "你伸手敲了敲方料端面，木質乾燥緊密，全無蟲蛀開裂。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_street_lamp_signpost",
                    "貝母街燈",
                    &["Look", "Read", "Operate"],
                    &[
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                    ],
                ),
            ]
        }
    } else if room_name.contains("關前") || room_name.contains("過磅") {
        vec![
            make_peninsula_object(
                "wt_street_platform_scale",
                "地秤",
                &["Look", "Operate"],
                &[
                    ("Look", "地面平嵌著一方厚重生鐵地秤台，側邊立著刻度精細的黃銅秤桿與大號滑動砝碼。"),
                    ("Operate", "你踏上地秤推動黃銅砝碼校驗重量，機括平順堅實，精準測算出車隊貨箱的載重斤兩。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "關前青石路旁立著一尊高大花崗岩石碑，鐫刻著通往長風草甸、老堰頭與大圳頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "驛道旁橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸與大車飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
        ]
    } else if room_name.contains("貨棧") {
        vec![
            make_peninsula_object(
                "wt_obj_steelyard_cart",
                "推車重木秤",
                &["Look", "Operate"],
                &[
                    ("Look", "高架庫棚下停放著兩部加重雙輪推車，車樑上架著帶有青銅滑動大舵砣的重型木桿秤。"),
                    ("Operate", "你推動舵砣校準木秤零點，沉實木樑起落平穩，刻度分毫不差。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "貨場一側整齊碼放著以油布覆蓋的貨垛，旁側堆著通直挺拔的毛竹與杉木圓料。"),
                    ("Check", "你細細檢查油布覆蓋的貨垛底部，防潮石墊高整齊，透氣乾燥。"),
                    ("Tap", "你輕叩粗壯的毛竹竿身，發出清脆結實的篤篤回音，質地堅韌。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬前街") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Tie", "Water", "FeedDraftBeast"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Tie", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ("Water", "你自石槽打起一桶清水倒進飲水槽，駝獸歡快地甩動鬃毛低頭暢飲。"),
                    ("FeedDraftBeast", "你抱來新鮮苜蓿草料與甘冽山泉水倒入槽中，替遠行歇腳的健壯駝獸刷拭背毛、添草飲水。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "街邊橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬街西口") {
        vec![
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "路旁立著一尊高大花崗岩石碑，鐫刻著通往長風草甸與老堰頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "駝鈴排架",
                &["Look", "Listen"],
                &[
                    ("Look", "棚架上懸掛著幾十只大小不一的熟銅駝鈴與原牛皮製鈴帶，在風中輕輕碰撞。"),
                    ("Listen", "微風掠過排架，熟銅鈴鐺激盪出清越悠遠的叮咚金鳴，宛若大漠行商長歌。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬街西段") || room_name.contains("草料") {
        vec![
            make_peninsula_object(
                "wt_obj_wagon_jack_wheel",
                "千斤重木頂",
                &["Look", "Check"],
                &[
                    ("Look", "地坪上整齊堆放著維修重輪大車專用的加厚千斤頂木楔與兩組備用硬木輪圈。"),
                    ("Check", "你查驗木頂機括與硬木輪圈的鐵皮緊固螺栓，咬合嚴密無隙。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "車馬街旁橫著深青石水槽，槽中滿注引自山澗的活水，供歇腳駝獸飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else if room_name.contains("車馬") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "駝鈴排架",
                &["Look", "Listen"],
                &[
                    ("Look", "棚架上懸掛著幾十只大小不一的熟銅駝鈴與原牛皮製鈴帶，在風中輕輕碰撞。"),
                    ("Listen", "微風掠過排架，熟銅鈴鐺激盪出清越悠遠的叮咚金鳴，宛若大漠行商長歌。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Tie", "Water"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Tie", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                    ("Water", "你自石槽打起一桶清水倒進飲水槽，駝獸歡快地甩動鬃毛低頭暢飲。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_lamp_signpost",
                "貝母街燈",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                    ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                    ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
                ],
            ),
        ]
    } else {
        return None;
    };
    Some(objs)
}

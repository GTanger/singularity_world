#[allow(unused_variables)]
fn peninsula_objects_chunk_1(room_name: &str) -> Option<Vec<RoomObject>> {
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

    let objs = if room_name.contains("會計")
        || room_name.contains("算學")
        || room_name.contains("算居")
        || room_name.contains("司書")
        || room_name.contains("商算")
    {
        vec![
            make_peninsula_object(
                "wt_abacus_ebony_desk",
                "烏木計數板",
                &["Look", "Operate", "Use"],
                &[
                    ("Look", "烏木公案上端正擺放著便攜計數板，滑鈕圓潤油亮，旁側立著硃砂墨碟與儲墨硬筆。"),
                    ("Operate", "你伸手撥動計數滑鈕，上下翻飛發出清脆沉著的扣合聲。"),
                    ("Use", "你提筆在草紙上演算一筆出海貨單，計數板撥轉分毫不差，帳目井然。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_abacus_counters",
                "會計帳冊夾",
                &["Look", "Calculate", "Read"],
                &[
                    ("Look", "條案上擺著硬木封面的試算表夾，內夾整齊的複式帳頁，旁側放著烏木便攜計數板與利率換算表。"),
                    ("Calculate", "你撥動計數板滑鈕逐行覆核試算表，借貸加減縱橫有序，運算如行雲流水，神清氣爽。"),
                    ("Read", "你翻開案頭的會計實務圖表，上面詳載著大宗物產折算與利息攤提之法。"),
                ],
            ),
            make_peninsula_object(
                "wt_abacus_fair_price_ledger",
                "行商價目冊",
                &["Look", "Read"],
                &[
                    ("Look", "厚實的松木帳案上攤開著《各地行商貨價公道帳冊》，墨跡清晰，記載著半島各坊各埠即時市價。"),
                    ("Read", "你細讀帳冊上的公道行情：糧穀、生鐵、海鹽與布疋之平準價目皆有明文，童叟無欺。"),
                ],
            ),
        ]
    } else if room_name.contains("書齋")
        || room_name.contains("小齋")
        || room_name.contains("別室")
        || room_name.contains("山房")
        || room_name.contains("星曆")
        || room_name.contains("讀書")
        || room_name.contains("篆刻")
        || room_name.contains("拓碑")
        || room_name.contains("雕版")
        || room_name.contains("校勘")
        || room_name.contains("墨香")
    {
        vec![
            make_peninsula_object(
                "wt_study_desk_inkstone",
                "烏木書案",
                &["Look", "Use", "Operate", "Read", "CollateInscriptions"],
                &[
                    ("Look", "黑沼沉水烏木雕琢的寬厚書案散發著幽靜木香，案上端硯墨色溫潤如玉，水注蓄著山泉，紫毫畫筆與狼毫楷筆整齊懸於紫檀筆架。"),
                    ("Use", "你取墨錠在端硯清泉中徐徐研磨，墨香四溢，提筆蘸墨在宣紙上信手書寫，筆意沉著，心神豁然澄澈。"),
                    ("Operate", "你整理案几上的鎮紙與宣紙摺頁，端整筆洗，室內窗明几淨，文氣盎然。"),
                    ("Read", "你展讀案几上攤開的手抄古卷，筆跡端勁，字句精微，深得先賢格物修齊之旨。"),
                    ("CollateInscriptions", "你展平古碑墨拓與竹綿宣紙，手持狼毫小楷逐字比勘碑文字口，訂正訛誤。"),
                ],
            ),
            make_peninsula_object(
                "wt_study_bamboo_bookshelf",
                "線裝古籍架",
                &["Look", "Use", "Read", "Operate", "SunBooks", "CollateInscriptions"],
                &[
                    ("Look", "靠牆的高大樟木架上排滿了竹綿紙線裝古籍與輿地總圖，底層整齊碼放著幾隻精編韌竹書簍，分納校勘稿卷。"),
                    ("Use", "你自架上抽出一冊線裝古籍翻閱，紙質柔韌吸墨，墨香沁人心脾，細讀之下頗有領悟。"),
                    ("Read", "你取下一卷半島風物誌稿細細研讀，字裡行間詳載水土人文與山川形勝，令人眼界大開。"),
                    ("Operate", "你將翻閱過的文稿重新按部首分類歸入竹編書簍，架欞齊整，井然有序。"),
                    ("SunBooks", "你自樟木架上捧出線裝古籍攤於通風日影下曝書防蠹，夾入芸香乾葉。"),
                    ("CollateInscriptions", "你自書簍取出金石碑帖拓本與古籍善本對照校勘，分門別類歸檔。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "案角立著造型典雅的旋蓋夜光貝母調光檯燈，貝母薄片拼嵌的燈罩泛著溫潤珠光，烏木旋蓋可細調光圈大小。"),
                    ("Use", "你旋動烏木燈蓋調節光暈，柔和清澈的珠白光芒映照書案，滿室皆是清幽安寧之氣。"),
                    ("Operate", "你輕旋貝母燈罩，光暈隨之微調收斂，光斑柔和均勻，極便於夜讀抄寫。"),
                ],
            ),
        ]
    } else if room_name.contains("客房") || room_name.contains("內室") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_home",
                "床榻",
                &["Look", "Use", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。"),
                    ("Use", "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞頓時消散大半，氣血與內息漸漸調和平復。"),
                    ("Rest", "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母檯燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_hotspring_tub",
                "杉木沐浴桶",
                &["Look", "Use", "Bath", "SoakHotSpring"],
                &[
                    ("Look", "精工打磨的深腹杉木沐浴桶散發著天然木香，桶畔連通暗渠熱泉注水口，備有乾淨的棉長絨擦巾。"),
                    ("Use", "你扳開注水銅栓，熱氣騰騰的溫泉水汩汩注入杉木桶，滿室蒸騰氤氳暖霧。"),
                    ("Bath", "你浸入溫熱的杉木泉水浴桶中，洗去一身塵泥與筋骨疲憊，周身毛孔舒張，倍感舒泰。"),
                    ("SoakHotSpring", "你浸入溫氏祛寒草木熱湯池中，溫潤泉水滲透四肢百骸，寒濕與疲勞盡數消散。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "几案上立著旋蓋夜光貝母調光檯燈，貝母薄片拼嵌的燈罩泛著溫潤珠光，烏木旋蓋可調節明暗。"),
                    ("Use", "你旋亮夜光貝母檯燈，珠白光暈柔和灑滿几案與床榻，室內一片溫馨安寧。"),
                    ("Operate", "你旋轉烏木外罩調節進光弧度，光影隨心明暗，極具生活雅趣。"),
                ],
            ),
        ]
    } else if is_wenyao_residential_or_study(room_name) {
        let h = room_name.bytes().fold(5381usize, |h, b| h.wrapping_mul(33).wrapping_add(b as usize));
        match h % 4 {
            0 => vec![
                make_peninsula_object(
                    "wt_home_dining_tea_table",
                    "樟木八仙桌",
                    &["Look", "Use", "Rest"],
                    &[
                        ("Look", "廳堂正中端正擺放著一張打磨光滑的厚實樟木八仙桌，桌上設有青瓷茶具與一壺新沏好的山風草茶。"),
                        ("Use", "你在八仙桌旁長凳落座，端起青瓷茶杯啜飲一口草茶，茶香清冽回甘，頓解喉燥。"),
                        ("Rest", "你靠在八仙桌旁歇息片刻，廳內窗欞明淨，空氣中飄散著幽淡樟木清香。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_shell_lamp",
                    "檯燈",
                    &["Look", "Use", "Operate"],
                    &[
                        ("Look", "几案上立著造型典雅的旋蓋夜光貝母調光檯燈，貝母薄片拼嵌的燈罩泛著溫潤珠光。"),
                        ("Use", "你旋動烏木燈蓋調節光暈，柔和清澈的珠白光芒映照廳堂，滿室安寧。"),
                        ("Operate", "你輕旋貝母燈罩調節光影明暗，溫潤澄澈。"),
                    ],
                ),
            ],
            1 => vec![
                make_peninsula_object(
                    "wt_reg_wenyao_home",
                    "床榻",
                    &["Look", "Use", "Rest", "Sleep", "Operate"],
                    &[
                        ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。"),
                        ("Use", "你伸手撫平松香木大床上的厚實棉被，被面乾燥蓬鬆，透著令人安心的松木清香。"),
                        ("Rest", "你坐在床沿卸下隨身行囊，舒展筋骨稍事休憩，滿室寧靜溫暖。"),
                        ("Sleep", "你合衣躺上溫暖的松香木床合眼小憩，羽絨暖被鬆軟溫暖，很快消解了跋涉疲頓。"),
                        ("Operate", "你旋轉榻旁貝母小檯燈的旋蓋，微調夜光角度，滿室光暈澄澈柔和。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_brass_tap_sink",
                    "水栓",
                    &["Look", "Use", "Bath"],
                    &[
                        ("Look", "打磨光滑的黃銅水栓連接著活水管道，下方是耐火磚砌成的溫石蓄熱水槽，槽邊備有山茶松脂皂與棉長絨擦巾。"),
                        ("Use", "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，草木皂香格外清新。"),
                        ("Bath", "你引入溫石槽加熱的熱水洗手淨面，熱氣騰騰驅散了旅途風塵，倍感舒泰。"),
                    ],
                ),
            ],
            2 => vec![
                make_peninsula_object(
                    "wt_home_ebony_tea_table",
                    "烏木茶盤几",
                    &["Look", "Use", "Rest"],
                    &[
                        ("Look", "窗下置著一張色澤深沉溫潤的沉水烏木茶几，几上平鋪著竹絲編織茶墊與一把細泥紫砂小茶壺。"),
                        ("Use", "你提壺往白瓷茶盅注滿清潤山泉熱茶，細品微苦甘涼的茶湯，神思悠然。"),
                        ("Rest", "你倚在茶几旁的軟墊靠枕上歇息片刻，窗外微風拂竹，室內清雅靜謐。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_pine_wardrobe",
                    "衣櫥",
                    &["Look", "Use", "Check"],
                    &[
                        ("Look", "靠牆立著高大的松香木對開門衣櫃，銅合頁打磨細緻，櫃門微敞，飄出乾燥的松脂與樟腦驅蛀清香。"),
                        ("Use", "你拉開衣櫃厚實的木門，將厚重的外袍與隨身行囊整齊掛在木架上，收納妥貼。"),
                        ("Check", "你檢視衣櫃內整齊分格的儲物隔板，各層鋪有防潮麻布，收納井井有條。"),
                    ],
                ),
            ],
            _ => vec![
                make_peninsula_object(
                    "wt_home_bamboo_rocker",
                    "竹編搖椅",
                    &["Look", "Rest", "Sit"],
                    &[
                        ("Look", "向陽窗欞旁安放著一張柔韌寬敞的青竹編織搖椅，弧形椅底打磨得極為光滑，墊著厚實的羊毛坐褥。"),
                        ("Rest", "你仰靠在竹編搖椅上隨意晃動，椅身輕柔搖曳，微風自窗欞拂入，愜意怡然。"),
                        ("Sit", "你在羊毛坐褥上穩穩落座，竹篾貼合背脊自然受力，疲勞隨之一空。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_home_jade_icebox",
                    "冰鑑櫃",
                    &["Look", "Use", "Check"],
                    &[
                        ("Look", "角落擺著一座雙層硬木箱櫃，內壁緊密鑲嵌著半嶺寒水玉板，櫃內寒氣繚繞，儲存著新鮮果乾與冰涼井泉水囊。"),
                        ("Use", "你掀開厚實櫃蓋取出一隻冰鎮甘蔗汁小竹筒，清涼甘甜沁入心脾，暑氣立消。"),
                        ("Check", "你查看櫃內寒水玉的凝霜狀況，玉板散發陣陣自然涼意，保鮮極佳。"),
                    ],
                ),
            ],
        }
    } else if room_name.contains("內門坊") {
        vec![
            make_peninsula_object(
                "wt_gate_stone_archway_board",
                "牌坊路引碑",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "巍峨挺拔的青石牌坊橫跨街衢，飛簷挑角雕飾精美，石柱旁立著黃銅包邊的路引告示牌，標注內城四坊與外環諸坊的界域與更次。"),
                    ("Read", "你仰頭研讀告示牌上的通行條例與各坊指引：內連文曜核心樞紐，外接四向繁華市邑與外環關防，車馬行人絡繹不絕。"),
                    ("Operate", "你拂去石牌下端刻石的塵土，石質路標凹刻的方位符號歷歷在目，指引清晰。"),
                ],
            ),
            make_peninsula_object(
                "wt_gate_water_culvert_drum",
                "歇腳石鼓",
                &["Look", "Use", "Rest"],
                &[
                    ("Look", "牌坊兩側鋪設著引自三水川的平整青石導水渠，渠水潺潺清冽；石階兩側立有厚實的雕花歇腳石鼓，供過往挑夫卸擔暫歇。"),
                    ("Use", "你走近導水石渠汲取一捧清涼泉水洗面，清冽的水流滌盡了行走街巷帶來的燥熱。"),
                    ("Rest", "你在石鼓旁卸下行囊坐下稍歇，石面光滑微涼，看著內外城交匯處的熙攘車馬，別有一番市井意趣。"),
                ],
            ),
            make_peninsula_object(
                "wt_gate_patrol_gong_stone",
                "巡更銅鑼",
                &["Look", "Ring", "Read"],
                &[
                    ("Look", "石坊側壁懸掛著巡夜守衛報時防險的沉厚黃銅響鑼，下方嵌入一塊歷經歲月摩挲的青石指路刻石。"),
                    ("Ring", "你輕叩懸於坊壁的巡更銅鑼，鑼聲沉渾悠長，聲震四街，驚起幾隻棲息在牌坊飛簷上的飛鳥。"),
                    ("Read", "你撫摸指路刻石上的楷書方位：向內直通鐘樓樞紐，向外直達城門外街，條理井然。"),
                ],
            ),
        ]
    } else if room_name.contains("貨船大埠") || room_name.contains("大埠") {
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
                "wt_obj_wharf_crane",
                "滑輪吊架",
                &["Look", "Operate"],
                &[
                    ("Look", "駁岸邊架著堅固的懸臂式黃銅滑輪吊架，厚木跳板斜搭在貨船甲板與碼頭石階之間。"),
                    ("Operate", "你拉動吊架垂下的麻繩試驗滑輪阻力，黃銅輪軸運轉平滑，省力穩當。"),
                ],
            ),
        ]
    } else if room_name.contains("鮮魚拍賣") || room_name.contains("魚市") {
        vec![
            make_peninsula_object(
                "wt_obj_fish_auction_trough",
                "鮮魚冰槽",
                &["Look", "Weigh", "Inspect"],
                &[
                    ("Look", "平整石板地上整齊排著引寒水玉蓄冷的青石冰槽，槽內盛滿新出水銀鱗閃爍的大海魚，旁邊立著高大的黃銅懸臂秤。"),
                    ("Weigh", "你撥動懸臂秤上的黃銅大砣校驗斤兩，秤桿平穩起落，顯示魚獲分量沉實。"),
                    ("Inspect", "你湊近冰槽細看，銀鱗大魚鰓色鮮紅、體表光澤，滿是江口活水的清鮮之氣。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_fish_baskets",
                "透水竹魚簍",
                &["Look", "Use"],
                &[
                    ("Look", "冰槽旁堆疊著無數浸泡得深褐發亮的透水大竹簍，案板上整齊排著剖魚刮鱗的厚背弧刃短刀。"),
                    ("Use", "你拿起刮鱗刀在粗石上隨手一磨，刀口鋒利平整，處理魚獲俐落乾脆。"),
                ],
            ),
        ]
    } else if room_name.contains("桅帆") || room_name.contains("造船") || room_name.contains("修船") {
        vec![
            make_peninsula_object(
                "wt_obj_ship_ribs_pitch",
                "造船龍骨架",
                &["Look", "Smell", "Caulking"],
                &[
                    ("Look", "作坊開敞的大棚下支起十多丈長的弧形硬木船肋架，旁側鐵皮火爐上滾沸著填補水密艙縫隙的生桐油松香瀝青鍋。"),
                    ("Smell", "濃烈的松脂焦香與熟桐油氣息撲鼻而來，是江海船匠最熟悉的熱烈味道。"),
                    ("Caulking", "你拿起一把寬頭嵌縫鐵刀在木板試槽刮磨，手感紮實，能使船體水密嚴絲合縫。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sail_canvas_bench",
                "帆布裁案",
                &["Look", "Touch"],
                &[
                    ("Look", "長條案板上鋪展著厚實堅韌的雙層亞麻帆布，竹筒裡插著數十根浸潤防腐蠟的粗骨針與麻繩卷。"),
                    ("Touch", "你按壓帆布質地，緊密厚實，經得起外海狂風暴雨的強力撕扯。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("漁網") || room_name.contains("曬網") || room_name.contains("繩索工坊") || room_name.contains("河鮮乾貨") || room_name.contains("鹽漬")) {
        vec![
            make_peninsula_object(
                "wt_obj_drying_nets_trough",
                "苧麻曬網架",
                &["Look", "Inspect", "Open"],
                &[
                    ("Look", "露天院落中橫架著數十丈長的曬網木架，晾曬著浸染柿漆防腐的深褐色苧麻大網，廊下整齊排列著加蓋重石的鹽漬大陶缸。"),
                    ("Inspect", "你摸了摸懸掛的苧麻漁網，網結緊湊堅韌，透著淡淡的柿漆甘澀香。"),
                    ("Open", "你掀開其中一只大醃缸的竹木蓋，滿缸粗鹽浸漬的青魚散發著鹹香撲鼻的發酵海味。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_rope_twist_wheel",
                "手搖絞繩輪",
                &["Look", "Operate"],
                &[
                    ("Look", "空地上立著三聯手搖木製絞繩輪，麻絲自輪齒穿出絞合為粗壯縴繩，旁側立有刻度拉力測試石柱。"),
                    ("Operate", "你搖動絞繩輪木柄，三股浸油麻線均勻絞合，緊實堅固。"),
                ],
            ),
        ]
    } else if room_name.contains("竹木場") || (room_name.contains("河運") && room_name.contains("倉")) {
        vec![
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "高大通風的倉棚下整齊碼放著以油布覆蓋的防潮糧穀包，場地另一側堆滿通直挺拔的三年生毛竹與直徑數尺的巨杉圓木。"),
                    ("Check", "你翻檢糧垛底層的防潮樟木墊板，乾燥清爽，通風良好。"),
                    ("Tap", "你屈指叩擊粗大毛竹竿，竹管清脆堅挺，正是紮造載重排筏的上等良材。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_steelyard_cart",
                "槓桿大磅秤",
                &["Look", "Operate"],
                &[
                    ("Look", "平整地面停著帶鐵輪的大型移動式槓桿磅秤，旁側備有引導大圓木滾動的斜坡硬木墊條。"),
                    ("Operate", "你推動磅秤上的配重鐵砣，滑動順暢，計重精確。"),
                ],
            ),
        ]
    } else if room_name.contains("水手茶") || room_name.contains("茶棚") || room_name.contains("茶飯館") {
        vec![
            make_peninsula_object(
                "wt_obj_river_tea_kettle",
                "紫銅大茶壺",
                &["Look", "Taste", "WarmHands"],
                &[
                    ("Look", "臨江茶座的炭火紅爐上煨著一把滾水翻騰的大號紫銅茶壺，粗木桌上整齊倒扣著青花粗陶大海碗。"),
                    ("Taste", "你倒了一大碗熱氣蒸騰的濃釅磚茶飲盡，微苦回甘，頓時驅散江風濕寒，通體舒泰。"),
                    ("WarmHands", "你將雙手懸在紫銅大茶壺上方烘烤，熱氣繚繞，暖透指尖。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_river_view_bench",
                "臨江木長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "粗松木長凳依著駁岸欄杆擺放，臨窗正對滔滔江流，江面上白帆點點。"),
                    ("Rest", "你在長凳上坐下倚欄遠眺，聽著江濤拍岸與水手號子，心神開闊。"),
                ],
            ),
        ]
    } else if room_name.contains("清淤") || room_name.contains("巡河") {
        vec![
            make_peninsula_object(
                "wt_obj_patrol_skiff_gear",
                "清淤鐵斗",
                &["Look", "Check"],
                &[
                    ("Look", "哨所石碼頭旁繫泊著兩艘吃水極淺的快船，駁岸邊架著數柄清理河道暗沙淤泥的長柄鋼齒鐵斗與救生浮木。"),
                    ("Check", "你檢查巡河快船上的桐油木槳與水路浮標，裝備齊全，隨時可出航巡視航道。"),
                ],
            ),
        ]
    } else if room_name.contains("宣紙") || room_name.contains("曬紙") || room_name.contains("造紙") {
        vec![
            make_peninsula_object(
                "wt_obj_paper_making_screen",
                "抄紙竹簾槽",
                &["Look", "Operate", "Touch"],
                &[
                    ("Look", "清泉注入的長條抄紙石槽中浮著如雪漿料，抄紙匠手持細密青竹簾輕輕一晃，便蕩出一張厚薄均勻的紙膜；旁側是燒炭增溫的平整烘紙夾石壁。"),
                    ("Operate", "你雙手托起青竹抄紙簾在漿槽中勻速一蕩，水流順簾縫漏下，簾面留下一層纖薄純淨的青檀纖維。"),
                    ("Touch", "你伸手撫摸烘紙石壁上新揭下的宣紙，紙質綿韌溫熱，如玉生香。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_pulp_vat",
                "木料浸泡池",
                &["Look", "Inspect"],
                &[
                    ("Look", "石砌方池中浸泡著經草木灰水發酵純化後的青檀樹皮，旁邊支著以巨木槓桿加壓的脫水木榨。"),
                    ("Inspect", "你探看池中纖維，色白如玉、柔韌細長，正是成就千年不腐宣紙之根基。"),
                ],
            ),
        ]
    } else if room_name.contains("徽墨") || room_name.contains("精墨") {
        vec![
            make_peninsula_object(
                "wt_obj_ink_soot_mortar",
                "搗墨鐵臼",
                &["Look", "Smell", "Pound"],
                &[
                    ("Look", "暗室中整齊排列著收集上等古松油煙的陶盞，石台上置有沉重的大鐵臼與包革重錘，工匠正千錘百煉將皮膠與松煙捶打成墨坯。"),
                    ("Smell", "空氣中瀰漫著古松燃煙的特有幽香，混雜著麝香與冰片的提神清冽氣息。"),
                    ("Pound", "你握緊搗墨木杵在鐵臼中奮力搗擊數次，墨坯筋道柔韌，烏黑生光。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_ink_mould_drying",
                "雕花墨模架",
                &["Look", "Inspect"],
                &[
                    ("Look", "木架上陳列著精雕山水雲龍紋的硬梨木墨模，旁側通風架上整齊排著以厚灰包裹慢速陰乾的墨錠成品。"),
                    ("Inspect", "你拿起一方成型墨錠端詳，表面紫光油亮、金粉填字，叩之有金石清音。"),
                ],
            ),
        ]
    } else if room_name.contains("活字") || room_name.contains("印書") {
        vec![
            make_peninsula_object(
                "wt_obj_movable_type_case",
                "活字字盤架",
                &["Look", "PickType", "RollInk"],
                &[
                    ("Look", "高聳的斜面字架上排列著按音韻偏旁分門別類的數萬枚錫鉛與硬木活字，工作台上放著蘸滿油亮墨汁的雙手羊毛滾筒與銅夾版框。"),
                    ("PickType", "你自字盤中精準拈出一枚工整反字「道」，鉛錫合金字面光滑如鏡，棱角分明。"),
                    ("RollInk", "你推動羊毛滾筒在字版上均勻滾塗油墨，黑潤發亮，字跡頓時清晰畢現。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_printing_press",
                "螺旋壓印機",
                &["Look", "Operate"],
                &[
                    ("Look", "鑄鐵底座的螺旋式手扳壓印機立在工坊正中，側邊晾架上夾著新印就的《文曜格物錄》書頁。"),
                    ("Operate", "你扳動螺旋下壓手柄，重板均勻受力壓覆紙面，揭開時字跡黑亮深透，濃淡勻停。"),
                ],
            ),
        ]
    } else if room_name.contains("裝幀") || room_name.contains("線裝") {
        vec![
            make_peninsula_object(
                "wt_obj_book_binding_press",
                "線裝壓書機",
                &["Look", "Operate", "Stitch"],
                &[
                    ("Look", "寬大的實木工作台上固定著旋柄木螺壓平機，旁邊整齊排著裁紙大鍘刀、打孔骨錐、蜂蠟塊與彩色熟絲線。"),
                    ("Operate", "你轉動木螺壓機旋柄，木板緊緊壓實新裝訂的線裝書脊，整飭端莊。"),
                    ("Stitch", "你拿起拋光骨針穿引浸蠟絲線，在紙頁預留的針眼間靈巧穿行，鎖線均勻平整。"),
                ],
            ),
        ]
    } else if room_name.contains("輿圖") || room_name.contains("格物") {
        vec![
            make_peninsula_object(
                "wt_obj_cartography_table",
                "測繪大畫案",
                &["Look", "Measure", "Inspect"],
                &[
                    ("Look", "數丈長的平整大畫案上鋪展著白塔半島全境水陸輿圖草稿，案頭擺著黃銅分規、量角器、等高測量照準儀與各色礦物顏料碟。"),
                    ("Measure", "你拿起精緻的黃銅兩腳分規在輿圖上測量文曜至風舶角的里數，刻度精微，毫釐不差。"),
                    ("Inspect", "你俯身仔細察看圖上繪製的山脈地鳴走向與沿海暗礁標注，測繪嚴謹入微。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_surveyor_theodolite",
                "經緯照準儀",
                &["Look", "Operate"],
                &[
                    ("Look", "畫案旁立著黃銅三腳架經緯照準儀，水晶目鏡擦拭得晶瑩剔透，旁立著黑白分格的測高水準木尺。"),
                    ("Operate", "你調整照準儀水平水泡，指針歸零，望遠鏡筒鎖定遠方地平山稜。"),
                ],
            ),
        ]
    } else if room_name.contains("傳習所") || room_name.contains("會計") || (room_name.contains("算學") && !room_name.contains("院")) {
        vec![
            make_peninsula_object(
                "wt_obj_abacus_counters",
                "會計帳冊夾",
                &["Look", "Calculate", "Read"],
                &[
                    ("Look", "條案上擺著以名貴紅木製作的雙排精密計數板，旁側配有一份工程成本試算表與幾何受力圖譜。"),
                    ("Calculate", "你指尖輕撥計數滑鈕，滑槽發出清脆扣響，運算如行雲流水，神清氣爽。"),
                    ("Read", "你翻開案頭的《工程數學與幾何實務》，上面推演著農水工程與造船載重的幾何受力演算法。"),
                ],
            ),
        ]
    } else if room_name.contains("測風") || room_name.contains("北辰") || room_name.contains("曆法") {
        vec![
            make_peninsula_object(
                "wt_obj_armillary_almanac",
                "定緯銅儀",
                &["Look", "Observe", "Read", "ChartAstrolabe"],
                &[
                    ("Look", "露天石台中央矗立著多層旋轉銅環交錯的北辰定緯儀，基座四周平嵌著刻滿二十四節氣晷影長短的黑曜石測影日晷。"),
                    ("Observe", "你俯身透過銅儀窺管對準北天極星，重環轉動滑潤無聲，指引天行經緯。"),
                    ("Read", "你俯瞰黑曜石規表上的日影刻度，比對當前時刻與太陽方位，推步精微。"),
                    ("ChartAstrolabe", "你鋪開竹綿宣紙與松煙墨拓，循星軌儀子午刻度拓下個人周天命格星盤。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wind_vane_gauge",
                "相風銅烏",
                &["Look", "Check", "ReadWindVane"],
                &[
                    ("Look", "測候台四角立著展翅相風銅烏，在微風中靈動旋轉；台內置有三級滴漏水鐘，刻箭隨水位均勻浮升。"),
                    ("Check", "你端看滴漏刻箭上的水位刻度，水流均勻如線，記時絲毫不差。"),
                    ("ReadWindVane", "你仰觀黃銅風向儀與相風銅烏轉速，比對滴漏刻度，精準推算出半島山海風候與潮汐走向。"),
                ],
            ),
        ]
    } else if room_name.contains("杏林") || (room_name.contains("醫館") && room_name.contains("百草")) {
        vec![
            make_peninsula_object(
                "wt_obj_herb_cabinet_mortar",
                "百眼藥櫃",
                &["Look", "Weigh", "Pound"],
                &[
                    ("Look", "整面高牆由數百個貼著楷書金漆藥名的抽屜藥櫃組成，長條藥案上擺著磨成光面的紫銅搗藥臼與分釐黃銅戥子秤。"),
                    ("Weigh", "你提起小巧的黃銅戥子秤，精確稱量出三錢清涼薄荷與二錢北山寒水玉細粉。"),
                    ("Pound", "你握起紫銅搗藥杵在藥臼中輕搗乾草藥，微苦回甘的藥香瞬間瀰漫開來。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_acupuncture_pulse_pillow",
                "沉香脈枕",
                &["Look", "Use", "ApplyMoxibustion"],
                &[
                    ("Look", "診案上墊著一具包著軟綾的沉香木切脈枕，旁側錦盒內整齊插著數十根極細的精淬金銀針。"),
                    ("Use", "你手腕輕搭在沉香脈枕上，心跳脈搏沉靜平穩，精神為之一爽。"),
                    ("ApplyMoxibustion", "你點燃溫氏陳年艾絨置於溫灸盒上溫熨經絡穴位，艾香透骨，扭挫瘀滯頓時化開。"),
                ],
            ),
        ]
    } else if room_name.contains("採藥棧") || room_name.contains("山林採藥") {
        vec![
            make_peninsula_object(
                "wt_obj_herb_drying_rack",
                "曬藥竹篾架",
                &["Look", "Smell"],
                &[
                    ("Look", "棧房院中支滿了一層層圓形竹篾席，曬著自北嶺險峰採回的山風草、石斛與野山參，牆角倒掛著帶防滑鐵爪的登山採藥草鞋。"),
                    ("Smell", "陽光曬透乾草藥散發出的濃郁藥香與泥土芬芳交織，令人提神醒腦。"),
                ],
            ),
        ]
    } else if room_name.contains("文房") || room_name.contains("文具行") {
        vec![
            make_peninsula_object(
                "wt_obj_brush_inkstone_rack",
                "紫檀筆架",
                &["Look", "TestBrush", "GrindInk"],
                &[
                    ("Look", "紫檀雕花筆架上懸掛著數十支自兼毫、純狼毫至紫毫的大中小楷毛筆，案頭臥著一方呵氣生津的端溪青石老硯。"),
                    ("TestBrush", "你取下一支兼毫長鋒在掌心輕輕按壓試筆，彈性十足，蓄墨豐沛。"),
                    ("GrindInk", "你提起松煙墨錠在青石硯台中滴水緩磨，硯面黑亮如漆，墨韻生光。"),
                ],
            ),
        ]
    } else if room_name.contains("水錘") || room_name.contains("大鍛坊") {
        vec![
            make_peninsula_object(
                "wt_obj_trip_hammer_trough",
                "水力重鐵錘",
                &["Look", "Watch", "Quench"],
                &[
                    ("Look", "外河急流驅動的巨木水輪帶動凸輪軸，數千斤重的生鐵大鍛錘以雷霆萬鈞之勢不斷夯擊砧檯，砧旁鑿有深廣的循環淬火石槽。"),
                    ("Watch", "你凝神注視燒紅的重型鋼軸在水錘轟擊下火星四濺，每一次撞擊都撼動腳底石板。"),
                    ("Quench", "匠人將鍛打完畢的鋼料浸入冷水槽，頓時蒸騰起滾滾白霧與刺啦劇響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_bellows_furnace",
                "木風箱高爐",
                &["Look", "Operate"],
                &[
                    ("Look", "耐火磚堆砌的高溫反射爐旁架著丈餘長的雙推桿活塞木風箱，爐口吞吐著熾烈的金白烈焰。"),
                    ("Operate", "你握住風箱推桿來回一推，風道轟然呼嘯，爐膛烈火猛地拔高數尺。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("大木") {
        vec![
            make_peninsula_object(
                "wt_obj_waterwheel_sawframe",
                "水動排鋸台",
                &["Look", "Smell", "Measure"],
                &[
                    ("Look", "水輪傳動皮帶驅動數排鋒利長鋸上下往復飛速拉動，整根參天巨木被精準剖切成筆直板材，刨木台上積了厚厚一層噴香刨花。"),
                    ("Smell", "新剖切開的百年雲杉木脂香氣撲面而來，清新宜人。"),
                    ("Measure", "你以曲尺檢驗剖切出的大樑木料，斷面平直方正，嚴合營造法度。"),
                ],
            ),
        ]
    } else if room_name.contains("切割") || room_name.contains("石材") {
        vec![
            make_peninsula_object(
                "wt_obj_stone_gang_saw",
                "切石框鋸台",
                &["Look", "Touch"],
                &[
                    ("Look", "粗壯鐵架上懸掛著加重無齒金剛砂框鋸，細水伴著鋼砂在巨石槽中慢慢切割，四周堆滿了平整如削的城防條石與石雕柱礎。"),
                    ("Touch", "你撫摸切割完畢的青石板側面，細密平整，嚴密無縫。"),
                ],
            ),
        ]
    } else if room_name.contains("精煉") || room_name.contains("青銅器皿") {
        vec![
            make_peninsula_object(
                "wt_obj_bronze_crucible",
                "熔銅坩堝爐",
                &["Look", "Inspect"],
                &[
                    ("Look", "鼓風皮囊呼呼作響，高溫爐膛中石墨坩堝內熔化著金紅透亮的銅錫合金液，旁邊地面整齊排列著加固木製翻砂模具。"),
                    ("Inspect", "你檢視翻砂模具中的浮雕紋飾，饕餮雲紋深峻清晰，成型精準。"),
                ],
            ),
        ]
    } else if room_name.contains("木炭窯") || (room_name.contains("炭窯") && !room_name.contains("院")) {
        vec![
            make_peninsula_object(
                "wt_obj_charcoal_kiln",
                "黏土悶炭窯",
                &["Look", "Knock"],
                &[
                    ("Look", "依山構築的半球形黏土炭窯窯門以濕泥封堵，窯頂煙囪正吐著淡淡青煙，棚下整齊碼放著敲擊如鐵金之聲的頂級硬木白炭。"),
                    ("Knock", "你拿起兩條黑亮白炭互敲，聲音清脆悠揚宛若玉石，足見炭質堅密無渣。"),
                ],
            ),
        ]
    } else if room_name.contains("磚瓦窯") || room_name.contains("官窯") || room_name.contains("燒造") {
        vec![
            make_peninsula_object(
                "wt_obj_long_kiln_firebox",
                "龍窯火膛",
                &["Look", "Check"],
                &[
                    ("Look", "依山勢斜臥的數十丈長條龍窯火眼遍布，火膛內火焰翻騰泛白，外側曬場上排滿了陰乾完畢等待入窯的青筒瓦坯與釉陶大缸。"),
                    ("Check", "你查看窯溫觀火孔，火色金白耀眼，正是瓷胎玻化凝固之關鍵火候。"),
                ],
            ),
        ]
    } else if room_name.contains("工具行") || (room_name.contains("農具") && room_name.contains("總")) {
        vec![
            make_peninsula_object(
                "wt_obj_farm_tool_rack",
                "鐵農具木架",
                &["Look", "TestEdge"],
                &[
                    ("Look", "牆上排滿了雙齒鎬、開荒大板鋤、月牙割麥鐮與重型碎土鐵耙，案頭擺著浸在礦物油中的雙面天然油石。"),
                    ("TestEdge", "你伸指輕彈割麥鐮刀背，鋼聲清越悠長，刃口經油石研磨鋒利如剃。"),
                ],
            ),
        ]
    } else if room_name.contains("織造坊") || room_name.contains("大染坊") || (room_name.contains("機杼") && room_name.contains("染")) {
        vec![
            make_peninsula_object(
                "wt_obj_loom_dye_vats",
                "踏板高織機",
                &["Look", "Touch", "DyeIndigoCloth"],
                &[
                    ("Look", "高大的立式踏板織機梭子飛擲，機旁數口齊腰深的大陶缸盛滿天然發酵群青染液，挑染出的深藍布疋迎風招展如雲霞。"),
                    ("Touch", "你摸了摸新織成的厚帆布料，經緯交錯緊實密致，具備絕佳抗風抗撕拉韌性。"),
                    ("DyeIndigoCloth", "你將織造緊密的棉麻坯布浸入天然發酵的山藍大染缸，反覆提晾透風發色，染出深邃耐雨的群青色澤。"),
                ],
            ),
        ]
    } else if room_name.contains("五金鋪") || room_name.contains("車輪鑲鐵") {
        vec![
            make_peninsula_object(
                "wt_obj_hardware_bins",
                "五金分格櫃",
                &["Look", "Check"],
                &[
                    ("Look", "鐵皮壁櫃上百餘個木匣盛滿各型鉚釘、螺栓、合頁與銷釘，地坪上置有一圈圓形冷水槽，專供燒紅車輪鋼箍套木輪後急速冷縮咬合。"),
                    ("Check", "你翻看鐵匣中打製好的精鋼鉚釘，釘體圓整，毫無毛刺。"),
                ],
            ),
        ]
    } else if room_name.contains("烈酒大飯鋪") || (room_name.contains("工匠") && room_name.contains("食堂")) {
        vec![
            make_peninsula_object(
                "wt_obj_forge_mess_cauldron",
                "大灶鐵炒鑊",
                &["Look", "Taste"],
                &[
                    ("Look", "泥石大灶上架著一口能容數百斤飯食的厚鑄鐵大鑊，熱油爆炒蔥薑大塊原牛肉香氣熏天，牆角立著數只大號土陶老白乾酒罈。"),
                    ("Taste", "你舀了一小勺濃油赤醬的牛肉燴菜品嚐，鹹香濃烈，極為開胃下飯。"),
                ],
            ),
        ]
    } else {
        return None;
    };
    Some(objs)
}

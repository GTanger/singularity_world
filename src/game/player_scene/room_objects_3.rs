#[allow(unused_variables)]
fn peninsula_objects_chunk_3(room_name: &str) -> Option<Vec<RoomObject>> {
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

    let objs = if !is_outdoor_road && !is_residential_house && room_name.contains("試招") && !room_name.contains("文曜") {
        vec![
            make_peninsula_object(
                "wt_arena_testing_pillar",
                "玄武試勁柱",
                &["Look", "Operate", "SparPillar"],
                &[
                    ("Look", "場中央立著取自烏石鼻的沉重玄武岩石柱，柱面留有深淺不一的掌印、拳痕與兵刃劈砍印記。"),
                    ("Operate", "你運勁凝神朝石柱發力擊出一記，拳風激盪石壁發出沉悶轟響，指掌微麻筋骨通暢。"),
                    ("SparPillar", "你沉腰落馬繞玄武試勁柱演練四序拆招，拳掌與兵刃交擊石柱發出沉雄悶響。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_wooden_weapon_rack",
                "鐵木兵器架",
                &["Look", "Use", "SparPillar"],
                &[
                    ("Look", "架上排滿各型配重勻稱的鐵木長棍、練習單刀、短槍與厚實的原牛皮護胸。"),
                    ("Use", "你抽出一桿沉實的鐵木長棍在手中虛晃挽出幾朵棍花，重心沉著穩當，手感極佳。"),
                    ("SparPillar", "你沉腰落馬繞玄武試勁柱演練四序拆招，拳掌與兵刃交擊石柱發出沉雄悶響。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_rest_bench",
                "歇腳粗布凳",
                &["Look", "Rest"],
                &[
                    ("Look", "場邊長凳鋪著耐磨粗布墊，旁邊常備著供習練者洗臉降溫的深井涼泉陶缸。"),
                    ("Rest", "你在粗布長凳上坐下吐納調息，平復躁動血氣，奔馳發力的筋骨漸漸恢復充盈精力。"),
                ],
            ),
        ]
    } else if room_name.contains("農具") {
        vec![
            make_peninsula_object(
                "wt_farm_tool_iron_plow",
                "雙齒精鐵犁",
                &["Look", "Operate", "PressCheese"],
                &[
                    ("Look", "為原牛翻耕黏土設計的淬火精鐵犁鏵，泥槽打磨平滑，犁尖銳利厚實。"),
                    ("Operate", "你試抬起犁鏵大轅感受配重，槓桿角度設計精巧，翻開深層黑土輕便省力。"),
                    ("PressCheese", "你協同牧耕勞作，將牧場原乳送入沉澱槽壓製乳酪，分理收成。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_bamboo_harrow",
                "竹編平土耙",
                &["Look", "Use"],
                &[
                    ("Look", "排布均勻的竹木平土排耙與輕便鋼口除草鋤，擺在農夫取用順手的高架上。"),
                    ("Use", "你拿起除草鋤試鋤地面浮泥，鋼口切入泥土利索乾脆，雜草根系迎刃而斷。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_whetstone_trough",
                "活水磨刀槽",
                &["Look", "Use"],
                &[
                    ("Look", "木架凹槽裡嵌著一粗一細兩合磨石，活水自小竹管徐徐滴淋在石面保持濕潤。"),
                    ("Use", "你把身邊的鐵刃貼在磨石上推磨幾回，泥漿流動間，原本微鈍的刃口頓時磨得鋥亮。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("乳酪") || room_name.contains("羊毛")) {
        vec![
            make_peninsula_object(
                "wt_cheese_curd_press",
                "螺旋壓酪架",
                &["Look", "Operate", "Taste", "PressCheese"],
                &[
                    ("Look", "大木架上設有黃銅螺旋加壓手柄，正將粗濾布中的凝乳塊瀝出金黃乳清，壓製硬乳酪輪。"),
                    ("Operate", "你握住螺旋桿加力旋轉半圈，木板沉重下壓，澄澈的乳清順著凹槽汩汩流出。"),
                    ("Taste", "你嚐了指尖沾上的一滴新鮮乳清，滋味微酸清冽，帶著濃醇的天然奶香。"),
                    ("PressCheese", "你旋緊螺旋木桿壓榨酪包，黃亮乳清緩緩瀝出，壓製出緊實濃郁的熟成硬乳酪。"),
                ],
            ),
            make_peninsula_object(
                "wt_wool_spinning_wheel",
                "手搖木紡車",
                &["Look", "Use", "PressCheese"],
                &[
                    ("Look", "案上堆放著洗淨蓬鬆的乳白山羊毛條，竹木紡車轉軸靈動，紡出勻細結實的線團。"),
                    ("Use", "你輕輕搖動紡車木柄，纖細羊毛在引線指引下迅速捻轉成光滑毛線，均勻綿密。"),
                    ("PressCheese", "你整理牧場原乳與工坊物產，配合木壓架壓去乳清，收緊熟成乳酪磚。"),
                ],
            ),
            make_peninsula_object(
                "wt_cheese_aging_shelf",
                "乳酪熟成架",
                &["Look", "Smell", "Open", "PressCheese"],
                &[
                    ("Look", "多層松木架內嵌寒水玉薄板調節溫涼，架上排滿刷洗乾淨、裹著黃蠟的牛埔陳年硬乳酪。"),
                    ("Smell", "你靠近熟成架細聞，發酵成熟的乳香濃郁厚重，混合著天然油脂與微鹹海風韻味。"),
                    ("Open", "你拉開底層遮光木簾，裡面陳列著熟成滿三個月的金黃乳酪輪，成色極佳。"),
                    ("PressCheese", "你巡視松木熟成架，挑選硬度適中的乳酪翻面壓實，香氣濃醇。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("選礦") {
        vec![
            make_peninsula_object(
                "wt_mine_wash_chute",
                "木製洗礦槽",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "山澗急流順著長木槽奔騰而下，槽底隔木截留住沉重赤鐵礦砂，洗去浮游紅泥。"),
                    ("Operate", "你以長木耙梳理泥槽底層礦石，比重較輕的雜石順水沖走，沉甸甸的鐵礦砂盡收格內。"),
                    ("HaulMineCart", "你引地下冷泉沖洗礦砂，分理碎石，將赤鐵重礦裝入滑車鐵斗。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_sample_tray",
                "礦石樣品箱",
                &["Look", "Use"],
                &[
                    ("Look", "分格木箱裡擺著高品位重鐵礦石塊與細膩如雪的高嶺瓷土樣粉，標籤字跡清晰。"),
                    ("Use", "你掂起一塊沉甸甸的黑紅赤鐵礦石，質地堅密沉實，正是鍛造上好精鐵的基石。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_sorting_sieve",
                "分級重鐵篩",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "懸掛在木架下的雙層鐵絲篩網，孔徑由大到小，將開採出的碎礦精確分級。"),
                    ("Operate", "你推動鐵篩手柄用力搖動，小顆粒礦砂簌簌漏下，大塊粗礦穩穩留於上層。"),
                    ("HaulMineCart", "你晃動重鐵篩分揀礦石，剔除雜岩，將沉重精礦推入礦車軌道。"),
                ],
            ),
        ]
    } else if room_name.contains("台車") {
        vec![
            make_peninsula_object(
                "wt_trolley_rail_carriage",
                "雙軸重台車",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。"),
                    ("Operate", "你握住車身旁的機械制動閘手柄扳動，鐵閘緊緊咬住輪緣，發出沉實摩擦聲。"),
                    ("HaulMineCart", "你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴，將滿車赤鐵重礦運出坑道。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_grease_pot",
                "桐油潤滑壺",
                &["Look", "Use"],
                &[
                    ("Look", "油箱邊放著盛滿防鏽桐油脂的長嘴鐵壺，以及用於緊固軌道銷釘的粗重鍛鐵鉗。"),
                    ("Use", "你提起油壺往台車轉軸注入口滴入幾滴桐油脂，原本微澀的輪軸隨即轉動無聲。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_brake_winch",
                "鋼索絞盤",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "固定在坑口條石基座上的大型手搖絞盤，上面纏繞著粗大耐磨的精鋼牽引索。"),
                    ("Operate", "你推動棘輪卡榫試轉絞盤，齒輪咬合緊密，能輕鬆將滿載台車拉上陡斜坡道。"),
                    ("HaulMineCart", "你收放絞盤鋼索導引重台車入軌，齒輪咬合聲響徹坑口，平穩起運重礦。"),
                ],
            ),
        ]
    } else if room_name.contains("燈塔") {
        vec![
            make_peninsula_object(
                "wt_lighthouse_fresnel_lens",
                "聚光巨鏡組",
                &["Look", "Operate"],
                &[
                    ("Look", "高塔頂層旋轉銅架上鑲嵌著碩大的夜光貝打磨薄片與厚琉璃凸透鏡，聚起刺破黑夜的耀目光柱。"),
                    ("Operate", "你輕推銅架轉輪微調鏡組方位，聚光透鏡緩緩轉動，光束精準掃過遠方險礁水域。"),
                ],
            ),
            make_peninsula_object(
                "wt_lighthouse_whale_oil_burner",
                "海油恆溫爐",
                &["Look", "WarmHands"],
                &[
                    ("Look", "精銅爐膛內燃燒著精煉海獸油，芯線粗實，散發著溫純明亮的橙紅火光與微溫。"),
                    ("WarmHands", "你將雙手覆在燈爐外緣銅罩上，高塔風烈之下的冰冷指尖瞬間被烘得暖和無比。"),
                ],
            ),
            make_peninsula_object(
                "wt_lighthouse_log_desk",
                "夜航日誌案",
                &["Look", "Read"],
                &[
                    ("Look", "書案上整齊擺放著潮汐圖表、銅墨水盒與記載過往船隻燈號答覆的夜航日誌。"),
                    ("Read", "你翻開夜航日誌閱覽，上面工整記著昨夜三艘外澳貨船平安繞過暗礁的航行記錄。"),
                ],
            ),
        ]
    } else if room_name.contains("冷燻") {
        vec![
            make_peninsula_object(
                "wt_smokehouse_kiln",
                "慢燻紅陶灶",
                &["Look", "Smell", "SmokeFish"],
                &[
                    ("Look", "地面砌出長排低溫煙道陶灶，正以受控的乾燥松木鋸屑慢火逼出濃郁木香冷煙。"),
                    ("Smell", "你深吸一口自陶灶溢出的煙氣，醇厚的松木煙燻香氣純淨甘醇，全無嗆鼻焦味。"),
                    ("SmokeFish", "你添入濕柏木與香草慢燻，灶內青煙繚繞，低溫透骨冷燻海鹽魚乾。"),
                ],
            ),
            make_peninsula_object(
                "wt_smokehouse_fish_rack",
                "冷燻海魚架",
                &["Look", "Taste", "SmokeFish"],
                &[
                    ("Look", "懸架上整齊掛著一排排泛著琥珀油亮金紅光澤的冷燻鯖魚與海苔魚乾，油脂豐厚。"),
                    ("Taste", "你撕下一小片冷燻魚肉放入口中，肉質緊實彈牙，鹹香甘醇中交織著深沉的松煙木香。"),
                    ("SmokeFish", "你翻動竹架上的醃製海魚，煙氣浸潤肉質，燻烤出金黃油亮的冷燻魚乾。"),
                ],
            ),
            make_peninsula_object(
                "wt_smokehouse_salt_barrel",
                "海鹽醃魚桶",
                &["Look", "Open"],
                &[
                    ("Look", "杉木大桶盛滿風舶角日曬結晶的潔白粗海鹽，用以在下燻爐前醃透鮮魚肉質。"),
                    ("Open", "你掀開杉木桶蓋，大顆粒海鹽晶瑩剔透，底下浸潤著新近剖好醃漬的鮮美魚肉。"),
                ],
            ),
        ]
    } else if room_name.contains("巨藻") {
        vec![
            make_peninsula_object(
                "wt_kelp_drying_trellis",
                "高架曬藻棚",
                &["Look", "Gather", "BurnLime"],
                &[
                    ("Look", "迎著海風搭建的巨型竹棚上懸垂著長達數丈的深海厚巨藻，海風吹拂下漸乾變韌。"),
                    ("Gather", "你從竹架邊緣摘下一束完全曬乾的深黑巨藻片折好，乾脆柔韌，滿是海味精華。"),
                    ("BurnLime", "你將高架曬乾的堅韌海藻與貝灰一同收整，備齊風舶角搓繩與造船用料。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_rope_twister",
                "海纜搓繩輪",
                &["Look", "Operate", "BurnLime"],
                &[
                    ("Look", "工匠用極富韌性的巨藻纖維與麻絲混合，在木轉輪上捻搓出不懼海水浸蝕的沉重船纜。"),
                    ("Operate", "你推動轉輪搖柄旋轉，條條藻絲纖維被均勻絞緊成粗壯纜繩，結實程度堅如生鐵。"),
                    ("BurnLime", "你轉動絞輪將浸過貝灰防腐液的巨藻與麻纖維絞合，搓製出粗實耐磨的麻繩卷。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_powder_mill",
                "藻粉研石臼",
                &["Look", "Smell", "Taste"],
                &[
                    ("Look", "青石臼中正用石杵搗研烘乾的厚巨藻脆片，研磨出的深翠色細粉即是提鮮神物。"),
                    ("Smell", "你湊近石臼輕聞，濃郁鮮美的天然海潮香氣撲面而來，引人食指大動。"),
                    ("Taste", "你以指尖沾少許海藻提鮮粉嚐了嚐，鹹鮮甘美無比，是半島菜餚不可或缺的調味秘方。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("灰窯") {
        vec![
            make_peninsula_object(
                "wt_lime_kiln_chamber",
                "海貝燃灰窯",
                &["Look", "WarmHands", "Operate", "BurnLime"],
                &[
                    ("Look", "烏石鼻以玄武岩壘砌的圓形灰窯，烈火正將堆積的海貝殼高溫煅燒成雪白的建材灰粉。"),
                    ("WarmHands", "你將身子貼近灰窯厚實的外牆，石壁蓄積的渾厚餘熱烘散了海邊帶來的陰冷濕氣。"),
                    ("Operate", "你使用長柄鐵扒撥動窯底通風孔讓空氣流通，爐膛火勢更加旺盛純青。"),
                    ("BurnLime", "你添入焦炭鼓風引燃灰窯，高溫煅燒海貝碎殼，出窯白灰熱氣騰騰，細密凝實。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_powder_bin",
                "熟貝灰石槽",
                &["Look", "Use"],
                &[
                    ("Look", "大石槽中盛滿過篩後的熟貝灰粉，細膩如白麵，遇水化作黏結力極強的砌石砂漿。"),
                    ("Use", "你取少許熟貝灰加水在陶盆中攪拌，灰漿濃稠細膩，抹在石縫間黏合得天衣無縫。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_shell_pile",
                "粗貝殼料堆",
                &["Look", "Gather"],
                &[
                    ("Look", "窯外堆積著如小山般的厚重海貝殼，皆是海風大浪自深海衝刷上海灘的天然饋贈。"),
                    ("Gather", "你自料堆拾起幾片質地特別厚重堅硬的大海貝殼，擦淨細沙收於手中。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("獵具") {
        vec![
            make_peninsula_object(
                "wt_hunt_heavy_crossbow",
                "牛角破甲弩",
                &["Look", "Operate", "PrepareMiasmaGear"],
                &[
                    ("Look", "工坊以大角原牛角片與堅木多層膠合的守林重弩，弩弦以原牛筋絞就，張力沉雄。"),
                    ("Operate", "你扳動腳蹬絞盤為重弩上弦，機括發出清脆卡響，鎖定弩機扣板，蓄勢待發。"),
                    ("PrepareMiasmaGear", "你校準破甲弩機，配齊紅磷引路信號筒與避瘴藥劑，整飭行囊以防黑沼瘴毒。"),
                ],
            ),
            make_peninsula_object(
                "wt_hunt_iron_trap",
                "帶齒鋼獸夾",
                &["Look", "Use", "PrepareMiasmaGear"],
                &[
                    ("Look", "塗有黑沼天然樹膠防鏽黑漆的踏板精鋼獸夾，彈簧沉重有力，專用於防範猛獸。"),
                    ("Use", "你拿起長鐵枝撥動安全鎖扣試探獸夾靈敏度，踏板微沉立刻金屬閉合，迅猛精準。"),
                    ("PrepareMiasmaGear", "你檢查捕獸齒夾的機簧韌度，塗抹避瘴油脂，備足探沼防護器物。"),
                ],
            ),
            make_peninsula_object(
                "wt_hunt_tallow_kit",
                "牛油護弦盒",
                &["Look", "Use"],
                &[
                    ("Look", "木盒內收著浸透精煉牛油的布團與蜂蠟塊，是防止弓弦受潮發黴的必備護理之物。"),
                    ("Use", "你取了浸油布團細細擦拭牛筋弩弦，油脂浸入纖維，原本乾燥的弦絲立顯光澤柔韌。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("竹編") || room_name.contains("編織工棚")) {
        vec![
            make_peninsula_object(
                "wt_bamboo_weaving_bench",
                "剖竹切篾台",
                &["Look", "Operate"],
                &[
                    ("Look", "平整木剖台上架著圓弧形雙柄剖竹刀，熟練工匠正將青竹劈成薄如蟬翼的均勻竹篾。"),
                    ("Operate", "你雙手握持刀柄順著竹管纖維平推，青竹瞬間應刃破開，剖出兩根粗細一致的平滑竹條。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_woven_crates",
                "雙層竹魚籠",
                &["Look", "Use"],
                &[
                    ("Look", "夾層塗有防水樹膠的密編雙層竹箱與魚籠，輕便結實，通風防潮，是水鄉日用良器。"),
                    ("Use", "你合上手提竹箱的竹編搭扣試提，箱身輕巧挺括，能承受數十斤重物而不變形。"),
                ],
            ),
            make_peninsula_object(
                "wt_bamboo_drying_rack",
                "青竹晾曬架",
                &["Look", "Gather"],
                &[
                    ("Look", "大棚架上整齊排晾著經過熱水蒸煮去糖防蛀的老青竹竿，竹皮碧綠油亮，質地極韌。"),
                    ("Gather", "你從架上挑選了一截粗細勻稱的乾燥老竹管收好，可用於製作器械或竹編配件。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("船塢") || room_name.contains("帆具") || room_name.contains("船材")) {
        vec![
            make_peninsula_object(
                "wt_shipyard_dry_dock",
                "雲杉龍骨架",
                &["Look", "Operate", "BurnLime"],
                &[
                    ("Look", "斜坡船台架著一整根長逾十丈的厚重雲杉大龍骨，工匠們正以青銅大釘和榫卯拼合肋骨。"),
                    ("Operate", "你拿起大木槌將一枚粗大青銅釘打入龍骨榫卯深處，木料緊密咬合，牢固無比。"),
                    ("BurnLime", "你調和熱貝灰桐油填補龍骨板縫，緊固麻繩卷加固船架，船塢工序有條不紊。"),
                ],
            ),
            make_peninsula_object(
                "wt_shipyard_sail_rigging",
                "巨藻船纜堆",
                &["Look", "Use"],
                &[
                    ("Look", "棚內堆放著浸過桐油的防風暴密織重帆布、滑車索具與搓絞結實的深海巨藻大絞纜。"),
                    ("Use", "你伸手扯動帆索穿過滑車組試拉，滑車轉動順暢無阻，粗重帆布應聲升起。"),
                ],
            ),
            make_peninsula_object(
                "wt_shipyard_caulking_tar",
                "桐油捻縫鍋",
                &["Look", "WarmHands", "Smell"],
                &[
                    ("Look", "鐵鍋正慢火加熱由桐油、熟貝灰與細麻絲調製的捻縫灰膏，用於填補船板接縫。"),
                    ("WarmHands", "你把雙手靠近鐵鍋周圍的溫熱泥灶，濃稠灰膏散發的熱度將指尖寒意全數烘化。"),
                    ("Smell", "你細聞灰鍋飄出的氣味，桐油的醇厚木香伴著貝灰氣息，是海港船塢熟悉的味道。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("商行") || room_name.contains("契據")) {
        vec![
            make_peninsula_object(
                "wt_merchant_ledger_counter",
                "樟木掌事台",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "整塊老樟木打造的掌事長台，台面擺滿銅秤、商盟印鑑、硃砂印泥與出入貨記帳契簿。"),
                    ("Read", "你閱覽櫃檯上攤開的貨運契約簿，上面詳細登記著近日運往各鎮的糧油與礦料數量。"),
                    ("Use", "你拿起硃砂印章在契據末尾端正蓋下一記鮮紅印記，商契正式生效立案。"),
                ],
            ),
            make_peninsula_object(
                "wt_merchant_currency_exchange",
                "商盟票據箱",
                &["Look", "Open"],
                &[
                    ("Look", "加固銅角木箱內整齊收存著各商號發行的兌換信票，以及分格盛裝零碎金銀的軟牛皮夾。"),
                    ("Open", "你拉開分格木抽屜查看，各色商行票據分類清晰，印鑑完備，流通無礙。"),
                ],
            ),
            make_peninsula_object(
                "wt_merchant_sample_case",
                "特產陳列櫥",
                &["Look", "Open"],
                &[
                    ("Look", "靠牆玻璃櫥內展示著半島各地的代表物產樣品：紅土鐵錠、牛埔乳酪、霜林茶葉與烏石海貝。"),
                    ("Open", "你輕推櫥門取出霜林茶葉樣品查看，茶葉乾爽翠綠，散發著頂級烘青茶的幽香。"),
                ],
            ),
        ]
    } else if !is_outdoor_road
        && (room_name.contains("聯運")
            || room_name.contains("貨運")
            || room_name == "老堰頭・村中泥徑"
            || (room_name.contains("半嶺鎮") && room_name.contains("街"))
            || (room_name.contains("大圳頭") && room_name.contains("街"))
            || (room_name.contains("南哨") && room_name.contains("街"))
            || (room_name.contains("風舶角") && room_name.contains("街")))
    {
        vec![
            make_peninsula_object(
                "wt_caravan_harness_rack",
                "雙輪大貨車",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "大棚下停滿雙輪大貨車，架上排著浸過牛油的厚牛皮馱墊與精鐵挽具。"),
                    ("Use", "你取下原牛挽具檢查皮帶扣環，牛皮堅韌柔軟，鐵扣打磨光滑，毫無磨損隱患。"),
                    ("Operate", "你協同車夫將重貨車大轅抬起卡入牽引插銷，車架結實穩固，準備發車上路。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use", "FeedDraftBeast"],
                &[
                    ("Look", "連通活水渠的深鑿青石長水槽，供來往跋涉的原牛與健壯駝獸低頭痛飲，水質清澈。"),
                    ("Use", "你拔開進水木栓注入新鮮井泉活水，奔波而來的原牛探頭大口痛飲，發出滿足低哼。"),
                    ("FeedDraftBeast", "你拔開活水木栓注滿清泉，並添入新鮮青草料，供遠途跋涉的原牛與駝獸飽餐歇息。"),
                ],
            ),
            make_peninsula_object(
                "wt_caravan_ticket_counter",
                "班次木牌案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "木牌案上每日更換木質班次籤牌，寫明發往大圳頭、風舶角與南哨的車馬出發時刻。"),
                    ("Read", "你細看班次牌，往大圳頭方向的大車午後即將發班，回程車隊則已進棚卸貨。"),
                    ("Use", "你向車馬行主事交付車資，登車準備前往下一處聯運重鎮。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("鐘樓") || room_name.contains("信使") || room_name.contains("瞭望")) {
        vec![
            make_peninsula_object(
                "wt_clocktower_bronze_bell",
                "報時青銅鐘",
                &["Look", "Operate", "CalibrateClock"],
                &[
                    ("Look", "高懸塔頂的厚重青銅巨鐘，垂下粗大的麻繩，每逢正點敲響，鐘聲清越浩蕩傳遍全境。"),
                    ("Operate", "你雙手握住粗麻繩合力一拉，青銅鐘錘重重撞上鐘體，發出一聲渾厚深沉的悠揚巨響。"),
                    ("CalibrateClock", "你比對青銅旋轉日晷的日影刻度，調校鐘樓擒縱齒輪與報時銅鐘，使全城更次分秒不差。"),
                ],
            ),
            make_peninsula_object(
                "wt_messenger_mailbag_desk",
                "防雨郵袋案",
                &["Look", "Use", "Read"],
                &[
                    ("Look", "案上排滿蓋有半島各鎮印戳的竹綿紙路引，以及供快騎隨身馱帶的雙扣防水郵袋。"),
                    ("Use", "你將封好的信札仔細裝入防雨郵袋扣緊銅扣，妥善交到整裝待發的信使手中。"),
                    ("Read", "你查看案上的郵路派遣日程，各聚落每日皆有快騎往返傳遞急件與民信。"),
                ],
            ),
            make_peninsula_object(
                "wt_messenger_lookout_perch",
                "青銅旋轉晷",
                &["Look", "Read", "Operate", "CalibrateClock"],
                &[
                    ("Look", "高台邊緣設有刻劃周密刻度線的青銅日規，晷針在陽光下投射出清晰分明的銳利陰影。"),
                    ("Read", "你俯視日規晷針指向，日影恰好落在午正刻度線上，時刻分秒不差。"),
                    ("Operate", "你輕推日規底盤旋鈕校準微調角度，與遠方主塔旗號完全對齊。"),
                    ("CalibrateClock", "你比對青銅旋轉日晷的日影刻度，調校鐘樓擒縱齒輪與報時銅鐘，使全城更次分秒不差。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("碼頭")
        || room_name.contains("泊位")
        || room_name.contains("竹筏埠")
        || room_name.contains("渡埠")
        || room_name.contains("水岸")
        || room_name.contains("河畔")
        || room_name.contains("渡口")
        || room_name.contains("竹筏"))
        && !room_name.contains("公井")
    {
        vec![
            make_peninsula_object(
                "wt_dock_riverbank_raft",
                "平底竹筏",
                &["Look", "Operate", "Use", "RowSwampRaft"],
                &[
                    ("Look", "水岸石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著數艘雙層防浸平底竹筏，筏上置有推水長竹篙與青竹魚簍。"),
                    ("Operate", "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。"),
                    ("Use", "你手持長竹篙在石階輕撐借力，竹筏滑入川流清澈碧流，沿岸水草豐美、水鳥掠波。"),
                    ("RowSwampRaft", "你解開纜繩點篙撐起竹筏，輕舟破開水尾寮蘆葦與黑沼迷霧，順流探尋澤中沉水烏木與止血水苔。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_stone_mooring_bollard",
                "繫船玄武樁",
                &["Look", "Operate"],
                &[
                    ("Look", "碼頭邊緣深埋地底的圓頭玄武岩石樁，樁身被粗大的巨藻船纜磨得油亮光滑。"),
                    ("Operate", "你接過水手拋來的粗纜繩，俐落打了一個雙套結套在石樁上，船身穩穩貼岸。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_cargo_gangplank",
                "雲杉卸貨板",
                &["Look", "Use", "BoardCoastalShip"],
                &[
                    ("Look", "連接貨船與石砌棧橋的厚實跳板，表面刻有縱橫防滑凹槽，方便水手挑擔搬運重貨。"),
                    ("Use", "你踏上平穩結實的木跳板大步走上棧橋，腳下厚板紋絲不晃，安穩可靠。"),
                    ("BoardCoastalShip", "你踏過厚實防滑的雲杉跳板登上環島商船，隨水手核對黑潮航線與潮汐水文。"),
                ],
            ),
            make_peninsula_object(
                "wt_dock_tide_pole",
                "青銅驗潮尺",
                &["Look", "Read"],
                &[
                    ("Look", "嵌在防波石堤側面的青銅驗潮尺，刻著醒目的深淺刻痕，潮水在其下輕輕湧動拍擊。"),
                    ("Read", "你低頭查看水線刻度，此時正值滿潮平水之際，各型吃水深的大船皆能順利進港泊靠。"),
                ],
            ),
        ]
    } else if room_name.contains("市集")
        || room_name.contains("特產行")
        || room_name.contains("水產")
        || room_name.contains("乾貨")
        || room_name.contains("海味")
        || room_name.contains("物資行")
        || room_name.contains("素材")
        || room_name.contains("調配")
        || room_name.contains("調撥")
    {
        vec![
            make_peninsula_object(
                "wt_market_produce_stand",
                "山海乾貨攤",
                &["Look", "Taste", "Smell", "SelectDriedSeafood"],
                &[
                    ("Look", "攤位上滿是紅土崁黑糖、酸甜山果乾、大圳頭米香、鹽烤海苔片與風櫃尾冷燻魚乾，琳瑯滿目。"),
                    ("Taste", "你捻起一塊紅土崁黑糖放入口中細嚼，純粹甘甜中帶著濃郁焦香，滋味醇厚無比。"),
                    ("Smell", "你細聞攤前混和著米香、海苔與果乾的甜美香氣，市集生活氣息分外濃郁。"),
                    ("SelectDriedSeafood", "你在乾貨攤前細細挑選色澤金黃透亮的風櫃尾冷燻烏魚子與乾貝，海味醇厚。"),
                ],
            ),
            make_peninsula_object(
                "wt_market_scale_counter",
                "黃銅雙盤秤",
                &["Look", "Operate", "SelectDriedSeafood"],
                &[
                    ("Look", "懸在木架上的精準黃銅雙盤大吊秤，配備一套由小至大的青銅砝碼，用於買賣過磅。"),
                    ("Operate", "你將砝碼置於左盤，右盤放入乾貨，秤桿微微擺動後平穩水平，分量精確公平。"),
                    ("SelectDriedSeafood", "你在黃銅雙盤秤前過磅揀選風櫃尾冷燻烏魚子禮包，斤兩十足。"),
                ],
            ),
            make_peninsula_object(
                "wt_market_water_tank",
                "活水海鮮盆",
                &["Look", "Use"],
                &[
                    ("Look", "引流海水的長木盆中魚蝦游動，水花翻滾，全為新近捕撈上岸的生猛活海鮮。"),
                    ("Use", "你拿起小竹撈網在水盆中輕輕一撈，網起一尾鱗光耀眼的生猛海魚，活力充沛。"),
                ],
            ),
        ]
    } else if is_residential_house
        || room_name.contains("住宅")
        || room_name.contains("民宅")
        || room_name.contains("居所")
        || room_name.contains("家屋")
        || room_name.contains("公寓")
        || room_name.contains("街屋")
        || room_name.contains("農舍")
        || room_name.contains("石屋")
        || room_name.contains("漁舍")
        || room_name.contains("暖屋")
        || room_name.contains("宿舍")
        || room_name.contains("高腳木屋")
        || room_name.contains("宅邸")
        || room_name.contains("宅院")
        || room_name.contains("莊園")
        || room_name.contains("別院")
        || room_name.contains("民舍")
    {
        vec![
            make_peninsula_object(
                "wt_home_brass_tap_sink",
                "水栓",
                &["Look", "Use", "Bath"],
                &[
                    ("Look", "打磨光滑的黃銅水栓連接著老堰頭活水管道，下方是紅土耐火磚砌成的溫石蓄熱水槽，槽邊擺著草木灰柑橘洗潔皂、軟毛竹柄潔齒刷與青鹽薄荷潔牙粉。"),
                    ("Use", "你擰開黃銅水栓，清冽純淨的活水流淌而出，隨後轉入溫水檔，溫暖的水流洗去塵泥，柑橘皂香格外清新。"),
                    ("Bath", "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_porcelain_lavatory",
                "白瓷沖水池",
                &["Look", "Use"],
                &[
                    ("Look", "紅土崁高嶺白瓷燒製的便池平滑光潔，引老堰頭活水沖洗，淨房牆側掛著柔軟的竹綿衛生草紙。"),
                    ("Use", "你拉動沖水黃銅把手，活水漩渦利落將便池沖洗得乾淨無痕，屋內潔淨乾燥無異味。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_cold_jade_cabinet",
                "冰鑑櫃",
                &["Look", "Open", "Taste"],
                &[
                    ("Look", "小巧的家用石櫃由松香木與北山寒水玉夾層打製，存放著鮮牛乳、板豆腐、脆瓜醬菜與桂花新米糕，旁有白釉瓷碗盤與烏木筷。"),
                    ("Open", "你拉開石櫃門，涼意撲面而來，盤中的米糕與鮮牛乳保存完好，散發著微甘甜香。"),
                    ("Taste", "你取了一塊桂花新米糕送入口中，軟糯香甜，米香與桂花清香在舌尖悠悠化開。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_pearl_lamp_desk",
                "烏木書案",
                &["Look", "Operate", "Read"],
                &[
                    ("Look", "松木書案邊角包著黃銅片，案上置有黃銅發條座鐘、鋼尖蘸水硬筆與竹綿紙記事手札，案角立著旋蓋夜光貝母檯燈。"),
                    ("Operate", "你旋轉烏木燈罩調光，夜光貝的珠白光芒自罩隙均勻灑落，柔和澄澈。"),
                    ("Read", "你翻開竹綿紙記事手札，上面工整記錄著日常糧柴開銷、修繕屋瓦的備忘與鄰里往來雜記。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_toolbox_and_mailbox",
                "家用工具箱",
                &["Look", "Open", "Use"],
                &[
                    ("Look", "玄關軟底布便鞋旁收著松木家用工具箱，門前防雨竹編信箱裡插著新到的書信。"),
                    ("Open", "你掀開工具箱銅扣，裡面分格收著羊角錘、旋釘鑿、防潮紅磷火籤與細麻繩。"),
                    ("Use", "你取出一枚防潮紅磷火籤在側邊輕擦，升起穩定微亮的火苗，隨後自信箱取出書信細閱。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_feather_bed_lamp",
                "羽絨暖臥榻",
                &["Look", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松木臥榻上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母調光檯燈，光暈柔和溫潤。"),
                    ("Rest", "你合衣在柔軟的羽絨臥榻上躺下，安神草的清香讓緊繃的神經徹底放鬆下來。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母燈。在靜謐祥和的屋內，你沉沉安睡，體魄與精神盡復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母燈的烏木旋蓋，外露的珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ),
        ]
    } else if room_name.contains("剪剃") || room_name.contains("理髮") {
        vec![
            make_peninsula_object(
                "wt_barber_folding_razor",
                "摺疊剃刀架",
                &["Look", "Use", "GroomAppearance"],
                &[
                    ("Look", "黃銅盤內整齊排著磨得雪亮的精鋼摺疊剃刀，旁側紫銅熱布巾架正冒著蒸騰水氣與薄荷清香。"),
                    ("Use", "你取過一把精鋼摺疊剃刀試試鋒刃，刀身微薄而銳利，剃刮修鬚乾淨俐落。"),
                    ("GroomAppearance", "你敷上薄荷熱毛巾，由修面師傅以精鋼剃刀與松香木梳修整儀容，神彩煥然一新。"),
                ],
            ),
            make_peninsula_object(
                "wt_barber_copper_chair",
                "洗頭木躺椅",
                &["Look", "Sit", "Rest", "GroomAppearance"],
                &[
                    ("Look", "厚實白蠟木躺椅側邊接連著紫銅溫水管道與陶泥洗頭槽，軟皮頭枕弧度貼合。"),
                    ("Sit", "你在紫銅洗頭椅上躺靠下來，溫熱的水氣自銅管微熱散發，頸肩頓時放鬆。"),
                    ("Rest", "你閉上雙眼在躺椅上小憩片刻，聽著水槽流水汩汩，洗去整日奔波的疲乏。"),
                    ("GroomAppearance", "你敷上薄荷熱毛巾，由修面師傅以精鋼剃刀與松香木梳修整儀容，神彩煥然一新。"),
                ],
            ),
        ]
    } else if room_name.contains("會計") || room_name.contains("商算") || room_name.contains("算學") {
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
                "wt_abacus_ledger_board",
                "核算大木牌",
                &["Look", "Read"],
                &[
                    ("Look", "青石牆面懸掛著黑漆大木板，上頭以白堊石筆端楷寫滿三水口各船號的貨品抽成與利息折算例題。"),
                    ("Read", "你仔細閱讀木牌上的算例，條理分明，正是半島商行事務員必修的進階算術。"),
                ],
            ),
        ]
    } else if !is_residential_house && (room_name.contains("習武") || (room_name.contains("試招") && room_name.contains("文曜"))) {
        vec![
            make_peninsula_object(
                "wt_martial_padded_wooden_blade",
                "包革木刀架",
                &["Look", "Use", "Operate", "SparPillar"],
                &[
                    ("Look", "沉重木架上陳列著厚牛皮包覆的練習重木刀與熟銅卸力護腕，刀身刻有岑家分家工坊鋼印。"),
                    ("Use", "你反手拔出一柄包革木刀揮斬兩式，重心沉穩合手，破風聲呼呼作響。"),
                    ("Operate", "你調整護腕上的銅扣束帶，勒緊手腕測試卸力彈性，極利於修習剛猛招法。"),
                    ("SparPillar", "你沉腰落馬繞玄武試勁柱演練四序拆招，拳掌與兵刃交擊石柱發出沉雄悶響。"),
                ],
            ),
            make_peninsula_object(
                "wt_martial_force_stone",
                "測勁玄武碑",
                &["Look", "Use", "Operate", "TestEngraving"],
                &[
                    ("Look", "院角立著一堵嵌有暗金紋理的厚重玄武石板，表面布滿被勁力拍擊出的微凹掌印與拳痕。"),
                    ("Use", "你運轉周身氣息推按測紋石板，掌力透入石紋，石板微鳴回饋內息剛柔，使你對發力關竅深有所得。"),
                    ("Operate", "你深吸一口氣運勁於掌，拍向測紋石板，石面暗紋隱隱亮起微光，回震沉實。"),
                    ("TestEngraving", "你運轉內息按上測紋石板並取隨身器物輕觸碑面，石紋微鳴回饋人器共鳴度。"),
                ],
            ),
        ]
    } else {
        return None;
    };
    Some(objs)
}

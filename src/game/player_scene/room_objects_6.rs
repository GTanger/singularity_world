#[allow(unused_variables)]
fn peninsula_objects_chunk_6(room_name: &str) -> Option<Vec<RoomObject>> {
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

    let objs = if room_name.contains("十字") && room_name.contains("西門") {
        vec![
            make_peninsula_object(
                "wt_trunk_road_milestone",
                "指路石碑",
                &["Look", "Read"],
                &[
                    ("Look", "十字街口立著一座四稜青石指路方碑，四面分別刻有通往西城門、公議堂與三水口的方向指引。"),
                    ("Read", "你辨讀指路石碑上的箭頭與工整刻字，城西主幹方位一目了然。"),
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
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
                ],
            ),
        ]
    } else if room_name.contains("西門") && !room_name.contains("草甸") && !room_name.contains("修補") {
        vec![
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
                "wt_obj_hitch_wood_fence",
                "拴馬長木欄",
                &["Look", "Use"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供長途駝隊歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
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
        ]
    } else if room_name.contains('橋') || room_name.contains("跳石") || room_name.contains("渡埠") {
        vec![
            make_peninsula_object(
                "wt_bridge_stepping_stones",
                "過溪跳石",
                &["Look", "Use", "Rest"],
                &[
                    ("Look", "平緩清淺水面上鋪設著一連串圓整青石跳石，旁側設有防滑青石護欄與繫船鐵環，清流自石隙潺潺穿過，映著水光。"),
                    ("Use", "你踩著露出水面的青石跳石穩健跨越清溪，腳下清流潺潺，水花濺在石階邊緣，平添幾分渡水之趣。"),
                    ("Rest", "你在跳石橋畔微涼的青石基座上歇坐片刻，聆聽溪水淙淙與微風拂過岸柳之聲，頓覺神清氣爽。"),
                ],
            ),
            make_peninsula_object(
                "wt_street_stone_bench",
                "歇腳石凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
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
    } else if room_name.contains("河灣") || room_name.contains("東灣") || room_name.contains("水脈") || room_name.contains("北渠") || room_name.contains("草坪") {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_riverbank",
                "橋欄",
                &["Look", "Use", "Rest", "Operate"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自石階下緩緩匯入水灣，波光瀲灩。"),
                    ("Use", "你走近青石欄杆探看繫船環與泊位，粗壯鐵環深嵌石中，吃水牢靠平穩。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往竹筏，水風徐來，心曠神怡。"),
                    ("Operate", "你輕撫石欄上斑駁溫潤的浪花雕飾，指尖微涼，遠望水天相映。"),
                ],
            ),
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
    } else if room_name.contains("阡陌水田")
        || room_name.contains("平整旱田")
        || room_name.contains("整地空壟")
        || room_name.contains("育苗菜畦")
        || room_name.contains("曬穀石坪")
        || room_name.contains("打穀圓場")
        || room_name.contains("穀倉前坪")
        || room_name.contains("農具棚架")
    {
        vec![
            make_peninsula_object(
                "wt_obj_waterwheel_wooden",
                "引水木水車",
                &["Look", "Operate"],
                &[
                    ("Look", "水渠支流上架著一座緩緩轉動的杉木筒車，水斗依次汲水傾入竹筧，灌溉連綿農田。"),
                    ("Operate", "你注視著水車齒輪帶動軸心勻速運轉，水流滔滔湧向田壟，聲響沉穩綿長。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_grain_drying_mat",
                "竹編曬穀席",
                &["Look", "Use"],
                &[
                    ("Look", "寬敞石坪上鋪展著數張細篾編織的巨幅曬穀席，金黃的稻穀薄薄鋪開，在日光下散發純淨穀香。"),
                    ("Use", "你手持木耙將曬席上的穀粒輕輕翻勻，穀粒沙沙作響，受熱均勻。"),
                ],
            ),
            make_peninsula_object(
                "wt_farm_tool_iron_plow",
                "雙齒精鐵犁",
                &["Look", "Operate", "PressCheese"],
                &[
                    ("Look", "為原牛翻耕黏土設計的淬火精鐵犁鏵，泥槽打磨平滑，犁尖銳利厚實。"),
                    ("Operate", "你握住雙齒犁的磨光硬木扶柄試著推動，重心穩健，精鐵犁鏵入土沉實。"),
                    ("PressCheese", "你協同牧耕勞作，將牧場原乳送入沉澱槽壓製乳酪，分理收成。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_flail_threshing_rack",
                "連枷打穀架",
                &["Look", "Operate"],
                &[
                    ("Look", "場院中央立著硬木搭起的傾斜打穀格架，架旁斜倚著幾柄以白蠟木製成的四節打穀連枷。"),
                    ("Operate", "你提起連枷凌空揮轉，梢竹劃過呼呼風聲，落下沉實有力的擊穀節奏。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_grain_hopper",
                "杉木儲糧斗",
                &["Look", "Open", "MillGrain"],
                &[
                    ("Look", "高懸的倒梯形杉木儲料斗連接著下料槽，旁邊架著點驗原糧重量的大秤。"),
                    ("Open", "你拉開儲料斗下方的調節木插板，金黃飽滿的原糧穀粒順槽嘩啦滑落。"),
                    ("MillGrain", "你啟開杉木料斗插板，金色稻穀源源湧入磨眼，水輪轟鳴研磨出新米漿。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_field_rest_stone",
                "田頭歇腳石",
                &["Look", "Rest"],
                &[
                    ("Look", "田埂交匯的大樟樹蔭下，立著一方天然平整的灰白大條石，石面已被農人磨得平滑微光。"),
                    ("Rest", "你在田頭大石上席地而坐，望著眼前綠意翻滾的無垠田壟，微風拂面神清氣爽。"),
                ],
            ),
        ]
    } else if room_name.contains("分畦藥圃")
        || room_name.contains("高山藥畦")
        || room_name.contains("山草藥畦")
        || room_name.contains("水草藥畦")
        || room_name.contains("防瘴艾圃")
        || room_name.contains("避風花圃")
        || room_name.contains("耐風草圃")
        || room_name.contains("曬藥竹場")
    {
        vec![
            make_peninsula_object(
                "wt_reg_wenyao_garden",
                "竹骨引水筧",
                &["Look", "Use", "Smell", "Water"],
                &[
                    ("Look", "依水岸而闢的分畦藥圃內，黑土整飭如墨，種著山風草、紫蘇與清涼薄荷，上方架設著以破半青竹搭建的竹骨引水筧，引清泉潺潺注入畦間。"),
                    ("Use", "你撥動引水筧竹筒引水澆灌畦間藥草，清泉滲入泥土，泥土草木清香頓時撲面而來。"),
                    ("Smell", "深吸一口氣，混雜著濕潤泥土、新鮮山風草與清涼薄荷的草木清香沁人心脾。"),
                    ("Water", "你撥正竹骨引水筧的水流轉向，清涼泉水順著竹管注入畦間泥土，滋養著青翠幼苗。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_herb_drying_trellis",
                "曬藥竹篾架",
                &["Look", "Turn"],
                &[
                    ("Look", "露天開闊處架著雙層竹篾編製的曬藥架，上面攤鋪著切片烘曬的當歸、黃耆與野生甘草。"),
                    ("Turn", "你輕輕翻動竹篩中的乾燥藥材切片，受光均勻，空氣中飄散著濃郁醇厚的甘草微甜。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_herb_stone_scoop",
                "青石澆水瓢",
                &["Look", "Use"],
                &[
                    ("Look", "蓄水池旁擱著一只以整塊青石刳空雕鑿的水瓢，木柄穿梢牢靠，瓢身沉實盛水不漏。"),
                    ("Use", "你舀起滿瓢清涼泉水，細細淋灑在翠綠的幼嫩藥苗根部，泥土芬芳撲鼻。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_windbreak_low_wall",
                "防風石矮牆",
                &["Look", "Rest"],
                &[
                    ("Look", "藥圃迎風面以天然山石乾砌起半人高的避風短牆，縫隙以泥草填塞，護佑幼嫩藥草免遭狂風侵襲。"),
                    ("Rest", "你背靠避風石牆坐下暫歇，牆背微溫避風，身前花草清香繚繞。"),
                ],
            ),
        ]
    } else if room_name.contains("中央牧坪")
        || room_name.contains("向陽牧坪")
        || room_name.contains("苜蓿草場")
        || room_name.contains("草料堆場")
        || room_name.contains("羊毛曬場")
        || room_name.contains("牧坪中道")
        || room_name.contains("環牧石板道")
        || room_name.contains("牧村出入口道")
    {
        vec![
            make_peninsula_object(
                "wt_caravan_water_trough",
                "青石飲馬槽",
                &["Look", "Use"],
                &[
                    ("Look", "牧道旁橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供群牛群羊飲用。"),
                    ("Use", "你伸手輕撩清涼槽水洗濯雙手，泉水甘冽澄澈。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_hitch_wood_fence",
                "拴馬長木欄",
                &["Look", "Use"],
                &[
                    ("Look", "粗大圓木架設的長木欄杆，專供放牧歇鞍時拴繫馱獸，木紋已被韁繩磨得光亮。"),
                    ("Use", "你將隨身繩索繞扣在堅固的原木欄杆上，木欄沉穩受力，極其吃勁。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_dry_haystack",
                "防潮乾草垛",
                &["Look", "Check"],
                &[
                    ("Look", "草場地勢隆起處高高堆起覆有油布防雨氈的乾牧草大垛，金黃乾燥的甜草散發清甜麥草香。"),
                    ("Check", "你抽出一根乾草掐斷驗看，草梗清脆乾燥，無半分返潮黴味。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_wool_drying_rack",
                "晾毛木排架",
                &["Look", "Check"],
                &[
                    ("Look", "木欄旁排立著傾斜的松木晾架，洗淨的原牛與原羊白毛攤晾其上，隨風微微蓬鬆翻動。"),
                    ("Check", "你伸手輕撫晾曬的羊毛，手感柔軟蓬鬆，油脂已被泉水漂洗得恰到好處。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_pasture_signpost",
                "牧場指路樁",
                &["Look", "Read"],
                &[
                    ("Look", "草道岔路口立著一根去皮紅松指路方樁，樁頂刻有奔牛與羔羊標記，指明各牧區與飲水塘方向。"),
                    ("Read", "你閱讀指路樁上的清晰刻槽，向陽草場與儲料庫方位一目了然。"),
                ],
            ),
        ]
    } else if room_name.contains("曬網石坪")
        || room_name.contains("錨鏈儲場")
        || room_name.contains("船木晾場")
        || room_name.contains("貝灰曬坪")
        || room_name.contains("纜繩工棚")
        || room_name.contains("望海石欄")
        || room_name.contains("卸貨石坪")
        || room_name.contains("海風茶亭")
        || room_name.contains("海鹽曬場")
        || room_name.contains("船帆晾架")
        || room_name.contains("錨石堆場")
        || room_name.contains("礁石望台")
        || room_name.contains("燈號石坪")
        || room_name.contains("礁石淺灘")
        || room_name.contains("巨藻晾場")
        || room_name.contains("風化礁岩")
        || room_name.contains("避風街")
        || room_name.contains("觀海橫街")
        || room_name.contains("烏石街")
        || room_name.contains("海港直街")
    {
        vec![
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
                "wt_obj_basalt_heavy_anchor",
                "玄武重鐵錨",
                &["Look", "Check"],
                &[
                    ("Look", "海灘石坪上斜臥著一具以玄武岩壓心、四爪生鐵鍛鑄的巨型防風鐵錨，錨爪粗壯如柱。"),
                    ("Check", "你伸手敲擊厚重的鐵錨身，發出沉實渾厚的嗡鳴，抓底分量十足。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_spruce_drying_rack",
                "雲杉晾木架",
                &["Look", "Check"],
                &[
                    ("Look", "岸邊通風排棚下，整齊架著用粗雲杉搭建的通風層架，架上鋪滿裁切整齊的造船龍骨板材。"),
                    ("Check", "你查驗架上木材的乾燥色澤，木理通直緊密，海水鹽霧更添硬度。"),
                ],
            ),
            make_peninsula_object(
                "wt_lime_powder_bin",
                "熟貝灰石槽",
                &["Look", "Use"],
                &[
                    ("Look", "大石槽中盛滿過篩後的熟貝灰粉，細膩如白麵，遇水化作黏結力極強的砌石砂漿。"),
                    ("Use", "你拿起木鏟翻了翻熟貝灰粉，粉末乾燥滑膩，是海防築堤與船底防漏之上品。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_rope_twist_wheel",
                "手搖絞繩輪",
                &["Look", "Operate"],
                &[
                    ("Look", "空地上立著三聯手搖木製絞繩輪，麻絲自輪齒穿出絞合為粗壯縴繩，旁側立有刻度拉力測試石柱。"),
                    ("Operate", "你伸手搖動木輪握把，齒輪牽動繩股絞緊，木架嗡嗡震響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_seawall_stone_railing",
                "望海青石欄",
                &["Look", "Rest"],
                &[
                    ("Look", "海岬高台邊緣修築著連綿厚實的防風浪青石圍欄，石面受海風雕琢呈現溫潤石痕。"),
                    ("Rest", "你扶欄憑高遠眺，碧海奔潮拍擊黑石礁崖，白浪滔天令人胸襟大開。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sea_salt_crystallizer",
                "海鹽結晶池",
                &["Look", "Check"],
                &[
                    ("Look", "平整海坪上以青石條分割成一方方淺水灰漿池，清澈鹵水在烈日下蒸發，泛起耀眼的雪白鹽霜。"),
                    ("Check", "你拈起一撮池畔凝結的粗鹽晶，晶體顆粒方正透明，鹹鮮純正。"),
                ],
            ),
            make_peninsula_object(
                "wt_kelp_drying_trellis",
                "高架曬藻棚",
                &["Look", "Gather", "BurnLime"],
                &[
                    ("Look", "迎著海風搭建的巨型竹棚上懸垂著長達數丈的深海厚巨藻，海風吹拂下漸乾變韌。"),
                    ("BurnLime", "你將高架曬乾的堅韌海藻與貝灰一同收整，備齊風舶角搓繩與造船用料。"),
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
        ]
    } else if room_name.contains("寒水玉採石坑")
        || room_name.contains("切石石坪")
        || room_name.contains("採石坡道")
        || room_name.contains("晾石前坪")
        || room_name.contains("玄武採石場")
        || room_name.contains("礦道石坪")
        || room_name.contains("西窯道")
        || room_name.contains("東選礦道")
        || room_name.contains("紅土磚場")
        || room_name.contains("選礦石坪")
        || room_name.contains("礦車儲坪")
        || room_name.contains("耐火泥池")
        || room_name.contains("耐火磚場")
        || room_name.contains("礦車鐵軌道")
        || room_name.contains("坑口中街")
    {
        vec![
            make_peninsula_object(
                "wt_obj_stone_gang_saw",
                "切石框鋸台",
                &["Look", "Touch"],
                &[
                    ("Look", "粗壯鐵架上懸掛著加重無齒金剛砂框鋸，細水伴著鋼砂在巨石槽中慢慢切割，四周堆滿了平整如削的城防條石與石雕柱礎。"),
                    ("Touch", "你伸手撫摸新切出的石板切面，石面平直如削，微帶金剛砂水漬的溫涼觸感。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_jade_material_rack",
                "寒水玉料架",
                &["Look", "Check"],
                &[
                    ("Look", "棚架內層層陳列著新開採出的半嶺寒水玉毛料，玉色青綠微透，石皮帶有一層細膩白霜。"),
                    ("Check", "你伸手摩挲玉料切面，入手冰寒徹骨，質地縝密溫潤。"),
                ],
            ),
            make_peninsula_object(
                "wt_trolley_rail_carriage",
                "雙軸重台車",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "坑道軌道上停著厚重雙軸台車，精鐵鑄造的車輪扣住木軌鐵皮，裝滿粗礦依然平穩。"),
                    ("Operate", "你推動台車制動鐵桿，生鐵閘瓦咬緊輪軸，發出沉實摩擦聲。"),
                    ("HaulMineCart", "你合力推動雙軸重台車滑入坑口鐵軌，伴隨鋼輪咬軌轟鳴，將滿車赤鐵重礦運出坑道。"),
                ],
            ),
            make_peninsula_object(
                "wt_mine_wash_chute",
                "木製洗礦槽",
                &["Look", "Operate", "HaulMineCart"],
                &[
                    ("Look", "山澗急流順著長木槽奔騰而下，槽底隔木截留住沉重赤鐵礦砂，洗去浮游紅泥。"),
                    ("Operate", "你拔起一片水閘木板，急流分注沖刷沉澱石泥，露出晶亮重礦砂。"),
                    ("HaulMineCart", "你引地下冷泉沖洗礦砂，分理碎石，將赤鐵重礦裝入滑車鐵斗。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_grade_iron_screen",
                "分級重鐵篩",
                &["Look", "Operate"],
                &[
                    ("Look", "堅固的三層傾斜編鐵粗篩，專供分揀不同粒徑的耐火黏土粒與礦砂。"),
                    ("Operate", "你手搖鐵篩連桿，不同規格的礦石砂粒依序分流滑入不同木斗。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_red_brick_drying_rack",
                "紅磚坯料架",
                &["Look", "Check"],
                &[
                    ("Look", "窯道兩側搭滿長條木架，密密麻麻排滿了脫模陰乾的紅土耐火泥磚坯，棱角分明。"),
                    ("Check", "你抽查泥磚坯的乾燥堅實度，黏土調配得當，磚身毫無裂紋變形。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_steel_cable_winch",
                "鋼索絞盤",
                &["Look", "Operate"],
                &[
                    ("Look", "採石斜坡頂端固定著厚重的生鐵鑄造絞盤，多股鋼絲絞索緊密盤繞於轉軸，備有防倒退棘輪。"),
                    ("Operate", "你扳動絞盤長柄試運轉，棘輪發出清脆卡嗒聲，制動齒輪咬合緊密。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_mine_tunnel_lantern",
                "礦道照明燈",
                &["Look", "Operate"],
                &[
                    ("Look", "石壁鐵托架上嵌著以密閉青銅網罩保護的夜光貝母礦燈，在幽深昏暗的石坪投下清亮冷光。"),
                    ("Operate", "你調整礦燈前端的遮光銅葉，聚光弧束指向前路，幽深坑道頓時清晰。"),
                ],
            ),
        ]
    } else if room_name.contains("山泉清槽")
        || room_name.contains("柴薪儲場")
        || room_name.contains("觀雲石台")
        || room_name.contains("試招石坪")
        || room_name.contains("儲木石坪")
        || room_name.contains("雲杉苗圃")
        || room_name.contains("雲杉林圃")
        || room_name.contains("護林松坡")
        || room_name.contains("嶺腳主徑")
        || room_name.contains("橫嶺山徑")
        || room_name.contains("杉木西巷")
        || room_name.contains("霜林中道")
        || room_name.contains("鋸木溪前街")
        || room_name.contains("杉林北道")
    {
        vec![
            make_peninsula_object(
                "wt_obj_timber_reed_stacks",
                "毛竹原木垛",
                &["Look", "Check", "Tap"],
                &[
                    ("Look", "高大通風的倉棚下整齊碼放著以油布覆蓋的防潮糧穀包，場地另一側堆滿通直挺拔的三年生毛竹與直徑數尺的巨杉圓木。"),
                    ("Check", "你抽驗木料質地，木質密實乾燥，松香氣息撲鼻。"),
                    ("Tap", "你輕叩粗壯的原木端面，聲如洪鐘，質地堅韌無蛀。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_hydraulic_saw",
                "水動排鋸架",
                &["Look", "Operate", "RefinePlank"],
                &[
                    ("Look", "巨型雙水車連桿驅動的直立鋼鋸架，上下往復飛速鋸切，原木如切泥般剖為平整厚木板。"),
                    ("Operate", "你扳動水車分水導流板，水流激增，排鋸隨之發出均勻剛勁的轟鳴切削聲。"),
                    ("RefinePlank", "你推動水動排鋸咬合雲杉巨木，精鋼鋸齒飛速剖切，裁出筆直平整的樟松木板料。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_mountain_milestone",
                "山道指路碑",
                &["Look", "Read", "HangVotivePlaque"],
                &[
                    ("Look", "陡坡拐角處矗立著一尊取自青脊山的灰黑巨石碑，正面深刻著通往白塔雪巔與各林場的箭頭路徑。"),
                    ("Read", "你仔細端詳碑文上的山勢地形指引，峰迴路轉的險要坡道清晰在目。"),
                    ("HangVotivePlaque", "你在散發松脂清香的北嶺雲杉木牌上題寫平安祈願，繫於山道古杉牌架之上，木牌隨山風清脆作響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_spring_water_trough",
                "引泉青石槽",
                &["Look", "Use"],
                &[
                    ("Look", "山徑旁依山岩鑿有一方青石泉槽，山頂清澈甘泉順竹筧潺潺注入，溢流順石階漫入草叢。"),
                    ("Use", "你俯身掬起清泉痛飲，泉水冰寒甘洌直透心脾，登山疲渴頓時冰消。"),
                ],
            ),
            make_peninsula_object(
                "wt_arena_testing_pillar",
                "玄武試勁柱",
                &["Look", "Operate", "SparPillar"],
                &[
                    ("Look", "場中央立著取自烏石鼻的沉重玄武岩石柱，柱面留有深淺不一的掌印、拳痕與兵刃劈砍印記。"),
                    ("Operate", "你凝神運勁一掌拍上試勁石柱，掌風在石面上激起沉悶回響，柱身厚重不搖。"),
                    ("SparPillar", "你沉腰落馬繞玄武試勁柱演練四序拆招，拳掌與兵刃交擊石柱發出沉雄悶響。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_sheltered_rest_stone",
                "避風歇腳石",
                &["Look", "Rest"],
                &[
                    ("Look", "巨岩凹處背風向陽的一方天然青石台，石面寬厚平整，四周崖壁恰好擋住高嶺寒風。"),
                    ("Rest", "你在石台上盤膝打坐調勻呼吸，背風處暖意微生，漸復體力。"),
                ],
            ),
        ]
    } else if room_name.contains("高棧木廊大道")
        || room_name.contains("北防務石街")
        || room_name.contains("哨鎮東西大街")
        || room_name.contains("棧道巡道")
        || room_name.contains("高架木坪")
        || room_name.contains("獵具檢修坪")
        || room_name.contains("篝火石坪")
        || room_name.contains("獸皮晾架")
        || room_name.contains("巡哨前坪")
        || room_name.contains("南哨沼口門")
        || room_name.contains("哨口木道")
        || room_name.contains("高腳烏木棧橋")
        || room_name.contains("水生木棧道")
        || room_name.contains("蘆花北棧")
        || room_name.contains("澤南橫棧")
        || room_name.contains("蘆葦淺澤")
        || room_name.contains("菱角淺塘")
    {
        vec![
            make_peninsula_object(
                "wt_obj_wormwood_dispelling_pot",
                "防瘴艾草盆",
                &["Look", "Smell"],
                &[
                    ("Look", "木棧道轉角處置有一只厚泥陶盆，盆內文火燃點著乾燥陳艾與雄黃香餅，青煙裊裊升騰。"),
                    ("Smell", "你輕嗅空氣中散發的濃烈艾草苦香，周遭蚊蚋退避，胸口濁氣為之一清。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_ebony_mooring_post",
                "烏木繫船樁",
                &["Look", "Use"],
                &[
                    ("Look", "木棧橋頭深打著數根油黑不腐的沼生千年烏木樁，樁頭刻有水鳥標記，套著浸油防黴纜繩。"),
                    ("Use", "你伸手推了推黑亮沉重的烏木樁，木質堅逾鐵石，吃水極深牢不可撼。"),
                ],
            ),
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
                "wt_bamboo_drying_rack",
                "青竹晾曬架",
                &["Look", "Gather"],
                &[
                    ("Look", "大棚架上整齊排晾著經過熱水蒸煮去糖防蛀的老青竹竿，竹皮碧綠油亮，質地極韌。"),
                    ("Gather", "你撫摸晾乾的青竹竿，竹節筆直勻稱，正是編造水排與造屋之良材。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_iron_bound_barricade",
                "包鐵拒馬欄",
                &["Look", "Check"],
                &[
                    ("Look", "哨口木道中央橫設著以重硬木拼組、四角包釘尖刺鐵皮的雙層拒馬防禦柵欄。"),
                    ("Check", "你審查拒馬欄上的生鐵加固鉚釘與交叉木柱，佈局堅固，能阻猛獸直衝。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_tall_watchtower_ladder",
                "高架瞭望梯",
                &["Look", "Climb"],
                &[
                    ("Look", "高棧坪旁立著一座直通樹冠瞭望台的粗大木梯，階梯踏板包覆防滑麻棕，扶手粗壯結實。"),
                    ("Climb", "你攀上木梯數級放眼環顧，沼澤深處的水網航道與蘆葦蕩漾盡收眼底。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_water_caltrop_skiff",
                "採菱木扁舟",
                &["Look", "Operate"],
                &[
                    ("Look", "棧道下繫著一艘吃水極淺的圓頭杉木小划子，艙內備有採菱竹撈網與幾隻防浸圓木桶。"),
                    ("Operate", "你踩上輕晃的扁舟以竹篙微點棧柱，扁舟輕巧滑入淺澤荷菱之間，游動自如。"),
                ],
            ),
        ]
    } else if room_name.contains("實心城牆")
        || room_name.contains("木柵圍欄")
        || room_name.contains("海防石柵")
        || room_name.contains("山隘石柵")
        || room_name.contains("防木重柵")
    {
        vec![
            make_peninsula_object(
                "wt_obj_stone_crenel_battlement",
                "條石垛口",
                &["Look", "Check"],
                &[
                    ("Look", "城牆外緣整齊排列著厚重規整的玄武岩條石垛口，留有向下俯瞰與開弓射箭的防禦缺口。"),
                    ("Check", "你伏在冰涼的條石垛口向外探視，牆外平原野地一覽無遺，扼守要道防守森嚴。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_patrol_tower_stair",
                "巡防望樓梯",
                &["Look", "Climb"],
                &[
                    ("Look", "依實心石牆內側修築的整齊青石台階直通敵樓高台，階石寬闊堅實，附設鍛鐵扶手。"),
                    ("Climb", "你拾級登梯登上石牆高處，登高憑風，城關防務格局嚴密了然。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_windproof_iron_brazier",
                "防風鐵火盆",
                &["Look", "Operate"],
                &[
                    ("Look", "城垛轉角處置有一座三角鍛鐵厚重火盆，盆身鏤刻避風雲紋，內置燃燒持久的硬木白炭。"),
                    ("Operate", "你用鐵火鉗撥動炭塊，暗紅炭火立刻迸出熾烈微芒，驅散城頭凜冽寒意。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_hardwood_barrier_stake",
                "硬木拒馬樁",
                &["Look", "Check"],
                &[
                    ("Look", "關隘通道兩側斜立著以合抱粗原木削尖、深嵌地面的防衝撞拒馬尖樁，木尖塗覆防腐黑漆。"),
                    ("Check", "你檢視拒馬樁基座的鎖固鐵鏈，排樁緊湊密集，形成阻遏大軍衝鋒的銅牆鐵壁。"),
                ],
            ),
        ]
    } else if (room_name.starts_with("文曜") && room_name.contains("三水")) || room_name.contains("南門") || room_name.contains("納涼") || room_name.contains("市集") || room_name.contains("早市") || room_name.contains("夜市") {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
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
    } else if room_name.contains("市街石道")
        || room_name.contains("鎮口道")
        || room_name.contains("市鎮街道")
        || room_name.contains("村中泥徑")
        || room_name.contains("村口路")
        || (is_peninsula
            && (room_name.contains('街')
                || room_name.contains('道')
                || room_name.contains('橋')
                || room_name.contains('路')
                || room_name.contains('徑')
                || room_name.contains('門')
                || room_name.contains('牆')
                || room_name.contains('欄')
                || room_name.contains('田')
                || room_name.contains('壟')
                || room_name.contains('圃')))
    {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
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
    } else if is_peninsula {
        vec![
            make_peninsula_object(
                "wt_street_stone_bench",
                "青石長凳",
                &["Look", "Rest"],
                &[
                    ("Look", "平整石道旁設有取自半嶺的青石長凳，已被往來行人的衣角磨得微亮，是歇腳閒談的絕佳所在。"),
                    ("Rest", "你在微涼溫潤的石凳上坐下歇息，看著街巷間往來穿梭的鄰里行客，步調悠閒自在。"),
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

#[allow(unused_variables)]
fn peninsula_objects_chunk_2(room_name: &str) -> Option<Vec<RoomObject>> {
    let is_peninsula = is_white_tower_peninsula_room(room_name);
    let is_outdoor_road = room_name.contains('街')
        || room_name.contains('道')
        || room_name.contains('巷')
        || (room_name.contains('橋') && !room_name.contains("旅社") && !room_name.contains("茶屋"))
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

    let objs = if room_name.contains("駝隊大驛站") || room_name.contains("駝馬大驛館") || room_name.contains("客商大旅社") {
        vec![
            make_peninsula_object(
                "wt_obj_caravan_hitch_trough",
                "拴馬長木欄",
                &["Look", "Feed", "Inspect", "FeedDraftBeast"],
                &[
                    ("Look", "寬闊的露天驛場四周立著一人多高的結實榆木栓馬欄，兩排長條青石水槽盛滿自來活水，長槽中堆著混合豆餅的切細苜蓿草。"),
                    ("Feed", "你抓起一把清香甘甜的豆餅乾草餵給槽邊的原牛駝獸，駝獸溫順地咀嚼，眼眸溫和。"),
                    ("Inspect", "你檢驗木欄上的韁繩繫扣，粗大牢實，能承受數十頭大牲口同時拉扯。"),
                    ("FeedDraftBeast", "你抱來新鮮苜蓿草料與甘冽山泉水倒入槽中，替遠行歇腳的健壯駝獸刷拭背毛、添草飲水。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_caravan_bell_rack",
                "鍛銅駝鈴架",
                &["Look", "Shake"],
                &[
                    ("Look", "廊下懸掛著成串音階各異的鍛銅駝鈴，長凳上擺著塗抹牛油熟化、防磨厚實的羊毛鞍墊。"),
                    ("Shake", "你輕搖銅駝鈴串，渾厚悠揚的鈴聲在開闊驛站迴盪，傳出數里之遙。"),
                ],
            ),
        ]
    } else if room_name.contains("皮毛大市") || room_name.contains("皮毛大貨棧") {
        vec![
            make_peninsula_object(
                "wt_obj_fur_baling_press",
                "生皮壓捆機",
                &["Look", "Touch", "Weigh"],
                &[
                    ("Look", "開闊的棚場裡立著重型螺旋木製壓捆機，數十張整理平整的厚牛皮被牢牢壓成緊實貨立方，上方橫樑懸掛著能稱數千斤的重鐵大桿秤。"),
                    ("Touch", "你伸手摩挲熟化好的優等原牛皮，皮板柔軟厚重，毛色油潤光潔。"),
                    ("Weigh", "你推動大桿秤秤砣，秤桿平穩懸浮，分量十足。"),
                ],
            ),
        ]
    } else if (room_name.contains("糧草") && room_name.contains("倉")) || room_name.contains("中轉站") {
        vec![
            make_peninsula_object(
                "wt_obj_ration_crate_stacks",
                "防潮糧草箱",
                &["Look", "Inspect"],
                &[
                    ("Look", "厚條石修築的防潮高台庫房內，整齊堆滿了塗有桐油的防雨樟木大箱，牆邊以十字交錯法碼放著壓成硬塊的高營養乾草飼料磚。"),
                    ("Inspect", "你撬開一只樟木箱封角查看，內裡防潮油紙完好，儲存的炒麥麵散發著烘烤麥香。"),
                ],
            ),
        ]
    } else if room_name.contains("大車坊") || room_name.contains("修造場") {
        vec![
            make_peninsula_object(
                "wt_obj_wagon_jack_wheel",
                "修車頂升機",
                &["Look", "Operate", "Inspect"],
                &[
                    ("Look", "工場中央以重型木質螺桿千斤頂將一輛雙輪大車懸空支起，修車匠正以大鐵錘敲擊校正包鐵硬木輪輻，砧檯上放著淬火車軸。"),
                    ("Operate", "你握住千斤頂橫木搖柄轉動半圈，沉重的車架穩步抬升，機構沉穩咬合。"),
                    ("Inspect", "你彎腰細看重型車輪的包鐵厚度，足有半指之厚，足以在荒原碎石路輾壓千里。"),
                ],
            ),
        ]
    } else if room_name.contains("拍賣行") || room_name.contains("牲畜") {
        vec![
            make_peninsula_object(
                "wt_obj_livestock_auction_ring",
                "牲口競價欄",
                &["Look", "Watch"],
                &[
                    ("Look", "環形露天木圍欄中央是平整開闊的展示沙地，四周設有階梯看台，一頭毛色鋥亮的大角原牛正由牽引繩引導繞場踱步展示體格。"),
                    ("Watch", "圍欄周邊行商目光如炬，暗暗比劃手指商定起拍叫價。"),
                ],
            ),
        ]
    } else if room_name.contains("獸醫") {
        vec![
            make_peninsula_object(
                "wt_obj_veterinary_box",
                "修蹄木架",
                &["Look", "Inspect"],
                &[
                    ("Look", "藥鋪內設有固定牲口蹄足的木架，案上擺滿了特大號除石鐵鉤、切蹄刀與銅烙鐵，旁邊石臼裡盛滿了以蜂蜜調和的健胃消脹大草藥丸。"),
                    ("Inspect", "你拿起一把沉重光潔的切蹄刀試了試刃口，弧度合手，利於修整駝獸硬蹄。"),
                ],
            ),
        ]
    } else if room_name.contains("生鐵庫") || room_name.contains("粗鹽") {
        vec![
            make_peninsula_object(
                "wt_obj_iron_salt_stacks",
                "生鐵錠垛",
                &["Look", "Tap"],
                &[
                    ("Look", "高敞乾燥的石庫內，重達數十斤一塊的梯形生鐵錠像磚牆般整齊交錯碼放，另一側防潮木架上整齊堆著密封嚴密的青鹽大包。"),
                    ("Tap", "你手指扣擊底層生鐵錠，鏗鏘作響，質地堅密純淨。"),
                ],
            ),
        ]
    } else if room_name.contains("武備") || room_name.contains("護運行") {
        vec![
            make_peninsula_object(
                "wt_obj_weapon_rack_stone",
                "護行兵器架",
                &["Look", "Sharpen", "Inspect"],
                &[
                    ("Look", "堅實木架上整齊插放著防身硬木長棍、包銅重皮盾與開山厚背獵刀，長凳上擺著盛滿菜籽油的細砂油石槽。"),
                    ("Sharpen", "你抽出一把厚背獵刀在油石槽上緩緩推磨，刀刃在油光映射下泛出冷冽寒光。"),
                    ("Inspect", "你端起包銅硬皮盾審視，盾面以生牛皮三重疊壓包覆，能有效抵擋荒原野獸撕咬。"),
                ],
            ),
        ]
    } else if room_name.contains("車行") {
        vec![
            make_peninsula_object(
                "wt_obj_weighbridge_milepost",
                "西道里程碑",
                &["Look", "Read"],
                &[
                    ("Look", "鋪面門前青石地嵌著一方重型過磅鐵台，側立一尊高大花崗岩石碑，鐫刻著通往長風草甸、老堰頭與大圳頭的準確里程與地勢險隘。"),
                    ("Read", "你俯身端讀里程路石刻字：西出城門五里為平川，十里有避風坡與取水深井，標記周詳。"),
                ],
            ),
        ]
    } else if room_name.contains("羊肉湯") || room_name.contains("酒坊") {
        vec![
            make_peninsula_object(
                "wt_obj_mutton_broth_pot",
                "羊肉大湯鍋",
                &["Look", "Taste", "Smell"],
                &[
                    ("Look", "粗麻避風棚下擺滿了寬厚木桌長凳，大灶上一口深腹鐵鍋翻滾著奶白色的原羊骨高湯，大蔥花與白胡椒粉香氣四溢。"),
                    ("Taste", "你端起粗瓷碗喝了一大口濃滾羊湯，湯鮮味濃，胡椒辛香熱辣直透四肢百骸。"),
                    ("Smell", "羊肉脂肪甘香混著麵香在空氣中熱烈翻滾，令人饞涎欲滴。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && room_name.contains("食堂") {
        vec![
            make_peninsula_object(
                "wt_dining_rice_milk_stall",
                "溫米漿桶",
                &["Look", "Taste", "AssistKitchen"],
                &[
                    ("Look", "老堰頭水磨溫米漿散發著醇厚米香，大杉木桶裡盛著風舶苦滷點製的熱豆花，攤旁蒸籠疊著熱騰出鍋的蔥香米烙餅與牛埔鮮肉包。"),
                    ("Taste", "你啜飲了一口熱米漿，濃郁米香混著微甘暖透入胃，豆花滑嫩微燙，帶著純粹的黃豆清香。"),
                    ("AssistKitchen", "你挽起衣袖在米漿桶與蒸籠旁幫廚備膳，分裝熱騰騰的米漿與原牛肉湯。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_iron_cauldron",
                "紅鐵燉肉釜",
                &["Look", "Taste", "WarmHands", "SeasonalBanquet", "AssistKitchen"],
                &[
                    ("Look", "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅鴛鴦鍋滾水翻騰，備有切仔湯麵配滷蛋與大圳頭溫米酒。"),
                    ("Taste", "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。"),
                    ("WarmHands", "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。"),
                    ("SeasonalBanquet", "你在紅鐵釜前添薪圍坐，鍋內歲時時令熱氣翻騰，同席之人舉杯共飲，暖意直透肺腑。"),
                    ("AssistKitchen", "你持長柄木杓在紅鐵燉肉釜前幫廚添薪撇沫，慢火細燉大角原牛肉湯。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_cold_jade_cabinet",
                "冰鑑櫃",
                &["Look", "Open"],
                &[
                    ("Look", "雙層松香木隔熱櫃內壁鑲滿北山寒水玉薄板，櫃身透著天然的清涼寒氣，冰鎮著板豆腐、鮮牛乳與寒水玉冰鎮仙草凍。"),
                    ("Open", "你拉開沉沉的櫃門，一股幽涼白霧迎面而來，裡面整齊碼放著白瓷罐裝的鮮牛乳與浸在井泉裡的涼仙草凍。"),
                ],
            ),
            make_peninsula_object(
                "wt_dining_sauce_rack",
                "醬料陶缸架",
                &["Look", "Smell"],
                &[
                    ("Look", "竹木架上整齊排著大圳頭壺底黑豆醬油、純米醋、脆瓜醬菜、豆腐乳與風櫃尾海味提鮮粉，陶缸口皆封著潔淨的白綿布蓋。"),
                    ("Smell", "你掀開黑豆醬油缸蓋深嗅，古法日曬釀造的濃醇豆香中透著微甘，令人胃口大開。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("旅社") || room_name.contains("茶")) {
        vec![
            make_peninsula_object(
                "wt_inn_milk_tea_stove",
                "煮茶泥爐",
                &["Look", "Taste", "RoastMountainTea"],
                &[
                    ("Look", "銅皮泥爐上坐著一把長嘴鐵壺，穩火慢煮著霜林烘青茶與牛埔鮮乳交融的鍋煮鮮奶茶；竹盤裡盛著冰鎮甘蔗檸檬汁，配黑糖米香、奶油酥餅與鹽烤海苔片。"),
                    ("Taste", "你捧起陶杯細品熱奶茶，醇厚乳香托著烘青茶的微澀回甘，甘蔗檸檬的酸甜則沁人心脾。"),
                    ("RoastMountainTea", "你在銅皮泥爐前文火翻焙霜林烘青茶葉，茶香清冽四溢，與同席茶客共品回甘。"),
                ],
            ),
            make_peninsula_object(
                "wt_inn_ebony_shell_chess",
                "烏木棋盤",
                &["Look", "Operate", "PlayChess"],
                &[
                    ("Look", "黑沼沉水烏木雕琢的棋盤線道深沉分明，兩只棋罐各盛著溫潤厚重的烏木黑子與烏石鼻厚海貝白子，茶客對弈殺局難解難分。"),
                    ("Operate", "你拈起一枚沉甸甸的白貝棋子扣在棋盤交叉處，發出清脆沉實的篤響，盤中局勢頓時生出新意。"),
                    ("PlayChess", "你端坐於烏木棋盤前拈子佈局，黑白交錯推演天地之數，心神凝定，雜念俱泯。"),
                ],
            ),
            make_peninsula_object(
                "wt_inn_gazette_stand",
                "旬報木報架",
                &["Look", "Read"],
                &[
                    ("Look", "松木斜面報架上夾著講堂街印行館竹綿紙印發的《半島旬報》，底層木格裡收著幾本供人消遣的白話故事小冊。"),
                    ("Read", "你翻開《半島旬報》，頭版記載著老堰頭水利翻修進展、風舶深水碼頭貨船期程，副刊刊著大圳頭秋收米價與林地軼聞。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_feather_bed_lamp",
                "客房羽絨榻",
                &["Look", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "獨立銅鎖客房內鋪著蓬鬆羽絨暖被、細編蘆葦涼席與山風草安神枕，床頭立著一盞旋蓋夜光貝母調光檯燈。"),
                    ("Rest", "你合衣躺上床榻，蓬鬆柔軟的羽絨與安神草清香迅速舒緩了周身筋骨，睡意悄然湧上。"),
                    ("Sleep", "你鑽入蓬鬆羽絨暖被，旋暗貝母燈，聽著窗外隱約微風沉沉入睡，醒來後體魄與精神盡復。"),
                    ("Operate", "你轉動床頭夜光貝母檯燈的烏木旋蓋，外露的柔和珠光自幽微漸轉明亮，將床邊照得一片溫馨澄澈。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("浴場") || room_name.contains("浴堂") || room_name.contains("石湯")) {
        vec![
            make_peninsula_object(
                "wt_bath_jade_pool",
                "恆溫熱湯池",
                &["Look", "Rest", "SoakHotSpring"],
                &[
                    ("Look", "寬闊的大石池以半嶺寒水玉鋪底，池壁嵌有岑氏恆溫導能火磚，池中浸泡著溫氏祛寒草木湯包，水色泛著微翠草露之潤。"),
                    ("Rest", "你浸入溫潤清透的熱湯中，草木藥氣滲透肌膚脈絡，周身毛孔舒張，奔波積累的疲乏頓時消散殆盡。"),
                    ("SoakHotSpring", "你浸入溫氏祛寒草木熱湯池中，溫潤泉水滲透四肢百骸，寒濕與疲勞盡數消散。"),
                ],
            ),
            make_peninsula_object(
                "wt_bath_mirror_washstand",
                "銀鏡盥洗台",
                &["Look", "Use", "GroomAppearance"],
                &[
                    ("Look", "光潔明亮的亮銀明水大鏡前，整齊備有軟毛竹柄潔齒刷、青鹽薄荷潔牙粉、山茶松脂香胰、蜂格棉面巾與松香木梳。"),
                    ("Use", "你取了潔牙粉與軟毛潔齒刷洗漱，青鹽薄荷的清涼瞬間沁透齒頰，再用山茶松脂香胰掬水洗面，容顏頓覺煥然一新。"),
                    ("GroomAppearance", "你敷上薄荷熱毛巾，由修面師傅以精鋼剃刀與松香木梳修整儀容，神彩煥然一新。"),
                ],
            ),
            make_peninsula_object(
                "wt_bath_pine_charcoal_dryer",
                "松炭烘衣櫥",
                &["Look", "DryOff"],
                &[
                    ("Look", "厚實的松香木大立櫥內裝多層橫木桿，底部設導能溫石與松炭烘槽，正散發著乾燥溫純的木質暖風。"),
                    ("DryOff", "你把微潮的衣物掛入櫥中關好木門，片刻後取出，衣物已被烘得乾爽蓬鬆，散發著好聞的松木陽光氣息。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("傢俱") || room_name.contains("巧木")) {
        vec![
            make_peninsula_object(
                "wt_wood_planing_bench",
                "刨木工作台",
                &["Look", "Operate", "FitMortiseTenon"],
                &[
                    ("Look", "厚重工匠台上架著導能刨刀、拉鋸與木鑿，台面落滿薄如蟬翼的松木刨花，木香撲鼻。"),
                    ("Operate", "你握起導能推刨在松木料上平推而過，木片應手捲起均勻刨花，木面光滑平整。"),
                    ("FitMortiseTenon", "你執起導能推刨與木鑿，依巧木坊墨線將沉水烏木與松木構件精準開榫合卯，嚴絲合縫。"),
                ],
            ),
            make_peninsula_object(
                "wt_wood_bed_and_cabinet",
                "松木彈力床",
                &["Look", "Sit", "Open"],
                &[
                    ("Look", "展示區陳設著繃好高彈棕網的松香木彈力大床，以及內嵌北山寒水玉薄板的保鮮石櫃。"),
                    ("Sit", "你在松香木大床上坐下試壓，棕繩床網承托沉穩富有回彈，坐感舒坦自然。"),
                    ("Open", "你拉開保鮮石櫃木門，夾層中的寒水玉散發清涼氣息，櫃格密合嚴整。"),
                ],
            ),
            make_peninsula_object(
                "wt_wood_display_shelf",
                "烏木茶盤几",
                &["Look", "Use"],
                &[
                    ("Look", "展示架上陳列著黑沼沉水烏木精雕茶盤、烏木筷、松香木梳與小巧置物木匣。"),
                    ("Use", "你端起沉水烏木茶盤撫摸，質地沉重溫潤，水漬不滲，顯出精湛木工手藝。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && (room_name.contains("服飾") || room_name.contains("織造") || room_name.contains("補給行")) {
        vec![
            make_peninsula_object(
                "wt_cloth_garment_rack",
                "防雨長衫架",
                &["Look", "Use", "DyeIndigoCloth"],
                &[
                    ("Look", "松木衣架上整齊掛著塗有黑沼樹膠的防雨長衫、透氣棉麻短衫與北山雪狼絨冬衣，做工紮實。"),
                    ("Use", "你取下一件防雨長衫披在身上試穿，衣料厚韌輕便，領口銅扣封得嚴實，能擋狂風急雨。"),
                    ("DyeIndigoCloth", "你取過素麻布料浸入後坊山藍染缸，晾乾後塗覆薄層熟桐油，製成防風抗雨的山藍斗篷。"),
                ],
            ),
            make_peninsula_object(
                "wt_cloth_bag_umbrella_counter",
                "帆布包傘櫃",
                &["Look", "Open"],
                &[
                    ("Look", "櫃檯陳列著雙扣帆布斜背包、軟牛皮零錢夾、竹骨晴雨摺疊傘、軟底居家布便鞋與隨手五色針線盒。"),
                    ("Open", "你拉開展示櫃門查看，隨手五色針線盒內整齊排著鋼針與彩線，斜背包分格實用耐磨。"),
                ],
            ),
            make_peninsula_object(
                "wt_cloth_tailor_sewing_bench",
                "裁縫量身几",
                &["Look", "Use", "MendTravelGear"],
                &[
                    ("Look", "量身木几上擺著軟皮尺、裁布銅剪與小巧便攜的隨手五色針線盒，方便隨時度量修改身形尺寸。"),
                    ("Use", "你抽出軟皮尺比對衣物袖長，拿出一枚細針將袖口微鬆的線頭俐落收好，熨貼合身。"),
                    ("MendTravelGear", "你在木砧檯與硬木楦前修補隨身行靴與桐油竹傘，針腳細密，底掌牢固如新。"),
                ],
            ),
        ]
    } else if room_name.contains("鍛造") || room_name.contains("鐵器") || room_name.contains("鑿石") {
        vec![
            make_peninsula_object(
                "wt_forge_furnace_anvil",
                "耐火鍛造台",
                &["Look", "Operate", "WarmHands", "QuenchBlade"],
                &[
                    ("Look", "以紅土耐火磚築成的高爐炭火正熾，爐口火光通明，精鋼鍛台旁擺著導能冷卻水槽與各色鍛錘。"),
                    ("Operate", "你握住重鐵錘在燒紅鐵坯上試敲一記，火星迸濺，金鐵交鳴聲清亮悠長。"),
                    ("WarmHands", "你伸開十指靠近鍛爐熱氣，滾燙厚重的熱浪烘走掌心寒氣，筋骨舒張。"),
                    ("QuenchBlade", "你將隨身器物探入紅土高爐均勻受熱，再浸入深井冷泉與淬火油槽急速回火，鋒刃重煥寒芒。"),
                ],
            ),
            make_peninsula_object(
                "wt_forge_brass_hardware_rack",
                "黃銅器物架",
                &["Look", "Use"],
                &[
                    ("Look", "展示架上排著拋光自來水黃銅水栓、輕便隨身保溫瓶、黃銅懷錶、精鋼摺疊剃刀與黃銅彈子門鎖。"),
                    ("Use", "你展開精鋼摺疊剃刀撫看刀口，刃口研磨得鋒銳無匹，轉軸銅釘開闔嚴密順滑。"),
                ],
            ),
            make_peninsula_object(
                "wt_forge_quench_trough",
                "精鋼淬火槽",
                &["Look", "Use", "TestEngraving", "QuenchBlade"],
                &[
                    ("Look", "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。"),
                    ("Use", "你舀水淋在細磨石上推磨鐵件，金屬毛刺漸去，刃口泛起雪亮微光。"),
                    ("TestEngraving", "你運轉內息按上測紋石板並取隨身器物輕觸碑面，石紋微鳴回饋人器共鳴度。"),
                    ("QuenchBlade", "你將隨身器物探入紅土高爐均勻受熱，再浸入深井冷泉與淬火油槽急速回火，鋒刃重煥寒芒。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && room_name.contains("觀星") {
        vec![
            make_peninsula_object(
                "wt_cat_0734",
                "黃銅星軌儀",
                &["Look", "Operate", "Read", "ChartAstrolabe"],
                &[
                    ("Look", "高聳於白石基座上的精緻銅儀，層層同心環刻劃著半島二十四節氣、晨昏潮位與天象經緯刻度，銅環轉動輕盈無聲。"),
                    ("Operate", "你輕推星軌儀外環，銅齒輪咬合轉動，星象指針精準對向今夜將在東天升起的導航主星。"),
                    ("Read", "你辨讀底座銅盤銘刻的星曆推算表，上面詳載著半島歷代學者觀測潮汐漲落與季風轉折的心血記錄。"),
                    ("ChartAstrolabe", "你鋪開竹綿宣紙與松煙墨拓，循星軌儀子午刻度拓下個人周天命格星盤。"),
                ],
            ),
            make_peninsula_object(
                "wt_obs_tide_weather_table",
                "潮汐觀測案",
                &["Look", "Read", "Use", "ReadWindVane"],
                &[
                    ("Look", "厚重的老樟木長案上整齊攤開著半島氣象水文走勢圖、多層手繪潮汐升降曲線與防風墨水盒，供學者評議研討。"),
                    ("Read", "你細讀長案上留存的近期海灣潮位紀錄，字跡密密麻麻，詳盡標註著湧浪高度與洋流變化趨勢。"),
                    ("Use", "你提筆在長案的備忘薄頁上記下今日天色與風向，供下一輪評議學者對照參考。"),
                    ("ReadWindVane", "你仰觀黃銅風向儀與相風銅烏轉速，比對滴漏刻度，精準推算出半島山海風候與潮汐走向。"),
                ],
            ),
            make_peninsula_object(
                "wt_obs_brass_telescope_rack",
                "青銅窺管架",
                &["Look", "Operate", "GrindOpticLens"],
                &[
                    ("Look", "架設於觀景石台邊緣的重型青銅窺管，配備精密螺紋微調旋鈕，旁側木架整齊插著數十卷絲帛手繪星圖。"),
                    ("Operate", "你俯身湊近青銅窺管的目鏡，微調螺紋對焦，遠方蒼穹中閃爍的繁星與雲層邊緣清晰呈現在眼前。"),
                    ("GrindOpticLens", "你取過紅土崁天然白水晶胚與細金剛砂，在光學磨盤上細心研磨校准青銅窺管鏡片。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && (room_name.contains("契書局") || room_name.contains("房地契")) {
        vec![
            make_peninsula_object(
                "wt_deed_cadastral_map_desk",
                "四坊地籍案",
                &["Look", "Read", "Use", "InspectLandRegistry"],
                &[
                    ("Look", "寬闊平整的核契楠木長案上平鋪著文曜四坊地籍疆界全圖，一旁陳設著玻璃螺紋墨水瓶與弧形吸墨器。"),
                    ("Read", "你俯身查看四坊地籍圖冊，文曜城中各街各巷街屋的界址、戶號與公產邊界皆以硃砂與濃墨標記分明。"),
                    ("Use", "你執起木柄弧形吸墨器在草擬文契上輕輕滾過，新寫就的黑墨油光瞬間被乾淨吸附，字跡烏黑硬朗。"),
                    ("InspectLandRegistry", "你俯身查閱文曜四坊地籍疆界全圖與各鎮公產清冊，將街廓界址與公庫民生實況鈔錄存查。"),
                ],
            ),
            make_peninsula_object(
                "wt_deed_household_registry",
                "樟木戶籍櫃",
                &["Look", "Read", "Open", "InspectLandRegistry"],
                &[
                    ("Look", "頂天立地的高大樟木檔案櫃，以生鐵包角加固，櫃門貼著分類封籤，保存著《半島全戶總譜》與歷任戶籍謄本。"),
                    ("Read", "你查閱外格貼附的戶籍檔案檢索目錄，各鎮居民落籍與各坊家戶丁冊皆按年月編次，巨細靡遺。"),
                    ("Open", "你拉開沉重的鐵扣樟木抽屜，一陣防蠹樟腦香撲面而來，厚重的羊皮紙總譜冊頁陳列井然。"),
                    ("InspectLandRegistry", "你俯身查閱文曜四坊地籍疆界全圖與各鎮公產清冊，將街廓界址與公庫民生實況鈔錄存查。"),
                ],
            ),
            make_peninsula_object(
                "wt_cat_0793",
                "紅印屋契摺",
                &["Look", "Read", "Acquire"],
                &[
                    ("Look", "厚實木盤中擺著蓋有文曜公議堂硃砂官印的正本屋契與租約摺。依半島律例，凡立契買下街屋者可獲發大門銅鎖鑰匙，支付 25 鎂即可辦理置產。"),
                    ("Read", "你細閱屋契條款，公議堂硃砂大印鮮明，清楚載明產權過戶、房屋修繕與大門配匙登錄章程，童叟無欺。"),
                    ("Acquire", "你向書吏出具購屋憑證並繳清規費，在紅印屋契與租約摺上鄭重畫押，順利領取屬於自己的置產契書與門牌底冊。"),
                ],
            ),
        ]
    } else if !is_outdoor_road
        && (room_name.contains("公議")
            || room_name.contains("公所")
            || room_name.contains("公會")
            || room_name.contains("礦務所")
            || room_name.contains("公事房")
            || room_name.contains("商盟")
            || room_name.contains("行會")
            || room_name.contains("總行")
            || room_name.contains("商會"))
    {
        vec![
            make_peninsula_object(
                "wt_guild_commission_board",
                "百工行會委託告示板",
                &["Look", "Read", "Use", "Deliver", "ClaimStrandedCargo", "DeliverStrandedCargo"],
                &[
                    ("Look", "行會正堂牆面懸掛著寬大的松木告示板，釘滿了各坊工匠、行商與巡守隊發布的急缺物料懸賞與護運委託。"),
                    ("Read", "你逐行研讀告示板上的急件委託，上載各聚落物資短缺通報、滯留物資急遞單與山道靖安警示。"),
                    ("Use", "你走近委託告示板，核對各坊工料需求與護送懸賞。"),
                    ("Deliver", "你核對隨身行囊中的急需物料，準備依委託條款交割換取酬勞。"),
                    ("ClaimStrandedCargo", "你揭下板上封蓋硃砂印章的受滯貨單，承諾替受阻行商將貨物送達目的聚落。"),
                    ("DeliverStrandedCargo", "你向行會交接管運的受滯貨物與單據，核銷並結算公庫託管賞格。"),
                ],
            ),
            make_peninsula_object(
                "wt_hall_bulletin_board",
                "民務告示欄",
                &["Look", "Read", "Use", "Deliver", "ClaimStrandedCargo", "DeliverStrandedCargo", "DeliberateCivicPact"],
                &[
                    ("Look", "牆側立著高大的厚木告示欄，張貼著新頒定的水額管理辦法、工坊評定標準與季節巡防名單。"),
                    ("Read", "你逐條細讀告示內容，公約條款字跡剛勁端正，旨在保障水土民生休養生息。"),
                    ("Use", "你走近告示欄，查閱各坊急徵物資與行會懸賞通告。"),
                    ("Deliver", "你核對行囊中的急缺物資與受滯貨物，準備依告示進行交割。"),
                    ("ClaimStrandedCargo", "你揭下告示欄上封蠟行會受滯貨單，接下前往指定聚落的護運承諾。"),
                    ("DeliverStrandedCargo", "你向告示欄掌事書吏出示受滯貨單與護運貨物，辦理公庫託管賞格交割。"),
                    ("DeliberateCivicPact", "你駐足民務告示欄前細讀四坊鄉約草案，與街坊鄰里共商水利與工坊條款。"),
                ],
            ),
            make_peninsula_object(
                "wt_hall_council_table",
                "厚木公議桌",
                &["Look", "Sit", "Read", "DeliberateCivicPact"],
                &[
                    ("Look", "廳堂中央擺設著長條大木會議桌，桌面整齊放著公議議程冊、公章墨盒與簽名木籌。"),
                    ("Sit", "你在厚重的長背木椅上坐定，廳內氣氛莊重嚴明，公議秩序井然。"),
                    ("Read", "你翻開議程冊閱覽，上面記錄著即將商討的供水調配方案與各區工料評定細則。"),
                    ("DeliberateCivicPact", "你在厚木公議桌前落座，翻開議程冊與街坊鄰里同席參議鄉約，共商民生大計。"),
                ],
            ),
            make_peninsula_object(
                "wt_hall_seal_archive",
                "雙鎖公文櫃",
                &["Look", "Open"],
                &[
                    ("Look", "靠牆安放著帶雙銅鎖的公文卷櫃，案上備有用於封存公函的專用紅陶印泥與封條紙。"),
                    ("Open", "你拉開已解鎖的文卷櫃抽屜，歷年公議決議文冊皆按年號編目整齊，保存極為完好。"),
                ],
            ),
        ]
    } else if (room_name.contains("藏書閣")
        || room_name.contains("印行館")
        || room_name.contains("書局")
        || room_name.contains("書舍")
        || room_name.contains("圖誌館"))
        && !room_name.contains("藏書閣東側石巷")
    {
        vec![
            make_peninsula_object(
                "wt_book_printing_shelf",
                "風物誌書架",
                &["Look", "Read", "TranscribeRareTome", "SunBooks"],
                &[
                    ("Look", "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。"),
                    ("Read", "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。"),
                    ("TranscribeRareTome", "你展平竹綿宣紙，提起鋼尖蘸水硬筆細心鈔錄《白塔風物誌》孤本考證，墨跡端凝。"),
                    ("SunBooks", "你將書架上的竹綿紙古籍移至向陽通風石坪翻曬防蠹，夾入辟蠹芸香。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_map_reading_table",
                "全圖閱覽桌",
                &["Look", "Read", "Open", "GrindOpticLens"],
                &[
                    ("Look", "長木桌上平鋪著精細手繪的白塔半島山海全圖，配備著黃銅助視鏡與精準標尺。"),
                    ("Read", "你細看圖上標註的水道、礦脈與城鎮驛站路線，對半島各地山海形勢瞭然於胸。"),
                    ("Open", "你翻開桌側的地圖抽屜，取出周邊海域潮汐水深航圖細細對照。"),
                    ("GrindOpticLens", "你使用細金剛砂與麂皮絨布細細研磨全圖閱覽桌上的黃銅助視鏡水晶鏡片，光路澄澈無瑕。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_pen_notebook_desk",
                "蘸水硬筆書案",
                &["Look", "Use", "TranscribeRareTome"],
                &[
                    ("Look", "寬大書案上鋪著氈布，銅筆筒裡插著鋼尖蘸水硬筆，案旁置有墨水瓶與竹綿紙記事手札。"),
                    ("Use", "你提起鋼尖蘸水硬筆蘸墨落筆，筆尖在竹綿紙上順滑游走，字跡清晰深沉，墨乾極速。"),
                    ("TranscribeRareTome", "你展平竹綿宣紙，提起鋼尖蘸水硬筆細心鈔錄《白塔風物誌》孤本考證，墨跡端凝。"),
                ],
            ),
        ]
    } else if !is_outdoor_road
        && (room_name.contains("學院")
            || room_name.contains("講習")
            || room_name.contains("蒙學")
            || room_name.contains("講舍")
            || room_name.contains("校舍")
            || room_name.contains("學舍")
            || (room_name.contains("講堂")
                && !room_name.contains("講堂街")
                && !room_name.contains("講堂外坊")
                && !room_name.contains("講堂內環")
                && !room_name.contains("講堂外環")))
    {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate", "AttendLecture"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨硬筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅硬筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                    ("AttendLecture", "你在講堂席間端坐靜聽先生講授當季山海物候、水利營造與商算策論，並提筆疾書要義。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_printing_shelf",
                "風物誌書架",
                &["Look", "Read"],
                &[
                    ("Look", "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。"),
                    ("Read", "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。"),
                ],
            ),
            make_peninsula_object(
                "wt_book_pen_notebook_desk",
                "蘸水硬筆書案",
                &["Look", "Use", "AttendLecture"],
                &[
                    ("Look", "寬大書案上鋪著氈布，銅筆筒裡插著鋼尖蘸水硬筆，案旁置有墨水瓶與竹綿紙記事手札。"),
                    ("Use", "你提起鋼尖蘸水硬筆蘸墨落筆，筆尖在竹綿紙上順滑游走，字跡清晰深沉，墨乾極速。"),
                    ("AttendLecture", "你在講堂席間端坐靜聽先生講授當季山海物候、水利營造與商算策論，並提筆疾書要義。"),
                ],
            ),
        ]
    } else if room_name.contains("診所") {
        vec![
            make_peninsula_object(
                "wt_clinic_herb_cabinet",
                "百草丸藥櫃",
                &["Look", "Smell", "Use", "PrepareMiasmaGear"],
                &[
                    ("Look", "百草藥櫃抽屜密布，桌上整齊排著黑沼止血水苔敷藥、避瘴清心丸、祛寒草木湯包與青鹽薄荷潔牙粉。"),
                    ("Smell", "你低頭深嗅藥盤，薄荷清涼、艾草微辛與止血水苔的草香交織，胸悶頓解。"),
                    ("Use", "你取了一小撮避瘴清心丸服下，清涼藥力入喉，周身氣血頓時平順舒暢。"),
                    ("PrepareMiasmaGear", "你拉開藥斗配齊避瘴清心丸與紅磷信號筒，藥香清冽，胸腹鬱結之氣隨之一清。"),
                ],
            ),
            make_peninsula_object(
                "wt_clinic_acupoint_bed",
                "竹席診脈榻",
                &["Look", "Rest", "ApplyMoxibustion"],
                &[
                    ("Look", "診榻鋪著細編竹席與山風草安神枕，榻邊點著驅蚊艾草香盤，靜謐祥和。"),
                    ("Rest", "你躺上竹席診榻靜臥歇息，草木香氣寧神靜氣，緊繃的身心逐漸放鬆下來。"),
                    ("ApplyMoxibustion", "你點燃溫氏陳年艾絨置於溫灸盒上溫熨經絡穴位，艾香透骨，扭挫瘀滯頓時化開。"),
                ],
            ),
            make_peninsula_object(
                "wt_clinic_decoction_stove",
                "煎藥小銅爐",
                &["Look", "Taste", "WarmHands", "ApplyMoxibustion"],
                &[
                    ("Look", "小銅爐上砂鍋文火慢熬著溫氏祛寒草木湯包，熱氣裊裊，飄出濃郁草木溫熱氣息。"),
                    ("Taste", "你啜飲一小口熬好的草木溫湯，微苦回甘，暖流順著喉嚨流向四肢百骸。"),
                    ("WarmHands", "你把手掌靠在銅爐外罩旁，受溫熱草本香露烘暖，指頭血氣活絡。"),
                    ("ApplyMoxibustion", "你點燃溫氏陳年艾絨置於溫灸盒上溫熨經絡穴位，艾香透骨，扭挫瘀滯頓時化開。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("石磨") {
        vec![
            make_peninsula_object(
                "wt_mill_waterwheel_stone",
                "水輪玄武磨",
                &["Look", "Operate", "MillGrain"],
                &[
                    ("Look", "老堰頭引水渠衝擊巨木水輪帶動齒輪轉動，兩合巨大玄武岩磨盤正飛速研磨新熟米穀。"),
                    ("Operate", "你扳動水流分水閘柄調節水勢，水輪轉速隨之放緩，磨盤研磨出的白米粉更顯細密勻淨。"),
                    ("MillGrain", "你合力推開水磨料門，巨輪轉動帶動石磨隆隆轟鳴，白米碎成細漿，研磨出老堰頭溫熱糙米漿。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_flour_sieve",
                "竹絲篩粉斗",
                &["Look", "Use", "MillGrain"],
                &[
                    ("Look", "細如青絲的密編竹篩斗分出粗米糠與細米粉，篩下積著雪白如霜的純米粉堆。"),
                    ("Use", "你雙手晃動懸掛的竹篩斗，雪白米粉簌簌落下，粗糠被乾淨留在篩網上方。"),
                    ("MillGrain", "你端起竹絲細篩抖動，篩漏出細密純白米粉，研磨分級一氣呵成。"),
                ],
            ),
            make_peninsula_object(
                "wt_mill_grain_hopper",
                "杉木儲糧斗",
                &["Look", "Open", "MillGrain"],
                &[
                    ("Look", "高懸的倒梯形杉木儲料斗連接著下料槽，旁邊架著點驗原糧重量的大秤。"),
                    ("Open", "你拉開料斗下方的調節插板，顆粒飽滿的金色稻穀嘩啦啦滑向磨眼，源源不斷。"),
                    ("MillGrain", "你啟開杉木料斗插板，金色稻穀源源湧入磨眼，水輪轟鳴研磨出新米漿。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("鋸木") {
        vec![
            make_peninsula_object(
                "wt_sawmill_hydraulic_saw",
                "水動排鋸架",
                &["Look", "Operate", "RefinePlank"],
                &[
                    ("Look", "水流帶動的長排精鋼鋸條轟鳴起伏，平穩將整根百年雲杉原木剖切成均勻厚板。"),
                    ("Operate", "你推動送木台車向前咬緊鋸齒，鋼鋸飛速下切，鋸路筆直順暢，木屑紛飛。"),
                    ("RefinePlank", "你推動水動排鋸咬合雲杉巨木，精鋼鋸齒飛速剖切，裁出筆直平整的樟松木板料。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_timber_bench",
                "原木檢寸台",
                &["Look", "Use", "RefinePlank"],
                &[
                    ("Look", "鐵木長台上刻有度量標尺，擺著散發濃烈松脂香氣的試切木塊與標記石筆。"),
                    ("Use", "你拿起石筆比對標尺在板材邊緣畫下一道規整的裁切基線，下料尺寸分毫不大。"),
                    ("RefinePlank", "你以墨斗與檢寸規標定厚度，推入長凳固定切削，整理出規格整齊的厚木板料。"),
                ],
            ),
            make_peninsula_object(
                "wt_sawmill_woodchip_bin",
                "乾燥刨花桶",
                &["Look", "Smell", "Gather"],
                &[
                    ("Look", "高大的松木大桶盛滿刨下的乾燥木片與刨花，這是燒製上等松炭的上乘原料。"),
                    ("Smell", "你捧起一把乾燥刨花深嗅，濃郁清冽的松脂油脂香撲面而來，乾爽宜人。"),
                    ("Gather", "你順手抓起一紮結實乾燥的松木刨花紮好，可作為生火引柴的上好引信。"),
                ],
            ),
        ]
    } else if !is_outdoor_road && !is_residential_house && room_name.contains("切石") {
        vec![
            make_peninsula_object(
                "wt_stonecut_wire_saw",
                "金剛砂拉鋸",
                &["Look", "Operate", "CutJade"],
                &[
                    ("Look", "工匠引細密合金絲拉鋸帶著濕潤金剛砂緩緩剖開巨大的北山寒水玉原石，石皮下透出溫潤玉光。"),
                    ("Operate", "你輕轉注水輪維持金剛砂泥漿順暢流入切割線，金屬絲勻速拉動，石料緩緩剖為兩半。"),
                    ("CutJade", "你拉動金剛砂鋼絲鋸緊貼寒水玉原石，冷泉注淋鋸槽，細細切剖出溫潤剔透的寒水玉料。"),
                ],
            ),
            make_peninsula_object(
                "wt_stonecut_jade_display",
                "寒水玉料架",
                &["Look", "Use", "CutJade"],
                &[
                    ("Look", "展示架上整齊排著剖切平整的寒水玉薄板，手觸如浸涼泉，是打造保鮮石櫃的極品材料。"),
                    ("Use", "你伸手撫摸一片寒水玉石板，指掌頓感沁涼透骨，板面質地堅密細潤無瑕。"),
                    ("CutJade", "你從架上挑選玉料毛胚，以刻刀與量尺剖分紋理，取出精選寒水玉石。"),
                ],
            ),
            make_peninsula_object(
                "wt_stonecut_grind_basin",
                "拋光琢石盆",
                &["Look", "Use", "CutJade"],
                &[
                    ("Look", "引流活水的不鏽石盆中浸著各目數的磨石與羊毛拋光氈，專用於精細琢磨石板邊角。"),
                    ("Use", "你取了一塊切好的寒水玉小塊在細石上加水研磨，石角稜邊很快變得圓潤光滑。"),
                    ("CutJade", "你浸潤細砂研磨琢石，水花泛起玉光，將原石稜角細心琢磨成形。"),
                ],
            ),
        ]
    } else if room_name.contains("觀測") {
        vec![
            make_peninsula_object(
                "wt_observatory_anemometer",
                "黃銅風向儀",
                &["Look", "Operate", "Read", "ReadWindVane"],
                &[
                    ("Look", "露台上架著精巧的八方黃銅風向羽與四轉杯，精準指引著半島上空雲霧與氣流動向。"),
                    ("Operate", "你輕推校正旋鈕調整齒輪平衡，指針隨風微微擺動，指向西北風向。"),
                    ("Read", "你查看儀器底盤上的刻度，風速平緩穩定，近期半島沿岸無暴風驟雨之虞。"),
                    ("ReadWindVane", "你仰觀黃銅風向儀與相風銅烏轉速，比對滴漏刻度，精準推算出半島山海風候與潮汐走向。"),
                ],
            ),
            make_peninsula_object(
                "wt_observatory_cloud_log",
                "天象觀測冊",
                &["Look", "Read"],
                &[
                    ("Look", "厚實的竹綿紙記錄冊詳實抄錄著半嶺與四方海灣十年間的節氣、降雨與霜凍星曆。"),
                    ("Read", "你翻閱記錄冊最近數頁，上面載明今日雲氣漸聚於北山山脊，傍晚或有山林微雨。"),
                ],
            ),
            make_peninsula_object(
                "wt_observatory_brass_telescope",
                "黃銅望遠鏡",
                &["Look", "Use"],
                &[
                    ("Look", "架設於石基轉軸上的長筒黃銅望遠鏡，鏡頭打磨純淨，筒身嵌有夜光貝校準旋鈕。"),
                    ("Use", "你伏身湊近目鏡遠眺，數十里外的海灣帆影與山原林色歷歷在目，極目千里。"),
                ],
            ),
        ]
    } else {
        return None;
    };
    Some(objs)
}

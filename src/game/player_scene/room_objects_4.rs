#[allow(unused_variables)]
fn peninsula_objects_chunk_4(room_name: &str) -> Option<Vec<RoomObject>> {
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

    let objs = if room_name.contains("柑仔店") || (room_name.contains("雜貨") && room_name.contains("文曜")) {
        vec![
            make_peninsula_object(
                "wt_grocery_match_incense_shelf",
                "艾草蚊香架",
                &["Look", "Use", "Acquire"],
                &[
                    ("Look", "木格貨架上擺滿了成盒的防潮紅磷火籤、一圈圈青綠艾草蚊香盤與居家必備的麻繩蠟燭。"),
                    ("Use", "你輕嗅艾草蚊香盤，清苦幽香沁人，正是驅除弄堂水氣與蚊蟲的良伴。"),
                    ("Acquire", "你伸手挑選了一小匣紅磷火籤與幾卷蚊香，隨時準備帶走備用。"),
                ],
            ),
            make_peninsula_object(
                "wt_grocery_candy_jar",
                "玻璃柑仔罐",
                &["Look", "Taste"],
                &[
                    ("Look", "老舊木櫃檯上放著幾只大號廣口玻璃罐，裡面裝著五顏六色的酸梅糖、黑糖麥芽糖與金桔軟糖。"),
                    ("Taste", "你拈起一顆金桔酸梅糖含在嘴裡，酸酸甜甜的滋味在舌尖化開，滿是市井老滋味。"),
                ],
            ),
        ]
    } else if room_name.contains("修傘") || room_name.contains("補靴") || room_name.contains("晴雨傘") || (room_name.contains("皮靴") && room_name.contains("工坊")) {
        vec![
            make_peninsula_object(
                "wt_repair_umbrella_bench",
                "修傘木砧檯",
                &["Look", "Operate", "Use", "MendTravelGear"],
                &[
                    ("Look", "矮木長凳前擺著修傘專用木砧板、幾捲熟桐油棉紙、破開的細韌紫竹篾骨與銅穿線錐。"),
                    ("Operate", "你拿起穿線錐將竹骨關節上的絲線拉緊打結，傘骨開合立即靈活如初。"),
                    ("Use", "你撐開一把補整桐油的棉紙晴雨傘試驗，傘面緊繃防雨，桐油幽香撲鼻。"),
                    ("MendTravelGear", "你在木砧檯與硬木楦前修補隨身行靴與桐油竹傘，針腳細密，底掌牢固如新。"),
                ],
            ),
            make_peninsula_object(
                "wt_repair_boots_last",
                "補靴硬木楦",
                &["Look", "Use", "MendTravelGear"],
                &[
                    ("Look", "結實硬木楦頭上套著一隻抹滿厚魚油的防水皮靴，旁側擺著牛皮掌皮、生鐵鞋釘與補靴小錘。"),
                    ("Use", "你拿起小錘輕敲皮靴後跟的鞋掌，鐵釘吃入堅木與硬皮，底掌牢固無比。"),
                    ("MendTravelGear", "你在木砧檯與硬木楦前修補隨身行靴與桐油竹傘，針腳細密，底掌牢固如新。"),
                ],
            ),
        ]
    } else if room_name.contains("天井") || room_name.contains("弄堂") {
        vec![
            make_peninsula_object(
                "wt_alley_bamboo_drying_rack",
                "竹竿晾衣架",
                &["Look", "Use"],
                &[
                    ("Look", "天井兩側牽著幾根筆直長竹竿，竿上掛著被海風吹拂飄動的藍染粗布衣物，竹夾子咬得極緊。"),
                    ("Use", "你隨手整理被風吹偏的竹夾，仰頭看著窄仄天頂透下的天光與白雲，格外閒適。"),
                ],
            ),
            make_peninsula_object(
                "wt_alley_stone_well",
                "青石古井",
                &["Look", "Use", "Wash", "DrawSpringWell"],
                &[
                    ("Look", "弄堂轉角處有一口被繩索磨出深深凹痕的八角青石古井，井水清澈冰涼，桶索完整。"),
                    ("Use", "你轉動轆轤搖起一桶沁涼甘甜的井水，水珠飛濺，暑氣全消。"),
                    ("Wash", "你舀起一勺清涼井水洗了洗臉與雙手，整個人神清氣爽。"),
                    ("DrawSpringWell", "你搖動轆轤自青石古井汲起一桶甘冽山泉，清涼潤嗓，與井畔街坊互道安好。"),
                ],
            ),
        ]
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") || room_name.contains("研墨小齋") || room_name.contains("校勘") {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨硬筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅硬筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                ],
            ),
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
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        vec![
            make_peninsula_object(
                "wt_academy_fountain_pen",
                "黃銅儲墨筆",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "烏木教案的正中央擱著一支溫潤沉甸的黃銅桿儲墨硬筆，筆尖以精煉銥金打磨得細膩微光，旁側襯著吸墨氈布與寫滿批註的竹綿紙講義。"),
                    ("Use", "你執起教案上的黃銅硬筆，銥金筆尖觸及竹綿紙面，黑墨勻稱流暢地順著筆鋒沁入紙紋，留下一行端勁清晰的手跡。"),
                    ("Operate", "你輕旋黃銅筆桿檢視儲墨囊，墨色飽滿澄澈，筆帽卡榫嚴絲合縫，足見文曜百工製筆之精巧。"),
                ],
            ),
            make_peninsula_object(
                "wt_astro_brass_armillary",
                "渾天推步儀",
                &["Look", "Use", "Operate", "Read", "ChartAstrolabe"],
                &[
                    ("Look", "多層同心黃銅環刻劃著十二辰次與周天度數，子午環與赤道環交相輝映，底座刻滿了文曜曆算學士世代推步的星度軌跡。"),
                    ("Use", "你推動星軌儀的精銅外環，齒輪與軸承咬合平順，星軌刻度隨之流轉對齊今日天象，心中豁然開朗。"),
                    ("Operate", "你輕輕旋轉子午度盤微調交角，黃銅星規與指針精準校準，測候律曆之奧妙盡在其中。"),
                    ("Read", "你俯身辨讀星軌儀底座鐫刻的文曜先賢推步法度，字跡端凝入微，深邃博大。"),
                    ("ChartAstrolabe", "你鋪開竹綿宣紙與松煙墨拓，循星軌儀子午刻度拓下個人周天命格星盤。"),
                ],
            ),
            make_peninsula_object(
                "wt_math_abacus_desk",
                "黑檀計數板",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "厚重松木長案上平放著一具黑檀木精密計數板，滑鈕圓潤黑亮，旁側鋪著墨跡未乾的算術推演草稿與算籌木匣。"),
                    ("Use", "你信手撥動黑檀計數滑鈕，扣合聲清脆利落，心中速算推演，思維條理頓時無比清晰。"),
                    ("Operate", "你推動計數板橫桿歸零復位，滑鈕整齊列陣，隨手整理案上的推算手札與筆墨。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "算案一側置有一盞精巧的旋蓋夜光貝母檯燈，乳白貝殼透光柔和，照亮密密麻麻的算題推算手稿。"),
                    ("Use", "你轉動烏木燈圈調節明暗，柔和白光灑在計數板與草紙上，省目清心。"),
                    ("Operate", "你微調貝母燈罩的角度使光束聚焦於核算案面，光斑清晰不炫目。"),
                ],
            ),
        ]
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        vec![
            make_peninsula_object(
                "wt_print_movable_type",
                "梨木活字盤",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "數以千計的梨木雕刻活字整齊碼放在分格木盤內，長案上擺著棕櫚墨刷、壓紙平砧與新刷松煙墨的排版木框。"),
                    ("Use", "你拿起棕櫚墨刷在排版活字上勻勻刷上一層松煙墨，覆上宣紙輕輕滾壓，字跡頓時端正顯映於紙上。"),
                    ("Operate", "你手持竹鑷在活字盤中檢選字模並嵌入版框，木塊卡榫緊密堅實，字面齊整如刀削。"),
                ],
            ),
            make_peninsula_object(
                "wt_paper_bamboo_vat",
                "抄紙青石槽",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "青石大槽中浸著雪白的竹漿紙漿，旁邊架著細如青絲的抄紙竹簾，內壁熱氣騰騰的夾道焙紙牆正烘乾著新起出的薄綿紙。"),
                    ("Use", "你雙手執竹簾浸入石槽，平順蕩料抄起一層細密纖維，揭紙貼於熱牆烘乾，紙面瑩白平滑，工藝之妙令人讚歎。"),
                    ("Operate", "你輕撫焙紙牆上烘乾的竹綿宣紙，纖維綿密細韌，揭下疊入紙捆，整齊平整。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "排版案頭立著防風夜光貝母檯燈，珠母薄片在水汽與墨香中泛著瑩潤清輝。"),
                    ("Use", "你旋亮夜光貝母燈罩，澄澈珠光映照著活字盤，微小筆畫一覽無遺。"),
                    ("Operate", "你調整燈罩遮光板，將清光引向拓印完成的書頁進行細緻校對。"),
                ],
            ),
        ]
    } else if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        vec![
            make_peninsula_object(
                "wt_herb_drying_kiln",
                "紅泥焙藥爐",
                &["Look", "Use", "Operate", "WarmHands", "RoastHerbs"],
                &[
                    ("Look", "雙層紅泥焙爐中炭火微溫，上下兩層密布通氣孔，溫火正烘烤著當歸、黃耆與山風草，濃郁藥香透滿全室。"),
                    ("Use", "你撥動紅泥爐膛的通氣瓦片微調火候，熱氣蒸騰間草木本質被徐徐焙透，藥性純粹，精神為之一振。"),
                    ("Operate", "你掀開頂層紅泥蓋檢查藥材焙乾程度，藥草微黃酥脆，火候恰到好處。"),
                    ("WarmHands", "你靠近紅泥焙藥爐的側壁烘手，厚重紅陶散發著沉穩持久的溫熱藥氣，周身寒氣盡消。"),
                    ("RoastHerbs", "你在紅泥焙藥爐前微調炭火通氣孔，依古法九蒸九曝炮製七星艾絨與當歸黃耆。"),
                ],
            ),
            make_peninsula_object(
                "wt_herb_bamboo_sieve",
                "切藥鍘刀案",
                &["Look", "Use", "Operate", "RoastHerbs"],
                &[
                    ("Look", "層層竹篩中攤曬著洗淨的鮮藥草，長木案上固定著厚刃生鐵切藥鍘刀，案面滿是整齊切片留下的草木汁痕。"),
                    ("Use", "你手握切藥鍘刀木柄壓下，利刃切開晾乾的草藥根莖，發出清脆嚓響，藥香濃烈撲面。"),
                    ("Operate", "你翻動竹篩中的藥草使其受風均勻，青綠與微褐色澤錯落，草木清香盈溢鼻端。"),
                    ("RoastHerbs", "你在切藥鍘刀案前勻稱切製晾乾的藥草根莖，配成九蒸七星艾絨藥包。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "切藥長案角設有一盞防潮夜光貝母調光檯燈，光暈溫和不燥，映照著鋪展的藥草竹篩。"),
                    ("Use", "你轉動旋蓋調整燈火光暈，白光籠罩切藥案板，便於辨識藥材切片厚度。"),
                    ("Operate", "你輕輕旋轉貝母外殼調整反光角度，照亮掛滿乾草藥的陰乾竹架。"),
                ],
            ),
        ]
    } else if room_name.contains("北渠") || room_name.contains("東灣") || room_name.contains("水脈") || room_name.contains("河灣") || room_name.contains("竹筏渡") {
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
                "wt_river_bamboo_raft",
                "平底竹筏",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "三水川石砌埠頭邊立著長年受流水沖刷的繫船青石樁，石孔穿過粗大桐油麻繩，平穩繫泊著防浸平底竹筏與長竹篙。"),
                    ("Use", "你手持長竹篙在石階輕撐借力，竹筏滑入三水川清澈碧流，沿岸水草豐美、水鳥掠波。"),
                    ("Operate", "你解開繫在青石樁上的桐油麻繩，平底竹筏在川流水波間輕輕蕩漾，吃水甚穩。"),
                ],
            ),
        ]
    } else if room_name.contains("青竹苑") || room_name.contains("藥圃") || room_name.contains("苗圃") {
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
                "wt_bamboo_tea_table",
                "竹林青石案",
                &["Look", "Use", "Rest", "Taste"],
                &[
                    ("Look", "翠竹掩映下設有一方青石茶几與兩只石凳，几上置有陶泥小火爐與青花瓷茶盞，修竹搖曳，幽靜怡人。"),
                    ("Use", "你在微涼的青石凳上端坐歇息，以竹筧引來的甘泉瀹茶，竹影婆娑入盞，頓覺身心放鬆，塵慮盡消。"),
                    ("Rest", "你坐在石凳上閉目調息，耳畔修竹隨風沙沙作響，清幽靜謐，心境空明。"),
                    ("Taste", "你端起溫熱的茶盞啜飲一口，竹瀝清香夾著焙茶回甘，回味悠長。"),
                ],
            ),
            make_peninsula_object(
                "wt_home_shell_lamp",
                "檯燈",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "青石茶案旁修竹幹上掛有一盞防雨夜光貝母燈，微風拂動竹葉，青光與珠光交融成趣。"),
                    ("Use", "你撥動燈籠上的遮罩，柔和珠白光華傾瀉於石案，照亮一壺清茶。"),
                    ("Operate", "你調整竹梢上的懸掛索，使貝母燈在竹風中保持平穩照映。"),
                ],
            ),
        ]
    } else if room_name.contains("樞紐石橋") || room_name.contains("樞紐橋") {
        vec![
            make_peninsula_object(
                "wt_bridge_sundial_pillar",
                "日晷",
                &["Look", "Read"],
                &[
                    ("Look", "石橋兩端立有堅固花崗岩指路柱，柱頂刻有青銅日晷晷針，陰影清晰指示著當前時刻。"),
                    ("Read", "你走近石柱研讀鐫刻的方位路引：北通三水口北街與總商行，東連講堂街學院主校區與東橋旅社，南往公議堂與分家巷百工，西達傳訊鐘樓與西門聯運。"),
                ],
            ),
            make_peninsula_object(
                "wt_bridge_stone_balustrade",
                "橋欄",
                &["Look", "Rest"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自橋下緩緩匯入港灣。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往貨船，河風拂面，令人心曠神怡。"),
                ],
            ),
        ]
    } else if room_name.contains("蒙學") {
        vec![
            make_peninsula_object(
                "wt_obj_school_recess_bell",
                "課間報時鐘",
                &["Look", "Listen"],
                &[
                    ("Look", "蒙學堂門廊挑樑下懸掛著一口精銅小撞鐘，鐘身刻有日昇月恆圖紋，垂著一根紅絲編結的鐘繩。"),
                    ("Listen", "微風掠過鐘槌，撞鐘隱約發出輕微清脆的金石幽鳴，悠揚動聽。"),
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
    } else if room_name.contains("藏書閣東側石巷") {
        vec![
            make_peninsula_object(
                "wt_obj_stone_rubbing_cabinet",
                "碑拓石櫥",
                &["Look", "Check", "CollateInscriptions"],
                &[
                    ("Look", "巷壁遮雨深簷下嵌著防潮硬木石櫥，玻璃紗門內整齊陳列著半島古碑拓本與金石拓片。"),
                    ("Check", "你透過紗門審視櫥中拓片，字口清晰遒勁，古意盎然。"),
                    ("CollateInscriptions", "你打開碑拓石櫥取出古碑拓片，逐字比勘金石筆意並摹拓精校副本。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_bamboo_stone_bench",
                "修竹石凳",
                &["Look", "Rest"],
                &[
                    ("Look", "深巷夾道萬竿翠竹掩映下，擺放著一組琢磨光滑的長條青石凳，竹葉篩下斑駁竹影。"),
                    ("Rest", "你在修竹石凳上靜坐片刻，耳邊唯聞竹葉沙沙天籟，塵慮頓消。"),
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
    } else if room_name.contains("觀星台道") {
        vec![
            make_peninsula_object(
                "wt_obj_armillary_almanac",
                "定緯銅儀",
                &["Look", "Observe", "Read", "ChartAstrolabe"],
                &[
                    ("Look", "露天石台中央矗立著多層旋轉銅環交錯的北辰定緯儀，基座四周平嵌著刻滿二十四節氣晷影長短的黑曜石測影日晷。"),
                    ("Observe", "你凝神推動外環赤道環，黃銅軸心平滑旋轉，指針精準指向北天極星，刻度縝密清晰。"),
                    ("Read", "你研讀日晷台面刻印的七十二候節令注解，上載寒暑推移與日影消長之理，字跡精工嚴謹。"),
                    ("ChartAstrolabe", "你鋪開竹綿宣紙與松煙墨拓，循星軌儀子午刻度拓下個人周天命格星盤。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_stair_stone_railing",
                "登階石欄",
                &["Look", "Rest"],
                &[
                    ("Look", "通往觀星台的石階兩側築有層疊漸升的青條石護欄，石面平正厚重，欄柱頂部琢磨成圓潤星珠。"),
                    ("Rest", "你扶著平涼的石欄稍作歇步，放眼回望，城郭屋瓦櫛比鱗次。"),
                ],
            ),
            make_peninsula_object(
                "wt_obj_mountain_milestone",
                "山道指路碑",
                &["Look", "Read", "HangVotivePlaque"],
                &[
                    ("Look", "石階拐角處矗立著一尊取自青脊山的灰黑巨石碑，正面深刻著通往觀星高台與北嶺山道的箭頭路徑。"),
                    ("Read", "你仔細端詳碑文上的山勢地形指引，峰迴路轉的石階坡道清晰在目。"),
                    ("HangVotivePlaque", "你在散發松脂清香的北嶺雲杉木牌上題寫平安祈願，繫於山道古杉牌架之上，木牌隨山風清脆作響。"),
                ],
            ),
        ]
    } else if room_name.contains("講堂街") || room_name.contains("學院巷") || room_name.contains("書院巷") {
        vec![
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
    } else if room_name.contains("東門") {
        if room_name.contains("紙坊路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_drying_paper_rack",
                    "曬紙竹簾架",
                    &["Look", "Use"],
                    &[
                        ("Look", "巷口通風處斜倚著數排高大的細篾竹簾架，整齊平鋪著抄造完成的濕潤竹綿宣紙，在微風中散發清幽竹香。"),
                        ("Use", "你伸手輕撫竹簾邊緣，宣紙纖維均勻平整，簾絲密緻結實。"),
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
        } else if room_name.contains("機樞工坊路口") {
            vec![
                make_peninsula_object(
                    "wt_obj_hardware_bins",
                    "五金分格櫃",
                    &["Look", "Check"],
                    &[
                        ("Look", "鐵皮壁櫃上百餘個木匣盛滿各型鉚釘、螺栓、合頁與銷釘，地坪上置有一圈圓形冷水槽，專供燒紅車輪鋼箍套木輪後急速冷縮咬合。"),
                        ("Check", "你隨手拉開幾隻抽屜，各規格榫釘排列井然，鐵脂氣味濃厚。"),
                    ],
                ),
                make_peninsula_object(
                    "wt_caravan_water_trough",
                    "青石飲馬槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "路口橫著三丈長的深青石水槽，槽中滿注引自山澗的活水，供來往駝獸與大車飲用。"),
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
        } else if room_name.contains("關前") {
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
                    "wt_obj_east_road_milestone",
                    "東道里程碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "關前立著一方厚實的青石里程方碑，石面鑿刻著通往風舶角與老堰頭的里數與主驛道轉折標記。"),
                        ("Read", "你俯身辨識石碑上的深深刻痕，各聚落里程字跡清晰端正。"),
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
            ]
        } else {
            vec![
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
                        ("Look", "街口立著高木柱，頂端掛著旋蓋夜光貝母防風燈，下方釘著標明各工坊與城門方向的竹編路標牌。"),
                        ("Read", "你抬頭辨認路標上的指引字跡，各區方向一目了然，便於行客認路通行。"),
                        ("Operate", "你輕輕拉動低垂的吊繩旋開貝母防風燈罩，柔和珠光照亮了腳下平整的石板路面。"),
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
            ]
        }
    } else if room_name.contains("南門") {
        if room_name.contains("鍛坊路口") {
            vec![
                make_peninsula_object(
                    "wt_forge_quench_trough",
                    "精鋼淬火槽",
                    &["Look", "Use"],
                    &[
                        ("Look", "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。"),
                        ("Use", "你伸手試了試槽沿的冷水，水溫冰涼刺骨，正是鍛造淬火的最佳溫度。"),
                    ],
                ),
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
        } else if room_name.contains("車馬行路口") {
            vec![
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
        } else if room_name.contains("十字") {
            vec![
                make_peninsula_object(
                    "wt_obj_south_road_milestone",
                    "指路石碑",
                    &["Look", "Read"],
                    &[
                        ("Look", "十字街口立著一座四稜青石指路短碑，四面分別刻有通往西門、南哨與分家巷的方向指引。"),
                        ("Read", "你辨讀指路石碑上的箭頭與工整刻字，市街方位一目了然。"),
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
                    "wt_street_news_board",
                    "旬報閱報欄",
                    &["Look", "Read"],
                    &[
                        ("Look", "街邊遮雨棚下立著寬闊的閱報木欄，上面張貼著最新一期的《半島旬報》與學院及蒙學堂告示。"),
                        ("Read", "你駐足細讀旬報頭條，上面記載著半島各城鎮的航線調度、礦石產量與學院最新評議文論。"),
                    ],
                ),
            ]
        } else {
            vec![
                make_peninsula_object(
                    "wt_obj_welcome_pine_bench",
                    "迎客松石凳",
                    &["Look", "Rest"],
                    &[
                        ("Look", "街角蒼勁舒展的迎客古松下方，設有一圈取自半嶺的磨面弧形青石長凳，濃蔭匝地。"),
                        ("Rest", "你在松蔭下的石凳上落座歇涼，松脂清香令人精神一振。"),
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
        }
    } else {
        return None;
    };
    Some(objs)
}

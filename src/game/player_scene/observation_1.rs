fn regional_specialty_object_for_room(room_name: &str) -> Option<RoomObject> {
    let is_outdoor_passage = room_name.contains('街')
        || room_name.contains('道')
        || room_name.contains('巷')
        || room_name.contains('橋')
        || room_name.contains('階');

    if !is_outdoor_passage
        && (room_name.starts_with("風舶角")
            || room_name.starts_with("風舶嶼")
            || room_name.starts_with("風櫃尾")
            || room_name.starts_with("烏石鼻"))
    {
        Some(make_peninsula_object(
            "wt_reg_coastal_specialty",
            "夜光貝風鈴",
            &["Look", "Operate", "Use"],
            &[
                ("Look", "屋簷與門廊下懸著烏石鼻海貝與夜光貝母串成的防風貝鈴，牆角立著裝配黃銅捲線輪的韌竹捲線釣竿、風櫃冷燻魚乾甕與防水樹膠雨衣。"),
                ("Operate", "海風拂過廊下，夜光貝母風鈴碰撞出清脆悅耳的叮咚聲，泛起溫潤珠光。"),
                ("Use", "你轉動韌竹釣竿上的黃銅捲線輪試了試手感，齒輪順滑輕巧，隨時能去港灣礁石垂釣。"),
            ],
        ))
    } else if !is_outdoor_passage
        && (room_name.starts_with("半嶺鎮")
            || room_name.starts_with("老堰頭")
            || room_name.starts_with("北嶺腳")
            || room_name.starts_with("霜林口"))
    {
        Some(make_peninsula_object(
            "wt_reg_mountain_specialty",
            "寒水玉茶盅",
            &["Look", "WarmHands", "Use"],
            &[
                ("Look", "松香木架上擺著半嶺寒水玉雕成的恆溫茶盅、松脂暖手爐，旁側掛著進山必備的內襯寒水玉採集背簍、卡式導能便攜爐與黃銅單筒望遠鏡。"),
                ("WarmHands", "你捧起溫潤的寒水玉茶盅與松脂暖手爐，北山特有的恆溫暖意順著掌心傳遍全身。"),
                ("Use", "你舉起黃銅單筒望遠鏡朝窗外遠眺，遠方北坡雲杉林與山徑景物清晰如在眼前。"),
            ],
        ))
    } else if !is_outdoor_passage && (room_name.starts_with("大圳頭") || room_name.starts_with("牛埔")) {
        Some(make_peninsula_object(
            "wt_reg_plains_specialty",
            "陳年米酒甕",
            &["Look", "Taste", "Smell", "SeasonalBanquet"],
            &[
                ("Look", "厚實的防潮羊毛氈墊旁，立著大圳頭紅泥封口的陳年米酒甕、牛埔熟成硬乳酪木盤，院門邊還放著看家牧羊犬的食盆與春耕艾草青糰蒸籠。"),
                ("Taste", "你切下一小片牛埔硬乳酪配著溫熱米酒品嚐，濃郁奶香與甘醇酒香交融，回味綿長。"),
                ("Smell", "陣陣大圳頭新米酒香與艾草清香撲鼻而來，滿是西南沃野的富足氣息。"),
                ("SeasonalBanquet", "你取下陳年米酒甕拍開封泥，醇香四溢，與席間街坊共享歲時時令佳餚。"),
            ],
        ))
    } else if !is_outdoor_passage && (room_name.starts_with("紅土崁") || room_name.starts_with("三號坑口")) {
        Some(make_peninsula_object(
            "wt_reg_mining_specialty",
            "烤番薯砂鍋",
            &["Look", "Taste", "WarmHands", "SeasonalBanquet"],
            &[
                ("Look", "耐火紅磚砌成的矮爐上煨著一只紅土炭火砂鍋，爐灰裡埋著紅土崁沙地種出的紅心番薯，牆角還收著導能測紋石板與防潮紅磷引火籤盒。"),
                ("Taste", "你剝開一顆現烤出爐的烤紅心番薯，金黃番薯肉軟糯流蜜，甜香燙口。"),
                ("WarmHands", "你湊近紅土炭火砂鍋烘手，耐火紅陶散發著持久溫厚的熱力。"),
                ("SeasonalBanquet", "你掀開砂鍋陶蓋，炭火煨烤的時令美味香氣撲鼻，圍爐取暖，神完氣足。"),
            ],
        ))
    } else if !is_outdoor_passage && (room_name.starts_with("南哨") || room_name.starts_with("水尾寮")) {
        Some(make_peninsula_object(
            "wt_reg_wetland_specialty",
            "艾草驅蚊盤",
            &["Look", "Smell", "Use"],
            &[
                ("Look", "沉水烏木矮几上點著溫氏診所配製的驅蚊避瘴艾草香盤，旁側擺著水尾寮細編蘆葦涼席、雙層竹提籃與防雨油布雙人帳篷。"),
                ("Smell", "艾草與山薄荷的清涼草木香氣在屋內緩緩繚繞，將南境濕熱水氣與蚊蟲全數驅散。"),
                ("Use", "你展開細編蘆葦涼席試摸，竹篾與蘆葦平滑冰涼，正是水鄉避暑的良物。"),
            ],
        ))
    } else if !room_name.contains('街')
        && !room_name.contains('道')
        && !room_name.contains('巷')
        && !room_name.contains('橋')
        && (room_name.contains("公議") || room_name.contains("契據"))
    {
        Some(make_peninsula_object(
            "wt_cat_0793",
            "紅印屋契匣",
            &["Look", "Operate", "Read"],
            &[
                ("Look", "紫檀木架上擺著文曜公議堂專用的紅印屋契匣與戶籍租約摺，上面整齊列著白塔半島各市鎮的房產地籍規章。"),
                ("Operate", "你端整公議堂印泥盒並展開屋契摺頁，準備簽章畫押或核對租約地界。"),
                ("Read", "你展閱細讀紅印屋契摺，公議堂戶籍署硃砂大印分明，載明半島產權過戶與承租法度。"),
            ],
        ))
    } else if room_name.contains("學院") && !room_name.contains("觀星") && !room_name.contains("學院口") && !room_name.contains("學院巷") && !room_name.contains("前街") {
        Some(make_peninsula_object(
            "wt_cat_0734",
            "黃銅星軌儀",
            &["Look", "Operate", "Read", "ChartAstrolabe"],
            &[
                ("Look", "精銅底座上高聳著精密的三百六十度黃銅星軌儀，多層同心環刻有節氣、潮汐與天象經緯刻度。"),
                ("Operate", "你輕輕推動星儀外環，黃銅齒輪與星軌指針順滑流轉，對準了今夜即將升起的海角星宿。"),
                ("Read", "你俯身辨讀星軌儀底盤銘刻的文曜先賢測候律歷，字跡端凝入微。"),
                ("ChartAstrolabe", "你鋪開竹綿宣紙與松煙墨拓，循星軌儀子午刻度拓下個人周天命格星盤。"),
            ],
        ))
    } else if !room_name.contains('街')
        && !room_name.contains('道')
        && !room_name.contains('巷')
        && !room_name.contains('橋')
        && (room_name.contains("商行") || room_name.contains("商盟") || room_name.contains("票號"))
    {
        Some(make_peninsula_object(
            "wt_cat_0790",
            "分格存摺櫃",
            &["Look", "Read", "DepositBank", "WithdrawBank", "Use"],
            &[
                ("Look", "厚重鐵木所造的分格存摺櫃，抽屜銅扣皆刻有三水口流水徽印，收存著各大商隊與鎮民的通兌簿冊與印鑑。"),
                ("Read", "你細讀櫃頂張貼的三水總商行通兌章程，載明各港口通兌匯水與信用票號兌領條款。"),
                ("DepositBank", "你在帳房案前出示存摺與信印，將隨身鎂幣妥善存入商行票號帳頭。"),
                ("WithdrawBank", "你憑分格存摺防偽硃印與帳房核對銀數，自票號結存中支取現鎂。"),
                ("Use", "你走近分格存摺櫃，向值守執事出示存摺辦理商行通兌手續。"),
            ],
        ))
    } else if room_name.starts_with("文曜") {
        if is_wenyao_residential_or_study(room_name)
            || room_name.contains("住宅")
            || room_name.contains("民宅")
            || room_name.contains("居所")
            || room_name.contains("宅邸")
            || room_name.contains("宅院")
            || room_name.contains("民舍")
            || room_name.contains("內室")
            || room_name.contains("客房")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_home",
                "床榻",
                &["Look", "Use", "Rest", "Sleep", "Operate"],
                &[
                    ("Look", "溫暖乾燥的松香木大床上鋪著蓬鬆羽絨暖被與山風草安神枕，榻旁小几上立著一盞旋蓋夜光貝母檯燈，光暈柔和溫潤。"),
                    ("Use", "你在松香木大床上舒展四肢，蓬鬆羽絨暖被帶來融融暖意，周身疲勞與肌肉酸脹頓時消散大半，氣血與內息漸漸調和平復。"),
                    ("Rest", "你合衣在柔軟的羽絨大床上躺下歇息，安神草的清香讓緊繃的心神徹底放鬆下來，周身氣血漸復充盈。"),
                    ("Sleep", "你鑽入溫暖蓬鬆的羽絨暖被，旋暗夜光貝母檯燈。在靜謐祥和的屋內，你安然入眠，體魄與心神盡皆恢復。"),
                    ("Operate", "你輕輕旋轉床頭夜光貝母檯燈的烏木旋蓋，珠白光芒柔和微調，滿室皆是安恬的暖意。"),
                ],
            ))
        } else if room_name.contains("渡埠")
            || room_name.contains("水岸")
            || room_name.contains("河畔")
            || room_name.contains("渡口")
            || room_name.contains("泊位")
            || room_name.contains("竹筏")
            || room_name.contains("三水川")
            || room_name.contains("北渠")
            || room_name.contains("東灣")
            || room_name.contains("水脈")
            || room_name.contains("河灣")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_riverbank",
                "橋欄",
                &["Look", "Use", "Rest", "Operate"],
                &[
                    ("Look", "青石欄杆雕飾著渡海浪花紋樣，橋墩下方垂著粗壯熟鐵繫船環，川流自石階下緩緩匯入水灣，波光瀲灩。"),
                    ("Use", "你走近青石欄杆探看繫船環與泊位，粗壯鐵環深嵌石中，吃水牢靠平穩。"),
                    ("Rest", "你伏在微涼平整的石欄杆上眺望川流與來往竹筏，水風徐來，心曠神怡。"),
                    ("Operate", "你輕撫石欄上斑駁溫潤的浪花雕飾，指尖微涼，遠望水天相映。"),
                ],
            ))
        } else if room_name.contains("青竹苑")
            || room_name.contains("藥圃")
            || room_name.contains("苗圃")
            || room_name.contains("花圃")
            || room_name.contains("草圃")
            || room_name.contains("水芹圃")
            || room_name.contains("修性圃")
            || room_name.contains("本草園")
            || room_name.contains("收曬園")
            || room_name.contains("藥草")
            || room_name.contains("竹苑")
            || room_name.contains("垂柳苑")
            || room_name.contains("草廬苑")
            || room_name.contains("石池苑")
            || room_name.contains("竹林")
            || room_name.contains("桂花林")
            || room_name.contains("杏壇林")
            || room_name.contains("印池")
            || room_name.contains("清水池")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_garden",
                "竹骨引水筧",
                &["Look", "Use", "Smell", "Water"],
                &[
                    ("Look", "依水岸而闢的分畦藥圃內，黑土整飭如墨，種著山風草、紫蘇與清涼薄荷，上方架設著以破半青竹搭建的竹骨引水筧，引清泉潺潺注入畦間。"),
                    ("Use", "你撥動引水筧竹筒引水澆灌畦間藥草，清泉滲入泥土，泥土草木清香頓時撲面而來。"),
                    ("Smell", "深吸一口氣，混雜著濕潤泥土、新鮮山風草與清涼薄荷的草木清香沁人心脾。"),
                    ("Water", "你撥正竹骨引水筧的水流轉向，清涼泉水順著竹管注入畦間泥土，滋養著青翠幼苗。"),
                ],
            ))
        } else if room_name.contains("庭院")
            || room_name.contains("小院")
            || room_name.contains("院落")
            || room_name.contains("門前")
            || room_name.contains("門首")
            || room_name.contains("天井")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_courtyard",
                "防雨竹信箱",
                &["Look", "Use", "Open", "Water"],
                &[
                    ("Look", "院門粉牆邊掛著一只以生桐油浸潤的防雨細密竹編信箱，石階旁安放著紅土崁粗陶花盆，栽植著一株枝葉青翠欲滴的粉白山茶花。"),
                    ("Use", "你駐足端詳院落門前整潔的竹編信箱與山茶花，心神怡然。"),
                    ("Open", "你輕輕掀開竹編信箱的防水翻蓋，內裡乾燥整潔，墊著吸濕的竹綿薄紙。"),
                    ("Water", "你自水缸舀起半瓢清水澆灌在山茶花根部，紅土散發出質樸的泥土芬芳，花葉愈顯精神。"),
                ],
            ))
        } else if room_name.contains("內門坊") {
            Some(make_peninsula_object(
                "wt_reg_wenyao_gate_plaza",
                "拴馬柱",
                &["Look", "Rest", "Operate"],
                &[
                    ("Look", "青石坊門前立有一對雕刻流雲海濤紋樣的石雕望柱，柱底鑿有厚實生鐵栓馬環，過路行商常在此歇鞍飲馬。"),
                    ("Rest", "你倚在青石望柱基座旁放鬆筋骨，看著巡坊守衛與往來商旅自牌坊下穿行，井然有序。"),
                    ("Operate", "你拉動生鐵栓馬石環試了試牢固度，鐵環嵌合緊密，堅不可摧。"),
                ],
            ))
        } else if room_name.contains("街")
            || room_name.contains("巷")
            || room_name.contains("道")
            || room_name.contains("橋")
            || room_name.contains("廣場")
            || room_name.contains("平坪")
            || room_name.contains("草坪")
            || room_name.contains("石坪")
            || room_name.contains("曬網坪")
            || room_name.contains("草料坪")
            || room_name.contains("試刀坪")
            || room_name.contains("聚工坪")
            || room_name.contains("平波坪")
            || room_name.contains("迴舟坪")
            || room_name.contains("整隊坪")
            || room_name.contains("石台")
            || room_name.contains("石階")
            || room_name.contains("石磯")
            || room_name.contains("棚廊")
            || room_name.contains("草亭苑")
            || room_name.contains("牧草甸")
            || room_name.contains("清草塘")
            || room_name.contains("望遠坡")
            || room_name.contains("晾木場")
            || room_name.contains("放牧場")
            || room_name.contains("堆疊場")
            || room_name.contains("過磅台")
            || room_name.contains("外環")
            || room_name.contains("內環")
            || room_name.ends_with("隅")
            || room_name.ends_with("口")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_street",
                "告示欄",
                &["Look", "Use", "Read", "Operate"],
                &[
                    ("Look", "青石街角立著造型典雅的文曜街燈，燈罩內嵌夜光貝母薄片；燈柱旁設有黃銅鑲邊的公佈欄，張貼著近期的市集行曆與商會通告。"),
                    ("Use", "你走近公佈欄與青石街燈，細細閱覽各項告示與通告。"),
                    ("Read", "你走近公佈欄細閱，上面以工整楷體張貼著四坊更次、各坊供水時間以及近期大角原牛隊通商班次。"),
                    ("Operate", "你擦拭公佈欄上的落塵，字跡分明，一目了然。"),
                ],
            ))
        } else if room_name.contains("工坊")
            || room_name.contains("作坊")
            || room_name.contains("大鍛坊")
            || room_name.contains("廠")
            || room_name.contains("窯")
            || room_name.contains("鋪")
            || room_name.contains("鍛")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_workshop_bench",
                "工具架",
                &["Look", "Use", "Operate"],
                &[
                    ("Look", "厚木壁板上分門別類掛著精鋼手錘、鉗夾、銼刀與鋼角尺，下方是引自暗渠的活水冷卻石槽，水汽蒸騰。"),
                    ("Use", "你走近活水石槽浸手洗去油灰，清涼活水洗滌疲乏，令人神清氣爽。"),
                    ("Operate", "你伸手整理掛架上的工具，擺放井然有序，刃口重鎢冷光內斂。"),
                ],
            ))
        } else if room_name.contains("大埠")
            || room_name.contains("拍賣")
            || room_name.contains("大市")
            || room_name.contains("棧")
            || (room_name.contains("商行")
                || room_name.contains("貨運行")
                || room_name.contains("車馬行")
                || room_name.contains("聯運")
                || room_name.contains("糧行"))
            || (room_name.contains("庫")
                && !room_name.contains("藏書")
                && !room_name.contains("印行")
                && !room_name.contains("典藏"))
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_market_post",
                "貨架棚",
                &["Look", "Read", "Operate"],
                &[
                    ("Look", "場地正中立著高大的商會通商立柱，上掛夜光貝防風燈，周圍搭建著堅固的防水油布貨架棚，便於堆疊周轉大宗貨物。"),
                    ("Read", "你走近立柱端讀鐫刻的商貿契稅規章與每日各港口船班、車隊發貨時序。"),
                    ("Operate", "你拉緊貨棚一角的防風挽繩，油布緊繃抗風，遮蔽妥帖。"),
                ],
            ))
        } else if (room_name.contains("院")
            || room_name.contains("館")
            || room_name.contains("局")
            || room_name.contains("台")
            || (room_name.contains("所")
                && !room_name.contains("巡防")
                && !room_name.contains("哨所")
                && !room_name.contains("診所")
                && !room_name.contains("居所")
                && !room_name.contains("宿所")))
            && !room_name.contains("藏書")
            && !room_name.contains("印行")
        {
            Some(make_peninsula_object(
                "wt_reg_wenyao_study_bench",
                "杉木公文案",
                &["Look", "Read", "Use"],
                &[
                    ("Look", "開闊的杉木公案上鋪著白綿吸墨紙，案旁設有整齊的公文插架與黃銅鎮紙，透著嚴謹典雅的學風。"),
                    ("Read", "你翻看案頭的記錄名冊與行事日程，字跡工楷，條分縷析。"),
                    ("Use", "你於案前稍坐端整儀容，案几整潔，心無雜念。"),
                ],
            ))
        } else {
            Some(make_peninsula_object(
                "wt_reg_wenyao_amenities",
                "水栓",
                &["Look", "Use", "Bath"],
                &[
                    ("Look", "打磨光潔的文曜黃銅水栓連接著暗渠活水管，下方是一座嵌有半嶺溫石的保溫熱水槽，槽旁備有竹綿長絨擦面巾與松脂皂角。"),
                    ("Use", "你旋動黃銅水栓，清純甘洌的活水嘩嘩流淌而出，洗滌手面，神清氣爽。"),
                    ("Bath", "你引入溫石槽加熱的熱水浸沐洗面，熱氣騰騰驅散了旅途風塵與肌肉酸脹，倍感舒泰。"),
                ],
            ))
        }
    } else {
        None
    }
}

pub fn is_final_map_public_role(s: &str) -> bool {
    FINAL_MAP_SCENE_TEXTS
        .iter()
        .any(|scene| s.contains(scene.role))
}
pub fn public_person_label_for_room(room_name: &str) -> Option<&'static str> {
    if let Some(role) = final_map_public_role_for_room(room_name) {
        Some(role)
    } else if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        Some("百草房焙藥匠人")
    } else if room_name.contains("診所") || room_name.contains("藥房") {
        Some("診所司藥學徒")
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        Some("文曜曆算生")
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        Some("講堂街印書匠")
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") {
        Some("講堂街讀書人")
    } else if room_name.contains("習武堂") || room_name.contains("試招院") || room_name.contains("武勇舍") || room_name.contains("習武") || (room_name.contains("試招") && room_name.contains("文曜")) {
        Some("分家巷習武青年")
    } else if room_name.contains("樞紐石橋") || room_name.contains("拱橋") || room_name.contains("竹筏渡") {
        Some("三水口往來行商")
    } else if room_name.contains("藥香巷") || room_name.contains("爐火巷") || room_name.contains("百工石道") {
        Some("分家巷百工匠徒")
    } else if (room_name.starts_with("文曜") && is_wenyao_residential_or_study(room_name))
        || ((room_name.contains("居") || room_name.contains("宅") || room_name.contains("院") || room_name.contains("舍")) && room_name.contains("文曜"))
    {
        Some("文曜街坊住戶")
    } else if room_name.contains("總商行") || room_name.contains("商盟") {
        Some("總商行核算員")
    } else if room_name.contains("食堂") || room_name.contains("大食") {
        Some("三水食堂外場侍者")
    } else if room_name.contains("浴場") || room_name.contains("溫水") {
        Some("泉香浴場湯池管理員")
    } else if room_name.contains("旅社") || room_name.contains("茶屋") {
        Some("東橋茶屋茶點侍者")
    } else if room_name.contains("鍛造") || room_name.contains("打鐵") {
        Some("分家巷司爐鐵匠")
    } else if room_name.contains("巧木") || room_name.contains("傢俱") {
        Some("巧木坊刨木匠人")
    } else if room_name.contains("雲錦") || room_name.contains("織造") || room_name.contains("服飾") {
        Some("雲錦坊織繡女工")
    } else if room_name.contains("聯運") || room_name.contains("車馬") {
        Some("西門聯運趕車腳夫")
    } else if room_name.contains("鐘樓") || room_name.contains("信使") {
        Some("鐘樓快騎信使")
    } else if room_name.contains("習武") || room_name.contains("試招") {
        Some("習武堂教習教頭")
    } else if room_name.contains("剪剃") || room_name.contains("理髮") {
        Some("剪剃鋪修面師傅")
    } else if room_name.contains("柑仔店") {
        Some("柑仔店店員")
    } else if room_name.contains("修傘") || room_name.contains("補靴") {
        Some("修傘補靴手藝匠")
    } else if room_name.contains("講堂街") || room_name.contains("學院巷") || room_name.contains("書院巷") {
        Some("講堂街抱書學生")
    } else if room_name.starts_with("文曜") && room_name.contains("三水") {
        Some("三水口往來行商")
    } else if room_name.starts_with("文曜") && (room_name.contains("分家巷") || room_name.contains("百工")) {
        Some("分家巷百工匠徒")
    } else if room_name.contains("西門") {
        Some("西門行路旅客")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("看貨人")
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("水尾寮渡口") || room_name.contains("水尾寮渡頭") || room_name.contains("北岸") || room_name.contains("等候") {
        Some("候渡者")
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        Some("引船人")
    } else if room_name.contains("湖港木棧") {
        Some("候船者")
    } else if room_name.contains("短船泊位") {
        Some("船戶")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some("分揀工")
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some("採集者")
    } else if room_name.contains("水倉") {
        Some("倉管")
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some("碼頭工")
    } else if room_name.contains("稅站棚") {
        Some("稅吏")
    } else if room_name.contains("船行門面") {
        Some("船行人")
    } else if room_name.contains("潮線貨場") {
        Some("搬貨人")
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some("倉管")
    } else if room_name.contains("商船候泊線") {
        Some("船員")
    } else if room_name.contains("東南船塢口") {
        Some("船匠")
    } else if room_name.contains("修船架") {
        Some("修船工")
    } else if room_name.contains("補給倉") {
        Some("倉管")
    } else if room_name.contains("碼頭木階") {
        Some("船客")
    } else if room_name.contains("帆布曬場") {
        Some("補帆人")
    } else if room_name.contains("船戶街口") {
        Some("船戶")
    } else if room_name.contains("外貿碼頭") {
        Some("船員")
    } else if room_name.contains("舶司前棚") {
        Some("舶司吏")
    } else if room_name.contains("商館街口") {
        Some("商館人")
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some("倉管")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯")
    } else if room_name.contains("匯兌小屋") {
        Some("兌票人")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some("船戶")
    } else if room_name.contains("港灣貨棚") {
        Some("搬魚人")
    } else if room_name.contains("漁網曬場") {
        Some("補網人")
    } else if room_name.contains("冷霧魚市") {
        Some("魚販")
    } else if room_name.contains("船戶屋排") {
        Some("船戶家屬")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("守望人")
    } else if room_name.contains("礦坑外棚") {
        Some("礦工")
    } else if room_name.contains("洗礦溝") {
        Some("洗礦工")
    } else if room_name.contains("粗料倉") {
        Some("倉管")
    } else if room_name.contains("工具修棚") {
        Some("修具工")
    } else if room_name.contains("礦渣坡") {
        Some("推車人")
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("車夫")
    } else if room_name.contains("糧倉外秤") {
        Some("秤手")
    } else if room_name.contains("倉門前場") {
        Some("守倉人")
    } else if room_name.contains("曬場邊") {
        Some("翻穀人")
    } else if room_name.contains("車行口") {
        Some("車夫")
    } else if room_name.contains("守倉棚") {
        Some("倉管")
    } else if room_name.contains("糧車泊位") {
        Some("搬糧人")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("看貨人")
    } else if room_name.contains("歇腳") {
        Some("歇腳旅人")
    } else if room_name.contains("踏板") {
        Some("候船者")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("上岸客")
    } else if room_name.contains("船戶") || room_name.contains("作業") {
        Some("渡工")
    } else if room_name.contains("倉棚") {
        Some("搬貨人")
    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "")
        .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
    {
        Some(venue.default_host_title)
    } else {
        None
    }
}
pub fn scene_observation_line(room_name: &str, pronoun: &str) -> Option<String> {
    if room_name.contains("烘藥房") || room_name.contains("百草烘藥") {
        Some(format!("{pronoun}正在雙層紅泥焙藥爐前專注微調通氣瓦片，翻動竹篩中的切片藥草，濃郁焙藥香氣在身側繚繞。"))
    } else if room_name.contains("診所") || room_name.contains("藥房") {
        Some(format!("{pronoun}正手持白瓷藥碾在研缽中研磨草藥粉末，動作沉穩細緻，空氣中散發著清涼的草木薄荷香。"))
    } else if room_name.contains("曆算") || room_name.contains("星曆") || room_name.contains("推步") {
        Some(format!("{pronoun}端坐於黑檀木計數板案前，指尖在滑鈕間飛快撥動，目光凝視著黃銅渾天星軌推步儀的子午刻度。"))
    } else if room_name.contains("印行館") || room_name.contains("造紙") || room_name.contains("典藏庫") {
        Some(format!("{pronoun}手持棕櫚墨刷在排版梨木活字上勻勻刷著松煙墨，隨後利落覆上竹綿宣紙滾壓印製。"))
    } else if room_name.contains("書齋") || room_name.contains("校勘齋") || room_name.contains("墨香居") {
        Some(format!("{pronoun}在沉水烏木書案前執筆抄校線裝古籍，窗外竹影搖曳，室內唯聞筆尖游走宣紙的沙沙微響。"))
    } else if room_name.contains("習武堂") || room_name.contains("試招院") || room_name.contains("武勇舍") || room_name.contains("習武") {
        Some(format!("{pronoun}在練武場青石板上紮穩馬步，手持包革練習木刀專注推演刀式，吐納均勻沉實。"))
    } else if (room_name.starts_with("文曜") && is_wenyao_residential_or_study(room_name))
        || ((room_name.contains("居") || room_name.contains("宅") || room_name.contains("院") || room_name.contains("舍")) && room_name.contains("文曜"))
    {
        Some(format!("{pronoun}在宅院門廊下收拾防雨竹編信箱與山茶盆栽，見有行客路過，和善地點頭致意。"))
    } else if room_name.contains("樞紐石橋") || room_name.contains("拱橋") || room_name.contains("竹筏渡") {
        Some(format!("{pronoun}正倚在雕花青石欄杆旁清點隨身貨單，招呼著川流泊位上的竹筏渡伕。"))
    } else if room_name.contains("藥香巷") || room_name.contains("爐火巷") || room_name.contains("百工石道") {
        Some(format!("{pronoun}正坐在門前條石上磨礪小刀與鑿子，不時抬手拭去額前微汗，神情專注踏實。"))
    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "")
        .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
    {
        if let Some((ev, _sp)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
            room_name,
            None,
            crate::game::now_unix(),
        ) {
            let primary_obj = ev.key_objects.split('、').next().unwrap_or(ev.key_objects).trim();
            if venue.has_counter_restricted_zone {
                Some(format!(
                    "{pronoun}正立於{}當值櫃檯內側，{}，案頭備有{}。",
                    venue.venue_kind,
                    venue.on_duty_fixture_activity,
                    primary_obj
                ))
            } else {
                Some(format!(
                    "{pronoun}正在{}內{}，手頭正料理著{}。",
                    venue.venue_kind,
                    venue.on_duty_fixture_activity,
                    primary_obj
                ))
            }
        } else if venue.has_counter_restricted_zone {
            Some(format!(
                "{pronoun}正立於{}當值櫃檯內側，{}，{}",
                venue.venue_kind, venue.on_duty_fixture_activity, venue.on_duty_demeanor
            ))
        } else {
            Some(format!(
                "{pronoun}正在{}內{}，{}",
                venue.venue_kind, venue.on_duty_fixture_activity, venue.on_duty_demeanor
            ))
        }
    } else if let Some(ambient) = crate::grid::resolve_room_ambient_visitor_activity(room_name, 0) {
        Some(format!("{pronoun}正在此處{}", ambient))
    } else {
        None
    }
}


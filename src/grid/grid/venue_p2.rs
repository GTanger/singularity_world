fn resolve_room_venue_object_p2(combined: &str, room_tags: &[String]) -> Option<RoomVenueObject> {
    // 19. 剪剃鋪/理容
    if ["剪剃鋪", "理容"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "理容剪剃鋪",
            default_host_title: "當值剃頭師傅",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正推磨精鋼摺疊剃刀並擰乾熱毛巾替客修面理容",
            on_duty_demeanor: "年逾六旬的理髮鋪老匠師，穿著一塵不染的白色工作衫，精瘦結實，笑容爽朗手腕極穩，工具袋裡插著磨刀布與牛角細齒梳。",
            lendable_tool: Some((
                "wt_cat_0630",
                "理髮鋪精鋼摺疊剃刀",
                "這把「理髮鋪精鋼摺疊剃刀」刃口推磨得吹毛斷髮，借你修整儀容，用時當心手下分寸。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0630", "理髮鋪精鋼摺疊剃刀", 12),
                ("wt_cat_0617", "牛尾軟毛竹柄潔齒刷", 4),
            ],
            trade_talk_pitch: "「特製的精鋼摺疊修容剃刀與牛骨柄豬鬃潔齒刷，皆是隨身洗漱佳品，鋼口鋒利耐用，買上一把受用多年。」",
            on_duty_talk_lines: &[
                "「哈哈，快請坐上紫銅躺椅！熱毛巾敷透鬚角，店裡這把精鋼摺疊剃刀保證又快又順，絕不刮破半點皮！」",
                "「講堂街的書院生員、公議堂的辦事員，哪一個修剪打理不是在店裡拾掇的？清清爽爽才見神采！」",
                "「修面要腕穩，洗頭要水溫恰好。紫銅水管裡的井水常年溫熱，躺下歇息片刻，包你神清氣爽。」",
            ],
        });
    }

    // 20. 浴場/溫泉
    if ["浴場", "溫泉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "溫泉浴場",
            default_host_title: "當值湯番",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正往寒水玉湯池投放祛寒草木湯包",
            on_duty_demeanor: "氣色紅潤、神情悠閒的浴場掌事，穿著寬鬆透氣的灰白亞麻休閒衫，腳穿防滑木底便履，腰間掛著竹牌，周身帶著淡淡的松脂香胰與溫水蒸氣清香。",
            lendable_tool: Some((
                "wt_cat_0619",
                "長絨蜂格棉面巾",
                "這條新烘乾的「長絨蜂格棉面巾」鬆軟吸水，借你擦拭風塵汗水，洗淨晾乾再還便可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0620", "茶籽松脂香胰", 4),
                ("wt_cat_0619", "長絨蜂格棉面巾", 5),
            ],
            trade_talk_pitch: "「架上有北山茶籽松脂香胰與大圳長絨蜂格棉面巾，吸水柔潤，買上一份進池，包你洗得通體舒暢。」",
            on_duty_talk_lines: &[
                "「舟車勞頓，身心俱疲了吧？快進熱湯池泡上一刻鐘！這泉水引自老堰頭活脈，底層鋪滿寒水玉，最能活絡筋骨。」",
                "「浴場用的是北山茶籽加松脂熬煉的手工香胰，去垢不傷膚，洗完身上泛著淡淡山林松香，清爽怡人。」",
                "「城中醫師常說，泡湯乃平民溫養第一妙法。發一身透汗、飲半盞清茶，什麼鬱結勞碌全給融化了。」",
            ],
        });
    }

    // 21. 織造/成衣/染坊/服飾/裁縫
    if room_tags.iter().any(|t| matches!(canonical_room_tag(t), "textile" | "fabric"))
        || ["織造", "成衣", "染坊", "服飾", "裁縫"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "織造染坊",
            default_host_title: "當值織匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正為遠行採購商量裁黑沼防雨樹膠長衫",
            on_duty_demeanor: "儀態端莊溫婉的服飾坊掌事，身穿親手剪裁的煙青色棉麻套裝長裙，腰間配著軟皮布尺，長髮整齊挽起，目光敏銳俐落。",
            lendable_tool: Some((
                "wt_cat_0835",
                "細棉麻透氣常服",
                "這件「細棉麻透氣常服」針腳細密、吸汗透氣，借你替換沾濕的衣衫，穿畢洗淨歸還即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0835", "細棉麻透氣常服", 14),
                ("wt_cat_0838", "樹膠防水防雨長衫", 18),
                ("wt_cat_0868", "素面玄武岩扣束腰長袍", 16),
            ],
            trade_talk_pitch: "「素面玄武岩扣剪裁外套、黑沼防雨連帽長衫皆有現貨，各色尺碼齊全，試穿合身便可直接交割。」",
            on_duty_talk_lines: &[
                "「請進。撫摸一下這匹大圳細棉麻，質地柔軟透氣，貼身裁件常服，盛暑天行路也不生悶汗。」",
                "「黑沼樹膠防雨長衫是坊裡的招牌，外塗三層密膠、內襯透氣薄棉，狂風驟雨打在身上順水滑落，滴水不沾。」",
                "「衣裝乃行者門面。無論治學士還是走南闖北的商旅，換上一身得體耐磨的剪裁外套，整個人便精神三分。」",
            ],
        });
    }

    // 22. 聯運/貨運/車隊/車馬
    if ["聯運", "貨運", "車隊", "車馬"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "車馬聯運站",
            default_host_title: "當值車頭",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調度雙輪駝獸大車裝卸四方貨件",
            on_duty_demeanor: "膚色黝黑、嗓音洪亮的聯運站長，身材魁梧結實，身穿厚實毛氈翻領背心，臂膀結實有力，眼神沉穩敏銳，手腕戴著防磨牛皮護腕。",
            lendable_tool: Some((
                "wt_cat_0891",
                "黃銅雙扣厚帆布斜背包",
                "這只「黃銅雙扣厚帆布斜背包」耐磨防潑水、負重省力，借你奔走四方裝載什物。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
                ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
            ],
            trade_talk_pitch: "「雙肩厚帆布遠行背囊與雙層烏木內膽保溫瓶皆是長途必備硬貨，容量大、保溫久，旅人獵戶必帶一具。」",
            on_duty_talk_lines: &[
                "「嘿！聯運車隊風雨無阻！無論是走風舶角的大宗海貨，還是運往大圳頭的新麥礦石，我們車隊從不誤期！」",
                "「出遠門行囊要緊湊。隨身背囊要用雙肩加厚帆布，防雨布套扣緊，遇上海嵐濃霧或顛簸山徑才不會散架。」",
                "「草獸性子溫順但耐力驚人，日行百里不喘粗氣。若想搭車去外鎮，先到票檯開好路單，鐘點一到準時發車！」",
            ],
        });
    }

    // 23. 水輪/石磨
    if room_tags.iter().any(|t| canonical_room_tag(t) == "grindstone")
        || ["水輪", "石磨"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "水輪石磨工坊",
            default_host_title: "當值石磨匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正引水推動玄武岩大石磨細磨新收稻穀與米漿",
            on_duty_demeanor: "樸實沉穩的水輪石磨匠師，穿著沾有細白米粉的粗麻雙帶圍胸短褂，專注傾聽著水輪與雙疊石磨運轉的節奏。",
            lendable_tool: None,
            counter_trade_offers: &[
                ("wt_cat_0388", "現磨溫米漿", 3),
                ("wt_cat_0277", "秋收蓬萊新米", 6),
            ],
            trade_talk_pitch: "「水輪石磨現磨的新米漿與精選糙米裝袋整齊，米香純正，買上兩袋帶回煮粥最是養人。」",
            on_duty_talk_lines: &[
                "「聽這石磨旋轉的水輪聲，水流勻稱，磨出的米粉才細滑如脂。」",
                "「老堰頭引來的高山活水衝力十足，帶動三座玄武岩雙疊大磨日夜不歇，供全鎮每日炊食之需。」",
                "「磨新稻要分兩道工，頭道脫殼、次道出漿，火候全在掌磨人這雙耳朵和掌心溫度裡。」",
            ],
        });
    }

    // ── 外圍聚落工坊與特定職司原型 ──

    // 24. 海事商盟
    if ["海事", "商盟"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "海事商盟",
            default_host_title: "商盟會長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢視碼頭泊位進港貨單與海潮預報",
            on_duty_demeanor: "面容堅毅、鬢角斑白的海事商盟老會長，穿著厚實防風毛呢大衣，目光沉穩如礁石，歷經數十年海上風浪依然從容不迫。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "出海靠天也靠盤，這枚「外澳夜光貝領航羅盤」借你隨船導航，切記防潮妥存。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
                ("wt_cat_0732", "外澳夜光貝領航羅盤", 15),
                ("wt_cat_1000", "深海巨藻船纜", 18),
            ],
            trade_talk_pitch: "「風舶海事商盟專備遠航海圖、夜光羅盤與深海船纜，經得起狂濤巨浪，隨船必備。」",
            on_duty_talk_lines: &[
                "「看海潮如閱人面，潮頭平緩則水下暗流湧動，行船半點大意不得。」",
                "「商盟立足港灣數十年，靠的就是每艘入港大船平安歸來，船貨互通、信義長存。」",
                "「港外近來風浪稍歇，若要啟航出海，先去碼頭驗核通關路單。」",
            ],
        });
    }

    // 25. 遠洋航務
    if ["遠洋", "船長"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "遠洋航務社",
            default_host_title: "遠洋船長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於航海圖案前量測外海航線與補給點",
            on_duty_demeanor: "英氣十足的遠洋大船女船長，身穿合身防水航海短襖，長髮高束，隨身帶著海圖筒，眼神銳利自信。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "這具夜光領航羅盤走針靈敏，借你出洋辨識水道，歸航時送回船署即可。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
                ("wt_cat_1000", "深海巨藻船纜", 18),
            ],
            trade_talk_pitch: "「遠洋航務處備有精準海圖冊與深海巨藻船纜，遠航必不可少。」",
            on_duty_talk_lines: &[
                "「外海雲湧雷鳴之際，船長的手握緊舵輪，全船三十條性命便全在這掌心沉穩之中。」",
                "「遠洋航線不比內河，暗礁密布、水文莫測，每一筆海圖標註都是前人用命換來的。」",
                "「風向已轉向偏東，趁著潮頭正好，補給齊備的大船便要解纜啟航。」",
            ],
        });
    }

    // 26. 造船塢 / 船塢
    if ["造船", "船塢"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "造船修船塢",
            default_host_title: "總造船師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以木工曲尺審視龍骨平直度與捻縫填膏",
            on_duty_demeanor: "體格魁梧的總造船師，穿著耐磨牛皮工作服，手持木工曲尺，身上帶著百年雲杉與防水桐油的氣息。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口木匠箱裡有修造船板的平鑿與曲尺，借你維修棧道船體，用完歸還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1000", "深海巨藻船纜", 18),
                ("wt_cat_1002", "桐油海貝灰捻縫灰膏", 8),
                ("wt_cat_1003", "極淺吃水平底竹筏", 25),
                ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
            ],
            trade_talk_pitch: "「船塢特製海藻船纜、捻縫桐油灰膏與平底竹筏皆有現貨，水上營生最講究器具精實。」",
            on_duty_talk_lines: &[
                "「一艘好船能載萬石行千里，龍骨若有一絲偏差，到了驚濤駭浪中便會生生扭斷。」",
                "「捻縫灰膏需用百年桐油調合細研貝灰，抹進船縫凝固後堅逾鐵石，滴水不漏。」",
                "「港裡那艘吃水深平底大船正在鋪設外板，缺熟練木工幫忙拉緊夾板繩索。」",
            ],
        });
    }

    // 27. 農牧總社 / 糧倉
    if ["農牧", "穀倉", "糧倉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "農牧總社",
            default_host_title: "農牧理事長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正翻查各墾區秋收稻穀入倉清單與草料儲備",
            on_duty_demeanor: "面容溫和、膚色古銅的農牧總社理事長，捲起麻布短衫袖口，雙手布滿厚繭，神情踏實從容。",
            lendable_tool: Some((
                "wt_cat_0895",
                "雙層竹編提籃",
                "這只雙層竹編提籃通風透氣，借你裝運田間蔬果糧米，輕便合用。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0277", "秋收蓬萊新米", 6),
                ("wt_cat_0950", "省力導能翻土犁", 16),
                ("wt_cat_1020", "手搖螺旋壓實乾草料壓捆機", 16),
            ],
            trade_talk_pitch: "「大圳頭新收蓬萊米、省力翻土犁與草料壓捆機都在倉裡，農忙備耕正是時候。」",
            on_duty_talk_lines: &[
                "「稼穡艱辛，春耕夏耘秋收冬藏，每一粒米都凝聚著泥土與汗水。」",
                "「今年大圳頭灌溉水路通暢，各莊稻穗顆粒飽滿，糧倉充實，四鎮百姓心裡才踏實。」",
                "「草料儲備要搶在霜降前曬乾壓捆，冬日牲口吃得飽足，開春才有氣力耕田拉車。」",
            ],
        });
    }

    // 28. 礦場坑口
    if room_tags.iter().any(|t| canonical_room_tag(t) == "mine")
        || ["礦務", "坑口"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "礦務坑口司",
            default_host_title: "礦務掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正督導礦車出坑並秤量赤鐵礦石品位",
            on_duty_demeanor: "體格結實的礦務掌事，穿著耐火皮革背心，眼神專注銳利，指間常帶著赤鐵礦粉的痕跡。",
            lendable_tool: Some((
                "wt_cat_0957",
                "鍛鋼小羊角錘",
                "這柄鍛鋼小錘硬度極高，借你探礦敲石取樣，莫落入深坑之中。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0097", "相思木硬炭", 6),
                ("wt_cat_0957", "鍛鋼小羊角錘", 10),
                ("wt_cat_0846", "卸力厚牛皮護腕", 14),
            ],
            trade_talk_pitch: "「坑口精選重礦相思木硬炭、採礦鍛鋼錘與護腕現貨齊全，下坑進山最需穩實裝備。」",
            on_duty_talk_lines: &[
                "「地底百尺坑道裡，岩層聲響便是警兆，凡下坑者務必眼明心細、嚴守通風。」",
                "「三號坑出的赤鐵礦品位極高，送進第一鍛造工坊的高溫爐，能出上等精鋼。」",
                "「坑口軌道日夜穿梭，每一車礦石拉上來，全島的鐵工器脈便又多了一分底氣。」",
            ],
        });
    }

    // 29. 燈塔燻坊
    if ["燈塔", "燻"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "燈塔燻坊",
            default_host_title: "燈塔巡守匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正維護燈塔透鏡燃芯並翻動松木燻魚架",
            on_duty_demeanor: "守望燈塔的資深匠師，穿著防風厚帆布短襖，目光望向海面，身上帶著冷燻松木屑與海風的乾爽氣息。",
            lendable_tool: Some((
                "wt_cat_0898",
                "防潮紅磷引火籤盒",
                "海邊潮氣重，這盒封蠟引火籤借你點火照明，遇風不滅。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0319", "海風脫水小蝦米", 6),
                ("wt_cat_0320", "乾製海生干貝柱", 10),
                ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
            ],
            trade_talk_pitch: "「燈塔特製冷燻乾貝、脫水海味與防風貝母夜燈皆在架上，海客行船最愛這口乾香。」",
            on_duty_talk_lines: &[
                "「燈塔之光是夜航者的眼睛，三十年風雨狂嘯，這盞主燈從未熄滅過一刻。」",
                "「底層燻坊用的是老松木冷燻，海風慢慢吹透，魚肉干貝緊實生香，能存上半年不壞。」",
                "「潮汐變化與天邊雲團相生相伴，看懂了燈塔外的大海，便懂了天道運行。」",
            ],
        });
    }

    // 30. 鑿石切玉坊
    if ["鑿石", "切石", "寒水玉"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "切石玉作坊",
            default_host_title: "切石匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正引銀砂流水切石機細剖寒水玉原石",
            on_duty_demeanor: "寒水玉切石匠師，神情沉靜專注，繫著防濺工作圍裙，仔細檢查切石水槽中的切面平整度。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "這塊測紋石板刻度精準，借你比對玉石平整度與導能紋路，小心輕放。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0559", "寒水玉雙層保鮮石櫃", 35),
                ("wt_cat_0949", "銀砂水力切石機", 22),
            ],
            trade_talk_pitch: "「水槽剖出的寒水玉質地清透、天然蓄寒，打造成雙層保鮮石櫃，盛夏蔬果久放如新。」",
            on_duty_talk_lines: &[
                "「切玉講究順應石脈，硬取硬鑿只會碎成亂石。銀砂細磨、以柔克堅，才見玉石內蘊靈性。」",
                "「北嶺山溪沖刷而下的寒水玉，蘊含冰川冷氣，凡醫館藏藥、食堂保鮮皆賴此物。」",
                "「下刀如運筆，水流衝力與砂粒粗細絲絲入扣，每一片薄板都需經三遍拋光。」",
            ],
        });
    }

    // 31. 水動鋸木坊
    if room_tags.iter().any(|t| canonical_room_tag(t) == "lumber")
        || ["鋸木"].iter().any(|k| combined.contains(k))
    {
        return Some(RoomVenueObject {
            venue_kind: "水動鋸木坊",
            default_host_title: "水車鋸木匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操控水車傳動大鋸剖開巨幅原木",
            on_duty_demeanor: "水車鋸木匠師，身材挺拔精悍，工作服上沾著細碎木屑，檢視木材紋理時眼神極為專注。",
            lendable_tool: Some((
                "wt_cat_0956",
                "柯氏松木家用工具箱",
                "這口工具箱備有木尺與推刨，借你修整木料，用完送回鋸木坊。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0947", "水力原木大鋸", 20),
                ("wt_cat_0956", "柯氏松木家用工具箱", 14),
            ],
            trade_talk_pitch: "「鋸木坊精剖雲杉厚木料、水力原木大鋸與家用木工箱皆備妥，建屋打傢俱皆用得上。」",
            on_duty_talk_lines: &[
                "「巨木排山倒海而來，水車運轉如雷鳴。看木紋走向下鋸，木板平直不翹曲。」",
                "「北山優質老雲杉，生長極緩，質地細密柔韌，是全城營造屋樑與大船龍骨的首選。」",
                "「水動大鋸全憑老堰頭活水驅動，省工省力，一日能剖數十根巨材。」",
            ],
        });
    }

    // 32. 牛埔乳酪毛氈牧坊
    if ["乳酪", "毛氈", "牧場"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "乳酪毛氈牧坊",
            default_host_title: "牧坊大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正壓濾新鮮乳酪並梳理熟羊毛壓製厚毛氈",
            on_duty_demeanor: "牛埔的乳酪與毛氈匠師，穿著厚毛氈背心，神態溫和開朗，雙手熟練地處理羊毛與乳製品。",
            lendable_tool: Some((
                "wt_cat_0891",
                "黃銅雙扣厚帆布斜背包",
                "這口雙扣背囊容量大、結實耐用，借你裝盛行路物資。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0501", "穩火慢煮醇厚鮮奶茶", 5),
                ("wt_cat_0836", "飛鼠厚絨禦寒冬衣", 22),
                ("wt_cat_0837", "雲豹毛領禦寒大氅", 28),
            ],
            trade_talk_pitch: "「新鮮乳酪乾、現煮醇厚鮮奶茶與厚絨禦寒毛氈衣物現貨充足，過冬遠行溫暖隨行。」",
            on_duty_talk_lines: &[
                "「牛埔草質肥美，牧群食百草甘泉，產出的乳汁脂香四溢，熬成濃縮乳酪能存數月。」",
                "「秋季羊毛剪下後需經三道水洗去脂，再以溫水重壓成氈，防風禦寒天下無雙。」",
                "「喝一碗現煮熱鮮奶茶，滿口醇香，身上寒氣頓時全消。」",
            ],
        });
    }

    // 33. 耐火磚窯
    if ["磚窯", "耐火磚"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "耐火磚窯坊",
            default_host_title: "磚窯匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正監看高溫窯爐燒製耐火磚並檢視冷卻磚色",
            on_duty_demeanor: "耐火磚窯資深匠師，膚色帶著窯火烘烤的紅潤，戴著厚牛皮隔熱袖套，對高溫燒製的耐火磚品質把關嚴格。",
            lendable_tool: Some((
                "wt_cat_0735",
                "導能測紋石板",
                "借你這塊測紋石板量測磚體規格與熱脹係數，切莫磕碰缺角。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0944", "紅土耐火磚高溫鐵爐", 20),
                ("wt_cat_0023", "粗陶紅黏土", 5),
                ("wt_cat_0097", "相思木硬炭", 6),
            ],
            trade_talk_pitch: "「窯裡新出爐一批耐火磚與高溫耐火鐵爐，能抗千度烈火，各工坊爐台修造不可或缺。」",
            on_duty_talk_lines: &[
                "「燒耐火磚全看黏土成色與窯火溫度，火候不足則鬆散易裂，火過則釉化失其蓄熱之能。」",
                "「半島各坊的鍛造爐、食堂大灶皆用我們窯裡出的紅土火磚，百年不塌不裂。」",
                "「出窯需等足三天自然冷卻，急著開窯吹了冷風，滿窯好磚便前功盡棄。」",
            ],
        });
    }

    // 34. 重型獵具坊
    if combined.contains("獵具") {
        return Some(RoomVenueObject {
            venue_kind: "重型獵具坊",
            default_host_title: "獵具大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正組裝彈簧複合獵網與鍛造獵叉配件",
            on_duty_demeanor: "重型獵具大匠，身材魁梧結實，穿著防刮厚皮護具，雙手沉穩有力，組裝大型獵網與複合獵具十分俐落。",
            lendable_tool: Some((
                "wt_cat_1021",
                "岩羊角手柄折疊獵刀",
                "這柄折疊獵刀鋒利耐用，借你山林防身或破開荊棘，回城時擦拭乾淨交還。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
                ("wt_cat_0846", "卸力厚牛皮護腕", 14),
                ("wt_cat_0840", "鍛造坊厚牛皮防燙圍裙", 16),
            ],
            trade_talk_pitch: "「岩羊角折疊獵刀、耐磨皮護腕與獵隊防護具齊備，出入深山大澤保你安然無虞。」",
            on_duty_talk_lines: &[
                "「進深山會巨獸，靠的不僅是力氣，更是稱手的獵具與冷靜心性。」",
                "「大型獵網用的是巨藻海纜與熟銅絞線合編，再狡黠兇猛的野獸也掙脫不得。」",
                "「出山打獵前，先仔細檢查刀刃開口與機簧卡榫，手頭器物可靠，命才保得住。」",
            ],
        });
    }

    // 35. 水尾寮竹筏渡口
    if ["高腳屋", "竹筏", "水尾寮"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "竹筏渡口坊",
            default_host_title: "竹筏匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正火烤老青竹彎折筏頭並編紮防水浮筒",
            on_duty_demeanor: "水尾寮的高腳屋與竹筏匠師，身形精瘦靈活，腰間繫著防滑繩索，在淺澤木棧道上行走自如。",
            lendable_tool: Some((
                "wt_cat_1006",
                "老青竹黃銅輪捲線釣竿",
                "這根老青竹釣竿彈性十足，借你在渡口淺灘垂釣魚蝦，順便消磨時光。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1003", "極淺吃水平底竹筏", 25),
                ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
            ],
            trade_talk_pitch: "「水尾寮特造平底竹筏與老竹捲線釣竿，吃水極淺，穿梭蘆葦淺澤平穩自如。」",
            on_duty_talk_lines: &[
                "「水尾寮水淺灘多，尋常大船進不來，只有這種平底竹筏才能如履平地。」",
                "「竹子要挑三年生老青竹，火烤出青油後結實耐泡，在水裡泡幾年都不發爛。」",
                "「坐筏渡河順水漂，遇急流橫篙一點，水勢再急也翻不了筏。」",
            ],
        });
    }

    // 36. 山徑補給行
    if ["山徑", "補給"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "山徑補給行",
            default_host_title: "補給掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正清點防倒風生鐵柴火爐與保溫乾糧行囊",
            on_duty_demeanor: "健談熱心的山徑補給行負責人，穿著防風厚背心，腰間工具帶插著修鞋工具，笑著招呼往來山路的採集員與旅人。",
            lendable_tool: Some((
                "wt_cat_0897",
                "薄層重礦烏木膽隨身保溫瓶",
                "山道風寒露重，這只保溫瓶借你隨身裝盛熱湯熱茶，溫暖驅寒。",
            )),
            counter_trade_offers: &[
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
                ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
                ("wt_cat_1037", "防倒風野營生鐵柴火爐", 15),
            ],
            trade_talk_pitch: "「防倒風生鐵柴火爐、雙扣斜背包與烏木保溫瓶，山路行路必備三大件，備齊出發免受風露寒侵。」",
            on_duty_talk_lines: &[
                "「翻過這座埡口就是黑沼與深山，外頭氣候多變，行囊打理得當才能走得安心。」",
                "「野營過夜最怕生火受潮，生鐵柴火爐擋風防雨，幾根枯枝就能煮出一鍋熱湯。」",
                "「往來山徑三十年，這間補給行就是行山人的家，進來坐坐喝口熱水歇歇腳。」",
            ],
        });
    }

    // 37. 巡防哨所
    if ["巡防", "防務"].iter().any(|k| combined.contains(k)) {
        return Some(RoomVenueObject {
            venue_kind: "巡防哨所",
            default_host_title: "哨所隊長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正登台瞭望外圍水澤動靜並校閱防務巡行排班",
            on_duty_demeanor: "身材修長矯健的防務隊長，身穿深綠色防刮獵裝，眼神如鷹隼般敏銳，時刻留意著林間與水澤的動靜。",
            lendable_tool: Some((
                "wt_cat_0732",
                "外澳夜光貝領航羅盤",
                "巡林辨位不可短少，這枚夜光羅盤借你巡視野外辨明北辰度數。",
            )),
            counter_trade_offers: &[
                ("wt_cat_1064", "巡山獵人隨身急救皮囊", 12),
                ("wt_cat_0846", "卸力厚牛皮護腕", 12),
                ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
            ],
            trade_talk_pitch: "「哨所常備巡山急救皮囊、卸力護腕與厚帆布巡山背包，野外巡行有備無患。」",
            on_duty_talk_lines: &[
                "「哨所日夜警惕，外圍沼澤與山林若有異常動靜，烽火快騎隨時出動。」",
                "「巡行野外最忌落單冒進，隨身帶好急救皮囊與指北羅盤，安全返回哨所為要。」",
                "「凡途經此處的過路行商，皆需在哨所登記簿留下行蹤與目的地，以備策應。」",
            ],
        });
    }
    None
}

/// 依據市集攤位、專門店鋪、倉庫、碼頭與街巷解析一等房間物件契約（RoomVenueObject）
///
/// 涵蓋 28 種市集攤販、專業作坊、商棧倉庫、驛站碼頭與公共街巷類型，
/// 統一封裝常備交易物件（counter_trade_offers）與當值對話語錄（on_duty_talk_lines）。
pub fn resolve_market_or_specialty_room_object(
    room_name: &str,
    extra_hint: &str,
) -> Option<RoomVenueObject> {
    if is_private_residence_room_name(room_name) {
        return None;
    }
    if is_street_or_gate_room_name(room_name) {
        return Some(RoomVenueObject {
            venue_kind: "街巷橋樑公共空地",
            default_host_title: "街坊居民",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於石橋欄杆旁駐足歇息，觀看街面人流往來",
            on_duty_demeanor: "神情安詳從容的街坊居民，安居樂業，對城中各坊動向頗為熟絡。",
            lendable_tool: None,
            counter_trade_offers: &[],
            trade_talk_pitch: "",
            on_duty_talk_lines: &[
                "「文曜城裡的街道很平整，作息也很規律。白天各行各業正常營業，到了晚上鐘樓報時收班，走在街上很安心。」",
                "「三水口連通外港，分家巷聚集各類工坊，講堂街則是學院與書局的所在地，城裡機能很齊全。」",
                "「初來文曜嗎？如果要找各處場所，順著三水匯流大石橋往四面看，各家招牌都很清楚。」",
            ],
        });
    }
    let combined = format!("{room_name} {extra_hint}");
    // 1. 鮮魚水產市集
    if combined.contains("鮮魚") || combined.contains("魚市") || combined.contains("海產") || combined.contains("水產") || combined.contains("活魚") || combined.contains("蝦蟹") || combined.contains("漁販") || combined.contains("魚販") || combined.contains("三水河鮮") || combined.contains("早市鮮魚拍賣場") || combined.contains("風舶早市菜場") || combined.contains("北灣早市菜場") {
        return Some(RoomVenueObject {
            venue_kind: "鮮魚水產市集",
            default_host_title: "當值魚販",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於碎冰魚槽前分揀鮮魚並以柳條穿鰓",
            on_duty_demeanor: "腰繫皮圍裙、手持刮鱗剖魚小刀的精明魚販，說話響亮俐落，周身帶著清爽海風與冰鮮氣息。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0224", "銀鱗土魠魚", 8),
            ("wt_cat_0226", "淺澤青殼河蝦", 6),
            ("wt_cat_0227", "活水肥美草魚", 7),
        ],
            trade_talk_pitch: "「清晨出海新靠岸的活水魚蝦，鱗光透亮，挑幾尾回去清蒸煲湯最是鮮甜。」",
            on_duty_talk_lines: &[
            "「晨光初透，風舶外海的銀鱗土魠魚與老堰頭肥草魚新卸船！冰鮮競價，手慢可就搶不到了。」",
            "「三水河網直通西海，這水尾寮青殼蝦活蹦亂跳，一進早市就引來各大食堂掌勺競價。」",
            "「泊位邊水汽重，買魚要看魚鰓成色。只要是我們風舶拍賣場出具的魚簍木牌，尾尾保鮮！」",
        ],
        });
    }

    // 2. 南北乾貨棧
    if combined.contains("乾貨") || combined.contains("海味") || combined.contains("南北雜貨") || combined.contains("臘味") || combined.contains("山珍") || combined.contains("山貨") || combined.contains("獵戶") || combined.contains("鹽漬") || combined.contains("水產乾貨") {
        return Some(RoomVenueObject {
            venue_kind: "南北乾貨棧",
            default_host_title: "當值乾貨商",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正翻動竹篩上的脫水蝦米與乾貝柱檢查乾燥度",
            on_duty_demeanor: "面容精瘦、熟諳水陸物產的老商賈，指尖微帶海鹽粉霜，待客細緻而健談。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0319", "海風脫水小蝦米", 6),
            ("wt_cat_0320", "乾製海生干貝柱", 10),
            ("wt_cat_0228", "礁石野生紫菜", 5),
        ],
            trade_talk_pitch: "「風吹日曬熟成的海味乾貨與山珍臘味，耐儲耐運，煲湯增鮮最為醇美。」",
            on_duty_talk_lines: &[
            "「烏石鼻乾製海貝與風櫃尾野生紫菜，曬足整整七日海風，煲湯入味最是鮮美醇厚。」",
            "「外海風急浪大，船隊遠航必備鹽漬白帶魚與脫水蝦米，耐放半年不壞，吃時用溫水泡發即可。」",
            "「乾貨儲庫通風防潮全靠頂層木百葉，若要大宗調撥發往西境，憑三水商行驗單立提。」",
        ],
        });
    }

    // 3. 醬園調味坊
    if combined.contains("醬園") || combined.contains("醋坊") || combined.contains("腐乳") || combined.contains("醬菜") || combined.contains("油坊") || combined.contains("麻油") || combined.contains("陳記醬園") {
        return Some(RoomVenueObject {
            venue_kind: "醬園調味坊",
            default_host_title: "當值釀造東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以竹筒長杓自老陶缸舀取烏醋過濾入瓶",
            on_duty_demeanor: "身著粗布深藍長衫的調味釀師，掌心常染豆醬濃香，眼神溫和沉穩。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0346", "熟成野山果醋", 5),
            ("wt_cat_0347", "老罈烏醋香料汁", 6),
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
        ],
            trade_talk_pitch: "「老罈慢釀黑豆醬與野山果醋，酸甘醇厚，備上一壺入菜提鮮勝過千味。」",
            on_duty_talk_lines: &[
            "「老罈黑豆醬油慢釀三年，揭開陶缸蓋滿街飄香；半嶺烏醋入菜生津，是文曜老店看家滋味。」",
            "「紅麴米醋酸香柔潤，配羊肉湯餅或海鮮最解腥膩。你若帶乾糧，不妨打一小竹筒醋汁備用。」",
        ],
        });
    }

    // 4. 酒坊烈酒鋪
    if combined.contains("酒坊") || combined.contains("紅爐烈酒") || combined.contains("酒鋪") || combined.contains("釀酒") || combined.contains("酒保") || combined.contains("烈酒大飯鋪") || combined.contains("沈家老酒") || combined.contains("老酒坊") || combined.contains("盤查酒鋪") {
        return Some(RoomVenueObject {
            venue_kind: "酒坊烈酒鋪",
            default_host_title: "當值酒保",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於通紅爐火旁溫熱封缸米酒並擦拭大碗",
            on_duty_demeanor: "性情爽直的酒家漢子，挽著衣袖，迎客熱絡，說話透著四方豪氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0506", "封缸陳年醇米酒", 12),
            ("wt_cat_0507", "蜜漬山果甜釀", 8),
            ("wt_cat_0444", "厚釜慢燉原牛肉湯", 10),
        ],
            trade_talk_pitch: "「陳年老酒滾燙下喉，配一大碗慢燉牛肉湯，驅寒生暖，最宜歇腳暖身。」",
            on_duty_talk_lines: &[
            "「大釜紅爐烈火正旺，封缸陳年米酒滾燙下喉！走長途的行商與打鐵師傅，進門先來兩大碗！」",
            "「北嶺山果甜釀配大塊燉牛肉，酒香夾著肉香，任憑門外寒風呼嘯，屋裡熱氣熏得人面紅耳赤。」",
            "「入城盤查先歇歇腳，喝碗溫酒定定神。西關卡哨查得嚴，但只要身家清白，飲飽便好上路。」",
        ],
        });
    }

    // 5. 麵食乾糧鋪
    if combined.contains("羊肉湯餅") || combined.contains("麵食") || combined.contains("餅鋪") || combined.contains("燒餅") || combined.contains("炊餅") || combined.contains("乾糧") || combined.contains("大食堂") || combined.contains("熱食") || combined.contains("飯館") || combined.contains("廚師") || combined.contains("伙夫") || combined.contains("西門麵食鋪") || combined.contains("乾糧備製作坊") {
        return Some(RoomVenueObject {
            venue_kind: "麵食乾糧鋪",
            default_host_title: "當值麵點掌勺",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正揉擀高筋白麵團，於沸湯鐵鍋前拉切刀削麵片",
            on_duty_demeanor: "圍著雪白圍裙的掌勺麵師，臂力沉勁，動作俐落，額角沁著熱氣汗珠。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0311", "鹽醃草飼羊肉條", 10),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0285", "手工日曬粗麵線", 5),
        ],
            trade_talk_pitch: "「滾湯羊肉湯餅與日曬乾糧麵條，香醇吸汁，隨身帶上幾扎長路不愁餓肚子。」",
            on_duty_talk_lines: &[
            "「大碗羊肉湯餅熱氣騰騰！北嶺鹽醃草飼羊肉慢燉化渣，手擀麵片筋道吸汁，撒一把野薄荷碎，痛快！」",
            "「大圳頭高筋白麵切得細勻，下鍋滾三滾便撈起，配上一勺熬透的熟牛油辣子，行客吃了腳下生風！」",
            "「出西門走戈壁駝道，隨身必帶乾烙饃與脫水麵筋。耐嚼扛餓，在沙漠裡揣在懷裡十天不餿。」",
        ],
        });
    }

    // 6. 茶棚茶飯館
    if combined.contains("茶棚") || combined.contains("茶飯館") || combined.contains("涼茶") || combined.contains("甜水") || combined.contains("水手茶飯館") || combined.contains("工匠大食堂") || combined.contains("食堂") || combined.contains("跑堂") || combined.contains("大廚") {
        return Some(RoomVenueObject {
            venue_kind: "茶棚茶飯館",
            default_host_title: "當值茶東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正提著大銅壺向粗瓷茶碗沖注翻滾的熱茶湯",
            on_duty_demeanor: "隨和熱心的大茶棚東家，笑瞇瞇地迎送四方茶客，耳聽八方、見聞極廣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0394", "切仔湯麵配滷蛋", 5),
            ("wt_cat_0392", "鮮肉大包子", 4),
            ("wt_cat_0500", "山風草炭焙烘青茶", 4),
        ],
            trade_talk_pitch: "「大碗熱茶配鮮肉包子與切仔麵，價廉物美，坐下歇歇腳喝碗茶再走。」",
            on_duty_talk_lines: &[
            "「大食堂飯鐘一響，千號工匠齊奔食案！厚釜原牛骨湯配十八摺肉包，熱氣騰騰管飽有力氣！」",
            "「碼頭茶棚倚江而建，粗瓷碗泡著高山濃茶。聽水手擺談外海鯨蹤與河道暗礁，最是長見識。」",
            "「切仔湯麵配滷蛋，一勺油蔥酥香飄半條街。靠岸水手吃飽抹嘴，轉身就去船塢扛大包。」",
        ],
        });
    }

    // 7. 鹽糧米行倉
    if combined.contains("鹽糧") || combined.contains("糧倉") || combined.contains("米行") || combined.contains("穀行") || combined.contains("草料行") || combined.contains("豆餅") || combined.contains("精鹽") || combined.contains("鹽棧") || combined.contains("糧商") || combined.contains("通濟河運總倉") || combined.contains("戰備糧草") || combined.contains("儲倉") || combined.contains("鹽巴草料行") || combined.contains("粗鹽") {
        return Some(RoomVenueObject {
            venue_kind: "鹽糧米行倉",
            default_host_title: "當值糧商掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以量斗平升核對白霜海鹽與新米袋數",
            on_duty_demeanor: "著青布夾襖、手握提秤與算盤的糧行司事，帳目分明，神色穩健。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0338", "日曬白霜細海鹽", 4),
            ("wt_cat_0284", "竹篩日曬摺疊麵", 4),
            ("wt_cat_0277", "秋收蓬萊新米", 6),
        ],
            trade_talk_pitch: "「半島精選新米與日曬細海鹽，囤糧防歉，戶戶少不得這些根本生計之物。」",
            on_duty_talk_lines: &[
            "「風舶白霜海鹽堆垛如雪山，大圳新米一袋袋入庫。通濟總倉連通內河，一日進出糧船不下百艘。」",
            "「戰備糧草倉四面石垣厚重，通風隔潮滴水不漏。巡防營備與各鎮應急，全指望這片地下防潮窖。」",
            "「草料壓捆機手搖絞緊，壓出的乾草磚緊實耐運。駝獸沿途嚼著草磚喝鹽水，走千里也不乏力。」",
        ],
        });
    }

    // 8. 紙料造紙坊
    if combined.contains("宣紙") || combined.contains("造紙") || combined.contains("竹綿紙") || combined.contains("青檀宣紙") || combined.contains("竹綿造紙精廬") || combined.contains("紙料儲庫") || combined.contains("善本紙料") {
        return Some(RoomVenueObject {
            venue_kind: "紙料造紙坊",
            default_host_title: "當值造紙匠師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢查簾床抄造出的竹綿濕紙平整度與烘紙壁火溫",
            on_duty_demeanor: "掌心帶著草木紙漿纖維痕跡的文質匠人，神情專注雅緻，舉止溫文。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0779", "竹綿印刷紙", 6),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ],
            trade_talk_pitch: "「青檀與竹綿精工造就的良紙，落墨如雲、抗蛀防脆，著書立說與繪製圖冊首選。」",
            on_duty_talk_lines: &[
            "「青檀表皮經三年水浸日曬，木臼舂爛後配竹綿抄造。這紙張細膩勻整、落墨如雲，百年不脆！」",
            "「簾床入水輕搖，紙漿如蟬翼鋪展。烘紙壁火溫要求極嚴，過熱則裂，微冷則澀，全在手感掌握。」",
            "「藏書庫善本紙料皆用花椒防蛀，青檀宣紙整捆入樟木箱封存，專供學院修撰與輿圖抄繪之用。」",
        ],
        });
    }

    // 9. 文房筆墨行
    if combined.contains("徽墨") || combined.contains("石硯") || combined.contains("筆墨") || combined.contains("文房") || combined.contains("松煙徽墨") || combined.contains("端溪石硯") || combined.contains("宣筆") || combined.contains("製毫") || combined.contains("筆墨文具") {
        return Some(RoomVenueObject {
            venue_kind: "文房筆墨行",
            default_host_title: "當值筆墨東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以細鹿皮擦拭端溪石硯與鋼尖蘸水筆",
            on_duty_demeanor: "長袍儒雅的老文房商，眼光銳利，隨身散發淡雅松煙與沉香氣息。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0821", "玻璃螺紋瓶裝黑墨水", 6),
            ("wt_cat_0780", "彈性精鋼尖蘸水硬筆", 10),
        ],
            trade_talk_pitch: "「端溪水岩老硯與耐磨精鋼蘸水筆，下墨均勻順手，文墨行家自知妙處。」",
            on_duty_talk_lines: &[
            "「古松點煙、鹿膠輕膠慢揉，千錘百鍊始成徽墨。研於石硯之上，墨色黝黑沉厚，發出淡淡松香。」",
            "「端溪水岩老坑石硯，石質溫潤如玉，呵氣即可研墨。磨刀不損石，落墨發清響，文人求之若渴。」",
            "「宣筆製毫挑選秋兔脊毛與野狼青毫，兼毫剛柔並濟。無論寫蠅頭小楷還是大字擘窠，提按皆隨心意。」",
        ],
        });
    }

    // 10. 印書裝幀書坊
    if combined.contains("活字") || combined.contains("印書") || combined.contains("裝幀") || combined.contains("書坊") || combined.contains("藏書閣") || combined.contains("講堂") || combined.contains("書生") || combined.contains("教習") || combined.contains("刻工") || combined.contains("印行") || combined.contains("古籍裝幀") || combined.contains("裱褙") || combined.contains("文房四寶") || combined.contains("藏書") || combined.contains("書局") || combined.contains("書舍") || combined.contains("典藏庫") || combined.contains("圖書典藏庫") {
        return Some(RoomVenueObject {
            venue_kind: "印書裝幀書坊",
            default_host_title: "當值書坊刻匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢視活字字盤排版並塗刷均勻印墨",
            on_duty_demeanor: "穿著沾有墨漬的靛藍長衫，目光篤定認真，指節因常年排字雕刻而堅實有力。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0784", "《白塔風物誌》", 8),
            ("wt_cat_0786", "每旬發行《半島旬報》", 3),
            ("wt_cat_0781", "竹綿紙牛皮記事手札", 8),
        ],
            trade_talk_pitch: "「活字印製《白塔風物誌》與每旬旬報新鮮出爐，買幾冊手札筆記，記錄行路見聞。」",
            on_duty_talk_lines: &[
            "「活字銅模逐字揀排，壓印在濕潤竹綿紙上。新一期《半島旬報》排版初定，墨香四溢，晌午即售！」",
            "「藏書別院卷帙浩繁，從前朝航海錄到當今農水格物皆有編目。借閱抄錄需淨手焚香，切莫折角。」",
            "「名家書畫裱褙講究『漿清如水，托如蟬翼』。經熟絹包邊、花綾隔水，殘破舊卷立時重獲生機。」",
        ],
        });
    }

    // 11. 格物輿圖算學館
    if combined.contains("輿圖") || combined.contains("測風") || combined.contains("曆算") || combined.contains("星象") || combined.contains("算學") || combined.contains("觀星") || combined.contains("格物輿圖") || combined.contains("北辰測風") || combined.contains("會計") || combined.contains("講舍") || combined.contains("觀星台") || combined.contains("學院") || combined.contains("蒙學堂") || combined.contains("學生") {
        return Some(RoomVenueObject {
            venue_kind: "格物輿圖算學館",
            default_host_title: "當值曆算教習",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以銅製分度圓規在海圖上量算水深與航道經緯",
            on_duty_demeanor: "戴著黃銅助視鏡、身形修長的格物學者，嚴謹沉靜，談及天文山川便神采奕奕。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0785", "航海圖冊《風舶航海錄》", 15),
            ("wt_cat_0731", "高空青銅測風儀", 18),
            ("wt_cat_0788", "烏木便攜計數板", 14),
        ],
            trade_talk_pitch: "「精密航海圖、高空測風儀與便攜計數板，遠洋出海與築堤興修之必備圭臬。」",
            on_duty_talk_lines: &[
            "「格物輿圖院測量文曜內外地脈，雙筒青銅測量鏡校準星位，繪就的水道深淺圖是各船長保命符。」",
            "「北辰高台上銅測風儀旋轉不休，候風官推算明日信風流向，旗號一掛，整座港口的船隊便知進退。」",
            "「計算精準快速。會計與曆算講堂正推演防洪渠堤壩受力與糧稅分帳，一絲一毫差錯都逃不過算尺。」",
        ],
        });
    }

    // 12. 琴棋金石雅集齋
    if combined.contains("斫琴") || combined.contains("棋院") || combined.contains("鑑藏") || combined.contains("古玩") || combined.contains("雅集") || combined.contains("古琴") || combined.contains("文曜棋院") || combined.contains("古物鑑藏齋") {
        return Some(RoomVenueObject {
            venue_kind: "琴棋金石雅集齋",
            default_host_title: "當值鑑藏師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正以紫光燈細細檢視古銅鏡紋理與古琴漆灰斷紋",
            on_duty_demeanor: "寬袍大袖、仙風道骨的古物雅士，舉止風雅超脫，鑑別器物明察秋毫。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0795", "烏木黑子白貝圍棋盤", 20),
            ("wt_cat_0796", "松香木雕刻象棋組", 16),
        ],
            trade_talk_pitch: "「烏木黑子白貝棋盤與松香木雕刻象棋，質地厚重溫潤，坐隱忘憂，陶冶性情。」",
            on_duty_talk_lines: &[
            "「老杉良桐陰乾五載，掏膛配金石之聲。指尖撫過絲弦，清越幽遠，如高山空谷流水潺潺。」",
            "「黑子玄武岩，白子老海貝。文曜棋院黑白交錯間，爭的不是一城一池，是天元推衍的大格局。」",
            "「古物鑑藏齋明窗淨几，青銅古鏡與殘卷古玉皆需紫光過目。是真古蹟還是後仿，眼力分毫不差。」",
        ],
        });
    }

    // 13. 杏林百草藥局
    if combined.contains("杏林百草醫館") || combined.contains("採藥棧") || combined.contains("藥材行") || combined.contains("藥鋪") || combined.contains("診所") || combined.contains("醫者") || combined.contains("大夫") || combined.contains("藥師") || combined.contains("百草醫館") || combined.contains("採藥") || combined.contains("藥材中轉站") || combined.contains("百草藥居") || combined.contains("脈絡診所") || combined.contains("醫師") || combined.contains("學徒") {
        return Some(RoomVenueObject {
            venue_kind: "杏林百草藥局",
            default_host_title: "當值醫師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正研磨拔毒水苔敷散並整飭銀針囊",
            on_duty_demeanor: "白袍溫潤的百草醫者，神情親切专注，隨身自帶淡淡草木芳冽香氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1054", "拔毒止血水苔敷藥", 10),
            ("wt_cat_1055", "白茫避瘴清心丸", 12),
            ("wt_cat_1060", "溫氏九寸脈絡銀針囊", 16),
        ],
            trade_talk_pitch: "「黑沼拔毒水苔、避瘴清心丸與脈絡銀針囊，遠涉山林防身辟瘴必不可少。」",
            on_duty_talk_lines: &[
            "「黑沼拔毒止血水苔敷於傷口，三息止血；南哨白茫避瘴清心丸含在舌下，入深林不染瘴氣。」",
            "「醫館診脈辨虛實，九寸脈絡銀針順行氣血。無論跌打骨折還是積勞內傷，照藥方抓藥調理自能康復。」",
            "「採藥客翻越北山絕壁帶回新鮮山草，藥棧天井正分揀陰乾。山氣芬芳，聞一口便覺神清氣爽。」",
        ],
        });
    }

    // 14. 百草烘藥溫浴鋪
    if combined.contains("烘藥房") || combined.contains("百草烘藥房") || combined.contains("香燭") || combined.contains("山風草香燭鋪") || combined.contains("浴場") || combined.contains("溫水浴場") {
        return Some(RoomVenueObject {
            venue_kind: "百草烘藥溫浴鋪",
            default_host_title: "當值烘藥師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於火道前翻焙艾草香盤並調和清涼草木湯包",
            on_duty_demeanor: "衣衫清爽俐落的藥鋪管事，手腳勤快敏捷，周身環繞著草木溫熏香氣。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1057", "溫氏導能溫石百草溫灸盒", 14),
            ("wt_cat_1058", "驅蚊艾草香盤", 6),
            ("wt_cat_0119", "山野清香薄荷葉", 4),
            ("wt_cat_1056", "祛寒草木湯包", 6),
        ],
            trade_talk_pitch: "「導能溫石灸盒、祛寒湯包與驅蚊艾草香盤，通經活血拔除寒濕，行路露營必備。」",
            on_duty_talk_lines: &[
            "「烘藥房火道均勻，溫石百草溫灸盒借火性引藥力通透骨髓，最去行商老客筋骨陰濕寒氣。」",
            "「艾草香盤佐以野薄荷與松脂，點上一盤，蚊蚋退避、室內清爽，常客長途露宿行囊裡必備數捆。」",
            "「引地熱溫泉水入池，祛寒草木湯包浸透熱浪。泡過盞茶工夫，周身毛孔舒張，筋骨疲乏盡消。」",
        ],
        });
    }

    // 15. 大鍛鐵匠鍛造坊
    if combined.contains("重鐵水錘") || combined.contains("大鍛坊") || combined.contains("鍛造") || combined.contains("鍛修") || combined.contains("車軸鐵箍") || combined.contains("磨刀") || combined.contains("磨刀利刃") || combined.contains("鐵蹄釘掌") || combined.contains("釘掌鋪") || combined.contains("鐵匠") || combined.contains("鍛徒") {
        return Some(RoomVenueObject {
            venue_kind: "大鍛鐵匠鍛造坊",
            default_host_title: "當值鍛造大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正揮動導能鍛錘將燒紅鐵條重擊成型，火星四濺",
            on_duty_demeanor: "膀大腰圓、肌肉虯結的重甲鐵匠，穿著厚牛皮防燙圍裙，聲若洪鐘、沉穩如山。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0945", "岑氏省力導能鍛錘", 18),
            ("wt_cat_1021", "岩羊角手柄折疊獵刀", 18),
            ("wt_cat_0957", "鍛鋼小羊角錘", 10),
        ],
            trade_talk_pitch: "「千錘百鍊精鋼刀錘與車軸耐磨鐵件，剛柔兼備，經久耐用。」",
            on_duty_talk_lines: &[
            "「水錘借水力轟鳴砸下，震得地面微顫！紅鐵出爐火星四濺，工坊大匠千錘百鍊，造就精鋼刃口。」",
            "「車軸鐵箍經大爐淬火熱套，收緊後與硬木車輪合為一體，任憑山路坎坷，萬里馳騁決不崩裂。」",
            "「打鐵蹄修磨掌，生鐵蹄鐵貼合馬蹄弧度，釘入三寸不傷蹄肉。換上新掌，馱獸奔走如踏平地。」",
        ],
        });
    }

    // 16. 農具五金交易場
    if combined.contains("農具") || combined.contains("大型農具工具行") || combined.contains("五金") || combined.contains("銅鐵五金") || combined.contains("鎖鑰") || combined.contains("鎖簧") || combined.contains("器物交易場") || combined.contains("交易場") {
        return Some(RoomVenueObject {
            venue_kind: "農具五金交易場",
            default_host_title: "當值五金東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正檢驗省力導能翻土犁鋼口與熟銅門把碰鎖",
            on_duty_demeanor: "精打細算、熟悉各行工具結構的老司事，手中常拿著黃銅卡尺與樣板。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0950", "省力導能翻土犁", 16),
            ("wt_cat_0685", "熟銅門把帶重鐵碰鎖", 10),
            ("wt_cat_0707", "青銅插銷暗扣雙重鎖", 12),
        ],
            trade_talk_pitch: "「導能翻土犁與熟銅雙重暗扣門鎖，開荒治業必備利器，牢固防撬防鏽。」",
            on_duty_talk_lines: &[
            "「省力導能翻土犁鋼口鋒利，入土三分泥浪翻滾；水力大石磨齒槽精修，磨豆磨面細滑如雪。」",
            "「熟銅碰鎖與青銅插銷雙扣暗簧，防撬防鏽經久耐用。門戶要道安上這把鎖，夜不閉戶也心安。」",
            "「器物交易場上各坊名匠齊聚，從袖珍羅盤到重型千斤頂一應俱全，現鎂過秤、憑單交割。」",
        ],
        });
    }

    // 17. 武備防具巡防所
    if combined.contains("武備") || combined.contains("護商武備鋪") || combined.contains("防具") || combined.contains("火浣布防具坊") || combined.contains("火政") || combined.contains("東門火政巡防所") || combined.contains("習武堂") {
        return Some(RoomVenueObject {
            venue_kind: "武備防具巡防所",
            default_host_title: "當值教頭",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正擦拭厚牛皮卸力護腕並整理火浣布防水長衫",
            on_duty_demeanor: "目光炯炯、身手矯健的武備教官，腰板筆直，言談間充滿行伍警覺。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0846", "卸力厚牛皮護腕", 14),
            ("wt_cat_0840", "鍛造坊厚牛皮防燙圍裙", 16),
            ("wt_cat_0838", "樹膠防水防雨長衫", 18),
        ],
            trade_talk_pitch: "「厚牛皮護腕與防水防燙長衫，耐磨防刺，戈壁涉水野外護身第一選。」",
            on_duty_talk_lines: &[
            "「行商過戈壁山林，護身軟甲與卸力銅護腕是保命行頭。刀刃劈在熟銅上滑開三分，便多一條生路。」",
            "「厚牛皮防燙圍裙防油防刺，火浣布經烈火焚燒不焦。火政巡守手持大鉤水桶，防患於未然。」",
            "「習武堂試招木人被劈得斑駁，拳風破空呼呼作響。出遠門護運隊招募好手，先在場子裡過上三招！」",
        ],
        });
    }

    // 18. 精銅鐘錶儀器坊
    if combined.contains("鐘錶") || combined.contains("儀器坊") || combined.contains("精銅鐘錶儀器坊") || combined.contains("琉璃燈") || combined.contains("白塔琉璃燈坊") || combined.contains("鐘樓") || combined.contains("傳訊鐘樓") || combined.contains("司鐘") {
        return Some(RoomVenueObject {
            venue_kind: "精銅鐘錶儀器坊",
            default_host_title: "當值儀器師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正戴著單目助視鏡微調黃銅座鐘擒縱機構與懷錶發條",
            on_duty_demeanor: "精細微工匠師，雙手沉穩無一絲顫抖，舉手投足恪守規律。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0729", "校準隨身黃銅懷錶", 18),
            ("wt_cat_0728", "岑氏黃銅發條座鐘", 24),
            ("wt_cat_0723", "旋蓋夜光貝母調光台燈", 15),
        ],
            trade_talk_pitch: "「校準黃銅懷錶與夜光貝母調光台燈，走時分毫不差，風吹雨淋光線柔和。」",
            on_duty_talk_lines: &[
            "「發條齒輪精密咬合，黃銅擺錘分毫不差。調準懷錶時刻，各路車船驛使才能嚴絲合縫交接。」",
            "「夜光貝母薄片鑲嵌於旋蓋台燈，光線柔和護眼，狂風吹拂亦不搖晃，藏書閣與船艙必備。」",
            "「信使總站信鴿起落、飛騎待發。八方急件過印登記，晨發文曜，暮達半島各哨，號令通行。」",
        ],
        });
    }

    // 19. 鋸木竹木作坊
    if combined.contains("鋸木坊") || combined.contains("連軸鋸木坊") || combined.contains("竹木行") || combined.contains("三水竹木行") || combined.contains("竹木場") || combined.contains("蘆葦竹木場") || combined.contains("竹編") || combined.contains("藤器") || combined.contains("竹編藤器作坊") || combined.contains("木作") || combined.contains("傢俱") || combined.contains("巧木居所") || combined.contains("榫眼") || combined.contains("木器榫眼作坊") || combined.contains("大木作") || combined.contains("車轅大木作") || combined.contains("大木車船坊") || combined.contains("木匠") {
        return Some(RoomVenueObject {
            venue_kind: "鋸木竹木作坊",
            default_host_title: "當值木作大匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正用水力原木大鋸剖切雲杉樑柱並校準燕尾榫眼",
            on_duty_demeanor: "衣履染著松柏碎木屑的老匠人，沉厚篤實，對木材紋理瞭如指掌。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0947", "水力原木大鋸", 20),
            ("wt_cat_0895", "雙層竹編提籃", 8),
            ("wt_cat_0622", "防靜電松香木梳", 5),
        ],
            trade_talk_pitch: "「水力切剖優質木材、雙層密編竹籃與松香木梳，榫卯精妙，堅固耐用。」",
            on_duty_talk_lines: &[
            "「水車連軸帶動原木大鋸，巨木如剖瓜切菜般破成整齊板材，木香撲鼻，是建房造船的骨幹良料。」",
            "「竹編雙層提籃密不漏水，輕巧韌實耐磨耐摔。漁戶裝魚、當家娘子買菜，用上十年依舊齊整。」",
            "「巧木傢俱純用古法榫卯，燕尾榫、抱肩榫咬合嚴密，不著一顆鐵釘，越是吃重越見緊實。」",
        ],
        });
    }

    // 20. 石材磚瓦窯坊
    if combined.contains("石材切割場") || combined.contains("切石") || combined.contains("磚瓦窯") || combined.contains("灰漿磚瓦窯") || combined.contains("木炭窯") || combined.contains("高山木炭窯") || combined.contains("官窯") || combined.contains("青釉官窯") || combined.contains("銅錫") || combined.contains("器皿精煉坊") || combined.contains("生鐵大棧房") || combined.contains("焦炭生鐵大棧房") || combined.contains("生鐵庫") || combined.contains("粗鹽生鐵庫") {
        return Some(RoomVenueObject {
            venue_kind: "石材磚瓦窯坊",
            default_host_title: "當值窯坊掌事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正操作水力切石機剖平玄武岩板並點驗窯燒青磚",
            on_duty_demeanor: "膚色黝黑、面容敦厚剛毅的窯坊老工長，雙手粗糙厚實如鐵石。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0949", "銀砂水力切石機", 22),
            ("wt_cat_0944", "紅土耐火磚高溫鐵爐", 20),
            ("wt_cat_0097", "相思木硬炭", 6),
            ("wt_cat_0023", "粗陶紅黏土", 5),
        ],
            trade_talk_pitch: "「高溫耐火爐磚、相思木硬炭與精切玄武石料，百工營造爐竈不可或缺。」",
            on_duty_talk_lines: &[
            "「半嶺水力切石機水霧濛濛，整塊玄武岩巨石剖切平整，鋪在街面百年車輾馬踏不凹陷。」",
            "「青石磚瓦窯烈焰熊熊，出窯青磚敲之有金石之聲。焦炭生鐵大棧房生鐵錠堆積如山，專供鍛坊。」",
            "「青櫟硬木高溫炭化，燃燒無煙且火力持久，是百工冶煉與城中富戶圍爐取暖的上品燃料。」",
        ],
        });
    }

    // 21. 染纈織造服飾坊
    if combined.contains("染布坊") || combined.contains("三水染布坊") || combined.contains("機杼大染坊") || combined.contains("染纈") || combined.contains("織造") || combined.contains("雲錦織造與服飾坊") || combined.contains("服飾坊") || combined.contains("剪剃鋪") || combined.contains("修傘補靴") || combined.contains("修傘") || combined.contains("補靴") || combined.contains("繡娘") {
        return Some(RoomVenueObject {
            venue_kind: "染纈織造服飾坊",
            default_host_title: "當值織染師",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正於靛藍染缸起布翻曬，丈量棉麻衣袍尺幅",
            on_duty_demeanor: "身段窈窕、十指染著淡淡靛青色的巧手織娘，氣質嫻雅溫和。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0835", "細棉麻透氣常服", 14),
            ("wt_cat_0838", "樹膠防水防雨長衫", 18),
            ("wt_cat_0868", "素面玄武岩扣束腰長袍", 16),
        ],
            trade_talk_pitch: "「細棉麻常服、防水長衫與玄武岩扣長袍，貼身透氣防雨耐磨，四時舒合。」",
            on_duty_talk_lines: &[
            "「機杼聲聲穿梭如流，大圳細棉織就柔軟布匹。紮染絞纈沉入靛藍大缸，出水展曬，雲紋天成！」",
            "「坊內師傅親自量體裁衣，玄武岩扣剪裁外套文質彬彬，厚棉麻常服吸汗耐磨，四時得體。」",
        ],
        });
    }

    // 22. 造船漁具修船塢
    if combined.contains("造船塢") || combined.contains("內河造船塢") || combined.contains("修船塢") || combined.contains("渡口修船塢") || combined.contains("桅帆") || combined.contains("大船桅帆作坊") || combined.contains("帆布麻繩織造坊") || combined.contains("漁網繩索工坊") || combined.contains("修網結繩工坊") || combined.contains("漁具") || combined.contains("簑笠") || combined.contains("漁具簑笠商號") || combined.contains("桐油作坊") || combined.contains("船用桐油作坊") || combined.contains("浮標竹笛作坊") || combined.contains("浮標") || combined.contains("船夫") {
        return Some(RoomVenueObject {
            venue_kind: "造船漁具修船塢",
            default_host_title: "當值船匠",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調配桐油海貝灰膏捻縫船板，試拉巨藻船纜",
            on_duty_demeanor: "常年在風浪中曬得黑紅的老水手兼船木師傅，豪邁粗獷，嗓門洪亮。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1000", "深海巨藻船纜", 18),
            ("wt_cat_1002", "桐油海貝灰捻縫灰膏", 8),
            ("wt_cat_1003", "極淺吃水平底竹筏", 25),
            ("wt_cat_1006", "老青竹黃銅輪捲線釣竿", 10),
        ],
            trade_talk_pitch: "「巨藻船纜、防漏捻縫膏、平底竹筏與老竹捲線釣竿，江海行船下網必備。」",
            on_duty_talk_lines: &[
            "「造船塢內雲杉大龍骨架設完畢，桐油海貝灰捻縫密不透水。大船入水之日，半城人都來看熱鬧。」",
            "「深海巨藻絞成粗大船纜，韌性勝似牛筋，驚濤駭浪中扣住巨錨，是遠洋客船全船人的定心骨。」",
            "「修網結繩需心明手巧，麻線經魚油浸泡耐爛抗腐。老漁翁織出的密目網，連蝦苗都漏不出一隻。」",
        ],
        });
    }

    // 23. 客舍大驛館
    if combined.contains("駝馬大驛站") || combined.contains("萬里駝隊大驛站") || combined.contains("駝馬大驛館") || combined.contains("駝馬大旅社") || combined.contains("長途行商旅社") || combined.contains("旅社") || combined.contains("風塵客商大旅社") || combined.contains("東橋旅社") || combined.contains("客舍") || combined.contains("青雲求學客舍") || combined.contains("歇舍") || combined.contains("行商長途歇舍") || combined.contains("四方匠師行館") || combined.contains("泊客旅舍") || combined.contains("風舶臨水旅社") || combined.contains("北灣臨水旅社") || combined.contains("臨水旅社") || combined.contains("茶屋") || (room_name.contains("行館") && !room_name.contains("印行館")) || extra_hint.contains("茶客") {
        return Some(RoomVenueObject {
            venue_kind: "客舍大驛館",
            default_host_title: "當值驛丞",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於櫃檯登記入住路引，為火爐添柴煮熱奶茶",
            on_duty_demeanor: "待客周到熱忱的旅社東家，熟悉八方商路與各地風土人情。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
            ("wt_cat_0897", "薄層重礦烏木膽隨身保溫瓶", 10),
            ("wt_cat_1037", "防倒風野營生鐵柴火爐", 15),
        ],
            trade_talk_pitch: "「雙扣厚帆布背包、隨身保溫瓶與野營防風柴火爐，長途跋涉溫暖隨行。」",
            on_duty_talk_lines: &[
            "「萬里駝隊在此卸鞍歇馬！驛館後院草料溫水管夠，前堂火爐煮著濃奶茶，洗去一身風塵。」",
            "「客舍床榻整潔乾爽，每間備有防倒風野營柴火爐與保溫瓶。行商過客推杯換盞，互通關山道路。」",
            "「四方匠師齊聚行館交流技藝，南腔北調熱鬧非凡。在此歇宿一晚，各地行情掌故全在耳中。」",
        ],
        });
    }

    // 24. 皮毛鞍具帳篷行
    if combined.contains("皮毛大市") || combined.contains("西門皮毛大市") || combined.contains("皮毛大貨棧") || combined.contains("皮貨行") || combined.contains("西門皮貨行") || combined.contains("皮帶鞍套") || combined.contains("皮鞘鞍具") || combined.contains("鞍轡") || combined.contains("遠行鞍轡帳篷行") || combined.contains("帳篷") || combined.contains("行囊修補行") || combined.contains("行囊") || combined.contains("行車雨布作坊") || combined.contains("雨布") {
        return Some(RoomVenueObject {
            venue_kind: "皮毛鞍具帳篷行",
            default_host_title: "當值皮貨東家",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正給熟牛皮馬鞍上蠟保養，縫補雙層防風行商帳篷",
            on_duty_demeanor: "指掌粗糙、腰間插著骨針與刮皮刀的皮貨老東家，性情敦厚老練。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0836", "飛鼠厚絨禦寒冬衣", 22),
            ("wt_cat_0837", "雲豹毛領禦寒大氅", 28),
            ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ("wt_cat_1040", "平底木桶式雙槳划艇", 20),
        ],
            trade_talk_pitch: "「雪狼毛領大氅、厚絨禦寒冬衣與牛皮錢夾，荒野防寒抗風不可缺。」",
            on_duty_talk_lines: &[
            "「牛埔與大圳頭送來的雪狼毛領大氅與熟牛皮鞍具，皮質油潤柔軟，披在身上能擋關外徹骨白毛風。」",
            "「雙層防風防水行商帳篷配厚木撐桿，風沙再大也能在戈壁撐起安穩避難所，行商長途必帶。」",
            "「牛皮馬鞍與護膝皆用雙股蠟線縫製，行囊修補師傅手藝精湛，崩了線的背囊片刻修整如新。」",
        ],
        });
    }

    // 25. 大車修造聯運總站
    if combined.contains("雙輪大車坊") || combined.contains("重載雙輪大車坊") || combined.contains("大車修造場") || combined.contains("車馬輪軸修造場") || combined.contains("長途車行") || combined.contains("西道長途車行") || combined.contains("車馬聯運總站") || combined.contains("聯運貨運行") || combined.contains("聯運") || combined.contains("貨運行") || combined.contains("油膏滑脂鋪") || combined.contains("滑脂") || combined.contains("搬運工") || combined.contains("車夫") {
        return Some(RoomVenueObject {
            venue_kind: "大車修造聯運總站",
            default_host_title: "當值車行把式",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正扳動液壓千斤頂頂起重載大車，塗抹輪軸滑脂",
            on_duty_demeanor: "滿手油污但精神奕奕的車馬把式，談吐爽快，深諳車馬運輸規矩。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1004", "韓氏雙輪駝獸大貨車", 30),
            ("wt_cat_0988", "重載輪軸手動液壓千斤頂", 18),
            ("wt_cat_0985", "駝獸生鐵打蹄鉗與平口銼", 12),
        ],
            trade_talk_pitch: "「雙輪重載貨車、液壓千斤頂與打蹄工具，車隊幹道行馳無礙。」",
            on_duty_talk_lines: &[
            "「重載雙輪大車採用硬木車轅與雙軸滾珠，馱載三千斤生鐵亦行若游水，是通濟大道上的巨無霸。」",
            "「車軸千斤頂輕輕一扳，重車騰空！抹上厚厚一層動物滑脂油膏，輪軸轉動無聲，能多跑千里路。」",
            "「長途車隊每日定時啟程發往半島各鎮。託運箱籠皆掛防水油布，路途再顛簸，貨物完好無損。」",
        ],
        });
    }

    // 26. 牲畜交易獸醫館
    if combined.contains("牲畜交易拍賣行") || combined.contains("牲畜拍賣") || combined.contains("獸醫館") || combined.contains("車馬獸醫館") {
        return Some(RoomVenueObject {
            venue_kind: "牲畜交易獸醫館",
            default_host_title: "當值獸醫館長",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正調配料豆草藥，操作手搖壓捆機壓實草料厚磚",
            on_duty_demeanor: "蓄著灰白短鬚的老獸醫，神色慈和，對各類牲畜馱獸習性了然於心。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_1020", "手搖螺旋壓實乾草料壓捆機", 16),
            ("wt_cat_0969", "重型生鐵熟牛皮防撞砧", 15),
        ],
            trade_talk_pitch: "「螺旋乾草料壓捆機與防撞砧，馱獸調養料草紮實，行路千里的好幫手。」",
            on_duty_talk_lines: &[
            "「拍賣場內號子嘹亮！牛埔的高頭健馬與耐旱駝獸正當場過眼驗齒，體格健碩，嘶鳴如雷。」",
            "「獸醫老館長專治走獸蹄疾與積熱，草藥調拌料豆灌服，針灸兩穴，立時消腫去乏、胃口大開。」",
            "「螺旋壓捆機壓出的乾草料厚磚碼在槽邊，健馬吃飽打個響鼻，渾身皮毛油光發亮。」",
        ],
        });
    }

    // 27. 碼頭商行匯兌護運所
    if combined.contains("貨船大埠") || combined.contains("川流貨船大埠") || combined.contains("北灣貨船大埠") || combined.contains("碼頭匯兌分號") || combined.contains("碼頭匯兌") || combined.contains("巡河哨所") || combined.contains("河工巡防哨居") || combined.contains("哨所") || combined.contains("長途護運哨所") || combined.contains("四方護運行總堂") || combined.contains("四方護運行") || combined.contains("護運行") || combined.contains("關防驗契所") || combined.contains("驗契") || combined.contains("公議堂") || combined.contains("契書") || combined.contains("總商行") || combined.contains("柑仔店") || combined.contains("帳房") || combined.contains("書辦") || combined.contains("通事") {
        return Some(RoomVenueObject {
            venue_kind: "碼頭商行匯兌護運所",
            default_host_title: "當值商行司事",
            has_counter_restricted_zone: true,
            on_duty_fixture_activity: "正核驗過關貨單商印，撥算盤匯兌鎂幣存摺",
            on_duty_demeanor: "穿著齊整圓領綢面短衫的帳房先生，算盤珠子撥得飛快，接人待物沉穩精準。",
            lendable_tool: None,
            counter_trade_offers: &[
            ("wt_cat_0790", "三水總商行鎂幣分格存摺", 12),
            ("wt_cat_0892", "軟牛皮分格零錢夾", 8),
            ("wt_cat_0891", "黃銅雙扣厚帆布斜背包", 12),
        ],
            trade_talk_pitch: "「總商行鎂幣存摺、分格零錢夾與斜背包，結算便利安全，流通全島無阻。」",
            on_duty_talk_lines: &[
            "「川流大埠貨船一艘接一艘靠岸，起重吊臂沒停過。碼頭匯兌分處可以直接換取鎂幣票據，全半島都能流通。」",
            "「四方護運行總部的護運員經驗豐富，長途車隊隨時注意幹道狀況，有他們隨行押車安穩得多。」",
            "「港務驗單處核對貨物清單與商號印章很仔細，手續辦妥蓋上封章，出城運輸就順暢了。」",
        ],
        });
    }

    // 28. 街巷橋樑公共空地
    if combined.contains("分家巷") || combined.contains("西門街") || combined.contains("講堂街") || combined.contains("石橋") {
        return Some(RoomVenueObject {
            venue_kind: "街巷橋樑公共空地",
            default_host_title: "街坊居民",
            has_counter_restricted_zone: false,
            on_duty_fixture_activity: "正於石橋欄杆旁駐足歇息，觀看街面人流往來",
            on_duty_demeanor: "神情安詳從容的街坊居民，安居樂業，對城中各坊動向頗為熟絡。",
            lendable_tool: None,
            counter_trade_offers: &[],
            trade_talk_pitch: "",
            on_duty_talk_lines: &[
            "「文曜城裡的街道很平整，作息也很規律。白天各行各業正常營業，到了晚上鐘樓報時收班，走在街上很安心。」",
            "「三水口連通外港，分家巷聚集各類工坊，講堂街則是學院與書局的所在地，城裡機能很齊全。」",
            "「初來文曜嗎？如果要找各處場所，順著三水匯流大石橋往四面看，各家招牌都很清楚。」",
        ],
        });
    }

    None
}

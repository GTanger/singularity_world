fn resolve_special_room_object_part2(target: &str) -> Option<crate::model::RoomObject> {
    if target == "wt_cat_0734" || target.starts_with("wt_astro_") || target == "wt_obj_armillary_almanac" || target.contains("星軌儀") || target.contains("推步儀") || target.contains("定緯銅儀") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cat_0734".to_string() },
            name: "渾天星軌銅儀".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ChartAstrolabe".to_string(), "Operate".to_string(), "Read".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "半嶺觀星台上架設的黃銅渾天星軌儀，多層精密同心銅環鐫刻著周天星宿赤道與黃道度數，齒輪咬合滴水運轉，推步歲差與日月運行。".to_string()),
                ("ChartAstrolabe".to_string(), "你備妥竹綿宣紙與松煙精墨，輕敷於星軌儀外圈經緯刻度上細緻拓印，將四時天象與個人本命周天星度完整摹拓成卷。".to_string()),
                ("Operate".to_string(), "你輕輕旋轉渾天儀的赤道環微調齒輪，銅環咬合發出清脆鳴響，極軸準星穩穩鎖定天頂北辰。".to_string()),
                ("Read".to_string(), "你凝神細讀銅儀基座刻劃的《北辰歲差推步曆訣》，推演當前歲時節氣與星野分野之奧妙。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_martial_force_stone" || target.contains("測勁玄武碑") || target.contains("導能測紋石板") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_martial_force_stone".to_string() },
            name: "測勁玄武碑".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "TestEngraving".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "工坊內以烏石鼻玄武岩厚板琢磨而成的測勁試刃石碑，表面嵌有暗金導能測紋凹槽，可供武者運息灌注測試勁力深淺與兵刃平衡。".to_string()),
                ("TestEngraving".to_string(), "你提聚周身真氣貫注掌心按向測勁玄武碑，金線紋理隨內息深淺明滅流轉，嗡然共鳴間將隨身兵刃的重心與鋒芒校準至分毫不差。".to_string()),
                ("Operate".to_string(), "你手持短刃順著玄武碑表面的導能凹槽徐徐劃過，檢視刃紋深淺與鋼材淬火應力。".to_string()),
                ("Use".to_string(), "你以指節叩擊玄武碑身，聽音辨析石質密度與導能靈敏度，平復吐納氣息。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_arena_testing_pillar" || target == "wt_martial_padded_wooden_blade" || target == "wt_arena_wooden_weapon_rack" || target.contains("玄武試勁柱") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_arena_testing_pillar".to_string() },
            name: "玄武試勁柱".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SparPillar".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "演武場中央矗立的玄武試勁巨柱，表面包裹著多層油鞣生牛皮與厚草繩，周遭擺放著包革木刀與兵器架，專供弟子磨礪拳腳招法與發勁沉雄之勢。".to_string()),
                ("SparPillar".to_string(), "你沉腰紮馬、抱元守一，繞柱演練「起勢・蓄勁、進步・破鋒、轉樞・卸力、收勢・歸元」四序武訣，拳風木刃撞擊石柱激起沉渾迴音。".to_string()),
                ("Operate".to_string(), "你調整試勁柱外裹的生牛皮護層繫繩，加固麻索結扣以承受更沉厚的重擊。".to_string()),
                ("Use".to_string(), "你取下一柄包革木刀掂量重心，貼近試勁柱比劃出刀角度與步法挪移。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_bath_jade_pool" || target == "wt_home_hotspring_tub" || target.contains("恆溫熱湯池") || target.contains("杉木沐浴桶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_bath_jade_pool".to_string() },
            name: "恆溫熱湯池".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "SoakHotSpring".to_string(), "Rest".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "以半嶺寒水玉打底壘砌的天然熱湯池，溫潤白霧蒸騰繚繞，池邊放著溫氏百草祛寒湯包，地脈溫泉汨汨注滿池中，散發著舒心草木幽香。".to_string()),
                ("SoakHotSpring".to_string(), "你寬衣踏入熱湯池中，融融暖意順著肌膚毛孔滲透周身四肢，拔除骨髓深處的陰寒潮氣與跋涉勞損，經絡血脈暢達通順。".to_string()),
                ("Rest".to_string(), "你斜倚池畔光潔的寒水玉石階閉目小憩，聆聽溫泉活水潺潺流響，心神俱皆鬆弛放空。".to_string()),
                ("Use".to_string(), "你舀起一木桶溫熱泉湯澆淋雙肩，取過山茶草本皂細細洗滌塵泥，神清氣爽。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_barber_") || target == "wt_bath_mirror_washstand" || target.contains("摺疊剃刀架") || target.contains("洗頭木躺椅") || target.contains("銀鏡盥洗台") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_barber_folding_razor".to_string() },
            name: "摺疊剃刀架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "GroomAppearance".to_string(), "Use".to_string(), "Rest".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "市井理髮鋪內的烏木梳妝几案，案上陳列著精鋼摺疊剃刀、牛角細齒梳、薄荷清香面膏與熱氣蒸騰的白布毛巾，透著幹練清爽的街坊人情味。".to_string()),
                ("GroomAppearance".to_string(), "你端坐於木躺椅上，覆上熱毛巾軟化鬍鬚，剃刀遊走間刮修鬢角、梳理鬚髮，清爽薄荷香氣沁人心脾，頓時容光煥發、神采奕奕。".to_string()),
                ("Use".to_string(), "你對著几前照影明晰的銀鏡整理衣冠領口，以細齒牛角梳順理髮髻。".to_string()),
                ("Rest".to_string(), "你靠在鋪著柔軟棉墊的洗頭躺椅上放鬆肩頸，享受溫水洗髮時的片刻安閒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target.starts_with("wt_repair_") || target == "wt_cloth_tailor_sewing_bench" || target.contains("修傘木砧檯") || target.contains("補靴硬木楦") || target.contains("裁縫量身几") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_repair_umbrella_bench".to_string() },
            name: "修傘木砧檯".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "MendTravelGear".to_string(), "Operate".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "街角匠人鋪的補綴木砧檯，檯面上散放著修補雨傘竹骨的細銅絲、生鐵底釘、補靴硬木楦與一罐透亮的熟桐油，專事修護旅者雨具與行腳革靴。".to_string()),
                ("MendTravelGear".to_string(), "你在砧檯前為行靴換上防滑耐磨的生鐵釘，抹勻厚魚油防水，並重新加固傘骨、刷上雙層熟桐油，整飭隨身行裝以備風雨長途。".to_string()),
                ("Operate".to_string(), "你使用木楦與小皮槌敲整行靴受壓變形的鞋頭，使靴筒重新貼合腳型。".to_string()),
                ("Use".to_string(), "你借用砧檯上的粗麻線與骨針穿引縫合行囊接縫，加固帶扣邊角。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_observatory_anemometer" || target == "wt_obj_wind_vane_gauge" || target == "wt_obs_tide_weather_table" || target.contains("黃銅風向儀") || target.contains("相風銅烏") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_observatory_anemometer".to_string() },
            name: "黃銅風向儀".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ReadWindVane".to_string(), "Operate".to_string(), "Read".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "山巔測候台高聳的相風黃銅烏與旋轉風速杯，下連精密齒輪傳動指針，案几上鋪開十九獵場風向圖與近海潮汐盈縮水經推步表。".to_string()),
                ("ReadWindVane".to_string(), "你仰觀風向儀指向與銅烏旋動疾徐，記錄高空雲氣走向與近海潮位漲落，對照滴漏水鐘推算十九獵場天候變化與安全航道。".to_string()),
                ("Operate".to_string(), "你撥動風速儀轉軸潤滑銅套，清除齒輪軸承上的山嵐露水，確保量測靈敏精準。".to_string()),
                ("Read".to_string(), "你查閱觀測案上連續多月的潮汐升降水經註記，參照節氣推衍海潮汛期。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_clinic_acupoint_bed" || target == "wt_clinic_decoction_stove" || target == "wt_obj_acupuncture_pulse_pillow" || target.contains("竹席診脈榻") || target.contains("溫灸盒") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_clinic_acupoint_bed".to_string() },
            name: "竹席診脈榻".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "ApplyMoxibustion".to_string(), "Rest".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "醫館內鋪設細編青竹涼席的診脈榻，榻頭擱著青玉診脈枕與紫銅溫灸盒，爐膛內文火慢煨著百草祛寒透骨湯，藥香幽微甘醇。".to_string()),
                ("ApplyMoxibustion".to_string(), "你伏臥於竹席榻上，司藥點燃陳年蘄艾置於溫灸盒中，溫熨關元與命門穴，艾火純陽之熱循經入髓，透散筋骨陰寒鬱結與扭傷痠楚。".to_string()),
                ("Rest".to_string(), "你枕著青玉脈枕合目靜躺，伴隨滿室淡淡中草藥香調勻呼吸，緊繃的四肢百骸逐漸放鬆舒緩。".to_string()),
                ("Use".to_string(), "你端起司藥盛出的溫熱百草煎劑趁熱啜飲，苦甘藥力化開胸腹鬱結之氣。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_obj_mountain_milestone" || target == "wt_shrine_votive_plaque" || target.contains("山道指路碑") || target.contains("祈福木牌") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_obj_mountain_milestone".to_string() },
            name: "山道指路碑".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "HangVotivePlaque".to_string(), "Read".to_string(), "Use".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "北嶺古山道旁矗立的深色花崗岩指路里程碑，碑旁橫架著古雲杉原木搭成的祈福木牌架，繫滿過往山客祈求行旅平安的厚實木牌，隨風輕叩。".to_string()),
                ("HangVotivePlaque".to_string(), "你取下一枚散發天然松脂清香的北嶺雲杉祈福木牌，提筆題寫行路平安字樣，虔誠懸於牌架之上，並收妥一枚隨身木牌作保平安之印記。".to_string()),
                ("Read".to_string(), "你拂去石碑青苔辨讀里程刻字，探明通往北嶺各處隘口、獵場與山泉古道的分岔方向。".to_string()),
                ("Use".to_string(), "你在指路石碑旁的青石歇腳坐下，飲泉歇息，整飭登山竹杖與綁腿。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_book_printing_shelf" || target.contains("風物誌書架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_book_printing_shelf".to_string() },
            name: "風物誌書架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Read".to_string(), "TranscribeRareTome".to_string(), "SunBooks".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "高大書架上整齊排著竹綿紙印製的《白塔風物誌》、每旬《半島旬報》與白話故事小冊，滿室紙墨芬芳。".to_string()),
                ("Read".to_string(), "你取下一卷《白塔風物誌》閱讀，書中記載著老堰頭水網修築沿革與寒水玉開採考證。".to_string()),
                ("TranscribeRareTome".to_string(), "你攤開竹綿宣紙與紅燭墨水，對著書架上的風物孤本恭謹鈔錄，筆端生風。".to_string()),
                ("SunBooks".to_string(), "你將書架上的竹綿紙古籍移至向陽通風石坪翻曬防蠹，夾入辟蠹芸香。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_clocktower_bronze_bell" || target.contains("報時青銅鐘") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_clocktower_bronze_bell".to_string() },
            name: "報時青銅鐘".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "CalibrateClock".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "高懸塔頂的厚重青銅巨鐘，垂下粗大的麻繩，每逢正點敲響，鐘聲清越浩蕩傳遍全境。".to_string()),
                ("Operate".to_string(), "你雙手握住粗麻繩合力一拉，青銅鐘錘重重撞上鐘體，發出一聲渾厚深沉的悠揚巨響。".to_string()),
                ("CalibrateClock".to_string(), "你參照青銅日晷刻度與滴漏校準鐘樓發條，登樓鳴鐘報刻，鐘聲悠揚傳遍四野。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_wood_planing_bench" || target.contains("刨木工作台") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_wood_planing_bench".to_string() },
            name: "刨木工作台".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "FitMortiseTenon".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "厚重工匠台上架著導能刨刀、拉鋸與木鑿，台面落滿薄如蟬翼的松木刨花，木香撲鼻。".to_string()),
                ("Operate".to_string(), "你握起導能推刨在松木料上平推而過，木片應手捲起均勻刨花，木面光滑平整。".to_string()),
                ("FitMortiseTenon".to_string(), "你手持角尺木鑿細細修校榫卯接榫，反覆推刨嵌合，嚴絲合縫、滴水不漏。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_forge_quench_trough" || target.contains("精鋼淬火槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_forge_quench_trough".to_string() },
            name: "精鋼淬火槽".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "TestEngraving".to_string(), "QuenchBlade".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "盛滿深井泉水與淬火油的大石槽，槽邊備有打磨精鋼刀具與門鎖配件的雙面細磨石。".to_string()),
                ("Use".to_string(), "你舀水淋在細磨石上推磨鐵件，金屬毛刺漸去，刃口泛起雪亮微光。".to_string()),
                ("TestEngraving".to_string(), "你運轉內息按上測紋石板並取隨身器物輕觸碑面，石紋微鳴回饋人器共鳴度。".to_string()),
                ("QuenchBlade".to_string(), "你將熾紅鋼刀猛然探入淬火油槽，白霧激盪翻騰，精鋼重煥寒芒。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_cloth_garment_rack" || target.contains("防雨長衫架") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_cloth_garment_rack".to_string() },
            name: "防雨長衫架".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "DyeIndigoCloth".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "松木衣架上整齊掛著塗有黑沼樹膠的防雨長衫、透氣棉麻短衫與北山雪狼絨冬衣，做工紮實。".to_string()),
                ("Use".to_string(), "你取下一件防雨長衫披在身上試穿，衣料厚韌輕便，領口銅扣封得嚴實，能擋狂風急雨。".to_string()),
                ("DyeIndigoCloth".to_string(), "你取過山藍染液浸潤粗麻料，反覆搓洗固色，為衣架上的防雨長衫添上一襲防雨藍靛。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_caravan_water_trough" || target.contains("青石飲馬槽") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_caravan_water_trough".to_string() },
            name: "青石飲馬槽".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "FeedDraftBeast".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "連通活水渠的深鑿青石長水槽，供來往跋涉的原牛與健壯駝獸低頭痛飲，水質清澈。".to_string()),
                ("Use".to_string(), "你拔開進水木栓注入新鮮井泉活水，奔波而來的原牛探頭大口痛飲，發出滿足低哼。".to_string()),
                ("FeedDraftBeast".to_string(), "你將新鮮甜燕麥與鮮嫩草料添入食槽，引來健壯馱獸垂首痛嚼，水槽泉水汨汨清澈。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_deed_cadastral_map_desk" || target.contains("四坊地籍案") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_deed_cadastral_map_desk".to_string() },
            name: "四坊地籍案".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Read".to_string(), "Use".to_string(), "InspectLandRegistry".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "寬闊平整的核契楠木長案上平鋪著文曜四坊地籍疆界全圖，一旁陳設著玻璃螺紋墨水瓶與弧形吸墨器。".to_string()),
                ("Read".to_string(), "你俯身查看四坊地籍圖冊，文曜城中各街各巷街屋的界址、戶號與公產邊界皆以硃砂與濃墨標記分明。".to_string()),
                ("Use".to_string(), "你執起木柄弧形吸墨器在草擬文契上輕輕滾過，新寫就的黑墨油光瞬間被乾淨吸附，字跡烏黑硬朗。".to_string()),
                ("InspectLandRegistry".to_string(), "你鋪開文曜四坊全圖，詳核街屋界標與公庫財賦，墨香撲鼻間取下一頁拓印地籍圖鈔。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_harbor_clockwise_ship" || target.contains("順時針環島海運商船") || target.contains("順時針環島海船") {
        return Some(crate::model::RoomObject {
            id: "wt_harbor_clockwise_ship".to_string(),
            name: "順時針環島海運商船".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "港灣深水泊位旁停靠著一艘雙桅硬帆的順時針環島海運商船，船首繪有破浪海燕紋章，水手們正忙碌清點壓艙石與淡水木桶。".to_string()),
                ("Use".to_string(), "你向船長交付船資，踏過跳板登上順時針環島海運商船，隨黑潮洋流啟程前往下一處海岸良港。".to_string()),
                ("BoardCoastalShip".to_string(), "你踏上順時針環島海運商船甲板，隨老舵手核對黑潮洋流與潮汐海圖，揚帆出海巡航。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dock_cargo_gangplank" || target.contains("雲杉卸貨板") {
        return Some(crate::model::RoomObject {
            id: "wt_dock_cargo_gangplank".to_string(),
            name: "雲杉卸貨板".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "BoardCoastalShip".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "石砌碼頭與貨船舷側之間搭著寬闊厚實的北嶺雲杉卸貨跳板，板面釘有防滑橫木條。".to_string()),
                ("Use".to_string(), "你穩步踏過微幅起伏的雲杉跳板，腳下木板發出沉穩紮實的吱呀聲響。".to_string()),
                ("BoardCoastalShip".to_string(), "你踏過厚實防滑的雲杉跳板登上環島商船，隨水手核對黑潮航線與潮汐水文。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_dining_iron_cauldron" || target == "wt_dining_rice_milk_stall" || target.contains("紅鐵燉肉釜") || target.contains("溫米漿桶") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_dining_iron_cauldron".to_string() },
            name: if target.contains("米漿") { "溫米漿桶".to_string() } else { "紅鐵燉肉釜".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "WarmHands".to_string(), "SeasonalBanquet".to_string(), "AssistKitchen".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅鐵厚底大釜慢燉著大角原牛肉與甜白蘿蔔，肉香四溢；旁邊紫銅熱桶溫著香濃米漿，熱氣騰騰。".to_string()),
                ("Taste".to_string(), "你舀起一勺慢燉的原牛肉湯送入口中，肉塊軟爛透味，湯頭鮮濃醇厚，整個人精神一振。".to_string()),
                ("WarmHands".to_string(), "你把雙手靠近大釜底座的耐火磚緣，溫熱厚重的火氣烘退了手掌上的水氣與寒意。".to_string()),
                ("SeasonalBanquet".to_string(), "你在紅鐵釜前添薪圍坐，鍋內歲時時令熱氣翻騰，同席之人舉杯共飲，暖意直透肺腑。".to_string()),
                ("AssistKitchen".to_string(), "你挽起衣袖在灶前添柴看火、揉麵分湯，協助大食堂備辦全城百工膳食。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_inn_milk_tea_stove" || target.contains("煮茶泥爐") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_inn_milk_tea_stove".to_string() },
            name: "煮茶泥爐".to_string(),
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "RoastMountainTea".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "紅泥小火爐上坐著青砂茶吊，炭火微熾，正煨焙著北嶺霜林採擷的高山青茶，茶韻幽長甘醇。".to_string()),
                ("Taste".to_string(), "你端起粗陶茶盞抿了一口熱茶，炭焙香氣在喉間化開，微苦回甘。".to_string()),
                ("RoastMountainTea".to_string(), "你夾起松木炭添入泥爐，輕搖竹扇翻焙鐵碾上的高山青茶，炭焙茶香四溢。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_herb_drying_kiln" || target == "wt_herb_bamboo_sieve" || target.contains("紅泥焙藥爐") || target.contains("切藥鍘刀案") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_herb_drying_kiln".to_string() },
            name: if target.contains("鍘刀") || target.contains("竹篩") { "切藥鍘刀案".to_string() } else { "紅泥焙藥爐".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Operate".to_string(), "RoastHerbs".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "百草烘藥房中整齊排設著紅泥焙藥爐與切藥鍘刀案，竹篩上鋪滿風乾的九蒸艾草、黃精與北嶺草藥，藥香撲鼻。".to_string()),
                ("Use".to_string(), "你手持切藥鍘刀切分曬透的草根藥材，刀口起落均勻，切出平整藥片。".to_string()),
                ("Operate".to_string(), "你撥勻焙藥爐中的文火炭灰，控制烘乾火候，使百草藥性均勻透發。".to_string()),
                ("RoastHerbs".to_string(), "你依古法炮製百草，文火慢焙陳艾與黃精，封入綿紙藥包妥善製成艾絨。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_study_desk_inkstone" || target == "wt_study_bamboo_bookshelf" || target == "wt_obj_stone_rubbing_cabinet" || target.contains("烏木書案") || target.contains("線裝古籍架") || target.contains("碑拓石櫥") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_study_desk_inkstone".to_string() },
            name: if target.contains("古籍") || target.contains("書架") { "線裝古籍架".to_string() } else if target.contains("石櫥") || target.contains("碑拓") { "碑拓石櫥".to_string() } else { "烏木書案".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Read".to_string(), "SunBooks".to_string(), "CollateInscriptions".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "古雅書齋中陳設著沉水烏木書案、線裝古籍架與碑拓石櫥，整齊收藏著歷代金石拓片與水土誌略。".to_string()),
                ("Use".to_string(), "你端坐烏木書案前研開松煙墨汁，提筆抄錄卷冊上的警句註釋。".to_string()),
                ("Read".to_string(), "你自架上取下一卷線裝古籍細讀，考證半島水文地脈與前賢遺制。".to_string()),
                ("SunBooks".to_string(), "你搬動書架卷冊置於向陽石坪翻曬，通風驅蠹，拂拭古籍紙灰。".to_string()),
                ("CollateInscriptions".to_string(), "你對照石櫥碑拓與古籍印本，逐字核勘金石異同與漫漶殘字，審定拓本。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_obs_brass_telescope_rack" || target == "wt_book_map_reading_table" || target.contains("青銅窺管架") || target.contains("全圖閱覽桌") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_obs_brass_telescope_rack".to_string() },
            name: if target.contains("閱覽") || target.contains("地圖") { "全圖閱覽桌".to_string() } else { "青銅窺管架".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Operate".to_string(), "Read".to_string(), "GrindOpticLens".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "觀星台與閱覽室內設有青銅窺管架與全圖閱覽桌，案旁備有細水研磨盤、金剛砂與紅土水晶鏡片毛胚。".to_string()),
                ("Operate".to_string(), "你微調青銅窺管架的仰角齒輪，對準遠方天際星位與海平線地標。".to_string()),
                ("Read".to_string(), "你在全圖閱覽桌上展閱半島星圖與等高線地勢總圖，參核各處隘口地標。".to_string()),
                ("GrindOpticLens".to_string(), "你蘸取細密金剛砂與冷泉活水，在旋轉磨盤上細細精磨紅土水晶鏡片，透光純淨明晰。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_market_produce_stand" || target == "wt_market_scale_counter" || target.contains("山海乾貨攤") || target.contains("黃銅雙盤秤") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_market_produce_stand".to_string() },
            name: if target.contains("秤") { "黃銅雙盤秤".to_string() } else { "山海乾貨攤".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Taste".to_string(), "Operate".to_string(), "SelectDriedSeafood".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "三水口海味乾貨行攤位上懸掛著金黃透亮的冷燻烏魚子、紫菜乾與乾貝，雙盤秤與精確砝碼擦拭得雪亮。".to_string()),
                ("Taste".to_string(), "你試嚐了一小片老頭家切下的薄烤烏魚子，醇濃黏齒、海味鮮香久久不散。".to_string()),
                ("Operate".to_string(), "你撥動黃銅雙盤秤上的滑碼與砝碼，稱量海味乾貨的斤兩份量，秤桿平準無差。".to_string()),
                ("SelectDriedSeafood".to_string(), "你迎著日光對光檢視烏魚子油潤成色，稱重包裝妥當，購下精美海味禮包。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_hall_council_table" || target == "wt_hall_bulletin_board" || target.contains("厚木公議桌") || target.contains("民務告示欄") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_hall_council_table".to_string() },
            name: if target.contains("告示") || target.contains("公佈") { "民務告示欄".to_string() } else { "厚木公議桌".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Sit".to_string(), "Read".to_string(), "DeliberateCivicPact".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "公議堂正中陳設著厚木公議桌，牆側立著民務告示欄，陳列著四坊公約草案、水利灌溉議程與公庫收支錄。".to_string()),
                ("Sit".to_string(), "你在公議桌旁厚木長椅落座，翻開桌上條陳，靜候街坊鄰里共商民事。".to_string()),
                ("Read".to_string(), "你閱覽告示欄與桌案上的鄉約條款，詳讀城坊水利分流與物料修防規定。".to_string()),
                ("DeliberateCivicPact".to_string(), "你與與會鄉紳街坊同席參議鄉約，各陳水土民生見解，共簽條約固本安民。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    if target == "wt_river_stone_steps" || target == "wt_alley_stone_well" || target.contains("汲水青石階") || target.contains("青石古井") {
        return Some(crate::model::RoomObject {
            id: if target.starts_with("wt_") { target.to_string() } else { "wt_river_stone_steps".to_string() },
            name: if target.contains("井") { "青石古井".to_string() } else { "汲水青石階".to_string() },
            owner: String::new(),
            sockets: vec!["Look".to_string(), "Use".to_string(), "Wash".to_string(), "DrawSpringWell".to_string(), "Rest".to_string(), "Sit".to_string()],
            responses: std::collections::HashMap::from([
                ("Look".to_string(), "沿岸石階浸入清冽山溪泉流，井台旁轆轤與青木水桶齊全，石隙間苔色青潤，水波盪漾。".to_string()),
                ("Use".to_string(), "你搖動轆轤打上一桶清澈泉水，水珠飛濺，清涼氣息拂面而來。".to_string()),
                ("Wash".to_string(), "你以冰涼甘冽的泉水清洗雙手與行裝泥塵，神清氣爽。".to_string()),
                ("DrawSpringWell".to_string(), "你自水岸公井汲取一竹筒清冽泉水飲用潤嗓，甘泉入口沁人心脾，頓消燥氣。".to_string()),
                ("Rest".to_string(), "你在沁涼的水岸青石階旁落座歇腳，聽著潺潺溪流拍岸，微風拂過面龐，行路疲憊頓消。".to_string()),
                ("Sit".to_string(), "你在沁涼的水岸青石階旁落座歇腳，聽著潺潺溪流拍岸，微風拂過面龐，行路疲憊頓消。".to_string()),
            ]),
            move_to_room_id: String::new(),
        });
    }
    None
}

fn resolve_room_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    let target = target_id.trim();
    if let Some(obj) = resolve_special_room_object_part1(room_id, target) {
        return Some(obj);
    }
    if let Some(obj) = resolve_special_room_object_part2(target) {
        return Some(obj);
    }
    get_object_by_id_in_room(room_id, target_id)
        .or_else(|| get_object_by_name_in_room(room_id, target_id))
        .or_else(|| get_object_and_room(target_id).and_then(|(o, r)| (r == room_id).then_some(o)))
        .or_else(|| resolve_grid_scene_object(room_id, target_id))
        .or_else(|| resolve_runtime_resource(room_id, target_id))
        .or_else(|| {
            game::player_layer_scene_objects_for_room(room_id)
                .into_iter()
                .find(|obj| obj.id == target_id || obj.name == target_id || obj.name.contains(target_id) || target_id.contains(&obj.name))
        })
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(target_id)
                .map(|e| e.to_room_object())
        })
}

fn resolve_room_name(player_room: &str) -> String {
    if let Some(coord) = parse_grid_room_id(player_room) {
        if let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord) {
            return cell.name.clone();
        }
    }
    player_room.to_string()
}

fn current_zone_weather(room_name: &str) -> crate::game::ZoneWeatherPhase {
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let tick = (crate::game::now_unix() / 180) as i64;
    crate::game::current_zone_weather_and_tide(z_idx, tick).0
}

fn current_zone_tide(room_name: &str) -> crate::game::ZoneTidePhase {
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let tick = (crate::game::now_unix() / 180) as i64;
    crate::game::current_zone_weather_and_tide(z_idx, tick).1
}

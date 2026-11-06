 /// 確保方格座標 (x, y) 有駐場環境人物（若該處尚無活體人物且非水域、非城牆）。
pub fn ensure_ambient_npc_at_grid(x: i32, y: i32, room_name: &str) -> Option<Character> {
    let (zone, _sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if terrain == crate::grid::Terrain::WaterDeep || terrain == crate::grid::Terrain::Wall {
        return None;
    }

    let time_bucket = (crate::game::now_unix() / 300) as i64;
    let seed = ((x as i64).wrapping_mul(374761393)
        ^ (y as i64).wrapping_mul(668265263)
        ^ time_bucket.wrapping_mul(1442695040888963407))
        .unsigned_abs();
    // 三層生態顯形分配：
    // - Tier 1 鎮場霸主（roll == 0）：預設隱匿不直接站樁，唯有尋跡觸發（room_name == "__apex__" 或已標記尋跡）始破霧顯形
    // - Tier 4 尋常小怪與靈禽（roll == 3, 4）：佔顯形遭遇 65%（野兔、穿山甲、獼猴、飛鼠、環頸雉、竹雞、招潮蟹、青毛蟹等直接顯形於格內）
    // - Tier 3 尋常中型獸群（roll == 2）：佔顯形遭遇 20%（梅花鹿、水鹿、山羌、長鬃山羊、山豬成群游走顯形）
    // - Tier 2 掠食者（roll == 1）：佔 7%
    let roll = if room_name == "__apex__" {
        0
    } else if room_name == "__tier2__" || room_name == "__tier2_overflow__" {
        1
    } else if room_name == "__tier3__" {
        2
    } else if room_name == "__tier4a__" {
        3
    } else if room_name == "__tier4b__" {
        4
    } else if is_apex_beast_tracked_at(x, y)
        && get_zone_trophic_state(nearest_hunting_zone_idx(x, y))
            .is_some_and(|s| s.apex_presence >= 0.95)
    {
        0
    } else {
        match seed % 100 {
            0..=34 => 3,  // 35% 尋常小型奇獸／甲殼小怪（格內顯形）
            35..=69 => 4, // 35% 尋常山海靈禽小怪（格內顯形）
            70..=89 => 2, // 20% 尋常中型主力獸群（格內顯形）
            _ => 1,       // 10% 掠食猛獸
        }
    };

    let (creature_name, creature_activity) = match zone {
            "長風草甸" => match roll {
                0 => ("長風四目水鹿王", "昂首聳立於草甸深處，雙叉巨角直指蒼穹，四目靈光威震四野"),
                1 => ("淺山錢紋石虎", "伏行於長風草甸灌叢陰影間，耳後白斑微動、伺機撲擊野兔"),
                2 => ("平原梅花鹿", "優雅地在草甸花叢間結群嚼草，白色梅花斑紋隨風微動"),
                3 => ("草甸野灰兔", "四足如飛，在齊膝芒草間迅疾掠過帶起陣陣草浪"),
                _ => ("草甸環頸雉", "披著絢麗五彩羽衣在芒草中昂首啼鳴，振翅破風"),
            },
            "古河巨木林" => match roll {
                0 => ("古河樟林雲豹", "無聲無息潛行於千年古樟巨枝之間，暗影中古玉雲紋隱現"),
                1 => ("錢紋石虎", "踏在千年古樟厚軟腐葉層上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("四目水鹿", "生有雙重眶下靈腺，在巨木霧氣中結群踏水而行"),
                3 => ("古林獼猴", "攀附於千年楠木藤蔓間靈巧擺盪，凝神張望林下動靜"),
                _ => ("巨木領角鴞", "棲息於高枝樹洞旁，夜色中金黃雙瞳凝視林間落葉"),
            },
            "赤石坡" => match roll {
                0 => ("赤石鯪鯉穿山甲", "盤踞於赤紅土石礦坡，周身烏金厚鱗在烈日下泛著鐵光"),
                1 => ("淺山錢紋石虎", "伏行於赤石灌木叢陰影間，耳後白斑微動、目光如電"),
                2 => ("赤崖長鬃山羊", "腳踏垂直陡峭的赤褐色岩壁，如履平地凌空躍動"),
                3 => ("赤坡山羌", "在赤石坡蕨草間警覺昂首，發出清亮短促的吠鳴"),
                _ => ("赤崖大冠鷲", "展開黑白條紋巨翼在赤石坡高空乘熱氣流盤旋長鳴"),
            },
            "落日岬" => match roll {
                0 => ("落日黑鳶王", "逆著落日海風在岬角萬仞高空盤旋，尾羽如魚尾開合控風"),
                1 => ("海岬百年綠蠵龜", "伏於落日殘照的岬角礫灘，背甲印刻著百載海潮紋理"),
                2 => ("海崖長鬃山羊", "於狂暴海風撕扯的岬角懸崖邊緣穩健覓食耐鹽灌木"),
                3 => ("烏腳香白鼻心", "穿梭於海岬防風林與林投果叢間，鼻端白線分明"),
                _ => ("迎風海角雕", "逆著浩瀚西風在海角高空盤旋，神目如電巡視汪洋"),
            },
            "驚濤岩岸" => match roll {
                0 => ("驚濤遊隼王", "立於怒濤飛濺的玄武岩尖端，俯衝速度冠絕百禽、破浪裂風"),
                1 => ("裂浪灰海獺", "順著拍岸碎浪滑入急流，在礁石暗湧間靈動捕魚"),
                2 => ("斷崖長鬃山羊", "踏立於海浪拍擊的險峻岩岸峭壁，迎著鹹霧穩立不搖"),
                3 => ("潮間青岩巨蟹", "披著青黑重甲駐守於潮池裂罅，雙螯張合鏗鏘作響"),
                _ => ("驚濤黑岩鷺", "羽色深沉如玄武礁岩，單足凝立於碎浪礁石上伺機啄魚"),
            },
            "巨藻淺灣" => match roll {
                0 => ("淺灣百年綠蠵龜", "悠然浮游於隨潮搖曳的巨藻林間，背甲泛著翠綠海藻幽光"),
                1 => ("渡海黑面琵鷺", "在淺灣潮間帶優雅左右掃動黑長扁喙，成群涉水覓食"),
                2 => ("淺灣灰海獺", "胸口抱著青石在浮動巨藻叢中翻身仰游，悠閒自得"),
                3 => ("潮汐招潮蟹", "高舉鮮紅大螯在淺灣泥沙灘上整齊揮舞，隨潮水退湧進退"),
                _ => ("白鷺候鳥", "單足凝立於淺灣清流白浪之中，凝視游魚伺機啄擊"),
            },
            "黑石灣" => match roll {
                0 => ("黑石成雙海鱟", "身披青銅馬蹄硬甲成對伏於黑石潮池，劍尾如戟古意盎然"),
                1 => ("黑背玄武岩龜", "伏在黝黑巨石上吐納海霧，背甲深沉如萬年古墨"),
                2 => ("玄武岩大海蟹", "藏身於黑石灣潮間玄武岩裂隙，甲殼堅硬如鐵石"),
                3 => ("浪蝕九孔鮑", "如鐵盔般緊緊吸附於黑石浪痕之上，殼表珍珠層生輝"),
                _ => ("白腹鰹鳥", "自黑石海灣上空收翼垂直扎入浪花，精準捕食潮間游魚"),
            },
            "翠冠巨木林" => match roll {
                0 => ("千年樟雲豹王", "立於最高樹冠交疊處俯瞰整座山谷，周身雲紋隱隱發光"),
                1 => ("錢紋石虎", "踏在林下厚軟苔原上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("巨角巡林水鹿", "頭戴分叉如戟的巨型長角，沉穩邁過翠綠灌木叢"),
                3 => ("烏腳香白鼻心", "順著老樟樹皮輕巧滑下，鼻端白線分明，自帶草果幽香"),
                _ => ("丹冠藍腹鷴", "寶藍色羽翼拖著雪白長尾，赤足朱冠在巨木林蔭下踱步"),
            },
            "清泉溪谷" => match roll {
                0 => ("清泉櫻花鉤吻鮭", "冰河孑遺靈魚逆著融雪清泉奮力躍水，身側胭脂梅花斑紋熠熠生輝"),
                1 => ("溪澗食蟹獴", "披著棕灰簑衣長毛在清泉卵石灘間敏捷穿梭，破殼取蟹"),
                2 => ("清泉山羌", "自溪邊蕨草叢探出小巧頭顱，低頭飲用冰涼泉水"),
                3 => ("清溪青毛蟹", "螯足生滿濃密青黑絨毛，在清澈見底的卵石溪流中側行"),
                _ => ("溪澗鉛色水鶇", "停在溪心濕石上輕巧展開栗紅尾羽，隨流水聲清脆鳴唱"),
            },
            "北坡雲杉林" => match roll {
                0 => ("胸白月熊王", "人立於百年雲杉之下發出渾厚咆哮，胸口一抹新月白毫如雪"),
                1 => ("雲杉霧林帝雉", "披著深藍光澤羽衣在雲杉冷霧間緩步巡行，氣度如王者"),
                2 => ("剛鬃黑山豬", "粗壯如石的獠牙向上挑起，前蹄刨掘著雲杉林下堅根"),
                3 => ("北坡山林獼猴", "掛在雲杉垂枝上抓耳撓腮，朝樹下警覺張望"),
                _ => ("高山角鷹", "展開寬闊黑褐花翎，傲視千山盤旋於雲杉密林之上"),
            },
            "西麓林道" => match roll {
                0 => ("西麓丹冠藍腹鷴", "朱紅肉冠配寶藍長羽，在西麓古道晨霧中從容踱步"),
                1 => ("長尾山娘", "拖著琉璃藍長尾成群掠過林道樹冠，彼此呼應守望"),
                2 => ("林道山羌", "在古道兩側闊葉林蔭下輕步覓食嫩葉，聞聲即躍入林"),
                3 => ("食蛇箱龜", "背甲金黃隆起如盾，從容不迫地在碎石落葉間挪行"),
                _ => ("山林竹雞", "在林道兩側密竹叢間結伴啄食草籽，拍翅低飛"),
            },
            "淺澤泥口" => match roll {
                0 => ("淺澤重角水牛王", "雙角寬闊如彎月，半身浸在涼爽泥澤中閉目反芻，氣息沉穩如山"),
                1 => ("沼澤食蟹獴", "咬著泥蟹在蘆葦泥渚間翻滾滑行，皮毛防濕油亮"),
                2 => ("泥口柴棺水龜", "背甲呈古銅褐色，成群靜伏於淺澤浮木上曬日吐納"),
                3 => ("紅螯泥沼蟹", "高舉血紅巨螯在泥孔外威嚇對手，吐著細密水泡"),
                _ => ("凌波水雉", "拖著修長飄逸尾羽輕踏菱角浮葉之上，宛若凌波仙禽"),
            },
            "白茫洲" => match roll {
                0 => ("黑沼百步蛇王", "盤踞於迷霧泥澤枯木之上，三角巨首與菱紋鱗甲透出凜然古靈威壓"),
                1 => ("黑沼飯匙倩", "在白茫洲水霧草叢中昂起前半身，頸部張開如飯匙般威懾四方"),
                2 => ("白茫四目水鹿", "通體沾滿晨霧水珠，頭頂雙叉長角踏過泥水無痕"),
                3 => ("翡翠綠樹蛙", "背脊閃爍翠玉光澤，靜靜蹲在濕潤水芋葉片上鳴叫"),
                _ => ("沼洲夜鷺", "縮頸佇立於迷霧淺灘枯木上，赤紅眼睛穿透重霧"),
            },
            "貼山深潭" => match roll {
                0 => ("寒潭千年花斑鱸鰻", "碗口粗的花斑巨軀潛伏於深碧潭底石洞，游動時捲起暗湧漩渦"),
                1 => ("高山苦花魚王", "銀光閃爍躍出寒潭激流，腹部金線在陽光下熠熠生輝"),
                2 => ("潭畔四目水鹿", "立於深潭岸邊青石上結伴俯身飲水，長角倒映於碧綠潭面"),
                3 => ("寒潭山椒魚", "冰河孑遺靈獸伏於寒潭水際冷石下，褐膚帶淺斑、喜極寒清泉"),
                _ => ("寒潭翠鳥", "披著寶石藍羽衣自潭邊蘆葦疾射入水，銜起銀魚破水而出"),
            },
            "青籠關" => match roll {
                0 => ("青籠大冠鷲王", "傲立於青籠關千仞險峰之巔，羽冠高聳、長鳴聲震動整條南境山隘"),
                1 => ("關隘錢紋石虎", "潛伏在關道荊棘石縫暗處，矯健斑紋身軀蓄勢待發"),
                2 => ("剛鬃黑山豬", "粗壯獠牙泛著冷光，在關隘山坳間拱土覓食"),
                3 => ("鯪鯉穿山甲", "捲成堅不可摧的烏金鱗球滾過山隘石坡，刀劍難傷"),
                _ => ("關隘長鬃山羊", "敏捷踏過險峻關石窄徑，灰色長鬃在山風中飛舞"),
            },
            "西嶺礦谷" => match roll {
                0 => ("裂石鯪鯉穿山甲王", "全身烏金鱗甲堅硬如百鍛精鋼，利爪一揮便刨開堅硬赤鐵礦岩"),
                1 => ("礦谷胸白月熊", "在西嶺礦谷深處岩洞前巡視領地，胸前新月白斑清晰奪目"),
                2 => ("青脊長鬃山羊", "攀行於近乎垂直的露頭礦脈岩壁上，蹄尖穩扣寸許石縫"),
                3 => ("礦谷黑野豬", "長吻帶泥在煤石堆旁衝撞覓食，渾身沾滿硬結礦泥"),
                _ => ("高山金翼白眉", "眉間白紋配金橙翼斑，在礦谷冷杉枝頭清脆吟唱"),
            },
            "東海峭壁" => match roll {
                0 => ("馭風角鷹王", "凌空盤旋俯瞰東海萬頃波浪，雙翼遮雲蔽日、頭頂雙簇角冠威凜"),
                1 => ("破浪遊隼", "自千仞海崖順風俯衝，雙翼收攏如精鋼利箭掠過海面"),
                2 => ("斷崖長鬃山羊", "腳踏險絕東海斷崖上的寸許石階，成群從容縱躍如飛"),
                3 => ("峭壁大赤飛鼠", "自海風呼嘯的懸崖樹梢縱身一躍，乘著上升氣流滑向對岸"),
                _ => ("峭壁岩鷚", "羽色如花崗岩斑駁，在臨海峭壁石隙中逆風跳躍覓食"),
            },
            "雲腰雪林" => match roll {
                0 => ("霧林黑長尾帝雉", "披著深藍曜黑羽衣拖著雪白橫紋長尾，漫步於高山圓柏與箭竹積雪中"),
                1 => ("雪線胸白月熊", "頂著嚴寒風雪在鐵杉雪林中巡狩，掌力能折斷百年冰枝"),
                2 => ("踏雪四目水鹿", "在寒霧積雪中安詳漫步，四目顧盼清明出塵"),
                3 => ("雪林山椒魚", "潛藏於雪林融冰石縫之下，耐受極寒冰泉"),
                _ => ("雪林酒紅朱雀", "全身羽色如陳年丹砂般鮮豔，在覆雪鐵杉枝頭宛若紅梅綻放"),
            },
            "白塔雪巔" => match roll {
                0 => ("雪巔霜冠大水鹿", "傲然立於冰封絕頂石柱之巔，霜白巨角如玉樹臨風、吐納天地至純清氣"),
                1 => ("極頂黑長尾帝雉", "在萬仞雪峰雲海邊緣漫步，長尾白紋與萬年玄冰交相輝映"),
                2 => ("雪脊長鬃山羊", "蹄踏玄冰雪脊，在萬丈虛空之上從容齧食高山冰苔"),
                3 => ("雪巔冰河山椒魚", "棲息於白塔極頂冰川融水石隙，見證萬載寒暑"),
                _ => ("雪峰岩鷚", "羽如灰石斑駁，在冰封岩頂上不懼酷寒歡快啄食細雪草籽"),
            },
            _ => match roll {
                0 => ("山林梅花鹿", "警覺地站在草丘上環顧四周風吹草動"),
                1 => ("烏腳香白鼻心", "在矮灌木叢中靈巧鑽動尋找甜美野果"),
                2 => ("雜林野灰兔", "在草葉間抽動鼻尖，後肢蹬地飛快奔竄"),
                3 => ("山林竹雞", "在路旁草葉間結隊咕咕低鳴覓食草籽"),
                _ => ("草叢龜殼花", "盤曲於厚軟草叢陰影間，冷眼凝視過往生靈"),
            },
    };

    let display_char = creature_name.chars().next().map(|c| c.to_string()).unwrap_or_default();

    let (vit, qi, dex) = match roll {
        0 => (120, 60, 30), // Tier 1 Apex
        1 => (55, 28, 26),  // Tier 2 Predator
        2 => (30, 15, 20),  // Tier 3 Medium Game
        3 => (16, 10, 24),  // Tier 4 Small Fauna
        _ => (18, 12, 28),  // Tier 4 Avian
    };
    Some(Character {
        id: format!("ambient_{}_{}", x, y),
        kind: EntityKind::Npc,
        display_char,
        x,
        y,
        move_state: crate::entity::MoveState::Idle,
        target_x: None,
        target_y: None,
        walk_or_run: crate::entity::WalkOrRun::Walk,
        move_started_at: None,
        vit,
        qi,
        dex,
        magnesium: 0,
        last_observed_at: Some(crate::game::now_unix()),
        created_at: crate::game::now_unix(),
        gender: None,
        soul_seed: None,
        display_title: creature_name.to_string(),
        activated_nodes: "[]".to_string(),
        equipment_slots: "{}".to_string(),
        inventory: "[]".to_string(),
        disposition: 0,
        thoughts: vec![],
        current_activity: {
            let gh = crate::store::current_game_hour();
            let mut act = creature_activity.to_string();
            if matches!(
                creature_name,
                "夜嘯大赤飛鼠" | "烏腳香白鼻心" | "錢紋石虎" | "石隙雪鴞" | "沼洲夜鷺" | "峭壁飛鼠"
            ) && !(6..18).contains(&gh)
            {
                act.push_str("（夜行時分：正乘著夜色幽暗無聲掠出，雙目在月華下爍爍生輝）");
            } else if matches!(
                creature_name,
                "逐日七叉白鹿" | "霧林黑長尾帝雉" | "丹冠藍腹鷴" | "白茫角鹿" | "踏雪四目水鹿"
            ) && (5..9).contains(&gh)
            {
                act.push_str("（晨霧瑞兆：正迎著破曉山嵐吐納朝露，神駿逼人）");
            } else if matches!(
                creature_name,
                "深海大王烏賊" | "深海皇帶龍魚" | "載嶼百年巨鼇" | "噴玉海翁巨鯨" | "海岬豆腐鯨鯊"
            ) && gh % 6 <= 1
            {
                act.push_str("（大潮現身：正乘著黑水潮汐退湧之際浮現海面，引動暗流轟鳴）");
            }
            act
        },
        grid_x: Some(x),
        grid_y: Some(y),
    })
}

/// 將山海環境生物名稱映射至千物總典物品 ID、名稱與《白塔山海誌》典籍描述。
pub fn ambient_creature_loot(creature_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if creature_name.ends_with('客')
        || creature_name.ends_with('人')
        || creature_name.ends_with('手')
        || creature_name.ends_with('民')
        || creature_name.ends_with('卒')
        || creature_name.ends_with('工')
        || creature_name.ends_with('農')
        || creature_name.ends_with('夫')
        || creature_name.ends_with('衛')
        || creature_name.ends_with('士')
        || creature_name.ends_with('員')
        || creature_name.ends_with('戶')
        || creature_name.ends_with('者')
        || creature_name.ends_with('商')
        || creature_name.ends_with('勇')
    {
        return None;
    }
    if creature_name.contains("帝雉") {
        Some(("wt_cat_0214", "霧林帝雉長尾翎", "《白塔山海誌》：烏雉生於雲霧鐵杉之巔，尾長三尺而橫白紋。"))
    } else if creature_name.contains("藍腹鷴") {
        Some(("wt_cat_0216", "丹冠藍腹鷴彩羽", "《白塔山海誌》：丹冠白尾鷴羽泛寶藍幽光，見之吉祥。"))
    } else if creature_name.contains("豹") {
        Some(("wt_cat_0172", "樟林雲豹軟毛皮", "《白塔山海誌》：雲豹身披古玉雲紋，佩其皮則山行不躓。"))
    } else if creature_name.contains("熊") {
        Some(("wt_cat_0205", "胸白月熊厚熊掌", "《白塔山海誌》：月熊胸生白毫如新月，其掌食之禦風寒。"))
    } else if creature_name.contains("穿山甲") || creature_name.contains("鯪鯉") {
        Some(("wt_cat_0188", "鯪鯉穿山甲鱗片", "《白塔山海誌》：鯪鯉遍體烏金硬鱗，炙之通經絡、綴甲禦刃。"))
    } else if creature_name.contains("龜") || creature_name.contains("鼇") {
        Some(("wt_cat_0252", "深潭野生老鱉", "《白塔山海誌》：靈龜玄甲堅實如盾，息於清泉林澗。"))
    } else if creature_name.contains("獴") || creature_name.contains("獺") || creature_name.contains("獾") {
        Some(("wt_cat_0183", "白鼻心烏腳香皮", "《白塔山海誌》：食蟹獴喜伏於溪澗泥澤捕食溪蟹水蟲，身形矯捷靈敏，毛皮厚實防潮。"))
    } else if creature_name.contains("蟹") || creature_name.contains("鱟") {
        Some(("wt_cat_0233", "紅螯大青蟹", "《白塔山海誌》：鐵甲巨螯伏於潮間泥灘，雙鉗力能夾石。"))
    } else if creature_name.contains("蛙") || creature_name.contains("蟾") || creature_name.contains("鯢") || creature_name.contains("山椒魚") {
        Some(("wt_cat_0275", "草叢野生牛蛙", "《白塔山海誌》：水澤兩棲靈物伏於寒潭草澗，喜清泉雨霧。"))
    } else if creature_name.contains("海膽") || creature_name.contains("螺") || creature_name.contains("鮑") {
        Some(("wt_cat_0223", "野生九孔鮑", "《白塔山海誌》：潮間岩穴甲殼海珍，味極甘鮮。"))
    } else if creature_name.contains("蛇") || creature_name.contains("蟒") || creature_name.contains("蝮") || creature_name.contains("百足") {
        Some(("wt_cat_0189", "百步蛇乾蛻", "《白塔山海誌》：百步大蛇首如三角鐵鏟，其蛻研粉可解瘴毒。"))
    } else if creature_name.contains("水鹿") || creature_name.contains("白鹿") || creature_name.contains("梅花鹿") || creature_name.contains("角鹿") || creature_name.contains("鹿") {
        Some(("wt_cat_0203", "高地水鹿厚鹿皮", "《白塔山海誌》：四目靈鹿生於高山溪谷，其皮堅韌耐水火。"))
    } else if creature_name.contains("山羌") || creature_name.contains("羌") {
        Some(("wt_cat_0209", "山羌柔軟麂皮", "《白塔山海誌》：山羌鳴聲如犬，其皮細膩溫軟。"))
    } else if creature_name.contains("羊") || creature_name.contains("羚") {
        Some(("wt_cat_0179", "長鬃山羊彎角", "《白塔山海誌》：長鬃岩羚善登千仞絕壁，其角堅韌可為號角。"))
    } else if creature_name.contains("地牛") || creature_name.contains("牛") {
        Some(("wt_cat_0187", "大角原牛厚蹄皮", "《白塔山海誌》：巨牛伏行草甸與地脈，蹄皮厚韌冠絕百獸。"))
    } else if creature_name.contains("豬") {
        Some(("wt_cat_0211", "海崖黑野豬牙", "《白塔山海誌》：剛鬃山豬力能拱石，其獠牙彎如新月。"))
    } else if creature_name.contains("飛鼠") || creature_name.contains("鼯") {
        Some(("wt_cat_0182", "大赤飛鼠厚絨毛", "《白塔山海誌》：赤鼯夜飛巨木之間，其絨輕暖勝狐裘。"))
    } else if creature_name.contains("白鼻心") || creature_name.contains("狐") || creature_name.contains("麝") || creature_name.contains("貂") || creature_name.contains("猴") {
        Some(("wt_cat_0183", "白鼻心烏腳香皮", "《白塔山海誌》：烏腳香與山海毛獸善棲林澗礁灣，毛皮細密隔水且帶草果清香。"))
    } else if creature_name.contains("石虎") || creature_name.contains("虎") || creature_name.contains("狼") || creature_name.contains("犬") {
        Some(("wt_cat_0171", "山林石虎斑皮", "《白塔山海誌》：石虎耳後生白斑，夜行山林迅捷如電。"))
    } else if creature_name.contains("兔") || creature_name.contains("鼠") {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《白塔山海誌》：野兔潛於芒草石縫，其絨護掌不畏寒霜。"))
    } else if creature_name.contains("鱷") || creature_name.contains("沼龍") || creature_name.contains("蜥") {
        Some(("wt_cat_0176", "泥甲巨鱷厚麟皮", "《白塔山海誌》：泥甲巨獸伏於腐葉深澤，背麟刀斧難入。"))
    } else if creature_name.contains("烏賊") || creature_name.contains("花枝") {
        Some(("wt_cat_0236", "大花枝鮮黑墨囊", "《白塔山海誌》：深海巨鰂眼如銅鑼，墨囊濃黑如漆。"))
    } else if creature_name.contains("鯨") || creature_name.contains("海翁") || creature_name.contains("鯊") {
        Some(("wt_cat_0220", "豆腐鯊厚砂魚皮", "《白塔山海誌》：海翁巨鯊浮海如嶼，其皮粗韌如砂石。"))
    } else if creature_name.contains("龍魚") || creature_name.contains("鬼頭刀") || creature_name.contains("苦花") || creature_name.contains("鯇") || creature_name.contains("鰻") || creature_name.contains("魚") {
        Some(("wt_cat_0224", "銀鱗土魠魚", "《白塔山海誌》：外海大魚銀鱗閃爍，食之益氣強身。"))
    } else if creature_name.contains("鷹") || creature_name.contains("雕") || creature_name.contains("隼") || creature_name.contains("鴞") {
        Some(("wt_cat_0192", "高山熊鷹翎羽", "《白塔山海誌》：高山角鷹盤旋絕壁，其翎破空不顫。"))
    } else if creature_name.contains("雉") || creature_name.contains("雞") || creature_name.contains("鳥") || creature_name.contains("鷗") || creature_name.contains("鷺") || creature_name.contains("燕") || creature_name.contains("鷚") || creature_name.contains("眉") || creature_name.contains("雀") || creature_name.contains("鳩") {
        Some(("wt_cat_0173", "水鳥蓬鬆絨羽", "《白塔山海誌》：山海飛禽絨羽輕柔，禦寒隔濕。"))
    } else {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《白塔山海誌》：山海林野走獸潛行草木岩壑之間，毛皮溫韌。"))
    }
}


pub fn resolve_shanhai_or_ambient_entity_id(id: &str) -> Option<Character> {
    if let Some(rest) = id.strip_prefix("shanhai_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 4 {
            let z_idx: usize = parts[0].parse().ok()?;
            let slot_idx: usize = parts[1].parse().ok()?;
            let x: i32 = parts[2].parse().ok()?;
            let y: i32 = parts[3].parse().ok()?;
            let tier_tag = ZONE_HABITAT_LAYOUTS
                .get(z_idx)
                .and_then(|v| v.get(slot_idx))
                .map(|e| if e.4 == "__tier2_overflow__" { "__tier2__" } else { e.4 })
                .unwrap_or(match slot_idx {
                    0 => "__apex__",
                    1..=6 => "__tier2__",
                    _ => "__tier4b__",
                });
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, tier_tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    if let Some(rest) = id.strip_prefix("ambient_flock_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().ok()?;
            let y: i32 = parts[1].parse().ok()?;
            let time_bucket = (crate::game::now_unix() / 180) as i64;
            let roam_gate = ((x as i64).wrapping_mul(1103515245)
                ^ (y as i64).wrapping_mul(12345)
                ^ time_bucket.wrapping_mul(2654435761))
                .unsigned_abs()
                % 100;
            let flock_tag = if roam_gate % 2 == 0 { "__tier4a__" } else { "__tier4b__" };
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, flock_tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    if let Some(rest) = id.strip_prefix("ambient_") {
        let parts: Vec<&str> = rest.split('_').collect();
        if parts.len() == 2 {
            let x: i32 = parts[0].parse().ok()?;
            let y: i32 = parts[1].parse().ok()?;
            let tag = if is_apex_beast_tracked_at(x, y) { "__apex__" } else { "" };
            if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, tag) {
                amb.id = id.to_string();
                return Some(amb);
            }
        }
        return None;
    }

    None
}

/// 為「入山百工」挑選遍布該獵場全境真實疆域的採集／匠作航點，並回傳所在獵場名稱與子地形（支援休養期自動轉場）。
pub fn pick_zone_artisan_waypoint_with_zone(
    zone_name: &str,
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str, &'static str) {
    let z_idx = CANONICAL_HUNTING_ZONES
        .iter()
        .position(|(zn, _, _)| *zn == zone_name)
        .unwrap_or(0);

    let is_fallow = get_zone_trophic_state(z_idx)
        .map(|s| s.resource_fallow_mode)
        .unwrap_or(false);

    let mut target_z_idx = z_idx;
    if is_fallow && (seed % 2 == 0) {
        let neighbors = nearest_geographic_zones(z_idx);
        let richest = neighbors
            .iter()
            .copied()
            .filter_map(|nid| {
                let st = get_zone_trophic_state(nid)?;
                if st.timber_w >= 550.0 || st.mineral_m >= 550.0 || st.fishery_f >= 550.0 {
                    let total = st.timber_w + st.mineral_m + st.fishery_f;
                    Some((total, nid))
                } else {
                    None
                }
            })
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        if let Some((_, best_nid)) = richest {
            target_z_idx = best_nid;
        }
    }

    let actual_zone_name: &'static str = CANONICAL_HUNTING_ZONES
        .get(target_z_idx)
        .map(|(zn, _, _)| *zn)
        .unwrap_or("長風草甸");

    let base_list = &ZONE_HABITAT_LAYOUTS[target_z_idx];
    if base_list.len() > 27 {
        let pool = &base_list[1..]; // 排除 slot 0 鎮場霸主屏息心
        let start = (seed as usize) % pool.len();
        for offset in 0..pool.len() {
            let &(px, py, sub_geo, ..) = &pool[(start + offset * 7) % pool.len()];
            if is_inside_settlement_urban_core(px, py) {
                continue;
            }
            let dist = (px - current.0).abs() + (py - current.1).abs();
            if dist >= 18 {
                return ((px, py), actual_zone_name, sub_geo);
            }
        }
        for &(px, py, sub_geo, ..) in pool {
            if !is_inside_settlement_urban_core(px, py) {
                return ((px, py), actual_zone_name, sub_geo);
            }
        }
        let &(px, py, sub_geo, ..) = &pool[start];
        return ((px, py), actual_zone_name, sub_geo);
    }
    let &(_, cx, cy) = &CANONICAL_HUNTING_ZONES[target_z_idx];
    ((cx, cy), actual_zone_name, "深山林地")
}

pub fn pick_zone_artisan_waypoint(
    zone_name: &str,
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let (pos, _actual_zone, sub_geo) = pick_zone_artisan_waypoint_with_zone(zone_name, seed, current);
    (pos, sub_geo)
}

static ISLAND_ROAD_MILESTONES: std::sync::OnceLock<Vec<(i32, i32, &'static str)>> =
    std::sync::OnceLock::new();

fn get_island_road_milestones() -> &'static [(i32, i32, &'static str)] {
    if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
        return list.as_slice();
    }
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let mut roads = Vec::new();
        for cell in grid.cells() {
            let coord = cell.coord;
            if (cell.terrain == crate::grid::Terrain::Road
                || cell.terrain == crate::grid::Terrain::Bridge)
                && !is_inside_settlement_urban_core(coord.x, coord.y)
            {
                let (zone, _, _) = crate::grid::r#gen::classify_wilderness_cell(coord.x, coord.y);
                roads.push((coord.x, coord.y, zone));
            }
        }
        roads.sort_by_key(|&(x, y, _)| (y, x));
        // 每隔 4 格取一處古道驛路里程點，確保均勻覆蓋全島 6,287 格驛道與古道網路
        let mut milestones = Vec::new();
        for (idx, item) in roads.into_iter().enumerate() {
            if idx % 4 == 0 {
                milestones.push(item);
            }
        }
        if !milestones.is_empty() {
            let _ = ISLAND_ROAD_MILESTONES.set(milestones);
            if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
                return list.as_slice();
            }
        }
    }
    &[]
}

/// 為「古道行旅（挑夫頭、擔夫、押貨管事、刀牌手、巡隘銃手）」挑選全島驛道與聯外古道網上的真實道路航點。
pub fn pick_island_trail_waypoint(
    zone_name: &str,
    home: (i32, i32),
    errand: (i32, i32),
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let milestones = get_island_road_milestones();
    if !milestones.is_empty() {
        // 優先挑選屬於該獵場或位於「居所 <-> 獵場 <-> 交割市集」走廊上的真實道路格
        let mid_x = (home.0 + errand.0) / 2;
        let mid_y = (home.1 + errand.1) / 2;
        let local: Vec<(i32, i32, &'static str)> = milestones
            .iter()
            .copied()
            .filter(|&(rx, ry, rzone)| {
                rzone == zone_name
                    || ((rx - mid_x).abs() <= 110 && (ry - mid_y).abs() <= 110)
                    || ((rx - home.0).abs() <= 95 && (ry - home.1).abs() <= 95)
            })
            .collect();
        let pool = if local.len() >= 6 {
            local.as_slice()
        } else {
            milestones
        };
        let start = (seed as usize) % pool.len();
        for step in 0..pool.len() {
            let (rx, ry, rzone) = pool[(start + step * 5) % pool.len()];
            let dist = (rx - current.0).abs() + (ry - current.1).abs();
            if (16..=140).contains(&dist) {
                return ((rx, ry), rzone);
            }
        }
        let (rx, ry, rzone) = pool[start];
        return ((rx, ry), rzone);
    }
    pick_zone_artisan_waypoint(zone_name, seed, current)
}

/// 19 大獵場首領異獸威壓與殘跡異兆（若該地帶或場景對應既定獵場）。
pub fn hunting_zone_apex_omen(zone_or_room: &str) -> Option<&'static str> {
    if zone_or_room.contains("長風草甸") {
        Some("草浪間隱現深深蹄印與磨角折斷的芒草叢，長風四目水鹿王與梅花鹿群奔踏而過的原野清氣猶在。")
    } else if zone_or_room.contains("古河巨木林") {
        Some("千年古樟樹幹三丈高處留有寸深的新鮮爪痕，古河樟林雲豹潛行於樹冠殘存的威壓令林間百鳥屏息。")
    } else if zone_or_room.contains("赤石坡") {
        Some("赤紅土石礦坡旁留有深邃掘洞與梅花狀貓科足印，赤石鯪鯉穿山甲與淺山錢紋石虎在此交錯出沒。")
    } else if zone_or_room.contains("落日岬") {
        Some("落日餘暉下的海岬高空盤旋著落日黑鳶王的魚尾剪影，礫灘上留有百年綠蠵龜爬行入海的寬闊軌跡。")
    } else if zone_or_room.contains("驚濤岩岸") {
        Some("怒濤拍擊的玄武岩尖端傳來破空銳嘯，驚濤遊隼王與裂浪灰海獺在狂風巨浪間各自稱雄。")
    } else if zone_or_room.contains("巨藻淺灣") {
        Some("隨潮搖曳的巨藻林間浮現百年綠蠵龜的翠綠背甲，淺灘上成群黑面琵鷺正迎著潮水展翅。")
    } else if zone_or_room.contains("黑石灣") {
        Some("玄武岩潮池中成雙成對的黑石海鱟披著青銅古甲緩步挪移，劍尾在黑石沙痕上劃出千年印記。")
    } else if zone_or_room.contains("翠冠巨木林") {
        Some("老樟樹幹三丈高處留有寸深的新鮮爪痕，千年樟雲豹王巡行殘存的淡淡樟木威壓令林間鳥雀不敢啼鳴。")
    } else if zone_or_room.contains("清泉溪谷") {
        Some("融雪清澈溪潭中銀光躍動，冰河孑遺清泉櫻花鉤吻鮭逆流躍上石階，溪畔卵石留有食蟹獴覓食足跡。")
    } else if zone_or_room.contains("北坡雲杉林") {
        Some("雲杉樹皮上留有高處抓痕與新鮮松脂，凍土雪泥上印著胸白月熊王如巨盆般的深印掌痕。")
    } else if zone_or_room.contains("西麓林道") {
        Some("晨霧瀰漫的闊葉林道間閃過一抹寶藍與朱紅光彩，西麓丹冠藍腹鷴與長尾山娘正掠過古道林隙。")
    } else if zone_or_room.contains("淺澤泥口") {
        Some("蘆葦泥澤水面浮現一對寬闊如彎月的巨角，淺澤重角水牛王靜浸泥水吐納，浮木上柴棺水龜安然曝背。")
    } else if zone_or_room.contains("白茫洲") {
        Some("腐葉與白瘴迷霧間留有碗口粗的三角菱紋滑痕，黑沼百步蛇王盤踞枯木，周遭蟲蛙盡數屏息。")
    } else if zone_or_room.contains("貼山深潭") {
        Some("碧綠如墨的深潭中央漩渦暗湧，寒潭千年花斑鱸鰻潛伏潭底石洞，水際冷石下隱現寒潭山椒魚蹤跡。")
    } else if zone_or_room.contains("青籠關") {
        Some("青籠關險峰高空迴盪著大冠鷲王清厲悠長的鳴聲，關道荊棘石縫間隱約得見錢紋石虎與剛鬃山豬的獸跡。")
    } else if zone_or_room.contains("西嶺礦谷") {
        Some("赤黑鐵礦岩壁下散落著被利爪刨開的碎礦石，裂石鯪鯉穿山甲王與青脊長鬃山羊在險峻礦脈間穿梭。")
    } else if zone_or_room.contains("東海峭壁") {
        Some("東海斷崖萬仞高空傳來撕裂長風的清厲長嘯，馭風角鷹王展開遮雲巨翼，斷崖長鬃山羊踏立絕壁。")
    } else if zone_or_room.contains("雲腰雪林") {
        Some("高山圓柏與冷杉積雪間留有修長三趾爪印，霧林黑長尾帝雉拖著雪白橫紋長尾自雪霧中優雅掠過。")
    } else if zone_or_room.contains("白塔雪巔") {
        Some("極寒冰峰石柱之上繚繞著純白清氣，雪巔霜冠大水鹿與極頂帝雉傲立雲海之巔，清氣貫通天地。")
    } else {
        None
    }
}

/// 地理環境吐納感知之天地來源與環境意象對應（供【觀境吐納】）。
/// 高山雪峰、流雲長風對應【氣源】，深潭、水澤、礁岸對應【水源】，其餘山野丘壑亦有微氣共鳴。
pub fn zone_ambient_word_source(zone_or_room: &str) -> Option<(crate::combat::wordplate::WordSource, &'static str)> {
    use crate::combat::wordplate::WordSource;

    if zone_or_room.contains("白塔雪巔")
        || zone_or_room.contains("雲腰雪林")
        || zone_or_room.contains("青籠關")
        || zone_or_room.contains("東海峭壁")
        || zone_or_room.contains("白茫洲")
        || zone_or_room.contains("風舶角")
    {
        Some((WordSource::Qi, "雲海罡風浩蕩，萬仞天風透體而過，天地至純清氣滌蕩心神。"))
    } else if zone_or_room.contains("貼山深潭")
        || zone_or_room.contains("清泉溪谷")
        || zone_or_room.contains("淺澤泥口")
        || zone_or_room.contains("驚濤岩岸")
        || zone_or_room.contains("巨藻淺灣")
        || zone_or_room.contains("落日岬")
        || zone_or_room.contains("黑石灣")
        || zone_or_room.contains("水尾寮")
        || zone_or_room.contains("烏石鼻")
    {
        Some((WordSource::Water, "碧波水精流轉，寒潭暗湧與潮聲拍岸共鳴，潤澤水靈沁潤心脾。"))
    } else if zone_or_room.contains("西嶺礦谷")
        || zone_or_room.contains("赤石坡")
        || zone_or_room.contains("紅土崁")
    {
        Some((WordSource::Mineral, "重岩地脈盤踞，金石沉著剛毅之質理厚重深邃，山骨厚土鎮守心神。"))
    } else if zone_or_room.contains("翠冠巨木林")
        || zone_or_room.contains("古河巨木林")
        || zone_or_room.contains("北坡雲杉林")
        || zone_or_room.contains("西麓林道")
        || zone_or_room.contains("長風草甸")
        || zone_or_room.contains("霜林口")
        || zone_or_room.contains("半嶺")
    {
        Some((WordSource::Plant, "草木葳蕤繁茂，萬木長青生發之氣綿綿不絕，蒼生靈機滋養百骸。"))
    } else {
        Some((WordSource::Qi, "天地吐納有節，微風徐徐流過四肢百骸，隱隱體察到周天游離清氣。"))
    }
}


/// 山海特產使用效果：
/// 回傳 `(category_tag, vit_gain, qi_gain, consume_item, effect_narrative)`。
pub fn shanhai_item_use_effect(item_id_or_name: &str) -> Option<(&'static str, i32, i32, bool, &'static str)> {
    match item_id_or_name {
        // 護身珍品／靈佩（不消耗道具）
        "wt_cat_0214" | "霧林帝雉長尾翎" => Some((
            "佩之・山海靈佩",
            0,
            10,
            false,
            "你將霧林帝雉長尾翎別於衣襟，周身湧起一陣清靈晨霧，心神寧靜如水。",
        )),
        "wt_cat_0216" | "丹冠藍腹鷴彩羽" => Some((
            "佩之・山海靈佩",
            5,
            5,
            false,
            "你撫摸著丹冠藍腹鷴的寶藍彩羽，清氣如朝霞瑞彩籠罩周身，祥瑞護持。",
        )),
        "wt_cat_0172" | "樟林雲豹軟毛皮" => Some((
            "佩之・山海靈佩",
            8,
            4,
            false,
            "你披上帶著古玉雲紋的雲豹軟毛皮，步履如風，身形輕靈如山林幽影。",
        )),
        "wt_cat_0171" | "山林石虎斑皮" => Some((
            "佩之・山海靈佩",
            6,
            3,
            false,
            "你將石虎斑皮繫於腰際，耳聰目明，黑夜中感知亦清晰入微。",
        )),
        "wt_cat_0188" | "鯪鯉穿山甲鱗片" => Some((
            "佩之・山海靈佩",
            10,
            0,
            false,
            "你手握金黑堅硬的鯪鯉麟片，筋骨經絡隱隱泛熱，肌理堅實如覆重甲。",
        )),
        "wt_cat_0176" | "泥甲巨鱷厚麟皮" => Some((
            "佩之・山海靈佩",
            12,
            0,
            false,
            "你撫過堅如頑石的巨鱷厚麟皮，如披重鎧，周身氣血穩固如山。",
        )),
        "wt_cat_0207" | "山豬王彎月獠牙" | "wt_cat_0211" | "海崖黑野豬牙" => Some((
            "佩之・山海靈佩",
            6,
            2,
            false,
            "你佩戴上如新月般剛硬的山豬尖牙，胸中湧起一股悍不畏死的山林勇氣。",
        )),
        "wt_cat_0183" | "白鼻心烏腳香皮" => Some((
            "佩之・山海靈佩",
            4,
            6,
            false,
            "你佩帶散發草果幽香的白鼻心皮，清香透鼻，令人百邪不侵、神清氣爽。",
        )),
        "wt_cat_0192" | "高山熊鷹翎羽" => Some((
            "佩之・山海靈佩",
            4,
            8,
            false,
            "你插上高山熊鷹的花翎羽毛，長風貫體，身輕體健仿若能乘風而起。",
        )),

        // 消耗性靈藥／珍餚（消耗道具）
        "wt_cat_0189" | "百步蛇乾蛻" => Some((
            "入藥・山海本草",
            10,
            15,
            true,
            "你將百步蛇乾蛻研粉調水服下，喉間微辛，一股清冽藥力順著經脈化開，深層瘴毒盡消。",
        )),
        "wt_cat_0205" | "胸白月熊厚熊掌" => Some((
            "食之・山海珍饈",
            25,
            10,
            true,
            "你細嚼慢嚥烹製的月熊厚掌，溫熱膏脂入口即化，全身暖流滾滾，四肢百骸力量沛然！",
        )),
        "wt_cat_0217" | "月熊凝白腹脂膏" => Some((
            "入藥・山海本草",
            20,
            8,
            true,
            "你將月熊凝白腹脂膏塗抹經絡或服食，肌膚溫潤生光，風寒暗傷頓時撫平。",
        )),
        "wt_cat_0064" | "深山牛樟芝" => Some((
            "入藥・山海本草",
            15,
            25,
            true,
            "你含化一片赤紅深山牛樟芝，苦盡甘來，精純元氣如瀑布般自氣海奔湧直上！",
        )),
        "wt_cat_0269" | "清溪活水黃金蜆" => Some((
            "食之・山海珍饈",
            12,
            12,
            true,
            "你烹食鮮美的清溪活水黃金蜆，湯鮮味美，五臟六腑備受滋潤，精氣神為之一振！",
        )),
        _ => None,
    }
}

/// 檢測玩家背包中是否佩戴山海護身靈佩珍品，提供戰鬥護持加成：
/// 回傳 `(talisman_name, vit_bonus, qi_bonus, bonus_narrative)`。
pub fn player_shanhai_talisman_bonus(inventory_json: &str) -> Option<(&'static str, i32, i32, &'static str)> {
    if inventory_json.contains("wt_cat_0176") {
        Some(("泥甲巨鱷厚麟皮", 12, 0, "泥甲護體，體魄固若金湯"))
    } else if inventory_json.contains("wt_cat_0188") {
        Some(("鯪鯉穿山甲鱗片", 10, 0, "烏金麟甲，氣血經絡通達無阻"))
    } else if inventory_json.contains("wt_cat_0172") {
        Some(("樟林雲豹軟毛皮", 8, 4, "古玉雲紋，身形敏捷化險為夷"))
    } else if inventory_json.contains("wt_cat_0171") {
        Some(("山林石虎斑皮", 6, 3, "石虎通靈，步法如電避實就虛"))
    } else if inventory_json.contains("wt_cat_0207") {
        Some(("山豬王彎月獠牙", 6, 2, "獠牙如月，雄渾剛毅撼動山川"))
    } else if inventory_json.contains("wt_cat_0211") {
        Some(("海崖黑野豬牙", 5, 2, "野豬尖牙，山林剛猛氣息加持"))
    } else if inventory_json.contains("wt_cat_0214") {
        Some(("霧林帝雉長尾翎", 0, 10, "帝雉白翎，清霧拂體元氣長存"))
    } else if inventory_json.contains("wt_cat_0216") {
        Some(("丹冠藍腹鷴彩羽", 5, 5, "藍鷴瑞羽，祥光普照體氣雙益"))
    } else if inventory_json.contains("wt_cat_0192") {
        Some(("高山熊鷹翎羽", 4, 8, "熊鷹花翎，神目如電長風護體"))
    } else {
        None
    }
}

/// 取得正方格附近格子的簡要視野（地圖渲染用）。
///
/// Tuple 欄位順序：`(x, y, kind, terrain, category, name, explored, walkable)`
/// - `kind`：Terrain enum 變體 snake_case（送前端切色/icon）
/// - `terrain`：中文標籤（向後相容）
/// - `category`：`"terrain"` / `"landmark"` / `"infra"`
#[allow(clippy::type_complexity)]
pub fn get_grid_cells_around(
    x: i32,
    y: i32,
    radius: i32,
) -> Vec<(i32, i32, String, String, String, String, bool, bool)> {
    let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
        return Vec::new();
    };
    let center = SquareCoord::new(x, y);
    let mut out = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            let c = SquareCoord::new(x + dx, y + dy);
            if center.chebyshev_distance(c) > radius {
                continue;
            }
            if let Some(cell) = grid.get(c) {
                out.push((
                    c.x,
                    c.y,
                    cell.terrain.kind_str().to_string(),
                    terrain_name_zh(cell.terrain).to_string(),
                    cell.terrain.category().to_string(),
                    cell.name.clone(),
                    cell.explored,
                    cell.terrain.walkable(),
                ));
            }
        }
    }
    out
}

/// 取得地形對應純符號（供前端或視圖查詢）
#[allow(dead_code)]
pub fn get_grid_cell_symbol(kind: &str) -> &'static str {
    crate::grid::Terrain::from_kind_str(kind)
        .map(|t| t.symbol())
        .unwrap_or("·")
}

#[must_use]
pub fn entity_kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Player => "player",
        EntityKind::Npc => "npc",
    }
}


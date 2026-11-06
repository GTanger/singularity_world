pub fn npc_sensory_appearance_line(entity_id: &str, display_title: &str) -> String {
    let combined = format!("{entity_id} {display_title}");
    if combined.contains("藏書閣") || combined.contains("館長") {
        return "身著深青色修身長外套，下擺帶著簡約水紋暗飾，鬚髮微霜，眼神清澈而深邃".to_string();
    }
    if combined.contains("房地契") || combined.contains("戶政") || combined.contains("審理戶籍") || combined.contains("lu_chengli") {
        return "穿著沉穩的墨色立領外套，束著精緻皮帶，氣度雍容，舉止端雅".to_string();
    }
    if combined.contains("總商行") {
        return "身穿寶藍色正裝外套，腰間掛著隨身懷錶與核算夾，指尖圓潤，氣度沉著圓融".to_string();
    }
    if combined.contains("第一鐵坊") || combined.contains("鍛造總匠") {
        return "身形如鐵塔般魁梧，赤臂外罩厚重牛皮防護圍兜，臂膀青筋虯結，滿身精金炭火氣息".to_string();
    }
    if combined.contains("教務長") || combined.contains("診所") {
        return "身著清爽月白色挺括短衫與長褲，袖口收緊，指間隱含艾草香氣，眉宇溫和謙潤".to_string();
    }
    if combined.contains("巧木") || combined.contains("木作") {
        return "滿頭短鬚微帶木屑，穿著耐磨牛筋布背心，腰間皮帶別滿各色雕花鑿與角尺".to_string();
    }
    if combined.contains("雲錦") || combined.contains("織造") {
        return "身著雅緻雪青色剪裁長裙，長髮俐落束起並插著銀頂量尺針，手指纖細白皙，氣質靈動婉約".to_string();
    }
    if combined.contains("大食堂") || combined.contains("主廚") || combined.contains("熬煮濃湯") || combined.contains("chen_wanfu") {
        return "身穿潔白雙排扣主廚服與厚布圍裙，笑口常開，身上帶著鮮香湯氣".to_string();
    }
    if combined.contains("溫水浴場") || combined.contains("浴場") {
        return "穿著寬鬆棉麻浴場休閒服，腳踏防滑木底便履，手持竹柄水勺，容光煥發".to_string();
    }
    if combined.contains("觀星台") || combined.contains("司天") {
        return "身穿繪有星圖暗紋的深藍觀測大衣，手持黃銅天體測角儀，眼神悠遠".to_string();
    }
    if combined.contains("鐘樓") || combined.contains("司鐘") {
        return "穿著灰布工裝衫，口袋插著精細小旋鑿與發條鑰匙，神情專注".to_string();
    }
    if combined.contains("聯運") || combined.contains("車馬") {
        return "面龐黝黑硬朗，穿著厚皮革背心與長靴，手握皮鞭，嗓音渾厚有力".to_string();
    }
    if combined.contains("蒙學") || combined.contains("講舍") {
        return "身著潔淨淺青短衫，面容溫和清瘦，懷抱啟蒙教材與一束鋼尖蘸水硬筆".to_string();
    }
    if combined.contains("商算") || combined.contains("會計") {
        return "身穿整潔洗白的藏青色短襖，右手拿著計數板，手指不時點算".to_string();
    }
    if combined.contains("書局") || combined.contains("印行") {
        return "身穿洗得乾淨挺括的淺墨灰工作長衫，神態溫和，指尖留有長年翻閱書頁與活字印墨痕跡".to_string();
    }
    if combined.contains("剪剃") || combined.contains("理容") {
        return "身著乾淨白色短袖工作衫，胸前革帶插滿各式修容細剪與剃刀，動作俐落如風".to_string();
    }
    if combined.contains("習武") || combined.contains("武館") {
        return "身穿黑色防風機能服，外罩暗紅半身防護背心，身姿挺拔如松，腰佩練習木刀".to_string();
    }
    if combined.contains("柑仔店") || combined.contains("雜貨") {
        return "穿著洗得泛黃的紅花布圍裙，兜裡裝滿五彩糖球，神情親切可親".to_string();
    }
    if combined.contains("東橋旅社") || combined.contains("茶屋") {
        return "身著淺茶色休閒外套，手握隨身懷錶，笑容溫厚周到，語調從容不迫".to_string();
    }
    if combined.contains("修傘") || combined.contains("補靴") {
        return "鼻樑架著黃銅圓框助視鏡，圍著滿是蠟痕與針孔的厚牛皮圍兜，指節粗大粗糙，身邊擺滿硬木鞋楦".to_string();
    }
    if combined.contains("公議堂") {
        return "身著端整莊重的深青色當值夾袍，隨身佩帶印章盒與公文夾，神色嚴謹".to_string();
    }
    if combined.contains("roast_beef") || combined.contains("炭烤") || combined.contains("燒肉") {
        return "身穿耐磨粗布短衫，腰繫沾滿炭灰的厚帆布防油圍裙，手持長柄鐵夾，滿身炭火烤肉香".to_string();
    }
    if combined.contains("noodle_soup") || combined.contains("切仔麵") {
        return "身著花布短衫外罩白棉布圍裙，挽起衣袖，右手握著長柄竹編笊籬，周身蒸氣繚繞".to_string();
    }
    if combined.contains("general_goods") || combined.contains("雜貨") || combined.contains("柑仔店") {
        return "頭戴整潔便帽，腰繫多袋帆布隨身兜，笑容熱情，手邊擺滿各式日常雜貨小件".to_string();
    }
    if combined.contains("herb_clinic") || combined.contains("百草") {
        return "身著俐落的杏黃色工作裙，腰間繫著分格布囊藥包與百草溫灸盒，神情溫柔專注".to_string();
    }
    if combined.contains("fresh_produce") || combined.contains("果菜") {
        return "頭戴寬簷遮陽竹笠，捲起深色棉布褲管，身前放著盛滿清水的竹筒噴水壺，滿面泥土清香與憨厚笑容".to_string();
    }
    if combined.contains("hot_congee") || combined.contains("熱粥") {
        return "面容慈祥，穿著乾淨的藍布對襟衫，手握一把扁平長柄木杓，神情篤定從容，身前陶鍋熱氣升騰".to_string();
    }
    if combined.contains("dried_seafood") || combined.contains("乾貨") {
        return "穿著防水牛筋皮背心與高筒膠靴，腰間繫著剖魚小皮鞘，身上帶著新鮮河水與海味鹹香".to_string();
    }
    if combined.contains("iron_repair") || combined.contains("鐵器") {
        return "身著深灰耐磨工作衫，右臂套著厚皮防燙手甲，目光如炬，專注沉靜".to_string();
    }
    if combined.contains("wood_craft") || combined.contains("木器") {
        return "穿著灰布工裝衫，肩扛刨平木料，腰間插著木工鋸與角尺，滿面憨厚笑容".to_string();
    }
    let slot = wenyao_household_slot_for(entity_id);
    slot.attire_line.to_string()
}

pub fn npc_brief_appearance_tag(entity_id: &str, display_title: &str) -> String {
    let combined = format!("{entity_id} {display_title}");
    if combined.contains("藏書閣") || combined.contains("館長") {
        return "身著深青長外套".to_string();
    }
    if combined.contains("房地契") || combined.contains("戶政") {
        return "身穿墨色立領外套".to_string();
    }
    if combined.contains("總商行") {
        return "身穿寶藍正裝外套".to_string();
    }
    if combined.contains("第一鐵坊") || combined.contains("鍛造總匠") {
        return "繫著厚重牛皮圍兜".to_string();
    }
    if combined.contains("教務長") || combined.contains("診所") {
        return "身著挺括白短衫".to_string();
    }
    if combined.contains("巧木") || combined.contains("木作") {
        return "腰別鑿尺工具帶".to_string();
    }
    if combined.contains("雲錦") || combined.contains("織造") {
        return "身穿雪青剪裁長裙".to_string();
    }
    if combined.contains("大食堂") || combined.contains("主廚") {
        return "圍著白棉布大圍裙".to_string();
    }
    if combined.contains("溫水浴場") || combined.contains("浴場") {
        return "穿著浴場休閒服".to_string();
    }
    if combined.contains("觀星台") || combined.contains("司天") {
        return "身穿深藍觀測大衣".to_string();
    }
    if combined.contains("鐘樓") || combined.contains("司鐘") {
        return "口袋插著發條鑰匙".to_string();
    }
    if combined.contains("聯運") || combined.contains("車馬") {
        return "穿著厚皮革背心".to_string();
    }
    if combined.contains("蒙學") || combined.contains("講舍") {
        return "身著淺青短衫".to_string();
    }
    if combined.contains("商算") || combined.contains("會計") {
        return "手持計數板".to_string();
    }
    if combined.contains("書局") || combined.contains("印行") {
        return "身穿挺括淺墨灰衫".to_string();
    }
    if combined.contains("剪剃") || combined.contains("理容") {
        return "佩掛各式修容細剪".to_string();
    }
    if combined.contains("習武") || combined.contains("武館") {
        return "身穿黑色機能服".to_string();
    }
    if combined.contains("柑仔店") || combined.contains("雜貨") {
        return "圍著紅花布圍裙".to_string();
    }
    if combined.contains("東橋旅社") || combined.contains("茶屋") {
        return "手握隨身懷錶".to_string();
    }
    if combined.contains("修傘") || combined.contains("補靴") {
        return "鼻架圓框水晶明目鏡".to_string();
    }
    if combined.contains("公議堂") {
        return "佩帶印章公文夾".to_string();
    }
    if combined.contains("roast_beef") || combined.contains("炭烤") || combined.contains("燒肉") {
        return "圍著防油帆布圍裙".to_string();
    }
    if combined.contains("noodle_soup") || combined.contains("切仔麵") {
        return "繫著素色布巾圍裙".to_string();
    }
    if combined.contains("general_goods") || combined.contains("雜貨") || combined.contains("柑仔店") {
        return "腰繫多袋隨身兜".to_string();
    }
    if combined.contains("herb_clinic") || combined.contains("百草") {
        return "身穿杏黃工作裙".to_string();
    }
    if combined.contains("fresh_produce") || combined.contains("果菜") {
        return "頭戴寬沿遮陽竹笠".to_string();
    }
    if combined.contains("hot_congee") || combined.contains("熱粥") {
        return "手握長柄木杓".to_string();
    }
    if combined.contains("dried_seafood") || combined.contains("乾貨") {
        return "身穿防潮高筒膠靴".to_string();
    }
    if combined.contains("iron_repair") || combined.contains("鐵器") {
        return "戴著防燙手甲".to_string();
    }
    if combined.contains("wood_craft") || combined.contains("木器") {
        return "肩扛刨平木料".to_string();
    }

    let slot = wenyao_household_slot_for(entity_id);
    let role_nouns = [
        "手", "師", "匠", "夫", "婦", "頭", "客", "商", "事", "官", "丁", "販", "生", "徒", "民", "戶", "眷", "姆", "工", "力",
    ];
    let raw_tag = if slot.brief_tag.is_empty()
        || slot.brief_tag.contains('・')
        || role_nouns.iter().any(|&n| slot.brief_tag.ends_with(n))
    {
        slot.attire_line.split('，').next().unwrap_or("身著常服").trim().to_string()
    } else {
        slot.brief_tag.trim().to_string()
    };
    let frag = raw_tag.trim();
    if frag.is_empty() {
        return String::new();
    }
    if frag.starts_with("腕纏")
        || frag.starts_with("手執")
        || frag.starts_with("雙手")
        || frag.starts_with("袖口")
        || frag.starts_with("肩披")
        || frag.starts_with("背負")
        || frag.starts_with("足蹬")
        || frag.starts_with("腳踏")
        || frag.starts_with("神色")
        || frag.starts_with("氣息")
        || frag.starts_with("目光")
    {
        return frag.to_string();
    }
    let has_verb_or_part = [
        "身著", "身穿", "穿著", "身帶", "身染", "身泛", "身飄", "身上", "著", "戴", "頭", "腰", "肩",
        "手", "腕", "圍", "繫", "佩", "口袋", "鼻", "背", "挽", "拎", "披", "踏", "腳", "足", "袖",
        "領", "襟", "額", "髮", "鬢", "面", "懷", "胸", "雙", "合社", "入山", "古道", "別著", "前兜",
        "外罩", "夾著", "套著", "套防", "懸", "抱著", "拇指", "指", "挑著", "掛著", "插", "攜",
        "散發", "斜", "沾", "滿身", "縈繞", "耳", "衣", "負", "頸", "髻",
    ]
    .iter()
    .any(|p| frag.starts_with(p));
    if has_verb_or_part {
        return frag.to_string();
    }
    if frag.starts_with("穿") {
        return format!("身{}", frag);
    }
    if frag.starts_with("帶") {
        return format!("身上{}", frag);
    }
    format!("身著{frag}")
}

pub fn normalize_archaic_clothing(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut res = s.to_string();
    res = res.replace("童生布衫", "學子青衫");
    res = res.replace("十三檔烏木竹骨算盤", "烏木便攜計數板");
    res = res.replace("銀票", "通兌匯票");
    res = res.replace("粗布短褂", "棉麻短衫");
    res = res.replace("粗布長褲", "耐磨長褲");
    res = res.replace("草鞋", "輕便布鞋");
    res = res.replace("正色玄端禮服", "深色正式禮服");
    res = res.replace("素布衣裙", "休閒長裙");
    res = res.replace("粗布短褐", "耐磨工作衫");
    res = res.replace("布裳", "日常上衣");
    res = res.replace("短褐", "耐磨工作衫");
    res = res.replace("吏目直裰", "深青色當值夾袍");
    res = res.replace("直裰", "當值夾袍");
    res = res.replace("深藍道袍", "深藍觀測大衣");
    res = res.replace("道袍", "觀測大衣");
    res = res.replace("交領羅裙", "剪裁長裙");
    res = res.replace("羅裙", "剪裁長裙");
    res = res.replace("窄袖布裙", "俐落工作裙");
    res = res.replace("短坎肩", "皮革背心");
    res = res.replace("素青直身衣", "淺青色短衫");
    res = res.replace("藏青夾袍", "藏青色短襖");
    res = res.replace("廣袖儒袍", "修身長外套");
    res = res.replace("細絹長袍", "休閒外套");
    res = res.replace("月白長袍", "挺括短衫");
    res = res.replace("墨色長衫", "墨色立領外套");
    res = res.replace("寶藍錦袍", "寶藍色正裝外套");
    res = res.replace("桐木屐", "防滑木底便履");
    res = res.replace("烏木算盤", "便攜計數板");
    res = res.replace("金絲算盤", "隨身懷錶與核算夾");
    res = res.replace("算盤", "計數板");
    res = res.replace("竹骨摺扇", "隨身懷錶");
    res = res.replace("摺扇", "隨身懷錶");
    res = res.replace("硃筆", "鋼尖蘸水硬筆");
    res
}

pub fn normalize_archaic_title(title: &str) -> String {
    if title.is_empty() {
        return String::new();
    }
    let mut res = title.to_string();
    const FULL_ROLES: &[(&str, &str)] = &[
        ("三水船塢腳夫", "三水船塢搬運員"),
        ("印行書局刻字匠", "西門車隊貨運員"),
        ("百工鐵坊鍛徒", "第一鍛造工坊技師"),
        ("文曜算學學子", "文曜學院學生"),
        ("文曜蒙學堂助教", "文曜蒙學堂助教"),
        ("文曜小學堂助教", "文曜蒙學堂助教"),
        ("溫氏藥居採藥人", "溫氏藥居採藥人"),
        ("三水商行押運夥計", "三水商行押運夥計"),
        ("東橋茶寮茶客", "東橋茶屋常客"),
        ("海味乾貨行商", "海產乾貨商"),
        ("公議堂抄錄書生", "公議堂抄錄書生"),
        ("車馬行趕車夫", "分家巷木工技師"),
        ("藏書閣校書生", "藏書閣校書生"),
        ("西門貨棧搬運工", "百工街鐵工"),
        ("北門巡街衛士", "北門巡守員"),
        ("南門紡織女工", "三水食堂廚務員"),
        ("三水渡口船夫", "講堂街治學士"),
        ("竹編器物手藝人", "織造坊裁縫師"),
        ("石橋挑擔貨郎", "石橋攤商"),
        ("觀星台測候學徒", "南街藥材商"),
        ("鐘樓守時雜役", "鐘樓報時員"),
        ("泉香浴道夥計", "泉香浴場侍者"),
        ("青石巷木工師傅", "西門修配工"),
        ("算學館記帳員", "三水川船務員"),
        ("遠道行旅散客", "外地旅客"),
    ];
    for &(old_r, new_r) in FULL_ROLES {
        if res.contains(old_r) {
            res = res.replace(old_r, new_r);
        }
    }
    const FRAGMENTS: &[(&str, &str)] = &[
        ("公議堂保甲司", "公議堂治安巡防處"),
        ("民務通事", "民務通事"),
        ("小學堂教師", "蒙學先生"),
        ("蒙學教諭", "蒙學先生"),
        ("教書先生", "蒙學先生"),
        ("抄錄書生", "抄錄書生"),
        ("校書生", "校書生"),
        ("挑擔貨郎", "攤商"),
        ("茶寮茶客", "茶屋常客"),
        ("講堂街抱卷學子", "講堂街抱書學生"),
        ("文曜曆算學子", "文曜曆算生"),
        ("算學學子", "學院學生"),
        ("學子", "學生"),
        ("文曜小學堂", "文曜蒙學堂"),
        ("文曜蒙學堂", "文曜蒙學堂"),
        ("蒙學堂助教", "蒙學助教"),
        ("小學堂助教", "蒙學助教"),
        ("小學堂", "蒙學堂"),
        ("蒙學堂", "蒙學堂"),
        ("跑堂夥計", "外場侍者"),
        ("跑堂", "外場侍者"),
        ("巡街衛士", "巡守員"),
        ("守時雜役", "報時員"),
        ("行旅散客", "外地旅客"),
        ("茶博士", "茶屋侍者"),
        ("鏢師", "隨隊護衛"),
        ("郎中", "駐店醫師"),
        ("腳夫", "搬運員"),
        ("貨郎", "攤商"),
        ("\u{638c}櫃", "掌事"),
        ("夥計", "店員"),
    ];
    for &(old_f, new_f) in FRAGMENTS {
        if res.contains(old_f) {
            res = res.replace(old_f, new_f);
        }
    }
    res
}

pub fn find_bfs_next_step(
    grid: &crate::grid::SquareGrid,
    start: (i32, i32),
    raw_goal: (i32, i32),
) -> Option<(i32, i32)> {
    if start == raw_goal {
        return None;
    }

    // 六座設牆城鎮（文曜、風舶角、風舶嶼、大圳頭、半嶺鎮、南哨）城門中繼路由：
    // 當起點在城牆內而終點在城牆外（或反之且距離較遠）時，自動先導引至最佳城門外口，徹底杜絕貼城牆死角（如 (16,16)、(-251,169)）來回卡死。
    const WALLED_CITIES: &[(i32, i32, i32, i32, [(i32, i32); 4])] = &[
        (
            -17,
            17,
            -17,
            17,
            [(0, 18), (0, -18), (18, 0), (-18, 0)],
        ), // 文曜
        (
            -270,
            -250,
            150,
            170,
            [(-260, 171), (-260, 149), (-249, 160), (-271, 160)],
        ), // 風舶角
        (
            -337,
            -323,
            153,
            167,
            [(-322, 160), (-338, 160), (-322, 160), (-338, 160)],
        ), // 風舶嶼
        (
            -187,
            -173,
            -147,
            -133,
            [(-180, -132), (-180, -148), (-172, -139), (-188, -139)],
        ), // 大圳頭
        (
            153,
            167,
            203,
            217,
            [(160, 218), (160, 202), (152, 210), (160, 218)],
        ), // 半嶺鎮
        (
            -7,
            7,
            -227,
            -213,
            [(0, -212), (0, -228), (8, -220), (-8, -220)],
        ), // 南哨
    ];
    let mut goal = raw_goal;
    for &(min_x, max_x, min_y, max_y, ref gates) in WALLED_CITIES {
        let start_in = (min_x..=max_x).contains(&start.0) && (min_y..=max_y).contains(&start.1);
        let goal_in =
            (min_x..=max_x).contains(&raw_goal.0) && (min_y..=max_y).contains(&raw_goal.1);
        if start_in && !goal_in {
            if let Some(&best_gate) = gates.iter().min_by_key(|&&(gx, gy)| {
                let d1 = (gx - start.0).abs() + (gy - start.1).abs();
                let d2 = (raw_goal.0 - gx).abs() + (raw_goal.1 - gy).abs();
                d1 + d2
            }) {
                if start != best_gate {
                    goal = best_gate;
                }
            }
            break;
        } else if !start_in
            && goal_in
            && ((raw_goal.0 - start.0).abs() + (raw_goal.1 - start.1).abs() > 28)
        {
            if let Some(&best_gate) = gates.iter().min_by_key(|&&(gx, gy)| {
                let d1 = (gx - start.0).abs() + (gy - start.1).abs();
                let d2 = (raw_goal.0 - gx).abs() + (raw_goal.1 - gy).abs();
                d1 + d2
            }) {
                if (start.0 - best_gate.0).abs() + (start.1 - best_gate.1).abs() > 3 {
                    goal = best_gate;
                }
            }
            break;
        }
    }

    let get_ordered_offsets = |pos: (i32, i32)| -> [(i32, i32); 4] {
        let mut ordered = [
            (0, 1),
            (1, 0),
            (0, -1),
            (-1, 0),
        ];
        ordered.sort_by_key(|&(dx, dy)| {
            let nx = pos.0 + dx;
            let ny = pos.1 + dy;
            let ddx = (goal.0 - nx) as i64;
            let ddy = (goal.1 - ny) as i64;
            let euclid_d2 = ddx * ddx + ddy * ddy;
            let is_road_or_bridge = grid
                .get(crate::grid::SquareCoord::new(nx, ny))
                .map(|c| {
                    c.terrain == crate::grid::Terrain::Road
                        || c.terrain == crate::grid::Terrain::Bridge
                })
                .unwrap_or(false);
            (if is_road_or_bridge { 0u8 } else { 1u8 }, euclid_d2)
        });
        ordered
    };

    let can_step = |fx: i32, fy: i32, tx: i32, ty: i32| -> bool {
        if !can_npc_step_between(grid, fx, fy, tx, ty) {
            return false;
        }
        if (tx, ty) == goal {
            return true;
        }
        let Some(tc) = grid.get(crate::grid::SquareCoord::new(tx, ty)) else {
            return false;
        };
        if tc.terrain == crate::grid::Terrain::House && (tx, ty) != goal {
            return false;
        }
        is_road_or_open_cell(tc)
    };

    let mut immediate_candidates = Vec::new();
    for &(dx, dy) in &get_ordered_offsets(start) {
        let nx = start.0 + dx;
        let ny = start.1 + dy;
        if can_step(start.0, start.1, nx, ny) {
            if (nx, ny) == goal {
                return Some((nx, ny));
            }
            immediate_candidates.push((nx, ny));
        }
    }


    let mut queue = std::collections::VecDeque::new();
    let mut visited = std::collections::HashSet::new();
    visited.insert(start);

    for &cand in &immediate_candidates {
        visited.insert(cand);
        queue.push_back((cand, cand));
    }

    let manhattan_total = (goal.0 - start.0).abs() + (goal.1 - start.1).abs();
    if manhattan_total <= 42 {
        let mut max_steps = 1800;
        while let Some((curr, first_step)) = queue.pop_front() {
            max_steps -= 1;
            if max_steps == 0 {
                break;
            }

            for &(dx, dy) in &get_ordered_offsets(curr) {
                let nx = curr.0 + dx;
                let ny = curr.1 + dy;
                if (nx, ny) == goal {
                    if can_step(curr.0, curr.1, nx, ny) {
                        return Some(first_step);
                    }
                }
                if !visited.contains(&(nx, ny)) && can_step(curr.0, curr.1, nx, ny) {
                    visited.insert((nx, ny));
                    queue.push_back(((nx, ny), first_step));
                }
            }
        }

        // 第二階段 BFS：若目標為外圍深巷民宅或山林營地，放寬為所有非水域可通行格
        let can_step_relaxed = |fx: i32, fy: i32, tx: i32, ty: i32| -> bool {
            can_npc_step_between(grid, fx, fy, tx, ty)
        };
        let mut q2 = std::collections::VecDeque::new();
        let mut vis2 = std::collections::HashSet::new();
        vis2.insert(start);
        for &(dx, dy) in &get_ordered_offsets(start) {
            let nx = start.0 + dx;
            let ny = start.1 + dy;
            if can_step_relaxed(start.0, start.1, nx, ny) {
                if (nx, ny) == goal {
                    return Some((nx, ny));
                }
                vis2.insert((nx, ny));
                q2.push_back(((nx, ny), (nx, ny)));
            }
        }
        let mut steps2 = 1800;
        while let Some((curr, first_step)) = q2.pop_front() {
            steps2 -= 1;
            if steps2 == 0 {
                break;
            }
            for &(dx, dy) in &get_ordered_offsets(curr) {
                let nx = curr.0 + dx;
                let ny = curr.1 + dy;
                if (nx, ny) == goal && can_step_relaxed(curr.0, curr.1, nx, ny) {
                    return Some(first_step);
                }
                if !vis2.contains(&(nx, ny)) && can_step_relaxed(curr.0, curr.1, nx, ny) {
                    vis2.insert((nx, ny));
                    q2.push_back(((nx, ny), first_step));
                }
            }
        }
    }

    // 第三階段：跨區長途尋路（城鎮 <-> 19 處獵場／鄰鎮古道）
    // 以深度 6 局部前瞻 BFS 尋求最貼近目標且優先踏上驛道／橋樑的可通行推進方向，徹底杜絕遠距目標超過單次 BFS 半徑而原地凍結。
    let can_step_long = |fx: i32, fy: i32, tx: i32, ty: i32| -> bool {
        can_npc_step_between(grid, fx, fy, tx, ty)
    };
    let mut q3 = std::collections::VecDeque::new();
    let mut vis3 = std::collections::HashSet::new();
    vis3.insert(start);
    for &(dx, dy) in &get_ordered_offsets(start) {
        let nx = start.0 + dx;
        let ny = start.1 + dy;
        if can_step_long(start.0, start.1, nx, ny) {
            if (nx, ny) == goal {
                return Some((nx, ny));
            }
            vis3.insert((nx, ny));
            q3.push_back(((nx, ny), (nx, ny), 1u8));
        }
    }
    let mut best_frontier_step: Option<(i32, i32)> = None;
    let mut best_frontier_score = i32::MAX;
    let mut best_fallback_step: Option<(i32, i32)> = None;
    let mut best_fallback_score = i32::MAX;
    const TARGET_LOOKAHEAD_DEPTH: u8 = 12;
    let mut steps3 = 900;
    while let Some((curr, first_step, depth)) = q3.pop_front() {
        steps3 -= 1;
        let ddx = (goal.0 - curr.0) as i64;
        let ddy = (goal.1 - curr.1) as i64;
        let euclid_d2 = (ddx * ddx + ddy * ddy) as i32;
        let road_bonus = grid
            .get(crate::grid::SquareCoord::new(curr.0, curr.1))
            .map(|c| {
                if c.terrain == crate::grid::Terrain::Road
                    || c.terrain == crate::grid::Terrain::Bridge
                {
                    90
                } else {
                    0
                }
            })
            .unwrap_or(0);
        let wobble =
            ((curr.0.wrapping_mul(73) ^ curr.1.wrapping_mul(199)).unsigned_abs() % 5) as i32;
        let score = euclid_d2 - road_bonus + wobble;
        if score < best_fallback_score {
            best_fallback_score = score;
            best_fallback_step = Some(first_step);
        }
        if (depth >= TARGET_LOOKAHEAD_DEPTH || curr == goal) && score < best_frontier_score {
            best_frontier_score = score;
            best_frontier_step = Some(first_step);
        }
        if steps3 == 0 || depth >= TARGET_LOOKAHEAD_DEPTH {
            continue;
        }
        for &(dx, dy) in &get_ordered_offsets(curr) {
            let nx = curr.0 + dx;
            let ny = curr.1 + dy;
            if !vis3.contains(&(nx, ny)) && can_step_long(curr.0, curr.1, nx, ny) {
                vis3.insert((nx, ny));
                q3.push_back(((nx, ny), first_step, depth + 1));
            }
        }
    }
    best_frontier_step.or(best_fallback_step)
}

#[allow(dead_code)]
pub fn zone_bounds(zone: &str, h: u64) -> (i32, i32, i32, i32) {
    match zone {
        "wt-c-wenyao" => (0, 0, 3, 3),
        "wt-w-windship-cape" => (-20, 16, 3, 3),
        "wt-w-windship-island" => (-30, 22, 3, 3),
        "wt-c-southwest-plain" => (-18, -14, 3, 3),
        "wt-c-sacred-tower" => (10, 18, 3, 3),
        "wt-s-south-sentinel" => (0, -22, 3, 3),
        "wt-e-green-ridge" => {
            if (h & 1) == 0 {
                (14, 6, 2, 2)
            } else {
                (16, -6, 2, 2)
            }
        }
        "wt-s-black-marsh" => {
            if (h & 1) == 0 {
                (-8, -26, 2, 2)
            } else {
                (12, -24, 2, 2)
            }
        }
        "wt-s-qinglong-pass" => (0, -31, 2, 2),
        "wt-n-sacred-mountain" => (2, 26, 2, 2),
        _ => (0, 0, 3, 3),
    }
}

pub fn is_road_or_open_cell(cell: &crate::grid::GridCell) -> bool {
    if matches!(
        cell.terrain,
        crate::grid::Terrain::House
            | crate::grid::Terrain::Academy
            | crate::grid::Terrain::Observatory
            | crate::grid::Terrain::Library
            | crate::grid::Terrain::GeneralStore
            | crate::grid::Terrain::Blacksmith
            | crate::grid::Terrain::Inn
            | crate::grid::Terrain::Workshop
            | crate::grid::Terrain::Clinic
            | crate::grid::Terrain::TownHall
            | crate::grid::Terrain::Courthouse
            | crate::grid::Terrain::Caravanserai
            | crate::grid::Terrain::Market
            | crate::grid::Terrain::Bank
            | crate::grid::Terrain::GuardPost
            | crate::grid::Terrain::Tavern
            | crate::grid::Terrain::Bathhouse
            | crate::grid::Terrain::Dock
            | crate::grid::Terrain::Farmhouse
            | crate::grid::Terrain::GuildHall
            | crate::grid::Terrain::Temple
            | crate::grid::Terrain::Barracks
            | crate::grid::Terrain::Warehouse
            | crate::grid::Terrain::Granary
            | crate::grid::Terrain::Jail
            | crate::grid::Terrain::Mint
            | crate::grid::Terrain::Stables
            | crate::grid::Terrain::Theater
            | crate::grid::Terrain::Arena
            | crate::grid::Terrain::Alchemist
            | crate::grid::Terrain::MageTower
            | crate::grid::Terrain::Embassy
            | crate::grid::Terrain::PrisonYard
            | crate::grid::Terrain::HolyTower
    ) {
        return false;
    }

    let leaf = cell.name.rsplit('・').next().unwrap_or(&cell.name);
    const BUILDING_LEAF_KEYWORDS: &[&str] = &[
        "宿舍", "書齋", "小齋", "講舍", "書局", "印行館", "剪剃鋪", "事務所", "工坊", "旅社",
        "茶屋", "傢俱坊", "服飾坊", "診所", "藥房", "試招院", "柑仔店", "公議堂", "總站", "貨運行",
        "宅", "居", "舍", "院", "閣", "堂", "樓", "館", "所", "寮",
    ];
    if BUILDING_LEAF_KEYWORDS.iter().any(|k| leaf.contains(k)) {
        return false;
    }

    if matches!(
        cell.terrain,
        crate::grid::Terrain::Road
            | crate::grid::Terrain::Bridge
            | crate::grid::Terrain::Plain
            | crate::grid::Terrain::Grassland
    ) {
        return true;
    }

    leaf.contains("街")
        || leaf.contains("路")
        || leaf.contains("道")
        || leaf.contains("巷")
        || leaf.contains("橋")
        || leaf.contains("廣場")
        || leaf.contains("埕")
        || leaf.contains("鎮口")
        || leaf.contains("城門")
        || leaf.contains("甕門")
        || leaf.contains("空地")
        || leaf.contains("野地")
}

pub fn is_building_cell_name_or_terrain(cell: &crate::grid::GridCell) -> bool {
    if is_road_or_open_cell(cell) {
        return false;
    }
    if cell.terrain.category() == "landmark" {
        return true;
    }
    let sym = cell.terrain.display_char();
    if matches!(sym, "⌂" | "𖠿" | "∆" | "⚑" | "$" | "⚒︎" | "+") {
        return true;
    }
    let name = &cell.name;
    name.contains("店")
        || name.contains("鋪")
        || name.contains("館")
        || name.contains("樓")
        || name.contains("舍")
        || name.contains("堂")
        || name.contains("房")
        || name.contains("宅")
        || name.contains("營")
        || name.contains("閣")
        || name.contains("院")
        || name.contains("坊")
        || name.contains("庫")
        || name.contains("所")
        || name.contains("居")
        || name.contains("局")
        || name.contains("莊")
        || name.contains("壇")
        || name.contains("台")
}

pub fn can_npc_step_between(
    grid: &crate::grid::SquareGrid,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
) -> bool {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    if dx.abs() + dy.abs() != 1 {
        return false;
    }
    let to_coord = crate::grid::SquareCoord::new(to_x, to_y);
    let Some(to_cell) = grid.get(to_coord) else {
        return false;
    };
    let to_in_settlement = grid.has_explicit_cell(to_coord)
        && crate::game::is_inside_settlement_urban_core(to_x, to_y);
    // 野外山脊／冰峰（Terrain::Mountain）允許獵團與百工翻越通行；深海、城牆與柵欄嚴格禁止通行
    if matches!(
        to_cell.terrain,
        crate::grid::Terrain::WaterDeep
            | crate::grid::Terrain::Wall
            | crate::grid::Terrain::Fence
    ) || (to_in_settlement && !to_cell.terrain.walkable())
    {
        return false;
    }
    let from_coord = crate::grid::SquareCoord::new(from_x, from_y);
    let Some(from_cell) = grid.get(from_coord) else {
        return false;
    };
    let from_in_settlement = grid.has_explicit_cell(from_coord)
        && crate::game::is_inside_settlement_urban_core(from_x, from_y);

    let from_is_bldg = is_building_cell_name_or_terrain(from_cell);
    let to_is_bldg = is_building_cell_name_or_terrain(to_cell);

    // 建築隔離規則：若起點與終點皆為建築（非道路／橋樑／廣場／空地），禁止直接穿牆步進
    // 但若任一建築為內院盲室（缺乏對外的直接非建築步進出口），則允許步進通行
    if from_is_bldg && to_is_bldg {
        if from_cell.name.is_empty() || from_cell.name != to_cell.name {
            let has_exit = |cx: i32, cy: i32| -> bool {
                for &(ndx, ndy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
                    let nb_coord = crate::grid::SquareCoord::new(cx + ndx, cy + ndy);
                    if let Some(nb_cell) = grid.get(nb_coord) {
                        if nb_cell.terrain.walkable()
                            && !is_building_cell_name_or_terrain(nb_cell)
                            && nb_cell.terrain != crate::grid::Terrain::Water
                        {
                            return true;
                        }
                    }
                }
                false
            };
            if has_exit(from_x, from_y) && has_exit(to_x, to_y) {
                return false;
            }
        }
    }

    // 城鎮水域隔離（僅限聚落核心區內之護城河／水渠，野外山溪與淺澤允許涉水渡過）
    if (from_cell.terrain == crate::grid::Terrain::Water
        || to_cell.terrain == crate::grid::Terrain::Water)
        && (from_in_settlement || to_in_settlement || from_is_bldg || to_is_bldg)
    {
        return false;
    }

    true
}


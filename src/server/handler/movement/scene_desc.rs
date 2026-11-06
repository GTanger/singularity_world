/// 將房間內未在描述中提及的核心固定景物（建築、陳設、機巧、工坊器具等）自然編織入場景描述末尾。
pub(crate) fn weave_scene_fixtures_into_desc(
    desc: &str,
    room_name: &str,
    objects: &[crate::server::protocol::ViewObject],
) -> String {
    if desc.trim().is_empty() || objects.is_empty() {
        return desc.to_string();
    }

    let mut unmentioned = Vec::new();
    for obj in objects {
        let name = obj.name.trim();
        if name.is_empty() || name.contains('×') {
            continue;
        }

        let is_immovable_attr = obj.attribute_tags.iter().any(|t| {
            t == "immovable" || t == "public_fixture" || t == "venue_counter"
        });

        let category_matches = matches!(
            obj.category_tag.as_str(),
            "architecture" | "furnishing" | "craft_tool" | "mechanism" | "vessel_container"
        );

        const FIXTURE_SOCKETS: &[&str] = &[
            "Operate", "Climb", "Cross", "Warm", "Watch", "Pray", "Sit",
            "DrawWater", "Wash", "Forge", "MillGrain", "DryGoods", "Smelt",
        ];

        let sockets_match = obj.actions.iter().any(|act| {
            FIXTURE_SOCKETS.iter().any(|&s| act.eq_ignore_ascii_case(s))
        });

        let is_fixture = category_matches || is_immovable_attr || sockets_match;
        if !is_fixture {
            continue;
        }

        if !is_immovable_attr {
            let has_look = obj.actions.iter().any(|a| a.eq_ignore_ascii_case("Look"));
            let has_move = obj.actions.iter().any(|a| a.eq_ignore_ascii_case("Move"));
            let only_look_move = has_look && has_move && obj.actions.iter().all(|a| {
                a.eq_ignore_ascii_case("Look") || a.eq_ignore_ascii_case("Move")
            });
            if only_look_move {
                continue;
            }
        }

        let char_count = name.chars().count();
        let mentioned = desc.contains(name) || (char_count >= 4 && {
            let suffix: String = name.chars().skip(char_count - 2).collect();
            desc.contains(&suffix)
        });

        if mentioned {
            continue;
        }

        if !unmentioned.iter().any(|n| n == name) {
            unmentioned.push(name.to_string());
            if unmentioned.len() == 4 {
                break;
            }
        }
    }

    if unmentioned.is_empty() {
        return desc.to_string();
    }

    let joined = match unmentioned.len() {
        1 => unmentioned[0].clone(),
        2 => format!("{}與{}", unmentioned[0], unmentioned[1]),
        _ => {
            let (last, prefix) = unmentioned.split_last().unwrap();
            format!("{}與{}", prefix.join("、"), last)
        }
    };

    let is_indoor = room_name.contains('室')
        || room_name.contains('堂')
        || room_name.contains('館')
        || room_name.contains('閣')
        || room_name.contains('齋')
        || room_name.contains('行')
        || room_name.contains('鋪')
        || room_name.contains('坊')
        || room_name.contains('家')
        || room_name.contains('宅')
        || room_name.contains('居');

    let is_waterfront = room_name.contains('橋')
        || room_name.contains('渡')
        || room_name.contains('岸')
        || room_name.contains('港')
        || room_name.contains("碼頭")
        || room_name.contains('灘');

    let lead_in = if is_indoor {
        if unmentioned.len() == 1 {
            format!("隅處設有一方{}。", joined)
        } else {
            format!("近旁設有{}。", joined)
        }
    } else if is_waterfront {
        format!("岸畔置有{}。", joined)
    } else {
        format!("近旁立有{}。", joined)
    };

    let mut res = desc.trim_end().to_string();
    if !res.ends_with('。') && !res.ends_with('！') && !res.ends_with('？') {
        res.push('。');
    }
    res.push_str(&lead_in);
    res
}

fn player_layer_scene_desc(
    desc: &str,
    terrain_name: &str,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> String {
    let mut text = desc.trim().to_string();
    let terrain = terrain_name.trim();

    let gh = (game_minute_of_day / 60) % 24;
    let is_night = gh <= 4 || gh >= 23;
    let is_empty = unnamed_total == 0;
    if is_night || is_empty {
        let replacement = if is_night {
            "，夜色深沉，街面靜謐無人"
        } else {
            "，四周一時安靜清朗"
        };
        for needle in [
            "，行人踏石而過，趣味盎然",
            ", 行人踏石而過，趣味盎然",
            "、行人踏石而過，趣味盎然",
            "。行人踏石而過，趣味盎然",
            "行人踏石而過，趣味盎然",
            "，人來人往",
            ", 人來人往",
            "人來人往",
            "，買賣聲此起彼落",
            ", 買賣聲此起彼落",
            "買賣聲此起彼落",
            "，來往居民互致問候",
            ", 來往居民互致問候",
            "來往居民互致問候",
        ] {
            if text.contains(needle) {
                text = text.replace(needle, replacement);
                break;
            }
        }
    }

    if let Some(scene_desc) = scene_desc_override(
        terrain,
        game_minute_of_day,
        climate,
        unnamed_total,
        recent_events,
    ) {
        return scene_desc;
    }
    let backend_leaks = [
        "目前登入後的起點",
        "登入後的暫存起點",
        "世界地圖尚未接入",
        "尚未寫入場景描述",
    ];

    if text.is_empty() || backend_leaks.iter().any(|needle| text.contains(needle)) {
        if terrain.contains("半嶺") || terrain.contains("北岸") || terrain.contains("等候") {
            return north_bank_waiting_scene_desc(
                game_minute_of_day,
                climate,
                unnamed_total,
                recent_events,
            );
        }
        let is_night = gh <= 4 || gh >= 23;
        let is_empty = unnamed_total == 0;
        if terrain.is_empty() {
            if is_night {
                return "一處可落腳的地方展在近前，夜色深沉，四周靜謐無人，幾條去處從眼前分開。".to_string();
            } else if is_empty {
                return "一處可落腳的地方展在近前，四周一時安靜清朗，幾條去處從眼前分開。".to_string();
            } else {
                return "一處可落腳的地方展在近前，周圍有人聲與腳步聲，幾條去處從眼前分開。".to_string();
            }
        }
        if is_night {
            return format!("{}在近前展開，夜色深沉，街面靜謐無人，幾條去處從眼前分開。", terrain);
        } else if is_empty {
            return format!("{}在近前展開，四周一時安靜清朗，幾條去處從眼前分開。", terrain);
        } else {
            return format!(
                "{}在近前展開，周圍有人聲與腳步聲，幾條去處從眼前分開。",
                terrain
            );
        }
    }

    text.to_string()
}


fn north_bank_waiting_scene_desc(
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> String {
    let (climate_key, climate_seed) = climate.unwrap_or(("clear", 0));
    let game_hour = (game_minute_of_day / 60) % 24;
    let ferry = FerryPhase::from_minute(game_minute_of_day);
    let seed = climate_seed
        .wrapping_add(game_hour as i64 * 31)
        .wrapping_add(unnamed_total as i64 * 17)
        .wrapping_add(ferry.seed_weight());
    let skeleton = "半嶺水岸渡口沿著中河鋪開，濕木踏板接著渡船；水尾寮渡頭市集和歇腳處貼在岸邊。";
    let phenomenon = north_bank_phenomenon(game_hour, climate_key, seed);
    let object_surface = north_bank_object_surface(seed);
    let ferry_presence = north_bank_ferry_presence(ferry, seed);
    let presence = north_bank_presence(unnamed_total, seed);
    let trace = north_bank_trace(recent_events, climate_key, ferry, unnamed_total, seed);
    format!("{skeleton}{phenomenon}{object_surface}{ferry_presence}{presence}{trace}")
}


enum ScenePhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FerryPhase {
    NorthDocked,
    CrossingSouth,
    SouthDocked,
    CrossingNorth,
}

impl FerryPhase {
    fn from_minute(game_minute_of_day: u32) -> Self {
        match game_minute_of_day % 40 {
            0..=9 => FerryPhase::NorthDocked,
            10..=19 => FerryPhase::CrossingSouth,
            20..=29 => FerryPhase::SouthDocked,
            _ => FerryPhase::CrossingNorth,
        }
    }

    fn seed_weight(self) -> i64 {
        match self {
            FerryPhase::NorthDocked => 0,
            FerryPhase::CrossingSouth => 19,
            FerryPhase::SouthDocked => 37,
            FerryPhase::CrossingNorth => 53,
        }
    }
}

fn scene_phase(hour: u32) -> ScenePhase {
    match hour % 24 {
        5..=10 => ScenePhase::Dawn,
        11..=16 => ScenePhase::Day,
        17..=19 => ScenePhase::Dusk,
        _ => ScenePhase::Night,
    }
}

fn scene_desc_override(
    room_name: &str,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> Option<String> {
    if room_name.contains("半嶺水岸渡口")
        || room_name.contains("水岸渡口")
        || room_name.contains("北岸")
        || room_name.contains("等候")
    {
        Some(north_bank_waiting_scene_desc(
            game_minute_of_day,
            climate,
            unnamed_total,
            recent_events,
        ))
    } else if room_name.contains("水尾寮渡頭市集")
        || room_name.contains("渡頭市集")
        || room_name.contains("市集")
    {
        Some("水尾寮渡頭市集沿著木架排開，價牌被河風推得輕晃；小件貨、乾糧與船票消息混在一起，被等船的人低聲議價。".to_string())
    } else if room_name.contains("歇腳") {
        Some("臨河歇腳處擺著幾條長凳，茶桶冒著淡淡熱氣；歇腳的人把包袱壓在腳邊，眼神仍留著河面的動靜。".to_string())
    } else if room_name.contains("踏板") {
        Some("渡船踏板濕滑發亮，一端扣著船繩，一端連著岸上的木樁；等船的人在踏板前收住腳步，聽水聲拍著船腹。".to_string())
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("南岸泥坡被來往腳印踩得發亮，河水退去後留下濕泥與碎草；下船的人沿坡上行，也有人回頭看渡船是否還會再靠。".to_string())
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("船戶作業點堆著纜繩、撐篙與修補木片；船邊的人低聲分派活計，偶爾有人伸手試一試船身晃動。".to_string())
    } else if room_name.contains("倉棚前場") {
        Some("倉棚前場半開著棚門，麻袋與木箱靠牆排放；搬貨的人在泥地上留下一串深腳印，等著下一趟船把貨送走。".to_string())
    } else {
        None
    }
}

#[derive(Debug)]
struct NpcCard<'a> {
    id: &'a str,
    label: String,
    card_state: &'static str,
    current_activity: &'a str,
    interaction_priority: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct VisibleNpcCard<'a> {
    id: &'a str,
    display_name: String,
    card_state: &'static str,
    current_activity: &'a str,
}
fn narrative_person_subject(card: &VisibleNpcCard<'_>, index: usize) -> String {
    if card.card_state == "named" && !card.display_name.starts_with("陌生人") {
        return card.display_name.clone();
    }

    let hint = format!("{} {}", card.display_name, card.current_activity);
    let sensory = crate::store::npc_sensory_appearance_line(card.id, &hint);

    let first_attire = sensory
        .split(['，', '、', '。'])
        .next()
        .map(|s| s.trim())
        .unwrap_or("");

    let role_pers = crate::store::npc_persistent_role_label(card.id, &hint);
    let role = if card.id.starts_with("wt_vendor_") || role_pers.contains("攤") {
        "攤販"
    } else if role_pers.contains("主事")
        || role_pers.contains("長")
        || role_pers.contains("總管")
        || role_pers.contains("會")
        || card.current_activity.contains("審理戶籍")
        || card.current_activity.contains("戶政")
    {
        "主事"
    } else if role_pers.contains("編") {
        "主編"
    } else if role_pers.contains("廚")
        || card.current_activity.contains("熬煮")
        || card.current_activity.contains("濃湯")
    {
        "主廚"
    } else if role_pers.contains("匠") || role_pers.contains("工") || role_pers.contains("鍛") || role_pers.contains("師") {
        "師傅"
    } else if role_pers.contains("生") || role_pers.contains("吏") || role_pers.contains("教") || role_pers.contains("學") || role_pers.contains("醫") {
        "人員"
    } else if role_pers.contains("商") || role_pers.contains("客") {
        "商旅"
    } else {
        "住戶"
    };

    let prefix = if index == 0 { "一名" } else { "另一名" };
    if !first_attire.is_empty() {
        let mut cleaned_attire = first_attire.replace("身穿", "身著").replace("穿著", "身著");
        if !cleaned_attire.starts_with("身著")
            && !cleaned_attire.starts_with("披著")
            && !cleaned_attire.starts_with("戴著")
            && !cleaned_attire.starts_with("繫著")
            && !cleaned_attire.starts_with("手持")
            && !cleaned_attire.starts_with("腰")
            && !cleaned_attire.starts_with("頭戴")
            && !cleaned_attire.starts_with("頭裹")
            && !cleaned_attire.starts_with("圍著")
            && !cleaned_attire.starts_with("面容")
            && !cleaned_attire.starts_with("手握")
            && !cleaned_attire.starts_with("肩")
        {
            cleaned_attire = format!("身著{cleaned_attire}");
        }
        let attire_desc = if cleaned_attire.contains('的') {
            cleaned_attire.replacen('的', "", 1)
        } else {
            cleaned_attire
        };
        format!("{prefix}{attire_desc}的{role}")
    } else {
        let brief = crate::store::npc_brief_appearance_tag(card.id, &card.display_name);
        if !brief.is_empty() && brief != "街坊居民" {
            let brief_clean = if brief.contains('的') {
                brief.replacen('的', "", 1)
            } else {
                brief.to_string()
            };
            format!("{prefix}{brief_clean}的{role}")
        } else if index == 0 {
            format!("一名在場{role}")
        } else {
            format!("另一名在場{role}")
        }
    }
}


fn build_player_layer_npc_cards<'a>(cards: &'a [NpcCard<'a>]) -> (Vec<VisibleNpcCard<'a>>, bool) {
    let mut selected: Vec<&NpcCard<'a>> = Vec::new();

    // 保持排序優先順序：先加入 interaction_priority 為 true 的角色，再加入其餘角色。
    for card in cards.iter().filter(|card| card.interaction_priority) {
        selected.push(card);
    }

    for card in cards {
        if card.interaction_priority {
            continue;
        }
        selected.push(card);
    }

    // 編號：同 label 桶內 ≥ 2 人按順序附上數字（1-based）。
    let mut label_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for card in &selected {
        *label_counts.entry(card.label.as_str()).or_insert(0) += 1;
    }

    let mut label_cursor: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let visible = selected
        .into_iter()
        .map(|card| {
            let count = label_counts.get(card.label.as_str()).copied().unwrap_or(1);
            let display_name = if count >= 2 {
                let cursor = label_cursor.entry(card.label.as_str()).or_insert(0);
                *cursor += 1;
                format!("{}{}", card.label, *cursor)
            } else {
                card.label.clone()
            };
            VisibleNpcCard {
                id: card.id,
                display_name,
                card_state: card.card_state,
                current_activity: card.current_activity,
            }
        })
        .collect();

    (visible, false)
}


fn player_layer_entry_log_for_scene(
    room_name: &str,
    stranger_total: usize,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
) -> String {
    if stranger_total == 0 {
        return String::new();
    }
    if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        return north_bank_entry_scene_flow(game_minute_of_day, climate, stranger_total);
    }
    scene_entry_log(room_name).unwrap_or("").to_string()
}

fn player_layer_reentry_log(room_name: &str, stranger_total: usize) -> &'static str {
    if stranger_total == 0 {
        return "";
    }
    scene_reentry_log(room_name).unwrap_or("")
}

fn player_layer_route_reentry_logs(room_name: &str, stranger_total: usize) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if stranger_total == 0 {
        return serde_json::Value::Object(out);
    }
    for (from_room, line) in scene_route_reentry_logs(room_name) {
        out.insert(from_room.to_string(), serde_json::json!(line));
    }
    serde_json::Value::Object(out)
}


fn scene_entry_log(room_name: &str) -> Option<&'static str> {
    if let Some(line) = game::final_map_entry_log_for_room(room_name) {
        Some(line)
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        Some(
            "船繩旁有人把濕繩重新繞上木樁，船身在水霧裡輕輕碰了一下岸。\n市集那頭有人壓低聲音討價，木牌被指節敲得輕響。\n等船人群往踏板口挪了一步，又停在河面傳來的水聲裡。",
        )
    } else if room_name.contains("中河谷地路口") {
        Some("你走到中河谷地路口。渡口人聲從身後貼來，泥路往田埂和市集外分開，腳印在路口交錯。")
    } else if room_name.contains("北堤泥路") {
        Some("你踏上北堤泥路。河霧退到堤邊，挑擔腳印沿泥面排開，往田埂方向的車痕更窄。")
    } else if room_name.contains("田埂水口") {
        Some("你走到田埂水口。細水從木閘下淌過，遠處渡口聲被田間蛙鳴壓低。")
    } else if room_name.contains("上田小路") {
        Some("你沿上田小路停下。稻葉貼著田埂，遠處只剩斷續人影和水聲。")
    } else if room_name.contains("田區路口") {
        Some("你走到田區路口。上田小路在身後收窄，水田、木牌和泥腳印把路分向灌渠、農舍與更深田埂。")
    } else if room_name.contains("灌渠木閘") {
        Some("你停在灌渠木閘旁。木閘下的水聲貼著田埂流過，有人蹲著看水尺，竹耙斜靠在泥邊。")
    } else if room_name.contains("秧苗田邊") {
        Some("你走到秧苗田邊。淺綠秧苗貼著水面鋪開，彎身插秧的人影只露出背脊，蛙聲從水溝裡冒起。")
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        Some(
            "你走到水尾寮水網口。田邊腳印在濕木棧上換成船路痕跡，引船人靠著篙影看水道，霧牌在木柱上輕晃。",
        )
    } else if room_name.contains("湖港木棧") {
        Some(
            "你踏上湖港木棧。濕繩圈和潮線貼在木板邊，候船的人給魚籃與蘆草捆讓路，幾處水聲在棧下碰到一起。",
        )
    } else if room_name.contains("短船泊位") {
        Some("你停在短船泊位旁。內水短船貼著木樁晃動，船戶扶著船舷催人站穩，船槳和篙影在水面交錯。")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some(
            "你走進魚貨分揀場。濕布鋪在魚籃下，分揀工把新上岸的魚貨推向秤邊，腥味和短賣低語順著木棧散開。",
        )
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some(
            "你走到蘆草貨棚。蘆草捆靠著曬架排開，採集者拖來的濕痕還沒乾，棚下草屑被腳步帶到板路上。",
        )
    } else if room_name.contains("水倉") {
        Some(
            "你來到水倉前場。板車停在潮濕倉門外，倉管把魚貨與蘆草分往不同棚口，木地板下回著悶悶水聲。",
        )
    } else if room_name.contains("糧倉外秤") {
        Some(
            "你走到糧倉外秤。下游田埂路的車轍在雨棚下收住，秤手把糧袋扶上秤盤，秤桿影子壓在稻殼粉裡。",
        )
    } else if room_name.contains("倉門前場") {
        Some(
            "你走到倉門前場。高倉門只開著一線，守倉人照著秤單放行，封條和木栓在門側晃出很短的聲音。",
        )
    } else if room_name.contains("曬場邊") {
        Some("你來到曬場邊。曬席貼著地面鋪開，翻穀人推著木耙走過，穀粒乾響順著風貼到外秤那頭。")
    } else if room_name.contains("車行口") {
        Some("你走到車行口。車轍壓進倉區泥面，車夫扶著車轅等前車挪動，排隊牌旁的麻繩繃得很緊。")
    } else if room_name.contains("守倉棚") {
        Some(
            "你走進守倉棚下。冊簿攤在茶壺旁，倉管把秤單、封條和車次對在一起，倉門前的聲音到這裡便低下去。",
        )
    } else if room_name.contains("糧車泊位") {
        Some(
            "你來到糧車泊位。車架停在泊位邊，搬糧人從車尾接下糧袋，麻繩和木輪聲一前一後往守倉棚那頭傳。",
        )
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some(
            "你走到河口潮泥碼頭。退潮後的黑泥貼著木樁，碼頭工把河上糧袋和海船木箱分到兩側，纜繩聲混著鹹潮味。",
        )
    } else if room_name.contains("稅站棚") {
        Some(
            "你走進稅站棚下。貨單攤在木桌上，稅吏用印章壓住捲起的紙角，棚外候查的木箱和糧袋排得很緊。",
        )
    } else if room_name.contains("船行門面") {
        Some(
            "你來到船行門面。潮濕航線板掛在門下，船行人把船位、牌價和雇船消息一口氣遞向碼頭與貨場。",
        )
    } else if room_name.contains("潮線貨場") {
        Some("你走到潮線貨場。木箱墊在石條上避開潮印，搬貨人趁水位低時把貨往候泊線和稅站棚兩側推。")
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some(
            "你停在風舶角港倉門前。雨布半遮著門口，倉管把河貨、海貨和放行貨單分開，門內木箱悶悶靠上牆。",
        )
    } else if room_name.contains("商船候泊線") {
        Some("你來到商船候泊線。等潮的船影停在外頭，船員把錨繩收成圈，候泊牌在風裡敲著木柱。")
    } else if room_name.contains("東南船塢口") {
        Some(
            "你走到東南船塢口。潮線貼著風舶角港倉門外沿過來，船匠把半修的船板靠在入口旁，木屑和瀝青味從修船架那頭飄出。",
        )
    } else if room_name.contains("修船架") {
        Some(
            "你走到修船架下。翻起的船身被木架撐住，修船工沿著肋木敲打，瀝青桶和楔子把通路擠得很窄。",
        )
    } else if room_name.contains("補給倉") {
        Some(
            "你停在補給倉前。淡水桶、乾糧箱和油布包分成幾堆，倉管照著近海船期把份量往碼頭木階那頭推。",
        )
    } else if room_name.contains("碼頭木階") {
        Some("你來到碼頭木階。半濕木階沿著繩柱往下，船客扶著欄繩上下，補給倉的水桶聲從身後貼過來。")
    } else if room_name.contains("帆布曬場") {
        Some("你走進帆布曬場。舊帆鋪在繩架上，補帆人沿裂口穿針，鹹風把外貿碼頭那頭的船旗聲帶過來。")
    } else if room_name.contains("船戶街口") {
        Some(
            "你走到船戶街口。招工牌掛在飯攤旁，船戶在門口比劃工錢與船期，帆布曬場和碼頭木階的人聲從兩側交進來。",
        )
    } else if room_name.contains("外貿碼頭") {
        Some(
            "你走到外貿碼頭。港旗在海風裡扯直，船員把外地貨箱從吊繩下扶穩，舶司前棚那邊已有人等著看貨單。",
        )
    } else if room_name.contains("舶司前棚") {
        Some("你走進舶司前棚下。濕貨單被壓在木桌上，舶司吏按住封條，等翻譯桌那邊把貨名和合約說清。")
    } else if room_name.contains("商館街口") {
        Some("你走到商館街口。褪色招牌掛在門前，商館人打開貨樣匣，外語低語和碼頭船旗聲一同靠近。")
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some(
            "你停在風舶角外洋棧倉前。雨布半遮著門口，倉管把異地木箱分成已查與待查兩側，門內拖箱聲沉沉傳出。",
        )
    } else if room_name.contains("翻譯桌") {
        Some(
            "你走到翻譯桌旁。合約攤在鎮紙下，翻譯把陌生貨名轉成在地話，舶司前棚的印章聲在旁邊等著落下。",
        )
    } else if room_name.contains("匯兌小屋") {
        Some("你走進匯兌小屋前。銅秤在桌邊輕晃，兌票人把商館街口報來的價碼寫進帳冊，錢票疊在手邊。")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some(
            "你走到風舶角避風港。冷海水貼著木樁晃動，船戶把結霜纜繩重新繞緊，避風船在內側輕碰碼頭。",
        )
    } else if room_name.contains("港灣貨棚") {
        Some("你走進港灣貨棚前。鹽箱和濕木箱靠牆排開，搬魚人把新卸下的魚籃推向冷霧魚市那頭。")
    } else if room_name.contains("漁網曬場") {
        Some("你走到漁網曬場。半乾濕網掛在木架上，補網人沿破口穿線，海風把避風碼頭的船聲帶過來。")
    } else if room_name.contains("冷霧魚市") {
        Some(
            "你來到冷霧魚市。木秤還沾著水珠，魚販把濕布下的魚貨攤開，買貨人的問價聲貼著霧氣低低傳開。",
        )
    } else if room_name.contains("船戶屋排") {
        Some("你走到船戶屋排。火盆煙氣貼著低屋簷，船戶家屬在輪值板前換下濕木牌，屋後風聲很細。")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("你踏上烏石鼻守望石階。霧牌和警鈴掛在風口，守望人扶著濕石欄，看向港外海面的顏色。")
    } else if room_name.contains("礦坑外棚") {
        Some("你走到礦坑外棚。濕支柱撐住低矮棚頂，吊燈光在坑口晃動，礦工把沾泥的鎬靠上木架。")
    } else if room_name.contains("洗礦溝") {
        Some("你走到洗礦溝旁。冷水沿木槽往下流，洗礦工把石籃壓進混濁水裡，碎料聲貼著坡面傳開。")
    } else if room_name.contains("粗料倉") {
        Some("你來到粗料倉前。濕礦袋堆在木牌下，倉管用炭線補上坑口和重量，車輪聲從西側舊道靠近。")
    } else if room_name.contains("工具修棚") {
        Some("你走進工具修棚前。裂柄鎬和車輪鐵圈掛在木架上，修具工把鬆鉚釘敲緊，火星短短一亮。")
    } else if room_name.contains("礦渣坡") {
        Some("你停在礦渣坡邊。灰黑碎石堆到安全線內，推車人把廢料倒下，幾道新滑坡痕沿坡面斜斜拉開。")
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("你走到紅土崁選礦車場。礦車停在磨亮車轍旁，車夫把繩索掛上車架，稱重木牌半埋在灰粉裡。")
    } else if room_name.contains("農舍前坪") {
        Some("你走進農舍前坪。鋤具靠在牆下，曬衣繩從低簷拉到木樁，屋前濕泥帶著從田裡回來的腳印。")
    } else if room_name.contains("穀場小棚") {
        Some("你來到穀場小棚。幾袋稻穀半遮在棚下，木篩靠著棚柱，稻殼被腳步踩得乾乾作響。")
    } else if room_name.contains("田埂深處") {
        Some("你走進田埂深處。主要腳步聲被留在身後，濕草貼著褲腳，水鳥在更低的田面一閃而過。")
    } else if room_name.contains("挑擔小徑") {
        Some("你轉進挑擔小徑。草邊腳印密起來，短貨與水桶像是常避著渡口正面從這裡穿過。")
    } else if room_name.contains("水尾寮外谷地路") || room_name.contains("外谷地路") {
        Some("你走到水尾寮外谷地路。攤架聲被留在身後，挑擔的人沿泥路把小件貨送進送出。")
    } else if room_name.contains("南側谷地路") {
        Some("你走上南側谷地路。泥坡外的人聲低下去，車轍往田埂與下游方向拉長。")
    } else if room_name.contains("下游田埂路") {
        Some("你走到下游田埂路。車轍變淺，水田和濕草把渡口貨聲接成遠遠的回響。")
    } else if room_name.contains("渡口貨路") {
        Some("你走上渡口貨路。雨布與車輪印把倉棚前場的貨流牽向更遠的河邊。")
    } else if room_name.contains("貨路外彎") {
        Some("你來到貨路外彎。路面在草坡前拐開，貨車痕和腳印都被河風磨得散亂。")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("你走進水尾寮渡頭市集。價牌掛在貨攤邊晃動，有人掂著小包零貨，也有人壓低聲音打聽下一班渡船。")
    } else if room_name.contains("歇腳") {
        Some("你來到臨河歇腳處。長凳旁有人捧著茶碗暖手，幾只包袱靠在腳邊，談話聲被河風磨得很低。")
    } else if room_name.contains("踏板") {
        Some("你停在渡船踏板前。濕木板隨船身輕晃，有人扶著繩柱等船戶示意，腳步聲在水面上散開。")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("你踏上南岸泥坡。濕泥黏住鞋底，有人提著包袱往坡上走，也有人站在坡邊等船再度靠穩。")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some(
            "你走近船戶作業點。纜繩被一圈圈收在木樁邊，船邊的人壓低聲音分派活計，篙影在水面上晃過。",
        )
    } else if room_name.contains("倉棚") {
        Some("你來到倉棚前場。半開的棚門旁堆著麻袋與木箱，搬貨的人在泥地上停停走走，等下一趟渡船。")
    } else {
        None
    }
}

fn north_bank_entry_scene_flow(
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
) -> String {
    let (climate_key, climate_seed) = climate.unwrap_or(("clear", 0));
    let game_hour = (game_minute_of_day / 60) % 24;
    let ferry = FerryPhase::from_minute(game_minute_of_day);
    let seed = climate_seed
        .wrapping_add(game_minute_of_day as i64 * 11)
        .wrapping_add(unnamed_total as i64 * 17)
        .wrapping_add(ferry.seed_weight());
    let ferry_line = north_bank_entry_ferry_flow(ferry, seed);
    let market_line = north_bank_entry_market_flow(game_hour, climate_key, seed + 7);
    let crowd_line = north_bank_entry_crowd_flow(unnamed_total, ferry, seed + 13);
    format!("{ferry_line}\n{market_line}\n{crowd_line}")
}

fn north_bank_entry_ferry_flow(ferry: FerryPhase, seed: i64) -> &'static str {
    match ferry {
        FerryPhase::NorthDocked => pick_scene_variant(
            &[
                "船繩旁有人把濕繩重新繞上木樁，船身貼岸時輕輕碰了一下踏板。",
                "踏板口的木板被船腹碰得短短一響，船繩被人往木樁上多繞半圈。",
            ],
            seed,
        ),
        FerryPhase::CrossingSouth => pick_scene_variant(
            &[
                "渡船離了北岸，船繩在木樁旁鬆下一圈，水面還留著撐篙攪開的痕。",
                "踏板前的人把腳步收住，船尾往南岸滑開，只剩船繩滴水落回木樁邊。",
            ],
            seed,
        ),
        FerryPhase::SouthDocked => pick_scene_variant(
            &[
                "北岸木樁旁的船繩垂著，對岸有人影起落，踏板前只剩等船的低聲催問。",
                "渡船停在南岸那頭，這邊有人把船繩理直，又抬眼看對岸是否放人。",
            ],
            seed,
        ),
        FerryPhase::CrossingNorth => pick_scene_variant(
            &[
                "船繩旁的人先把繩尾理開，船頭從河霧裡近了一截，踏板前的人聲短短壓低。",
                "水面傳來撐篙聲，船繩旁有人把繩圈攤開，等著船腹重新貼岸。",
            ],
            seed,
        ),
    }
}

fn north_bank_entry_market_flow(game_hour: u32, climate_key: &str, seed: i64) -> &'static str {
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "市集棚腳還在滴水，有人把價牌往乾處挪，討價聲被雨後濕氣壓得更低。",
                "市集湯攤熱氣混著雨味往上冒，木牌被指節敲了一下，聲音很快被水聲吞掉。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "市集價牌在霧裡只露出邊角，討價的人靠得更近，聲音貼著貨擔傳開。",
                "霧把市集那頭收得很窄，有人摸著木牌邊緣報價，又順勢壓低嗓音。",
            ],
            seed,
        ),
        "wind" => pick_scene_variant(
            &[
                "河風吹得市集棚布低低鼓起，價牌碰在木架上，討價聲斷成幾截。",
                "市集那頭有人按住晃動的木牌，另一手把零貨往棚柱內側推。",
            ],
            seed,
        ),
        _ => match scene_phase(game_hour) {
            ScenePhase::Dawn => pick_scene_variant(
                &[
                    "市集把早市價牌掛起，湯桶熱氣先冒出來，早到的人低聲問第一班船。",
                    "天光還薄，市集的人把木牌翻到外側，貨聲比人聲更早貼上岸邊。",
                ],
                seed,
            ),
            ScenePhase::Day => pick_scene_variant(
                &[
                    "市集那頭有人壓低聲音討價，木牌被指節敲得輕響。",
                    "日光照開市集價牌，有人把小件貨往外推半寸，便被問價聲攔住。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "市集開始收暗，有人把價牌往燈邊移，討價聲貼著棚布低低延續。",
                    "暮色壓到棚口，貨聲變少，剩下幾句趕船前的短價來回咬著。",
                ],
                seed,
            ),
            ScenePhase::Night => pick_scene_variant(
                &[
                    "市集只剩幾圈黃暈，有人隔著燈影問價，木牌輕響後又沉進夜風裡。",
                    "夜裡市集的貨聲收得很低，湯攤火光貼著木牌晃，問船的人不再大聲說話。",
                ],
                seed,
            ),
        },
    }
}

fn north_bank_entry_crowd_flow(unnamed_total: usize, ferry: FerryPhase, seed: i64) -> &'static str {
    if unnamed_total >= 12 {
        match ferry {
            FerryPhase::NorthDocked => {
                "等船人群往踏板口擠出一條窄縫，包袱和貨擔又在船繩聲裡短短合攏。"
            }
            FerryPhase::CrossingSouth => {
                "踏板前的人群一起往後收，空出的濕木邊很快被包袱和貨擔佔住。"
            }
            FerryPhase::SouthDocked => {
                "人群壓在岸邊等對岸放船，幾個挑擔客換了肩，空隙被後面腳步補上。"
            }
            FerryPhase::CrossingNorth => "等船人群往踏板口挪了一步，又停在河面傳來的水聲裡。",
        }
    } else if unnamed_total >= 5 {
        pick_scene_variant(
            &[
                "幾撥候渡的人把包袱挪到腳邊，視線在市集、踏板和河面之間來回。",
                "候渡的人從歇腳棚邊站起來，往踏板口看了一眼，又被旁邊貨擔擋住去路。",
            ],
            seed,
        )
    } else {
        pick_scene_variant(
            &[
                "零散候渡者靠著木樁等船，腳步聲在濕木板上顯得很清楚。",
                "岸邊只剩少數人守著包袱，河面一響，幾道視線便同時轉過去。",
            ],
            seed,
        )
    }
}

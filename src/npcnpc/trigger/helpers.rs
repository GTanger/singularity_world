fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
#[allow(dead_code)]
trait KeyObjectsJoinExt {
    fn join(&self, sep: &str) -> String;
}

impl KeyObjectsJoinExt for str {
    fn join(&self, sep: &str) -> String {
        self.split('、')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(sep)
    }
}

fn trunc_rune(s: &str, max: usize) -> String {
    let ch: Vec<char> = s.chars().collect();
    if ch.len() <= max {
        return s.to_string();
    }
    ch[..max].iter().collect()
}

fn sprintf_s(template: &str, args: &[&str]) -> String {
    let mut s = template.to_string();
    for a in args {
        if let Some(pos) = s.find("%s") {
            s.replace_range(pos..pos + 2, a);
        }
    }
    s
}

fn player_layer_room_name(room_id: &str) -> String {
    let raw_name = if let Some(coord) = crate::grid::parse_grid_room_id(room_id)
        && let Some(cell) = crate::server::grid_manager::get_runtime_grid_cell(coord)
    {
        if cell.name.trim().is_empty() {
            crate::grid::terrain_name_zh(cell.terrain).to_string()
        } else {
            cell.name
        }
    } else {
        let name = db::get_room_name(room_id).unwrap_or_default();
        if name.trim().is_empty() {
            "半島街巷".to_string()
        } else {
            name
        }
    };
    crate::store::clean_wilderness_place_label(&raw_name)
        .replace("大圳頭・西南原", "大圳頭・圳頭")
        .replace("西門外坊・西南原", "西門外坊・西門")
        .replace("西南原", "大圳頭")
        .replace("防爆防風", "防風")
        .replace("黑沼關隘", "下營")
        .replace("鹽田澳", "烏石鼻")
        .replace("青籠關", "下營")
        .replace("黑沼林澤", "水尾寮")
        .replace("黑沼林", "水尾寮")
        .replace("白塔山麓", "半嶺鎮")
        .replace("北聖山", "北嶺腳")
        .replace("西北礦帶", "紅土崁")
}

pub fn resolve_viewer_npc_label(player_id: &str, c: &crate::entity::Character, room_name: &str) -> String {
    if crate::store::player_knows_npc(player_id, &c.id) {
        return crate::store::resolve_npc_person_name(&c.id, &c.display_title, None);
    }
    let (is_staff, has_uniform, in_counter_zone) =
        crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
            room_name,
            &c.id,
            &c.display_title,
            &c.current_activity,
            &c.equipment_slots,
        );
    if is_staff {
        let role_title = crate::grid::resolve_any_room_venue_object(room_name, &c.display_title)
            .map(|v| v.default_host_title)
            .unwrap_or("當值主事");
        if in_counter_zone && has_uniform {
            return format!("身著制服站於櫃檯內側的{role_title}");
        } else if in_counter_zone {
            return format!("站在櫃檯內側的{role_title}");
        } else if has_uniform {
            return format!("身著工作制服的{role_title}");
        }
        return role_title.to_string();
    }
    let tag = crate::store::npc_brief_appearance_tag(&c.id, &c.display_title);
    let role = if room_name.contains("工坊") || room_name.contains("鍛造") || room_name.contains("木作") || room_name.contains("鐵坊") {
        "匠人"
    } else if room_name.contains("藥") || room_name.contains("診") || room_name.contains("脈") {
        "醫工"
    } else if room_name.contains("學") || room_name.contains("講堂") || room_name.contains("書") || room_name.contains("印行") || room_name.contains("觀星") {
        "學者"
    } else if room_name.contains("商") || room_name.contains("鋪") || room_name.contains("市集") || room_name.contains("行") {
        "行商"
    } else if room_name.contains("渡") || room_name.contains("津") || room_name.contains("水尾") || room_name.contains("河") {
        "候渡客"
    } else if room_name.contains("茶") || room_name.contains("食") || room_name.contains("泉") || room_name.contains("旅社") || room_name.contains("驛館") {
        "茶客"
    } else {
        "街坊"
    };
    if !tag.is_empty() {
        format!("{tag}的{role}")
    } else {
        format!("一名{role}")
    }
}

pub fn format_staged_npc_dialogue(
    pattern_idx: usize,
    label_a: &str,
    label_b: &str,
    line_a: &str,
    line_b: &str,
    activity_a: &str,
    activity_b: &str,
) -> String {
    let mut stripped_a = String::with_capacity(activity_a.len());
    let mut in_bracket = false;
    for ch in activity_a.chars() {
        if ch == '【' {
            in_bracket = true;
        } else if ch == '】' {
            in_bracket = false;
        } else if !in_bracket {
            stripped_a.push(ch);
        }
    }
    let clean = stripped_a.trim().trim_start_matches("正在").trim_start_matches('正').trim();

    let is_transit = clean.contains("動身前往")
        || clean.contains("前往")
        || clean.contains("趕往")
        || clean.contains("步行")
        || clean.contains("沿街")
        || clean.contains("返回")
        || clean.contains("邁步")
        || clean.contains("穿行");
    let is_rest = clean.contains("安歇")
        || clean.contains('歇')
        || clean.contains('睡')
        || clean.contains("吹燈")
        || clean.contains("就寢")
        || clean.contains("無所事事");
    let is_eating = clean.contains("用膳")
        || clean.contains('吃')
        || clean.contains('飲')
        || clean.contains('茶')
        || clean.contains('湯')
        || clean.contains("果腹")
        || clean.contains("墊肚")
        || clean.contains("熱粥")
        || clean.contains("粗麵");

    let act_clause = if clean.is_empty() || is_transit || is_rest || is_eating {
        String::new()
    } else if clean.chars().count() > 10 {
        "手頭活計略作停歇，".to_string()
    } else {
        format!("停下手中{clean}的活計，")
    };
    let _ = activity_b;
    match pattern_idx % 8 {
        0 => format!("{label_a}{act_clause}側過身對{label_b}言道：「{line_a}」\n{label_b}微微點頭，隨口應道：「{line_b}」"),
        1 => format!("{label_a}將手頭物事略作打理，向身側的{label_b}開口：「{line_a}」\n{label_b}抬眼笑應：「{line_b}」"),
        2 => format!("只聽{label_a}若有所思地望向身旁，同{label_b}搭話：「{line_a}」\n{label_b}順著話頭接道：「{line_b}」"),
        3 => format!("{label_a}朝{label_b}低聲說道：「{line_a}」\n{label_b}神情認真地思忖片刻，答道：「{line_b}」"),
        4 => format!("{label_a}放緩步調，轉頭對{label_b}說道：「{line_a}」\n{label_b}側耳聽完，溫聲應著：「{line_b}」"),
        5 => format!("案几旁，{label_a}手上動作未停，對{label_b}言道：「{line_a}」\n{label_b}語調平穩地回道：「{line_b}」"),
        6 => format!("{label_a}見{label_b}迎面而立，便啟齒笑道：「{line_a}」\n{label_b}會心一笑，朗聲回道：「{line_b}」"),
        _ => format!("{label_a}壓低嗓音，向{label_b}問起：「{line_a}」\n{label_b}略一沉吟，輕聲答話：「{line_b}」"),
    }
}

fn player_layer_pair_labels(room_name: &str) -> (String, String) {
    if ["渡", "埠", "橋", "臨河"].iter().any(|k| room_name.contains(k)) {
        ("候渡客".to_string(), "舟子".to_string())
    } else if let Some(venue) = crate::grid::resolve_any_room_venue_object(room_name, "") {
        (
            venue.default_host_title.to_string(),
            format!("來{}辦事的街坊", venue.venue_kind),
        )
    } else if room_name.contains("坊") || room_name.contains("匠") {
        ("掌爐匠人".to_string(), "工坊幫工".to_string())
    } else if room_name.contains("商") || room_name.contains("市") {
        ("採買行商".to_string(), "掌案帳房".to_string())
    } else {
        ("行路街客".to_string(), "鄰坊居民".to_string())
    }
}

fn replace_private_names_for_player(text: &str, names: &[(&str, &str)]) -> String {
    let mut out = text.to_string();
    for (private, public) in names {
        let private = private.trim();
        if !private.is_empty() {
            out = out.replace(private, public);
        }
    }
    out
}

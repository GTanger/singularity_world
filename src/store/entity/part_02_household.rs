pub fn deterministic_wenyao_full_name_with_gender(
    npc_id: &str,
    gender_hint: Option<&str>,
) -> String {
    deterministic_wenyao_full_name_salted(npc_id, gender_hint, 0)
}

pub fn deterministic_wenyao_full_name_salted(
    npc_id: &str,
    gender_hint: Option<&str>,
    salt: usize,
) -> String {
    let is_female = npc_effective_is_female(npc_id, gender_hint);

    let mut z: u64 = 0xcbf2_9ce4_8422_2325_u64.wrapping_add((salt as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
    for b in npc_id.bytes() {
        z ^= b as u64;
        z = z.wrapping_mul(0x1000_0000_01b3);
    }
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    let h = z as usize;

    if is_female {
        let surname = WENYAO_SURNAMES[(h / WENYAO_FEMALE_GIVEN.len()) % WENYAO_SURNAMES.len()];
        let given = WENYAO_FEMALE_GIVEN[h % WENYAO_FEMALE_GIVEN.len()];
        format!("{surname}{given}")
    } else {
        let surname = WENYAO_SURNAMES[(h / WENYAO_MALE_GIVEN.len()) % WENYAO_SURNAMES.len()];
        let given = WENYAO_MALE_GIVEN[h % WENYAO_MALE_GIVEN.len()];
        format!("{surname}{given}")
    }
}


pub fn resolve_npc_person_name(
    entity_id: &str,
    display_title: &str,
    gender_hint: Option<&str>,
) -> String {
    let mut resolved_gender = gender_hint.map(str::to_string);
    if resolved_gender.is_none() {
        if let Some(arc) = super::get_store()
            && let Some(s) = arc.try_read()
            && let Some(ent) = s.entities.get(entity_id)
            && !ent.gender.is_empty()
        {
            resolved_gender = Some(ent.gender.clone());
        }
    }
    let is_female = npc_effective_is_female(entity_id, resolved_gender.as_deref());

    const COMPOUND_SURNAMES: &[&str] = &[
        "歐陽", "司馬", "諸葛", "上官", "司徒", "司空", "皇甫", "慕容",
        "宇文", "長孫", "聞人", "東方", "獨孤", "南宮", "萬俟", "尉遲",
    ];
    match entity_id.trim() {
        "wt_npc_ma_buxue" | "馬補靴" => return "馬守成".to_string(),
        "wt_npc_zhou_yidao" | "周一刀" => return "周敬修".to_string(),
        "wt_npc_zhao_guanxing" | "趙觀星" => return "趙衡宇".to_string(),
        "馬行頭" => return "馬振遠".to_string(),
        "田大犁" => return "田長耕".to_string(),
        _ => {}
    }
    let check_candidate = |cand: &str| -> Option<String> {
        let r = cand.trim();
        if is_role_or_placeholder_label(r) {
            return None;
        }

        let (surname, given) = if let Some(comp) = COMPOUND_SURNAMES.iter().find(|&&cs| r.starts_with(cs)) {
            (*comp, &r[comp.len()..])
        } else {
            let mut chars = r.chars();
            if let Some(first_char) = chars.next() {
                let first_len = first_char.len_utf8();
                (&r[..first_len], &r[first_len..])
            } else {
                ("", "")
            }
        };

        if surname.is_empty() || given.is_empty() {
            return None;
        }

        let is_npc_prefix = entity_id.starts_with("npc_");
        if !is_npc_prefix {
            return Some(r.to_string());
        }

        let valid_surname = WENYAO_SURNAMES.contains(&surname) || COMPOUND_SURNAMES.contains(&surname);
        if !valid_surname {
            return None;
        }
        if is_female {
            if !WENYAO_FEMALE_GIVEN.contains(&given) {
                return None;
            }
        } else if !WENYAO_MALE_GIVEN.contains(&given) {
            return None;
        }

        let given_chars: Vec<char> = given.chars().collect();
        if is_npc_prefix && given_chars.len() != 2 {
            return None;
        }
        if given_chars.len() == 2 && given_chars[0] == given_chars[1] {
            return None;
        }

        let has_male_char = given_chars.iter().any(|c| MALE_ONLY_NAME_CHARS.contains(c));
        let has_female_char = given_chars.iter().any(|c| FEMALE_ONLY_NAME_CHARS.contains(c));

        let has_neutral_male_seed = given_chars.iter().any(|c| matches!(c, '平' | '安' | '寧' | '明'));
        if has_neutral_male_seed && has_female_char {
            return None;
        }

        if has_male_char && has_female_char {
            return None;
        }
        if is_female {
            if !has_female_char || has_male_char {
                return None;
            }
        } else if has_female_char {
            return None;
        }
        Some(r.to_string())
    };

    let raw_hint = display_title.trim();
    if raw_hint.contains('|') || raw_hint.contains('｜') {
        if let Some(right) = raw_hint.rsplit(['|', '｜']).next() {
            if let Some(valid) = check_candidate(right) {
                return valid;
            }
        }
    } else if let Some(valid) = check_candidate(raw_hint) {
        return valid;
    }

    if let Some(st_title) = crate::server::grid_manager::resident_schedule_display_title(entity_id) {
        let st_title = st_title.trim();
        if st_title.contains('|') || st_title.contains('｜') {
            if let Some(right) = st_title.rsplit(['|', '｜']).next()
                && let Some(valid) = check_candidate(right)
            {
                return valid;
            }
        } else if let Some(valid) = check_candidate(st_title) {
            return valid;
        }
    }
    if let Some(arc) = super::get_store()
        && let Some(s) = arc.try_read()
        && let Some(ent) = s.entities.get(entity_id)
    {
        let st_title = ent.display_title.trim();
        if st_title.contains('|') || st_title.contains('｜') {
            if let Some(right) = st_title.rsplit(['|', '｜']).next()
                && let Some(valid) = check_candidate(right)
            {
                return valid;
            }
        } else if let Some(valid) = check_candidate(st_title) {
            return valid;
        }
    }
    if !entity_id.starts_with("npc_") && !entity_id.starts_with("wt_") {
        if let Some(valid) = check_candidate(entity_id) {
            return valid;
        }
    }
    deterministic_wenyao_full_name_with_gender(entity_id, resolved_gender.as_deref())
}

pub fn clean_wilderness_place_label(name: &str) -> String {
    let mut trimmed = name.trim();
    for suffix in ["・道路", "・山道", "・林道", "・石徑", "・步道"] {
        if let Some(stripped) = trimmed.strip_suffix(suffix) {
            trimmed = stripped;
            break;
        }
    }
    if let Some((prefix, sub)) = trimmed.split_once('・') {
        if sub.is_empty() || sub == "道路" || prefix.ends_with(sub) || (sub == "草原" && prefix.contains("草甸")) {
            prefix.to_string()
        } else {
            format!("{prefix}・{sub}")
        }
    } else {
        trimmed.to_string()
    }
}

pub fn resolve_npc_full_label(
    entity_id: &str,
    display_title: &str,
    gender_hint: Option<&str>,
) -> String {
    let person = resolve_npc_person_name(entity_id, display_title, gender_hint);
    let role_raw = npc_persistent_role_label(entity_id, display_title);
    let role_clean = role_raw
        .split(['|', '｜'])
        .next()
        .unwrap_or(&role_raw)
        .trim();
    if !role_clean.is_empty() && role_clean != person {
        format!("{role_clean}｜{person}")
    } else {
        person
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NpcDestinationTarget {
    pub x: i32,
    pub y: i32,
    pub name: &'static str,
    pub purpose: &'static str,
    pub stay_activity: &'static str,
}

#[allow(dead_code)]
pub const WENYAO_TARGETS: &[NpcDestinationTarget] = &[
    NpcDestinationTarget { x: -1, y: 2, name: "三水大食堂", purpose: "用餐休息", stay_activity: "正在木桌旁享用熱湯麵與現磨米漿" },
    NpcDestinationTarget { x: -1, y: 1, name: "三水總商行", purpose: "交割貨單", stay_activity: "正在櫃檯前核對山海貨單與鎂幣帳目" },
    NpcDestinationTarget { x: 1, y: 3, name: "印行書局", purpose: "採買書冊", stay_activity: "正在翻閱新印好的竹綿紙書冊與旬報" },
    NpcDestinationTarget { x: 1, y: -1, name: "第一鐵坊", purpose: "修整器物", stay_activity: "正在爐火旁查看鍛鐵器物與工具刃口" },
    NpcDestinationTarget { x: 2, y: 2, name: "溫家藥居", purpose: "抓藥調理", stay_activity: "正在藥櫃前等候秤量草藥" },
    NpcDestinationTarget { x: 1, y: 1, name: "文曜學院", purpose: "旁聽課程", stay_activity: "正在講堂廊下研討數學與會計與專業教材" },
    NpcDestinationTarget { x: -1, y: -1, name: "公議堂", purpose: "查閱告示", stay_activity: "正在公議堂前核對近期政令與委託" },
    NpcDestinationTarget { x: -2, y: 0, name: "三水渡埠街", purpose: "探問船期", stay_activity: "正在渡埠街邊打聽往來貨船班次" },
    NpcDestinationTarget { x: -1, y: 3, name: "乾貨海味鋪", purpose: "挑選乾貨", stay_activity: "正在挑選風乾海味與遠行乾糧" },
    NpcDestinationTarget { x: 2, y: 0, name: "茶屋巷口", purpose: "喝茶聊天", stay_activity: "正在茶屋竹椅上啜飲熱茶歇息" },
    NpcDestinationTarget { x: -2, y: 2, name: "老水夫舍", purpose: "返家歇息", stay_activity: "正在屋內整理隨身行囊與衣物" },
    NpcDestinationTarget { x: 2, y: 4, name: "研墨小齋", purpose: "閱讀資料", stay_activity: "正在案前整理筆記" },
];

pub use super::households::{WenyaoHouseholdSlot, WENYAO_HOUSEHOLD_SLOTS};

pub static HOUSEHOLD_MEMBER_ASSIGNMENTS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, usize>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn total_household_slots_len() -> usize {
    WENYAO_HOUSEHOLD_SLOTS.len() + super::outer_households::OUTER_HOUSEHOLD_SLOTS.len()
}

pub fn household_slot_at(idx: usize) -> &'static WenyaoHouseholdSlot {
    let wy_len = WENYAO_HOUSEHOLD_SLOTS.len();
    let total = total_household_slots_len();
    let i = idx % total;
    if i < wy_len {
        &WENYAO_HOUSEHOLD_SLOTS[i]
    } else {
        &super::outer_households::OUTER_HOUSEHOLD_SLOTS[i - wy_len]
    }
}

pub fn household_slot_index(slot: &WenyaoHouseholdSlot) -> usize {
    let sz = std::mem::size_of::<WenyaoHouseholdSlot>().max(1);
    let ptr = slot as *const WenyaoHouseholdSlot;
    let wy_range = WENYAO_HOUSEHOLD_SLOTS.as_ptr_range();
    if wy_range.contains(&ptr) {
        return (ptr as usize - wy_range.start as usize) / sz;
    }
    let out_range = super::outer_households::OUTER_HOUSEHOLD_SLOTS.as_ptr_range();
    if out_range.contains(&ptr) {
        return WENYAO_HOUSEHOLD_SLOTS.len() + (ptr as usize - out_range.start as usize) / sz;
    }
    0
}

pub fn normalize_household_home_settlement(home_name: &str) -> &'static str {
    const OUTER_SETTLEMENTS: &[&str] = &[
        "風舶角", "風舶嶼", "大圳頭", "半嶺鎮", "南哨", "老堰頭", "北嶺腳", "霜林口",
        "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮", "下營",
    ];
    for &s in OUTER_SETTLEMENTS {
        if home_name.starts_with(s) {
            return s;
        }
    }
    "文曜"
}

pub fn resident_macro_settlement(entity_id: &str, display_title: &str) -> Option<&'static str> {
    if entity_id.starts_with("npc_") {
        return None;
    }
    let title = if !display_title.trim().is_empty() {
        display_title
    } else {
        crate::server::grid_manager::resident_schedule_display_title(entity_id).unwrap_or("")
    };
    if title.contains("老堰頭") || title.contains("翠冠巨木") {
        return Some("老堰頭");
    }
    if title.contains("風舶角")
        || title.contains("風舶嶼")
        || title.contains("風舶海事")
        || title.contains("大船塢")
        || title.contains("北冕號")
        || title.contains("外澳領航")
    {
        return Some("風舶角");
    }
    if title.contains("大圳") || title.contains("草甸改良農具") {
        return Some("大圳頭");
    }
    if title.contains("三號坑口") {
        return Some("三號坑口");
    }
    if title.contains("半嶺") || title.contains("寒水玉切石") {
        return Some("半嶺鎮");
    }
    if title.contains("南哨") || title.contains("百茫白蘚") {
        return Some("南哨");
    }
    if title.contains("風櫃尾") {
        return Some("風櫃尾");
    }
    if title.contains("烏石鼻") {
        return Some("烏石鼻");
    }
    if title.contains("北嶺") {
        return Some("北嶺腳");
    }
    if title.contains("霜林口") {
        return Some("霜林口");
    }
    if title.contains("牛埔") || title.contains("長風草甸") {
        return Some("牛埔");
    }
    if title.contains("紅土崁") {
        return Some("紅土崁");
    }
    if title.contains("水尾寮") {
        return Some("水尾寮");
    }
    if title.contains("下營") {
        return Some("下營");
    }
    if let Some((home_room, _)) =
        crate::server::grid_manager::resident_schedule_home_and_work(entity_id)
    {
        let s = home_room.split('・').next().unwrap_or(home_room);
        let s = match s {
            "風舶嶼" | "築港頭" | "大船塢" => "風舶角",
            "北嶺腳" | "北嶺" => "北嶺腳",
            "半嶺鎮" | "半嶺" => "半嶺鎮",
            other => other,
        };
        const OUTER_SETTLEMENTS: &[&str] = &[
            "風舶角", "大圳頭", "半嶺鎮", "南哨", "老堰頭", "北嶺腳", "霜林口",
            "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮", "下營",
        ];
        for &m in OUTER_SETTLEMENTS {
            if s == m || home_room.starts_with(m) {
                return Some(m);
            }
        }
        return Some("文曜");
    }
    if crate::server::grid_manager::resident_schedule_display_title(entity_id).is_some() {
        return Some("文曜");
    }
    Some("文曜")
}

pub fn wenyao_household_slot_for(entity_id: &str) -> &'static WenyaoHouseholdSlot {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in entity_id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if !entity_id.starts_with("npc_") {
        let target_settlement = resident_macro_settlement(entity_id, "").unwrap_or("文曜");
        if target_settlement == "文曜" {
            if let Some((home_room, work_room)) =
                crate::server::grid_manager::resident_schedule_home_and_work(entity_id)
            {
                if let Some(slot) = WENYAO_HOUSEHOLD_SLOTS
                    .iter()
                    .find(|s| s.home_name == home_room && s.work_name == work_room)
                {
                    return slot;
                }
                if let Some(slot) = WENYAO_HOUSEHOLD_SLOTS
                    .iter()
                    .find(|s| s.home_name == home_room)
                {
                    return slot;
                }
            }
            return &WENYAO_HOUSEHOLD_SLOTS[(h >> 8) as usize % WENYAO_HOUSEHOLD_SLOTS.len()];
        } else {
            let matching: Vec<&'static WenyaoHouseholdSlot> =
                super::outer_households::OUTER_HOUSEHOLD_SLOTS
                    .iter()
                    .filter(|s| s.home_name.starts_with(target_settlement))
                    .collect();
            if let Some((home_room, _)) =
                crate::server::grid_manager::resident_schedule_home_and_work(entity_id)
            {
                if let Some(&slot) = matching.iter().find(|s| s.home_name == home_room) {
                    return slot;
                }
            }
            if !matching.is_empty() {
                return matching[(h >> 8) as usize % matching.len()];
            }
        }
    }
    if let Some(&idx) = HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(entity_id) {
        return household_slot_at(idx);
    }
    household_slot_at((h >> 8) as usize)
}

pub use crate::game::{
    current_game_clock_full, current_game_hour, current_game_hour_and_min,
    current_game_time_prompt_context, format_elapsed_duration_zh, format_game_calendar_stamp,
    format_game_time_prompt_context, format_kaipu_date_label, format_memory_temporal_anchor,
    format_schedule_window_sense, format_short_relative_time, game_clock_at_unix,
    game_season_label, game_time_cfg, kaipu_calendar_parts,
};

pub fn is_deprecated_npc_id(id: &str) -> bool {
    id.starts_with("npc_amb_")
        || id.starts_with("wt-physical-")
        || matches!(
            id,
            "wt_npc_chen_wanfu"
                | "wt_npc_shen_qinghe"
                | "wt_npc_bai_quansheng"
                | "wt_npc_ke_chengqi"
                | "wt_npc_su_jinxin"
        )
}

static BLDG_COORD_CACHE: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, (i32, i32)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn resolve_bldg_coord_or_fallback(bldg_name: &str, fallback: (i32, i32)) -> (i32, i32) {
    if fallback.0.abs() > 20 || fallback.1.abs() > 20 {
        return fallback;
    }
    if let Some(&coord) = BLDG_COORD_CACHE.read().get(bldg_name) {
        return coord;
    }
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid()
        && let Some(cell) =
            crate::server::grid_manager::resolve_settlement_room_coord(&grid, bldg_name)
    {
        let coord = (cell.coord.x, cell.coord.y);
        BLDG_COORD_CACHE.write().insert(bldg_name.to_string(), coord);
        return coord;
    }
    fallback
}

pub fn resolve_wenyao_npc_schedule(
    slot: &'static WenyaoHouseholdSlot,
    hour: i32,
    work_errand_toggle: bool,
    _origin_name: &str,
) -> ((i32, i32), String, String, String, u8) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in slot.attire_line.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let slot_idx = household_slot_index(slot);
    let is_wenyao_slot = slot.home_x.abs() <= 20 && slot.home_y.abs() <= 20;
    if hour >= 22 || hour < 5 {
        let is_night_watch_place = slot.work_name.contains("巡防")
            || slot.work_name.contains("哨")
            || slot.work_name.contains("關口")
            || slot.work_name.contains("門坊")
            || slot.work_name.contains("旅社")
            || slot.work_name.contains("驛館")
            || slot.work_name.contains("歇舍");
        if is_night_watch_place && (slot_idx % 6 == 0) {
            let work_coord = resolve_bldg_coord_or_fallback(slot.work_name, (slot.work_x, slot.work_y));
            return (
                work_coord,
                slot.work_name.to_string(),
                "夜間值守".to_string(),
                "正在夜間值守巡看門戶與燈火".to_string(),
                8,
            );
        }
        if is_wenyao_slot && hour == 22 && slot_idx % 12 == 0 {
            return match (slot_idx / 12) % 3 {
                0 => (
                    (0, -1),
                    "文曜・三水口南街".to_string(),
                    "夜市宵夜".to_string(),
                    "正在三水口南街宵夜攤車前趁熱品嚐現煮切仔湯麵與黑糖薑汁熱豆花".to_string(),
                    10,
                ),
                1 => (
                    (0, 0),
                    "文曜・三水匯流大石橋".to_string(),
                    "晚間散步".to_string(),
                    "正在三水匯流大石橋憑欄吹著晚風，遠眺三水川兩岸燈火".to_string(),
                    10,
                ),
                _ => (
                    resolve_bldg_coord_or_fallback("東橋旅社與山風茶屋", (2, -1)),
                    "東橋旅社與山風茶屋".to_string(),
                    "夜宿飲茶".to_string(),
                    "正在東橋旅社大廳守著溫熱銅茶壺，與夜行商旅低聲交談".to_string(),
                    12,
                ),
            };
        }
        if is_wenyao_slot && slot_idx == 0 {
            return (
                (0, 0),
                "文曜・三水匯流大石橋".to_string(),
                "夜間巡行".to_string(),
                "正提著防風提燈在三水匯流大石橋巡看三水川水勢與夜間街道".to_string(),
                12,
            );
        }
        if is_wenyao_slot && slot_idx == 15 {
            return (
                (0, -1),
                "文曜・三水口南街".to_string(),
                "夜間巡行".to_string(),
                "正沿著三水口南街巡看收攤後的街坊門鎖與水渠".to_string(),
                12,
            );
        }
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        let sleep_act = match slot_idx % 3 {
            0 => "正在內室榻上熄燈安歇",
            1 => "已在屋內床榻上熟睡養神",
            _ => "正在屋內熄燈就寢安歇",
        };
        return (
            home_coord,
            slot.home_name.to_string(),
            "返家安歇".to_string(),
            sleep_act.to_string(),
            10,
        );
    }
    if is_wenyao_slot && (18..=21).contains(&hour) && work_errand_toggle && ((h.wrapping_add(hour as u64)) % 3 == 0) {
        return match (h.wrapping_add(hour as u64)) % 18 {
            0 => (
                (0, 0),
                "三水匯流大石橋".to_string(),
                "橋頭交會".to_string(),
                "正在三水匯流大石橋燈火下與往來行商交談並眺望三水川水色".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(0)) % 11) as u8,
            ),
            1 => (
                (0, 1),
                "三水口北街".to_string(),
                "街坊巡行".to_string(),
                "正提著防風氣死風燈沿三水口北街石板道緩步穿行".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(1)) % 11) as u8,
            ),
            2 => (
                (0, -1),
                "文曜・三水口南街".to_string(),
                "夜市宵夜".to_string(),
                "正在三水口南街夜市攤位間品嚐現烤原牛肉串、貼爐胡椒餅與熱切仔麵".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(2)) % 11) as u8,
            ),
            3 => (
                (1, 0),
                "三水東街".to_string(),
                "學院夜談".to_string(),
                "正手捧書卷沿三水東街與同學好友漫步討論".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(3)) % 11) as u8,
            ),
            4 => (
                (-1, 0),
                "三水口西街".to_string(),
                "街肆散步".to_string(),
                "正沿著三水口西街石板道散步吹風，與街坊鄰居閒話家常".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(4)) % 11) as u8,
            ),
            5 => (
                resolve_bldg_coord_or_fallback("三水大食堂", (-1, 2)),
                "三水大食堂".to_string(),
                "晚間聚膳".to_string(),
                "正在三水大食堂大啖現煮原牛骨湯麵與蔥油烙餅".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(5)) % 11) as u8,
            ),
            6 => (
                resolve_bldg_coord_or_fallback("東橋旅社與山風茶屋", (2, -1)),
                "東橋旅社與山風茶屋".to_string(),
                "茶屋夜敘".to_string(),
                "正在東橋茶屋臨水窗邊品茗聽雨、暢談四方見聞".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(6)) % 11) as u8,
            ),
            7 => (
                resolve_bldg_coord_or_fallback("三水總商行總部", (-1, 1)),
                "三水總商行總部".to_string(),
                "晚盤結算".to_string(),
                "正在三水總商行燈下核對當日跨鎮通兌匯票與總帳".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(7)) % 11) as u8,
            ),
            8 => (
                resolve_bldg_coord_or_fallback("學院主校區", (1, 1)),
                "學院主校區".to_string(),
                "講堂夜讀".to_string(),
                "正在學院講堂貝母燈下抄錄會計與曆算與山海風物卷宗".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(8)) % 11) as u8,
            ),
            9 => (
                resolve_bldg_coord_or_fallback("岑氏第一鍛造工坊", (1, -1)),
                "岑氏第一鍛造工坊".to_string(),
                "守爐夜鍛".to_string(),
                "正在岑氏第一鍛造工坊紅鐵高爐旁照看夜間爐溫與火色".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(9)) % 11) as u8,
            ),
            10 => (
                resolve_bldg_coord_or_fallback("北內門坊", (0, 10)),
                "北內門坊".to_string(),
                "內坊巡望".to_string(),
                "正在北內門坊高大牌樓下檢視環城車馬路燈火與北向水道".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(10)) % 11) as u8,
            ),
            11 => (
                resolve_bldg_coord_or_fallback("南內門坊", (0, -10)),
                "南內門坊".to_string(),
                "內門會商".to_string(),
                "正在南內門坊石階前與南來車隊腳伕交接貨單印信".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(11)) % 11) as u8,
            ),
            12 => (
                resolve_bldg_coord_or_fallback("東內門坊", (10, 0)),
                "東內門坊".to_string(),
                "東坊漫步".to_string(),
                "正在東內門坊青石大道旁駐足觀看百工坊匠人運送新出爐器物".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(12)) % 11) as u8,
            ),
            13 => (
                resolve_bldg_coord_or_fallback("西內門坊", (-10, 0)),
                "西內門坊".to_string(),
                "西關點卯".to_string(),
                "正在西內門坊望樓下清點入城大車隊伍並核驗通行路引".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(13)) % 11) as u8,
            ),
            14 => (
                resolve_bldg_coord_or_fallback("三水外坊・臨江水手茶飯館", (-13, 13)),
                "三水外坊・臨江水手茶飯館".to_string(),
                "水手茶敘".to_string(),
                "正在臨江水手茶飯館靠窗木桌旁就著河風大啖熱騰騰的魚湯與麥餅".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(14)) % 11) as u8,
            ),
            15 => (
                resolve_bldg_coord_or_fallback("講堂外坊・翰墨清茶書舍", (13, 13)),
                "講堂外坊・翰墨清茶書舍".to_string(),
                "書舍清談".to_string(),
                "正在翰墨清茶書舍雅座間翻閱新印善本並與學士低聲論道".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(15)) % 11) as u8,
            ),
            16 => (
                resolve_bldg_coord_or_fallback("百工外坊・紅爐烈酒大飯鋪", (13, -13)),
                "百工外坊・紅爐烈酒大飯鋪".to_string(),
                "烈酒暖身".to_string(),
                "正在紅爐烈酒大飯鋪條凳上痛飲老燒酒、聽四方匠人高談闊論".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(16)) % 11) as u8,
            ),
            _ => (
                resolve_bldg_coord_or_fallback("西門外坊・大碗羊肉湯餅鋪", (-13, -13)),
                "西門外坊・大碗羊肉湯餅鋪".to_string(),
                "湯餅飽腹".to_string(),
                "正在大碗羊肉湯餅鋪熱氣中捧著大海碗大口喝著滾燙羊肉濃湯".to_string(),
                18 + (((h.wrapping_add(hour as u64)).wrapping_add(17)) % 11) as u8,
            ),
        };
    }
    let is_home_window = (hour >= 5 && hour < slot.morning_hour)
        || (hour >= slot.evening_hour && hour < 22);
    if is_home_window {
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        let mut home_act = if slot.home_stay_activity.is_empty() {
            "正在屋內整理器物與家務".to_string()
        } else {
            slot.home_stay_activity.to_string()
        };
        if hour >= 18 && (home_act.contains("清晨") || home_act.contains("五更") || home_act.contains("晨光")) {
            home_act = "正在屋內燈下整理器物與家務".to_string();
        }
        return (
            home_coord,
            slot.home_name.to_string(),
            "返家安歇".to_string(),
            home_act,
            20 + ((h % 11) as u8),
        );
    }
    if hour == 12 {
        let work_coord = resolve_bldg_coord_or_fallback(slot.work_name, (slot.work_x, slot.work_y));
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        if !is_wenyao_slot {
            let errand_coord = resolve_bldg_coord_or_fallback(slot.errand_name, (slot.errand_x, slot.errand_y));
            return match h % 3 {
                0 => (
                    errand_coord,
                    slot.errand_name.to_string(),
                    "午間用膳與採買".to_string(),
                    slot.errand_stay_activity.to_string(),
                    20 + ((h % 8) as u8),
                ),
                1 => (
                    work_coord,
                    slot.work_name.to_string(),
                    "午間便當小憩".to_string(),
                    format!("正在{}工位旁打開木製便當盒享用熱飯菜，順道歇口氣", slot.work_name),
                    20 + ((h % 8) as u8),
                ),
                _ => (
                    home_coord,
                    slot.home_name.to_string(),
                    "返家午膳".to_string(),
                    format!("正在{}自家飯桌旁與家人享用熱騰騰的午飯", slot.home_name),
                    20 + ((h % 8) as u8),
                ),
            };
        }
        return match h % 12 {
            0 => (
                resolve_bldg_coord_or_fallback("三水大食堂", (-1, 2)),
                "三水大食堂".to_string(),
                "用膳歇腳".to_string(),
                "正在三水大食堂木桌旁享用熱騰騰的現煮飯食與湯水".to_string(),
                20 + ((h % 8) as u8),
            ),
            1 => (
                resolve_bldg_coord_or_fallback("東橋旅社與山風茶屋", (2, -1)),
                "東橋旅社與山風茶屋".to_string(),
                "品茗用膳".to_string(),
                "正在東橋茶屋臨水座頭品茗享用糕點茶食".to_string(),
                18 + ((h % 8) as u8),
            ),
            2 => (
                (0, 1),
                "文曜・三水口北街".to_string(),
                "早市買熱食".to_string(),
                "正在三水口北街早市攤車前挑選熱騰騰的荷葉糯米炊飯與現磨溫米漿".to_string(),
                20 + ((h % 8) as u8),
            ),
            3 => (
                (0, -1),
                "文曜・三水口南街".to_string(),
                "南街買麵點".to_string(),
                "正在三水口南街熟食鋪旁享用現煮湯麵與小菜".to_string(),
                20 + ((h % 8) as u8),
            ),
            4 => (
                (2, 1),
                "講堂外坊・翰墨清茶書舍".to_string(),
                "書舍茶點".to_string(),
                "正在清茶書舍廊下就著熱茶享用午間茶點".to_string(),
                20 + ((h % 8) as u8),
            ),
            5 => (
                (1, 0),
                "文曜・三水口納涼棚廊".to_string(),
                "棚廊午憩".to_string(),
                "正在三水口納涼棚廊木椅上打開自備便當享用午餐".to_string(),
                20 + ((h % 8) as u8),
            ),
            6 => (
                (-3, -1),
                "西門・車馬聯運總站".to_string(),
                "站前茶棚".to_string(),
                "正在聯運總站茶水棚旁吃碗熱湯麵歇腳".to_string(),
                20 + ((h % 8) as u8),
            ),
            7 => (
                (0, 2),
                "文曜・鐘樓前街".to_string(),
                "鐘樓午歇".to_string(),
                "正在鐘樓前街樹蔭石凳上享用熱騰騰的酥皮胡椒餅與熱湯".to_string(),
                20 + ((h % 8) as u8),
            ),
            8 => (
                (-1, -1),
                "文曜・公議堂東街".to_string(),
                "街畔小憩".to_string(),
                "正在公議堂東街長凳旁喝著溫茶歇息".to_string(),
                20 + ((h % 8) as u8),
            ),
            9 | 10 => (
                work_coord,
                slot.work_name.to_string(),
                "午間便當小憩".to_string(),
                format!("正在{}工位旁打開木製便當盒享用熱飯菜，順道歇口氣", slot.work_name),
                20 + ((h % 8) as u8),
            ),
            _ => (
                home_coord,
                slot.home_name.to_string(),
                "返家午膳".to_string(),
                format!("正在{}自家飯桌旁享用熱騰騰的午飯", slot.home_name),
                20 + ((h % 8) as u8),
            ),
        };
    }
    if !work_errand_toggle {
        let mut work_coord = resolve_bldg_coord_or_fallback(slot.work_name, (slot.work_x, slot.work_y));
        let mut stay_activity = slot.work_stay_activity.to_string();
        let mut name = slot.work_name.to_string();
        if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
            if let Some(cell) = grid.get(crate::grid::SquareCoord::new(work_coord.0, work_coord.1)) {
                let cell_leaf = cell.name.rsplit('・').next().unwrap_or(&cell.name);
                let is_residence_query = slot.work_name.contains("宅")
                    || slot.work_name.contains("居")
                    || slot.work_name.contains("舍")
                    || slot.work_name.contains("宿")
                    || slot.work_name.contains("院")
                    || slot.work_name.contains("齋")
                    || slot.work_name.contains("廬")
                    || slot.work_name.contains("寮");
                if (cell.terrain == crate::grid::Terrain::House || cell_leaf.contains("宿舍")) && !is_residence_query {
                    work_coord = (slot.work_x, slot.work_y);
                }
                if let Some(actual_cell) = grid.get(crate::grid::SquareCoord::new(work_coord.0, work_coord.1)) {
                    let actual_leaf = actual_cell.name.rsplit('・').next().unwrap_or(&actual_cell.name);
                    if actual_leaf != slot.work_name && stay_activity.contains(slot.work_name) {
                        stay_activity = stay_activity.replace(slot.work_name, actual_leaf);
                    }
                    if actual_leaf != slot.work_name && name == slot.work_name {
                        name = actual_leaf.to_string();
                    }
                }
            }
        }
        (
            work_coord,
            name,
            slot.work_purpose.to_string(),
            stay_activity,
            20 + ((h % 11) as u8),
        )
    } else {
        let mut errand_coord =
            resolve_bldg_coord_or_fallback(slot.errand_name, (slot.errand_x, slot.errand_y));
        let mut stay_activity = slot.errand_stay_activity.to_string();
        let mut name = slot.errand_name.to_string();
        if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
            if let Some(cell) = grid.get(crate::grid::SquareCoord::new(errand_coord.0, errand_coord.1)) {
                let cell_leaf = cell.name.rsplit('・').next().unwrap_or(&cell.name);
                let is_residence_query = slot.errand_name.contains("宅")
                    || slot.errand_name.contains("居")
                    || slot.errand_name.contains("舍")
                    || slot.errand_name.contains("宿")
                    || slot.errand_name.contains("院")
                    || slot.errand_name.contains("齋")
                    || slot.errand_name.contains("廬")
                    || slot.errand_name.contains("寮");
                if (cell.terrain == crate::grid::Terrain::House || cell_leaf.contains("宿舍")) && !is_residence_query {
                    errand_coord = (slot.errand_x, slot.errand_y);
                }
                if let Some(actual_cell) = grid.get(crate::grid::SquareCoord::new(errand_coord.0, errand_coord.1)) {
                    let actual_leaf = actual_cell.name.rsplit('・').next().unwrap_or(&actual_cell.name);
                    if actual_leaf != slot.errand_name && stay_activity.contains(slot.errand_name) {
                        stay_activity = stay_activity.replace(slot.errand_name, actual_leaf);
                    }
                    if actual_leaf != slot.errand_name && name == slot.errand_name {
                        name = actual_leaf.to_string();
                    }
                }
            }
        }
        (
            errand_coord,
            name,
            slot.errand_purpose.to_string(),
            stay_activity,
            16 + ((h % 9) as u8),
        )
    }
}


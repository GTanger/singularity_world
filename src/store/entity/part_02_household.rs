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
    let trimmed = entity_id.trim();
    if !trimmed.is_empty() {
        if trimmed == "馬行頭" {
            let found = crate::server::grid_manager::schedule::iter_resident_schedules()
                .find(|def| def.id == "馬行頭" || def.id == "wt_npc_ma_buxue")
                .and_then(|def| {
                    if def.id == "馬行頭" {
                        def.display_title.split(['|', '｜']).nth(1).map(str::trim)
                    } else {
                        None
                    }
                })
                .filter(|p| !p.is_empty());
            return found.unwrap_or("馬振遠").to_string();
        }
        if let Some(person) = crate::server::grid_manager::schedule::iter_resident_schedules()
            .find(|def| {
                def.id == trimmed
                    || def.display_title.split(['|', '｜']).next() == Some(trimmed)
                    || (def.id == "wt_npc_ma_buxue" && trimmed == "馬補靴")
                    || (def.id == "wt_npc_zhou_yidao" && trimmed == "周一刀")
                    || (def.id == "wt_npc_zhao_guanxing" && trimmed == "趙觀星")
                    || (def.id == "田大犁" && trimmed == "田大犁")
            })
            .and_then(|def| def.display_title.split(['|', '｜']).nth(1).map(str::trim))
            .filter(|p| !p.is_empty())
        {
            return person.to_string();
        }
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

pub fn held_vessel(slot: &WenyaoHouseholdSlot) -> (&'static str, &'static str) {
    let spec = if !slot.work_name.is_empty() {
        let s1 = crate::npc::causal_core::resolve_vocational_vessel(slot.work_name);
        if s1.item_id != "craft_woven_basket" || slot.work_purpose.is_empty() {
            s1
        } else {
            crate::npc::causal_core::resolve_vocational_vessel(slot.work_purpose)
        }
    } else {
        crate::npc::causal_core::resolve_vocational_vessel(slot.work_purpose)
    };
    if let Some(cat) = crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS
        .iter()
        .find(|o| o.id == spec.catalog_id)
    {
        (cat.id, cat.name)
    } else {
        (spec.catalog_id, spec.display_name)
    }
}

pub fn synthesize_grounded_venue_activity(
    place_name: &str,
    fallback_act: &str,
    vessel_name: &str,
    seed: u64,
) -> String {
    let place = place_name.trim();
    if place.is_empty() {
        return fallback_act.to_string();
    }
    let venue_opt = crate::grid::resolve_any_room_venue_object(place, "");
    let fixture_name = match &venue_opt {
        Some(v) => {
            let k = v.venue_kind;
            if k.contains("鍛造") || k.contains("鐵") {
                "紅鐵高爐與砧台"
            } else if k.contains("醫館") || k.contains("藥") || k.contains("診所") {
                "百草藥櫃與煎藥爐"
            } else if k.contains("食堂") || k.contains("飯") || k.contains("食") {
                "大骨湯鍋與白案蒸籠"
            } else if k.contains("茶") {
                "老茶壺與臨窗茶案"
            } else if k.contains("浴") || k.contains("溫水") {
                "溫熱石砌泉池"
            } else if k.contains("木作") || k.contains("傢俱") {
                "木作刨台與工具架"
            } else if k.contains("書") || k.contains("學") || k.contains("閣") {
                "書案與善本卷宗架"
            } else if k.contains("商行") || k.contains("契書") || k.contains("公議") {
                "臨街櫃檯與帳冊案几"
            } else {
                "場地櫃檯與常備長凳"
            }
        }
        None => {
            if place.contains("橋") {
                "青石雕花欄杆"
            } else if place.contains("街") || place.contains("巷") {
                "街角路燈與避風簷廊"
            } else if place.contains("食堂") || place.contains("飯") {
                "大骨湯鍋與白案蒸籠"
            } else if place.contains("茶") {
                "老茶壺與臨窗茶案"
            } else {
                "歇腳長凳"
            }
        }
    };

    if !fallback_act.is_empty()
        && (fallback_act.contains(place) || fallback_act.contains(fixture_name))
    {
        return fallback_act.to_string();
    }

    if let Some(v) = &venue_opt {
        let amb_opt = crate::grid::resolve_room_ambient_visitor_activity(place, (seed % 13) as usize);
        if let Some(amb) = amb_opt {
            return format!("正在{}，{}", place, amb);
        }
        let v_act = v.on_duty_fixture_activity.trim();
        if !v_act.is_empty() && (seed % 2 == 0) {
            return format!("正在{}，{}", place, v_act);
        }
        if !vessel_name.is_empty() && (seed % 4 == 0) {
            return format!("正在{}的{}旁，隨身收好【{}】並歇息歇腳", place, fixture_name, vessel_name);
        }
        return format!("正在{}的{}旁歇腳整理，與鄰座街坊閒話家常", place, fixture_name);
    }

    if !fallback_act.is_empty() {
        return fallback_act.to_string();
    }
    format!("正在{}的{}旁歇腳休憩", place, fixture_name)
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
    NpcDestinationTarget { x: -1, y: 1, name: "三水總商行總部", purpose: "交割貨單", stay_activity: "正在櫃檯前核對山海貨單與鎂幣帳目" },
    NpcDestinationTarget { x: 1, y: 4, name: "講堂街書局與印行館", purpose: "採買書冊", stay_activity: "正在翻閱新印好的竹綿紙書冊與旬報" },
    NpcDestinationTarget { x: 2, y: -4, name: "岑氏第一鍛造工坊", purpose: "修整器物", stay_activity: "正在爐火旁查看鍛鐵器物與工具刃口" },
    NpcDestinationTarget { x: 1, y: -4, name: "溫氏脈絡診所", purpose: "抓藥調理", stay_activity: "正在藥櫃前等候秤量草藥" },
    NpcDestinationTarget { x: 1, y: 1, name: "學院主校區", purpose: "旁聽課程", stay_activity: "正在講堂廊下研討數學與會計與專業教材" },
    NpcDestinationTarget { x: -1, y: -1, name: "公議堂", purpose: "查閱告示", stay_activity: "正在公議堂前核對近期政令與委託" },
    NpcDestinationTarget { x: -2, y: 0, name: "三水渡埠街", purpose: "探問船期", stay_activity: "正在渡埠街邊打聽往來貨船班次" },
    NpcDestinationTarget { x: -1, y: 4, name: "三水口乾貨海味行", purpose: "挑選乾貨", stay_activity: "正在挑選風乾海味與遠行乾糧" },
    NpcDestinationTarget { x: 2, y: -1, name: "東橋旅社與山風茶屋", purpose: "喝茶聊天", stay_activity: "正在茶屋竹椅上啜飲熱茶歇息" },
    NpcDestinationTarget { x: -4, y: 6, name: "三水口・老水夫舍", purpose: "返家歇息", stay_activity: "正在屋內整理隨身行囊與衣物" },
    NpcDestinationTarget { x: 4, y: 7, name: "講堂街・研墨小齋", purpose: "閱讀資料", stay_activity: "正在案前整理筆記" },
];

pub use super::households::{WenyaoHouseholdSlot, WENYAO_HOUSEHOLD_SLOTS};

pub static HOUSEHOLD_MEMBER_ASSIGNMENTS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, usize>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
pub static HOUSEHOLD_NAME_TO_SLOT_IDX: std::sync::LazyLock<
    std::collections::HashMap<String, usize>,
> = std::sync::LazyLock::new(|| {
    let total = total_household_slots_len();
    let mut map = std::collections::HashMap::with_capacity(total);
    for idx in 0..total {
        let slot = household_slot_at(idx);
        let name = slot.name();
        if !name.is_empty() {
            map.entry(name.to_string()).or_insert(idx);
        }
    }
    map
});

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
    if let Some(&idx) = HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(entity_id) {
        return Some(normalize_household_home_settlement(household_slot_at(idx).home_name));
    }
    if let Some(&idx) = HOUSEHOLD_NAME_TO_SLOT_IDX.get(entity_id) {
        return Some(normalize_household_home_settlement(household_slot_at(idx).home_name));
    }
    if crate::server::grid_manager::resident_schedule_display_title(entity_id).is_some() {
        return Some("文曜");
    }
    Some("文曜")
}

pub fn wenyao_household_slot_for(entity_id: &str) -> &'static WenyaoHouseholdSlot {
    if entity_id == "npc_m3_029_pack_runner" {
        return &WENYAO_HOUSEHOLD_SLOTS[92];
    }
    if let Some(&idx) = HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(entity_id) {
        return household_slot_at(idx);
    }
    if let Some(&idx) = HOUSEHOLD_NAME_TO_SLOT_IDX.get(entity_id) {
        return household_slot_at(idx);
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in entity_id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
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
    if hour >= 21 || hour < 5 {
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
            0 => "正在內室榻上熄燈就寢",
            1 => "已在屋內床榻上熟睡養神",
            _ => "正在屋內熄燈就寢休息",
        };
        return (
            home_coord,
            slot.home_name.to_string(),
            "返家歇息".to_string(),
            sleep_act.to_string(),
            10,
        );
    }
    if (18..=19).contains(&hour)
        && hour >= slot.evening_hour
        && work_errand_toggle
        && ((h.wrapping_add(hour as u64)) % 3 == 0)
    {
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
        let target_tags = &["teahouse", "canteen", "bathhouse"];
        let found_venue = grid_opt
            .as_ref()
            .and_then(|g| g.find_nearest_room_by_tags(home_coord, 16, target_tags, h));

        let (dest_coord, dest_name, stay_act) = if let Some((c, n)) = found_venue {
            let act = synthesize_grounded_venue_activity(
                &n,
                "",
                held_vessel(slot).1,
                h.wrapping_add(hour as u64),
            );
            (c, n, act)
        } else if is_wenyao_slot {
            let fallback_venues = [
                ((2, -1), "東橋旅社與山風茶屋"),
                ((-1, 2), "三水大食堂"),
                ((-4, 1), "泉香溫水浴場"),
            ];
            let fallback_choice = fallback_venues[(h as usize) % fallback_venues.len()];
            let n = fallback_choice.1.to_string();
            let act = synthesize_grounded_venue_activity(
                &n,
                "",
                held_vessel(slot).1,
                h.wrapping_add(hour as u64),
            );
            (fallback_choice.0, n, act)
        } else {
            let act = if slot.home_stay_activity.is_empty() {
                "正在屋內整理器物與家務".to_string()
            } else {
                slot.home_stay_activity.to_string()
            };
            (home_coord, slot.home_name.to_string(), act)
        };

        let ticks = 18 + (((h.wrapping_add(hour as u64)) % 11) as u8);
        return (dest_coord, dest_name, "晚間休歇".to_string(), stay_act, ticks);
    }
    if hour >= 5 && hour < slot.morning_hour {
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        let home_leaf = slot.home_name.rsplit('・').next().unwrap_or(slot.home_name);
        let work_leaf = slot.work_name.rsplit('・').next().unwrap_or(slot.work_name);
        let morning_act = format!("正在{home_leaf}屋內晨起洗漱、用過早膳，準備動身前往{work_leaf}上工");
        return (
            home_coord,
            slot.home_name.to_string(),
            "晨起備工".to_string(),
            morning_act,
            15 + ((h % 9) as u8),
        );
    }
    let is_evening_rest_window = (hour >= slot.evening_hour && hour < 21) || hour == 20;
    if is_evening_rest_window {
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
            "返家歇息".to_string(),
            home_act,
            20 + ((h % 11) as u8),
        );
    }
    if hour == 12 {
        let work_coord = resolve_bldg_coord_or_fallback(slot.work_name, (slot.work_x, slot.work_y));
        let home_coord = resolve_bldg_coord_or_fallback(slot.home_name, (slot.home_x, slot.home_y));
        let errand_coord = resolve_bldg_coord_or_fallback(slot.errand_name, (slot.errand_x, slot.errand_y));
        if h % 5 == 0 {
            return (
                errand_coord,
                slot.errand_name.to_string(),
                "午間跑腿採買".to_string(),
                slot.errand_stay_activity.to_string(),
                20 + ((h % 8) as u8),
            );
        }

        let grid_opt = crate::server::grid_manager::get_runtime_square_grid();
        let found_venue = grid_opt
            .as_ref()
            .and_then(|g| g.find_nearest_room_by_tags(work_coord, 16, &["canteen", "hot_food", "teahouse"], h));

        let (dest_coord, dest_name, stay_act) = if let Some((c, n)) = found_venue {
            let venue_fixture = match crate::grid::resolve_any_room_venue_object(&n, "") {
                Some(v) => {
                    let k = v.venue_kind;
                    if k.contains("食") || k.contains("飯") || k.contains("食堂") {
                        "大骨湯鍋與白案蒸籠"
                    } else if k.contains("茶") {
                        "老茶壺與臨窗茶案"
                    } else {
                        "木製餐桌與長凳"
                    }
                }
                None => {
                    if n.contains("食") || n.contains("飯") {
                        "大骨湯鍋與白案蒸籠"
                    } else if n.contains("茶") {
                        "老茶壺與臨窗茶案"
                    } else {
                        "木製餐桌與長凳"
                    }
                }
            };
            let act = format!("正在{}的{}旁享用熱騰騰的現煮飯食與湯水，歇腳小憩", n, venue_fixture);
            (c, n, act)
        } else if is_wenyao_slot {
            let n = "三水大食堂".to_string();
            let venue_fixture = "大骨湯鍋與白案蒸籠";
            let act = format!("正在{}的{}旁享用熱騰騰的現煮飯食與湯水，歇腳小憩", n, venue_fixture);
            ((-1, 2), n, act)
        } else {
            let act = synthesize_grounded_venue_activity(
                slot.home_name,
                &slot.home_stay_activity,
                held_vessel(slot).1,
                h.wrapping_add(hour as u64),
            );
            (home_coord, slot.home_name.to_string(), act)
        };

        return (
            dest_coord,
            dest_name,
            "午間聚膳".to_string(),
            stay_act,
            20 + ((h % 8) as u8),
        );
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
        let grounded_act = synthesize_grounded_venue_activity(&name, &stay_activity, held_vessel(slot).1, h);
        (
            work_coord,
            name,
            slot.work_purpose.to_string(),
            grounded_act,
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
        let grounded_act = synthesize_grounded_venue_activity(&name, &stay_activity, held_vessel(slot).1, h);
        (
            errand_coord,
            name,
            slot.errand_purpose.to_string(),
            grounded_act,
            16 + ((h % 9) as u8),
        )
    }
}


//! store 實體 — entities + entity_rooms 的讀寫、座標綁定、轉帳。
//! 寫入路徑走 writer queue（PG 權威）+ 必要時 in-memory mirror；entity_rooms 不再寫 JSON。

use super::{Entity, Store};

#[derive(Debug, Clone)]
pub struct NpcGridState {
    pub current: (i32, i32),
    pub dest: (i32, i32),
    pub dest_name: String,
    pub origin_name: String,
    pub stay_activity: String,
    pub purpose: String,
    pub stay_ticks_remaining: u8,
    pub work_errand_toggle: bool,
    pub consecutive_work_streaks: u8,
}

impl NpcGridState {
    pub fn current_activity(&self) -> String {
        let hour = current_game_hour();
        if self.current == self.dest {
            self.stay_activity.clone()
        } else if (self.purpose == "安歇養神" || self.purpose == "返家安歇" || self.purpose == "夜間返家") {
            if hour >= 20 || hour < 5 {
                format!("正踏著夜色沿街返回【{}】準備安歇", self.dest_name)
            } else {
                format!("正沿街返回【{}】稍作歇息", self.dest_name)
            }
        } else if hour >= 22 || hour < 5 {
            format!("正踏著夜色沿街返回【{}】準備安歇", self.dest_name)
        } else if self.purpose == "享用熱食" || self.purpose == "品茗用膳" || self.purpose == "洗沐放鬆" {
            format!("剛收工，正沿街前往【{}】{}", self.dest_name, self.purpose)
        } else if !self.purpose.is_empty() {
            format!("正沿街前往【{}】{}", self.dest_name, self.purpose)
        } else {
            format!("正沿街前往【{}】", self.dest_name)
        }
    }
}

pub static LIVE_NPC_GRID_STATES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, NpcGridState>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub static LIVE_NPC_OBLIGATIONS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, (String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn get_active_npc_obligation(entity_id: &str) -> Option<(String, String)> {
    LIVE_NPC_OBLIGATIONS.read().get(entity_id).cloned()
}

pub fn open_active_npc_obligation(entity_id: &str, obligation_kind: &str, subject_ref: &str) {
    LIVE_NPC_OBLIGATIONS.write().insert(
        entity_id.to_string(),
        (obligation_kind.to_string(), subject_ref.to_string()),
    );
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::open_obligation(
            &pool,
            entity_id,
            obligation_kind,
            subject_ref,
            None,
            "runtime:grid_flow",
            crate::game::now_unix(),
        );
    }
}

pub fn clear_active_npc_obligation(entity_id: &str, obligation_kind: &str) {
    {
        let mut map = LIVE_NPC_OBLIGATIONS.write();
        if let Some((k, _)) = map.get(entity_id) {
            if obligation_kind.is_empty() || k == obligation_kind {
                map.remove(entity_id);
            }
        }
    }
    if let Some(pool) = crate::pg::pool() {
        let _ = crate::pg::living_world::resolve_obligations_by_kind(
            &pool,
            entity_id,
            obligation_kind,
            crate::game::now_unix(),
        );
    }
}

static NPC_INTERACTION_PAUSE_UNTIL: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, i64>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn pause_npc_for_interaction(entity_id: &str, seconds: i64) {
    let until = crate::game::now_unix().saturating_add(seconds.max(1));
    NPC_INTERACTION_PAUSE_UNTIL
        .write()
        .insert(entity_id.to_string(), until);
}

pub fn unpause_npc_for_interaction(entity_id: &str) {
    NPC_INTERACTION_PAUSE_UNTIL.write().remove(entity_id);
}

pub fn is_npc_paused_for_interaction(entity_id: &str) -> bool {
    let now = crate::game::now_unix();
    if let Some(&until) = NPC_INTERACTION_PAUSE_UNTIL.read().get(entity_id) {
        return now < until;
    }
    false
}

#[derive(Debug, Clone)]
pub struct ActiveNpcDialogue {
    pub room_id: String,
    pub npc_a: String,
    pub npc_b: String,
    pub text: String,
    pub updated_at: i64,
}

static ACTIVE_NPC_DIALOGUES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, ActiveNpcDialogue>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn record_active_npc_dialogue(room_id: &str, npc_a: &str, npc_b: &str, text: &str) {
    let dialogue = ActiveNpcDialogue {
        room_id: room_id.to_string(),
        npc_a: npc_a.to_string(),
        npc_b: npc_b.to_string(),
        text: text.to_string(),
        updated_at: crate::game::now_unix(),
    };
    let mut map = ACTIVE_NPC_DIALOGUES.write();
    map.insert(npc_a.to_string(), dialogue.clone());
    map.insert(npc_b.to_string(), dialogue);
}

pub fn get_active_npc_dialogue(entity_id: &str) -> Option<ActiveNpcDialogue> {
    let now = crate::game::now_unix();
    let map = ACTIVE_NPC_DIALOGUES.read();
    if let Some(entry) = map.get(entity_id) {
        if now.saturating_sub(entry.updated_at) <= 180 {
            return Some(entry.clone());
        }
    }
    None
}

pub const PERSISTENT_NPC_ROLES: [&str; 24] = [
    "三水船塢搬運員",
    "西門車隊貨運員",
    "第一鍛造工坊技師",
    "文曜學院學生",
    "溫氏診所護理員",
    "三水商行外務員",
    "東橋茶屋常客",
    "海產乾貨商",
    "公議堂紀錄員",
    "分家巷木工技師",
    "藏書閣編輯助理",
    "百工街鐵工",
    "北門巡守員",
    "三水食堂廚務員",
    "講堂街研究員",
    "織造坊裁縫師",
    "石橋攤商",
    "文曜國小助教",
    "南街藥材商",
    "鐘樓報時員",
    "泉香浴場服務員",
    "西門修配工",
    "三水川船務員",
    "外地旅客",
];

const SPECIFIC_ROLE_KEYWORDS: &[&str] = &[
    "掌櫃", "主事", "師傅", "教習", "學徒", "醫師", "舵手", "船夫", "鐵匠",
    "帳房", "巡衛", "茶博士", "庖師", "腳夫", "會長", "主編", "大匠", "匠師",
    "教務", "站長", "船長", "理事", "主任", "隊長", "嚮導", "大廚", "場主",
    "木匠", "夥計", "侍者", "工匠", "坊主", "教頭", "行首", "堂主", "館主",
    "衛士", "書生", "學子", "老師", "教師", "助教", "刻字", "雜役", "茶客", "行商", "貨郎", "散客",
    "員", "技師", "學生", "常客", "助理", "鐵工", "攤商", "裁縫師", "旅客", "商旅", "文吏", "客商",
];

pub fn npc_persistent_role_label(entity_id: &str, display_title: &str) -> String {
    let trimmed = display_title.trim();
    if !trimmed.is_empty() {
        if trimmed.contains('|') || trimmed.contains('｜') {
            return trimmed.to_string();
        }
        if SPECIFIC_ROLE_KEYWORDS.iter().any(|kw| trimmed.contains(kw)) {
            return trimmed.to_string();
        }
    }

    let slot = wenyao_household_slot_for(entity_id);
    if slot.attire_line.contains("磨刀") || slot.work_purpose.contains("廚刀") {
        return "三水流動磨刀匠".to_string();
    }
    if slot.work_name.contains("鐵坊") || slot.work_name.contains("鍛造") {
        return "岑氏鐵坊鍛造匠師".to_string();
    }
    if slot.work_name.contains("木") || slot.work_name.contains("傢俱") {
        return "柯氏木作匠師".to_string();
    }
    if slot.work_name.contains("印行") || slot.work_name.contains("書局") {
        return "印行館校勘與印務執事".to_string();
    }
    if slot.work_name.contains("學院") || slot.work_name.contains("講堂") || slot.work_name.contains("國小") {
        return "文曜學院助教".to_string();
    }
    if slot.work_name.contains("藥") || slot.work_name.contains("診所") || slot.work_name.contains("脈絡") {
        return "三水醫藥司藥師".to_string();
    }

    // 剝離城市/行政區域前綴，使職銜簡練自然
    let mut clean_work = slot.work_name.to_string();
    for prefix in &[
        "文曜・",
        "三水外坊・",
        "講堂外坊・",
        "百工外坊・",
        "西門外坊・",
        "三水口・",
        "西門・",
        "北門・",
        "南門・",
        "東門・",
    ] {
        if let Some(stripped) = clean_work.strip_prefix(prefix) {
            clean_work = stripped.to_string();
        }
    }

    // 特定天然角色職銜判定
    if clean_work.contains("巡河") || clean_work.contains("哨所") || clean_work.contains("望樓") {
        if clean_work.ends_with("哨所") {
            let base = clean_work.trim_end_matches("哨所");
            if base.contains("巡河") {
                return format!("{base}哨官");
            } else {
                return format!("{base}巡河哨官");
            }
        }
        return format!("{clean_work}巡防執事");
    }
    if clean_work.contains("碼頭") || clean_work.contains("渡") || clean_work.contains("埠") {
        return format!("{clean_work}船務執事");
    }
    if clean_work.contains("食堂") || clean_work.contains("茶") || clean_work.contains("酒") || clean_work.contains("麵") {
        return format!("{clean_work}掌事");
    }
    if clean_work.contains("商行") || clean_work.contains("帳房") || clean_work.contains("公議") {
        return format!("{clean_work}管事");
    }
    format!("{clean_work}執事")
}

pub static KNOWN_NPC_NAMES: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashSet<(String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashSet::new()));

pub static OBSERVED_NPCS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashSet<(String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashSet::new()));

pub fn mark_player_knows_npc(player_id: &str, npc_id: &str) {
    if player_id.is_empty() || npc_id.is_empty() {
        return;
    }
    KNOWN_NPC_NAMES
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
    OBSERVED_NPCS
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
}

pub fn mark_player_observed_npc(player_id: &str, npc_id: &str) {
    if player_id.is_empty() || npc_id.is_empty() {
        return;
    }
    OBSERVED_NPCS
        .write()
        .insert((player_id.to_string(), npc_id.to_string()));
}

pub fn player_knows_npc(player_id: &str, npc_id: &str) -> bool {
    if player_id.is_empty() || npc_id.is_empty() {
        return false;
    }
    KNOWN_NPC_NAMES
        .read()
        .contains(&(player_id.to_string(), npc_id.to_string()))
}

pub fn player_observed_npc(player_id: &str, npc_id: &str) -> bool {
    if player_id.is_empty() || npc_id.is_empty() {
        return false;
    }
    OBSERVED_NPCS
        .read()
        .contains(&(player_id.to_string(), npc_id.to_string()))
        || KNOWN_NPC_NAMES
            .read()
            .contains(&(player_id.to_string(), npc_id.to_string()))
}

pub fn is_role_or_placeholder_label(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty()
        || t.starts_with("npc_")
        || t.starts_with("wt_npc_")
        || t.chars().all(|c| c.is_ascii())
        || t.contains("陌生人")
        || t.contains("行人")
        || t.contains("眼前之人")
        || t.contains('·')
        || t.contains('・')
    {
        return true;
    }
    const COMPOUND_SURNAMES: &[&str] = &[
        "歐陽", "司馬", "諸葛", "上官", "司徒", "司空", "皇甫", "慕容", "宇文", "長孫", "聞人",
        "東方", "獨孤", "南宮", "萬俟", "尉遲",
    ];
    let char_count = t.chars().count();
    if char_count >= 4 && !COMPOUND_SURNAMES.iter().any(|&cs| t.starts_with(cs)) {
        return true;
    }
    const ROLE_MARKERS: &[&str] = &[
        "掌櫃", "主事", "師傅", "教習", "學徒", "醫師", "舵手", "船夫", "鐵匠", "帳房", "巡衛",
        "茶博士", "庖師", "腳夫", "會長", "主編", "大匠", "匠師", "教務", "站長", "船長", "理事",
        "主任", "隊長", "嚮導", "大廚", "場主", "木匠", "夥計", "侍者", "工匠", "坊主", "教頭",
        "行首", "堂主", "館主", "衛士", "書生", "學子", "老師", "教師", "助教", "刻字", "雜役", "茶客", "行商",
        "貨郎", "散客", "住戶", "儒生", "馱夫", "藥童", "巡丁", "居民", "旅人", "食客", "農夫",
        "漁夫", "信使", "錄事", "門生", "大嫂", "伴當", "門房", "船戶", "渡客", "候渡者", "搬魚人",
        "補網人", "補帆人", "海關吏", "商館人", "跑堂", "幫廚", "郎中", "大夫", "醫者", "鏢師",
        "武師", "挑夫", "車夫", "馬夫", "樵夫", "女工", "記帳員", "手藝人", "採藥人",
        "員", "技師", "學生", "常客", "助理", "鐵工", "攤商", "裁縫師", "旅客", "商旅", "文吏", "客商",
    ];
    ROLE_MARKERS.iter().any(|&kw| t.contains(kw))
}

pub fn deterministic_wenyao_full_name(npc_id: &str) -> String {
    deterministic_wenyao_full_name_with_gender(npc_id, None)
}

pub fn deterministic_wenyao_full_name_with_gender(
    npc_id: &str,
    gender_hint: Option<&str>,
) -> String {
    let mut resolved_gender = gender_hint.map(str::to_string);
    if resolved_gender.is_none() {
        if let Some(arc) = super::get_store() {
            let s = arc.read();
            if let Some(ent) = s.get_entity(npc_id) {
                if !ent.gender.is_empty() {
                    resolved_gender = Some(ent.gender.clone());
                }
            }
        }
    }

    let slot = wenyao_household_slot_for(npc_id);
    let slot_female_clue = slot.attire_line.contains("裙")
        || slot.attire_line.contains("簪")
        || slot.attire_line.contains("髮髻")
        || slot.brief_tag.contains("裙")
        || slot.work_purpose.contains("女")
        || slot.work_stay_activity.contains("女")
        || slot.home_stay_activity.contains("女")
        || slot.attire_line.contains("娘")
        || slot.attire_line.contains("小姐")
        || slot.attire_line.contains("婦");

    let is_female = if slot_female_clue {
        true
    } else {
        match resolved_gender.as_deref() {
            Some("F") | Some("f") | Some("女") | Some("female") => true,
            Some("M") | Some("m") | Some("男") | Some("male") => false,
            _ => false,
        }
    };

    const SURNAMES: &[&str] = &[
        "陳", "林", "黃", "張", "李", "王", "吳", "劉", "蔡", "楊",
        "許", "鄭", "謝", "洪", "郭", "邱", "曾", "廖", "賴", "徐",
        "周", "葉", "蘇", "莊", "江", "呂", "何", "羅", "高", "蕭",
        "柯", "溫", "秦", "顧", "沈", "方", "岑", "陸", "白", "韓",
    ];
    const FEMALE_GIVEN: &[&str] = &[
        "雅筑", "佳穎", "怡君", "詩涵", "婷萱", "語彤", "涵晴", "宥蓁",
        "心怡", "婉婷", "淑芬", "筱涵", "姿妤", "采潔", "郁婷", "馨儀",
    ];
    const MALE_GIVEN: &[&str] = &[
        "冠廷", "柏翰", "承恩", "家瑋", "宇翔", "哲維", "建宏", "俊傑",
        "志豪", "宥辰", "敬堯", "浩然", "子軒", "廷恩", "振宇", "彥廷",
    ];

    let mut h: usize = 0xcbf2_9ce4_8422_2325;
    for b in npc_id.bytes() {
        h = h.wrapping_mul(31).wrapping_add(b as usize);
    }

    let surname = SURNAMES[(h >> 8) % SURNAMES.len()];
    if is_female {
        let given = FEMALE_GIVEN[h % FEMALE_GIVEN.len()];
        format!("{surname}{given}")
    } else {
        let given = MALE_GIVEN[h % MALE_GIVEN.len()];
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
        if let Some(arc) = super::get_store() {
            let s = arc.read();
            if let Some(ent) = s.get_entity(entity_id) {
                if !ent.gender.is_empty() {
                    resolved_gender = Some(ent.gender.clone());
                }
            }
        }
    }
    let slot = wenyao_household_slot_for(entity_id);
    let slot_female_clue = slot.attire_line.contains("裙")
        || slot.attire_line.contains("簪")
        || slot.attire_line.contains("髮髻")
        || slot.brief_tag.contains("裙")
        || slot.work_purpose.contains("女")
        || slot.work_stay_activity.contains("女")
        || slot.home_stay_activity.contains("女")
        || slot.attire_line.contains("娘")
        || slot.attire_line.contains("小姐")
        || slot.attire_line.contains("婦");

    let is_female = if slot_female_clue {
        true
    } else {
        match resolved_gender.as_deref() {
            Some("F") | Some("f") | Some("女") | Some("female") => true,
            Some("M") | Some("m") | Some("男") | Some("male") => false,
            _ => false,
        }
    };

    let check_candidate = |cand: &str| -> Option<String> {
        let r = cand.trim();
        if is_role_or_placeholder_label(r) {
            return None;
        }
        if is_female && (r.contains("安勇") || r.contains("勇") || r.contains("剛") || r.contains("強") || r.contains("豪")) {
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

    if let Some(arc) = super::get_store() {
        let s = arc.read();
        if let Some(ent) = s.get_entity(entity_id) {
            let st_title = ent.display_title.trim();
            if st_title.contains('|') || st_title.contains('｜') {
                if let Some(right) = st_title.rsplit(['|', '｜']).next() {
                    if let Some(valid) = check_candidate(right) {
                        return valid;
                    }
                }
            } else if let Some(valid) = check_candidate(st_title) {
                return valid;
            }
        }
    }

    deterministic_wenyao_full_name_with_gender(entity_id, resolved_gender.as_deref())
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
pub struct NpcDestinationTarget {
    pub x: i32,
    pub y: i32,
    pub name: &'static str,
    pub purpose: &'static str,
    pub stay_activity: &'static str,
}

pub const WENYAO_TARGETS: &[NpcDestinationTarget] = &[
    NpcDestinationTarget { x: -1, y: 2, name: "三水大食堂", purpose: "用餐休息", stay_activity: "正在木桌旁享用熱湯麵與現磨米漿" },
    NpcDestinationTarget { x: -1, y: 1, name: "三水總商行", purpose: "交割貨單", stay_activity: "正在櫃檯前核對山海貨單與鎂幣帳目" },
    NpcDestinationTarget { x: 1, y: 3, name: "印行書局", purpose: "採買書冊", stay_activity: "正在翻閱剛印好的竹綿紙書冊與旬報" },
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

pub fn wenyao_household_slot_for(entity_id: &str) -> &'static WenyaoHouseholdSlot {
    if let Some(&idx) = HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(entity_id) {
        return &WENYAO_HOUSEHOLD_SLOTS[idx % WENYAO_HOUSEHOLD_SLOTS.len()];
    }
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in entity_id.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let idx = ((h >> 8) as usize) % WENYAO_HOUSEHOLD_SLOTS.len();
    &WENYAO_HOUSEHOLD_SLOTS[idx]
}

pub fn current_game_hour() -> i32 {
    static TIME_CFG: std::sync::OnceLock<(i64, f64)> = std::sync::OnceLock::new();
    let &(epoch, scale) = TIME_CFG.get_or_init(|| {
        let cfg = crate::config::Server::load_default();
        (cfg.game_time_epoch_unix, cfg.game_time_scale)
    });
    let now = crate::game::now_unix();
    let (_, h, _, _) = crate::game::game_time_now(now, epoch, scale);
    h
}

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
    let slot_idx = {
        let base = WENYAO_HOUSEHOLD_SLOTS.as_ptr() as usize;
        let curr = slot as *const _ as usize;
        if curr >= base && std::mem::size_of::<WenyaoHouseholdSlot>() > 0 {
            (curr - base) / std::mem::size_of::<WenyaoHouseholdSlot>()
        } else {
            h as usize
        }
    };
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
        if hour == 22 && slot_idx % 12 == 0 {
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
                    "文曜・三水口中心樞紐石橋".to_string(),
                    "晚間散步".to_string(),
                    "正在中心樞紐石橋憑欄吹著晚風，遠眺三水川兩岸燈火".to_string(),
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
        if slot_idx == 0 {
            return (
                (0, 0),
                "文曜・三水口中心樞紐石橋".to_string(),
                "夜間巡行".to_string(),
                "正提著防風提燈在中心樞紐石橋巡看三水川水勢與夜間街道".to_string(),
                12,
            );
        }
        if slot_idx == 15 {
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
    if (18..=21).contains(&hour) && work_errand_toggle && ((h.wrapping_add(hour as u64)) % 3 == 0) {
        return match (h.wrapping_add(hour as u64)) % 18 {
            0 => (
                (0, 0),
                "中心樞紐石橋".to_string(),
                "橋頭交會".to_string(),
                "正在中心樞紐石橋燈火下與往來行商交談並眺望三水川水色".to_string(),
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
                "正在三水總商行燈下核對當日跨鎮通兌銀票與總帳".to_string(),
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
                resolve_bldg_coord_or_fallback("講堂外坊・翰墨清茶書肆", (13, 13)),
                "講堂外坊・翰墨清茶書肆".to_string(),
                "書肆清談".to_string(),
                "正在翰墨清茶書肆雅座間翻閱新印善本並與學士低聲論道".to_string(),
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
                "講堂外坊・翰墨清茶書肆".to_string(),
                "書肆茶點".to_string(),
                "正在清茶書肆廊下就著熱茶享用午間茶點".to_string(),
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
                "正在鐘樓前街樹蔭石凳上享用剛買的酥皮胡椒餅與熱湯".to_string(),
                20 + ((h % 8) as u8),
            ),
            8 => (
                (-1, -1),
                "文曜・公議堂前街".to_string(),
                "街畔小憩".to_string(),
                "正在公議堂前街長凳旁喝著溫茶歇息".to_string(),
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

pub fn npc_sensory_appearance_line(entity_id: &str, display_title: &str) -> String {
    let title = display_title.trim();
    if title.contains("陸知微") || entity_id == "wt_npc_lu_zhiwei" {
        return "身著深青色修身長外套，下擺帶著簡約水紋暗飾，鬚髮微霜，眼神清澈而深邃".to_string();
    }
    if title.contains("陸承禮") || entity_id == "wt_npc_lu_chengli" {
        return "穿著沉穩的墨色立領外套，束著精緻皮帶，氣度雍容，舉止端方".to_string();
    }
    if title.contains("陸長貴") || entity_id == "wt_npc_lu_changgui" {
        return "身著端整莊重的深青色公務外套，隨身佩帶印章盒與公文夾，神色嚴謹".to_string();
    }
    if title.contains("陸秉衡") || entity_id == "wt_npc_lu_bingheng" {
        return "身穿寶藍色正裝外套，腰間掛著隨身懷錶與核算夾，指尖圓潤，氣度沉著圓融".to_string();
    }
    if title.contains("岑守岳") || entity_id == "wt_npc_cen_shouyue" {
        return "身形如鐵塔般魁梧，赤臂外罩厚重牛皮防護圍兜，臂膀青筋虯結，滿身精金炭火氣息".to_string();
    }
    if title.contains("岑拓") || entity_id == "wt_npc_cen_tuo" {
        return "身著深灰耐磨工作衫，右臂套著厚皮防燙手甲，目光如炬，專注沉靜".to_string();
    }
    if title.contains("岑雲野") || entity_id == "wt_npc_cen_yunye" {
        return "身穿黑色防風機能服，外罩暗紅半身防護背心，身姿挺拔如松，腰佩練習木刀".to_string();
    }
    if title.contains("溫懷瑾") || entity_id == "wt_npc_wen_huaijin" {
        return "身著清爽月白色挺括襯衫與長褲，袖口收緊，指間隱含艾草香氣，眉宇溫和謙潤".to_string();
    }
    if title.contains("溫晴") || title.contains("溫采薇") || entity_id == "wt_npc_wen_qing" || entity_id == "wt_npc_wen_caiwei" {
        return "身著俐落的杏黃色俐落工作裙，腰間繫著分格布囊藥包與百草溫灸盒，神情溫柔專注".to_string();
    }
    if title.contains("柯成器") || entity_id == "wt_npc_ke_chengqi" {
        return "滿頭短鬚微帶木屑，穿著耐磨牛筋布背心，腰間皮帶別滿各色雕花鑿與角尺".to_string();
    }
    if title.contains("柯有才") || entity_id == "wt_npc_ke_youcai" {
        return "穿著灰布工作襯衫，肩扛刨平木料，滿面憨厚笑容".to_string();
    }
    if title.contains("蘇錦心") || entity_id == "wt_npc_su_jinxin" {
        return "身著雅緻雪青色剪裁長裙，長髮俐落束起並插著銀頂量尺針，手指纖細白皙，氣質靈動婉約".to_string();
    }
    if title.contains("陳萬福") || entity_id == "wt_npc_chen_wanfu" {
        return "身形微豐健朗，穿著乾淨的白棉布大圍裙，笑口常開，身上帶著鮮香湯氣".to_string();
    }
    if title.contains("白泉生") || entity_id == "wt_npc_bai_quansheng" {
        return "穿著寬鬆棉麻浴場休閒服，腳踏防滑木底涼拖，手持竹柄水勺，容光煥發".to_string();
    }
    if title.contains("趙觀星") || entity_id == "wt_npc_zhao_guanxing" {
        return "身穿繪有星圖暗紋的深藍觀測大衣，手持黃銅天體測角儀，眼神悠遠".to_string();
    }
    if title.contains("林時中") || entity_id == "wt_npc_lin_shizhong" {
        return "穿著灰布工作襯衫，口袋插著精細小起子與發條鑰匙，神情專注".to_string();
    }
    if title.contains("韓鐵山") || entity_id == "wt_npc_han_tieshan" {
        return "面龐黝黑硬朗，穿著厚皮革背心與長靴，手握皮鞭，嗓音渾厚有力".to_string();
    }
    if title.contains("陸景軒") || entity_id == "wt_npc_lu_jingxuan" {
        return "身著潔淨淺青色襯衫，面容溫和清瘦，懷抱啟蒙教材與一束鋼尖沾水筆".to_string();
    }
    if title.contains("秦明誠") || entity_id == "wt_npc_qin_mingcheng" {
        return "身穿整潔洗白的藏青色夾克，右手拿著計數板，手指不時點算".to_string();
    }
    if title.contains("周一刀") || entity_id == "wt_npc_zhou_yidao" {
        return "身著乾淨白色短袖工作衫，胸前革帶插滿各式修容細剪與剃刀，動作俐落如風".to_string();
    }
    if title.contains("許阿甘") || entity_id == "wt_npc_xu_agan" {
        return "穿著洗得泛黃的紅花布圍裙，兜裡裝滿五彩糖球，神情親切可親".to_string();
    }
    if title.contains("沈清河") || entity_id == "wt_npc_shen_qinghe" {
        return "身著淺茶色休閒外套，手握隨身懷錶，笑容溫厚周到，語調從容不迫".to_string();
    }
    if title.contains("鄭火旺") || entity_id == "wt_vendor_roast_beef" {
        return "身穿耐磨粗布短衫，腰繫沾滿炭灰的厚帆布防油圍裙，手持長柄鐵夾，滿身炭火烤肉香".to_string();
    }
    if title.contains("林罔市") || entity_id == "wt_vendor_noodle_soup" {
        return "身著花布短衫外罩白棉布圍裙，挽起衣袖，右手握著長柄竹編笊籬，周身蒸氣繚繞".to_string();
    }
    if title.contains("張阿發") || entity_id == "wt_vendor_pepper_bun" {
        return "頭裹吸汗白布巾，雙臂戴著厚實的防燙棉布手套，手持長柄扁鐵鏟，身上散發濃郁蔥香與胡椒香".to_string();
    }
    if title.contains("蘇秋月") || entity_id == "wt_vendor_herbal_tea" {
        return "身著青藍色素雅短衫，手腕套著光潔銀鐲，手中輕搖小銅盞，攤前散發著冰爽甘美的仙草與黑糖清香".to_string();
    }
    if title.contains("陳順興") || entity_id == "wt_vendor_tofu_pudding" {
        return "面容慈祥，穿著乾淨的藍布對襟衫，手握一把扁平銅杓，神情篤定從容".to_string();
    }
    if title.contains("吳秀琴") || entity_id == "wt_vendor_rice_roll" {
        return "圍著藍條紋廚房圍裙，戴著乾淨防油套袖，雙手麻利翻動竹簾與白棉布，手腳極其俐落".to_string();
    }
    if title.contains("周文貴") || entity_id == "wt_vendor_soy_milk" {
        return "身著寬鬆白布短褂，肩上搭著吸汗白毛巾，手持長柄木勺在大銅壺與瓷碗間傾倒濃香米漿".to_string();
    }
    if title.contains("李春生") || entity_id == "wt_vendor_morning_produce" {
        return "頭戴寬簷遮陽竹笠，捲起深色棉布褲管，身前放著盛滿清水的竹筒噴水壺，滿面泥土清香與憨厚笑容".to_string();
    }
    if title.contains("許海生") || entity_id == "wt_vendor_morning_fish" {
        return "穿著防水牛筋皮背心與高筒膠靴，腰間繫著剖魚小皮鞘，身上帶著新鮮河水與海味鹹香".to_string();
    }
    if title.contains("馬補靴") || entity_id == "wt_npc_ma_buxue" {
        return "鼻樑架著黃銅圓框老花鏡，圍著滿是蠟痕與針孔的厚牛皮圍兜，指節粗大粗糙，身邊擺滿硬木鞋楦".to_string();
    }
    let slot = wenyao_household_slot_for(entity_id);
    slot.attire_line.to_string()
}

pub fn npc_brief_appearance_tag(entity_id: &str, display_title: &str) -> String {
    let title = display_title.trim();
    if title.contains("陸知微") || entity_id == "wt_npc_lu_zhiwei" {
        return "身著深青長外套".to_string();
    }
    if title.contains("陸承禮") || entity_id == "wt_npc_lu_chengli" {
        return "身穿墨色立領外套".to_string();
    }
    if title.contains("陸長貴") || entity_id == "wt_npc_lu_changgui" {
        return "佩帶印章公文夾".to_string();
    }
    if title.contains("陸秉衡") || entity_id == "wt_npc_lu_bingheng" {
        return "身穿寶藍正裝外套".to_string();
    }
    if title.contains("岑守岳") || entity_id == "wt_npc_cen_shouyue" {
        return "繫著厚重牛皮圍兜".to_string();
    }
    if title.contains("岑拓") || entity_id == "wt_npc_cen_tuo" {
        return "戴著防燙手甲".to_string();
    }
    if title.contains("岑雲野") || entity_id == "wt_npc_cen_yunye" {
        return "身穿黑色機能服".to_string();
    }
    if title.contains("溫懷瑾") || entity_id == "wt_npc_wen_huaijin" {
        return "身著挺括白襯衫".to_string();
    }
    if title.contains("溫晴") || title.contains("溫采薇") || entity_id == "wt_npc_wen_qing" || entity_id == "wt_npc_wen_caiwei" {
        return "身穿杏黃工作裙".to_string();
    }
    if title.contains("柯成器") || entity_id == "wt_npc_ke_chengqi" {
        return "腰別鑿尺工具帶".to_string();
    }
    if title.contains("柯有才") || entity_id == "wt_npc_ke_youcai" {
        return "肩扛刨平木料".to_string();
    }
    if title.contains("蘇錦心") || entity_id == "wt_npc_su_jinxin" {
        return "身穿雪青剪裁長裙".to_string();
    }
    if title.contains("陳萬福") || entity_id == "wt_npc_chen_wanfu" {
        return "圍著白棉布大圍裙".to_string();
    }
    if title.contains("白泉生") || entity_id == "wt_npc_bai_quansheng" {
        return "穿著浴場休閒服".to_string();
    }
    if title.contains("趙觀星") || entity_id == "wt_npc_zhao_guanxing" {
        return "身穿深藍觀測大衣".to_string();
    }
    if title.contains("林時中") || entity_id == "wt_npc_lin_shizhong" {
        return "口袋插著發條鑰匙".to_string();
    }
    if title.contains("韓鐵山") || entity_id == "wt_npc_han_tieshan" {
        return "穿著厚皮革背心".to_string();
    }
    if title.contains("陸景軒") || entity_id == "wt_npc_lu_jingxuan" {
        return "身著淺青襯衫".to_string();
    }
    if title.contains("秦明誠") || entity_id == "wt_npc_qin_mingcheng" {
        return "手持計數板".to_string();
    }
    if title.contains("周一刀") || entity_id == "wt_npc_zhou_yidao" {
        return "佩掛各式修容細剪".to_string();
    }
    if title.contains("許阿甘") || entity_id == "wt_npc_xu_agan" {
        return "圍著紅花布圍裙".to_string();
    }
    if title.contains("沈清河") || entity_id == "wt_npc_shen_qinghe" {
        return "手握隨身懷錶".to_string();
    }
    if title.contains("鄭火旺") || entity_id == "wt_vendor_roast_beef" {
        return "圍著防油帆布圍裙手持長夾".to_string();
    }
    if title.contains("林罔市") || entity_id == "wt_vendor_noodle_soup" {
        return "手持竹編笊籬蒸氣繚繞".to_string();
    }
    if title.contains("張阿發") || entity_id == "wt_vendor_pepper_bun" {
        return "戴著防燙棉手套持扁鐵鏟".to_string();
    }
    if title.contains("蘇秋月") || entity_id == "wt_vendor_herbal_tea" {
        return "手搖清脆銅盞盛裝仙草凍".to_string();
    }
    if title.contains("陳順興") || entity_id == "wt_vendor_tofu_pudding" {
        return "手握扁平銅杓片熱豆花".to_string();
    }
    if title.contains("吳秀琴") || entity_id == "wt_vendor_rice_roll" {
        return "圍藍條紋圍裙麻利包飯糰".to_string();
    }
    if title.contains("周文貴") || entity_id == "wt_vendor_soy_milk" {
        return "肩搭白毛巾手持木勺舀米漿".to_string();
    }
    if title.contains("李春生") || entity_id == "wt_vendor_morning_produce" {
        return "戴遮陽竹笠持噴水壺理鮮菜".to_string();
    }
    if title.contains("許海生") || entity_id == "wt_vendor_morning_fish" {
        return "穿高筒膠靴換水保鮮活魚".to_string();
    }
    if title.contains("馬補靴") || entity_id == "wt_npc_ma_buxue" {
        return "戴圓框老花鏡圍牛皮圍兜".to_string();
    }

    let slot = wenyao_household_slot_for(entity_id);
    slot.brief_tag.to_string()
}

pub fn normalize_archaic_clothing(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut res = s.to_string();
    res = res.replace("童生布衫", "學生制服襯衫");
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
    res = res.replace("吏目直裰", "深青色公務外套");
    res = res.replace("直裰", "公務外套");
    res = res.replace("深藍道袍", "深藍觀測大衣");
    res = res.replace("道袍", "觀測大衣");
    res = res.replace("交領羅裙", "剪裁長裙");
    res = res.replace("羅裙", "剪裁長裙");
    res = res.replace("窄袖布裙", "俐落工作裙");
    res = res.replace("短坎肩", "皮革背心");
    res = res.replace("素青直身衣", "淺青色襯衫");
    res = res.replace("藏青夾袍", "藏青色夾克");
    res = res.replace("廣袖儒袍", "修身長外套");
    res = res.replace("細絹長袍", "休閒外套");
    res = res.replace("月白長袍", "挺括襯衫");
    res = res.replace("墨色長衫", "墨色立領外套");
    res = res.replace("寶藍錦袍", "寶藍色正裝外套");
    res = res.replace("桐木屐", "防滑木底涼拖");
    res = res.replace("烏木算盤", "便攜計數板");
    res = res.replace("金絲算盤", "隨身懷錶與核算夾");
    res = res.replace("算盤", "計數板");
    res = res.replace("竹骨摺扇", "隨身懷錶");
    res = res.replace("摺扇", "隨身懷錶");
    res = res.replace("硃筆", "鋼尖沾水筆");
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
        ("文曜蒙學堂助教", "文曜國小助教"),
        ("文曜小學堂助教", "文曜國小助教"),
        ("溫氏藥居採藥人", "溫氏診所護理員"),
        ("三水商行押運夥計", "三水商行外務員"),
        ("東橋茶寮茶客", "東橋茶屋常客"),
        ("海味乾貨行商", "海產乾貨商"),
        ("公議堂抄錄書生", "公議堂紀錄員"),
        ("車馬行趕車夫", "分家巷木工技師"),
        ("藏書閣校書生", "藏書閣編輯助理"),
        ("西門貨棧搬運工", "百工街鐵工"),
        ("北門巡街衛士", "北門巡守員"),
        ("南門紡織女工", "三水食堂廚務員"),
        ("三水渡口船夫", "講堂街研究員"),
        ("竹編器物手藝人", "織造坊裁縫師"),
        ("石橋挑擔貨郎", "石橋攤商"),
        ("觀星台測候學徒", "南街藥材商"),
        ("鐘樓守時雜役", "鐘樓報時員"),
        ("泉香浴道夥計", "泉香浴場服務員"),
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
        ("公議堂保甲司", "公議堂治安巡防課"),
        ("民務通事", "民事調解員"),
        ("小學堂教師", "國小老師"),
        ("蒙學教諭", "國小老師"),
        ("教書先生", "國小老師"),
        ("抄錄書生", "紀錄員"),
        ("校書生", "編輯助理"),
        ("挑擔貨郎", "攤商"),
        ("茶寮茶客", "茶屋常客"),
        ("講堂街抱卷學子", "講堂街抱書學生"),
        ("文曜曆算學子", "文曜曆算系學生"),
        ("算學學子", "學院學生"),
        ("學子", "學生"),
        ("文曜小學堂", "文曜國小"),
        ("文曜蒙學堂", "文曜國小"),
        ("蒙學堂助教", "國小助教"),
        ("小學堂助教", "國小助教"),
        ("小學堂", "國小"),
        ("蒙學堂", "國小"),
        ("跑堂夥計", "外場服務員"),
        ("跑堂", "外場服務員"),
        ("巡街衛士", "巡守員"),
        ("守時雜役", "報時員"),
        ("行旅散客", "外地旅客"),
        ("茶博士", "茶屋侍者"),
        ("鏢師", "隨隊安保員"),
        ("郎中", "駐店醫師"),
        ("腳夫", "搬運員"),
        ("貨郎", "攤商"),
        ("掌櫃", "店長"),
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
    goal: (i32, i32),
) -> Option<(i32, i32)> {
    if start == goal {
        return None;
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
            let dist = (goal.0 - nx).abs() + (goal.1 - ny).abs();
            let is_road_or_bridge = grid
                .get(crate::grid::SquareCoord::new(nx, ny))
                .map(|c| c.terrain == crate::grid::Terrain::Road || c.terrain == crate::grid::Terrain::Bridge)
                .unwrap_or(false);
            (dist, if is_road_or_bridge { 0u8 } else { 1u8 })
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

    let mut max_steps = 1000;
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

    // 第二階段 BFS：若目標為外圍深巷民宅（需穿過前院／廊道方可抵達），放寬為所有非水域可通行格
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
    let mut steps2 = 1000;
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
    None
}

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
        "宿舍", "書齋", "小齋", "教室", "書局", "印行館", "剪剃鋪", "事務所", "工坊", "旅社",
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
    let Some(to_cell) = grid.get(crate::grid::SquareCoord::new(to_x, to_y)) else {
        return false;
    };
    if !to_cell.terrain.walkable() {
        return false;
    }
    let Some(from_cell) = grid.get(crate::grid::SquareCoord::new(from_x, from_y)) else {
        return false;
    };

    let from_is_bldg = is_building_cell_name_or_terrain(from_cell);
    let to_is_bldg = is_building_cell_name_or_terrain(to_cell);

    // 建築隔離規則：若起點與終點皆為建築（非道路／橋樑／廣場／空地），禁止直接穿牆步進
    if from_is_bldg && to_is_bldg {
        if from_cell.name.is_empty() || from_cell.name != to_cell.name {
            return false;
        }
    }

    // 城鎮水域隔離
    if (from_cell.terrain == crate::grid::Terrain::Water || to_cell.terrain == crate::grid::Terrain::Water)
        && (!from_cell.zone.is_empty() || !to_cell.zone.is_empty() || from_is_bldg || to_is_bldg)
    {
        return false;
    }

    true
}

impl Store {
    // ── entity_rooms ──

    pub fn get_entity_room(&self, entity_id: &str) -> String {
        if let Some(e) = self.entities.get(entity_id)
            && let Some((x, y)) = self.effective_entity_grid(e)
        {
            return crate::grid::grid_room_id_from_coord(x, y);
        }
        if let Some(place_id) = self.get_entity_place_id(entity_id)
            && let Some(place) = self.get_place(&place_id)
        {
            return place.compatibility_room_id();
        }
        self.entity_rooms
            .get(entity_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_entity_room(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
            let place_id = crate::world::place::legacy_place_id(coord);
            if self.get_place(&place_id).is_some() {
                return self.set_entity_place(entity_id, &place_id);
            }
        }
        self.entity_rooms
            .insert(entity_id.to_string(), room_id.to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityRoom {
            entity_id: entity_id.to_string(),
            room_id: room_id.to_string(),
        });
        Ok(())
    }
    /// 直接更新實體所在房間或虛擬格（如 `grid:x:y`），不更動場所圖狀態。
    pub fn move_entity(&mut self, entity_id: &str, room_id: &str) -> anyhow::Result<()> {
        self.entity_rooms
            .insert(entity_id.to_string(), room_id.to_string());
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityRoom {
            entity_id: entity_id.to_string(),
            room_id: room_id.to_string(),
        });
        Ok(())
    }

    /// 寫入或更新完整實體資料並同步快取與非同步持久化。
    pub fn upsert_entity_full(&mut self, e: Entity) -> anyhow::Result<()> {
        self.put_entity(e)
    }

    /// 設定實體的表面可觀測行為。
    pub fn set_entity_activity(&mut self, entity_id: &str, activity: &str) -> anyhow::Result<()> {
        if let Some(e) = self.entities.get_mut(entity_id) {
            e.current_activity = activity.to_string();
        }
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityActivity {
            entity_id: entity_id.to_string(),
            activity: activity.to_string(),
        });
        Ok(())
    }

    /// 讀取 NPC 實體的即時表面行為（若在文曜定居居民狀態中則取用記憶體 LIVE_NPC_GRID_STATES，否則回傳 e.current_activity）。
    pub fn effective_npc_activity(&self, e: &Entity) -> String {
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(&e.id) {
            return state.current_activity();
        }
        if !e.current_activity.is_empty() {
            return e.current_activity.clone();
        }
        String::new()
    }

    pub fn ensure_live_npcs_seeded(&self) {
        static LIVE_SEEDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !LIVE_SEEDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            self.ensure_household_assignments();
            for e in self.entities.values() {
                if e.kind == "npc" && e.vit > 0 && e.grid_x.is_none() {
                    let _ = self.effective_entity_grid(e);
                }
            }
        }
    }

    pub fn entity_ids_in_room(&self, room_id: &str) -> Vec<String> {
        if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
            self.ensure_live_npcs_seeded();
            let mut ids = Vec::new();
            let mut seen = std::collections::HashSet::new();

            // 1. LIVE_NPC_GRID_STATES whose state.current == (coord.x, coord.y)
            {
                let states = LIVE_NPC_GRID_STATES.read();
                for (id, state) in states.iter() {
                    if state.current == (coord.x, coord.y) {
                        if seen.insert(id.clone()) {
                            ids.push(id.clone());
                        }
                    }
                }
            }

            // 2. Canonical NPCs directly from self.entities at (coord.x, coord.y) not tracked in LIVE_NPC_GRID_STATES
            for (id, e) in &self.entities {
                if e.kind == "npc"
                    && e.vit > 0
                    && e.grid_x == Some(coord.x)
                    && e.grid_y == Some(coord.y)
                {
                    if seen.insert(id.clone()) {
                        ids.push(id.clone());
                    }
                }
            }

            // 3. Players or non-grid entities in self.entity_rooms at room_id
            for (eid, rid) in &self.entity_rooms {
                if rid == room_id {
                    let is_tracked_live_npc = LIVE_NPC_GRID_STATES.read().contains_key(eid);
                    let is_tracked_canon_npc = self
                        .entities
                        .get(eid)
                        .map(|e| e.kind == "npc" && e.grid_x.is_some() && e.grid_y.is_some())
                        .unwrap_or(false);
                    if !is_tracked_live_npc && !is_tracked_canon_npc {
                        if seen.insert(eid.clone()) {
                            ids.push(eid.clone());
                        }
                    }
                }
            }

            return ids;
        }
        self.entity_rooms
            .iter()
            .filter(|(_, rid)| rid.as_str() == room_id)
            .map(|(eid, _)| eid.clone())
            .collect()
    }

    pub fn all_entity_ids(&self) -> Vec<String> {
        self.entities.keys().cloned().collect()
    }

    pub fn get_npc_ids_with_room(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entity_rooms
            .keys()
            .filter(|eid| {
                self.entities
                    .get(eid.as_str())
                    .is_some_and(|e| e.kind == "npc" && e.vit > 0)
            })
            .cloned()
            .collect();
        let live = LIVE_NPC_GRID_STATES.read();
        for id in live.keys() {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    pub fn get_player_ids_with_room(&self) -> Vec<String> {
        self.entity_rooms
            .keys()
            .filter(|eid| {
                self.entities
                    .get(eid.as_str())
                    .is_some_and(|e| e.kind == "player")
            })
            .cloned()
            .collect()
    }

    // ── entities ──

    pub fn get_entity(&self, id: &str) -> Option<Entity> {
        if is_deprecated_npc_id(id) {
            return None;
        }
        let mut e = self.entities.get(id).cloned()?;
        if e.kind == "npc" && (e.grid_x.is_none() || e.grid_y.is_none()) {
            let _ = self.effective_entity_grid(&e);
        }
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(id) {
            let mut act = state.current_activity();
            let hour = current_game_hour();
            if (hour >= 22 || hour < 5) && state.current == state.dest {
                let is_watch = state.dest_name.contains("巡防")
                    || state.dest_name.contains("哨")
                    || state.dest_name.contains("關口")
                    || state.dest_name.contains("道口")
                    || state.dest_name.contains("水口")
                    || state.dest_name.contains("渡口")
                    || state.dest_name.contains("門坊")
                    || state.dest_name.contains("旅社")
                    || state.dest_name.contains("驛館")
                    || state.dest_name.contains("歇舍");
                if !is_watch && !act.contains("安歇") && !act.contains("熟睡") && !act.contains("就寢") {
                    let slot = wenyao_household_slot_for(id);
                    let (_, _, _, sleep_act, _) = resolve_wenyao_npc_schedule(slot, hour, false, &state.dest_name);
                    act = sleep_act;
                }
            }
            if !act.is_empty() {
                e.current_activity = act;
            }
            if e.kind == "npc" {
                e.grid_x = Some(state.current.0);
                e.grid_y = Some(state.current.1);
            }
        }
        Some(e)
    }

    pub fn put_entity(&mut self, e: Entity) -> anyhow::Result<()> {
        let mut e = e;
        if e.activated_nodes.is_empty() {
            e.activated_nodes = r#"["N000"]"#.to_string();
        }
        if e.inventory.is_empty() {
            e.inventory = "[]".to_string();
        }
        self.entities.insert(e.id.clone(), e.clone());
        crate::pg::entity::upsert_async(e);
        Ok(())
    }

    /// PostgreSQL 已先提交的新人物只鏡射進快取，不再排一筆可能覆蓋後續位置的舊快照。
    pub fn mirror_new_entity_from_authority(&mut self, mut entity: Entity) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.entities.contains_key(&entity.id),
            "實體已存在：{}",
            entity.id
        );
        if entity.activated_nodes.is_empty() {
            entity.activated_nodes = r#"["N000"]"#.to_string();
        }
        if entity.inventory.is_empty() {
            entity.inventory = "[]".to_string();
        }
        self.entities.insert(entity.id.clone(), entity);
        Ok(())
    }

    pub fn update_entity(&mut self, id: &str, f: impl FnOnce(&mut Entity)) -> anyhow::Result<()> {
        if let Some(e) = self.entities.get_mut(id) {
            f(e);
            let e_clone = e.clone();
            crate::pg::entity::upsert_async(e_clone);
        }
        Ok(())
    }

    /// PostgreSQL 已在同一交易完成背包變動時，只校正讀取快取，不再排第二次寫入。
    pub fn mirror_inventory_from_authority(
        &mut self,
        entity_id: &str,
        inventory: &str,
    ) -> anyhow::Result<()> {
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.inventory = inventory.to_string();
        Ok(())
    }

    /// PostgreSQL 已完成共用交易後，校正快取並以同一快照壓在較早排隊的寫入之後。
    pub fn mirror_economy_from_authority(
        &mut self,
        entity_id: &str,
        magnesium: i32,
        inventory: &str,
        current_activity: &str,
    ) -> anyhow::Result<()> {
        let entity = self
            .entities
            .get_mut(entity_id)
            .ok_or_else(|| anyhow::anyhow!("實體不存在：{entity_id}"))?;
        entity.magnesium = magnesium;
        entity.inventory = inventory.to_string();
        entity.current_activity = current_activity.to_string();
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityEconomySnapshot {
            entity_id: entity_id.to_string(),
            magnesium,
            inventory: inventory.to_string(),
            current_activity: current_activity.to_string(),
        });
        Ok(())
    }

    /// 舊座標入口；立即轉為正式 place_id，座標只作相容鏡像。
    pub fn set_entity_grid(&mut self, entity_id: &str, x: i32, y: i32) -> anyhow::Result<()> {
        let mut states = LIVE_NPC_GRID_STATES.write();
        if let Some(state) = states.get_mut(entity_id) {
            state.current = (x, y);
        } else {
            let slot = wenyao_household_slot_for(entity_id);
            let hour = current_game_hour();
            let (dest, dest_name, purpose, stay_activity, stay_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, slot.home_name);
            states.insert(
                entity_id.to_string(),
                NpcGridState {
                    current: (x, y),
                    dest,
                    dest_name,
                    origin_name: slot.home_name.to_string(),
                    stay_activity,
                    purpose,
                    stay_ticks_remaining: stay_ticks,
                    work_errand_toggle: false,
                    consecutive_work_streaks: 0,
                },
            );
        }
        drop(states);
        self.set_entity_grid_compat(entity_id, x, y)
    }

    /// 計算實體在正方格地圖上的有效座標：
    /// - 若已有明確 `(grid_x, grid_y)` 則直接採用；
    /// - 若為存活 NPC，先查執行期連續座標表；
    /// - 若尚未初始化，依所屬區域決定性選取初始可行走方格，並登錄執行期座標表。
    pub fn ensure_household_assignments(&self) {
        if !HOUSEHOLD_MEMBER_ASSIGNMENTS.read().is_empty() {
            return;
        }
        let mut eligible_ids: Vec<String> = self
            .entities
            .values()
            .filter(|e| {
                e.kind == "npc"
                    && e.vit > 0
                    && e.grid_x.is_none()
                    && !is_deprecated_npc_id(&e.id)
            })
            .map(|e| e.id.clone())
            .collect();
        eligible_ids.sort();
        let mut map = HOUSEHOLD_MEMBER_ASSIGNMENTS.write();
        if map.is_empty() {
            for (idx, id) in eligible_ids
                .into_iter()
                .take(WENYAO_HOUSEHOLD_SLOTS.len())
                .enumerate()
            {
                map.insert(id, idx);
            }
        }
    }

    pub fn effective_entity_grid(&self, e: &Entity) -> Option<(i32, i32)> {
        if is_deprecated_npc_id(&e.id) {
            return None;
        }
        if let (Some(x), Some(y)) = (e.grid_x, e.grid_y) {
            return Some((x, y));
        }
        if e.kind != "npc" || e.vit <= 0 {
            return None;
        }
        if let Some(state) = LIVE_NPC_GRID_STATES.read().get(&e.id) {
            return Some(state.current);
        }

        self.ensure_household_assignments();
        let slot_idx = *HOUSEHOLD_MEMBER_ASSIGNMENTS.read().get(&e.id)?;
        let slot = &WENYAO_HOUSEHOLD_SLOTS[slot_idx % WENYAO_HOUSEHOLD_SLOTS.len()];

        let hour = current_game_hour();
        let init_toggle = slot_idx % 2 == 1;
        let (dest, dest_name, purpose, stay_activity, stay_ticks) =
            resolve_wenyao_npc_schedule(slot, hour, init_toggle, slot.home_name);
        let init_coord = dest;
        let origin_name = slot.home_name.to_string();
        let stay_ticks_remaining = stay_ticks;
        LIVE_NPC_GRID_STATES.write().insert(
            e.id.clone(),
            NpcGridState {
                current: init_coord,
                dest,
                dest_name,
                origin_name,
                stay_activity,
                purpose,
                stay_ticks_remaining,
                work_errand_toggle: init_toggle,
                consecutive_work_streaks: 0,
            },
        );
        Some(init_coord)
    }

    /// 統計所有在場非棄用 NPC 於各地圖座標的即時人數。
    pub fn npc_counts_by_grid(&self) -> std::collections::HashMap<(i32, i32), u16> {
        let mut counts = std::collections::HashMap::new();
        for e in self.entities.values() {
            if e.kind == "npc" && e.vit > 0 && !is_deprecated_npc_id(&e.id) {
                if let Some(coord) = self.effective_entity_grid(e) {
                    *counts.entry(coord).or_insert(0) += 1;
                }
            }
        }
        counts
    }

    /// 推進所有定居居民在正方格地圖上的日常作息與步進（每 tick 沿街道邁進 1 格或於場所內停留作業）。
    pub fn advance_household_npcs_one_step(&self) -> Vec<(String, String, String)> {
        static SEEDED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !SEEDED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            self.ensure_household_assignments();
            let mut need_init_mg = Vec::new();
            for e in self.entities.values() {
                if e.kind == "npc" && e.vit > 0 && e.grid_x.is_none() {
                    let _ = self.effective_entity_grid(e);
                    if e.magnesium <= 0 {
                        need_init_mg.push(e.id.clone());
                    }
                }
            }
            if !need_init_mg.is_empty() {
                if let Ok(handle) = tokio::runtime::Handle::try_current() {
                    handle.spawn(async move {
                        for (idx, id) in need_init_mg.into_iter().enumerate() {
                            let init_mg = 18 + ((idx % 5) as i32) * 4;
                            let _ = crate::db::add_magnesium(&id, init_mg);
                        }
                    });
                } else {
                    for (idx, id) in need_init_mg.into_iter().enumerate() {
                        let init_mg = 18 + ((idx % 5) as i32) * 4;
                        let _ = crate::db::add_magnesium(&id, init_mg);
                    }
                }
            }
        }
        let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
            return Vec::new();
        };
        let ids: Vec<String> = LIVE_NPC_GRID_STATES.read().keys().cloned().collect();
        let mut moves = Vec::new();
        for id in ids {
            if let Some(((ox, oy), (nx, ny))) = self.step_npc_grid_coord_with_grid(&id, &grid)
                && (ox, oy) != (nx, ny)
            {
                moves.push((
                    id,
                    crate::grid::grid_room_id_from_coord(ox, oy),
                    crate::grid::grid_room_id_from_coord(nx, ny),
                ));
            }
        }
        moves
    }

    /// 沿道路相鄰可通行方格 BFS 步進 NPC 座標（每次最多移動 1 格）。
    pub fn step_npc_grid_coord(
        &self,
        entity_id: &str,
        _route_dir: &str,
        _target_zone: &str,
        _seed: u64,
    ) -> Option<((i32, i32), (i32, i32))> {
        let grid = crate::server::grid_manager::get_runtime_square_grid()?;
        self.step_npc_grid_coord_with_grid(entity_id, &grid)
    }

    pub fn step_npc_grid_coord_with_grid(
        &self,
        entity_id: &str,
        grid: &crate::grid::SquareGrid,
    ) -> Option<((i32, i32), (i32, i32))> {
        let e = self.entities.get(entity_id)?;
        let (old_x, old_y) = self.effective_entity_grid(e)?;
        if is_npc_paused_for_interaction(entity_id) {
            return Some(((old_x, old_y), (old_x, old_y)));
        }

        let hour = current_game_hour();
        let slot = wenyao_household_slot_for(entity_id);

        let mut states = LIVE_NPC_GRID_STATES.write();
        let state = states.entry(entity_id.to_string()).or_insert_with(|| {
            let (dest, dest_name, purpose, stay_activity, stay_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, slot.home_name);
            NpcGridState {
                current: (old_x, old_y),
                dest,
                dest_name,
                origin_name: slot.home_name.to_string(),
                stay_activity,
                purpose,
                stay_ticks_remaining: stay_ticks,
                work_errand_toggle: false,
                consecutive_work_streaks: 0,
            }
        });

        if e.grid_x.is_some() && e.grid_y.is_some() {
            return None;
        }

        if hour >= 22 || hour < 5 {
            let (home_or_watch_dest, target_name, target_purpose, target_activity, target_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, &state.dest_name);
            state.dest = home_or_watch_dest;
            state.dest_name = target_name;
            state.purpose = target_purpose;
            state.stay_activity = target_activity;
            state.stay_ticks_remaining = target_ticks;
            if state.current == state.dest {
                return Some(((old_x, old_y), (old_x, old_y)));
            }
        } else if (5..22).contains(&hour)
            && (state.purpose == "返家安歇" || state.purpose == "安歇養神" || state.purpose == "夜間返家")
        {
            let (day_dest, day_dest_name, day_purpose, day_activity, day_ticks) =
                resolve_wenyao_npc_schedule(slot, hour, false, &state.dest_name);
            state.dest = day_dest;
            state.dest_name = day_dest_name;
            state.purpose = day_purpose;
            state.stay_activity = day_activity;
            state.stay_ticks_remaining = day_ticks;
            if state.current == state.dest {
                return Some(((old_x, old_y), (old_x, old_y)));
            }
        }
        if state.current == state.dest {
            if state.stay_ticks_remaining > 0 {
                state.stay_ticks_remaining -= 1;
                return Some(((old_x, old_y), (old_x, old_y)));
            }
            let now_unix = crate::game::now_unix();
            let ent_id = entity_id.to_string();
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for b in entity_id.as_bytes() {
                h ^= u64::from(*b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
            // 1. Appraise completed activity at state.dest_name with state.purpose
            let dest_name_appraisal = state.dest_name.clone();
            let purpose_appraisal = state.purpose.clone();
            let is_dining = [
                "食堂", "茶", "飯", "湯", "麵", "膳", "飲",
            ]
            .iter()
            .any(|kw| dest_name_appraisal.contains(kw) || purpose_appraisal.contains(kw));
            let is_bath = dest_name_appraisal.contains("浴場");
            let is_work = [
                "工坊", "鍛", "木", "商行", "埠", "站", "院", "堂", "館", "閣", "所",
            ]
            .iter()
            .any(|kw| dest_name_appraisal.contains(kw) || purpose_appraisal.contains(kw));

            if is_dining {
                let ent_id_c = ent_id.clone();
                let loc = dest_name_appraisal.clone();
                std::thread::spawn(move || {
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(&ent_id_c, |ent| {
                            if ent.magnesium >= 2 {
                                ent.magnesium = ent.magnesium.saturating_sub(2);
                            }
                            crate::db::add_thought_to_entity_with_flavor(
                                ent,
                                crate::db::ThoughtSource::Ate,
                                now_unix,
                                format!("在{}用過熱騰騰的茶飯，肚裡暖和舒坦", loc),
                            );
                            ent.disposition = (ent.disposition + 8).clamp(-100, 100);
                        });
                    }
                });
            } else if is_bath {
                let ent_id_c = ent_id.clone();
                std::thread::spawn(move || {
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(&ent_id_c, |ent| {
                            crate::db::add_thought_to_entity_with_flavor(
                                ent,
                                crate::db::ThoughtSource::Rested,
                                now_unix,
                                "在泉香溫水浴場泡過熱湯，一身疲乏盡數散去",
                            );
                            ent.disposition = (ent.disposition + 10).clamp(-100, 100);
                        });
                    }
                });
            } else if is_work {
                let ent_id_c = ent_id.clone();
                let loc = dest_name_appraisal.clone();
                let purp = purpose_appraisal.clone();
                let earn = 6 + (h % 7) as i32;
                let streaks = state.consecutive_work_streaks;
                state.consecutive_work_streaks = state.consecutive_work_streaks.saturating_add(1);
                std::thread::spawn(move || {
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(&ent_id_c, |ent| {
                            ent.magnesium = ent.magnesium.saturating_add(earn);
                            if streaks >= 2 {
                                crate::db::add_thought_to_entity_with_delta(
                                    ent,
                                    crate::db::ThoughtSource::Daily,
                                    -4,
                                    now_unix,
                                    format!("在{}連軸忙完「{}」，肩背有些痠沉，想找地方歇口氣喝杯熱茶", loc, purp),
                                );
                            } else {
                                crate::db::add_thought_to_entity_with_delta(
                                    ent,
                                    crate::db::ThoughtSource::WorkDone,
                                    5,
                                    now_unix,
                                    format!("在{}忙完一輪「{}」，手頭進了幾枚鎂幣，心裡踏實", loc, purp),
                                );
                            }
                        });
                    }
                });
            } else {
                state.consecutive_work_streaks = 0;
            }

            // Prune stale duplicate thoughts if needed
            let has_stale_broke = e.magnesium >= 35 && e.thoughts.iter().any(|t| t.flavor.as_deref().unwrap_or("").contains("鎂袋空了"));
            let daily_count = e.thoughts.iter().filter(|t| t.flavor.as_deref().unwrap_or("").contains("今日花用又見底")).count();
            if has_stale_broke || daily_count > 2 {
                let ent_id_c = ent_id.clone();
                std::thread::spawn(move || {
                    if let Some(arc) = crate::store::get_store() {
                        let mut s = arc.write();
                        let _ = s.update_entity(&ent_id_c, |ent| {
                            if ent.magnesium >= 35 {
                                ent.thoughts.retain(|t| !t.flavor.as_deref().unwrap_or("").contains("鎂袋空了"));
                            }
                            let mut kept_daily = 0;
                            ent.thoughts.retain(|t| {
                                if t.flavor.as_deref().unwrap_or("").contains("今日花用又見底") {
                                    kept_daily += 1;
                                    kept_daily <= 2
                                } else {
                                    true
                                }
                            });
                            crate::db::clean_and_recalc_entity_disposition(ent, now_unix);
                        });
                    }
                });
            }
            let prev_loc_name = state.dest_name.clone();
            let mut obligation_handled = false;

            if (7..=20).contains(&hour) {
                if let Some((ob_kind, _)) = get_active_npc_obligation(entity_id) {
                    clear_active_npc_obligation(entity_id, &ob_kind);
                    if ob_kind == "restock_goods" {
                        crate::server::action::trade::restock_shop_items_for_seller(entity_id);
                    }
                    if (ob_kind == "repair_tool" || ob_kind == "repay_debt" || ob_kind == "treat_injury")
                        && e.magnesium >= 2
                    {
                        let ent_id = entity_id.to_string();
                        std::thread::spawn(move || {
                            let _ = crate::db::add_magnesium(&ent_id, -2);
                        });
                    }
                } else {
                    let now_sec = crate::game::now_unix();
                    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
                    for b in entity_id.as_bytes() {
                        h ^= u64::from(*b);
                        h = h.wrapping_mul(0x0100_0000_01b3);
                    }
                    let time_bucket = (now_sec / 30) as u64;
                    let roll = (h.wrapping_add(time_bucket).wrapping_mul(1103515245) >> 16) % 100;
                    if roll < 18 {
                        let ob_choice = (h.wrapping_add(time_bucket) >> 4) % 8;
                        let (ob_kind, ob_subject, ob_dest, ob_dest_name, ob_act, ob_purpose) = match ob_choice {
                            0 => (
                                "repair_tool",
                                "手頭器物刃口與榫卯微損待修",
                                resolve_bldg_coord_or_fallback("岑氏第一鍛造工坊", (1, -1)),
                                "文曜・岑氏第一鍛造工坊",
                                "正在鍛造工坊請匠師修整受損器物",
                                "修整器物",
                            ),
                            1 => (
                                "repair_tool",
                                "隨身木作器具榫頭鬆動待修",
                                resolve_bldg_coord_or_fallback("巧木居所傢俱坊", (1, -2)),
                                "文曜・巧木居所傢俱坊",
                                "正在木作工坊請匠人加固器具榫卯",
                                "修整木器",
                            ),
                            2 => (
                                "restock_goods",
                                "手頭物料與店內備貨見底待補",
                                resolve_bldg_coord_or_fallback("西門車馬聯運總站", (-2, -1)),
                                "文曜・西門車馬聯運總站",
                                "正在清點並提領新運抵的補充物料",
                                "補倉提貨",
                            ),
                            3 => (
                                "restock_goods",
                                "日常用料短缺需赴雜貨行添購",
                                resolve_bldg_coord_or_fallback("分家巷口雜貨柑仔店", (1, -3)),
                                "文曜・分家巷口雜貨柑仔店",
                                "正在雜貨行添購新到貨的日用物料",
                                "添購物料",
                            ),
                            4 => (
                                "treat_injury",
                                "連日奔波筋骨微酸需敷藥調理",
                                resolve_bldg_coord_or_fallback("溫氏脈絡診所", (1, -1)),
                                "文曜・溫氏脈絡診所",
                                "正在敷用活血草藥調理筋骨勞損",
                                "敷藥調理",
                            ),
                            5 => (
                                "treat_injury",
                                "奔走風寒需赴湯池溫水祛乏",
                                resolve_bldg_coord_or_fallback("泉香溫水浴場", (-2, 2)),
                                "文曜・泉香溫水浴場",
                                "正在溫水湯池旁熱敷舒緩筋骨疲乏",
                                "溫水祛乏",
                            ),
                            6 => (
                                "repay_debt",
                                "手頭一筆貨單票據需赴總商行核銷",
                                resolve_bldg_coord_or_fallback("三水總商行", (-1, 1)),
                                "文曜・三水總商行",
                                "正在總商行櫃台憑摺核銷往來貨單",
                                "核銷貨單",
                            ),
                            _ => (
                                "restock_goods",
                                "海味與南北乾貨備料待補",
                                resolve_bldg_coord_or_fallback("三水口乾貨海味行", (-1, 3)),
                                "文曜・三水口乾貨海味行",
                                "正在海味行秤提新運抵的風櫃魚乾與海鹽",
                                "提領乾貨",
                            ),
                        };

                        open_active_npc_obligation(entity_id, ob_kind, ob_subject);
                        state.origin_name = prev_loc_name.clone();
                        state.dest = ob_dest;
                        state.dest_name = ob_dest_name.to_string();
                        state.purpose = ob_purpose.to_string();
                        state.stay_activity = ob_act.to_string();
                        state.stay_ticks_remaining = 16 + ((h % 10) as u8);
                        obligation_handled = true;
                    }
                }
            }

            if !obligation_handled {
                let personality = crate::db::expand_soul_seed_to_personality(e.soul_seed.unwrap_or(h as i64));
                let last_thought_text = e.thoughts.last().and_then(|t| t.flavor.as_deref()).unwrap_or("");
                let is_fatigued = e.disposition <= -12
                    || last_thought_text.contains("痠沉")
                    || last_thought_text.contains("疲乏")
                    || last_thought_text.contains("煩悶");

                let mut override_handled = false;

                // Case A: Fatigue / Bad Mood Recovery
                if is_fatigued {
                    let targets = [
                        ((-1, 2), "三水大食堂"),
                        ((2, -2), "泉香溫水浴場"),
                        ((2, -1), "東橋旅社與山風茶屋"),
                        ((13, 13), "講堂外坊・翰墨清茶書肆"),
                    ];
                    let idx = ((h.wrapping_add(hour as u64)) % 4) as usize;
                    let (coord, target_name) = targets[idx];
                    state.origin_name = prev_loc_name.clone();
                    state.dest = coord;
                    state.dest_name = target_name.to_string();
                    state.purpose = "舒緩心緒".to_string();
                    state.stay_activity = format!("因連日忙碌心緒微疲，特地在{}歇腳喝口熱茶舒緩精神", target_name);
                    state.stay_ticks_remaining = 20 + ((h % 8) as u8);
                    state.consecutive_work_streaks = 0;
                    override_handled = true;
                }
                // Case B: Economic Drive
                else if e.magnesium < 35 && (7..=20).contains(&hour) {
                    let targets = [
                        ((-1, 1), "三水總商行總部"),
                        ((-3, -1), "西門・車馬聯運總站"),
                        ((slot.work_x, slot.work_y), slot.work_name),
                    ];
                    let idx = ((h.wrapping_add(hour as u64)) % 3) as usize;
                    let (coord, target_name) = targets[idx];
                    state.origin_name = prev_loc_name.clone();
                    state.dest = coord;
                    state.dest_name = target_name.to_string();
                    state.purpose = "籌措生計".to_string();
                    state.stay_activity = format!("因手頭銀錢吃緊，主動趕往{}多接一趟差事貼補家用", target_name);
                    state.stay_ticks_remaining = 24 + ((h % 10) as u8);
                    let ent_id_c = ent_id.clone();
                    std::thread::spawn(move || {
                        let _ = crate::db::add_magnesium(&ent_id_c, 12);
                    });
                    override_handled = true;
                }
                // Case C: Social / Curiosity Drive
                else if personality.boldness > 0.62 && (11..=20).contains(&hour) && (h.wrapping_add(now_unix as u64)) % 5 == 0 {
                    let targets = [
                        ((0, 0), "文曜・中心樞紐石橋"),
                        ((1, 0), "文曜・三水口納涼棚廊"),
                        ((13, -13), "百工外坊・紅爐烈酒大飯鋪"),
                    ];
                    let idx = ((h.wrapping_add(hour as u64)) % 3) as usize;
                    let (coord, target_name) = targets[idx];
                    state.origin_name = prev_loc_name.clone();
                    state.dest = coord;
                    state.dest_name = target_name.to_string();
                    state.purpose = "探聽風物".to_string();
                    state.stay_activity = format!("正駐足{}與往來同道寒暄，打聽半島各處的新近消息", target_name);
                    state.stay_ticks_remaining = 18 + ((h % 8) as u8);
                    state.consecutive_work_streaks = 0;
                    override_handled = true;
                }

                if !override_handled {
                    if (7..=11).contains(&hour) || (13..=21).contains(&hour) {
                        state.work_errand_toggle = !state.work_errand_toggle;
                    } else {
                        state.work_errand_toggle = false;
                    }

                    let (next_dest, next_dest_name, next_purpose, next_stay_activity, next_ticks) =
                        resolve_wenyao_npc_schedule(slot, hour, state.work_errand_toggle, &prev_loc_name);

                    state.origin_name = prev_loc_name;
                    state.dest = next_dest;
                    state.dest_name = next_dest_name;
                    state.purpose = next_purpose;
                    state.stay_activity = next_stay_activity;
                    state.stay_ticks_remaining = next_ticks;
                }
            }
        }

        if state.current != state.dest {
            let next_coord = find_bfs_next_step(grid, state.current, state.dest)
                .unwrap_or(state.current);
            let prev_coord = state.current;
            state.current = next_coord;

            return Some((prev_coord, next_coord));
        }

        Some(((old_x, old_y), (old_x, old_y)))
    }

    /// 與指定正方格座標重合的實體 id（依 `entity_ids_in_room`）。
    pub fn entity_ids_at_grid(&self, x: i32, y: i32) -> Vec<String> {
        self.entity_ids_in_room(&crate::grid::grid_room_id_from_coord(x, y))
    }

    /// 有正方格座標的存活 NPC id 列表。
    pub fn get_npc_ids_with_grid(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.entities
            .values()
            .filter(|e| e.kind == "npc" && e.vit > 0 && e.grid_x.is_some() && e.grid_y.is_some())
            .map(|e| e.id.clone())
            .collect();
        let live = LIVE_NPC_GRID_STATES.read();
        for id in live.keys() {
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }
        ids
    }

    pub fn transfer_magnesium(
        &mut self,
        from_id: &str,
        to_id: &str,
        amount: i32,
    ) -> anyhow::Result<()> {
        if amount <= 0 {
            anyhow::bail!("轉帳鎂數須為正");
        }
        let from_mg = self
            .entities
            .get(from_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉出實體"))?
            .magnesium;
        let _to = self
            .entities
            .get(to_id)
            .ok_or_else(|| anyhow::anyhow!("找不到轉入實體"))?;
        if from_mg < amount {
            anyhow::bail!("鎂不足");
        }
        self.entities.get_mut(from_id).unwrap().magnesium = from_mg - amount;
        let to_mg = self.entities.get(to_id).unwrap().magnesium;
        self.entities.get_mut(to_id).unwrap().magnesium = to_mg + amount;
        if let Some(from_e) = self.entities.get(from_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(from_e.clone()));
        }
        if let Some(to_e) = self.entities.get(to_id) {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::UpsertEntity(to_e.clone()));
        }
        self.persist_entities()
    }

    pub fn get_entities_in_box(
        &self,
        x_min: i32,
        x_max: i32,
        y_min: i32,
        y_max: i32,
        kind: &str,
    ) -> Vec<Entity> {
        self.entities
            .values()
            .filter(|e| {
                (kind.is_empty() || e.kind == kind)
                    && e.x >= x_min
                    && e.x <= x_max
                    && e.y >= y_min
                    && e.y <= y_max
            })
            .cloned()
            .collect()
    }

    pub fn get_moving_entity_ids(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.move_state == "moving")
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn npc_ids_with_missing_soul_seed(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && e.soul_seed.is_none())
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// 回傳缺少方格座標（`grid_x` 或 `grid_y` 為 None）的 NPC id 列表。
    pub fn npc_ids_missing_grid(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|(_, e)| e.kind == "npc" && (e.grid_x.is_none() || e.grid_y.is_none()))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn clear_all_entities(&mut self) -> anyhow::Result<()> {
        self.entities.clear();
        self.entity_rooms.clear();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let _ = conn.execute("DELETE FROM entities", &[]);
            let _ = conn.execute("DELETE FROM entity_rooms", &[]);
        }
        self.persist_entities()?;
        self.persist_entity_rooms()
    }
}

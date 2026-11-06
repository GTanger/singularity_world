//! 白塔半島百大日常劇本切面總集（6 大類・108 齣具體日常劇本切面）
//!
//! 由《白塔坦頡的故事線》全十章 38 段、15 座聚落 34 戶當家與《白塔半島千物總典》1,120 件世界物件推導生成。

#[derive(Debug, Clone, Copy)]
pub struct PeninsulaScriptEntry {
    pub id: &'static str,
    pub category_num: &'static str,
    pub category_tag: &'static str,
    pub category_name: &'static str,
    pub title: &'static str,
    pub location_and_npc: &'static str,
    pub key_objects: &'static str,
    pub trigger_and_causal_chain: &'static str,
    pub player_actions: &'static str,
    pub outcome_and_world_change: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPack {
    pub id: &'static str,
    pub pack_id: &'static str,
    pub status: &'static str,
    pub category_num: &'static str,
    pub category_tag: &'static str,
    pub category_name: &'static str,
    pub room_tags: Vec<String>,
    pub key_objects: &'static str,
    pub event_title: String,
    pub trigger_and_causal_chain: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptPack {
    pub id: &'static str,
    pub pack_id: &'static str,
    pub current_step: usize,
    pub title: String,
    pub host_slot_actor: String,
    pub partner_slot_actor: String,
    pub location_context: String,
    pub player_actions: String,
    pub outcome_and_world_change: String,
}

pub const EXEMPLAR_NAMES: &[&str] = &[
    "秦水生", "陳萬福", "方石", "方赤土", "梁千木", "林正雄", "許海音", "蘇錦心", "牧長風", "韓鐵山",
    "田百稼", "周萬生", "石萬鈞", "岑守淵", "雷千鈞", "羅烈", "鄭烏石", "顧海平", "洪守明", "許萬川",
    "陸秉衡", "岑守岳", "岑拓", "柯成器", "沈清河", "白泉生", "溫晴", "袁山茶", "葉青囊", "阮水生",
    "馬山行", "高望雲", "陸知微", "沈硯舟", "陸承禮", "陸長貴", "陸敬軒", "秦明誠", "林時中", "趙衡宇",
    "周敬修", "岑雲野", "許阿甘", "馬守成", "溫懷瑾", "馬振遠", "田長耕", "趙觀星", "周一刀", "馬補靴",
];


include!("peninsula_scripts/scripts_part_1.rs");
include!("peninsula_scripts/scripts_part_2.rs");

const TOTAL_SCRIPTS: usize = 108;

const fn concat_scripts() -> [PeninsulaScriptEntry; TOTAL_SCRIPTS] {
    let mut result = [SCRIPTS_PART_1[0]; TOTAL_SCRIPTS];
    let parts: [&[PeninsulaScriptEntry]; 2] = [SCRIPTS_PART_1, SCRIPTS_PART_2];
    let mut i = 0;
    let mut part_idx = 0;
    while part_idx < parts.len() {
        let part = parts[part_idx];
        let mut j = 0;
        while j < part.len() {
            result[i] = part[j];
            i += 1;
            j += 1;
        }
        part_idx += 1;
    }
    result
}

static ALL_PENINSULA_DAILY_SCRIPTS: [PeninsulaScriptEntry; TOTAL_SCRIPTS] = concat_scripts();
pub const PENINSULA_DAILY_SCRIPTS: &[PeninsulaScriptEntry] = &ALL_PENINSULA_DAILY_SCRIPTS;

include!("peninsula_scripts/methods.rs");
include!("peninsula_scripts/tests.rs");

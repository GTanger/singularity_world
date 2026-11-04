// wordplate 模組 — 500 詞 ＋ 360 星盤 × 隨身【詞匣】 × 田忌賽馬四戰術序位核心系統。
// 嚴守台灣正體中文，徹底拔除 Codex 偽科幻詞彙；後台資料嚴謹，前台體感去數值化。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// 詞盤與詞匣系統領域錯誤分類（符合 RIVP 錯誤分類規範）
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WordPlateError {
    #[error("字訣不存在：{0}")]
    WordNotFound(String),
    #[error("字匣中字訣數量不足：{0}（現有 {1}，需求 {2}）")]
    InsufficientWordCount(String, u32, u32),
    #[error("星盤節點不可鑲嵌：{0}")]
    NodeNotInlayable(String),
    #[error("非法星盤節點編號：{0}")]
    InvalidNodeId(String),
    #[error("造句字數不符規則：長度 {0}（僅允許三言、五言、七言）")]
    InvalidSentenceLength(usize),
    #[error("星盤節點重複鑲嵌：{0}")]
    DuplicateNodeInlay(String),
    #[error("設定檔解析失敗：{0}")]
    ConfigParseError(String),
    #[error("戰術槽位未配置：{0}")]
    TacticalSlotEmpty(String),
}

/// 載體適配列舉（6 種武器與身法親和形態）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CarrierAffinity {
    #[default]
    Unarmed,
    Blade,
    Pole,
    Blunt,
    Shield,
    Footwork,
}

impl CarrierAffinity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Unarmed => "徒手拳掌",
            Self::Blade => "刀劍鋒芒",
            Self::Pole => "長槍大戟",
            Self::Blunt => "重錘鈍器",
            Self::Shield => "堅盾重禦",
            Self::Footwork => "步法身形",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Unarmed => "unarmed",
            Self::Blade => "blade",
            Self::Pole => "pole",
            Self::Blunt => "blunt",
            Self::Shield => "shield",
            Self::Footwork => "footwork",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "unarmed" => Some(Self::Unarmed),
            "blade" => Some(Self::Blade),
            "pole" => Some(Self::Pole),
            "blunt" => Some(Self::Blunt),
            "shield" => Some(Self::Shield),
            "footwork" => Some(Self::Footwork),
            _ => None,
        }
    }
}

/// 器物無分工具與武器（手持即載體）：依據器物名稱與識別碼，推論其物理形態與載體親和度
pub fn infer_item_carrier_affinity(item_id: &str, item_name: &str) -> Option<CarrierAffinity> {
    let text = format!("{item_id} {item_name}").to_lowercase();

    // 1. 撬棍為重鈍器，先判以防被「棍」搶先判入長兵
    if text.contains("撬棍") || text.contains("crowbar") {
        return Some(CarrierAffinity::Blunt);
    }
    // 鍋蓋為盾牌，先判以防被「鍋」搶先判入重鈍器
    if text.contains("鍋蓋") {
        return Some(CarrierAffinity::Shield);
    }

    // 堅盾重禦
    let shield_keys = ["盾", "傘", "牌", "蓋", "砧板", "背簍", "shield", "umbrella"];
    if shield_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Shield);
    }

    // 步法身形
    let foot_keys = ["靴", "鞋", "屐", "綁腿", "扇", "風氅", "斗篷", "boots", "shoes", "cloak", "fan"];
    if foot_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Footwork);
    }

    // 徒手拳掌
    let unarmed_keys = ["掌套", "手套", "護腕", "銀針", "握石", "指環", "glove", "bracer", "ring", "needle", "fist"];
    if unarmed_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Unarmed);
    }

    // 刀劍鋒芒
    let blade_keys = ["刀", "劍", "匕", "刃", "剪", "鋸", "鑿", "刨", "鍘", "鐮", "blade", "sword", "knife", "dagger", "saw", "shear", "sickle"];
    if blade_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Blade);
    }

    // 長槍大戟
    let pole_keys = ["槍", "矛", "戟", "棍", "杖", "竿", "槳", "鍬", "鏟", "鋤", "長柄", "飛爪", "鞭", "spear", "pole", "staff", "rod", "whip", "hoe", "spade", "shovel"];
    if pole_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Pole);
    }

    // 重錘鈍器
    let blunt_keys = ["鎬", "錘", "斧", "杵", "砧", "棒", "錨", "鍋", "釜", "磚", "hammer", "axe", "mace", "club", "pick", "anvil", "pot", "brick"];
    if blunt_keys.iter().any(|&k| text.contains(k)) {
        return Some(CarrierAffinity::Blunt);
    }

    None
}

/// 作用目標部位列舉（6 種周天身軀與兵刃部位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TargetPart {
    #[serde(rename = "head")]
    Head,
    #[default]
    #[serde(rename = "torso")]
    Torso,
    #[serde(rename = "arm")]
    Arm,
    #[serde(rename = "leg")]
    Leg,
    #[serde(rename = "weapon")]
    Weapon,
    #[serde(rename = "self")]
    SelfTarget,
}

impl TargetPart {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Head => "頭顱要害",
            Self::Torso => "軀幹心脈",
            Self::Arm => "雙臂關節",
            Self::Leg => "下盤腿足",
            Self::Weapon => "手中兵刃",
            Self::SelfTarget => "自身周天",
        }
    }
}

/// 五源分類列舉（採獵與結晶生成之五行五態源質）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WordSource {
    Beast,
    Plant,
    Mineral,
    Water,
    Qi,
}

impl WordSource {
    pub fn from_code_or_tag(s: &str) -> Option<Self> {
        match s {
            "beast" | "獸" => Some(Self::Beast),
            "plant" | "植" => Some(Self::Plant),
            "mineral" | "礦" => Some(Self::Mineral),
            "water" | "水" => Some(Self::Water),
            "qi" | "氣" => Some(Self::Qi),
            _ => None,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Beast => "beast",
            Self::Plant => "plant",
            Self::Mineral => "mineral",
            Self::Water => "water",
            Self::Qi => "qi",
        }
    }

    pub fn tag_zh(&self) -> &'static str {
        match self {
            Self::Beast => "獸",
            Self::Plant => "植",
            Self::Mineral => "礦",
            Self::Water => "水",
            Self::Qi => "氣",
        }
    }

    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Beast => "獸源",
            Self::Plant => "植源",
            Self::Mineral => "礦源",
            Self::Water => "水源",
            Self::Qi => "氣源",
        }
    }
}

/// 詞元品質階位列舉（丙品、乙品、甲品）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WordQuality {
    #[default]
    Bing,
    Yi,
    Jia,
}

impl WordQuality {
    pub fn from_code(s: &str) -> Option<Self> {
        match s {
            "bing" | "丙" | "丙品" => Some(Self::Bing),
            "yi" | "乙" | "乙品" => Some(Self::Yi),
            "jia" | "甲" | "甲品" => Some(Self::Jia),
            _ => None,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Bing => "bing",
            Self::Yi => "yi",
            Self::Jia => "jia",
        }
    }

    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Bing => "丙品",
            Self::Yi => "乙品",
            Self::Jia => "甲品",
        }
    }

    pub fn power_multiplier(&self) -> f32 {
        match self {
            Self::Bing => 1.0,
            Self::Yi => 1.25,
            Self::Jia => 1.55,
        }
    }

    pub fn magnesium_value(&self) -> i32 {
        match self {
            Self::Bing => 12,
            Self::Yi => 28,
            Self::Jia => 60,
        }
    }
}

/// 生成字晶物品編號
pub fn crystal_item_id(source: WordSource, quality: WordQuality) -> String {
    format!("crystal_{}_{}", source.code(), quality.code())
}

/// 解析字晶物品編號為 (源質, 品質)
pub fn parse_crystal_item_id(item_id: &str) -> Option<(WordSource, WordQuality)> {
    let s = item_id.strip_prefix("crystal_")?;
    let (src_str, qual_str) = s.rsplit_once('_')?;
    let source = WordSource::from_code_or_tag(src_str)?;
    let quality = WordQuality::from_code(qual_str)?;
    Some((source, quality))
}

/// SplitMix64 偽隨機混淆器
pub fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

/// 剝名結晶品質判定
pub fn roll_strip_quality(harvest_score: f32, material_tier: u8, is_whole_corpse: bool, seed: u64) -> WordQuality {
    let rand_val = (splitmix64(seed) % 100) as u32;
    let jia_threshold = 6
        + (material_tier as u32) * 4
        + (harvest_score * 12.0) as u32
        + if is_whole_corpse { 10 } else { 0 };
    let yi_threshold = 28
        + (material_tier as u32) * 8
        + (harvest_score * 20.0) as u32
        + if is_whole_corpse { 15 } else { 0 };

    if rand_val < jia_threshold {
        WordQuality::Jia
    } else if rand_val < yi_threshold {
        WordQuality::Yi
    } else {
        WordQuality::Bing
    }
}

/// 碎晶提出成數計算（3..=9，官方機構保底 7）
pub fn compute_extraction_ratio(forge_score: f32, is_official_agency: bool) -> u8 {
    let base = 3.0 + (forge_score * 4.0);
    let clamped = base.round().clamp(3.0, 9.0) as u8;
    if is_official_agency {
        clamped.max(7).min(9)
    } else {
        clamped
    }
}

/// 12 維連續向量：4D 能量形態 ＋ 3D 田忌賽馬時序 ＋ 5D 負載效應
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Vector12D {
    // 4D 能量形態
    pub focus: f64,      // 聚念
    pub shock: f64,      // 震盪
    pub flow: f64,       // 流轉
    pub sustain: f64,    // 持恆

    // 3D 田忌賽馬時序節奏
    pub tempo_early: f64, // 起手先發
    pub tempo_mid: f64,   // 中盤交鋒
    pub tempo_late: f64,  // 收尾後勁

    // 5D 負載效應
    pub penetration: f64, // 破甲
    pub reach: f64,       // 範圍
    pub vitality: f64,    // 調息
    pub qi_cost: f64,     // 消耗
    pub recoil: f64,      // 反震
}

impl Vector12D {
    pub fn is_non_zero(&self) -> bool {
        self.focus > 0.0
            || self.shock > 0.0
            || self.flow > 0.0
            || self.sustain > 0.0
            || self.tempo_early > 0.0
            || self.tempo_mid > 0.0
            || self.tempo_late > 0.0
            || self.penetration > 0.0
            || self.reach > 0.0
            || self.vitality > 0.0
            || self.qi_cost > 0.0
            || self.recoil > 0.0
    }

    pub fn to_array(&self) -> [f64; 12] {
        [
            self.focus,
            self.shock,
            self.flow,
            self.sustain,
            self.tempo_early,
            self.tempo_mid,
            self.tempo_late,
            self.penetration,
            self.reach,
            self.vitality,
            self.qi_cost,
            self.recoil,
        ]
    }
}

impl std::ops::Index<usize> for Vector12D {
    type Output = f64;
    fn index(&self, index: usize) -> &Self::Output {
        match index {
            0 => &self.focus,
            1 => &self.shock,
            2 => &self.flow,
            3 => &self.sustain,
            4 => &self.tempo_early,
            5 => &self.tempo_mid,
            6 => &self.tempo_late,
            7 => &self.penetration,
            8 => &self.reach,
            9 => &self.vitality,
            10 => &self.qi_cost,
            11 => &self.recoil,
            _ => panic!("Vector12D index out of bounds: {}", index),
        }
    }
}

impl std::ops::IndexMut<usize> for Vector12D {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match index {
            0 => &mut self.focus,
            1 => &mut self.shock,
            2 => &mut self.flow,
            3 => &mut self.sustain,
            4 => &mut self.tempo_early,
            5 => &mut self.tempo_mid,
            6 => &mut self.tempo_late,
            7 => &mut self.penetration,
            8 => &mut self.reach,
            9 => &mut self.vitality,
            10 => &mut self.qi_cost,
            11 => &mut self.recoil,
            _ => panic!("Vector12D index out of bounds: {}", index),
        }
    }
}

/// 相容舊版 6 維戰鬥維度
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GameDimsCompat6D {
    pub atk: f64,
    pub def: f64,
    pub spd: f64,
    pub dot: f64,
    pub aoe: f64,
    pub heal: f64,
}

/// 4 維百工與行旅向量（一字雙境：在戰陣為武相，在日常為藝相）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct CraftDims4D {
    pub forge: f64,    // 鍛鑄百工（打鐵、織造、修繕、結晶提出成數）
    pub culinary: f64, // 庖廚丹膳（山海合味烹飪、煎藥、焙茶、釀酒）
    pub harvest: f64,  // 採獵剝名（十九獵場狩獵部位、屍體剝名甲乙品階率、農礦採集）
    pub wayfare: f64,  // 山海行旅（黑水洋流航行、險坡迷霧行旅、星盤內視探明）
}

impl CraftDims4D {
    pub fn is_non_zero(&self) -> bool {
        self.forge > 0.0 || self.culinary > 0.0 || self.harvest > 0.0 || self.wayfare > 0.0
    }

    pub fn primary_aspect(&self) -> CraftAspect {
        let mut best_aspect = CraftAspect::Forge;
        let mut max_val = self.forge;

        if self.culinary > max_val {
            max_val = self.culinary;
            best_aspect = CraftAspect::Culinary;
        }
        if self.harvest > max_val {
            max_val = self.harvest;
            best_aspect = CraftAspect::Harvest;
        }
        if self.wayfare > max_val {
            best_aspect = CraftAspect::Wayfare;
        }

        best_aspect
    }
}

/// 百工藝相分類列舉
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftAspect {
    Forge,
    Culinary,
    Harvest,
    Wayfare,
}

impl CraftAspect {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Forge => "鍛鑄百工",
            Self::Culinary => "庖廚丹膳",
            Self::Harvest => "採獵百工",
            Self::Wayfare => "山海行旅",
        }
    }
}

/// 星軌通暢度（五階感官體感，前端表面去數值化）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum OrbitFluency {
    ChangLiu = 5, // 暢流（阻力極低）
    ShunTong = 4, // 順通
    PingWen = 3,  // 平穩
    ZhiSe = 2,    // 滯澀
    XianZu = 1,   // 險阻（阻力極高）
}

impl OrbitFluency {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ChangLiu => "暢流",
            Self::ShunTong => "順通",
            Self::PingWen => "平穩",
            Self::ZhiSe => "滯澀",
            Self::XianZu => "險阻",
        }
    }

    /// 後台阻力數值映射至感官五階
    pub fn from_cost(cost: f64) -> Self {
        if cost <= 7.0 {
            Self::ChangLiu
        } else if cost <= 11.0 {
            Self::ShunTong
        } else if cost <= 16.0 {
            Self::PingWen
        } else if cost <= 21.0 {
            Self::ZhiSe
        } else {
            Self::XianZu
        }
    }

    /// 能量轉化效率倍率
    pub fn efficiency_multiplier(&self) -> f64 {
        match self {
            Self::ChangLiu => 1.25,
            Self::ShunTong => 1.10,
            Self::PingWen => 1.00,
            Self::ZhiSe => 0.85,
            Self::XianZu => 0.70,
        }
    }

    /// 反衝反震減免系數（暢流星軌反震最低）
    pub fn recoil_mitigation(&self) -> f64 {
        match self {
            Self::ChangLiu => 0.60,
            Self::ShunTong => 0.80,
            Self::PingWen => 1.00,
            Self::ZhiSe => 1.25,
            Self::XianZu => 1.50,
        }
    }
}

/// 拓撲經絡阻力階級（三階感官體相，內部運算使用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CostTier {
    Smooth, // 暢流
    Normal, // 平穩 / 順通
    Severe, // 險阻
}

impl CostTier {
    pub fn from_cost(cost: f64) -> Self {
        if cost <= 9.0 {
            Self::Smooth
        } else if cost <= 18.0 {
            Self::Normal
        } else {
            Self::Severe
        }
    }
}

/// 依據八大主要經脈分群索引取得地文感官名稱（無代號、無數字）
pub fn group_meridian_name(g: usize) -> &'static str {
    match g % 8 {
        0 => "手太陰肺脈",
        1 => "足少陽膽脈",
        2 => "足厥陰肝脈",
        3 => "手陽明大腸脈",
        4 => "足少陰腎脈",
        5 => "手少陰心脈",
        6 => "任督中樞",
        7 => "帶脈",
        _ => "經脈支絡",
    }
}

/// 單字詞元型別別名
pub type WordEntry = WordElement;

/// 戰鬥架勢狀態（三階動態架勢：架勢沉穩、步履微晃、架勢潰散）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PostureState {
    #[default]
    Steady,
    Shaken,
    Broken,
}

impl PostureState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steady => "架勢沉穩",
            Self::Shaken => "步履微晃",
            Self::Broken => "架勢潰散",
        }
    }

    pub fn degrade(self) -> Self {
        match self {
            Self::Steady => Self::Shaken,
            Self::Shaken | Self::Broken => Self::Broken,
        }
    }
}

/// 戰場氣機殘留（五類天地氣場交互態）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BattlefieldResidue {
    ScorchingHeat, // 熾烈燥氣
    FreezingFrost, // 凝寒霜氣
    DeepTremor,    // 沉雄暗震
    GaleEdge,      // 疾風銳勢
    VortexPull,    // 牽引旋流
}

impl BattlefieldResidue {
    pub fn label(self) -> &'static str {
        match self {
            Self::ScorchingHeat => "熾烈燥氣",
            Self::FreezingFrost => "凝寒霜氣",
            Self::DeepTremor => "沉雄暗震",
            Self::GaleEdge => "疾風銳勢",
            Self::VortexPull => "牽引旋流",
        }
    }

    pub fn keyword(self) -> &'static str {
        match self {
            Self::ScorchingHeat => "燥",
            Self::FreezingFrost => "寒",
            Self::DeepTremor => "震",
            Self::GaleEdge => "疾風",
            Self::VortexPull => "旋流",
        }
    }
}

/// 氣機殘留交互（順應共鳴或煞氣逆沖）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResidueInteraction {
    Resonance, // 順應共鳴
    Backlash,  // 煞氣逆沖
}

impl ResidueInteraction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Resonance => "順應現場殘留氣機，借勢共鳴而威勢倍增",
            Self::Backlash => "強行逆沖現場殘留氣場，經脈受激致架勢微滯",
        }
    }
}

/// 單字詞元資料結構
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordElement {
    pub id: String,
    pub ch: String,
    pub semantic: String,
    pub sources: Vec<String>,
    pub desc: String,
    pub embedding_desc: Option<String>,
    pub game_dims: GameDimsCompat6D,
    pub vector_12d: Vector12D,
    pub craft_dims: CraftDims4D,
    pub carrier_affinity: CarrierAffinity,
    pub target_part: TargetPart,
}
/// 全服 500 詞元快取註冊表
pub struct WordRegistry {
    by_id: HashMap<String, WordElement>,
    by_char: HashMap<String, Vec<String>>,
    ordered_ids: Vec<String>,
}

const RAW_WORDS_JSON: &str = include_str!("../../data/config/word_elements.json");
const RAW_GAMEDIMS_JSON: &str = include_str!("../../data/config/word_elements_gamedims.json");

static GLOBAL_REGISTRY: LazyLock<WordRegistry> = LazyLock::new(|| {
    WordRegistry::from_json_str(RAW_WORDS_JSON, RAW_GAMEDIMS_JSON)
        .expect("內嵌 500 詞元設定檔解析不可失敗")
});

impl WordRegistry {
    /// 獲取全局靜態快取的詞元庫
    pub fn global() -> &'static WordRegistry {
        &GLOBAL_REGISTRY
    }

    /// 從 JSON 字串解析並建立註冊表
    pub fn from_json_str(words_json: &str, gamedims_json: &str) -> Result<Self, WordPlateError> {
        #[derive(Deserialize)]
        struct RawWordsDoc {
            elements: Vec<RawWordEntry>,
        }
        #[derive(Deserialize)]
        struct RawWordEntry {
            id: String,
            #[serde(rename = "char")]
            ch: String,
            semantic: String,
            sources: Vec<String>,
            desc: String,
            #[serde(default)]
            embedding_desc: Option<String>,
        }

        #[derive(Deserialize)]
        struct RawGamedimsDoc {
            elements: Vec<RawGamedimsEntry>,
        }
        #[derive(Deserialize)]
        struct RawGamedimsEntry {
            id: String,
            game_dims: GameDimsCompat6D,
            vector_12d: Vector12D,
            #[serde(default)]
            craft_dims: CraftDims4D,
            carrier_affinity: CarrierAffinity,
            target_part: TargetPart,
        }
        let words_doc: RawWordsDoc = serde_json::from_str(words_json)
            .map_err(|e| WordPlateError::ConfigParseError(format!("word_elements: {e}")))?;
        let gamedims_doc: RawGamedimsDoc = serde_json::from_str(gamedims_json)
            .map_err(|e| WordPlateError::ConfigParseError(format!("word_elements_gamedims: {e}")))?;

        let mut gamedims_map: HashMap<String, RawGamedimsEntry> =
            HashMap::with_capacity(gamedims_doc.elements.len());
        for entry in gamedims_doc.elements {
            gamedims_map.insert(entry.id.clone(), entry);
        }

        let mut by_id = HashMap::with_capacity(words_doc.elements.len());
        let mut by_char: HashMap<String, Vec<String>> = HashMap::new();
        let mut ordered_ids = Vec::with_capacity(words_doc.elements.len());

        for w in words_doc.elements {
            let gd = gamedims_map
                .remove(&w.id)
                .ok_or_else(|| WordPlateError::ConfigParseError(format!("字訣 ID 於 gamedims 中遺失：{}", w.id)))?;

            let elem = WordElement {
                id: w.id.clone(),
                ch: w.ch.clone(),
                semantic: w.semantic,
                sources: w.sources,
                desc: w.desc,
                embedding_desc: w.embedding_desc,
                game_dims: gd.game_dims,
                vector_12d: gd.vector_12d,
                craft_dims: gd.craft_dims,
                carrier_affinity: gd.carrier_affinity,
                target_part: gd.target_part,
            };
            by_char.entry(w.ch).or_default().push(w.id.clone());
            ordered_ids.push(w.id.clone());
            by_id.insert(w.id, elem);
        }

        Ok(Self {
            by_id,
            by_char,
            ordered_ids,
        })
    }

    pub fn get_by_id(&self, id: &str) -> Option<&WordElement> {
        self.by_id.get(id)
    }

    pub fn get_by_char(&self, ch: &str) -> Option<&[String]> {
        self.by_char.get(ch).map(|v| v.as_slice())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &WordElement> {
        self.by_id.values()
    }

    pub fn ordered_elements(&self) -> impl Iterator<Item = &WordElement> {
        self.ordered_ids.iter().filter_map(|id| self.by_id.get(id))
    }

    /// 依五源分類獲取所屬詞元清單
    pub fn words_by_source(&self, source: WordSource) -> Vec<&WordElement> {
        let tag = source.tag_zh();
        self.ordered_elements()
            .filter(|w| w.sources.iter().any(|s| s == tag))
            .collect()
    }

    /// 從指定源質庫中依種子與品質確定性隨機抽取一個詞元
    pub fn roll_word_from_source(&self, source: WordSource, quality: WordQuality, seed: u64) -> &WordElement {
        let candidates = self.words_by_source(source);
        if candidates.is_empty() {
            panic!("源質庫中無任何詞元：{:?}", source);
        }
        let mixed_seed = seed ^ ((quality as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let rand_val = splitmix64(mixed_seed);
        let idx = (rand_val as usize) % candidates.len();
        candidates[idx]
    }
}

/// 隨身【詞匣】：自由收藏、增減、查詢玩家已擁有的單字詞元
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WordCasket {
    inventory: BTreeMap<String, u32>,
    #[serde(default)]
    pub qualities: BTreeMap<String, WordQuality>,
    #[serde(default)]
    pub charges: BTreeMap<String, u8>,
    #[serde(default)]
    pub extraction_ratios: BTreeMap<String, u8>,
    #[serde(default)]
    pub item_inscriptions: BTreeMap<String, Vec<String>>,
}

impl WordCasket {
    pub fn new() -> Self {
        Self {
            inventory: BTreeMap::new(),
            qualities: BTreeMap::new(),
            charges: BTreeMap::new(),
            extraction_ratios: BTreeMap::new(),
            item_inscriptions: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, word_id: impl Into<String>, count: u32) {
        if count == 0 {
            return;
        }
        *self.inventory.entry(word_id.into()).or_insert(0) += count;
    }

    pub fn add_extracted(&mut self, word_id: impl Into<String>, count: u32, quality: WordQuality, ratio: u8) {
        if count == 0 {
            return;
        }
        let wid = word_id.into();
        let current_quality = self.qualities.get(&wid).copied().unwrap_or(WordQuality::Bing);
        if quality > current_quality {
            self.qualities.insert(wid.clone(), quality);
        } else {
            self.qualities.entry(wid.clone()).or_insert(quality);
        }
        let current_ratio = self.extraction_ratios.get(&wid).copied().unwrap_or(0);
        if ratio > current_ratio {
            self.extraction_ratios.insert(wid.clone(), ratio);
        } else {
            self.extraction_ratios.entry(wid.clone()).or_insert(ratio);
        }
        self.charges.entry(wid.clone()).or_insert(0);
        *self.inventory.entry(wid).or_insert(0) += count;
    }

    pub fn quality_of(&self, word_id: &str) -> WordQuality {
        self.qualities.get(word_id).copied().unwrap_or_default()
    }

    pub fn charge_of(&self, word_id: &str) -> u8 {
        self.charges.get(word_id).copied().unwrap_or(0)
    }

    pub fn ratio_of(&self, word_id: &str) -> u8 {
        self.extraction_ratios.get(word_id).copied().unwrap_or(0)
    }

    pub fn remove(&mut self, word_id: &str, count: u32) -> Result<(), WordPlateError> {
        if count == 0 {
            return Ok(());
        }
        let current = self.inventory.get(word_id).copied().unwrap_or(0);
        if current < count {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                current,
                count,
            ));
        }
        if current == count {
            self.inventory.remove(word_id);
            self.qualities.remove(word_id);
            self.charges.remove(word_id);
            self.extraction_ratios.remove(word_id);
        } else {
            self.inventory.insert(word_id.to_string(), current - count);
        }
        Ok(())
    }

    pub fn count(&self, word_id: &str) -> u32 {
        self.inventory.get(word_id).copied().unwrap_or(0)
    }

    pub fn has(&self, word_id: &str) -> bool {
        self.count(word_id) > 0
    }

    pub fn total_distinct(&self) -> usize {
        self.inventory.len()
    }

    pub fn total_count(&self) -> u32 {
        self.inventory.values().sum()
    }

    pub fn list(&self) -> Vec<(&str, u32)> {
        self.inventory.iter().map(|(k, v)| (k.as_str(), *v)).collect()
    }

    /// 同名疊煉：消耗同名詞元 1 枚，積蓄靈韻；靈韻充盈圓滿時觸發進階判定
    pub fn stack_refine(
        &mut self,
        word_id: &str,
        xuan_luck_bonus: f32,
        seed: u64,
    ) -> Result<(WordQuality, u8, bool, String), WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 2 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                current_count,
                2,
            ));
        }

        // 消耗同名詞元 1 枚
        self.remove(word_id, 1)?;

        let current_quality = self.quality_of(word_id);
        let current_charge = self.charge_of(word_id);
        let clamped_xuan = xuan_luck_bonus.clamp(0.0, 0.35);

        let rand_val = splitmix64(seed);
        let charge_gain = (20.0 + (rand_val % 21) as f32 + clamped_xuan * 40.0).round() as u8;
        let total_charge = (current_charge as u32) + (charge_gain as u32);

        if total_charge >= 100 {
            let ascend_rand = splitmix64(seed ^ 0xA5A5A5A55A5A5A5A);
            let roll = (ascend_rand % 100) as f32 / 100.0;
            let success_threshold = match current_quality {
                WordQuality::Bing => 0.60 + (clamped_xuan as f64 * 0.5) as f32, // 60%~77.5%
                WordQuality::Yi => 0.30 + (clamped_xuan as f64 * 0.4) as f32,   // 30%~44%
                WordQuality::Jia => 0.0, // 甲品已登極境，不再升階
            };

            if current_quality != WordQuality::Jia && roll < success_threshold {
                let next_quality = match current_quality {
                    WordQuality::Bing => WordQuality::Yi,
                    WordQuality::Yi => WordQuality::Jia,
                    WordQuality::Jia => WordQuality::Jia,
                };
                let remaining_charge = ((total_charge - 100) as u8).min(100);
                self.qualities.insert(word_id.to_string(), next_quality);
                self.charges.insert(word_id.to_string(), remaining_charge);
                let narrative = format!(
                    "同名真意交融共鳴，字訣【{}】由{}躍升為{}，靈韻相融，筆意愈發凝練。",
                    word_id,
                    current_quality.label_zh(),
                    next_quality.label_zh()
                );
                Ok((next_quality, remaining_charge, true, narrative))
            } else {
                // 未能升階或已至甲品極境
                let capped_charge = if current_quality == WordQuality::Jia {
                    100
                } else {
                    80.min(total_charge as u8)
                };
                self.charges.insert(word_id.to_string(), capped_charge);
                let narrative = if current_quality == WordQuality::Jia {
                    format!(
                        "字訣【{}】已臻甲品極境，同名相疊淬煉精華，靈韻充盈圓滿，一舉破繭昇華。",
                        word_id
                    )
                } else {
                    format!(
                        "同名真意衝擊極限未果，字訣【{}】維持{}，氣機微盪，靈韻略微回落。",
                        word_id,
                        current_quality.label_zh()
                    )
                };
                Ok((current_quality, capped_charge, false, narrative))
            }
        } else {
            let new_charge = total_charge as u8;
            self.charges.insert(word_id.to_string(), new_charge);
            let narrative = format!(
                "同名疊煉相融，字訣【{}】（{}）靈韻浸潤，筆意神彩更添一重。",
                word_id,
                current_quality.label_zh()
            );
            Ok((current_quality, new_charge, false, narrative))
        }
    }

    /// 鎔鑄化鎂：將 1 枚詞元鎔化為通用經濟資產「鎂」
    pub fn mint_to_magnesium(&mut self, word_id: &str) -> Result<(i32, WordQuality, String), WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 1 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                0,
                1,
            ));
        }
        let q = self.quality_of(word_id);
        let val = q.magnesium_value();
        self.remove(word_id, 1)?;
        let narrative = format!(
            "鎔鑄{}字訣【{}】，化歸本源，化為 {} 鎂。",
            q.label_zh(),
            word_id,
            val
        );
        Ok((val, q, narrative))
    }

    /// 外盤器物銘文：將詞匣中的詞元烙印至持物或防具上（最多 3 字，超額擠出最舊詞元歸還詞匣）
    pub fn inscribe_item(
        &mut self,
        item_id: impl Into<String>,
        word_id: &str,
    ) -> Result<Vec<String>, WordPlateError> {
        let current_count = self.count(word_id);
        if current_count < 1 {
            return Err(WordPlateError::InsufficientWordCount(
                word_id.to_string(),
                0,
                1,
            ));
        }
        self.remove(word_id, 1)?;
        let key = item_id.into();
        let mut popped_oldest = None;
        let result = {
            let list = self.item_inscriptions.entry(key).or_default();
            if list.len() >= 3 {
                popped_oldest = Some(list.remove(0));
            }
            list.push(word_id.to_string());
            list.clone()
        };
        if let Some(oldest) = popped_oldest {
            self.add(oldest, 1);
        }
        Ok(result)
    }

    /// 查詢特定器物之銘文字譜
    pub fn inscribed_words_of(&self, item_id: &str) -> &[String] {
        self.item_inscriptions
            .get(item_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 格式化器物銘文後綴（例如「 [薄][巧][解]」；無則回傳空字串）
    pub fn format_inscribed_suffix(&self, item_id: &str, registry: &WordRegistry) -> String {
        let words = self.inscribed_words_of(item_id);
        if words.is_empty() {
            return String::new();
        }
        let mut out = String::from(" ");
        for wid in words {
            let ch = registry
                .get_by_id(wid)
                .map(|e| e.ch.as_str())
                .unwrap_or_else(|| {
                    if wid.is_ascii() {
                        "銘"
                    } else {
                        wid.as_str()
                    }
                });
            out.push('[');
            out.push_str(ch);
            out.push(']');
        }
        out
    }

    /// 計算器物銘文在百工日常（打鐵、山海合味烹飪、採獵剝名、行旅）的四維增益
    pub fn inscribed_craft_bonus(&self, item_id: &str, registry: &WordRegistry) -> CraftDims4D {
        let mut total = CraftDims4D::default();
        for wid in self.inscribed_words_of(item_id) {
            if let Some(elem) = registry.get_by_id(wid) {
                let mult = self.quality_of(wid).power_multiplier() as f64 * 0.35;
                total.forge += elem.craft_dims.forge * mult;
                total.culinary += elem.craft_dims.culinary * mult;
                total.harvest += elem.craft_dims.harvest * mult;
                total.wayfare += elem.craft_dims.wayfare * mult;
            }
        }
        total
    }

    /// 計算器物銘文在戰陣交鋒時的（攻擊、破甲、禦勁）加成
    pub fn inscribed_combat_bonus(&self, item_id: &str, registry: &WordRegistry) -> (f64, f64, f64) {
        let mut atk = 0.0;
        let mut pen = 0.0;
        let mut def = 0.0;
        for wid in self.inscribed_words_of(item_id) {
            if let Some(elem) = registry.get_by_id(wid) {
                let mult = self.quality_of(wid).power_multiplier() as f64;
                let v = elem.vector_12d.to_array();
                atk += (v[0] + v[1] + v[2]) * mult;
                pen += (v[2] + v[3] + v[9]) * mult;
                def += (v[4] + v[5] + v[8]) * mult;
            }
        }
        (atk, pen, def)
    }
}

/// 解析並驗證星盤節點（N000 不可鑲字，N001..N360 全可鑲字）
pub fn parse_star_node(node_id: &str) -> Result<usize, WordPlateError> {
    if node_id == "N000" {
        return Err(WordPlateError::NodeNotInlayable(
            "N000（生之樞紐為本源核心，不可鑲嵌字訣）".to_string(),
        ));
    }
    if !node_id.starts_with('N') {
        return Err(WordPlateError::InvalidNodeId(node_id.to_string()));
    }
    let num_str = &node_id[1..];
    let num: usize = num_str
        .parse()
        .map_err(|_| WordPlateError::InvalidNodeId(node_id.to_string()))?;
    if !(1..=360).contains(&num) {
        return Err(WordPlateError::InvalidNodeId(node_id.to_string()));
    }
    Ok(num)
}

/// 單一星位鑲嵌資訊
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StarInlay {
    pub node_id: String,
    pub word_id: String,
    pub fluency: OrbitFluency,
}

impl StarInlay {
    pub fn new(node_id: impl Into<String>, word_id: impl Into<String>) -> Self {
        Self {
            node_id: node_id.into(),
            word_id: word_id.into(),
            fluency: OrbitFluency::PingWen,
        }
    }

    pub fn with_fluency(
        node_id: impl Into<String>,
        word_id: impl Into<String>,
        fluency: OrbitFluency,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            word_id: word_id.into(),
            fluency,
        }
    }
}

/// 合成後的招式心法特徵
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SynthesizedTechnique {
    pub canonical_name: String,
    pub custom_name: Option<String>,
    pub effective_name: String,
    pub length: usize,
    pub vector_12d: Vector12D,
    pub craft_dims: CraftDims4D,
    pub primary_craft: CraftAspect,
    pub primary_carrier: CarrierAffinity,
    pub primary_target: TargetPart,
    pub fluency_summary: OrbitFluency,
    #[serde(default)]
    pub word_chars: Vec<String>,
    #[serde(default)]
    pub motion_sequence: Vec<String>,
    #[serde(default)]
    pub meridian_detour_note: String,
    #[serde(default)]
    pub practice_feel: String,
    #[serde(default)]
    pub soul_seed: Option<i64>,
    #[serde(default)]
    pub practice_count: u32,
    #[serde(default)]
    pub anatomy_familiarity: u32,
    #[serde(default)]
    pub learned_delta: f64,
}

impl SynthesizedTechnique {
    /// 一字雙境總括描述（在戰陣為武相，在日常為藝相）
    pub fn dual_context_summary(&self) -> String {
        format!(
            "【交鋒武相】「{}」（{}／{}，偏向{}攻向{}，星軌{}）｜【百工藝相】主修{}（鍛鑄 {:.2}／丹膳 {:.2}／採獵 {:.2}／行旅 {:.2}）",
            self.effective_name,
            self.canonical_name,
            match self.length {
                3 => "三言心法",
                5 => "五言真訣",
                7 => "七言天篇",
                _ => "句式心法",
            },
            self.primary_carrier.label(),
            self.primary_target.label(),
            self.fluency_summary.label(),
            self.primary_craft.label(),
            self.craft_dims.forge,
            self.craft_dims.culinary,
            self.craft_dims.harvest,
            self.craft_dims.wayfare,
        )
    }
    /// 根據手頭實際器物載體，推導此招式的分解基礎碎招動作序列（如「沉腰蹲步 → 蹬地躍起 → 貼身上勾拳」或「踏步斜探 → 順紋上劈 → 反腕下砍」）
    pub fn derive_elementary_motions_for_carrier(
        &self,
        carrier_override: Option<CarrierAffinity>,
    ) -> Vec<String> {
        let carrier = carrier_override.unwrap_or(self.primary_carrier);
        if carrier == self.primary_carrier && !self.motion_sequence.is_empty() {
            return self.motion_sequence.clone();
        }
        derive_elementary_motions_with_fluency(
            &self.word_chars,
            carrier,
            self.primary_target,
            self.fluency_summary,
        )
    }

    /// 計算句法招式之共鳴階層（三階共鳴階梯）
    pub fn resonance_tier(&self) -> u8 {
        if self.length >= 5 && matches!(self.fluency_summary, OrbitFluency::ChangLiu | OrbitFluency::ShunTong) {
            3
        } else if self.length >= 3 && !matches!(self.fluency_summary, OrbitFluency::XianZu) {
            2
        } else {
            1
        }
    }

    /// 依共鳴階層提供之威力加成倍率
    pub fn resonance_multiplier(&self) -> f64 {
        match self.resonance_tier() {
            3 => 1.55,
            2 => 1.25,
            _ => 1.0,
        }
    }

    /// 檢視招式特徵與字印意象，推導施展後遺留於戰場之殘留氣場（BattlefieldResidue）
    pub fn emitted_residue(&self) -> Option<BattlefieldResidue> {
        if self.resonance_tier() < 2 {
            return None;
        }

        let has_char = |targets: &[&str]| -> bool {
            self.word_chars.iter().any(|wc| {
                targets.contains(&wc.as_str()) || targets.iter().any(|&t| wc.contains(t))
            })
        };

        if has_char(&["火", "烈", "炎", "焚", "灼", "陽", "熔"]) || self.vector_12d[4] > 0.35 {
            Some(BattlefieldResidue::ScorchingHeat)
        } else if has_char(&["冰", "寒", "霜", "雪", "凝", "冷", "陰"]) || self.vector_12d[5] > 0.35 {
            Some(BattlefieldResidue::FreezingFrost)
        } else if self.primary_carrier == CarrierAffinity::Blunt
            || self.vector_12d[1] > 0.45
            || self.vector_12d[6] > 0.4
        {
            Some(BattlefieldResidue::DeepTremor)
        } else if self.vector_12d[2] > 0.45 || self.vector_12d[0] > 0.45 {
            Some(BattlefieldResidue::GaleEdge)
        } else if self.vector_12d[7] > 0.35 {
            Some(BattlefieldResidue::VortexPull)
        } else {
            None
        }
    }

    /// 判定當前招式與上一序位交鋒遺留之戰場氣機殘留（BattlefieldResidue）之交互作用
    pub fn check_residue_interaction(&self, residue: BattlefieldResidue) -> Option<ResidueInteraction> {
        let emitted = self.emitted_residue();
        match residue {
            BattlefieldResidue::ScorchingHeat => {
                if emitted == Some(BattlefieldResidue::ScorchingHeat) || emitted == Some(BattlefieldResidue::GaleEdge) {
                    Some(ResidueInteraction::Resonance)
                } else if emitted == Some(BattlefieldResidue::FreezingFrost) {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::FreezingFrost => {
                if emitted == Some(BattlefieldResidue::FreezingFrost) || self.primary_carrier == CarrierAffinity::Blade {
                    Some(ResidueInteraction::Resonance)
                } else if emitted == Some(BattlefieldResidue::ScorchingHeat) {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::DeepTremor => {
                if self.primary_carrier == CarrierAffinity::Blunt
                    || emitted == Some(BattlefieldResidue::DeepTremor)
                    || self.primary_carrier == CarrierAffinity::Footwork
                {
                    Some(ResidueInteraction::Resonance)
                } else if self.primary_carrier == CarrierAffinity::Pole
                    && matches!(self.fluency_summary, OrbitFluency::ZhiSe | OrbitFluency::XianZu)
                {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::GaleEdge => {
                if emitted == Some(BattlefieldResidue::GaleEdge)
                    || self.primary_carrier == CarrierAffinity::Footwork
                    || emitted == Some(BattlefieldResidue::ScorchingHeat)
                {
                    Some(ResidueInteraction::Resonance)
                } else if self.primary_carrier == CarrierAffinity::Blunt
                    && (self.vector_12d[2] < 0.35 || matches!(self.fluency_summary, OrbitFluency::ZhiSe | OrbitFluency::XianZu))
                {
                    Some(ResidueInteraction::Backlash)
                } else {
                    None
                }
            }
            BattlefieldResidue::VortexPull => {
                if emitted == Some(BattlefieldResidue::VortexPull)
                    || self.primary_carrier == CarrierAffinity::Shield
                    || self.primary_carrier == CarrierAffinity::Unarmed
                {
                    Some(ResidueInteraction::Resonance)
                } else {
                    None
                }
            }
        }
    }
}

/// 由句中字印、器物載體、目標部位與星軌通暢度，確定性推導出三節基礎碎招動作（同法不同相）
pub fn derive_elementary_motions_with_fluency(
    word_chars: &[String],
    carrier: CarrierAffinity,
    target: TargetPart,
    fluency: OrbitFluency,
) -> Vec<String> {
    let seed_hash: usize = word_chars
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            ch.chars()
                .next()
                .map(|c| (c as usize).wrapping_mul(i + 3))
                .unwrap_or(i + 1)
        })
        .sum::<usize>()
        .wrapping_add((fluency as usize).wrapping_mul(17));
    let target_label = match target {
        TargetPart::Head => "面門",
        TargetPart::Torso => "胸脅",
        TargetPart::Arm => "腕肘",
        TargetPart::Leg => "下盤",
        TargetPart::Weapon => "兵刃握處",
        TargetPart::SelfTarget => "周身氣門",
    };
    let (step1_pool, step2_pool, step3_pool): (&[&str], &[&str], &[&str]) = match carrier {
        CarrierAffinity::Blade => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["行雲踏步", "流光抽刃", "側步掠鋒"],
                &["順紋上劈", "連環點刺", "靈動削脈"],
                &["旋刃卸骨", "順勢收鋒切入", "游刃截擊"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "沉腕咬刃"],
                &["硬折刀勢", "貼骨挫力", "遲滯挑筋"],
                &["反腕重劈", "力竭頓刃", "咬牙拖劈"],
            ),
            OrbitFluency::PingWen => (
                &["踏步斜探", "沉腕抽刃", "側步掠鋒"],
                &["順紋上劈", "貼骨挑筋", "斜削腕脈"],
                &["反腕下砍", "旋刃卸骨", "順勢收鋒切入"],
            ),
        },
        CarrierAffinity::Blunt => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["借勢旋身", "行雲提柄", "順風掄臂"],
                &["連綿翻砸", "沉柄引力", "斜掄順勢擊"],
                &["連環壓落", "借轉崩錘", "流勢推砸"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "沉腰死紮"],
                &["硬拗力道", "沉柄頂推", "截氣橫掄"],
                &["傾力重砸", "震臂崩砸", "頓地回擊"],
            ),
            OrbitFluency::PingWen => (
                &["沉腰踏步", "雙手提柄", "擰腰蓄臂"],
                &["橫掄壓頂", "沉柄推撞", "斜掄掃擊"],
                &["傾力重砸", "順勢沉器壓落", "震臂崩砸"],
            ),
        },
        CarrierAffinity::Pole => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["靈蛇探梢", "滑步流端", "隨風圈桿"],
                &["順桿遊撥", "如電連刺", "抖腕崩顫"],
                &["穿花點穴", "迴桿行雲劈", "順勢挑浪"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "頓步卡架"],
                &["硬別桿身", "拗力直刺", "粗重橫封"],
                &["透步攢刺", "咬牙下劈", "挫身頂桿"],
            ),
            OrbitFluency::PingWen => (
                &["撐步圈桿", "滑步探尖", "斂腕平端"],
                &["順桿攔撥", "抖腕直刺", "橫桿掃擊"],
                &["透步攢刺", "迴桿劈落", "順勢挑崩"],
            ),
        },
        CarrierAffinity::Shield => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["滑步迎隙", "隨風偏卸", "巧轉掩身"],
                &["遊刃引偏", "如水化勁", "借勢挑架"],
                &["反身推擊", "順勁壓身", "巧震退敵"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "死紮步位"],
                &["硬抗重轟", "鈍挫卡鎖", "貼肘死抵"],
                &["合身頂撞", "震肩反擊", "頓步爆發"],
            ),
            OrbitFluency::PingWen => (
                &["斂步橫架", "沉肩偏卸", "側身掩肘"],
                &["格開來勢", "卸力引偏", "貼肘封門"],
                &["合身頂撞", "震肩反擊", "沉肘崩開"],
            ),
        },
        CarrierAffinity::Footwork => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["踏波穿影", "借風旋身", "穿花繞隙"],
                &["行雲流轉", "騰挪如電", "落葉飄搖"],
                &["掠影切入死角", "凌空折向", "隨風旋踢"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "挫步重頓"],
                &["勉強側移", "踉蹌避鋒", "回旋沉紮"],
                &["回身掃腿", "借力飄退", "咬牙沉步切入"],
            ),
            OrbitFluency::PingWen => (
                &["側身滑步", "借勢旋身", "踏隙折轉"],
                &["繞行側翼", "縮步避鋒", "騰躍換位"],
                &["掠影切入死角", "借力飄退", "旋身掃擊下盤"],
            ),
        },
        CarrierAffinity::Unarmed => match fluency {
            OrbitFluency::ChangLiu | OrbitFluency::ShunTong => (
                &["流水推雲", "沾衣貼掌", "穿花走步"],
                &["順水推舟", "遊魚穿掌", "靈巧化摔"],
                &["寸勁疊爆", "游身拍按", "連環點穴"],
            ),
            OrbitFluency::ZhiSe | OrbitFluency::XianZu => (
                &["凝息迴轉", "借勢繞隙", "咬牙沉步"],
                &["死力鎖骨", "頂肘硬格", "挫腕擒拿"],
                &["貼身上勾拳", "雙掌吐勁", "沉肘重擊"],
            ),
            OrbitFluency::PingWen => (
                &["沉腰蹲步", "斂肘探掌", "錯步欺身"],
                &["蹬地躍起", "旋腕扣脈", "順勢格臂"],
                &["貼身上勾拳", "雙掌吐勁", "沉肘重擊"],
            ),
        },
    };
    let s1 = step1_pool[seed_hash % step1_pool.len()].to_string();
    let s2 = step2_pool[(seed_hash / 3) % step2_pool.len()].to_string();
    let s3_base = step3_pool[(seed_hash / 7) % step3_pool.len()];
    let s3 = format!("{s3_base}直取{target_label}");
    vec![s1, s2, s3]
}

/// 由句中字印、器物載體與目標部位，確定性推導出三節基礎碎招動作（起勢、承轉、收合）
pub fn derive_elementary_motions_from_chars(
    word_chars: &[String],
    carrier: CarrierAffinity,
    target: TargetPart,
) -> Vec<String> {
    derive_elementary_motions_with_fluency(word_chars, carrier, target, OrbitFluency::PingWen)
}


/// 星盤造句成相（三言／五言／七言句法）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSentence {
    pub inlays: Vec<StarInlay>,
    pub custom_name: Option<String>,
    #[serde(default)]
    pub soul_seed: Option<i64>,
    #[serde(default)]
    pub practice_count: Option<u32>,
    #[serde(default)]
    pub anatomy_familiarity: Option<u32>,
    #[serde(default)]
    pub learned_delta: Option<f64>,
}

impl StarSentence {
    /// 建立三言/五言/七言造句，嚴格校驗星盤節點與長度
    pub fn new(inlays: Vec<StarInlay>) -> Result<Self, WordPlateError> {
        let len = inlays.len();
        if len != 3 && len != 5 && len != 7 {
            return Err(WordPlateError::InvalidSentenceLength(len));
        }

        let mut seen_nodes = HashSet::with_capacity(len);
        for inlay in &inlays {
            let _ = parse_star_node(&inlay.node_id)?;
            if !seen_nodes.insert(&inlay.node_id) {
                return Err(WordPlateError::DuplicateNodeInlay(inlay.node_id.clone()));
            }
        }

        Ok(Self {
            inlays,
            custom_name: None,
            soul_seed: None,
            practice_count: None,
            anatomy_familiarity: None,
            learned_delta: None,
        })
    }
    /// 成相著譜定名
    pub fn with_custom_name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    pub fn set_custom_name(&mut self, name: impl Into<String>) {
        self.custom_name = Some(name.into());
    }

    pub fn with_soul_seed(mut self, seed: i64) -> Self {
        self.soul_seed = Some(seed);
        self
    }

    pub fn set_soul_seed(&mut self, seed: i64) {
        self.soul_seed = Some(seed);
    }

    /// 結合 12D 向量與星軌通暢度合成招式心法
    pub fn synthesize(&self, registry: &WordRegistry) -> Result<SynthesizedTechnique, WordPlateError> {
        let length = self.inlays.len();
        let length_mult = match length {
            3 => 1.0,
            5 => 1.8,
            7 => 2.8,
            _ => return Err(WordPlateError::InvalidSentenceLength(length)),
        };

        let words: Vec<&WordElement> = self
            .inlays
            .iter()
            .map(|inlay| {
                registry
                    .get_by_id(&inlay.word_id)
                    .ok_or_else(|| WordPlateError::WordNotFound(inlay.word_id.clone()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let sum_eff: f64 = self.inlays.iter().map(|i| i.fluency.efficiency_multiplier()).sum();
        let avg_eff = sum_eff / length as f64;

        let sum_recoil: f64 = self.inlays.iter().map(|i| i.fluency.recoil_mitigation()).sum();
        let avg_recoil = sum_recoil / length as f64;

        let sum_rank: usize = self.inlays.iter().map(|i| i.fluency as usize).sum();
        let avg_rank = (sum_rank as f64 / length as f64).round() as usize;
        let fluency_summary = match avg_rank {
            5 => OrbitFluency::ChangLiu,
            4 => OrbitFluency::ShunTong,
            3 => OrbitFluency::PingWen,
            2 => OrbitFluency::ZhiSe,
            _ => OrbitFluency::XianZu,
        };

        let mut focus = 0.0;
        let mut shock = 0.0;
        let mut flow = 0.0;
        let mut sustain = 0.0;
        let mut tempo_early = 0.0;
        let mut tempo_mid = 0.0;
        let mut tempo_late = 0.0;
        let mut penetration = 0.0;
        let mut reach = 0.0;
        let mut vitality = 0.0;
        let mut qi_cost = 0.0;
        let mut recoil = 0.0;

        let mut craft_forge = 0.0;
        let mut craft_culinary = 0.0;
        let mut craft_harvest = 0.0;
        let mut craft_wayfare = 0.0;

        for (idx, (inlay, word)) in self.inlays.iter().zip(words.iter()).enumerate() {
            let eff = inlay.fluency.efficiency_multiplier();
            let pos_ratio = idx as f64 / ((length.saturating_sub(1)).max(1) as f64);

            let (early_w, mid_w, late_w, focus_w, flow_w, shock_w, pen_w, reach_w, sustain_w) =
                if idx == 0 {
                    // 起／前段發端
                    (1.35, 0.90, 0.75, 1.25, 0.95, 0.85, 1.00, 0.90, 1.05)
                } else if idx + 1 == length {
                    // 合／末段收束釋放
                    (0.75, 0.95, 1.40, 0.90, 0.95, 1.30, 1.10, 1.25, 0.95)
                } else {
                    // 承轉／中段流轉變化
                    (0.90, 1.30, 0.95, 1.00, 1.25, 1.00, 1.20, 1.05, 1.15)
                };

            let v = &word.vector_12d;
            focus += v.focus * focus_w * eff;
            shock += v.shock * shock_w * eff;
            flow += v.flow * flow_w * eff;
            sustain += v.sustain * sustain_w * eff;
            tempo_early += v.tempo_early * early_w * eff;
            tempo_mid += v.tempo_mid * mid_w * eff;
            tempo_late += v.tempo_late * late_w * eff;
            penetration += v.penetration * pen_w * eff;
            reach += v.reach * reach_w * eff;
            vitality += v.vitality * eff;
            qi_cost += v.qi_cost;
            recoil += v.recoil * inlay.fluency.recoil_mitigation();

            // 累計百工與行旅維度（受時序與星軌通暢度影響）
            let cd = &word.craft_dims;
            let forge_w = 1.0 + 0.15 * pos_ratio;
            let culinary_w = 1.0 + 0.15 * pos_ratio;
            let harvest_w = 1.0 + 0.15 * (1.0 - pos_ratio);
            let wayfare_w = 1.0 + 0.15 * (1.0 - pos_ratio);

            craft_forge += cd.forge * forge_w * eff;
            craft_culinary += cd.culinary * culinary_w * eff;
            craft_harvest += cd.harvest * harvest_w * eff;
            craft_wayfare += cd.wayfare * wayfare_w * eff;
        }

        let combined_craft = CraftDims4D {
            forge: craft_forge * length_mult,
            culinary: craft_culinary * length_mult,
            harvest: craft_harvest * length_mult,
            wayfare: craft_wayfare * length_mult,
        };
        let primary_craft = combined_craft.primary_aspect();

        let combined_vector = Vector12D {
            focus: focus * length_mult,
            shock: shock * length_mult,
            flow: flow * length_mult,
            sustain: sustain * length_mult,
            tempo_early: tempo_early * length_mult,
            tempo_mid: tempo_mid * length_mult,
            tempo_late: tempo_late * length_mult,
            penetration: penetration * length_mult * avg_eff,
            reach: reach * length_mult * avg_eff,
            vitality: vitality * length_mult * avg_eff,
            qi_cost: qi_cost * length_mult,
            recoil: recoil * avg_recoil,
        };

        let mut carrier_counts = HashMap::new();
        for w in &words {
            *carrier_counts.entry(w.carrier_affinity).or_insert(0usize) += 1;
        }
        let primary_carrier = carrier_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(c, _)| c)
            .unwrap_or(CarrierAffinity::Unarmed);

        let mut target_counts = HashMap::new();
        for w in &words {
            *target_counts.entry(w.target_part).or_insert(0usize) += 1;
        }
        let primary_target = target_counts
            .into_iter()
            .max_by_key(|&(_, count)| count)
            .map(|(t, _)| t)
            .unwrap_or(TargetPart::Torso);

        fn semantic_weight(semantic: &str) -> i32 {
            match semantic {
                "動" => 10,
                "屬" => 9,
                "受" => 8,
                "構" => 7,
                "態" => 6,
                "補" => 5,
                "否" => 4,
                "主" => 3,
                "助" => 2,
                "玄" => 1,
                _ => 0,
            }
        }

        let mut indexed_weights: Vec<(usize, i32, &str)> = words
            .iter()
            .enumerate()
            .map(|(idx, w)| (idx, semantic_weight(&w.semantic), w.ch.as_str()))
            .collect();
        indexed_weights.sort_by(|a, b| b.1.cmp(&a.1));
        let keep_count = match length {
            3 => 2.min(indexed_weights.len()),
            5 => 3.min(indexed_weights.len()),
            _ => 4.min(indexed_weights.len()),
        };
        let mut selected = indexed_weights[..keep_count].to_vec();
        selected.sort_by_key(|(idx, _, _)| *idx);
        let canonical_name: String = selected.into_iter().map(|(_, _, ch)| ch).collect();

        let effective_name = self
            .custom_name
            .as_ref()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| canonical_name.clone());

        let word_chars: Vec<String> = words.iter().map(|w| w.ch.clone()).collect();
        let motion_sequence = derive_elementary_motions_with_fluency(
            &word_chars,
            primary_carrier,
            primary_target,
            fluency_summary,
        );

        let seed = self.soul_seed.unwrap_or(0);
        let meridian_detour_note = compute_auto_detour_meridian_note(&words, seed);
        let motions_feel = motion_sequence.join("、");
        let practice_feel = format!(
            "以{}之序演練，氣勢{}。{}",
            motions_feel,
            fluency_summary.label(),
            meridian_detour_note
        );

        Ok(SynthesizedTechnique {
            canonical_name,
            custom_name: self.custom_name.clone(),
            effective_name,
            length,
            vector_12d: combined_vector,
            craft_dims: combined_craft,
            primary_craft,
            primary_carrier,
            primary_target,
            fluency_summary,
            word_chars,
            motion_sequence,
            meridian_detour_note,
            practice_feel,
            soul_seed: self.soul_seed,
            practice_count: self.practice_count.unwrap_or(25),
            anatomy_familiarity: self.anatomy_familiarity.unwrap_or(20),
            learned_delta: self.learned_delta.unwrap_or(0.0),
        })
    }
}

/// 田忌賽馬四戰術序位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TacticalSlot {
    #[default]
    Opening,  // 起手式（爭奪先手與試探破綻）
    Midgame,  // 常態連攜（氣脈流轉與正面破甲）
    Finisher, // 破綻殺招（收尾絕殺與乾坤一擲）
    Defense,  // 守勢身法（卸勁固本與化解殺意）
}

impl TacticalSlot {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Opening => "起手式",
            Self::Midgame => "常態連攜",
            Self::Finisher => "破綻殺招",
            Self::Defense => "守勢身法",
        }
    }
}

/// 戰術配置方案（包含四戰術序位）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TacticalLoadout {
    pub opening: Option<StarSentence>,
    pub midgame: Option<StarSentence>,
    pub finisher: Option<StarSentence>,
    pub defense: Option<StarSentence>,
}

impl TacticalLoadout {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_slot(&mut self, slot: TacticalSlot, sentence: StarSentence) {
        match slot {
            TacticalSlot::Opening => self.opening = Some(sentence),
            TacticalSlot::Midgame => self.midgame = Some(sentence),
            TacticalSlot::Finisher => self.finisher = Some(sentence),
            TacticalSlot::Defense => self.defense = Some(sentence),
        }
    }

    pub fn get_slot(&self, slot: TacticalSlot) -> Option<&StarSentence> {
        match slot {
            TacticalSlot::Opening => self.opening.as_ref(),
            TacticalSlot::Midgame => self.midgame.as_ref(),
            TacticalSlot::Finisher => self.finisher.as_ref(),
            TacticalSlot::Defense => self.defense.as_ref(),
        }
    }

    /// 計算活躍槽位之百工四維總和（若均未配置則回傳全 0 向量）
    pub fn total_craft_dims(&self, registry: &WordRegistry) -> CraftDims4D {
        let mut sum = CraftDims4D::default();
        let slots = [&self.opening, &self.midgame, &self.finisher, &self.defense];
        for slot in slots.into_iter().flatten() {
            if let Ok(tech) = slot.synthesize(registry) {
                sum.forge += tech.craft_dims.forge;
                sum.culinary += tech.craft_dims.culinary;
                sum.harvest += tech.craft_dims.harvest;
                sum.wayfare += tech.craft_dims.wayfare;
            }
        }
        sum
    }

    /// 計算詞匣中擁有之玄意詞元帶來的幸運加成
    /// 每擁有 1 種不同的「玄」語意詞元提供 +0.025，上限為 +0.30
    pub fn xuan_luck_bonus(&self, casket: &WordCasket, registry: &WordRegistry) -> f32 {
        let mut xuan_count = 0_usize;
        for (word_id, count) in casket.list() {
            if count > 0 {
                if let Some(elem) = registry.get_by_id(word_id) {
                    if elem.semantic == "玄" {
                        xuan_count += 1;
                    }
                }
            }
        }
        ((xuan_count as f32) * 0.025).min(0.30)
    }
}

/// 生命種子（soul_seed）＋ 361 星盤（760 條星軌阻力）＝ 萬人萬相戰術配置建構
pub fn build_loadout_from_soul_seed(
    seed: i64,
    casket: Option<&WordCasket>,
    registry: &WordRegistry,
) -> TacticalLoadout {
    let costs = crate::db::expand_soul_seed_to_topology_costs(seed);
    let (alpha, beta, gamma) = crate::db::expand_soul_seed_to_combat_axes(seed);

    // 1. 計算 20 主經脈樞紐（Hub 1..=20）之阻力綜合評價
    let mut hub_scores: Vec<(usize, f64)> = (1..=20)
        .map(|h| {
            let a_cost = costs.get(h - 1).copied().unwrap_or(13.0);
            let b_start = 20 + 5 * (h - 1);
            let b_avg: f64 = (0..5)
                .map(|j| costs.get(b_start + j).copied().unwrap_or(13.0))
                .sum::<f64>()
                / 5.0;
            let total_resistance = a_cost * 0.4 + b_avg * 0.6;
            (h, total_resistance)
        })
        .collect();

    // 依阻力由低到高（最順通）排序
    hub_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // 取出最通暢的 4 條相異經脈樞紐，分別對應 [Opening, Midgame, Finisher, Defense]
    let selected_hubs = [
        hub_scores[0].0,
        hub_scores[1 % hub_scores.len()].0,
        hub_scores[2 % hub_scores.len()].0,
        hub_scores[3 % hub_scores.len()].0,
    ];

    // 2. 獲取備選詞元列表
    let casket_words: Vec<String> = casket
        .map(|c| {
            c.list()
                .into_iter()
                .filter(|(w, cnt)| {
                    *cnt > 0
                        && !matches!(
                            *w,
                            "qiao_shu" | "mu" | "he_tai" | "shui" | "huo_shou"
                        )
                })
                .map(|(w, _)| w.to_string())
                .collect()
        })
        .unwrap_or_default();

    let use_casket = casket_words.len() >= 4;

    let slots = [
        TacticalSlot::Opening,
        TacticalSlot::Midgame,
        TacticalSlot::Finisher,
        TacticalSlot::Defense,
    ];

    let mut loadout = TacticalLoadout::new();
    let mut used_word_ids = HashSet::new();

    for (slot_idx, &slot) in slots.iter().enumerate() {
        let hub = selected_hubs[slot_idx];

        // 在該經脈樞紐下選取 3 節點路徑：1 個 Hub 節點 + 1 個 Logic 節點 + 1 個 Peripheral 節點
        let node1 = format!("N{:03}", hub);
        let cost1 = costs.get(hub - 1).copied().unwrap_or(10.0);

        let b_base = 20 + 5 * (hub - 1);
        let mut best_j = 0;
        let mut min_b = f64::MAX;
        for j in 0..5 {
            let c = costs.get(b_base + j).copied().unwrap_or(13.0);
            if c < min_b {
                min_b = c;
                best_j = j;
            }
        }
        let node2 = format!("N{:03}", 20 + 5 * (hub - 1) + best_j + 1);
        let cost2 = min_b;

        let c_base = 120 + 12 * (hub - 1);
        let mut best_s = 0;
        let mut min_c = f64::MAX;
        for s in 0..12 {
            let c = costs.get(c_base + s).copied().unwrap_or(13.0);
            if c < min_c {
                min_c = c;
                best_s = s;
            }
        }
        let node3 = format!("N{:03}", 120 + 12 * (hub - 1) + best_s + 1);
        let cost3 = min_c;

        let nodes = [node1, node2, node3];
        let node_costs = [cost1, cost2, cost3];

        // 選取 3 個詞元
        let mut chosen_words = Vec::with_capacity(3);
        for pos in 0..3 {
            let wid = if use_casket {
                let mut picked = None;
                for offset in 0..casket_words.len() {
                    let idx = (slot_idx * 3 + pos + offset) % casket_words.len();
                    let candidate = &casket_words[idx];
                    if !used_word_ids.contains(candidate) || offset == casket_words.len() - 1 {
                        picked = Some(candidate.clone());
                        break;
                    }
                }
                picked.unwrap_or_else(|| casket_words[pos % casket_words.len()].clone())
            } else {
                let mut best_word = String::new();
                let mut best_score = -999999.0;
                let seed_u = seed as u64;

                for (elem_idx, elem) in registry.ordered_elements().enumerate() {
                    if used_word_ids.contains(&elem.id) {
                        continue;
                    }
                    let v = &elem.vector_12d;
                    let slot_score = match slot {
                        TacticalSlot::Opening => v.tempo_early * 2.2 + v.focus * 1.5 + beta * 1.8,
                        TacticalSlot::Midgame => v.tempo_mid * 2.0 + v.flow * 1.6 + v.penetration * 1.5,
                        TacticalSlot::Finisher => v.tempo_late * 2.4 + v.shock * 2.0 + alpha * 2.0,
                        TacticalSlot::Defense => v.sustain * 2.2 + v.vitality * 1.8 + gamma * 2.0,
                    };

                    let hash = (seed_u ^ (elem_idx as u64).wrapping_mul(1000000007))
                        .wrapping_add(((slot_idx * 3 + pos) as u64).wrapping_mul(2654435761))
                        .wrapping_mul(6364136223846793005);
                    let hash_f = (hash % 1000) as f64 / 1000.0 * 2.5;

                    let total = slot_score + hash_f;
                    if total > best_score {
                        best_score = total;
                        best_word = elem.id.clone();
                    }
                }

                if best_word.is_empty() {
                    let fallback_idx = (slot_idx * 3 + pos + (seed.abs() as usize % 100)) % registry.len().max(1);
                    registry.ordered_elements().nth(fallback_idx).map(|e| e.id.clone()).unwrap_or_else(|| "wu".to_string())
                } else {
                    best_word
                }
            };

            used_word_ids.insert(wid.clone());
            chosen_words.push(wid);
        }

        let inlays: Vec<StarInlay> = (0..3)
            .map(|i| {
                StarInlay::with_fluency(
                    nodes[i].clone(),
                    chosen_words[i].clone(),
                    OrbitFluency::from_cost(node_costs[i]),
                )
            })
            .collect();

        if let Ok(mut sentence) = StarSentence::new(inlays) {
            let ch_first = registry.get_by_id(&chosen_words[0]).map(|e| e.ch.as_str()).unwrap_or("勢");
            let ch_last = registry.get_by_id(&chosen_words[2]).map(|e| e.ch.as_str()).unwrap_or("決");

            let suffix_pool: &[&str] = match slot {
                TacticalSlot::Opening => &[
                    "探隙勢",
                    "奪人勢",
                    "截脈式",
                    "掠影勢",
                    "破空式",
                    "掣電勢",
                    "穿雲勢",
                    "破虛式",
                ],
                TacticalSlot::Midgame => &[
                    "截江連",
                    "奔湧連",
                    "流水擊",
                    "碎骨連",
                    "妙手連",
                    "拂柳連",
                    "驟雨連",
                    "崩濤連",
                ],
                TacticalSlot::Finisher => &[
                    "裂骨擊",
                    "一擲破",
                    "雷霆破",
                    "斷岳轟",
                    "穿隙破",
                    "透骨殺",
                    "開天轟",
                    "崩解殺",
                ],
                TacticalSlot::Defense => &[
                    "金湯身",
                    "卸勁步",
                    "換斗步",
                    "避實身",
                    "周天護",
                    "無轉身",
                    "潛水步",
                    "抱一身",
                ],
            };

            let s_idx = ((seed.abs() as usize) / 7 + slot_idx) % suffix_pool.len();
            let dyn_name = format!("{}{}{}", ch_first, ch_last, suffix_pool[s_idx]);
            sentence.set_custom_name(dyn_name);
            sentence.set_soul_seed(seed);
            loadout.set_slot(slot, sentence);
        }
    }

    loadout
}

/// 依據造句詞元星位與角色生命種子，推導自動繞行經絡阻力之體感描述（同法不同相）
pub fn compute_auto_detour_meridian_note(_words: &[&WordEntry], soul_seed: i64) -> String {
    crate::combat::human_phase::InnateBonePhase::from_seed(soul_seed).describe_bone_phase_zh()
}

/// 依據生命種子體察周天經絡在靜修體悟中的自發分流與最暢通經脈
pub fn somatic_meridian_awareness(soul_seed: i64) -> String {
    crate::combat::human_phase::InnateBonePhase::from_seed(soul_seed).describe_bone_phase_zh()
}

/// 戰術交鋒每階段記錄（承載碎招、套路與成招技能狀態）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhaseClashRecord {
    pub phase: usize,
    pub slot: TacticalSlot,
    pub a_technique_name: String,
    pub b_technique_name: String,
    pub a_initiative_score: f64,
    pub b_initiative_score: f64,
    pub damage_to_a: i32,
    pub damage_to_b: i32,
    pub narrative: String,
    #[serde(default)]
    pub a_manifested_as_skill: bool,
    #[serde(default)]
    pub b_manifested_as_skill: bool,
    #[serde(default)]
    pub counter_reason: Option<String>,
    #[serde(default)]
    pub a_motions: Vec<String>,
    #[serde(default)]
    pub b_motions: Vec<String>,
    #[serde(default)]
    pub a_completed_motions: usize,
    #[serde(default)]
    pub b_completed_motions: usize,
    #[serde(default)]
    pub a_target_part: TargetPart,
    #[serde(default)]
    pub b_target_part: TargetPart,
    #[serde(default)]
    pub a_carrier: CarrierAffinity,
    #[serde(default)]
    pub b_carrier: CarrierAffinity,
    #[serde(default)]
    pub a_posture_after: PostureState,
    #[serde(default)]
    pub b_posture_after: PostureState,
    #[serde(default)]
    pub active_residue_before: Option<BattlefieldResidue>,
    #[serde(default)]
    pub residue_left_after: Option<BattlefieldResidue>,
    #[serde(default)]
    pub a_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub b_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub a_resonance_tier: u8,
    #[serde(default)]
    pub b_resonance_tier: u8,
}

/// 單回合交鋒結算成果（落實「碎招連貫叫套路，成功叫技能，失敗被格擋就只是碎招」）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoutClashOutcome {
    pub winner_side: String, // "side_a", "side_b", or "draw"
    pub a_manifested_as_skill: bool,
    pub b_manifested_as_skill: bool,
    pub a_technique_name: String,
    pub b_technique_name: String,
    pub a_initiative_score: f64,
    pub b_initiative_score: f64,
    pub damage_to_a: i32,
    pub damage_to_b: i32,
    pub counter_reason: Option<String>,
    pub narrative: String,
    #[serde(default)]
    pub a_motions: Vec<String>,
    #[serde(default)]
    pub b_motions: Vec<String>,
    #[serde(default)]
    pub a_completed_motions: usize,
    #[serde(default)]
    pub b_completed_motions: usize,
    #[serde(default)]
    pub a_target_part: TargetPart,
    #[serde(default)]
    pub b_target_part: TargetPart,
    #[serde(default)]
    pub a_carrier: CarrierAffinity,
    #[serde(default)]
    pub b_carrier: CarrierAffinity,
    #[serde(default)]
    pub a_posture_after: PostureState,
    #[serde(default)]
    pub b_posture_after: PostureState,
    #[serde(default)]
    pub active_residue_before: Option<BattlefieldResidue>,
    #[serde(default)]
    pub residue_left_after: Option<BattlefieldResidue>,
    #[serde(default)]
    pub a_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub b_residue_interaction: Option<ResidueInteraction>,
    #[serde(default)]
    pub a_resonance_tier: u8,
    #[serde(default)]
    pub b_resonance_tier: u8,
}

/// 田忌賽馬交鋒總報告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalClashReport {
    pub winner: String,
    pub rounds: Vec<PhaseClashRecord>,
    pub side_a_final_hp: i32,
    pub side_b_final_hp: i32,
    pub summary: String,
}

/// 單序位交鋒判定（融合相同量級專克拆招、跨量級一力降十會、戰場殘留氣機與架勢判定）
pub fn resolve_bout(
    slot: TacticalSlot,
    phase: usize,
    tech_a: Option<&SynthesizedTechnique>,
    tech_b: Option<&SynthesizedTechnique>,
    momentum_a: f64,
    momentum_b: f64,
    weight_a: f64,
    weight_b: f64,
    def_mit_a: f64,
    def_mit_b: f64,
    a_flawed_prior: bool,
    b_flawed_prior: bool,
) -> BoutClashOutcome {
    resolve_bout_with_state(
        slot,
        phase,
        tech_a,
        tech_b,
        momentum_a,
        momentum_b,
        weight_a,
        weight_b,
        def_mit_a,
        def_mit_b,
        a_flawed_prior,
        b_flawed_prior,
        PostureState::Steady,
        PostureState::Steady,
        None,
    )
}

/// 帶入架勢狀態與殘留氣場之單序位完整交鋒判定
pub fn resolve_bout_with_state(
    slot: TacticalSlot,
    _phase: usize,
    tech_a: Option<&SynthesizedTechnique>,
    tech_b: Option<&SynthesizedTechnique>,
    momentum_a: f64,
    momentum_b: f64,
    weight_a: f64,
    weight_b: f64,
    def_mit_a: f64,
    def_mit_b: f64,
    a_flawed_prior: bool,
    b_flawed_prior: bool,
    a_prior_posture: PostureState,
    b_prior_posture: PostureState,
    active_residue: Option<BattlefieldResidue>,
) -> BoutClashOutcome {
    let name_a = tech_a.map(|t| t.effective_name.clone()).unwrap_or_else(|| "隨風拂袖".to_string());
    let name_b = tech_b.map(|t| t.effective_name.clone()).unwrap_or_else(|| "凝神自守".to_string());

    let (raw_score_a, pen_a, shock_a) = if let Some(t) = tech_a {
        let sc = match slot {
            TacticalSlot::Opening => t.vector_12d.tempo_early * 1.6 + t.vector_12d.focus * 1.0,
            TacticalSlot::Midgame => t.vector_12d.tempo_mid * 1.5 + t.vector_12d.flow * 1.2,
            TacticalSlot::Finisher => t.vector_12d.tempo_late * 1.8 + t.vector_12d.shock * 1.5,
            TacticalSlot::Defense => t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2,
        };
        (sc, t.vector_12d.penetration, t.vector_12d.shock)
    } else {
        (0.25, 0.1, 0.1)
    };

    let (raw_score_b, pen_b, shock_b) = if let Some(t) = tech_b {
        let sc = match slot {
            TacticalSlot::Opening => t.vector_12d.tempo_early * 1.6 + t.vector_12d.focus * 1.0,
            TacticalSlot::Midgame => t.vector_12d.tempo_mid * 1.5 + t.vector_12d.flow * 1.2,
            TacticalSlot::Finisher => t.vector_12d.tempo_late * 1.8 + t.vector_12d.shock * 1.5,
            TacticalSlot::Defense => t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2,
        };
        (sc, t.vector_12d.penetration, t.vector_12d.shock)
    } else {
        (0.25, 0.1, 0.1)
    };

    let res_tier_a = tech_a.map(|t| t.resonance_tier()).unwrap_or(1);
    let res_tier_b = tech_b.map(|t| t.resonance_tier()).unwrap_or(1);
    let res_mult_a = tech_a.map(|t| t.resonance_multiplier()).unwrap_or(1.0);
    let res_mult_b = tech_b.map(|t| t.resonance_multiplier()).unwrap_or(1.0);

    let mut score_a = raw_score_a * momentum_a * res_mult_a;
    let mut score_b = raw_score_b * momentum_b * res_mult_b;

    // 殘留氣機交互作用判定
    let a_residue_interaction = active_residue.and_then(|res| tech_a.and_then(|t| t.check_residue_interaction(res)));
    let b_residue_interaction = active_residue.and_then(|res| tech_b.and_then(|t| t.check_residue_interaction(res)));

    if let Some(ResidueInteraction::Resonance) = a_residue_interaction {
        score_a *= 1.15;
    } else if let Some(ResidueInteraction::Backlash) = a_residue_interaction {
        score_a *= 0.88;
    }

    if let Some(ResidueInteraction::Resonance) = b_residue_interaction {
        score_b *= 1.15;
    } else if let Some(ResidueInteraction::Backlash) = b_residue_interaction {
        score_b *= 0.88;
    }

    // 1. 跨量級碾壓（為什麼近身競技要分級，因為「一力降十會」！）
    let ratio_a = weight_a / weight_b.max(0.1);
    let ratio_b = weight_b / weight_a.max(0.1);
    let mut counter_reason = None;

    let a_evades_heavy_b = if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;
        (v_a.penetration + v_a.flow >= 2.6)
            && (v_a.tempo_early >= v_b.tempo_early + 0.3)
            && (v_b.sustain < 0.5)
    } else {
        false
    };
    let b_evades_heavy_a = if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;
        (v_b.penetration + v_b.flow >= 2.6)
            && (v_b.tempo_early >= v_a.tempo_early + 0.3)
            && (v_a.sustain < 0.5)
    } else {
        false
    };

    if ratio_a >= 1.45 && !b_evades_heavy_a {
        score_a += (ratio_a - 1.0) * 1.8;
        counter_reason = Some("重勢硬壓：挾千鈞重勢正面硬壓，沛然巨力直接震散對方巧勁架勢".to_string());
    } else if ratio_b >= 1.45 && !a_evades_heavy_b {
        score_b += (ratio_b - 1.0) * 1.8;
        counter_reason = Some("重勢硬壓：挾千鈞重勢正面硬壓，沛然巨力直接震散對方巧勁架勢".to_string());
    } else if ratio_b >= 1.45 && a_evades_heavy_b {
        score_a += 0.75;
        counter_reason = Some("入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string());
    } else if ratio_a >= 1.45 && b_evades_heavy_a {
        score_b += 0.75;
        counter_reason = Some("入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string());
    } else if let (Some(t_a), Some(t_b)) = (tech_a, tech_b) {
        // 2. 同量級拆招（「玉女劍法專克全真劍法：以巧破剛」）
        let v_a = &t_a.vector_12d;
        let v_b = &t_b.vector_12d;

        // A 克制 B 判定
        if (v_b.shock + v_b.sustain >= 0.9) && (v_a.penetration + v_a.flow >= 0.9) && (v_a.tempo_early >= v_b.tempo_early - 0.1) {
            score_a += 0.60;
            counter_reason = Some("以巧破剛：身形微側，鋒芒恰恰切入對方換氣收勢的空隙".to_string());
        } else if (v_b.flow >= 0.9 && v_b.shock < 0.6) && (v_a.shock + v_a.reach >= 1.1) {
            score_a += 0.50;
            counter_reason = Some("以重破巧：大開大闔，以沉渾重勁強行封死對方騰挪退路".to_string());
        } else if (v_b.tempo_late >= 0.8 && v_b.qi_cost >= 0.7) && (v_a.tempo_early + v_a.focus >= 1.0) {
            score_a += 0.50;
            counter_reason = Some("以守待竭：在對方後勁尚未蓄滿之際搶先欺身，半途截斷其氣機".to_string());
        } else if (v_a.shock + v_a.sustain >= 0.9) && (v_b.penetration + v_b.flow >= 0.9) && (v_b.tempo_early >= v_a.tempo_early - 0.1) {
            // B 克制 A 判定
            score_b += 0.60;
            counter_reason = Some("以巧破剛：身形微側，鋒芒恰恰切入對方換氣收勢的空隙".to_string());
        } else if (v_a.flow >= 0.9 && v_a.shock < 0.6) && (v_b.shock + v_b.reach >= 1.1) {
            score_b += 0.50;
            counter_reason = Some("以重破巧：大開大闔，以沉渾重勁強行封死對方騰挪退路".to_string());
        } else if (v_a.tempo_late >= 0.8 && v_a.qi_cost >= 0.7) && (v_b.tempo_early + v_b.focus >= 1.0) {
            score_b += 0.50;
            counter_reason = Some("以守待竭：在對方後勁尚未蓄滿之際搶先欺身，半途截斷其氣機".to_string());
        }
    }

    let a_carrier = tech_a
        .map(|t| t.primary_carrier)
        .unwrap_or(CarrierAffinity::Unarmed);
    let b_carrier = tech_b
        .map(|t| t.primary_carrier)
        .unwrap_or(CarrierAffinity::Unarmed);
    let a_target_part = tech_a
        .map(|t| t.primary_target)
        .unwrap_or(TargetPart::Torso);
    let b_target_part = tech_b
        .map(|t| t.primary_target)
        .unwrap_or(TargetPart::Torso);
    let a_motions = tech_a
        .map(|t| t.derive_elementary_motions_for_carrier(None))
        .unwrap_or_else(|| {
            derive_elementary_motions_from_chars(&[], a_carrier, a_target_part)
        });
    let b_motions = tech_b
        .map(|t| t.derive_elementary_motions_for_carrier(None))
        .unwrap_or_else(|| {
            derive_elementary_motions_from_chars(&[], b_carrier, b_target_part)
        });
    let seed_a = tech_a
        .and_then(|t| t.soul_seed)
        .unwrap_or_else(|| {
            let key = tech_a
                .map(|t| format!("{}:{slot:?}", t.effective_name))
                .unwrap_or_else(|| format!("neutral_empty_slot:{slot:?}"));
            crate::combat::human_phase::resolve_entity_soul_seed(&key, None)
        });
    let seed_b = tech_b
        .and_then(|t| t.soul_seed)
        .unwrap_or_else(|| {
            let key = tech_b
                .map(|t| format!("{}:{slot:?}", t.effective_name))
                .unwrap_or_else(|| format!("neutral_empty_slot:{slot:?}"));
            crate::combat::human_phase::resolve_entity_soul_seed(&key, None)
        });
    let bone_a = crate::combat::human_phase::InnateBonePhase::from_seed(seed_a);
    let bone_b = crate::combat::human_phase::InnateBonePhase::from_seed(seed_b);

    let practice_a = tech_a.map(|t| t.practice_count).unwrap_or(25);
    let practice_b = tech_b.map(|t| t.practice_count).unwrap_or(25);
    let anatomy_a = tech_a.map(|t| t.anatomy_familiarity).unwrap_or(20);
    let anatomy_b = tech_b.map(|t| t.anatomy_familiarity).unwrap_or(20);
    let learned_a = tech_a.map(|t| t.learned_delta).unwrap_or(0.0);
    let learned_b = tech_b.map(|t| t.learned_delta).unwrap_or(0.0);

    let three_phase_res = crate::combat::human_phase::evaluate_three_phase_bout_ext(
        slot,
        &bone_a,
        &bone_b,
        weight_a,
        weight_b,
        a_carrier,
        b_carrier,
        a_prior_posture,
        b_prior_posture,
        a_motions.len(),
        b_motions.len(),
        _phase,
        practice_a,
        practice_b,
        anatomy_a,
        anatomy_b,
        learned_a,
        learned_b,
        if a_flawed_prior { 0.25 } else { 0.0 },
        if b_flawed_prior { 0.25 } else { 0.0 },
    );

    if counter_reason.is_none() {
        score_a += three_phase_res.score_delta_a;
        score_b += three_phase_res.score_delta_b;
        if let Some(cr) = &three_phase_res.counter_reason {
            counter_reason = Some(cr.clone());
        }
    }
    if let Some(cr) = counter_reason.as_deref() {
        if cr.starts_with("重勢硬壓") || cr.starts_with("重勢硬撼") {
            if (score_a > score_b && weight_a < weight_b)
                || (score_b > score_a && weight_b < weight_a)
            {
                counter_reason = Some(
                    "入微破勢：避開千鈞正面鋒芒，趁對方舊力初盡、新力未生之骨縫切入".to_string(),
                );
            }
        }
    }

    let mut notes = Vec::new();
    if let Some(r) = counter_reason.as_deref() {
        notes.push(r.to_string());
    }
    if let Some(res_int) = a_residue_interaction {
        notes.push(format!("甲方{}", res_int.label()));
    }
    if let Some(res_int) = b_residue_interaction {
        notes.push(format!("乙方{}", res_int.label()));
    }
    let counter_note = if notes.is_empty() {
        String::new()
    } else {
        format!("（{}）", notes.join("；"))
    };

    let is_draw = (score_a - score_b).abs() < 1e-6;

    if is_draw {
        let a_completed_motions = if !three_phase_res.a_manifested_as_skill && three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange {
            three_phase_res.a_completed_motions.clamp(1, a_motions.len().max(1))
        } else {
            a_motions.len().max(1)
        };
        let b_completed_motions = if !three_phase_res.b_manifested_as_skill && three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange {
            three_phase_res.b_completed_motions.clamp(1, b_motions.len().max(1))
        } else {
            b_motions.len().max(1)
        };
        let a_chain_str = a_motions.join("、");
        let b_chain_str = b_motions.join("、");
        let narr = format!(
            "雙方同時出招，甲方之「{name_a}」（{a_chain_str}）與乙方之「{name_b}」（{b_chain_str}）於半空激烈碰撞！氣勁互相抵消激盪，雙方各自震退，勢均力敵，難分高下！{counter_note}"
        );
        let a_post = if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_prior_posture.degrade()
        } else {
            a_prior_posture
        };
        let b_post = if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_prior_posture.degrade()
        } else {
            b_prior_posture
        };
        let residue_left_after = tech_a
            .and_then(|t| t.emitted_residue())
            .or_else(|| tech_b.and_then(|t| t.emitted_residue()));

        BoutClashOutcome {
            winner_side: "draw".to_string(),
            a_manifested_as_skill: false,
            b_manifested_as_skill: false,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: 0,
            damage_to_b: 0,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    } else if score_a > score_b {
        let base_dmg = match slot {
            TacticalSlot::Opening => (score_a - score_b) * 6.0 + pen_a * 8.0 + 3.0,
            TacticalSlot::Midgame => (score_a - score_b) * 7.0 + pen_a * 10.0 + 4.0,
            TacticalSlot::Finisher => {
                let flaw_mult = if b_flawed_prior { 1.5 } else { 1.0 };
                ((score_a - score_b) * 9.0 + shock_a * 12.0 + 6.0) * flaw_mult
            }
            TacticalSlot::Defense => (score_a - score_b) * 5.0 + 2.0,
        };
        let final_dmg = (base_dmg - def_mit_b).max(2.0).round() as i32;
        let a_completed_motions = a_motions.len().max(1);
        let b_completed_motions = if three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange && !three_phase_res.b_manifested_as_skill {
            three_phase_res.b_completed_motions.clamp(1, b_motions.len().max(1))
        } else if score_a - score_b >= 0.8 {
            1
        } else {
            2.min(b_motions.len().saturating_sub(1).max(1))
        };
        let a_chain_str = a_motions.join("、");
        let b_frag_str = b_motions
            .iter()
            .take(b_completed_motions)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let target_label = a_target_part.label();

        let narr = match slot {
            TacticalSlot::Opening => format!(
                "甲方將「{a_chain_str}」順暢連貫，起承轉合一氣呵成，顯化成招為技能「{name_a}」！乙方甫自起手做出「{b_frag_str}」，便在半途被截斷氣機，套路當場潰散為零散碎招，{target_label}受創！{counter_note}"
            ),
            TacticalSlot::Midgame => format!(
                "甲方乘勝將「{a_chain_str}」融會貫通，成招顯化為技能「{name_a}」！乙方勉強遞出「{b_frag_str}」苦撐，氣息浮動間被硬生截斷，只餘零星碎招便被破開防線，{target_label}受震！{counter_note}"
            ),
            TacticalSlot::Finisher => format!(
                "甲方周身勁力合一，「{a_chain_str}」連環貫徹，悍然顯化為絕殺技能「{name_a}」！乙方意圖運轉「{b_frag_str}」抵禦，終因招式受阻散為零碎招架，{target_label}遭重創失據！{counter_note}"
            ),
            TacticalSlot::Defense => format!(
                "甲方守勢「{a_chain_str}」圓融貫通，顯化為技能「{name_a}」！乙方僅遞出「{b_frag_str}」便散為碎招，未能突破防線，反受震退！{counter_note}"
            ),
        };

        let mut a_post = a_prior_posture;
        if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_post = a_post.degrade();
        }

        let mut b_post = if final_dmg >= 22 || b_prior_posture == PostureState::Shaken || b_prior_posture == PostureState::Broken {
            PostureState::Broken
        } else if final_dmg >= 11 || three_phase_res.b_posture_after == PostureState::Shaken {
            PostureState::Shaken
        } else {
            PostureState::Steady
        };
        if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_post = b_post.degrade();
        }

        let residue_left_after = tech_a.and_then(|t| t.emitted_residue());

        BoutClashOutcome {
            winner_side: "side_a".to_string(),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: 0,
            damage_to_b: final_dmg,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    } else {
        let base_dmg = match slot {
            TacticalSlot::Opening => (score_b - score_a) * 6.0 + pen_b * 8.0 + 3.0,
            TacticalSlot::Midgame => (score_b - score_a) * 7.0 + pen_b * 10.0 + 4.0,
            TacticalSlot::Finisher => {
                let flaw_mult = if a_flawed_prior { 1.5 } else { 1.0 };
                ((score_b - score_a) * 9.0 + shock_b * 12.0 + 6.0) * flaw_mult
            }
            TacticalSlot::Defense => (score_b - score_a) * 5.0 + 2.0,
        };
        let final_dmg = (base_dmg - def_mit_a).max(2.0).round() as i32;
        let b_completed_motions = b_motions.len().max(1);
        let a_completed_motions = if three_phase_res.verdict != crate::combat::human_phase::ThreePhaseVerdict::StandardExchange && !three_phase_res.a_manifested_as_skill {
            three_phase_res.a_completed_motions.clamp(1, a_motions.len().max(1))
        } else if score_b - score_a >= 0.8 {
            1
        } else {
            2.min(a_motions.len().saturating_sub(1).max(1))
        };
        let b_chain_str = b_motions.join("、");
        let a_frag_str = a_motions
            .iter()
            .take(a_completed_motions)
            .cloned()
            .collect::<Vec<_>>()
            .join("、");
        let target_label = b_target_part.label();

        let narr = match slot {
            TacticalSlot::Opening => format!(
                "乙方神速搶制，「{b_chain_str}」貫徹到底，顯化成招為技能「{name_b}」！甲方甫自做出「{a_frag_str}」，便在半途遭強勢截斷，套路當場潰散為零散碎招，{target_label}受創！{counter_note}"
            ),
            TacticalSlot::Midgame => format!(
                "乙方「{b_chain_str}」起承轉合無懈可擊，成招顯化為技能「{name_b}」！甲方遞出「{a_frag_str}」後攻角遭全面壓制散為碎招，退避不及致{target_label}受震！{counter_note}"
            ),
            TacticalSlot::Finisher => format!(
                "乙方後勁爆發，「{b_chain_str}」一氣呵成顯化為技能「{name_b}」！甲方僅做出「{a_frag_str}」便被打散為碎招，防線瓦解，{target_label}遭重創！{counter_note}"
            ),
            TacticalSlot::Defense => format!(
                "乙方守勢「{b_chain_str}」化境成招，顯化為技能「{name_b}」！甲方「{a_frag_str}」散為零碎招架，無功而返，反受震退！{counter_note}"
            ),
        };

        let mut b_post = b_prior_posture;
        if b_residue_interaction == Some(ResidueInteraction::Backlash) {
            b_post = b_post.degrade();
        }

        let mut a_post = if final_dmg >= 22 || a_prior_posture == PostureState::Shaken || a_prior_posture == PostureState::Broken {
            PostureState::Broken
        } else if final_dmg >= 11 || three_phase_res.a_posture_after == PostureState::Shaken {
            PostureState::Shaken
        } else {
            PostureState::Steady
        };
        if a_residue_interaction == Some(ResidueInteraction::Backlash) {
            a_post = a_post.degrade();
        }

        let residue_left_after = tech_b.and_then(|t| t.emitted_residue());

        BoutClashOutcome {
            winner_side: "side_b".to_string(),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_technique_name: name_a,
            b_technique_name: name_b,
            a_initiative_score: score_a,
            b_initiative_score: score_b,
            damage_to_a: final_dmg,
            damage_to_b: 0,
            counter_reason,
            narrative: narr,
            a_motions,
            b_motions,
            a_completed_motions,
            b_completed_motions,
            a_target_part,
            b_target_part,
            a_carrier,
            b_carrier,
            a_posture_after: a_post,
            b_posture_after: b_post,
            active_residue_before: active_residue,
            residue_left_after,
            a_residue_interaction,
            b_residue_interaction,
            a_resonance_tier: res_tier_a,
            b_resonance_tier: res_tier_b,
        }
    }
}

/// 帶入雙方攻防權重之田忌賽馬時序對決判定
pub fn resolve_tianji_matchup_with_weights(
    side_a: &TacticalLoadout,
    side_a_hp: i32,
    side_b: &TacticalLoadout,
    side_b_hp: i32,
    weight_a: f64,
    weight_b: f64,
    registry: &WordRegistry,
) -> Result<TacticalClashReport, WordPlateError> {
    let mut current_hp_a = side_a_hp.max(1);
    let mut current_hp_b = side_b_hp.max(1);
    let mut rounds = Vec::with_capacity(3);
    let mut posture_a = PostureState::Steady;
    let mut posture_b = PostureState::Steady;
    let mut active_residue = None;

    // 守勢身法預備
    let def_a_tech = side_a.defense.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let def_b_tech = side_b.defense.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let def_a_mit = def_a_tech.as_ref().map(|t| t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2).unwrap_or(0.0);
    let def_b_mit = def_b_tech.as_ref().map(|t| t.vector_12d.sustain * 1.5 + t.vector_12d.vitality * 1.2).unwrap_or(0.0);

    // 一、起手式（Opening）交鋒
    let op_a = side_a.opening.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let op_b = side_b.opening.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let bout_1 = resolve_bout_with_state(
        TacticalSlot::Opening,
        1,
        op_a.as_ref(),
        op_b.as_ref(),
        1.0,
        1.0,
        weight_a,
        weight_b,
        def_a_mit,
        def_b_mit,
        false,
        false,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_1.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_1.damage_to_b).max(0);
    posture_a = bout_1.a_posture_after;
    posture_b = bout_1.b_posture_after;
    active_residue = bout_1.residue_left_after;

    let (momentum_a, momentum_b) = if bout_1.winner_side == "side_a" {
        (1.2, 0.85)
    } else if bout_1.winner_side == "side_b" {
        (0.85, 1.2)
    } else {
        (1.0, 1.0)
    };

    rounds.push(PhaseClashRecord {
        phase: 1,
        slot: TacticalSlot::Opening,
        a_technique_name: bout_1.a_technique_name,
        b_technique_name: bout_1.b_technique_name,
        a_initiative_score: bout_1.a_initiative_score,
        b_initiative_score: bout_1.b_initiative_score,
        damage_to_a: bout_1.damage_to_a,
        damage_to_b: bout_1.damage_to_b,
        narrative: bout_1.narrative,
        a_manifested_as_skill: bout_1.a_manifested_as_skill,
        b_manifested_as_skill: bout_1.b_manifested_as_skill,
        counter_reason: bout_1.counter_reason,
        a_motions: bout_1.a_motions,
        b_motions: bout_1.b_motions,
        a_completed_motions: bout_1.a_completed_motions,
        b_completed_motions: bout_1.b_completed_motions,
        a_target_part: bout_1.a_target_part,
        b_target_part: bout_1.b_target_part,
        a_carrier: bout_1.a_carrier,
        b_carrier: bout_1.b_carrier,
        a_posture_after: bout_1.a_posture_after,
        b_posture_after: bout_1.b_posture_after,
        active_residue_before: bout_1.active_residue_before,
        residue_left_after: bout_1.residue_left_after,
        a_residue_interaction: bout_1.a_residue_interaction,
        b_residue_interaction: bout_1.b_residue_interaction,
        a_resonance_tier: bout_1.a_resonance_tier,
        b_resonance_tier: bout_1.b_resonance_tier,
    });

    // 二、常態連攜（Midgame）交鋒
    let mid_a = side_a.midgame.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let mid_b = side_b.midgame.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let a_flawed = (bout_1.winner_side == "side_b" && (bout_1.b_initiative_score - bout_1.a_initiative_score > 0.4)) || bout_1.a_posture_after == PostureState::Broken;
    let b_flawed = (bout_1.winner_side == "side_a" && (bout_1.a_initiative_score - bout_1.b_initiative_score > 0.4)) || bout_1.b_posture_after == PostureState::Broken;

    let bout_2 = resolve_bout_with_state(
        TacticalSlot::Midgame,
        2,
        mid_a.as_ref(),
        mid_b.as_ref(),
        momentum_a,
        momentum_b,
        weight_a,
        weight_b,
        def_a_mit * 0.7,
        def_b_mit * 0.7,
        a_flawed,
        b_flawed,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_2.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_2.damage_to_b).max(0);
    posture_a = bout_2.a_posture_after;
    posture_b = bout_2.b_posture_after;
    active_residue = bout_2.residue_left_after;

    let (momentum_a_fin, momentum_b_fin) = if bout_2.winner_side == "side_a" {
        (1.2, 0.85)
    } else if bout_2.winner_side == "side_b" {
        (0.85, 1.2)
    } else {
        (1.0, 1.0)
    };

    rounds.push(PhaseClashRecord {
        phase: 2,
        slot: TacticalSlot::Midgame,
        a_technique_name: bout_2.a_technique_name,
        b_technique_name: bout_2.b_technique_name,
        a_initiative_score: bout_2.a_initiative_score,
        b_initiative_score: bout_2.b_initiative_score,
        damage_to_a: bout_2.damage_to_a,
        damage_to_b: bout_2.damage_to_b,
        narrative: bout_2.narrative,
        a_manifested_as_skill: bout_2.a_manifested_as_skill,
        b_manifested_as_skill: bout_2.b_manifested_as_skill,
        counter_reason: bout_2.counter_reason,
        a_motions: bout_2.a_motions,
        b_motions: bout_2.b_motions,
        a_completed_motions: bout_2.a_completed_motions,
        b_completed_motions: bout_2.b_completed_motions,
        a_target_part: bout_2.a_target_part,
        b_target_part: bout_2.b_target_part,
        a_carrier: bout_2.a_carrier,
        b_carrier: bout_2.b_carrier,
        a_posture_after: bout_2.a_posture_after,
        b_posture_after: bout_2.b_posture_after,
        active_residue_before: bout_2.active_residue_before,
        residue_left_after: bout_2.residue_left_after,
        a_residue_interaction: bout_2.a_residue_interaction,
        b_residue_interaction: bout_2.b_residue_interaction,
        a_resonance_tier: bout_2.a_resonance_tier,
        b_resonance_tier: bout_2.b_resonance_tier,
    });

    // 三、破綻殺招（Finisher）交鋒
    let fin_a = side_a.finisher.as_ref().map(|s| s.synthesize(registry)).transpose()?;
    let fin_b = side_b.finisher.as_ref().map(|s| s.synthesize(registry)).transpose()?;

    let a_flawed_fin = (bout_2.winner_side == "side_b" && (bout_2.b_initiative_score - bout_2.a_initiative_score > 0.5)) || bout_2.a_posture_after == PostureState::Broken;
    let b_flawed_fin = (bout_2.winner_side == "side_a" && (bout_2.a_initiative_score - bout_2.b_initiative_score > 0.5)) || bout_2.b_posture_after == PostureState::Broken;

    let bout_3 = resolve_bout_with_state(
        TacticalSlot::Finisher,
        3,
        fin_a.as_ref(),
        fin_b.as_ref(),
        momentum_a_fin,
        momentum_b_fin,
        weight_a,
        weight_b,
        def_a_mit * 0.5,
        def_b_mit * 0.5,
        a_flawed_fin,
        b_flawed_fin,
        posture_a,
        posture_b,
        active_residue,
    );
    current_hp_a = (current_hp_a - bout_3.damage_to_a).max(0);
    current_hp_b = (current_hp_b - bout_3.damage_to_b).max(0);

    rounds.push(PhaseClashRecord {
        phase: 3,
        slot: TacticalSlot::Finisher,
        a_technique_name: bout_3.a_technique_name,
        b_technique_name: bout_3.b_technique_name,
        a_initiative_score: bout_3.a_initiative_score,
        b_initiative_score: bout_3.b_initiative_score,
        damage_to_a: bout_3.damage_to_a,
        damage_to_b: bout_3.damage_to_b,
        narrative: bout_3.narrative,
        a_manifested_as_skill: bout_3.a_manifested_as_skill,
        b_manifested_as_skill: bout_3.b_manifested_as_skill,
        counter_reason: bout_3.counter_reason,
        a_motions: bout_3.a_motions,
        b_motions: bout_3.b_motions,
        a_completed_motions: bout_3.a_completed_motions,
        b_completed_motions: bout_3.b_completed_motions,
        a_target_part: bout_3.a_target_part,
        b_target_part: bout_3.b_target_part,
        a_carrier: bout_3.a_carrier,
        b_carrier: bout_3.b_carrier,
        a_posture_after: bout_3.a_posture_after,
        b_posture_after: bout_3.b_posture_after,
        active_residue_before: bout_3.active_residue_before,
        residue_left_after: bout_3.residue_left_after,
        a_residue_interaction: bout_3.a_residue_interaction,
        b_residue_interaction: bout_3.b_residue_interaction,
        a_resonance_tier: bout_3.a_resonance_tier,
        b_resonance_tier: bout_3.b_resonance_tier,
    });

    let winner = if current_hp_a > current_hp_b {
        "side_a".to_string()
    } else if current_hp_b > current_hp_a {
        "side_b".to_string()
    } else {
        "draw".to_string()
    };

    let summary = match winner.as_str() {
        "side_a" => "數合交手告終，甲方憑藉精妙路數掌握節奏，氣機悠長沉穩，贏得此役！".to_string(),
        "side_b" => "數合交手告終，乙方後發制人底蘊深厚，守中帶攻，奪下勝局！".to_string(),
        _ => "數合交手勢均力敵，雙方身形氣機並駕齊驅，各自收勢！".to_string(),
    };

    Ok(TacticalClashReport {
        winner,
        rounds,
        side_a_final_hp: current_hp_a,
        side_b_final_hp: current_hp_b,
        summary,
    })
}

/// 執行田忌賽馬時序對決判定（保證零自殺判定、傷害單向承載與受控反衝）
pub fn resolve_tianji_matchup(
    side_a: &TacticalLoadout,
    side_a_hp: i32,
    side_b: &TacticalLoadout,
    side_b_hp: i32,
    registry: &WordRegistry,
) -> Result<TacticalClashReport, WordPlateError> {
    resolve_tianji_matchup_with_weights(
        side_a,
        side_a_hp,
        side_b,
        side_b_hp,
        1.0,
        1.0,
        registry,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_500_words_load_with_nonzero_12d_vectors() {
        let registry = WordRegistry::global();
        assert_eq!(registry.len(), 500, "詞元總數必須為 500");

        for word in registry.iter() {
            assert!(
                word.vector_12d.is_non_zero(),
                "詞元 [{}] {} 的 12D 向量全為零！",
                word.id,
                word.ch
            );
            // 驗證四維能量形態非負
            assert!(word.vector_12d.focus >= 0.0);
            assert!(word.vector_12d.shock >= 0.0);
            assert!(word.vector_12d.flow >= 0.0);
            assert!(word.vector_12d.sustain >= 0.0);

            // 驗證三維時序非負
            assert!(word.vector_12d.tempo_early >= 0.0);
            assert!(word.vector_12d.tempo_mid >= 0.0);
            assert!(word.vector_12d.tempo_late >= 0.0);

            // 驗證五維負載非負
            assert!(word.vector_12d.penetration >= 0.0);
            assert!(word.vector_12d.reach >= 0.0);
            assert!(word.vector_12d.vitality >= 0.0);
            assert!(word.vector_12d.qi_cost >= 0.0);
            assert!(word.vector_12d.recoil >= 0.0);
        }
    }

    #[test]
    fn test_all_360_star_nodes_inlayable_and_sentences() {
        let registry = WordRegistry::global();

        // 1. 驗證 N000 不可鑲字
        assert_eq!(
            parse_star_node("N000"),
            Err(WordPlateError::NodeNotInlayable("N000（生之樞紐為本源核心，不可鑲嵌字訣）".into()))
        );

        // 2. 驗證 N001..N360 全數合法且可解析
        for i in 1..=360 {
            let node_id = format!("N{i:03}");
            assert_eq!(parse_star_node(&node_id), Ok(i));
        }

        // 3. 驗證非法編號拒絕
        assert!(parse_star_node("N361").is_err());
        assert!(parse_star_node("N999").is_err());
        assert!(parse_star_node("X001").is_err());
        assert!(parse_star_node("").is_err());

        // 4. 驗證全 360 個星位皆可造句鑲字（以 3 言、5 言、7 言覆蓋）
        // 選取前幾顆詞元
        let words: Vec<&WordElement> = registry.ordered_elements().take(7).collect();
        assert_eq!(words.len(), 7);

        // 測試 3 言句式（N001..N003）
        let inlays_3 = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N002", &words[1].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N003", &words[2].id, OrbitFluency::PingWen),
        ];
        let sent_3 = StarSentence::new(inlays_3).expect("三言句式建立成功");
        let tech_3 = sent_3.synthesize(registry).expect("三言句式合成成功");
        assert_eq!(tech_3.length, 3);
        assert!(tech_3.vector_12d.is_non_zero());
        assert!(!tech_3.canonical_name.is_empty());

        // 測試 5 言句式（N004..N008）
        let inlays_5 = (4..=8)
            .enumerate()
            .map(|(idx, n)| {
                StarInlay::with_fluency(format!("N{n:03}"), &words[idx % words.len()].id, OrbitFluency::ShunTong)
            })
            .collect();
        let sent_5 = StarSentence::new(inlays_5).expect("五言句式建立成功");
        let tech_5 = sent_5.synthesize(registry).expect("五言句式合成成功");
        assert_eq!(tech_5.length, 5);
        assert!(tech_5.vector_12d.is_non_zero());

        // 測試 7 言句式（N009..N015）
        let inlays_7 = (9..=15)
            .enumerate()
            .map(|(idx, n)| {
                StarInlay::with_fluency(format!("N{n:03}"), &words[idx % words.len()].id, OrbitFluency::PingWen)
            })
            .collect();
        let sent_7 = StarSentence::new(inlays_7).expect("七言句式建立成功");
        let tech_7 = sent_7.synthesize(registry).expect("七言句式合成成功");
        assert_eq!(tech_7.length, 7);
        assert!(tech_7.vector_12d.is_non_zero());

        // 驗證遍歷所有 360 個節點皆可正確造句成相
        for chunk_start in (1..=360).step_by(3) {
            let chunk_end = (chunk_start + 2).min(360);
            if chunk_end - chunk_start + 1 == 3 {
                let chunk_inlays: Vec<StarInlay> = (chunk_start..=chunk_end)
                    .map(|n| StarInlay::new(format!("N{n:03}"), &words[0].id))
                    .collect();
                let sent = StarSentence::new(chunk_inlays).expect("全節點覆蓋三言造句合法");
                let synth = sent.synthesize(registry).expect("全節點合成合法");
                assert_eq!(synth.length, 3);
            }
        }

        // 驗證字數限制拒絕（4 言與 6 言非法）
        let invalid_inlays_4 = (1..=4).map(|n| StarInlay::new(format!("N{n:03}"), &words[0].id)).collect();
        assert_eq!(
            StarSentence::new(invalid_inlays_4).err(),
            Some(WordPlateError::InvalidSentenceLength(4))
        );

        // 驗證節點重複鑲嵌拒絕
        let dup_inlays = vec![
            StarInlay::new("N010", &words[0].id),
            StarInlay::new("N010", &words[1].id),
            StarInlay::new("N011", &words[2].id),
        ];
        assert_eq!(
            StarSentence::new(dup_inlays).err(),
            Some(WordPlateError::DuplicateNodeInlay("N010".into()))
        );
    }

    #[test]
    fn test_tactical_slots_and_tianji_matchup_cleanly() {
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(15).collect();

        // 甲方：起手快劍風格
        let mut loadout_a = TacticalLoadout::new();
        let sent_a_open = StarSentence::new(vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N002", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N003", &words[2].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("流星絕影劍");
        let sent_a_mid = StarSentence::new(vec![
            StarInlay::with_fluency("N004", &words[3].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N005", &words[4].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N006", &words[5].id, OrbitFluency::ShunTong),
        ]).unwrap().with_custom_name("驚雷連環擊");
        let sent_a_fin = StarSentence::new(vec![
            StarInlay::with_fluency("N007", &words[6].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N008", &words[7].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N009", &words[8].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("萬象天樞破");
        let sent_a_def = StarSentence::new(vec![
            StarInlay::with_fluency("N010", &words[9].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N011", &words[10].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N012", &words[11].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("凌波游雲步");

        loadout_a.set_slot(TacticalSlot::Opening, sent_a_open);
        loadout_a.set_slot(TacticalSlot::Midgame, sent_a_mid);
        loadout_a.set_slot(TacticalSlot::Finisher, sent_a_fin);
        loadout_a.set_slot(TacticalSlot::Defense, sent_a_def);

        // 乙方：穩健持重重甲風格
        let mut loadout_b = TacticalLoadout::new();
        let sent_b_open = StarSentence::new(vec![
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[2].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N023", &words[3].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("泰山沉岳勢");
        let sent_b_mid = StarSentence::new(vec![
            StarInlay::with_fluency("N024", &words[4].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N025", &words[5].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N026", &words[6].id, OrbitFluency::PingWen),
        ]).unwrap().with_custom_name("金鐘伏魔罩");
        let sent_b_fin = StarSentence::new(vec![
            StarInlay::with_fluency("N027", &words[7].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N028", &words[8].id, OrbitFluency::ShunTong),
            StarInlay::with_fluency("N029", &words[9].id, OrbitFluency::ShunTong),
        ]).unwrap().with_custom_name("千鈞開山斧");
        let sent_b_def = StarSentence::new(vec![
            StarInlay::with_fluency("N030", &words[10].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N031", &words[11].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N032", &words[12].id, OrbitFluency::ChangLiu),
        ]).unwrap().with_custom_name("鐵壁無縫身");

        loadout_b.set_slot(TacticalSlot::Opening, sent_b_open);
        loadout_b.set_slot(TacticalSlot::Midgame, sent_b_mid);
        loadout_b.set_slot(TacticalSlot::Finisher, sent_b_fin);
        loadout_b.set_slot(TacticalSlot::Defense, sent_b_def);

        let report = resolve_tianji_matchup(&loadout_a, 100, &loadout_b, 100, registry).expect("田忌賽馬對決順暢無誤");

        // 驗證三階段完整
        assert_eq!(report.rounds.len(), 3);
        assert_eq!(report.rounds[0].slot, TacticalSlot::Opening);
        assert_eq!(report.rounds[1].slot, TacticalSlot::Midgame);
        assert_eq!(report.rounds[2].slot, TacticalSlot::Finisher);

        // 驗證零自殺判定：受傷者絕非自殘，氣血扣除受控
        assert!(report.side_a_final_hp >= 0);
        assert!(report.side_b_final_hp >= 0);

        // 驗證定名正確帶入
        assert_eq!(report.rounds[0].a_technique_name, "流星絕影劍");
        assert_eq!(report.rounds[0].b_technique_name, "泰山沉岳勢");

        // 驗證勝負有明確判定
        assert!(["side_a", "side_b", "draw"].contains(&report.winner.as_str()));
    }

    #[test]
    fn test_word_casket_operations() {
        let mut casket = WordCasket::new();
        assert_eq!(casket.total_count(), 0);
        assert_eq!(casket.total_distinct(), 0);

        casket.add("wu", 3);
        casket.add("zhan", 1);
        assert_eq!(casket.count("wu"), 3);
        assert_eq!(casket.count("zhan"), 1);
        assert_eq!(casket.total_count(), 4);
        assert_eq!(casket.total_distinct(), 2);
        assert!(casket.has("wu"));

        // 移除一部分
        assert!(casket.remove("wu", 2).is_ok());
        assert_eq!(casket.count("wu"), 1);
        assert_eq!(casket.total_count(), 2);

        // 溢出移除應報錯
        assert_eq!(
            casket.remove("wu", 5),
            Err(WordPlateError::InsufficientWordCount("wu".into(), 1, 5))
        );

        // 扣光時完全移除
        assert!(casket.remove("wu", 1).is_ok());
        assert_eq!(casket.count("wu"), 0);
        assert!(!casket.has("wu"));
        assert_eq!(casket.total_distinct(), 1);
    }

    #[test]
    fn test_positional_word_order_breaks_commutativity() {
        let registry = WordRegistry::global();

        // 使用三顆不同的詞元："su", "zhan", "qiang"
        assert!(registry.get_by_id("su").is_some());
        assert!(registry.get_by_id("zhan").is_some());
        assert!(registry.get_by_id("qiang").is_some());

        // 正序造句：[su, zhan, qiang]
        let inlays_abc = vec![
            StarInlay::new("N001", "su"),
            StarInlay::new("N002", "zhan"),
            StarInlay::new("N003", "qiang"),
        ];
        let sent_abc = StarSentence::new(inlays_abc).expect("正序三言句式建立成功");
        let tech_abc = sent_abc.synthesize(registry).expect("正序三言句式合成成功");

        // 逆序造句：[qiang, zhan, su]
        let inlays_cba = vec![
            StarInlay::new("N001", "qiang"),
            StarInlay::new("N002", "zhan"),
            StarInlay::new("N003", "su"),
        ];
        let sent_cba = StarSentence::new(inlays_cba).expect("逆序三言句式建立成功");
        let tech_cba = sent_cba.synthesize(registry).expect("逆序三言句式合成成功");

        // 斷言起承轉合語序權重打破交換律：[A, B, C] != [C, B, A]
        assert_ne!(
            tech_abc.vector_12d.tempo_early,
            tech_cba.vector_12d.tempo_early,
            "起手先發時序受字序影響不相等"
        );
        assert_ne!(
            tech_abc.vector_12d.shock,
            tech_cba.vector_12d.shock,
            "震盪能量受字序影響不相等"
        );
    }

    #[test]
    fn test_semantic_contrast_and_dual_context_hundred_crafts() {
        let registry = WordRegistry::global();

        let w_qiang = registry.get_by_id("qiang").expect("強 詞元存在");
        let w_ruo = registry.get_by_id("ruo").expect("弱 詞元存在");
        let w_su = registry.get_by_id("su").expect("速 詞元存在");
        let w_huan = registry.get_by_id("huan").expect("緩 詞元存在");

        // 驗證語意對比：強的 shock 嚴格大於 弱，速的 tempo_early 嚴格大於 緩
        assert!(
            w_qiang.vector_12d.shock > w_ruo.vector_12d.shock,
            "強 之震盪衝擊 ({}) 應嚴格大於 弱 ({})",
            w_qiang.vector_12d.shock,
            w_ruo.vector_12d.shock
        );
        assert!(
            w_su.vector_12d.tempo_early > w_huan.vector_12d.tempo_early,
            "速 之起手先發 ({}) 應嚴格大於 緩 ({})",
            w_su.vector_12d.tempo_early,
            w_huan.vector_12d.tempo_early
        );

        // 以「鑄」「熾」「器」造句，驗證主修百工為鍛鑄百工且一字雙境描述完備
        assert!(registry.get_by_id("zhu").is_some());
        assert!(registry.get_by_id("chi").is_some());
        assert!(registry.get_by_id("qi_shou").is_some());

        let inlays = vec![
            StarInlay::new("N001", "zhu"),
            StarInlay::new("N002", "chi"),
            StarInlay::new("N003", "qi_shou"),
        ];
        let sent = StarSentence::new(inlays).expect("鑄熾器三言造句成功");
        let tech = sent.synthesize(registry).expect("鑄熾器合成成功");

        assert_eq!(
            tech.primary_craft,
            CraftAspect::Forge,
            "鑄熾器組合之主修藝相應為鍛鑄百工"
        );
        assert!(
            tech.craft_dims.is_non_zero(),
            "鍛鑄百工向量非零"
        );

        let summary = tech.dual_context_summary();
        assert!(!summary.is_empty(), "一字雙境摘要不可為空");
        assert!(summary.contains("【交鋒武相】"), "摘要應包含交鋒武相");
        assert!(summary.contains("【百工藝相】"), "摘要應包含百工藝相");
        assert!(summary.contains("鍛鑄百工"), "摘要應提及鍛鑄百工");
    }

    #[test]
    fn test_five_sources_coverage_and_roll_word_from_source() {
        let registry = WordRegistry::global();

        let beast = registry.words_by_source(WordSource::Beast);
        let plant = registry.words_by_source(WordSource::Plant);
        let mineral = registry.words_by_source(WordSource::Mineral);
        let water = registry.words_by_source(WordSource::Water);
        let qi = registry.words_by_source(WordSource::Qi);

        assert_eq!(beast.len(), 193, "獸源詞元應為 193 個");
        assert_eq!(plant.len(), 153, "植源詞元應為 153 個");
        assert_eq!(mineral.len(), 205, "礦源詞元應為 205 個");
        assert_eq!(water.len(), 161, "水源詞元應為 161 個");
        assert_eq!(qi.len(), 212, "氣源詞元應為 212 個");

        // 驗證抽取結果均包含對應源質標籤
        for (source, expected_tag) in [
            (WordSource::Beast, "獸"),
            (WordSource::Plant, "植"),
            (WordSource::Mineral, "礦"),
            (WordSource::Water, "水"),
            (WordSource::Qi, "氣"),
        ] {
            let elem = registry.roll_word_from_source(source, WordQuality::Bing, 42);
            assert!(
                elem.sources.iter().any(|s| s == expected_tag),
                "抽出的詞元 {:?} 應包含源質標籤 {}",
                elem.id,
                expected_tag
            );
        }
    }

    #[test]
    fn test_crystal_id_and_parsing_and_strip_ratio() {
        let id = crystal_item_id(WordSource::Beast, WordQuality::Jia);
        assert_eq!(id, "crystal_beast_jia");
        let parsed = parse_crystal_item_id(&id);
        assert_eq!(parsed, Some((WordSource::Beast, WordQuality::Jia)));

        let q = roll_strip_quality(0.5, 2, true, 12345);
        assert!(matches!(q, WordQuality::Bing | WordQuality::Yi | WordQuality::Jia));

        let ratio_civil = compute_extraction_ratio(0.8, false);
        assert!((3..=9).contains(&ratio_civil));
        let ratio_official = compute_extraction_ratio(0.2, true);
        assert!((7..=9).contains(&ratio_official));
    }

    #[test]
    fn test_casket_stack_refine_and_mint_to_magnesium() {
        let mut casket = WordCasket::new();

        // 未擁有時鎔鑄失敗
        assert!(casket.mint_to_magnesium("huo").is_err());

        // 存入 3 枚「火」
        casket.add_extracted("huo", 3, WordQuality::Bing, 7);
        assert_eq!(casket.count("huo"), 3);
        assert_eq!(casket.quality_of("huo"), WordQuality::Bing);
        assert_eq!(casket.ratio_of("huo"), 7);
        assert_eq!(casket.charge_of("huo"), 0);

        // 鎔鑄化鎂 1 枚
        let (mag_val, q, narrative) = casket.mint_to_magnesium("huo").expect("鎔鑄成功");
        assert_eq!(mag_val, 12);
        assert_eq!(q, WordQuality::Bing);
        assert!(narrative.contains("12 鎂"));
        assert_eq!(casket.count("huo"), 2);

        // 同名疊煉（目前有 2 枚，足夠消耗 1 枚）
        let (new_q, new_charge, _ascended, _refine_narrative) =
            casket.stack_refine("huo", 0.1, 7777).expect("疊煉成功");
        assert_eq!(casket.count("huo"), 1);
        assert!(new_charge > 0 || matches!(new_q, WordQuality::Yi | WordQuality::Jia));

        // 只剩 1 枚時再次疊煉應回傳不足錯誤
        let err = casket.stack_refine("huo", 0.1, 8888);
        assert!(matches!(err, Err(WordPlateError::InsufficientWordCount(_, 1, 2))));

        // 鎔鑄最後 1 枚，確認完全清理
        casket.mint_to_magnesium("huo").expect("最後一枚鎔鑄成功");
        assert_eq!(casket.count("huo"), 0);
        assert_eq!(casket.quality_of("huo"), WordQuality::Bing);
        assert_eq!(casket.charge_of("huo"), 0);
        assert_eq!(casket.ratio_of("huo"), 0);
    }

    #[test]
    fn test_tactical_loadout_craft_dims_and_xuan_luck_bonus() {
        let registry = WordRegistry::global();
        let mut casket = WordCasket::new();

        // 初始玄運加成 0
        let mut loadout = TacticalLoadout::new();
        assert_eq!(loadout.xuan_luck_bonus(&casket, registry), 0.0);

        // 放入「幸」「運」兩種玄意詞元
        casket.add("xing_xuan", 2);
        casket.add("yun", 1);
        let bonus = loadout.xuan_luck_bonus(&casket, registry);
        assert!((bonus - 0.05).abs() < 1e-4);

        // 配置開首招式
        let inlays = vec![
            StarInlay::new("N001", "zhu"),
            StarInlay::new("N002", "chi"),
            StarInlay::new("N003", "qi_shou"),
        ];
        let sent = StarSentence::new(inlays).expect("造句成功");
        loadout.set_slot(TacticalSlot::Opening, sent);

        let total_craft = loadout.total_craft_dims(registry);
        assert!(total_craft.forge > 0.0, "開首招式應有鍛鑄維度貢獻");
    }

    #[test]
    fn test_inscribe_item_capacity_and_returns() {
        let registry = WordRegistry::global();
        let mut casket = WordCasket::new();

        // 獲取前 4 個詞元 id
        let words: Vec<String> = registry.ordered_elements().take(4).map(|e| e.id.clone()).collect();
        assert_eq!(words.len(), 4);

        // 給予詞匣各 1 枚
        for w in &words {
            casket.add(w, 1);
        }

        // 銘刻前 3 字
        let res1 = casket.inscribe_item("item_blade_01", &words[0]);
        assert!(res1.is_ok());
        assert_eq!(casket.count(&words[0]), 0);

        let res2 = casket.inscribe_item("item_blade_01", &words[1]);
        assert!(res2.is_ok());
        assert_eq!(casket.count(&words[1]), 0);

        let res3 = casket.inscribe_item("item_blade_01", &words[2]);
        assert!(res3.is_ok());
        assert_eq!(casket.count(&words[2]), 0);
        assert_eq!(casket.inscribed_words_of("item_blade_01"), &[words[0].clone(), words[1].clone(), words[2].clone()]);

        // 銘刻第 4 字：最舊的 words[0] 必須被擠出並返還詞匣！
        let res4 = casket.inscribe_item("item_blade_01", &words[3]);
        assert!(res4.is_ok());
        assert_eq!(casket.count(&words[0]), 1, "最舊詞元應被歸還至詞匣");
        assert_eq!(casket.count(&words[3]), 0);
        assert_eq!(
            casket.inscribed_words_of("item_blade_01"),
            &[words[1].clone(), words[2].clone(), words[3].clone()]
        );

        // 銘文後綴展示
        let suffix = casket.format_inscribed_suffix("item_blade_01", registry);
        assert!(suffix.starts_with(" ["));
        assert!(suffix.ends_with(']'));

        // 銘文百工與戰鬥增益
        let craft_bonus = casket.inscribed_craft_bonus("item_blade_01", registry);
        assert!(craft_bonus.is_non_zero());
        let (atk, pen, def) = casket.inscribed_combat_bonus("item_blade_01", registry);
        assert!(atk > 0.0 || pen > 0.0 || def > 0.0);
    }

    #[test]
    fn test_infer_item_carrier_affinity_tools_as_weapons() {
        // 刀劍鋒芒：菜刀、柴刀
        assert_eq!(
            infer_item_carrier_affinity("tool_kitchen_knife", "圓弧雙柄削皮彎刀"),
            Some(CarrierAffinity::Blade)
        );
        assert_eq!(
            infer_item_carrier_affinity("woodcutting_machete", "精鋼柴刀"),
            Some(CarrierAffinity::Blade)
        );

        // 重錘鈍器：木棒/球棒、十字鎬、撬棍
        assert_eq!(
            infer_item_carrier_affinity("wooden_club", "結實木棒"),
            Some(CarrierAffinity::Blunt)
        );
        assert_eq!(
            infer_item_carrier_affinity("mining_pick", "青銅鶴嘴鎬"),
            Some(CarrierAffinity::Blunt)
        );
        assert_eq!(
            infer_item_carrier_affinity("iron_crowbar", "鍛鐵撬棍"),
            Some(CarrierAffinity::Blunt)
        );

        // 堅盾重禦：油紙傘、鍋蓋、木砧板
        assert_eq!(
            infer_item_carrier_affinity("rain_umbrella", "青竹油紙傘"),
            Some(CarrierAffinity::Shield)
        );
        assert_eq!(
            infer_item_carrier_affinity("pot_lid", "生鐵鍋蓋"),
            Some(CarrierAffinity::Shield)
        );

        // 長槍大戟：鐵鏟、長柄鋤頭
        assert_eq!(
            infer_item_carrier_affinity("farm_shovel", "掘地鐵鏟"),
            Some(CarrierAffinity::Pole)
        );

        // 步法身形：草鞋
        assert_eq!(
            infer_item_carrier_affinity("woven_shoes", "行路草鞋"),
            Some(CarrierAffinity::Footwork)
        );

        // 徒手拳掌：皮手套
        assert_eq!(
            infer_item_carrier_affinity("leather_gloves", "粗皮護腕手套"),
            Some(CarrierAffinity::Unarmed)
        );
    }

    #[test]
    fn test_ten_soul_seeds_produce_distinct_loadouts() {
        let registry = WordRegistry::global();
        let seeds = [
            1001i64, 2002i64, 3003i64, 4004i64, 5005i64,
            6006i64, 7007i64, 8008i64, 9009i64, 10010i64,
        ];

        let mut distinct_openings = HashSet::new();
        let mut distinct_names = HashSet::new();

        for &s in &seeds {
            let loadout = build_loadout_from_soul_seed(s, None, registry);
            assert!(loadout.opening.is_some(), "起手式不可為空");
            assert!(loadout.midgame.is_some(), "中盤式不可為空");
            assert!(loadout.finisher.is_some(), "終末式不可為空");
            assert!(loadout.defense.is_some(), "守勢不可為空");

            let op = loadout.opening.as_ref().unwrap();
            let word_seq: Vec<String> = op.inlays.iter().map(|i| i.word_id.clone()).collect();
            distinct_openings.insert(word_seq);

            if let Some(name) = &op.custom_name {
                distinct_names.insert(name.clone());
            }
        }

        assert_eq!(distinct_openings.len(), 10, "10 種不同的靈魂種子必須產出 10 種互異的起手式詞譜");
        assert_eq!(distinct_names.len(), 10, "10 種不同的靈魂種子必須產出 10 種互異的動態招式名號");
    }

    #[test]
    fn test_resolve_bout_style_countering_and_weight_class_domination() {
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(6).collect();

        // 構建巧勁流技法（高先發、高流轉、高破甲）
        let inlay_light = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N121", &words[2].id, OrbitFluency::ChangLiu),
        ];
        let sent_light = StarSentence::new(inlay_light).unwrap().with_custom_name("輕靈巧解式");
        let mut tech_light = sent_light.synthesize(registry).unwrap();
        tech_light.vector_12d.tempo_early = 1.5;
        tech_light.vector_12d.flow = 1.2;
        tech_light.vector_12d.penetration = 1.2;
        tech_light.vector_12d.shock = 0.2;
        tech_light.vector_12d.sustain = 0.2;

        // 構建剛猛厚重技法（高震盪、高持恆、厚重如全真剛硬）
        let inlay_heavy = vec![
            StarInlay::with_fluency("N002", &words[3].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[4].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N122", &words[5].id, OrbitFluency::PingWen),
        ];
        let sent_heavy = StarSentence::new(inlay_heavy).unwrap().with_custom_name("沉嶽劈山勢");
        let mut tech_heavy = sent_heavy.synthesize(registry).unwrap();
        tech_heavy.vector_12d.tempo_early = 0.5;
        tech_heavy.vector_12d.shock = 1.6;
        tech_heavy.vector_12d.sustain = 1.4;
        tech_heavy.vector_12d.penetration = 0.3;
        tech_heavy.vector_12d.flow = 0.3;

        // 情況一：同量級拆招（玉女克全真：以巧破剛）
        // 雙方量級均為 100.0（ratio 1.0 < 1.45）
        let bout_finesse = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_light),
            Some(&tech_heavy),
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_finesse.winner_side, "side_a");
        assert!(bout_finesse.a_manifested_as_skill, "巧勁勝出方應正式成招顯化為技能");
        assert!(!bout_finesse.b_manifested_as_skill, "受克制厚重方套路應崩散為碎招");
        assert!(bout_finesse.narrative.contains("碎招"), "敘事應明確體現散為碎招");
        assert!(bout_finesse.counter_reason.as_ref().unwrap().contains("以巧破剛"));

        // 情況二：跨量級碾壓（一力降十會）
        // 厚重方體魄量級提升至 250.0，巧勁方為 100.0（ratio 2.5 >= 1.45）
        let bout_crush = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_light),
            Some(&tech_heavy),
            1.0,
            1.0,
            100.0,
            250.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_crush.winner_side, "side_b");
        assert!(bout_crush.b_manifested_as_skill, "量級碾壓方成招顯化為技能");
        assert!(!bout_crush.a_manifested_as_skill, "巧勁方套路被絕對力量轟散為碎招");
        assert!(bout_crush.narrative.contains("碎招"));
        assert!(bout_crush.counter_reason.as_ref().unwrap().contains("重勢硬壓"));

        // 情況三：十會解一力（以無厚入有間：當重擊方一味蠻幹毫無守勢根底 sustain < 0.5，而庖丁入微者具備極致穿透與流轉 >= 2.6 時，可避開正面蠻力切入骨縫反拆）
        let mut tech_master_paoding = tech_light.clone();
        tech_master_paoding.vector_12d.penetration = 1.5;
        tech_master_paoding.vector_12d.flow = 1.4;
        let mut tech_clumsy_brute = tech_heavy.clone();
        tech_clumsy_brute.vector_12d.sustain = 0.2;
        let bout_paoding = resolve_bout(
            TacticalSlot::Opening,
            1,
            Some(&tech_master_paoding),
            Some(&tech_clumsy_brute),
            1.0,
            1.0,
            100.0,
            180.0,
            0.0,
            0.0,
            false,
            false,
        );
        assert_eq!(bout_paoding.winner_side, "side_a");
        assert!(bout_paoding.counter_reason.as_ref().unwrap().contains("入微破勢"));
    }

    #[test]
    fn test_posture_residue_and_fluency_integration() {
        // 1. PostureState 狀態與衰減
        assert_eq!(PostureState::default(), PostureState::Steady);
        assert_eq!(PostureState::Steady.label(), "架勢沉穩");
        assert_eq!(PostureState::Shaken.label(), "步履微晃");
        assert_eq!(PostureState::Broken.label(), "架勢潰散");
        assert_eq!(PostureState::Steady.degrade(), PostureState::Shaken);
        assert_eq!(PostureState::Shaken.degrade(), PostureState::Broken);
        assert_eq!(PostureState::Broken.degrade(), PostureState::Broken);

        // 2. BattlefieldResidue 與 ResidueInteraction 意象與標籤
        assert_eq!(BattlefieldResidue::ScorchingHeat.label(), "熾烈燥氣");
        assert_eq!(BattlefieldResidue::ScorchingHeat.keyword(), "燥");
        assert_eq!(BattlefieldResidue::FreezingFrost.label(), "凝寒霜氣");
        assert_eq!(BattlefieldResidue::FreezingFrost.keyword(), "寒");
        assert_eq!(BattlefieldResidue::DeepTremor.label(), "沉雄暗震");
        assert_eq!(BattlefieldResidue::DeepTremor.keyword(), "震");
        assert_eq!(BattlefieldResidue::GaleEdge.label(), "疾風銳勢");
        assert_eq!(BattlefieldResidue::GaleEdge.keyword(), "疾風");
        assert_eq!(BattlefieldResidue::VortexPull.label(), "牽引旋流");
        assert_eq!(BattlefieldResidue::VortexPull.keyword(), "旋流");

        assert!(ResidueInteraction::Resonance.label().contains("借勢共鳴"));
        assert!(ResidueInteraction::Backlash.label().contains("逆沖"));

        // 3. 同法不同相：相同字印在不同星軌通暢度（OrbitFluency）下推導出不同碎招序列
        let sample_chars = vec!["分".to_string(), "骨".to_string(), "斬".to_string()];
        let motions_smooth = derive_elementary_motions_with_fluency(
            &sample_chars,
            CarrierAffinity::Blade,
            TargetPart::Torso,
            OrbitFluency::ChangLiu,
        );
        let motions_hindered = derive_elementary_motions_with_fluency(
            &sample_chars,
            CarrierAffinity::Blade,
            TargetPart::Torso,
            OrbitFluency::ZhiSe,
        );
        assert_eq!(motions_smooth.len(), 3);
        assert_eq!(motions_hindered.len(), 3);
        assert_ne!(
            motions_smooth, motions_hindered,
            "同法不同相：暢流與滯澀之星軌通暢度應推導出完全不同的動作風格"
        );

        // 4. 三階共鳴倍率與階層
        let registry = WordRegistry::global();
        let words: Vec<&WordElement> = registry.ordered_elements().take(6).collect();

        // 5言暢流 -> tier 3 (1.55)
        let inlays_tier3 = vec![
            StarInlay::with_fluency("N001", &words[0].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N021", &words[1].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N121", &words[2].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N221", &words[3].id, OrbitFluency::ChangLiu),
            StarInlay::with_fluency("N321", &words[4].id, OrbitFluency::ChangLiu),
        ];
        let sent_tier3 = StarSentence::new(inlays_tier3).unwrap();
        let tech_tier3 = sent_tier3.synthesize(registry).unwrap();
        assert_eq!(tech_tier3.resonance_tier(), 3);
        assert!((tech_tier3.resonance_multiplier() - 1.55).abs() < 1e-6);

        // 3言平穩 -> tier 2 (1.25)
        let inlays_tier2 = vec![
            StarInlay::with_fluency("N002", &words[0].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N022", &words[1].id, OrbitFluency::PingWen),
            StarInlay::with_fluency("N122", &words[2].id, OrbitFluency::PingWen),
        ];
        let sent_tier2 = StarSentence::new(inlays_tier2).unwrap();
        let tech_tier2 = sent_tier2.synthesize(registry).unwrap();
        assert_eq!(tech_tier2.resonance_tier(), 2);
        assert!((tech_tier2.resonance_multiplier() - 1.25).abs() < 1e-6);

        // 3言險阻 -> tier 1 (1.0)
        let inlays_tier1 = vec![
            StarInlay::with_fluency("N003", &words[0].id, OrbitFluency::XianZu),
            StarInlay::with_fluency("N023", &words[1].id, OrbitFluency::XianZu),
            StarInlay::with_fluency("N123", &words[2].id, OrbitFluency::XianZu),
        ];
        let sent_tier1 = StarSentence::new(inlays_tier1).unwrap();
        let tech_tier1 = sent_tier1.synthesize(registry).unwrap();
        assert_eq!(tech_tier1.resonance_tier(), 1);
        assert!((tech_tier1.resonance_multiplier() - 1.0).abs() < 1e-6);

        // 5. resolve_bout_with_state：勢均力敵平局
        let bout_draw = resolve_bout_with_state(
            TacticalSlot::Opening,
            1,
            None,
            None,
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
            PostureState::Steady,
            PostureState::Steady,
            None,
        );
        assert_eq!(bout_draw.winner_side, "draw");
        assert_eq!(bout_draw.damage_to_a, 0);
        assert_eq!(bout_draw.damage_to_b, 0);
        assert_eq!(bout_draw.a_posture_after, PostureState::Steady);
        assert_eq!(bout_draw.b_posture_after, PostureState::Steady);

        // 6. resolve_bout_with_state：殘留反沖（Backlash）導致架勢受阻下墜，但絕無自殘扣血（零自傷）
        let mut tech_flame = tech_tier2.clone();
        tech_flame.word_chars = vec!["烈".to_string(), "火".to_string()];
        tech_flame.vector_12d.tempo_early = 1.2;

        let mut tech_frost = tech_tier2.clone();
        tech_frost.word_chars = vec!["凝".to_string(), "霜".to_string()];
        tech_frost.vector_12d.tempo_early = 0.3;
        tech_frost.vector_12d[4] = 0.0;

        assert_eq!(tech_flame.emitted_residue(), Some(BattlefieldResidue::ScorchingHeat));
        assert_eq!(tech_frost.emitted_residue(), Some(BattlefieldResidue::FreezingFrost));

        // 戰場殘留「熾烈燥氣」，甲方出燥氣（順應共鳴），乙方出霜氣（煞氣逆沖）
        let bout_residue = resolve_bout_with_state(
            TacticalSlot::Opening,
            1,
            Some(&tech_flame),
            Some(&tech_frost),
            1.0,
            1.0,
            100.0,
            100.0,
            0.0,
            0.0,
            false,
            false,
            PostureState::Steady,
            PostureState::Steady,
            Some(BattlefieldResidue::ScorchingHeat),
        );
        assert_eq!(bout_residue.winner_side, "side_a");
        assert_eq!(bout_residue.a_residue_interaction, Some(ResidueInteraction::Resonance));
        assert_eq!(bout_residue.b_residue_interaction, Some(ResidueInteraction::Backlash));
        // 甲方無自傷，乙方傷害僅來自甲方的單向輸出（無自我反噬自傷）
        assert_eq!(bout_residue.damage_to_a, 0);
        assert!(bout_residue.damage_to_b > 0);
        // 乙方遭受反沖且受到傷害，架勢必然衰減為步履微晃或潰散
        assert_ne!(bout_residue.b_posture_after, PostureState::Steady);

        // 7. TacticalClashReport.summary 敘事嚴禁出現任何半形或阿拉伯數字
        let mut loadout_a = TacticalLoadout::new();
        let mut loadout_b = TacticalLoadout::new();
        loadout_a.set_slot(TacticalSlot::Opening, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Midgame, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Finisher, sent_tier2.clone());
        loadout_a.set_slot(TacticalSlot::Defense, sent_tier2.clone());

        loadout_b.set_slot(TacticalSlot::Opening, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Midgame, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Finisher, sent_tier1.clone());
        loadout_b.set_slot(TacticalSlot::Defense, sent_tier1.clone());

        let report = resolve_tianji_matchup(&loadout_a, 100, &loadout_b, 100, registry).expect("田忌對決成功");
        assert!(
            !report.summary.chars().any(|c| c.is_ascii_digit()),
            "戰報總結必須是純文字話語，嚴格禁止出現任何阿拉伯數字：{}",
            report.summary
        );
    }

    #[test]
    fn test_auto_detour_meridian_note_and_somatic_awareness() {
        let registry = WordRegistry::global();
        let seeds = [100_003i64, 42i64, 999_999i64, -12_345i64, 0i64];

        for &seed in &seeds {
            let awareness = somatic_meridian_awareness(seed);
            assert!(
                !awareness.is_empty(),
                "體感覺察不可為空"
            );
            assert!(
                !awareness.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "體感覺察嚴禁包含任何阿拉伯數字或全形數字：{}",
                awareness
            );
            assert!(
                !awareness.contains("G0") && !awareness.contains("g0"),
                "體感覺察嚴禁外洩內部群組代號 G0：{}",
                awareness
            );
            assert!(
                awareness.contains("交手發力之際") || awareness.contains("暢流"),
                "體感覺察必須包含關鍵體察字詞：{}",
                awareness
            );

            // 測試詞元造句之自動繞行體感
            let sample_words: Vec<&WordEntry> = ["wu", "zhan", "su", "pi", "dun_dong"]
                .iter()
                .filter_map(|id| registry.get_by_id(id))
                .collect();

            let note = compute_auto_detour_meridian_note(&sample_words, seed);
            assert!(
                !note.is_empty(),
                "經脈繞行描述不可為空"
            );
            assert!(
                !note.chars().any(|c| c.is_ascii_digit() || ('０'..='傷').contains(&c) && c.is_numeric()),
                "經脈繞行描述嚴禁包含任何數字：{}",
                note
            );
            assert!(
                !note.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "經脈繞行描述嚴禁包含數字：{}",
                note
            );
            assert!(
                !note.contains("G0") && !note.contains("g0"),
                "經脈繞行描述嚴禁外洩代號 G0：{}",
                note
            );

            // 驗證合成招式中包含經脈繞行資訊與 practice_feel
            let inlays = vec![
                StarInlay::new("N001", "wu"),
                StarInlay::new("N002", "zhan"),
                StarInlay::new("N003", "su"),
            ];
            let sentence = StarSentence::new(inlays)
                .expect("造句成功")
                .with_soul_seed(seed);
            let tech = sentence.synthesize(registry).expect("合成成功");
            assert_eq!(tech.meridian_detour_note, note);
            assert!(tech.practice_feel.contains(&note));
            assert!(
                !tech.practice_feel.chars().any(|c| c.is_ascii_digit() || ('０'..='９').contains(&c)),
                "演練體感嚴禁包含任何數字：{}",
                tech.practice_feel
            );
            assert!(
                !tech.practice_feel.contains("G0") && !tech.practice_feel.contains("g0"),
                "演練體感嚴禁包含代號 G0：{}",
                tech.practice_feel
            );
        }
    }
}

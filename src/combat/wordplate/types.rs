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

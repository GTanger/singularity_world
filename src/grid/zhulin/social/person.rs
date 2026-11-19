//! 無職稱真活居民本體：`1 底板（SoulSeed）× 4 內在插件槽` ＋ `SHALU_ZHULIN_WORLD_SEED` 零寫死生成。
//!
//! 鐵律：
//! 1. 零寫死人名、零固定職業框（身份永遠由當下所在空間與正在執行的行動觀測合成）。
//! 2. `10 核心執念 × 12 處事稜角 = 120 組` 脾氣採同街正交排斥抽樣，確保對街同行脾氣鮮明對立。
//! 3. 所有隨身器物與欠單皆為具名留痕的 `ProvenanceVessel`，八卦認知嚴格上限 8 條。

use serde::{Deserialize, Serialize};

/// 八卦與主觀認知槽硬上限（規格書 §4.2）。
pub const MAX_SUBJECTIVE_BELIEFS: usize = 8;

/// 槽位 1a：10 大核心執念。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CoreObsession {
    CraftPerfection, // 手藝極致
    HoardAndOwn,     // 攢鎂置產
    ProtectKin,      // 護短重義
    OutshinePeers,   // 爭強壓頭
    PlaySafe,        // 惜命求穩
    SeekWonders,     // 探秘尋奇
    FineAppetite,    // 饕餮養體
    HonorLedger,     // 清帳守信
    SelfMadeShop,    // 自立門戶
    BoldVengeance,   // 快意恩仇
}

impl CoreObsession {
    pub const ALL: [Self; 10] = [
        Self::CraftPerfection,
        Self::HoardAndOwn,
        Self::ProtectKin,
        Self::OutshinePeers,
        Self::PlaySafe,
        Self::SeekWonders,
        Self::FineAppetite,
        Self::HonorLedger,
        Self::SelfMadeShop,
        Self::BoldVengeance,
    ];

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::CraftPerfection => "手藝極致",
            Self::HoardAndOwn => "攢鎂置產",
            Self::ProtectKin => "護短重義",
            Self::OutshinePeers => "爭強壓頭",
            Self::PlaySafe => "惜命求穩",
            Self::SeekWonders => "探秘尋奇",
            Self::FineAppetite => "饕餮養體",
            Self::HonorLedger => "清帳守信",
            Self::SelfMadeShop => "自立門戶",
            Self::BoldVengeance => "快意恩仇",
        }
    }
}

/// 槽位 1b：12 種處事稜角。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConductEdge {
    Prideful,       // 死要面子
    PennyPincher,   // 算帳入骨
    GrudgeHolder,   // 記仇算舊帳
    TsundereSoft,   // 嘴硬心軟
    Snobbish,       // 看人下菜
    LooseTongued,   // 耳根軟愛傳話
    Perfectionist,  // 挑剔潔癖
    RiskGambler,    // 嗜賭敢衝
    Suspicious,     // 多疑防人
    BluntHothead,   // 急躁直腸
    QuietSlowWarm,  // 慢熱寡言
    PettyBargainer, // 愛佔小便宜
}

impl ConductEdge {
    pub const ALL: [Self; 12] = [
        Self::Prideful,
        Self::PennyPincher,
        Self::GrudgeHolder,
        Self::TsundereSoft,
        Self::Snobbish,
        Self::LooseTongued,
        Self::Perfectionist,
        Self::RiskGambler,
        Self::Suspicious,
        Self::BluntHothead,
        Self::QuietSlowWarm,
        Self::PettyBargainer,
    ];

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Prideful => "死要面子",
            Self::PennyPincher => "算帳入骨",
            Self::GrudgeHolder => "記仇算舊帳",
            Self::TsundereSoft => "嘴硬心軟",
            Self::Snobbish => "看人下菜",
            Self::LooseTongued => "耳根軟愛傳話",
            Self::Perfectionist => "挑剔潔癖",
            Self::RiskGambler => "嗜賭敢衝",
            Self::Suspicious => "多疑防人",
            Self::BluntHothead => "急躁直腸",
            Self::QuietSlowWarm => "慢熱寡言",
            Self::PettyBargainer => "愛佔小便宜",
        }
    }
}

/// 槽位 1：骨底脾氣組合（120 組正交空間）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemperamentPlugin {
    pub obsession: CoreObsession,
    pub edge: ConductEdge,
}

/// 槽位 2：當下缺口插件（純內生消耗驅動，區間 `[0.0, 1.0]`，絕不外露原始數字）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActiveDeficit {
    /// 飢腸口糧缺口
    pub hunger: f32,
    /// 筋骨疲乏缺口
    pub fatigue: f32,
    /// 進山暗傷缺口
    pub injury: f32,
    /// 隨身工具捲刃磨損缺口
    pub tool_wear: f32,
    /// 爐台／鋪面備料短缺缺口
    pub stock_gap: f32,
    /// 房租與帳期催迫缺口
    pub debt_pressure: f32,
}

impl Default for ActiveDeficit {
    fn default() -> Self {
        Self {
            hunger: 0.20,
            fatigue: 0.15,
            injury: 0.0,
            tool_wear: 0.10,
            stock_gap: 0.15,
            debt_pressure: 0.05,
        }
    }
}

impl ActiveDeficit {
    pub fn clamp_all(&mut self) {
        self.hunger = self.hunger.clamp(0.0, 1.0);
        self.fatigue = self.fatigue.clamp(0.0, 1.0);
        self.injury = self.injury.clamp(0.0, 1.0);
        self.tool_wear = self.tool_wear.clamp(0.0, 1.0);
        self.stock_gap = self.stock_gap.clamp(0.0, 1.0);
        self.debt_pressure = self.debt_pressure.clamp(0.0, 1.0);
    }

    /// 轉譯為去數值化台灣白話體態短語。
    pub fn body_vibe_zh(&self) -> &'static str {
        if self.injury >= 0.55 {
            "步履微跛、肩背帶著暗傷"
        } else if self.hunger >= 0.72 {
            "神色略顯飢乏，正盤算著覓口熱食"
        } else if self.fatigue >= 0.75 {
            "眉間透著倦意，打算找處歇腳"
        } else if self.debt_pressure >= 0.65 {
            "眉頭緊鎖，正盤算著手頭帳期"
        } else if self.tool_wear >= 0.65 {
            "腰間器物刃口微鈍，正惦記著找人校磨"
        } else {
            "神色從容，氣息勻暢"
        }
    }
}

/// 槽位 3：器物留痕事件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceEvent {
    pub game_day: i32,
    pub note: String,
}

/// 槽位 3：隨身留痕器物／欠單實物插件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceVessel {
    pub vessel_id: String,
    pub item_id: String,
    pub label: String,
    pub maker_id: String,
    pub history: Vec<ProvenanceEvent>,
}

impl ProvenanceVessel {
    pub fn record_event(&mut self, game_day: i32, note: impl Into<String>) {
        self.history.push(ProvenanceEvent {
            game_day,
            note: note.into(),
        });
        if self.history.len() > 6 {
            let overflow = self.history.len() - 6;
            self.history.drain(0..overflow);
        }
    }

    /// 去數值化觀察描述（一眼讀出器物過往）。
    pub fn inspect_line(&self) -> String {
        if let Some(last) = self.history.last() {
            format!("{}（{}）", self.label, last.note)
        } else {
            self.label.clone()
        }
    }
}

/// 槽位 4：主觀認知與八卦種類。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeliefKind {
    WitnessedTrade {
        target_id: String,
        item_label: String,
    },
    WitnessedDebt {
        debtor_id: String,
        creditor_id: String,
    },
    WitnessedRescue {
        rescuer_id: String,
        saved_id: String,
    },
    SuspectedPoach {
        suspect_id: String,
        gx: i32,
        gy: i32,
    },
    SecretGroveTip {
        gx: i32,
        gy: i32,
        quality_tag: String,
    },
    StreetRumor {
        summary: String,
    },
}

/// 槽位 4：單條帶來源之主觀認知（嚴格上限 8 條）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectiveBelief {
    pub kind: BeliefKind,
    pub source_actor_id: String,
    pub game_day: i32,
    /// `1..=100` 內部確信度（僅供後端推演，不外露數字）
    pub confidence: u8,
}

/// 當前正在執行的生活意圖（純動態情境狀態，非永久職業）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResidentActivityKind {
    TendHomeRest,        // 歸宅歇息
    EatWarmMeal,         // 赴街市覓食
    TendHearthStove,     // 守灶烹煮
    ForgeAndSharpen,     // 開爐鍛打校磨
    TendFarmPlot,        // 照料熟地農畦
    RunArcadeCounter,    // 坐鎮騎樓前堂應市
    HaulCartGoods,       // 運貨穿梭街口
    ForageBambooWild,    // 入竹海採筍伐竹
    SeekClinicCare,      // 尋醫敷藥療傷
    CollectOrSettleDebt, // 催討或清償欠帳
    StreetGossip,        // 街頭打聽消息
}

impl ResidentActivityKind {
    /// 依當下行為合成無職稱情境觀察稱呼（未結識時使用，絕非寫死職業標籤）。
    pub fn observed_role_zh(self) -> &'static str {
        match self {
            Self::TendHearthStove => "灶前掌杓人",
            Self::ForgeAndSharpen => "爐畔鍛工",
            Self::TendFarmPlot => "田間農人",
            Self::RunArcadeCounter => "騎樓掌櫃",
            Self::HaulCartGoods => "推車腳夫",
            Self::ForageBambooWild => "背簍採筍客",
            Self::SeekClinicCare => "負傷求診客",
            Self::CollectOrSettleDebt => "討帳過客",
            Self::EatWarmMeal => "街市食客",
            Self::StreetGossip => "街坊閒客",
            Self::TendHomeRest => "歸宅住戶",
        }
    }

    pub fn activity_phrase_zh(self) -> &'static str {
        match self {
            Self::TendHearthStove => "正守在灶邊翻炒熱食，香氣四溢",
            Self::ForgeAndSharpen => "正守在炭爐旁敲打校磨器物刃口",
            Self::TendFarmPlot => "正俯身整理熟地田壟與引水溝",
            Self::RunArcadeCounter => "正站在騎樓前堂招呼街坊清點貨色",
            Self::HaulCartGoods => "正推著載貨竹車沿石板街往來送貨",
            Self::ForageBambooWild => "正背著竹簍在莿竹林間尋筍採材",
            Self::SeekClinicCare => "正按著患處沿街尋醫調理傷勢",
            Self::CollectOrSettleDebt => "正攥著欠單在街上盤算清算舊帳",
            Self::EatWarmMeal => "正坐下來端起熱湯乾糧補充氣力",
            Self::StreetGossip => "正倚在騎樓柱旁與過客交換四方風聞",
            Self::TendHomeRest => "正回到住處床榻旁歇息養神",
        }
    }
}

/// 單一無職稱真活居民實體（1 底板 × 4 內在插件槽）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialResident {
    pub id: String,
    pub soul_seed: u64,
    pub true_name: String,
    pub gender_is_female: bool,
    /// 當前三維空間位置（與玩家共用 `grid_x, grid_y, grid_z`）
    pub gx: i32,
    pub gy: i32,
    pub gz: i16,
    /// 慣常落腳住處／騎樓座標（可隨破產或置產流動遷移）
    pub home_gx: i32,
    pub home_gy: i32,
    pub home_gz: i16,
    /// 初始營生起點是否為竹海先鋒（約 15%），後續隨經濟流動自由轉換
    pub pioneer_origin: bool,
    /// 隨身鎂晶（嚴格守恆）
    pub magnesium: i32,
    /// 隨身物資存量（簡明守恆品項：`food`, `bamboo`, `shoot`, `herb`）
    pub food_stock: i32,
    pub bamboo_stock: i32,
    pub shoot_stock: i32,
    pub herb_stock: i32,
    /// 手藝與採獵歷練（自然成長，形成優先連接樞紐）
    pub craft_skill: f32,
    pub wild_skill: f32,
    /// 槽位 1：骨底脾氣
    pub temperament: TemperamentPlugin,
    /// 槽位 2：當下缺口
    pub deficit: ActiveDeficit,
    /// 槽位 3：隨身留痕器物（最多保留 4 件代表器物）
    pub vessels: Vec<ProvenanceVessel>,
    /// 槽位 4：主觀認知與八卦（上限 8 條）
    pub beliefs: Vec<SubjectiveBelief>,
    /// 當前正在執行的意圖與已持續拍數（供 Hysteresis 遲滯防抖）
    pub current_activity: ResidentActivityKind,
    pub activity_ticks: u16,
}

impl SocialResident {
    /// 寫入一條主觀認知（同源去重，嚴格維持上限 `8` 條）。
    pub fn push_belief(&mut self, belief: SubjectiveBelief) {
        if let Some(existing) = self.beliefs.iter_mut().find(|b| b.kind == belief.kind) {
            existing.game_day = belief.game_day;
            existing.confidence = existing.confidence.max(belief.confidence);
            existing.source_actor_id = belief.source_actor_id;
            return;
        }
        if self.beliefs.len() >= MAX_SUBJECTIVE_BELIEFS {
            if let Some((drop_idx, _)) = self
                .beliefs
                .iter()
                .enumerate()
                .min_by_key(|(_, b)| (b.confidence, b.game_day))
            {
                self.beliefs.remove(drop_idx);
            }
        }
        self.beliefs.push(belief);
    }

    /// 供未結識觀察者看到的無職稱稱呼（已結識始回傳 `true_name`）。
    pub fn observed_name(&self, known: bool) -> &str {
        if known {
            &self.true_name
        } else {
            self.current_activity.observed_role_zh()
        }
    }
}

/// SplitMix64 確定性混洗（零外部隨機狀態，重跑 100% 等價）。
#[inline]
pub fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

const SURNAMES: [&str; 40] = [
    "陳", "林", "黃", "張", "李", "王", "吳", "劉", "蔡", "楊", "許", "鄭", "謝", "洪", "郭", "邱",
    "曾", "廖", "賴", "徐", "周", "葉", "蘇", "莊", "呂", "江", "何", "蕭", "羅", "高", "潘", "簡",
    "朱", "鍾", "游", "彭", "詹", "胡", "施", "沈",
];

const MALE_GIVEN_A: [&str; 24] = [
    "阿", "文", "志", "建", "明", "清", "家", "俊", "永", "正", "國", "宏", "進", "木", "火", "石",
    "金", "順", "添", "榮", "慶", "義", "德", "承",
];
const MALE_GIVEN_B: [&str; 24] = [
    "成", "財", "旺", "土", "水", "生", "發", "福", "山", "海", "河", "泉", "良", "雄", "豪", "忠",
    "平", "安", "和", "興", "隆", "昌", "盛", "豐",
];
const FEMALE_GIVEN_A: [&str; 24] = [
    "秀", "淑", "美", "惠", "麗", "玉", "秋", "春", "素", "月", "金", "寶", "彩", "碧", "芬", "芳",
    "怡", "雅", "靜", "婉", "慧", "佳", "宜", "欣",
];
const FEMALE_GIVEN_B: [&str; 24] = [
    "蘭", "英", "梅", "珍", "華", "芬", "霞", "珠", "蓮", "菊", "香", "琴", "雲", "枝", "花", "玲",
    "君", "婷", "如", "雯", "貞", "惠", "敏", "青",
];

fn derive_taiwan_name(seed: u64, is_female: bool) -> String {
    let h1 = splitmix64(seed ^ 0x11);
    let h2 = splitmix64(seed ^ 0x22);
    let h3 = splitmix64(seed ^ 0x33);
    let sur = SURNAMES[(h1 as usize) % SURNAMES.len()];
    let (a, b) = if is_female {
        (
            FEMALE_GIVEN_A[(h2 as usize) % FEMALE_GIVEN_A.len()],
            FEMALE_GIVEN_B[(h3 as usize) % FEMALE_GIVEN_B.len()],
        )
    } else {
        (
            MALE_GIVEN_A[(h2 as usize) % MALE_GIVEN_A.len()],
            MALE_GIVEN_B[(h3 as usize) % MALE_GIVEN_B.len()],
        )
    };
    if h1 % 7 == 0 {
        format!("{sur}{b}")
    } else {
        format!("{sur}{a}{b}")
    }
}

/// 西麓街鎮騎樓街屋出生與落腳點位（全數為具二樓住家之騎樓街屋，避免在露天石板街面歇息）。
const TOWN_ANCHOR_COORDS: [(i32, i32, i16); 13] = [
    (-6, 1, 0),
    (-5, 1, 0),
    (-4, 1, 0),
    (-3, 1, 0),
    (-2, 1, 0),
    (-6, -1, 0),
    (-5, -1, 0),
    (-4, -1, 0),
    (-3, -1, 0),
    (-2, -1, 0),
    (-5, -2, 0),
    (-4, -2, 0),
    (-3, -2, 0),
];

const WILD_PIONEER_COORDS: [(i32, i32); 8] = [
    (1, 0),
    (2, 0),
    (1, -1),
    (2, 1),
    (3, -1),
    (0, -2),
    (-1, -2),
    (2, -2),
];

/// 以 `SHALU_ZHULIN_WORLD_SEED` 零寫死確定性生成 `count` 名（建議 `300..=400`）真活居民。
///
/// - 約 85% 出生於城內熟地與騎樓街鎮、約 15% 出生為竹海採獵先鋒。
/// - 同一街屋起點相鄰居民自動執行「正交排斥抽樣」，確保脾氣鮮明對立。
pub fn generate_residents(world_seed: u64, count: usize) -> Vec<SocialResident> {
    let mut residents = Vec::with_capacity(count);
    let mut used_names = std::collections::HashSet::with_capacity(count);
    let mut last_temp_by_anchor: std::collections::HashMap<(i32, i32), TemperamentPlugin> =
        std::collections::HashMap::new();

    for idx in 0..count {
        let base_seed = splitmix64(world_seed.wrapping_add((idx as u64).wrapping_mul(0xA5A5_1337)));
        let is_female = (base_seed & 1) == 1;
        // 避免重名
        let mut name_salt = 0u64;
        let true_name = loop {
            let cand = derive_taiwan_name(base_seed.wrapping_add(name_salt), is_female);
            if used_names.insert(cand.clone()) {
                break cand;
            }
            name_salt = name_salt.wrapping_add(1);
        };

        // 85% 城鎮熟地 : 15% 竹海先鋒
        let pioneer_origin = (idx % 20) < 3; // 精準 15%
        let (home_gx, home_gy, home_gz) = TOWN_ANCHOR_COORDS[idx % TOWN_ANCHOR_COORDS.len()];
        let (gx, gy, gz) = if pioneer_origin {
            let (wx, wy) = WILD_PIONEER_COORDS[idx % WILD_PIONEER_COORDS.len()];
            (wx, wy, 0)
        } else {
            (home_gx, home_gy, home_gz)
        };

        // 正交排斥抽樣：同起點下一位居民避開前一位之執念與稜角
        let mut obs_idx = ((base_seed >> 8) as usize) % CoreObsession::ALL.len();
        let mut edge_idx = ((base_seed >> 16) as usize) % ConductEdge::ALL.len();
        if let Some(prev) = last_temp_by_anchor.get(&(home_gx, home_gy)) {
            if CoreObsession::ALL[obs_idx] == prev.obsession {
                obs_idx =
                    (obs_idx + 1 + ((base_seed >> 24) as usize % 3)) % CoreObsession::ALL.len();
            }
            if ConductEdge::ALL[edge_idx] == prev.edge {
                edge_idx =
                    (edge_idx + 1 + ((base_seed >> 28) as usize % 5)) % ConductEdge::ALL.len();
            }
        }
        let temperament = TemperamentPlugin {
            obsession: CoreObsession::ALL[obs_idx],
            edge: ConductEdge::ALL[edge_idx],
        };
        last_temp_by_anchor.insert((home_gx, home_gy), temperament);

        let id = format!("zl_res_{:04}", idx + 1);
        let h4 = splitmix64(base_seed ^ 0x44);
        let h5 = splitmix64(base_seed ^ 0x55);
        let magnesium = 12 + (h4 % 48) as i32; // 12..59 初始本金，呈自然差異（無特權大戶寫死）
        let craft_skill = 0.25 + ((h4 >> 12) % 55) as f32 / 100.0;
        let wild_skill = if pioneer_origin {
            0.45 + ((h5 >> 8) % 45) as f32 / 100.0
        } else {
            0.15 + ((h5 >> 8) % 35) as f32 / 100.0
        };

        let (vessel_item, vessel_label) = if pioneer_origin {
            ("zl_item_bamboo_stone_axe", "老籜纏柄開山竹斧")
        } else {
            match idx % 4 {
                0 => ("zl_item_iron_cleaver", "厚背熟鐵片刀"),
                1 => ("zl_item_forge_tongs", "雙股淬火鍛鉗"),
                2 => ("zl_item_farm_hoe", "寬刃熟地鐵鋤"),
                _ => ("zl_item_bamboo_pack", "雙肩密編老竹簍"),
            }
        };
        let starter_vessel = ProvenanceVessel {
            vessel_id: format!("vsl_{:04}_0", idx + 1),
            item_id: vessel_item.to_string(),
            label: vessel_label.to_string(),
            maker_id: format!("zl_res_{:04}", ((idx + 7) % count) + 1),
            history: vec![ProvenanceEvent {
                game_day: 0,
                note: "開埠初年自西麓街坊校妥隨身".to_string(),
            }],
        };

        let initial_activity = if pioneer_origin {
            ResidentActivityKind::ForageBambooWild
        } else {
            match idx % 5 {
                0 => ResidentActivityKind::TendFarmPlot,
                1 => ResidentActivityKind::TendHearthStove,
                2 => ResidentActivityKind::ForgeAndSharpen,
                3 => ResidentActivityKind::RunArcadeCounter,
                _ => ResidentActivityKind::HaulCartGoods,
            }
        };

        residents.push(SocialResident {
            id,
            soul_seed: base_seed,
            true_name,
            gender_is_female: is_female,
            gx,
            gy,
            gz,
            home_gx,
            home_gy,
            home_gz,
            pioneer_origin,
            magnesium,
            food_stock: 3 + (h5 % 4) as i32,
            bamboo_stock: if pioneer_origin { 2 } else { 1 },
            shoot_stock: if pioneer_origin { 1 } else { 0 },
            herb_stock: (h5 % 2) as i32,
            craft_skill,
            wild_skill,
            temperament,
            deficit: ActiveDeficit {
                hunger: 0.15 + ((h4 >> 20) % 25) as f32 / 100.0,
                fatigue: 0.10 + ((h5 >> 20) % 25) as f32 / 100.0,
                injury: 0.0,
                tool_wear: 0.05 + ((h4 >> 28) % 20) as f32 / 100.0,
                stock_gap: 0.10 + ((h5 >> 28) % 25) as f32 / 100.0,
                debt_pressure: 0.0,
            },
            vessels: vec![starter_vessel],
            beliefs: Vec::new(),
            current_activity: initial_activity,
            activity_ticks: 0,
        });
    }

    residents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grid::SHALU_ZHULIN_WORLD_SEED;

    #[test]
    fn generates_320_unique_residents_with_85_15_ratio_and_orthogonal_temperament() {
        let list = generate_residents(SHALU_ZHULIN_WORLD_SEED, 320);
        assert_eq!(list.len(), 320);
        let pioneers = list.iter().filter(|r| r.pioneer_origin).count();
        assert_eq!(pioneers, 48); // 精準 15%
        let unique_names: std::collections::HashSet<&str> =
            list.iter().map(|r| r.true_name.as_str()).collect();
        assert_eq!(unique_names.len(), 320);
        // 確定性：重跑完全一致
        let again = generate_residents(SHALU_ZHULIN_WORLD_SEED, 320);
        assert_eq!(list[0].true_name, again[0].true_name);
        assert_eq!(list[319].soul_seed, again[319].soul_seed);
    }

    #[test]
    fn belief_slot_caps_at_8() {
        let mut list = generate_residents(SHALU_ZHULIN_WORLD_SEED, 2);
        let r = &mut list[0];
        for d in 1..=15 {
            r.push_belief(SubjectiveBelief {
                kind: BeliefKind::StreetRumor {
                    summary: format!("傳聞第{d}則"),
                },
                source_actor_id: "zl_res_0002".to_string(),
                game_day: d,
                confidence: (30 + d) as u8,
            });
        }
        assert_eq!(r.beliefs.len(), MAX_SUBJECTIVE_BELIEFS);
    }
}

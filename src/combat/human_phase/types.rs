/// 模組一：先天稟賦雙層守恆（對數功率守恆 × 帶界單純形 10,000 阻抗矩陣）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InnateBonePhase {
    pub seed: i64,
    pub alpha: f64,      // [0.5, 2.0] 力矩峰值倍率基底
    pub beta: f64,       // [0.5, 2.0] 出手頻率基底 (alpha * beta in [0.861, 1.162])
    pub gamma: f64,      // [-1.0, 1.0] >0 守正秩序相, <0 奇正流轉相
    pub r_matrix: [[f64; 5]; 3], // 3 (0:體, 1:氣, 2:敏) x 5 (0:起, 1:承, 2:轉, 3:協, 4:合), sum == 10000.0, each in [300.0, 1200.0]
    pub vit_0: f64,      // 50.0 + 0.02 * (6000.0 - sum_v r_matrix[0][v])
    pub qi_0: f64,       // 50.0 + 0.02 * (6000.0 - sum_v r_matrix[1][v])
    pub dex_0: f64,      // 50.0 + 0.02 * (6000.0 - sum_v r_matrix[2][v]) (vit_0 + qi_0 + dex_0 == 310.0)
}

/// 碎招運動學參數定義
#[derive(Debug, Clone)]
pub struct ElementaryMotionDef {
    pub name: String,
    pub carrier: CarrierAffinity,
    pub craft_domain: Option<&'static str>, // None for martial; Some("harvest"|"forge"|"culinary"|"wayfare") for craft
    pub stage: usize, // 0:起, 1:承, 2:轉, 3:協, 4:合
    pub p_in: [f64; 4],  // [z, omega, v_r, theta]
    pub p_out: [f64; 4], // [z, omega, v_r, theta]
    pub w_3d: [f64; 3],  // 體、氣、敏 (sum == 1.0)
    pub torque_kappa: f64,
    pub edge_focus_p: f64,
    pub timing: (u8, u8, u8), // (tau_pre, tau_act, tau_rec) in Ticks
    pub counter_fit: [f64; 3], // [剛猛硬架, 後撤閃避, 貼身搶攻]
    pub feature_16d: [f64; 16], // 12D 戰鬥 + 4D 百工
}

/// 湧現成相套路
#[derive(Debug, Clone)]
pub struct EmergentRoutine {
    pub domain: &'static str, // "martial" or "production"
    pub slot_code: &'static str, // "opening" | "midgame" | "finisher" | "defense" | "harvest" | "forge" | "culinary" | "wayfare"
    pub slot_label_zh: &'static str, // "起手武相" | "連攜武相" | "破綻武相" | "守勢武相" | "採獵藝相" | "百工藝相" | "庖廚藝相" | "行旅藝相"
    pub carrier: CarrierAffinity,
    pub main_motions: [String; 5], // 起、承、轉、協、合 五常碎招名稱
    pub turn_branch_motion: String, // 「轉」位應變分支碎招名稱
    pub turn_counter_target_zh: &'static str, // "剛猛硬架" | "後撤閃避" | "貼身搶攻"
    pub compensated_stages: Vec<usize>, // 因個人骨相阻抗而自動代償繞路之階段索引 (0..5)
    pub continuity_c: f64, // \bar{C}_e(T)
    pub variation_v: f64,  // V_e(T)
    pub max_gap_ticks: u8, // \tau^{gap}_e in Ticks (>= 1)
    pub phase_specialty_zh: &'static str, // "一力降十會（正面硬撼）" | "專克拆招（玉女克全真）" | "庖丁解牛（入微穿隙）"
    pub omp_roots: (String, String), // (w1_char, w2_char)
    pub omp_word_ids: (String, String), // (w1_id, w2_id)
    pub formal_name: String, // 自然成相真名
    pub origin_basis_zh: String, // 完整白話成相根源與代償說明
    pub learned_and_familiar_zh: String, // 五常碎招鏈、轉位應變、隨身器物與語意字根說明
    pub auto_trigger_desc_zh: String, // 連貫度、換招空檔與三相判準說明
}

/// 三相交鋒勝負判準
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThreePhaseVerdict {
    Phase1ImpulseCrush { winner_is_a: bool, lambda_ratio: f64 },
    Phase2KinematicIntercept { interceptor_is_a: bool, intercept_score: f64, gap_ticks: f64 },
    Phase3SeamThreading { threader_is_a: bool, xi_index: f64, bypass_pct: f64 },
    StandardExchange,
}

/// 三相交鋒結算結果
#[derive(Debug, Clone)]
pub struct ThreePhaseEvalResult {
    pub verdict: ThreePhaseVerdict,
    pub score_delta_a: f64,
    pub score_delta_b: f64,
    pub counter_reason: Option<String>,
    pub a_manifested_as_skill: bool,
    pub b_manifested_as_skill: bool,
    pub a_completed_motions: usize,
    pub b_completed_motions: usize,
    pub a_posture_after: PostureState,
    pub b_posture_after: PostureState,
}

/// 第三相入微穿隙（庖丁解牛）結算
#[derive(Debug, Clone, Copy)]
pub struct SeamThreadingEval {
    pub xi_index: f64,        // [0.0, 2.30]
    pub hardness_bypass: f64, // 1.0 - exp(-xi_index), up to ~0.90
    pub wear_multiplier: f64, // max(0.05, exp(-xi_index))
}

/// 山海野獸與生靈解剖學形體分類（人獸共處同一物理時序法則，惟肢體載體與本能碎招依解剖構造顯化）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeastAnatomy {
    /// 飛禽羽族（竹雞、環頸雉、藍腹鷴、帝雉、鷹、鷲、隼、鴞、鷺等）
    Winged,
    /// 鱗潛蛇蟒與兩棲水族（百步蛇、飯匙倩、龜殼花、花斑鱸鰻、山椒魚、樹蛙等）
    Serpentine,
    /// 重甲鱗殼（穿山甲、綠蠵龜、箱龜、青毛蟹、招潮蟹、海鱟等）
    Carapace,
    /// 角蹄走獸（水鹿、梅花鹿、山羌、長鬃山羊、山豬、水牛等）
    HoofedHorned,
    /// 利爪毛獸（雲豹、石虎、月熊、獼猴、白鼻心、食蟹獴、野灰兔等）
    Quadruped,
}

impl BeastAnatomy {
    pub fn from_species_title(title: &str) -> Self {
        if title.contains('雞')
            || title.contains('雉')
            || title.contains('鷴')
            || title.contains('鷹')
            || title.contains('鷲')
            || title.contains('隼')
            || title.contains('鴞')
            || title.contains('鳶')
            || title.contains('雕')
            || title.contains('鷺')
            || title.contains('鳥')
            || title.contains('鶇')
            || title.contains('雀')
            || title.contains('鷚')
            || title.contains('眉')
            || title.contains('燕')
            || title.contains('娘')
            || title.contains('鷗')
        {
            Self::Winged
        } else if title.contains('蛇')
            || title.contains('倩')
            || title.contains('花')
            || title.contains('鰻')
            || title.contains('魚')
            || title.contains('鮭')
            || title.contains('蛙')
            || title.contains('鯢')
            || title.contains("烏賊")
        {
            Self::Serpentine
        } else if title.contains('龜')
            || title.contains('鱉')
            || title.contains('鼇')
            || title.contains('蟹')
            || title.contains('鱟')
            || title.contains("穿山甲")
            || title.contains("鯪鯉")
            || title.contains('鮑')
            || title.contains('鱷')
        {
            Self::Carapace
        } else if title.contains('鹿')
            || title.contains('羌')
            || title.contains('羊')
            || title.contains('牛')
            || title.contains('豬')
        {
            Self::HoofedHorned
        } else {
            Self::Quadruped
        }
    }

    pub fn carrier_label(self) -> &'static str {
        match self {
            Self::Winged => "雙翼利爪",
            Self::Serpentine => "鱗軀毒牙",
            Self::Carapace => "堅甲巨螯",
            Self::HoofedHorned => "犄角勁蹄",
            Self::Quadruped => "利爪獠牙",
        }
    }
}

/// 依生靈解剖學形體與交鋒階段，推導符合生物本能之成招名稱與五段碎招動作鏈
pub fn natural_beast_moves(
    anatomy: BeastAnatomy,
    slot: TacticalSlot,
    seed: u64,
) -> (&'static str, Vec<String>) {
    let alt = (seed % 2) == 1;
    match (anatomy, slot) {
        (BeastAnatomy::Winged, TacticalSlot::Opening) => (
            if alt { "振翅疾掠" } else { "驚羽撲擊" },
            vec![
                "振翅低掠".into(),
                "貼地疾竄".into(),
                "側翼閃避".into(),
                "雙爪撲抓".into(),
                "銳喙疾啄".into(),
            ],
        ),
        (BeastAnatomy::Winged, TacticalSlot::Midgame) => (
            if alt { "穿林掠影" } else { "掠影啄擊" },
            vec![
                "騰空拍翼".into(),
                "凌空急折".into(),
                "飛羽掠影".into(),
                "利爪連抓".into(),
                "俯衝疾啄".into(),
            ],
        ),
        (BeastAnatomy::Winged, _) => (
            if alt { "裂風撲殺" } else { "貫勢撲啄" },
            vec![
                "昂首振翮".into(),
                "乘風疾撲".into(),
                "展翼橫掃".into(),
                "利爪扣擊".into(),
                "貫勢猛啄".into(),
            ],
        ),
        (BeastAnatomy::Serpentine, TacticalSlot::Opening) => (
            if alt { "破草疾竄" } else { "貼地突噬" },
            vec![
                "盤身蓄勢".into(),
                "貼地游竄".into(),
                "昂首吐信".into(),
                "曲身彈射".into(),
                "毒牙突噬".into(),
            ],
        ),
        (BeastAnatomy::Serpentine, TacticalSlot::Midgame) => (
            if alt { "游身絞擊" } else { "盤繞猛噬" },
            vec![
                "順勢游走".into(),
                "滑身繞側".into(),
                "鱗軀橫掃".into(),
                "盤繞絞纏".into(),
                "昂首猛噬".into(),
            ],
        ),
        (BeastAnatomy::Serpentine, _) => (
            if alt { "電射噬喉" } else { "甩尾狂噬" },
            vec![
                "縮身蓄勁".into(),
                "破草疾射".into(),
                "昂首張吻".into(),
                "甩尾抽擊".into(),
                "雷霆猛噬".into(),
            ],
        ),
        (BeastAnatomy::Carapace, TacticalSlot::Opening) => (
            if alt { "沉甲橫撞" } else { "鐵甲衝擊" },
            vec![
                "沉甲伏地".into(),
                "穩踏泥石".into(),
                "側身卸力".into(),
                "鐵甲衝撞".into(),
                "巨螯鉗擊".into(),
            ],
        ),
        (BeastAnatomy::Carapace, TacticalSlot::Midgame) => (
            if alt { "旋甲橫掃" } else { "雙螯夾擊" },
            vec![
                "縮首護腹".into(),
                "轉身旋甲".into(),
                "雙螯開合".into(),
                "橫身猛推".into(),
                "重尾掃擊".into(),
            ],
        ),
        (BeastAnatomy::Carapace, _) => (
            if alt { "覆甲崩撞" } else { "烏金捲撞" },
            vec![
                "背甲蓄勢".into(),
                "四足蹬地".into(),
                "烏金捲撞".into(),
                "巨力掀石".into(),
                "全力壓頂".into(),
            ],
        ),
        (BeastAnatomy::HoofedHorned, TacticalSlot::Opening) => (
            if alt { "蹬蹄突進" } else { "壓角衝撞" },
            vec![
                "低首壓角".into(),
                "前蹄刨土".into(),
                "蹬地疾衝".into(),
                "側身擺首".into(),
                "雙角頂撞".into(),
            ],
        ),
        (BeastAnatomy::HoofedHorned, TacticalSlot::Midgame) => (
            if alt { "蠻角橫掃" } else { "騰躍踹擊" },
            vec![
                "踏石騰躍".into(),
                "旋身甩首".into(),
                "長角架格".into(),
                "後蹄猛踹".into(),
                "昂首挑擊".into(),
            ],
        ),
        (BeastAnatomy::HoofedHorned, _) => (
            if alt { "奔雷貫角" } else { "狂奔猛撞" },
            vec![
                "長嘶蓄勢".into(),
                "四蹄狂奔".into(),
                "捲起塵土".into(),
                "垂首貫角".into(),
                "正面猛撞".into(),
            ],
        ),
        (BeastAnatomy::Quadruped, TacticalSlot::Opening) => (
            if alt { "伏叢撲擊" } else { "疾躍撕抓" },
            vec![
                "伏身蓄勢".into(),
                "四足蹬躍".into(),
                "破草疾撲".into(),
                "利爪撕抓".into(),
                "側身閃掠".into(),
            ],
        ),
        (BeastAnatomy::Quadruped, TacticalSlot::Midgame) => (
            if alt { "騰躍撕咬" } else { "旋身連撲" },
            vec![
                "繞身游走".into(),
                "曲肢急彈".into(),
                "雙爪按撲".into(),
                "利齒撕咬".into(),
                "甩尾掃擊".into(),
            ],
        ),
        (BeastAnatomy::Quadruped, _) => (
            if alt { "裂喉猛撲" } else { "狂暴撲殺" },
            vec![
                "低吼弓背".into(),
                "凌空撲躍".into(),
                "前爪重拍".into(),
                "獠牙鎖喉".into(),
                "全力撲殺".into(),
            ],
        ),
    }
}

const R_BAR: f64 = 10000.0 / 15.0; // ~666.6666667
const TAU_R: f64 = 140.0;
const SIGMA_P_SQ: f64 = 1.0; // 2 * sigma_p^2 = 2.0
const RHO_SELF: f64 = 0.55;
const ETA_L: f64 = 0.35;
pub const ETA_F: f64 = 0.65;
pub const F_0: f64 = 25.0;
pub const P_PRACTICE: f64 = 0.45;
pub const ETA_F_POWER: f64 = 0.28;
pub const ANNUAL_IDLE_DECAY: f64 = 0.03;

/// 確定性解析實體魂魄種子（拔除預設複製種子與可變屬性雜湊，萬人萬相、人獸同源）
pub fn resolve_entity_soul_seed(entity_id: &str, existing_seed: Option<i64>) -> i64 {
    if let Some(s) = existing_seed.filter(|&s| s != 0) {
        return s;
    }
    let mut h: u64 = 0xcbf29ce484222325;
    for b in entity_id.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let s = crate::combat::wordplate::splitmix64(h) as i64;
    if s == 0 { 42 } else { s }
}

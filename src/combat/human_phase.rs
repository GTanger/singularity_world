// human_phase 模組 — 個人之相與自然成相系統（從「露西」演化為「人」）
// 嚴守台灣正體中文，無任何簡體字與禁止字眼；零 unsafe、零 bare #[allow]。
// 涵蓋五大數理模組：
// 1. 先天稟賦雙層守恆（對數功率守恆 × 帶界單純形 10,000 阻抗矩陣 R_e）
// 2. 碎招運動學算子與有界非交換銜接度 C_e
// 3. 套路分支樹、連貫性與變化性、Viterbi 5 階段路徑湧現
// 4. 事件分派三相動力學判準（一力降十會、專克拆招、庖丁解牛）
// 5. 過完備語意字典 2-Step Residual OMP 殘差正交匹配定名

use std::sync::LazyLock;
use crate::combat::wordplate::{
    CarrierAffinity, PostureState, TacticalSlot, WordElement, WordRegistry,
};

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

impl InnateBonePhase {
    /// 依唯一 soul_seed 初始化確定性骨相
    pub fn from_seed(seed: i64) -> Self {
        let mut rng_state = seed as u64;

        // 輔助函式：自 splitmix64 取 [0.0, 1.0) 浮點數
        let next_f64 = |state: &mut u64| -> f64 {
            let val = crate::combat::wordplate::splitmix64(*state);
            *state = state.wrapping_add(0x9E3779B97F4A7C15);
            (val >> 11) as f64 / ((1u64 << 53) as f64)
        };

        let ln_2 = 2.0f64.ln();
        let u_alpha = next_f64(&mut rng_state);
        let ln_alpha = -ln_2 + u_alpha * (2.0 * ln_2);

        let u_eps = next_f64(&mut rng_state);
        let eps = -0.15 + u_eps * 0.30;

        let raw_ln_beta = -ln_alpha + eps;
        let ln_beta = raw_ln_beta.clamp(-ln_2, ln_2);

        let u_gamma = next_f64(&mut rng_state);
        let gamma = -1.0 + u_gamma * 2.0;

        let alpha = ln_alpha.exp();
        let beta = ln_beta.exp();

        // 抽樣 15 個 Gamma(18, 1) 變量
        let mut r_flat = [0.0f64; 15];
        let mut sum_raw = 0.0f64;
        for i in 0..15 {
            let mut g_val = 0.0f64;
            for _ in 0..18 {
                let u = next_f64(&mut rng_state).clamp(1e-12, 1.0);
                g_val += -u.ln();
            }
            r_flat[i] = g_val;
            sum_raw += g_val;
        }

        if sum_raw <= 0.0 {
            sum_raw = 1.0;
        }
        for i in 0..15 {
            r_flat[i] = (r_flat[i] / sum_raw) * 10000.0;
        }

        // 帶界單純形水位投影 (Bounded Simplex Water-Filling Projection) 12 次迭代
        for _ in 0..12 {
            for v in r_flat.iter_mut() {
                *v = v.clamp(300.0, 1200.0);
            }
            let cur_sum: f64 = r_flat.iter().sum();
            let deficit = 10000.0 - cur_sum;
            let mut free_sum = 0.0f64;
            let mut free_mask = [false; 15];
            for i in 0..15 {
                let val = r_flat[i];
                if (deficit > 0.0 && val < 1199.99) || (deficit < 0.0 && val > 300.01) {
                    free_mask[i] = true;
                    free_sum += 1.0;
                }
            }
            if free_sum > 0.0 {
                let step = deficit / free_sum;
                for i in 0..15 {
                    if free_mask[i] {
                        r_flat[i] += step;
                    }
                }
            } else {
                break;
            }
        }

        // 最後確切 clamp [300.0, 1200.0] 與精確再歸一化，防止浮點微小誤差飄移
        for v in r_flat.iter_mut() {
            *v = v.clamp(300.0, 1200.0);
        }
        let final_sum: f64 = r_flat.iter().sum();
        if final_sum > 0.0 && (final_sum - 10000.0).abs() > 1e-9 {
            let factor = 10000.0 / final_sum;
            for v in r_flat.iter_mut() {
                *v = (*v * factor).clamp(300.0, 1200.0);
            }
            let post_sum: f64 = r_flat.iter().sum();
            let diff = 10000.0 - post_sum;
            r_flat[0] = (r_flat[0] + diff).clamp(300.0, 1200.0);
        }

        let mut r_matrix = [[0.0f64; 5]; 3];
        for u in 0..3 {
            for v in 0..5 {
                r_matrix[u][v] = r_flat[u * 5 + v];
            }
        }

        // 外顯三維計算
        let sum_vit: f64 = r_matrix[0].iter().sum();
        let sum_qi: f64 = r_matrix[1].iter().sum();
        let sum_dex: f64 = r_matrix[2].iter().sum();

        let vit_0 = 50.0 + 0.02 * (6000.0 - sum_vit);
        let qi_0 = 50.0 + 0.02 * (6000.0 - sum_qi);
        let dex_0 = 50.0 + 0.02 * (6000.0 - sum_dex);

        Self {
            seed,
            alpha,
            beta,
            gamma,
            r_matrix,
            vit_0,
            qi_0,
            dex_0,
        }
    }

    /// 爆發力矩放大係數
    pub fn impact_factor(&self) -> f64 {
        self.alpha.sqrt()
    }

    /// 有界個人身體順暢因子 S_e(u, v) in (0.25, 1.75)
    pub fn cell_smoothness(&self, axis: usize, stage: usize) -> f64 {
        let r_val = self.r_matrix[axis][stage];
        0.25 + 1.50 / (1.0 + ((r_val - R_BAR) / TAU_R).exp())
    }

    /// 該發力階段阻抗最低（最順暢）之主幹屬性 (0:體, 1:氣, 2:敏)
    pub fn stage_best_axis(&self, stage: usize) -> usize {
        let mut best_axis = 0;
        let mut min_r = self.r_matrix[0][stage];
        for axis in 1..3 {
            if self.r_matrix[axis][stage] < min_r {
                min_r = self.r_matrix[axis][stage];
                best_axis = axis;
            }
        }
        best_axis
    }

    /// 階段發力流暢度評語
    pub fn stage_fluency_label(&self, stage: usize) -> &'static str {
        let best_s = self.cell_smoothness(self.stage_best_axis(stage), stage);
        if best_s >= 1.30 {
            "渾然天成"
        } else if best_s >= 1.00 {
            "順暢自如"
        } else if best_s >= 0.70 {
            "平穩守中"
        } else {
            "略需借勢"
        }
    }

    /// 天生體魄稱號（零數字、純正體白話）
    pub fn bone_archetype_title(&self) -> String {
        let prefix = if self.alpha >= 1.35 {
            "沉腰重岳"
        } else if self.alpha <= 0.75 {
            "輕靈矯捷"
        } else {
            "骨肉勻稱"
        };

        let suffix = if self.gamma >= 0.35 {
            "氣脈沉厚"
        } else if self.gamma <= -0.35 {
            "氣機靈動"
        } else {
            "氣息平和"
        };

        format!("{prefix}・{suffix}")
    }
    /// 完整白話骨相描述（零數字、零 N001/G01）
    pub fn describe_bone_phase_zh(&self) -> String {
        let power_desc = if self.alpha >= 1.25 {
            "天生筋骨粗壯，下盤沉實，落步發力帶有重岳崩山之勢；出招雖稍顯沉穩，但每一擊皆力透筋骨。"
        } else if self.alpha <= 0.80 {
            "天生筋絡纖長，動靜靈動輕捷，呼吸換氣與步履銜接極為迅疾；雖難以蠻力撼動重甲，但出招連綿如穿花引線。"
        } else {
            "氣血與筋骨協調勻稱，發力剛柔並濟，既能正面立足抗衡，亦能從容調息換勢。"
        };

        let temperament_desc = if self.gamma >= 0.25 {
            "其人氣息沉穩厚實，舉止遵循法度，最善於在長線拆解之中尋找對手的破綻與銜接空隙。"
        } else if self.gamma <= -0.25 {
            "其人性喜靈動求變，氣機周流不滯，遭遇強敵沉重壓制時極擅長側身偏轉、借勢卸力求生。"
        } else {
            "周身氣脈平穩中和，守勢與攻勢流轉隨心，不偏執於剛猛亦不滯於巧變。"
        };

        let stage_names = ["起勢", "承接", "轉折", "協調", "合擊"];
        let axis_names = ["筋骨發力", "內息吐納", "手腕步法"];
        let mut min_r = f64::MAX;
        let mut max_r = f64::MIN;
        let mut best_pair = (0usize, 0usize);
        let mut tight_pair = (0usize, 2usize);
        for u in 0..3 {
            for v in 0..5 {
                let r = self.r_matrix[u][v];
                if r < min_r {
                    min_r = r;
                    best_pair = (u, v);
                }
                if r > max_r {
                    max_r = r;
                    tight_pair = (u, v);
                }
            }
        }
        let chain_desc = format!(
            "交手發力之際，尤以{}時之{}最為順暢；若遇{}時{}稍滯，身形亦能自然順勢卸轉、借力換步。",
            stage_names[best_pair.1],
            axis_names[best_pair.0],
            stage_names[tight_pair.1],
            axis_names[tight_pair.0],
        );

        format!("{power_desc}{temperament_desc}{chain_desc}")
    }

    /// 五常發力順暢白話說明
    pub fn describe_five_stage_fluency_zh(&self) -> String {
        let labels = [
            self.stage_fluency_label(0),
            self.stage_fluency_label(1),
            self.stage_fluency_label(2),
            self.stage_fluency_label(3),
            self.stage_fluency_label(4),
        ];

        let axes_names = ["筋骨體魄", "氣血周流", "靈敏身形"];
        let best_0 = axes_names[self.stage_best_axis(0)];
        let best_2 = axes_names[self.stage_best_axis(2)];
        let best_4 = axes_names[self.stage_best_axis(4)];

        format!(
            "出招節奏自起手之{}、中盤承轉之{}，一路延續至收勢之{}；起手憑{}爭先，中盤仗{}周旋，收勢以{}定局。",
            labels[0], labels[2], labels[4], best_0, best_2, best_4
        )
    }

    /// 三相自發適配傾向總結（白話）
    pub fn auto_trigger_summary_zh(&self) -> String {
        let crush_prop = if self.alpha >= 1.15 {
            "正面交鋒時佔盡千鈞優勢，極易以渾厚衝量逼散敵方架勢；"
        } else {
            "正面硬撼需審慎借器物分量立足，不宜與沉猛之勢直面死磕；"
        };

        let intercept_prop = if self.gamma >= 0.15 {
            "對換招縫隙與對手破綻極度敏銳，擅長以截擊瓦解連招；"
        } else {
            "遭遇交鋒多以變招游移周旋，借勢轉向以尋良機；"
        };

        let threading_prop = if self.dex_0 >= 100.0 || self.beta >= 1.10 {
            "刃口落點細微精準，在百工解剖與實戰破綻切入時皆能直尋骨隙筋節。"
        } else {
            "出手偏重連貫平穩，講究力道透徹而非純粹求巧。"
        };

        format!("{crush_prop}{intercept_prop}{threading_prop}")
    }
}

// ==========================================
// 模組二：碎招運動學庫與物理計算
// ==========================================

/// 四維姿態距離平方
pub fn posture_distance_sq(p1: &[f64; 4], p2: &[f64; 4]) -> f64 {
    let dz = p1[0] - p2[0];
    let d_omega = p1[1] - p2[1];
    let dv = p1[2] - p2[2];
    let d_theta = p1[3] - p2[3];

    let cos_val = d_theta.cos();
    let theta_term = 2.0 * (1.0 - cos_val).clamp(0.0, 2.0);

    // 等量綱權重均為 1.0
    dz * dz + d_omega * d_omega + dv * dv + theta_term
}

/// 非交換運動學匹配核 K(m_i -> m_{i+1}) in (0, 1]
pub fn kinematic_kernel(m1: &ElementaryMotionDef, m2: &ElementaryMotionDef) -> f64 {
    // 強制起 -> 承 -> 轉 -> 協 -> 合 階段推進
    if m2.stage != m1.stage + 1 {
        return 0.0;
    }

    let self_penalty = if m1.name == m2.name { RHO_SELF } else { 1.0 };
    let d_sq = posture_distance_sq(&m1.p_out, &m2.p_in);
    let exp_term = (-d_sq / (2.0 * SIGMA_P_SQ)).exp();

    self_penalty * exp_term
}

/// 個人身體順暢因子 S_e(m)
pub fn motion_smoothness(bone: &InnateBonePhase, motion: &ElementaryMotionDef) -> f64 {
    let stage = motion.stage.min(4);
    let r_eff = motion.w_3d[0] * bone.r_matrix[0][stage]
        + motion.w_3d[1] * bone.r_matrix[1][stage]
        + motion.w_3d[2] * bone.r_matrix[2][stage];

    0.25 + 1.50 / (1.0 + ((r_eff - R_BAR) / TAU_R).exp())
}

/// 無上限練習冪律深度 m_e(f_eff) = (f_eff / F_0)^0.45
pub fn practice_depth(effective_f: f64) -> f64 {
    (effective_f.max(0.0) / F_0).powf(P_PRACTICE)
}

/// 年度有效時間預算分流：隨修習門數廣度單調遞減
pub fn effective_time_budget(total_time: f64, active_routines: usize, switch_cost: f64) -> f64 {
    let penalty = switch_cost.max(0.0) * (active_routines.saturating_sub(1) as f64);
    total_time.max(0.0) / (1.0 + penalty)
}

/// 閒置用進廢退衰減：每年按固定比例衰減練習量
pub fn apply_annual_idle_decay(f: f64) -> f64 {
    f.max(0.0) * (1.0 - ANNUAL_IDLE_DECAY)
}

/// 手札研讀領悟增量 Delta L = 1.0 - exp(-s * units / 3.0)
pub fn manual_comprehension_delta(routine_smoothness: f64, study_units: f64) -> f64 {
    1.0 - (-routine_smoothness.max(0.25) * study_units.max(0.0) / 3.0).exp()
}

/// 結合骨相順暢度之無上限熟練因子 H_e（順性放大練習深度，拔除人為封頂）
pub fn mastery_factor_with_smoothness(
    learned_delta: f64,
    familiarity_count: u32,
    smoothness_s: f64,
) -> f64 {
    let f_eff = smoothness_s.max(0.25) * (familiarity_count as f64);
    1.0 + ETA_L * learned_delta.clamp(0.0, 1.0) + ETA_F_POWER * practice_depth(f_eff)
}

/// 後天所學與熟練因子 H_e（預設順暢度 1.0）
pub fn mastery_factor(learned_delta: f64, familiarity_count: u32) -> f64 {
    mastery_factor_with_smoothness(learned_delta, familiarity_count, 1.0)
}

/// 綜合銜接流暢度 C_e(m_i -> m_{i+1})
pub fn transition_continuity(
    bone: &InnateBonePhase,
    m1: &ElementaryMotionDef,
    m2: &ElementaryMotionDef,
    learned_delta: f64,
    familiarity_count: u32,
) -> f64 {
    let k = kinematic_kernel(m1, m2);
    if k <= 0.0 {
        return 0.0;
    }
    let s = motion_smoothness(bone, m2);
    let h = mastery_factor_with_smoothness(learned_delta, familiarity_count, s);

    k * s * h
}
/// 換招空檔破綻窗 tau^gap in Ticks (>= 1)
pub fn transition_gap_ticks(
    bone: &InnateBonePhase,
    m1: &ElementaryMotionDef,
    m2: &ElementaryMotionDef,
    c_val: f64,
) -> u8 {
    let base = (m1.timing.2 as f64) + (m2.timing.0 as f64);
    let denom = 1.0 + 0.55 * bone.beta.sqrt() * c_val;
    let ticks = (base / denom).ceil() as u8;
    ticks.max(1)
}

// ==========================================
// 碎招知識庫建立
// ==========================================

static GLOBAL_MOTION_CATALOG: LazyLock<Vec<ElementaryMotionDef>> =
    LazyLock::new(build_elementary_motion_catalog);

fn build_elementary_motion_catalog() -> Vec<ElementaryMotionDef> {
    let mut catalog = Vec::with_capacity(180);

    let carriers = [
        CarrierAffinity::Unarmed,
        CarrierAffinity::Blade,
        CarrierAffinity::Pole,
        CarrierAffinity::Blunt,
        CarrierAffinity::Shield,
        CarrierAffinity::Footwork,
    ];

    // 為每種載體在 5 個階段各提供 3 個候選碎招（martial motions, craft_domain: None）
    for carrier in carriers {
        for stage in 0..5 {
            for variant in 0..3 {
                let name = match (carrier, stage, variant) {
                    // 徒手拳掌
                    (CarrierAffinity::Unarmed, 0, 0) => "沉腰蹲步",
                    (CarrierAffinity::Unarmed, 0, 1) => "虛步亮掌",
                    (CarrierAffinity::Unarmed, 0, 2) => "提膝側身",
                    (CarrierAffinity::Unarmed, 1, 0) => "蹬地躍起",
                    (CarrierAffinity::Unarmed, 1, 1) => "探馬前衝",
                    (CarrierAffinity::Unarmed, 1, 2) => "擰腰推肘",
                    (CarrierAffinity::Unarmed, 2, 0) => "貼身上勾拳",
                    (CarrierAffinity::Unarmed, 2, 1) => "順水推舟",
                    (CarrierAffinity::Unarmed, 2, 2) => "雲手引化",
                    (CarrierAffinity::Unarmed, 3, 0) => "旋身纏抱",
                    (CarrierAffinity::Unarmed, 3, 1) => "錯骨分筋",
                    (CarrierAffinity::Unarmed, 3, 2) => "封門閉氣",
                    (CarrierAffinity::Unarmed, 4, 0) => "雙撞掌擊面",
                    (CarrierAffinity::Unarmed, 4, 1) => "截脈透骨打",
                    (CarrierAffinity::Unarmed, 4, 2) => "崩山重貫頂",

                    // 刀劍鋒芒
                    (CarrierAffinity::Blade, 0, 0) => "踏步斜探",
                    (CarrierAffinity::Blade, 0, 1) => "流光抽刃",
                    (CarrierAffinity::Blade, 0, 2) => "沉腕藏鋒",
                    (CarrierAffinity::Blade, 1, 0) => "順紋上劈",
                    (CarrierAffinity::Blade, 1, 1) => "連環點刺",
                    (CarrierAffinity::Blade, 1, 2) => "貼骨挑筋",
                    (CarrierAffinity::Blade, 2, 0) => "旋刃卸骨",
                    (CarrierAffinity::Blade, 2, 1) => "游刃截擊",
                    (CarrierAffinity::Blade, 2, 2) => "順勢收鋒切入",
                    (CarrierAffinity::Blade, 3, 0) => "纏頭裹腦",
                    (CarrierAffinity::Blade, 3, 1) => "分水穿肋",
                    (CarrierAffinity::Blade, 3, 2) => "翻腕截脈",
                    (CarrierAffinity::Blade, 4, 0) => "斷浪直斬",
                    (CarrierAffinity::Blade, 4, 1) => "驚鴻一瞥刺",
                    (CarrierAffinity::Blade, 4, 2) => "反刃撩雲截",

                    // 長槍大戟
                    (CarrierAffinity::Pole, 0, 0) => "靈蛇探梢",
                    (CarrierAffinity::Pole, 0, 1) => "低樁埋桿",
                    (CarrierAffinity::Pole, 0, 2) => "滑步流端",
                    (CarrierAffinity::Pole, 1, 0) => "順桿遊撥",
                    (CarrierAffinity::Pole, 1, 1) => "如電連刺",
                    (CarrierAffinity::Pole, 1, 2) => "抖腕崩顫",
                    (CarrierAffinity::Pole, 2, 0) => "穿花點穴",
                    (CarrierAffinity::Pole, 2, 1) => "順勢挑浪",
                    (CarrierAffinity::Pole, 2, 2) => "壓桿入懷",
                    (CarrierAffinity::Pole, 3, 0) => "回馬封喉",
                    (CarrierAffinity::Pole, 3, 1) => "攔拿扎胸",
                    (CarrierAffinity::Pole, 3, 2) => "橫掃千軍",
                    (CarrierAffinity::Pole, 4, 0) => "崩槍裂石",
                    (CarrierAffinity::Pole, 4, 1) => "白虹貫日刺",
                    (CarrierAffinity::Pole, 4, 2) => "沉桿鎖喉打",

                    // 重錘鈍器
                    (CarrierAffinity::Blunt, 0, 0) => "沉腰踏步",
                    (CarrierAffinity::Blunt, 0, 1) => "提柄旋身",
                    (CarrierAffinity::Blunt, 0, 2) => "順風掄臂",
                    (CarrierAffinity::Blunt, 1, 0) => "橫掄壓頂",
                    (CarrierAffinity::Blunt, 1, 1) => "沉柄推撞",
                    (CarrierAffinity::Blunt, 1, 2) => "斜掄掃擊",
                    (CarrierAffinity::Blunt, 2, 0) => "借轉崩錘",
                    (CarrierAffinity::Blunt, 2, 1) => "流勢推砸",
                    (CarrierAffinity::Blunt, 2, 2) => "沉底撩撞",
                    (CarrierAffinity::Blunt, 3, 0) => "旋風蕩寇",
                    (CarrierAffinity::Blunt, 3, 1) => "背身掛錘",
                    (CarrierAffinity::Blunt, 3, 2) => "頓地引震",
                    (CarrierAffinity::Blunt, 4, 0) => "傾力重砸",
                    (CarrierAffinity::Blunt, 4, 1) => "震臂崩擊",
                    (CarrierAffinity::Blunt, 4, 2) => "連環翻砸打",

                    // 堅盾重禦
                    (CarrierAffinity::Shield, 0, 0) => "紮步架盾",
                    (CarrierAffinity::Shield, 0, 1) => "斜身合盾",
                    (CarrierAffinity::Shield, 0, 2) => "沉肘立壁",
                    (CarrierAffinity::Shield, 1, 0) => "頂盾前推",
                    (CarrierAffinity::Shield, 1, 1) => "旋盾卸力",
                    (CarrierAffinity::Shield, 1, 2) => "撞門破勢",
                    (CarrierAffinity::Shield, 2, 0) => "盾角削面",
                    (CarrierAffinity::Shield, 2, 1) => "扣盾纏臂",
                    (CarrierAffinity::Shield, 2, 2) => "側身夾擊",
                    (CarrierAffinity::Shield, 3, 0) => "護體反震",
                    (CarrierAffinity::Shield, 3, 1) => "鐵門閉鎖",
                    (CarrierAffinity::Shield, 3, 2) => "滑步引流",
                    (CarrierAffinity::Shield, 4, 0) => "撼山沉擊",
                    (CarrierAffinity::Shield, 4, 1) => "猛推震跌",
                    (CarrierAffinity::Shield, 4, 2) => "碎骨重撞",

                    // 步法身形
                    (CarrierAffinity::Footwork, 0, 0) => "點水起落",
                    (CarrierAffinity::Footwork, 0, 1) => "側行遊弋",
                    (CarrierAffinity::Footwork, 0, 2) => "含胸蓄勢",
                    (CarrierAffinity::Footwork, 1, 0) => "疾步繞身",
                    (CarrierAffinity::Footwork, 1, 1) => "如影隨形",
                    (CarrierAffinity::Footwork, 1, 2) => "踏雪無痕",
                    (CarrierAffinity::Footwork, 2, 0) => "回首折向",
                    (CarrierAffinity::Footwork, 2, 1) => "移形換位",
                    (CarrierAffinity::Footwork, 2, 2) => "穿林脫身",
                    (CarrierAffinity::Footwork, 3, 0) => "欺身突進",
                    (CarrierAffinity::Footwork, 3, 1) => "游離分影",
                    (CarrierAffinity::Footwork, 3, 2) => "轉角牽引",
                    (CarrierAffinity::Footwork, 4, 0) => "疾閃穿背",
                    (CarrierAffinity::Footwork, 4, 1) => "追魂奪魄步",
                    (CarrierAffinity::Footwork, 4, 2) => "踏空定勢",

                    _ => "碎招流形",
                };

                // 設定物理姿態特徵
                let z_base = match stage {
                    0 => -0.5,
                    1 => 0.0,
                    2 => 0.4,
                    3 => 0.1,
                    _ => 0.6,
                };
                let p_in = [
                    z_base + (variant as f64) * 0.1,
                    -0.2 + (variant as f64) * 0.2,
                    0.1 * (stage as f64),
                    (variant as f64) * 0.5,
                ];
                let p_out = [
                    z_base + 0.2 + (variant as f64) * 0.1,
                    0.2 + (variant as f64) * 0.2,
                    0.2 * (stage as f64),
                    (variant as f64) * 0.5 + 0.3,
                ];

                let w_3d = match variant {
                    0 => [0.60, 0.20, 0.20], // 偏體
                    1 => [0.20, 0.60, 0.20], // 偏氣
                    _ => [0.20, 0.20, 0.60], // 偏敏
                };

                let base_torque = match carrier {
                    CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
                    CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
                    _ => 0.85,
                };
                let torque = match variant {
                    0 => base_torque * 1.35,
                    1 => base_torque * 1.00,
                    _ => base_torque * 0.80,
                };

                let base_edge = match carrier {
                    CarrierAffinity::Blade => 1.8,
                    CarrierAffinity::Pole => 1.4,
                    CarrierAffinity::Unarmed => 0.95,
                    _ => 0.45,
                };
                let edge = match variant {
                    0 => base_edge * 0.85,
                    1 => base_edge * 1.05,
                    _ => base_edge * 1.30,
                };

                let timing = match variant {
                    0 => (3, 4, 3),
                    1 => (2, 3, 2),
                    _ => (1, 2, 1),
                };

                let counter_fit = match variant {
                    0 => [0.85, 0.25, 0.25], // 克剛猛硬架
                    1 => [0.25, 0.85, 0.25], // 克後撤閃避
                    _ => [0.25, 0.25, 0.85], // 克貼身搶攻
                };

                let mut feat = [0.05f64; 16];
                feat[stage] += 0.40;
                feat[variant + 5] += 0.35;

                catalog.push(ElementaryMotionDef {
                    name: name.to_string(),
                    carrier,
                    craft_domain: None,
                    stage,
                    p_in,
                    p_out,
                    w_3d,
                    torque_kappa: torque,
                    edge_focus_p: edge,
                    timing,
                    counter_fit,
                    feature_16d: feat,
                });
            }
        }
    }

    // 百工專用碎招（採獵 harvest、鍛鑄 forge、庖廚 culinary、行旅 wayfare）
    // 4 個領域 × 5 個階段 × 3 種變體（偏體、偏氣、偏敏）= 60 個百工碎招
    let craft_domains = [
        ("harvest", 0usize, CarrierAffinity::Blade),
        ("forge", 1usize, CarrierAffinity::Blunt),
        ("culinary", 2usize, CarrierAffinity::Blade),
        ("wayfare", 3usize, CarrierAffinity::Footwork),
    ];

    for &(domain, c_idx, default_carrier) in &craft_domains {
        for stage in 0..5 {
            for variant in 0..3 {
                let (name, p_in, p_out, w_3d, torque, edge, c_fit) = match (domain, stage, variant) {
                    // harvest（採獵）
                    ("harvest", 0, 0) => ("巡山識痕", [-0.3, 0.0, 0.2, 0.0], [-0.2, 0.1, 0.2, 0.2], [0.6, 0.2, 0.2], 1.1, 1.2, [0.7, 0.2, 0.3]),
                    ("harvest", 0, 1) => ("聆風辨獸", [-0.2, 0.1, 0.1, 0.1], [-0.1, 0.2, 0.2, 0.3], [0.2, 0.6, 0.2], 0.9, 1.3, [0.3, 0.7, 0.2]),
                    ("harvest", 0, 2) => ("撥草伺伏", [-0.3, -0.1, 0.3, 0.2], [-0.2, 0.0, 0.3, 0.3], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("harvest", 1, 0) => ("探跡尋隙", [-0.2, 0.1, 0.2, 0.2], [-0.1, 0.2, 0.3, 0.4], [0.5, 0.3, 0.2], 1.2, 1.3, [0.8, 0.2, 0.2]),
                    ("harvest", 1, 1) => ("踏雪追蹤", [-0.1, 0.2, 0.2, 0.3], [0.0, 0.3, 0.2, 0.5], [0.2, 0.5, 0.3], 0.9, 1.4, [0.2, 0.7, 0.3]),
                    ("harvest", 1, 2) => ("伺機掩近", [-0.2, 0.0, 0.3, 0.3], [-0.1, 0.1, 0.4, 0.5], [0.2, 0.2, 0.6], 0.9, 1.5, [0.3, 0.2, 0.8]),

                    ("harvest", 2, 0) => ("順紋剝解", [-0.1, 0.2, 0.3, 0.4], [0.1, 0.3, 0.1, 0.6], [0.5, 0.2, 0.3], 1.3, 1.5, [0.7, 0.3, 0.2]),
                    ("harvest", 2, 1) => ("尋筋走刃", [0.0, 0.3, 0.2, 0.5], [0.1, 0.4, 0.2, 0.7], [0.2, 0.5, 0.3], 1.0, 1.6, [0.2, 0.8, 0.2]),
                    ("harvest", 2, 2) => ("巧取逆鱗", [-0.1, 0.1, 0.4, 0.5], [0.0, 0.2, 0.3, 0.7], [0.2, 0.3, 0.5], 0.9, 1.7, [0.2, 0.3, 0.8]),

                    ("harvest", 3, 0) => ("剔骨離筋", [0.1, 0.3, 0.1, 0.6], [0.2, 0.2, 0.2, 0.8], [0.6, 0.2, 0.2], 1.4, 1.6, [0.8, 0.2, 0.2]),
                    ("harvest", 3, 1) => ("抽絲剝繭", [0.1, 0.4, 0.2, 0.7], [0.2, 0.3, 0.2, 0.9], [0.2, 0.6, 0.2], 0.9, 1.6, [0.3, 0.7, 0.2]),
                    ("harvest", 3, 2) => ("入理剖膜", [0.0, 0.2, 0.3, 0.7], [0.1, 0.3, 0.2, 0.8], [0.2, 0.2, 0.6], 0.9, 1.8, [0.2, 0.3, 0.8]),

                    ("harvest", 4, 0) => ("收皮納寶", [0.2, 0.2, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.2, 1.2, [0.7, 0.3, 0.2]),
                    ("harvest", 4, 1) => ("整束歸囊", [0.2, 0.3, 0.2, 0.9], [0.0, 0.0, 0.0, 0.0], [0.2, 0.5, 0.3], 0.9, 1.3, [0.2, 0.8, 0.2]),
                    ("harvest", 4, 2) => ("輕裝收穫", [0.1, 0.3, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.2, 0.8]),

                    // forge（鍛鑄）
                    ("forge", 0, 0) => ("開爐看火", [-0.4, 0.1, 0.0, 0.1], [-0.3, 0.2, 0.1, 0.3], [0.6, 0.2, 0.2], 1.3, 0.8, [0.8, 0.2, 0.2]),
                    ("forge", 0, 1) => ("推橐引風", [-0.3, 0.2, 0.1, 0.2], [-0.2, 0.3, 0.2, 0.3], [0.2, 0.6, 0.2], 1.1, 0.8, [0.3, 0.7, 0.2]),
                    ("forge", 0, 2) => ("觀色候溫", [-0.3, 0.1, 0.2, 0.1], [-0.2, 0.2, 0.2, 0.2], [0.2, 0.3, 0.5], 1.0, 0.9, [0.2, 0.3, 0.8]),

                    ("forge", 1, 0) => ("引鐵入砧", [-0.3, 0.2, 0.1, 0.3], [-0.2, 0.3, 0.2, 0.5], [0.6, 0.2, 0.2], 1.4, 0.9, [0.8, 0.2, 0.2]),
                    ("forge", 1, 1) => ("平鉗定形", [-0.2, 0.3, 0.2, 0.4], [-0.1, 0.3, 0.2, 0.5], [0.2, 0.6, 0.2], 1.2, 0.9, [0.2, 0.7, 0.3]),
                    ("forge", 1, 2) => ("輕點引路", [-0.2, 0.2, 0.3, 0.4], [-0.1, 0.2, 0.3, 0.5], [0.2, 0.3, 0.5], 1.0, 1.1, [0.2, 0.3, 0.8]),

                    ("forge", 2, 0) => ("翻轉受力", [-0.2, 0.3, 0.2, 0.5], [0.0, 0.4, 0.2, 0.7], [0.5, 0.3, 0.2], 1.5, 1.0, [0.8, 0.2, 0.2]),
                    ("forge", 2, 1) => ("旋砧借勢", [-0.1, 0.4, 0.2, 0.6], [0.0, 0.4, 0.3, 0.7], [0.2, 0.5, 0.3], 1.2, 1.0, [0.2, 0.8, 0.2]),
                    ("forge", 2, 2) => ("順紋輕敲", [-0.1, 0.3, 0.3, 0.5], [0.0, 0.3, 0.3, 0.7], [0.2, 0.3, 0.5], 1.1, 1.2, [0.3, 0.2, 0.8]),

                    ("forge", 3, 0) => ("重錘排渣", [0.0, 0.4, 0.2, 0.7], [0.2, 0.2, 0.3, 0.9], [0.7, 0.2, 0.1], 1.6, 1.0, [0.9, 0.2, 0.1]),
                    ("forge", 3, 1) => ("連環疊打", [0.0, 0.4, 0.3, 0.7], [0.1, 0.3, 0.3, 0.8], [0.2, 0.6, 0.2], 1.3, 1.1, [0.3, 0.7, 0.2]),
                    ("forge", 3, 2) => ("密錘封孔", [0.0, 0.3, 0.3, 0.8], [0.1, 0.2, 0.2, 0.9], [0.2, 0.2, 0.6], 1.1, 1.3, [0.2, 0.3, 0.8]),

                    ("forge", 4, 0) => ("淬水成鋒", [0.2, 0.2, 0.3, 0.9], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.3, 1.4, [0.7, 0.3, 0.2]),
                    ("forge", 4, 1) => ("浴火定性", [0.1, 0.3, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 1.1, 1.3, [0.2, 0.8, 0.2]),
                    ("forge", 4, 2) => ("冷鍛凝刃", [0.1, 0.2, 0.2, 0.9], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 1.0, 1.5, [0.2, 0.3, 0.8]),

                    // culinary（庖廚）
                    ("culinary", 0, 0) => ("辨氣識味", [-0.2, 0.0, 0.1, 0.1], [-0.1, 0.1, 0.1, 0.2], [0.5, 0.3, 0.2], 0.9, 1.2, [0.7, 0.2, 0.3]),
                    ("culinary", 0, 1) => ("調和五味", [-0.2, 0.1, 0.1, 0.1], [-0.1, 0.2, 0.1, 0.3], [0.2, 0.6, 0.2], 0.8, 1.2, [0.3, 0.7, 0.2]),
                    ("culinary", 0, 2) => ("按刀量材", [-0.2, 0.0, 0.2, 0.2], [-0.1, 0.1, 0.2, 0.3], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("culinary", 1, 0) => ("順刀片肉", [-0.1, 0.1, 0.1, 0.2], [0.0, 0.2, 0.2, 0.4], [0.5, 0.2, 0.3], 1.0, 1.5, [0.8, 0.2, 0.2]),
                    ("culinary", 1, 1) => ("走刃分肌", [-0.1, 0.2, 0.1, 0.3], [0.0, 0.3, 0.2, 0.4], [0.2, 0.5, 0.3], 0.9, 1.6, [0.2, 0.8, 0.2]),
                    ("culinary", 1, 2) => ("巧斬軟骨", [-0.1, 0.1, 0.2, 0.3], [0.0, 0.2, 0.3, 0.5], [0.2, 0.2, 0.6], 0.9, 1.7, [0.2, 0.3, 0.8]),

                    ("culinary", 2, 0) => ("候火調鼎", [0.0, 0.2, 0.2, 0.4], [0.1, 0.3, 0.1, 0.6], [0.5, 0.3, 0.2], 1.1, 1.2, [0.8, 0.2, 0.2]),
                    ("culinary", 2, 1) => ("掀蓋順氣", [0.0, 0.3, 0.2, 0.4], [0.1, 0.4, 0.2, 0.6], [0.2, 0.6, 0.2], 0.8, 1.2, [0.2, 0.8, 0.2]),
                    ("culinary", 2, 2) => ("旋杓勻熱", [0.0, 0.2, 0.3, 0.5], [0.1, 0.3, 0.2, 0.6], [0.2, 0.2, 0.6], 0.8, 1.3, [0.3, 0.2, 0.8]),

                    ("culinary", 3, 0) => ("起杓收汁", [0.1, 0.3, 0.1, 0.6], [0.2, 0.2, 0.2, 0.8], [0.6, 0.2, 0.2], 1.2, 1.2, [0.8, 0.2, 0.2]),
                    ("culinary", 3, 1) => ("文火漫煨", [0.1, 0.4, 0.2, 0.6], [0.2, 0.3, 0.2, 0.8], [0.2, 0.6, 0.2], 0.8, 1.2, [0.2, 0.8, 0.2]),
                    ("culinary", 3, 2) => ("引香點綴", [0.1, 0.3, 0.2, 0.7], [0.2, 0.2, 0.3, 0.8], [0.2, 0.2, 0.6], 0.8, 1.4, [0.2, 0.3, 0.8]),

                    ("culinary", 4, 0) => ("合味定鼎", [0.2, 0.2, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.5, 0.3, 0.2], 1.0, 1.3, [0.7, 0.3, 0.2]),
                    ("culinary", 4, 1) => ("盛盤封香", [0.2, 0.3, 0.2, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 0.8, 1.3, [0.2, 0.8, 0.2]),
                    ("culinary", 4, 2) => ("剔骨呈鮮", [0.2, 0.2, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.8, 1.5, [0.2, 0.2, 0.8]),

                    // wayfare（行旅）
                    ("wayfare", 0, 0) => ("觀天辨向", [-0.2, 0.0, 0.3, 0.0], [-0.1, 0.1, 0.4, 0.2], [0.5, 0.3, 0.2], 0.9, 0.8, [0.7, 0.3, 0.2]),
                    ("wayfare", 0, 1) => ("束裝提氣", [-0.2, 0.1, 0.2, 0.1], [-0.1, 0.2, 0.3, 0.2], [0.2, 0.6, 0.2], 0.8, 0.8, [0.3, 0.7, 0.2]),
                    ("wayfare", 0, 2) => ("探風定步", [-0.2, 0.0, 0.3, 0.1], [-0.1, 0.1, 0.4, 0.3], [0.2, 0.2, 0.6], 0.7, 0.9, [0.2, 0.3, 0.8]),

                    ("wayfare", 1, 0) => ("踏草尋徑", [-0.1, 0.1, 0.4, 0.2], [0.0, 0.2, 0.3, 0.4], [0.5, 0.2, 0.3], 1.0, 0.8, [0.8, 0.2, 0.2]),
                    ("wayfare", 1, 1) => ("涉水探石", [-0.1, 0.2, 0.3, 0.2], [0.0, 0.3, 0.3, 0.4], [0.2, 0.5, 0.3], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 1, 2) => ("循嶺攀行", [-0.1, 0.1, 0.4, 0.3], [0.0, 0.2, 0.4, 0.5], [0.2, 0.2, 0.6], 0.8, 0.9, [0.2, 0.3, 0.8]),

                    ("wayfare", 2, 0) => ("側身避瘴", [0.0, 0.2, 0.3, 0.4], [0.1, 0.3, 0.2, 0.6], [0.5, 0.3, 0.2], 1.0, 0.9, [0.7, 0.3, 0.2]),
                    ("wayfare", 2, 1) => ("借藤越澗", [0.0, 0.3, 0.3, 0.4], [0.1, 0.4, 0.3, 0.6], [0.2, 0.5, 0.3], 0.9, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 2, 2) => ("旋步卸滑", [0.0, 0.2, 0.4, 0.5], [0.1, 0.3, 0.3, 0.7], [0.2, 0.2, 0.6], 0.8, 1.0, [0.2, 0.2, 0.8]),

                    ("wayfare", 3, 0) => ("勻息長驅", [0.1, 0.3, 0.2, 0.6], [0.2, 0.2, 0.3, 0.8], [0.6, 0.2, 0.2], 1.1, 0.8, [0.8, 0.2, 0.2]),
                    ("wayfare", 3, 1) => ("順風借勢", [0.1, 0.4, 0.3, 0.6], [0.2, 0.3, 0.3, 0.8], [0.2, 0.6, 0.2], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 3, 2) => ("換步輕行", [0.1, 0.3, 0.3, 0.7], [0.2, 0.2, 0.4, 0.8], [0.2, 0.2, 0.6], 0.7, 1.0, [0.2, 0.3, 0.8]),

                    ("wayfare", 4, 0) => ("安營定址", [0.2, 0.2, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.6, 0.2, 0.2], 1.1, 0.8, [0.7, 0.3, 0.2]),
                    ("wayfare", 4, 1) => ("歸途斂息", [0.2, 0.3, 0.3, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.6, 0.2], 0.8, 0.9, [0.2, 0.8, 0.2]),
                    ("wayfare", 4, 2) => ("登高極目", [0.2, 0.2, 0.4, 0.8], [0.0, 0.0, 0.0, 0.0], [0.2, 0.2, 0.6], 0.7, 1.0, [0.2, 0.2, 0.8]),

                    _ => ("行旅步法", [0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], [0.33, 0.33, 0.34], 1.0, 1.0, [0.33, 0.33, 0.34]),
                };

                let mut feat = [0.03f64; 16];
                feat[12 + c_idx] = 0.80;
                feat[stage] += 0.30;
                feat[variant + 5] += 0.20;

                catalog.push(ElementaryMotionDef {
                    name: name.to_string(),
                    carrier: default_carrier,
                    craft_domain: Some(domain),
                    stage,
                    p_in,
                    p_out,
                    w_3d,
                    torque_kappa: torque,
                    edge_focus_p: edge,
                    timing: (2, 3, 2),
                    counter_fit: c_fit,
                    feature_16d: feat,
                });
            }
        }
    }

    catalog
}

// ==========================================
// 模組三：Viterbi 5 階段最優路徑湧現
// ==========================================

fn find_viterbi_5stages_for_slot(
    bone: &InnateBonePhase,
    slot_opt: Option<TacticalSlot>,
    candidates_by_stage: &[Vec<&ElementaryMotionDef>; 5],
    learned_delta: f64,
    familiarity_count: u32,
) -> (Vec<ElementaryMotionDef>, Vec<usize>, f64) {
    // 動態分配 dp 表，完全支援任意候選數量，杜絕固定陣列越界
    let mut dp = Vec::with_capacity(5);
    for s in 0..5 {
        dp.push(vec![(0.0f64, 0usize); candidates_by_stage[s].len()]);
    }

    // 階段權重與戰術槽偏好加成評估
    let slot_bonus = |slot: TacticalSlot, m: &ElementaryMotionDef| -> f64 {
        match slot {
            TacticalSlot::Opening => {
                let speed_bonus = (3.0 - (m.timing.0 as f64).min(3.0)) * 0.35;
                let agi_bonus = m.w_3d[2] * 0.75;
                let stage_fit = if m.stage <= 1 { m.w_3d[2] * 0.40 } else { m.w_3d[1] * 0.25 };
                1.0 + speed_bonus + agi_bonus + stage_fit
            }
            TacticalSlot::Midgame => {
                let qi_bonus = m.w_3d[1] * 0.95;
                let flow_bonus = if m.timing.0 == 2 { 0.35 } else { 0.0 };
                1.0 + qi_bonus + flow_bonus
            }
            TacticalSlot::Finisher => {
                let torque_b = (m.torque_kappa - 0.8).max(0.0) * 0.85;
                let vit_b = m.w_3d[0] * 1.45;
                let stage_fit = if m.stage >= 2 { m.w_3d[0] * 0.65 } else { 0.0 };
                1.0 + torque_b + vit_b + stage_fit
            }
            TacticalSlot::Defense => {
                let rec_b = (3.0 - (m.timing.2 as f64).min(3.0)) * 0.25;
                let foot_bonus = if m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield {
                    0.65
                } else {
                    0.0
                };
                1.0 + rec_b + foot_bonus + m.w_3d[2] * 0.25
            }
        }
    };

    // Stage 0 初始化
    for (i, m) in candidates_by_stage[0].iter().enumerate() {
        let s0 = motion_smoothness(bone, m);
        let mult = slot_opt.map(|sl| slot_bonus(sl, m)).unwrap_or(1.0);
        dp[0][i] = (s0 * mult, 0);
    }

    // 階段 1..5 推進
    for s in 1..5 {
        for (cur_idx, m_cur) in candidates_by_stage[s].iter().enumerate() {
            let mult = slot_opt.map(|sl| slot_bonus(sl, m_cur)).unwrap_or(1.0);
            let mut best_val = -1.0f64;
            let mut best_prev = 0;
            for (prev_idx, m_prev) in candidates_by_stage[s - 1].iter().enumerate() {
                let prev_score = dp[s - 1][prev_idx].0;
                let c_val = transition_continuity(bone, m_prev, m_cur, learned_delta, familiarity_count);
                let score = prev_score * c_val * mult;
                if score > best_val {
                    best_val = score;
                    best_prev = prev_idx;
                }
            }
            dp[s][cur_idx] = (best_val, best_prev);
        }
    }

    // 回溯最優路徑
    let mut best_end_val = -1.0f64;
    let mut best_end_idx = 0;
    for (i, _) in candidates_by_stage[4].iter().enumerate() {
        if dp[4][i].0 > best_end_val {
            best_end_val = dp[4][i].0;
            best_end_idx = i;
        }
    }

    let mut path_indices = vec![0; 5];
    path_indices[4] = best_end_idx;
    for s in (1..5).rev() {
        path_indices[s - 1] = dp[s][path_indices[s]].1;
    }

    let mut result_motions = Vec::with_capacity(5);
    let mut compensated_stages = Vec::new();

    for s in 0..5 {
        let picked = candidates_by_stage[s][path_indices[s]].clone();
        let best_bone_axis = bone.stage_best_axis(s);
        let motion_axis = if picked.w_3d[0] >= picked.w_3d[1] && picked.w_3d[0] >= picked.w_3d[2] {
            0
        } else if picked.w_3d[1] >= picked.w_3d[2] {
            1
        } else {
            2
        };

        if motion_axis != 0 && bone.r_matrix[0][s] > 750.0 && motion_axis == best_bone_axis {
            compensated_stages.push(s);
        }

        result_motions.push(picked);
    }

    // 四段轉移幾何平均：沿回溯選出的 5 階段路徑重新累乘四步實際轉移分數（含跨軸代償 0.88）
    let mut geom_product = 1.0f64;
    for s in 0..4 {
        let m_prev = &result_motions[s];
        let m_next = &result_motions[s + 1];
        let mut step_c = transition_continuity(bone, m_prev, m_next, learned_delta, familiarity_count);
        if compensated_stages.contains(&(s + 1)) {
            step_c *= 0.88;
        }
        geom_product *= step_c.max(1e-6);
    }
    let continuity_c = geom_product.powf(0.25).max(0.1);

    (result_motions, compensated_stages, continuity_c)
}

fn find_viterbi_5stages(
    bone: &InnateBonePhase,
    candidates_by_stage: &[Vec<&ElementaryMotionDef>; 5],
    learned_delta: f64,
    familiarity_count: u32,
) -> (Vec<ElementaryMotionDef>, Vec<usize>, f64) {
    find_viterbi_5stages_for_slot(bone, None, candidates_by_stage, learned_delta, familiarity_count)
}

/// 輕量無字串配置的輔助函式：直接執行 5 階段 Viterbi 算出真實幾何平均連貫度與轉位變化性
pub fn compute_viterbi_continuity_and_variation(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    learned_delta: f64,
    familiarity_count: u32,
) -> (f64, f64) {
    let catalog = &*GLOBAL_MOTION_CATALOG;
    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] =
        [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    for m in catalog {
        if m.craft_domain.is_none() && m.carrier == carrier && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    if slot == TacticalSlot::Defense && carrier != CarrierAffinity::Footwork {
        for m in catalog {
            if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Footwork && m.stage < 5 {
                by_stage[m.stage].push(m);
            }
        }
    }

    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in catalog {
                if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Unarmed && m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let (motions, _compensated, continuity_c) = find_viterbi_5stages_for_slot(
        bone,
        Some(slot),
        &by_stage,
        learned_delta,
        familiarity_count,
    );

    let main_turn_name = &motions[2].name;
    let preferred_target_idx = match slot {
        TacticalSlot::Opening => 2,
        TacticalSlot::Midgame => 0,
        TacticalSlot::Finisher => 1,
        TacticalSlot::Defense => 0,
    };

    let mut best_branch_fit = -1.0f64;
    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            let target_fit = m.counter_fit[preferred_target_idx];
            if target_fit > best_branch_fit {
                best_branch_fit = target_fit;
            }
        }
    }

    if best_branch_fit < 0.0 {
        for m in &by_stage[2] {
            if m.name != *main_turn_name {
                let fit_sum = m.counter_fit[0].max(m.counter_fit[1]).max(m.counter_fit[2]);
                if fit_sum > best_branch_fit {
                    best_branch_fit = fit_sum;
                }
            }
        }
    }

    let variation_v = (best_branch_fit.max(0.4) * (1.0 + 0.25 * (-bone.gamma).max(0.0))).clamp(0.40, 1.80);
    (continuity_c, variation_v)
}

/// 依個人骨相、載體與器物推導個人臨陣武相
pub fn derive_personal_martial_routine(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    vessel_name: &str,
    learned_bonus: f64,
    familiarity_count: u32,
    registry: &WordRegistry,
) -> EmergentRoutine {
    let catalog = &*GLOBAL_MOTION_CATALOG;

    // 依載體挑選各階段候選碎招（過濾掉百工碎招）
    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for m in catalog {
        if m.craft_domain.is_none() && m.carrier == carrier && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    // 若為 Defense 守勢，額外納入 Footwork 步法碎招以強化移形換位防禦
    if slot == TacticalSlot::Defense && carrier != CarrierAffinity::Footwork {
        for m in catalog {
            if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Footwork && m.stage < 5 {
                by_stage[m.stage].push(m);
            }
        }
    }

    // 若某階段不足，以徒手補充（確保排除百工）
    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in catalog {
                if m.craft_domain.is_none() && m.carrier == CarrierAffinity::Unarmed && m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let (motions, compensated, c_val) = find_viterbi_5stages_for_slot(
        bone,
        Some(slot),
        &by_stage,
        learned_bonus,
        familiarity_count,
    );

    let main_motions = [
        motions[0].name.clone(),
        motions[1].name.clone(),
        motions[2].name.clone(),
        motions[3].name.clone(),
        motions[4].name.clone(),
    ];

    // 尋找「轉」位應變分支碎招（根據 slot 戰術意圖鎖定最佳 counter_target）
    let main_turn_name = &main_motions[2];
    let mut best_branch_name = "順勢側滑步".to_string();
    let mut best_branch_fit = -1.0f64;

    // 根據 slot 設定期待克制目標
    let preferred_target_idx = match slot {
        TacticalSlot::Opening => 2,   // 貼身搶攻 (克搶先手)
        TacticalSlot::Midgame => 0,   // 剛猛硬架 (破其僵持)
        TacticalSlot::Finisher => 1,  // 後撤閃避 (追襲閃避)
        TacticalSlot::Defense => 0,   // 剛猛硬架 / 借力化勁
    };

    let mut counter_target_zh = match preferred_target_idx {
        0 => "剛猛硬架",
        1 => "後撤閃避",
        _ => "貼身搶攻",
    };

    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            let carrier_pref = if slot == TacticalSlot::Defense && (m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield) {
                0.35
            } else {
                0.0
            };
            let target_fit = m.counter_fit[preferred_target_idx] + carrier_pref;
            if target_fit > best_branch_fit {
                best_branch_fit = target_fit;
                best_branch_name = m.name.clone();
                counter_target_zh = match preferred_target_idx {
                    0 => "剛猛硬架",
                    1 => "後撤閃避",
                    _ => "貼身搶攻",
                };
            }
        }
    }

    // 兜底：若特定目標未找到合適分支，取任意最高 fit
    if best_branch_fit < 0.0 {
        for m in &by_stage[2] {
            if m.name != *main_turn_name {
                let carrier_pref = if slot == TacticalSlot::Defense && (m.carrier == CarrierAffinity::Footwork || m.carrier == CarrierAffinity::Shield) {
                    0.35
                } else {
                    0.0
                };
                let fit_sum = m.counter_fit[0].max(m.counter_fit[1]).max(m.counter_fit[2]) + carrier_pref;
                if fit_sum > best_branch_fit {
                    best_branch_fit = fit_sum;
                    best_branch_name = m.name.clone();
                    if m.counter_fit[1] >= m.counter_fit[0] && m.counter_fit[1] >= m.counter_fit[2] {
                        counter_target_zh = "後撤閃避";
                    } else if m.counter_fit[2] >= m.counter_fit[0] && m.counter_fit[2] >= m.counter_fit[1] {
                        counter_target_zh = "貼身搶攻";
                    } else {
                        counter_target_zh = "剛猛硬架";
                    }
                }
            }
        }
    }

    // 變化性指標 V_e
    let variation_v = (best_branch_fit * (1.0 + 0.25 * (-bone.gamma).max(0.0))).clamp(0.40, 1.80);
    // 換招最大空檔 ticks
    let mut max_gap = 1u8;
    for i in 0..4 {
        let gap = transition_gap_ticks(bone, &motions[i], &motions[i + 1], c_val);
        if gap > max_gap {
            max_gap = gap;
        }
    }

    // 套路特徵向量 16D：隨 slot、bone 與碎招深度調變
    let mut feat_16d = [0.0f64; 16];
    for (i, m) in motions.iter().enumerate() {
        let weight = (i + 1) as f64 / 15.0;
        for k in 0..16 {
            feat_16d[k] += m.feature_16d[k] * weight;
        }
    }
    feat_16d[0] *= bone.impact_factor();
    feat_16d[1] *= bone.beta;

    // 根據戰術位 slot 與骨相三相 (alpha, beta, gamma) 加註鮮明特徵權重
    match slot {
        TacticalSlot::Opening => {
            feat_16d[0] += 0.35 * bone.beta; // 疾發速戰
            feat_16d[2] += 0.45;             // 敏捷出探
            feat_16d[5] += 0.30;             // 先聲奪人
        }
        TacticalSlot::Midgame => {
            feat_16d[1] += 0.40;             // 氣流連綿
            feat_16d[3] += 0.35;             // 順滑運轉
            feat_16d[6] += 0.30;             // 連環應變
        }
        TacticalSlot::Finisher => {
            feat_16d[0] += 0.60 * bone.alpha;// 巔峰重勢破堅
            feat_16d[4] += 0.50;             // 定鼎擊穿
            feat_16d[7] += 0.35;             // 終結一式
        }
        TacticalSlot::Defense => {
            feat_16d[8] += 0.50;             // 固若金湯
            feat_16d[9] += 0.40 * (1.0 + bone.gamma.max(0.0)); // 守正不移
            feat_16d[11] += 0.30;            // 避虛就實
        }
    }

    // 2-Step Residual OMP 選字定名
    let ((w1_id, w2_id), (w1_ch, w2_ch)) = residual_omp_select_roots(&feat_16d, registry);

    let (slot_code, slot_label_zh) = match slot {
        TacticalSlot::Opening => ("opening", "起手先機"),
        TacticalSlot::Midgame => ("midgame", "中盤連攜"),
        TacticalSlot::Finisher => ("finisher", "破敵收勢"),
        TacticalSlot::Defense => ("defense", "守勢應變"),
    };

    let phase_specialty_zh = match slot {
        TacticalSlot::Finisher => {
            if bone.dex_0 >= 110.0 {
                "尋隙游刃"
            } else {
                "重勢硬撼"
            }
        }
        TacticalSlot::Opening | TacticalSlot::Defense => {
            if bone.gamma >= -0.10 {
                "借勢拆招"
            } else if bone.alpha >= 1.25 {
                "重勢硬撼"
            } else {
                "尋隙游刃"
            }
        }
        TacticalSlot::Midgame => {
            if bone.alpha >= 1.20 {
                "重勢硬撼"
            } else if bone.gamma >= 0.10 {
                "借勢拆招"
            } else {
                "尋隙游刃"
            }
        }
    };

    let first2 = |s: &str| -> String { s.chars().take(2).collect() };
    let last2 = |s: &str| -> String {
        let chs: Vec<char> = s.chars().collect();
        let len = chs.len();
        if len >= 2 {
            chs[len - 2..].iter().collect()
        } else {
            s.to_string()
        }
    };
    let last3_or_2 = |s: &str| -> String {
        let chs: Vec<char> = s.chars().collect();
        let len = chs.len();
        if len >= 5 {
            chs[len - 3..].iter().collect()
        } else if len >= 2 {
            format!("{}式", chs[len - 2..].iter().collect::<String>())
        } else {
            format!("{s}式")
        }
    };

    let formal_name = match slot {
        TacticalSlot::Opening => format!("{}{}式", first2(&main_motions[0]), last2(&main_motions[1])),
        TacticalSlot::Midgame => format!("{}{}式", first2(&main_motions[2]), last2(&main_motions[3])),
        TacticalSlot::Finisher => format!("{}{}", first2(&main_motions[3]), last3_or_2(&main_motions[4])),
        TacticalSlot::Defense => {
            let suffix = if carrier == CarrierAffinity::Shield { "勢" } else { "步" };
            let tail = last2(&main_motions[4]).trim_end_matches('步').trim_end_matches('勢').to_string();
            if tail.chars().count() == 1 {
                format!("{}{}{}", first2(&main_motions[1]), last2(&main_motions[2]), suffix)
            } else {
                format!("{}{}{}", first2(&main_motions[1]), tail, suffix)
            }
        }
    };

    let trimmed_vessel = vessel_name.trim();
    let is_unarmed = trimmed_vessel.is_empty() || trimmed_vessel == "徒手" || carrier == CarrierAffinity::Unarmed;
    let vessel_clean = if is_unarmed {
        "雙拳肉掌"
    } else {
        trimmed_vessel
    };

    let origin_basis_zh = match slot {
        TacticalSlot::Opening => {
            let comp = if compensated.is_empty() {
                if is_unarmed {
                    "提氣墊步間身拳合一，搶在對手出手之前壓制其勢頭。"
                } else {
                    "提氣墊步間人器合一，搶在對手出手之前壓制其勢頭。"
                }
            } else {
                "起手時順著腕力與步法微調重心，避開生硬對撞，搶在對手出手之前壓制其勢頭。"
            };
            if is_unarmed {
                format!(
                    "臨敵之際雙拳護身疾步搶進，以敏銳步法試探敵隙。{}",
                    comp
                )
            } else {
                format!(
                    "臨敵之際手持{}疾步搶進，以敏銳步法試探敵隙。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Midgame => {
            let comp = if compensated.is_empty() {
                "手腕吞吐與周身勁力連貫一氣，招招環扣不露端倪。"
            } else {
                "換氣過渡間順著手腕慣性流轉卸力，令攻勢綿延不絕。"
            };
            if is_unarmed {
                format!(
                    "近身交接後，雙掌、呼吸與步法隨身形開展綿密周旋。{}",
                    comp
                )
            } else {
                format!(
                    "兵刃交接後，手腕、呼吸與步法隨{}開展綿密周旋。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Finisher => {
            let tip_word = if is_unarmed {
                "拳鋒"
            } else if carrier == CarrierAffinity::Blunt || carrier == CarrierAffinity::Shield {
                "器身"
            } else {
                "鋒刃"
            };
            let comp = if compensated.is_empty() {
                format!("整勁順勢壓入{}，一擊定奪勝負。", tip_word)
            } else {
                "借腰胯與肩背重心自然傾注，將全身力道凝聚於末梢一擊定局。".to_string()
            };
            if is_unarmed {
                format!(
                    "窺準對手氣竭露怯之際，聚全身蓄勁凝於{}，直擊門戶破綻。{}",
                    vessel_clean,
                    comp
                )
            } else {
                format!(
                    "窺準對手氣竭露怯之際，聚全身蓄勁貫入{}之中，直劈門戶破綻。{}",
                    vessel_clean,
                    comp
                )
            }
        }
        TacticalSlot::Defense => {
            let comp = if compensated.is_empty() {
                "下盤重心沉穩，步法隨機牽引，化去來襲鋒芒。"
            } else {
                "順著腰胯與足尖自然卸力偏轉，避開正鋒死磕。"
            };
            if carrier == CarrierAffinity::Footwork || is_unarmed {
                format!(
                    "遇險時足下起落移形換位，於方寸間騰挪閃轉、避開敵勢銳鋒。{}",
                    comp
                )
            } else {
                format!(
                    "受壓時足下步法與重心隨機換位，斜閃引化來勢並持{}從容拉開戰距。{}",
                    vessel_clean,
                    comp
                )
            }
        }
    };

    let learned_and_familiar_zh = match slot {
        TacticalSlot::Opening => format!(
            "起手先機循［{} → {} → {} → {} → {}］疾發爭先；中途若遇對手{}，可順勢變招改走［{}］搶佔上風。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Midgame => format!(
            "連攜攻防循［{} → {} → {} → {} → {}］流轉不息；交鋒中途若遇對手{}，可順勢變招改走［{}］拆解來勢。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Finisher => format!(
            "破勢定局循［{} → {} → {} → {} → {}］合力驟落；臨陣若遇對手{}，可中途煞勁改走［{}］追襲破敵。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
        TacticalSlot::Defense => format!(
            "守勢引卸循［{} → {} → {} → {} → {}］沉穩周旋；避讓之際若遇對手{}，可借勢轉身改走［{}］化解殺招。",
            main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
            counter_target_zh, best_branch_name
        ),
    };

    let fluency_word = if c_val >= 1.25 {
        "渾然天成，招意綿密無隙"
    } else if c_val >= 0.95 {
        "順暢自如，換氣轉勢間隙極短"
    } else {
        "平穩紮實，講究沉著應對"
    };

    let mastery_tier = if familiarity_count >= 30 || learned_bonus >= 0.8 {
        "信手拈來"
    } else if familiarity_count >= 10 || learned_bonus >= 0.3 {
        "熟能生巧"
    } else {
        "初具法度"
    };

    let auto_trigger_desc_zh = match slot {
        TacticalSlot::Opening => format!(
            "起手攻勢已達「{}」，{}；長於{}，能在接戰第一刻捕捉空隙疾速發動。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Midgame => format!(
            "中盤連環已臻「{}」，{}；長於{}，能隨器物手感與呼吸節奏綿密銜接。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Finisher => format!(
            "破陣殺勢已入「{}」，{}；長於{}，能乘敵手露怯或門戶大開時傾力定局。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
        TacticalSlot::Defense => format!(
            "避實禦敵已具「{}」，{}；長於{}，能在受壓之際借步法身形從容重整架勢。",
            mastery_tier, fluency_word, phase_specialty_zh
        ),
    };

    EmergentRoutine {
        domain: "martial",
        slot_code,
        slot_label_zh,
        carrier,
        main_motions,
        turn_branch_motion: best_branch_name,
        turn_counter_target_zh: counter_target_zh,
        compensated_stages: compensated,
        continuity_c: c_val,
        variation_v,
        max_gap_ticks: max_gap,
        phase_specialty_zh,
        omp_roots: (w1_ch, w2_ch),
        omp_word_ids: (w1_id, w2_id),
        formal_name,
        origin_basis_zh,
        learned_and_familiar_zh,
        auto_trigger_desc_zh,
    }
}

/// 依個人骨相與百工歷練推導個人百工藝相
pub fn derive_personal_craft_routine(
    bone: &InnateBonePhase,
    craft_code: &str, // "harvest" | "forge" | "culinary" | "wayfare"
    carrier: CarrierAffinity,
    vessel_name: &str,
    familiarity_count: u32,
    anatomy_familiarity: u32,
    registry: &WordRegistry,
) -> EmergentRoutine {
    let catalog = build_elementary_motion_catalog();

    let (slot_code, domain_label_zh, craft_idx, craft_specialty_zh, craft_suffix) = match craft_code {
        "harvest" => ("harvest", "山林採獵", 0usize, "順理游刃", "法"),
        "forge" => ("forge", "爐錘鍛造", 1usize, "借勢運錘", "法"),
        "culinary" => ("culinary", "庖廚火候", 2usize, "循理調鼎", "訣"),
        "wayfare" => ("wayfare", "涉遠行旅", 3usize, "借勢健行", "步"),
        _ => ("harvest", "百工手藝", 0usize, "順理游刃", "法"),
    };

    let mut by_stage: [Vec<&ElementaryMotionDef>; 5] = [Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    for m in &catalog {
        if m.craft_domain == Some(craft_code) && m.stage < 5 {
            by_stage[m.stage].push(m);
        }
    }

    // 防呆兜底
    for s in 0..5 {
        if by_stage[s].is_empty() {
            for m in &catalog {
                if m.stage == s {
                    by_stage[s].push(m);
                }
            }
        }
    }

    let learned_delta = ((anatomy_familiarity as f64) * 0.05).min(1.2);
    let (motions, compensated, c_val) = find_viterbi_5stages(
        bone,
        &by_stage,
        learned_delta,
        familiarity_count,
    );

    let main_motions = [
        motions[0].name.clone(),
        motions[1].name.clone(),
        motions[2].name.clone(),
        motions[3].name.clone(),
        motions[4].name.clone(),
    ];

    let main_turn_name = &main_motions[2];
    let mut best_branch_name = main_motions[2].clone();
    for m in &by_stage[2] {
        if m.name != *main_turn_name {
            best_branch_name = m.name.clone();
            break;
        }
    }

    let mut feat_16d = [0.0f64; 16];
    for (i, m) in motions.iter().enumerate() {
        let weight = (i + 1) as f64 / 15.0;
        for k in 0..16 {
            feat_16d[k] += m.feature_16d[k] * weight;
        }
    }
    feat_16d[12 + craft_idx] += 0.85;

    let ((w1_id, w2_id), (w1_ch, w2_ch)) = residual_omp_select_roots(&feat_16d, registry);
    let first2_c: String = main_motions[0].chars().take(2).collect();
    let last2_c: String = {
        let chs: Vec<char> = main_motions[3].chars().collect();
        if chs.len() >= 2 {
            chs[chs.len() - 2..].iter().collect()
        } else {
            main_motions[3].clone()
        }
    };
    let formal_name = format!("{}{}{}", first2_c, last2_c.trim_end_matches(craft_suffix), craft_suffix);

    let tool_clean = if vessel_name.trim().is_empty() {
        "隨身器具"
    } else {
        vessel_name.trim()
    };

    let origin_basis_zh = match craft_code {
        "harvest" => {
            let comp_note = if compensated.is_empty() {
                "手眼與刃口配合純熟，下刀輕快俐落。"
            } else {
                "割剝時順應手腕習慣調整入刃角度，省力而不傷皮張。"
            };
            format!(
                "日常持用{}巡歷山林獵場，於採集與剝解獵物時摸索皮肉筋節走向。{}",
                tool_clean,
                comp_note
            )
        }
        "forge" => {
            let comp_note = if compensated.is_empty() {
                "呼吸與落錘節拍相合，久鍛而臂力不衰。"
            } else {
                "揮錘間借腰背擺動分擔腕肘負荷，連綿敲打依然沉穩。"
            };
            format!(
                "日常持用{}守候爐火砧台，依火色冷熱調諧重錘起落與修整節奏。{}",
                tool_clean,
                comp_note
            )
        }
        "culinary" => {
            let comp_note = if compensated.is_empty() {
                "運刀與翻炒一氣呵成，火候拿捏分毫不差。"
            } else {
                "分切與掌勺時順著手腕巧勁施力，久立灶前亦從容不迫。"
            };
            format!(
                "日常持用{}料理庖廚膳食，於分切食材與文武火候收放間養成細膩手感。{}",
                tool_clean,
                comp_note
            )
        }
        "wayfare" => {
            let comp_note = if compensated.is_empty() {
                "步幅與吐納自然相合，長途跋涉依然氣息平順。"
            } else {
                "遇陡坡石階時自然微調步幅與重心，減輕雙腿疲勞。"
            };
            format!(
                "日常佩備{}穿梭山野古道，於涉水攀嶺間磨礪出悠長穩健的跋涉步調。{}",
                tool_clean,
                comp_note
            )
        }
        _ => format!(
            "日常持用{}從事百工勞作，於反覆操持間自然養成的熟練手藝。",
            tool_clean
        ),
    };

    let contingency_zh = match craft_code {
        "harvest" => format!("若遇筋骨糾結或皮膜堅韌處，可順勢改以［{}］從容落刃。", best_branch_name),
        "forge" => format!("若遇礦鐵含雜或火色驟變，可即時改以［{}］調勻力道。", best_branch_name),
        "culinary" => format!("若遇肉質厚薄不一或鼎中火候微變，可隨時改以［{}］收束湯味。", best_branch_name),
        "wayfare" => format!("若遇山徑濕滑或溪谷陡峭，可隨步改以［{}］穩住身形。", best_branch_name),
        _ => format!("若遇質地生硬或節奏微滯，可隨機改以［{}］順暢應對。", best_branch_name),
    };

    let learned_and_familiar_zh = format!(
        "工序循［{} → {} → {} → {} → {}］次序推進；{}",
        main_motions[0], main_motions[1], main_motions[2], main_motions[3], main_motions[4],
        contingency_zh
    );
    let auto_trigger_desc_zh = match craft_code {
        "harvest" => "下刀順應皮毛與骨節紋理，下料俐落完整，不易傷及刃口與皮張。".to_string(),
        "forge" => "運錘落點沉穩勻稱，能依火色精準拿捏力道，減少雜質殘留。".to_string(),
        "culinary" => "刀工與火候拿捏得宜，能留住食材本味與湯汁精華。".to_string(),
        _ => "遠行換氣勻長，涉水攀嶺時步履穩健，耐勞而不易氣促。".to_string(),
    };

    EmergentRoutine {
        domain: "production",
        slot_code,
        slot_label_zh: domain_label_zh,
        carrier,
        main_motions,
        turn_branch_motion: best_branch_name,
        turn_counter_target_zh: "肌理阻滯",
        compensated_stages: compensated,
        continuity_c: c_val,
        variation_v: 1.0,
        max_gap_ticks: 1,
        phase_specialty_zh: craft_specialty_zh,
        omp_roots: (w1_ch, w2_ch),
        omp_word_ids: (w1_id, w2_id),
        formal_name,
        origin_basis_zh,
        learned_and_familiar_zh,
        auto_trigger_desc_zh,
    }
}

/// 將標準教材手札適配至個人骨相，自然產生代償變體
pub fn adapt_manual_to_bone_phase(
    bone: &InnateBonePhase,
    slot: TacticalSlot,
    carrier: CarrierAffinity,
    manual_name: &str,
    manual_roots: &[String],
    registry: &WordRegistry,
) -> (EmergentRoutine, String) {
    let mut routine = derive_personal_martial_routine(
        bone,
        slot,
        carrier,
        "隨身器物",
        1.0,
        15,
        registry,
    );

    if manual_roots.len() >= 2 {
        let ch1 = &manual_roots[0];
        let ch2 = &manual_roots[1];
        routine.omp_roots = (ch1.clone(), ch2.clone());
        let clean_manual: String = manual_name
            .trim_matches(|c: char| c == '《' || c == '》')
            .trim_end_matches("手札")
            .trim_end_matches("訣")
            .trim_end_matches("譜")
            .chars()
            .take(4)
            .collect();
        if !clean_manual.is_empty() {
            routine.formal_name = format!("{}式", clean_manual);
        }

        let id1 = registry.get_by_char(ch1).and_then(|ids| ids.first().cloned()).unwrap_or_else(|| routine.omp_word_ids.0.clone());
        let id2 = registry.get_by_char(ch2).and_then(|ids| ids.first().cloned()).unwrap_or_else(|| routine.omp_word_ids.1.clone());
        routine.omp_word_ids = (id1, id2);
        routine.learned_and_familiar_zh = format!(
            "招式循［{} → {} → {} → {} → {}］一氣連貫；交鋒中途若遇對手{}，可順勢變招改走［{}］。深得《{}》所載之法度精髓。",
            routine.main_motions[0],
            routine.main_motions[1],
            routine.main_motions[2],
            routine.main_motions[3],
            routine.main_motions[4],
            routine.turn_counter_target_zh,
            routine.turn_branch_motion,
            manual_name,
        );
    }

    let adaptation_log = if routine.compensated_stages.is_empty() {
        format!("此本《{}》與自身體魄底蘊絲絲入扣，依法演練即可發揮十成威力。", manual_name)
    } else {
        format!(
            "研讀《{}》之際，順應自身手腕與步法發力習慣微調了轉折角度，化為得心應手之路數。",
            manual_name
        )
    };
    routine.origin_basis_zh = format!(
        "依自身「{}」之體魄底蘊，參酌《{}》之法度運轉。{}",
        bone.bone_archetype_title(),
        manual_name,
        adaptation_log
    );

    (routine, adaptation_log)
}

// ==========================================
// 模組四：事件分派之三相動力學判準
// ==========================================

/// 載體預備時間基底
fn carrier_pre_base(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Footwork => 1.0,
        CarrierAffinity::Blade | CarrierAffinity::Unarmed => 1.5,
        CarrierAffinity::Pole => 2.0,
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 2.5,
    }
}

/// 載體肢段幾何係數
pub fn carrier_limb_factor(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Unarmed | CarrierAffinity::Footwork => 0.85,
        CarrierAffinity::Blade => 1.0,
        CarrierAffinity::Pole => 1.15,
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.30,
    }
}

/// 載體基準器物配重（公斤）
pub fn carrier_base_tool_mass(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Blunt => 14.0,
        CarrierAffinity::Shield => 10.0,
        CarrierAffinity::Pole => 5.0,
        CarrierAffinity::Blade => 3.5,
        CarrierAffinity::Unarmed | CarrierAffinity::Footwork => 0.0,
    }
}

/// 載體截擊係數 carrier_int
pub fn carrier_intercept_factor(c: CarrierAffinity) -> f64 {
    match c {
        CarrierAffinity::Footwork => 1.30,
        CarrierAffinity::Blade => 1.15,
        CarrierAffinity::Unarmed => 1.05,
        CarrierAffinity::Pole => 0.95,
        CarrierAffinity::Shield => 0.85,
        CarrierAffinity::Blunt => 0.75,
    }
}

/// 人獸對稱有效質量 M_eff：肢段有效質量與載體基準配重
pub fn effective_carrier_mass(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    _external_weight: f64,
) -> f64 {
    let m_limb = (0.12 * bone.vit_0 + 0.06 * bone.qi_0)
        * bone.alpha.sqrt()
        * carrier_limb_factor(carrier);
    let m_tool = carrier_base_tool_mass(carrier);
    m_limb + m_tool
}

/// 連續浮點時序計算：預備時間 tau_pre 與換招空檔破綻窗 tau_gap
pub fn compute_continuous_timings(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    single_move_depth_m: f64,
    continuity_c: f64,
    scar_severity: f64,
) -> (f64, f64) {
    let pre_base = carrier_pre_base(carrier);
    let tau_pre = 1.0 + ((pre_base - 0.6).max(0.2))
        / (bone.beta.sqrt() * (1.0 + 0.35 * single_move_depth_m.max(0.0)));

    let tau_gap = (1.0
        + (pre_base * 1.40)
            / (bone.beta.sqrt()
                * (1.0 + 0.15 * bone.gamma.max(0.0))
                * (1.0 + 0.22 * continuity_c.max(0.0))))
        * (1.0 + 0.50 * scar_severity.max(0.0));

    (tau_pre, tau_gap)
}

/// 多局識破適應因子 R_read in [0.0, 0.75]
pub fn read_adaptation_factor(bout_index: usize, defender_v: f64, attacker_v: f64) -> f64 {
    (0.22
        * (bout_index as f64)
        * defender_v.clamp(0.1, 1.0)
        * (1.15 - 0.65 * attacker_v.clamp(0.1, 1.0)))
    .clamp(0.0, 0.75)
}

/// 第三相入微穿隙（庖丁解牛）評估：支援無上限熟練練習深度
pub fn evaluate_seam_threading_with_depth(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    edex: f64,
    target_tightness_kappa: f64,
    anatomy_familiarity: u32,
    single_move_depth_m: f64,
) -> SeamThreadingEval {
    let p_tool = match carrier {
        CarrierAffinity::Blade => 1.80,
        CarrierAffinity::Pole => 1.40,
        CarrierAffinity::Unarmed => 1.00,
        _ => 0.50,
    };

    let dex_beta = edex * bone.beta;
    let match_term = dex_beta / (dex_beta + target_tightness_kappa.max(1.0));

    let fam_term = 1.0
        + 0.25 * ((anatomy_familiarity as f64) / 20.0).powf(P_PRACTICE)
        + 0.12 * single_move_depth_m.max(0.0);

    let raw_xi = p_tool * match_term * fam_term;
    let xi_index = raw_xi.clamp(0.0, 2.30);

    let hardness_bypass = 1.0 - (-xi_index).exp();
    let wear_multiplier = 0.05f64.max((-xi_index).exp());

    SeamThreadingEval {
        xi_index,
        hardness_bypass,
        wear_multiplier,
    }
}

/// 第三相入微穿隙（庖丁解牛）評估（相容原簽名）
pub fn evaluate_seam_threading(
    bone: &InnateBonePhase,
    carrier: CarrierAffinity,
    edex: f64,
    target_seam_tightness: f64,
    anatomy_familiarity: u32,
) -> SeamThreadingEval {
    evaluate_seam_threading_with_depth(
        bone,
        carrier,
        edex,
        target_seam_tightness,
        anatomy_familiarity,
        0.0,
    )
}

/// 事件分派之三相交鋒評定（擴充版：支援多局識破、練習深度與傷殘係數）
pub fn evaluate_three_phase_bout_ext(
    slot: TacticalSlot,
    bone_a: &InnateBonePhase,
    bone_b: &InnateBonePhase,
    weight_a: f64,
    weight_b: f64,
    carrier_a: CarrierAffinity,
    carrier_b: CarrierAffinity,
    prior_posture_a: PostureState,
    prior_posture_b: PostureState,
    a_motions_len: usize,
    b_motions_len: usize,
    bout_index: usize,
    practice_a: u32,
    practice_b: u32,
    anatomy_a: u32,
    anatomy_b: u32,
    learned_a: f64,
    learned_b: f64,
    scar_a: f64,
    scar_b: f64,
) -> ThreePhaseEvalResult {
    let learned_delta_a = learned_a.clamp(0.0, 1.0);
    let learned_delta_b = learned_b.clamp(0.0, 1.0);
    let (cont_a, var_v_a) = compute_viterbi_continuity_and_variation(
        bone_a,
        slot,
        carrier_a,
        learned_delta_a,
        practice_a,
    );
    let (cont_b, var_v_b) = compute_viterbi_continuity_and_variation(
        bone_b,
        slot,
        carrier_b,
        learned_delta_b,
        practice_b,
    );
    let depth_a = practice_depth(practice_a as f64);
    let depth_b = practice_depth(practice_b as f64);
    let (tau_pre_a, tau_gap_a) =
        compute_continuous_timings(bone_a, carrier_a, depth_a, cont_a, scar_a);
    let (tau_pre_b, tau_gap_b) =
        compute_continuous_timings(bone_b, carrier_b, depth_b, cont_b, scar_b);

    // 多局識破適應：以真實轉位變化性 (var_v_b, var_v_a) 相互推演，收窄破綻窗
    let r_read_a = read_adaptation_factor(bout_index, var_v_a, var_v_b);
    let r_read_b = read_adaptation_factor(bout_index, var_v_b, var_v_a);

    let tau_gap_eff_a = 1.0 + (tau_gap_a - 1.0).max(0.0) * (1.0 - r_read_a);
    let tau_gap_eff_b = 1.0 + (tau_gap_b - 1.0).max(0.0) * (1.0 - r_read_b);

    // 計算角度對齊因子 Omega in [0.4, 1.35]
    let omega_factor = match slot {
        TacticalSlot::Opening => 1.25,
        TacticalSlot::Midgame => 1.0,
        TacticalSlot::Finisher => 0.85,
        TacticalSlot::Defense => 1.35,
    };

    // 計算雙方有效質量與衝量
    let kappa_a = match carrier_a {
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
        CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
        _ => 0.8,
    };
    let kappa_b = match carrier_b {
        CarrierAffinity::Blunt | CarrierAffinity::Shield => 1.4,
        CarrierAffinity::Blade | CarrierAffinity::Pole => 1.0,
        _ => 0.8,
    };

    let weight_scale_a = if weight_a > 0.0 && weight_b > 0.0 {
        (weight_a / 50.0).clamp(0.35, 2.5)
    } else {
        1.0
    };
    let weight_scale_b = if weight_a > 0.0 && weight_b > 0.0 {
        (weight_b / 50.0).clamp(0.35, 2.5)
    } else {
        1.0
    };

    let m_eff_a = effective_carrier_mass(bone_a, carrier_a, weight_a) * weight_scale_a;
    let m_eff_b = effective_carrier_mass(bone_b, carrier_b, weight_b) * weight_scale_b;

    let i_a = kappa_a * m_eff_a.max(4.0) * bone_a.vit_0 * bone_a.impact_factor();
    let i_b = kappa_b * m_eff_b.max(4.0) * bone_b.vit_0 * bone_b.impact_factor();

    let post_mult_a = match prior_posture_a {
        PostureState::Steady => 1.0,
        PostureState::Shaken => 0.7,
        PostureState::Broken => 0.3,
    };
    let post_mult_b = match prior_posture_b {
        PostureState::Steady => 1.0,
        PostureState::Shaken => 0.7,
        PostureState::Broken => 0.3,
    };

    let s_dex_a = bone_a.cell_smoothness(2, 2);
    let s_dex_b = bone_b.cell_smoothness(2, 2);

    let i_hold_a = post_mult_a * m_eff_a.max(4.0) * bone_a.vit_0 * (1.0 + 0.35 * (-bone_a.gamma).max(0.0) * s_dex_a);
    let i_hold_b = post_mult_b * m_eff_b.max(4.0) * bone_b.vit_0 * (1.0 + 0.35 * (-bone_b.gamma).max(0.0) * s_dex_b);

    // 1. 第一相【一力降十會（正面硬撼）】：Lambda >= 1.5 且實際量級高於對手至少 15%
    let lambda_a = i_a / i_hold_b.max(1.0);
    let lambda_b = i_b / i_hold_a.max(1.0);

    let a_can_crush_b = lambda_a >= 1.5
        && weight_a >= weight_b * 1.15
        && !(carrier_b == CarrierAffinity::Footwork && slot == TacticalSlot::Defense);
    let b_can_crush_a = lambda_b >= 1.5
        && weight_b >= weight_a * 1.15
        && !(carrier_a == CarrierAffinity::Footwork && slot == TacticalSlot::Defense);
    if a_can_crush_b && (!b_can_crush_a || lambda_a > lambda_b + 1e-6) {
        let bounded_delta = 1.8 * (lambda_a - 1.0).min(1.20);
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase1ImpulseCrush {
                winner_is_a: true,
                lambda_ratio: lambda_a,
            },
            score_delta_a: bounded_delta,
            score_delta_b: -1.2,
            counter_reason: Some("重勢硬撼：千鈞重勢硬撼而下，沛然巨力震散敵方招式架勢！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_completed_motions: a_motions_len.max(3),
            b_completed_motions: 1,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if b_can_crush_a && (!a_can_crush_b || lambda_b > lambda_a + 1e-6) {
        let bounded_delta = 1.8 * (lambda_b - 1.0).min(1.20);
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase1ImpulseCrush {
                winner_is_a: false,
                lambda_ratio: lambda_b,
            },
            score_delta_a: -1.2,
            score_delta_b: bounded_delta,
            counter_reason: Some("重勢硬撼：千鈞重勢硬撼而下，沛然巨力震散敵方招式架勢！".to_string()),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_completed_motions: 1,
            b_completed_motions: b_motions_len.max(3),
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 2. 第二相【專克拆招（玉女克全真）】：連續相對餘裕截擊
    let margin_ab = ((tau_gap_eff_b - tau_pre_a) / tau_gap_eff_b.max(1e-6)).max(0.0);
    let margin_ba = ((tau_gap_eff_a - tau_pre_b) / tau_gap_eff_a.max(1e-6)).max(0.0);

    let int_a = if i_a / i_hold_b.max(1.0) >= 0.55 && margin_ab > 0.0 {
        margin_ab
            * carrier_intercept_factor(carrier_a)
            * (1.0 + 0.20 * bone_a.gamma.max(0.0))
            * (1.0 + 0.15 * cont_a)
            * omega_factor
    } else {
        -1.0
    };

    let int_b = if i_b / i_hold_a.max(1.0) >= 0.55 && margin_ba > 0.0 {
        margin_ba
            * carrier_intercept_factor(carrier_b)
            * (1.0 + 0.20 * bone_b.gamma.max(0.0))
            * (1.0 + 0.15 * cont_b)
            * omega_factor
    } else {
        -1.0
    };

    if int_a >= 0.20 && int_a > int_b + 0.05 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: true,
                intercept_score: int_a,
                gap_ticks: tau_gap_eff_b,
            },
            score_delta_a: 1.2,
            score_delta_b: -0.8,
            counter_reason: Some("借勢拆招：身形微側切入換招死角，當場截斷其後續連段！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: false,
            a_completed_motions: a_motions_len,
            b_completed_motions: 2,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if int_b >= 0.20 && int_b > int_a + 0.05 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase2KinematicIntercept {
                interceptor_is_a: false,
                intercept_score: int_b,
                gap_ticks: tau_gap_eff_a,
            },
            score_delta_a: -0.8,
            score_delta_b: 1.2,
            counter_reason: Some("借勢拆招：身形微側切入換招死角，當場截斷其後續連段！".to_string()),
            a_manifested_as_skill: false,
            b_manifested_as_skill: true,
            a_completed_motions: 2,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 3. 第三相【入微穿隙（庖丁解牛）】：依架勢與時序差計算動態緊密度
    let posture_tightness = |p: PostureState| -> f64 {
        match p {
            PostureState::Steady => 100.0,
            PostureState::Shaken => 70.0,
            PostureState::Broken => 40.0,
        }
    };

    let kappa_target_b = posture_tightness(prior_posture_b)
        / (1.0 + 0.30 * (tau_gap_eff_b - tau_pre_a).max(0.0));
    let kappa_target_a = posture_tightness(prior_posture_a)
        / (1.0 + 0.30 * (tau_gap_eff_a - tau_pre_b).max(0.0));

    let seam_a = evaluate_seam_threading_with_depth(
        bone_a,
        carrier_a,
        bone_a.dex_0,
        kappa_target_b,
        anatomy_a,
        depth_a,
    );
    let seam_b = evaluate_seam_threading_with_depth(
        bone_b,
        carrier_b,
        bone_b.dex_0,
        kappa_target_a,
        anatomy_b,
        depth_b,
    );

    if seam_a.xi_index >= 1.50 && seam_a.xi_index > seam_b.xi_index + 0.30 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: true,
                xi_index: seam_a.xi_index,
                bypass_pct: seam_a.hardness_bypass * 100.0,
            },
            score_delta_a: 0.85 * seam_a.xi_index,
            score_delta_b: -0.5,
            counter_reason: Some("尋隙游刃：游刃順應骨縫筋節穿入，避實擊虛直透要害！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: true,
            a_completed_motions: a_motions_len,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a,
            b_posture_after: prior_posture_b.degrade(),
        };
    } else if seam_b.xi_index >= 1.50 && seam_b.xi_index > seam_a.xi_index + 0.30 {
        return ThreePhaseEvalResult {
            verdict: ThreePhaseVerdict::Phase3SeamThreading {
                threader_is_a: false,
                xi_index: seam_b.xi_index,
                bypass_pct: seam_b.hardness_bypass * 100.0,
            },
            score_delta_a: -0.5,
            score_delta_b: 0.85 * seam_b.xi_index,
            counter_reason: Some("尋隙游刃：游刃順應骨縫筋節穿入，避實擊虛直透要害！".to_string()),
            a_manifested_as_skill: true,
            b_manifested_as_skill: true,
            a_completed_motions: 2,
            b_completed_motions: b_motions_len,
            a_posture_after: prior_posture_a.degrade(),
            b_posture_after: prior_posture_b,
        };
    }

    // 均勢交鋒
    ThreePhaseEvalResult {
        verdict: ThreePhaseVerdict::StandardExchange,
        score_delta_a: 0.0,
        score_delta_b: 0.0,
        counter_reason: None,
        a_manifested_as_skill: true,
        b_manifested_as_skill: true,
        a_completed_motions: a_motions_len,
        b_completed_motions: b_motions_len,
        a_posture_after: prior_posture_a,
        b_posture_after: prior_posture_b,
    }
}

/// 事件分派之三相交鋒評定（維持原簽名 100% 相容）
pub fn evaluate_three_phase_bout(
    slot: TacticalSlot,
    bone_a: &InnateBonePhase,
    bone_b: &InnateBonePhase,
    weight_a: f64,
    weight_b: f64,
    carrier_a: CarrierAffinity,
    carrier_b: CarrierAffinity,
    prior_posture_a: PostureState,
    prior_posture_b: PostureState,
    a_motions_len: usize,
    b_motions_len: usize,
) -> ThreePhaseEvalResult {
    evaluate_three_phase_bout_ext(
        slot,
        bone_a,
        bone_b,
        weight_a,
        weight_b,
        carrier_a,
        carrier_b,
        prior_posture_a,
        prior_posture_b,
        a_motions_len,
        b_motions_len,
        0,
        25,
        25,
        20,
        20,
        0.5,
        0.5,
        0.0,
        0.0,
    )
}

// 模組五：2-Step Residual OMP 殘差正交匹配定名
// ==========================================

const CLASSICAL_PARTICLES: &[&str] = &[
    "夫", "蓋", "焉", "哉", "乎", "也", "矣", "兮", "耳", "爾", "歟", "耶", "云", "之", "其", "於", "以", "而", "則", "乃", "且", "若", "所", "者"
];

fn dot_product_16(a: &[f64; 16], b: &[f64; 16]) -> f64 {
    let mut sum = 0.0;
    for i in 0..16 {
        sum += a[i] * b[i];
    }
    sum
}

fn norm_l2_16(a: &[f64; 16]) -> f64 {
    let d = dot_product_16(a, a);
    if d <= 0.0 {
        0.0
    } else {
        d.sqrt()
    }
}

/// 2 步殘差正交匹配追蹤定名
pub fn residual_omp_select_roots(
    feature_16d: &[f64; 16],
    registry: &WordRegistry,
) -> ((String, String), (String, String)) {
    // 1. 正規化目標向量
    let feat_norm = norm_l2_16(feature_16d);
    let mut phi_hat = [0.0f64; 16];
    if feat_norm > 1e-9 {
        for i in 0..16 {
            phi_hat[i] = feature_16d[i] / feat_norm;
        }
    } else {
        phi_hat[0] = 1.0;
    }

    // 2. 候選字根庫：過濾虛字並將 (12D + 4D) 轉為 16D 單位向量
    let mut candidates: Vec<(&WordElement, [f64; 16])> = Vec::with_capacity(registry.len());

    for elem in registry.ordered_elements() {
        if CLASSICAL_PARTICLES.contains(&elem.ch.as_str()) || elem.semantic == "助" {
            continue;
        }
        let mut v16 = [0.0f64; 16];
        let arr12 = elem.vector_12d.to_array();
        for i in 0..12 {
            v16[i] = arr12[i];
        }
        v16[12] = elem.craft_dims.harvest;
        v16[13] = elem.craft_dims.forge;
        v16[14] = elem.craft_dims.culinary;
        v16[15] = elem.craft_dims.wayfare;

        let n = norm_l2_16(&v16);
        if n > 1e-9 {
            for i in 0..16 {
                v16[i] /= n;
            }
            candidates.push((elem, v16));
        }
    }

    if candidates.is_empty() {
        return (
            ("w_default_1".to_string(), "w_default_2".to_string()),
            ("開".to_string(), "闔".to_string()),
        );
    }

    // 步驟 1：找內積最大之第一字根
    let mut best_score1 = -1.0f64;
    let mut best_idx1 = 0;

    for (idx, (_, u)) in candidates.iter().enumerate() {
        let score = dot_product_16(&phi_hat, u);
        if score > best_score1 {
            best_score1 = score;
            best_idx1 = idx;
        }
    }

    let (elem1, u1) = &candidates[best_idx1];
    let proj1 = dot_product_16(&phi_hat, u1);

    // 步驟 2：計算正交殘差 r1 = phi_hat - <phi_hat, u1> u1
    let mut r1 = [0.0f64; 16];
    for i in 0..16 {
        r1[i] = phi_hat[i] - proj1 * u1[i];
    }
    let r1_norm = norm_l2_16(&r1);
    let mut r1_hat = [0.0f64; 16];
    if r1_norm > 1e-9 {
        for i in 0..16 {
            r1_hat[i] = r1[i] / r1_norm;
        }
    } else {
        // 若殘差極小，以正交擾動維度代入
        r1_hat[(best_idx1 + 1) % 16] = 1.0;
    }

    // 步驟 3：從殘差中挑選第二字根 (w != w1*)
    let mut best_score2 = -1.0f64;
    let mut best_idx2 = 0;

    for (idx, (elem, u)) in candidates.iter().enumerate() {
        if elem.id == elem1.id || elem.ch == elem1.ch {
            continue;
        }
        let score = dot_product_16(&r1_hat, u);
        if score > best_score2 {
            best_score2 = score;
            best_idx2 = idx;
        }
    }

    let (elem2, _) = &candidates[best_idx2];

    (
        (elem1.id.clone(), elem2.id.clone()),
        (elem1.ch.clone(), elem2.ch.clone()),
    )
}

// ==========================================
// 單元測試（涵蓋五大模組與代數守恆）
// ==========================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::wordplate::{CarrierAffinity, PostureState, TacticalSlot, WordRegistry};

    #[test]
    fn test_module1_algebraic_conservation() {
        for seed in [12345i64, 20261028, 999999, -42, 777] {
            let bone = InnateBonePhase::from_seed(seed);

            // 1. 對數空間功率守恆
            assert!(bone.alpha >= 0.5 && bone.alpha <= 2.0);
            assert!(bone.beta >= 0.5 && bone.beta <= 2.0);
            let p_mult = bone.alpha * bone.beta;
            assert!(
                p_mult >= 0.860 && p_mult <= 1.165,
                "power product {p_mult} out of bounds"
            );

            // 2. 15 區運動鏈阻抗矩陣總和守恆
            let mut sum_r = 0.0f64;
            for u in 0..3 {
                for v in 0..5 {
                    let r = bone.r_matrix[u][v];
                    assert!(r >= 299.99 && r <= 1200.01, "r_val {r} out of bounds");
                    sum_r += r;
                }
            }
            assert!(
                (sum_r - 10000.0).abs() < 1e-4,
                "R sum {sum_r} != 10000.0"
            );

            // 3. 外顯三維守恆
            let total_stats = bone.vit_0 + bone.qi_0 + bone.dex_0;
            assert!(
                (total_stats - 310.0).abs() < 1e-4,
                "Stats sum {total_stats} != 310.0"
            );

            // 4. 順暢度界限
            for u in 0..3 {
                for v in 0..5 {
                    let s = bone.cell_smoothness(u, v);
                    assert!(s > 0.25 && s < 1.75);
                }
            }

            // 5. 白話文字不可包含阿拉伯數字或禁詞
            let arch_title = bone.bone_archetype_title();
            let desc_zh = bone.describe_bone_phase_zh();
            let five_zh = bone.describe_five_stage_fluency_zh();
            let auto_zh = bone.auto_trigger_summary_zh();

            for s in [&arch_title, &desc_zh, &five_zh, &auto_zh] {
                assert!(!s.contains("還有"));
                assert!(!s.contains("資料庫"));
                assert!(!s.contains("110V"));
                assert!(!s.contains("念場"));
                assert!(!s.chars().any(|c| c.is_ascii_digit()));
            }
        }
    }

    #[test]
    fn test_module2_and_3_viterbi_emergence() {
        let registry = WordRegistry::global();
        let bone = InnateBonePhase::from_seed(20261028);

        let routine = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Opening,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );

        assert_eq!(routine.domain, "martial");
        assert_eq!(routine.carrier, CarrierAffinity::Blade);
        assert_eq!(routine.main_motions.len(), 5);
        assert!(!routine.turn_branch_motion.is_empty());
        assert!(routine.continuity_c > 0.0);
        assert!(routine.max_gap_ticks >= 1);
        assert!(!routine.omp_roots.0.is_empty());
        assert!(!routine.omp_roots.1.is_empty());
        assert!(!routine.formal_name.is_empty());
    }

    #[test]
    fn test_module4_three_phase_eval() {
        let bone_heavy = InnateBonePhase::from_seed(9999);
        let bone_light = InnateBonePhase::from_seed(8888);

        // 跨量級硬撼：120kg 巨錘 vs 40kg 匕首
        let res_crush = evaluate_three_phase_bout(
            TacticalSlot::Opening,
            &bone_heavy,
            &bone_light,
            120.0,
            40.0,
            CarrierAffinity::Blunt,
            CarrierAffinity::Blade,
            PostureState::Steady,
            PostureState::Steady,
            5,
            5,
        );

        match res_crush.verdict {
            ThreePhaseVerdict::Phase1ImpulseCrush { winner_is_a, .. } => {
                assert!(winner_is_a);
                assert_eq!(res_crush.b_completed_motions, 1);
                assert!(!res_crush.b_manifested_as_skill);
                assert_eq!(res_crush.b_posture_after, PostureState::Shaken);
            }
            _ => panic!("Expected Phase1ImpulseCrush"),
        }

        // 庖丁解牛穿隙
        let seam = evaluate_seam_threading(
            &bone_light,
            CarrierAffinity::Blade,
            120.0,
            80.0,
            25,
        );
        assert!(seam.xi_index >= 0.0 && seam.xi_index <= 2.30);
        assert!(seam.hardness_bypass >= 0.0 && seam.hardness_bypass <= 0.92);
        assert!(seam.wear_multiplier >= 0.05);
    }

    #[test]
    fn test_determinism_soul_seed_and_limb_mass() {
        let seed1 = resolve_entity_soul_seed("entity_npc_tiger_99", None);
        let seed2 = resolve_entity_soul_seed("entity_npc_tiger_99", None);
        assert_eq!(seed1, seed2);
        assert_ne!(seed1, 0);
        assert_ne!(seed1, 100_003);

        let explicit = resolve_entity_soul_seed("entity_npc_tiger_99", Some(777));
        assert_eq!(explicit, 777);

        let bone = InnateBonePhase::from_seed(seed1);
        let m_unarmed = effective_carrier_mass(&bone, CarrierAffinity::Unarmed, 0.0);
        let m_blunt = effective_carrier_mass(&bone, CarrierAffinity::Blunt, 15.0);
        assert!(m_unarmed > 0.0);
        assert!(m_blunt > m_unarmed);
    }

    #[test]
    fn test_continuous_timings_and_read_adaptation() {
        let bone = InnateBonePhase::from_seed(42);
        let (tau_pre, tau_gap) =
            compute_continuous_timings(&bone, CarrierAffinity::Blade, 1.2, 0.6, 0.0);
        assert!(tau_pre >= 1.0);
        assert!(tau_gap >= 1.0);

        let r0 = read_adaptation_factor(0, 0.8, 0.5);
        assert_eq!(r0, 0.0);
        let r3 = read_adaptation_factor(3, 0.8, 0.5);
        assert!(r3 > 0.0 && r3 <= 0.75);

        let (pre_scar, gap_scar) =
            compute_continuous_timings(&bone, CarrierAffinity::Blade, 1.2, 0.6, 0.25);
        assert!(gap_scar > tau_gap);
        assert_eq!(pre_scar, tau_pre);
    }

    #[test]
    fn test_unbounded_practice_and_smoothness_scaling() {
        let p1 = practice_depth(25.0);
        assert!((p1 - 1.0).abs() < 1e-4);
        let p100 = practice_depth(100.0);
        assert!(p100 > p1);

        let budget1 = effective_time_budget(1000.0, 1, 0.2);
        let budget3 = effective_time_budget(1000.0, 3, 0.2);
        assert_eq!(budget1, 1000.0);
        assert!(budget3 < budget1);

        let decayed = apply_annual_idle_decay(100.0);
        assert!((decayed - 97.0).abs() < 1e-4);

        let delta_high_s = manual_comprehension_delta(1.2, 3.0);
        let delta_low_s = manual_comprehension_delta(0.4, 3.0);
        assert!(delta_high_s > delta_low_s);

        let h_high_s = mastery_factor_with_smoothness(0.5, 50, 1.5);
        let h_low_s = mastery_factor_with_smoothness(0.5, 50, 0.5);
        assert!(h_high_s > h_low_s);
    }

    #[test]
    fn test_seam_b_mirror_and_real_viterbi_coupling() {
        let bone_dexterous = InnateBonePhase::from_seed(8888);
        let _bone_stiff = InnateBonePhase::from_seed(9999);

        let (cont, var_v) = compute_viterbi_continuity_and_variation(
            &bone_dexterous,
            TacticalSlot::Midgame,
            CarrierAffinity::Blade,
            0.8,
            40,
        );
        assert!(cont > 0.0);
        assert!(var_v >= 0.40 && var_v <= 1.80);

        // 測試乙方 (defender) 穿隙勝出分支
        let res_seam_b = evaluate_three_phase_bout_ext(
            TacticalSlot::Midgame,
            &bone_dexterous,
            &bone_dexterous,
            60.0,
            60.0,
            CarrierAffinity::Blade,
            CarrierAffinity::Blade,
            PostureState::Shaken,
            PostureState::Steady,
            5,
            5,
            0,
            25,
            25,
            0,
            200,
            0.8,
            0.8,
            0.0,
            0.0,
        );

        match res_seam_b.verdict {
            ThreePhaseVerdict::Phase3SeamThreading { threader_is_a, xi_index, .. } => {
                assert!(!threader_is_a);
                assert!(xi_index >= 1.50);
                assert_eq!(res_seam_b.a_posture_after, PostureState::Broken);
                assert_eq!(res_seam_b.b_posture_after, PostureState::Steady);
                assert_eq!(res_seam_b.a_completed_motions, 2);
            }
            _ => panic!("Expected Phase3SeamThreading with threader_is_a == false, got {:?}", res_seam_b.verdict),
        }
    }

    #[test]
    fn test_module5_residual_omp_roots() {
        let registry = WordRegistry::global();
        let feat = [
            0.8, 0.2, 0.1, 0.4, 0.0, 0.0, 0.5, 0.1, 0.2, 0.3, 0.0, 0.0, 0.2, 0.7, 0.1, 0.0,
        ];

        let ((id1, id2), (ch1, ch2)) = residual_omp_select_roots(&feat, registry);
        assert_ne!(id1, id2);
        assert_ne!(ch1, ch2);
        assert!(!CLASSICAL_PARTICLES.contains(&ch1.as_str()));
        assert!(!CLASSICAL_PARTICLES.contains(&ch2.as_str()));
    }

    #[test]
    fn test_tactical_slots_diversity_and_craft_isolation() {
        let registry = WordRegistry::global();
        let bone = InnateBonePhase::from_seed(20261028);

        // 1. 驗證同一 bone 下，4 個 TacticalSlot 產生 4 個不同的 formal_name
        let r_open = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Opening,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_mid = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Midgame,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_fin = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Finisher,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );
        let r_def = derive_personal_martial_routine(
            &bone,
            TacticalSlot::Defense,
            CarrierAffinity::Blade,
            "青鋼長劍",
            0.5,
            10,
            registry,
        );

        let names = [
            &r_open.formal_name,
            &r_mid.formal_name,
            &r_fin.formal_name,
            &r_def.formal_name,
        ];
        let mut unique_names = std::collections::HashSet::new();
        for name in names {
            unique_names.insert(name);
        }
        assert_eq!(unique_names.len(), 4, "Expected 4 distinct names for 4 slots, got: {:?}", names);

        // 2. 驗證 Blade 武相動作中絕不混入百工動作
        let catalog = build_elementary_motion_catalog();
        let craft_names: std::collections::HashSet<String> = catalog
            .iter()
            .filter(|m| m.craft_domain.is_some())
            .map(|m| m.name.clone())
            .collect();

        for routine in [&r_open, &r_mid, &r_fin, &r_def] {
            for m_name in &routine.main_motions {
                assert!(
                    !craft_names.contains(m_name),
                    "Martial routine contains craft motion: {m_name}"
                );
            }
            assert!(!routine.learned_and_familiar_zh.contains("\\to"));
        }

        // 3. 驗證 wayfare 行旅藝相有 5 個各不相同的階段碎招，且零 "\\to"
        let r_wayfare = derive_personal_craft_routine(
            &bone,
            "wayfare",
            CarrierAffinity::Footwork,
            "羅盤與行囊",
            10,
            10,
            registry,
        );
        let mut wayfare_motion_set = std::collections::HashSet::new();
        for m_name in &r_wayfare.main_motions {
            wayfare_motion_set.insert(m_name);
        }
        assert_eq!(
            wayfare_motion_set.len(),
            5,
            "Wayfare craft routine must have 5 distinct stage motions, got: {:?}",
            r_wayfare.main_motions
        );
        assert!(!r_wayfare.learned_and_familiar_zh.contains("\\to"));
        assert!(!r_wayfare.auto_trigger_desc_zh.contains("\\to"));
    }
}

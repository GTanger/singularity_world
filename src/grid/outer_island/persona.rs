//! 「脾氣 × 個性 × 性格」三層時間尺度萬人萬相核心引擎（`soul_seed` 唯一條碼純函數展開＋後天記憶漂移）。
//!
//! 嚴守正典 §五：
//! 1. 先天 `InnatePersona`（118B）100% 由唯一 `soul_seed` 與 `InnateBonePhase` 中度相關（ρ≈0.5）純函數展開，零資料庫冗餘。
//! 2. 生理與經濟需求僅劃定目標，16 維主觀情境 × 秩-4 連續矩陣 W → 8 大反應傾向決定具體處事手路。
//! 3. 對話全程零性格形容詞標籤，由 Rust 先行鎖死立場、底價、讓步斜率與施壓抗性，封死 RLHF 順從與舞台劇小丑化。

use serde::{Deserialize, Serialize};

use crate::combat::human_phase::InnateBonePhase;
use crate::combat::wordplate::splitmix64;

/// 八項核心人生價值軸（8! = 40,320 種偏序排列）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueAxis {
    SurvivalSafety = 0,   // 活命安穩
    KinshipLoyalty = 1,   // 親族同袍
    CraftPride = 2,       // 手藝體面
    PactIntegrity = 3,    // 契約口風
    TangibleGain = 4,     // 現錢實利
    Autonomy = 5,         // 自在無束
    OrderHierarchy = 6,   // 強弱法度
    CuriosityVenture = 7, // 未知奇物
}

impl ValueAxis {
    pub const ALL: [Self; 8] = [
        Self::SurvivalSafety,
        Self::KinshipLoyalty,
        Self::CraftPride,
        Self::PactIntegrity,
        Self::TangibleGain,
        Self::Autonomy,
        Self::OrderHierarchy,
        Self::CuriosityVenture,
    ];

    pub fn from_idx(idx: u8) -> Self {
        Self::ALL[usize::from(idx % 8)]
    }

    pub fn in_world_motive_zh(self) -> &'static str {
        match self {
            Self::SurvivalSafety => "平安活命要緊",
            Self::KinshipLoyalty => "自家人與老交情不能丟",
            Self::CraftPride => "手藝招牌與臉面不能砸",
            Self::PactIntegrity => "說出口的字據口風得算數",
            Self::TangibleGain => "現錢實貨落袋才踏實",
            Self::Autonomy => "不受外人指手畫腳",
            Self::OrderHierarchy => "港裡規矩先後不能亂",
            Self::CuriosityVenture => "沒見過的新奇路數總想摸清",
        }
    }
}

/// 八大基礎處事反應傾向（經領域可行矩陣投影為具體手路）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResponseTendency {
    Confront = 0,     // 正面硬頂／當場討說法
    Withdraw = 1,     // 抽身避險／留後路
    Bargain = 2,      // 開價交換／算清條件
    SeekAlly = 3,     // 找保人同夥結勢
    Circumvent = 4,   // 迂迴變通／另闢蹊徑
    GrudgeLedger = 5, // 不動聲色記帳留後算
    ShelterWeak = 6,  // 護短擔待／幫襯弱小
    ComplyOrder = 7,  // 依規矩順勢辦理
}

impl ResponseTendency {
    pub const ALL: [Self; 8] = [
        Self::Confront,
        Self::Withdraw,
        Self::Bargain,
        Self::SeekAlly,
        Self::Circumvent,
        Self::GrudgeLedger,
        Self::ShelterWeak,
        Self::ComplyOrder,
    ];

    pub fn from_idx(idx: usize) -> Self {
        Self::ALL[idx % 8]
    }
}

/// 16 維主觀情境感知向量（同一客觀事件，因個人親疏、傷疲與利害而異）
#[derive(Debug, Clone, Copy, Default)]
pub struct Situation16D {
    pub v: [i32; 16],
}

impl Situation16D {
    #[expect(clippy::too_many_arguments, reason = "16D canonical perceptual basis")]
    pub fn new(
        threat: i32,
        power_gap: i32,
        familiarity: i32,
        grudge_favor: i32,
        gain_loss: i32,
        scarcity: i32,
        urgency: i32,
        publicity_face: i32,
        rule_clarity: i32,
        deception_cue: i32,
        weak_present: i32,
        novelty: i32,
        self_strain: i32,
        kin_group: i32,
        stake_scale: i32,
        escape_route: i32,
    ) -> Self {
        Self {
            v: [
                threat,
                power_gap,
                familiarity,
                grudge_favor,
                gain_loss,
                scarcity,
                urgency,
                publicity_face,
                rule_clarity,
                deception_cue,
                weak_present,
                novelty,
                self_strain,
                kin_group,
                stake_scale,
                escape_route,
            ],
        }
    }
}

/// ① 先天脾氣（6 維連續神經動力，[-64, 64]，與骨相中度相關 ρ≈0.5 且期望收益中性）
#[derive(Debug, Clone, Copy)]
pub struct Temperament6D {
    pub ignition: i8, // 點火門檻（負＝一點就著搶先機但多誤警；正＝沉得住氣但慢半拍）
    pub slope: i8,    // 爆發斜率（正＝出手猛烈耗力快；負＝平緩綿長）
    pub approach_avoid: i8, // 趨避反射（正＝遇異象往前湊；負＝先退半步看退路）
    pub damping: i8,  // 消退阻尼（正＝悶氣記掛極久；負＝罵完轉頭就散）
    pub persistence: i8, // 耐煩定力（正＝坐得住冷板凳；負＝久候易躁）
    pub pain_tolerance: i8, // 痛感耐受（正＝帶傷硬撐；負＝見痛即警）
}

/// 5 維無標籤語用說話手路（供對話投射，零性格形容詞）
#[derive(Debug, Clone, Copy)]
pub struct PragmaticVoice {
    pub verbosity: i8,      // 負＝惜字如金兩短句；正＝愛把來龍去脈講透
    pub directness: i8,     // 正＝開門見山報底價；負＝先探口風留三分話
    pub metaphor_slot: u8,  // 比喻習慣來源（本行器物／海象風浪／帳冊斤兩／山林獵跡）
    pub hedge_habit: i8,    // 轉折保留語氣強度
    pub question_ratio: i8, // 反問對方比例
}

/// 先天純函數展開之完整個體基底（118 Bytes，不落資料庫，由 `soul_seed` 隨時重算）
#[derive(Debug, Clone, Copy)]
pub struct InnatePersona {
    pub soul_seed: u64,
    pub temperament: Temperament6D,
    pub caps_u: [[i8; 4]; 16],
    pub caps_v: [[i8; 8]; 4],
    pub base_values: [u8; 8], // 總和固定 == 240（零上位神種守恆）
    pub taboo_axis: ValueAxis,
    pub soft_spot_axis: ValueAxis,
    pub concession_slope: i8, // [-24, 24]：正＝軟磨會讓步；負＝越糾纏越反感漲價
    pub pragmatics: PragmaticVoice,
}

/// 性情大變永久紀錄（一生最多 2 次，強制綁定具體記憶事件 ID）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PermanentCharacterShift {
    pub anchor_event_id: String,
    pub shifted_day: i64,
    pub summary_zh: String,
    pub value_delta: [i8; 8],
    pub tendency_delta: [i8; 8],
}

/// ③ 後天性格漂移狀態（僅存此差值進資料庫，含基線慢速回歸與性情大變三道閘）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CharacterDriftState {
    pub daily_value_drift: [i8; 8],
    pub daily_tendency_bias: [i8; 8],
    pub rumination_event_id: Option<String>,
    pub rumination_nights: u8,
    pub rumination_axis: u8,
    pub rumination_intensity: u8,
    pub permanent_shifts: Vec<PermanentCharacterShift>,
}

/// 對話前純 Rust 鎖定之本輪硬立場（LLM 嚴禁推翻）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueHardStance {
    CordialOpen,   // 坦率應答、平價交易
    HaggleFirm,    // 守緊底線、一文不讓
    RefuseColdly,  // 踩中逆鱗或反覆糾纏，直接拒客
    GrudgingFavor, // 念及舊恩或軟肋，破例通融但口風緊
    GuardedEvade,  // 戒心高張、只講表象不露底細
}

#[derive(Debug, Clone)]
pub struct StanceVerdict {
    pub stance: DialogueHardStance,
    pub dominant_tendency: ResponseTendency,
    pub top_value: ValueAxis,
    pub price_adjust_pct: i32,
    pub speech_rule_zh: String,
    pub posture_zh: String,
}

fn next_centered_i8(st: &mut u64, half_span: i32) -> i8 {
    *st = splitmix64(st.wrapping_add(0x9E37_79B9_7F4A_7C15));
    let span = (half_span * 2 + 1).max(3) as u64;
    let raw = (*st % span) as i32 - half_span;
    raw.clamp(-120, 120) as i8
}

/// 由唯一 `soul_seed` 純函數展開先天骨相耦合之「脾氣＋個性」（ρ≈0.5 部分相關，保留反差）
pub fn derive_innate_persona(soul_seed: u64) -> InnatePersona {
    let bone = InnateBonePhase::from_seed(soul_seed as i64);
    let mut st = splitmix64(soul_seed ^ 0xD1B5_4A32_D192_ED03);

    // 骨相潛因子標準化至 [-32, 32]
    let bone_fast = (((bone.beta - 1.0) * 32.0).round() as i32).clamp(-32, 32);
    let bone_heavy = (((bone.alpha - 1.0) * 32.0).round() as i32).clamp(-32, 32);
    let bone_order = ((bone.gamma * 32.0).round() as i32).clamp(-32, 32);

    // ρ ≈ 0.5：一半取自骨相動力，一半取自獨立靈魂正交自由度；且維持正負代價交換
    let orth_ig = i32::from(next_centered_i8(&mut st, 32));
    let ignition = ((-bone_fast + orth_ig) / 2).clamp(-64, 64) as i8; // 點火閾低＝快反應但易誤警
    let slope = ((bone_heavy + i32::from(next_centered_i8(&mut st, 32))) / 2).clamp(-64, 64) as i8;
    let approach_avoid =
        (-bone_order / 2 + i32::from(next_centered_i8(&mut st, 40))).clamp(-64, 64) as i8;
    // 等價代價守恆：點火極快者消退通常較快，沉緩重岳者一旦記掛消退阻尼更深
    let damping =
        ((i32::from(ignition) / 2) + i32::from(next_centered_i8(&mut st, 36))).clamp(-64, 64) as i8;
    let persistence =
        ((bone_order / 2) + (bone_heavy / 3) + i32::from(next_centered_i8(&mut st, 36)))
            .clamp(-64, 64) as i8;
    let pain_pct = (((bone.vit_0 - 103.0) * 1.2).round() as i32).clamp(-28, 28);
    let pain_tolerance = (pain_pct + i32::from(next_centered_i8(&mut st, 36))).clamp(-64, 64) as i8;

    let mut caps_u = [[0i8; 4]; 16];
    for row in &mut caps_u {
        for cell in row {
            *cell = next_centered_i8(&mut st, 32);
        }
    }
    let mut caps_v = [[0i8; 8]; 4];
    for row in &mut caps_v {
        for cell in row {
            *cell = next_centered_i8(&mut st, 32);
        }
    }

    // 8 項核心價值單純形投影：總和嚴格守恆 == 240，單項介於 [12, 58]
    let mut raw_v = [0u32; 8];
    let mut sum_v = 0u32;
    for item in &mut raw_v {
        st = splitmix64(st.wrapping_add(1));
        let w = 15 + (st % 85) as u32;
        *item = w;
        sum_v += w;
    }
    let mut base_values = [30u8; 8];
    let mut running = 0i32;
    for i in 0..8 {
        let scaled = ((raw_v[i] * 240 + sum_v / 2) / sum_v).clamp(12, 58) as u8;
        base_values[i] = scaled;
        running += i32::from(scaled);
    }
    // 精確配平至 240
    let mut diff = 240 - running;
    let mut idx_fix = 0usize;
    while diff != 0 {
        let slot = idx_fix % 8;
        if diff > 0 && base_values[slot] < 60 {
            base_values[slot] += 1;
            diff -= 1;
        } else if diff < 0 && base_values[slot] > 10 {
            base_values[slot] -= 1;
            diff += 1;
        }
        idx_fix += 1;
        if idx_fix > 512 {
            break;
        }
    }
    debug_assert_eq!(base_values.iter().map(|&v| u32::from(v)).sum::<u32>(), 240);

    // 逆鱗取權重最高或次高之敏感軸；軟肋取另一正交軸
    st = splitmix64(st.wrapping_add(7));
    let mut order_idx: [usize; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    order_idx.sort_by(|&a, &b| base_values[b].cmp(&base_values[a]).then_with(|| a.cmp(&b)));
    let taboo_pick = order_idx[(st % 2) as usize];
    let soft_pick = order_idx[2 + ((st >> 8) % 3) as usize];

    let concession_slope = next_centered_i8(&mut st, 24);
    let pragmatics = PragmaticVoice {
        verbosity: next_centered_i8(&mut st, 40),
        directness: next_centered_i8(&mut st, 40),
        metaphor_slot: (st % 4) as u8,
        hedge_habit: next_centered_i8(&mut st, 36),
        question_ratio: next_centered_i8(&mut st, 36),
    };

    InnatePersona {
        soul_seed,
        temperament: Temperament6D {
            ignition,
            slope,
            approach_avoid,
            damping,
            persistence,
            pain_tolerance,
        },
        caps_u,
        caps_v,
        base_values,
        taboo_axis: ValueAxis::from_idx(taboo_pick as u8),
        soft_spot_axis: ValueAxis::from_idx(soft_pick as u8),
        concession_slope,
        pragmatics,
    }
}

impl InnatePersona {
    /// 合成先天＋後天漂移＋永久性情大變後之當前有效價值權重
    pub fn effective_values(&self, drift: &CharacterDriftState) -> [i32; 8] {
        let mut out = [0i32; 8];
        for i in 0..8 {
            let mut v = i32::from(self.base_values[i]) + i32::from(drift.daily_value_drift[i]);
            for perm in &drift.permanent_shifts {
                v += i32::from(perm.value_delta[i]);
            }
            out[i] = v.clamp(5, 90);
        }
        out
    }

    pub fn top_value_axis(&self, drift: &CharacterDriftState) -> ValueAxis {
        let vals = self.effective_values(drift);
        let mut best_i = 0usize;
        for (i, &v) in vals.iter().enumerate().skip(1) {
            if v > vals[best_i] {
                best_i = i;
            }
        }
        ValueAxis::from_idx(best_i as u8)
    }

    /// 評估 16 維主觀處境下之 8 大反應傾向分數（純整數確定性計算）
    pub fn evaluate_tendencies(&self, sit: &Situation16D, drift: &CharacterDriftState) -> [i32; 8] {
        let mut lat = [0i32; 4];
        for r in 0..16 {
            let x = sit.v[r];
            if x != 0 {
                for k in 0..4 {
                    lat[k] += x * i32::from(self.caps_u[r][k]);
                }
            }
        }
        let eff_vals = self.effective_values(drift);
        let mut scores = [0i32; 8];
        for out in 0..8 {
            let mut perm_t = 0i32;
            for p in &drift.permanent_shifts {
                perm_t += i32::from(p.tendency_delta[out]);
            }
            let mut sc =
                (i32::from(drift.daily_tendency_bias[out]) + perm_t) * 48 + eff_vals[out] * 24;
            for k in 0..4 {
                sc += (lat[k] * i32::from(self.caps_v[k][out])) >> 4;
            }
            scores[out] = sc;
        }
        // 脾氣神經動力調變：點火門檻低者硬頂傾向加權，避害高者退避加權
        scores[ResponseTendency::Confront as usize] -= i32::from(self.temperament.ignition) * 12;
        scores[ResponseTendency::Withdraw as usize] -=
            i32::from(self.temperament.approach_avoid) * 12;
        scores[ResponseTendency::GrudgeLedger as usize] += i32::from(self.temperament.damping) * 12;
        scores
    }

    pub fn choose_dominant_tendency(
        &self,
        sit: &Situation16D,
        drift: &CharacterDriftState,
    ) -> ResponseTendency {
        let sc = self.evaluate_tendencies(sit, drift);
        let mut best = 0usize;
        for (i, &v) in sc.iter().enumerate().skip(1) {
            if v > sc[best] {
                best = i;
            }
        }
        ResponseTendency::from_idx(best)
    }

    /// 產生純世界內的身體姿態與神情微線索（不露出任何設定術語）
    pub fn body_posture_cue(
        &self,
        hunger: u8,
        fatigue: u8,
        anger: u8,
        moving_to: Option<&str>,
    ) -> String {
        if let Some(dst_name) = moving_to {
            if self.temperament.ignition < -15 {
                return format!("腳步急促正趕往{}", dst_name);
            }
            return format!("正順著街石走向{}", dst_name);
        }
        if anger >= 45 {
            if self.temperament.ignition < 0 {
                "指節微白、眉頭緊鎖".to_string()
            } else {
                "面色沉靜卻一言不發".to_string()
            }
        } else if fatigue >= 60 || hunger >= 60 {
            if self.temperament.pain_tolerance > 15 {
                "額角見汗仍沉肩幹活".to_string()
            } else {
                "氣息微促、不時揉著酸沉肩頸".to_string()
            }
        } else if self.temperament.approach_avoid > 20 {
            "眼神活絡打量往來行人".to_string()
        } else if self.temperament.persistence > 20 {
            "神色專注守著手邊活計".to_string()
        } else {
            "神態從容在鋪前忙碌".to_string()
        }
    }

    /// 對話前純 Rust 鎖死立場與零形容詞語用規則（封死 RLHF 順從與舞台劇小丑）
    pub fn arbitrate_dialogue_stance(
        &self,
        drift: &CharacterDriftState,
        role: &str,
        acquainted: bool,
        kept_pacts: u16,
        broken_pacts: u16,
        repeated_pressure_count: u8,
        anger: u8,
        utterance: &str,
    ) -> StanceVerdict {
        let asks_discount = utterance.contains("便宜")
            || utterance.contains("算少")
            || utterance.contains("賒")
            || utterance.contains("拜託")
            || utterance.contains("送我");
        let threatens = utterance.contains("滾")
            || utterance.contains("殺")
            || utterance.contains("砸")
            || utterance.contains("廢話");

        let sit = Situation16D::new(
            if threatens { 55 } else { 5 },
            0,
            if acquainted { 35 } else { -20 },
            i32::from(kept_pacts) * 20 - i32::from(broken_pacts) * 45,
            if asks_discount { -25 } else { 10 },
            15,
            10,
            25,
            20,
            if broken_pacts > 0 { 40 } else { 0 },
            0,
            if acquainted { 0 } else { 20 },
            i32::from(anger),
            0,
            20,
            20,
        );
        let tendency = self.choose_dominant_tendency(&sit, drift);
        let top_value = self.top_value_axis(drift);

        // 讓步曲線 × 施壓抗性記憶：同一招賣慘／施壓重複第 2 次以上，抗性陡增
        let pressure_penalty = i32::from(repeated_pressure_count) * 14;
        let effective_concession = i32::from(self.concession_slope) - pressure_penalty;

        let (stance, price_adjust_pct) = if broken_pacts > 0
            && (self.taboo_axis == ValueAxis::PactIntegrity
                || tendency == ResponseTendency::Confront)
        {
            (DialogueHardStance::RefuseColdly, 30)
        } else if threatens {
            if tendency == ResponseTendency::Withdraw {
                (DialogueHardStance::GuardedEvade, 15)
            } else {
                (DialogueHardStance::RefuseColdly, 25)
            }
        } else if asks_discount {
            if effective_concession >= 12 && acquainted && broken_pacts == 0 {
                (DialogueHardStance::GrudgingFavor, -5)
            } else if effective_concession <= -8 || repeated_pressure_count >= 2 {
                (DialogueHardStance::HaggleFirm, 10)
            } else {
                (DialogueHardStance::HaggleFirm, 0)
            }
        } else if kept_pacts >= 2 {
            (DialogueHardStance::GrudgingFavor, -5)
        } else if !acquainted && self.temperament.approach_avoid < -18 {
            (DialogueHardStance::GuardedEvade, 0)
        } else {
            (DialogueHardStance::CordialOpen, 0)
        };

        let len_rule = if self.pragmatics.verbosity < -12 {
            "每次回話不超過兩句短話，絕不多費口舌"
        } else if self.pragmatics.verbosity > 15 {
            "回話帶上眼前活計細節，約兩到三句"
        } else {
            "回話精煉自然，約一到兩句"
        };
        let direct_rule = if self.pragmatics.directness >= 0 {
            "有話直說、直接挑明條件，不講客套虛詞"
        } else {
            "說話留三分餘地，先看對方來意再亮底細"
        };
        let metaphor_rule = match self.pragmatics.metaphor_slot % 4 {
            0 => format!("偶爾自然帶出{}手邊器物或活計的實在比方", role),
            1 => "偶爾以海風潮汐或船期晴雨打比方".to_string(),
            2 => "習慣算清銅幣斤兩與前後得失".to_string(),
            _ => "只談眼前看得見摸得著的實事".to_string(),
        };
        let stance_rule = match stance {
            DialogueHardStance::CordialOpen => "平實應答，但不無故白送財物或替陌生人擔保",
            DialogueHardStance::HaggleFirm => {
                "報價與規矩一文不讓，絕不可答應降價、白送或無憑據賒帳"
            }
            DialogueHardStance::RefuseColdly => {
                "對方踩了你的底線，直接冷淡回絕送客，絕不可鬆口答應請求"
            }
            DialogueHardStance::GrudgingFavor => {
                "念在過往交情可給一句實在指點，但仍須守住手頭生計本錢"
            }
            DialogueHardStance::GuardedEvade => "對生面孔保持戒心，不透露私密家底與他人行蹤",
        };

        StanceVerdict {
            stance,
            dominant_tendency: tendency,
            top_value,
            price_adjust_pct,
            speech_rule_zh: format!(
                "{}；{}；{}；{}。內心最看重「{}」。",
                stance_rule,
                len_rule,
                direct_rule,
                metaphor_rule,
                top_value.in_world_motive_zh()
            ),
            posture_zh: self.body_posture_cue(0, 0, anger, None),
        }
    }
}

impl CharacterDriftState {
    /// 每夜睡眠整合：日常偏置向先天基線慢速回歸（防 DF 壓力螺旋），並檢查「性情大變三道硬閘」
    pub fn nightly_consolidate_and_check_shift(
        &mut self,
        innate: &InnatePersona,
        game_day: i64,
        unresolved_hardship: bool,
        top_event_opt: Option<(&str, ValueAxis, u8, &str)>,
    ) -> Option<String> {
        // 1. 日常慢速基線回歸（每 4 日微幅回縮 1 點，保持韌性不鎖死）
        if game_day % 4 == 0 {
            for d in &mut self.daily_value_drift {
                if *d > 0 {
                    *d -= 1;
                } else if *d < 0 {
                    *d += 1;
                }
            }
            for b in &mut self.daily_tendency_bias {
                if *b > 0 {
                    *b -= 1;
                } else if *b < 0 {
                    *b += 1;
                }
            }
        }

        let Some((ev_id, touched_axis, intensity, ev_summary)) = top_event_opt else {
            if unresolved_hardship
                && self.permanent_shifts.len() < 2
                && self.rumination_intensity >= 80
                && let Some(anchor_id) = self.rumination_event_id.clone()
            {
                self.rumination_nights = self.rumination_nights.saturating_add(1);
                if self.rumination_nights >= 3
                    && !self
                        .permanent_shifts
                        .iter()
                        .any(|s| s.anchor_event_id == anchor_id)
                {
                    let ax_i = usize::from(self.rumination_axis % 8);
                    let mut v_delta = [0i8; 8];
                    let mut t_delta = [0i8; 8];
                    v_delta[ax_i] = 12;
                    v_delta[ValueAxis::SurvivalSafety as usize] = 6;
                    t_delta[ResponseTendency::GrudgeLedger as usize] = 10;
                    t_delta[ResponseTendency::Confront as usize] = 6;
                    let lesson = "連夜輾轉難眠反芻舊創後，處世底線已徹底改寫".to_string();
                    self.permanent_shifts.push(PermanentCharacterShift {
                        anchor_event_id: anchor_id,
                        shifted_day: game_day,
                        summary_zh: lesson.clone(),
                        value_delta: v_delta,
                        tendency_delta: t_delta,
                    });
                    self.rumination_event_id = None;
                    self.rumination_nights = 0;
                    return Some(lesson);
                }
            } else {
                self.rumination_nights = self.rumination_nights.saturating_sub(1);
                if self.rumination_nights == 0 {
                    self.rumination_event_id = None;
                }
            }
            return None;
        };

        // 一般高顯著事件微調日常偏置（限幅 [-18, 18]）
        let ax_i = touched_axis as usize;
        if intensity >= 45 {
            self.daily_value_drift[ax_i] = (self.daily_value_drift[ax_i] + 2).clamp(-18, 18);
        }

        // 2. 性情大變三道硬閘（一生最多 2 次永久改寫基線）：
        // 閘一：觸及個人首位核心價值或踩破逆鱗；
        // 閘二：事件衝擊強度 >= 80；
        // 閘三：連續 >= 3 夜睡眠反芻同一創傷／大恩未消解。
        let top_ax = innate.top_value_axis(self);
        let touches_core = touched_axis == innate.taboo_axis || touched_axis == top_ax;
        if self.permanent_shifts.len() < 2 && touches_core && intensity >= 80 {
            if self.rumination_event_id.as_deref() == Some(ev_id) {
                self.rumination_nights = self.rumination_nights.saturating_add(1);
            } else {
                self.rumination_event_id = Some(ev_id.to_string());
                self.rumination_nights = 1;
                self.rumination_axis = touched_axis as u8;
                self.rumination_intensity = intensity;
            }

            if self.rumination_nights >= 3
                && !self
                    .permanent_shifts
                    .iter()
                    .any(|s| s.anchor_event_id == ev_id)
            {
                let mut v_delta = [0i8; 8];
                let mut t_delta = [0i8; 8];
                v_delta[ax_i] = 12;
                v_delta[ValueAxis::SurvivalSafety as usize] = 6;
                t_delta[ResponseTendency::GrudgeLedger as usize] = 10;
                t_delta[ResponseTendency::Confront as usize] = 6;
                let lesson = format!("自從親歷「{}」連夜難眠後，處世底線已徹底改寫", ev_summary);
                self.permanent_shifts.push(PermanentCharacterShift {
                    anchor_event_id: ev_id.to_string(),
                    shifted_day: game_day,
                    summary_zh: lesson.clone(),
                    value_delta: v_delta,
                    tendency_delta: t_delta,
                });
                self.rumination_event_id = None;
                self.rumination_nights = 0;
                return Some(lesson);
            }
        }
        None
    }
}

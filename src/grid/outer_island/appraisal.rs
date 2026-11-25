//! 單一認知仲裁瓶頸：事件因果評估（OCC × `Temperament6D` × 逆鱗軟肋 × 親疏）與定長 4 格心事槽 `[Option<FocusConcern>; 4]`。

use serde::{Deserialize, Serialize};

use crate::combat::wordplate::splitmix64;

use super::clock_and_lang::SolarClockSnapshot;
use super::desire_sim::{BellDecisionIndex, evaluate_personalized_desire};
use super::household_economy::find_town_tavern_and_keeper;
use super::mind::{
    ActiveIntention, CognitiveMind, LifeTurningPoint, MAX_TURNING_POINTS, tie_flags,
};
use super::persona::{InnatePersona, ValueAxis, derive_innate_persona};
use super::types::{IslandWorldDelta, WorldStaticDoc};
use super::world_weave_bio::settlement_name_zh;

pub const MAX_FOCUS_CONCERNS: usize = 4;

/// 四大具對象指向之核心情緒（與生理 `hunger`/`fatigue` 合組 `AffectTension` 六維）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimaryEmotion {
    Fear,
    Anger,
    Grief,
    Hope,
}

impl PrimaryEmotion {
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Fear => "憂懼",
            Self::Anger => "憤懣",
            Self::Grief => "悲戚",
            Self::Hope => "期盼",
        }
    }
}

/// 結構化心事成因（零自由字串，確保萬人常駐低記憶體與確定性查表）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConcernCause {
    LivelihoodShortfall, // 屋內斷糧或銅幣見底
    DebtDefaulted,       // 自身欠帳逾期未還
    DebtPressured,       // 遭催討欠款或他人拖欠己帳
    PactBetrayed,        // 遭毀諾背信或惡意施壓
    RivalEncounter,      // 當街撞見反目仇親或嫌隙對象
    KinshipRift,         // 至親／師門決裂傷痛
    KinshipWarmth,       // 蒙親友分食接濟或傾談寬慰
    PactHonored,         // 守信結清舊帳或互助履約
    ToolDamaged,         // 隨身生計器物磨損待修
    CrossTownYearning,   // 記掛外鎮久未通音訊之至親摯友
    CraftBreakthrough,   // 閉門琢磨器物順手或生計轉穩
}

impl ConcernCause {
    pub fn base_privacy(self) -> u8 {
        match self {
            Self::KinshipRift => 72,
            Self::PactBetrayed => 64,
            Self::DebtDefaulted => 62,
            Self::LivelihoodShortfall => 52,
            Self::RivalEncounter => 48,
            Self::DebtPressured => 42,
            Self::CrossTownYearning => 34,
            Self::ToolDamaged => 24,
            Self::KinshipWarmth => 28,
            Self::PactHonored => 22,
            Self::CraftBreakthrough => 18,
        }
    }

    pub fn default_axis(self) -> ValueAxis {
        match self {
            Self::LivelihoodShortfall => ValueAxis::SurvivalSafety,
            Self::DebtDefaulted | Self::DebtPressured | Self::PactBetrayed | Self::PactHonored => {
                ValueAxis::PactIntegrity
            }
            Self::RivalEncounter => ValueAxis::Autonomy,
            Self::KinshipRift | Self::KinshipWarmth | Self::CrossTownYearning => {
                ValueAxis::KinshipLoyalty
            }
            Self::ToolDamaged | Self::CraftBreakthrough => ValueAxis::CraftPride,
        }
    }

    pub fn is_irreversible_loss(self) -> bool {
        matches!(self, Self::KinshipRift | Self::PactBetrayed)
    }
}

/// 定長心事槽單格結構（唯一驅動快思維加權、慢思維入隊與雙表面投影之真實心緒來源）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusConcern {
    pub event_hash: u64,
    pub target_id: Option<String>,
    pub cause: ConcernCause,
    pub axis: ValueAxis,
    pub emotion: PrimaryEmotion,
    pub valence: i8,
    pub intensity: u8,
    pub born_bell: i64,
    #[serde(default)]
    pub last_boost_day: i64,
    pub ttl_bells: u16,
    pub privacy: u8,
}

/// `11436` 慢思考與高張力轉向產出之中文封閉方針列舉（避開英文字母過濾器並強制附帶期限）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStrategyKind {
    ClearDebt,         // 清償舊欠
    KinMutualAid,      // 親故互助
    ForageSupplement,  // 採集貼補
    AvoidRift,         // 避開嫌隙
    HoneCraft,         // 精研手藝
    SeekCrossTownNews, // 打聽音訊
}

impl GoalStrategyKind {
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::ClearDebt => "清償舊欠",
            Self::KinMutualAid => "親故互助",
            Self::ForageSupplement => "採集貼補",
            Self::AvoidRift => "避開嫌隙",
            Self::HoneCraft => "精研手藝",
            Self::SeekCrossTownNews => "打聽音訊",
        }
    }

    pub fn tag_slug(self) -> &'static str {
        match self {
            Self::ClearDebt => "clear_debt",
            Self::KinMutualAid => "kin_mutual_aid",
            Self::ForageSupplement => "forage_supplement",
            Self::AvoidRift => "avoid_rift",
            Self::HoneCraft => "hone_craft",
            Self::SeekCrossTownNews => "seek_crosstown_news",
        }
    }

    pub fn from_zh_text(s: &str) -> Option<Self> {
        if s.contains("清償") || s.contains("還債") || s.contains("舊欠") || s.contains("清帳")
        {
            Some(Self::ClearDebt)
        } else if s.contains("親故")
            || s.contains("互助")
            || s.contains("商議")
            || s.contains("家眷")
        {
            Some(Self::KinMutualAid)
        } else if s.contains("採集")
            || s.contains("貼補")
            || s.contains("山海")
            || s.contains("外快")
        {
            Some(Self::ForageSupplement)
        } else if s.contains("避開")
            || s.contains("嫌隙")
            || s.contains("繞行")
            || s.contains("遠離")
        {
            Some(Self::AvoidRift)
        } else if s.contains("精研")
            || s.contains("手藝")
            || s.contains("打磨")
            || s.contains("本行")
        {
            Some(Self::HoneCraft)
        } else if s.contains("打聽")
            || s.contains("音訊")
            || s.contains("外鎮")
            || s.contains("消息")
        {
            Some(Self::SeekCrossTownNews)
        } else {
            None
        }
    }
}

/// 附有效期限、起始狀態快照與對象錨點之中程行動方針
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalStrategy {
    pub kind: GoalStrategyKind,
    pub target_id: Option<String>,
    pub set_day: i64,
    pub expires_day: i64,
    #[serde(default)]
    pub initial_wear: u8,
    #[serde(default)]
    pub had_debt_at_set: bool,
    #[serde(default)]
    pub initial_forage_window: i64,
}

fn hash_concern_key(cause: ConcernCause, target_id: Option<&str>, salt: u64) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15 ^ u64::from(cause as u8) ^ salt;
    if let Some(tid) = target_id {
        for b in tid.as_bytes() {
            h = splitmix64(h ^ u64::from(*b));
        }
    }
    splitmix64(h)
}

impl CognitiveMind {
    /// 核心因果評估器：同一客觀事件依個人 `Temperament6D`、價值權重、逆鱗／軟肋與對人親疏折射為差異化情緒與心事
    pub fn appraise_event(
        &mut self,
        innate: &InnatePersona,
        cause: ConcernCause,
        target_id: Option<&str>,
        axis_override: Option<ValueAxis>,
        base_intensity: u8,
        abs_bell: i64,
    ) -> bool {
        let axis = axis_override.unwrap_or_else(|| cause.default_axis());
        let is_positive = matches!(
            cause,
            ConcernCause::KinshipWarmth
                | ConcernCause::PactHonored
                | ConcernCause::CraftBreakthrough
        );

        let base_val = i32::from(innate.base_values[axis as usize]);
        let mut mult_pct: i32 = 82 + (base_val * 4) / 3;
        if !is_positive && axis == innate.taboo_axis {
            mult_pct = (mult_pct * 175) / 100;
        } else if axis == innate.soft_spot_axis {
            mult_pct = (mult_pct * 142) / 100;
        }

        let mut warmth_val: i8 = 0;
        let mut tie_mask: u8 = 0;
        if let Some(tid) = target_id
            && let Some(soc) = self.ltm.social.get(tid)
        {
            warmth_val = soc.warmth;
            tie_mask = soc.tie_mask;
            if !is_positive {
                if soc.warmth >= 30 || (soc.tie_mask & tie_flags::BLOOD_KIN) != 0 {
                    mult_pct += 28;
                } else if soc.warmth <= -20 {
                    mult_pct += 18;
                }
            } else if soc.warmth >= 25 {
                mult_pct += 24;
            }
        }

        let emotion = if is_positive {
            PrimaryEmotion::Hope
        } else {
            match cause {
                ConcernCause::LivelihoodShortfall => PrimaryEmotion::Fear,
                ConcernCause::KinshipRift => PrimaryEmotion::Grief,
                ConcernCause::PactBetrayed => {
                    if warmth_val >= 32
                        || (tie_mask & (tie_flags::BLOOD_KIN | tie_flags::SWORN_COMRADE)) != 0
                    {
                        if innate.temperament.approach_avoid >= 12 {
                            PrimaryEmotion::Anger
                        } else {
                            PrimaryEmotion::Grief
                        }
                    } else if innate.temperament.approach_avoid >= -8 {
                        PrimaryEmotion::Anger
                    } else {
                        PrimaryEmotion::Fear
                    }
                }
                ConcernCause::DebtDefaulted => {
                    if innate.temperament.approach_avoid >= 18 {
                        PrimaryEmotion::Anger
                    } else {
                        PrimaryEmotion::Fear
                    }
                }
                ConcernCause::DebtPressured | ConcernCause::RivalEncounter => {
                    if i32::from(innate.temperament.approach_avoid)
                        - i32::from(innate.temperament.ignition) / 2
                        >= -4
                    {
                        PrimaryEmotion::Anger
                    } else {
                        PrimaryEmotion::Fear
                    }
                }
                ConcernCause::ToolDamaged => {
                    if innate.temperament.ignition < -10 {
                        PrimaryEmotion::Anger
                    } else {
                        PrimaryEmotion::Fear
                    }
                }
                ConcernCause::CrossTownYearning => PrimaryEmotion::Grief,
                ConcernCause::KinshipWarmth
                | ConcernCause::PactHonored
                | ConcernCause::CraftBreakthrough => PrimaryEmotion::Hope,
            }
        };

        let mut scaled = (i32::from(base_intensity) * mult_pct) / 100;
        if emotion == PrimaryEmotion::Grief {
            scaled -= i32::from(innate.temperament.pain_tolerance) / 5;
        }
        let min_gate = (24 + i32::from(innate.temperament.ignition) / 4).clamp(12, 42);
        if scaled < min_gate {
            return false;
        }

        let max_step = (42 + i32::from(innate.temperament.slope) / 3).clamp(22, 68);
        let final_intensity = scaled.min(max_step + 20).clamp(14, 92) as u8;
        let ttl_bells = (16 + i32::from(innate.temperament.persistence) / 4).clamp(8, 36) as u16;
        let valence: i8 = if is_positive {
            (final_intensity.min(90)) as i8
        } else {
            -(final_intensity.min(92) as i8)
        };
        let privacy = (i32::from(cause.base_privacy())
            - i32::from(innate.pragmatics.directness) / 5)
            .clamp(12, 90) as u8;
        let cur_day = abs_bell.div_euclid(12);
        let ev_hash = hash_concern_key(cause, target_id, cur_day.unsigned_abs());

        // L9 + R2-M1 修正：正向事件僅紓解「同一對象」或「同一價值軸／同類生計」的可逆負面心事，絕不因無關日常小確幸抹平全體負面心緒！
        if is_positive {
            for slot in &mut self.stm.focus_concerns {
                if let Some(c) = slot {
                    let same_target = target_id.is_some() && c.target_id.as_deref() == target_id;
                    let related_livelihood =
                        matches!(axis, ValueAxis::SurvivalSafety | ValueAxis::TangibleGain)
                            && c.cause == ConcernCause::LivelihoodShortfall;
                    if c.cause.is_irreversible_loss() {
                        if same_target {
                            c.intensity = c.intensity.saturating_sub(18);
                        }
                    } else if c.valence < 0 {
                        // R5-M1 修正：具特定對象之心事（如記掛外鎮親人、欠某人債）僅在 `same_target` 時紓解，不被鄰居閒聊同軸洗掉！
                        let drop = if same_target {
                            26
                        } else if c.target_id.is_none() && (c.axis == axis || related_livelihood) {
                            14
                        } else {
                            0
                        };
                        c.intensity = c.intensity.saturating_sub(drop);
                    }
                    if c.intensity < 10 {
                        *slot = None;
                    }
                }
            }
        }

        // M1 修正：同因合併設「每日最多強化 1 次」剎車且不刷滿 TTL，確保每響鐘阻尼衰減始終有效！
        let clean_tid = target_id.filter(|s| !s.is_empty()).map(str::to_string);
        for slot in &mut self.stm.focus_concerns {
            if let Some(existing) = slot
                && existing.cause == cause
                && existing.target_id == clean_tid
            {
                if cur_day > existing.last_boost_day {
                    let daily_gain = ((max_step / 3) as u8).clamp(6, 16);
                    existing.intensity = existing
                        .intensity
                        .saturating_add(daily_gain)
                        .max(final_intensity)
                        .min(92);
                    existing.ttl_bells = existing.ttl_bells.saturating_add(6).min(36);
                    existing.last_boost_day = cur_day;
                } else if final_intensity > existing.intensity {
                    existing.intensity = final_intensity;
                }
                existing.emotion = emotion;
                existing.valence = if is_positive {
                    existing.intensity.min(90) as i8
                } else {
                    -(existing.intensity.min(92) as i8)
                };
                existing.event_hash = ev_hash;
                self.recompute_affect_from_concerns();
                return true;
            }
        }

        let new_concern = FocusConcern {
            event_hash: ev_hash,
            target_id: clean_tid,
            cause,
            axis,
            emotion,
            valence,
            intensity: final_intensity,
            born_bell: abs_bell,
            last_boost_day: cur_day,
            ttl_bells,
            privacy,
        };

        if let Some(empty_slot) = self.stm.focus_concerns.iter_mut().find(|s| s.is_none()) {
            *empty_slot = Some(new_concern);
            self.recompute_affect_from_concerns();
            return true;
        }
        if let Some((weak_idx, weak_int)) = self
            .stm
            .focus_concerns
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|c| (i, c.intensity)))
            .min_by_key(|(_, inten)| *inten)
            && final_intensity > weak_int.saturating_add(4)
        {
            self.stm.focus_concerns[weak_idx] = Some(new_concern);
            self.recompute_affect_from_concerns();
            return true;
        }
        false
    }

    pub fn resolve_concerns_matching(
        &mut self,
        cause: ConcernCause,
        target_id: Option<&str>,
        reduce_by: u8,
    ) {
        let mut changed = false;
        for slot in &mut self.stm.focus_concerns {
            if let Some(c) = slot
                && c.cause == cause
                && (target_id.is_none() || c.target_id.as_deref() == target_id)
            {
                c.intensity = c.intensity.saturating_sub(reduce_by);
                if c.intensity < 10 {
                    *slot = None;
                }
                changed = true;
            }
        }
        if changed {
            self.recompute_affect_from_concerns();
        }
    }

    pub fn decay_concerns_and_sync_affect(&mut self, innate: &InnatePersona) {
        let base_decay = (4 - i32::from(innate.temperament.damping) / 22).clamp(1, 7) as u8;
        for slot in &mut self.stm.focus_concerns {
            if let Some(c) = slot {
                c.ttl_bells = c.ttl_bells.saturating_sub(1);
                let step = if c.cause.is_irreversible_loss() {
                    (base_decay / 2).max(1)
                } else {
                    base_decay
                };
                c.intensity = c.intensity.saturating_sub(step);
                if c.ttl_bells == 0 || c.intensity < 10 {
                    *slot = None;
                }
            }
        }
        self.recompute_affect_from_concerns();
    }

    pub fn recompute_affect_from_concerns(&mut self) {
        let mut buckets = [0u16; 4];
        for c in self.stm.focus_concerns.iter().flatten() {
            let idx = match c.emotion {
                PrimaryEmotion::Fear => 0,
                PrimaryEmotion::Anger => 1,
                PrimaryEmotion::Grief => 2,
                PrimaryEmotion::Hope => 3,
            };
            if buckets[idx] == 0 {
                buckets[idx] = u16::from(c.intensity);
            } else {
                buckets[idx] = buckets[idx].saturating_add(u16::from(c.intensity) / 2);
            }
        }
        self.stm.affect.fear = buckets[0].min(95) as u8;
        self.stm.affect.anger = buckets[1].min(95) as u8;
        self.stm.affect.grief = buckets[2].min(95) as u8;
        self.stm.affect.hope = buckets[3].min(95) as u8;
    }

    pub fn top_concern(&self) -> Option<&FocusConcern> {
        self.stm
            .focus_concerns
            .iter()
            .flatten()
            .max_by_key(|c| c.intensity)
    }

    pub fn active_concerns_sorted(&self) -> Vec<&FocusConcern> {
        let mut out: Vec<&FocusConcern> = self.stm.focus_concerns.iter().flatten().collect();
        out.sort_by_key(|c| std::cmp::Reverse((c.intensity, c.born_bell)));
        out
    }

    /// 掛載具明確到期日與起始狀態快照之中程行動方針（M6 + R2-M5 修正：綁定實質行為計數與語意對應成因）
    pub fn apply_goal_strategy_with_snapshot(
        &mut self,
        kind: GoalStrategyKind,
        target_id: Option<String>,
        game_day: i64,
        duration_days: i64,
        initial_wear: u8,
        had_debt_at_set: bool,
    ) {
        let span = duration_days.clamp(2, 6);
        self.stm.active_goal_tag = kind.tag_slug().to_string();
        self.stm.goal_strategy = Some(GoalStrategy {
            kind,
            target_id,
            set_day: game_day,
            expires_day: game_day + span,
            initial_wear,
            had_debt_at_set,
            initial_forage_window: self.stm.last_forage_window,
        });
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "explicit strategy termination state check"
    )]
    pub fn tick_goal_strategy_expiry(
        &mut self,
        game_day: i64,
        has_unsettled_or_written_off_debt: bool,
        larder_rations: u32,
        copper: i64,
        tool_wear: u8,
        innate: &InnatePersona,
        abs_bell: i64,
    ) {
        let (accomplished, expired, kind, tid_opt) = {
            let Some(ref strat) = self.stm.goal_strategy else {
                return;
            };
            let ok = match strat.kind {
                // R3-L1：唯有真正結清（settled）且無未清或逾限黑冊欠款，才算守信達成 ClearDebt！
                GoalStrategyKind::ClearDebt => {
                    strat.had_debt_at_set && !has_unsettled_or_written_off_debt && copper >= 4
                }
                GoalStrategyKind::ForageSupplement => {
                    self.stm.last_forage_window > strat.initial_forage_window
                        && larder_rations >= 4
                        && copper >= 8
                }
                GoalStrategyKind::HoneCraft => strat.initial_wear > 0 && tool_wear == 0,
                GoalStrategyKind::AvoidRift => {
                    game_day >= strat.set_day + 2 && self.stm.affect.anger < 25
                }
                GoalStrategyKind::KinMutualAid | GoalStrategyKind::SeekCrossTownNews => false,
            };
            (
                ok,
                game_day >= strat.expires_day,
                strat.kind,
                strat.target_id.clone(),
            )
        };
        if accomplished {
            self.stm.goal_strategy = None;
            self.stm.active_goal_tag.clear();
            match kind {
                GoalStrategyKind::ClearDebt => {
                    let _ = self.appraise_event(
                        innate,
                        ConcernCause::PactHonored,
                        tid_opt.as_deref(),
                        Some(ValueAxis::PactIntegrity),
                        42,
                        abs_bell,
                    );
                }
                GoalStrategyKind::ForageSupplement => {
                    let _ = self.appraise_event(
                        innate,
                        ConcernCause::PactHonored,
                        None,
                        Some(ValueAxis::TangibleGain),
                        42,
                        abs_bell,
                    );
                }
                GoalStrategyKind::KinMutualAid => {
                    let _ = self.appraise_event(
                        innate,
                        ConcernCause::KinshipWarmth,
                        tid_opt.as_deref(),
                        Some(ValueAxis::KinshipLoyalty),
                        42,
                        abs_bell,
                    );
                }
                GoalStrategyKind::HoneCraft => {
                    let _ = self.appraise_event(
                        innate,
                        ConcernCause::CraftBreakthrough,
                        None,
                        Some(ValueAxis::CraftPride),
                        42,
                        abs_bell,
                    );
                }
                GoalStrategyKind::AvoidRift | GoalStrategyKind::SeekCrossTownNews => {
                    // R4-L5 修正：避開嫌隙平息怒氣直接紓解嫌隙心事，絕不誤寫為「打磨器物」！
                    self.resolve_concerns_matching(
                        ConcernCause::RivalEncounter,
                        tid_opt.as_deref(),
                        24,
                    );
                }
            }
        } else if expired {
            self.stm.goal_strategy = None;
            self.stm.active_goal_tag.clear();
        }
    }

    /// M2 修正：嚴格區分「事件當事人」與「同場旁觀者」，絕不讓旁觀路人誤領當事人之催債或鍛紋突破心事！
    #[expect(
        clippy::too_many_arguments,
        reason = "sensory-to-appraisal bridge context"
    )]
    pub fn appraise_sensory_episode_auto(
        &mut self,
        innate: &InnatePersona,
        self_actor_id: &str,
        subject_id: &str,
        is_direct_participant: bool,
        event_id: &str,
        salience: u8,
        affect_charge: u8,
        touched_axis: ValueAxis,
        abs_bell: i64,
    ) {
        if affect_charge < 28 && salience < 40 {
            return;
        }
        let other_opt = if !subject_id.is_empty() && subject_id != self_actor_id {
            Some(subject_id)
        } else {
            None
        };
        if event_id.contains("engrave_rune") {
            if is_direct_participant {
                let _ = self.appraise_event(
                    innate,
                    ConcernCause::CraftBreakthrough,
                    None,
                    Some(ValueAxis::CraftPride),
                    46,
                    abs_bell,
                );
            }
            return;
        }
        if event_id.contains("debt_chase") || event_id.contains("confront_scene") {
            // 債主本人已在 settle_due_contracts 評估過；旁觀者僅在發動者為自身嫌隙對象時觸發 RivalEncounter
            if let Some(oid) = other_opt
                && self.ltm.social.get(oid).is_some_and(|s| s.warmth <= -20)
            {
                let _ = self.appraise_event(
                    innate,
                    ConcernCause::RivalEncounter,
                    Some(oid),
                    Some(touched_axis),
                    affect_charge,
                    abs_bell,
                );
            }
            return;
        }
        if let Some(oid) = other_opt
            && self.ltm.social.get(oid).is_some_and(|s| s.warmth <= -22)
        {
            let _ = self.appraise_event(
                innate,
                ConcernCause::RivalEncounter,
                Some(oid),
                Some(touched_axis),
                affect_charge.max(salience / 2),
                abs_bell,
            );
        }
    }

    /// 冪等寫入前史家門／街坊熟識關係（相容舊呼叫端）
    pub fn seed_historical_social_tie(
        &mut self,
        target_id: &str,
        target_name: &str,
        note_zh: &str,
    ) {
        let _ = self.fold_social_memory_from_episode(
            &format!("seed_tie:{target_id}"),
            -364,
            target_id,
            target_name,
            "",
            0,
            "",
            true,
            false,
            45,
            8,
            note_zh,
            "",
        );
    }

    pub fn push_turning_point(
        &mut self,
        anchor_event_id: &str,
        game_day: i64,
        lesson_zh: &str,
        new_goal_tag: &str,
    ) {
        let clipped: String = lesson_zh.trim().chars().take(88).collect();
        if self.ltm.turning_points.iter().any(|t| {
            t.anchor_event_id == anchor_event_id
                || ((game_day - t.game_day).abs() <= 91 && t.lesson_zh.trim() == clipped)
        }) {
            return;
        }
        if self.ltm.turning_points.len() >= MAX_TURNING_POINTS {
            let old = self.ltm.turning_points.remove(0);
            self.ltm.cold_outbox.push(old);
        }
        self.ltm.turning_points.push(LifeTurningPoint {
            anchor_event_id: anchor_event_id.to_string(),
            game_day,
            lesson_zh: clipped,
            new_goal_tag: new_goal_tag.to_string(),
        });
        if !new_goal_tag.is_empty() {
            self.stm.active_goal_tag = new_goal_tag.to_string();
        }
    }
}

const MIND_AWAKENED_MARKER: &str = "mind_awakened_v1";
const MIND_BIO_CLEAN_MARKER: &str = "mind_bio_clean_v1";
const PIT_RUMOR_SEEDED_MARKER: &str = "pit_rumor_seeded_v1";

/// 冪等一次性喚醒既有存檔居民之真實經歷心事與個人化意圖（100% 由本人 `ltm.social` 裂痕／毀諾／跨鎮親故經 `Temperament6D` 因果折射，零人工配額！）
pub fn awaken_lived_concerns_and_intentions_once(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    super::phase_and_vessels::ensure_chaos_rune_pool_seeded(delta);
    for a in delta.actors.values_mut() {
        if a.alive
            && a.setup_done
            && a.controller_id.is_none()
            && !a.actor_id.starts_with("pl:")
            && !a.explored_windows.contains(PIT_RUMOR_SEEDED_MARKER)
        {
            if a.role.contains("酒館")
                || a.role.contains("獵")
                || (a.race == "矮人" && a.role.contains("鐵"))
            {
                a.known_rumors.insert("pit_rumor".to_string());
            }
            a.explored_windows
                .insert(PIT_RUMOR_SEEDED_MARKER.to_string());
            a.mark_mutated();
        }
        if a.alive
            && a.setup_done
            && a.controller_id.is_none()
            && !a.actor_id.starts_with("pl:")
            && !a.explored_windows.contains(MIND_BIO_CLEAN_MARKER)
        {
            let mut seen = std::collections::BTreeSet::new();
            a.mind.ltm.turning_points.retain(|tp| {
                !tp.anchor_event_id.starts_with("rust_slow")
                    && !tp.lesson_zh.contains("生計連日受阻（手頭剩")
                    && seen.insert(tp.lesson_zh.trim().to_string())
            });
            let _ =
                super::world_weave_bio::refresh_actor_biography_if_needed(doc, a, clock.game_days);
            a.explored_windows.insert(MIND_BIO_CLEAN_MARKER.to_string());
            a.mark_mutated();
        }
    }

    let any_unawakened = delta.actors.values().any(|a| {
        a.alive
            && a.setup_done
            && a.controller_id.is_none()
            && !a.actor_id.starts_with("pl:")
            && !a.explored_windows.contains(MIND_AWAKENED_MARKER)
    });
    if !any_unawakened {
        return;
    }

    let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
    let mut cell_tav: std::collections::BTreeMap<u32, (Option<String>, Option<String>)> =
        std::collections::BTreeMap::new();
    for c in doc.cells.values() {
        if c.kind == "settlement" {
            cell_tav.insert(
                c.cell_id,
                find_town_tavern_and_keeper(doc, delta, c.cell_id),
            );
        }
    }
    let bell_idx = BellDecisionIndex::build(delta);
    let ids: Vec<String> = delta.actors.keys().cloned().collect();

    for aid in ids {
        let dec_opt = delta.actors.get(&aid).and_then(|a| {
            if !a.alive
                || !a.setup_done
                || a.controller_id.is_some()
                || aid.starts_with("pl:")
                || a.explored_windows.contains(MIND_AWAKENED_MARKER)
            {
                return None;
            }
            let hc = doc.nodes.get(&a.home_node).map_or(4616, |n| n.cell_id);
            let (t_opt, k_opt) = cell_tav
                .get(&hc)
                .map_or((None, None), |(t, k)| (t.as_deref(), k.as_deref()));
            let inn = derive_innate_persona(a.soul_seed.max(1));
            Some(evaluate_personalized_desire(
                doc,
                delta,
                &bell_idx,
                a,
                &inn,
                t_opt,
                k_opt,
                clock.bell_index.clamp(4, 8),
                abs_bell,
            ))
        });

        let Some(dec) = dec_opt else {
            continue;
        };
        let Some(a) = delta.actors.get_mut(&aid) else {
            continue;
        };
        a.explored_windows.insert(MIND_AWAKENED_MARKER.to_string());
        let innate = derive_innate_persona(a.soul_seed.max(1));
        let my_town = settlement_name_zh(doc, a);

        let rift_tid = a
            .mind
            .ltm
            .social
            .values()
            .filter(|s| s.is_kin() && s.warmth <= -30 && !s.target_id.starts_with("pl:"))
            .min_by_key(|s| s.warmth)
            .map(|s| s.target_id.clone());
        let betray_tid = a
            .mind
            .ltm
            .social
            .values()
            .filter(|s| s.broken_pacts > 0 && s.warmth <= -24 && !s.target_id.starts_with("pl:"))
            .min_by_key(|s| s.warmth)
            .map(|s| s.target_id.clone());
        let yearn_tid = a
            .mind
            .ltm
            .social
            .values()
            .filter(|s| {
                !s.cohabiting
                    && !s.target_id.starts_with("pl:")
                    && !s.target_region_zh.is_empty()
                    && s.target_region_zh != my_town
                    && s.warmth >= 42
            })
            .max_by_key(|s| s.warmth)
            .map(|s| s.target_id.clone());

        if let Some(tid) = rift_tid {
            let _ = a.mind.appraise_event(
                &innate,
                ConcernCause::KinshipRift,
                Some(&tid),
                None,
                40,
                abs_bell,
            );
        }
        if let Some(tid) = betray_tid {
            let _ = a.mind.appraise_event(
                &innate,
                ConcernCause::PactBetrayed,
                Some(&tid),
                None,
                40,
                abs_bell,
            );
        }
        if let Some(tid) = yearn_tid {
            let _ = a.mind.appraise_event(
                &innate,
                ConcernCause::CrossTownYearning,
                Some(&tid),
                None,
                38,
                abs_bell,
            );
        }
        if a.bound_weapon.wear_stage >= 2 {
            let _ =
                a.mind
                    .appraise_event(&innate, ConcernCause::ToolDamaged, None, None, 38, abs_bell);
        }

        if a.mind.stm.intentions.is_empty()
            || a.mind
                .stm
                .intentions
                .back()
                .is_some_and(|it| it.reason_zh.contains("守緊本行手藝"))
        {
            if a.mind.stm.intentions.len() >= 4 {
                a.mind.stm.intentions.pop_front();
            }
            a.mind.stm.intentions.push_back(ActiveIntention {
                kind: dec.kind.to_string(),
                target_node: dec.target_node,
                reason_zh: dec.reason_zh,
                created_bell: abs_bell,
            });
        }
        a.mark_mutated();
    }
}

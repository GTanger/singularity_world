//! 玩家與全體居民 100% 同構之認知大腦（感官五道閘 → 短期記憶 STM → 睡眠整合 → 三大長期記憶 LTM）。
//!
//! 嚴守正典 §二：
//! - 語言隔閡對全體居民與玩家一視同仁生效（聽不懂只記「見人低語，聽不懂內容」）。
//! - 睡眠整合遺忘 95% 平庸日常，將高張力事件依個人脾氣著色沉澱進情節 LTM，連續受阻 >=3 日升格慢思考。
//! - 每位居民皆擁有獨立的個人世界探索迷霧（`known_nodes` / `known_cells`），無全知視角。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::clock_and_lang::{language_name_zh, mask_speech_by_proficiency};
use super::persona::{CharacterDriftState, InnatePersona, ValueAxis};

pub const MAX_STM_INTENTIONS: usize = 4;
pub const MAX_STM_FOCUS: usize = 7;
pub const MAX_DAY_EPISODES: usize = 64;
pub const MAX_RECENT_TALKS: usize = 4;
pub const MAX_HOT_EPISODIC_LTM: usize = 256;
pub const MAX_TURNING_POINTS: usize = 16;
pub const MAX_SOCIAL_JUDGMENTS: usize = 64;

/// 視聽公開度三級（封死「同節點即偷聽私語」漏洞）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPublicity {
    PublicLoud,   // 當街喧嘩、公開交鋒、高聲叫賣
    TableWhisper, // 同桌／當面低語（旁人須通曉該語言 >=60 方能聽見片段）
    Covert,       // 暗中塞錢／撬鎖（預設旁人不知情）
}

/// 記憶來源證據三態（親眼目睹 ≠ 聽本人自吹 ≠ 酒館轉述）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceTier {
    DirectWitness,    // 親眼見其動手
    ClaimedBySpeaker, // 聽當事人自稱或持戰利品炫耀（可被真目擊者拆穿）
    TavernHearsay,    // 酒館／班車轉述聽聞
}

impl EvidenceTier {
    pub fn tag_zh(self) -> &'static str {
        match self {
            Self::DirectWitness => "親眼所見",
            Self::ClaimedBySpeaker => "聽本人自稱",
            Self::TavernHearsay => "街坊轉述",
        }
    }
}

/// 當下行動意圖（STM 意圖棧項目）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveIntention {
    pub kind: String, // "walk_work" | "buy_meal" | "tavern_rest" | "go_home_sleep" | "seek_material" | "patrol"
    pub target_node: String, // 目標空間節點（必須在本人 known_nodes 內！）
    pub reason_zh: String,
    pub created_bell: i64,
}

/// 六維即時生理與情緒張力（0..=100）
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AffectTension {
    pub hunger: u8,
    pub fatigue: u8,
    pub fear: u8,
    pub anger: u8,
    pub grief: u8,
    pub hope: u8,
}

/// 雙向當面交談餘溫紀錄（對玩家與對其他居民一視同仁，持久化不失憶）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TalkTurnRecord {
    pub partner_id: String,
    pub partner_name: String,
    pub game_day: i64,
    pub bell_label: String,
    pub partner_said: String,
    pub self_replied: String,
}

/// 當日原始感官事件（STM 環形緩衝）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensoryEpisode {
    pub event_id: String,
    pub game_day: i64,
    pub bell_idx: u8,
    pub node_id: String,
    pub actor_id: String,
    pub actor_name: String,
    pub tier: EvidenceTier,
    pub salience: u8,      // 0..=100
    pub affect_charge: u8, // 0..=100
    pub touched_axis: u8,  // ValueAxis index
    pub summary_zh: String,
}

/// 短期記憶／工作記憶（STM —— 清醒當下腦容量）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortTermMemory {
    pub intentions: VecDeque<ActiveIntention>,
    pub working_focus: Vec<String>,
    pub day_episodes: VecDeque<SensoryEpisode>,
    pub affect: AffectTension,
    pub recent_talks: VecDeque<TalkTurnRecord>,
    pub active_goal_tag: String,
    pub consecutive_blocked_days: u8,
    pub today_goal_progressed: bool,
    pub last_sleep_day: i64,
}

/// 情節長期記憶條目（Episodic LTM，只存個人主觀感知投影，禁回讀客觀真相）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubjectiveEpisodeLtm {
    pub event_id: String,
    pub origin_day: i64,
    pub node_id: String,
    pub subject_actor_id: String,
    pub subject_name: String,
    pub tier: EvidenceTier,
    pub credence: u8,    // 0..=100 可信度
    pub salience: u8,    // 初始顯著性
    pub rehearsals: u16, // 重溫／提起次數（抵抗艾賓浩斯遺忘）
    pub subjective_zh: String,
}

impl SubjectiveEpisodeLtm {
    /// 艾賓浩斯保留權重（重溫次數與顯著性越高越難遺忘）
    pub fn retention_score(&self, current_day: i64) -> i64 {
        let age = (current_day - self.origin_day).max(0);
        let base = i64::from(self.salience) * 4 + i64::from(self.rehearsals.min(30)) * 18;
        base - age * 2
    }
}

/// 人生轉折微敘事槽（16 格熱槽，Rust 睡眠規則與 `11436` 慢思考共用同一寫入契約）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeTurningPoint {
    pub anchor_event_id: String,
    pub game_day: i64,
    pub lesson_zh: String,
    pub new_goal_tag: String,
}

/// 對認識對象之行為證據紀錄（Semantic & Social LTM，嚴禁抽象好感度條！）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SocialEvidenceSummary {
    pub target_id: String,
    pub target_name: String,
    pub acquainted: bool,
    pub trade_count: u32,
    pub kept_pacts: u16,
    pub broken_pacts: u16,
    pub helped_in_crisis: u16,
    pub pressure_tactic_count: u8, // 記住對方重複賣慘／施壓次數以提高抗性
    pub last_seen_day: i64,
    pub notable_note_zh: String,
}

/// 長期記憶三大分支（Episodic + Semantic/MapFog + Procedural）
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LongTermMemory {
    pub episodic: Vec<SubjectiveEpisodeLtm>,
    pub turning_points: Vec<LifeTurningPoint>,
    pub cold_outbox: Vec<LifeTurningPoint>,
    pub known_nodes: BTreeSet<String>,
    pub known_cells: BTreeSet<u32>,
    pub price_impressions: BTreeMap<String, (i64, i64)>, // good_slug -> (price_copper, observed_day)
    pub social: BTreeMap<String, SocialEvidenceSummary>,
    pub emergent_phase_name: Option<String>,
}

/// 玩家與全體居民 100% 共用之認知大腦結構
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CognitiveMind {
    pub stm: ShortTermMemory,
    pub ltm: LongTermMemory,
    pub pending_slow_think_score: u32,
    pub slow_think_wait_bells: u16,
}

impl CognitiveMind {
    /// 初始化個人起始已知地圖迷霧（僅知曉自身住處、工作點與所屬聚落公共街區，絕不天生全知全島！）
    pub fn seed_local_map_knowledge<'a>(
        &mut self,
        home_node: &str,
        work_node: &str,
        home_cell: u32,
        local_town_nodes: impl Iterator<Item = &'a str>,
    ) {
        self.ltm.known_cells.insert(home_cell);
        self.ltm.known_nodes.insert(home_node.to_string());
        self.ltm.known_nodes.insert(work_node.to_string());
        for nid in local_town_nodes {
            self.ltm.known_nodes.insert(nid.to_string());
        }
    }

    /// 踏足新節點時同步更新個人語義地圖迷霧（玩家與居民完全同一函式）
    pub fn record_footstep(&mut self, node_id: &str, cell_id: u32) {
        self.ltm.known_nodes.insert(node_id.to_string());
        self.ltm.known_cells.insert(cell_id);
    }

    /// 感官注意五道閘門：過濾同節點事件，套用四族語言遮罩後寫入短期記憶 STM
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit 5-gate sensory filter inputs"
    )]
    pub fn ingest_sensory_event(
        &mut self,
        listener_langs: &BTreeMap<String, u8>,
        publicity: EventPublicity,
        spoken_lang_opt: Option<&str>,
        is_direct_participant: bool,
        event_id: &str,
        game_day: i64,
        bell_idx: u8,
        node_id: &str,
        actor_id: &str,
        actor_name: &str,
        tier: EvidenceTier,
        salience: u8,
        affect_charge: u8,
        touched_axis: ValueAxis,
        raw_content_zh: &str,
    ) -> bool {
        if publicity == EventPublicity::Covert && !is_direct_participant {
            return false;
        }

        let perceived_text = if let Some(lang_code) = spoken_lang_opt {
            let prof = if lang_code == "harbor_trade" {
                listener_langs.get(lang_code).copied().unwrap_or(80)
            } else {
                listener_langs.get(lang_code).copied().unwrap_or(0)
            };
            if publicity == EventPublicity::TableWhisper && !is_direct_participant && prof < 60 {
                format!(
                    "見{}壓低嗓音以{}交談，但聽不懂內容",
                    actor_name,
                    language_name_zh(lang_code)
                )
            } else if prof < 80 && !is_direct_participant {
                let (masked, _) =
                    mask_speech_by_proficiency(actor_name, lang_code, prof, raw_content_zh);
                masked
            } else {
                raw_content_zh.to_string()
            }
        } else {
            raw_content_zh.to_string()
        };

        // 顯著性過低且非當事人之瑣碎背景音直接忽略
        if !is_direct_participant && salience < 25 {
            return false;
        }

        // 同日同一 event_id 不重複灌爆 STM
        if self.stm.day_episodes.iter().any(|e| e.event_id == event_id) {
            return false;
        }

        // 若環滿且最舊一筆屬生死級大事（salience >= 80），覆寫前先保護歸檔進 LTM
        if self.stm.day_episodes.len() >= MAX_DAY_EPISODES
            && let Some(evicted) = self.stm.day_episodes.pop_front()
            && evicted.salience >= 80
        {
            self.promote_episode_to_ltm(&evicted, None);
        }

        self.stm.day_episodes.push_back(SensoryEpisode {
            event_id: event_id.to_string(),
            game_day,
            bell_idx,
            node_id: node_id.to_string(),
            actor_id: actor_id.to_string(),
            actor_name: actor_name.to_string(),
            tier,
            salience,
            affect_charge,
            touched_axis: touched_axis as u8,
            summary_zh: perceived_text,
        });
        true
    }

    /// 記錄雙向對話餘溫與人際識名證據（對玩家與對居民完全對等）
    pub fn record_talk_exchange(
        &mut self,
        partner_id: &str,
        partner_name: &str,
        game_day: i64,
        bell_label: &str,
        partner_said: &str,
        self_replied: &str,
        used_pressure_tactic: bool,
    ) {
        if self.stm.recent_talks.len() >= MAX_RECENT_TALKS {
            self.stm.recent_talks.pop_front();
        }
        self.stm.recent_talks.push_back(TalkTurnRecord {
            partner_id: partner_id.to_string(),
            partner_name: partner_name.to_string(),
            game_day,
            bell_label: bell_label.to_string(),
            partner_said: partner_said.chars().take(64).collect(),
            self_replied: self_replied.chars().take(84).collect(),
        });

        self.ensure_social_slot_capacity(partner_id);
        let entry = self
            .ltm
            .social
            .entry(partner_id.to_string())
            .or_insert_with(|| SocialEvidenceSummary {
                target_id: partner_id.to_string(),
                target_name: partner_name.to_string(),
                ..SocialEvidenceSummary::default()
            });
        entry.target_name = partner_name.to_string();
        entry.acquainted = true;
        entry.last_seen_day = game_day;
        if used_pressure_tactic {
            entry.pressure_tactic_count = entry.pressure_tactic_count.saturating_add(1);
        }
    }

    pub fn record_pact_or_trade_evidence(
        &mut self,
        partner_id: &str,
        partner_name: &str,
        game_day: i64,
        kept: bool,
        helped_crisis: bool,
        note_zh: &str,
    ) {
        self.ensure_social_slot_capacity(partner_id);
        let entry = self
            .ltm
            .social
            .entry(partner_id.to_string())
            .or_insert_with(|| SocialEvidenceSummary {
                target_id: partner_id.to_string(),
                target_name: partner_name.to_string(),
                ..SocialEvidenceSummary::default()
            });
        entry.target_name = partner_name.to_string();
        entry.acquainted = true;
        entry.trade_count = entry.trade_count.saturating_add(1);
        entry.last_seen_day = game_day;
        if kept {
            entry.kept_pacts = entry.kept_pacts.saturating_add(1);
        } else {
            entry.broken_pacts = entry.broken_pacts.saturating_add(1);
        }
        if helped_crisis {
            entry.helped_in_crisis = entry.helped_in_crisis.saturating_add(1);
        }
        if !note_zh.is_empty() {
            entry.notable_note_zh = note_zh.chars().take(48).collect();
        }
    }

    fn ensure_social_slot_capacity(&mut self, incoming_id: &str) {
        if self.ltm.social.contains_key(incoming_id) || self.ltm.social.len() < MAX_SOCIAL_JUDGMENTS
        {
            return;
        }
        // 淘汰無破約、無救命恩情且最久未見之過客
        if let Some(evict_key) = self
            .ltm
            .social
            .iter()
            .filter(|(_, s)| s.broken_pacts == 0 && s.helped_in_crisis == 0)
            .min_by_key(|(_, s)| (s.kept_pacts, s.last_seen_day))
            .map(|(k, _)| k.clone())
        {
            self.ltm.social.remove(&evict_key);
        }
    }

    /// 依個人脾氣與個性著色後沉澱單筆事件進入情節 LTM（去重並強化重溫次數）
    pub fn promote_episode_to_ltm(
        &mut self,
        ep: &SensoryEpisode,
        innate_opt: Option<&InnatePersona>,
    ) {
        if let Some(existing) = self
            .ltm
            .episodic
            .iter_mut()
            .find(|x| x.event_id == ep.event_id)
        {
            existing.rehearsals = existing.rehearsals.saturating_add(1);
            if ep.tier == EvidenceTier::DirectWitness {
                existing.tier = EvidenceTier::DirectWitness;
                existing.credence = 100;
            } else if existing.tier != EvidenceTier::DirectWitness {
                existing.credence = (existing.credence.saturating_add(6)).min(85);
            }
            return;
        }

        // 記憶編碼著色：避害高者凸顯地點風險，阻尼高者凸顯涉事人名與得失
        let colored_zh = if let Some(inn) = innate_opt {
            if inn.temperament.approach_avoid < -20 {
                format!("{}（暗記此處凶險需留心）", ep.summary_zh)
            } else if inn.temperament.damping > 20 {
                format!("{}（牢記{}當時舉動）", ep.summary_zh, ep.actor_name)
            } else {
                ep.summary_zh.clone()
            }
        } else {
            ep.summary_zh.clone()
        };

        let init_credence = match ep.tier {
            EvidenceTier::DirectWitness => 100,
            EvidenceTier::ClaimedBySpeaker => 62,
            EvidenceTier::TavernHearsay => 50,
        };

        if self.ltm.episodic.len() >= MAX_HOT_EPISODIC_LTM
            && let Some((min_idx, _)) = self
                .ltm
                .episodic
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.retention_score(ep.game_day))
        {
            self.ltm.episodic.swap_remove(min_idx);
        }

        self.ltm.episodic.push(SubjectiveEpisodeLtm {
            event_id: ep.event_id.clone(),
            origin_day: ep.game_day,
            node_id: ep.node_id.clone(),
            subject_actor_id: ep.actor_id.clone(),
            subject_name: ep.actor_name.clone(),
            tier: ep.tier,
            credence: init_credence,
            salience: ep.salience,
            rehearsals: 1,
            subjective_zh: colored_zh,
        });
    }

    /// 夜間睡眠記憶整合（純 Rust 為全體居民與安寢玩家一視同仁執行！）
    pub fn run_sleep_consolidation(
        &mut self,
        innate: &InnatePersona,
        drift: &mut CharacterDriftState,
        game_day: i64,
        copper_shortfall: bool,
    ) {
        if self.stm.last_sleep_day >= game_day {
            return;
        }
        self.stm.last_sleep_day = game_day;

        // 1. 挑出當日顯著性×情緒張力最高之前 3 件事沉澱進情節 LTM，其餘平庸瑣事遺忘清空
        let mut episodes: Vec<SensoryEpisode> = self.stm.day_episodes.drain(..).collect();
        episodes.sort_by(|a, b| {
            let sa = u16::from(a.salience) + u16::from(a.affect_charge);
            let sb = u16::from(b.salience) + u16::from(b.affect_charge);
            sb.cmp(&sa)
        });

        let mut top_for_drift: Option<(String, ValueAxis, u8, String)> = None;
        for (idx, ep) in episodes.iter().take(3).enumerate() {
            if ep.salience >= 35 || ep.affect_charge >= 35 {
                self.promote_episode_to_ltm(ep, Some(innate));
                if idx == 0 {
                    top_for_drift = Some((
                        ep.event_id.clone(),
                        ValueAxis::from_idx(ep.touched_axis),
                        ep.salience.max(ep.affect_charge),
                        ep.summary_zh.clone(),
                    ));
                }
            }
        }

        // 2. 後天性格漂移與「性情大變三道硬閘」檢查
        let unresolved_hardship = !self.stm.today_goal_progressed || copper_shortfall;
        let shift_ref = top_for_drift
            .as_ref()
            .map(|(id, ax, inten, sum)| (id.as_str(), *ax, *inten, sum.as_str()));
        if let Some(shift_lesson) = drift.nightly_consolidate_and_check_shift(
            innate,
            game_day,
            unresolved_hardship,
            shift_ref,
        ) {
            let anchor = top_for_drift
                .as_ref()
                .map_or_else(|| format!("day:{game_day}"), |(id, _, _, _)| id.clone());
            self.push_turning_point(&anchor, game_day, &shift_lesson, "guarded_shift");
        }

        // 3. 檢查跨日目標受阻與升格 `11436` 慢思考優先分（零玩家距離偏置！）
        if !self.stm.today_goal_progressed || copper_shortfall {
            self.stm.consecutive_blocked_days = self.stm.consecutive_blocked_days.saturating_add(1);
        } else {
            self.stm.consecutive_blocked_days = 0;
        }
        self.stm.today_goal_progressed = false;

        let max_surprise = top_for_drift
            .as_ref()
            .map_or(0u32, |(_, _, inten, _)| u32::from(*inten));
        if self.stm.consecutive_blocked_days >= 3 || max_surprise >= 75 {
            let stakes = if copper_shortfall { 3u32 } else { 2u32 };
            let blocked = u32::from(self.stm.consecutive_blocked_days.max(1));
            let age_bonus = u32::from(self.slow_think_wait_bells / 4 + 1);
            self.pending_slow_think_score = (max_surprise.max(40)) * stakes * blocked * age_bonus;
        }

        // 睡眠舒緩疲勞與日常怒氣（依個人消退阻尼保留餘緒）
        self.stm.affect.fatigue = self.stm.affect.fatigue.saturating_sub(55);
        let anger_drop = if innate.temperament.damping > 20 {
            15
        } else {
            35
        };
        self.stm.affect.anger = self.stm.affect.anger.saturating_sub(anger_drop);
        self.stm.working_focus.clear();
    }

    pub fn push_turning_point(
        &mut self,
        anchor_event_id: &str,
        game_day: i64,
        lesson_zh: &str,
        new_goal_tag: &str,
    ) {
        if self.ltm.turning_points.len() >= MAX_TURNING_POINTS {
            let old = self.ltm.turning_points.remove(0);
            self.ltm.cold_outbox.push(old);
        }
        self.ltm.turning_points.push(LifeTurningPoint {
            anchor_event_id: anchor_event_id.to_string(),
            game_day,
            lesson_zh: lesson_zh.chars().take(90).collect(),
            new_goal_tag: new_goal_tag.to_string(),
        });
        if !new_goal_tag.is_empty() {
            self.stm.active_goal_tag = new_goal_tag.to_string();
        }
    }
}

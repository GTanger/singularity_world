//! 玩家與全體居民 100% 同構之認知大腦（感官五道閘 → 短期記憶 STM → 睡眠整合 → 三大長期記憶 LTM）。
//!
//! 嚴守正典 §二：
//! - 語言隔閡對全體居民與玩家一視同仁生效（聽不懂只記「見人低語，聽不懂內容」）。
//! - 睡眠整合遺忘 95% 平庸日常，將高張力事件依個人脾氣著色沉澱進情節 LTM，連續受阻 >=3 日升格慢思考。
//! - 每位居民皆擁有獨立的個人世界探索迷霧（`known_nodes` / `known_cells`），無全知視角。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::appraisal::{FocusConcern, GoalStrategy};
use super::clock_and_lang::{language_name_zh, mask_speech_by_proficiency};
use super::persona::{CharacterDriftState, InnatePersona, ValueAxis};

pub const MAX_STM_INTENTIONS: usize = 4;
pub const MAX_STM_FOCUS: usize = 7;
pub const MAX_DAY_EPISODES: usize = 64;
pub const MAX_RECENT_TALKS: usize = 4;
pub const MAX_HOT_EPISODIC_LTM: usize = 256;
pub const MAX_TURNING_POINTS: usize = 16;
pub const MAX_SOCIAL_JUDGMENTS: usize = 64;
pub const MAX_GENESIS_SOCIAL_SLOTS: usize = 40;
pub const MAX_GENESIS_TOTAL_SLOTS: usize = 48;
pub const MAX_RECOGNIZED_FACES: usize = 256;

/// 多重名分紐帶位元旗標（支援重疊名分，如「既是同門師兄弟又是姻親」；與實際親疏冷暖正交獨立）
pub mod tie_flags {
    pub const BLOOD_KIN: u8 = 1 << 0;
    pub const IN_LAW: u8 = 1 << 1;
    pub const MASTER_DISCIPLE: u8 = 1 << 2;
    pub const PEER_COHORT: u8 = 1 << 3;
    pub const SWORN_COMRADE: u8 = 1 << 4;
    pub const TRADE_BROKER: u8 = 1 << 5;

    pub fn priority_rank(mask: u8) -> u8 {
        for (bit, rk) in [
            (BLOOD_KIN, 6),
            (IN_LAW, 5),
            (MASTER_DISCIPLE, 4),
            (SWORN_COMRADE, 3),
            (PEER_COHORT, 2),
            (TRADE_BROKER, 1),
        ] {
            if (mask & bit) != 0 {
                return rk;
            }
        }
        0
    }
}

/// 視聽公開度三級（封死「同節點即偷聽私語」漏洞）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventPublicity {
    PublicLoud,   // 當街喧嘩、公開交鋒、高聲叫賣
    TableWhisper, // 同桌／當面低語（旁人須通曉該語言 >=60 方能聽見片段）
    Covert,       // 暗中塞錢／撬鎖（預設旁人不知情）
}

/// 記憶來源證據四態（親眼目睹 ≠ 家傳口述 ≠ 聽本人自吹 ≠ 街坊轉述）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceTier {
    DirectWitness,    // 親眼見其動手
    FamilyOral,       // 屋簷下長輩／師父口傳家訓（主觀深信度高，客觀保真度隨世代衰減）
    ClaimedBySpeaker, // 聽當事人自稱或持戰利品炫耀（可被真目擊者拆穿）
    TavernHearsay,    // 酒館／班車轉述聽聞
}

impl EvidenceTier {
    pub fn tag_zh(self) -> &'static str {
        match self {
            Self::DirectWitness => "親眼所見",
            Self::FamilyOral => "家傳口述",
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub working_focus: Vec<String>,
    #[serde(default)]
    pub focus_concerns: [Option<FocusConcern>; 4],
    #[serde(default)]
    pub goal_strategy: Option<GoalStrategy>,
    #[serde(default)]
    pub last_slow_think_day: i64,
    #[serde(default)]
    pub last_forage_window: i64,
    #[serde(default)]
    pub last_self_confront_stamp: i64,
    #[serde(default)]
    pub last_detour_bell: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_trade_offer: Option<super::phase_and_vessels::PeerTradeOffer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_spar_from: Option<(String, String, i64)>,
    pub day_episodes: VecDeque<SensoryEpisode>,
    pub affect: AffectTension,
    pub recent_talks: VecDeque<TalkTurnRecord>,
    pub active_goal_tag: String,
    pub consecutive_blocked_days: u8,
    pub today_goal_progressed: bool,
    pub last_sleep_day: i64,
}

fn default_full_fidelity() -> u8 {
    100
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
    pub credence: u8, // 0..=100 主觀深信度（Conviction，家訓複誦可極高）
    #[serde(default = "default_full_fidelity")]
    pub fidelity: u8, // 0..=100 客觀保真度（Fidelity，僅供系統內部稽核，角色不可見）
    pub salience: u8, // 初始顯著性
    pub rehearsals: u16, // 重溫／提起次數（抵抗艾賓浩斯遺忘）
    pub subjective_zh: String,
}

impl SubjectiveEpisodeLtm {
    /// 艾賓浩斯保留權重（重溫次數與顯著性越高越難遺忘；前史負數 origin_day 按溫和對數折算免被洗空）
    pub fn retention_score(&self, current_day: i64) -> i64 {
        let age = (current_day - self.origin_day).max(0);
        let age_penalty = if age <= 60 {
            age * 2
        } else {
            120 + ((age - 60) / 30).min(160)
        };
        let base = i64::from(self.salience) * 4 + i64::from(self.rehearsals.min(30)) * 18;
        base - age_penalty
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

fn default_social_familiarity() -> u8 {
    35
}

/// 對他人的主觀記憶印象槽（關係即是記憶的一部分：名分 `tie_mask` 與熟稔度 `familiarity`、心頭冷暖 `warmth` 正交分離！）
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
    pub tie_mask: u8,
    pub tie_label_zh: String,
    pub target_region_zh: String,
    #[serde(default = "default_social_familiarity")]
    pub familiarity: u8, // 0..=100 往來熟稔度（常來往高，長年斷訊低）
    pub warmth: i8, // -100..=100 心頭冷暖（正＝親厚信賴，0＝平淡，負＝疏遠嫌隙）
    pub folded_events: Vec<String>,
    pub secret_hook_zh: String,
    #[serde(default)]
    pub last_fam_gain_day: i64,
    #[serde(default)]
    pub last_confront_day: i64,
    pub from_genesis: bool,
    pub cohabiting: bool,
}

impl SocialEvidenceSummary {
    pub fn is_kin(&self) -> bool {
        (self.tie_mask & (tie_flags::BLOOD_KIN | tie_flags::IN_LAW)) != 0
    }

    /// 轉譯為去數值、符合世界內體感的「往來親疏 × 心頭冷暖」雙軸描述
    pub fn closeness_warmth_label_zh(&self) -> &'static str {
        match (self.warmth, self.familiarity) {
            (42..=i8::MAX, 65..=u8::MAX) => "朝夕熟稔・心頭親厚",
            (42..=i8::MAX, 35..=64) => "時常往來・深信不疑",
            (42..=i8::MAX, _) => "異地少見・心頭記掛",
            (i8::MIN..=-18, 65..=u8::MAX) => "同簷朝夕・話不投機",
            (i8::MIN..=-18, 32..=64) => "偶有照面・心存嫌隙",
            (i8::MIN..=-18, _) => "長年疏遠・舊怨未消",
            (_, 60..=u8::MAX) => "日常熟識・平淡相處",
            (_, 30..=59) => "點頭之交・各忙生計",
            _ => "音訊漸稀・印象平淡",
        }
    }
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
    pub recognized_faces: BTreeMap<String, i64>, // 256 槽 LRU 認臉熟客名單（防掌櫃樞紐失憶）
    pub emergent_phase_name: Option<String>,
    #[serde(default)]
    pub mastery: super::phase_and_vessels::ProceduralMastery,
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
        let entry = self.touch_active_social_slot(partner_id, partner_name, 35, game_day);
        if game_day > entry.last_fam_gain_day || entry.last_fam_gain_day == 0 {
            entry.familiarity = entry.familiarity.saturating_add(4).min(95);
            entry.last_fam_gain_day = game_day.max(1);
        }
        if used_pressure_tactic {
            entry.pressure_tactic_count = entry.pressure_tactic_count.saturating_add(1);
            entry.warmth = entry.warmth.saturating_sub(4).max(-95);
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
        let entry = self.touch_active_social_slot(partner_id, partner_name, 40, game_day);
        let prev_broken = entry.broken_pacts;
        let prev_helped = entry.helped_in_crisis;
        entry.familiarity = entry.familiarity.saturating_add(6).min(98);
        entry.trade_count = entry.trade_count.saturating_add(1);
        if kept {
            entry.kept_pacts = entry.kept_pacts.saturating_add(1);
            entry.warmth = entry.warmth.saturating_add(6).min(95);
        } else {
            entry.broken_pacts = entry.broken_pacts.saturating_add(1);
            entry.warmth = entry.warmth.saturating_sub(28).max(-95);
        }
        if helped_crisis {
            entry.helped_in_crisis = entry.helped_in_crisis.saturating_add(1);
            entry.warmth = entry.warmth.saturating_add(22).min(98);
        }
        if !note_zh.is_empty() {
            entry.notable_note_zh = note_zh.chars().take(72).collect();
        }
        if !partner_id.starts_with("pl:") {
            if !kept && prev_broken == 0 {
                self.push_turning_point(
                    &format!("pact_break:{partner_id}:{game_day}"),
                    game_day,
                    &format!("遭{}背棄然諾吃過虧，自此對外往來更為戒慎", partner_name),
                    "",
                );
            } else if helped_crisis && prev_helped == 0 {
                self.push_turning_point(
                    &format!("crisis_aid:{partner_id}:{game_day}"),
                    game_day,
                    &format!("危急關頭蒙{}仗義相救，將這份恩情銘記於心", partner_name),
                    "",
                );
            }
        }
    }

    fn touch_active_social_slot(
        &mut self,
        partner_id: &str,
        partner_name: &str,
        init_fam: u8,
        game_day: i64,
    ) -> &mut SocialEvidenceSummary {
        self.touch_recognized_face(partner_id, game_day);
        self.ensure_social_slot_capacity(partner_id);
        let entry = self
            .ltm
            .social
            .entry(partner_id.to_string())
            .or_insert_with(|| SocialEvidenceSummary {
                target_id: partner_id.to_string(),
                familiarity: init_fam,
                ..Default::default()
            });
        entry.target_name = partner_name.to_string();
        entry.acquainted = true;
        entry.from_genesis = false;
        entry.last_seen_day = game_day;
        entry
    }

    pub fn touch_recognized_face(&mut self, target_id: &str, seen_day: i64) {
        if target_id.is_empty() {
            return;
        }
        if !self.ltm.recognized_faces.contains_key(target_id)
            && self.ltm.recognized_faces.len() >= MAX_RECOGNIZED_FACES
            && let Some(oldest_k) = self
                .ltm
                .recognized_faces
                .iter()
                .min_by_key(|(k, d)| (k.starts_with("pl:"), **d))
                .map(|(k, _)| k.clone())
        {
            self.ltm.recognized_faces.remove(&oldest_k);
        }
        let d = self
            .ltm
            .recognized_faces
            .entry(target_id.to_string())
            .or_insert(seen_day);
        if seen_day > *d {
            *d = seen_day;
        }
    }

    fn ensure_social_slot_capacity(&mut self, incoming_id: &str) {
        if self.ltm.social.contains_key(incoming_id) || self.ltm.social.len() < MAX_SOCIAL_JUDGMENTS
        {
            return;
        }
        // 優先淘汰前史無名分點頭客，次之開服後點頭客，最後保底淘汰最舊紀錄以嚴守 64 槽硬上限！
        if let Some((evict_key, last_day, was_acq)) = self
            .ltm
            .social
            .iter()
            .min_by_key(|(k, s)| {
                (
                    k.starts_with("pl:"),
                    s.broken_pacts > 0 || s.helped_in_crisis > 0,
                    tie_flags::priority_rank(s.tie_mask),
                    !s.from_genesis,
                    s.familiarity,
                    s.last_seen_day,
                )
            })
            .map(|(k, s)| (k.clone(), s.last_seen_day, s.acquainted))
        {
            self.ltm.social.remove(&evict_key);
            if was_acq {
                self.touch_recognized_face(&evict_key, last_day);
            }
        }
    }

    fn distill_evicted_episode_residue(&mut self, evicted: &SubjectiveEpisodeLtm) {
        if evicted.subject_actor_id.is_empty() {
            return;
        }
        if let Some(soc) = self.ltm.social.get_mut(&evicted.subject_actor_id)
            && soc.notable_note_zh.is_empty()
        {
            soc.notable_note_zh = evicted.subjective_zh.chars().take(72).collect();
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

        let (init_credence, init_fidelity) = match ep.tier {
            EvidenceTier::DirectWitness => (100, 95),
            EvidenceTier::FamilyOral => (88, 60),
            EvidenceTier::ClaimedBySpeaker => (62, 52),
            EvidenceTier::TavernHearsay => (50, 42),
        };

        if self.ltm.episodic.len() >= MAX_HOT_EPISODIC_LTM
            && let Some((min_idx, _)) = self
                .ltm
                .episodic
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.retention_score(ep.game_day))
        {
            let evicted = self.ltm.episodic.swap_remove(min_idx);
            self.distill_evicted_episode_residue(&evicted);
        }

        self.ltm.episodic.push(SubjectiveEpisodeLtm {
            event_id: ep.event_id.clone(),
            origin_day: ep.game_day,
            node_id: ep.node_id.clone(),
            subject_actor_id: ep.actor_id.clone(),
            subject_name: ep.actor_name.clone(),
            tier: ep.tier,
            credence: init_credence,
            fidelity: init_fidelity,
            salience: ep.salience,
            rehearsals: 1,
            subjective_zh: colored_zh,
        });
    }

    /// 寫入創世逆推或跨代口傳之個人／家族長期記憶（嚴守主觀確信度 `credence` 與客觀保真度 `fidelity` 分離）
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit historical episode fields"
    )]
    pub fn ingest_historical_episode(
        &mut self,
        event_id: &str,
        origin_day: i64,
        node_id: &str,
        subject_actor_id: &str,
        subject_name: &str,
        tier: EvidenceTier,
        credence: u8,
        fidelity: u8,
        salience: u8,
        rehearsals: u16,
        subjective_zh: String,
    ) {
        if self.ltm.episodic.iter().any(|e| e.event_id == event_id) {
            return;
        }
        if self.ltm.episodic.len() >= MAX_HOT_EPISODIC_LTM
            && let Some((min_idx, _)) = self
                .ltm
                .episodic
                .iter()
                .enumerate()
                .min_by_key(|(_, x)| x.retention_score(origin_day.max(0)))
        {
            let evicted = self.ltm.episodic.swap_remove(min_idx);
            self.distill_evicted_episode_residue(&evicted);
        }
        self.ltm.episodic.push(SubjectiveEpisodeLtm {
            event_id: event_id.to_string(),
            origin_day,
            node_id: node_id.to_string(),
            subject_actor_id: subject_actor_id.to_string(),
            subject_name: subject_name.to_string(),
            tier,
            credence,
            fidelity,
            salience,
            rehearsals,
            subjective_zh,
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

        // R3-M2 修正：僅在尚無執行中方針，且「自身負面心事高漲（`valence < 0`）」或「連續受阻 >= 3 日」時升格慢思考，不讓正向溫情或旁觀街景擠爆佇列！
        let max_surprise = top_for_drift
            .as_ref()
            .map_or(0u32, |(_, _, inten, _)| u32::from(*inten));
        let max_neg_concern_int = self
            .stm
            .focus_concerns
            .iter()
            .flatten()
            .filter(|c| c.valence < 0)
            .map(|c| u32::from(c.intensity))
            .max()
            .unwrap_or(0);
        let cooldown_ok = game_day - self.stm.last_slow_think_day >= 3 || max_neg_concern_int >= 78;
        if self.stm.goal_strategy.is_none()
            && cooldown_ok
            && (self.stm.consecutive_blocked_days >= 3
                || max_neg_concern_int >= 74
                || (copper_shortfall && max_neg_concern_int >= 42))
        {
            let stakes = if copper_shortfall { 3u32 } else { 2u32 };
            let blocked = u32::from(self.stm.consecutive_blocked_days.max(1));
            let age_bonus = u32::from(self.slow_think_wait_bells / 4 + 1);
            let peak = max_surprise.max(max_neg_concern_int).max(40);
            self.pending_slow_think_score = peak * stakes * blocked * age_bonus;
        }

        // 睡眠舒緩疲勞、自然衰減心事；R3-M1：無毀諾暗線之一般口角嫌隙隨時間每 12 日微幅回溫 +2，防單向沉底
        if game_day > 0 && game_day % 12 == 0 {
            let cur_stamp = (game_day + 10_000).max(1);
            for soc in self.ltm.social.values_mut() {
                if !soc.cohabiting
                    && game_day.saturating_sub(soc.last_seen_day) >= 12
                    && soc.familiarity > 18
                {
                    soc.familiarity = soc.familiarity.saturating_sub(1);
                }
                if soc.warmth < -18
                    && soc.broken_pacts == 0
                    && soc.secret_hook_zh.is_empty()
                    && (soc.last_confront_day == 0 || cur_stamp >= soc.last_confront_day + 8)
                {
                    soc.warmth = (soc.warmth + 2).min(-12);
                }
            }
        }
        // 睡眠期間依個人節律錯相記掛外鎮久未見面之至親摯友（CrossTownYearning，避免同夜齊發）
        let stagger_mod = (i64::from(innate.base_values[0]) + game_day).rem_euclid(8);
        if game_day > 0 && stagger_mod == 0 {
            let yearn_tid = self
                .ltm
                .social
                .values()
                .filter(|s| {
                    !s.cohabiting
                        && !s.target_id.starts_with("pl:")
                        && !s.target_region_zh.is_empty()
                        && s.warmth >= 42
                        && game_day.saturating_sub(s.last_seen_day) >= 12
                })
                .max_by_key(|s| s.warmth)
                .map(|s| s.target_id.clone());
            if let Some(tid) = yearn_tid {
                let _ = self.appraise_event(
                    innate,
                    super::appraisal::ConcernCause::CrossTownYearning,
                    Some(&tid),
                    None,
                    38,
                    game_day * 12,
                );
            }
        }
        self.stm.affect.fatigue = self.stm.affect.fatigue.saturating_sub(55);
        self.decay_concerns_and_sync_affect(innate);
        self.stm.working_focus.clear();
    }

    /// 單向增量摺疊一樁生命事件至對人記憶槽（帶 `event_id` 去重保證 100% 冪等，創世超額自動溢流至 `recognized_faces`）
    #[expect(
        clippy::too_many_arguments,
        reason = "explicit social memory fold inputs"
    )]
    pub fn fold_social_memory_from_episode(
        &mut self,
        event_id: &str,
        origin_day: i64,
        target_id: &str,
        target_name: &str,
        target_region_zh: &str,
        tie_mask: u8,
        tie_label_zh: &str,
        direct_acquainted: bool,
        cohabiting: bool,
        familiarity_hint: u8,
        warmth_delta: i8,
        note_zh: &str,
        secret_hook_zh: &str,
    ) -> bool {
        if target_id.is_empty() {
            return false;
        }
        if self
            .ltm
            .social
            .get(target_id)
            .is_some_and(|e| e.folded_events.iter().any(|x| x == event_id))
        {
            return false;
        }
        if direct_acquainted {
            self.touch_recognized_face(target_id, origin_day);
        }
        // 創世前史配額保護：無名分熟客限 40 槽、創世總槽限 48 槽，硬性預留 >=16 槽給開服後玩家與新事件！
        let genesis_count = self
            .ltm
            .social
            .iter()
            .filter(|(k, s)| !k.starts_with("pl:") && s.from_genesis)
            .count();
        if !self.ltm.social.contains_key(target_id)
            && ((tie_mask == 0
                && secret_hook_zh.is_empty()
                && genesis_count >= MAX_GENESIS_SOCIAL_SLOTS)
                || genesis_count >= MAX_GENESIS_TOTAL_SLOTS)
        {
            return false;
        }

        self.ensure_social_slot_capacity(target_id);
        let entry = self
            .ltm
            .social
            .entry(target_id.to_string())
            .or_insert_with(|| SocialEvidenceSummary {
                target_id: target_id.to_string(),
                target_name: target_name.to_string(),
                familiarity: familiarity_hint,
                last_seen_day: origin_day,
                from_genesis: true,
                ..Default::default()
            });
        if entry.folded_events.len() >= 32 {
            entry.folded_events.remove(0);
        }
        entry.folded_events.push(event_id.to_string());
        entry.cohabiting |= cohabiting;

        entry.target_name = target_name.to_string();
        if !target_region_zh.is_empty() {
            entry.target_region_zh = target_region_zh.to_string();
        }
        let prev_mask = entry.tie_mask;
        entry.tie_mask |= tie_mask;
        if !tie_label_zh.is_empty() {
            let new_rk = tie_flags::priority_rank(tie_mask);
            let old_rk = tie_flags::priority_rank(prev_mask);
            if entry.tie_label_zh.is_empty() {
                entry.tie_label_zh = tie_label_zh.to_string();
            } else if new_rk > old_rk {
                entry.tie_label_zh = if old_rk >= 3
                    && !entry.tie_label_zh.contains('兼')
                    && entry.tie_label_zh != tie_label_zh
                {
                    format!("{tie_label_zh}兼{}", entry.tie_label_zh)
                } else {
                    tie_label_zh.to_string()
                };
            } else if new_rk >= 3
                && (prev_mask & tie_mask) == 0
                && !entry.tie_label_zh.contains('兼')
                && entry.tie_label_zh != tie_label_zh
            {
                entry.tie_label_zh = format!("{}兼{tie_label_zh}", entry.tie_label_zh);
            }
        }
        if direct_acquainted {
            entry.acquainted = true;
        }
        entry.familiarity = entry.familiarity.max(familiarity_hint);
        if origin_day > entry.last_seen_day {
            entry.last_seen_day = origin_day;
        }
        entry.warmth = (i16::from(entry.warmth) + i16::from(warmth_delta)).clamp(-95, 95) as i8;
        if entry.kept_pacts == 0
            && entry.broken_pacts == 0
            && entry.trade_count == 0
            && warmth_delta >= 0
        {
            entry.kept_pacts = 1;
        }
        if !note_zh.is_empty() && (entry.notable_note_zh.is_empty() || entry.from_genesis) {
            entry.notable_note_zh = note_zh.chars().take(76).collect();
        }
        if !secret_hook_zh.is_empty() && entry.secret_hook_zh.is_empty() {
            entry.secret_hook_zh = secret_hook_zh.chars().take(84).collect();
        }
        true
    }
}

//! 主迴圈與 NPC 社交可調參數（對齊 Go `config/simulation.go` + `data/config/simulation.json`）。

use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;

/// 與 `simulation.json` 對齊的結構（缺檔時使用 [`default_simulation`]）。
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SimulationParams {
    pub max_assignments_per_venue: i32,
    pub room_event: RoomEventCfg,
    pub quality_gate_default_max_runes: i32,
    pub npc_npc_pair_pick: NpcNpcPairPickCfg,
    pub npc_npc_social: NpcNpcSocialCfg,
    pub dialogue_anchor_rumor: DialogueAnchorRumorCfg,
    pub dyad_update: DyadUpdateCfg,
    /// 隨機 NPC 間對話計數器（對齊 Go `random_npc_dialogue_ticks`）。
    pub random_npc_dialogue_ticks: RandomNpcDialogueTicksCfg,
    /// 傳聞池衰減呼叫間隔（秒）。
    #[serde(default = "default_rumor_decay_interval_sec")]
    pub rumor_decay_interval_sec: i32,
    /// 傳聞 digest 重建間隔（分鐘）。
    #[serde(default = "default_rumor_digest_interval_min")]
    pub rumor_digest_interval_min: i32,
    /// 經濟 pulse 傳聞（對齊 Go `economy_pulse`）。
    #[serde(default)]
    pub economy_pulse: EconomyPulseCfg,
    /// 新生 NPC 傳聞 TTL（秒）。
    #[serde(default = "default_spawn_rumor_ttl_sec")]
    pub spawn_rumor_ttl_sec: i64,
    /// 求職撮合檢查間隔（秒）。
    #[serde(default = "default_job_matching_interval_sec")]
    pub job_matching_interval_sec: i32,
    /// 撮合成功傳聞 TTL（秒）。
    #[serde(default = "default_job_match_rumor_ttl_sec")]
    pub job_match_rumor_ttl_sec: i64,
    /// 黎明／深夜心境微調（對齊 Go `disposition_time_of_day`）。
    #[serde(default)]
    pub disposition_time_of_day: DispositionTimeOfDayCfg,
    /// 閒置／巡邏計時（對齊 Go `idle`）。
    #[serde(default)]
    pub idle: IdleCfg,
    /// 地圖移動檢查間隔（tick 數，對齊 `travel_tick_interval`）。
    #[serde(default = "default_travel_tick_interval")]
    pub travel_tick_interval: i32,
    /// 在職 NPC 巡邏擲骰上限（對齊 `wander_roll_max`）。
    #[serde(default = "default_wander_roll_max")]
    pub wander_roll_max: i32,
    /// 微互動觸發機率 0–100（對齊 `micro_interaction_chance_percent`）。
    #[serde(default = "default_micro_interaction_chance_percent")]
    pub micro_interaction_chance_percent: i32,
}

fn default_rumor_decay_interval_sec() -> i32 {
    30
}

fn default_rumor_digest_interval_min() -> i32 {
    2
}

fn default_spawn_rumor_ttl_sec() -> i64 {
    3600
}

fn default_job_matching_interval_sec() -> i32 {
    30
}

fn default_job_match_rumor_ttl_sec() -> i64 {
    7200
}

fn default_travel_tick_interval() -> i32 {
    30
}

fn default_wander_roll_max() -> i32 {
    10
}

fn default_micro_interaction_chance_percent() -> i32 {
    15
}

/// 閒置 tick 首次與後續間隔（對齊 Go `Sim.Idle`）。
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IdleCfg {
    pub first_trigger_min: i32,
    pub first_trigger_span: i32,
    pub interval_min: i32,
    pub interval_span: i32,
}

impl Default for IdleCfg {
    fn default() -> Self {
        Self {
            first_trigger_min: 25,
            first_trigger_span: 35,
            interval_min: 60,
            interval_span: 60,
        }
    }
}

/// 特定時段一次性調整全 NPC 心境（對齊 Go）。
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DispositionTimeOfDayCfg {
    #[serde(default = "default_dawn_hour_start")]
    pub dawn_hour_start: i32,
    #[serde(default = "default_dawn_hour_end")]
    pub dawn_hour_end: i32,
    #[serde(default = "default_late_night_hour_end")]
    pub late_night_hour_end: i32,
    #[serde(default = "default_dawn_delta")]
    pub dawn_delta: i32,
    #[serde(default = "default_late_night_delta")]
    pub late_night_delta: i32,
}

fn default_dawn_hour_start() -> i32 {
    6
}
fn default_dawn_hour_end() -> i32 {
    9
}
fn default_late_night_hour_end() -> i32 {
    5
}
fn default_dawn_delta() -> i32 {
    1
}
fn default_late_night_delta() -> i32 {
    -1
}

impl Default for DispositionTimeOfDayCfg {
    fn default() -> Self {
        Self {
            dawn_hour_start: 6,
            dawn_hour_end: 9,
            late_night_hour_end: 5,
            dawn_delta: 1,
            late_night_delta: -1,
        }
    }
}

/// 每若干遊戲日寫入一條經濟類傳聞（對齊 Go）。
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EconomyPulseCfg {
    #[serde(default = "default_economy_pulse_modulo")]
    pub game_day_modulo: i32,
    #[serde(default = "default_economy_pulse_rumor_ttl")]
    pub rumor_ttl_sec: i64,
}

fn default_economy_pulse_modulo() -> i32 {
    3
}

fn default_economy_pulse_rumor_ttl() -> i64 {
    5400
}

impl Default for EconomyPulseCfg {
    fn default() -> Self {
        Self {
            game_day_modulo: 3,
            rumor_ttl_sec: 5400,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RoomEventCfg {
    pub max_events: i32,
    pub ttl_sec: i64,
    pub rumor_ttl_sec: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NpcNpcPairPickCfg {
    pub thread_stale_sec: i32,
    pub thread_cooldown_sec: i32,
    pub score_base_noise: i32,
    pub dyad_sentiment_penalty_threshold: i32,
    pub dyad_sentiment_penalty: i32,
    pub same_venue_score_bonus: i32,
    pub pair_last_talk_penalty_sec: i32,
    pub pair_last_talk_penalty: i32,
    pub same_display_name_penalty: i32,
    pub familiarity_score_divisor: i32,
    pub same_dyad_venue_tag_bonus: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct NpcNpcSocialCfg {
    pub room_recent_player_talk_cooldown_sec: i32,
    pub broadcast_skip_recent_talk_sec: i32,
    pub default_dialogue_score_threshold: i32,
    pub player_gossip_bias_percent: i32,
    pub rumor_top_k_default: i32,
    pub rumor_top_k_player_in_room: i32,
    pub recent_room_events_sliding_max: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DialogueAnchorRumorCfg {
    pub min_anchor_runes: i32,
    pub ttl_sec: i64,
    pub weight: i32,
    pub source_score: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DyadUpdateCfg {
    pub familiarity_delta_per_exchange: i32,
    pub sentiment_cap: i32,
    pub quarrel_tag_sentiment_max: i32,
    pub clear_quarrel_sentiment_min: i32,
}

/// 主迴圈「觸發－輕」：有玩家房時重置 tick 範圍（對齊 Go）。
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RandomNpcDialogueTicksCfg {
    pub initial_min: i32,
    pub initial_span: i32,
    pub reset_min_with_player: i32,
    pub reset_span_with_player: i32,
}

impl Default for RandomNpcDialogueTicksCfg {
    fn default() -> Self {
        Self {
            initial_min: 80,
            initial_span: 40,
            reset_min_with_player: 115,
            reset_span_with_player: 75,
        }
    }
}

impl Default for RoomEventCfg {
    fn default() -> Self {
        Self {
            max_events: 5,
            ttl_sec: 120,
            rumor_ttl_sec: 1800,
        }
    }
}

impl Default for NpcNpcPairPickCfg {
    fn default() -> Self {
        Self {
            thread_stale_sec: 90,
            thread_cooldown_sec: 300,
            score_base_noise: 4,
            dyad_sentiment_penalty_threshold: -30,
            dyad_sentiment_penalty: 2,
            same_venue_score_bonus: 3,
            pair_last_talk_penalty_sec: 120,
            pair_last_talk_penalty: 20,
            same_display_name_penalty: 15,
            familiarity_score_divisor: 20,
            same_dyad_venue_tag_bonus: 2,
        }
    }
}

impl Default for NpcNpcSocialCfg {
    fn default() -> Self {
        Self {
            room_recent_player_talk_cooldown_sec: 60,
            broadcast_skip_recent_talk_sec: 15,
            default_dialogue_score_threshold: 35,
            player_gossip_bias_percent: 40,
            rumor_top_k_default: 2,
            rumor_top_k_player_in_room: 3,
            recent_room_events_sliding_max: 3,
        }
    }
}

impl Default for DialogueAnchorRumorCfg {
    fn default() -> Self {
        Self {
            min_anchor_runes: 4,
            ttl_sec: 3600,
            weight: 2,
            source_score: 1,
        }
    }
}

impl Default for DyadUpdateCfg {
    fn default() -> Self {
        Self {
            familiarity_delta_per_exchange: 2,
            sentiment_cap: 100,
            quarrel_tag_sentiment_max: -30,
            clear_quarrel_sentiment_min: 25,
        }
    }
}

impl Default for SimulationParams {
    fn default() -> Self {
        default_simulation()
    }
}

#[must_use]
pub fn default_simulation() -> SimulationParams {
    SimulationParams {
        max_assignments_per_venue: 2,
        room_event: RoomEventCfg::default(),
        quality_gate_default_max_runes: 52,
        npc_npc_pair_pick: NpcNpcPairPickCfg::default(),
        npc_npc_social: NpcNpcSocialCfg::default(),
        dialogue_anchor_rumor: DialogueAnchorRumorCfg::default(),
        dyad_update: DyadUpdateCfg::default(),
        random_npc_dialogue_ticks: RandomNpcDialogueTicksCfg::default(),
        rumor_decay_interval_sec: 30,
        rumor_digest_interval_min: 2,
        economy_pulse: EconomyPulseCfg::default(),
        spawn_rumor_ttl_sec: 3600,
        job_matching_interval_sec: 30,
        job_match_rumor_ttl_sec: 7200,
        disposition_time_of_day: DispositionTimeOfDayCfg::default(),
        idle: IdleCfg::default(),
        travel_tick_interval: 30,
        wander_roll_max: 10,
        micro_interaction_chance_percent: 15,
    }
}

static SIM: OnceLock<SimulationParams> = OnceLock::new();

/// 讀取 `data/config/simulation.json`（或 `custom_path`）；失敗時以內建預設填入並記錄警告。
pub fn load_simulation(custom_path: &str) -> anyhow::Result<()> {
    let p = if custom_path.is_empty() {
        Path::new("data/config/simulation.json").to_path_buf()
    } else {
        Path::new(custom_path).to_path_buf()
    };
    let raw = fs::read_to_string(&p).or_else(|_| fs::read_to_string(Path::new("..").join(&p)));
    let s = match raw {
        Ok(text) => match serde_json::from_str::<SimulationParams>(&text) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("simulation.json 解析失敗，使用內建預設: {e}");
                default_simulation()
            }
        },
        Err(e) => {
            tracing::warn!("simulation.json 讀取失敗，使用內建預設: {e}");
            default_simulation()
        }
    };
    let _ = SIM.set(s);
    Ok(())
}

/// 取得已載入的模擬參數（未呼叫 [`load_simulation`] 時為內建預設）。
#[must_use]
pub fn sim() -> &'static SimulationParams {
    SIM.get_or_init(default_simulation)
}

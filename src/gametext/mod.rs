// gametext 模組 — 從 data/config/gametext.json 載入文案、敘事模板、規則詞表。
// 對齊 Go gametext/gametext.go。

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::RwLock;
use serde::Deserialize;

const DEFAULT_PATH: &str = "data/config/gametext.json";

// ── JSON 反序列化結構 ──

#[derive(Debug, Clone, Default, Deserialize)]
struct RootJson {
    #[serde(default)]
    client_errors: HashMap<String, String>,
    #[serde(default)]
    brain_arrival: HashMap<String, String>,
    #[serde(default)]
    dyad_relation: DyadRelationJson,
    #[serde(default)]
    time_of_day: Vec<TimeBandJson>,
    #[serde(default)]
    dyad_tags: HashMap<String, String>,
    #[serde(default)]
    topic_ids: HashMap<String, String>,
    #[serde(default)]
    npc_social: NpcSocialJson,
    #[serde(default)]
    runtime_fmt: RuntimeFmtJson,
    #[serde(default)]
    brain_effects: BrainEffectsJson,
    #[serde(default)]
    economy_rumor_lines: Vec<String>,
    #[serde(default)]
    dialogue_scorer: DialogueScorerJson,
    #[serde(default)]
    sanitize: SanitizeJson,
    #[serde(default)]
    quality_gate: QualityGateJson,
    #[serde(default)]
    narration_dialogue_hints: String,
    #[serde(default)]
    anchor_negators: Vec<String>,
    #[serde(default)]
    sentiment: SentimentJson,
    #[serde(default)]
    raw_event_trim_prefixes: Vec<String>,
    #[serde(default)]
    default_npc_title: String,
    #[serde(default)]
    occupation_default_waiter: String,
    #[serde(default)]
    occupation_clerk: String,
    #[serde(default)]
    admin_wipe_entities_response: String,
    #[serde(default)]
    default_manager_title: String,
    #[serde(default)]
    debug_pair_tag_same_venue: String,
    #[serde(default)]
    dialogue_markers: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DyadRelationJson {
    #[serde(default)]
    pub nil_dyad: String,
    #[serde(default)]
    pub sentiment_low: String,
    #[serde(default)]
    pub sentiment_high: String,
    #[serde(default)]
    pub familiarity_high: String,
    #[serde(default)]
    pub familiarity_mid: String,
    #[serde(default)]
    pub familiarity_low: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TimeBandJson {
    #[serde(default)]
    pub min_hour: i32,
    #[serde(default)]
    pub max_hour: i32,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct NpcSocialJson {
    #[serde(default)]
    pub rumor_prefix: String,
    #[serde(default)]
    pub player_present_context_line: String,
    #[serde(default)]
    pub known_anchors_prefix: String,
    #[serde(default)]
    pub tone_seed_prefix: String,
    #[serde(default)]
    pub player_gossip_topic_suffix: String,
    #[serde(default)]
    pub rumor_echo_line_fmt: String,
    #[serde(default)]
    pub summary_fmt: String,
    #[serde(default)]
    pub archival_with_fmt: String,
    #[serde(default)]
    pub broadcast_fmt: String,
    #[serde(default)]
    pub newcomer_rumor: String,
    #[serde(default)]
    pub job_match_rumor_fmt: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuntimeFmtJson {
    #[serde(default)]
    pub shift_leave_fmt: String,
    #[serde(default)]
    pub shift_leave_to_shop_fmt: String,
    #[serde(default)]
    pub shift_arrive: String,
    #[serde(default)]
    pub travel_leave_fmt: String,
    #[serde(default)]
    pub travel_arrive_fmt: String,
    #[serde(default)]
    pub push_leave_fmt: String,
    #[serde(default)]
    pub push_arrive_fmt: String,
    #[serde(default)]
    pub push_shift_leave_detail_fmt: String,
    #[serde(default)]
    pub shift_leave_work: String,
    #[serde(default)]
    pub wander_arrive_home: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct BrainEffectsJson {
    #[serde(default)]
    pub beg_event_fmt: String,
    #[serde(default)]
    pub gather_event_fmt: String,
    #[serde(default)]
    pub hired_event_fmt: String,
    #[serde(default)]
    pub talk_social: String,
    #[serde(default)]
    pub trade_no_stock: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DialogueScorerJson {
    #[serde(default)]
    pub poison: Vec<String>,
    #[serde(default)]
    pub intimate_when_cold: Vec<String>,
    #[serde(default)]
    pub formal_when_warm: Vec<String>,
    #[serde(default)]
    pub pickup_signals: Vec<String>,
    #[serde(default)]
    pub question_markers: Vec<String>,
    #[serde(default)]
    pub wasteland_markers: Vec<String>,
    #[serde(default)]
    pub relation_cold_markers: Vec<String>,
    #[serde(default)]
    pub relation_warm_markers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SanitizeJson {
    #[serde(default)]
    pub think_tokens: Vec<String>,
    #[serde(default)]
    pub code_fence: String,
    #[serde(default)]
    pub self_correct_markers: Vec<String>,
    #[serde(default)]
    pub assistant_markers: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct QualityGateJson {
    #[serde(default)]
    pub bad_meta: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SentimentJson {
    #[serde(default)]
    pub shift_or_chat_substrings: Vec<String>,
    #[serde(default)]
    pub pry_substring: String,
    #[serde(default)]
    pub outside_wild_substrings: Vec<String>,
    #[serde(default)]
    pub weather_substring: String,
    #[serde(default)]
    pub neg_words: Vec<String>,
    #[serde(default)]
    pub pos_words: Vec<String>,
}

// ── 全域狀態 ──

static STORE: RwLock<Option<RootJson>> = RwLock::new(None);

/// 載入 gametext JSON，可重複呼叫（冪等）。
pub fn load(custom_path: &str) -> anyhow::Result<()> {
    let p = if custom_path.is_empty() { DEFAULT_PATH } else { custom_path };
    let raw = fs::read_to_string(p)
        .or_else(|_| fs::read_to_string(Path::new("..").join(p)))
        .map_err(|e| anyhow::anyhow!("gametext: read {:?}: {}", p, e))?;
    let r: RootJson = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("gametext: parse: {}", e))?;
    let mut guard = STORE.write().unwrap();
    *guard = Some(r);
    Ok(())
}

/// 程式啟動時呼叫，讀取失敗即 panic。
pub fn must_load() {
    if let Err(e) = load("") {
        eprintln!("gametext: {e}");
        std::process::exit(1);
    }
}

/// 是否已成功載入。
pub fn loaded() -> bool {
    STORE.read().unwrap().is_some()
}

/// 重新載入（測試用）。
pub fn reload() -> anyhow::Result<()> {
    {
        let mut guard = STORE.write().unwrap();
        *guard = None;
    }
    load("")
}

// ── 公開查詢函式 ──

fn with_root<F, T>(f: F) -> T
where
    F: FnOnce(&RootJson) -> T,
    T: Default,
{
    let guard = STORE.read().unwrap();
    match guard.as_ref() {
        Some(r) => f(r),
        None => T::default(),
    }
}

/// 回傳客戶端錯誤訊息，找不到則回傳 key 本身。
pub fn client(key: &str) -> String {
    with_root(|r| {
        r.client_errors.get(key).cloned().unwrap_or_else(|| key.to_string())
    })
}

/// 格式化客戶端訊息（用 key 查模板再代入參數）。
pub fn clientf(key: &str, args: &[&str]) -> String {
    with_root(|r| {
        match r.client_errors.get(key) {
            Some(tpl) => {
                let mut result = tpl.clone();
                for (i, arg) in args.iter().enumerate() {
                    result = result.replacen(&format!("{{{}}}", i), arg, 1);
                }
                result
            }
            None => key.to_string(),
        }
    })
}

/// 取得 brain_arrival 敘事模板（支援 `%s` 與 `{}` 占位）。
pub fn brain_arrival(intent_key: &str, npc_name: &str) -> String {
    with_root(|r| {
        r.brain_arrival.get(intent_key)
            .map(|tpl| {
                if tpl.contains("{}") {
                    tpl.replace("{}", npc_name)
                } else {
                    tpl.replacen("%s", npc_name, 1)
                }
            })
            .unwrap_or_default()
    })
}

/// 回傳 dyad 關係提示文字。
pub fn dyad_relation_hint(nil_dyad: bool, sentiment: i32, familiarity: i32) -> String {
    with_root(|r| {
        let d = &r.dyad_relation;
        if nil_dyad {
            return d.nil_dyad.clone();
        }
        if sentiment <= -40 {
            return d.sentiment_low.clone();
        }
        if sentiment >= 40 {
            return d.sentiment_high.clone();
        }
        if familiarity >= 70 {
            d.familiarity_high.clone()
        } else if familiarity >= 35 {
            d.familiarity_mid.clone()
        } else {
            d.familiarity_low.clone()
        }
    })
}

/// 回傳時段標籤。
pub fn time_of_day_label(hour: i32) -> String {
    with_root(|r| {
        for b in &r.time_of_day {
            if hour >= b.min_hour && hour < b.max_hour {
                return b.label.clone();
            }
        }
        r.time_of_day.last().map(|b| b.label.clone()).unwrap_or_default()
    })
}

pub fn dyad_tag(key: &str) -> String {
    with_root(|r| r.dyad_tags.get(key).cloned().unwrap_or_default())
}

pub fn topic_id(key: &str) -> String {
    with_root(|r| r.topic_ids.get(key).cloned().unwrap_or_default())
}

pub fn npc_social() -> NpcSocialJson {
    with_root(|r| r.npc_social.clone())
}

pub fn runtime_fmt() -> RuntimeFmtJson {
    with_root(|r| r.runtime_fmt.clone())
}

pub fn brain_effects() -> BrainEffectsJson {
    with_root(|r| r.brain_effects.clone())
}

pub fn economy_rumor_lines() -> Vec<String> {
    with_root(|r| r.economy_rumor_lines.clone())
}

pub fn dialogue_scorer() -> DialogueScorerJson {
    with_root(|r| r.dialogue_scorer.clone())
}

pub fn sanitize() -> SanitizeJson {
    with_root(|r| r.sanitize.clone())
}

pub fn quality_gate_bad_meta() -> Vec<String> {
    with_root(|r| r.quality_gate.bad_meta.clone())
}

pub fn narration_dialogue_hints() -> String {
    with_root(|r| r.narration_dialogue_hints.clone())
}

pub fn anchor_negators() -> Vec<String> {
    with_root(|r| r.anchor_negators.clone())
}

pub fn sentiment() -> SentimentJson {
    with_root(|r| r.sentiment.clone())
}

pub fn raw_event_trim_prefixes() -> Vec<String> {
    with_root(|r| r.raw_event_trim_prefixes.clone())
}

pub fn default_npc_title() -> String {
    with_root(|r| r.default_npc_title.clone())
}

pub fn occupation_waiter() -> String {
    with_root(|r| r.occupation_default_waiter.clone())
}

pub fn occupation_clerk() -> String {
    with_root(|r| r.occupation_clerk.clone())
}

pub fn admin_wipe_response() -> String {
    with_root(|r| r.admin_wipe_entities_response.clone())
}

pub fn default_manager_title() -> String {
    with_root(|r| r.default_manager_title.clone())
}

pub fn debug_pair_tag_same_venue() -> String {
    with_root(|r| r.debug_pair_tag_same_venue.clone())
}

pub fn dialogue_markers() -> String {
    with_root(|r| r.dialogue_markers.clone())
}

/// hint 是否包含 subs 中任一子字串。
pub fn topic_contains_any(hint: &str, subs: &[String]) -> bool {
    subs.iter().any(|s| !s.is_empty() && hint.contains(s.as_str()))
}

/// 截斷至 n 個 Unicode 字元，超出部分以「…」取代（對齊 Go `gametext.TruncRune`）。
pub fn trunc_rune(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= n {
        return s.to_string();
    }
    chars[..n].iter().collect::<String>() + "\u{2026}"
}

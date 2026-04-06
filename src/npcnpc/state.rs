// 房間事件滑窗、配對最後對話時間、統計；對齊既有 npcnpc/state_events。

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::db;
use crate::store::NpcRumor;

use super::types::LastSocialChoice;

/// 與 `data/config/simulation.json` 預設一致（Phase：之後可改讀設定檔）。
const ROOM_EVENT_TTL_SEC: i64 = 120;
const ROOM_EVENT_MAX: usize = 5;
const ROOM_EVENT_RUMOR_TTL_SEC: i64 = 1800;

#[derive(Debug, Clone)]
struct RoomEvent {
    at: i64,
    #[allow(dead_code)]
    kind: String,
    subject: String,
    detail: String,
}

static ROOM_EVENTS_BY_RID: LazyLock<RwLock<HashMap<String, Vec<RoomEvent>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static PAIR_TALK_AT: LazyLock<RwLock<HashMap<String, i64>>> = LazyLock::new(|| RwLock::new(HashMap::new()));

static LAST_CHOICE: LazyLock<RwLock<Option<LastSocialChoice>>> = LazyLock::new(|| RwLock::new(None));

fn default_stats() -> HashMap<String, i64> {
    [
        ("attempt", 0),
        ("success", 0),
        ("fail_no_model", 0),
        ("fail_recent_player_talk", 0),
        ("fail_not_enough_npc", 0),
        ("fail_no_pair", 0),
        ("fail_llm_empty", 0),
        ("fail_quality_gate", 0),
        ("fail_anchor_conflict", 0),
        ("fail_score_too_low", 0),
        ("score_pass", 0),
        ("score_disabled_skip", 0),
        ("fail_quality_unicode_garbage", 0),
        ("fail_quality_leaked_id", 0),
        ("fail_quality_markdown_residue", 0),
        ("fail_quality_too_short", 0),
        ("fail_quality_suspicious_narration", 0),
        ("fail_quality_json_residue", 0),
        ("skip_npc_npc_summary_dup", 0),
        ("skip_npc_npc_archival_dup", 0),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect()
}

static STATS: LazyLock<RwLock<HashMap<String, i64>>> = LazyLock::new(|| RwLock::new(default_stats()));

pub fn inc_stat(key: &str) {
    if let Ok(mut g) = STATS.write() {
        *g.entry(key.to_string()).or_insert(0) += 1;
    }
}

pub fn stats_snapshot() -> HashMap<String, i64> {
    STATS.read().map(|g| g.clone()).unwrap_or_default()
}

pub fn reset_stats() {
    if let Ok(mut g) = STATS.write() {
        *g = default_stats();
    }
}

pub fn last_choice_snapshot() -> Option<LastSocialChoice> {
    LAST_CHOICE.read().ok().and_then(|g| g.clone())
}

pub fn set_last_choice(c: LastSocialChoice) {
    if let Ok(mut g) = LAST_CHOICE.write() {
        *g = Some(c);
    }
}

pub fn clear_last_choice() {
    if let Ok(mut g) = LAST_CHOICE.write() {
        *g = None;
    }
}

fn canonical_pair_key(id_a: &str, id_b: &str) -> String {
    if id_a > id_b {
        format!("{id_b}|{id_a}")
    } else {
        format!("{id_a}|{id_b}")
    }
}

pub fn get_pair_last_talk(id_a: &str, id_b: &str) -> i64 {
    let key = canonical_pair_key(id_a, id_b);
    PAIR_TALK_AT.read().ok().and_then(|g| g.get(&key).copied()).unwrap_or(0)
}

pub fn set_pair_last_talk(id_a: &str, id_b: &str, at: i64) {
    let key = canonical_pair_key(id_a, id_b);
    if let Ok(mut g) = PAIR_TALK_AT.write() {
        g.insert(key, at);
    }
}

fn trunc_rune(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn push_rumor_from_room_event(room_id: &str, kind: &str, subject: &str, detail: &str, now: i64) {
    if room_id.is_empty() || detail.trim().is_empty() {
        return;
    }
    let zone = db::get_room(room_id)
        .ok()
        .flatten()
        .map(|r| r.zone)
        .unwrap_or_default();
    let text = format!("{}：{}", subject.trim(), detail.trim());
    let text = text.trim().to_string();
    if text.is_empty() {
        return;
    }
    let id = format!(
        "evt|{}|{}|{}",
        room_id,
        kind,
        trunc_rune(&text, 24)
    );
    let rumor = NpcRumor {
        id,
        text: trunc_rune(&text, 28),
        room_id: room_id.to_string(),
        zone,
        source: "room_event".to_string(),
        source_score: 2,
        weight: 1,
        mention_count: 0,
        last_used_at: 0,
        blocked_until: 0,
        penalty_count: 0,
        last_penalty_at: 0,
        last_penalty_reason: String::new(),
        updated_at: now,
        expires_at: now + ROOM_EVENT_RUMOR_TTL_SEC,
    };
    let _ = db::upsert_npc_rumor(rumor);
}

/// 推入房間滑動視窗事件並寫入傳聞（對齊既有 `PushRoomEvent`）。
pub fn push_room_event(room_id: &str, kind: &str, subject: &str, detail: &str) {
    if room_id.is_empty() {
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    if let Ok(mut map) = ROOM_EVENTS_BY_RID.write() {
        let list = map.entry(room_id.to_string()).or_default();
        list.push(RoomEvent {
            at: now,
            kind: kind.to_string(),
            subject: subject.to_string(),
            detail: detail.to_string(),
        });
        let cutoff = now - ROOM_EVENT_TTL_SEC;
        list.retain(|e| e.at >= cutoff);
        if list.len() > ROOM_EVENT_MAX {
            *list = list[list.len() - ROOM_EVENT_MAX..].to_vec();
        }
    }
    push_rumor_from_room_event(room_id, kind, subject, detail, now);
}

/// 房間近期事件敘述（對齊既有 `RecentRoomEvents`）。
pub fn recent_room_events(room_id: &str, max_count: usize) -> Vec<String> {
    if room_id.is_empty() || max_count == 0 {
        return Vec::new();
    }
    let list = ROOM_EVENTS_BY_RID
        .read()
        .ok()
        .and_then(|g| g.get(room_id).cloned())
        .unwrap_or_default();
    if list.is_empty() {
        return Vec::new();
    }
    let start = list.len().saturating_sub(max_count);
    let slice = &list[start..];
    let mut out = Vec::new();
    for e in slice {
        let mut t = e.subject.trim().to_string();
        if !e.detail.is_empty() {
            if t.is_empty() {
                t = e.detail.trim().to_string();
            } else {
                t = format!("{}，{}", t, e.detail.trim());
            }
        }
        if !t.is_empty() {
            out.push(t);
        }
    }
    out
}

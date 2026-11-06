// 主題遮罩、品質閘、錨點與情緒增量，對齊既有 npcnpc/helpers。

use crate::db::{self, rune_lcs_similarity};
use crate::gametext;
use crate::npc::NpcTopicMask;
use crate::store::NpcDyad;

use super::state::recent_room_events;
pub use crate::ai::sanitize::{replace_clause_initial_temporal_cai, strip_wt_catalog_codes};

const QUALITY_GATE_DEFAULT_MAX_RUNES: i32 = 52;

/// 依房間與時段建構主題遮罩（對齊既有 `TopicMaskForRoom`）。
pub fn topic_mask_for_room(room_id: &str, hour: i32) -> NpcTopicMask {
    let venue_ids = db::get_venue_ids_for_room(room_id).unwrap_or_default();
    let room_tags = db::get_room(room_id)
        .ok()
        .flatten()
        .map(|r| r.tags)
        .unwrap_or_default();
    NpcTopicMask {
        is_work_venue: !venue_ids.is_empty(),
        is_night_time: !(5..21).contains(&hour),
        has_room_event: !recent_room_events(room_id, 1).is_empty(),
        room_tags,
    }
}

/// 兩 NPC 是否有共用工作場所（對齊既有 `NpcPairSameVenue`）。
#[must_use]
pub fn npc_pair_same_venue(id_a: &str, id_b: &str) -> bool {
    let Ok(as_a) = db::get_assignments_for_entity(id_a) else {
        return false;
    };
    let Ok(as_b) = db::get_assignments_for_entity(id_b) else {
        return false;
    };
    if as_a.is_empty() || as_b.is_empty() {
        return false;
    }
    let mut vid = std::collections::HashSet::new();
    for a in &as_a {
        if !a.venue_id.is_empty() {
            vid.insert(a.venue_id.as_str());
        }
    }
    as_b.iter()
        .any(|b| !b.venue_id.is_empty() && vid.contains(b.venue_id.as_str()))
}

#[must_use]
pub fn has_tag(tags: &[String], t: &str) -> bool {
    tags.iter().any(|v| v == t)
}

pub fn add_tag(tags: &[String], t: &str) -> Vec<String> {
    if t.is_empty() || has_tag(tags, t) {
        return tags.to_vec();
    }
    let mut out = tags.to_vec();
    out.push(t.to_string());
    out
}

pub fn remove_tag(tags: &[String], t: &str) -> Vec<String> {
    if t.is_empty() || tags.is_empty() {
        return tags.to_vec();
    }
    tags.iter().filter(|v| *v != t).cloned().collect()
}

/// 雙元關係提示句（對齊既有 `dyadRelationHint`）。
#[must_use]
pub fn dyad_relation_hint(d: Option<&NpcDyad>) -> String {
    match d {
        None => gametext::dyad_relation_hint(true, 0, 0),
        Some(di) => gametext::dyad_relation_hint(false, di.sentiment, di.familiarity),
    }
}

/// 依台詞與主題推估情緒增量（對齊既有 `SentimentDeltaFromDialogue`）。
#[must_use]
pub fn sentiment_delta_from_dialogue(
    topic_hint: &str,
    line_a: &str,
    line_b: &str,
    familiarity: i32,
    topic_id: &str,
) -> i32 {
    let mut delta = 0;
    let sen = gametext::sentiment();
    if gametext::topic_contains_any(topic_hint, &sen.shift_or_chat_substrings) {
        delta += 1;
    }
    if !sen.pry_substring.is_empty()
        && topic_hint.contains(sen.pry_substring.as_str())
        && familiarity < 20
    {
        delta -= 1;
    }
    let outside = gametext::topic_id("outside_rumor");
    let weather = gametext::topic_id("weather");
    let empty_topic_outside_weather = topic_id.is_empty()
        && (gametext::topic_contains_any(topic_hint, &sen.outside_wild_substrings)
            || (!sen.weather_substring.is_empty()
                && topic_hint.contains(sen.weather_substring.as_str())));
    if topic_id == outside || topic_id == weather || empty_topic_outside_weather {
        delta += 1;
    }
    let joined = format!("{line_a} {line_b}");
    for w in &sen.neg_words {
        if !w.is_empty() && joined.contains(w.as_str()) {
            delta -= 2;
            break;
        }
    }
    for w in &sen.pos_words {
        if !w.is_empty() && joined.contains(w.as_str()) {
            delta += 1;
            break;
        }
    }
    delta
}

// 清理 NPC 台詞（去除舞台提示 narration、簡化字/兒化音替換）
include!("helpers/clean.rs");
include!("helpers/quality.rs");
include!("helpers/tests.rs");

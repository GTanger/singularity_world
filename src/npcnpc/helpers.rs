// 主題遮罩、品質閘、錨點與情緒增量，對齊 Go npcnpc/helpers.go。

use crate::db::{self, rune_lcs_similarity};
use crate::gametext;
use crate::npc::NpcTopicMask;
use crate::store::NpcDyad;

use super::state::recent_room_events;

const QUALITY_GATE_DEFAULT_MAX_RUNES: i32 = 52;

/// 依房間與時段建構主題遮罩（對齊 Go `TopicMaskForRoom`）。
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

/// 兩 NPC 是否有共用工作場所（對齊 Go `NpcPairSameVenue`）。
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
    as_b.iter().any(|b| !b.venue_id.is_empty() && vid.contains(b.venue_id.as_str()))
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

/// 雙元關係提示句（對齊 Go `dyadRelationHint`）。
#[must_use]
pub fn dyad_relation_hint(d: Option<&NpcDyad>) -> String {
    match d {
        None => gametext::dyad_relation_hint(true, 0, 0),
        Some(di) => gametext::dyad_relation_hint(false, di.sentiment, di.familiarity),
    }
}

/// 依台詞與主題推估情緒增量（對齊 Go `SentimentDeltaFromDialogue`）。
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
            || (!sen.weather_substring.is_empty() && topic_hint.contains(sen.weather_substring.as_str())));
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

/// 單句品質閘（對齊 Go `qualityGateNpcLine`）；通過則 `(true, "")`。
pub fn quality_gate_npc_line(line: &str, max_runes: i32) -> (bool, &'static str) {
    let max = if max_runes <= 0 {
        QUALITY_GATE_DEFAULT_MAX_RUNES
    } else {
        max_runes
    };
    let s = line.trim();
    if s.is_empty() {
        return (false, "empty");
    }
    if s.contains('\u{fffd}') {
        return (false, "unicode_garbage");
    }
    let t = s.trim();
    let t_runes: Vec<char> = t.chars().collect();
    if t == "{" || t == "}" || (t.starts_with('{') && t_runes.len() <= 4) {
        return (false, "json_residue");
    }
    let low = s.to_lowercase();
    for leak in ["npc_", "entity_", "room_"] {
        if low.contains(leak) {
            return (false, "leaked_id");
        }
    }
    if low.contains("street") || low.contains("zone") {
        return (false, "leaked_id");
    }
    if s.contains("---") || s.contains("```") || s.contains("\n\n") {
        return (false, "markdown_residue");
    }
    let n = s.chars().count();
    if n <= 2 {
        return (false, "too_short");
    }
    if n > max as usize {
        return (false, "too_long");
    }
    for b in gametext::quality_gate_bad_meta() {
        if !b.is_empty() && s.contains(b.as_str()) {
            return (false, "meta_text");
        }
    }
    let gopen: char = '\u{300c}';
    let gclose: char = '\u{300d}';
    if s.matches(gopen).count() > 2 || s.matches(gclose).count() > 2 {
        return (false, "nested_quotes");
    }
    if has_long_repeat(s) {
        return (false, "repetition");
    }
    if looks_like_npc_npc_narration_line(s) {
        return (false, "suspicious_narration");
    }
    (true, "")
}

fn looks_like_npc_npc_narration_line(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 || !s.ends_with('。') {
        return false;
    }
    let hints = gametext::dialogue_markers();
    for ch in r {
        if hints.contains(ch) {
            return false;
        }
    }
    true
}

#[must_use]
pub fn should_skip_npc_npc_summary_update(
    old_summary: &str,
    new_summary: &str,
    line_a: &str,
    line_b: &str,
) -> bool {
    let old_summary = old_summary.trim();
    let new_summary = new_summary.trim();
    if old_summary.is_empty() {
        return false;
    }
    if new_summary == old_summary {
        return true;
    }
    if rune_lcs_similarity(old_summary, new_summary) >= 0.7 {
        return true;
    }
    let la = line_a.trim();
    let lb = line_b.trim();
    if !la.is_empty() && rune_lcs_similarity(old_summary, la) >= 0.7 {
        return true;
    }
    if !lb.is_empty() && rune_lcs_similarity(old_summary, lb) >= 0.7 {
        return true;
    }
    false
}

#[must_use]
pub fn dedupe_social_context_lines(lines: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let key = line.to_lowercase();
        if seen.contains(&key) {
            continue;
        }
        seen.insert(key);
        out.push(line.to_string());
    }
    out
}

fn has_long_repeat(s: &str) -> bool {
    let r: Vec<char> = s.chars().collect();
    if r.len() < 8 {
        return false;
    }
    let mut i = 0;
    while i + 3 < r.len() {
        let ch = r[i];
        if ch == ' ' {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < r.len() && r[j] == ch {
            j += 1;
        }
        if j - i >= 4 {
            return true;
        }
        i += 1;
    }
    false
}

#[must_use]
pub fn merge_thread_anchors(old: &[String], fresh: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::new();
    for v in old.iter().chain(fresh.iter()) {
        let v = v.trim();
        if v.is_empty() || !seen.insert(v.to_string()) {
            continue;
        }
        out.push(v.to_string());
    }
    if out.len() > 4 {
        out = out[out.len() - 4..].to_vec();
    }
    out
}

/// 錨與台詞是否矛盾（對齊 Go `anchorConsistencyCheck`）。
pub fn anchor_consistency_check(existing: &[String], line_a: &str, line_b: &str) -> (bool, String) {
    if existing.is_empty() {
        return (true, String::new());
    }
    let joined = format!("{} {}", line_a.trim(), line_b.trim()).trim().to_string();
    if joined.is_empty() {
        return (true, String::new());
    }
    let negators = gametext::anchor_negators();
    for a in existing {
        let a = a.trim();
        if a.is_empty() {
            continue;
        }
        let runes: Vec<char> = a.chars().collect();
        for i in 0..runes.len().saturating_sub(1) {
            let k: String = runes[i..i + 2].iter().collect();
            if !joined.contains(&k) {
                continue;
            }
            for n in &negators {
                let nk = format!("{n}{k}");
                let kn = format!("{k}{n}");
                if joined.contains(&nk) || joined.contains(&kn) {
                    return (false, format!("anchor negated: {k}"));
                }
            }
        }
    }
    (true, String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_anchors_cap_four() {
        let old = vec!["a".into(), "b".into(), "c".into(), "d".into(), "e".into()];
        let fresh = vec!["f".into()];
        let m = merge_thread_anchors(&old, &fresh);
        assert_eq!(m.len(), 4);
        assert_eq!(m.last().map(String::as_str), Some("f"));
    }

    #[test]
    fn dedupe_lines_casefold() {
        let v = vec!["Hi".into(), "hi".into(), "x".into()];
        assert_eq!(dedupe_social_context_lines(&v).len(), 2);
    }
}

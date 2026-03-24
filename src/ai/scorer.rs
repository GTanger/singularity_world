// NPC 對話評分，對齊 Go ai/scorer.go。

use serde::Serialize;

use crate::gametext;

/// 單輪 NPC 對話評分（0–100），供除錯與遙測。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct DialogueScoreDetail {
    pub length: i32,
    pub anchor: i32,
    pub relation: i32,
    pub repeat: i32,
    pub diversity: i32,
    pub dialogue_feel: i32,
    pub identity: i32,
    pub narration: i32,
    #[serde(skip_serializing_if = "is_zero_i32")]
    pub tone_drift: i32,
    pub total: i32,
    #[serde(skip_serializing_if = "str_is_empty")]
    pub killed_by: String,
}

fn is_zero_i32(v: &i32) -> bool {
    *v == 0
}

fn str_is_empty(s: &str) -> bool {
    s.is_empty()
}

fn wasteland_tone_penalty(combined: &str, markers: &[String]) -> i32 {
    if combined.is_empty() {
        return 0;
    }
    for m in markers {
        if !m.is_empty() && combined.contains(m.as_str()) {
            return -10;
        }
    }
    0
}

fn rune_overlap_ratio(a: &str, b: &str) -> f64 {
    let ra: Vec<char> = a.chars().collect();
    let rb: Vec<char> = b.chars().collect();
    if ra.is_empty() || rb.is_empty() {
        return 0.0;
    }
    use std::collections::HashSet;
    let set: HashSet<char> = ra.iter().copied().collect();
    let match_count = rb.iter().filter(|r| set.contains(r)).count();
    let bigger = ra.len().max(rb.len());
    match_count as f64 / bigger as f64
}

/// 依 gametext 規則為兩句 NPC 對白評分。
#[allow(clippy::too_many_arguments)]
pub fn score_npc_dialogue(
    line_a: &str,
    line_b: &str,
    room_name: &str,
    recent_events: &[String],
    topic_hint: &str,
    relation_hint: &str,
    npc_npc_memory: &str,
    speaker_name: &str,
    listener_name: &str,
    recent_archival_lines: &[String],
) -> DialogueScoreDetail {
    let mut d = DialogueScoreDetail::default();
    let combined = format!("{line_a}{line_b}");
    let sc = gametext::dialogue_scorer();

    for p in &sc.poison {
        if p.is_empty() {
            continue;
        }
        let low_c = combined.to_lowercase();
        let low_p = p.to_lowercase();
        if low_c.contains(&low_p) {
            d.killed_by = format!("poison:{p}");
            d.total = 0;
            return d;
        }
    }

    let len_a = line_a.chars().count() as i32;
    let len_b = line_b.chars().count() as i32;
    d.length = match () {
        _ if (4..=28).contains(&len_a) && (4..=28).contains(&len_b) => 15,
        _ if len_a <= 2 || len_b <= 2 => 0,
        _ if len_a > 40 || len_b > 40 => 3,
        _ => 10,
    };

    let mut anchor_pool: Vec<&str> = vec![room_name, topic_hint, npc_npc_memory];
    anchor_pool.extend(recent_events.iter().map(|s| s.as_str()));
    let mut hits = 0i32;
    for anchor in anchor_pool {
        let anchor = anchor.trim();
        if anchor.is_empty() {
            continue;
        }
        let runes: Vec<char> = anchor.chars().collect();
        for i in 0..runes.len().saturating_sub(1) {
            let bigram: String = runes[i..i + 2].iter().collect();
            if combined.contains(&bigram) {
                hits += 1;
                break;
            }
        }
    }
    d.anchor = (hits * 5).min(20);

    d.relation = 7;
    let cold = sc
        .relation_cold_markers
        .iter()
        .any(|sub| !sub.is_empty() && relation_hint.contains(sub.as_str()));
    if cold {
        let intimate = sc.intimate_when_cold.iter().any(|w| {
            !w.is_empty() && combined.contains(w.as_str())
        });
        if intimate {
            d.relation = 0;
        } else {
            d.relation = 10;
        }
    } else {
        let warm = sc
            .relation_warm_markers
            .iter()
            .any(|sub| !sub.is_empty() && relation_hint.contains(sub.as_str()));
        if warm {
            let formal = sc.formal_when_warm.iter().any(|w| {
                !w.is_empty() && combined.contains(w.as_str())
            });
            d.relation = if formal { 3 } else { 10 };
        }
    }

    d.repeat = 0;
    for prev in recent_archival_lines {
        let prev = prev.trim();
        if prev.is_empty() {
            continue;
        }
        if prev == line_a || prev == line_b {
            d.repeat = -25;
            break;
        }
        if rune_overlap_ratio(prev, line_a) > 0.85 || rune_overlap_ratio(prev, line_b) > 0.85 {
            d.repeat = -15;
            break;
        }
    }

    let mut bigrams: Vec<String> = Vec::new();
    for line in [line_a, line_b] {
        let r: Vec<char> = line.chars().collect();
        for i in 0..r.len().saturating_sub(1) {
            bigrams.push(r[i..i + 2].iter().collect());
        }
    }
    if !bigrams.is_empty() {
        use std::collections::HashSet;
        let unique: HashSet<&str> = bigrams.iter().map(|s| s.as_str()).collect();
        let ratio = unique.len() as f64 / bigrams.len() as f64;
        d.diversity = (ratio * 10.0) as i32;
    }

    for sig in &sc.pickup_signals {
        if !sig.is_empty() && line_b.contains(sig.as_str()) {
            d.dialogue_feel += 8;
            break;
        }
    }
    for q in &sc.question_markers {
        if !q.is_empty() && line_a.contains(q.as_str()) {
            d.dialogue_feel += 7;
            break;
        }
    }
    if d.dialogue_feel > 15 {
        d.dialogue_feel = 15;
    }

    d.identity = 10;
    if !speaker_name.is_empty() && line_a.contains(speaker_name) {
        d.identity -= 5;
    }
    if !listener_name.is_empty() && line_b.contains(listener_name) {
        d.identity -= 5;
    }

    let dm = gametext::dialogue_markers();
    let has_marker = dm.chars().any(|m| combined.contains(m));
    if !has_marker {
        d.narration = -20;
    }

    d.tone_drift = wasteland_tone_penalty(&combined, &sc.wasteland_markers);

    d.total = d.length
        + d.anchor
        + d.relation
        + d.repeat
        + d.diversity
        + d.dialogue_feel
        + d.identity
        + d.narration
        + d.tone_drift;
    d.total = d.total.clamp(0, 100);
    d
}

/// 剝除房間事件顯示前綴，供評分用（對齊 Go `RawEventsForDialogueScore`）。
pub fn raw_events_for_dialogue_score(recent_events: &[String]) -> Vec<String> {
    let prefixes = gametext::raw_event_trim_prefixes();
    let mut raw = Vec::new();
    for ev in recent_events {
        let mut ev = ev.trim().to_string();
        for p in &prefixes {
            if !p.is_empty() && ev.starts_with(p.as_str()) {
                ev = ev[p.len()..].to_string();
            }
        }
        let ev = ev.trim();
        if !ev.is_empty() {
            raw.push(ev.to_string());
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn ensure_gametext() {
        INIT.call_once(|| {
            let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/config/gametext.json");
            if p.is_file() {
                let _ = gametext::load(p.to_str().unwrap());
            }
        });
    }

    #[test]
    fn score_npc_dialogue_reasonable_dialogue() {
        ensure_gametext();
        if !gametext::loaded() {
            return;
        }
        let d = score_npc_dialogue(
            "今天街口風大，你還出攤啊？",
            "嗯，總得討生活嘛，浮生這邊人還多。",
            "四路街口",
            &[String::from("天色漸暗")],
            "閒聊",
            "兩人算點頭之交，語氣中性偏熟",
            "",
            "葉俊",
            "楊康偉",
            &[],
        );
        assert!(d.total >= 30, "expected decent score, got {} detail={d:?}", d.total);
        assert!(d.killed_by.is_empty(), "unexpected kill: {}", d.killed_by);
    }

    #[test]
    fn score_npc_dialogue_poison() {
        ensure_gametext();
        if !gametext::loaded() {
            return;
        }
        let d = score_npc_dialogue(
            "npc_1 說",
            "好",
            "房",
            &[],
            "",
            "",
            "",
            "A",
            "B",
            &[],
        );
        assert_eq!(d.total, 0);
        assert!(!d.killed_by.is_empty());
    }

    #[test]
    fn score_npc_dialogue_wasteland_tone_penalty() {
        ensure_gametext();
        if !gametext::loaded() {
            return;
        }
        let d = score_npc_dialogue(
            "你聽說沒？廢土那邊又出事。",
            "唉，日子難過。",
            "街口",
            &[],
            "閒聊",
            "",
            "",
            "A",
            "B",
            &[],
        );
        assert_eq!(d.tone_drift, -10, "detail={d:?}");
    }

    #[test]
    fn score_npc_dialogue_ok_with_edi_and_fusui() {
        ensure_gametext();
        if !gametext::loaded() {
            return;
        }
        let d = score_npc_dialogue(
            "城外惡地這季別單走。",
            "嗯，聽說霜林邊緣有獸跡。",
            "城門",
            &[String::from("游離輻射")],
            "城外聞",
            "",
            "",
            "A",
            "B",
            &[],
        );
        assert_eq!(d.tone_drift, 0, "惡地/霜林不應觸發廢土漂移, detail={d:?}");
        assert!(d.total >= 25, "expected passable score, got {} {d:?}", d.total);
    }
}

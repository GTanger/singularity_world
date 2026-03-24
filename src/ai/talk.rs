// 玩家↔NPC／NPC↔NPC 的 prompt 組裝、回應解析與 Ollama /api/chat，對齊 Go ai/talk.go。

use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use super::prompts::{
    meta_bad_markers, npc_to_npc_collapse_say_separators, npc_to_npc_context_line_fmt,
    npc_to_npc_fallback_listener_reply, npc_to_npc_identity_prefix, npc_to_npc_listener_label,
    npc_to_npc_memory_line_fmt, npc_to_npc_player_present_note, npc_to_npc_recent_event_bullet,
    npc_to_npc_recent_events_header, npc_to_npc_relation_line_fmt, npc_to_npc_room_atmosphere_prefix,
    npc_to_npc_room_desc_trunc_suffix, npc_to_npc_room_tags_prefix, npc_to_npc_rules_block,
    npc_to_npc_speaker_label, npc_to_npc_task_intro, npc_to_npc_tone_after_world,
    npc_to_npc_topic_line_fmt, npc_to_npc_user_message, player_npc_behavior_rules,
    player_npc_identity_prefix, player_npc_persona_line, player_npc_room_context_fmt,
    player_npc_sensitivity_prefix, player_npc_space_consistency_rule, player_npc_style_example_bullet,
    player_npc_style_examples_header, player_npc_traditional_rule, player_npc_user_plain_fmt,
    player_npc_user_with_memory_fmt, world_phenomena_cognition_prompt,
};

fn sprintf_two_percent_s(template: &str, first: &str, second: &str) -> String {
    let Some(i) = template.find("%s") else {
        return template.to_string();
    };
    let mut out = String::new();
    out.push_str(&template[..i]);
    out.push_str(first);
    let rest = &template[i + 2..];
    let Some(j) = rest.find("%s") else {
        out.push_str(rest);
        return out;
    };
    out.push_str(&rest[..j]);
    out.push_str(second);
    out.push_str(&rest[j + 2..]);
    out
}

fn sprintf_one_percent_s(template: &str, value: &str) -> String {
    template.replacen("%s", value, 1)
}

/// 組出玩家↔NPC Talk 的 system 與 user（對齊 Go `PlayerNPCTalkBuildPrompts`）。
pub fn player_npc_talk_build_prompts(
    player_input: &str,
    npc_backstory: &str,
    npc_memory_snippets: &[String],
    style_examples: &[String],
    sensitivity_hint: &str,
    room_display_name: &str,
) -> (String, String) {
    let mut sb = String::new();
    sb.push_str(&player_npc_persona_line());
    sb.push_str(&world_phenomena_cognition_prompt());
    let room_display_name = room_display_name.trim();
    if !room_display_name.is_empty() {
        let fmt_line_owned = player_npc_room_context_fmt();
        let fmt_line = fmt_line_owned.trim();
        if !fmt_line.is_empty() {
            sb.push_str(&sprintf_one_percent_s(fmt_line, room_display_name));
        }
        let mut sc = player_npc_space_consistency_rule().trim().to_string();
        if !sc.is_empty() {
            if !sc.ends_with('\n') {
                sc.push('\n');
            }
            sb.push_str(&sc);
        }
    }
    sb.push_str(&player_npc_behavior_rules());
    sb.push_str(&player_npc_traditional_rule());
    if !npc_backstory.is_empty() {
        sb.push_str(&player_npc_identity_prefix());
        sb.push_str(npc_backstory);
        sb.push('\n');
    }
    if !sensitivity_hint.is_empty() {
        sb.push_str(&player_npc_sensitivity_prefix());
        sb.push_str(sensitivity_hint);
        sb.push('\n');
    }
    if !style_examples.is_empty() {
        sb.push_str(&player_npc_style_examples_header());
        let bullet = player_npc_style_example_bullet();
        for ex in style_examples {
            sb.push_str(&bullet);
            sb.push_str(ex.trim());
            sb.push('\n');
        }
    }
    let user = if !npc_memory_snippets.is_empty() {
        sprintf_two_percent_s(
            &player_npc_user_with_memory_fmt(),
            player_input,
            &npc_memory_snippets.join("\n"),
        )
    } else {
        sprintf_one_percent_s(&player_npc_user_plain_fmt(), player_input)
    };
    (sb, user)
}

/// 若回覆以「…」包一層，抽出內文（對齊 Go `CallAITalk` 結尾處理）。
pub fn strip_reply_guillemet_outer(reply: &str) -> String {
    let reply = reply.trim();
    let gopen = '\u{300c}';
    let gclose = '\u{300d}';
    if reply.starts_with(gopen) && reply.contains(gclose) && let Some(i) = reply.find(gclose) {
        let start = gopen.len_utf8();
        if i > start {
            return reply[start..i].to_string();
        }
    }
    reply.to_string()
}

/// 呼叫 Ollama `/api/chat`（對齊 Go `CallAITalk`）。
#[allow(clippy::too_many_arguments)]
pub fn call_ai_talk(
    base_url: &str,
    model: &str,
    player_input: &str,
    npc_backstory: &str,
    npc_memory_snippets: &[String],
    style_examples: &[String],
    sensitivity_hint: &str,
    room_display_name: &str,
) -> anyhow::Result<String> {
    let base_url = base_url.trim().trim_end_matches('/');
    let model = model.trim();
    if base_url.is_empty() || model.is_empty() {
        anyhow::bail!("ollama not configured");
    }
    let (system_prompt, user_msg) = player_npc_talk_build_prompts(
        player_input,
        npc_backstory,
        npc_memory_snippets,
        style_examples,
        sensitivity_hint,
        room_display_name,
    );
    let body = json!({
        "model": model,
        "think": false,
        "stream": false,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_msg},
        ],
        "options": {
            "num_predict": 100,
            "temperature": 0.85,
        }
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let url = format!("{base_url}/api/chat");
    let resp = client.post(&url).json(&body).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("ollama {}", resp.status());
    }
    let out: OllamaChatResponse = resp.json()?;
    let reply = out.message.content.trim();
    if reply.is_empty() {
        anyhow::bail!("empty reply");
    }
    Ok(strip_reply_guillemet_outer(reply))
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

pub fn is_npc_npc_meta_or_assistant_line(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return true;
    }
    let low = t.to_lowercase();
    if t.starts_with("---") || t.starts_with("——") {
        return true;
    }
    for b in meta_bad_markers() {
        if b.is_empty() {
            continue;
        }
        if t.contains(&b) || low.contains(&b.to_lowercase()) {
            return true;
        }
    }
    false
}

pub fn strip_outer_guillemet_line(s: &str) -> String {
    let gopen = '\u{300c}';
    let gclose = '\u{300d}';
    let mut s = s.trim().to_string();
    loop {
        let chs: Vec<char> = s.chars().collect();
        if chs.len() < 2 || chs.first() != Some(&gopen) || chs.last() != Some(&gclose) {
            break;
        }
        let inner: String = chs[1..chs.len() - 1].iter().collect();
        let next = inner.trim().to_string();
        if next.is_empty() || next == s {
            break;
        }
        s = next;
    }
    s.trim().to_string()
}

pub fn strip_leading_speaker_name(line: &str, speaker_name: &str) -> String {
    let line = line.trim();
    let speaker_name = speaker_name.trim();
    if speaker_name.is_empty() || !line.starts_with(speaker_name) {
        return line.to_string();
    }
    let rest = line[speaker_name.len()..].trim_start();
    if rest.is_empty() {
        return line.to_string();
    }
    let mut it = rest.chars();
    let Some(r) = it.next() else {
        return line.to_string();
    };
    if matches!(r, '：' | ':' | '，' | ',') {
        return line.to_string();
    }
    rest.to_string()
}

pub fn collapse_nested_say_guillemets(s: &str) -> String {
    let gclose = '\u{300d}';
    let mut s = s.trim().to_string();
    let seps = npc_to_npc_collapse_say_separators();
    if !seps.is_empty() {
        loop {
            let mut best_idx: Option<usize> = None;
            let mut best_len = 0usize;
            for sep in &seps {
                if sep.is_empty() {
                    continue;
                }
                if let Some(i) = s.rfind(sep.as_str()) {
                    let take = match best_idx {
                        None => true,
                        Some(b) => i >= b,
                    };
                    if take {
                        best_idx = Some(i);
                        best_len = sep.len();
                    }
                }
            }
            let Some(idx) = best_idx else {
                break;
            };
            let rest = &s[idx + best_len..];
            let Some(j) = rest.find(gclose) else {
                break;
            };
            if j == 0 {
                break;
            }
            let inner = rest[..j].trim();
            if inner.is_empty() {
                break;
            }
            s = inner.to_string();
        }
    }
    let gc = gclose.to_string();
    let g2 = format!("{gclose}{gclose}");
    while s.ends_with(&g2) {
        if let Some(stripped) = s.strip_suffix(&gc) {
            s = stripped.trim_end().to_string();
        } else {
            break;
        }
    }
    s.trim().to_string()
}

pub fn parse_npc_to_npc_lines(
    raw: &str,
    speaker_name: &str,
    listener_name: &str,
) -> Option<(String, String)> {
    let mut picked: Vec<String> = Vec::new();
    for part in raw.split('\n') {
        let line = part.trim();
        if is_npc_npc_meta_or_assistant_line(line) {
            continue;
        }
        picked.push(line.to_string());
        if picked.len() >= 2 {
            break;
        }
    }
    if picked.len() < 2 {
        return None;
    }
    let mut line_a = strip_outer_guillemet_line(&picked[0]);
    let mut line_b = strip_outer_guillemet_line(&picked[1]);
    line_a = strip_leading_speaker_name(&line_a, speaker_name);
    line_b = strip_leading_speaker_name(&line_b, listener_name);
    line_a = collapse_nested_say_guillemets(&line_a);
    line_b = collapse_nested_say_guillemets(&line_b);
    if line_a.is_empty() || line_b.is_empty() {
        return None;
    }
    Some((line_a, line_b))
}

#[derive(Debug, Clone, Default, Deserialize, serde::Serialize)]
pub struct NpcNpcDialogue {
    pub a: String,
    pub b: String,
    #[serde(default)]
    pub anchors: Vec<String>,
}

pub fn parse_npc_npc_dialogue_json(
    raw: &str,
    speaker_name: &str,
    listener_name: &str,
) -> Option<NpcNpcDialogue> {
    let txt = raw.trim();
    if txt.is_empty() {
        return None;
    }
    let start = txt.find('{')?;
    let end = txt.rfind('}')?;
    if end <= start {
        return None;
    }
    let slice = &txt[start..=end];
    let mut d: NpcNpcDialogue = serde_json::from_str(slice).ok()?;
    d.a = collapse_nested_say_guillemets(&strip_leading_speaker_name(
        &strip_outer_guillemet_line(d.a.trim()),
        speaker_name,
    ));
    d.b = collapse_nested_say_guillemets(&strip_leading_speaker_name(
        &strip_outer_guillemet_line(d.b.trim()),
        listener_name,
    ));
    if d.a.is_empty() || d.b.is_empty() {
        return None;
    }
    let mut anchors = Vec::new();
    for mut a in std::mem::take(&mut d.anchors) {
        a = a.trim().to_string();
        if a.is_empty() || is_npc_npc_meta_or_assistant_line(&a) {
            continue;
        }
        let runes: Vec<char> = a.chars().collect();
        if runes.len() > 18 {
            a = runes[..18].iter().collect();
        }
        anchors.push(a);
        if anchors.len() >= 2 {
            break;
        }
    }
    d.anchors = anchors;
    Some(d)
}

/// 呼叫 Ollama 產生 NPC↔NPC 兩句（對齊 Go `CallAITalkNPCToNPC`）。
#[allow(clippy::too_many_arguments)]
pub fn call_ai_talk_npc_to_npc(
    base_url: &str,
    model: &str,
    speaker_name: &str,
    listener_name: &str,
    speaker_backstory: &str,
    listener_backstory: &str,
    room_name: &str,
    time_label: &str,
    recent_room_events: &[String],
    relation_hint: &str,
    npc_npc_memory: &str,
    topic_hint: &str,
    room_description: &str,
    room_tags: &str,
    player_in_room: bool,
) -> anyhow::Result<(String, String, Vec<String>)> {
    let base_url = base_url.trim().trim_end_matches('/');
    let model = model.trim();
    if base_url.is_empty() || model.is_empty() {
        anyhow::bail!("ollama not configured");
    }
    let mut sb = String::new();
    sb.push_str(&npc_to_npc_task_intro());
    sb.push_str(&world_phenomena_cognition_prompt());
    sb.push_str(&npc_to_npc_tone_after_world());
    sb.push_str(&npc_to_npc_rules_block());
    if player_in_room {
        sb.push_str(&npc_to_npc_player_present_note());
    }
    sb.push_str(&npc_to_npc_speaker_label());
    sb.push_str(speaker_name);
    sb.push('\n');
    if !speaker_backstory.is_empty() {
        sb.push_str(&npc_to_npc_identity_prefix());
        sb.push_str(speaker_backstory);
        sb.push('\n');
    }
    sb.push_str(&npc_to_npc_listener_label());
    sb.push_str(listener_name);
    sb.push('\n');
    if !listener_backstory.is_empty() {
        sb.push_str(&npc_to_npc_identity_prefix());
        sb.push_str(listener_backstory);
        sb.push('\n');
    }
    sb.push_str(&sprintf_two_percent_s(
        &npc_to_npc_context_line_fmt(),
        room_name,
        time_label,
    ));
    let mut room_description = room_description.trim().to_string();
    if !room_description.is_empty() {
        let runes: Vec<char> = room_description.chars().collect();
        if runes.len() > 120 {
            let mut suf = npc_to_npc_room_desc_trunc_suffix();
            if suf.is_empty() {
                suf = "\u{2026}".to_string();
            }
            room_description = runes[..120].iter().collect::<String>() + &suf;
        }
        sb.push_str(&npc_to_npc_room_atmosphere_prefix());
        sb.push_str(&room_description);
        sb.push('\n');
    }
    let room_tags = room_tags.trim();
    if !room_tags.is_empty() {
        sb.push_str(&npc_to_npc_room_tags_prefix());
        sb.push_str(room_tags);
        sb.push('\n');
    }
    if !recent_room_events.is_empty() {
        sb.push_str(&npc_to_npc_recent_events_header());
        let bullet = npc_to_npc_recent_event_bullet();
        for ev in recent_room_events {
            let ev = ev.trim();
            if ev.is_empty() {
                continue;
            }
            sb.push_str(&bullet);
            sb.push_str(ev);
            sb.push('\n');
        }
    }
    if !topic_hint.is_empty() {
        sb.push_str(&sprintf_one_percent_s(&npc_to_npc_topic_line_fmt(), topic_hint));
    }
    if !relation_hint.is_empty() {
        sb.push_str(&sprintf_one_percent_s(
            &npc_to_npc_relation_line_fmt(),
            relation_hint,
        ));
    }
    if !npc_npc_memory.is_empty() {
        sb.push_str(&sprintf_one_percent_s(
            &npc_to_npc_memory_line_fmt(),
            npc_npc_memory,
        ));
    }
    let body = json!({
        "model": model,
        "think": false,
        "stream": false,
        "messages": [
            {"role": "system", "content": sb},
            {"role": "user", "content": npc_to_npc_user_message()},
        ],
        "options": {
            "num_predict": 120,
            "temperature": 0.68,
        }
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?;
    let url = format!("{base_url}/api/chat");
    let resp = client.post(&url).json(&body).send()?;
    if !resp.status().is_success() {
        anyhow::bail!("ollama {}", resp.status());
    }
    let out: OllamaChatResponse = resp.json()?;
    let raw = out.message.content.trim();
    if raw.is_empty() {
        anyhow::bail!("empty reply");
    }
    if let Some(d) = parse_npc_npc_dialogue_json(raw, speaker_name, listener_name) {
        return Ok((d.a, d.b, d.anchors));
    }
    if let Some((a, b)) = parse_npc_to_npc_lines(raw, speaker_name, listener_name) {
        return Ok((a, b, Vec::new()));
    }
    let fallback = npc_to_npc_fallback_listener_reply();
    for part in raw.split('\n') {
        let line = part.trim();
        if line.is_empty() || is_npc_npc_meta_or_assistant_line(line) {
            continue;
        }
        let line = collapse_nested_say_guillemets(&strip_leading_speaker_name(
            &strip_outer_guillemet_line(line),
            speaker_name,
        ));
        if !line.is_empty() {
            return Ok((line, fallback, Vec::new()));
        }
    }
    anyhow::bail!("could not parse two lines")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprintf_two_matches_go_order() {
        let t = "A%sB%sC";
        assert_eq!(sprintf_two_percent_s(t, "1", "2"), "A1B2C");
    }

    #[test]
    fn strip_leading_speaker_skips_when_followed_by_colon() {
        let line = "老王：你好";
        assert_eq!(strip_leading_speaker_name(line, "老王"), line);
    }

    #[test]
    fn strip_leading_speaker_strips_when_no_punct() {
        assert_eq!(
            strip_leading_speaker_name("老王你來啦", "老王"),
            "你來啦"
        );
    }

    #[test]
    fn parse_npc_to_npc_two_lines() {
        let raw = "第一句話\n第二句話";
        let (a, b) = parse_npc_to_npc_lines(raw, "", "").expect("ok");
        assert_eq!(a, "第一句話");
        assert_eq!(b, "第二句話");
    }

    #[test]
    fn parse_npc_npc_json_minimal() {
        let raw = r#"{"a":"甲好","b":"乙回"}"#;
        let d = parse_npc_npc_dialogue_json(raw, "", "").expect("ok");
        assert_eq!(d.a, "甲好");
        assert_eq!(d.b, "乙回");
    }

    #[test]
    fn is_meta_line_dash() {
        assert!(is_npc_npc_meta_or_assistant_line("---"));
        assert!(is_npc_npc_meta_or_assistant_line("——開頭"));
    }
}

// 玩家↔NPC／NPC↔NPC 的 prompt 組裝、回應解析與 Ollama /api/chat，對齊既有 ai/talk。

use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use super::prompts::{
    meta_bad_markers, npc_to_npc_collapse_say_separators, npc_to_npc_context_line_fmt,
    npc_to_npc_fallback_listener_reply, npc_to_npc_identity_prefix, npc_to_npc_listener_label,
    npc_to_npc_memory_line_fmt, npc_to_npc_player_present_note, npc_to_npc_recent_event_bullet,
    npc_to_npc_recent_events_header, npc_to_npc_relation_line_fmt,
    npc_to_npc_room_atmosphere_prefix, npc_to_npc_room_desc_trunc_suffix,
    npc_to_npc_room_tags_prefix, npc_to_npc_rules_block, npc_to_npc_speaker_label,
    npc_to_npc_task_intro, npc_to_npc_tone_after_world, npc_to_npc_topic_line_fmt,
    npc_to_npc_user_message, player_npc_behavior_rules, player_npc_identity_prefix,
    player_npc_persona_line, player_npc_room_context_fmt, player_npc_sensitivity_prefix,
    player_npc_space_consistency_rule, player_npc_style_example_bullet,
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

/// 組出玩家↔NPC Talk 的 system 與 user（對齊既有 `PlayerNPCTalkBuildPrompts`）。
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
    if !sensitivity_hint.is_empty() {
        sb.push_str(&player_npc_sensitivity_prefix());
        sb.push_str(sensitivity_hint);
        sb.push('\n');
        sb.push_str("回覆必須控制在 35 字以內，口語自然，嚴禁違背【已拍板決策】的立場。\n");
    }
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

/// 若回覆以「…」包一層，抽出內文（對齊既有 `CallAITalk` 結尾處理）。
pub fn strip_reply_guillemet_outer(reply: &str) -> String {
    let reply = reply.trim();
    let gopen = '\u{300c}';
    let gclose = '\u{300d}';
    if reply.starts_with(gopen)
        && reply.contains(gclose)
        && let Some(i) = reply.find(gclose)
    {
        let start = gopen.len_utf8();
        if i > start {
            return reply[start..i].to_string();
        }
    }
    reply.to_string()
}

pub const LOCAL_DIALOGUE_GPU_URL: &str = "http://127.0.0.1:11434/v1/chat/completions";
pub const LOCAL_DIALOGUE_GPU_MODEL: &str = "google_gemma-4-26B-A4B-it-Q4_1";
pub const LOCAL_DIALOGUE_GPU_TIMEOUT_SECS: u64 = 25;
pub const LOCAL_FAST_GPU_URL: &str = "http://127.0.0.1:11436/v1/chat/completions";
pub const LOCAL_FAST_GPU_MODEL: &str = "/home/tanger/Models/Qwen3-4B-Instruct-2507-Q8_0.gguf";
pub const LOCAL_FAST_GPU_TIMEOUT_SECS: u64 = 8;

fn resolve_dialogue_api_key(endpoint: &str) -> String {
    let lower = endpoint.to_lowercase();
    if lower.contains("127.0.0.1") || lower.contains("localhost") {
        return String::new();
    }
    std::env::var("PLAYER_TALK_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_default()
}
/// 去除思考模型輸出的 `<think>...</think>` 標籤及其內部內容。
pub fn strip_think_blocks(s: &str) -> String {
    let mut result = s.to_string();
    while let Some(start) = result.find("<think>") {
        if let Some(end) = result[start..].find("</think>") {
            result.replace_range(start..start + end + "</think>".len(), "");
        } else {
            result.truncate(start);
            break;
        }
    }
    if let Some(end) = result.find("</think>") {
        result.replace_range(..end + "</think>".len(), "");
    }
    result.trim().to_string()
}

/// 呼叫 OpenAI 相容的 `/v1/chat/completions` API。
/// 支援 `chat_template_kwargs: {"enable_thinking": false}` 防止模型浪費 token 在思考區塊，
/// 並自動去除任何殘存的 `<think>...</think>`。
pub fn call_openai_compatible_completions(
    endpoint_or_base: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_prompt: &str,
    timeout_secs: u64,
    max_tokens: u32,
) -> anyhow::Result<String> {
    let raw = endpoint_or_base.trim().trim_end_matches('/');
    if raw.is_empty() {
        anyhow::bail!("empty endpoint or base url");
    }
    let url = if raw.ends_with("/chat/completions") {
        raw.to_string()
    } else {
        format!("{raw}/chat/completions")
    };
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt},
        ],
        "temperature": 0.85,
        "max_tokens": max_tokens,
        "cache_prompt": true,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });
    if url.to_lowercase().contains("openrouter.ai") {
        body.as_object_mut()
            .expect("object")
            .insert("reasoning".to_string(), json!({ "effort": "none" }));
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    let mut req = client.post(&url).json(&body);
    let key = api_key.trim();
    if !key.is_empty() {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req.send()?;
    if !resp.status().is_success() {
        anyhow::bail!("openai compatible status {}", resp.status());
    }
    let val: serde_json::Value = resp.json()?;
    let content = val
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("empty choices in response"))?;
    let cleaned = strip_think_blocks(content);
    if cleaned.is_empty() {
        anyhow::bail!("empty content after removing think tags");
    }
    Ok(strip_reply_guillemet_outer(&cleaned))
}

/// 呼叫玩家↔NPC 對話生成：
/// 優先相容 OpenAI 端點與 Ollama 端點；若未配置或連線失敗，自動回退至本機極速 GPU 端點 (127.0.0.1:11436)。
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
    let (system_prompt, user_msg) = player_npc_talk_build_prompts(
        player_input,
        npc_backstory,
        npc_memory_snippets,
        style_examples,
        sensitivity_hint,
        room_display_name,
    );

    let base_lower = base_url.to_lowercase();
    let is_openai_compat = base_lower.contains("/v1")
        || base_lower.contains("openrouter")
        || base_lower.contains(":11436")
        || base_lower.contains(":11434");

    if is_openai_compat {
        let ep = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else if base_url.ends_with("/v1") {
            format!("{base_url}/chat/completions")
        } else if (base_url.contains(":11436") || base_url.contains(":11434"))
            && !base_url.contains("/v1")
        {
            format!("{base_url}/v1/chat/completions")
        } else {
            format!("{base_url}/chat/completions")
        };
        let api_key = resolve_dialogue_api_key(&ep);
        let timeout_secs = if ep.contains(":11434") {
            LOCAL_DIALOGUE_GPU_TIMEOUT_SECS
        } else {
            LOCAL_FAST_GPU_TIMEOUT_SECS
        };
        match call_openai_compatible_completions(
            &ep,
            &api_key,
            model,
            &system_prompt,
            &user_msg,
            timeout_secs,
            110,
        ) {
            Ok(reply) => return Ok(reply),
            Err(e) => {
                tracing::warn!("openai-compatible dialogue call failed ({e}), trying local dialogue server");
            }
        }
    } else if !base_url.trim().is_empty() && !model.trim().is_empty() {
        let body = json!({
            "model": model,
            "think": false,
            "stream": false,
            "messages": [
                {"role": "system", "content": &system_prompt},
                {"role": "user", "content": &user_msg},
            ],
            "options": {
                "num_predict": 100,
                "temperature": 0.85,
            }
        });
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(LOCAL_FAST_GPU_TIMEOUT_SECS))
            .build()
        {
            let url = format!("{}/api/chat", base_url.trim().trim_end_matches('/'));
            if let Ok(resp) = client.post(&url).json(&body).send() {
                if resp.status().is_success() {
                    if let Ok(out) = resp.json::<OllamaChatResponse>() {
                        let reply = out.message.content.trim();
                        let cleaned = strip_think_blocks(reply);
                        if !cleaned.is_empty() {
                            return Ok(strip_reply_guillemet_outer(&cleaned));
                        }
                    }
                }
            }
        }
    }

    // 自動回退至本機極速 GPU 伺服器
    call_openai_compatible_completions(
        LOCAL_FAST_GPU_URL,
        "",
        LOCAL_FAST_GPU_MODEL,
        &system_prompt,
        &user_msg,
        LOCAL_FAST_GPU_TIMEOUT_SECS,
        120,
    )
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

/// 解析 NPC↔NPC 生成文字（支援 JSON、雙行、及前綴過濾）。
pub fn parse_npc_npc_response_text(
    raw: &str,
    speaker_name: &str,
    listener_name: &str,
) -> anyhow::Result<(String, String, Vec<String>)> {
    let raw = strip_think_blocks(raw);
    let raw = raw.trim();
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

/// 呼叫 LLM 產生 NPC↔NPC 兩句對話。
/// 支援 OpenAI 與 Ollama 格式；若未配置或呼叫失敗，自動回退至本機極速 GPU (127.0.0.1:11436)。
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
        sb.push_str(&sprintf_one_percent_s(
            &npc_to_npc_topic_line_fmt(),
            topic_hint,
        ));
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

    let user_msg = npc_to_npc_user_message();

    // 優先嘗試本機極速 4B GPU (11436)，NPC↔NPC 快速回應且不阻塞 11434 玩家對話
    if let Ok(raw) = crate::ai::openai_chat::call_openai_compatible_chat(
        LOCAL_FAST_GPU_URL,
        "",
        LOCAL_FAST_GPU_MODEL,
        &sb,
        &user_msg,
        220,
    ) {
        if let Ok(res) = parse_npc_npc_response_text(&raw, speaker_name, listener_name) {
            return Ok(res);
        }
    }
    let base_lower = base_url.to_lowercase();
    let is_openai_compat = base_lower.contains("/v1")
        || base_lower.contains("openrouter")
        || base_lower.contains(":11436")
        || base_lower.contains(":11434");

    if is_openai_compat {
        let ep = if base_url.ends_with("/chat/completions") {
            base_url.to_string()
        } else if base_url.ends_with("/v1") {
            format!("{base_url}/chat/completions")
        } else if (base_url.contains(":11436") || base_url.contains(":11434"))
            && !base_url.contains("/v1")
        {
            format!("{base_url}/v1/chat/completions")
        } else {
            format!("{base_url}/chat/completions")
        };
        let api_key = resolve_dialogue_api_key(&ep);
        let timeout_secs = if ep.contains(":11434") {
            LOCAL_DIALOGUE_GPU_TIMEOUT_SECS
        } else {
            LOCAL_FAST_GPU_TIMEOUT_SECS
        };
        if let Ok(raw) = call_openai_compatible_completions(
            &ep,
            &api_key,
            model,
            &sb,
            &user_msg,
            timeout_secs,
            250,
        ) {
            if let Ok(res) = parse_npc_npc_response_text(&raw, speaker_name, listener_name) {
                return Ok(res);
            }
        }
    } else if !base_url.is_empty() && !model.is_empty() {
        let body = json!({
            "model": model,
            "think": false,
            "stream": false,
            "messages": [
                {"role": "system", "content": &sb},
                {"role": "user", "content": &user_msg},
            ],
            "options": {
                "num_predict": 250,
                "temperature": 0.88,
            }
        });
        if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(LOCAL_FAST_GPU_TIMEOUT_SECS))
            .build()
        {
            let url = format!("{}/api/chat", base_url);
            if let Ok(resp) = client.post(&url).json(&body).send() {
                if resp.status().is_success() {
                    if let Ok(out) = resp.json::<OllamaChatResponse>() {
                        if let Ok(res) = parse_npc_npc_response_text(&out.message.content, speaker_name, listener_name) {
                            return Ok(res);
                        }
                    }
                }
            }
        }
    }

    // 回退至本機極速 GPU 伺服器
    let raw = call_openai_compatible_completions(
        LOCAL_FAST_GPU_URL,
        "",
        LOCAL_FAST_GPU_MODEL,
        &sb,
        &user_msg,
        LOCAL_FAST_GPU_TIMEOUT_SECS,
        250,
    )?;
    parse_npc_npc_response_text(&raw, speaker_name, listener_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprintf_two_matches_legacy_order() {
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
        assert_eq!(strip_leading_speaker_name("老王你來啦", "老王"), "你來啦");
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

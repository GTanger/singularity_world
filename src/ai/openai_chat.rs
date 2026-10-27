// OpenAI 相容 Chat Completions，對齊既有 ai/openai_chat。

use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use super::talk::{strip_reply_guillemet_outer, strip_think_blocks};

#[derive(Debug, Deserialize)]
struct OpenAiChatResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMsg,
}

#[derive(Debug, Deserialize)]
struct OpenAiMsg {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

/// 呼叫 `baseURL/chat/completions`（`baseURL` 須為 `…/v1`，例如 `https://api.openai.com/v1`）。
pub fn call_openai_compatible_chat(
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
    max_tokens: u32,
) -> anyhow::Result<String> {
    let raw = base_url.trim().trim_end_matches('/');
    let model = model.trim();
    if raw.is_empty() || model.is_empty() {
        anyhow::bail!("player talk api not configured");
    }
    let endpoint = if raw.ends_with("/chat/completions") {
        raw.to_string()
    } else {
        format!("{raw}/chat/completions")
    };
    let is_local = raw.contains("127.0.0.1") || raw.contains("localhost");
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_message},
        ],
        "temperature": 0.82,
        "max_tokens": max_tokens,
        "cache_prompt": true,
        "chat_template_kwargs": {
            "enable_thinking": false
        }
    });
    if base_url.to_lowercase().contains("openrouter.ai") {
        body.as_object_mut()
            .expect("object")
            .insert("reasoning".to_string(), json!({ "effort": "none" }));
    }
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(10))
        .build()?;
    let mut req = client.post(&endpoint).json(&body);
    let key = api_key.trim();
    if !is_local && !key.is_empty() {
        req = req.header("Authorization", format!("Bearer {key}"));
    }
    let resp = req.send()?;
    if !resp.status().is_success() {
        anyhow::bail!("chat api {}", resp.status());
    }
    let out: OpenAiChatResponse = resp.json()?;
    let choice = out
        .choices
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty choices"))?;
    let raw_text = choice
        .message
        .content
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| choice.message.reasoning_content.as_deref())
        .unwrap_or("");
    let reply = strip_think_blocks(raw_text.trim());
    if reply.is_empty() {
        anyhow::bail!("empty reply");
    }
    Ok(strip_reply_guillemet_outer(&reply))
}

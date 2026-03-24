// OpenAI 相容 Chat Completions，對齊 Go ai/openai_chat.go。

use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use super::talk::strip_reply_guillemet_outer;

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
    content: String,
}

/// 呼叫 `baseURL/chat/completions`（`baseURL` 須為 `…/v1`，例如 `https://api.openai.com/v1`）。
pub fn call_openai_compatible_chat(
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    user_message: &str,
) -> anyhow::Result<String> {
    let base_url = base_url.trim().trim_end_matches('/');
    let model = model.trim();
    if base_url.is_empty() || model.is_empty() {
        anyhow::bail!("player talk api not configured");
    }
    let endpoint = format!("{base_url}/chat/completions");
    let mut body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_message},
        ],
        "temperature": 0.82,
        "max_tokens": 200,
    });
    if base_url.to_lowercase().contains("openrouter.ai") {
        body.as_object_mut()
            .expect("object")
            .insert(
                "reasoning".to_string(),
                json!({ "effort": "none" }),
            );
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()?;
    let mut req = client.post(&endpoint).json(&body);
    let key = api_key.trim();
    if !key.is_empty() {
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
    let reply = choice.message.content.trim();
    if reply.is_empty() {
        anyhow::bail!("empty reply");
    }
    Ok(strip_reply_guillemet_outer(reply))
}

// OpenAI 相容 Chat Completions，對齊既有 ai/openai_chat。

use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};
use std::time::Duration;

use super::talk::{strip_reply_guillemet_outer, strip_think_blocks};

pub const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const DEFAULT_OPENROUTER_TALK_MODEL: &str = "qwen/qwen3.5-flash-02-23";

static PLAYER_OPENROUTER_KEYS: LazyLock<RwLock<HashMap<String, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 讀取玩家在設定頁綁定的個人 OpenRouter API Key（若未設定或已清除則回傳 `None`，代表走本機模型）。
pub fn get_player_openrouter_key(player_id: &str) -> Option<String> {
    let pid = player_id.trim();
    if pid.is_empty() {
        return None;
    }
    if let Some(cached) = PLAYER_OPENROUTER_KEYS
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .get(pid)
    {
        return cached.clone();
    }
    let mut loaded: Option<String> = None;
    if let Some(store_arc) = crate::store::get_store() {
        let pool_opt = store_arc.read().db_pool.clone();
        if let Some(pool) = pool_opt
            && let Ok(mut conn) = pool.get()
            && let Ok(row_opt) = conn.query_opt(
                "SELECT openrouter_key FROM player_llm_keys WHERE player_id = $1",
                &[&pid],
            )
            && let Some(row) = row_opt
        {
            let k: String = row.get(0);
            let trimmed = k.trim();
            if !trimmed.is_empty() {
                loaded = Some(trimmed.to_string());
            }
        }
    }
    PLAYER_OPENROUTER_KEYS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(pid.to_string(), loaded.clone());
    loaded
}

/// 設定或清除玩家的個人 OpenRouter API Key；回傳更新後是否處於已啟用 OpenRouter 狀態。
pub fn set_player_openrouter_key(player_id: &str, raw_key: &str) -> bool {
    let pid = player_id.trim();
    if pid.is_empty() {
        return false;
    }
    let trimmed = raw_key.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c == '*' || c == '•' || c == '·') {
        return get_player_openrouter_key(pid).is_some();
    }
    let normalized = if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    };
    PLAYER_OPENROUTER_KEYS
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .insert(pid.to_string(), normalized.clone());

    if let Some(store_arc) = crate::store::get_store() {
        let pool_opt = store_arc.read().db_pool.clone();
        if let Some(pool) = pool_opt
            && let Ok(mut conn) = pool.get()
        {
            let db_val = normalized.as_deref().unwrap_or("");
            let _ = conn.execute(
                "INSERT INTO player_llm_keys (player_id, openrouter_key)
                 VALUES ($1, $2)
                 ON CONFLICT (player_id) DO UPDATE SET openrouter_key = EXCLUDED.openrouter_key",
                &[&pid, &db_val],
            );
        }
    }
    normalized.is_some()
}
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
    call_openai_compatible_chat_with_history(
        base_url,
        api_key,
        model,
        system_prompt,
        &[],
        user_message,
        max_tokens,
    )
}

/// 支援原生多輪 `messages`（`system` -> `user`/`assistant` 交替 -> 當前 `user`）之 Chat Completions 呼叫。
pub fn call_openai_compatible_chat_with_history(
    base_url: &str,
    api_key: &str,
    model: &str,
    system_prompt: &str,
    history: &[(String, String)],
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
    }
    ;
    let is_local = raw.contains("127.0.0.1") || raw.contains("localhost");
    let mut messages = Vec::with_capacity(2 + history.len() * 2);
    messages.push(json!({"role": "system", "content": system_prompt}));
    for (p_in, n_rep) in history {
        let p = p_in.trim();
        let clean_r = crate::ai::sanitize::sanitize_npc_dialogue_line(n_rep.trim());
        let r = clean_r.trim();
        if !p.is_empty() && !r.is_empty() {
            messages.push(json!({"role": "user", "content": p}));
            messages.push(json!({"role": "assistant", "content": r}));
        }
    }
    messages.push(json!({"role": "user", "content": user_message}));
    let mut body = json!({
        "model": model,
        "messages": messages,
        "temperature": 0.78,
        "repeat_penalty": 1.12,
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

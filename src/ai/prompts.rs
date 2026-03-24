// LLM 固定文案載入，對齊 Go ai/prompts.go（data/templates/llm_prompts.json）。

use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, RwLock};

pub const DEFAULT_LLM_PROMPTS_PATH: &str = "data/templates/llm_prompts.json";

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PlayerNpcPrompts {
    pub persona_line: String,
    pub room_context_fmt: String,
    pub space_consistency_rule: String,
    pub behavior_rules: String,
    pub traditional_chinese_rule: String,
    pub identity_prefix: String,
    pub sensitivity_prefix: String,
    pub style_examples_header: String,
    pub style_example_bullet: String,
    pub user_plain_fmt: String,
    pub user_with_memory_fmt: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct NpcToNpcPrompts {
    pub task_intro: String,
    pub tone_after_world: String,
    pub rules_block: String,
    pub player_present_note: String,
    pub user_message: String,
    pub speaker_label: String,
    pub listener_label: String,
    pub identity_prefix: String,
    pub context_line_fmt: String,
    pub room_atmosphere_prefix: String,
    pub room_tags_prefix: String,
    pub recent_events_header: String,
    pub recent_event_bullet: String,
    pub topic_line_fmt: String,
    pub relation_line_fmt: String,
    pub memory_line_fmt: String,
    pub room_desc_trunc_suffix: String,
    pub fallback_listener_reply: String,
    pub collapse_say_separators: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct FiltersPrompts {
    pub meta_bad_markers: Vec<String>,
}

/// 與 `llm_prompts.json` 對應（`deny_unknown_fields` 不加，略過 `_說明` 等）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct LlmPromptsJson {
    pub world_phenomena_cognition: String,
    pub player_npc: PlayerNpcPrompts,
    pub npc_to_npc: NpcToNpcPrompts,
    pub filters: FiltersPrompts,
}

struct PromptsState {
    path: String,
    loaded: bool,
    cache: LlmPromptsJson,
}

impl PromptsState {
    fn new() -> Self {
        Self {
            path: DEFAULT_LLM_PROMPTS_PATH.to_string(),
            loaded: false,
            cache: LlmPromptsJson::default(),
        }
    }
}

static STATE: LazyLock<RwLock<PromptsState>> = LazyLock::new(|| RwLock::new(PromptsState::new()));

/// 測試用：重設路徑並清空快取（對齊 Go `SetLLMPromptsPathForTest`）。
pub fn set_llm_prompts_path_for_test(path: impl Into<String>) {
    let mut g = STATE.write().unwrap();
    g.path = path.into();
    g.loaded = false;
    g.cache = LlmPromptsJson::default();
}

/// 載入 JSON（冪等；讀檔或解析失敗時仍標記 `loaded`，與 Go 一致）。
pub fn load_llm_prompts() -> anyhow::Result<()> {
    let mut g = STATE.write().unwrap();
    if g.loaded {
        return Ok(());
    }
    let primary = Path::new(&g.path);
    let raw = fs::read_to_string(primary)
        .or_else(|_| fs::read_to_string(Path::new("..").join(&g.path)));
    let raw = match raw {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("[ai] llm_prompts.json: {} (path={:?})", e, g.path);
            g.loaded = true;
            return Err(e.into());
        }
    };
    match serde_json::from_str::<LlmPromptsJson>(&raw) {
        Ok(root) => {
            g.cache = root;
            g.loaded = true;
            Ok(())
        }
        Err(e) => {
            tracing::warn!("[ai] llm_prompts.json parse: {}", e);
            g.loaded = true;
            Err(e.into())
        }
    }
}

fn read_cache<R>(f: impl FnOnce(&LlmPromptsJson) -> R) -> R {
    let _ = load_llm_prompts();
    let g = STATE.read().unwrap();
    f(&g.cache)
}

/// 世界現象級認知文案（尾端保證換行），對齊 Go `WorldPhenomenaCognitionPrompt`。
pub fn world_phenomena_cognition_prompt() -> String {
    read_cache(|c| {
        let mut s = c.world_phenomena_cognition.trim().to_string();
        if s.is_empty() {
            return String::new();
        }
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s
    })
}

pub fn player_npc_persona_line() -> String {
    read_cache(|c| c.player_npc.persona_line.clone())
}
pub fn player_npc_room_context_fmt() -> String {
    read_cache(|c| c.player_npc.room_context_fmt.clone())
}
pub fn player_npc_space_consistency_rule() -> String {
    read_cache(|c| c.player_npc.space_consistency_rule.clone())
}
pub fn player_npc_behavior_rules() -> String {
    read_cache(|c| c.player_npc.behavior_rules.clone())
}
pub fn player_npc_traditional_rule() -> String {
    read_cache(|c| c.player_npc.traditional_chinese_rule.clone())
}
pub fn player_npc_identity_prefix() -> String {
    read_cache(|c| c.player_npc.identity_prefix.clone())
}
pub fn player_npc_sensitivity_prefix() -> String {
    read_cache(|c| c.player_npc.sensitivity_prefix.clone())
}
pub fn player_npc_style_examples_header() -> String {
    read_cache(|c| c.player_npc.style_examples_header.clone())
}
pub fn player_npc_style_example_bullet() -> String {
    read_cache(|c| c.player_npc.style_example_bullet.clone())
}
pub fn player_npc_user_plain_fmt() -> String {
    read_cache(|c| c.player_npc.user_plain_fmt.clone())
}
pub fn player_npc_user_with_memory_fmt() -> String {
    read_cache(|c| c.player_npc.user_with_memory_fmt.clone())
}

pub fn npc_to_npc_task_intro() -> String {
    read_cache(|c| c.npc_to_npc.task_intro.clone())
}
pub fn npc_to_npc_tone_after_world() -> String {
    read_cache(|c| c.npc_to_npc.tone_after_world.clone())
}
pub fn npc_to_npc_rules_block() -> String {
    read_cache(|c| c.npc_to_npc.rules_block.clone())
}
pub fn npc_to_npc_player_present_note() -> String {
    read_cache(|c| c.npc_to_npc.player_present_note.clone())
}
pub fn npc_to_npc_user_message() -> String {
    read_cache(|c| c.npc_to_npc.user_message.clone())
}
pub fn npc_to_npc_speaker_label() -> String {
    read_cache(|c| c.npc_to_npc.speaker_label.clone())
}
pub fn npc_to_npc_listener_label() -> String {
    read_cache(|c| c.npc_to_npc.listener_label.clone())
}
pub fn npc_to_npc_identity_prefix() -> String {
    read_cache(|c| c.npc_to_npc.identity_prefix.clone())
}
pub fn npc_to_npc_context_line_fmt() -> String {
    read_cache(|c| c.npc_to_npc.context_line_fmt.clone())
}
pub fn npc_to_npc_room_atmosphere_prefix() -> String {
    read_cache(|c| c.npc_to_npc.room_atmosphere_prefix.clone())
}
pub fn npc_to_npc_room_tags_prefix() -> String {
    read_cache(|c| c.npc_to_npc.room_tags_prefix.clone())
}
pub fn npc_to_npc_recent_events_header() -> String {
    read_cache(|c| c.npc_to_npc.recent_events_header.clone())
}
pub fn npc_to_npc_recent_event_bullet() -> String {
    read_cache(|c| c.npc_to_npc.recent_event_bullet.clone())
}
pub fn npc_to_npc_topic_line_fmt() -> String {
    read_cache(|c| c.npc_to_npc.topic_line_fmt.clone())
}
pub fn npc_to_npc_relation_line_fmt() -> String {
    read_cache(|c| c.npc_to_npc.relation_line_fmt.clone())
}
pub fn npc_to_npc_memory_line_fmt() -> String {
    read_cache(|c| c.npc_to_npc.memory_line_fmt.clone())
}
pub fn npc_to_npc_room_desc_trunc_suffix() -> String {
    read_cache(|c| c.npc_to_npc.room_desc_trunc_suffix.clone())
}
pub fn npc_to_npc_fallback_listener_reply() -> String {
    read_cache(|c| c.npc_to_npc.fallback_listener_reply.clone())
}
pub fn npc_to_npc_collapse_say_separators() -> Vec<String> {
    read_cache(|c| c.npc_to_npc.collapse_say_separators.clone())
}

pub fn meta_bad_markers() -> Vec<String> {
    read_cache(|c| c.filters.meta_bad_markers.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn loads_repo_llm_prompts() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let p = root.join(DEFAULT_LLM_PROMPTS_PATH);
        if !p.is_file() {
            return;
        }
        set_llm_prompts_path_for_test(p.to_string_lossy().to_string());
        load_llm_prompts().expect("parse");
        let w = world_phenomena_cognition_prompt();
        assert!(!w.is_empty());
        assert!(w.ends_with('\n'));
        assert!(!player_npc_persona_line().is_empty());
        assert!(!meta_bad_markers().is_empty());
    }
}

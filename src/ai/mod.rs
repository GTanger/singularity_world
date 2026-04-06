// ai 模組 — LLM 相關；逐步對齊既有 ai/。

pub mod openai_chat;
pub mod prompts;
pub mod sanitize;
pub mod scorer;
pub mod talk;

pub use prompts::{
    load_llm_prompts, meta_bad_markers, npc_to_npc_collapse_say_separators,
    npc_to_npc_context_line_fmt, npc_to_npc_fallback_listener_reply, npc_to_npc_identity_prefix,
    npc_to_npc_listener_label, npc_to_npc_memory_line_fmt, npc_to_npc_player_present_note,
    npc_to_npc_recent_event_bullet, npc_to_npc_recent_events_header, npc_to_npc_relation_line_fmt,
    npc_to_npc_room_atmosphere_prefix, npc_to_npc_room_desc_trunc_suffix, npc_to_npc_room_tags_prefix,
    npc_to_npc_rules_block, npc_to_npc_speaker_label, npc_to_npc_task_intro, npc_to_npc_tone_after_world,
    npc_to_npc_topic_line_fmt, npc_to_npc_user_message, player_npc_behavior_rules,
    player_npc_identity_prefix, player_npc_persona_line, player_npc_room_context_fmt,
    player_npc_sensitivity_prefix, player_npc_space_consistency_rule, player_npc_style_example_bullet,
    player_npc_style_examples_header, player_npc_traditional_rule, player_npc_user_plain_fmt,
    player_npc_user_with_memory_fmt, set_llm_prompts_path_for_test, world_phenomena_cognition_prompt,
    DEFAULT_LLM_PROMPTS_PATH, LlmPromptsJson, NpcToNpcPrompts, PlayerNpcPrompts,
};
pub use openai_chat::call_openai_compatible_chat;
pub use scorer::{raw_events_for_dialogue_score, score_npc_dialogue, DialogueScoreDetail};
pub use sanitize::{sanitize_npc_dialogue_line, strip_npc_line_speaker_prefix};
pub use talk::{
    call_ai_talk, call_ai_talk_npc_to_npc, collapse_nested_say_guillemets,
    is_npc_npc_meta_or_assistant_line, parse_npc_npc_dialogue_json, parse_npc_to_npc_lines,
    player_npc_talk_build_prompts, strip_leading_speaker_name, strip_outer_guillemet_line,
    strip_reply_guillemet_outer, NpcNpcDialogue,
};

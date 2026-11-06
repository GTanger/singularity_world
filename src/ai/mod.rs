// ai 模組 — LLM 相關；逐步對齊既有 ai/。

pub mod openai_chat;
pub mod decision_scorer;
pub mod prompts;
pub mod sanitize;
pub mod scorer;
pub mod talk;
pub mod narrative;
pub mod jsonish;

pub use openai_chat::{
    DEFAULT_OPENROUTER_BASE_URL, DEFAULT_OPENROUTER_TALK_MODEL, call_openai_compatible_chat,
    call_openai_compatible_chat_with_history, get_player_openrouter_key, set_player_openrouter_key,
};
pub use decision_scorer::{
    NpcDecisionInput, NpcStanceChoice, NpcStanceDecision, build_decision_prompt,
    score_npc_interaction_stance,
};
pub use prompts::{
    DEFAULT_LLM_PROMPTS_PATH, LlmPromptsJson, NpcToNpcPrompts, PlayerNpcPrompts, load_llm_prompts,
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
    player_npc_user_with_memory_fmt, set_llm_prompts_path_for_test,
    world_phenomena_cognition_prompt,
};
pub use sanitize::{sanitize_npc_dialogue_line, strip_npc_line_speaker_prefix};
pub use scorer::{DialogueScoreDetail, raw_events_for_dialogue_score, score_npc_dialogue};
pub use talk::{
    LOCAL_FAST_GPU_MODEL, LOCAL_FAST_GPU_URL, NpcNpcDialogue, call_ai_talk,
    call_ai_talk_npc_to_npc, collapse_nested_say_guillemets,
    is_npc_npc_meta_or_assistant_line, parse_npc_npc_dialogue_json, parse_npc_to_npc_lines,
    player_npc_talk_build_prompts, strip_leading_speaker_name, strip_outer_guillemet_line,
    strip_reply_guillemet_outer,
};
pub use narrative::{
    call_llm_for_skill_data, compose_combat_bout_from_data, compose_combat_bouts_concurrent,
    compose_craft_highlight_from_data, deterministic_craft_highlight_fallback,
    render_deterministic_bout_from_data, render_deterministic_skill_from_data,
    synthesize_skill_from_data, verify_combat_bout_output, verify_craft_output,
    verify_skill_output, CombatBoutFactPacket, CraftActionFactPacket, SkillDataPacket,
    SynthesizedSkillProse,
};
pub use jsonish::{
    parse_jsonish, parse_jsonish_with_meta, repair_jsonish_str, JsonishFix, JsonishMeta,
};

// npcnpc 模組 — NPC↔NPC 社交輔助、房間事件滑窗、除錯型別（對齊 Go npcnpc/）。

mod helpers;
mod state;
mod trigger;
mod types;

pub use helpers::{
    add_tag, anchor_consistency_check, dedupe_social_context_lines, dyad_relation_hint, has_tag,
    merge_thread_anchors, npc_pair_same_venue, quality_gate_npc_line, remove_tag,
    sentiment_delta_from_dialogue, should_skip_npc_npc_summary_update, topic_mask_for_room,
};
pub use state::{
    clear_last_choice, get_pair_last_talk, inc_stat, last_choice_snapshot, push_room_event,
    recent_room_events, reset_stats, set_last_choice, set_pair_last_talk, stats_snapshot,
};
pub use trigger::try_trigger_npc_npc_in_room;
pub use types::{DebugResp, LastSocialChoice, PairDebug, RumorDebugItem};

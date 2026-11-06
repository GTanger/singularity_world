//! `do_action` — Talk 分支：LLM 對話（走 Ollama 或 OpenAI-compatible），fallback pool。

use crate::ai::{
    call_ai_talk, call_openai_compatible_chat_with_history,
    decision_scorer::{NpcDecisionInput, NpcStanceChoice, score_npc_interaction_stance},
    player_npc_talk_build_prompts,
};
use crate::db::{
    self, FAV_TALK, adjust_favorability, get_disposition, get_room_name, pick_style_examples,
};
use crate::entity::{Character, EntityKind, Verb};
use crate::event;
use crate::game;

use super::super::conversation::{flush_conversation_and_append, get_recent_turns};
use super::super::handler::{WsConnection, push_refresh};
use super::super::protocol::{ActionResultMsg, ClientMsg};
use super::display_target_name;

include!("talk/stance.rs");
include!("talk/cognitive.rs");
include!("talk/venue.rs");
include!("talk/effects.rs");
include!("talk/execute.rs");
include!("talk/tests.rs");

//! 同房 NPC↔NPC 觸發（對齊既有 `npcnpc/trigger`）。

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::Rng;

use crate::ai::{
    call_ai_talk_npc_to_npc, raw_events_for_dialogue_score, sanitize_npc_dialogue_line,
    score_npc_dialogue, strip_npc_line_speaker_prefix,
};
use crate::config::{Server, sim};
use crate::db::{
    self, get_npc_npc_conversation_summary, insert_npc_npc_dialogue_archival,
    mark_rumor_used_by_text, penalize_rumor_by_text, recent_npc_npc_archival_lines_for_entity,
    set_npc_npc_conversation_summary, set_npc_npc_dyad, set_npc_npc_thread, top_npc_rumors,
    upsert_lexicon_term, upsert_npc_rumor,
};
use crate::entity::EntityKind;
use crate::gametext;
use crate::npc::{
    find_npc_npc_topic_id_by_hint, get_npc_npc_topic_by_id, pick_random_npc_npc_topic_for_pair,
};
use crate::server::SessionStore;
use crate::store::{NpcRumor, NpcThread};

use super::helpers::{
    add_tag, anchor_consistency_check, dedupe_social_context_lines, dyad_relation_hint,
    merge_thread_anchors, npc_pair_same_venue, quality_gate_npc_line, remove_tag,
    sentiment_delta_from_dialogue, should_skip_npc_npc_summary_update, topic_mask_for_room,
};
use super::state::{get_pair_last_talk, inc_stat, set_last_choice, set_pair_last_talk};
use super::types::LastSocialChoice;


include!("trigger/helpers.rs");
include!("trigger/pair_picker.rs");
include!("trigger/dialogue.rs");
include!("trigger/outcome.rs");
include!("trigger/tests.rs");

/// 在指定房嘗試觸發一次 NPC 間 AI 對話（對齊 `TryTriggerNpcNpcInRoom`）。
#[allow(clippy::too_many_lines)]
pub fn try_trigger_npc_npc_in_room(
    session_store: &SessionStore,
    cfg: &Server,
    room_id: &str,
    topic_hint_in: &str,
    game_hour: i32,
) -> bool {
    inc_stat("attempt");
    if cfg.ollama_disable {
        inc_stat("fail_disabled");
        return false;
    }
    let cd = Duration::from_secs(
        sim()
            .npc_npc_social
            .room_recent_player_talk_cooldown_sec
            .max(0) as u64,
    );
    if session_store.room_has_player_with_recent_talk(room_id, cd) {
        inc_stat("fail_recent_player_talk");
        return false;
    }
    let Ok(entities) = db::get_entities_in_room(room_id, game_hour) else {
        inc_stat("fail_not_enough_npc");
        return false;
    };
    let is_night = game_hour >= 22 || game_hour < 5;
    let npcs: Vec<_> = entities
        .into_iter()
        .filter(|e| {
            if !matches!(e.kind, EntityKind::Npc) {
                return false;
            }
            let act = &e.current_activity;
            if act.contains("安歇")
                || act.contains("熟睡")
                || act.contains("就寢")
                || act.contains("熄燈")
            {
                return false;
            }
            if is_night {
                let slot = crate::store::wenyao_household_slot_for(&e.id);
                if slot.home_name == room_id
                    || room_id.contains("居")
                    || room_id.contains("宅")
                    || room_id.contains("舍")
                    || room_id.contains("齋")
                {
                    return false;
                }
            }
            true
        })
        .collect();
    if npcs.len() < 2 {
        inc_stat("fail_not_enough_npc");
        return false;
    }
    let now_unix = now_unix();
    let Some(mut pair) = select_best_npc_pair(&npcs, now_unix) else {
        return false;
    };
    let Some(dialogue) = conduct_npc_dialogue(
        session_store,
        cfg,
        room_id,
        topic_hint_in,
        game_hour,
        now_unix,
        &mut pair,
    ) else {
        return false;
    };
    commit_and_broadcast_dialogue(
        session_store,
        room_id,
        now_unix,
        pair,
        dialogue,
    )
}

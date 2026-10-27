use std::collections::HashMap;

use crate::db::{load_pending_journeys, set_entity_journey};
use crate::npc::IntentType;
use crate::pg::living_world::ResidentContinuity;
use crate::world::journey::EntityJourney;

use super::NpcState;

pub(super) fn persist_journey(entity_id: &str, state: &NpcState, consequence: &str, now_unix: i64) {
    let reason = state.last_intent.unwrap_or(IntentType::Idle).as_key();
    set_entity_journey(EntityJourney {
        entity_id: entity_id.to_string(),
        reason: reason.to_string(),
        destination_place_id: state.destination_place_id.clone(),
        planned_path: state.path.clone(),
        consequence: consequence.to_string(),
        updated_at: now_unix,
    });
}

pub(super) fn persist_continuity(entity_id: &str, state: &NpcState, now_unix: i64) {
    let Some(pool) = crate::pg::pool() else {
        return;
    };
    let continuity = ResidentContinuity {
        entity_id: entity_id.to_string(),
        control_mode: "autonomous".to_string(),
        life_status: "alive".to_string(),
        current_intent: state
            .last_intent
            .map(|intent| intent.as_key().to_string())
            .unwrap_or_else(|| "reassess".to_string()),
        intent_reason: state.intent_reason.clone(),
        destination_place_id: state.destination_place_id.clone(),
        blocked_by: state.blocked_by.clone(),
        intent_started_at: state.intent_started_at,
        next_wake_at: Some(state.next_wake_at),
        last_action_id: None,
        revision: state.revision,
        updated_at: now_unix,
        source_kind: "runtime_decision".to_string(),
        source_ref: "white_tower_living_world_v1".to_string(),
    };
    if let Err(error) = crate::pg::living_world::upsert_continuity(&pool, &continuity) {
        tracing::warn!("resident continuity persist failed {entity_id}: {error}");
    }
}

pub(super) fn restore_states(now_unix: i64) -> HashMap<String, NpcState> {
    let pending = load_pending_journeys().unwrap_or_default();
    let continuities = crate::pg::pool()
        .and_then(|pool| crate::pg::living_world::load_autonomous(&pool).ok())
        .unwrap_or_default();
    if continuities.is_empty() {
        return pending
            .into_iter()
            .filter_map(|(entity_id, journey)| {
                let intent = intent_from_key(&journey.reason)?;
                Some((
                    entity_id,
                    NpcState {
                        path: journey.planned_path,
                        destination_place_id: journey.destination_place_id,
                        last_intent: Some(intent),
                        intent_reason: journey.consequence,
                        intent_started_at: journey.updated_at,
                        next_wake_at: now_unix,
                        blocked_by: None,
                        revision: 0,
                    },
                ))
            })
            .collect();
    }
    continuities
        .into_iter()
        .map(|(entity_id, continuity)| {
            let journey = pending.get(&entity_id);
            (
                entity_id,
                NpcState {
                    path: journey
                        .map(|journey| journey.planned_path.clone())
                        .unwrap_or_default(),
                    destination_place_id: continuity.destination_place_id,
                    last_intent: intent_from_key(&continuity.current_intent),
                    intent_reason: continuity.intent_reason,
                    intent_started_at: continuity.intent_started_at,
                    next_wake_at: continuity.next_wake_at.unwrap_or(now_unix),
                    blocked_by: continuity.blocked_by,
                    revision: continuity.revision,
                },
            )
        })
        .collect()
}

fn intent_from_key(key: &str) -> Option<IntentType> {
    match key {
        "seek_job" => Some(IntentType::SeekJob),
        "beg" => Some(IntentType::Beg),
        "trade" => Some(IntentType::Trade),
        "gather" => Some(IntentType::Gather),
        "wander" => Some(IntentType::Wander),
        "work" => Some(IntentType::Work),
        "idle" => Some(IntentType::Idle),
        "socialize" => Some(IntentType::Socialize),
        _ => None,
    }
}

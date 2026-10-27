//! 人物旅程診斷與續行門面。

use std::collections::HashMap;

use crate::world::journey::{EntityJourney, PersonWorldState};

pub fn set_entity_journey(journey: EntityJourney) {
    crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityJourney(journey));
}

pub fn get_entity_journey(entity_id: &str) -> anyhow::Result<Option<EntityJourney>> {
    let pool = crate::pg::pool().ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    crate::pg::world_journey::get(&pool, entity_id)
}

pub fn load_pending_journeys() -> anyhow::Result<HashMap<String, EntityJourney>> {
    let pool = crate::pg::pool().ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    crate::pg::world_journey::load_pending(&pool)
}

pub fn get_person_world_state(entity_id: &str) -> anyhow::Result<Option<PersonWorldState>> {
    let Some(store) = crate::store::get_store() else {
        return Ok(None);
    };
    let Some(entity) = store.read().get_entity(entity_id) else {
        return Ok(None);
    };
    let journey = get_entity_journey(entity_id)?.unwrap_or(EntityJourney {
        entity_id: entity_id.to_string(),
        reason: String::new(),
        destination_place_id: None,
        planned_path: Vec::new(),
        consequence: String::new(),
        updated_at: 0,
    });
    Ok(Some(PersonWorldState {
        entity_id: entity.id,
        place_id: entity.place_id.unwrap_or_default(),
        current_activity: entity.current_activity,
        reason: journey.reason,
        destination_place_id: journey.destination_place_id,
        planned_path: journey.planned_path,
        consequence: journey.consequence,
        updated_at: journey.updated_at,
    }))
}

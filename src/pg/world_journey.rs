//! 人物旅程狀態的 PostgreSQL 主資料存取。

use std::collections::HashMap;

use r2d2_postgres::postgres::GenericClient;

use crate::store::sql::DbPool;
use crate::world::journey::EntityJourney;

pub fn ensure_schema(pool: &DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS entity_journeys (
            entity_id TEXT PRIMARY KEY,
            reason TEXT NOT NULL,
            destination_place_id TEXT,
            planned_path TEXT NOT NULL DEFAULT '[]',
            consequence TEXT NOT NULL DEFAULT '',
            updated_at BIGINT NOT NULL
        )",
        &[],
    )?;
    Ok(())
}

pub fn upsert(pool: &DbPool, journey: &EntityJourney) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    upsert_in(&mut *conn, journey)
}

pub(crate) fn upsert_in(
    client: &mut impl GenericClient,
    journey: &EntityJourney,
) -> anyhow::Result<()> {
    let path = serde_json::to_string(&journey.planned_path)?;
    client.execute(
        "INSERT INTO entity_journeys
         (entity_id,reason,destination_place_id,planned_path,consequence,updated_at)
         VALUES ($1,$2,$3,$4,$5,$6)
         ON CONFLICT(entity_id) DO UPDATE SET reason=EXCLUDED.reason,
         destination_place_id=EXCLUDED.destination_place_id,planned_path=EXCLUDED.planned_path,
         consequence=EXCLUDED.consequence,updated_at=EXCLUDED.updated_at",
        &[
            &journey.entity_id,
            &journey.reason,
            &journey.destination_place_id,
            &path,
            &journey.consequence,
            &journey.updated_at,
        ],
    )?;
    Ok(())
}

pub fn get(pool: &DbPool, entity_id: &str) -> anyhow::Result<Option<EntityJourney>> {
    let mut conn = pool.get()?;
    conn.query_opt(
        "SELECT entity_id,reason,destination_place_id,planned_path,consequence,updated_at
         FROM entity_journeys WHERE entity_id=$1",
        &[&entity_id],
    )?
    .map(journey_from_row)
    .transpose()
}

pub fn load_pending(pool: &DbPool) -> anyhow::Result<HashMap<String, EntityJourney>> {
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT entity_id,reason,destination_place_id,planned_path,consequence,updated_at
         FROM entity_journeys WHERE planned_path <> '[]' ORDER BY entity_id",
        &[],
    )?;
    let mut journeys = HashMap::new();
    for row in rows {
        let journey = journey_from_row(row)?;
        journeys.insert(journey.entity_id.clone(), journey);
    }
    Ok(journeys)
}

fn journey_from_row(row: r2d2_postgres::postgres::Row) -> anyhow::Result<EntityJourney> {
    let path: String = row.get(3);
    Ok(EntityJourney {
        entity_id: row.get(0),
        reason: row.get(1),
        destination_place_id: row.get(2),
        planned_path: serde_json::from_str(&path)?,
        consequence: row.get(4),
        updated_at: row.get(5),
    })
}

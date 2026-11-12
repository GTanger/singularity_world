//! Entity PostgreSQL upsert；單筆熱路徑與批次世界結算共用同一 SQL。

use r2d2_postgres::postgres::GenericClient;

use crate::store::Entity;
use crate::store::sql::DbPool;

const UPSERT_SQL: &str = "
INSERT INTO entities (id,kind,display_char,x,y,move_state,target_x,target_y,
 walk_or_run,move_started_at,vit,qi,dex,magnesium,last_observed_at,created_at,
 gender,soul_seed,display_title,activated_nodes,equipment_slots,inventory,disposition,
 current_activity,place_id,grid_x,grid_y,thoughts,grid_z,inventory_seq)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
 $20,$21,$22,$23,$24,$25,$26,$27,$28,$29,$30)
ON CONFLICT (id) DO UPDATE SET
 kind=EXCLUDED.kind,display_char=EXCLUDED.display_char,x=EXCLUDED.x,y=EXCLUDED.y,
 move_state=EXCLUDED.move_state,target_x=EXCLUDED.target_x,target_y=EXCLUDED.target_y,
 walk_or_run=EXCLUDED.walk_or_run,move_started_at=EXCLUDED.move_started_at,
 vit=EXCLUDED.vit,qi=EXCLUDED.qi,dex=EXCLUDED.dex,magnesium=EXCLUDED.magnesium,
 last_observed_at=EXCLUDED.last_observed_at,created_at=EXCLUDED.created_at,
 gender=EXCLUDED.gender,soul_seed=EXCLUDED.soul_seed,display_title=EXCLUDED.display_title,
 activated_nodes=EXCLUDED.activated_nodes,equipment_slots=EXCLUDED.equipment_slots,
 inventory=CASE WHEN EXCLUDED.inventory_seq >= entities.inventory_seq THEN EXCLUDED.inventory ELSE entities.inventory END,
 inventory_seq=GREATEST(entities.inventory_seq, EXCLUDED.inventory_seq),grid_z=EXCLUDED.grid_z,
 disposition=EXCLUDED.disposition,
 current_activity=EXCLUDED.current_activity,place_id=EXCLUDED.place_id,grid_x=EXCLUDED.grid_x,grid_y=EXCLUDED.grid_y,thoughts=EXCLUDED.thoughts";

pub fn upsert(pool: &DbPool, entity: &Entity) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    execute_upsert(&mut *conn, entity)
}

pub fn upsert_batch(pool: &DbPool, entities: &[Entity]) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    for entity in entities {
        execute_upsert(&mut tx, entity)?;
    }
    tx.commit()?;
    Ok(())
}

fn execute_upsert(client: &mut impl GenericClient, entity: &Entity) -> anyhow::Result<()> {
    let thoughts = serde_json::to_string(&entity.thoughts)?;
    client.execute(
        UPSERT_SQL,
        &[
            &entity.id,
            &entity.kind,
            &entity.display_char,
            &entity.x,
            &entity.y,
            &entity.move_state,
            &entity.target_x,
            &entity.target_y,
            &entity.walk_or_run,
            &entity.move_started_at,
            &entity.vit,
            &entity.qi,
            &entity.dex,
            &entity.magnesium,
            &entity.last_observed_at,
            &entity.created_at,
            &entity.gender,
            &entity.soul_seed,
            &entity.display_title,
            &entity.activated_nodes,
            &entity.equipment_slots,
            &entity.inventory,
            &entity.disposition,
            &entity.current_activity,
            &entity.place_id,
            &entity.grid_x,
            &entity.grid_y,
            &thoughts,
            &entity.grid_z,
            &entity.inventory_seq,
        ],
    )?;
    Ok(())
}

pub fn upsert_async(entity: Entity) {
    super::writer::submit(super::writer::WriteOp::UpsertEntity(entity));
}

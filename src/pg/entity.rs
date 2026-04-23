//! Entity PG upsert — 繞過 store lock、獨立 IO 路徑。
//!
//! 從 Store::pg_upsert_entity 遷出。舊架構每次 update_entity 都在 store write lock
//! 下跑 PG INSERT/UPDATE 幾百個欄位、再加 persist_entities 寫整份 entities.json 到
//! 磁碟。玩家/NPC 每次移動都走這路徑，writer 霸佔 store lock 把 login 卡死。
//!
//! 本模組 pub fn 直走 `&DbPool`；callsite 可在 drop store lock 之後再呼叫。

use crate::store::Entity;
use crate::store::sql::DbPool;

pub fn upsert(pool: &DbPool, e: &Entity) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO entities (id, kind, display_char, x, y, move_state, target_x, target_y,
            walk_or_run, move_started_at, vit, qi, dex, magnesium, last_observed_at,
            created_at, gender, soul_seed, display_title, activated_nodes,
            equipment_slots, inventory, disposition, current_activity,
            grid_x, grid_y)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25,$26)
         ON CONFLICT (id) DO UPDATE SET
            kind=EXCLUDED.kind, display_char=EXCLUDED.display_char, x=EXCLUDED.x, y=EXCLUDED.y,
            move_state=EXCLUDED.move_state, target_x=EXCLUDED.target_x, target_y=EXCLUDED.target_y,
            walk_or_run=EXCLUDED.walk_or_run, move_started_at=EXCLUDED.move_started_at,
            vit=EXCLUDED.vit, qi=EXCLUDED.qi, dex=EXCLUDED.dex, magnesium=EXCLUDED.magnesium,
            last_observed_at=EXCLUDED.last_observed_at, created_at=EXCLUDED.created_at,
            gender=EXCLUDED.gender, soul_seed=EXCLUDED.soul_seed, display_title=EXCLUDED.display_title,
            activated_nodes=EXCLUDED.activated_nodes, equipment_slots=EXCLUDED.equipment_slots,
            inventory=EXCLUDED.inventory, disposition=EXCLUDED.disposition,
            current_activity=EXCLUDED.current_activity,
            grid_x=EXCLUDED.grid_x, grid_y=EXCLUDED.grid_y",
        &[&e.id, &e.kind, &e.display_char, &e.x, &e.y, &e.move_state,
          &e.target_x, &e.target_y, &e.walk_or_run, &e.move_started_at,
          &e.vit, &e.qi, &e.dex, &e.magnesium, &e.last_observed_at,
          &e.created_at, &e.gender, &e.soul_seed, &e.display_title,
          &e.activated_nodes, &e.equipment_slots, &e.inventory, &e.disposition,
          &e.current_activity, &e.grid_x, &e.grid_y],
    )?;
    Ok(())
}

/// 提交 entity upsert 到 PG writer service。
/// caller 可在 store write lock 內呼叫——submit 只是 channel send（μs 級）。
/// Queue 滿時 submit 會阻塞（backpressure，不會無界堆積 thread）。
/// 寫入順序 = submit 順序（單 worker 保證）。
pub fn upsert_async(e: Entity) {
    super::writer::submit(super::writer::WriteOp::UpsertEntity(e));
}

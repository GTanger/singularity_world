//! Event log PG CRUD — 繞過 store RwLock 直打 PG。
//!
//! 從 Store::append_event / last_by_entity / events_in_range / recent_by_entity 遷出。
//! append_event 是**最高頻的寫入路徑**（每個 NPC 動作、玩家行為、戰鬥都寫），
//! 原本拿 store write lock 做 blocking PG INSERT 是 login 卡死的首要元兇。

use crate::store::EventEntry;
use crate::store::sql::DbPool;

pub fn append(
    pool: &DbPool,
    at: i64,
    entity_id: &str,
    event_type: &str,
    payload: &str,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO event_log (entity_id, event_type, payload, created_at) VALUES ($1, $2, $3, $4)",
        &[&entity_id, &event_type, &payload, &at],
    )?;
    Ok(())
}

pub fn last_by_entity(pool: &DbPool, entity_id: &str, event_type: &str, at: i64) -> String {
    if let Ok(mut conn) = pool.get() {
        let row = conn.query_opt(
            "SELECT payload FROM event_log \
             WHERE entity_id = $1 AND event_type = $2 AND created_at <= $3 \
             ORDER BY created_at DESC, id DESC LIMIT 1",
            &[&entity_id, &event_type, &at],
        );
        if let Ok(Some(row)) = row {
            return row.get::<_, String>(0);
        }
    }
    String::new()
}

pub fn events_in_range(
    pool: &DbPool,
    entity_id: &str,
    from_at: i64,
    to_at: i64,
) -> Vec<EventEntry> {
    let mut results = Vec::new();
    if let Ok(mut conn) = pool.get()
        && let Ok(rows) = conn.query(
            "SELECT created_at, entity_id, event_type, payload FROM event_log \
             WHERE entity_id = $1 AND created_at >= $2 AND created_at <= $3 \
             ORDER BY created_at ASC, id ASC",
            &[&entity_id, &from_at, &to_at],
        )
    {
        for row in rows {
            results.push(EventEntry {
                at: row.get(0),
                entity_id: row.get(1),
                event_type: row.get(2),
                payload: row.get(3),
            });
        }
    }
    results
}

/// 由新到舊
pub fn recent_by_entity(pool: &DbPool, entity_id: &str, n: usize) -> Vec<EventEntry> {
    let mut results = Vec::new();
    if let Ok(mut conn) = pool.get()
        && let Ok(rows) = conn.query(
            "SELECT created_at, entity_id, event_type, payload FROM event_log \
             WHERE entity_id = $1 \
             ORDER BY created_at DESC, id DESC LIMIT $2",
            &[&entity_id, &(n as i64)],
        )
    {
        for row in rows {
            results.push(EventEntry {
                at: row.get(0),
                entity_id: row.get(1),
                event_type: row.get(2),
                payload: row.get(3),
            });
        }
    }
    results
}

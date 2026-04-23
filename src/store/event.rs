//! store 事件紀錄 — event_log 表的寫入（走 writer queue）與讀取（同步查 PG）。
//!
//! 寫入：`append_event` 不阻塞，立刻回。
//! 讀取：`last_by_entity` / `events_in_range` / `recent_by_entity` 直接走 db_pool 同步查。

use super::{EventEntry, Store};

impl Store {
    /// PG 寫入走 writer queue，不阻塞任何 lock。
    pub fn append_event(
        &self,
        at: i64,
        entity_id: &str,
        event_type: &str,
        payload: &str,
    ) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::AppendEvent {
            at,
            entity_id: entity_id.to_string(),
            event_type: event_type.to_string(),
            payload: payload.to_string(),
        });
        Ok(())
    }

    pub fn last_by_entity(&self, entity_id: &str, event_type: &str, at: i64) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let row = conn.query_opt(
                "SELECT payload FROM event_log
                     WHERE entity_id = $1 AND event_type = $2 AND created_at <= $3
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
        &self,
        entity_id: &str,
        from_at: i64,
        to_at: i64,
    ) -> Vec<EventEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            if let Ok(rows) = conn.query(
                "SELECT created_at, entity_id, event_type, payload FROM event_log
                     WHERE entity_id = $1 AND created_at >= $2 AND created_at <= $3
                     ORDER BY created_at ASC, id ASC",
                &[&entity_id, &from_at, &to_at],
            ) {
                for row in rows {
                    results.push(EventEntry {
                        at: row.get(0),
                        entity_id: row.get(1),
                        event_type: row.get(2),
                        payload: row.get(3),
                    });
                }
            }
        }
        results
    }

    /// 由新到舊（對齊既有 `RecentByEntity`）。
    pub fn recent_by_entity(&self, entity_id: &str, n: usize) -> Vec<EventEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            let n_i64 = n as i64;
            if let Ok(rows) = conn.query(
                "SELECT created_at, entity_id, event_type, payload FROM event_log
                     WHERE entity_id = $1
                     ORDER BY created_at DESC, id DESC LIMIT $2",
                &[&entity_id, &n_i64],
            ) {
                for row in rows {
                    results.push(EventEntry {
                        at: row.get(0),
                        entity_id: row.get(1),
                        event_type: row.get(2),
                        payload: row.get(3),
                    });
                }
            }
        }
        results
    }
}

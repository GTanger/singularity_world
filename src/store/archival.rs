//! store 長期記憶 — archival 表的讀寫、修剪。寫入走 writer queue，讀取同步查 PG。

use super::{ArchivalEntry, Store};

impl Store {
    pub fn append_archival(&mut self, entry: ArchivalEntry) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::AppendArchival(entry));
        Ok(())
    }

    pub fn get_archival_by_entity(&self, entity_id: &str) -> Vec<ArchivalEntry> {
        let mut results = Vec::new();
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            if let Ok(rows) = conn.query(
                "SELECT entity_id, content, tag, created_at FROM archival WHERE entity_id = $1 ORDER BY created_at ASC, id ASC",
                &[&entity_id],
            ) {
                for row in rows {
                    results.push(ArchivalEntry {
                        entity_id: row.get(0),
                        content: row.get(1),
                        tag: row.get(2),
                        created_at: row.get(3),
                    });
                }
            }
        }
        results
    }

    pub fn trim_archival_per_entity(&mut self, max: usize) {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::TrimArchival {
            max: max as i64,
        });
    }
}

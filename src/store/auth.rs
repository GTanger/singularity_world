//! store 密碼認證 — auth 表的讀寫。寫入走 writer queue，讀取同步查 PG。

use super::Store;

impl Store {
    pub fn set_auth(&mut self, entity_id: &str, password_hash: &str) -> anyhow::Result<()> {
        crate::pg::writer::submit(crate::pg::writer::WriteOp::SetAuth {
            entity_id: entity_id.to_string(),
            password_hash: password_hash.to_string(),
        });
        Ok(())
    }

    pub fn get_auth(&self, entity_id: &str) -> String {
        if let Some(pool) = &self.db_pool
            && let Ok(mut conn) = pool.get()
        {
            if let Ok(Some(row)) = conn.query_opt(
                "SELECT password_hash FROM auth WHERE entity_id = $1",
                &[&entity_id],
            ) {
                return row.get::<_, String>(0);
            }
        }
        String::new()
    }
}

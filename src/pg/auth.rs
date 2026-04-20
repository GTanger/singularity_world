//! Auth PG CRUD — 繞過 store RwLock 直打 PG。
//!
//! 從 Store::set_auth / Store::get_auth 遷出。
//! 原本每次 set_auth 在 store write lock 下做 PG INSERT、login 同時的 get_entity
//! read 排隊等，是 2026-04-20 login 卡 10s bug 的寫端源頭。

use crate::store::sql::DbPool;

pub fn set(pool: &DbPool, entity_id: &str, password_hash: &str) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "INSERT INTO auth (entity_id, password_hash) VALUES ($1, $2) \
         ON CONFLICT(entity_id) DO UPDATE SET password_hash=EXCLUDED.password_hash",
        &[&entity_id, &password_hash],
    )?;
    Ok(())
}

pub fn get(pool: &DbPool, entity_id: &str) -> String {
    if let Ok(mut conn) = pool.get()
        && let Ok(Some(row)) = conn.query_opt(
            "SELECT password_hash FROM auth WHERE entity_id = $1",
            &[&entity_id],
        )
    {
        return row.get::<_, String>(0);
    }
    String::new()
}

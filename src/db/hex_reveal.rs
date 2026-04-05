//! Hex 格網：多人各自「已揭露」格（PostgreSQL）。世界格網主資料在 `hex_world` 表，另備份 `data/hex/grid.json`。

use crate::store;

/// 標記該玩家已觀測此格。回傳 **是否為首次寫入**（`false` 表示早於本次已揭露）。
pub fn mark_player_hex_revealed(entity_id: &str, cell_id: &str) -> anyhow::Result<bool> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;
    let n = conn.execute(
        "INSERT INTO hex_player_reveal (entity_id, cell_id, revealed_at)
         VALUES ($1, $2, $3)
         ON CONFLICT (entity_id, cell_id) DO NOTHING",
        &[&entity_id, &cell_id, &now],
    )?;
    Ok(n > 0)
}

pub fn is_player_hex_revealed(entity_id: &str, cell_id: &str) -> anyhow::Result<bool> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let row = conn.query_one(
        "SELECT COUNT(*)::bigint FROM hex_player_reveal WHERE entity_id = $1 AND cell_id = $2",
        &[&entity_id, &cell_id],
    )?;
    let n: i64 = row.get(0);
    Ok(n > 0)
}

/// 依揭露時間排序，供客戶端同步迷霧。
pub fn list_player_hex_revealed(
    entity_id: &str,
    limit: i64,
    offset: i64,
) -> anyhow::Result<Vec<String>> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let rows = conn.query(
        "SELECT cell_id FROM hex_player_reveal
         WHERE entity_id = $1
         ORDER BY revealed_at ASC, cell_id ASC
         LIMIT $2 OFFSET $3",
        &[&entity_id, &limit, &offset],
    )?;
    Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
}

pub fn count_player_hex_revealed(entity_id: &str) -> anyhow::Result<i64> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let row = conn.query_one(
        "SELECT COUNT(*)::bigint FROM hex_player_reveal WHERE entity_id = $1",
        &[&entity_id],
    )?;
    Ok(row.get(0))
}

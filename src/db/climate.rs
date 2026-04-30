//! 區域氣候系統 — PostgreSQL 持久層（`region_climate` 表）。
//!
//! M3 migration：`CREATE TABLE IF NOT EXISTS region_climate` 在每次呼叫時自動確保存在。

use crate::store;

/// 區域氣候資料列。
#[derive(Debug, Clone)]
pub struct RegionClimate {
    pub region_id: String,
    /// `clear` / `rain` / `wind` / `plum_rain` / `snow`
    pub event_type: String,
    pub started_at_unix: i64,
    /// 散文池抽取種子（前端用）
    pub prose_seed: i64,
}

const CREATE_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS region_climate (
        region_id       TEXT   PRIMARY KEY,
        event_type      TEXT   NOT NULL DEFAULT 'clear',
        started_at_unix BIGINT NOT NULL DEFAULT 0,
        prose_seed      BIGINT NOT NULL DEFAULT 0
    )
";

/// 查詢指定區域的氣候（表不存在時自動建立）。
pub fn get_region_climate(region_id: &str) -> anyhow::Result<Option<RegionClimate>> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    let rows = conn.query(
        "SELECT region_id, event_type, started_at_unix, prose_seed
         FROM region_climate WHERE region_id = $1",
        &[&region_id],
    )?;

    if rows.is_empty() {
        return Ok(None);
    }
    let row = &rows[0];
    Ok(Some(RegionClimate {
        region_id: row.get(0),
        event_type: row.get(1),
        started_at_unix: row.get(2),
        prose_seed: row.get(3),
    }))
}

/// 更新或插入區域氣候（simulation tick 呼叫）。
pub fn upsert_region_climate(climate: &RegionClimate) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    conn.execute(
        "INSERT INTO region_climate (region_id, event_type, started_at_unix, prose_seed)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (region_id) DO UPDATE SET
            event_type = EXCLUDED.event_type,
            started_at_unix = EXCLUDED.started_at_unix,
            prose_seed = EXCLUDED.prose_seed",
        &[
            &climate.region_id,
            &climate.event_type,
            &climate.started_at_unix,
            &climate.prose_seed,
        ],
    )?;
    Ok(())
}

/// 取得所有區域氣候（simulation tick 批量更新用）。
pub fn list_region_climates() -> anyhow::Result<Vec<RegionClimate>> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    conn.execute(CREATE_TABLE, &[])?;

    let rows = conn.query(
        "SELECT region_id, event_type, started_at_unix, prose_seed FROM region_climate",
        &[],
    )?;
    Ok(rows
        .iter()
        .map(|r| RegionClimate {
            region_id: r.get(0),
            event_type: r.get(1),
            started_at_unix: r.get(2),
            prose_seed: r.get(3),
        })
        .collect())
}

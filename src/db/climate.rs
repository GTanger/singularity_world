//! 區域氣候系統 — PostgreSQL 持久層（`region_climate` 表）。
//! 資料表由 store 連線池啟動邊界建立；每分鐘氣候推進只做資料讀寫。

use crate::store;

static CLIMATE_CACHE: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, RegionClimate>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
static CLIMATE_LOADED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

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

/// 查詢指定區域的氣候。
pub fn get_region_climate(region_id: &str) -> anyhow::Result<Option<RegionClimate>> {
    if !CLIMATE_LOADED.load(std::sync::atomic::Ordering::Acquire) {
        if let Ok(all) = list_region_climates() {
            let mut cache = CLIMATE_CACHE.write();
            for c in all {
                cache.insert(c.region_id.clone(), c);
            }
            CLIMATE_LOADED.store(true, std::sync::atomic::Ordering::Release);
        }
    }

    let cache = CLIMATE_CACHE.read();
    if let Some(c) = cache.get(region_id) {
        return Ok(Some(c.clone()));
    }
    drop(cache);

    let Some(pool) = store::get_db_pool() else {
        return Ok(None);
    };
    let mut conn = pool.get()?;

    let rows = conn.query(
        "SELECT region_id, event_type, started_at_unix, prose_seed
         FROM region_climate WHERE region_id = $1",
        &[&region_id],
    )?;

    if rows.is_empty() {
        return Ok(None);
    }
    let row = &rows[0];
    let climate = RegionClimate {
        region_id: row.get(0),
        event_type: row.get(1),
        started_at_unix: row.get(2),
        prose_seed: row.get(3),
    };
    CLIMATE_CACHE
        .write()
        .insert(region_id.to_string(), climate.clone());
    Ok(Some(climate))
}

/// 更新或插入區域氣候（simulation tick 呼叫）。
pub fn upsert_region_climate(climate: &RegionClimate) -> anyhow::Result<()> {
    CLIMATE_CACHE
        .write()
        .insert(climate.region_id.clone(), climate.clone());

    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("後端儲存連線不可用"))?;
    let climate_clone = climate.clone();
    std::thread::spawn(move || {
        if let Ok(mut conn) = pool.get() {
            let _ = conn.execute(
                "INSERT INTO region_climate (region_id, event_type, started_at_unix, prose_seed)
                 VALUES ($1, $2, $3, $4)
                 ON CONFLICT (region_id) DO UPDATE SET
                    event_type = EXCLUDED.event_type,
                    started_at_unix = EXCLUDED.started_at_unix,
                    prose_seed = EXCLUDED.prose_seed",
                &[
                    &climate_clone.region_id,
                    &climate_clone.event_type,
                    &climate_clone.started_at_unix,
                    &climate_clone.prose_seed,
                ],
            );
        }
    });
    Ok(())
}

/// 取得所有區域氣候（simulation tick 批量更新用）。
pub fn list_region_climates() -> anyhow::Result<Vec<RegionClimate>> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("後端儲存連線不可用"))?;
    let mut conn = pool.get()?;

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

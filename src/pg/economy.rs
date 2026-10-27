//! 每遊戲日經濟結算；以 day receipt 保證重啟不重複扣款。

use crate::store::sql::DbPool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyExpenseRow {
    pub entity_id: String,
    pub magnesium: i32,
    pub charged: bool,
    pub became_broke: bool,
}

pub fn ensure_schema(pool: &DbPool) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS world_daily_economy (
            game_day INTEGER PRIMARY KEY,
            applied_at BIGINT NOT NULL,
            affected_count INTEGER NOT NULL DEFAULT 0
        )",
        &[],
    )?;
    Ok(())
}

pub fn apply_daily_expense_once(
    pool: &DbPool,
    game_day: i32,
    now_unix: i64,
    expense: i32,
) -> anyhow::Result<Option<Vec<DailyExpenseRow>>> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let claimed = tx.execute(
        "INSERT INTO world_daily_economy(game_day,applied_at)
         VALUES ($1,$2) ON CONFLICT(game_day) DO NOTHING",
        &[&game_day, &now_unix],
    )?;
    if claimed == 0 {
        tx.rollback()?;
        return Ok(None);
    }
    let rows = tx.query(
        "WITH charged AS (
           UPDATE entities e SET magnesium=GREATEST(0,e.magnesium-$1)
           FROM entity_rooms er WHERE er.entity_id=e.id AND e.kind='npc' AND e.magnesium>0
           RETURNING e.id,e.magnesium
         )
         SELECT e.id,COALESCE(c.magnesium,e.magnesium),c.id IS NOT NULL,
                c.id IS NOT NULL AND c.magnesium=0
         FROM entities e JOIN entity_rooms er ON er.entity_id=e.id
         LEFT JOIN charged c ON c.id=e.id WHERE e.kind='npc'",
        &[&expense],
    )?;
    let results: Vec<_> = rows
        .into_iter()
        .map(|row| DailyExpenseRow {
            entity_id: row.get(0),
            magnesium: row.get(1),
            charged: row.get(2),
            became_broke: row.get(3),
        })
        .collect();
    insert_broke_events(&mut tx, now_unix, &results)?;
    let count = i32::try_from(results.iter().filter(|row| row.charged).count())?;
    tx.execute(
        "UPDATE world_daily_economy SET affected_count=$1 WHERE game_day=$2",
        &[&count, &game_day],
    )?;
    tx.commit()?;
    Ok(Some(results))
}

fn insert_broke_events(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
    now_unix: i64,
    results: &[DailyExpenseRow],
) -> anyhow::Result<()> {
    for result in results.iter().filter(|row| row.became_broke) {
        tx.execute(
            "INSERT INTO event_log(entity_id,event_type,payload,created_at)
             VALUES ($1,'broke','鎂歸零',$2)",
            &[&result.entity_id, &now_unix],
        )?;
    }
    Ok(())
}

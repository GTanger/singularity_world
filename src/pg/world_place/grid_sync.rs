//! 舊 `legacy_grid` 場所與路線退役收斂（微步 3-0 起永久停用舊格網回填）。

use crate::grid::SquareGrid;
use crate::store::sql::DbPool;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationMigrationReport {
    pub legacy_places: usize,
    pub legacy_routes: usize,
    pub migrated_entities: u64,
    pub unresolved_entities: i64,
    pub mirrored_entities: u64,
}

pub fn sync_legacy_grid(
    pool: &DbPool,
    _grid: &SquareGrid,
) -> anyhow::Result<LocationMigrationReport> {
    let mut conn = pool.get()?;
    super::ensure_schema(&mut conn)?;
    let codex_before = codex_coord(&mut conn)?;
    let mut tx = conn.transaction()?;
    tx.execute(
        "UPDATE world_places SET status='retired',traversable=FALSE
         WHERE source_kind='legacy_grid' AND (status<>'retired' OR traversable)",
        &[],
    )?;
    tx.execute(
        "UPDATE world_routes SET status='retired'
         WHERE source_kind='legacy_grid' AND status<>'retired'",
        &[],
    )?;
    let migrated = tx.execute(
        "UPDATE entities e SET place_id=NULL
         FROM world_places p
         WHERE e.place_id=p.id AND p.source_kind='legacy_grid' AND e.id<>'codex'",
        &[],
    )?;
    let unresolved: i64 = tx
        .query_one(
            "SELECT COUNT(*)::bigint FROM entities WHERE place_id IS NULL",
            &[],
        )?
        .get(0);
    tx.commit()?;
    let codex_after = codex_coord(&mut conn)?;
    anyhow::ensure!(
        codex_before == codex_after,
        "codex 的舊座標在位置遷移中被改動"
    );
    Ok(LocationMigrationReport {
        legacy_places: 0,
        legacy_routes: 0,
        migrated_entities: migrated,
        unresolved_entities: unresolved,
        mirrored_entities: 0,
    })
}

fn codex_coord(conn: &mut crate::store::sql::Connection) -> anyhow::Result<Option<(i32, i32)>> {
    let Some(row) = conn.query_opt("SELECT grid_x,grid_y FROM entities WHERE id='codex'", &[])?
    else {
        return Ok(None);
    };
    let x: Option<i32> = row.get(0);
    let y: Option<i32> = row.get(1);
    Ok(x.zip(y))
}

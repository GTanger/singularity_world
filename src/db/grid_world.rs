//! 正方格世界格網 — PostgreSQL 持久層（`grid_cells` 表）。
//!
//! 讀取：逐列重建 `SquareGrid`；世界種子取自既有 `hex_world` 單例（共用同一列）。
//! 寫入：交易內清空 `grid_cells` → 全量 upsert → 回傳。
//! 對外由 `crate::db::load_square_grid` / `save_square_grid_to_pg` 包裝。

use crate::grid::{GridCell, SquareCoord, SquareGrid};
use crate::hex::Terrain;
use crate::store;

use serde::de::DeserializeOwned;
use serde::Serialize;

// ─── serde 輔助（對稱 hex_world.rs）─────────────────────────────────

fn enum_snake_string<T: Serialize>(v: &T) -> anyhow::Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        _ => anyhow::bail!("expected string-like serde value"),
    }
}

fn enum_from_snake_str<T: DeserializeOwned>(s: &str) -> anyhow::Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(s.to_string()))?)
}

// ─── 載入 ────────────────────────────────────────────────────────────

/// 自 PostgreSQL `grid_cells` 讀取正方格世界格網。
///
/// 若連線不可用或 `grid_cells` 為空則回傳 `None`。
/// 世界種子取自 `hex_world WHERE id = 1`，確保兩地圖使用同一個世界種子。
pub(super) fn load_square_grid() -> Option<SquareGrid> {
    let pool = store::get_db_pool()?;
    let mut conn = pool.get().ok()?;

    // 確保資料表存在（首次啟動時建立）
    if let Err(e) = conn.execute(
        "CREATE TABLE IF NOT EXISTS grid_cells (
            x           INTEGER NOT NULL,
            y           INTEGER NOT NULL,
            terrain     TEXT    NOT NULL,
            name        TEXT    NOT NULL DEFAULT '',
            zone        TEXT    NOT NULL DEFAULT '',
            tags        TEXT    NOT NULL DEFAULT '[]',
            description TEXT    NOT NULL DEFAULT '',
            objects     TEXT    NOT NULL DEFAULT '[]',
            explored    BOOLEAN NOT NULL DEFAULT FALSE,
            PRIMARY KEY (x, y)
        )",
        &[],
    ) {
        tracing::warn!("[grid_world] 建立 grid_cells 表失敗：{e}");
        return None;
    }

    // 若表格為空則無格網可載入
    let count: i64 = conn
        .query_one("SELECT COUNT(*)::bigint FROM grid_cells", &[])
        .ok()?
        .get(0);
    if count == 0 {
        return None;
    }

    // 取世界種子（共用 hex_world 單例；查無記錄時用預設值）
    let world_seed: u64 = conn
        .query_opt("SELECT world_seed FROM hex_world WHERE id = 1", &[])
        .ok()
        .flatten()
        .map(|r| r.get::<_, i64>(0) as u64)
        .filter(|&s| s != 0)
        .unwrap_or(20_260_414);

    // 逐列重建格子
    let rows = conn
        .query(
            "SELECT x, y, terrain, name, zone, tags, description, objects, explored
             FROM grid_cells ORDER BY x, y",
            &[],
        )
        .ok()?;

    let mut grid = SquareGrid::new();
    grid.set_world_seed(world_seed);

    for row in rows {
        let x: i32 = row.get(0);
        let y: i32 = row.get(1);
        let terrain_s: String = row.get(2);
        let terrain: Terrain = match enum_from_snake_str(&terrain_s) {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("[grid_world] 地形解析失敗 ({x},{y}) '{terrain_s}'：{e}");
                continue;
            }
        };
        let name: String = row.get(3);
        let zone: String = row.get(4);
        let tags_s: String = row.get(5);
        let description: String = row.get(6);
        let objects_s: String = row.get(7);
        let explored: bool = row.get(8);

        let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
        let objects = serde_json::from_str(&objects_s).unwrap_or_default();
        let coord = SquareCoord::new(x, y);

        let mut cell = GridCell::new(coord, terrain, name);
        cell.zone = zone;
        cell.tags = tags;
        cell.description = description;
        cell.objects = objects;
        cell.explored = explored;

        grid.insert(cell);
    }

    tracing::info!("[grid_world] 載入 {} 格（PostgreSQL grid_cells）", grid.len());
    Some(grid)
}

// ─── 儲存 ────────────────────────────────────────────────────────────

/// 將正方格世界格網全量寫入 PostgreSQL `grid_cells`（交易內執行）。
pub(super) fn save_square_grid_to_pg(grid: &SquareGrid) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;

    // 確保資料表存在
    conn.execute(
        "CREATE TABLE IF NOT EXISTS grid_cells (
            x           INTEGER NOT NULL,
            y           INTEGER NOT NULL,
            terrain     TEXT    NOT NULL,
            name        TEXT    NOT NULL DEFAULT '',
            zone        TEXT    NOT NULL DEFAULT '',
            tags        TEXT    NOT NULL DEFAULT '[]',
            description TEXT    NOT NULL DEFAULT '',
            objects     TEXT    NOT NULL DEFAULT '[]',
            explored    BOOLEAN NOT NULL DEFAULT FALSE,
            PRIMARY KEY (x, y)
        )",
        &[],
    )?;

    let mut t = conn.transaction()?;

    // 全量替換：清空後重寫（對稱 hex_world.rs persist_grid 的清空策略）
    t.execute("DELETE FROM grid_cells", &[])?;

    for cell in grid.cells() {
        let terrain = enum_snake_string(&cell.terrain)?;
        let tags = serde_json::to_string(&cell.tags)?;
        let objects = serde_json::to_string(&cell.objects)?;
        t.execute(
            "INSERT INTO grid_cells (x, y, terrain, name, zone, tags, description, objects, explored)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            &[
                &cell.coord.x,
                &cell.coord.y,
                &terrain,
                &cell.name,
                &cell.zone,
                &tags,
                &cell.description,
                &objects,
                &cell.explored,
            ],
        )?;
    }

    t.commit()?;
    tracing::info!("[grid_world] PG 寫入成功：{} 格", grid.len());
    Ok(())
}

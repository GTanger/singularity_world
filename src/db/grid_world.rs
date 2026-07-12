//! 正方格世界格網 — PostgreSQL 持久層（`grid_cells` 表）。
//!
//! 讀取：逐列重建 `SquareGrid`；世界種子取固定常數（hex 世界觀已下線，不再共用）。
//! 寫入：交易內清空 `grid_cells` → 全量 upsert → 回傳。
//! 對外由 `crate::db::load_square_grid` / `save_square_grid_to_pg` 包裝。

use crate::grid::{Freshness, GridCell, SquareCoord, SquareGrid, Terrain};
use crate::store;

use serde::Serialize;
use serde::de::DeserializeOwned;

const GRID_WORLD_SEED: u64 = 20_260_414;

// ─── serde 輔助 ────────────────────────────────────────────────────

fn enum_snake_string<T: Serialize>(v: &T) -> anyhow::Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        _ => anyhow::bail!("expected string-like serde value"),
    }
}

fn enum_from_snake_str<T: DeserializeOwned>(s: &str) -> anyhow::Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(
        s.to_string(),
    ))?)
}

// ─── 載入 ────────────────────────────────────────────────────────────

/// 自 PostgreSQL `grid_cells` 讀取正方格世界格網。
///
/// 若連線不可用或 `grid_cells` 為空則回傳 `None`。
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
            freshness   TEXT    NOT NULL DEFAULT 'fresh',
            exits       TEXT    NOT NULL DEFAULT '[]',
            PRIMARY KEY (x, y)
        )",
        &[],
    ) {
        tracing::warn!("[grid_world] 建立 grid_cells 表失敗：{e}");
        return None;
    }

    // M1 migration：補 freshness 欄位（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE grid_cells ADD COLUMN IF NOT EXISTS freshness TEXT NOT NULL DEFAULT 'fresh'",
        &[],
    );

    // M2 migration：補 exits 欄位（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE grid_cells ADD COLUMN IF NOT EXISTS exits TEXT NOT NULL DEFAULT '[]'",
        &[],
    );

    // 若表格為空則無格網可載入
    let count: i64 = conn
        .query_one("SELECT COUNT(*)::bigint FROM grid_cells", &[])
        .ok()?
        .get(0);
    if count == 0 {
        return None;
    }

    let world_seed: u64 = GRID_WORLD_SEED;

    // 逐列重建格子
    let rows = conn
        .query(
            "SELECT x, y, terrain, name, zone, tags, description, objects, explored, freshness, exits
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
        let freshness_s: String = row.get(9);
        let freshness: Freshness = enum_from_snake_str(&freshness_s).unwrap_or_default();
        let exits_json: String = row.get(10);
        let explicit_exits: Vec<crate::grid::ExplicitExit> =
            serde_json::from_str(&exits_json).unwrap_or_default();

        let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
        let objects = serde_json::from_str(&objects_s).unwrap_or_default();
        let coord = SquareCoord::new(x, y);

        let mut cell = GridCell::new(coord, terrain, name);
        cell.zone = zone;
        cell.tags = tags;
        cell.description = description;
        cell.objects = objects;
        cell.explored = explored;
        cell.freshness = freshness;
        cell.explicit_exits = explicit_exits;

        grid.insert(cell);
    }

    tracing::info!(
        "[grid_world] 載入 {} 格（PostgreSQL grid_cells）",
        grid.len()
    );
    Some(grid)
}

// ─── 儲存 ────────────────────────────────────────────────────────────

/// 將正方格世界格網全量寫入 PostgreSQL `grid_cells`（交易內執行）。
pub(super) fn save_square_grid_to_pg(grid: &SquareGrid) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;

    // 確保資料表存在（含 freshness + exits 欄位）
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
            freshness   TEXT    NOT NULL DEFAULT 'fresh',
            exits       TEXT    NOT NULL DEFAULT '[]',
            PRIMARY KEY (x, y)
        )",
        &[],
    )?;

    // M1 migration：補 freshness 欄位（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE grid_cells ADD COLUMN IF NOT EXISTS freshness TEXT NOT NULL DEFAULT 'fresh'",
        &[],
    );

    // M2 migration：補 exits 欄位（舊表可能沒有）
    let _ = conn.execute(
        "ALTER TABLE grid_cells ADD COLUMN IF NOT EXISTS exits TEXT NOT NULL DEFAULT '[]'",
        &[],
    );

    let mut t = conn.transaction()?;

    // 增量 upsert（不再全表 DELETE + re-INSERT，避免 600+ 行交易卡住 grid lock）
    for cell in grid.cells() {
        let terrain = enum_snake_string(&cell.terrain)?;
        let tags = serde_json::to_string(&cell.tags)?;
        let objects = serde_json::to_string(&cell.objects)?;
        let freshness = enum_snake_string(&cell.freshness)?;
        let exits_json =
            serde_json::to_string(&cell.explicit_exits).unwrap_or_else(|_| "[]".to_string());
        t.execute(
            "INSERT INTO grid_cells (x, y, terrain, name, zone, tags, description, objects, explored, freshness, exits)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (x, y) DO UPDATE SET
                terrain=EXCLUDED.terrain, name=EXCLUDED.name, zone=EXCLUDED.zone,
                tags=EXCLUDED.tags, description=EXCLUDED.description,
                objects=EXCLUDED.objects, explored=EXCLUDED.explored,
                freshness=EXCLUDED.freshness, exits=EXCLUDED.exits",
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
                &freshness,
                &exits_json,
            ],
        )?;
    }

    t.commit()?;
    tracing::info!("[grid_world] PG 寫入成功：{} 格", grid.len());
    Ok(())
}

// ─── 單格 freshness 更新（直接 PG update，不走全量 save）────────────

/// 更新單一格子的 freshness（explore handler 呼叫，不走全量 grid save）。
pub fn update_cell_freshness(x: i32, y: i32, freshness: Freshness) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let freshness_s = enum_snake_string(&freshness)?;
    conn.execute(
        "UPDATE grid_cells SET freshness = $1 WHERE x = $2 AND y = $3",
        &[&freshness_s, &x, &y],
    )?;
    Ok(())
}

/// 將所有無玩家在場的 non-fresh 格子回復為 fresh（simulation tick 呼叫）。
/// `occupied_coords`：有玩家在場的格座標列表。
pub fn reset_freshness_for_vacant_cells(occupied_coords: &[(i32, i32)]) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;

    if occupied_coords.is_empty() {
        // 沒有玩家在場時，全部回 fresh
        conn.execute(
            "UPDATE grid_cells SET freshness = 'fresh' WHERE freshness <> 'fresh'",
            &[],
        )?;
    } else {
        // 有玩家時，排除有人格子後回 fresh
        // 用 ANY + unnest 避免動態 SQL 拼接
        let xs: Vec<i32> = occupied_coords.iter().map(|(x, _)| *x).collect();
        let ys: Vec<i32> = occupied_coords.iter().map(|(_, y)| *y).collect();
        // PostgreSQL：unnest 兩個陣列配對
        conn.execute(
            "UPDATE grid_cells SET freshness = 'fresh'
             WHERE freshness <> 'fresh'
               AND (x, y) NOT IN (SELECT unnest($1::int[]), unnest($2::int[]))",
            &[&xs, &ys],
        )?;
    }
    Ok(())
}

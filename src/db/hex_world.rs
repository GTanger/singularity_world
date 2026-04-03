//! 六角野外世界格網 — PostgreSQL 正規化表（`hex_cells`、`hex_barriers`、`hex_portals`、`hex_transport_edges` + `hex_world` 單例）。
//! 啟動時若 `hex_cells` 為空且 `hex_world.grid_json` 有舊資料，一次性遷移後清空 `grid_json`。
//! 寫入：交易內清空子表 → 寫入 → 更新單例；另由 hex_editor 寫 JSON 備份檔。

use std::collections::{HashMap, HashSet};

use r2d2_postgres::postgres::Transaction;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::hex::{
    HexCell, HexCoord, HexDir, HexGrid, Portal, Terrain, TransportEdge, TransportEndpoint,
};
use crate::store;
use crate::store::sql::Connection;

fn enum_snake_string<T: Serialize>(v: &T) -> anyhow::Result<String> {
    match serde_json::to_value(v)? {
        serde_json::Value::String(s) => Ok(s),
        _ => anyhow::bail!("expected string-like serde value"),
    }
}

fn enum_from_snake_str<T: DeserializeOwned>(s: &str) -> anyhow::Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(s.to_string()))?)
}

/// 自資料庫讀取世界格網；無連線、無列且無可遷移之舊 JSON 則 `None`（對外經 `crate::db::load_hex_grid`）。
pub(super) fn load_hex_grid() -> Option<HexGrid> {
    let pool = store::get_db_pool()?;
    let mut conn = pool.get().ok()?;
    load_hex_grid_conn(&mut conn).ok().flatten()
}

fn load_hex_grid_conn(conn: &mut Connection) -> anyhow::Result<Option<HexGrid>> {
    let n: i64 = conn.query_one("SELECT COUNT(*)::bigint FROM hex_cells", &[])?.get(0);
    if n == 0 {
        return try_migrate_legacy_json(conn);
    }
    Ok(Some(load_normalized(conn)?))
}

/// 僅在 `hex_cells` 為空時：自 `hex_world.grid_json` 匯入並寫入正規化表。
fn try_migrate_legacy_json(conn: &mut Connection) -> anyhow::Result<Option<HexGrid>> {
    let s: String = conn
        .query_opt("SELECT COALESCE(grid_json, '') FROM hex_world WHERE id = 1", &[])?
        .map(|r| r.get::<_, String>(0))
        .unwrap_or_default();
    if s.trim().is_empty() {
        return Ok(None);
    }
    let grid: HexGrid = serde_json::from_str(s.trim())?;
    let mut t = conn.transaction()?;
    persist_grid(&mut t, &grid)?;
    t.commit()?;
    Ok(Some(grid))
}

fn load_normalized(conn: &mut Connection) -> anyhow::Result<HexGrid> {
    let world_seed: u64 = conn
        .query_opt("SELECT world_seed FROM hex_world WHERE id = 1", &[])?
        .map(|r| r.get::<_, i64>(0) as u64)
        .unwrap_or(0);

    let cell_rows = conn.query(
        "SELECT q, r, terrain, name, zone, tags::text, description, objects::text
         FROM hex_cells ORDER BY q, r",
        &[],
    )?;
    let mut cells: HashMap<HexCoord, HexCell> = HashMap::with_capacity(cell_rows.len());
    for row in cell_rows {
        let q: i32 = row.get(0);
        let r: i32 = row.get(1);
        let terrain_s: String = row.get(2);
        let terrain: Terrain = enum_from_snake_str(&terrain_s)?;
        let name: String = row.get(3);
        let zone: String = row.get(4);
        let tags_s: String = row.get(5);
        let description: String = row.get(6);
        let objects_s: String = row.get(7);
        let tags: Vec<String> = serde_json::from_str(&tags_s).unwrap_or_default();
        let objects = serde_json::from_str(&objects_s).unwrap_or_default();
        let coord = HexCoord::new(q, r);
        cells.insert(
            coord,
            HexCell {
                coord,
                terrain,
                name,
                zone,
                tags,
                description,
                objects,
            },
        );
    }

    let bar_rows = conn.query("SELECT q, r, dir FROM hex_barriers", &[])?;
    let mut barriers: HashSet<(HexCoord, HexDir)> = HashSet::with_capacity(bar_rows.len());
    for row in bar_rows {
        let q: i32 = row.get(0);
        let r: i32 = row.get(1);
        let dir_s: String = row.get(2);
        let dir: HexDir = enum_from_snake_str(&dir_s)?;
        barriers.insert((HexCoord::new(q, r), dir));
    }

    let portal_rows = conn.query(
        "SELECT name, from_q, from_r, to_q, to_r, bidirectional, counts_as_official_link
         FROM hex_portals ORDER BY sort_order ASC, id ASC",
        &[],
    )?;
    let mut portals: Vec<Portal> = Vec::with_capacity(portal_rows.len());
    for row in portal_rows {
        portals.push(Portal {
            name: row.get(0),
            from: HexCoord::new(row.get(1), row.get(2)),
            to: HexCoord::new(row.get(3), row.get(4)),
            bidirectional: row.get(5),
            counts_as_official_link: row.get(6),
        });
    }

    let edge_rows = conn.query(
        "SELECT edge_id, a_kind, a_q, a_r, a_settlement_id, b_kind, b_q, b_r, b_settlement_id, mode, operational, link_class, weight
         FROM hex_transport_edges ORDER BY sort_order ASC, id ASC",
        &[],
    )?;
    let mut transport_edges: Vec<TransportEdge> = Vec::with_capacity(edge_rows.len());
    for row in edge_rows {
        let edge_id: Option<String> = row.get(0);
        let a_kind: i32 = row.get(1);
        let a_q: Option<i32> = row.get(2);
        let a_r: Option<i32> = row.get(3);
        let a_settlement_id: Option<String> = row.get(4);
        let b_kind: i32 = row.get(5);
        let b_q: Option<i32> = row.get(6);
        let b_r: Option<i32> = row.get(7);
        let b_settlement_id: Option<String> = row.get(8);
        let mode_s: String = row.get(9);
        let operational: bool = row.get(10);
        let lc_s: String = row.get(11);
        let weight: Option<f64> = row.get(12);

        let endpoint_a = if a_kind == 0 {
            TransportEndpoint::Cell(HexCoord::new(
                a_q.ok_or_else(|| anyhow::anyhow!("hex_transport_edges: a_q"))?,
                a_r.ok_or_else(|| anyhow::anyhow!("hex_transport_edges: a_r"))?,
            ))
        } else {
            TransportEndpoint::Settlement {
                settlement_id: a_settlement_id
                    .ok_or_else(|| anyhow::anyhow!("hex_transport_edges: a_settlement_id"))?,
            }
        };
        let endpoint_b = if b_kind == 0 {
            TransportEndpoint::Cell(HexCoord::new(
                b_q.ok_or_else(|| anyhow::anyhow!("hex_transport_edges: b_q"))?,
                b_r.ok_or_else(|| anyhow::anyhow!("hex_transport_edges: b_r"))?,
            ))
        } else {
            TransportEndpoint::Settlement {
                settlement_id: b_settlement_id
                    .ok_or_else(|| anyhow::anyhow!("hex_transport_edges: b_settlement_id"))?,
            }
        };

        transport_edges.push(TransportEdge {
            id: edge_id,
            endpoint_a,
            endpoint_b,
            mode: enum_from_snake_str(&mode_s)?,
            operational,
            link_class: enum_from_snake_str(&lc_s)?,
            weight,
        });
    }

    Ok(HexGrid::from_parts(
        world_seed,
        cells,
        barriers,
        portals,
        transport_edges,
    ))
}

fn persist_grid(t: &mut Transaction<'_>, grid: &HexGrid) -> anyhow::Result<()> {
    t.execute("DELETE FROM hex_transport_edges", &[])?;
    t.execute("DELETE FROM hex_portals", &[])?;
    t.execute("DELETE FROM hex_barriers", &[])?;
    t.execute("DELETE FROM hex_cells", &[])?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let ws_i64 = grid.world_seed() as i64;
    t.execute(
        "INSERT INTO hex_world (id, world_seed, grid_json, updated_at) VALUES (1, $1, '', $2)
         ON CONFLICT (id) DO UPDATE SET world_seed = EXCLUDED.world_seed, grid_json = '', updated_at = EXCLUDED.updated_at",
        &[&ws_i64, &now],
    )?;

    for cell in grid.cells() {
        let terrain = enum_snake_string(&cell.terrain)?;
        let tags = serde_json::to_string(&cell.tags)?;
        let objects = serde_json::to_string(&cell.objects)?;
        t.execute(
            "INSERT INTO hex_cells (q, r, terrain, name, zone, tags, description, objects)
             VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7, $8::jsonb)",
            &[
                &cell.coord.q,
                &cell.coord.r,
                &terrain,
                &cell.name,
                &cell.zone,
                &tags,
                &cell.description,
                &objects,
            ],
        )?;
    }

    for (coord, dir) in grid.barriers_iter() {
        let d = enum_snake_string(&dir)?;
        t.execute(
            "INSERT INTO hex_barriers (q, r, dir) VALUES ($1, $2, $3)",
            &[&coord.q, &coord.r, &d],
        )?;
    }

    for (i, p) in grid.portals().iter().enumerate() {
        let sort_order: i32 = i32::try_from(i).unwrap_or(i32::MAX);
        t.execute(
            "INSERT INTO hex_portals (sort_order, name, from_q, from_r, to_q, to_r, bidirectional, counts_as_official_link)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[
                &sort_order,
                &p.name,
                &p.from.q,
                &p.from.r,
                &p.to.q,
                &p.to.r,
                &p.bidirectional,
                &p.counts_as_official_link,
            ],
        )?;
    }

    for (i, e) in grid.transport_edges().iter().enumerate() {
        let sort_order: i32 = i32::try_from(i).unwrap_or(i32::MAX);
        let (a_kind, a_q, a_r, a_sid): (i32, Option<i32>, Option<i32>, Option<&str>) = match &e.endpoint_a {
            TransportEndpoint::Cell(c) => (0, Some(c.q), Some(c.r), None),
            TransportEndpoint::Settlement { settlement_id } => (1, None, None, Some(settlement_id.as_str())),
        };
        let (b_kind, b_q, b_r, b_sid): (i32, Option<i32>, Option<i32>, Option<&str>) = match &e.endpoint_b {
            TransportEndpoint::Cell(c) => (0, Some(c.q), Some(c.r), None),
            TransportEndpoint::Settlement { settlement_id } => (1, None, None, Some(settlement_id.as_str())),
        };
        let mode = enum_snake_string(&e.mode)?;
        let link_class = enum_snake_string(&e.link_class)?;

        let edge_id_sql: Option<&str> = e.id.as_deref();
        t.execute(
            "INSERT INTO hex_transport_edges (sort_order, edge_id, a_kind, a_q, a_r, a_settlement_id, b_kind, b_q, b_r, b_settlement_id, mode, operational, link_class, weight)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
            &[
                &sort_order,
                &edge_id_sql,
                &a_kind,
                &a_q,
                &a_r,
                &a_sid,
                &b_kind,
                &b_q,
                &b_r,
                &b_sid,
                &mode,
                &e.operational,
                &link_class,
                &e.weight,
            ],
        )?;
    }

    Ok(())
}

/// 寫入 PostgreSQL（交易內替換子表）（對外經 `crate::db::save_hex_grid_to_pg`）。
pub(super) fn save_hex_grid_to_pg(grid: &HexGrid) -> anyhow::Result<()> {
    let pool = store::get_db_pool().ok_or_else(|| anyhow::anyhow!("資料庫連線不可用"))?;
    let mut conn = pool.get()?;
    let mut t = conn.transaction()?;
    persist_grid(&mut t, grid)?;
    t.commit()?;
    Ok(())
}

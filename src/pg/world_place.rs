//! 世界場所 PostgreSQL 主資料：正典拓撲容器、舊格素材與實體位置遷移。

mod grid_sync;

use crate::store::sql::{Connection, DbPool};
use crate::world::place::{PlaceGraph, WHITE_TOWER_WORLD_ID, WorldPlace, WorldRoute};
use crate::world::topology::builtin_white_tower_topology;

pub use grid_sync::{LocationMigrationReport, sync_legacy_grid};

const CREATE_PLACES: &str = "
CREATE TABLE IF NOT EXISTS world_places (
    id TEXT PRIMARY KEY,
    world_id TEXT NOT NULL,
    topology_node_ref TEXT,
    parent_place_id TEXT,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    scale TEXT NOT NULL,
    canon BOOLEAN NOT NULL,
    status TEXT NOT NULL,
    traversable BOOLEAN NOT NULL,
    source_kind TEXT NOT NULL,
    zone TEXT NOT NULL DEFAULT '',
    tags TEXT NOT NULL DEFAULT '[]',
    description TEXT NOT NULL DEFAULT '',
    legacy_grid_x INTEGER,
    legacy_grid_y INTEGER
)";

const CREATE_ROUTES: &str = "
CREATE TABLE IF NOT EXISTS world_routes (
    id TEXT PRIMARY KEY,
    from_place_id TEXT NOT NULL,
    direction TEXT NOT NULL,
    to_place_id TEXT NOT NULL,
    mode TEXT NOT NULL,
    travel_cost INTEGER NOT NULL,
    canon BOOLEAN NOT NULL,
    status TEXT NOT NULL,
    source_kind TEXT NOT NULL,
    UNIQUE (from_place_id, direction, to_place_id)
)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceSeedReport {
    pub topology_places: usize,
}

pub fn ensure_topology_places(pool: &DbPool) -> anyhow::Result<PlaceSeedReport> {
    let topology = builtin_white_tower_topology()?;
    let mut conn = pool.get()?;
    ensure_schema(&mut conn)?;
    let mut tx = conn.transaction()?;
    for node in &topology.nodes {
        let id = topology_place_id(&node.id);
        let parent = node.parent_ref.as_deref().map(topology_place_id);
        let scale = if node.parent_ref.is_none() {
            "world_region"
        } else {
            "geography"
        };
        tx.execute(
            "INSERT INTO world_places
             (id,world_id,topology_node_ref,parent_place_id,name,kind,scale,canon,status,
              traversable,source_kind,zone,tags,description,legacy_grid_x,legacy_grid_y)
             VALUES ($1,$2,$3,$4,$5,$6,$7,TRUE,'accepted',FALSE,'topology','','[]',$8,NULL,NULL)
             ON CONFLICT (id) DO UPDATE SET world_id=EXCLUDED.world_id,
             topology_node_ref=EXCLUDED.topology_node_ref,parent_place_id=EXCLUDED.parent_place_id,
             name=EXCLUDED.name,kind=EXCLUDED.kind,scale=EXCLUDED.scale,canon=TRUE,
             status='accepted',traversable=FALSE,source_kind='topology',description=EXCLUDED.description",
            &[
                &id,
                &WHITE_TOWER_WORLD_ID,
                &node.id,
                &parent,
                &node.display_name,
                &node.kind,
                &scale,
                &node.summary,
            ],
        )?;
    }
    tx.commit()?;
    Ok(PlaceSeedReport {
        topology_places: topology.nodes.len(),
    })
}

pub fn load_graph(pool: &DbPool) -> anyhow::Result<PlaceGraph> {
    let mut conn = pool.get()?;
    ensure_schema(&mut conn)?;
    PlaceGraph::new(load_places(&mut conn)?, load_routes(&mut conn)?)
}

pub fn set_entity_place(
    pool: &DbPool,
    entity_id: &str,
    place_id: Option<&str>,
    grid_x: Option<i32>,
    grid_y: Option<i32>,
    grid_z: i16,
    room_id: &str,
) -> anyhow::Result<()> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE entities SET place_id=$1, grid_x=$2, grid_y=$3, grid_z=$4 WHERE id=$5",
        &[&place_id, &grid_x, &grid_y, &grid_z, &entity_id],
    )?;
    anyhow::ensure!(changed == 1, "實體不存在：{entity_id}");
    tx.execute(
        "INSERT INTO entity_rooms(entity_id,room_id) VALUES ($1,$2)
         ON CONFLICT(entity_id) DO UPDATE SET room_id=EXCLUDED.room_id",
        &[&entity_id, &room_id],
    )?;
    tx.commit()?;
    Ok(())
}

/// 玩家與居民共用的相鄰移動提交：位置、相容房間與行動收據同一交易完成。
#[allow(clippy::too_many_arguments)]
pub fn move_entity_place(
    pool: &DbPool,
    entity_id: &str,
    from_place_id: &str,
    to_place_id: &str,
    grid_x: Option<i32>,
    grid_y: Option<i32>,
    room_id: &str,
    route: &WorldRoute,
    journey: Option<&crate::world::journey::EntityJourney>,
    now_unix: i64,
) -> anyhow::Result<String> {
    let mut conn = pool.get()?;
    let mut tx = conn.transaction()?;
    let changed = tx.execute(
        "UPDATE entities SET place_id=$1,grid_x=$2,grid_y=$3
         WHERE id=$4 AND place_id=$5",
        &[&to_place_id, &grid_x, &grid_y, &entity_id, &from_place_id],
    )?;
    anyhow::ensure!(
        changed == 1,
        "人物位置已改變，拒絕提交過期移動：{entity_id}"
    );
    tx.execute(
        "INSERT INTO entity_rooms(entity_id,room_id) VALUES ($1,$2)
         ON CONFLICT(entity_id) DO UPDATE SET room_id=EXCLUDED.room_id",
        &[&entity_id, &room_id],
    )?;
    let fallback_journey = crate::world::journey::EntityJourney {
        entity_id: entity_id.to_string(),
        reason: "move".to_string(),
        destination_place_id: Some(to_place_id.to_string()),
        planned_path: Vec::new(),
        consequence: format!("由{from_place_id}抵達{to_place_id}"),
        updated_at: now_unix,
    };
    let committed_journey = journey.unwrap_or(&fallback_journey);
    anyhow::ensure!(
        committed_journey.entity_id == entity_id,
        "移動與旅程人物不一致：{entity_id}"
    );
    crate::pg::world_journey::upsert_in(&mut tx, committed_journey)?;
    let action_id = crate::pg::living_world::record_action_in_tx(
        &mut tx,
        crate::pg::living_world::ActionDraft {
            actor_id: entity_id,
            action_key: "move",
            target_ref: &route.id,
            place_id: to_place_id,
            preconditions: serde_json::json!({
                "from_place_id": from_place_id,
                "route_status": route.status,
                "direction": route.direction
            }),
            effects: serde_json::json!({
                "place_id": {"before": from_place_id, "after": to_place_id},
                "travel_cost": route.travel_cost
            }),
            outcome: "moved",
            source_ref: "runtime:world_routes",
            occurred_at: now_unix,
        },
    )?;
    tx.commit()?;
    Ok(action_id)
}

pub(crate) fn ensure_schema(conn: &mut Connection) -> anyhow::Result<()> {
    conn.execute(CREATE_PLACES, &[])?;
    conn.execute(CREATE_ROUTES, &[])?;
    conn.execute(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_world_places_legacy_grid
         ON world_places (legacy_grid_x, legacy_grid_y) WHERE source_kind='legacy_grid'",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_world_routes_from ON world_routes(from_place_id)",
        &[],
    )?;
    conn.execute(
        "ALTER TABLE entities ADD COLUMN IF NOT EXISTS place_id TEXT",
        &[],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_entities_place_id ON entities(place_id)",
        &[],
    )?;
    Ok(())
}

fn load_places(conn: &mut Connection) -> anyhow::Result<Vec<WorldPlace>> {
    let rows = conn.query(
        "SELECT id,world_id,topology_node_ref,parent_place_id,name,kind,scale,canon,status,
                traversable,source_kind,zone,tags,description,legacy_grid_x,legacy_grid_y
         FROM world_places
         WHERE status NOT IN ('retired','retained_runtime')
         ORDER BY id",
        &[],
    )?;
    rows.into_iter().map(place_from_row).collect()
}

fn place_from_row(row: r2d2_postgres::postgres::Row) -> anyhow::Result<WorldPlace> {
    let tags: String = row.get(12);
    Ok(WorldPlace {
        id: row.get(0),
        world_id: row.get(1),
        topology_node_ref: row.get(2),
        parent_place_id: row.get(3),
        name: row.get(4),
        kind: row.get(5),
        scale: row.get(6),
        canon: row.get(7),
        status: row.get(8),
        traversable: row.get(9),
        source_kind: row.get(10),
        zone: row.get(11),
        tags: serde_json::from_str(&tags)?,
        description: row.get(13),
        legacy_grid_x: row.get(14),
        legacy_grid_y: row.get(15),
    })
}

fn load_routes(conn: &mut Connection) -> anyhow::Result<Vec<WorldRoute>> {
    let rows = conn.query(
        "SELECT id,from_place_id,direction,to_place_id,mode,travel_cost,canon,status,source_kind
         FROM world_routes WHERE status='active' ORDER BY id",
        &[],
    )?;
    Ok(rows
        .into_iter()
        .map(|row| WorldRoute {
            id: row.get(0),
            from_place_id: row.get(1),
            direction: row.get(2),
            to_place_id: row.get(3),
            mode: row.get(4),
            travel_cost: row.get(5),
            canon: row.get(6),
            status: row.get(7),
            source_kind: row.get(8),
        })
        .collect())
}

fn topology_place_id(node_id: &str) -> String {
    format!("topology:{node_id}")
}

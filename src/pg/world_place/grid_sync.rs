//! 舊方格資料無損升為 material 場所，並原子校正實體的位置鏡像。

use crate::grid::{Direction, SquareGrid};
use crate::store::sql::DbPool;
use crate::world::place::{LEGACY_WORLD_ID, WorldPlace, WorldRoute, legacy_place_id};

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
    grid: &SquareGrid,
) -> anyhow::Result<LocationMigrationReport> {
    let (places, routes) = build_legacy_records(grid)?;
    let mut conn = pool.get()?;
    super::ensure_schema(&mut conn)?;
    let codex_before = codex_coord(&mut conn)?;
    let mut tx = conn.transaction()?;
    tx.execute(
        "UPDATE world_places SET status='retired',traversable=FALSE
         WHERE source_kind='legacy_grid'",
        &[],
    )?;
    upsert_places(&mut tx, &places)?;
    tx.execute(
        "DELETE FROM world_routes WHERE source_kind='legacy_grid'",
        &[],
    )?;
    upsert_routes(&mut tx, &routes)?;
    let migrated = migrate_missing_place_ids(&mut tx)?;
    let mirrored = mirror_entity_locations(&mut tx)?;
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
        legacy_places: places.len(),
        legacy_routes: routes.len(),
        migrated_entities: migrated,
        unresolved_entities: unresolved,
        mirrored_entities: mirrored,
    })
}

fn build_legacy_records(grid: &SquareGrid) -> anyhow::Result<(Vec<WorldPlace>, Vec<WorldRoute>)> {
    let mut places = Vec::new();
    let mut routes = Vec::new();
    for cell in grid.cells() {
        places.push(place_from_cell(cell)?);
        if cell.explicit_exits.is_empty() {
            for direction in Direction::ALL {
                let target = cell.coord.neighbor(direction);
                if grid.can_walk(cell.coord, target) {
                    routes.push(route_for(grid, cell.coord, target, direction)?);
                }
            }
        } else {
            for exit in &cell.explicit_exits {
                routes.push(route_for(grid, cell.coord, exit.target, exit.dir)?);
            }
        }
    }
    places.sort_by(|a, b| a.id.cmp(&b.id));
    routes.sort_by(|a, b| a.id.cmp(&b.id));
    Ok((places, routes))
}

fn place_from_cell(cell: &crate::grid::GridCell) -> anyhow::Result<WorldPlace> {
    let name = if cell.name.trim().is_empty() {
        crate::grid::terrain_name_zh(cell.terrain).to_string()
    } else {
        cell.name.clone()
    };
    Ok(WorldPlace {
        id: legacy_place_id(cell.coord),
        world_id: LEGACY_WORLD_ID.to_string(),
        topology_node_ref: None,
        parent_place_id: None,
        name,
        kind: cell.terrain.kind_str().to_string(),
        scale: "place".to_string(),
        canon: false,
        status: "material".to_string(),
        traversable: true,
        source_kind: "legacy_grid".to_string(),
        zone: cell.zone.clone(),
        tags: cell.tags.clone(),
        description: cell.description.clone(),
        legacy_grid_x: Some(cell.coord.x),
        legacy_grid_y: Some(cell.coord.y),
    })
}

fn route_for(
    grid: &SquareGrid,
    from: crate::grid::SquareCoord,
    to: crate::grid::SquareCoord,
    direction: Direction,
) -> anyhow::Result<WorldRoute> {
    let target = grid
        .get(to)
        .ok_or_else(|| anyhow::anyhow!("舊格出口指向不存在座標：{from} -> {to}"))?;
    let terrain_cost = target.terrain.move_cost();
    let (mode, cost) = if terrain_cost.is_finite() {
        ("walk", (terrain_cost * 2.0).round() as i32)
    } else {
        ("passage", 2)
    };
    Ok(WorldRoute {
        id: format!("legacy:route:{}:{}:{}:{}", from.x, from.y, to.x, to.y),
        from_place_id: legacy_place_id(from),
        direction: direction.label_zh().to_string(),
        to_place_id: legacy_place_id(to),
        mode: mode.to_string(),
        travel_cost: cost,
        canon: false,
        status: "active".to_string(),
        source_kind: "legacy_grid".to_string(),
    })
}

fn upsert_places(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
    places: &[WorldPlace],
) -> anyhow::Result<()> {
    for place in places {
        let tags = serde_json::to_string(&place.tags)?;
        tx.execute(
            "INSERT INTO world_places
             (id,world_id,topology_node_ref,parent_place_id,name,kind,scale,canon,status,
              traversable,source_kind,zone,tags,description,legacy_grid_x,legacy_grid_y)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
             ON CONFLICT (id) DO UPDATE SET name=EXCLUDED.name,kind=EXCLUDED.kind,
             status=EXCLUDED.status,traversable=EXCLUDED.traversable,zone=EXCLUDED.zone,
             tags=EXCLUDED.tags,description=EXCLUDED.description,
             legacy_grid_x=EXCLUDED.legacy_grid_x,legacy_grid_y=EXCLUDED.legacy_grid_y",
            &[
                &place.id,
                &place.world_id,
                &place.topology_node_ref,
                &place.parent_place_id,
                &place.name,
                &place.kind,
                &place.scale,
                &place.canon,
                &place.status,
                &place.traversable,
                &place.source_kind,
                &place.zone,
                &tags,
                &place.description,
                &place.legacy_grid_x,
                &place.legacy_grid_y,
            ],
        )?;
    }
    Ok(())
}

fn upsert_routes(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
    routes: &[WorldRoute],
) -> anyhow::Result<()> {
    for route in routes {
        tx.execute(
            "INSERT INTO world_routes
             (id,from_place_id,direction,to_place_id,mode,travel_cost,canon,status,source_kind)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (id) DO UPDATE SET from_place_id=EXCLUDED.from_place_id,
             direction=EXCLUDED.direction,to_place_id=EXCLUDED.to_place_id,mode=EXCLUDED.mode,
             travel_cost=EXCLUDED.travel_cost,canon=EXCLUDED.canon,status=EXCLUDED.status",
            &[
                &route.id,
                &route.from_place_id,
                &route.direction,
                &route.to_place_id,
                &route.mode,
                &route.travel_cost,
                &route.canon,
                &route.status,
                &route.source_kind,
            ],
        )?;
    }
    Ok(())
}

fn migrate_missing_place_ids(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
) -> anyhow::Result<u64> {
    Ok(tx.execute(
        "UPDATE entities e SET place_id=p.id FROM world_places p
         WHERE e.place_id IS NULL AND p.source_kind='legacy_grid'
         AND p.legacy_grid_x=e.grid_x AND p.legacy_grid_y=e.grid_y",
        &[],
    )?)
}

fn mirror_entity_locations(
    tx: &mut r2d2_postgres::postgres::Transaction<'_>,
) -> anyhow::Result<u64> {
    let changed = tx.execute(
        "UPDATE entities e SET grid_x=p.legacy_grid_x,grid_y=p.legacy_grid_y
         FROM world_places p WHERE e.place_id=p.id AND p.legacy_grid_x IS NOT NULL
         AND (e.grid_x IS DISTINCT FROM p.legacy_grid_x OR e.grid_y IS DISTINCT FROM p.legacy_grid_y)",
        &[],
    )?;
    tx.execute(
        "INSERT INTO entity_rooms(entity_id,room_id)
         SELECT e.id,'grid:' || p.legacy_grid_x::text || ':' || p.legacy_grid_y::text
         FROM entities e JOIN world_places p ON p.id=e.place_id
         WHERE p.legacy_grid_x IS NOT NULL
         ON CONFLICT(entity_id) DO UPDATE SET room_id=EXCLUDED.room_id",
        &[],
    )?;
    Ok(changed)
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

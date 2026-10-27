use crate::store;
use axum::{Json, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct EcoMapCell {
    x: i32,
    y: i32,
    t: String,
    z: String,
    n: String,
    s: String,
    o: Vec<String>,
}

#[derive(Serialize)]
struct EcoEntity {
    id: String,
    name: String,
    x: i32,
    y: i32,
    kind: String,
    activity: String,
}

trait TerrainExt {
    fn as_str(&self) -> &'static str;
}

impl TerrainExt for crate::grid::Terrain {
    fn as_str(&self) -> &'static str {
        self.kind_str()
    }
}

trait CellExt {
    fn symbol(&self) -> &'static str;
}

impl CellExt for crate::grid::GridCell {
    fn symbol(&self) -> &'static str {
        self.terrain.symbol()
    }
}

/// 取得全城地圖數據（柵格化）
pub async fn eco_map() -> impl IntoResponse {
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let cells: Vec<EcoMapCell> = grid
            .cells()
            .map(|cell| EcoMapCell {
                x: cell.coord.x,
                y: cell.coord.y,
                t: cell.terrain.as_str().to_string(),
                z: cell.zone.clone(),
                n: cell.name.clone(),
                s: cell.symbol().to_string(),
                o: cell.objects.iter().map(|obj| obj.name.clone()).collect(),
            })
            .collect();
        return Json(cells).into_response();
    }

    let pool = match store::get_db_pool() {
        Some(p) => p,
        None => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };

    let cells: Vec<EcoMapCell> = if let Ok(rows) = conn.query(
        "SELECT x, y, terrain, zone, name, symbol FROM grid_cells",
        &[],
    ) {
        rows.iter()
            .map(|r| EcoMapCell {
                x: r.get(0),
                y: r.get(1),
                t: r.get(2),
                z: r.get(3),
                n: r.get(4),
                s: r.get(5),
                o: Vec::new(),
            })
            .collect()
    } else {
        let rows = conn
            .query("SELECT x, y, terrain, zone, name FROM grid_cells", &[])
            .unwrap_or_default();
        rows.iter()
            .map(|r| {
                let x: i32 = r.get(0);
                let y: i32 = r.get(1);
                let t: String = r.get(2);
                let z: String = r.get(3);
                let n: String = r.get(4);
                let s = crate::grid::Terrain::from_kind_str(&t)
                    .map(|tr| tr.symbol().to_string())
                    .unwrap_or_else(|| "·".to_string());
                EcoMapCell {
                    x,
                    y,
                    t,
                    z,
                    n,
                    s,
                    o: Vec::new(),
                }
            })
            .collect()
    };

    Json(cells).into_response()
}

/// 取得所有實體的即時位置
pub async fn eco_entities() -> impl IntoResponse {
    let Some(st_arc) = store::get_store() else {
        return Json(Vec::<EcoEntity>::new()).into_response();
    };
    let st = st_arc.read();

    let entities: Vec<EcoEntity> = st
        .entities
        .values()
        .filter_map(|e| {
            let (x, y) = st.effective_entity_grid(e)?;
            let name = if e.display_title.trim().is_empty() {
                e.id.clone()
            } else {
                e.display_title.clone()
            };
            let activity = e.current_activity.clone();
            Some(EcoEntity {
                id: e.id.clone(),
                name,
                x,
                y,
                kind: e.kind.clone(),
                activity,
            })
        })
        .collect();

    Json(entities).into_response()
}

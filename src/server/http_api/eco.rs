use axum::{Json, response::IntoResponse};
use serde::Serialize;
use crate::store;

#[derive(Serialize)]
struct EcoMapCell {
    x: i32,
    y: i32,
    t: String,
    z: String,
}

#[derive(Serialize)]
struct EcoEntity {
    id: String,
    x: i32,
    y: i32,
    kind: String,
}

/// 取得全城地圖數據（柵格化）
pub async fn eco_map() -> impl IntoResponse {
    let pool = match store::get_db_pool() {
        Some(p) => p,
        None => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };
    
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return Json(Vec::<EcoMapCell>::new()).into_response(),
    };

    let rows = conn.query("SELECT x, y, terrain, zone FROM grid_cells", &[]).unwrap_or_default();
    let cells: Vec<EcoMapCell> = rows.iter().map(|r| {
        EcoMapCell {
            x: r.get(0),
            y: r.get(1),
            t: r.get(2),
            z: r.get(3),
        }
    }).collect();

    Json(cells).into_response()
}

/// 取得所有實體的即時位置
pub async fn eco_entities() -> impl IntoResponse {
    let Some(st_arc) = store::get_store() else {
        return Json(Vec::<EcoEntity>::new()).into_response();
    };
    let st = st_arc.read();
    
    let entities: Vec<EcoEntity> = st.entities.values().map(|e| {
        EcoEntity {
            id: e.id.clone(),
            x: e.grid_x.unwrap_or(0),
            y: e.grid_y.unwrap_or(0),
            kind: e.kind.clone(),
        }
    }).collect();

    Json(entities).into_response()
}

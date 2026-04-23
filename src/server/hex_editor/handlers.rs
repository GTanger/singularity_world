//! HTTP handlers：reveal / cell / wall / portal / transport / save / reload / path / neighbors。

use axum::extract::{Json, Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::db::load_hex_grid;
use crate::hex::{
    allowed_terrain_for_pin, api_contract_pins, generate_wild_cell, is_player_spawn_pin,
    is_undeletable_contract, reveal_hex_disk, HexCell, HexCoord, HexDir, HexGrid, LinkLayer,
    Portal, TransportEdge, TransportEndpoint,
};
use crate::server::http_api::AdminQuery;
use crate::server::run::AppState;

use super::state::{authz_check, hex_state, save_to_disk};
use super::types::{
    err_json, ok_count, ok_json, CellReq, PathGetQuery, PortalReq, RevealRegionReq, RevealReq,
    TransportEdgeReq, WallReq, WorldSeedReq,
};

macro_rules! auth {
    ($state:expr, $q:expr) => {
        if let Some(r) = authz_check(&$state.cfg, &$q) {
            return r;
        }
    };
}

/// POST /api/hex/reveal — 單格首次揭露（契約釘死）
pub async fn reveal_post(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<RevealReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(req.q, req.r);
    let mut grid = hex_state().grid.write().unwrap();
    if let Some(cell) = grid.get(coord).cloned() {
        drop(grid);
        return Json(serde_json::json!({
            "ok": true,
            "already_revealed": true,
            "cell": cell,
        }))
        .into_response();
    }
    let cell = generate_wild_cell(&grid, coord);
    grid.insert(cell.clone());
    drop(grid);
    let _ = save_to_disk();
    Json(serde_json::json!({
        "ok": true,
        "already_revealed": false,
        "cell": cell,
    }))
    .into_response()
}

/// POST /api/hex/reveal-region — 批量揭露（近→遠順序）
pub async fn reveal_region_post(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<RevealRegionReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let center = HexCoord::new(req.center_q, req.center_r);
    let mut grid = hex_state().grid.write().unwrap();
    let new_cells = reveal_hex_disk(&mut grid, center, req.radius);
    drop(grid);
    let _ = save_to_disk();
    let count = hex_state().grid.read().unwrap().len();
    Json(serde_json::json!({
        "ok": true,
        "new_cells": new_cells,
        "total_cells": count,
    }))
    .into_response()
}

/// PUT /api/hex/world-seed — 設定 world_seed（揭露用 RNG）
pub async fn world_seed_put(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<WorldSeedReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let mut grid = hex_state().grid.write().unwrap();
    grid.set_world_seed(req.world_seed);
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// GET /api/hex/grid — 回傳完整 grid，附 `contract_pins`。
pub async fn grid_get(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    auth!(st, q);
    let grid = hex_state().grid.read().unwrap();
    let mut v = match serde_json::to_value(&*grid) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    if let Some(obj) = v.as_object_mut() {
        obj.insert(
            "contract_pins".to_string(),
            serde_json::to_value(api_contract_pins()).unwrap_or_else(|_| serde_json::json!([])),
        );
    }
    Json(v).into_response()
}

/// PUT /api/hex/cell — 建立或更新一個格子
pub async fn cell_put(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<CellReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(req.q, req.r);
    if !allowed_terrain_for_pin(coord, req.terrain) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "遊戲釘死之出生格 (0,0) 須為草原（grassland）"
            })),
        )
            .into_response();
    }
    let mut tags = req.tags;
    if is_player_spawn_pin(coord) && !tags.iter().any(|t| t == "player_spawn") {
        tags.push("player_spawn".into());
    }
    let mut cell = HexCell::new(coord, req.terrain, &req.name);
    cell.zone = req.zone;
    cell.tags = tags;
    cell.description = req.description;
    cell.objects = req.objects;

    let mut grid = hex_state().grid.write().unwrap();
    grid.insert(cell);
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// PUT /api/hex/cells — 批次建立/更新
pub async fn cells_put(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(reqs): Json<Vec<CellReq>>,
) -> impl IntoResponse {
    auth!(st, q);
    let count = reqs.len();
    let mut grid = hex_state().grid.write().unwrap();
    for req in reqs {
        let coord = HexCoord::new(req.q, req.r);
        if !allowed_terrain_for_pin(coord, req.terrain) {
            drop(grid);
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "error": "批次內含遊戲釘死之出生格 (0,0)，該格須為草原（grassland）"
                })),
            )
                .into_response();
        }
        let mut tags = req.tags;
        if is_player_spawn_pin(coord) && !tags.iter().any(|t| t == "player_spawn") {
            tags.push("player_spawn".into());
        }
        let mut cell = HexCell::new(coord, req.terrain, &req.name);
        cell.zone = req.zone;
        cell.tags = tags;
        cell.description = req.description;
        cell.objects = req.objects;
        grid.insert(cell);
    }
    drop(grid);
    let _ = save_to_disk();
    ok_count(count).into_response()
}

/// DELETE /api/hex/cell/{q}/{r}
pub async fn cell_delete(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Path((cq, cr)): Path<(i32, i32)>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(cq, cr);
    if is_undeletable_contract(coord) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": "遊戲釘死之契約格不可刪除（出生點 0,0）"
            })),
        )
            .into_response();
    }
    let mut grid = hex_state().grid.write().unwrap();
    let existed = grid.remove(coord).is_some();
    drop(grid);
    if existed {
        let _ = save_to_disk();
    }
    Json(serde_json::json!({"ok": true, "existed": existed})).into_response()
}

/// PUT /api/hex/wall
pub async fn wall_put(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<WallReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let a = HexCoord::new(req.aq, req.ar);
    let b = HexCoord::new(req.bq, req.br);
    if !a.is_adjacent(b) {
        return err_json("格子不相鄰").into_response();
    }
    let mut grid = hex_state().grid.write().unwrap();
    if req.remove {
        grid.remove_wall(a, b);
    } else {
        grid.add_wall(a, b);
    }
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// POST /api/hex/portal
pub async fn portal_post(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<PortalReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let portal = Portal {
        name: req.name,
        from: HexCoord::new(req.from_q, req.from_r),
        to: HexCoord::new(req.to_q, req.to_r),
        bidirectional: req.bidirectional,
        counts_as_official_link: req.counts_as_official_link,
    };
    let mut grid = hex_state().grid.write().unwrap();
    grid.add_portal(portal);
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// POST /api/hex/transport-edge
pub async fn transport_edge_post(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<TransportEdgeReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let edge = TransportEdge {
        id: req.id,
        endpoint_a: TransportEndpoint::Cell(HexCoord::new(req.aq, req.ar)),
        endpoint_b: TransportEndpoint::Cell(HexCoord::new(req.bq, req.br)),
        mode: req.mode,
        operational: req.operational,
        link_class: req.link_class,
        weight: req.weight,
    };
    let mut grid = hex_state().grid.write().unwrap();
    grid.add_transport_edge(edge);
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// POST /api/hex/save
pub async fn save(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    auth!(st, q);
    match save_to_disk() {
        Ok(()) => {
            let count = hex_state().grid.read().unwrap().len();
            ok_count(count).into_response()
        }
        Err(e) => err_json(&e).into_response(),
    }
}

/// POST /api/hex/reload
pub async fn reload(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    auth!(st, q);
    let new_grid = if let Some(g) = load_hex_grid() {
        g
    } else {
        let path = &hex_state().path;
        match std::fs::read_to_string(path) {
            Ok(json) => match serde_json::from_str::<HexGrid>(&json) {
                Ok(g) => g,
                Err(e) => return err_json(&format!("JSON 解析失敗：{e}")).into_response(),
            },
            Err(e) => {
                return err_json(&format!("PG 無有效資料且讀取備份失敗：{e}")).into_response();
            }
        }
    };
    let count = new_grid.len();
    *hex_state().grid.write().unwrap() = new_grid;
    let _ = save_to_disk();
    ok_count(count).into_response()
}

/// GET /api/hex/path
pub async fn path_get(
    State(st): State<AppState>,
    Query(q): Query<PathGetQuery>,
) -> impl IntoResponse {
    auth!(st, q.admin);
    let from = HexCoord::new(q.from_q, q.from_r);
    let to = HexCoord::new(q.to_q, q.to_r);
    let layer: LinkLayer = q.layer.into();
    let grid = hex_state().grid.read().unwrap();
    let path = grid.find_path_layer(from, to, layer);
    Json(serde_json::json!({
        "from": from,
        "to": to,
        "layer": match layer {
            LinkLayer::Official => "official",
            LinkLayer::Exploration => "exploration",
        },
        "path": path,
        "reachable": path.is_some(),
    }))
    .into_response()
}

/// GET /api/hex/neighbors/{q}/{r}
pub async fn neighbors_get(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Path((cq, cr)): Path<(i32, i32)>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(cq, cr);
    let grid = hex_state().grid.read().unwrap();
    let walkable: Vec<HexCoord> = coord
        .neighbors()
        .into_iter()
        .filter(|n| grid.can_walk(coord, *n))
        .collect();
    let all_dirs: Vec<serde_json::Value> = HexDir::ALL
        .iter()
        .map(|d| {
            let nb = coord.neighbor(*d);
            serde_json::json!({
                "dir": d,
                "coord": nb,
                "exists": grid.contains(nb),
                "walkable": grid.can_walk(coord, nb),
                "blocked": grid.is_blocked(coord, *d),
            })
        })
        .collect();
    Json(serde_json::json!({
        "coord": coord,
        "walkable": walkable,
        "directions": all_dirs,
    }))
    .into_response()
}

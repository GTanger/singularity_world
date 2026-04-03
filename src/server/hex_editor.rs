//! 六角格地圖編輯器 API
//!
//! 以 `HexGrid` 為核心，提供 cell CRUD、barrier、儲存/載入。
//! 持久化：**PostgreSQL `hex_world` 為主**，`data/hex/grid.json` 為備份。
//! 前端透過這組 API 操作地圖，不再經由 room-editor。

use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use axum::extract::{Json, Path, Query, State};
use axum::response::IntoResponse;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::db::{load_hex_grid, save_hex_grid_to_pg};
use crate::hex::{
    generate_wild_cell, reveal_hex_disk, HexCell, HexCoord, HexDir, HexGrid, LinkLayer, Portal,
    Terrain, TransportEdge, TransportEndpoint, TransportLinkClass, TransportMode,
};
use crate::model::RoomObject;
use crate::server::http_api::AdminQuery;
use crate::server::run::AppState;

// ── 全域狀態 ─────────────────────────────────────────────────────

static HEX_STATE: OnceLock<HexState> = OnceLock::new();

struct HexState {
    grid: RwLock<HexGrid>,
    path: PathBuf,
}

/// 啟動時呼叫：優先自 PostgreSQL `hex_world` 載入；無列則自 `grid.json` 種子灌入 PG。
pub fn init(data_path: &str) {
    let path = PathBuf::from(data_path);
    let grid = if let Some(g) = load_hex_grid() {
        g
    } else if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(json) => {
                let g: HexGrid = serde_json::from_str(&json).unwrap_or_default();
                if let Err(e) = save_hex_grid_to_pg(&g) {
                    tracing::warn!("hex_world: 種子寫入 PG 失敗：{e}");
                }
                g
            }
            Err(_) => HexGrid::new(),
        }
    } else {
        HexGrid::new()
    };
    let count = grid.len();
    let _ = HEX_STATE.set(HexState {
        grid: RwLock::new(grid),
        path,
    });
    tracing::info!("hex_editor: 載入 {count} 格（主資料：PostgreSQL hex_world）");
}

fn hex_state() -> &'static HexState {
    HEX_STATE.get().expect("hex_editor::init 未呼叫")
}

fn save_to_disk() -> Result<(), String> {
    let st = hex_state();
    let grid = st.grid.read().map_err(|e| e.to_string())?;
    if let Err(e) = save_hex_grid_to_pg(&grid) {
        tracing::warn!("hex_world: PG 寫入失敗（仍寫入 JSON 備份）：{e}");
    }
    if let Some(dir) = st.path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&*grid).map_err(|e| e.to_string())?;
    std::fs::write(&st.path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// 確保座標在世界格網中已有契約；若無則決定性生成並落盤。  
/// 供多人玩法：每位玩家再於 `hex_player_reveal` 標記自己的「已見」。
pub fn ensure_world_cell_at(coord: HexCoord) -> Result<(HexCell, bool), String> {
    let mut grid = hex_state().grid.write().map_err(|e| e.to_string())?;
    if let Some(c) = grid.get(coord) {
        return Ok((c.clone(), false));
    }
    let cell = generate_wild_cell(&grid, coord);
    grid.insert(cell.clone());
    drop(grid);
    save_to_disk()?;
    Ok((cell, true))
}

/// 新角色野外**唯一**出生點：世界座標 **(0,0)** 契約為**草原**（`Terrain::Grassland`）。
/// 若該格尚不存在則揭露生成；若已存在但非草原則改地形並落盤（PostgreSQL + 備份）。
pub fn ensure_player_spawn_grassland_coord() -> Result<(i32, i32), String> {
    let coord = HexCoord::new(0, 0);
    let (mut cell, _) = ensure_world_cell_at(coord)?;
    if cell.terrain != Terrain::Grassland {
        cell.terrain = Terrain::Grassland;
        cell.name = format!("草原·{}", coord.to_cell_id());
        {
            let mut grid = hex_state().grid.write().map_err(|e| e.to_string())?;
            grid.insert(cell);
        }
        save_to_disk()?;
    }
    Ok((coord.q, coord.r))
}

fn is_admin(cfg: &crate::config::Server, q: &AdminQuery) -> bool {
    if cfg.management_key.is_empty() {
        return true;
    }
    q.mg_key.as_deref() == Some(&cfg.management_key)
}

macro_rules! auth {
    ($state:expr, $q:expr) => {
        if !is_admin(&$state.cfg, &$q) {
            return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"unauthorized"}))).into_response();
        }
    };
}

// ── 請求 / 回應型別 ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CellReq {
    pub q: i32,
    pub r: i32,
    #[serde(default)]
    pub terrain: Terrain,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub objects: Vec<RoomObject>,
}

#[derive(Deserialize)]
pub struct WallReq {
    pub aq: i32,
    pub ar: i32,
    pub bq: i32,
    pub br: i32,
    #[serde(default)]
    pub remove: bool,
}

#[derive(Deserialize)]
pub struct PortalReq {
    pub name: String,
    pub from_q: i32,
    pub from_r: i32,
    pub to_q: i32,
    pub to_r: i32,
    #[serde(default = "default_true")]
    pub bidirectional: bool,
    /// 預設 false：一般 portal 不計入官方聯外子圖
    #[serde(default)]
    pub counts_as_official_link: bool,
}

#[derive(Deserialize)]
pub struct TransportEdgeReq {
    pub id: Option<String>,
    pub aq: i32,
    pub ar: i32,
    pub bq: i32,
    pub br: i32,
    pub mode: TransportMode,
    #[serde(default = "default_true")]
    pub operational: bool,
    #[serde(default)]
    pub link_class: TransportLinkClass,
    pub weight: Option<f64>,
}

fn default_true() -> bool {
    true
}

/// GET /api/hex/path 查詢參數（含 `mg_key`）
#[derive(Deserialize)]
pub struct PathGetQuery {
    #[serde(flatten)]
    pub admin: AdminQuery,
    pub from_q: i32,
    pub from_r: i32,
    pub to_q: i32,
    pub to_r: i32,
    /// `official`（官方聯外子圖）或 `exploration`（預設）
    #[serde(default)]
    pub layer: HexPathLayer,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HexPathLayer {
    #[default]
    Exploration,
    Official,
}

impl From<HexPathLayer> for LinkLayer {
    fn from(v: HexPathLayer) -> Self {
        match v {
            HexPathLayer::Exploration => LinkLayer::Exploration,
            HexPathLayer::Official => LinkLayer::Official,
        }
    }
}

/// POST /api/hex/reveal — 單格揭露（已存在則不重算）
#[derive(Deserialize)]
pub struct RevealReq {
    pub q: i32,
    pub r: i32,
}

/// POST /api/hex/reveal-region — 批量揭露
#[derive(Deserialize)]
pub struct RevealRegionReq {
    pub center_q: i32,
    pub center_r: i32,
    pub radius: i32,
}

/// PUT /api/hex/world-seed
#[derive(Deserialize)]
pub struct WorldSeedReq {
    pub world_seed: u64,
}

fn ok_json() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true}))
}

fn ok_count(n: usize) -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "count": n}))
}

fn err_json(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": msg})))
}

// ── Handlers ─────────────────────────────────────────────────────

/// POST /api/hex/reveal — 野外單格首次揭露（契約釘死）
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

/// PUT /api/hex/world-seed — 設定野外生成種子
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

/// GET /api/hex/grid — 回傳完整 grid
pub async fn grid_get(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    auth!(st, q);
    let grid = hex_state().grid.read().unwrap();
    Json(serde_json::to_value(&*grid).unwrap()).into_response()
}

/// PUT /api/hex/cell — 建立或更新一個格子
pub async fn cell_put(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<CellReq>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(req.q, req.r);
    let mut cell = HexCell::new(coord, req.terrain, &req.name);
    cell.zone = req.zone;
    cell.tags = req.tags;
    cell.description = req.description;
    cell.objects = req.objects;

    let mut grid = hex_state().grid.write().unwrap();
    grid.insert(cell);
    drop(grid);
    let _ = save_to_disk();
    ok_json().into_response()
}

/// PUT /api/hex/cells — 批次建立/更新多個格子
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
        let mut cell = HexCell::new(coord, req.terrain, &req.name);
        cell.zone = req.zone;
        cell.tags = req.tags;
        cell.description = req.description;
        cell.objects = req.objects;
        grid.insert(cell);
    }
    drop(grid);
    let _ = save_to_disk();
    ok_count(count).into_response()
}

/// DELETE /api/hex/cell/{q}/{r} — 刪除一個格子
pub async fn cell_delete(
    State(st): State<AppState>,
    Query(q): Query<AdminQuery>,
    Path((cq, cr)): Path<(i32, i32)>,
) -> impl IntoResponse {
    auth!(st, q);
    let coord = HexCoord::new(cq, cr);
    let mut grid = hex_state().grid.write().unwrap();
    let existed = grid.remove(coord).is_some();
    drop(grid);
    if existed {
        let _ = save_to_disk();
    }
    Json(serde_json::json!({"ok": true, "existed": existed})).into_response()
}

/// PUT /api/hex/wall — 新增或移除牆壁
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

/// POST /api/hex/portal — 新增傳送門
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

/// POST /api/hex/transport-edge — 新增一條明式交通邊（雙端皆為格座標）
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

/// POST /api/hex/save — 手動持久化
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

/// POST /api/hex/reload — 自 PostgreSQL 重新載入（無列時退回 `grid.json` 備份）
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

/// GET /api/hex/path — BFS 最短路（`layer`：official | exploration）
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

/// GET /api/hex/neighbors/{q}/{r} — 查詢可行走的鄰居
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

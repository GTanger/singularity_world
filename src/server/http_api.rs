//! HTTP API 端點（對齊 Go `admin_rooms.go`、`api_rooms_data.go`、`api_player_room.go`、
//! `api_topology.go`、`config.ServeDesignConstants`、行內 `wipe-entities`）。

use axum::extract::{Json, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::{config, db, gametext, model, store};

// ── /api/design-constants ──

pub async fn design_constants() -> impl IntoResponse {
    Json(config::design_constants())
}

// ── 管理：檢查管理金鑰 ──

#[derive(Deserialize)]
pub struct AdminQuery {
    pub mg_key: Option<String>,
}

fn is_admin_authorized(cfg: &config::Server, query: &AdminQuery) -> bool {
    // 若未設定管理金鑰，預設開放（相容開發模式）
    if cfg.management_key.is_empty() {
        return true;
    }
    query.mg_key.as_deref() == Some(&cfg.management_key)
}

// ── /api/admin/wipe-entities ──

pub async fn wipe_entities(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權的管理操作"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    if let Err(e) = s.clear_all_entities() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    let body = gametext::admin_wipe_response();
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], body).into_response()
}

// ── /api/player-room ──

#[derive(Deserialize)]
pub struct PlayerRoomQuery {
    id: Option<String>,
    pw: Option<String>,
}

#[derive(Serialize)]
pub struct PlayerRoomResponse {
    player_id: String,
    room_id: String,
}

pub async fn player_room(Query(q): Query<PlayerRoomQuery>) -> impl IntoResponse {
    let (Some(player_id), Some(password)) = (q.id, q.pw) else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"需提供 id 與 pw 參數"}))).into_response();
    };
    let ok = db::verify_password(&player_id, &password).unwrap_or(false);
    if !ok {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"身份驗證失敗"}))).into_response();
    }
    let room_id = match db::get_entity_room(&player_id) {
        Ok(r) => r,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"查詢房間失敗"}))).into_response(),
    };
    Json(PlayerRoomResponse { player_id, room_id }).into_response()
}

// ── /data/rooms.json ──

#[derive(Serialize)]
struct RoomsDataResponse {
    rooms: Vec<RoomDataItem>,
    exits: Vec<ExitDataItem>,
}

#[derive(Serialize)]
struct RoomDataItem {
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
    zone: String,
    objects: Vec<model::RoomObject>,
}

#[derive(Serialize)]
struct ExitDataItem {
    from: String,
    direction: String,
    to: String,
}

pub async fn rooms_data() -> impl IntoResponse {
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let s = match st.read() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let ids = s.room_ids();
    let mut rooms = Vec::with_capacity(ids.len());
    let mut exits = Vec::with_capacity(256);
    for id in &ids {
        let Some(room) = s.get_room(id) else { continue };
        rooms.push(RoomDataItem {
            id: room.id.clone(),
            name: room.name.clone(),
            description: room.description.clone(),
            tags: room.tags.clone(),
            zone: room.zone.clone(),
            objects: room.objects.clone(),
        });
        for e in s.get_exits_for_room(id) {
            exits.push(ExitDataItem {
                from: id.clone(),
                direction: e.direction,
                to: e.to_room_id,
            });
        }
    }
    Json(RoomsDataResponse { rooms, exits }).into_response()
}

// ── /api/rooms (admin CRUD) ──

// GET /api/rooms → 列出所有房間
pub async fn list_rooms(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let s = match st.read() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let ids = s.room_ids();
    let mut list = Vec::with_capacity(ids.len());
    for id in &ids {
        if let Some(r) = s.get_room(id) {
            list.push(serde_json::json!({
                "id": r.id, "name": r.name, "description": r.description,
                "tags": r.tags, "zone": r.zone,
            }));
        }
    }
    Json(serde_json::json!({"rooms": list})).into_response()
}

// POST /api/rooms → 新增房間
#[derive(Deserialize)]
pub struct CreateRoomReq {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

pub async fn create_room(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Json(body): Json<CreateRoomReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if body.id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"need id, name, description"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let name = if body.name.is_empty() { body.id.clone() } else { body.name };
    let room = model::Room {
        id: body.id.clone(),
        name,
        description: body.description,
        tags: vec![],
        zone: String::new(),
        objects: vec![],
    };
    {
        let mut s = match st.write() {
            Ok(guard) => guard,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
        };
        if s.get_room(&body.id).is_some() {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"room id already exists"}))).into_response();
        }
        s.upsert_room_data(room, None);
    }
    (StatusCode::CREATED, Json(serde_json::json!({"id": body.id}))).into_response()
}

// GET /api/rooms/:id → 取得單一房間詳細資料（包含物件）
pub async fn get_room_admin(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let s = match st.read() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let Some(room) = s.get_room(&id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    let exits = s.get_exits_for_room(&id);
    Json(serde_json::json!({
        "room": room,
        "exits": exits
    })).into_response()
}

// PUT /api/rooms/:id → 更新房間
#[derive(Deserialize)]
pub struct UpdateRoomReq {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    zone: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    objects: Vec<model::RoomObject>,
}

pub async fn update_room(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path(id): Path<String>,
    Json(body): Json<UpdateRoomReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let Some(mut room) = s.get_room(&id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    room.name = body.name;
    room.description = body.description;
    room.zone = body.zone;
    room.tags = body.tags;
    room.objects = body.objects;

    let exits_snapshot: Vec<model::Exit> = s.get_exits_for_room(&id);
    s.upsert_room_data(room, Some(exits_snapshot));
    Json(serde_json::json!({"id": id})).into_response()
}

// DELETE /api/rooms/:id → 刪除房間
pub async fn delete_room(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    if db::get_spawn_room_id() == id {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"cannot delete spawn room (界壁)"}))).into_response();
    }
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    s.delete_room_data(&id);
    Json(serde_json::json!({"deleted": id})).into_response()
}

// POST /api/rooms/:id/rename → 重新命名房間 ID
#[derive(Deserialize)]
pub struct RenameRoomReq {
    pub new_id: String,
}

pub async fn rename_room(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path(id): Path<String>,
    Json(body): Json<RenameRoomReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    if let Err(e) = s.rename_room(&id, &body.new_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    Json(serde_json::json!({"ok": true, "old_id": id, "new_id": body.new_id})).into_response()
}

// POST /api/rooms/:id/exits → 新增出口
#[derive(Deserialize)]
pub struct AddExitReq {
    direction: String,
    to_room_id: String,
}

pub async fn add_exit(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path(id): Path<String>,
    Json(body): Json<AddExitReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let Some(room) = s.get_room(&id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    let mut exits = s.get_exits_for_room(&id);
    // 若已有同方向出口，替換；否則新增
    if let Some(pos) = exits.iter().position(|e| e.direction == body.direction) {
        exits[pos].to_room_id = body.to_room_id.clone();
        exits[pos].to_room_name = s.get_room_name(&body.to_room_id);
    } else {
        let to_name = s.get_room_name(&body.to_room_id);
        exits.push(model::Exit {
            direction: body.direction.clone(),
            to_room_id: body.to_room_id.clone(),
            to_room_name: to_name,
        });
    }
    s.upsert_room_data(room, Some(exits));
    (StatusCode::CREATED, Json(serde_json::json!({"from": id, "direction": body.direction, "to": body.to_room_id}))).into_response()
}

// DELETE /api/rooms/:from_id/exits/:direction → 移除出口
pub async fn remove_exit(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    Query(q): Query<AdminQuery>,
    Path((from_id, direction)): Path<(String, String)>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"store lock poisoned"}))).into_response(),
    };
    let Some(room) = s.get_room(&from_id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    let exits: Vec<model::Exit> = s.get_exits_for_room(&from_id)
        .into_iter()
        .filter(|e| e.direction != direction)
        .collect();
    s.upsert_room_data(room, Some(exits));
    Json(serde_json::json!({"removed": format!("{from_id} {direction}")})).into_response()
}

// ── /api/topology ──

#[derive(Serialize)]
struct TopoNode {
    id: String,
    name: String,
    zone: i32,
    system: String,
}

#[derive(Serialize)]
struct TopoEdge {
    from: String,
    to: String,
    #[serde(rename = "type")]
    edge_type: String,
    cost: f64,
}

#[derive(Serialize)]
struct TopoResponse {
    player_id: String,
    nodes: Vec<TopoNode>,
    edges: Vec<TopoEdge>,
}

const HUB_NAMES: [&str; 20] = [
    "天極", "脈衝", "震淵", "游離", "弦絲",
    "曜核", "凜晶", "淵流", "萬象", "解離",
    "鎮閾", "衡定", "穹壁", "重塑", "逆熵",
    "神淵", "識閾", "坍縮", "無相", "越權",
];

const HUB_SYSTEMS: [&str; 20] = [
    "體", "敏", "體", "敏", "敏",
    "氣", "氣", "氣", "氣", "氣",
    "體", "氣", "體", "體", "氣",
    "氣", "敏", "氣", "敏", "敏",
];

const LOGIC_NAMES: [&str; 5] = ["起", "承", "轉", "協", "合"];
const PERIPHERAL_NAMES: [&str; 12] = [
    "探", "觸", "納", "蓄", "濾", "析",
    "融", "衍", "律", "束", "釋", "散",
];

fn node_id(n: usize) -> String {
    format!("N{n:03}")
}

fn build_topology_nodes() -> Vec<TopoNode> {
    let mut nodes = Vec::with_capacity(361);
    nodes.push(TopoNode { id: "N000".into(), name: "生之奇點".into(), zone: 0, system: String::new() });
    for i in 1..=20 {
        nodes.push(TopoNode {
            id: node_id(i), name: HUB_NAMES[i - 1].into(), zone: 1, system: HUB_SYSTEMS[i - 1].into(),
        });
    }
    for i in 1..=20usize {
        let sys = HUB_SYSTEMS[i - 1];
        for j in 1..=5usize {
            let nid = 20 + 5 * (i - 1) + j;
            nodes.push(TopoNode { id: node_id(nid), name: LOGIC_NAMES[j - 1].into(), zone: 2, system: sys.into() });
        }
    }
    for i in 1..=20usize {
        let sys = HUB_SYSTEMS[i - 1];
        for s in 1..=12usize {
            let nid = 120 + 12 * (i - 1) + s;
            nodes.push(TopoNode { id: node_id(nid), name: PERIPHERAL_NAMES[s - 1].into(), zone: 3, system: sys.into() });
        }
    }
    nodes
}

fn build_topology_edges(costs: &[f64]) -> Vec<TopoEdge> {
    let mut edges = Vec::with_capacity(760);
    let mut idx = 0usize;
    // A 邊：N000 → 20 主樞
    for i in 1..=20usize {
        edges.push(TopoEdge { from: "N000".into(), to: node_id(i), edge_type: "A".into(), cost: costs[idx] });
        idx += 1;
    }
    // B 邊：主樞 → 邏輯閘
    for i in 1..=20usize {
        for j in 1..=5usize {
            let blue = 20 + 5 * (i - 1) + j;
            edges.push(TopoEdge { from: node_id(i), to: node_id(blue), edge_type: "B".into(), cost: costs[idx] });
            idx += 1;
        }
    }
    // C 邊：邏輯閘 → 微型狀態
    let blue_green_map: [[usize; 3]; 5] = [
        [1, 2, 3], [3, 4, 5], [5, 6, 7], [8, 9, 10], [10, 11, 12],
    ];
    for i in 1..=20usize {
        for (j, green_slots) in blue_green_map.iter().enumerate() {
            let blue_id = 20 + 5 * (i - 1) + (j + 1);
            for &gs in green_slots {
                let green_id = 120 + 12 * (i - 1) + gs;
                edges.push(TopoEdge { from: node_id(blue_id), to: node_id(green_id), edge_type: "C".into(), cost: costs[idx] });
                idx += 1;
            }
        }
    }
    // D 邊：微型狀態環
    for i in 1..=20usize {
        let base = 120 + 12 * (i - 1);
        for s in 1..=12usize {
            let from = base + s;
            let next = s % 12 + 1;
            let to = base + next;
            edges.push(TopoEdge { from: node_id(from), to: node_id(to), edge_type: "D".into(), cost: costs[idx] });
            idx += 1;
        }
    }
    // E 邊：邏輯閘環
    for i in 1..=20usize {
        let base = 20 + 5 * (i - 1);
        for j in 1..=5usize {
            let from = base + j;
            let next = j % 5 + 1;
            let to = base + next;
            edges.push(TopoEdge { from: node_id(from), to: node_id(to), edge_type: "E".into(), cost: costs[idx] });
            idx += 1;
        }
    }
    edges
}

#[derive(Deserialize)]
pub struct TopologyQuery {
    id: Option<String>,
    pw: Option<String>,
}

pub async fn topology(Query(q): Query<TopologyQuery>) -> impl IntoResponse {
    let (Some(player_id), Some(password)) = (q.id, q.pw) else {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"需提供 id 與 pw 參數"}))).into_response();
    };
    let ok = db::verify_password(&player_id, &password).unwrap_or(false);
    if !ok {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"身份驗證失敗"}))).into_response();
    }
    let ent = match db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"角色不存在"}))).into_response(),
    };
    let Some(seed) = ent.soul_seed else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":"角色無 SoulSeed"}))).into_response();
    };
    let costs = db::expand_soul_seed_to_topology_costs(seed);
    let resp = TopoResponse {
        player_id,
        nodes: build_topology_nodes(),
        edges: build_topology_edges(&costs),
    };
    Json(resp).into_response()
}

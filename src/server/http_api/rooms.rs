//! 房間 CRUD + 出口 + rooms.json。

use axum::extract::{Json, Path, Query};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::{db, model, store};

use super::admin::{is_admin_authorized, AdminQuery};

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
    let s = st.read();
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
    let s = st.read();
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
        let mut s = st.write();
        if s.get_room(&body.id).is_some() {
            return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"room id already exists"}))).into_response();
        }
        s.upsert_room_data(room, None);
    }
    (StatusCode::CREATED, Json(serde_json::json!({"id": body.id}))).into_response()
}

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
    let s = st.read();
    let Some(room) = s.get_room(&id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    let exits = s.get_exits_for_room(&id);
    Json(serde_json::json!({
        "room": room,
        "exits": exits
    })).into_response()
}

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
    let mut s = st.write();
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
    let mut s = st.write();
    s.delete_room_data(&id);
    Json(serde_json::json!({"deleted": id})).into_response()
}

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
    let mut s = st.write();
    if let Err(e) = s.rename_room(&id, &body.new_id) {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    Json(serde_json::json!({"ok": true, "old_id": id, "new_id": body.new_id})).into_response()
}

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
    let mut s = st.write();
    let Some(room) = s.get_room(&id) else {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response();
    };
    let mut exits = s.get_exits_for_room(&id);
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
    let mut s = st.write();
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

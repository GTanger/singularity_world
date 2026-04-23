//! HTTP handlers：graph / create / update / delete / link / layout / reload / groups。

use std::fs;

use axum::extract::{Json, Path as AxPath, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;

use crate::server::http_api::AdminQuery;
use crate::server::run::AppState;
use crate::store;

use super::io::{
    add_or_replace_exit, ensure_move_object_for_exit, ensure_store_room, is_admin_authorized,
    load_groups, load_layout, normalize_id_for_file, read_room_file_by_id, rooms_base_path,
    save_groups, save_layout, walk_room_files, write_room_file,
};
use super::types::{
    CreateReq, LayoutReq, LinkReq, RoomEditorEdge, RoomEditorExit, RoomEditorGraphResp,
    RoomEditorNode, RoomEditorRoomFile, UpdateReq,
};

// GET /api/room-editor/graph
pub async fn graph(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let s = st.read();
    let mut ids = s.room_ids();
    ids.sort();
    let mut nodes = Vec::with_capacity(ids.len());
    let mut edges = Vec::with_capacity(256);
    for id in &ids {
        let Some(r) = s.get_room(id) else { continue };
        nodes.push(RoomEditorNode {
            id: r.id.clone(),
            name: r.name,
            description: r.description,
            zone: r.zone,
            tags: r.tags,
            objects: r.objects,
        });
        for e in s.get_exits_for_room(id) {
            edges.push(RoomEditorEdge {
                from: id.clone(),
                to: e.to_room_id,
                direction: e.direction,
            });
        }
    }
    Json(RoomEditorGraphResp {
        nodes,
        edges,
        layout: load_layout(),
        base_path: rooms_base_path().to_string_lossy().into_owned(),
    })
    .into_response()
}

// POST /api/room-editor/room
pub async fn create(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<CreateReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if req.id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid request"}))).into_response();
    }
    let idx = match walk_room_files() {
        Ok(i) => i,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    };
    if idx.contains_key(&req.id) {
        return (StatusCode::CONFLICT, Json(serde_json::json!({"error":"room id exists"}))).into_response();
    }
    let mut f = RoomEditorRoomFile {
        id: req.id.clone(),
        name: if req.name.is_empty() { req.id.clone() } else { req.name },
        description: req.description,
        zone: req.zone,
        tags: req.tags,
        objects: req.objects,
        exits: vec![],
    };
    if !req.clone_from.is_empty()
        && let Ok((src, _)) = read_room_file_by_id(&req.clone_from)
    {
        f.description = src.description;
        f.tags = src.tags;
        f.zone = src.zone;
        f.objects = src.objects;
    }
    let out_path = rooms_base_path().join("editor").join(format!("{}.json", normalize_id_for_file(&req.id)));
    if let Err(e) = write_room_file(&out_path, &f) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    ensure_store_room(&f);
    Json(serde_json::json!({"ok": true, "id": f.id, "path": out_path.to_string_lossy()})).into_response()
}

// PUT /api/room-editor/room/:id
pub async fn update(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    AxPath(id): AxPath<String>,
    Json(req): Json<UpdateReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let (mut f, path) = match read_room_file_by_id(&id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response(),
    };
    f.name = if req.name.is_empty() { f.id.clone() } else { req.name };
    f.description = req.description;
    f.zone = req.zone;
    f.tags = req.tags;
    f.objects = req.objects;
    if let Err(e) = write_room_file(&path, &f) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    ensure_store_room(&f);
    Json(serde_json::json!({"ok": true, "id": id})).into_response()
}

// DELETE /api/room-editor/room/:id
pub async fn delete(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    AxPath(id): AxPath<String>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let (_, path) = match read_room_file_by_id(&id) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"room not found"}))).into_response(),
    };
    if let Err(e) = fs::remove_file(&path) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    if let Ok(idx) = walk_room_files() {
        for rid in idx.keys() {
            if let Ok((mut rf, p)) = read_room_file_by_id(rid) {
                let before = rf.exits.len();
                rf.exits.retain(|ex| ex.to != id);
                if rf.exits.len() != before {
                    let _ = write_room_file(&p, &rf);
                    ensure_store_room(&rf);
                }
            }
        }
    }
    if let Some(st) = store::get_store() {
        let mut s = st.write();
        s.delete_room_data(&id);
    }
    Json(serde_json::json!({"ok": true, "deleted": id})).into_response()
}

// POST /api/room-editor/link
pub async fn link_create(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<LinkReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if req.from.is_empty() || req.to.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid request"}))).into_response();
    }
    let (mut from, from_path) = match read_room_file_by_id(&req.from) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"from room not found"}))).into_response(),
    };
    let (to, _to_path) = match read_room_file_by_id(&req.to) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"to room not found"}))).into_response(),
    };
    let dir = if req.direction.trim().is_empty() { to.name.clone() } else { req.direction };
    add_or_replace_exit(&mut from.exits, RoomEditorExit { direction: dir.clone(), to: req.to.clone() });
    ensure_move_object_for_exit(&mut from, &req.to, &dir, &to.name);
    if let Err(e) = write_room_file(&from_path, &from) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    ensure_store_room(&from);
    if req.reverse
        && let Ok((mut rev, rev_path)) = read_room_file_by_id(&req.to)
    {
        let rd = if req.reverse_direction.trim().is_empty() { from.name.clone() } else { req.reverse_direction.clone() };
        add_or_replace_exit(&mut rev.exits, RoomEditorExit { direction: rd.clone(), to: req.from.clone() });
        ensure_move_object_for_exit(&mut rev, &req.from, &rd, &from.name);
        let _ = write_room_file(&rev_path, &rev);
        ensure_store_room(&rev);
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

// DELETE /api/room-editor/link
pub async fn link_delete(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<LinkReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if req.from.is_empty() || req.to.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"invalid request"}))).into_response();
    }
    let (mut from, from_path) = match read_room_file_by_id(&req.from) {
        Ok(v) => v,
        Err(_) => return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error":"from room not found"}))).into_response(),
    };
    from.exits.retain(|ex| ex.to != req.to);
    let _ = write_room_file(&from_path, &from);
    ensure_store_room(&from);
    if req.reverse
        && let Ok((mut rev, rev_path)) = read_room_file_by_id(&req.to)
    {
        rev.exits.retain(|ex| ex.to != req.from);
        let _ = write_room_file(&rev_path, &rev);
        ensure_store_room(&rev);
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

// PUT /api/room-editor/layout
pub async fn layout(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(req): Json<LayoutReq>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if let Err(e) = save_layout(&req.positions) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

// POST /api/room-editor/reload
pub async fn reload(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    let Some(st) = store::get_store() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(serde_json::json!({"error":"store not initialized"}))).into_response();
    };
    let mut s = st.write();
    if let Err(e) = s.reload_rooms() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

// GET /api/room-editor/groups
pub async fn groups_get(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    Json(load_groups()).into_response()
}

// POST /api/room-editor/groups
pub async fn groups_post(
    State(state): State<AppState>,
    Query(q): Query<AdminQuery>,
    Json(groups): Json<Vec<Vec<String>>>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    if let Err(e) = save_groups(&groups) {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    Json(serde_json::json!({"ok": true})).into_response()
}

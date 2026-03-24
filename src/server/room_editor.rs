//! 房間心智圖編輯器 HTTP API（對齊 Go `room_editor_*.go` 全套）。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use axum::extract::{Json, Path as AxPath};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::{model, store, config};
use crate::server::http_api::{AdminQuery};

fn is_admin_authorized(cfg: &config::Server, query: &AdminQuery) -> bool {
    if cfg.management_key.is_empty() { return true; }
    query.mg_key.as_deref() == Some(&cfg.management_key)
}

// ── 型別（對齊 Go room_editor_types.go） ──

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoomEditorExit {
    direction: String,
    to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoomEditorRoomFile {
    id: String,
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    zone: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    exits: Vec<RoomEditorExit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    objects: Vec<model::RoomObject>,
}

#[derive(Serialize)]
struct RoomEditorNode {
    id: String,
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    zone: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    objects: Vec<model::RoomObject>,
}

#[derive(Serialize)]
struct RoomEditorEdge {
    from: String,
    to: String,
    direction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RoomEditorPos {
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct RoomEditorGraphResp {
    nodes: Vec<RoomEditorNode>,
    edges: Vec<RoomEditorEdge>,
    layout: HashMap<String, RoomEditorPos>,
    base_path: String,
}

// ── 路徑常數 ──

fn rooms_base_path() -> PathBuf {
    Path::new("data").join("rooms")
}

fn room_editor_layout_path() -> PathBuf {
    Path::new("data").join("runtime").join("room_editor_layout.json")
}

fn room_editor_groups_path() -> PathBuf {
    Path::new("data").join("runtime").join("editor_groups.json")
}

// ── layout 讀寫 ──

fn load_layout() -> HashMap<String, RoomEditorPos> {
    let Ok(b) = fs::read_to_string(room_editor_layout_path()) else { return HashMap::new() };
    serde_json::from_str(&b).unwrap_or_default()
}

fn save_layout(m: &HashMap<String, RoomEditorPos>) -> anyhow::Result<()> {
    let path = room_editor_layout_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let b = serde_json::to_string_pretty(m)?;
    fs::write(path, b)?;
    Ok(())
}

// ── groups 讀寫 ──

fn load_groups() -> Vec<Vec<String>> {
    let Ok(b) = fs::read_to_string(room_editor_groups_path()) else { return vec![] };
    serde_json::from_str(&b).unwrap_or_default()
}

fn save_groups(groups: &[Vec<String>]) -> anyhow::Result<()> {
    let path = room_editor_groups_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let b = serde_json::to_string_pretty(groups)?;
    fs::write(path, b)?;
    Ok(())
}

// ── 檔案系統：walk / read / write ──

fn walk_room_files() -> anyhow::Result<HashMap<String, PathBuf>> {
    let base = rooms_base_path();
    let mut idx = HashMap::new();
    walk_dir_recursive(&base, &mut idx)?;
    Ok(idx)
}

fn walk_dir_recursive(dir: &Path, idx: &mut HashMap<String, PathBuf>) -> anyhow::Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_dir_recursive(&path, idx)?;
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem.starts_with('_') {
            continue;
        }
        let Ok(b) = fs::read_to_string(&path) else { continue };
        let Ok(f) = serde_json::from_str::<RoomEditorRoomFile>(&b) else { continue };
        if f.id.is_empty() { continue; }
        idx.insert(f.id.clone(), path);
    }
    Ok(())
}

fn read_room_file_by_id(id: &str) -> anyhow::Result<(RoomEditorRoomFile, PathBuf)> {
    let idx = walk_room_files()?;
    let Some(path) = idx.get(id) else {
        anyhow::bail!("room not found");
    };
    let b = fs::read_to_string(path)?;
    let f: RoomEditorRoomFile = serde_json::from_str(&b)?;
    Ok((f, path.clone()))
}

fn write_room_file(path: &Path, f: &RoomEditorRoomFile) -> anyhow::Result<()> {
    if f.id.is_empty() {
        anyhow::bail!("invalid room");
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let b = serde_json::to_string_pretty(f)?;
    fs::write(path, b)?;
    Ok(())
}

fn normalize_id_for_file(id: &str) -> String {
    id.trim()
      .to_lowercase()
      .replace([' ', '/', '\\'], "_")
}

fn ensure_store_room(room: &RoomEditorRoomFile) {
    let Some(st) = store::get_store() else { return };
    let mut s = match st.write() {
        Ok(guard) => guard,
        Err(_) => return, // 靜靜失敗
    };
    let r = model::Room {
        id: room.id.clone(),
        name: room.name.clone(),
        description: room.description.clone(),
        tags: room.tags.clone(),
        zone: room.zone.clone(),
        objects: room.objects.clone(),
    };
    let exits: Vec<model::Exit> = room.exits.iter().map(|ex| {
        let to_name = s.get_room_name(&ex.to);
        model::Exit {
            direction: ex.direction.clone(),
            to_room_id: ex.to.clone(),
            to_room_name: if to_name.is_empty() { ex.to.clone() } else { to_name },
        }
    }).collect();
    s.upsert_room_data(r, Some(exits));
}

// ── exit helpers ──

fn add_or_replace_exit(list: &mut Vec<RoomEditorExit>, ex: RoomEditorExit) {
    if ex.direction.trim().is_empty() || ex.to.trim().is_empty() { return; }
    if let Some(pos) = list.iter().position(|e| e.direction == ex.direction) {
        list[pos] = ex;
    } else {
        list.push(ex);
    }
}

fn ensure_move_object_for_exit(room: &mut RoomEditorRoomFile, to_room_id: &str, direction: &str, target_name: &str) {
    if to_room_id.trim().is_empty() { return; }
    let dir = if direction.trim().is_empty() { to_room_id } else { direction };
    let tname = if target_name.trim().is_empty() { to_room_id } else { target_name };
    let default_move_text = format!("你前往「{tname}」。");

    for o in room.objects.iter_mut() {
        if o.move_to_room_id != to_room_id && o.id != to_room_id { continue; }
        o.move_to_room_id = to_room_id.to_string();
        if o.id.is_empty() { o.id = to_room_id.to_string(); }
        if o.name.is_empty() { o.name = dir.to_string(); }
        if !o.sockets.contains(&"Move".to_string()) { o.sockets.push("Move".into()); }
        if o.responses.get("Move").map(|s| s.trim().is_empty()).unwrap_or(true) {
            o.responses.insert("Move".into(), default_move_text);
        }
        return;
    }
    room.objects.push(model::RoomObject {
        id: to_room_id.to_string(),
        name: dir.to_string(),
        owner: String::new(),
        sockets: vec!["Move".into()],
        responses: HashMap::from([("Move".into(), default_move_text)]),
        move_to_room_id: to_room_id.to_string(),
    });
}

// ══════════ Handlers ══════════

// GET /api/room-editor/graph
pub async fn graph(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
    let mut ids = s.room_ids();
    ids.sort();
    let mut nodes = Vec::with_capacity(ids.len());
    let mut edges = Vec::with_capacity(256);
    for id in &ids {
        let Some(r) = s.get_room(id) else { continue };
        nodes.push(RoomEditorNode {
            id: r.id.clone(), name: r.name, description: r.description,
            zone: r.zone, tags: r.tags, objects: r.objects,
        });
        for e in s.get_exits_for_room(id) {
            edges.push(RoomEditorEdge { from: id.clone(), to: e.to_room_id, direction: e.direction });
        }
    }
    Json(RoomEditorGraphResp {
        nodes, edges, layout: load_layout(), base_path: rooms_base_path().to_string_lossy().into_owned(),
    }).into_response()
}

// POST /api/room-editor/room
#[derive(Deserialize)]
pub struct CreateReq {
    id: String,
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
    #[serde(default)]
    clone_from: String,
}

pub async fn create(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
        && let Ok((src, _)) = read_room_file_by_id(&req.clone_from) {
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
#[derive(Deserialize)]
pub struct UpdateReq {
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

pub async fn update(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
    // 清除其他房間指向被刪房間的出口
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
        if let Ok(mut s) = st.write() {
            s.delete_room_data(&id);
        }
    }
    Json(serde_json::json!({"ok": true, "deleted": id})).into_response()
}

// POST /api/room-editor/link
#[derive(Deserialize)]
pub struct LinkReq {
    from: String,
    to: String,
    #[serde(default)]
    direction: String,
    #[serde(default)]
    reverse: bool,
    #[serde(default)]
    reverse_direction: String,
}

pub async fn link_create(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
        && let Ok((mut rev, rev_path)) = read_room_file_by_id(&req.to) {
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
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
        && let Ok((mut rev, rev_path)) = read_room_file_by_id(&req.to) {
            rev.exits.retain(|ex| ex.to != req.from);
            let _ = write_room_file(&rev_path, &rev);
            ensure_store_room(&rev);
        }
    Json(serde_json::json!({"ok": true})).into_response()
}

// PUT /api/room-editor/layout
#[derive(Deserialize)]
pub struct LayoutReq {
    positions: HashMap<String, RoomEditorPos>,
}

pub async fn layout(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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
    if let Err(e) = s.reload_rooms() {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
    }
    StatusCode::NO_CONTENT.into_response()
}

// GET /api/room-editor/groups
pub async fn groups_get(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
) -> impl IntoResponse {
    if !is_admin_authorized(&state.cfg, &q) {
        return (StatusCode::FORBIDDEN, Json(serde_json::json!({"error":"未經授權"}))).into_response();
    }
    Json(load_groups()).into_response()
}

// POST /api/room-editor/groups
pub async fn groups_post(
    axum::extract::State(state): axum::extract::State<crate::server::run::AppState>,
    axum::extract::Query(q): axum::extract::Query<AdminQuery>,
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

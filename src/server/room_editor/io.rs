//! 檔案系統 I/O：layout / groups / walk / read / write / store 同步 + 授權。

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::server::http_api::AdminQuery;
use crate::{config, model, store};

use super::types::{RoomEditorExit, RoomEditorPos, RoomEditorRoomFile};

pub(super) fn is_admin_authorized(cfg: &config::Server, query: &AdminQuery) -> bool {
    if cfg.management_key.is_empty() {
        return true;
    }
    query.mg_key.as_deref() == Some(&cfg.management_key)
}

// ── 路徑常數 ──

pub(super) fn rooms_base_path() -> PathBuf {
    Path::new("data").join("rooms")
}

fn room_editor_layout_path() -> PathBuf {
    Path::new("data").join("runtime").join("room_editor_layout.json")
}

fn room_editor_groups_path() -> PathBuf {
    Path::new("data").join("runtime").join("editor_groups.json")
}

// ── layout 讀寫 ──

pub(super) fn load_layout() -> HashMap<String, RoomEditorPos> {
    let Ok(b) = fs::read_to_string(room_editor_layout_path()) else {
        return HashMap::new();
    };
    serde_json::from_str(&b).unwrap_or_default()
}

pub(super) fn save_layout(m: &HashMap<String, RoomEditorPos>) -> anyhow::Result<()> {
    let path = room_editor_layout_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let b = serde_json::to_string_pretty(m)?;
    fs::write(path, b)?;
    Ok(())
}

// ── groups 讀寫 ──

pub(super) fn load_groups() -> Vec<Vec<String>> {
    let Ok(b) = fs::read_to_string(room_editor_groups_path()) else {
        return vec![];
    };
    serde_json::from_str(&b).unwrap_or_default()
}

pub(super) fn save_groups(groups: &[Vec<String>]) -> anyhow::Result<()> {
    let path = room_editor_groups_path();
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let b = serde_json::to_string_pretty(groups)?;
    fs::write(path, b)?;
    Ok(())
}

// ── walk / read / write ──

pub(super) fn walk_room_files() -> anyhow::Result<HashMap<String, PathBuf>> {
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
        if f.id.is_empty() {
            continue;
        }
        idx.insert(f.id.clone(), path);
    }
    Ok(())
}

pub(super) fn read_room_file_by_id(id: &str) -> anyhow::Result<(RoomEditorRoomFile, PathBuf)> {
    let idx = walk_room_files()?;
    let Some(path) = idx.get(id) else {
        anyhow::bail!("room not found");
    };
    let b = fs::read_to_string(path)?;
    let f: RoomEditorRoomFile = serde_json::from_str(&b)?;
    Ok((f, path.clone()))
}

pub(super) fn write_room_file(path: &Path, f: &RoomEditorRoomFile) -> anyhow::Result<()> {
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

pub(super) fn normalize_id_for_file(id: &str) -> String {
    id.trim().to_lowercase().replace([' ', '/', '\\'], "_")
}

pub(super) fn ensure_store_room(room: &RoomEditorRoomFile) {
    let Some(st) = store::get_store() else { return };
    let mut s = st.write();
    let r = model::Room {
        id: room.id.clone(),
        name: room.name.clone(),
        description: room.description.clone(),
        tags: room.tags.clone(),
        zone: room.zone.clone(),
        objects: room.objects.clone(),
    };
    let exits: Vec<model::Exit> = room
        .exits
        .iter()
        .map(|ex| {
            let to_name = s.get_room_name(&ex.to);
            model::Exit {
                direction: ex.direction.clone(),
                to_room_id: ex.to.clone(),
                to_room_name: if to_name.is_empty() { ex.to.clone() } else { to_name },
            }
        })
        .collect();
    s.upsert_room_data(r, Some(exits));
}

// ── exit helpers ──

pub(super) fn add_or_replace_exit(list: &mut Vec<RoomEditorExit>, ex: RoomEditorExit) {
    if ex.direction.trim().is_empty() || ex.to.trim().is_empty() {
        return;
    }
    if let Some(pos) = list.iter().position(|e| e.direction == ex.direction) {
        list[pos] = ex;
    } else {
        list.push(ex);
    }
}

pub(super) fn ensure_move_object_for_exit(
    room: &mut RoomEditorRoomFile,
    to_room_id: &str,
    direction: &str,
    target_name: &str,
) {
    if to_room_id.trim().is_empty() {
        return;
    }
    let dir = if direction.trim().is_empty() { to_room_id } else { direction };
    let tname = if target_name.trim().is_empty() { to_room_id } else { target_name };
    let default_move_text = format!("你前往「{tname}」。");

    for o in room.objects.iter_mut() {
        if o.move_to_room_id != to_room_id && o.id != to_room_id {
            continue;
        }
        o.move_to_room_id = to_room_id.to_string();
        if o.id.is_empty() {
            o.id = to_room_id.to_string();
        }
        if o.name.is_empty() {
            o.name = dir.to_string();
        }
        if !o.sockets.contains(&"Move".to_string()) {
            o.sockets.push("Move".into());
        }
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

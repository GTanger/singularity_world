//! 房間／物件／格點設定：純 store facade，不涉 NPC display cache。

use crate::store;

use super::{room_hex, ErrNoStore};

/// 創生預設房間名稱。
pub const SPAWN_ROOM_NAME: &str = "宜林";

/// 取得創生預設房間 id。
pub fn get_spawn_room_id() -> String {
    let Some(arc) = store::get_store() else { return "lobby".to_string() };
    let s = arc.read();
    let id = s.get_room_id_by_name(SPAWN_ROOM_NAME);
    if id.is_empty() { "lobby".to_string() } else { id }
}

/// 有房間座標的 NPC id 列表（對齊既有 `GetNPCIDsWithRoom`）。
#[must_use]
pub fn get_npc_ids_with_room() -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read();
    s.get_npc_ids_with_room()
}

/// 有房間座標的玩家 id 列表（對齊既有 `GetPlayerIDsWithRoom`）。
#[must_use]
pub fn get_player_ids_with_room() -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read();
    s.get_player_ids_with_room()
}

/// 回傳房間的戰鬥地形標籤。
pub fn terrain_from_room(room_id: &str) -> String {
    let Some(arc) = store::get_store() else { return String::new() };
    let s = arc.read();
    let Some(room) = s.get_room(room_id) else { return String::new() };
    for t in &room.tags {
        let lt = t.trim().to_lowercase();
        if matches!(lt.as_str(), "lush" | "chaos" | "silent" | "grip") {
            return lt;
        }
    }
    if room.zone.trim().to_lowercase() == "chaos" {
        return "chaos".to_string();
    }
    String::new()
}

/// 依 id 查房間（對齊既有 `GetRoom`）。
pub fn get_room(room_id: &str) -> anyhow::Result<Option<crate::model::Room>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s.get_room(room_id))
}

/// 房間顯示名稱（對齊既有 `GetRoomName`）。
pub fn get_room_name(room_id: &str) -> anyhow::Result<String> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s.get_room_name(room_id))
}

/// 將實體設為在指定房間（對齊既有 `SetEntityRoom`）。
/// 若 `room_id` 為 `hex:q:r` 或於 `room_hex_overlay.json`／創生房規則可解析為六角，則改寫權威座標為 [`set_entity_hex`]。
pub fn set_entity_room(entity_id: &str, room_id: &str) -> anyhow::Result<()> {
    if let Some((q, r)) = crate::hex::parse_hex_room_id(room_id) {
        return set_entity_hex(entity_id, q, r);
    }
    if let Some((q, r)) = room_hex::room_hex_for_world_room(room_id) {
        return set_entity_hex(entity_id, q, r);
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.set_entity_room(entity_id, room_id)?;
    s.clear_entity_hex(entity_id)
}

/// 權威六角座標；見 [`store::Store::set_entity_hex`]（同步 `entity_rooms` 為 `hex:…`）。
pub fn set_entity_hex(entity_id: &str, q: i32, r: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.set_entity_hex(entity_id, q, r)
}

/// 權威正方格座標；見 [`store::Store::set_entity_grid`]（同步 `entity_rooms` 為 `grid:…`）。
pub fn set_entity_grid(entity_id: &str, x: i32, y: i32) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.set_entity_grid(entity_id, x, y)
}

/// 有正方格座標的存活 NPC id 列表。
#[must_use]
pub fn get_npc_ids_with_grid() -> Vec<String> {
    let Some(arc) = store::get_store() else {
        return Vec::new();
    };
    let s = arc.read();
    s.get_npc_ids_with_grid()
}

/// 設定實體的表面可觀測行為（玩家 Look 時看到的「在做什麼」）。
pub fn set_entity_activity(entity_id: &str, activity: &str) -> anyhow::Result<()> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.set_entity_activity(entity_id, activity)
}

/// 房間出口列表（對齊 `GetExitsForRoom`）。
pub fn get_exits_for_room(room_id: &str) -> anyhow::Result<Vec<crate::model::Exit>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s.get_exits_for_room(room_id))
}

/// 房間內可互動物件（對齊 `GetObjectsInRoom`）。
pub fn get_objects_in_room(room_id: &str) -> anyhow::Result<Vec<crate::model::RoomObject>> {
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let s = arc.read();
    Ok(s.get_room(room_id).map(|r| r.objects).unwrap_or_default())
}

/// 物件是否具備指定動詞插座（對齊 `ObjectHasSocket`）。
#[must_use]
pub fn object_has_socket(obj: &crate::model::RoomObject, action: &str) -> bool {
    obj.sockets.iter().any(|s| s.eq_ignore_ascii_case(action))
}

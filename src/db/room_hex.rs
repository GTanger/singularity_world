//! 房間 id 與六角座標對齊（`data/config/room_hex_overlay.json` + 創生房預設）。

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::Deserialize;

use crate::hex::{hex_room_id_from_coord, parse_hex_room_id};

#[derive(Debug, Deserialize)]
#[serde(default)]
struct OverlayFile {
    spawn_hex: SpawnHex,
    /// 明確覆寫：房間 id → 六角
    #[serde(default)]
    rooms: HashMap<String, SpawnHex>,
}

impl Default for OverlayFile {
    fn default() -> Self {
        Self {
            spawn_hex: SpawnHex { q: 0, r: 0 },
            rooms: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
struct SpawnHex {
    #[serde(default)]
    q: i32,
    #[serde(default)]
    r: i32,
}

static OVERLAY: OnceLock<OverlayFile> = OnceLock::new();

fn overlay() -> &'static OverlayFile {
    OVERLAY.get_or_init(|| {
        let path = "data/config/room_hex_overlay.json";
        let raw = std::fs::read_to_string(path).unwrap_or_else(|_| "{}".to_string());
        serde_json::from_str(&raw).unwrap_or_default()
    })
}

/// 已為 `hex:q:r` 或可由覆寫／創生房規則對應時，回傳 `(q,r)`。
pub fn resolve_room_to_hex(room_id: &str) -> Option<(i32, i32)> {
    if let Some(p) = parse_hex_room_id(room_id) {
        return Some(p);
    }
    room_hex_for_world_room(room_id)
}

/// 僅世界房間 id（非 `hex:` 前綴）之對應；無則檢查是否為創生預設房。
pub fn room_hex_for_world_room(room_id: &str) -> Option<(i32, i32)> {
    if room_id.is_empty() || parse_hex_room_id(room_id).is_some() {
        return None;
    }
    let o = overlay();
    if let Some(h) = o.rooms.get(room_id) {
        return Some((h.q, h.r));
    }
    let spawn_id = super::get_spawn_room_id();
    if room_id == spawn_id {
        return Some((o.spawn_hex.q, o.spawn_hex.r));
    }
    None
}

/// 用於集合比對：能對應六角者回傳 `hex:q:r`，否則回傳原字串。
#[must_use]
pub fn canonical_location_key(room_id: &str) -> String {
    if let Some((q, r)) = resolve_room_to_hex(room_id) {
        return hex_room_id_from_coord(q, r);
    }
    room_id.to_string()
}

/// 兩個位置字串是否指同一格（`hex:…` 與可解析之世界房 id 視為相同）。
#[must_use]
pub fn location_keys_equivalent(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    canonical_location_key(a) == canonical_location_key(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_hex_stable() {
        assert_eq!(canonical_location_key("hex:0:0"), "hex:0:0");
    }

    #[test]
    fn equivalent_hex_and_self() {
        assert!(location_keys_equivalent("hex:1:-2", "hex:1:-2"));
    }

    #[test]
    fn inequivalent_distinct_unmapped() {
        assert!(!location_keys_equivalent("room_a", "room_b"));
    }
}

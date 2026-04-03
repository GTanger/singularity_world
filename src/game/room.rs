//! 房間視野與依出口移動（對齊 Go `game/room.go`）。

use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::hex::{hex_room_id_from_coord, HexCoord, HexDir};
use crate::store;
use crate::model::{Exit, Room, RoomObject};

/// 當前房間視野。
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub exits: Vec<Exit>,
    pub entities: Vec<Character>,
    pub objects: Vec<RoomObject>,
}

/// 依房間 id 載入視野；`game_hour` 0–23 供 NPC 職稱，`-1` 表示不套用下班規則。
pub fn get_room_view(room_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(room) = db::get_room(room_id)? else {
        return Ok(None);
    };
    let exits = db::get_exits_for_room(room_id)?;
    let entities = db::get_entities_in_room(room_id, game_hour)?;
    let objects = db::get_objects_in_room(room_id)?;
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

/// 由六角格權威資料組裝 `RoomView`（登入／野外主路徑）。
///
/// 鄰接出口僅列出 [`crate::hex::HexGrid::can_walk`] 可通行者；同格實體見 [`crate::db::get_entities_at_hex`]。
pub fn get_hex_room_view(
    _player_id: &str,
    q: i32,
    r: i32,
    game_hour: i32,
) -> anyhow::Result<Option<RoomView>> {
    let Some(grid) = db::load_hex_grid() else {
        return Ok(None);
    };
    let coord = HexCoord::new(q, r);
    let Some(cell) = grid.get(coord) else {
        return Ok(None);
    };
    let room = Room {
        id: hex_room_id_from_coord(q, r),
        name: cell.name.clone(),
        tags: cell.tags.clone(),
        zone: cell.zone.clone(),
        description: cell.description.clone(),
        objects: cell.objects.clone(),
    };
    let mut exits: Vec<Exit> = Vec::new();
    for nc in grid.walkable_neighbors(coord) {
        let ncoord = nc.coord;
        let Some(dir) = coord.direction_to(ncoord) else {
            continue;
        };
        exits.push(Exit {
            direction: dir.label_zh().to_string(),
            to_room_id: hex_room_id_from_coord(ncoord.q, ncoord.r),
            to_room_name: nc.name.clone(),
        });
    }
    let entities = db::get_entities_at_hex(q, r, game_hour)?;
    let objects = cell.objects.clone();
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

fn hex_dir_from_exit_label(dir: &str) -> Option<HexDir> {
    let t = dir.trim();
    HexDir::ALL.into_iter().find(|&d| d.label_zh() == t)
}

/// 依視野出口方向（與 [`HexDir::label_zh`] 相同之方位字）在六角網上走一步。
/// 成功時已寫入 [`crate::db::set_entity_hex`]（含 `entity_rooms`）。
pub fn move_by_hex_direction(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let Some(grid) = db::load_hex_grid() else {
        return Ok((String::new(), false));
    };
    let arc = store::get_store().ok_or_else(|| anyhow::anyhow!("no store"))?;
    let s = arc.read().unwrap();
    let Some(e) = s.get_entity(entity_id) else {
        return Ok((String::new(), false));
    };
    let (Some(q), Some(r)) = (e.hex_q, e.hex_r) else {
        return Ok((String::new(), false));
    };
    drop(s);
    let Some(hdir) = hex_dir_from_exit_label(dir) else {
        return Ok((String::new(), false));
    };
    let coord = HexCoord::new(q, r);
    let to = coord.neighbor(hdir);
    if !grid.can_walk(coord, to) {
        return Ok((hex_room_id_from_coord(q, r), false));
    }
    db::set_entity_hex(entity_id, to.q, to.r)?;
    Ok((hex_room_id_from_coord(to.q, to.r), true))
}

/// 若實體尚無房間則設為預設房間，並回傳房間 id。
pub fn ensure_entity_in_room(entity_id: &str, default_room_id: &str) -> anyhow::Result<String> {
    let mut room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        db::set_entity_room(entity_id, default_room_id)?;
        room_id = default_room_id.to_string();
    }
    Ok(room_id)
}

/// 依出口方向移動；回傳 `(新房間 id, 是否成功)`。
pub fn move_by_exit(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        return Ok((String::new(), false));
    }
    let exits = db::get_exits_for_room(&room_id)?;
    for ex in &exits {
        if ex.direction.trim() == dir {
            db::set_entity_room(entity_id, &ex.to_room_id)?;
            return Ok((ex.to_room_id.clone(), true));
        }
    }
    let objs = db::get_objects_in_room(&room_id)?;
    for o in objs {
        if o.name.trim() != dir {
            continue;
        }
        if o.move_to_room_id.is_empty() || !object_has_socket(&o, "Move") {
            continue;
        }
        db::set_entity_room(entity_id, &o.move_to_room_id)?;
        return Ok((o.move_to_room_id.clone(), true));
    }
    Ok((String::new(), false))
}

#[must_use]
pub fn entity_kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Player => "player",
        EntityKind::Npc => "npc",
    }
}

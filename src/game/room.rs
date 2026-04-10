use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::hex::{hex_room_id_from_coord, HexCoord, HexDir, Terrain};
use crate::model::{Exit, Room, RoomObject};
use crate::store;
use serde::{Deserialize, Serialize};

/// 當前房間視野。
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub exits: Vec<Exit>,
    pub entities: Vec<Character>,
    pub objects: Vec<RoomObject>,
}

/// 六角格區域視野（MVP 眼圖使用）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexAreaView {
    pub center: HexCoord,
    pub cells: Vec<HexCellView>,
    pub entities: Vec<HexEntityView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexCellView {
    pub q: i32,
    pub r: i32,
    pub terrain: Terrain,
    pub name: String,
    pub display_char: String,
    pub color: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_font: Option<String>,
    pub move_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexEntityView {
    pub id: String,
    pub q: i32,
    pub r: i32,
    pub display_char: String,
    pub kind: String,
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

/// 由六角格權威資料組裝 `RoomView`（登入／Hex 格網主路徑）。
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

/// 輔助函式：取得給定半徑內的所有六角座標。
fn range_coords(center: HexCoord, radius: i32) -> Vec<HexCoord> {
    let mut results = Vec::new();
    for q in -radius..=radius {
        for r in (-radius).max(-q - radius)..=(radius).min(-q + radius) {
            results.push(HexCoord::new(center.q + q, center.r + r));
        }
    }
    results
}

/// 取得六角格區域視野（MVP 眼圖使用）。
pub fn get_hex_area_view(
    q: i32,
    r: i32,
    radius: i32,
    game_hour: i32,
) -> anyhow::Result<HexAreaView> {
    let center = HexCoord::new(q, r);
    let mut cells = Vec::new();
    let mut entities = Vec::new();

    let grid = crate::server::hex_editor::get_runtime_grid()
        .or_else(db::load_hex_grid)
        .ok_or_else(|| anyhow::anyhow!("no grid"))?;

    // 1. 取得範圍內所有格子
    for coord in range_coords(center, radius) {
        if let Some(cell) = grid.get(coord) {
            cells.push(HexCellView {
                q: coord.q,
                r: coord.r,
                terrain: cell.terrain,
                name: cell.name.clone(),
                display_char: cell.terrain.display_char().to_string(),
                color: cell.terrain.color().to_string(),
                text_color: cell.terrain.text_color().map(String::from),
                text_font: cell.terrain.text_font().map(String::from),
                move_cost: cell.terrain.move_cost(),
            });

            // 2. 取得該格實體
            let ents = db::get_entities_at_hex(coord.q, coord.r, game_hour)?;
            for e in ents {
                entities.push(HexEntityView {
                    id: e.id,
                    q: coord.q,
                    r: coord.r,
                    display_char: e.display_char,
                    kind: entity_kind_str(&e.kind).to_string(),
                });
            }
        }
    }

    Ok(HexAreaView {
        center,
        cells,
        entities,
    })
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
    let Some(grid) = crate::server::hex_editor::get_runtime_grid()
        .or_else(db::load_hex_grid) else {
        return Ok((String::new(), false));
    };
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

use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::grid::{Direction, SquareCoord, grid_room_id_from_coord, terrain_name_zh};
use crate::model::{Exit, Room, RoomObject};
use crate::store;

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

// ─── 正方格地圖 ───────────────────────────────────────────────────

fn grid_objects_to_room_objects(objs: &[crate::grid::GridObject]) -> Vec<RoomObject> {
    objs.iter()
        .map(|o| RoomObject {
            id: o.id.clone(),
            name: o.name.clone(),
            owner: String::new(),
            sockets: vec!["Look".into()],
            responses: Default::default(),
            move_to_room_id: String::new(),
        })
        .collect()
}

/// 依方位字在正方格上走一步。成功時已寫入 DB。
pub fn move_by_grid_direction(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let Some(gdir) = Direction::from_zh(dir) else {
        return Ok((String::new(), false));
    };
    let arc = store::get_store().ok_or_else(|| anyhow::anyhow!("no store"))?;
    let s = arc.read();
    let Some(e) = s.get_entity(entity_id) else {
        return Ok((String::new(), false));
    };
    let (Some(x), Some(y)) = (e.grid_x, e.grid_y) else {
        return Ok((String::new(), false));
    };
    drop(s);

    let from = SquareCoord::new(x, y);
    let to = from.neighbor(gdir);

    let can = crate::server::grid_manager::with_square_grid_mut(|grid| {
        if !grid.can_walk(from, to) {
            return false;
        }
        crate::grid::reveal_grid_disk(grid, to, 2);
        true
    });
    if can != Some(true) {
        return Ok((grid_room_id_from_coord(x, y), false));
    }
    db::set_entity_grid(entity_id, to.x, to.y)?;
    Ok((grid_room_id_from_coord(to.x, to.y), true))
}

/// 組裝正方格 RoomView（當前格的實體、出口、物件）。
pub fn get_grid_room_view(
    _player_id: &str,
    x: i32,
    y: i32,
    game_hour: i32,
) -> anyhow::Result<Option<RoomView>> {
    let grid = crate::server::grid_manager::get_runtime_square_grid()
        .ok_or_else(|| anyhow::anyhow!("grid not loaded"))?;
    let coord = SquareCoord::new(x, y);
    let Some(cell) = grid.get(coord) else {
        return Ok(None);
    };
    let room_objs = grid_objects_to_room_objects(&cell.objects);
    let room = Room {
        id: grid_room_id_from_coord(x, y),
        name: crate::grid::terrain_name_zh(cell.terrain).to_string(),
        tags: cell.tags.clone(),
        zone: cell.zone.clone(),
        description: cell.description.clone(),
        objects: room_objs.clone(),
    };
    let mut exits: Vec<Exit> = Vec::new();
    for nc in grid.neighbors(coord) {
        if !nc.terrain.walkable() {
            continue;
        }
        let Some(dir) = coord.direction_to(nc.coord) else {
            continue;
        };
        exits.push(Exit {
            direction: dir.label_zh().to_string(),
            to_room_id: grid_room_id_from_coord(nc.coord.x, nc.coord.y),
            to_room_name: crate::grid::terrain_name_zh(nc.terrain).to_string(),
        });
    }
    let entities = db::get_entities_at_grid(x, y, game_hour)?;
    let objects = room_objs;
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

/// 取得正方格附近格子的簡要視野（地圖渲染用）。
///
/// Tuple 欄位順序：`(x, y, kind, terrain, category, name, explored, walkable)`
/// - `kind`：Terrain enum 變體 snake_case（送前端切色/icon）
/// - `terrain`：中文標籤（向後相容）
/// - `category`：`"terrain"` / `"landmark"` / `"infra"`
#[allow(clippy::type_complexity)]
pub fn get_grid_cells_around(
    x: i32,
    y: i32,
    radius: i32,
) -> Vec<(i32, i32, String, String, String, String, bool, bool)> {
    let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
        return Vec::new();
    };
    let center = SquareCoord::new(x, y);
    let mut out = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            let c = SquareCoord::new(x + dx, y + dy);
            if center.chebyshev_distance(c) > radius {
                continue;
            }
            if let Some(cell) = grid.get(c) {
                out.push((
                    c.x,
                    c.y,
                    cell.terrain.kind_str().to_string(),
                    terrain_name_zh(cell.terrain).to_string(),
                    cell.terrain.category().to_string(),
                    cell.name.clone(),
                    cell.explored,
                    cell.terrain.walkable(),
                ));
            }
        }
    }
    out
}

#[must_use]
pub fn entity_kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Player => "player",
        EntityKind::Npc => "npc",
    }
}

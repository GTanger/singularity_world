//! 移動／探索／離場觀測清除。六角與正方格雙路徑。

use crate::db;
use crate::entity::Character;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::gametext;

use super::super::protocol::{BlockedMsg, ClientMsg, MovedMsg};
use super::{current_game_hour, WsConnection};

/// 探索當前正方格：標記該格為 explored，重新推送 grid_view。
/// 層 3（grid_player_reveal DB）待 SW-4 實作；此版本僅重送視野以確保前端同步。
pub(super) fn handle_explore_grid(conn: &WsConnection) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let ent = match db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    let (gx, gy) = match (ent.grid_x, ent.grid_y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            conn.send_error("not in grid");
            return;
        }
    };
    // 把當前格標記為 explored（觸發主動探索語意）
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
            cell.explored = true;
        }
        grid.mark_dirty();
    });
    let gh = current_game_hour(&conn.cfg);
    send_grid_view(conn, &player_id, gx, gy, gh);
}

pub(super) fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    if msg.direction.is_empty() {
        conn.send_error("direction required");
        return;
    }
    let old_ent = db::get_entity(&player_id).ok().flatten();
    let Some(oe) = old_ent else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };

    // 正方格路徑
    if oe.grid_x.is_some() && oe.grid_y.is_some() {
        handle_move_grid(conn, &player_id, &msg.direction, &oe);
        return;
    }
    conn.send_error(gametext::client("move_failed"));
}

fn handle_move_grid(conn: &WsConnection, player_id: &str, direction: &str, _oe: &Character) {
    let (new_room, ok) = match game::move_by_grid_direction(player_id, direction) {
        Ok(v) => v,
        Err(_) => {
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    if !ok {
        let _ = event::append(game::now_unix(), player_id, BLOCKED, direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.to_string(),
        });
        return;
    }
    let Some(coord) = crate::grid::parse_grid_room_id(&new_room) else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };
    let gh = current_game_hour(&conn.cfg);
    send_grid_view(conn, player_id, coord.x, coord.y, gh);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.to_string(),
        room_id: new_room,
        room_name: String::new(),
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
}

pub(super) fn send_grid_view(conn: &WsConnection, player_id: &str, x: i32, y: i32, game_hour: i32) {
    use super::super::broadcast::build_room_view_msg;
    use super::super::protocol::{GridCellView, GridViewMsg};
    use crate::npc::narrative;

    let view = match game::get_grid_room_view(player_id, x, y, game_hour) {
        Ok(Some(v)) => v,
        _ => return,
    };
    let base = build_room_view_msg(&view, player_id, &conn.cfg);
    let cells_raw = game::get_grid_cells_around(x, y, 6);
    let cells: Vec<GridCellView> = cells_raw
        .into_iter()
        .map(|(cx, cy, terrain, name, explored, walkable)| GridCellView {
            x: cx,
            y: cy,
            terrain,
            name,
            explored,
            walkable,
        })
        .collect();

    // 為方格 NPC 填入行為敘事句（以當前 unix 秒 + id 為種子）
    let tick_seed = game::now_unix() as u64;
    let entities = base.entities.into_iter().map(|mut ent| {
        if (ent.kind == "npc" || ent.kind == "Npc") && ent.behavior_text.is_empty() {
            ent.behavior_text = narrative::idle(&ent.id, &ent.display_name, tick_seed);
        }
        ent
    }).collect();

    let grid_msg = GridViewMsg {
        msg_type: "grid_view".into(),
        player_x: x,
        player_y: y,
        cells,
        room_name: base.room_name,
        description: base.description,
        exits: base.exits,
        entities,
        objects: base.objects,
        server_unix: base.server_unix,
        game_time_sec_since_midnight: base.game_time_sec_since_midnight,
        game_days_since_epoch: base.game_days_since_epoch,
    };
    conn.send_json(&grid_msg);
}


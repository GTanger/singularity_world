//! `do_action` — 物件分支：房內非實體目標（RoomObject）的互動與 Move 穿梭。

use crate::db::{
    self, get_object_and_room, get_object_by_id_in_room, get_object_by_name_in_room,
    object_has_socket, object_response, set_entity_grid,
};
use crate::event::{self, types};
use crate::game;
use crate::gametext;
use crate::grid::parse_grid_room_id;

use super::super::broadcast::{refresh_room_views_for_room, send_room_view_to_session};
use super::super::handler::WsConnection;
use super::super::protocol::{ActionResultMsg, MovedMsg};
use super::{client_err_fmt, current_game_hour};

fn resolve_room_object(room_id: &str, target_id: &str) -> Option<crate::model::RoomObject> {
    get_object_by_id_in_room(room_id, target_id)
        .or_else(|| get_object_by_name_in_room(room_id, target_id))
        .or_else(|| {
            get_object_and_room(target_id).and_then(|(o, r)| (r == room_id).then_some(o))
        })
}

pub(super) fn do_object_branch(
    conn: &WsConnection,
    pid: &str,
    player_room: &str,
    target_id: &str,
    action: &str,
) {
    let Some(obj) = resolve_room_object(player_room, target_id) else {
        conn.send_error(gametext::client("target_not_found"));
        return;
    };
    if !object_has_socket(&obj, action) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return;
    }
    let mut narrative = object_response(&obj, action);
    if narrative.is_empty() {
        narrative = format!(
            "你對【{}】執行了「{action}」，但似乎沒有什麼特別的反應。",
            obj.name
        );
    }
    let now = game::now_unix();
    let _ = event::append(now, pid, types::OBSERVED, &obj.id);
    if !apply_object_move(conn, pid, player_room, &obj, action) {
        return;
    }
    let (others, move_target_id) = object_followup_buttons(&obj, player_room, action);
    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: action.into(),
        target_id: obj.id.clone(),
        target_name: obj.name.clone(),
        narrative,
        success: true,
        actions: others,
        move_target_id,
    });
}

fn object_followup_buttons(
    obj: &crate::model::RoomObject,
    player_room: &str,
    action: &str,
) -> (Vec<String>, String) {
    let mut others: Vec<String> = obj
        .sockets
        .iter()
        .filter(|s| *s != action)
        .cloned()
        .collect();
    let mut move_target_id = String::new();
    if action == "Look" && others.is_empty() && object_response(obj, "Look").is_empty() {
        let Ok(objs) = db::get_objects_in_room(player_room) else {
            return (others, move_target_id);
        };
        let idx = objs.iter().position(|o| o.id == obj.id);
        if let Some(i) = idx {
            for range in [(i + 1..objs.len()), (0..i)] {
                for j in range {
                    let o = &objs[j];
                    if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                        move_target_id.clone_from(&o.id);
                        break;
                    }
                }
                if !move_target_id.is_empty() {
                    break;
                }
            }
        } else {
            for o in &objs {
                if !o.move_to_room_id.is_empty() && object_has_socket(o, "Move") {
                    move_target_id.clone_from(&o.id);
                    break;
                }
            }
        }
        if !move_target_id.is_empty() {
            others = vec!["Move".into()];
        }
    }
    (others, move_target_id)
}

fn apply_object_move(
    conn: &WsConnection,
    pid: &str,
    _player_room: &str,
    obj: &crate::model::RoomObject,
    action: &str,
) -> bool {
    if action != "Move" || obj.move_to_room_id.is_empty() {
        return true;
    }
    let Some(coord) = parse_grid_room_id(&obj.move_to_room_id) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let gh = current_game_hour(&conn.cfg);
    let Ok(view_opt) = game::get_grid_room_view(pid, coord.x, coord.y, gh) else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    let Some(view) = view_opt else {
        conn.send_error(gametext::client("move_cannot_go"));
        return false;
    };
    if set_entity_grid(pid, coord.x, coord.y).is_err() {
        conn.send_error(gametext::client("move_failed"));
        return false;
    }
    let Some(session) = conn.sessions.get(pid) else {
        conn.send_error(gametext::client("move_failed"));
        return false;
    };
    send_room_view_to_session(&session, &view, pid, &conn.cfg);
    let rid = crate::grid::grid_room_id_from_coord(coord.x, coord.y);
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: pid.to_string(),
        room_id: rid.clone(),
        room_name: view.room.name.clone(),
    };
    if let Ok(bytes) = serde_json::to_vec(&moved) {
        conn.hub.broadcast(bytes);
    }
    refresh_room_views_for_room(&conn.sessions, &conn.cfg, &rid);
    true
}

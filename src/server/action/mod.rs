//! WebSocket `do_action` 主幹：分派到同房實體（talk/look/combat/borrow/trade）
//! 或房間物件（look/use/move）的動作。實際處理見各子模組。

mod borrow;
mod combat;
mod look;
mod object;
mod talk;
mod trade;

use crate::config::Server;
use crate::db;
use crate::entity::{self, Character, EntityKind, Verb};
use crate::game;
use crate::gametext;

use super::handler::{player_id, WsConnection};
use super::protocol::ClientMsg;

/// 與目標是否在同一可互動位置（六角重合優先，否則比對 `entity_rooms` 字串）。
fn entities_share_playable_cell(
    player: &Character,
    target: &Character,
    player_room: &str,
) -> bool {
    if let (Some(pq), Some(pr), Some(tq), Some(tr)) =
        (player.hex_q, player.hex_r, target.hex_q, target.hex_r)
        && pq == tq
        && pr == tr
    {
        return true;
    }
    let Ok(tr) = db::get_entity_room(&target.id) else {
        return false;
    };
    !player_room.is_empty() && db::location_keys_equivalent(player_room, &tr)
}

/// 分派 `do_action`：先試同房實體，再試房間物件。
pub fn handle_do_action(conn: &WsConnection, msg: &ClientMsg) {
    let Some(pid) = player_id(conn) else {
        conn.send_error(gametext::client("need_login"));
        return;
    };
    let target_id = msg.entity_id.clone();
    let mut action = msg.action.clone();
    if action == "Attack" {
        action = "Slay".into();
    }
    if target_id.is_empty() || action.is_empty() {
        conn.send_error(gametext::client("missing_target_or_action"));
        return;
    }
    if target_id == pid {
        conn.send_error(gametext::client("cannot_self_action"));
        return;
    }
    let Ok(Some(player_ch)) = db::get_entity(&pid) else {
        conn.send_error(gametext::client("get_self_failed"));
        return;
    };
    let Ok(player_room) = db::get_entity_room(&pid) else {
        conn.send_error(gametext::client("no_current_room"));
        return;
    };
    if player_room.is_empty() {
        conn.send_error(gametext::client("no_current_room"));
        return;
    }

    if try_entity_branch(conn, msg, &pid, &player_ch, &player_room, &target_id, &action) {
        return;
    }
    object::do_object_branch(conn, &pid, &player_room, &target_id, &action);
}

fn try_entity_branch(
    conn: &WsConnection,
    msg: &ClientMsg,
    pid: &str,
    player: &Character,
    player_room: &str,
    target_id: &str,
    action: &str,
) -> bool {
    let Ok(target_opt) = db::get_entity(target_id) else {
        return false;
    };
    let Some(target) = target_opt else {
        return false;
    };
    if !entities_share_playable_cell(player, &target, player_room) {
        conn.send_error(gametext::client("target_not_same_room"));
        return true;
    }
    if !entity_socket_ok(conn, &target, target_id, player_room, action) {
        return true;
    }
    let now = game::now_unix();
    match action {
        "Look" => look::do_entity_look(conn, pid, now, target_id, &target),
        "Talk" => talk::do_entity_talk(conn, msg, pid, player_room, target_id, &target, now),
        "Subdue" | "Slay" => {
            combat::do_entity_combat(conn, pid, player_room, target_id, action, &target, now);
        }
        "Borrow" => borrow::do_entity_borrow(conn, pid, target_id, &target, now),
        "Trade" => {
            let _ = trade::do_entity_trade(conn, msg, pid, target_id, &target, now);
        }
        _ => conn.send_error(client_err_fmt("unknown_action_fmt", action)),
    }
    true
}

pub(super) fn client_err_fmt(key: &str, arg: &str) -> String {
    gametext::client(key).replacen("%s", arg, 1)
}

fn entity_socket_ok(
    conn: &WsConnection,
    target: &Character,
    target_id: &str,
    player_room: &str,
    action: &str,
) -> bool {
    if target.kind == EntityKind::Npc {
        let sockets = db::get_sockets_for_npc(target_id, player_room);
        if !sockets.iter().any(|s| s == action) {
            conn.send_error(client_err_fmt("cannot_action_fmt", action));
            return false;
        }
        if !db::is_default_socket(action) {
            let in_venue =
                db::entity_in_venue_at_room(target_id, player_room).unwrap_or(false);
            if !in_venue {
                conn.send_error(client_err_fmt("target_not_at_workplace_fmt", action));
                return false;
            }
        }
        return true;
    }
    let Some(v) = verb_from_action(action) else {
        conn.send_error(client_err_fmt("unknown_action_fmt", action));
        return false;
    };
    if !entity::has_socket(&Character::sockets(), &v) {
        conn.send_error(client_err_fmt("cannot_action_fmt", action));
        return false;
    }
    true
}

fn verb_from_action(action: &str) -> Option<Verb> {
    match action {
        "Talk" => Some(Verb::Talk),
        "Borrow" => Some(Verb::Borrow),
        "Subdue" => Some(Verb::Subdue),
        "Slay" => Some(Verb::Slay),
        "Look" => Some(Verb::Look),
        "Trade" => Some(Verb::Trade),
        _ => None,
    }
}

pub(super) fn display_target_name(target: &Character) -> String {
    if !target.display_title.is_empty() {
        target.display_title.clone()
    } else {
        target.id.clone()
    }
}

pub(super) fn current_game_hour(cfg: &Server) -> i32 {
    if cfg.game_time_epoch_unix == 0 {
        return 12;
    }
    let (_, h, _, _) = game::game_time_now(
        game::now_unix(),
        cfg.game_time_epoch_unix,
        cfg.game_time_scale,
    );
    h
}

//! WebSocket 訊息分派（對齊 Go `server/handler.go` + `handler_auth` + `handler_move` 子集）。

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::config::Server;
use crate::db::{self, compute_resource_maxes, expand_soul_seed_to_origin_sentence, expand_soul_seed_to_topology_costs};
use crate::entity::Character;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::gametext;

use super::broadcast::send_room_view_to_session;
use super::hub::Hub;
use super::protocol::{BlockedMsg, ClientMsg, MeMsg, MovedMsg, PongMsg};
use super::session::{Session, SessionStore};
use uuid::Uuid;

/// 單一 WS 連線脈絡（對齊 Go `Client` + session 綁定）。
pub struct WsConnection {
    pub conn_id: Uuid,
    pub tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub player_id: RwLock<Option<String>>,
    pub sessions: Arc<SessionStore>,
    pub hub: Arc<Hub>,
    pub cfg: Server,
}

impl WsConnection {
    pub fn send_error(&self, message: impl Into<String>) {
        let msg = serde_json::json!({
            "type": "error",
            "message": message.into(),
        });
        let _ = self.tx.blocking_send(msg.to_string().into_bytes());
    }

    fn send_json<T: serde::Serialize>(&self, v: &T) {
        if let Ok(bytes) = serde_json::to_vec(v) {
            let _ = self.tx.blocking_send(bytes);
        }
    }
}

pub fn handle_message(conn: &WsConnection, raw: &[u8]) {
    let msg: ClientMsg = match serde_json::from_slice(raw) {
        Ok(m) => m,
        Err(_) => {
            conn.send_error("invalid json");
            return;
        }
    };
    match msg.msg_type.as_str() {
        "login" => handle_login(conn, &msg),
        "create_character" => handle_create_character(conn, &msg),
        "move" => handle_move(conn, &msg),
        "ping" => {
            conn.send_json(&PongMsg {
                msg_type: "pong".into(),
            });
        }
        other => {
            conn.send_error(format!("unknown type: {other}"));
        }
    }
}

fn handle_login(conn: &WsConnection, msg: &ClientMsg) {
    if msg.player_id.is_empty() {
        conn.send_error(gametext::client("login_need_id"));
        return;
    }
    if msg.password.is_empty() {
        conn.send_error(gametext::client("login_need_password"));
        return;
    }
    let Ok(ent) = db::get_entity(&msg.player_id) else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    let Some(ent) = ent else {
        conn.send_error(gametext::client("login_not_found"));
        return;
    };
    if !matches!(ent.kind, crate::entity::EntityKind::Player) {
        conn.send_error(gametext::client("login_not_player"));
        return;
    }
    if !db::has_password_for_entity(&msg.player_id) {
        conn.send_error(gametext::client("login_no_password"));
        return;
    }
    let Ok(ok) = db::verify_password(&msg.player_id, &msg.password) else {
        conn.send_error(gametext::client("login_verify_failed"));
        return;
    };
    if !ok {
        conn.send_error(gametext::client("login_wrong_password"));
        return;
    }
    login_success(conn, &msg.player_id);
}

fn handle_create_character(conn: &WsConnection, msg: &ClientMsg) {
    if msg.player_id.is_empty() {
        conn.send_error(gametext::client("create_need_id"));
        return;
    }
    if msg.password.is_empty() {
        conn.send_error(gametext::client("create_need_password"));
        return;
    }
    if msg.password.len() < 6 {
        conn.send_error(gametext::client("create_password_short"));
        return;
    }
    if msg.player_id.len() < 2 || msg.player_id.len() > 32 {
        conn.send_error(gametext::client("create_id_len"));
        return;
    }
    let Ok(existing) = db::get_entity(&msg.player_id) else {
        conn.send_error(gametext::client("create_failed"));
        return;
    };
    if existing.is_some() {
        conn.send_error(gametext::client("create_id_taken"));
        return;
    }
    let mut display_char = msg.display_char.clone();
    if display_char.is_empty() {
        display_char = gametext::client("display_char_default");
    }
    let chs: Vec<char> = display_char.chars().collect();
    if chs.len() > 1 {
        display_char = chs[0].to_string();
    }
    let gender = if msg.gender == "女" {
        "F"
    } else {
        "M"
    };
    if db::insert_entity(&msg.player_id, &display_char, gender).is_err() {
        conn.send_error(gametext::client("create_entity_failed"));
        return;
    }
    let spawn = db::get_spawn_room_id();
    if db::set_entity_room(&msg.player_id, &spawn).is_err() {
        conn.send_error(gametext::client("create_room_failed"));
        return;
    }
    if db::create_auth(&msg.player_id, &msg.password).is_err() {
        conn.send_error(gametext::client("create_auth_failed"));
        return;
    }
    login_success(conn, &msg.player_id);
}

fn login_success(conn: &WsConnection, player_id: &str) {
    let spawn = db::get_spawn_room_id();
    let room_id = match game::ensure_entity_in_room(player_id, &spawn) {
        Ok(r) => r,
        Err(_) => {
            conn.send_error(gametext::client("load_room_failed"));
            return;
        }
    };
    if let Ok(mut g) = conn.player_id.write() {
        *g = Some(player_id.to_string());
    }
    let gh = current_game_hour(&conn.cfg);
    let mut view = match game::get_room_view(&room_id, gh) {
        Ok(v) => v,
        Err(_) => {
            conn.send_error(gametext::client("load_view_failed"));
            return;
        }
    };
    if view.is_none() {
        let _ = db::set_entity_room(player_id, &spawn);
        view = match game::get_room_view(&spawn, gh) {
            Ok(v) => v,
            Err(_) => {
                conn.send_error(gametext::client("load_view_failed"));
                return;
            }
        };
        if view.is_none() {
            conn.send_error(gametext::client("load_view_failed"));
            return;
        }
    }
    let view = view.expect("checked");
    let session = Session::with_outbound(player_id, conn.conn_id, conn.tx.clone());
    conn.sessions.set(player_id, session.clone());
    let ent = db::get_entity(player_id).ok().flatten();
    let (vit, qi, dex) = ent
        .as_ref()
        .map(|e| (e.vit, e.qi, e.dex))
        .unwrap_or((10, 10, 10));
    let rm = compute_resource_maxes(vit, qi, dex);
    send_room_view_to_session(&session, &view, player_id, &conn.cfg);
    let now = game::now_unix();
    for e in &view.entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) && e.id != player_id {
            let _ = db::record_meet(&e.id, player_id);
        }
    }
    game::observe_room(&view.room.id, player_id, now);
    send_me_with_status(conn, &session, ent.as_ref(), player_id, &view.room.id, &view.room.name, vit, qi, dex, &rm);
}

fn current_game_hour(cfg: &Server) -> i32 {
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

fn parse_activated_nodes(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return vec!["N000".into()];
    }
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_else(|_| vec!["N000".into()])
}

#[allow(clippy::too_many_arguments)]
fn send_me_with_status(
    conn: &WsConnection,
    _session: &Session,
    ent: Option<&Character>,
    player_id: &str,
    room_id: &str,
    room_name: &str,
    vit: i32,
    qi: i32,
    dex: i32,
    rm: &db::ResourceMaxes,
) {
    let mut msg = MeMsg {
        msg_type: "me".into(),
        player_id: player_id.to_string(),
        room_id: room_id.to_string(),
        room_name: room_name.to_string(),
        vit,
        qi,
        dex,
        hp_cur: rm.hp_max as i32,
        hp_max: rm.hp_max as i32,
        inner_cur: rm.inner_max as i32,
        inner_max: rm.inner_max as i32,
        spirit_cur: rm.spirit_max as i32,
        spirit_max: rm.spirit_max as i32,
        stamina_cur: rm.stamina_max as i32,
        stamina_max: rm.stamina_max as i32,
        display_title: String::new(),
        origin_sentence: String::new(),
        activated_nodes: vec![],
        topology_costs: vec![],
        equipment_slots: HashMap::new(),
        equipment_names: HashMap::new(),
    };
    if let Some(ent) = ent {
        if !ent.display_title.is_empty() {
            msg.display_title.clone_from(&ent.display_title);
        }
        if let Some(seed) = ent.soul_seed {
            msg.origin_sentence = expand_soul_seed_to_origin_sentence(seed);
            msg.topology_costs = expand_soul_seed_to_topology_costs(seed);
        }
        msg.activated_nodes = parse_activated_nodes(&ent.activated_nodes);
        if !ent.equipment_slots.is_empty() {
            if let Ok(slots) = serde_json::from_str::<HashMap<String, String>>(&ent.equipment_slots) {
                msg.equipment_slots = slots;
            }
            msg.equipment_names = db::get_item_names(&ent.equipment_slots);
        }
    } else {
        msg.activated_nodes = vec!["N000".into()];
    }
    conn.send_json(&msg);
}

fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    if msg.direction.is_empty() {
        conn.send_error("direction required");
        return;
    }
    let old_room = db::get_entity_room(&player_id).unwrap_or_default();
    let (new_room, ok, err) = match game::move_by_exit(&player_id, &msg.direction) {
        Ok(v) => (v.0, v.1, None),
        Err(e) => (String::new(), false, Some(e)),
    };
    if err.is_some() {
        conn.send_error("move failed");
        return;
    }
    if !ok {
        let _ = event::append(
            game::now_unix(),
            &player_id,
            BLOCKED,
            &msg.direction,
        );
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: msg.direction.clone(),
        });
        return;
    }
    on_leave_room(conn.sessions.as_ref(), &old_room, &player_id);
    let gh = current_game_hour(&conn.cfg);
    let view = match game::get_room_view(&new_room, gh) {
        Ok(Some(v)) => v,
        Ok(None) | Err(_) => {
            conn.send_error("load room failed");
            return;
        }
    };
    let Some(sess) = conn.sessions.get(&player_id) else {
        conn.send_error("session lost");
        return;
    };
    send_room_view_to_session(&sess, &view, &player_id, &conn.cfg);
    let now = game::now_unix();
    game::observe_room(&view.room.id, &player_id, now);
    for e in &view.entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) && e.id != player_id {
            let _ = db::record_meet(&e.id, &player_id);
        }
    }
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.clone(),
        room_id: new_room.clone(),
        room_name: view.room.name.clone(),
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
}

fn on_leave_room(store: &SessionStore, room_id: &str, left_player_id: &str) {
    if room_id.is_empty() {
        return;
    }
    for s in store.all_sessions() {
        if s.player_id.is_empty() || s.player_id == left_player_id {
            continue;
        }
        if db::get_entity_room(&s.player_id).unwrap_or_default() == room_id {
            return;
        }
    }
    let Ok(entities) = db::get_entities_in_room(room_id, -1) else {
        return;
    };
    for e in entities {
        if matches!(e.kind, crate::entity::EntityKind::Npc) {
            let _ = db::clear_last_observed(&e.id);
        }
    }
}

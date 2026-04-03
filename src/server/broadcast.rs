//! 依房間推送敘事／視野（對齊 Go `server/broadcast.go`）。

use crate::config::Server;
use crate::db;
use crate::entity::{EntityKind, Verb};
use crate::game::{self, RoomView};

use super::protocol::{ExitView, NarrateMsg, RoomViewMsg, ViewEntity, ViewObject};
use super::session::Session;

fn must_json<T: serde::Serialize>(v: &T) -> Vec<u8> {
    serde_json::to_vec(v).unwrap_or_else(|_| br#"{"type":"error","message":"json"}"#.to_vec())
}

/// 同房在線玩家推送 `NarrateMsg`（世界房間 id 經 [`db::resolve_room_to_hex`] 與玩家六角對齊）。
pub fn send_narrate_to_room(store: &super::session::SessionStore, room_id: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    let msg = must_json(&NarrateMsg {
        msg_type: "narrate".into(),
        text: text.to_string(),
    });
    if let Some((q, r)) = db::resolve_room_to_hex(room_id) {
        for s in store.all_sessions() {
            let Ok(Some(ch)) = db::get_entity(&s.player_id) else {
                continue;
            };
            if ch.hex_q == Some(q) && ch.hex_r == Some(r) {
                let _ = s.try_send_bytes(msg.clone());
            }
        }
        return;
    }
    for s in store.all_sessions() {
        let Ok(rid) = db::get_entity_room(&s.player_id) else {
            continue;
        };
        if db::location_keys_equivalent(&rid, room_id) {
            let _ = s.try_send_bytes(msg.clone());
        }
    }
}

fn player_verb_strings() -> Vec<String> {
    crate::entity::Character::sockets()
        .into_iter()
        .map(|v| match v {
            Verb::Talk => "Talk",
            Verb::Borrow => "Borrow",
            Verb::Subdue => "Subdue",
            Verb::Slay => "Slay",
            Verb::Look => "Look",
            Verb::Trade => "Trade",
            _ => "Look",
        })
        .map(str::to_string)
        .collect()
}

/// 建構 `RoomViewMsg`（對齊 Go `sendRoomView` 資料欄位）。
pub fn build_room_view_msg(view: &RoomView, viewer_player_id: &str, cfg: &Server) -> RoomViewMsg {
    use crate::db::get_sockets_for_npc;

    let room_id = view.room.id.clone();
    let mut entities: Vec<ViewEntity> = Vec::new();
    for e in &view.entities {
        let kind = game::entity_kind_str(&e.kind);
        let mut display_name = e.id.clone();
        if e.kind == EntityKind::Npc && !e.display_title.is_empty() {
            display_name.clone_from(&e.display_title);
        }
        let mut actions: Vec<String> = Vec::new();
        if e.id != viewer_player_id {
            if e.kind == EntityKind::Npc {
                actions = get_sockets_for_npc(&e.id, &room_id);
            } else {
                actions = player_verb_strings();
            }
        }
        entities.push(ViewEntity {
            id: e.id.clone(),
            kind: kind.to_string(),
            display_char: e.display_char.clone(),
            display_name,
            actions,
        });
    }
    let exits: Vec<ExitView> = view
        .exits
        .iter()
        .map(|ex| ExitView {
            direction: ex.direction.clone(),
            to_room_id: ex.to_room_id.clone(),
            to_room_name: ex.to_room_name.clone(),
        })
        .collect();
    let objects: Vec<ViewObject> = view
        .objects
        .iter()
        .map(|o| ViewObject {
            id: o.id.clone(),
            name: o.name.clone(),
            actions: o.sockets.clone(),
        })
        .collect();
    let now = game::now_unix();
    let (sec_midnight, _, _, days) = game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    RoomViewMsg {
        msg_type: "view".into(),
        room_id: view.room.id.clone(),
        room_name: view.room.name.clone(),
        description: view.room.description.clone(),
        exits,
        entities,
        objects,
        server_unix: now,
        game_time_sec_since_midnight: sec_midnight,
        game_days_since_epoch: days,
    }
}

pub fn send_room_view_to_session(session: &Session, view: &RoomView, viewer_player_id: &str, cfg: &Server) {
    let msg = build_room_view_msg(view, viewer_player_id, cfg);
    let _ = session.try_send_bytes(must_json(&msg));
}

/// 對指定位置內在線玩家推送最新視野（`room_id` 為 `hex:…` 或可解析為六角之世界房間 id）。
pub fn refresh_room_views_for_room(store: &super::session::SessionStore, cfg: &Server, room_id: &str) {
    let Some((q, r)) = db::resolve_room_to_hex(room_id) else {
        return;
    };
    let now = game::now_unix();
    let (_, gh, _, _) = game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    for s in store.all_sessions() {
        let Ok(Some(ch)) = db::get_entity(&s.player_id) else {
            continue;
        };
        if ch.hex_q != Some(q) || ch.hex_r != Some(r) {
            continue;
        }
        let Ok(Some(view)) = game::get_hex_room_view(&s.player_id, q, r, gh) else {
            continue;
        };
        send_room_view_to_session(&s, &view, &s.player_id, cfg);
    }
}

/// 對所有在線玩家依其權威六角座標推送視野。
pub fn broadcast_room_views(store: &super::session::SessionStore, cfg: &Server) {
    let now = game::now_unix();
    let (_, gh, _, _) = game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    for s in store.all_sessions() {
        let Ok(Some(ch)) = db::get_entity(&s.player_id) else {
            continue;
        };
        let (Some(q), Some(r)) = (ch.hex_q, ch.hex_r) else {
            continue;
        };
        let Ok(Some(view)) = game::get_hex_room_view(&s.player_id, q, r, gh) else {
            continue;
        };
        send_room_view_to_session(&s, &view, &s.player_id, cfg);
    }
}

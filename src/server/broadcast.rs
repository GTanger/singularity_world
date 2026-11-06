//! 依房間推送敘事／視野（對齊既有 `server/broadcast`）。

use crate::config::Server;
use crate::db;
use crate::entity::{EntityKind, Verb};
use crate::game::{self, RoomView};

use super::protocol::{ExitView, NarrateMsg, NpcDialogueMsg, RoomViewMsg, ViewEntity, ViewObject};
use super::session::{Session, player_location_key};

fn must_json<T: serde::Serialize>(v: &T) -> Vec<u8> {
    serde_json::to_vec(v).unwrap_or_else(|_| br#"{"type":"error","message":"json"}"#.to_vec())
}

/// 同房在線玩家推送 `NarrateMsg`。
pub fn send_narrate_to_room(store: &super::session::SessionStore, room_id: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    let want = db::canonical_location_key(room_id);
    let msg = must_json(&NarrateMsg {
        msg_type: "narrate".into(),
        text: text.to_string(),
    });
    for s in store.all_sessions() {
        if player_location_key(&s.player_id) == want {
            let _ = s.try_send_bytes(msg.clone());
        }
    }
}

/// 對指定在線連線推送 `NarrateMsg`。
pub fn send_narrate_to_player(session: &Session, text: &str) {
    if text.is_empty() {
        return;
    }
    let msg = must_json(&NarrateMsg {
        msg_type: "narrate".into(),
        text: text.to_string(),
    });
    let _ = session.try_send_bytes(msg);
}

/// 對指定在線連線推送 `NpcDialogueMsg`（彈窗內對話更新）。
pub fn send_npc_dialogue_to_player(session: &Session, npc_a: &str, npc_b: &str, text: &str) {
    if text.is_empty() {
        return;
    }
    let msg = must_json(&NpcDialogueMsg {
        msg_type: "npc_dialogue".into(),
        npc_a: npc_a.to_string(),
        npc_b: npc_b.to_string(),
        text: text.to_string(),
    });
    let _ = session.try_send_bytes(msg);
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

/// 建構 `RoomViewMsg`（對齊既有 `sendRoomView` 資料欄位）。
pub fn build_room_view_msg(view: &RoomView, viewer_player_id: &str, cfg: &Server) -> RoomViewMsg {
    use crate::db::get_sockets_for_npc;

    let room_id = view.room.id.clone();
    let available_market_operators = crate::pg::pool()
        .and_then(|pool| {
            crate::pg::world_market::available_operator_ids_for_buyer(&pool, viewer_player_id).ok()
        })
        .unwrap_or_default();
    let mut entities: Vec<ViewEntity> = Vec::new();
    for e in &view.entities {
        let kind = game::entity_kind_str(&e.kind);
        let mut display_name = e.id.clone();
        if e.kind == EntityKind::Npc {
            if (e.id.starts_with("ambient_") || e.id.starts_with("shanhai_")) && !e.display_title.trim().is_empty() {
                display_name = e.display_title.trim().to_string();
            } else {
                display_name = "陌生人".to_string();
                let named = crate::db::get_acquaintance(viewer_player_id, &e.id)
                    .ok()
                    .flatten()
                    .map(|a| a.named)
                    .unwrap_or(false);
                if named {
                    let gender_str = match e.gender {
                        Some(crate::entity::Gender::F) => Some("F"),
                        Some(crate::entity::Gender::M) => Some("M"),
                        None => None,
                    };
                    display_name =
                        crate::store::resolve_npc_full_label(&e.id, &e.display_title, gender_str);
                }
            }
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
            interaction_priority: available_market_operators.contains(&e.id),
            current_activity: e.current_activity.clone(),
            behavior_text: String::new(),
        });
    }
    let exits: Vec<ExitView> = view
        .exits
        .iter()
        .map(|ex| ExitView {
            direction: ex.direction.clone(),
            to_room_id: ex.to_room_id.clone(),
            to_room_name: ex.to_room_name.clone(),
            route_id: ex.route_id.clone(),
            mode: ex.mode.clone(),
            travel_cost: ex.travel_cost,
        })
        .collect();
    let objects: Vec<ViewObject> = view
        .objects
        .iter()
        .map(|o| ViewObject {
            id: o.id.clone(),
            name: o.name.clone(),
            object_type: if o.id.starts_with("resource:") {
                "resource".to_string()
            } else {
                "object".to_string()
            },
            actions: o.sockets.clone(),
            tag: o.tag(),
            category_tag: o.category_tag().to_string(),
            attribute_tags: o.attribute_tags(),
            tags: o.tags(),
        })
        .collect();
    let now = game::now_unix();
    let (sec_midnight, _, _, days) =
        game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    let runtime_cell = crate::grid::parse_grid_room_id(&view.room.id)
        .and_then(crate::server::grid_manager::get_runtime_grid_cell);
    let terrain_str = runtime_cell
        .as_ref()
        .map(|c| format!("{:?}", c.terrain))
        .unwrap_or_default();
    let venue_obj = crate::grid::resolve_any_room_venue_object(&view.room.name, &view.room.zone);
    let venue_kind_str = venue_obj
        .as_ref()
        .map(|v| v.venue_kind.to_string())
        .unwrap_or_default();
    let is_indoor = runtime_cell
        .as_ref()
        .is_some_and(|c| matches!(c.terrain, crate::grid::Terrain::House))
        || (venue_obj.is_some()
            && !view.room.name.contains('街')
            && !view.room.name.contains('橋')
            && !view.room.name.contains('路')
            && !view.room.name.contains('道')
            && !view.room.name.contains('埠')
            && !view.room.name.contains("碼頭"));
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
        terrain: terrain_str,
        venue_kind: venue_kind_str,
        is_indoor,
    }
}

pub fn send_room_view_to_session(
    session: &Session,
    view: &RoomView,
    viewer_player_id: &str,
    cfg: &Server,
) {
    let msg = build_room_view_msg(view, viewer_player_id, cfg);
    let _ = session.try_send_bytes(must_json(&msg));
}

/// 對指定位置內在線玩家推送最新視野（可排除特定玩家）。
pub fn refresh_room_views_for_room_except(
    store: &super::session::SessionStore,
    cfg: &Server,
    room_id: &str,
    exclude_player_id: Option<&str>,
) {
    let now = game::now_unix();
    let (_, gh, _, _) = game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    let target_key = db::canonical_location_key(room_id);
    for s in store.all_sessions() {
        if let Some(ex_id) = exclude_player_id {
            if s.player_id == ex_id {
                continue;
            }
        }
        let player_coord = db::get_entity(&s.player_id)
            .ok()
            .flatten()
            .and_then(|e| e.grid_x.zip(e.grid_y));
        if let Some((gx, gy)) = player_coord {
            if crate::grid::grid_room_id_from_coord(gx, gy) == target_key
                || player_location_key(&s.player_id) == target_key
            {
                super::handler::push_grid_view_to_session(&s, cfg, gx, gy, gh);
                continue;
            }
        }
        if player_location_key(&s.player_id) != target_key {
            continue;
        }
        let Some(place_id) = db::get_entity_place_id(&s.player_id) else {
            continue;
        };
        let Ok(Some(view)) = game::get_place_room_view(&place_id, gh) else {
            continue;
        };
        send_room_view_to_session(&s, &view, &s.player_id, cfg);
    }
}

/// 對指定位置內在線玩家推送最新視野。
pub fn refresh_room_views_for_room(
    store: &super::session::SessionStore,
    cfg: &Server,
    room_id: &str,
) {
    refresh_room_views_for_room_except(store, cfg, room_id, None);
}

/// 對所有在線玩家依其正式場所推送視野。
pub fn broadcast_room_views(store: &super::session::SessionStore, cfg: &Server) {
    let now = game::now_unix();
    let (_, gh, _, _) = game::game_time_now(now, cfg.game_time_epoch_unix, cfg.game_time_scale);
    for s in store.all_sessions() {
        let player_coord = db::get_entity(&s.player_id)
            .ok()
            .flatten()
            .and_then(|e| e.grid_x.zip(e.grid_y));
        if let Some((gx, gy)) = player_coord {
            super::handler::push_grid_view_to_session(&s, cfg, gx, gy, gh);
            continue;
        }
        let Some(place_id) = db::get_entity_place_id(&s.player_id) else {
            continue;
        };
        let Ok(Some(view)) = game::get_place_room_view(&place_id, gh) else {
            continue;
        };
        send_room_view_to_session(&s, &view, &s.player_id, cfg);
    }
}

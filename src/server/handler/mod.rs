//! WebSocket 訊息分派（對齊既有 `server/handler` + `handler_auth` + `handler_move` 子集）。
//!
//! 按動作域拆分：auth（登入／創角）、inventory（背包／裝備／狀態）、movement（移動／探索）。

mod auth;
pub mod inventory;
pub(crate) mod movement;
pub mod query;

pub(crate) use inventory::{
    format_entity_item_with_inscription, get_player_tactical_loadout, get_player_word_casket,
    npc_autonomous_word_and_equip_step, push_refresh,
};
pub use inventory::entity_total_craft_dims;
#[cfg(test)]
pub(crate) use inventory::set_player_word_casket;
pub(crate) use movement::{emit_presence_reflex_on_arrival, send_place_view};
use std::sync::{Arc, RwLock};

use crate::config::Server;
use crate::game;

use super::action::handle_do_action;
use super::hub::Hub;
use super::protocol::{ClientMsg, PongMsg};
use super::session::SessionStore;
use uuid::Uuid;

/// 單一 WS 連線脈絡（對齊既有 `Client` + session 綁定）。
pub struct WsConnection {
    pub conn_id: Uuid,
    pub tx: tokio::sync::mpsc::Sender<Vec<u8>>,
    pub player_id: RwLock<Option<String>>,
    pub sessions: Arc<SessionStore>,
    pub hub: Arc<Hub>,
    pub cfg: Server,
}

pub fn push_grid_view_to_session(
    session: &super::session::Session,
    cfg: &Server,
    x: i32,
    y: i32,
    game_hour: i32,
) {
    let Some(tx) = session.outbound.clone() else {
        return;
    };
    let conn = WsConnection {
        conn_id: session.connection_id,
        tx,
        player_id: RwLock::new(Some(session.player_id.clone())),
        sessions: Arc::new(SessionStore::new()),
        hub: Arc::new(Hub::new(1)),
        cfg: cfg.clone(),
    };
    movement::send_grid_view(&conn, &session.player_id, x, y, game_hour);
}

impl WsConnection {
    pub fn send_error(&self, message: impl Into<String>) {
        let msg = serde_json::json!({
            "type": "error",
            "message": message.into(),
        });
        // try_send：channel 滿就丟（玩家重連會重抓 state），不 park blocking thread
        // 用 blocking_send 時若 client 連線 lagging，可能累積死鎖把 tokio blocking pool 吃光
        let _ = self.tx.try_send(msg.to_string().into_bytes());
    }

    pub(super) fn send_json<T: serde::Serialize>(&self, v: &T) {
        if let Ok(bytes) = serde_json::to_vec(v) {
            let _ = self.tx.try_send(bytes);
        }
    }
}

pub fn handle_message(conn: &WsConnection, raw: &[u8]) {
    let mut msg: ClientMsg = match serde_json::from_slice(raw) {
        Ok(m) => m,
        Err(_) => {
            conn.send_error("invalid json");
            return;
        }
    };
    match msg.msg_type.as_str() {
        "login" => auth::handle_login(conn, &msg),
        "create_character" => auth::handle_create_character(conn, &msg),
        "move" | "grid_move" | "move_to" => movement::handle_move(conn, &msg),
        "enter_room" => {
            if msg.direction.is_empty() {
                msg.direction = "進".to_string();
            }
            movement::handle_move(conn, &msg);
        }
        "ping" => {
            conn.send_json(&PongMsg {
                msg_type: "pong".into(),
            });
        }
        "get_entity_status" | "get_status" => query::handle_get_status(conn, &msg),
        "set_offline_directive" => query::handle_set_offline_directive(conn, &msg),
        "set_openrouter_key" => query::handle_set_openrouter_key(conn, &msg),
        "get_inventory" => inventory::handle_get_inventory(conn),
        "equip_item" => inventory::handle_equip_item(conn, &msg),
        "unequip_item" => inventory::handle_unequip_item(conn, &msg),
        "drop_item" | "discard_item" => inventory::handle_drop_item(conn, &msg),
        "use_item" | "inventory_action" => inventory::handle_use_item(conn, &msg),
        "sell_item" => inventory::handle_sell_item(conn, &msg),
        "craft_item" => inventory::handle_craft_item(conn, &msg),
        "mutate_item" => inventory::handle_mutate_item(conn, &msg),
        "print_topology_debug" => inventory::handle_print_topology_debug(conn),
        "do_action" | "action" | "object_action" => handle_do_action(conn, &msg),
        "explore" => movement::handle_explore_grid(conn),
        "scout" => movement::handle_scout(conn, &msg),
        "observe" => handle_observe(conn, &msg),
        "focus_entity" => {
            if !msg.entity_id.is_empty() {
                crate::store::pause_npc_for_interaction(&msg.entity_id, 180);
                if let Some(pid) = player_id(conn)
                    && let Ok(Some(player)) = crate::db::get_entity(&pid)
                    && let (Some(px), Some(py)) = (player.grid_x, player.grid_y)
                {
                    crate::store::try_grace_latch_npc_to_player_cell(&msg.entity_id, px, py, 3);
                }
            }
        }
        "unfocus_entity" => {
            if !msg.entity_id.is_empty() {
                crate::store::unpause_npc_for_interaction(&msg.entity_id);
            }
        }
        "get_tactical_state" => inventory::handle_get_tactical_state(conn),
        "update_tactics" => inventory::handle_update_tactics(conn, &msg),
        "mutate_terrain" => movement::handle_mutate_terrain(conn, &msg),
        "tactical_choice" => crate::server::action::combat::handle_tactical_choice(conn, &msg),
        other => {
            conn.send_error(format!("unknown type: {other}"));
        }
    }
}

/// 玩家觀察 NPC：更新 acquaintance.observed=true，重推 room_enter 讓前端刷新卡片。
/// WS 訊息格式：`{ "type": "observe", "npc_id": "<id>" }`
fn handle_observe(conn: &WsConnection, msg: &ClientMsg) {
    let Some(player_id) = player_id(conn) else {
        conn.send_error("login first");
        return;
    };
    if msg.entity_id.is_empty() {
        conn.send_error("npc_id required");
        return;
    }
    crate::store::pause_npc_for_interaction(&msg.entity_id, 180);
    if let Ok(Some(player)) = crate::db::get_entity(&player_id)
        && let (Some(px), Some(py)) = (player.grid_x, player.grid_y)
    {
        crate::store::try_grace_latch_npc_to_player_cell(&msg.entity_id, px, py, 3);
    }
    if let Err(e) = crate::db::set_observed(&player_id, &msg.entity_id) {
        tracing::warn!("[observe] set_observed 失敗：{e}");
    }
}

// ── 共用小工具（跨子模組）──

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

pub(super) fn parse_activated_nodes(raw: &str) -> Vec<String> {
    if raw.trim().is_empty() {
        return vec!["N000".into()];
    }
    let list: Vec<String> = serde_json::from_str(raw).unwrap_or_else(|_| vec!["N000".into()]);
    if list.is_empty() {
        vec!["N000".into()]
    } else {
        list
    }
}

pub(super) fn player_id(conn: &WsConnection) -> Option<String> {
    conn.player_id.read().ok().and_then(|g| g.clone())
}

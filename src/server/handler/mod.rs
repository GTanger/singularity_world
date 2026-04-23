//! WebSocket 訊息分派（對齊既有 `server/handler` + `handler_auth` + `handler_move` 子集）。
//!
//! 按動作域拆分：auth（登入／創角）、inventory（背包／裝備／狀態）、movement（移動／探索）。

mod auth;
mod inventory;
mod movement;

pub(crate) use inventory::push_refresh;

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
    let msg: ClientMsg = match serde_json::from_slice(raw) {
        Ok(m) => m,
        Err(_) => {
            conn.send_error("invalid json");
            return;
        }
    };
    if msg.msg_type != "ping" {
        tracing::info!("[handle_message] type={} pid={} conn={}",
            msg.msg_type, msg.player_id, conn.conn_id);
    }
    match msg.msg_type.as_str() {
        "login" => auth::handle_login(conn, &msg),
        "create_character" => auth::handle_create_character(conn, &msg),
        "move" => movement::handle_move(conn, &msg),
        "ping" => {
            conn.send_json(&PongMsg {
                msg_type: "pong".into(),
            });
        }
        "get_entity_status" => inventory::handle_get_entity_status(conn, &msg),
        "get_inventory" => inventory::handle_get_inventory(conn),
        "equip_item" => inventory::handle_equip_item(conn, &msg),
        "unequip_item" => inventory::handle_unequip_item(conn, &msg),
        "print_topology_debug" => inventory::handle_print_topology_debug(conn),
        "do_action" => handle_do_action(conn, &msg),
        "explore" => movement::handle_explore_grid(conn),
        other => {
            conn.send_error(format!("unknown type: {other}"));
        }
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

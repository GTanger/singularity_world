//! HTTP／WebSocket 伺服器層（逐步遷移自 Go `server/`）。

mod action;
mod broadcast;
mod conversation;
mod handler;
mod hub;
mod protocol;
mod run;
mod session;
mod simulation_loop;
pub mod hex_editor;
pub mod http_api;
pub mod room_editor;

pub use broadcast::{
    broadcast_room_views, build_room_view_msg, refresh_room_views_for_room, send_narrate_to_room,
};
pub use handler::{handle_message, WsConnection};
pub use hub::{Hub, SEND_BUFFER_SIZE};
pub use protocol::{
    ActionResultMsg, BlockedMsg, ClientMsg, EntityStatusMsg, ErrorMsg, ExitView, InventoryItemView,
    InventoryMsg, MeMsg, MovedMsg, NarrateMsg, PongMsg, RoomViewMsg, TopologyDebugAckMsg,
    ViewEntity, ViewObject,
};
pub use run::{run, AppState};
pub use session::{sanitize_talk_snippet, Session, SessionStore};

//! HTTP／WebSocket 伺服器層（Axum `server/`）。

mod action;
mod broadcast;
mod conversation;
pub mod grid_manager;
mod handler;
pub mod http_api;
mod hub;
pub(crate) mod m3_029;
mod protocol;
pub mod room_editor;
mod run;
mod session;
mod simulation_loop;

pub use broadcast::{
    broadcast_room_views, build_room_view_msg, refresh_room_views_for_room, send_narrate_to_room,
};
pub use handler::{WsConnection, handle_message};
pub use hub::{Hub, SEND_BUFFER_SIZE};
pub use protocol::{
    ActionResultMsg, BlockedMsg, ClientMsg, EntityStatusMsg, ErrorMsg, ExitView, InventoryItemView,
    InventoryMsg, MeMsg, MovedMsg, NarrateMsg, PongMsg, RoomViewMsg, TopologyDebugAckMsg,
    ViewEntity, ViewObject,
};
pub use run::{AppState, run};
pub use session::{Session, SessionStore, sanitize_talk_snippet};

//! HTTP／WebSocket 伺服器層（Axum `server/`）。

pub(crate) mod action;
mod broadcast;
pub mod conversation;
pub mod grid_manager;
pub(crate) mod handler;
pub mod http_api;
mod hub;
pub(crate) mod m3_029;
mod protocol;
pub mod room_editor;
mod run;
pub mod session;
mod simulation_loop;

pub use broadcast::{
    broadcast_room_views, build_room_view_msg, refresh_room_views_for_room, send_narrate_to_player,
    send_narrate_to_room, send_npc_dialogue_to_player,
};
pub use handler::{WsConnection, handle_message};
pub use hub::{Hub, SEND_BUFFER_SIZE};
pub use protocol::{
    ActionResultMsg, BlockedMsg, ClientMsg, EntityStatusMsg, ErrorMsg, ExitView, InventoryItemView,
    InventoryMsg, MeMsg, MovedMsg, NarrateMsg, NpcDialogueMsg, PongMsg, RoomViewMsg,
    TopologyDebugAckMsg, ViewEntity, ViewObject,
};
pub use run::{AppState, run};
pub use session::{Session, SessionStore, sanitize_talk_snippet};

//! HTTP API 端點（按域拆分：admin / player / rooms）。
//!
//! 路由在 `server/run.rs`，此處僅匯出 handler 與共用型別。

mod admin;
mod player;
mod rooms;

pub use admin::{design_constants, wipe_entities, AdminQuery};
pub use player::{player_room, topology};
pub use rooms::{
    add_exit, create_room, delete_room, get_room_admin, list_rooms, remove_exit, rename_room,
    rooms_data, update_room,
};

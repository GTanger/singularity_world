//! HTTP API 端點（按域拆分：admin / player / rooms / hex）。
//!
//! 路由在 `server/run.rs`，此處僅匯出 handler 與共用型別。

mod admin;
mod hex;
mod player;
mod rooms;

pub use admin::{design_constants, wipe_entities, AdminQuery};
pub use hex::{
    hex_explore, hex_move, hex_my_revealed, hex_player_reveal, hex_scout, hex_view,
};
pub use player::{player_room, topology};
pub use rooms::{
    add_exit, create_room, delete_room, get_room_admin, list_rooms, remove_exit, rename_room,
    rooms_data, update_room,
};

//! HTTP API 端點（按域拆分：admin / player / rooms / eco）。
//!
//! 路由在 `server/run.rs`，此處僅匯出 handler 與共用型別。

mod admin;
mod eco;
mod player;
mod rooms;
mod substrate_atlas;

pub use admin::{AdminQuery, design_constants, wipe_entities};
pub use eco::{eco_entities, eco_map, eco_npc_thoughts, eco_pulse, handle_eco_entities, handle_eco_npc_thoughts};
pub use player::{climate, game_time, player_room, topology};
pub use rooms::{
    add_exit, create_room, delete_room, get_room_admin, list_rooms, remove_exit, rename_room,
    rooms_data, update_room,
};
pub use substrate_atlas::{substrate_basemap, substrate_cell, substrate_meta, substrate_window};

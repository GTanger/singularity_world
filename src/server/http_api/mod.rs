//! HTTP API 端點（按域拆分：admin / player / rooms / eco）。
//!
//! 路由在 `server/run.rs`，此處僅匯出 handler 與共用型別。

mod admin;
mod chronicle_api;
mod eco;
mod island_api;
mod player;
mod rim_api;
mod rim_auth;
mod rim_portrait_api;
mod rim_talk_api;
mod rooms;
mod substrate_atlas;

pub use admin::{AdminQuery, design_constants, wipe_entities};
pub use chronicle_api::{
    chronicle_actor_get, chronicle_article_get, chronicle_export_md_post, chronicle_index_get,
    chronicle_page_get, world_atlas_node_get, world_atlas_page_get, world_atlas_snapshot_get,
};
pub use eco::{
    eco_entities, eco_map, eco_npc_thoughts, eco_pulse, handle_eco_entities,
    handle_eco_npc_thoughts,
};
pub use island_api::{
    island_act_post, island_leave_post, island_map_get, island_move_post, island_pulse_get,
    island_qa_cleanup_post, island_scroll_get, island_setup_post, island_state_get,
    island_talk_post,
};
pub use player::{climate, game_time, player_room, topology};
pub use rim_api::{rim_cmd, rim_state, spawn_rim_tick_loop};
pub use rim_auth::{rim_auth_logout, rim_auth_me, rim_auth_post};
pub use rim_portrait_api::{rim_byok_get, rim_byok_post, rim_portrait_image, rim_portrait_regen};
pub use rim_talk_api::{rim_talk_post, rim_trade_post};
pub use rooms::{
    add_exit, create_room, delete_room, get_room_admin, list_rooms, remove_exit, rename_room,
    rooms_data, update_room,
};
pub use substrate_atlas::{
    substrate_basemap, substrate_cell, substrate_meta, substrate_social_summary, substrate_window,
};

// game 模組 — 遊戲時間、主迴圈 tick、視野與觀測（對齊既有 game/）。

mod chunk_view;
mod gametime;
mod movement_tick;
mod observe;
pub mod object_ecology;
pub mod peninsula_catalog;
pub mod peninsula_scripts;
pub mod player_scene;
mod room;
mod tick;
mod view_sim;
mod zone;

pub use crate::model::RoomObject;
pub use chunk_view::{ChunkView, get_chunk_and_entities_in_view};
pub use gametime::{TimePeriod, game_time_now, time_period, time_period_label};
pub use movement_tick::{MovedResult, advance_movement, entity_at};
pub use observe::{Observed, Observer, collapse, now_unix, observe_room};
pub use player_scene::{
    final_map_crowd_label_for_room, final_map_entry_log_for_room,
    final_map_observation_template_for_room, final_map_public_role_for_room,
    final_map_reentry_log_for_room, is_final_map_public_role, is_wenyao_residential_or_study,
    micro_interaction_lines_for_room, player_layer_scene_objects_for_room,
    public_person_label_for_room,
};
pub use room::{
    RoomView, ambient_creature_loot, ensure_entity_in_room, entity_kind_str,
    get_grid_cells_around, get_grid_room_view, get_place_room_view, get_room_view,
    hunting_zone_apex_omen, move_by_exit, move_by_grid_direction, move_by_place_route,
    player_shanhai_talisman_bonus, shanhai_item_use_effect,
};
pub use tick::spawn_loop;
pub use view_sim::{Pos, in_view_entity_ids, run_view_simulation, simulate_one_tick};
pub use zone::{CHUNK_SIZE, VIEW_RADIUS, chunk_bounds, chunk_index, in_chunk, in_view};

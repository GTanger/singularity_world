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
pub use gametime::{
    TimePeriod, current_game_clock_full, current_game_hour, current_game_hour_and_min,
    current_game_time_prompt_context, format_elapsed_duration_zh, format_game_calendar_stamp,
    format_game_time_prompt_context, format_kaipu_date_label, format_memory_temporal_anchor,
    format_schedule_window_sense, format_short_relative_time, game_clock_at_unix,
    game_season_label, game_time_cfg, game_time_now, kaipu_calendar_parts, time_period,
    time_period_label,
};
pub use movement_tick::{MovedResult, advance_movement, entity_at};
pub use observe::{Observed, Observer, collapse, now_unix, observe_room};
pub use player_scene::{
    final_map_crowd_label_for_room, final_map_entry_log_for_room,
    final_map_observation_template_for_room, final_map_public_role_for_room,
    final_map_reentry_log_for_room, is_final_map_public_role, is_wenyao_residential_or_study,
    micro_interaction_lines_for_room, player_layer_scene_objects_for_cell,
    player_layer_scene_objects_for_room, public_person_label_for_room,
};
pub use room::{
    CANONICAL_HUNTING_ZONES, LIVE_ZONE_TROPHIC_WEB, RecentHuntKillSite, RoomView,
    ShanhaiHabitatSlot, ZoneTidePhase, ZoneTrophicState, ZoneWeatherPhase, ambient_creature_loot,
    current_zone_weather_and_tide, dynamic_hunting_zone_ecology_omen, ensure_ambient_npc_at_grid,
    ensure_entity_in_room, entity_kind_str, get_grid_cells_around, get_grid_room_view,
    get_place_room_view, get_room_view, get_zone_trophic_state, harvest_hunting_zone_flora,
    harvest_zone_detritus_fungi, harvest_zone_natural_resource, hunting_zone_apex_omen,
    is_alpine_hunting_zone, is_apex_beast_tracked_at, is_coastal_hunting_zone,
    is_inside_settlement_urban_core, is_zone_in_fallow_stewardship,
    is_zone_predator_spillover_active, live_zone_shanhai_slots, mark_apex_beast_tracked_at,
    move_by_exit, move_by_grid_direction, move_by_place_route, nearest_geographic_zones,
    nearest_hunting_zone_idx, peek_recent_hunt_kill_site, pick_island_trail_waypoint,
    pick_zone_artisan_waypoint, pick_zone_artisan_waypoint_with_zone,
    player_shanhai_talisman_bonus, record_shanhai_slot_hunted, record_wilderness_kill_at_coord,
    resolve_shanhai_or_ambient_entity_id, resolve_zone_idx_from_name_or_settlement,
    shanhai_item_use_effect, step_all_zones_trophic_web_if_needed, step_zone_trophic_web_once,
    take_recent_hunt_kill_site,
    zone_ambient_word_source,
};
pub use tick::spawn_loop;
pub use view_sim::{Pos, in_view_entity_ids, run_view_simulation, simulate_one_tick};
pub use zone::{CHUNK_SIZE, VIEW_RADIUS, chunk_bounds, chunk_index, in_chunk, in_view};

/// 查詢指定聚落或場所目前的天候與海潮走勢（以當前 UNIX 秒數為基準）
pub fn zone_weather_and_tide(settlement_or_room: &str, now_unix: i64) -> (ZoneWeatherPhase, ZoneTidePhase) {
    let z_idx = resolve_zone_idx_from_name_or_settlement(settlement_or_room);
    current_zone_weather_and_tide(z_idx, now_unix / 180)
}

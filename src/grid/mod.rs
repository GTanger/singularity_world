// 正方格地圖模組。
pub mod cell;
pub mod coord;
pub mod r#gen;
#[allow(clippy::module_inception)]
pub mod grid;
pub mod reveal;
pub mod shalu_zhulin;
pub mod substrate;
pub mod village_template;
pub mod world_skeleton;
pub mod zhulin;

pub use cell::{Freshness, GridObject, GridObjectKind, Terrain};
pub use coord::{
    Direction, SquareCoord, grid_room_id_from_coord, grid_room_id_from_coord_z,
    parse_grid_room_id, parse_grid_room_id_z,
};
pub use r#gen::generate_white_tower_grid;
pub use grid::{
    ExplicitExit, GridCell, RoomVenueObject, SquareGrid, canonical_room_tag, derive_room_tags,
    is_private_residence_room_name, is_street_or_gate_room_name, resolve_any_room_venue_object,
    resolve_market_or_specialty_room_object, resolve_room_ambient_visitor_activity,
    resolve_room_venue_object,
};
pub use reveal::{reveal_grid_disk, terrain_name_zh};
pub use shalu_zhulin::{
    SHALU_ZHULIN_WORLD_SEED, generate_shalu_zhulin_grid, resolve_shalu_zhulin_object,
};
pub use village_template::ensure_seed_village;
pub use world_skeleton::ensure_builtin_world_skeleton;

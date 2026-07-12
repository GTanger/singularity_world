// 正方格地圖模組。
pub mod cell;
pub mod coord;
#[allow(clippy::module_inception)]
pub mod grid;
pub mod reveal;
pub mod village_template;
pub mod world_skeleton;

pub use cell::{Freshness, GridObject, GridObjectKind, Terrain};
pub use coord::{Direction, SquareCoord, grid_room_id_from_coord, parse_grid_room_id};
pub use grid::{ExplicitExit, GridCell, SquareGrid};
pub use reveal::{reveal_grid_disk, terrain_name_zh};
pub use village_template::ensure_seed_village;
pub use world_skeleton::ensure_builtin_world_skeleton;

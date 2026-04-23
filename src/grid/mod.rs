// 正方格地圖模組。
pub mod cell;
pub mod coord;
#[allow(clippy::module_inception)]
pub mod grid;
pub mod reveal;

pub use cell::{GridObject, GridObjectKind, Terrain};
pub use coord::{Direction, SquareCoord, grid_room_id_from_coord, parse_grid_room_id};
pub use grid::{GridCell, SquareGrid};
pub use reveal::{reveal_grid_disk, terrain_name_zh};

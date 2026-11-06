//! 伺服器端地圖格子生命週期管理與運行時快取。
//! 整合 PostgreSQL 持久化與記憶體內運行時快取，確保單例與跨線程安全。

use std::sync::{OnceLock, RwLock};

use crate::db::{load_square_grid, save_square_grid_to_pg};
use crate::grid::{GridCell, GridObjectKind, SquareCoord, SquareGrid};

include!("grid_manager/state.rs");
include!("grid_manager/expeditions.rs");
include!("grid_manager/schedule_part1.rs");
include!("grid_manager/schedule_part2.rs");
include!("grid_manager/sync.rs");
include!("grid_manager/simulation.rs");

pub(crate) mod schedule {
    pub(crate) use super::iter_resident_schedules;
}

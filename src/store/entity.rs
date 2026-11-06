//! store 實體 — entities + entity_rooms 的讀寫、座標綁定、轉帳。
//! 寫入路徑走 writer queue（PG 權威）+ 必要時 in-memory mirror；entity_rooms 不再寫 JSON。

use super::{Entity, Store};

include!("entity/part_01_state.rs");
include!("entity/part_02_household.rs");
include!("entity/part_03_navigation.rs");
include!("entity/part_04_store_crud.rs");
include!("entity/part_05_store_simulation.rs");
include!("entity/part_06_store_movement.rs");

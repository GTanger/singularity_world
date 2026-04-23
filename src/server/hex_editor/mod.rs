//! 六角格地圖編輯器 API
//!
//! 以 `HexGrid` 為核心，提供 cell CRUD、barrier、儲存/載入。
//! 持久化：**PostgreSQL `hex_world` 為主**，`data/hex/grid.json` 為備份。
//! 前端透過這組 API 操作地圖，不再經由 room-editor。
//!
//! 子模組：`state`（全域 + 生命週期）/ `types`（請求回應）/ `handlers`（路由處理）。

mod handlers;
mod state;
mod types;

pub use state::{
    ensure_player_spawn_grassland_coord, ensure_world_cell_at, get_runtime_grid, init,
    mark_cell_explored,
};

pub use handlers::{
    cell_delete, cell_put, cells_put, grid_get, neighbors_get, path_get, portal_post, reload,
    reveal_post, reveal_region_post, save, transport_edge_post, wall_put, world_seed_put,
};

//! 六角格地圖系統
//!
//! 取代舊的 Room + Exit 模型。核心概念：
//! - **HexCoord**：axial 座標 (q, r)，平頂佈局
//! - **HexDir**：六方向，E/NE/NW/W/SW/SE
//! - **Terrain**：地形，決定通行性與移動成本
//! - **HexCell**：格子單元（座標 + 地形 + 名稱 + 物件）
//! - **HexGrid**：地圖容器（`world_seed` + 格子 + 障壁 + 傳送門 + 交通邊 + 尋路；`LinkLayer` 區分官方／探索圖）
//! - **reveal**：格網首次揭露（黑格→彩格）之決定性生成（`generate_wild_cell`／`reveal_hex_disk`）
//!
//! 相鄰格之間預設六方皆通。阻斷用 barrier（牆、懸崖），
//! 非相鄰連接用 portal（傳送門、隧道）。

mod cell;
mod contract_pins;
mod coord;
mod grid;
mod reveal;

pub use cell::{HexCell, Terrain};
pub use contract_pins::{
    api_contract_pins, allowed_terrain_for_pin, is_player_spawn_pin, is_undeletable_contract,
    ContractPin, PLAYER_SPAWN,
};
pub use coord::{hex_room_id_from_coord, parse_hex_room_id, CellIdParseError, HexCoord, HexDir};
pub use grid::{
    HexGrid, LinkLayer, Portal, TransportEdge, TransportEndpoint, TransportLinkClass, TransportMode,
};
pub use reveal::{generate_wild_cell, mix_coord_seed, reveal_hex_disk};

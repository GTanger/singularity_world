//! 遊戲釘死之 Hex 格契約（彩格）— **單一來源**，須與 `hex_editor::ensure_player_spawn_grassland_coord` 等邏輯一致。
//! 地圖編輯器與遊戲本體共用 `GET /api/hex/grid`；此模組列出**不得被編輯器刪除／破壞契約**之座標與角色。

use super::coord::HexCoord;
use super::Terrain;

/// 玩家 Hex 世界出生點（與 `ensure_player_spawn_grassland_coord` 相同座標）。
pub const PLAYER_SPAWN: HexCoord = HexCoord { q: 0, r: 0 };

/// 是否為遊戲釘死、不可刪除之格（日後可擴充列表）。
pub fn is_undeletable_contract(coord: HexCoord) -> bool {
    coord == PLAYER_SPAWN
}

/// 是否為出生格契約（須維持草原 + `player_spawn` 標籤）。
pub fn is_player_spawn_pin(coord: HexCoord) -> bool {
    coord == PLAYER_SPAWN
}

/// 釘死格若被 PUT，允許之地形（其餘應拒絕）。
pub fn allowed_terrain_for_pin(coord: HexCoord, t: Terrain) -> bool {
    if is_player_spawn_pin(coord) {
        return t == Terrain::Grassland;
    }
    true
}

/// API `contract_pins` 陣列（供地圖編輯器與工具對齊遊戲釘死彩格）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContractPin {
    pub q: i32,
    pub r: i32,
    /// 與遊戲邏輯對應之角色 id，例如 `player_spawn`
    pub role: &'static str,
}

pub fn api_contract_pins() -> Vec<ContractPin> {
    vec![ContractPin {
        q: PLAYER_SPAWN.q,
        r: PLAYER_SPAWN.r,
        role: "player_spawn",
    }]
}

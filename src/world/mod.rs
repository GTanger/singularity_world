// world 模組 — 地圖格點、地形、區塊載入與移動碰撞，對齊既有 world/。

mod chunk;
mod map;
mod movement;
mod terrain_display;

pub use chunk::{load_chunk, terrain_from_rune};
pub use map::{
    terrain_blocking, Grid, TERRAIN_ALLEY, TERRAIN_DOOR, TERRAIN_DOOR_CLOSED, TERRAIN_FARM,
    TERRAIN_FLOOR, TERRAIN_FOG, TERRAIN_GRASS, TERRAIN_ICE, TERRAIN_LAVA, TERRAIN_MOUNTAIN,
    TERRAIN_RIVER, TERRAIN_ROAD, TERRAIN_STONE, TERRAIN_SWAMP, TERRAIN_TREE, TERRAIN_VALLEY,
    TERRAIN_WALL, TERRAIN_WASTELAND, TERRAIN_WATER,
};
pub use movement::can_move_to;
pub use terrain_display::{display_terrain, display_terrain_rng, terrain_meta_by_type, TerrainMeta};

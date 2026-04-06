// 地圖格點與地形，對齊既有 world/map。

/// 牆
pub const TERRAIN_WALL: &str = "牆";
/// 門
pub const TERRAIN_DOOR: &str = "門";
/// 關
pub const TERRAIN_DOOR_CLOSED: &str = "關";
/// 道
pub const TERRAIN_ROAD: &str = "道";
/// 巷
pub const TERRAIN_ALLEY: &str = "巷";
/// 草
pub const TERRAIN_GRASS: &str = "草";
/// 木
pub const TERRAIN_TREE: &str = "木";
/// 山
pub const TERRAIN_MOUNTAIN: &str = "山";
/// 石
pub const TERRAIN_STONE: &str = "石";
/// 沼
pub const TERRAIN_SWAMP: &str = "沼";
/// 川
pub const TERRAIN_RIVER: &str = "川";
/// 水
pub const TERRAIN_WATER: &str = "水";
/// 荒
pub const TERRAIN_WASTELAND: &str = "荒";
/// 火
pub const TERRAIN_LAVA: &str = "火";
/// 冰
pub const TERRAIN_ICE: &str = "冰";
/// 田
pub const TERRAIN_FARM: &str = "田";
/// 谷
pub const TERRAIN_VALLEY: &str = "谷";
/// 霧
pub const TERRAIN_FOG: &str = "霧";
/// 地（屋內地板）
pub const TERRAIN_FLOOR: &str = "地";

/// 該地形字是否阻擋移動（牆、關、川、水）。
pub fn terrain_blocking(t: &str) -> bool {
    t == TERRAIN_WALL || t == TERRAIN_DOOR_CLOSED || t == TERRAIN_RIVER || t == TERRAIN_WATER
}

/// 大地圖格點：每格一個地形一字（UTF-8 字串）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<String>,
}

impl Grid {
    /// 建立 w×h 地圖，預設為草。
    pub fn new(w: usize, h: usize) -> Self {
        let mut cells = Vec::with_capacity(w * h);
        cells.resize(w * h, TERRAIN_GRASS.to_string());
        Self {
            width: w,
            height: h,
            cells,
        }
    }

    /// 越界視為牆。
    pub fn at(&self, x: i32, y: i32) -> &str {
        if x < 0 || y < 0 {
            return TERRAIN_WALL;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.width || y >= self.height {
            return TERRAIN_WALL;
        }
        &self.cells[y * self.width + x]
    }

    pub fn set_at(&mut self, x: usize, y: usize, terrain: String) {
        if x < self.width && y < self.height {
            self.cells[y * self.width + x] = terrain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocking_matches_legacy() {
        assert!(terrain_blocking(TERRAIN_WALL));
        assert!(terrain_blocking(TERRAIN_DOOR_CLOSED));
        assert!(terrain_blocking(TERRAIN_RIVER));
        assert!(terrain_blocking(TERRAIN_WATER));
        assert!(!terrain_blocking(TERRAIN_DOOR));
        assert!(!terrain_blocking(TERRAIN_ROAD));
    }

    #[test]
    fn at_out_of_bounds_is_wall() {
        let g = Grid::new(3, 3);
        assert_eq!(g.at(-1, 0), TERRAIN_WALL);
        assert_eq!(g.at(3, 0), TERRAIN_WALL);
    }
}

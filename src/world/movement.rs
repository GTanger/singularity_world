// 移動與碰撞，對齊 Go world/movement.go。

use super::map::{terrain_blocking, Grid};

/// 格 (x,y) 是否可進入：不越界且地形非阻擋。
pub fn can_move_to(g: &Grid, x: i32, y: i32) -> bool {
    !terrain_blocking(g.at(x, y))
}

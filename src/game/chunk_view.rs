// 當前區塊地形與視野內實體，對齊既有 game/chunk_view。

use std::path::Path;

use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::db;
use crate::entity::Character;
use crate::world::{
    self, display_terrain, display_terrain_rng, load_chunk, terrain_blocking, Grid,
};

use super::zone::{chunk_bounds, chunk_index, in_chunk, in_view, CHUNK_SIZE, VIEW_RADIUS};

/// 當前區塊地形 ＋ 視野內實體（對齊既有 `ChunkView`）。
#[derive(Debug, Clone)]
pub struct ChunkView {
    pub cx: i32,
    pub cy: i32,
    pub grid: Grid,
    pub entities: Vec<Character>,
}

/// 依觀測者世界座標載入區塊並回傳視野內實體。
pub fn get_chunk_and_entities_in_view(
    wx: i32,
    wy: i32,
    maps_path: &Path,
) -> anyhow::Result<ChunkView> {
    let (cx, cy) = chunk_index(wx, wy);
    let grid = load_chunk(cx, cy, maps_path)?;
    let (x0, y0, x1, y1) = chunk_bounds(cx, cy);
    let x_min = x0 - VIEW_RADIUS;
    let x_max = x1 - 1 + VIEW_RADIUS;
    let y_min = y0 - VIEW_RADIUS;
    let y_max = y1 - 1 + VIEW_RADIUS;
    let list = db::get_entities_in_box(x_min, x_max, y_min, y_max, "")?;
    let entities: Vec<Character> = list
        .into_iter()
        .filter(|e| in_view(wx, wy, e.x, e.y))
        .collect();
    Ok(ChunkView { cx, cy, grid, entities })
}

impl ChunkView {
    /// 扁平顯示字列，長度 151×151（對齊既有 `ChunkRows`）。
    #[must_use]
    pub fn chunk_rows(&self) -> Vec<String> {
        self.chunk_rows_with_colors().0
    }

    /// 每格依種子亂數取顯示字與色（對齊既有 `ChunkRowsWithColors`）。
    #[must_use]
    pub fn chunk_rows_with_colors(&self) -> (Vec<String>, Vec<String>) {
        let size = (CHUNK_SIZE as usize).saturating_mul(CHUNK_SIZE as usize);
        let mut rows = Vec::with_capacity(size);
        let mut colors = Vec::with_capacity(size);
        let h = self.grid.height.min(CHUNK_SIZE as usize);
        let w = self.grid.width.min(CHUNK_SIZE as usize);
        for y in 0..h {
            for x in 0..w {
                let t = self.grid.at(x as i32, y as i32);
                let seed = i64::from(self.cx) * 1_000_000
                    + i64::from(self.cy) * 1_000
                    + i64::try_from(y).unwrap_or(0) * i64::from(CHUNK_SIZE)
                    + i64::try_from(x).unwrap_or(0);
                let mut rng = StdRng::seed_from_u64(seed as u64);
                let (ch, col) = display_terrain_rng(t, &mut rng);
                rows.push(ch);
                colors.push(col);
            }
        }
        while rows.len() < size {
            let (ch, col) = display_terrain(world::TERRAIN_GRASS);
            rows.push(ch);
            colors.push(col);
        }
        (rows, colors)
    }

    /// 世界座標是否在當前區塊內且可通行（對齊既有 `CanMoveToWorld`）。
    #[must_use]
    pub fn can_move_to_world(&self, wx: i32, wy: i32) -> bool {
        if !in_chunk(wx, wy, self.cx, self.cy) {
            return false;
        }
        let (x0, y0, _, _) = chunk_bounds(self.cx, self.cy);
        let lx = wx - x0;
        let ly = wy - y0;
        !terrain_blocking(self.grid.at(lx, ly))
    }
}

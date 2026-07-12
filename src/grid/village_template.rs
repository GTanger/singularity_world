//! Minimal entry cell for an uncommitted world map.
//!
//! This intentionally does not seed a village, wilderness ring, or procedural
//! surroundings. The map content has to come from an explicit design/import
//! pass, not from an automatic placeholder.

use super::cell::Terrain;
use super::coord::SquareCoord;
use super::grid::{GridCell, SquareGrid};

/// If the grid is empty, seed only the neutral entry cell. Returns inserted
/// cell count.
pub fn ensure_seed_village(grid: &mut SquareGrid) -> usize {
    if !grid.is_empty() {
        return 0;
    }

    let mut cell = GridCell::new(SquareCoord::ORIGIN, Terrain::Urban, "起點");
    cell.zone = "未定義".to_string();
    cell.description =
        "你在一處可落腳的地方站定，近處有人聲與腳步聲，幾條去處從眼前分開。".to_string();
    cell.explored = true;
    grid.insert(cell);
    1
}

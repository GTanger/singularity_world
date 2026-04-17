//! 正方格地圖執行期狀態管理器。
//!
//! 以 `OnceLock<GridState>` + `RwLock<SquareGrid>` 為核心：
//! - 啟動時自 PostgreSQL 載入，若為空則自動揭露原點半徑 5（11×11 = 121 格）後寫回。
//! - 提供唯讀快照（`get_runtime_square_grid`）與可變操作（`with_square_grid_mut`）。
//! - 對稱 `hex_editor.rs` 的 `HEX_STATE` 設計，使兩地圖管線結構一致。

use std::sync::{OnceLock, RwLock};

use crate::db::{load_square_grid, save_square_grid_to_pg};
use crate::grid::{reveal_grid_disk, SquareCoord, SquareGrid};

// ─── 全域狀態 ────────────────────────────────────────────────────────

static GRID_STATE: OnceLock<GridState> = OnceLock::new();

struct GridState {
    grid: RwLock<SquareGrid>,
}

// ─── 初始化 ──────────────────────────────────────────────────────────

/// 啟動時呼叫：優先自 PostgreSQL `grid_cells` 載入；
/// 若表格為空則自動從原點揭露半徑 5（(2×5+1)² = 121 格）並寫回 PG。
pub fn init() {
    let mut grid = match load_square_grid() {
        Some(g) => g,
        None => SquareGrid::new(),
    };

    // 格數不足時自動揭露起始區域
    if grid.len() <= 1 {
        let revealed = reveal_grid_disk(&mut grid, SquareCoord::ORIGIN, 5);
        tracing::info!("grid_manager: 格數不足，自動揭露起始區域（+{revealed} 格）");
        if let Err(e) = save_square_grid_to_pg(&grid) {
            tracing::warn!("grid_manager: 初始揭露寫入 PG 失敗：{e}");
        }
    }

    let count = grid.len();
    let _ = GRID_STATE.set(GridState {
        grid: RwLock::new(grid),
    });
    tracing::info!("grid_manager: 載入 {count} 格（主資料：PostgreSQL grid_cells）");
}

fn grid_state() -> &'static GridState {
    GRID_STATE.get().expect("grid_manager::init 未呼叫")
}

// ─── 公開介面 ─────────────────────────────────────────────────────────

/// 取得執行期正方格地圖的唯讀快照（供視野查詢、地圖渲染用）。
pub fn get_runtime_square_grid() -> Option<SquareGrid> {
    GRID_STATE
        .get()
        .and_then(|s| s.grid.read().ok().map(|g| (*g).clone()))
}

/// 以可變引用操作正方格地圖。僅在格數變動時寫回 PG。
///
/// 用於揭露新格、資源扣量、NPC 路徑標記等需要改動地圖的場景。
pub fn with_square_grid_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut SquareGrid) -> R,
{
    let st = grid_state();
    let mut grid = st.grid.write().ok()?;
    let before = grid.len();
    let result = f(&mut grid);
    if grid.len() != before || grid.dirty() {
        if let Err(e) = save_square_grid_to_pg(&grid) {
            tracing::warn!("with_square_grid_mut: PG 寫入失敗：{e}");
        }
        grid.clear_dirty();
    }
    Some(result)
}

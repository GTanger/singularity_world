//! 全域 HexState、初始化、核心生命週期（ensure / mark / spawn / save）。

use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;

use crate::db::{load_hex_grid, save_hex_grid_to_pg};
use crate::hex::{generate_wild_cell, HexCell, HexCoord, HexGrid, Terrain};
use crate::server::http_api::AdminQuery;

static HEX_STATE: OnceLock<HexState> = OnceLock::new();

pub(super) struct HexState {
    pub(super) grid: RwLock<HexGrid>,
    pub(super) path: PathBuf,
}

/// 啟動時呼叫：優先自 PostgreSQL `hex_world` 載入；無列則自 `grid.json` 種子灌入 PG。
pub fn init(data_path: &str) {
    let path = PathBuf::from(data_path);
    let grid = if let Some(g) = load_hex_grid() {
        g
    } else if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(json) => {
                let g: HexGrid = serde_json::from_str(&json).unwrap_or_default();
                if let Err(e) = save_hex_grid_to_pg(&g) {
                    tracing::warn!("hex_world: 種子寫入 PG 失敗：{e}");
                }
                g
            }
            Err(_) => HexGrid::new(),
        }
    } else {
        HexGrid::new()
    };
    let count = grid.len();
    let _ = HEX_STATE.set(HexState {
        grid: RwLock::new(grid),
        path,
    });
    tracing::info!("hex_editor: 載入 {count} 格（主資料：PostgreSQL hex_world）");
}

pub(super) fn hex_state() -> &'static HexState {
    HEX_STATE.get().expect("hex_editor::init 未呼叫")
}

/// 取得執行期 hex grid 的唯讀快照（供 game 模組視野查詢用）。
pub fn get_runtime_grid() -> Option<HexGrid> {
    HEX_STATE.get().and_then(|s| s.grid.read().ok().map(|g| g.clone()))
}

pub(super) fn save_to_disk() -> Result<(), String> {
    let st = hex_state();
    let grid = st.grid.read().map_err(|e| e.to_string())?;
    if let Err(e) = save_hex_grid_to_pg(&grid) {
        tracing::warn!("hex_world: PG 寫入失敗（仍寫入 JSON 備份）：{e}");
    }
    if let Some(dir) = st.path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&*grid).map_err(|e| e.to_string())?;
    std::fs::write(&st.path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// 確保座標在世界格網中已有契約；若無則決定性生成並落盤。
pub fn ensure_world_cell_at(coord: HexCoord) -> Result<(HexCell, bool), String> {
    let mut grid = hex_state().grid.write().map_err(|e| e.to_string())?;
    if let Some(c) = grid.get(coord) {
        return Ok((c.clone(), false));
    }
    let cell = generate_wild_cell(&grid, coord);
    grid.insert(cell.clone());
    drop(grid);
    save_to_disk()?;
    Ok((cell, true))
}

/// 將指定座標標記為已探索（explored = true）並持久化。
pub fn mark_cell_explored(coord: HexCoord) -> Result<(), String> {
    let mut grid = hex_state().grid.write().map_err(|e| e.to_string())?;
    if let Some(mut cell) = grid.get(coord).cloned()
        && !cell.explored
    {
        cell.explored = true;
        grid.insert(cell);
        drop(grid);
        save_to_disk()?;
    }
    Ok(())
}

/// 新角色 Hex 世界**唯一**出生點：座標 **(0,0)** 契約為**草原**（`Terrain::Grassland`）。
pub fn ensure_player_spawn_grassland_coord() -> Result<(i32, i32), String> {
    let coord = HexCoord::new(0, 0);
    let (mut cell, _) = ensure_world_cell_at(coord)?;
    let mut changed = false;
    if cell.terrain != Terrain::Grassland {
        cell.terrain = Terrain::Grassland;
        cell.name = format!("草原·{}", coord.to_cell_id());
        changed = true;
    }
    if !cell.tags.iter().any(|t| t == "player_spawn") {
        cell.tags.push("player_spawn".into());
        changed = true;
    }
    if changed {
        {
            let mut grid = hex_state().grid.write().map_err(|e| e.to_string())?;
            grid.insert(cell);
        }
        save_to_disk()?;
    }
    Ok((coord.q, coord.r))
}

pub(super) fn is_admin(cfg: &crate::config::Server, q: &AdminQuery) -> bool {
    if cfg.management_key.is_empty() {
        return true;
    }
    q.mg_key.as_deref() == Some(&cfg.management_key)
}

/// 管理授權檢查：未通過回傳 403 Response，通過則回傳 None。
pub(super) fn authz_check(cfg: &crate::config::Server, q: &AdminQuery) -> Option<Response> {
    if is_admin(cfg, q) {
        return None;
    }
    Some(
        (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error":"unauthorized"})),
        )
            .into_response(),
    )
}

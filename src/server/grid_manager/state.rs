static GRID_STATE: OnceLock<GridState> = OnceLock::new();

struct GridState {
    grid: RwLock<SquareGrid>,
}

// ─── 初始化 ──────────────────────────────────────────────────────────

/// 啟動時呼叫：載入 PostgreSQL `grid_cells` 的格網快照。
/// 若尚未切換至【紅土西麓】新生世界格網，立即生成並寫入 PG。
pub fn init() {
    let mut grid = match load_square_grid() {
        Some(g) => g,
        None => SquareGrid::new(),
    };

    let is_shalu_zhulin = grid.world_seed() == crate::grid::SHALU_ZHULIN_WORLD_SEED
        && grid.len() == 9801
        && grid
            .get(crate::grid::SquareCoord::new(0, 0))
            .map(|c| c.zone == "紅土西麓" && c.name == "伏流卵石灘")
            .unwrap_or(false)
        && grid
            .get(crate::grid::SquareCoord::new(-1, 0))
            .map(|c| {
                c.name == "刺竹林"
                    && c.objects
                        .iter()
                        .any(|o| o.id == "zl_bamboo_culm" && o.max_quantity >= 6)
            })
            .unwrap_or(false)
        && !grid
            .cells()
            .any(|c| c.coord.x <= -30 && crate::grid::zhulin::is_bamboo_slot_cell(c));
    if !is_shalu_zhulin {
        tracing::info!(
            "grid_manager: 切換為滅世後的新生自然世界【紅土西麓 ↔ 台地稜線】(舊格數={}, 舊種子={})",
            grid.len(),
            grid.world_seed()
        );
        grid = crate::grid::generate_shalu_zhulin_grid();
        grid.clear_dirty();
        if let Err(e) = save_square_grid_to_pg(&grid) {
            tracing::warn!("grid_manager: 儲存紅土西麓地圖至 PG 失敗：{e}");
        }
    }
    grid.clear_dirty();

    let count = grid.len();
    let is_peninsula = grid.has_peninsula_wilderness();
    let _ = GRID_STATE.set(GridState {
        grid: RwLock::new(grid),
    });
    tracing::info!("grid_manager: 載入完成，共 {count} 格【紅土西麓】正方格地圖");
    if is_peninsula {
        let init_hour = crate::store::current_game_hour();
        sync_white_tower_scheduled_residents(init_hour);
    }
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

/// 取得執行期正方格地圖的唯讀快照（get_runtime_square_grid 之公開簡稱別名）。
pub fn get_square_grid() -> Option<SquareGrid> {
    get_runtime_square_grid()
}

/// 單元測試專用：設置執行期正方格地圖快照
pub fn set_runtime_square_grid_for_testing(grid: SquareGrid) {
    if let Some(state) = GRID_STATE.get() {
        if let Ok(mut g) = state.grid.write() {
            *g = grid;
        }
    } else {
        let _ = GRID_STATE.set(GridState {
            grid: std::sync::RwLock::new(grid),
        });
    }
}

/// 取得執行期正方格地圖單一格子的唯讀快照（避免單格查詢 clone 整張 14,424 格地圖）。
pub fn get_runtime_grid_cell(coord: SquareCoord) -> Option<GridCell> {
    GRID_STATE
        .get()
        .and_then(|s| s.grid.read().ok().and_then(|g| g.get(coord).cloned()))
}

/// 依房間名稱取得執行期正方格地圖單一格子的唯讀快照（零整圖 clone）。
pub fn get_runtime_grid_cell_by_name(room_name: &str) -> Option<GridCell> {
    GRID_STATE.get().and_then(|s| {
        s.grid
            .read()
            .ok()
            .and_then(|g| resolve_settlement_room_coord(&g, room_name).cloned())
    })
}

/// 以可變引用操作正方格地圖。僅在格數變動時寫回 PG。
///
/// 用於揭露新格、資源扣量、NPC 路徑標記等需要改動地圖的場景。
///
/// **鎖策略**：callback 執行和 PG 寫入分開。callback 在 grid write lock 內跑；
/// 若需寫 PG，clone snapshot 後釋放鎖、在無鎖狀態做 PG IO。
/// 避免快速連點移動時 600+ 行 INSERT 把 grid lock 霸佔秒級、整個 server 卡死。
pub fn with_square_grid_mut<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut SquareGrid) -> R,
{
    let st = grid_state();
    let (result, snapshot) = {
        let mut grid = st.grid.write().ok()?;
        let before = grid.len();
        let result = f(&mut grid);
        let snapshot = if grid.len() != before || grid.dirty() {
            grid.clear_dirty();
            Some(grid.clone())
        } else {
            None
        };
        (result, snapshot)
    }; // grid write lock 在此釋放

    if let Some(snap) = snapshot
        && let Err(e) = save_square_grid_to_pg(&snap)
    {
        tracing::warn!("with_square_grid_mut: PG 寫入失敗：{e}");
    }
    Some(result)
}

/// 僅於記憶體內修改正方格地圖並清除 dirty 標記（供已自行透過 `update_cell_state` 單格寫入 PG 的呼叫端使用，避免 14,424 格全表重寫）。
pub fn with_square_grid_mut_in_memory<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut SquareGrid) -> R,
{
    let st = grid_state();
    let mut grid = st.grid.write().ok()?;
    let result = f(&mut grid);
    grid.clear_dirty();
    Some(result)
}

/// 白塔半島 15 座村鎮（八大家族＋文曜民生五坊＋四鎮九村當家）34 位排程住民作息定義。
pub(crate) struct ResidentSchedule {
    pub(crate) id: &'static str,
    pub(crate) gender: &'static str,
    pub(crate) display_title: &'static str,
    pub(crate) resolve: fn(hour: i32) -> (&'static str, &'static str),
}

pub static RESIDENT_MOVE_TARGETS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, (i32, i32, String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub static RESIDENT_IDLE_TICKS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, u32>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
pub static RESIDENT_GOALS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, String>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn get_resident_goal(npc_id: &str) -> Option<String> {
    RESIDENT_GOALS.read().get(npc_id).cloned()
}

pub fn set_resident_goal(npc_id: &str, goal_kind: &str) {
    RESIDENT_GOALS.write().insert(npc_id.to_string(), goal_kind.to_string());
}

pub fn clear_resident_goal(npc_id: &str) {
    RESIDENT_GOALS.write().remove(npc_id);
}

pub trait EntityGoalExt {
    fn goal_kind(&self) -> Option<String>;
}

impl EntityGoalExt for crate::store::Entity {
    fn goal_kind(&self) -> Option<String> {
        get_resident_goal(&self.id)
    }
}

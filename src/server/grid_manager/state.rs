static GRID_STATE: OnceLock<GridState> = OnceLock::new();

struct GridState {
    grid: RwLock<SquareGrid>,
}

// ─── 初始化 ──────────────────────────────────────────────────────────

/// 啟動時呼叫：載入 PostgreSQL `grid_cells` 的格網快照。
/// 若載入之舊格數不足 3,000 格或未對齊聚落座標，自動升級為白塔半島 80 萬格無縫大地圖。
pub fn init() {
    let mut grid = match load_square_grid() {
        Some(g) => g,
        None => SquareGrid::new(),
    };

    let before_count = grid.len();
    let center_is_wen_yao = grid
        .get(crate::grid::SquareCoord::new(0, 0))
        .map(|c| c.zone == "文曜" && c.name.contains("文曜"))
        .unwrap_or(false);
    let lao_yan_tou_at_proper_pos = grid
        .get(crate::grid::SquareCoord::new(0, 125))
        .map(|c| c.zone == "老堰頭")
        .unwrap_or(false);
    let wen_yao_expanded = grid
        .get(crate::grid::SquareCoord::new(0, 10))
        .map(|c| c.zone == "文曜")
        .unwrap_or(false);
    let center_north_named = grid
        .get(crate::grid::SquareCoord::new(0, 1))
        .map(|c| c.name == "文曜・三水口北街")
        .unwrap_or(false)
        && grid
            .get(crate::grid::SquareCoord::new(0, 0))
            .map(|c| c.name == "文曜・三水匯流大石橋")
            .unwrap_or(false)
        && grid
            .get(crate::grid::SquareCoord::new(0, -1))
            .map(|c| c.name == "文曜・三水口南街")
            .unwrap_or(false);
    let distinct_wenyao_streets = grid
        .get(crate::grid::SquareCoord::new(0, 2))
        .map(|c| c.name == "文曜・鐘樓前街")
        .unwrap_or(false);
    let segmented_outer_streets = grid
        .get(crate::grid::SquareCoord::new(0, 11))
        .map(|c| c.name == "文曜・北門外街南段")
        .unwrap_or(false)
        && grid
            .get(crate::grid::SquareCoord::new(-10, 10))
            .map(|c| c.name == "文曜・三水內環西北隅")
            .unwrap_or(false);
    let deed_office_updated = grid
        .get(crate::grid::SquareCoord::new(-2, -1))
        .map(|c| c.name.contains("房地契書局") || c.description.contains("地籍契書專局"))
        .unwrap_or(false);
    let wen_yao_has_water = grid
        .get(crate::grid::SquareCoord::new(-2, 2))
        .map(|c| c.terrain == crate::grid::Terrain::Water)
        .unwrap_or(false);
    let fen_jia_has_clinic = grid
        .cells()
        .any(|c| c.zone == "文曜" && c.name.contains("溫氏脈絡診所") && c.coord.x > 0 && c.coord.y < 0);
    let academy_has_fountain_pen = grid
        .cells()
        .any(|c| c.zone == "文曜" && c.name.contains("學院") && c.description.contains("黃銅硬筆") && !c.description.contains("文曜文曜"));
    let outer_workshops_upgraded = grid
        .get(crate::grid::SquareCoord::new(-8, 2))
        .map(|c| c.terrain == crate::grid::Terrain::Workshop)
        .unwrap_or(false);
    let has_da_zhun_tou = grid.cells().any(|c| c.zone == "大圳頭");
    let has_san_shui_shang_xing = grid.cells().any(|c| c.name.contains("三水總商行") || c.name == "三水總商行");
    let has_meta_slop = grid.cells().any(|c| {
        c.description.contains("模擬人生")
            || c.description.contains("單次")
            || c.description.contains("算盤")
            || c.description.contains("銀票")
            || c.name.contains("客棧")
            || c.name.contains(concat!("茶", "肆"))
            || c.name.contains(concat!("鏢", "局"))
            || c.description.contains("行人踏石而過")
            || c.description.contains("牛肉肉乾")
            || c.objects.iter().any(|o| o.name == "培植靈草")
            || c.name.contains("蒙學堂")
            || c.name.contains("書院")
            || c.name.contains("算學舍")
            || c.description.contains("學子")
            || c.name.contains("修傘補靴攤")
            || c.description.contains("歇腳麵攤")
            || c.description.contains("焦香烤番薯攤")
    });
    let has_stale_zones_or_roads = grid.cells().any(|c| {
        c.zone == "百工原"
            || c.name == "百工原大道"
            || c.name.contains("百工原")
            || c.name == "長風大道"
            || c.name == "西南原大道"
            || c.zone.contains("幹道")
            || c.zone.contains("山道")
            || c.zone == "千岩谷"
            || c.zone == "赤霞林"
            || c.zone == "落雁灘"
            || c.zone == "聽濤岬"
            || c.zone == "避風灣"
            || c.zone == "白塔雪峰"
            || c.zone == "白塔遺蹟"
            || c.zone == "天梯石階"
            || c.zone == "三水川"
            || (c.coord.x == -18 && c.coord.y == 0 && c.zone != "西麓林道")
            || (c.coord.x == -18 && c.coord.y == -1 && c.zone != "西麓林道")
            || c.name.contains("築港頭商行大道")
            || c.name.contains("街巷空坪")
            || c.name.contains("西南原")
            || c.name.contains("青脊山")
            || c.name.contains("\u{638c}櫃")
            || c.name.contains("梁架居")
            || c.name.contains("煉金格物所")
            || c.name.contains("導能翻土犁")
            || c.name.contains("宣筆\u{5236}毫坊")
            || c.name.contains("竹筏\u{624e}作宅")
    }) || grid.get(SquareCoord::new(-18, 0)).map_or(true, |c| c.objects.is_empty())
        || grid.get(SquareCoord::new(2, 125)).map_or(true, |c| c.terrain == crate::grid::Terrain::Bridge)
        || grid.get(SquareCoord::new(-280, 160)).map_or(true, |c| c.zone != "天然港灣道")
        || grid.get(SquareCoord::new(100, 115)).map_or(true, |c| c.terrain == crate::grid::Terrain::Water)
        || grid.get(SquareCoord::new(-270, 160)).map_or(true, |c| c.terrain == crate::grid::Terrain::Wall)
        || grid.get(SquareCoord::new(-285, -147)).map_or(true, |c| c.terrain != crate::grid::Terrain::Road)
        || grid.get(SquareCoord::new(-75, -281)).map_or(true, |c| c.terrain != crate::grid::Terrain::Bridge)
        || grid.get(SquareCoord::new(6, 164)).map_or(true, |c| c.terrain != crate::grid::Terrain::Road)
        || grid.get(SquareCoord::new(-300, 195)).map_or(true, |c| c.zone != "天然港灣道")
        || grid.get(SquareCoord::new(0, -499)).map_or(true, |c| c.zone != "青籠關");
    let has_non_wenyao_houses = grid
        .get(crate::grid::SquareCoord::new(110, 115))
        .map(|c| c.zone == "北嶺腳")
        .unwrap_or(false)
        && grid
            .cells()
            .any(|c| c.name.contains("馬家鐵齒登山釘鞋工坊宅") && c.description.contains("防風青石板"))
        && grid
            .cells()
            .any(|c| c.name.contains("架高避水木屋") && c.description.contains("耐水烏木高椿"))
        && !grid
            .cells()
            .any(|c| c.zone == "長風草甸" && c.terrain == crate::grid::Terrain::Grassland && c.objects.is_empty());
    if before_count < 3000
        || !center_is_wen_yao
        || !lao_yan_tou_at_proper_pos
        || !wen_yao_expanded
        || !center_north_named
        || !distinct_wenyao_streets
        || !segmented_outer_streets
        || !has_da_zhun_tou
        || !has_san_shui_shang_xing
        || has_meta_slop
        || has_stale_zones_or_roads
        || !has_non_wenyao_houses
        || !deed_office_updated
        || !wen_yao_has_water
        || !fen_jia_has_clinic
        || !academy_has_fountain_pen
        || !outer_workshops_upgraded
    {
        tracing::info!(
            "grid_manager: 偵測到舊版格網或需更新為 80 萬格架構 (count={}, center_is_wen_yao={}, lao_yan_tou_at_proper_pos={}, wen_yao_expanded={})，全面切換為白塔半島大地圖",
            before_count,
            center_is_wen_yao,
            lao_yan_tou_at_proper_pos,
            wen_yao_expanded
        );
        grid = crate::grid::generate_white_tower_grid();
        grid.clear_dirty();
        if let Err(e) = save_square_grid_to_pg(&grid) {
            tracing::warn!("grid_manager: 儲存白塔半島大地圖至 PG 失敗：{e}");
        }
    }
    grid.clear_dirty();

    let count = grid.len();
    let _ = GRID_STATE.set(GridState {
        grid: RwLock::new(grid),
    });
    tracing::info!("grid_manager: 載入完成，共 {count} 顯式格白塔半島無縫大地圖（覆蓋 80 萬格）");
    let init_hour = crate::store::current_game_hour();
    sync_white_tower_scheduled_residents(init_hour);
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

/// 依聚落房間名稱取得執行期正方格地圖單一格子的唯讀快照（零整圖 clone）。
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

/// 白塔半島 15 座聚落（八大家族＋文曜民生五坊＋四鎮九村當家）34 位排程住民作息定義。
struct ResidentSchedule {
    id: &'static str,
    gender: &'static str,
    display_title: &'static str,
    resolve: fn(hour: i32) -> (&'static str, &'static str),
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

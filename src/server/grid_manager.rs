//! 正方格地圖執行期狀態管理器。
//!
//! 以 `OnceLock<GridState>` + `RwLock<SquareGrid>` 為核心：
//! - 啟動時自 PostgreSQL 載入退役格網，只供舊管理工具回查。
//! - 提供唯讀快照（`get_runtime_square_grid`）與可變操作（`with_square_grid_mut`）。
//! - 對稱 `hex_editor.rs` 的 `HEX_STATE` 設計，使兩地圖管線結構一致。

use std::sync::{OnceLock, RwLock};

use crate::db::{load_square_grid, save_square_grid_to_pg};
use crate::grid::{GridCell, GridObjectKind, SquareCoord, SquareGrid};

// ─── 全域狀態 ────────────────────────────────────────────────────────

static GRID_STATE: OnceLock<GridState> = OnceLock::new();

struct GridState {
    grid: RwLock<SquareGrid>,
}

// ─── 初始化 ──────────────────────────────────────────────────────────

/// 啟動時呼叫：載入 PostgreSQL `grid_cells` 的格網快照。
/// 若載入之舊格數不足 3,000 格或未對齊正典聚落座標，自動升級為白塔半島 80 萬格正典無縫大地圖。
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
    let lao_yan_tou_at_canon_pos = grid
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
    let has_canonical_non_wenyao_houses = grid
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
        || !lao_yan_tou_at_canon_pos
        || !wen_yao_expanded
        || !center_north_named
        || !distinct_wenyao_streets
        || !segmented_outer_streets
        || !has_da_zhun_tou
        || !has_san_shui_shang_xing
        || has_meta_slop
        || has_stale_zones_or_roads
        || !has_canonical_non_wenyao_houses
        || !deed_office_updated
        || !wen_yao_has_water
        || !fen_jia_has_clinic
        || !academy_has_fountain_pen
        || !outer_workshops_upgraded
    {
        tracing::info!(
            "grid_manager: 偵測到舊版格網或需更新為正典 80 萬格架構 (count={}, center_is_wen_yao={}, lao_yan_tou_at_canon_pos={}, wen_yao_expanded={})，全面切換為白塔半島正典大地圖",
            before_count,
            center_is_wen_yao,
            lao_yan_tou_at_canon_pos,
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
    sync_canonical_white_tower_npcs(init_hour);
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

/// 白塔半島 15 座聚落（八大家族＋文曜民生五坊＋四鎮九村當家）34 位正典核心人物作息定義。
struct CanonicalNpcSchedule {
    id: &'static str,
    gender: &'static str,
    display_title: &'static str,
    resolve: fn(hour: i32) -> (&'static str, &'static str),
}

pub static CANONICAL_NPC_TARGETS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, (i32, i32, String, String)>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub static CANONICAL_NPC_IDLE_TICKS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, u32>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));
pub static CANONICAL_NPC_GOALS: std::sync::LazyLock<
    parking_lot::RwLock<std::collections::HashMap<String, String>>,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

pub fn get_canonical_npc_goal(npc_id: &str) -> Option<String> {
    CANONICAL_NPC_GOALS.read().get(npc_id).cloned()
}

pub fn set_canonical_npc_goal(npc_id: &str, goal_kind: &str) {
    CANONICAL_NPC_GOALS.write().insert(npc_id.to_string(), goal_kind.to_string());
}

pub fn clear_canonical_npc_goal(npc_id: &str) {
    CANONICAL_NPC_GOALS.write().remove(npc_id);
}

pub trait EntityGoalExt {
    fn goal_kind(&self) -> Option<String>;
}

impl EntityGoalExt for crate::store::Entity {
    fn goal_kind(&self) -> Option<String> {
        get_canonical_npc_goal(&self.id)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CanonicalExpeditionDef {
    pub npc_id: &'static str,
    pub goal_kind: &'static str,
    pub target_coord: (i32, i32),
    pub target_name: &'static str,
    pub activity: &'static str,
    pub thought: &'static str,
    pub return_thought: &'static str,
    pub obligation_kind: &'static str,
    pub obligation_desc: &'static str,
}

pub const CANONICAL_EXPEDITIONS: &[CanonicalExpeditionDef] = &[
    CanonicalExpeditionDef {
        npc_id: "岑拓",
        goal_kind: "caravan_route",
        target_coord: (-22, 0),
        target_name: "西麓林道",
        activity: "押著滿載鐵器與茶磚的雙輪貨車行進在西麓林道上，隨時留意路旁獸蹤",
        thought: "西麓林道今日風乾路實，趁天色尚早把這批貨趕送出去，回程還能收些山貨",
        return_thought: "在城外林道完成了商隊貨物巡察與交易，正準備沿路返回城裡",
        obligation_kind: "stranded_cargo",
        obligation_desc: "押送鐵器與茶磚商隊車巡視西麓林道",
    },
    CanonicalExpeditionDef {
        npc_id: "溫晴",
        goal_kind: "wilderness_expedition",
        target_coord: (-20, 1),
        target_name: "西麓林道",
        activity: "背著竹編藥簍在西麓林道旁辨識新冒芽的光葉藥草與山茶籽",
        thought: "百草堂裡的退熱草藥快見底了，趁今日無雨親自出西門採些鮮葉回去炮製",
        return_thought: "在城外林道採齊了所需的藥材與山貨，正準備沿路返回城裡",
        obligation_kind: "restock_goods",
        obligation_desc: "採集光葉藥草與山茶籽以備炮製",
    },
    CanonicalExpeditionDef {
        npc_id: "馬行頭",
        goal_kind: "caravan_route",
        target_coord: (-18, 0),
        target_name: "西麓林道",
        activity: "在西城門外的西麓林道起點檢查駝獸蹄鐵與車轍路況",
        thought: "出西門這段林道車轍頗深，得叮囑車夫們過坡時收緊韁繩",
        return_thought: "在城外林道檢查完路面車轍與蹄鐵，正準備沿路返回車馬行",
        obligation_kind: "repair_tool",
        obligation_desc: "檢查西麓林道起點路況與駝獸蹄鐵車軛",
    },
    CanonicalExpeditionDef {
        npc_id: "方石",
        goal_kind: "wilderness_expedition",
        target_coord: (0, 18),
        target_name: "翠冠巨木林",
        activity: "在翠冠巨木林邊緣勘查林木生長與地脈礦脈走勢，記錄可供開採之物產",
        thought: "北門外這片巨木林地氣豐沛，得仔細巡看幾處新露頭的礦石與巨木材質",
        return_thought: "巨木林物產勘查完畢，記錄齊全，正準備回坊間核實物料儲備",
        obligation_kind: "repair_tool",
        obligation_desc: "勘查北門外巨木林與礦石分佈記錄",
    },
    CanonicalExpeditionDef {
        npc_id: "林山",
        goal_kind: "wilderness_expedition",
        target_coord: (0, 20),
        target_name: "翠冠巨木林",
        activity: "在翠冠巨木林間巡查獵徑與巨木枝葉，留意林間野獸足跡與落葉堆",
        thought: "翠冠巨木林深處獸蹤漸密，得多加留意野豬與山鹿的活動範圍",
        return_thought: "巡林一圈未見異常，帶回了幾份鮮松脂與巡林筆記",
        obligation_kind: "restock_goods",
        obligation_desc: "巡查翠冠巨木林獵徑與林間野獸蹤跡",
    },
    CanonicalExpeditionDef {
        npc_id: "高望雲",
        goal_kind: "wilderness_expedition",
        target_coord: (60, 160),
        target_name: "清泉溪谷",
        activity: "在清泉溪谷高處搭設測風儀，記錄高空雲層流向與山谷濕度",
        thought: "半嶺北坡的風向每至午後便有微變，需親自上雲杉林高處核準氣象測值",
        return_thought: "今日雲杉林氣象測值已完整錄下，這週的降水預測有了著落",
        obligation_kind: "repair_tool",
        obligation_desc: "半嶺北坡高空風向與溪谷濕度實地校準",
    },
    CanonicalExpeditionDef {
        npc_id: "雷橫山",
        goal_kind: "wilderness_expedition",
        target_coord: (0, -340),
        target_name: "淺澤泥口",
        activity: "在淺澤泥口灘塗邊檢查重型捕獸夾與耐腐竹樁，排除水澤陷坑危險",
        thought: "泥口潮水退去時常有猛獸涉水，得確保捕獸夾與耐腐機關牢固",
        return_thought: "淺澤泥口的獵具機關已全數調校牢固，可保南哨沿線防務無虞",
        obligation_kind: "repair_tool",
        obligation_desc: "巡檢淺澤泥口重型獵具與防禦機關",
    },
    CanonicalExpeditionDef {
        npc_id: "雷千鈞",
        goal_kind: "wilderness_expedition",
        target_coord: (0, -340),
        target_name: "淺澤泥口",
        activity: "在淺澤泥口灘塗邊檢查重型捕獸夾與耐腐竹樁，排除水澤陷坑危險",
        thought: "泥口潮水退去時常有猛獸涉水，得確保捕獸夾與耐腐機關牢固",
        return_thought: "淺澤泥口的獵具機關已全數調校牢固，可保南哨沿線防務無虞",
        obligation_kind: "repair_tool",
        obligation_desc: "巡檢淺澤泥口重型獵具與防禦機關",
    },
    CanonicalExpeditionDef {
        npc_id: "葉青",
        goal_kind: "wilderness_expedition",
        target_coord: (120, -320),
        target_name: "白茫洲",
        activity: "在白茫洲蘆葦叢深處採集祛瘴白蘚與水蓼鮮葉，並測試水質毒性",
        thought: "白茫洲水草間的白蘚正是藥性最足的時節，多採些回去配製避瘴丸",
        return_thought: "在白茫洲採得新鮮白蘚與清心水草，回去便能開爐煉製避瘴散",
        obligation_kind: "restock_goods",
        obligation_desc: "採集白茫洲祛瘴白蘚與避瘴水草鮮葉",
    },
    CanonicalExpeditionDef {
        npc_id: "葉青囊",
        goal_kind: "wilderness_expedition",
        target_coord: (120, -320),
        target_name: "白茫洲",
        activity: "在白茫洲蘆葦叢深處採集祛瘴白蘚與水蓼鮮葉，並測試水質毒性",
        thought: "白茫洲水草間的白蘚正是藥性最足的時節，多採些回去配製避瘴丸",
        return_thought: "在白茫洲採得新鮮白蘚與清心水草，回去便能開爐煉製避瘴散",
        obligation_kind: "restock_goods",
        obligation_desc: "採集白茫洲祛瘴白蘚與避瘴水草鮮葉",
    },
    CanonicalExpeditionDef {
        npc_id: "田大犁",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -195),
        target_name: "長風草甸",
        activity: "在長風草甸開闊處實地測試重型翻土犁刀的入土深度與草根切斷力",
        thought: "草甸的黑土質地緊密，這款改良犁頭還得再調大兩分傾角才能更省獸力",
        return_thought: "草甸犁刀試驗紀錄詳實，回工坊後即可敲定新一批農具規格",
        obligation_kind: "repair_tool",
        obligation_desc: "長風草甸實地試驗改良農具與犁頭深度",
    },
    CanonicalExpeditionDef {
        npc_id: "田百稼",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -195),
        target_name: "長風草甸",
        activity: "在長風草甸開闊處實地測試重型翻土犁刀的入土深度與草根切斷力",
        thought: "草甸的黑土質地緊密，這款改良犁頭還得再調大兩分傾角才能更省獸力",
        return_thought: "草甸犁刀試驗紀錄詳實，回工坊後即可敲定新一批農具規格",
        obligation_kind: "repair_tool",
        obligation_desc: "長風草甸實地試驗改良農具與犁頭深度",
    },
    CanonicalExpeditionDef {
        npc_id: "牧野",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -162),
        target_name: "長風草甸",
        activity: "在長風草甸巡視放牧的長毛牛群，檢查草場豐茂度與飲水坑水質",
        thought: "春季草甸草勢漸盛，得讓牛群往北面多移幾十丈，避開低窪積水坑",
        return_thought: "今日長風草甸牛群健旺無恙，草色正美，心裡格外舒暢",
        obligation_kind: "restock_goods",
        obligation_desc: "巡視長風草甸放牧牛群與草場飲水點",
    },
    CanonicalExpeditionDef {
        npc_id: "牧長風",
        goal_kind: "wilderness_expedition",
        target_coord: (-180, -162),
        target_name: "長風草甸",
        activity: "在長風草甸巡視放牧的長毛牛群，檢查草場豐茂度與飲水坑水質",
        thought: "春季草甸草勢漸盛，得讓牛群往北面多移幾十丈，避開低窪積水坑",
        return_thought: "今日長風草甸牛群健旺無恙，草色正美，心裡格外舒暢",
        obligation_kind: "restock_goods",
        obligation_desc: "巡視長風草甸放牧牛群與草場飲水點",
    },
];

pub fn find_canonical_expedition(npc_id: &str) -> Option<&'static CanonicalExpeditionDef> {
    CANONICAL_EXPEDITIONS.iter().find(|e| e.npc_id == npc_id)
}

fn initial_thought_for_npc(id: &str, title: &str) -> String {
    if let Some(exp) = find_canonical_expedition(id) {
        return exp.thought.to_string();
    }
    if title.contains("聯運") || title.contains("總站") || title.contains("車馬") {
        "各路商隊與駝車進出頻繁，得確保草料充足、車馬蹄鐵安妥".to_string()
    } else if title.contains("總商行") || title.contains("會長") || title.contains("票號") {
        "三水總商行的跨鎮通兌匯票已清點完畢，各口岸船運平穩".to_string()
    } else if title.contains("藏書") || title.contains("主編") || title.contains("風物誌") {
        "古卷殘篇與各鎮新送來的地鳴紀錄對比頗有收穫，得抓緊修纂風物誌".to_string()
    } else if title.contains("水輪") || title.contains("石磨") {
        "聽著堰渠水輪推動大石磨的低沉嗡鳴，心裡盤算著明日送往文曜三水大食堂的現磨溫米漿與大圳頭新穀碾期".to_string()
    } else if title.contains("鍛") || title.contains("鐵") {
        "爐火正旺，這批新出爐的精鐵兵刃得趁熱淬火才能堅韌並濟".to_string()
    } else if title.contains("學院") || title.contains("教務") {
        "文曜學院新一期的格物課綱擬定了大半，得為學子多添幾部古算籍".to_string()
    } else if title.contains("食堂") || title.contains("主廚") {
        "今日鮮魚湯底文火慢煨恰到好處，待會得備好新鮮菜蔬迎客人".to_string()
    } else if title.contains("浴場") || title.contains("溫泉") {
        "泉香浴場水溫適中，寒水玉引流清澈，正適合遠行歸人洗去一身風塵".to_string()
    } else if title.contains("木作") || title.contains("木匠") {
        "這根老料烏木乾燥得宜，正好用來打造大旅社的雕花案桌".to_string()
    } else if title.contains("織") || title.contains("染") || title.contains("服飾") {
        "新染就的青藍織錦光澤均勻，正合適裁剪換季的厚實袍服".to_string()
    } else if title.contains("匠") || title.contains("工") {
        "手頭這批工料正加緊整修，心裡琢磨著今日工序進度".to_string()
    } else if title.contains("醫") || title.contains("診") {
        "藥櫃裡的常備藥材得及早補齊，免得病患急需時周轉不及".to_string()
    } else if title.contains("商") || title.contains("店") || title.contains("攤") {
        "盤算著今日進貨帳目與市面行情，打點鋪面準備開門接客".to_string()
    } else if title.contains("獵") || title.contains("巡") || title.contains("防") {
        "留意著城郊各處道路與荒野動靜，提防野獸與異動".to_string()
    } else {
        "手頭事務料理妥當，心裡踏實沉穩".to_string()
    }
}


static CANONICAL_NPC_SCHEDULES: [CanonicalNpcSchedule; 59] = [
        // 1. 陸秉衡
        CanonicalNpcSchedule {
            id: "陸秉衡",
            gender: "M",
            display_title: "三水總商行老會長|陸秉衡",
            resolve: |hour| match hour {
                5..=6 => (
                    "三水大食堂",
                    "坐在靠窗桌邊喝著熱騰騰的鮮魚湯，笑呵呵地聽過路行商聊各地見聞",
                ),
                7..=13 => (
                    "三水總商行總部",
                    "翻看各城鎮送來的總帳冊，開立通行鎂幣存摺與跨鎮通兌匯票，偶爾指點年輕主管幾句商務規矩",
                ),
                14..=16 => (
                    "三水外坊・川流貨船大埠",
                    "巡視川流貨船大埠的泊位與商隊駁運，核對各港口送達的糧鹽通兌清單",
                ),
                17..=18 => (
                    "三水總商行總部",
                    "在總部核算全日貨款與船引回執，簽署發往風舶角的定期水運班次",
                ),
                19..=20 => (
                    "泉香溫水浴場",
                    "泡在溫熱的石砌泉池裡舒展筋骨，與熟識的老街坊閒話家常",
                ),
                _ => (
                    "三水口・陸家宅邸",
                    "在庭院藤椅上乘涼喝茶，聽孫女陸知微講述藏書閣新整理出的古卷",
                ),
            },
        },
        // 2. 陸知微
        CanonicalNpcSchedule {
            id: "陸知微",
            gender: "F",
            display_title: "文曜藏書閣主編|陸知微",
            resolve: |hour| match hour {
                8..=13 => (
                    "文曜藏書閣與風物誌編纂處",
                    "在木架間比對先祖渡海時留下的航海古卷殘篇與北嶺腳送來的地鳴觀測紀錄",
                ),
                14..=16 => (
                    "講堂外坊・格物輿圖外院",
                    "在格物輿圖外院與測繪教席比對半島新繪製的沿海沙洲水路輿圖與洋流觀測錄",
                ),
                17..=18 => (
                    "文曜藏書閣與風物誌編纂處",
                    "整理自外院謄抄回來的輿圖定點筆記，校勘《白塔風物誌》地脈篇章",
                ),
                19..=21 => (
                    "三水大食堂",
                    "點了幾樣清淡小菜，邊吃邊在隨身筆記本上記下採訪來的風物傳聞",
                ),
                _ => (
                    "三水口・陸家宅邸",
                    "在書房燈下整理新一期《白塔風物誌》的文稿",
                ),
            },
        },
        // 3. 岑守岳
        CanonicalNpcSchedule {
            id: "岑守岳",
            gender: "M",
            display_title: "岑氏第一鍛造工坊大匠師|岑守岳",
            resolve: |hour| match hour {
                8..=13 => (
                    "岑氏第一鍛造工坊",
                    "站在通紅的火爐旁檢視打製好的獵具與農具，耳提面命地叮囑學徒把握柄打磨順手",
                ),
                14..=16 => (
                    "百工外坊・重鐵水錘大鍛坊",
                    "巡視外坊重鐵水錘大鍛坊的鍛造進度，指點水錘淬火火候與巨型鋼砧敲擊節奏",
                ),
                17..=18 => (
                    "岑氏第一鍛造工坊",
                    "回宗坊清點新鍛成的大宗開山鐵器，叮嚀學徒做好防鏽浸油封存",
                ),
                19..=21 => (
                    "泉香溫水浴場",
                    "泡在熱池裡洗去一身炭火煤灰，嘴上咕噥著山上二房那些倔脾氣的傢伙",
                ),
                _ => (
                    "分家巷・岑氏匠師宗宅",
                    "在院子裡擦拭老一輩留下的鍛錘，心裡暗自惦記著山上弟弟的近況",
                ),
            },
        },
        // 4. 岑拓
        CanonicalNpcSchedule {
            id: "岑拓",
            gender: "M",
            display_title: "岑氏鍛造工坊匠師|岑拓",
            resolve: |hour| match hour {
                8..=19 => (
                    "岑氏第一鍛造工坊",
                    "拿著大圳頭新運來的青脊重礦與半嶺鎮的寒水玉，專注地試打新式器物",
                ),
                20..=21 => (
                    "三水大食堂",
                    "與西門聯運站的車隊夥伴同桌吃飯，託他們順路把新畫好的器物草圖帶上山給二叔公",
                ),
                _ => (
                    "分家巷・岑氏鍛藝居",
                    "在燈下修改器物圖紙，琢磨著怎麼讓山上山下兩邊的長輩都點頭服氣",
                ),
            },
        },
        // 5. 溫懷瑾
        CanonicalNpcSchedule {
            id: "溫懷瑾",
            gender: "M",
            display_title: "文曜學院教務長|溫懷瑾",
            resolve: |hour| match hour {
                8..=17 => (
                    "文曜學院主校區",
                    "在綠蔭講堂裡帶著年輕學生讀譜識字，講解感應天地同源能量的基礎法門",
                ),
                18..=20 => (
                    "東橋旅社",
                    "坐在大廳茶桌旁喝著溫熱的草木茶，與外地來的學者交流山海見聞",
                ),
                _ => (
                    "講堂街・溫家藥圃居",
                    "在院中整理歷年留存的字韻教學手札",
                ),
            },
        },
        // 6. 溫晴
        CanonicalNpcSchedule {
            id: "溫晴",
            gender: "F",
            display_title: "溫氏脈絡診所主治醫師|溫晴",
            resolve: |hour| match hour {
                8..=13 => (
                    "溫氏脈絡診所",
                    "替進山採集或狩獵受傷的居民敷上黑沼止血水苔藥膏，調理筋骨脈絡",
                ),
                14..=16 => (
                    "講堂外坊・杏林百草醫館",
                    "在杏林百草醫館與坐堂醫師會診，研討採藥棧送來的新鮮山林本草與調治方劑",
                ),
                17..=19 => (
                    "溫氏脈絡診所",
                    "回到診所整理外坊採買回來的特等藥材，細心為夜診居民抓配溫養草藥",
                ),
                20..=21 => (
                    "泉香溫水浴場",
                    "將新鮮調配好的溫養草木湯包交給浴場掌事，順道泡湯舒緩一天的疲憊",
                ),
                _ => (
                    "講堂街・溫家藥圃居",
                    "在藥房裡分揀北嶺腳採回來的光葉藥草",
                ),
            },
        },
        // 7. 韓鐵山
        CanonicalNpcSchedule {
            id: "韓鐵山",
            gender: "M",
            display_title: "西門車馬聯運總站長|韓鐵山",
            resolve: |hour| match hour {
                7..=13 => (
                    "西門車馬聯運總站",
                    "嗓門洪亮地指揮駝獸大車裝卸貨物，叮囑前往風舶角與大圳頭的車隊注意路況",
                ),
                14..=16 => (
                    "西門外坊・萬里駝隊大驛站",
                    "巡視西門外坊萬里駝隊大驛站的駝棚與大車停泊場，調配西部荒原重載貨隊的交接班次",
                ),
                17 => (
                    "西門車馬聯運總站",
                    "清點全日出入城車馬路單與貨棧押運契條，確認明晨首班車隊草料裝備齊全",
                ),
                18..=21 => (
                    "東橋旅社",
                    "在一樓茶座擺開棋盤找老朋友下棋喝茶，大聲聊著各條幹道的最新消息",
                ),
                _ => (
                    "西門・韓氏總站宅院",
                    "巡看院後棚裡歇息的健壯草獸，準備隔天一早的出車班次",
                ),
            },
        },
        // 8. 許萬川
        CanonicalNpcSchedule {
            id: "許萬川",
            gender: "M",
            display_title: "風舶海事商盟會長|許萬川",
            resolve: |hour| match hour {
                8..=17 => (
                    "風舶海事商盟總會",
                    "端坐在廳堂核對入港船隻的航線紀錄，再三告誡年輕船長別貿然闖進外海的海獸場域",
                ),
                18..=20 => (
                    "海風海鮮食堂",
                    "喝著鮮甜的石斑魚湯，聽老船員們提起外海最近的風浪變化",
                ),
                _ => (
                    "築港頭・許家宅邸",
                    "站在二樓窗前望著風舶海港的點點漁火，擔心孫女許海音又動了遠航外海的念頭",
                ),
            },
        },
        // 9. 許海音
        CanonicalNpcSchedule {
            id: "許海音",
            gender: "F",
            display_title: "遠洋大船北冕號船長|許海音",
            resolve: |hour| match hour {
                8..=14 => (
                    "深水泊位與卸貨碼頭",
                    "站在『北冕號』甲板上檢查加固後的船帆與纜繩，為下一趟往北繞過白塔雪巔的遠航做準備",
                ),
                15..=18 => (
                    "林氏大船塢",
                    "與總造船師林正雄湊在海圖前，熱切地討論遠古故島另外幾條遷徙航線可能的去向",
                ),
                19..=21 => (
                    "海風海鮮食堂",
                    "與船員們圍桌大啖現烤好的海魚，打聽東海峭壁與外海擾流的情報",
                ),
                _ => (
                    "築港頭・許家宅邸",
                    "在房裡翻閱先祖留下的渡海航海日誌抄本",
                ),
            },
        },
        // 10. 林正雄
        CanonicalNpcSchedule {
            id: "林正雄",
            gender: "M",
            display_title: "林氏大船塢總造船師|林正雄",
            resolve: |hour| match hour {
                7..=18 => (
                    "林氏大船塢",
                    "拍著北坡百年雲杉與溫木心材拼合的新船龍骨，親自督導工匠張掛能抗住外海大風浪的船帆",
                ),
                19..=21 => (
                    "港灣溫水浴場",
                    "泡在溫水池裡洗去身上的木屑與桐油味，盤算著新船下水試航的日子",
                ),
                _ => (
                    "大船塢・林家宅院",
                    "在工作台前繪製遠洋大船的改良結構圖",
                ),
            },
        },
        // 11. 周萬生
        CanonicalNpcSchedule {
            id: "周萬生",
            gender: "M",
            display_title: "大圳頭農牧總社理事長|周萬生",
            resolve: |hour| match hour {
                7..=17 => (
                    "大圳頭・大圳農牧總社",
                    "核對大圳頭的水閘放水時刻與牛埔的牧草收成，確保送往各城食堂的米糧蔬果充足",
                ),
                18..=20 => (
                    "田園風味大食堂",
                    "笑咪咪地品嚐當季新收成的燉菜，聽農人與牧戶聊田裡和草甸上的收成",
                ),
                _ => (
                    "大圳頭・周家莊園",
                    "在莊園院子裡修剪花木，享受平穩踏實的農家夜晚",
                ),
            },
        },
        // 12. 方石
        CanonicalNpcSchedule {
            id: "方石",
            gender: "M",
            display_title: "三號坑口礦務掌事|方石",
            resolve: |hour| match hour {
                7..=16 => (
                    "三號坑口礦務所",
                    "帶著礦工在三號坑口深處開鑿紅鐵礦與深山重礦，叮囑大夥檢查百年雲杉支木是否牢靠",
                ),
                17..=19 => (
                    "紅土崁選礦場",
                    "挑選成色最好的青脊重礦與赤紋鐵礦，準備交由聯運車隊送往文曜城",
                ),
                _ => (
                    "紅土崁・方家宅院",
                    "對著燈光檢視今日從三號坑口採出的深山重礦成色，整理明日要交給韓家車隊的貨單",
                ),
            },
        },
        // 13. 岑守淵
        CanonicalNpcSchedule {
            id: "岑守淵",
            gender: "M",
            display_title: "白塔與半嶺實戰名宿|岑守淵",
            resolve: |hour| match hour {
                7..=16 => (
                    "半嶺試招場",
                    "雙手抱胸站在場邊指點年輕修行者與獵人切磋身手，嗓音沉厚地糾正他們使力的破綻",
                ),
                17..=19 => (
                    "青脊石湯溫水浴場",
                    "泡在熱氣蒸騰的山泉石池裡驅散山間寒氣，看著山下轉交來的新器物草圖露出一絲笑意",
                ),
                20..=21 => (
                    "半嶺山野茶屋食堂",
                    "吃著熱騰騰的山產鍋物，聽自北山雲腰雪林下山的獵人描述北嶺深處的動靜",
                ),
                _ => (
                    "半嶺鎮・岑家二房別院",
                    "在靜室裡擦拭隨身兵刃，望著北方雲霧繚繞的白塔沉思",
                ),
            },
        },
        // 14. 羅烈
        CanonicalNpcSchedule {
            id: "羅烈",
            gender: "M",
            display_title: "南哨防務總隊長兼首席嚮導|羅烈",
            resolve: |hour| match hour {
                7..=14 => (
                    "南哨防務公議廳",
                    "與哨衛隊員核對水尾寮北口的巡防路線，記錄近期白茫洲霧氣消散時刻的異常變化",
                ),
                15..=18 => (
                    "入林嚮導公會",
                    "向準備進林採集的獵人交代水尾寮外的水路標記，並打聽那些自稱從『下營』以南穿林而來的異鄉行商下落",
                ),
                19..=21 => (
                    "南哨原野食堂",
                    "與巡防歸來的隊員們大口吃著烤肉與熱湯，留神聽著市集上傳來的南陸消息",
                ),
                _ => (
                    "高棧坪・羅家宅院",
                    "在燈下修補水尾寮深處的水路地圖，思索下營以南那片未知陸地的模樣",
                ),
            },
        },
        // 15. 陳萬福（三水食堂陳家）
        CanonicalNpcSchedule {
            id: "陳萬福",
            gender: "M",
            display_title: "三水食堂當家大廚|陳萬福",
            resolve: |hour| match hour {
                5..=10 => (
                    "三水大食堂",
                    "在灶前將老堰頭送來的現磨溫米漿與濃豆漿點成滑嫩熱豆花，煎著兩面金黃的蔥香米烙餅、蒸出熱騰騰的牛埔鮮肉包，替早起出門的鎮民裝滿隨身保溫瓶",
                ),
                11..=21 => (
                    "三水大食堂",
                    "守在嵌著蓄熱重礦底座的紅鐵厚底大釜旁，慢火細燉牛埔原牛鮮肉湯、風舶濃白海魚湯與切仔湯麵配滷蛋，從寒水玉保鮮石櫃取出冰鎮板豆腐與脆瓜醬菜招待食客",
                ),
                _ => (
                    "三水口・陳氏庖廚居",
                    "在廚房擰開黃銅水栓洗淨白釉瓷碗，清點大圳頭送來的壺底黑豆醬油與新米，準備明早開灶的食材",
                ),
            },
        },
        // 16. 沈清河（東橋旅社／東橋茶屋沈家）
        CanonicalNpcSchedule {
            id: "沈清河",
            gender: "M",
            display_title: "東橋旅社與茶屋掌事|沈清河",
            resolve: |hour| match hour {
                7..=21 => (
                    "東橋旅社",
                    "在一樓東橋茶屋慢煮鍋煮鮮奶茶與冰鎮甘蔗檸檬汁，端出奶油酥餅、黑糖米香與鹽烤海苔片招待下烏木白貝圍棋的茶客，並替投宿旅客打點樓上鋪著蓬鬆羽絨暖被、山風草安神枕與旋蓋夜光貝母台燈的獨立銅鎖單人客房",
                ),
                _ => (
                    "東橋畔・沈氏臨水居",
                    "在旋蓋夜光貝母台燈下核對今日房牌與茶屋帳目，聽著桌角黃銅發條座鐘的滴答聲歇息",
                ),
            },
        },
        // 17. 白泉生（泉香溫水浴場白家）
        CanonicalNpcSchedule {
            id: "白泉生",
            gender: "M",
            display_title: "泉香溫水浴場場主|白泉生",
            resolve: |hour| match hour {
                7..=21 => (
                    "泉香溫水浴場",
                    "巡看老堰頭活水暗管與鋪滿半嶺寒水玉的恆溫石池，在亮銀明水鏡盥洗台擺妥青鹽薄荷潔牙粉、軟毛竹柄潔齒刷、山茶松脂香胰、山茶護髮潤膚油與蜂格棉面巾",
                ),
                _ => (
                    "三水口・白氏湯屋居",
                    "檢視浴池壁下恆溫導能火磚與家用溫石熱水槽的蓄熱狀況，確保明早第一池早湯熱氣充足",
                ),
            },
        },
        // 18. 柯成器（木作傢俱工坊柯家）
        CanonicalNpcSchedule {
            id: "柯成器",
            gender: "M",
            display_title: "木作傢俱工坊大木匠|柯成器",
            resolve: |hour| match hour {
                8..=19 => (
                    "巧木居所傢俱坊",
                    "握著導能刨刀將北山松香良木打製成繃著彈力床網的大床、衣櫃與鑲貝書案，組裝內嵌北山寒水玉薄板的寒水玉保鮮石櫃，並車製旋蓋夜光貝母調光台燈與烏木筷",
                ),
                20..=21 => (
                    "東橋旅社",
                    "在一樓茶屋喝著鍋煮鮮奶茶、拈著黑糖米香，與林氏大船塢來的木料師傅商議下一批北山曲木與烏木邊材的交接",
                ),
                _ => (
                    "分家巷・柯氏木作居",
                    "換上軟底居家布便鞋，在旋蓋貝母台燈下用鋼尖蘸水硬筆在竹綿紙手札上繪製新式保鮮石櫃與桌椅圖樣",
                ),
            },
        },
        // 19. 蘇錦心（織造服飾坊蘇家）
        CanonicalNpcSchedule {
            id: "蘇錦心",
            gender: "F",
            display_title: "織造服飾坊主理人|蘇錦心",
            resolve: |hour| match hour {
                8..=19 => (
                    "雲錦織造與服飾坊",
                    "用大圳頭細棉麻裁製透氣常服，縫製北山雪狼絨冬衣、塗黑沼樹膠的防雨長衫、雙扣帆布斜背包與軟底居家布便鞋，並替客人挑選軟牛皮零錢夾與竹骨晴雨摺疊傘",
                ),
                20..=21 => (
                    "泉香溫水浴場",
                    "泡在溫潤的寒水玉池裡洗去疲乏，用山茶護髮潤膚油與松香木梳整理長髮，與溫晴閒聊換季衣料",
                ),
                _ => (
                    "分家巷・蘇氏繡坊居",
                    "蓋著曬滿陽光香氣的蓬鬆羽絨暖被、枕著山風草安神枕安穩入眠",
                ),
            },
        },
        // 20. 顧海平（風舶嶼・外澳領航公會顧家）
        CanonicalNpcSchedule {
            id: "顧海平",
            gender: "M",
            display_title: "外澳領航公會首座|顧海平",
            resolve: |hour| match hour {
                7..=17 => (
                    "外澳領航公會",
                    "校準嵌著夜光貝母刻度的夜光領航羅盤，指點進港停靠刮蠣修帆的遠洋船長避開西岸暗礁",
                ),
                18..=20 => (
                    "潮音溫水浴堂",
                    "泡在溫水池裡洗去一身海風鹹味，聽才下船的水手聊起北冕海的洋流變化",
                ),
                _ => (
                    "避風港船伕家屋",
                    "在燈下修訂風舶海港的潮汐與水深手冊",
                ),
            },
        },
        // 21. 洪守明（風櫃尾燈塔與冷燻工坊洪家）
        CanonicalNpcSchedule {
            id: "洪守明",
            gender: "M",
            display_title: "風櫃尾守塔與冷燻匠師|洪守明",
            resolve: |hour| match hour {
                8..=17 => (
                    "水產冷燻工坊",
                    "借著海蝕風洞終年不息的強勁海風，用北山松木屑將深海大魚冷燻成久放不壞的風櫃冷燻魚乾",
                ),
                18..=23 => (
                    "風櫃尾燈塔",
                    "在海岬高處的燈塔頂層添入大魚油脂與巨藻燈芯，守望夜裡歸航的船隊",
                ),
                _ => (
                    "風櫃尾・洪家守塔與冷燻匠師宅",
                    "聽著窗外海蝕洞如風櫃般的低鳴聲入睡",
                ),
            },
        },
        // 22. 鄭烏石（烏石鼻鑿石與貝灰鄭家）
        CanonicalNpcSchedule {
            id: "鄭烏石",
            gender: "M",
            display_title: "烏石鼻鑿石與灰窯匠師|鄭烏石",
            resolve: |hour| match hour {
                7..=16 => (
                    "玄武岩鑿石工坊",
                    "將堅硬沉重的烏黑礁岩鑿成遠洋大船定泊用的烏石重錨與防風屋基石",
                ),
                17..=20 => (
                    "烏石鼻海貝灰窯",
                    "巡看窯內燒製的海貝殼，篩出雪白防水的海貝灰漿裝袋待運",
                ),
                _ => (
                    "烏石鼻・鄭家烏石重錨鑿石老宅",
                    "在塗滿雪白海貝灰漿、滴水不滲的石屋裡歇息",
                ),
            },
        },
        // 23. 秦水生（老堰頭水輪石磨秦家）
        CanonicalNpcSchedule {
            id: "秦水生",
            gender: "M",
            display_title: "老堰頭水輪石磨首座|秦水生",
            resolve: |hour| match hour {
                7..=17 => (
                    "水輪石磨工坊",
                    "巡看老堰頭湍急溪水推動的重礦包邊大石磨，將大圳頭運來的稻麥與北山藥材碾得細膩均勻",
                ),
                18..=20 => (
                    "堰渠水畔食堂",
                    "喝著現磨米漿與熱麵湯，與北郊農戶商議明日引水入渠的時刻",
                ),
                _ => (
                    "老堰頭・秦氏水輪石磨傳承老宅",
                    "檢查引水石堰的水門插板，伴著水輪轉動的規律水聲歇息",
                ),
            },
        },
        // 24. 馬山行（北嶺腳山徑補給馬家）
        CanonicalNpcSchedule {
            id: "馬山行",
            gender: "M",
            display_title: "北嶺山徑補給行掌事|馬山行",
            resolve: |hour| match hour {
                6..=18 => (
                    "北嶺山徑補給行",
                    "替準備進山的樵夫與採藥人檢修鐵齒登山釘鞋與硬木藤背架，並分發添滿炭火的松脂暖手爐",
                ),
                19..=21 => (
                    "嶺腳暖火食堂",
                    "與下山歇腳的採藥人圍爐喝湯，打聽北山雲杉林裡最近的獸群動靜",
                ),
                _ => (
                    "北嶺腳・馬家鐵齒登山釘鞋工坊宅",
                    "在屋裡編織新的硬木藤背架，晾乾林間採回的山風草",
                ),
            },
        },
        // 25. 梁千木（霜林口水車鋸木梁家）
        CanonicalNpcSchedule {
            id: "梁千木",
            gender: "M",
            display_title: "霜林口水車鋸木匠師|梁千木",
            resolve: |hour| match hour {
                7..=16 => (
                    "原木水車鋸木坊",
                    "借著高山融雪溪流推動水力大鋸，將合抱粗的百年雲杉與松香良木裁成標準船骨材與傢俱板材",
                ),
                17..=19 => (
                    "霜林護林瞭望所",
                    "登上瞭望所清點今日伐木與『伐一補三』補種雲杉苗的數目",
                ),
                _ => (
                    "霜林口・梁氏水車鋸木大匠宅",
                    "在散發濃郁松木香的原木屋裡烘乾衣靴，保養水力大鋸的鋸片",
                ),
            },
        },
        // 26. 石萬鈞（半嶺鎮寒水玉切石工坊石家）
        CanonicalNpcSchedule {
            id: "石萬鈞",
            gender: "M",
            display_title: "寒水玉切石工坊大匠|石萬鈞",
            resolve: |hour| match hour {
                7..=17 => (
                    "寒水玉切石工坊",
                    "操持銀砂水力切石機，用溪底亮銀砂配韌鐵弦線將堅韌的寒水玉切得平整如鏡，並打磨凝神用的寒水玉握石",
                ),
                18..=20 => (
                    "青脊石湯溫水浴場",
                    "泡在自己親手鋪設的寒水玉石池裡，與岑守淵聊起新一批寒水玉礦脈的成色",
                ),
                _ => (
                    "半嶺鎮・石家寒水玉切石匠宅",
                    "在燈下挑選紋理最溫潤的寒水玉原石，準備明日切製送往文曜城",
                ),
            },
        },
        // 27. 袁山茶（半嶺鎮山泉旅社袁家）
        CanonicalNpcSchedule {
            id: "袁山茶",
            gender: "F",
            display_title: "半嶺山泉旅社掌事|袁山茶",
            resolve: |hour| match hour {
                7..=21 => (
                    "半嶺山泉旅社",
                    "用高山雪蓮葉與老薑熬煮驅寒的雪嶺暖身湯，招待上山試招切磋與採石的旅人",
                ),
                _ => (
                    "半嶺鎮・袁家半嶺山泉客舍",
                    "清點旅社備用的厚毛氈與烘乾的山茶葉",
                ),
            },
        },
        // 28. 高望雲（半嶺鎮氣象觀測所高家）
        CanonicalNpcSchedule {
            id: "高望雲",
            gender: "M",
            display_title: "半嶺氣象觀測師|高望雲",
            resolve: |hour| match hour {
                6..=18 => (
                    "半嶺氣象觀測所",
                    "守在岩台上的青銅測風儀旁，觀測白塔雪巔雲霧帶與風壓變化，將風雪與溪水漲落訊號傳往山下",
                ),
                19..=21 => (
                    "半嶺山野茶屋食堂",
                    "喝著熱茶整理今日的風向與雨量紀錄冊",
                ),
                _ => (
                    "半嶺鎮・高家青銅測風儀觀測居",
                    "推窗仰望雪巔夜空星象，核對明日的四時星象與風候",
                ),
            },
        },
        // 29. 田百稼（大圳頭農具與百工坊田家）
        CanonicalNpcSchedule {
            id: "田百稼",
            gender: "M",
            display_title: "大圳頭農具百工大匠|田百稼",
            resolve: |hour| match hour {
                7..=18 => (
                    "大圳頭・大圳農具與百工坊",
                    "調校一人扶把即可翻耕水田的省力導能翻土犁，並檢修榨取日照穀木油的水力榨油車",
                ),
                19..=21 => (
                    "田園風味大食堂",
                    "與周萬生同桌品嚐新米飯與燉菜，打聽各處農莊翻土犁的使用狀況",
                ),
                _ => (
                    "大圳頭・田氏農具匠人老宅",
                    "在院裡繪製改良水閘齒輪與農具的草圖",
                ),
            },
        },
        // 30. 牧長風（牛埔乳酪與毛氈坊牧家）
        CanonicalNpcSchedule {
            id: "牧長風",
            gender: "M",
            display_title: "牛埔乳酪與毛氈匠師|牧長風",
            resolve: |hour| match hour {
                7..=17 => (
                    "羊毛與乳酪精製坊",
                    "將大角原牛乳汁發酵成耐存放的牛埔硬乳酪與奶油，並把剪下的綿密獸毛壓製成防潮禦寒的厚毛氈",
                ),
                18..=20 => (
                    "牧野歇腳旅社",
                    "請往來的車隊腳夫品嚐新熟成的乳酪，託他們將厚毛氈捎往半嶺鎮與風舶雙港",
                ),
                _ => (
                    "牛埔・牧家牛埔硬乳酪發酵老宅",
                    "巡看草甸圍欄裡安睡的原牛群與疾行草獸",
                ),
            },
        },
        // 31. 方赤土（紅土崁選礦與耐火磚窯方家分支）
        CanonicalNpcSchedule {
            id: "方赤土",
            gender: "M",
            display_title: "紅土崁選礦與耐火磚匠師|方赤土",
            resolve: |hour| match hour {
                7..=17 => (
                    "紅土崁選礦場",
                    "督導洗選三號坑口運來的赤紋鐵礦，並用篩下的高耐火紅黏土燒製能承受千度爐溫的紅土耐火磚",
                ),
                18..=20 => (
                    "紅土百工鐵器坊",
                    "檢視三號坑口礦道鋪設用的輕便鐵槽軌與坑口滑輪台車輪軸",
                ),
                _ => (
                    "紅土崁・方氏分支選礦與耐火磚老宅",
                    "與姪兒方石核對明日要發往文曜分家巷的重礦與耐火磚車次",
                ),
            },
        },
        // 32. 雷千鈞（南哨重型獵具工坊雷家）
        CanonicalNpcSchedule {
            id: "雷千鈞",
            gender: "M",
            display_title: "南哨重型獵具大匠|雷千鈞",
            resolve: |hour| match hour {
                7..=18 => (
                    "重型獵具工坊",
                    "用風舶角送來的巨藻韌索混編三號坑口紅鐵絲，打製困捕泥甲巨鱷的鐵絲巨藻獵網、飛爪索與紅磷引路信號筒",
                ),
                19..=21 => (
                    "高棧坪溫水浴場",
                    "泡在溫水池裡拔除南境濕氣，與羅烈討論新一批紅磷信號筒在黑沼濃霧中的照遠效果",
                ),
                _ => (
                    "南哨・雷家重型獵具大匠宅",
                    "在乾燥密封的木箱裡封存新配好的紅磷信號筒",
                ),
            },
        },
        // 33. 葉青囊（南哨脈絡解毒診所葉家）
        CanonicalNpcSchedule {
            id: "葉青囊",
            gender: "M",
            display_title: "南哨解毒診所主治醫師|葉青囊",
            resolve: |hour| match hour {
                7..=19 => (
                    "南哨脈絡解毒診所",
                    "用白茫洲採回的霧晶與浮洲奇花煉製避瘴清心丸，替自水尾寮外澤林歸來的採集隊員拔除濕氣與蟲毒",
                ),
                20..=21 => (
                    "南哨原野食堂",
                    "喝著驅濕的辛香熱肉湯，叮囑年輕獵人進林前務必帶齊避瘴丸與止血水苔敷藥",
                ),
                _ => (
                    "南哨・葉家白茫避瘴清心齋",
                    "在燈下給文曜城的師妹溫晴寫信，交換最新一批黑沼草藥的炮製心得",
                ),
            },
        },
        // 34. 阮水生（水尾寮高腳屋與平底竹筏阮家）
        CanonicalNpcSchedule {
            id: "阮水生",
            gender: "M",
            display_title: "水尾寮竹筏與高腳屋匠師|阮水生",
            resolve: |hour| match hour {
                6..=17 => (
                    "蘆葦與竹編工棚",
                    "用韌竹與浮木紮製在爛泥淺灘上也能貼水滑行的平底竹筏，並檢修水尾寮高腳木屋底下的沉水烏木基樁",
                ),
                18..=20 => (
                    "水尾寮平底竹筏埠",
                    "幫歸來的採集竹筏卸下新打撈出的沉水烏木、止血水苔與鮮活沼蝦，準備送往高棧坪收購行",
                ),
                _ => (
                    "水尾寮・阮家高腳木屋與平底竹筏匠老宅",
                    "坐在通風防潮的高腳木屋廊下修補捕魚竹簍，聽著屋下淺澤的蛙鳴入眠",
                ),
            },
        },
        // 35. wt_npc_lu_chengli (公議堂地政登記員陸承禮)
        CanonicalNpcSchedule {
            id: "wt_npc_lu_chengli",
            gender: "M",
            display_title: "公議堂地政登記員|陸承禮",
            resolve: |hour| match hour {
                6..=19 => (
                    "公議堂房地契書局",
                    "正在桌前核對《半島全戶總譜》與紅印屋契，加蓋鋼印發放黃銅門牌",
                ),
                20..=21 => (
                    "三水大食堂",
                    "在食堂角落與調解員陸長貴清點今日備案的屋契檔案，吃著熱湯麵",
                ),
                _ => (
                    "三水口・票號總管居",
                    "在檯燈下核對房地產權清冊並與家人閒聊",
                ),
            },
        },
        // 36. wt_npc_lu_changgui (公議堂民務通事陸長貴)
        CanonicalNpcSchedule {
            id: "wt_npc_lu_changgui",
            gender: "M",
            display_title: "公議堂民務通事|陸長貴",
            resolve: |hour| match hour {
                8..=11 => (
                    "三水匯流大石橋",
                    "站在三水匯流大石橋旁指引外地商旅認路，解答各項定居入籍與過戶手續",
                ),
                12..=17 => (
                    "公議堂",
                    "在公議堂前廳調解各街區鄰里糾紛，協調分家工坊排煙與給水維護",
                ),
                18..=20 => (
                    "東橋旅社",
                    "在茶屋大廳喝著鍋煮鮮奶茶，向外地旅客了解各村鎮近況",
                ),
                _ => (
                    "三水口・石板弄堂天井",
                    "在天井石桌旁核對明日待調解的民事紀錄、與鄰居打招呼",
                ),
            },
        },
        // 37. wt_npc_lu_jingxuan (蒙學先生陸敬軒)
        CanonicalNpcSchedule {
            id: "wt_npc_lu_jingxuan",
            gender: "M",
            display_title: "蒙學先生|陸敬軒",
            resolve: |hour| match hour {
                8..=16 => (
                    "學院主校區",
                    "帶領文曜蒙學堂的學生朗讀《白塔基礎讀本》，在練習板上指導書寫",
                ),
                17..=19 => (
                    "藏書閣",
                    "在藏書閣中挑選適合學童閱讀的四季自然插畫圖鑑",
                ),
                _ => (
                    "講堂街・蒙學陸先生舍",
                    "在貝母檯燈下批改學生作業並指導助教整理教材",
                ),
            },
        },
        // 38. wt_npc_qin_mingcheng (百工核算師秦明誠)
        CanonicalNpcSchedule {
            id: "wt_npc_qin_mingcheng",
            gender: "M",
            display_title: "百工核算師|秦明誠",
            resolve: |hour| match hour {
                8..=17 => (
                    "百工商算",
                    "左手壓著出貨帳冊、右手操作隨身計數板，核算各工坊出貨成本與損耗",
                ),
                18..=20 => (
                    "三水大食堂",
                    "邊吃著蔥香米烙餅邊在記事本上核算下一季的採購鎂幣預算",
                ),
                _ => (
                    "講堂街・秦氏算學居",
                    "在書桌前檢視實習核算員提交的帳務試算表",
                ),
            },
        },
        // 39. wt_npc_lin_shizhong (傳訊鐘樓司鐘長林時中)
        CanonicalNpcSchedule {
            id: "wt_npc_lin_shizhong",
            gender: "M",
            display_title: "傳訊鐘樓司鐘長|林時中",
            resolve: |hour| match hour {
                6..=19 => (
                    "傳訊鐘樓與信使總站",
                    "在鐘樓瞭望台校對旋轉日規與報時青銅大鐘，並為整裝待發的快馬信使核驗防雨郵袋與各鎮收發印戳",
                ),
                20..=21 => (
                    "三水大食堂",
                    "在食堂喝著熱騰騰的原牛骨湯，與收工的快馬信使核對明日發往風舶角與老堰頭的早班信件",
                ),
                _ => (
                    "三水口・信使快騎宿所",
                    "在透鏡聚焦檯燈下保養隨身黃銅懷錶的發條齒輪",
                ),
            },
        },
        // 40. wt_npc_zhao_guanxing (學院觀星台司天掌度趙觀星)
        CanonicalNpcSchedule {
            id: "wt_npc_zhao_guanxing",
            gender: "M",
            display_title: "學院觀星台司天掌度|趙觀星",
            resolve: |hour| match hour {
                8..=21 => (
                    "學院觀星台與評議廳",
                    "站在青石度盤旁調整三百六十度星軌銅儀，記錄節氣日影與北山氣流變化，供學院評議廳編修年度曆譜",
                ),
                _ => (
                    "講堂街・石硯墨香居",
                    "在書桌前整理白天測得的星度交角與潮汐推算表",
                ),
            },
        },
        // 41. wt_npc_zhou_yidao (講堂街理髮老師傅周一刀)
        CanonicalNpcSchedule {
            id: "wt_npc_zhou_yidao",
            gender: "M",
            display_title: "講堂街理髮老師傅|周一刀",
            resolve: |hour| match hour {
                7..=19 => (
                    "講堂街老字號剪剃鋪",
                    "在磨刀布上俐落推磨精鋼摺疊剃刀，擰乾紫銅溫水管裡冒著熱氣的蜂格棉毛巾，笑著替街坊學生修剪頭髮與修面",
                ),
                20..=21 => (
                    "東橋旅社與山風茶屋",
                    "在茶屋喝著烘青茶，與街坊鄰居聊著三水口與講堂街的日常趣聞",
                ),
                _ => (
                    "講堂街・抄經校勘齋",
                    "用山茶防鏽油細心保養使用多年的精鋼剃刀與理髮剪刀",
                ),
            },
        },
        // 42. wt_npc_cen_yunye (分家巷武術館教練岑雲野)
        CanonicalNpcSchedule {
            id: "wt_npc_cen_yunye",
            gender: "M",
            display_title: "分家巷武術館教練|岑雲野",
            resolve: |hour| match hour {
                6..=17 => (
                    "習武堂",
                    "指導年輕學員站穩重心，手持包革練習木刀拆解防身卸力技巧",
                ),
                18..=20 => (
                    "泉香溫水浴場",
                    "泡在溫水池中放鬆筋骨，與韓鐵山聊著近來的訓練心得",
                ),
                _ => (
                    "分家巷・岑氏武勇舍",
                    "在武術館庭院與住宿學員進行對練與呼吸調整",
                ),
            },
        },
        // 43. wt_npc_xu_agan (分家巷柑仔店阿婆許阿甘)
        CanonicalNpcSchedule {
            id: "wt_npc_xu_agan",
            gender: "F",
            display_title: "分家巷柑仔店阿婆|許阿甘",
            resolve: |hour| match hour {
                6..=20 => (
                    "分家巷口雜貨柑仔店",
                    "坐在竹編藤椅上擦拭裝滿黑糖麥芽糖與鹽津梅餅的廣口玻璃罐，招呼下工的鐵匠學徒添購紅磷火籤與艾草蚊香盤",
                ),
                _ => (
                    "分家巷・精工火磚匠廬",
                    "在暖屋裡清點今日收進的零碎鎂幣，把新進的樟木防蟲丸分裝入袋",
                ),
            },
        },
        // 44. wt_npc_ma_buxue (西門修傘補靴老匠馬補靴)
        CanonicalNpcSchedule {
            id: "wt_npc_ma_buxue",
            gender: "M",
            display_title: "西門修傘補靴老匠|馬補靴",
            resolve: |hour| match hour {
                6..=18 => (
                    "西門車馬前街",
                    "在街邊支起老匠晴雨傘與皮靴修補攤，將磨損皮靴套上硬木楦頭釘補厚牛皮掌，並替韌竹骨晴雨傘刷上防水熟桐油",
                ),
                19..=20 => (
                    "三水大食堂",
                    "已收攤將修鞋工具箱挑到桌邊，端著一碗熱切仔湯麵配滷蛋，與車夫閒聊路況",
                ),
                _ => (
                    "西門・皮靴油布匠居",
                    "在煤油燈下揉製塗抹皮靴接縫用的蜂蠟松脂防水膏",
                ),
            },
        },
        // 45. wt_vendor_roast_beef (炭烤牛肉番薯攤東家鄭火旺)
        CanonicalNpcSchedule {
            id: "wt_vendor_roast_beef",
            gender: "M",
            display_title: "炭烤牛肉番薯攤東家|鄭火旺",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在炭烤牛肉番薯攤前翻動滋滋冒油的原牛肉串與流蜜紅心番薯，向往來行客高聲叫賣",
                ),
                14 => (
                    "三水口・石板弄堂天井",
                    "正在院裡將穿好的原牛肉串與洗淨紅心番薯裝上木輪攤車，準備推往三水口南街擺攤",
                ),
                _ => (
                    "三水口・石板弄堂天井",
                    "已收攤推車返家，擦淨鐵網與炭爐後在屋內安歇",
                ),
            },
        },
        // 46. wt_vendor_noodle_soup (現煮切仔麵攤女東家林罔市)
        CanonicalNpcSchedule {
            id: "wt_vendor_noodle_soup",
            gender: "F",
            display_title: "現煮切仔麵攤女東家|林罔市",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在現煮大骨切仔麵攤前笊籬起落、淋上濃白大骨湯與油蔥酥，熱情招呼夜歸人吃麵",
                ),
                14 => (
                    "三水口・陳氏庖廚居",
                    "正在灶前將熬足三個鐘頭的大骨高湯裝入保溫銅桶，準備推車前往三水口南街擺攤",
                ),
                _ => (
                    "三水口・陳氏庖廚居",
                    "已收攤推車返家，刷洗大湯鍋與碗筷後在屋內安歇",
                ),
            },
        },
        // 47. wt_vendor_pepper_bun (貼爐胡椒餅攤師傅張阿發)
        CanonicalNpcSchedule {
            id: "wt_vendor_pepper_bun",
            gender: "M",
            display_title: "貼爐胡椒餅攤師傅|張阿發",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在貼爐蔥香胡椒餅攤前用鐵夾從炭火鐵桶壁鏟下金黃酥脆的胡椒餅，大聲吆喝現出爐的熱餅",
                ),
                14 => (
                    "西門・草料客商居",
                    "正在揉製老麵、切妥滿滿的三星青蔥與黑胡椒豬肉餡，準備推車出門擺攤",
                ),
                _ => (
                    "西門・草料客商居",
                    "已收攤推車返家，封好烤桶餘燼後在屋內安歇",
                ),
            },
        },
        // 48. wt_vendor_herbal_tea (寒水玉冰鎮仙草凍攤主蘇秋月)
        CanonicalNpcSchedule {
            id: "wt_vendor_herbal_tea",
            gender: "F",
            display_title: "寒水玉冰鎮仙草凍攤主|蘇秋月",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在寒水玉冰鎮仙草凍攤前手搖銅盞叮噹作響，盛裝慢熬仙草凍與清涼百草飲招呼客人解膩",
                ),
                14 => (
                    "分家巷・蘇氏繡坊居",
                    "正在將文火慢熬的仙草乾汁凝凍裝入冰鎮陶甕，準備推車出門擺攤",
                ),
                _ => (
                    "分家巷・蘇氏繡坊居",
                    "已收攤推車返家，晾乾銅盞與陶甕後在屋內安歇",
                ),
            },
        },
        // 49. wt_vendor_tofu_pudding (黑糖粉圓豆花攤伯陳順興)
        CanonicalNpcSchedule {
            id: "wt_vendor_tofu_pudding",
            gender: "M",
            display_title: "黑糖粉圓豆花攤伯|陳順興",
            resolve: |hour| match hour {
                15..=23 | 0..=4 => (
                    "三水口南街",
                    "正在古早味黑糖粉圓豆花攤前用扁平銅杓片出綿密熱豆花、舀入Q彈黑糖粉圓與薑汁叫賣",
                ),
                14 => (
                    "三水口・晨汲青石宅",
                    "正在用古井泉水沖點鹽滷熱豆花、熬煮手炒黑糖粉圓，準備推車出門擺攤",
                ),
                _ => (
                    "三水口・晨汲青石宅",
                    "已收攤推車返家，浸泡明日用的飽滿黃豆後在屋內安歇",
                ),
            },
        },
        // 50. wt_vendor_rice_roll (紫米飯糰早點攤姨吳秀琴)
        CanonicalNpcSchedule {
            id: "wt_vendor_rice_roll",
            gender: "F",
            display_title: "紫米飯糰早點攤姨|吳秀琴",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現包紫米飯糰早點攤前掀開大木桶蒸氣，俐落盛裝荷葉糯米炊飯與老油條飯糰，招呼居民買早點",
                ),
                15..=17 => (
                    "三水口乾貨海味行",
                    "已收攤歇息，正在挑選明日炊飯用的乾荷葉、香脆菜脯與肉鬆",
                ),
                _ => (
                    "三水口・河畔迎客小院",
                    "已收攤返家，淘洗浸泡糯米與紫米後在屋內安歇",
                ),
            },
        },
        // 51. wt_vendor_soy_milk (現磨米漿杏仁茶攤叔周文貴)
        CanonicalNpcSchedule {
            id: "wt_vendor_soy_milk",
            gender: "M",
            display_title: "現磨米漿杏仁茶攤叔|周文貴",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現磨米漿杏仁茶攤前攪拌濃香滾燙的現磨溫米漿與杏仁茶，配著酥脆現炸油條高聲叫賣",
                ),
                15..=17 => (
                    "分家巷口雜貨柑仔店",
                    "已收攤歇息，正在添購烘烤過的新鮮北港花生與南杏仁",
                ),
                _ => (
                    "三水口・河畔迎客小院",
                    "已收攤返家，擦拭石磨與銅壺後在屋內安歇",
                ),
            },
        },
        // 52. wt_vendor_morning_produce (時令脆甜蔬果攤農李春生)
        CanonicalNpcSchedule {
            id: "wt_vendor_morning_produce",
            gender: "M",
            display_title: "時令脆甜蔬果攤農|李春生",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在清晨帶露時令蔬果攤前噴水整理新摘下的老堰頭水芹菜脆嫩莖與脆甜高麗菜，熱情招呼買菜居民",
                ),
                15..=17 => (
                    "三水大食堂",
                    "已收攤賣完今日鮮菜，正坐在食堂喝碗熱湯歇腳",
                ),
                _ => (
                    "分家巷・柯氏木作居",
                    "已收攤返家，捆紮明日要用的竹編菜簍後在屋內安歇",
                ),
            },
        },
        // 53. wt_vendor_morning_fish (現撈河鮮蛤蜊攤販許海生)
        CanonicalNpcSchedule {
            id: "wt_vendor_morning_fish",
            gender: "M",
            display_title: "現撈河鮮蛤蜊攤販|許海生",
            resolve: |hour| match hour {
                5..=14 => (
                    "三水口北街",
                    "正在現撈三水河鮮蛤蜊攤前給淺木盆換上清水，讓水尾寮淺澤青殼河蝦與大文蛤保鮮，大聲吆喝早市客人挑選",
                ),
                15..=17 => (
                    "三水口乾貨海味行",
                    "已收攤歇息，將剩餘鮮魚河蝦交給海味行鹽漬烘乾",
                ),
                _ => (
                    "三水口・老水夫舍",
                    "已收攤返家，修補魚網與木盆後在屋內安歇",
                ),
            },
        },
        // 54. 馬行頭 (wt_mahanger / 車馬行大行頭)
        CanonicalNpcSchedule {
            id: "馬行頭",
            gender: "M",
            display_title: "西麓車馬大行頭|馬行頭",
            resolve: |hour| match hour {
                6..=16 => (
                    "西門車馬聯運總站",
                    "在總站前調配出城前往西麓林道的大車，檢查韁繩、蹄鐵與路面車轍",
                ),
                17..=20 => (
                    "東橋旅社",
                    "在旅社大堂與各路車伕核對西麓林道與沿線物料運送回執",
                ),
                _ => (
                    "西門・韓氏總站宅院",
                    "在後院馬廄照料健壯草獸，整理明晨西出林道車隊裝備",
                ),
            },
        },
        // 55. 林山 (wt_linshan / 巡林獵戶)
        CanonicalNpcSchedule {
            id: "林山",
            gender: "M",
            display_title: "翠冠巨木巡林獵戶|林山",
            resolve: |hour| match hour {
                6..=16 => (
                    "水輪石磨工坊",
                    "在老堰頭石磨工坊周邊巡查巨木林獵徑與林木生長，排查野獸足跡與落葉堆",
                ),
                17..=20 => (
                    "堰渠水畔食堂",
                    "在水畔食堂大堂喝著熱茶，與過路獵人交流巨木林深處獸蹤情報",
                ),
                _ => (
                    "北嶺腳・馬家鐵齒登山釘鞋工坊宅",
                    "在屋內修整獵弓、皮靴與引路標記，記錄巡林心得",
                ),
            },
        },
        // 56. 雷橫山 (wt_leihengshan / 水澤機關匠人)
        CanonicalNpcSchedule {
            id: "雷橫山",
            gender: "M",
            display_title: "南哨水澤機關匠人|雷橫山",
            resolve: |hour| match hour {
                6..=17 => (
                    "重型獵具工坊",
                    "在工坊調校防腐捕獸夾與耐潮鐵樁，準備運往淺澤泥口與水澤邊緣防務",
                ),
                18..=20 => (
                    "南哨原野食堂",
                    "與巡邏守衛同桌用膳，盤算各處泥口獵具機關磨損情況",
                ),
                _ => (
                    "南哨・雷家重型獵具大匠宅",
                    "在油燈下保養防腐工具與滑輪索具，備足替換部件",
                ),
            },
        },
        // 57. 葉青 (wt_yeqing / 白茫白蘚採藥師)
        CanonicalNpcSchedule {
            id: "葉青",
            gender: "F",
            display_title: "百茫白蘚採藥師|葉青",
            resolve: |hour| match hour {
                6..=17 => (
                    "南哨脈絡解毒診所",
                    "在診所整理烘乾的白茫洲白蘚與藥草，調配清熱避瘴草藥包",
                ),
                18..=20 => (
                    "南哨原野食堂",
                    "在大堂分發避瘴藥茶，叮囑夜巡守衛防範水澤濕氣",
                ),
                _ => (
                    "南哨・葉家白茫避瘴清心齋",
                    "在藥室翻檢陰乾的鮮葉與藥膏，記錄草藥配伍比例",
                ),
            },
        },
        // 58. 田大犁 (wt_tiandali / 草甸改良農具匠師)
        CanonicalNpcSchedule {
            id: "田大犁",
            gender: "M",
            display_title: "草甸改良農具匠師|田大犁",
            resolve: |hour| match hour {
                6..=17 => (
                    "大圳頭・大圳農具與百工坊",
                    "在工坊鍛造改良型草甸翻土犁刀，測試鋼口韌度與犁架配重",
                ),
                18..=20 => (
                    "田園風味大食堂",
                    "與莊戶農夫切磋開墾草甸硬土的農具心得，記錄改進建議",
                ),
                _ => (
                    "大圳頭・田氏農具匠人老宅",
                    "在油燈下打磨農具樣模與齒輪配件，準備明日測試",
                ),
            },
        },
        // 59. 牧野 (wt_muye / 長風草甸巡牧官)
        CanonicalNpcSchedule {
            id: "牧野",
            gender: "M",
            display_title: "長風草甸巡牧官|牧野",
            resolve: |hour| match hour {
                6..=17 => (
                    "羊毛與乳酪精製坊",
                    "在精製坊協調長風草甸牛群放牧路線與草料調配，清點毛氈原料",
                ),
                18..=20 => (
                    "牧野歇腳旅社",
                    "在旅社大堂歇息喝熱奶茶，向往來商隊打聽草甸邊界水源狀況",
                ),
                _ => (
                    "牛埔・牧家牛埔硬乳酪發酵老宅",
                    "檢查草場圍欄與飲水坑備忘錄，確認草甸畜群安然入圈",
                ),
            },
        },
];

/// 於白晝時段（6..=20）調度西門遠征與商隊人物（岑拓、溫晴、馬行頭）前往城外西麓林道
pub fn dispatch_west_gate_expedition_npcs(store: &mut crate::store::Store) {
    let now_ts = crate::game::now_unix();
    let dispatches = [
        (
            "岑拓",
            "caravan_route",
            (-19, 0),
            (-22, 0),
            "西麓林道",
            "押著滿載鐵器與茶磚的雙輪貨車行進在西麓林道上，隨時留意路旁獸蹤",
            "西麓林道今日風乾路實，趁天色尚早把這批貨趕送出去，回程還能收些山貨",
            "stranded_cargo",
            "押送鐵器與茶磚商隊車巡視西麓林道",
        ),
        (
            "溫晴",
            "wilderness_expedition",
            (-20, 1),
            (-18, 0),
            "西麓林道",
            "背著竹編藥簍在西麓林道旁辨識新冒芽的光葉藥草與山茶籽",
            "百草堂裡的退熱草藥快見底了，趁今日無雨親自出西門採些鮮葉回去炮製",
            "restock_goods",
            "採集光葉藥草與山茶籽以備炮製",
        ),
        (
            "馬行頭",
            "caravan_route",
            (-18, 0),
            (-21, 0),
            "西麓林道",
            "在西城門外的西麓林道起點檢查駝獸蹄鐵與車轍路況",
            "出西門這段林道車轍頗深，得叮囑車夫們過坡時收緊韁繩",
            "repair_tool",
            "檢查西麓林道起點路況與駝獸蹄鐵車軛",
        ),
    ];

    for (id, goal, (sx, sy), (tx, ty), target_name, activity, thought, ob_kind, ob_desc) in dispatches {
        if get_canonical_npc_goal(id).is_some() {
            continue;
        }
        set_canonical_npc_goal(id, goal);
        CANONICAL_NPC_IDLE_TICKS.write().insert(id.to_string(), 0);
        CANONICAL_NPC_TARGETS.write().insert(
            id.to_string(),
            (tx, ty, target_name.to_string(), activity.to_string()),
        );
        crate::store::open_active_npc_obligation(id, ob_kind, ob_desc);

        if let Some(mut existing) = store.get_entity(id) {
            existing.grid_x = Some(sx);
            existing.grid_y = Some(sy);
            existing.target_x = Some(tx);
            existing.target_y = Some(ty);
            existing.current_activity = activity.to_string();
            crate::db::add_thought_to_entity_with_delta(
                &mut existing,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                thought.to_string(),
            );
            crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);
            let ent_sync = existing.clone();
            let _ = store.upsert_entity_full(existing);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(id, &format!("grid:{sx}:{sy}"));
        } else {
            let display_char = id.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "人".to_string());
            let mut entity = crate::store::Entity {
                id: id.to_string(),
                kind: "npc".to_string(),
                display_char,
                x: 0,
                y: 0,
                move_state: "idle".to_string(),
                target_x: Some(tx),
                target_y: Some(ty),
                walk_or_run: String::new(),
                move_started_at: None,
                vit: 100,
                qi: 100,
                dex: 50,
                magnesium: 500,
                last_observed_at: None,
                created_at: now_ts,
                gender: if id == "溫晴" { "F".to_string() } else { "M".to_string() },
                soul_seed: None,
                display_title: format!("|{}", id),
                activated_nodes: r#"["N000"]"#.to_string(),
                equipment_slots: "{}".to_string(),
                inventory: "[]".to_string(),
                disposition: 0,
                current_activity: activity.to_string(),
                place_id: None,
                grid_x: Some(sx),
                grid_y: Some(sy),
                thoughts: vec![],
            };
            crate::db::add_thought_to_entity_with_delta(
                &mut entity,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                thought.to_string(),
            );
            crate::db::clean_and_recalc_entity_disposition(&mut entity, now_ts);
            let ent_sync = entity.clone();
            let _ = store.upsert_entity_full(entity);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(id, &format!("grid:{sx}:{sy}"));
        }
    }
}

/// 依遊戲時刻將白塔半島 59 位核心正典人物同步至正方格大地圖對應建築，更新其作息目標。
static HOUSEHOLD_ENTITIES_SEEDED_ONCE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static LAST_SYNCED_CANONICAL_HOUR: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(-1);

pub fn sync_canonical_white_tower_npcs(game_hour: i32) {
    if LAST_SYNCED_CANONICAL_HOUR.swap(game_hour, std::sync::atomic::Ordering::SeqCst) == game_hour
        && HOUSEHOLD_ENTITIES_SEEDED_ONCE.load(std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let grid = match get_runtime_square_grid() {
        Some(g) => g,
        None => return,
    };

    crate::server::handler::inventory::ensure_flagship_specimen_qin_shuisheng_wordplate_initialized();
    let Some(store_arc) = crate::store::get_store() else {
        return;
    };
    let mut store = store_arc.write();
    crate::npc::causal_core::normalize_and_seed_circular_economy_once(&mut store);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 日間時段（6..=20）若有西麓商隊與採藥人物尚未出發，即時調度派駐至西城門外西麓林道
    if (6..=20).contains(&game_hour) {
        dispatch_west_gate_expedition_npcs(&mut store);
    }

    for def in &CANONICAL_NPC_SCHEDULES {
        if (def.id.starts_with("wt_vendor_") || def.id == "wt_npc_ma_buxue")
            && (game_hour == 5 || game_hour == 15 || game_hour == 23)
        {
            crate::npc::causal_core::enqueue_arrival_effect(
                crate::npc::causal_core::ArrivalEffect::RestockShopItems {
                    seller_id: def.id.to_string(),
                },
            );
        }
        let (bldg_name, mut activity) = (def.resolve)(game_hour);
        if !def.id.starts_with("wt_vendor_") && (game_hour >= 22 || game_hour < 5) {
            let is_night_watch = def.id == "雷千鈞"
                || def.display_title.contains("巡防")
                || def.display_title.contains("哨長")
                || def.display_title.contains("守塔")
                || def.display_title.contains("值守");
            if !is_night_watch {
                if !activity.contains("歇")
                    && !activity.contains("睡")
                    && !activity.contains("寢")
                    && !activity.contains("宿")
                {
                    activity = "正在內室榻上熄燈安歇";
                }
                clear_canonical_npc_goal(def.id);
            }
        }
        let cell = resolve_settlement_room_coord(&grid, bldg_name);
        let Some(cell) = cell else {
            tracing::warn!(
                "sync_canonical_white_tower_npcs: 找不到建築「{bldg_name}」，跳過人物「{}」之更新",
                def.id
            );
            continue;
        };
        let (x, y) = (cell.coord.x, cell.coord.y);

        if store.get_entity(def.id).is_some() {
            let (tx, ty, activity_owned) = crate::pg::living_world::step_canonical_resident_human_life_loop(
                &mut store,
                def.id,
                x,
                y,
                bldg_name,
                activity,
                game_hour as i32,
                crate::game::now_unix(),
            );
            let activity = activity_owned.as_str();
            let mut existing = store.get_entity(def.id).unwrap();
            existing.kind = "npc".to_string();
            // 保留既有體力/精力/敏捷與鎂幣數值，避免整點被清空或強制覆蓋
            if existing.vit <= 0 {
                existing.vit = 100;
            }
            if existing.qi <= 0 {
                existing.qi = 100;
            }
            if existing.dex <= 0 {
                existing.dex = 50;
            }
            if existing.magnesium < 0 {
                existing.magnesium = 0;
            }
            existing.place_id = None;
            existing.display_title = def.display_title.to_string();

            // 白晝與傍晚時段（6..=21）保護正典自主目標（荒野遠征、商隊行進、因果義務差事等），不被整點時刻重設打斷
            let is_autonomous_active = (6..=21).contains(&game_hour)
                && matches!(
                    existing.goal_kind().as_deref(),
                    Some("wilderness_expedition")
                        | Some("caravan_route")
                        | Some("obligation_errand")
                        | Some("autonomous_relief")
                        | Some("autonomous_livelihood")
                );
            if is_autonomous_active {
                let now_ts = crate::game::now_unix();
                crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);
                let ent_sync = existing.clone();
                let _ = store.upsert_entity_full(existing);
                crate::pg::entity::upsert_async(ent_sync);
                continue;
            }

            let is_on_duty_vendor = def.id.starts_with("wt_vendor_")
                && (bldg_name == "三水口南街" || bldg_name == "三水口北街");
            let cur_coord = (existing.grid_x, existing.grid_y);
            if let (Some(cur_x), Some(cur_y)) = cur_coord {
                if (cur_x, cur_y) == (tx, ty) || is_on_duty_vendor {
                    existing.grid_x = Some(tx);
                    existing.grid_y = Some(ty);
                    existing.current_activity = activity.to_string();
                    CANONICAL_NPC_TARGETS.write().remove(def.id);
                    let _ = store.move_entity(def.id, &format!("grid:{tx}:{ty}"));
                } else {
                    // 不論日間夜間皆不瞬移，寫入執行期目標表由 advance_canonical_npcs_one_step 逐格走！
                    let dest_bldg_name = grid
                        .get(crate::grid::SquareCoord::new(tx, ty))
                        .map(|c| c.name.as_str())
                        .unwrap_or(bldg_name);
                    CANONICAL_NPC_TARGETS.write().insert(
                        def.id.to_string(),
                        (tx, ty, dest_bldg_name.to_string(), activity.to_string()),
                    );
                }
            } else {
                existing.grid_x = Some(tx);
                existing.grid_y = Some(ty);
                existing.current_activity = activity.to_string();
                let _ = store.move_entity(def.id, &format!("grid:{tx}:{ty}"));
            }

            // Prune stale broken/daily thoughts and provide fresh character-specific thought
            let now_ts = crate::game::now_unix();
            existing.thoughts.retain(|t| {
                let f = t.flavor.as_deref().unwrap_or("");
                !f.contains("鎂袋空了")
                    && !f.contains("今日花用又見底")
                    && !f.contains("記掛著「內室榻上熄燈安歇」")
                    && !f.contains("記掛著『診所抓藥』")
                    && !f.contains("記掛著『手頭差事』")
            });
            if existing.thoughts.is_empty() {
                let init_flavor = initial_thought_for_npc(def.id, &def.display_title);
                crate::db::add_thought_to_entity_with_delta(
                    &mut existing,
                    crate::db::ThoughtSource::WorkDone,
                    8,
                    now_ts,
                    init_flavor,
                );
            }
            crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);

            let ent_sync = existing.clone();
            let _ = store.upsert_entity_full(existing);
            crate::pg::entity::upsert_async(ent_sync);
        } else {
            let display_char = def
                .display_title
                .rsplit('|')
                .next()
                .unwrap_or(def.id)
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "人".to_string());
            let init_flavor = initial_thought_for_npc(def.id, def.display_title);
            let mut entity = crate::store::Entity {
                id: def.id.to_string(),
                kind: "npc".to_string(),
                display_char,
                x: 0,
                y: 0,
                move_state: "idle".to_string(),
                target_x: None,
                target_y: None,
                walk_or_run: String::new(),
                move_started_at: None,
                vit: 100,
                qi: 100,
                dex: 50,
                magnesium: 500,
                last_observed_at: None,
                created_at: now,
                gender: def.gender.to_string(),
                soul_seed: None,
                display_title: def.display_title.to_string(),
                activated_nodes: r#"["N000"]"#.to_string(),
                equipment_slots: "{}".to_string(),
                inventory: "[]".to_string(),
                disposition: 0,
                current_activity: activity.to_string(),
                place_id: None,
                grid_x: Some(x),
                grid_y: Some(y),
                thoughts: vec![],
            };
            let now_ts = crate::game::now_unix();
            crate::db::add_thought_to_entity_with_delta(
                &mut entity,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                init_flavor,
            );
            crate::db::clean_and_recalc_entity_disposition(&mut entity, now_ts);
            let ent_sync = entity.clone();
            let _ = store.upsert_entity_full(entity);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(def.id, &format!("grid:{x}:{y}"));
        }
    }
    if !HOUSEHOLD_ENTITIES_SEEDED_ONCE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        store.ensure_all_household_entities_seeded();
    }
}

fn find_settlement_secondary_building<'a>(
    grid: &'a crate::grid::SquareGrid,
    (cx, cy): (i32, i32),
    query: &str,
) -> Option<&'a crate::grid::GridCell> {
    let cur_cell = grid.get(crate::grid::SquareCoord::new(cx, cy));
    let cur_zone = cur_cell.map(|c| c.zone.as_str()).unwrap_or("");
    let cur_prefix = cur_cell
        .and_then(|c| c.name.split('・').next())
        .unwrap_or("");

    let is_candidate = |c: &&crate::grid::GridCell| {
        (c.coord.x != cx || c.coord.y != cy)
            && !matches!(
                c.terrain,
                crate::grid::Terrain::Road
                    | crate::grid::Terrain::Bridge
                    | crate::grid::Terrain::Wall
            )
            && c.name.contains(query)
            && (
                (!cur_zone.is_empty() && c.zone == cur_zone)
                    || (!cur_prefix.is_empty() && c.name.starts_with(cur_prefix))
                    || (c.coord.x - cx).abs() + (c.coord.y - cy).abs() <= 20
            )
    };

    grid.cells().filter(is_candidate).min_by_key(|c| {
        (c.coord.x - cx).abs() + (c.coord.y - cy).abs()
    })
}

fn step_canonical_npc_idles(grid: &crate::grid::SquareGrid) {
    let hour = crate::store::current_game_hour();
    if !(6..=21).contains(&hour) {
        CANONICAL_NPC_IDLE_TICKS.write().clear();
        return;
    }

    let Some(store_arc) = crate::store::get_store() else {
        return;
    };

    let active_targets = {
        let r = CANONICAL_NPC_TARGETS.read();
        r.keys().cloned().collect::<std::collections::HashSet<String>>()
    };

    for def in &CANONICAL_NPC_SCHEDULES {
        let id = def.id;
        if def.display_title.contains("水輪") || def.display_title.contains("石磨") {
            continue;
        }
        if active_targets.contains(id) {
            CANONICAL_NPC_IDLE_TICKS.write().remove(id);
            continue;
        }

        // 攤販業者由正典時刻表嚴格控管擺攤、備料、採買與返家時段，跳過隨機次級建築漫遊
        if id.starts_with("wt_vendor_") || id == "wt_npc_ma_buxue" {
            let is_vending = if id.starts_with("wt_vendor_") {
                true
            } else {
                (6..=18).contains(&hour)
            };
            if is_vending {
                continue;
            }
        }

        let ticks = {
            let mut w = CANONICAL_NPC_IDLE_TICKS.write();
            let entry = w.entry(id.to_string()).or_insert(0);
            *entry += 1;
            *entry
        };
        let active_goal = get_canonical_npc_goal(id);
        let is_on_expedition = matches!(
            active_goal.as_deref(),
            Some("wilderness_expedition") | Some("caravan_route")
        );

        if is_on_expedition {
            if ticks < 3 {
                continue;
            }
            CANONICAL_NPC_IDLE_TICKS.write().insert(id.to_string(), 0);
            // 抵達荒野目標格停留採集/狩獵/商務達標（idle_ticks >= 3），紀錄 WorkDone 心聲並動身返回工坊/主宅
            let mut harvested_name: Option<String> = None;
            let mut harvested_item_id: Option<&'static str> = None;
            let now_ts = crate::game::now_unix();

            // 自當前荒野單元格扣減首個可採集資源（Flora 或 Mineral）
            let ent_coord = {
                let s = store_arc.read();
                s.get_entity(id).and_then(|e| match (e.grid_x, e.grid_y) {
                    (Some(x), Some(y)) => Some((x, y)),
                    _ => None,
                })
            };

            if let Some((gx, gy)) = ent_coord {
                let cell_update = with_square_grid_mut_in_memory(|g| {
                    if let Some(cell) = g.get_mut(SquareCoord::new(gx, gy)) {
                        for obj in &mut cell.objects {
                            if obj.quantity > 0
                                && matches!(obj.kind, GridObjectKind::Flora | GridObjectKind::Mineral)
                            {
                                obj.quantity = obj.quantity.saturating_sub(1);
                                let obj_id = obj.id.clone();
                                let obj_name = obj.name.clone();
                                let item_id = if obj_id.starts_with("wild_fruit")
                                    || obj_name.contains("漿果")
                                    || obj_name.contains("果")
                                    || obj_name.contains("山茶籽")
                                    || obj_name.contains("燕窩")
                                {
                                    "harvest_berries"
                                } else if obj_id.starts_with("wood")
                                    || obj_name.contains("木材")
                                    || obj_name.contains("原木")
                                    || obj_name.contains("枯木")
                                    || obj_name.contains("倒木")
                                    || obj_name.contains("心材")
                                    || obj_name.contains("良材")
                                    || obj_name.contains("雲杉")
                                    || obj_name.contains("烏木")
                                    || obj_name.contains("韌竹")
                                    || obj_name.contains("鐵杉")
                                    || obj_name.contains("穀木")
                                    || obj_name.contains("樹脂")
                                {
                                    if obj_name.contains("溫木") || obj_name.contains("烏木") || obj_name.contains("心材") || obj_name.contains("樹脂") {
                                        "harvest_rare_wood"
                                    } else if obj_name.contains("雲杉") || obj_name.contains("原木") {
                                        "harvest_wood"
                                    } else {
                                        "harvest_deadwood"
                                    }
                                } else if obj_id.starts_with("herb")
                                    || obj_name.contains("草")
                                    || obj_name.contains("苔")
                                    || obj_name.contains("蓮")
                                    || obj_name.contains("藤")
                                    || obj_name.contains("葦")
                                    || obj_name.contains("百草")
                                {
                                    if obj_name.contains("藤") {
                                        "harvest_vine"
                                    } else {
                                        "harvest_herb"
                                    }
                                } else if obj_id.starts_with("ore")
                                    || obj_name.contains("礦")
                                    || obj_name.contains("玉")
                                    || obj_name.contains("石")
                                    || obj_name.contains("岩")
                                    || obj_name.contains("晶")
                                    || obj_name.contains("髓")
                                    || obj_name.contains("貝")
                                    || obj_name.contains("砂")
                                    || obj_name.contains("黏土")
                                    || obj_name.contains("鹽")
                                {
                                    if obj_name.contains("玉") || obj_name.contains("晶") || obj_name.contains("髓") {
                                        "harvest_crystal"
                                    } else if obj_name.contains("鐵") || obj_name.contains("銅") || obj_name.contains("礦") || obj_name.contains("煤") {
                                        "harvest_ore"
                                    } else {
                                        "harvest_hardstone"
                                    }
                                } else {
                                    "harvest_herb"
                                };
                                let snapshot = cell.clone();
                                g.mark_dirty();
                                return Some((obj_name, item_id, snapshot));
                            }
                        }
                    }
                    None
                });

                if let Some(Some((name, it_id, cell_snapshot))) = cell_update {
                    let _ = crate::db::update_cell_state(
                        cell_snapshot.coord.x,
                        cell_snapshot.coord.y,
                        cell_snapshot.explored,
                        cell_snapshot.freshness,
                        &cell_snapshot.objects,
                    );
                    harvested_name = Some(name);
                    harvested_item_id = Some(it_id);
                }
            }

            if let Some(item_id) = harvested_item_id {
                let _ = crate::db::add_to_inventory(id, item_id, 1);
            }

            let return_flavor = if let Some(h_name) = &harvested_name {
                format!("在荒野採集齊備了「{}」，妥善收入囊中，備妥返程", h_name)
            } else if let Some(exp_def) = find_canonical_expedition(id) {
                exp_def.return_thought.to_string()
            } else {
                "在城外完成了既定事務，正準備動身返回城內".to_string()
            };

            {
                let mut s = store_arc.write();
                crate::npc::causal_core::pay_from_settlement_treasury(&mut s, id, "文曜", 12);
                if let Some(mut ent) = s.get_entity(id) {
                    crate::db::add_thought_to_entity_with_delta(
                        &mut ent,
                        crate::db::ThoughtSource::WorkDone,
                        8,
                        now_ts,
                        return_flavor,
                    );
                    crate::db::clean_and_recalc_entity_disposition(&mut ent, now_ts);
                    let ent_sync = ent.clone();
                    let _ = s.upsert_entity_full(ent);
                    crate::pg::entity::upsert_async(ent_sync);
                }
            }
            if let Some((ob_kind, _)) = crate::store::get_active_npc_obligation(id) {
                crate::store::clear_active_npc_obligation(id, &ob_kind);
            }
            clear_canonical_npc_goal(id);

            let (prim_bldg, prim_act) = (def.resolve)(hour);
            let prim_cell = resolve_settlement_room_coord(grid, prim_bldg);
            if let Some(pc) = prim_cell {
                CANONICAL_NPC_TARGETS.write().insert(
                    id.to_string(),
                    (pc.coord.x, pc.coord.y, prim_bldg.to_string(), prim_act.to_string()),
                );
            }
            continue;
        }


        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for b in id.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let threshold = 25 + ((h % 16) as u32); // 25..=40 秒

        if ticks < threshold {
            continue;
        }

        CANONICAL_NPC_IDLE_TICKS.write().insert(id.to_string(), 0);

        let (prim_bldg, prim_act) = (def.resolve)(hour);
        let prim_cell = resolve_settlement_room_coord(grid, prim_bldg);

        let cur_coord = {
            let s = store_arc.read();
            s.get_entity(id).and_then(|e| match (e.grid_x, e.grid_y) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => None,
            })
        };
        let Some((cx, cy)) = cur_coord else {
            continue;
        };

        let at_primary = if let Some(pc) = prim_cell {
            (pc.coord.x, pc.coord.y) == (cx, cy)
        } else {
            false
        };

        if !at_primary {
            // 目前正在次級目標或義務場所，停留結束後結清義務並走回原駐點
            {
                let mut s = store_arc.write();
                crate::npc::causal_core::pay_from_settlement_treasury(&mut s, id, "文曜", 12);
                if let Some(mut ent) = s.get_entity(id) {
                    crate::db::clean_and_recalc_entity_disposition(&mut ent, crate::game::now_unix());
                    let ent_sync = ent.clone();
                    let _ = s.upsert_entity_full(ent);
                    crate::pg::entity::upsert_async(ent_sync);
                }
            }
            if let Some((ob_kind, _)) = crate::store::get_active_npc_obligation(id) {
                crate::store::clear_active_npc_obligation(id, &ob_kind);
            }
            if let Some(pc) = prim_cell {
                CANONICAL_NPC_TARGETS.write().insert(
                    id.to_string(),
                    (pc.coord.x, pc.coord.y, prim_bldg.to_string(), prim_act.to_string()),
                );
            }
        } else {
            // 目前正在原駐點
            // 1. 若該人物為正典遠征/巡獵/商隊人物且時刻在日間（6..=20），啟動荒野巡獵或商隊遠征出巡
            if (6..=20).contains(&hour) && let Some(exp_def) = find_canonical_expedition(id) {
                set_canonical_npc_goal(id, exp_def.goal_kind);
                crate::store::open_active_npc_obligation(id, exp_def.obligation_kind, exp_def.obligation_desc);

                let now_ts = crate::game::now_unix();
                {
                    let mut s = store_arc.write();
                    if let Some(mut ent) = s.get_entity(id) {
                        crate::db::add_thought_to_entity_with_delta(
                            &mut ent,
                            crate::db::ThoughtSource::WorkDone,
                            8,
                            now_ts,
                            exp_def.thought.to_string(),
                        );
                        crate::db::clean_and_recalc_entity_disposition(&mut ent, now_ts);
                        let ent_sync = ent.clone();
                        let _ = s.upsert_entity_full(ent);
                        crate::pg::entity::upsert_async(ent_sync);
                    }
                }
                CANONICAL_NPC_TARGETS.write().insert(
                    id.to_string(),
                    (
                        exp_def.target_coord.0,
                        exp_def.target_coord.1,
                        exp_def.target_name.to_string(),
                        exp_def.activity.to_string(),
                    ),
                );
                continue;
            }

            // 2. 檢查待辦因果義務或指派同聚落次級目標
            let active_ob = crate::store::get_active_npc_obligation(id);
            if let Some((ob_kind, _ob_subject)) = active_ob {
                let (target_query, target_act) = match ob_kind.as_str() {
                    "repair_tool" => ("工坊", "正在工坊請匠師修檢磨損器物與工具"),
                    "restock_goods" => ("商行", "正在清點並提領新運抵的補充物料"),
                    "treat_injury" => ("診所", "正在敷用活血草藥調理筋骨勞損"),
                    "repay_debt" => ("總商行", "正在商行櫃台憑摺核銷往來貨單"),
                    _ => ("商行", "正在前往料理手頭待辦事務"),
                };
                let secondary_cell = find_settlement_secondary_building(grid, (cx, cy), target_query)
                    .or_else(|| resolve_settlement_room_coord(grid, target_query));
                if let Some(sc) = secondary_cell {
                    CANONICAL_NPC_TARGETS.write().insert(
                        id.to_string(),
                        (sc.coord.x, sc.coord.y, sc.name.clone(), target_act.to_string()),
                    );
                }
            } else {
                let queries = ["食堂", "茶屋", "工坊", "商行", "聯運", "貨棧", "書舍"];
                let q_idx = ((h.wrapping_add(hour as u64)) as usize) % queries.len();
                let query = queries[q_idx];
                let secondary_cell = find_settlement_secondary_building(grid, (cx, cy), query)
                    .or_else(|| {
                        queries.iter().find_map(|q| find_settlement_secondary_building(grid, (cx, cy), q))
                    });
                if let Some(sc) = secondary_cell {
                    let venue_obj = crate::grid::resolve_room_venue_object(&sc.name, "")
                        .or_else(|| crate::grid::resolve_market_or_specialty_room_object(&sc.name, ""));
                    let act = if let Some(v) = venue_obj {
                        let vk = v.venue_kind;
                        if vk.contains("食堂") || vk.contains("食肆") || vk.contains("餐") || vk.contains("麵") {
                            "正在食堂用餐小憩並品嚐熱湯餐點"
                        } else if vk.contains("茶") || vk.contains("旅社") || vk.contains("客") || vk.contains("酒") {
                            "正在茶屋小憩並品嚐熱茶糕點"
                        } else if vk.contains("商行") || vk.contains("聯運") || vk.contains("貨棧") || vk.contains("海事") || vk.contains("車馬") || vk.contains("市集") {
                            "正在巡視貨棧物料與往來車馬商情"
                        } else if vk.contains("工坊") || vk.contains("鍛造") || vk.contains("鐵") || vk.contains("木作") || vk.contains("石磨") || vk.contains("造船") || vk.contains("修船") || vk.contains("窯") {
                            "正在察看工坊匠人營造進度與器物修檢"
                        } else if vk.contains("醫館") || vk.contains("藥") || vk.contains("診所") {
                            "正在醫館問診調息並檢視道地藥材"
                        } else if vk.contains("浴") || vk.contains("溫泉") {
                            "正在溫水浴場沐浴調息、舒緩筋骨"
                        } else if vk.contains("巡防") || vk.contains("哨所") || vk.contains("武") {
                            "正在巡防哨所察看周遭防務動向"
                        } else if vk.contains("書") || vk.contains("講堂") || vk.contains("學") {
                            "正在典籍書舍翻閱卷宗並觀摩研習"
                        } else if sc.name.contains("食堂") {
                            "正在食堂用餐小憩並品嚐熱湯餐點"
                        } else if sc.name.contains("茶屋") || sc.name.contains("茶寮") {
                            "正在茶屋小憩並品嚐熱茶糕點"
                        } else if sc.name.contains("商行") || sc.name.contains("聯運") || sc.name.contains("貨棧") {
                            "正在巡視貨棧物料與往來車馬商情"
                        } else if sc.name.contains("工坊") || sc.name.contains("鍛造") {
                            "正在察看工坊匠人營造進度與器物修檢"
                        } else {
                            "正在周遭街坊中場巡視並察看市面商情"
                        }
                    } else if sc.name.contains("食堂") {
                        "正在食堂用餐小憩並品嚐熱湯餐點"
                    } else if sc.name.contains("茶屋") || sc.name.contains("茶寮") {
                        "正在茶屋小憩並品嚐熱茶糕點"
                    } else if sc.name.contains("商行") || sc.name.contains("聯運") || sc.name.contains("貨棧") {
                        "正在巡視貨棧物料與往來車馬商情"
                    } else if sc.name.contains("工坊") || sc.name.contains("鍛造") {
                        "正在察看工坊匠人營造進度與器物修檢"
                    } else {
                        "正在周遭街坊中場巡視並察看市面商情"
                    };
                    CANONICAL_NPC_TARGETS.write().insert(
                        id.to_string(),
                        (sc.coord.x, sc.coord.y, sc.name.clone(), act.to_string()),
                    );
                }
            }
        }
    }
}

/// 遍歷 CANONICAL_NPC_TARGETS，對尚未抵達目標建築的正典人物，在 SquareGrid 上以 BFS 邁進 1 格，
/// 並回傳所有發生移動的 (npc_id, from_room_id, to_room_id) 清單，供推播給在場玩家。
pub fn advance_canonical_npcs_one_step() -> Vec<(String, String, String)> {
    let grid = match get_runtime_square_grid() {
        Some(g) => g,
        None => return Vec::new(),
    };
    let Some(store_arc) = crate::store::get_store() else {
        return Vec::new();
    };

    // 日間（06:00..21:59）正典人物停留節拍與中場巡視／因果義務驅動
    step_canonical_npc_idles(&grid);

    let targets: Vec<(String, (i32, i32, String, String))> = {
        CANONICAL_NPC_TARGETS
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    };

    if targets.is_empty() {
        return Vec::new();
    }

    let mut moves = Vec::new();
    let mut finished = Vec::new();

    for (id, (target_x, target_y, bldg_name, target_activity)) in targets {
        if crate::store::is_npc_paused_for_interaction(&id) {
            continue;
        }
        let cur_coords = {
            let store = store_arc.read();
            let Some(ent) = store.get_entity(&id) else {
                finished.push(id);
                continue;
            };
            (ent.grid_x, ent.grid_y)
        };

        let (Some(cur_x), Some(cur_y)) = cur_coords else {
            let to_room_id = format!("grid:{target_x}:{target_y}");
            let place_id = crate::world::place::legacy_place_id(crate::grid::SquareCoord::new(target_x, target_y));
            {
                let mut store = store_arc.write();
                if let Some(ent) = store.entities.get_mut(&id) {
                    ent.grid_x = Some(target_x);
                    ent.grid_y = Some(target_y);
                    ent.target_x = None;
                    ent.target_y = None;
                    ent.place_id = Some(place_id.clone());
                    ent.current_activity = target_activity.clone();
                }
                store.entity_rooms.insert(id.clone(), to_room_id.clone());
            }
            if crate::pg::writer::is_active() {
                crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                    entity_id: id.clone(),
                    place_id,
                    grid_x: Some(target_x),
                    grid_y: Some(target_y),
                    room_id: to_room_id,
                });
                crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityActivity {
                    entity_id: id.clone(),
                    activity: target_activity.clone(),
                });
            }
            finished.push(id);
            continue;
        };

        if (cur_x, cur_y) == (target_x, target_y) {
            let hour = crate::store::current_game_hour();
            let final_act = {
                let mut store = store_arc.write();
                if let Some(ent) = store.entities.get_mut(&id) {
                    ent.target_x = None;
                    ent.target_y = None;
                    ent.current_activity = target_activity.clone();
                }
                let (_, _, act) = crate::pg::living_world::step_canonical_resident_human_life_loop(
                    &mut store,
                    &id,
                    target_x,
                    target_y,
                    &bldg_name,
                    &target_activity,
                    hour as i32,
                    crate::game::now_unix(),
                );
                act
            };
            if crate::pg::writer::is_active() {
                crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityActivity {
                    entity_id: id.clone(),
                    activity: final_act,
                });
            }
            finished.push(id);
            continue;
        }
        let hour = crate::store::current_game_hour();

        // 以 BFS 尋找往目標座標邁進的最短可通行路徑第一步；尋路失敗時嚴禁穿牆瞬移（不在持有寫鎖時執行）
        let next_step = crate::store::find_bfs_next_step(&grid, (cur_x, cur_y), (target_x, target_y));
        let Some((nx, ny)) = next_step else {
            finished.push(id);
            continue;
        };

        let from_room_id = crate::grid::grid_room_id_from_coord(cur_x, cur_y);
        let to_room_id = crate::grid::grid_room_id_from_coord(nx, ny);
        let place_id = crate::world::place::legacy_place_id(crate::grid::SquareCoord::new(nx, ny));

        let is_expedition = matches!(
            get_canonical_npc_goal(&id).as_deref(),
            Some("wilderness_expedition") | Some("caravan_route")
        );

        let new_activity = if (nx, ny) == (target_x, target_y) || is_expedition {
            if (nx, ny) == (target_x, target_y) {
                finished.push(id.clone());
            }
            target_activity.clone()
        } else if target_activity.contains("前往")
            || target_activity.contains("返回")
            || target_activity.contains("趕往")
            || target_activity.contains("步向")
        {
            target_activity.clone()
        } else if id.starts_with("wt_vendor_") {
            let display_bldg_name = bldg_name.replace("大圳頭・西南原", "大圳頭・圳頭").replace("西門外坊・西南原", "西門外坊・西門").replace("西南原", "大圳頭").replace("防爆防風", "防風");
            if bldg_name.contains("街") {
                format!("正推著攤車前往【{}】準備擺攤營業", display_bldg_name)
            } else {
                format!("已收攤，正推著攤車返回【{}】歇息", display_bldg_name)
            }
        } else if (hour >= 22 || hour < 5) && !target_activity.contains("值守") {
            let display_bldg_name = bldg_name.replace("大圳頭・西南原", "大圳頭・圳頭").replace("西門外坊・西南原", "西門外坊・西門").replace("西南原", "大圳頭").replace("防爆防風", "防風");
            format!("正踏著夜色沿街返回【{}】準備安歇", display_bldg_name)
        } else {
            let display_bldg_name = bldg_name.replace("大圳頭・西南原", "大圳頭・圳頭").replace("西門外坊・西南原", "西門外坊・西門").replace("西南原", "大圳頭").replace("防爆防風", "防風");
            format!(
                "正動身前往{}，準備{}",
                display_bldg_name,
                target_activity.trim_start_matches("正在")
            )
        };

        // 短暫取得 store_arc.write() 只更新記憶體中的 grid_x, grid_y, current_activity 與 entity_rooms
        let mut final_activity = new_activity;
        let mut arrived_player = false;
        {
            let mut store = store_arc.write();
            let is_player = store
                .entities
                .get(&id)
                .map(|e| e.kind == "player" || e.kind == "Player")
                .unwrap_or(false);
            if let Some(ent) = store.entities.get_mut(&id) {
                ent.grid_x = Some(nx);
                ent.grid_y = Some(ny);
                ent.target_x = if (nx, ny) == (target_x, target_y) { None } else { Some(target_x) };
                ent.target_y = if (nx, ny) == (target_x, target_y) { None } else { Some(target_y) };
                ent.place_id = Some(place_id.clone());
                ent.current_activity = final_activity.clone();
            }
            store.entity_rooms.insert(id.clone(), to_room_id.clone());
            if (nx, ny) == (target_x, target_y) {
                if !is_player {
                    let (_, _, act) = crate::pg::living_world::step_canonical_resident_human_life_loop(
                        &mut store,
                        &id,
                        target_x,
                        target_y,
                        &bldg_name,
                        &target_activity,
                        hour as i32,
                        crate::game::now_unix(),
                    );
                    final_activity = act;
                } else {
                    arrived_player = true;
                }
            }
        }

        if arrived_player {
            let _ = crate::npc::causal_core::step_offline_player_once(&id, crate::game::now_unix());
        }

        if crate::pg::writer::is_active() {
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityPlace {
                entity_id: id.clone(),
                place_id,
                grid_x: Some(nx),
                grid_y: Some(ny),
                room_id: to_room_id.clone(),
            });
            crate::pg::writer::submit(crate::pg::writer::WriteOp::SetEntityActivity {
                entity_id: id.clone(),
                activity: final_activity,
            });
        }

        if from_room_id != to_room_id {
            moves.push((id.clone(), from_room_id, to_room_id));
        }
    }

    if !finished.is_empty() {
        let mut w = CANONICAL_NPC_TARGETS.write();
        for id in finished {
            w.remove(&id);
        }
    }

    moves
}

/// 依建築名稱或關鍵字在正方格地圖上查找最匹配的房間格（優先精確匹配，次選子字串匹配）。
pub fn resolve_settlement_room_coord<'a>(
    grid: &'a crate::grid::SquareGrid,
    bldg_query: &str,
) -> Option<&'a crate::grid::GridCell> {
    // 0. 核心街道與橋樑座標之 O(1) 直查
    match bldg_query {
        "三水匯流大石橋" | "中心樞紐石橋" => return grid.get(crate::grid::SquareCoord::new(0, 0)),
        "三水口北街" => return grid.get(crate::grid::SquareCoord::new(0, 1)),
        "三水口南街" => return grid.get(crate::grid::SquareCoord::new(0, -1)),
        "三水東街" | "三水口東街" => return grid.get(crate::grid::SquareCoord::new(1, 0)),
        "三水口西街" => return grid.get(crate::grid::SquareCoord::new(-1, 0)),
        "西門車馬前街" => return grid.get(crate::grid::SquareCoord::new(-3, 0)),
        "講堂街西口" | "講堂街" => return grid.get(crate::grid::SquareCoord::new(1, 0)),
        "西門東街" => return grid.get(crate::grid::SquareCoord::new(-1, 0)),
        "北津石板拱橋" => return grid.get(crate::grid::SquareCoord::new(0, 3)),
        "鐘樓前街" => return grid.get(crate::grid::SquareCoord::new(0, 2)),
        "公議堂前街" => return grid.get(crate::grid::SquareCoord::new(-1, -1)),
        "西麓林道・道路" | "西麓林道" => return grid.get(crate::grid::SquareCoord::new(-18, 0)),
        "翠冠巨木林" => return grid.get(crate::grid::SquareCoord::new(0, 18)),
        "清泉溪谷" => return grid.get(crate::grid::SquareCoord::new(60, 160)),
        "北坡雲杉林" => return grid.get(crate::grid::SquareCoord::new(80, 260)),
        "淺澤泥口" => return grid.get(crate::grid::SquareCoord::new(0, -340)),
        "白茫洲" => return grid.get(crate::grid::SquareCoord::new(120, -320)),
        "長風草甸" => return grid.get(crate::grid::SquareCoord::new(-180, -195)),
        _ => {}
    }

    let want_road_or_bridge = bldg_query.contains('橋') || bldg_query.contains('街') || bldg_query.contains('路') || bldg_query.contains('道');
    let is_bldg_cell = |c: &&crate::grid::GridCell| {
        want_road_or_bridge
            || !matches!(
                c.terrain,
                crate::grid::Terrain::Road
                    | crate::grid::Terrain::Bridge
                    | crate::grid::Terrain::Wall
            )
    };
    // 1. 完全或前綴/包含精確匹配（優先建築格，排除街道誤中「公議堂前街」等）
    if let Some(cell) = grid.cells().filter(is_bldg_cell).find(|c| c.name == bldg_query || c.name.ends_with(&format!("・{bldg_query}"))) {
        return Some(cell);
    }
    if let Some(cell) = grid.cells().filter(is_bldg_cell).find(|c| c.name.contains(bldg_query)) {
        return Some(cell);
    }
    let leaf = bldg_query.rsplit('・').next().unwrap_or(bldg_query);
    if !leaf.is_empty()
        && leaf != bldg_query
        && let Some(cell) = grid.cells().filter(is_bldg_cell).find(|c| c.name.contains(leaf))
    {
        return Some(cell);
    }
    let query_no_dot = bldg_query.replace('・', "");
    if !query_no_dot.is_empty()
        && let Some(cell) = grid.cells().filter(is_bldg_cell).find(|c| c.name.replace('・', "").contains(&query_no_dot))
    {
        return Some(cell);
    }
    // 2. 關鍵字別名與館舍分群匹配
    let is_residence_query = bldg_query.contains("宅")
        || bldg_query.contains("居")
        || bldg_query.contains("舍")
        || bldg_query.contains("宿")
        || bldg_query.contains("院")
        || bldg_query.contains("齋")
        || bldg_query.contains("廬")
        || bldg_query.contains("寮");
    let aliases: &[&str] = if bldg_query.contains("公議堂") {
        &["公議堂", "房地契"]
    } else if bldg_query.contains("藏書閣") {
        &["藏書閣", "風物誌"]
    } else if bldg_query.contains("鐘樓") {
        &["傳訊鐘樓", "鐘樓", "信使"]
    } else if bldg_query.contains("學院主校區") || bldg_query.contains("文曜學院") {
        &["學院主校區", "文曜學院", "學院"]
    } else if bldg_query.contains("三水大食堂") || bldg_query.contains("食堂") {
        &["三水大食堂", "食堂"]
    } else if bldg_query.contains("泉香溫水浴場") || bldg_query.contains("浴場") {
        &["泉香溫水浴場", "浴場"]
    } else if bldg_query.contains("東橋旅社") || bldg_query.contains("旅社") {
        &["東橋旅社", "茶屋", "旅社"]
    } else if bldg_query.contains("巧木居所") || bldg_query.contains("巧木") {
        &["巧木居所", "傢俱坊", "巧木"]
    } else if bldg_query.contains("雲錦織造") || bldg_query.contains("雲錦") {
        &["雲錦織造", "服飾坊", "雲錦"]
    } else if bldg_query.contains("百工商算") || bldg_query.contains("商算") {
        &["商算學舍", "百工商算", "算學"]
    } else if bldg_query.contains("習武堂") || bldg_query.contains("試招") {
        &["習武堂", "試招院", "試招"]
    } else if bldg_query.contains("三水匯流大石橋") || bldg_query.contains("中心樞紐石橋") || bldg_query.contains("樞紐橋") {
        &["三水匯流大石橋", "中心樞紐石橋", "中心樞紐橋", "樞紐橋"]
    } else if bldg_query.contains("川流貨船大埠") || bldg_query.contains("貨船大埠") || bldg_query.contains("貨運碼頭") {
        &["川流貨船大埠", "三水口貨運碼頭", "貨運碼頭", "碼頭匯兌分號"]
    } else if bldg_query.contains("農牧總社") {
        &["大圳農牧總社", "農牧總社"]
    } else if bldg_query.contains("農具與百工坊") {
        &["大圳農具與百工坊", "農具與百工坊"]
    } else if (bldg_query.contains("三水口石砌住宅") || bldg_query.contains("三水口")) && is_residence_query {
        &[
            "票號總管居",
            "石板弄堂天井",
            "陸家宅邸",
            "晨汲青石宅",
            "老水夫舍",
            "信使快騎宿所",
            "陳氏庖廚居",
            "白氏湯屋居",
            "河畔迎客小院",
        ]
    } else if (bldg_query.contains("分家巷石板民宅") || bldg_query.contains("分家巷")) && is_residence_query {
        &[
            "岑氏武勇舍",
            "岑氏匠師宗宅",
            "柯氏木作居",
            "蘇氏繡坊居",
            "岑氏鍛藝居",
            "沈氏臨水居",
            "精工火磚匠廬",
        ]
    } else if (bldg_query.contains("講堂街・溫家宅院") || bldg_query.contains("溫家宅院") || bldg_query.contains("講堂街")) && is_residence_query {
        &[
            "蒙學陸先生舍",
            "秦氏算學居",
            "陸氏清雅書齋",
            "溫家藥圃居",
            "抄經校勘齋",
            "石硯墨香居",
        ]
    } else if (bldg_query.contains("車隊駝伕宿社") || bldg_query.contains("西門")) && is_residence_query {
        &[
            "韓氏總站宅院",
            "駝獸牧夫宿所",
            "皮靴油布匠居",
            "遠行行商棧舍",
            "大車修理工寮",
            "行道車馬行舍",
            "聯運公秤舍",
            "草料客商居",
        ]
    } else if (bldg_query.contains("住宅") || bldg_query.contains("民宅") || bldg_query.contains("家屋") || bldg_query.contains("民舍")) && is_residence_query {
        &[
            "陸家宅邸",
            "票號總管居",
            "石板弄堂天井",
            "青石宅",
            "老水夫舍",
            "匠居",
            "宗宅",
            "書齋",
            "算學居",
            "先生舍",
            "校勘齋",
            "墨香居",
            "武勇舍",
            "棧舍",
            "工寮",
            "公秤舍",
            "客商居",
            "宿舍",
            "宿所",
            "民宅",
            "宅院",
            "宅",
        ]
    } else {
        &[]
    };

    if !aliases.is_empty() {
        let mut h: usize = 0xcbf2_9ce4_8422_2325;
        for b in bldg_query.as_bytes() {
            h ^= *b as usize;
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        let n = aliases.len();
        let start_idx = h % n;
        for i in 0..n {
            let alias = aliases[(start_idx + i) % n];
            if let Some(cell) = grid.cells().filter(is_bldg_cell).find(|c| c.name.contains(alias)) {
                return Some(cell);
            }
        }
    }
    None
}

//! 白塔半島正典 800×1,000（800,000 格）無縫純符號大地圖生成器
//!
//! 依據 2026-07-03 白塔半島正典設定：
//! - 範圍：x: -400..=399 (寬 800), y: -500..=499 (高 1,000)，總格數 800,000 格，(0, 0) 為中心文曜城。
//! - 15 座定案聚落（共 405 棟建築原尺寸攤平）：
//!   - 2 城（各 55 棟建築，外圍實心城牆 ■ 與四向城門 ::）：文曜、風舶角
//!   - 4 鎮（各 40 棟建築，外圍木柵圍欄 ≠ 與鎮口 ::）：風舶嶼、大圳頭、半嶺鎮、南哨
//!   - 9 村（各 15 棟建築，無圍欄，周邊接田壟 ▦/▤/☷/≡）：
//!     老堰頭、北嶺腳、霜林口、風櫃尾、烏石鼻、牛埔、紅土崁、三號坑口、水尾寮
//! - 19 處正典自然獵場區（地名格式「區名・地貌」）：
//!   長風草甸・草原、古河巨木林・密林、赤石坡・礦坡、落日岬・海岬、驚濤岩岸・岩岸、
//!   巨藻淺灣・淺灣、黑石灣・礫灣、翠冠巨木林・密林、清泉溪谷・溪谷、北坡雲杉林・雲杉、
//!   西麓林道・林道、淺澤泥口・泥口、白茫洲・泥澤、貼山深潭・深潭、青籠關・山隘、
//!   西嶺礦谷・礦谷、東海峭壁・峭壁、雲腰雪林・雪林、白塔雪巔・雪巔（外海：北冕海・深水、東海・深水）
use std::collections::{HashMap, HashSet};

use crate::grid::cell::{Freshness, GridObject, GridObjectKind, Terrain};
use crate::grid::coord::SquareCoord;
use crate::grid::grid::{GridCell, SquareGrid};

pub const MIN_X: i32 = -400;
pub const MAX_X: i32 = 399;
pub const MIN_Y: i32 = -500;
pub const MAX_Y: i32 = 499;
pub const GRID_WIDTH: usize = 800;
pub const GRID_HEIGHT: usize = 1000;
pub const TOTAL_CELLS: usize = 800_000;

pub fn in_peninsula_bounds(coord: SquareCoord) -> bool {
    (MIN_X..=MAX_X).contains(&coord.x) && (MIN_Y..=MAX_Y).contains(&coord.y)
}
/// 生成白塔半島 800×1,000（800,000 格）正典無縫大地圖（顯式聚落、道路與獵場錨點）
pub fn generate_white_tower_grid() -> SquareGrid {
    let mut grid = SquareGrid::new();
    grid.set_world_seed(2026_07_03);

    // 1. 產生道路與橋樑骨架集
    let roads_and_bridges = generate_road_network();

    // 2. 聚落佈局（15 聚落，共 405 棟建築）
    let settlements = generate_all_settlements();

    // 優先填入 15 聚落格
    for (_, cell) in settlements {
        grid.insert(cell);
    }

    // 填入道路與橋樑（區名與名稱一律依所在正典獵場區以「區名・道路／木橋」生成，不硬編假大道名）
    for (coord, (terrain, _, _)) in roads_and_bridges {
        if !grid.has_explicit_cell(coord) {
            let (zone, _, wild_terrain) = classify_wilderness_cell(coord.x, coord.y);
            let is_bay_bridge = coord.y == 160 && (-322..=-271).contains(&coord.x);
            let is_water_crossing = wild_terrain == Terrain::Water || wild_terrain == Terrain::WaterDeep || terrain == Terrain::Bridge;
            let actual_terrain = if is_bay_bridge || is_water_crossing { Terrain::Bridge } else { Terrain::Road };
            let actual_zone = if is_bay_bridge { "天然港灣道" } else { zone };
            let sub_geo = match actual_terrain {
                Terrain::Bridge => {
                    if is_bay_bridge {
                        "跨海長橋"
                    } else {
                        "木橋"
                    }
                }
                _ => "道路",
            };
            let name = format!("{}・{}", actual_zone, sub_geo);
            let mut cell = GridCell::new(coord, actual_terrain, name);
            cell.zone = actual_zone.to_string();
            cell.description = format!(
                "{}境內平整通達的{}，連通半島各方聚落與自然獵場。",
                actual_zone, sub_geo
            );
            cell.explored = true;
            cell.freshness = Freshness::Fresh;
            cell.objects = generate_road_cell_objects(coord, actual_zone, actual_terrain == Terrain::Bridge);
            grid.insert(cell);
        }
    }
    // 顯式生成連續水系與大圳主渠（使玩家自文曜、老堰頭、大圳頭、北嶺腳出城時即刻可見連通河流）
    // 1. 北源清泉水脈 (y in 18..=385)
    for y in 18..=385 {
        let (rx, half_w) = if (118..=132).contains(&y) {
            (-1, 0)
        } else if (104..118).contains(&y) {
            (-2 - (118 - y) / 5, 1)
        } else if (133..=147).contains(&y) {
            (-2 - (y - 132) / 5, 1)
        } else {
            (-5 + peninsula_wave(y, 90, 4, 40, 2), 1)
        };
        for dx in -half_w..=half_w {
            let coord = SquareCoord::new(rx + dx, y);
            if !grid.has_explicit_cell(coord) {
                grid.insert(materialize_wilderness_cell(coord));
            }
        }
    }
    // 2. 東源清泉溪 (x in 18..=151)
    for x in 18..=151 {
        let ry = if x <= 100 {
            2 + ((x - 18) * 113) / 82 + peninsula_wave(x, 48, 3, 24, 2)
        } else {
            115 + ((x - 100) * 93) / 51 + peninsula_wave(x, 48, 3, 24, 2)
        };
        for dy in -1..=1 {
            let coord = SquareCoord::new(x, ry + dy);
            if !grid.has_explicit_cell(coord) {
                grid.insert(materialize_wilderness_cell(coord));
            }
        }
    }
    // 3. 西麓山溪 (x in -135..=-18)
    for x in -135..=-18 {
        let wy_creek = 4 + ((-x - 18) * 76) / 117 + peninsula_wave(x, 52, 4, 26, 2);
        for dy in -1..=1 {
            let coord = SquareCoord::new(x, wy_creek + dy);
            if !grid.has_explicit_cell(coord) {
                grid.insert(materialize_wilderness_cell(coord));
            }
        }
    }
    // 4. 南流大河 (y in -340..=-18)
    for y in -340..=-18 {
        let sr_x = if (-42..=-18).contains(&y) {
            2 + ((-18 - y) * 8) / 24
        } else if (-232..=-208).contains(&y) {
            10
        } else {
            10 + peninsula_wave(y, 80, 4, 36, 2)
        };
        for dx in -1..=1 {
            let coord = SquareCoord::new(sr_x + dx, y);
            if !grid.has_explicit_cell(coord) {
                grid.insert(materialize_wilderness_cell(coord));
            }
        }
    }
    // 5. 西南大圳 (x in -235..=8)
    for x in -235..=8 {
        let cy = if x >= -155 {
            -55 - ((8 - x) * 83) / 163
        } else {
            -138
        };
        let coord = SquareCoord::new(x, cy);
        if !grid.has_explicit_cell(coord) {
            grid.insert(materialize_wilderness_cell(coord));
        }
    }
    // 6. 天然港灣道與風舶海峽南北出海口水域顯式落地
    for y in 140..=180 {
        for x in -322..=-271 {
            let coord = SquareCoord::new(x, y);
            if !grid.has_explicit_cell(coord) {
                grid.insert(materialize_wilderness_cell(coord));
            }
        }
    }
    for &(x, y) in &[(-300, 195), (-300, 110)] {
        let coord = SquareCoord::new(x, y);
        if !grid.has_explicit_cell(coord) {
            grid.insert(materialize_wilderness_cell(coord));
        }
    }

    // 顯式生成 19 處正典獵場核心地貌與礦口（確保 cells() 掃描與快速驗證完整涵蓋 5 大分區 19 獵場）
    let hunting_ground_centers = [
        // 一、大圳頭（4 獵場區：長風草甸、古河巨木林、赤石坡、落日岬）
        (-180, -195), // 長風草甸（大圳頭與牛埔之間向陽廣袤草甸）
        (-80, -60),   // 古河巨木林（文曜西南至大圳頭、紅土崁之間古河階巨木林）
        (-85, -180),  // 赤石坡（紅土崁南側赤石階地礦坡）
        (-340, -140), // 落日岬（大圳頭最西端入海海岬）
        // 二、風舶雙港與近海（3 獵場區：驚濤岩岸、巨藻淺灣、黑石灣）
        (-360, 180),  // 驚濤岩岸（風舶嶼與風櫃尾西岸）
        (-300, 260),  // 巨藻淺灣（風舶雙港北側礁石淺灣）
        (-260, -20),  // 黑石灣（烏石鼻南側玄武岩海灣）
        // 三、文曜—北坡森林—白塔（4 獵場區：翠冠巨木林、清泉溪谷、北坡雲杉林、西麓林道）
        (-130, 80),   // 西麓林道（文曜往西北通往風舶角的西麓山腰林道）
        (60, 160),    // 清泉溪谷（文曜東北至老堰頭、半嶺鎮之間融雪溪谷）
        (0, 240),     // 翠冠巨木林（老堰頭往北溫暖巨木林）
        (80, 300),    // 北坡雲杉林（北坡高海拔百年雲杉林）
        // 四、南哨—水尾澤—青籠關（4 獵場區：淺澤泥口、白茫洲、貼山深潭、青籠關）
        (0, -340),    // 淺澤泥口（南哨南門外淺水泥澤）
        (-120, -320), // 白茫洲（水尾寮南側水尾澤迷霧泥澤）
        (120, -320),  // 貼山深潭（水尾澤東側緊貼青脊山腳深潭）
        (0, -450),    // 青籠關（極南陸路險隘）
        // 五、青脊山主脊—白塔（4 秘境獵場區：西嶺礦谷、東海峭壁、雲腰雪林、白塔雪巔）
        (180, 40),    // 西嶺礦谷（青脊山西側縱深礦谷）
        (320, 200),   // 東海峭壁（半島東側臨海絕壁）
        (180, 320),   // 雲腰雪林（白塔山腰耐寒鐵杉雪林）
        (0, 420),     // 白塔雪巔（半島極北雪白主峰）
        // 六、外海錨點（北冕海、東海）
        (0, 485),     // 北冕海
        (360, 0),     // 東海
        // 七、半島南端與青籠關南隘口延伸
        (0, -490),    // 青籠關南古隘道
        (-20, -485),  // 青籠關南側幽深籠林
    ];
    for &(cx, cy) in &hunting_ground_centers {
        for dy in -2..=2 {
            for dx in -2..=2 {
                let coord = SquareCoord::new(cx + dx, cy + dy);
                if !grid.has_explicit_cell(coord) {
                    grid.insert(materialize_wilderness_cell(coord));
                }
            }
        }
    }

    grid.clear_dirty();
    grid
}



include!("gen/wilderness.rs");
include!("gen/road_objects.rs");
include!("gen/cell_objects.rs");
include!("gen/road_network.rs");
include!("gen/city_wen_yao_outer.rs");
include!("gen/city_wen_yao_core.rs");
include!("gen/city_wen_yao_slots.rs");
include!("gen/city_feng_bo_jiao.rs");
include!("gen/towns.rs");
include!("gen/villages.rs");
include!("gen/settlement_defs.rs");
include!("gen/tests.rs");

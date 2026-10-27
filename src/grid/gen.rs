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
        // 一、西南原（4 獵場區：長風草甸、古河巨木林、赤石坡、落日岬）
        (-180, -195), // 長風草甸（大圳頭與牛埔之間向陽廣袤草甸）
        (-80, -60),   // 古河巨木林（文曜西南至大圳頭、紅土崁之間古河階巨木林）
        (-85, -180),  // 赤石坡（紅土崁南側赤石階地礦坡）
        (-340, -140), // 落日岬（西南原最西端入海海岬）
        // 二、風舶雙港與近海（3 獵場區：驚濤岩岸、巨藻淺灣、黑石灣）
        (-360, 180),  // 驚濤岩岸（風舶嶼與風櫃尾西岸）
        (-300, 260),  // 巨藻淺灣（風舶雙港北側礁石淺灣）
        (-260, -20),  // 黑石灣（烏石鼻南側玄武岩海灣）
        // 三、文曜—北坡森林—白塔（4 獵場區：翠冠巨木林、清泉溪谷、北坡雲杉林、西麓林道）
        (-130, 80),   // 西麓林道（文曜往西北通往風舶角的西麓山腰林道）
        (60, 160),    // 清泉溪谷（文曜東北至老堰頭、半嶺鎮之間融雪溪谷）
        (0, 240),     // 翠冠巨木林（老堰頭往北溫暖巨木林）
        (80, 300),    // 北坡雲杉林（北坡高海拔百年雲杉林）
        // 四、南哨—黑沼林—青籠關（4 獵場區：淺澤泥口、白茫洲、貼山深潭、青籠關）
        (0, -340),    // 淺澤泥口（南哨南門外淺水泥澤）
        (-120, -320), // 白茫洲（水尾寮南側黑沼林迷霧泥澤）
        (120, -320),  // 貼山深潭（黑沼林東側緊貼青脊山腳深潭）
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

// ─── 19 處自然獵場分類 ───────────────────────────────────────────────

const SIN16: [i32; 16] = [0, 38, 71, 92, 100, 92, 71, 38, 0, -38, -71, -92, -100, -92, -71, -38];

fn peninsula_wave(t: i32, p1: i32, a1: i32, p2: i32, a2: i32) -> i32 {
    let idx1 = ((t.rem_euclid(p1) * 16) / p1) as usize & 15;
    let idx2 = ((t.rem_euclid(p2) * 16) / p2) as usize & 15;
    (SIN16[idx1] * a1 + SIN16[idx2] * a2) / 100
}

/// 依 (x, y) 座標決定正典 19 獵場與外海之 (zone, sub_geo_name, terrain)。
/// 判斷座標是否位於白塔半島連續河流與運河水系中
pub fn is_peninsula_river(x: i32, y: i32) -> Option<(&'static str, &'static str, Terrain)> {
    if x.abs() <= 17 && y.abs() <= 17 {
        return None;
    }

    // 1. 北源清泉水脈 (y in 18..=385)
    if (18..=385).contains(&y) {
        let (river_x, half_w) = if (118..=132).contains(&y) {
            (-1, 0)
        } else if (104..118).contains(&y) {
            (-2 - (118 - y) / 5, 1)
        } else if (133..=147).contains(&y) {
            (-2 - (y - 132) / 5, 1)
        } else {
            (-5 + peninsula_wave(y, 90, 4, 40, 2), 1)
        };
        if (x - river_x).abs() <= half_w {
            let zone = if y >= 375 { "白塔雪巔" } else { "翠冠巨木林" };
            return Some((zone, "河道", Terrain::Water));
        }
    }

    // 2. 東源清泉溪 (x in 18..=151)
    if (18..=151).contains(&x) {
        let ry = if x <= 100 {
            2 + ((x - 18) * 113) / 82 + peninsula_wave(x, 48, 3, 24, 2)
        } else {
            115 + ((x - 100) * 93) / 51 + peninsula_wave(x, 48, 3, 24, 2)
        };
        if (y - ry).abs() <= 1 {
            return Some(("清泉溪谷", "溪流", Terrain::Water));
        }
    }

    // 3. 西麓山溪 (x in -135..=-18, excluding x == -18 && (y == 0 || y == -1))
    if (-135..=-18).contains(&x) && !(x == -18 && (y == 0 || y == -1)) {
        let wy_creek = 4 + ((-x - 18) * 76) / 117 + peninsula_wave(x, 52, 4, 26, 2);
        if (y - wy_creek).abs() <= 1 {
            return Some(("西麓林道", "山溪", Terrain::Water));
        }
    }

    // 4. 南流大河 (y in -340..=-18)
    if (-340..=-18).contains(&y) {
        let sr_x = if (-42..=-18).contains(&y) {
            2 + ((-18 - y) * 8) / 24
        } else if (-232..=-208).contains(&y) {
            10
        } else {
            10 + peninsula_wave(y, 80, 4, 36, 2)
        };
        if (x - sr_x).abs() <= 1 {
            let zone = if y <= -120 { "淺澤泥口" } else { "古河巨木林" };
            return Some((zone, "河道", Terrain::Water));
        }
    }

    // 5. 西南大圳 (x in -235..=8)
    if (-235..=8).contains(&x) {
        let cy = if x >= -155 {
            -55 - ((8 - x) * 83) / 163
        } else {
            -138
        };
        if (y - cy).abs() == 0 {
            let zone = if x <= -180 {
                "長風草甸"
            } else if x <= -80 {
                "古河巨木林"
            } else {
                "淺澤泥口"
            };
            return Some((zone, "大圳", Terrain::Water));
        }
    }

    None
}

/// 依 (x, y) 座標決定正典 19 獵場與外海之 (zone, sub_geo_name, terrain)。
pub fn classify_wilderness_cell(x: i32, y: i32) -> (&'static str, &'static str, Terrain) {
    let wx = peninsula_wave(x, 160, 18, 64, 8);
    let wy = peninsula_wave(y, 180, 22, 72, 10);

    // 1. 風舶外島（風舶嶼 -330, 160、驚濤岩岸 -360, 180、風櫃尾 -360, 95）
    let w_isl = peninsula_wave(y, 48, 3, 20, 2);
    let south_cap = if y < 90 { (90 - y) * 2 } else { 0 };
    let north_cap = if y > 196 { (y - 196) * 2 } else { 0 };
    let island_west = if (89..=101).contains(&y) {
        -366
    } else {
        -376 + w_isl + south_cap + north_cap
    };
    let island_east = if (146..=174).contains(&y) {
        -323 - ((y - 160).abs().saturating_sub(7)) / 2
    } else if y > 174 {
        -326 - ((y - 174) * 18) / 32 + w_isl / 2 - north_cap
    } else {
        -326 - ((146 - y) * 18) / 62 + w_isl / 2 - south_cap
    };
    let on_fengbo_island = (84..=206).contains(&y) && x >= island_west && x <= island_east;

    // 2. 半島外海海岸線
    let north_taper = if y > 220 { (y - 220) / 3 } else { 0 };
    let south_taper = 0;
    let coast_west = if y >= 245 {
        -355 + north_taper + wy
    } else if y >= 175 {
        -270 - ((y - 175) * (77 - wy)) / 70
    } else if y >= 145 {
        -270
    } else if y >= -28 {
        if (28..=42).contains(&y) {
            -266
        } else if (-24..=-16).contains(&y) {
            -265
        } else {
            -268 + peninsula_wave(y, 56, 3, 24, 2)
        }
    } else if y >= -115 {
        -268 - ((-29 - y) * (87 - wy)) / 86
    } else {
        let mut cw = -355 + south_taper + wy;
        if (-165..=-115).contains(&y) && cw > -355 {
            cw = -355;
        }
        cw
    };

    let mut coast_east = 335 - north_taper - south_taper / 2 + wy;
    if (175..=225).contains(&y) && coast_east < 332 {
        coast_east = 332;
    }
    let coast_north = 455 + wx - (x.abs() / 5);
    // 3. 天然港灣道與深水外海判定（當不在風舶外島上時）
    if !on_fengbo_island {
        if (84..=206).contains(&y) && x > island_east && x < coast_west {
            return ("天然港灣道", "海灣", Terrain::Water);
        } else if (207..=230).contains(&y) && x >= (-356 + (y - 206) * 2 + w_isl) && x < coast_west {
            return ("天然港灣道", "北水道口", Terrain::Water);
        } else if (60..=83).contains(&y) && x >= (-356 + (84 - y) * 3 + w_isl) && x < coast_west {
            return ("天然港灣道", "南水道口", Terrain::Water);
        }
        if y > coast_north || x < coast_west {
            return ("北冕海", "深水", Terrain::WaterDeep);
        }
        if x > coast_east {
            return ("東海", "深水", Terrain::WaterDeep);
        }
    }
    // 文曜西門外關鍵錨點保持西麓林道正典
    if x == -18 && (y == 0 || y == -1) {
        return ("西麓林道", "林道", Terrain::Forest);
    }

    // 19 處正典獵場核心 5x5 錨點優先保持正典主地貌
    let hunting_ground_anchors = [
        ((-180, -195), "長風草甸", "草原", Terrain::Grassland),
        ((-80, -60), "古河巨木林", "密林", Terrain::Forest),
        ((-85, -180), "赤石坡", "礦坡", Terrain::MinePass),
        ((-340, -140), "落日岬", "海岬", Terrain::Beach),
        ((-360, 180), "驚濤岩岸", "岩岸", Terrain::Beach),
        ((-300, 260), "巨藻淺灣", "淺灣", Terrain::Beach),
        ((-260, -20), "黑石灣", "礫灣", Terrain::Beach),
        ((-130, 80), "西麓林道", "林道", Terrain::Forest),
        ((60, 160), "清泉溪谷", "溪谷", Terrain::Water),
        ((0, 240), "翠冠巨木林", "密林", Terrain::Forest),
        ((80, 300), "北坡雲杉林", "雲杉", Terrain::Forest),
        ((0, -340), "淺澤泥口", "泥口", Terrain::Swamp),
        ((-120, -320), "白茫洲", "泥澤", Terrain::Swamp),
        ((120, -320), "貼山深潭", "深潭", Terrain::Swamp),
        ((0, -450), "青籠關", "山隘", Terrain::MinePass),
        ((180, 40), "西嶺礦谷", "礦谷", Terrain::MinePass),
        ((320, 200), "東海峭壁", "峭壁", Terrain::MinePass),
        ((180, 320), "雲腰雪林", "雪林", Terrain::Forest),
        ((0, 420), "白塔雪巔", "雪巔", Terrain::HolyTower),
    ];
    for &((cx, cy), z, sg, t) in &hunting_ground_anchors {
        if (x - cx).abs() <= 2 && (y - cy).abs() <= 2 {
            return (z, sg, t);
        }
    }

    // 連續河流與水系（僅文曜城外）
    if let Some(river) = is_peninsula_river(x, y) {
        return river;
    }

    // 凝聚性 3x3 叢集雜湊（用於自然微地貌變化，消除純色塊）
    let gx = x.div_euclid(3);
    let gy = y.div_euclid(3);
    let h = ((gx.wrapping_mul(73856093) ^ gy.wrapping_mul(19349663)).rem_euclid(100)) as usize;

    // 19 處正典獵場分區判定與自然微地貌生成
    if y >= 375 + wx / 2 && x <= 135 + wy / 2 && x >= -210 + wy / 2 {
        if x.abs() <= 15 && (405..=435).contains(&y) {
            return ("白塔雪巔", "雪巔", Terrain::HolyTower);
        }
        if h < 22 {
            return ("白塔雪巔", "礦階", Terrain::MinePass);
        } else if h < 26 {
            return ("白塔雪巔", "高山草甸", Terrain::Grassland);
        } else if h < 30 {
            return ("白塔雪巔", "高山藥叢", Terrain::HerbGarden);
        }
        return ("白塔雪巔", "冰峰", Terrain::Mountain);
    }
    if y >= 255 + wx / 2 && x > 135 + wy / 2 && x < 312 + wy / 3 {
        if h < 10 {
            return ("雲腰雪林", "林間草地", Terrain::Grassland);
        } else if h < 15 {
            return ("雲腰雪林", "野生草藥圃", Terrain::HerbGarden);
        } else if h < 19 {
            return ("雲腰雪林", "林間石坎", Terrain::MinePass);
        }
        return ("雲腰雪林", "雪林", Terrain::Forest);
    }
    if x >= coast_east - 28 {
        if x == 320 && (198..=202).contains(&y) {
            return ("東海峭壁", "峭壁", Terrain::MinePass);
        }
        if h < 22 {
            return ("東海峭壁", "礦階", Terrain::MinePass);
        } else if h < 26 {
            return ("東海峭壁", "高山草甸", Terrain::Grassland);
        } else if h < 30 {
            return ("東海峭壁", "高山藥叢", Terrain::HerbGarden);
        }
        return ("東海峭壁", "峭壁", Terrain::Mountain);
    }

    // 4 處濱海獵場（風舶外島・驚濤岩岸、巨藻淺灣、黑石灣、落日岬）
    if on_fengbo_island {
        let zone = "驚濤岩岸";
        if x <= island_west + 6 || x >= island_east - 3 || y <= 88 || y >= 196 {
            if h < 10 {
                return (zone, "海蝕礁石", Terrain::Mountain);
            } else if h < 18 && x <= island_west + 4 {
                return (zone, "潮間帶潮池", Terrain::Water);
            } else if h < 30 {
                return (zone, "海濱灌叢", Terrain::Grassland);
            }
            return (zone, "岩岸", Terrain::Beach);
        } else {
            if h < 32 {
                return (zone, "海濱林", Terrain::Forest);
            } else if h < 56 {
                return (zone, "海濱草坡", Terrain::Grassland);
            } else if h < 76 {
                return (zone, "海蝕岩丘", Terrain::MinePass);
            }
            return (zone, "岩岸", Terrain::Beach);
        }
    }

    let is_ju_zao = x <= -220 + wy / 2 && y >= 171 + wx / 3;
    let is_hei_shi = x <= -220 + wy / 2 && (-79 + wx / 2..=149).contains(&y);
    let is_luo_ri = x <= -261 + wy / 2 && (-260 + wx / 2..=-80 + wx / 2).contains(&y);
    if is_ju_zao || is_hei_shi || is_luo_ri {
        let (zone, sub_geo) = if is_ju_zao {
            ("巨藻淺灣", "淺灣")
        } else if is_hei_shi {
            ("黑石灣", "礫灣")
        } else {
            ("落日岬", "海岬")
        };
        if is_luo_ri && x > -305 {
            // 落日岬東側內陸礦麓（三號坑口外圍）：礦脈岩坡、向陽草坡與耐風林地
            if h < 42 {
                return (zone, "礦脈岩坡", Terrain::MinePass);
            } else if h < 74 {
                return (zone, "向陽草坡", Terrain::Grassland);
            }
            return (zone, "耐風林地", Terrain::Forest);
        } else if x > coast_west + 14 {
            // 內陸過渡帶：海濱林、草坡與岩丘
            if h < 30 {
                return (zone, "海濱林", Terrain::Forest);
            } else if h < 50 {
                return (zone, "海濱草坡", Terrain::Grassland);
            } else if h < 65 {
                return (zone, "海蝕岩丘", Terrain::MinePass);
            }
            return (zone, "海灘", Terrain::Beach);
        } else {
            // 臨海潮間帶：海蝕礁石、潮池、海濱灌叢與沙灘
            if h < 8 {
                return (zone, "海蝕礁石", Terrain::Mountain);
            } else if h < 15 {
                return (zone, "潮間帶潮池", Terrain::Water);
            } else if h < 24 {
                return (zone, "海濱灌叢", Terrain::Grassland);
            }
            return (zone, sub_geo, Terrain::Beach);
        }
    }

    if ((-260 + wy / 2..=-125 + wy / 2).contains(&x) && (-260 + wx / 2..=-80 + wx / 2).contains(&y))
        || ((-219 + wy / 2..=-141 + wy / 2).contains(&x) && (-79 + wx / 2..=-1).contains(&y))
    {
        if h < 8 {
            return ("長風草甸", "草甸樹叢", Terrain::Forest);
        } else if h < 14 {
            return ("長風草甸", "野花草甸", Terrain::HerbGarden);
        } else if h < 18 {
            return ("長風草甸", "微風土丘", Terrain::MinePass);
        }
        return ("長風草甸", "草原", Terrain::Grassland);
    }
    if (-124 + wy / 2..=-30 + wy / 2).contains(&x) && (-260 + wx / 2..=-116 + wx / 2).contains(&y) {
        if h < 22 || ((-87..=-83).contains(&x) && (-182..=-178).contains(&y)) {
            return ("赤石坡", "礦坡", Terrain::MinePass);
        } else if h < 26 {
            return ("赤石坡", "高山草甸", Terrain::Grassland);
        } else if h < 30 {
            return ("赤石坡", "高山藥叢", Terrain::HerbGarden);
        }
        return ("赤石坡", "山脊", Terrain::Mountain);
    }
    if (-140 + wy / 2..=-1).contains(&x) && (-115 + wx / 2..=-18).contains(&y) {
        if h < 10 {
            return ("古河巨木林", "林間草地", Terrain::Grassland);
        } else if h < 15 {
            return ("古河巨木林", "野生草藥圃", Terrain::HerbGarden);
        } else if h < 19 {
            return ("古河巨木林", "林間石坎", Terrain::MinePass);
        }
        return ("古河巨木林", "密林", Terrain::Forest);
    }
    if y <= -410 + wx / 2 + x / 10 {
        if x.abs() <= 4 {
            return ("青籠關", "山隘", Terrain::MinePass);
        }
        if h < 22 {
            return ("青籠關", "礦階", Terrain::MinePass);
        } else if h < 48 {
            return ("青籠關", "幽深籠林", Terrain::Forest);
        } else if h < 54 {
            return ("青籠關", "高山草甸", Terrain::Grassland);
        } else if h < 60 {
            return ("青籠關", "高山藥叢", Terrain::HerbGarden);
        }
        return ("青籠關", "山脊", Terrain::Mountain);
    }

    // 3 處水澤獵場（白茫洲、貼山深潭、淺澤泥口）
    let is_bai_mang = y <= -261 + wx / 2 && x <= -51 + wy / 2;
    let is_tie_shan = y <= -200 + wx / 2 && (60 + wy / 2..=185 + wy / 2).contains(&x);
    let is_qian_ze = (y <= -120 + wx / 2 && (-50 + wy / 2..=59 + wy / 2).contains(&x))
        || ((-119 + wx / 2..=-18).contains(&y) && (0..=39).contains(&x));
    if is_bai_mang || is_tie_shan || is_qian_ze {
        let (zone, sub_geo) = if is_bai_mang {
            ("白茫洲", "泥澤")
        } else if is_tie_shan {
            ("貼山深潭", "深潭")
        } else {
            ("淺澤泥口", "泥口")
        };
        let pool_lim = if is_tie_shan { 28 } else { 12 };
        if h < pool_lim {
            return (zone, "沼澤水窪", Terrain::Water);
        } else if h < pool_lim + 6 {
            return (zone, "蘆葦洲", Terrain::Grassland);
        } else if h < pool_lim + 12 {
            return (zone, "枯木林", Terrain::Forest);
        }
        return (zone, sub_geo, Terrain::Swamp);
    }

    if (x >= 141 + wy / 2 && (40 + wx / 2..=254 + wx / 2).contains(&y))
        || (x >= 40 + wy / 2 && (-199 + wx / 2..=39 + wx / 2).contains(&y))
        || (x > 185 + wy / 2 && y <= -200 + wx / 2)
    {
        if h < 22 || ((178..=182).contains(&x) && (38..=42).contains(&y)) {
            return ("西嶺礦谷", "礦谷", Terrain::MinePass);
        } else if h < 26 {
            return ("西嶺礦谷", "高山草甸", Terrain::Grassland);
        } else if h < 30 {
            return ("西嶺礦谷", "高山藥叢", Terrain::HerbGarden);
        }
        return ("西嶺礦谷", "山脊", Terrain::Mountain);
    }
    if (41 + wy / 2..=140 + wy / 2).contains(&x) && (241 + wx / 2..=374 + wx / 2).contains(&y) {
        if h < 10 {
            return ("北坡雲杉林", "林間草地", Terrain::Grassland);
        } else if h < 15 {
            return ("北坡雲杉林", "野生草藥圃", Terrain::HerbGarden);
        } else if h < 19 {
            return ("北坡雲杉林", "林間石坎", Terrain::MinePass);
        }
        return ("北坡雲杉林", "雲杉", Terrain::Forest);
    }

    // 清泉溪谷：林地與草甸溪谷，北部設有溪流支脈貫通至 (60, 160)
    if (20 + wy / 3..=140 + wy / 2).contains(&x) && (40 + wx / 2..=240 + wx / 2).contains(&y) {
        if (55..=65).contains(&x) && (140..=230).contains(&y) {
            let stream_x = 60 + peninsula_wave(y, 40, 3, 20, 1);
            if (x - stream_x).abs() <= 1 {
                return ("清泉溪谷", "溪流", Terrain::Water);
            }
        }
        if (58..=62).contains(&x) && (158..=162).contains(&y) {
            return ("清泉溪谷", "溪谷", Terrain::Water);
        }
        if h < 10 {
            return ("清泉溪谷", "林間草地", Terrain::Grassland);
        } else if h < 15 {
            return ("清泉溪谷", "野生草藥圃", Terrain::HerbGarden);
        } else if h < 19 {
            return ("清泉溪谷", "林間石坎", Terrain::MinePass);
        } else if h < 35 {
            return ("清泉溪谷", "溪旁草灘", Terrain::Grassland);
        }
        return ("清泉溪谷", "密林", Terrain::Forest);
    }

    if y >= 221 + wx / 2 || ((-19..=19).contains(&x) && (18..=220 + wx / 2).contains(&y)) {
        if h < 10 {
            return ("翠冠巨木林", "林間草地", Terrain::Grassland);
        } else if h < 15 {
            return ("翠冠巨木林", "野生草藥圃", Terrain::HerbGarden);
        } else if h < 19 {
            return ("翠冠巨木林", "林間石坎", Terrain::MinePass);
        }
        return ("翠冠巨木林", "密林", Terrain::Forest);
    }

    if h < 10 {
        return ("西麓林道", "林間草地", Terrain::Grassland);
    } else if h < 15 {
        return ("西麓林道", "野生草藥圃", Terrain::HerbGarden);
    } else if h < 19 {
        return ("西麓林道", "林間石坎", Terrain::MinePass);
    }
    ("西麓林道", "林道", Terrain::Forest)
}

fn generate_wilderness_desc(zone: &str, sub_geo: &str) -> String {
    match zone {
        "白塔雪巔" => "雲霧磁場之巔，白塔半島極頂冰雪神峰，罡風呼嘯凜冽，星紋石與神山結晶隱現於千仞冰崖。".to_string(),
        "雲腰雪林" => "白塔山腰積雪覆蓋的耐寒鐵杉雪林，冷杉枝頭凝結冰魄晶，傳聞有雪嶺白獅漫步於冰晶雪蓮叢中。".to_string(),
        "北坡雲杉林" => "北坡挺拔茂密的百年雲杉原木林，樹皮凝結霜紋松脂結晶，巨熊與雪蹤狼群踏雪巡行。".to_string(),
        "翠冠巨木林" => "參天巨木冠蓋蔽日的溫暖森林，鐵心巨木與光葉藥草茂盛生長，林豹與巡林鹿出沒林間。".to_string(),
        "古河巨木林" => "遠古河床兩岸倒伏沉積的巨木密林，蘊藏溫木心材、天然樹脂與清冽溪底礪石。".to_string(),
        "西麓林道" => "西麓山腰起伏的蜿蜒林道，灌木叢中滿布野生山茶籽與白鮮草，林道潛伏獸匿跡其間。".to_string(),
        "清泉溪谷" => "自高山融雪順流而下的清澈溪谷，水流湍急清甜，河床沉積寒水玉原石與亮銀礦砂。".to_string(),
        "長風草甸" => "微風吹拂如綠浪起伏的連綿廣袤草原，重角原牛低頭吃草，生長著強韌鐵線長草與日照穀木。".to_string(),
        "赤石坡" => "富含赤紋鐵礦與伴生銅砂的階地礦坡，採礦車轍印痕清晰，岩壁乾燥溫熱。".to_string(),
        "西嶺礦谷" => "青脊山西側高峻崎嶇的縱深礦谷，裂石山獸出沒，岩層深處蘊藏青脊重礦與山心石髓。".to_string(),
        "青籠關" => "白塔半島唯一對外陸路險隘門戶，兩側峭壁如削，關口粗石堆砌，一夫當關萬夫莫開。".to_string(),
        "貼山深潭" => "緊貼青脊山腳深不見底的冷冽深潭，水深浪靜，潭底沉澱冷鐵重礦與水底奇石。".to_string(),
        "白茫洲" => "常年籠罩在不散晨霧中的廣袤沼澤濕地，泥濘深陷，水苔豐茂，岩穴孕育白茫霧晶與避毒奇花。".to_string(),
        "淺澤泥口" => "水氣氤氳的泥濘水澤入口，生長著柔韌蘆葦與沼生烏木，泥甲鱷獸潛伏泥塘中。".to_string(),
        "落日岬" => "突出西南海面迎受海風的高峻岬角，岩石風化呈橙紅，生長防風韌竹與海岬迎風草。".to_string(),
        "驚濤岩岸" => "面對外海洶湧怒濤的玄武岩陡岸，巨浪拍打岩壁濺起白沫，海蝕奇石與硬殼海貝錯落潮間帶。".to_string(),
        "巨藻淺灣" => "西北礁石環抱的天然淺灣，海底生長繁茂的深海巨藻藤，夜光貝母隨水波泛著幽光。".to_string(),
        "黑石灣" => "鋪滿黑曜玄武岩碎礫與海鹽結晶的黑色海灣，潮平水闊，常有近海大魚與水獸覓食。".to_string(),
        "東海峭壁" => "半島東側臨海高聳千仞的雄奇海蝕絕壁，馭風巨禽築巢石壁，風切石與海蝕奇礦交織生長。".to_string(),
        "北冕海" => "極北之冰洋，波濤深邃浩瀚，遠眺隱約可見雪峰倒影。".to_string(),
        "東海" => "半島東側之浩蕩汪洋，潮聲如雷，晨曦在此破曉升起。".to_string(),
        "天然港灣道" => "風舶雙港之間天然形成的開闊海灣水道，水深浪定，為往來外海島嶼與半島的重要航道。".to_string(),
        _ => format!("白塔半島開闊的自然天地，呈現{}的地貌景致。", sub_geo),
    }
}
static WILDERNESS_PROTOTYPES: std::sync::LazyLock<HashMap<(&'static str, &'static str), GridCell>> =
    std::sync::LazyLock::new(|| {
        let mut map = HashMap::new();
        let prototypes = [
            ("白塔雪巔", "雪巔", Terrain::HolyTower),
            ("白塔雪巔", "冰峰", Terrain::Mountain),
            ("白塔雪巔", "礦階", Terrain::MinePass),
            ("白塔雪巔", "高山草甸", Terrain::Grassland),
            ("白塔雪巔", "高山藥叢", Terrain::HerbGarden),
            ("白塔雪巔", "河道", Terrain::Water),
            ("雲腰雪林", "雪林", Terrain::Forest),
            ("雲腰雪林", "林間草地", Terrain::Grassland),
            ("雲腰雪林", "野生草藥圃", Terrain::HerbGarden),
            ("雲腰雪林", "林間石坎", Terrain::MinePass),
            ("北坡雲杉林", "雲杉", Terrain::Forest),
            ("北坡雲杉林", "林間草地", Terrain::Grassland),
            ("北坡雲杉林", "野生草藥圃", Terrain::HerbGarden),
            ("北坡雲杉林", "林間石坎", Terrain::MinePass),
            ("翠冠巨木林", "密林", Terrain::Forest),
            ("翠冠巨木林", "林間草地", Terrain::Grassland),
            ("翠冠巨木林", "野生草藥圃", Terrain::HerbGarden),
            ("翠冠巨木林", "林間石坎", Terrain::MinePass),
            ("翠冠巨木林", "河道", Terrain::Water),
            ("古河巨木林", "密林", Terrain::Forest),
            ("古河巨木林", "林間草地", Terrain::Grassland),
            ("古河巨木林", "野生草藥圃", Terrain::HerbGarden),
            ("古河巨木林", "林間石坎", Terrain::MinePass),
            ("古河巨木林", "河道", Terrain::Water),
            ("古河巨木林", "大圳", Terrain::Water),
            ("西麓林道", "林道", Terrain::Forest),
            ("西麓林道", "林間草地", Terrain::Grassland),
            ("西麓林道", "野生草藥圃", Terrain::HerbGarden),
            ("西麓林道", "林間石坎", Terrain::MinePass),
            ("西麓林道", "山溪", Terrain::Water),
            ("清泉溪谷", "溪谷", Terrain::Water),
            ("清泉溪谷", "溪流", Terrain::Water),
            ("清泉溪谷", "密林", Terrain::Forest),
            ("清泉溪谷", "林間草地", Terrain::Grassland),
            ("清泉溪谷", "野生草藥圃", Terrain::HerbGarden),
            ("清泉溪谷", "林間石坎", Terrain::MinePass),
            ("清泉溪谷", "溪旁草灘", Terrain::Grassland),
            ("長風草甸", "草原", Terrain::Grassland),
            ("長風草甸", "草甸樹叢", Terrain::Forest),
            ("長風草甸", "野花草甸", Terrain::HerbGarden),
            ("長風草甸", "微風土丘", Terrain::MinePass),
            ("長風草甸", "大圳", Terrain::Water),
            ("赤石坡", "礦坡", Terrain::MinePass),
            ("赤石坡", "山脊", Terrain::Mountain),
            ("赤石坡", "高山草甸", Terrain::Grassland),
            ("赤石坡", "高山藥叢", Terrain::HerbGarden),
            ("西嶺礦谷", "礦谷", Terrain::MinePass),
            ("西嶺礦谷", "山脊", Terrain::Mountain),
            ("西嶺礦谷", "高山草甸", Terrain::Grassland),
            ("西嶺礦谷", "高山藥叢", Terrain::HerbGarden),
            ("青籠關", "山隘", Terrain::MinePass),
            ("青籠關", "山脊", Terrain::Mountain),
            ("青籠關", "礦階", Terrain::MinePass),
            ("青籠關", "幽深籠林", Terrain::Forest),
            ("青籠關", "高山草甸", Terrain::Grassland),
            ("青籠關", "高山藥叢", Terrain::HerbGarden),
            ("貼山深潭", "深潭", Terrain::Swamp),
            ("貼山深潭", "沼澤水窪", Terrain::Water),
            ("貼山深潭", "蘆葦洲", Terrain::Grassland),
            ("貼山深潭", "枯木林", Terrain::Forest),
            ("白茫洲", "泥澤", Terrain::Swamp),
            ("白茫洲", "沼澤水窪", Terrain::Water),
            ("白茫洲", "蘆葦洲", Terrain::Grassland),
            ("白茫洲", "枯木林", Terrain::Forest),
            ("淺澤泥口", "泥口", Terrain::Swamp),
            ("淺澤泥口", "沼澤水窪", Terrain::Water),
            ("淺澤泥口", "蘆葦洲", Terrain::Grassland),
            ("淺澤泥口", "枯木林", Terrain::Forest),
            ("淺澤泥口", "河道", Terrain::Water),
            ("淺澤泥口", "大圳", Terrain::Water),
            ("落日岬", "海岬", Terrain::Beach),
            ("落日岬", "海灘", Terrain::Beach),
            ("落日岬", "海蝕礁石", Terrain::Mountain),
            ("落日岬", "潮間帶潮池", Terrain::Water),
            ("落日岬", "海濱灌叢", Terrain::Grassland),
            ("落日岬", "海濱林", Terrain::Forest),
            ("落日岬", "海濱草坡", Terrain::Grassland),
            ("落日岬", "海蝕岩丘", Terrain::MinePass),
            ("落日岬", "礦脈岩坡", Terrain::MinePass),
            ("落日岬", "向陽草坡", Terrain::Grassland),
            ("落日岬", "耐風林地", Terrain::Forest),
            ("驚濤岩岸", "岩岸", Terrain::Beach),
            ("驚濤岩岸", "海灣", Terrain::Water),
            ("驚濤岩岸", "海灘", Terrain::Beach),
            ("驚濤岩岸", "海蝕礁石", Terrain::Mountain),
            ("驚濤岩岸", "潮間帶潮池", Terrain::Water),
            ("驚濤岩岸", "海濱灌叢", Terrain::Grassland),
            ("驚濤岩岸", "海濱林", Terrain::Forest),
            ("驚濤岩岸", "海濱草坡", Terrain::Grassland),
            ("驚濤岩岸", "海蝕岩丘", Terrain::MinePass),
            ("巨藻淺灣", "淺灣", Terrain::Beach),
            ("巨藻淺灣", "海灘", Terrain::Beach),
            ("巨藻淺灣", "海蝕礁石", Terrain::Mountain),
            ("巨藻淺灣", "潮間帶潮池", Terrain::Water),
            ("巨藻淺灣", "海濱灌叢", Terrain::Grassland),
            ("巨藻淺灣", "海濱林", Terrain::Forest),
            ("巨藻淺灣", "海濱草坡", Terrain::Grassland),
            ("巨藻淺灣", "海蝕岩丘", Terrain::MinePass),
            ("黑石灣", "礫灣", Terrain::Beach),
            ("黑石灣", "海灘", Terrain::Beach),
            ("黑石灣", "海蝕礁石", Terrain::Mountain),
            ("黑石灣", "潮間帶潮池", Terrain::Water),
            ("黑石灣", "海濱灌叢", Terrain::Grassland),
            ("黑石灣", "海濱林", Terrain::Forest),
            ("黑石灣", "海濱草坡", Terrain::Grassland),
            ("黑石灣", "海蝕岩丘", Terrain::MinePass),
            ("東海峭壁", "峭壁", Terrain::Mountain),
            ("東海峭壁", "礦階", Terrain::MinePass),
            ("東海峭壁", "高山草甸", Terrain::Grassland),
            ("東海峭壁", "高山藥叢", Terrain::HerbGarden),
            ("北冕海", "深水", Terrain::WaterDeep),
            ("東海", "深水", Terrain::WaterDeep),
            ("天然港灣道", "海灣", Terrain::Water),
            ("天然港灣道", "北水道口", Terrain::Water),
            ("天然港灣道", "南水道口", Terrain::Water),
            ("天然港灣道", "海峽水道", Terrain::Water),
            ("天然港灣道", "海灣航道", Terrain::Water),
            ("天然港灣道", "海灣棧橋", Terrain::Bridge),
        ];
        for (z, g, t) in prototypes {
            let name = format!("{}・{}", z, g);
            let desc = generate_wilderness_desc(z, g);
            let mut cell = GridCell::new(SquareCoord::new(0, 0), t, name);
            cell.zone = z.to_string();
            cell.description = desc;
            cell.explored = false;
            cell.freshness = Freshness::Fresh;
            map.insert((z, g), cell);
        }
        map
    });

static FALLBACK_PROTOTYPES: std::sync::LazyLock<HashMap<Terrain, GridCell>> =
    std::sync::LazyLock::new(|| {
        let mut map = HashMap::new();
        let types = [
            (Terrain::Plain, "荒野平地"),
            (Terrain::Forest, "林地"),
            (Terrain::ForestHeavy, "密林"),
            (Terrain::Mountain, "山地"),
            (Terrain::Hills, "丘陵"),
            (Terrain::Water, "水面"),
            (Terrain::WaterDeep, "深水"),
            (Terrain::Swamp, "沼澤"),
            (Terrain::Grassland, "草原"),
            (Terrain::Beach, "海灘"),
            (Terrain::MinePass, "石階"),
            (Terrain::HerbGarden, "草藥圃"),
            (Terrain::HolyTower, "聖塔之巔"),
        ];
        for (t, label) in types {
            let mut cell = GridCell::new(SquareCoord::new(0, 0), t, format!("白塔半島・{}", label));
            cell.zone = "白塔半島".to_string();
            cell.description = format!("白塔半島開闊的自然地貌：{}。", label);
            cell.explored = false;
            cell.freshness = Freshness::Fresh;
            map.insert(t, cell);
        }
        map
    });

pub fn procedural_wilderness_cell_ref(coord: SquareCoord) -> Option<&'static GridCell> {
    if !in_peninsula_bounds(coord) {
        return None;
    }
    let (zone, sub_geo, terrain) = classify_wilderness_cell(coord.x, coord.y);
    if let Some(cell) = WILDERNESS_PROTOTYPES.get(&(zone, sub_geo)) {
        return Some(cell);
    }
    FALLBACK_PROTOTYPES.get(&terrain)
}

pub fn materialize_wilderness_cell(coord: SquareCoord) -> GridCell {
    let (zone, sub_geo, terrain) = classify_wilderness_cell(coord.x, coord.y);
    let name = format!("{}・{}", zone, sub_geo);
    let desc = generate_wilderness_desc(zone, sub_geo);
    let mut cell = GridCell::new(coord, terrain, name);
    cell.zone = zone.to_string();
    cell.description = desc;
    cell.explored = false;
    cell.freshness = Freshness::Fresh;
    cell.objects = generate_cell_objects(coord, terrain);
    cell
}

// ─── 自然資源與道路物件生成 ───────────────────────────────────────────

/// 為荒野道路與木橋格生成路旁界石、營火與符合分區之正典風物與採集節點。
pub fn generate_road_cell_objects(
    coord: SquareCoord,
    zone: &str,
    is_bridge: bool,
) -> Vec<GridObject> {
    let mut objs = Vec::new();
    let is_gate_exit = matches!((coord.x, coord.y), (-18, 0) | (18, 0) | (0, 18) | (0, -18));
    let is_resting_stop = (coord.x.abs() + coord.y.abs()) % 10 == 0;

    // 1. 文曜四城門正外方格或每約 10 格道路休息處：指路石碑與野地營火
    if is_gate_exit || is_resting_stop {
        let waymark_name = match (coord.x, coord.y) {
            (-18, 0) => "西麓指路石碑".to_string(),
            (18, 0) => "東原指路石碑".to_string(),
            (0, 18) => "北麓指路石碑".to_string(),
            (0, -18) => "南郊指路石碑".to_string(),
            _ => {
                if is_bridge {
                    "橋頭石碑".to_string()
                } else {
                    "指路石碑".to_string()
                }
            }
        };
        objs.push(GridObject {
            id: format!("waymark_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Other,
            name: waymark_name,
            quantity: 1,
            max_quantity: 1,
            regrowth_per_tick: 0,
        });
        objs.push(GridObject {
            id: format!("campfire_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Other,
            name: "野地營火".to_string(),
            quantity: 5,
            max_quantity: 10,
            regrowth_per_tick: 1,
        });
    }

    // 2. 所有道路／橋樑格皆配置 1~2 件符合所在分區之正典路旁風物與採集資源
    let seed = ((coord.x as i64 * 374761393) ^ (coord.y as i64 * 668265263)).unsigned_abs();
    let roll = seed % 100;

    if is_bridge {
        objs.push(GridObject {
            id: format!("bridge_plank_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Other,
            name: "備用橋木板".to_string(),
            quantity: 2,
            max_quantity: 4,
            regrowth_per_tick: 1,
        });
        if roll % 2 == 0 {
            objs.push(GridObject {
                id: format!("bridge_trough_{}_{}", coord.x, coord.y),
                kind: GridObjectKind::Other,
                name: "繫馬石環".to_string(),
                quantity: 1,
                max_quantity: 1,
                regrowth_per_tick: 0,
            });
        }
    } else {
        match zone {
            "西麓林道" => {
                objs.push(GridObject {
                    id: format!("pine_wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "風乾松柴".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("herb_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "光葉藥草".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("stone_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "護路木樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                }
            }
            "長風草甸" => {
                objs.push(GridObject {
                    id: format!("meadow_grass_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "長風牧草".to_string(),
                    quantity: 5,
                    max_quantity: 10,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("rest_stone_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "車轍歇腳石".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("grain_wood_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "日照穀木枝".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "古河巨木林" => {
                objs.push(GridObject {
                    id: format!("fallen_wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "落木枯枝".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("boundary_stone_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "林間界石".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("resin_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "天然松脂塊".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "清泉溪谷" => {
                objs.push(GridObject {
                    id: format!("creek_herb_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "溪畔水芹".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("horse_trough_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "飲馬石槽".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("jade_ore_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "溪底青石子".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "赤石坡" => {
                objs.push(GridObject {
                    id: format!("road_ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "路旁赤碎石".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("slope_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "防滑路樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("copper_sand_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "伴生銅砂".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "落日岬" => {
                objs.push(GridObject {
                    id: format!("bamboo_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "防風竹枝".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("cape_cairn_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "避風石堆".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("wind_grass_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "海岬迎風草".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "驚濤岩岸" => {
                objs.push(GridObject {
                    id: format!("coast_stone_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "海蝕碎石".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("coast_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "繫纜石樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("shell_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "硬殼海貝".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "巨藻淺灣" | "天然港灣道" => {
                objs.push(GridObject {
                    id: format!("kelp_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "巨藻鮮葉".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("bay_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "渡頭木樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("shell_bead_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "灘頭夜光螺".to_string(),
                        quantity: 2,
                        max_quantity: 4,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "黑石灣" => {
                objs.push(GridObject {
                    id: format!("basalt_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "玄武岩塊".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("tide_step_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "避潮石階石".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("sea_salt_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "粗海鹽結晶".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "翠冠巨木林" => {
                objs.push(GridObject {
                    id: format!("canopy_wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "溫木落材".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("forest_sign_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "行道石龕".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("light_herb_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "路側光葉草".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "北坡雲杉林" => {
                objs.push(GridObject {
                    id: format!("spruce_cone_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "雲杉脂松果".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("snow_cairn_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "踏雪石壘".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("spruce_branch_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "落雪雲杉枝".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "淺澤泥口" => {
                objs.push(GridObject {
                    id: format!("reed_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "柔韌蘆葦".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("marsh_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "墊步木墩".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("marsh_wood_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "沉水黑木枝".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "白茫洲" => {
                objs.push(GridObject {
                    id: format!("marsh_herb_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "白茫避瘴草".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("fog_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "霧徑木標".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("moss_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "止血水苔".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "貼山深潭" => {
                objs.push(GridObject {
                    id: format!("deep_ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "冷鐵碎石".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("water_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "繫繩石柱".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("water_moss_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "潭邊深水草".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "青籠關" => {
                objs.push(GridObject {
                    id: format!("pass_ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "碎重礦石".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("pass_sign_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "盤查關標".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("flint_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "引火碎石".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "西嶺礦谷" => {
                objs.push(GridObject {
                    id: format!("coal_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "赤紋煤塊".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("mine_cairn_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "車轍路石".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("iron_clay_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                        name: "紅黏土塊".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "東海峭壁" => {
                objs.push(GridObject {
                    id: format!("cliff_ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "風切石碎屑".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("cliff_railing_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "絕壁護木樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("cliff_nest_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "落崖海燕枝".to_string(),
                        quantity: 2,
                        max_quantity: 4,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "雲腰雪林" => {
                objs.push(GridObject {
                    id: format!("snow_wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "鐵杉斷木".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("snow_cairn_mark_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "防迷雪標".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("lotus_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "冰雪幼苞".to_string(),
                        quantity: 2,
                        max_quantity: 4,
                        regrowth_per_tick: 1,
                    });
                }
            }
            "白塔雪巔" => {
                objs.push(GridObject {
                    id: format!("peak_stone_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "冰晶碎礫".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("peak_post_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Other,
                        name: "踏雪鐵樁".to_string(),
                        quantity: 1,
                        max_quantity: 1,
                        regrowth_per_tick: 0,
                    });
                } else {
                    objs.push(GridObject {
                        id: format!("star_ore_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Mineral,
                    name: "星紋碎石".to_string(),
                        quantity: 2,
                        max_quantity: 4,
                        regrowth_per_tick: 1,
                    });
                }
            }
            _ => {
                objs.push(GridObject {
                    id: format!("road_stone_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Other,
                    name: "路旁歇腳石".to_string(),
                    quantity: 1,
                    max_quantity: 1,
                    regrowth_per_tick: 0,
                });
                if roll % 2 == 0 {
                    objs.push(GridObject {
                        id: format!("road_herb_{}_{}", coord.x, coord.y),
                        kind: GridObjectKind::Flora,
                        name: "路側野雜草".to_string(),
                        quantity: 3,
                        max_quantity: 6,
                        regrowth_per_tick: 1,
                    });
                }
            }
        }
    }

    objs
}

pub fn generate_cell_objects(coord: SquareCoord, terrain: Terrain) -> Vec<GridObject> {
    if terrain == Terrain::WaterDeep {
        return Vec::new();
    }

    let (zone, _, _) = classify_wilderness_cell(coord.x, coord.y);

    if terrain == Terrain::Road || terrain == Terrain::Bridge {
        let is_bay_bridge = coord.y == 160 && (-330..=-260).contains(&coord.x);
        let actual_zone = if is_bay_bridge { "天然港灣道" } else { zone };
        return generate_road_cell_objects(coord, actual_zone, terrain == Terrain::Bridge);
    }

    let mut objs = Vec::new();

    // 專屬人工田圃產出
    if terrain == Terrain::HerbGarden {
        objs.push(GridObject {
            id: format!("herbs_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Flora,
            name: "藥圃光葉草".to_string(),
            quantity: 8,
            max_quantity: 12,
            regrowth_per_tick: 2,
        });
        return objs;
    } else if terrain == Terrain::FarmField {
        objs.push(GridObject {
            id: format!("grain_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Flora,
            name: "豐茂水稻".to_string(),
            quantity: 10,
            max_quantity: 15,
            regrowth_per_tick: 2,
        });
        return objs;
    } else if terrain == Terrain::DryField {
        objs.push(GridObject {
            id: format!("wheat_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Flora,
            name: "耐旱麥穗".to_string(),
            quantity: 8,
            max_quantity: 12,
            regrowth_per_tick: 2,
        });
        return objs;
    } else if terrain == Terrain::EmptyFurrow {
        objs.push(GridObject {
            id: format!("soil_{}_{}", coord.x, coord.y),
            kind: GridObjectKind::Other,
            name: "濕潤沃土".to_string(),
            quantity: 4,
            max_quantity: 8,
            regrowth_per_tick: 1,
        });
        return objs;
    }

    let seed = ((coord.x as i64 * 374761393) ^ (coord.y as i64 * 668265263)).unsigned_abs();
    let roll = seed % 100;

    // 1. 主要正典採集資源（保證必定產出 1 件，roll % 3 == 0 時額外產出第 2 件伴生資源）
    match zone {
        "長風草甸" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("鐵線長草", "herb", 4, 8)
            } else {
                ("長風牧草", "meadow_grass", 5, 10)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "日照穀木".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "古河巨木林" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("巨木樹脂", "resin", 5, 10)
            } else {
                ("落木枯枝", "fallen_wood", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "溫木良材".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "赤石坡" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("赤鐵礦露頭", "iron_ore", 6, 12)
            } else {
                ("赤紋鐵礦", "ore", 5, 10)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ore_{}_{}_2", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "伴生銅砂".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "落日岬" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("防風韌竹", "wood", 5, 10)
            } else {
                ("海岬迎風草", "herb", 3, 6)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("sand_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "海角石英砂".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "驚濤岩岸" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("海蝕奇石", "ore", 5, 10)
            } else {
                ("硬殼海貝", "shell", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("seaweed_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "潮平海藻".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "巨藻淺灣" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("巨藻鮮葉", "kelp_leaf", 5, 10)
            } else {
                ("深海巨藻藤", "herb", 6, 12)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 2,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "夜光貝母".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "黑石灣" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("玄武岩塊", "basalt_block", 5, 10)
            } else {
                ("黑曜玄武岩", "ore", 6, 12)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ore_{}_{}_2", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "海鹽結晶".to_string(),
                    quantity: 5,
                    max_quantity: 10,
                    regrowth_per_tick: 1,
                });
            }
        }
        "翠冠巨木林" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("鐵心巨木苗", "canopy_sapling", 4, 8)
            } else {
                ("光葉藥草", "herb", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("wood_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "溫木良材".to_string(),
                    quantity: 5,
                    max_quantity: 10,
                    regrowth_per_tick: 1,
                });
            }
        }
        "清泉溪谷" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("寒水玉原石", "ore", 5, 10)
            } else {
                ("溪畔水芹", "creek_herb", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: if prefix == "ore" { GridObjectKind::Mineral } else { GridObjectKind::Flora },
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ore_{}_{}_2", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "溪底亮銀礦".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "北坡雲杉林" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("雲杉脂松果", "spruce_cone", 5, 10)
            } else {
                ("雲杉原木", "wood", 6, 12)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("herb_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "松脂結晶".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "西麓林道" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("野生山茶籽", "wild_fruit", 4, 8)
            } else {
                ("光葉藥草", "herb", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("herb_{}_{}_2", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "白鮮草".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "淺澤泥口" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("柔韌蘆葦", "herb", 5, 10)
            } else {
                ("沼生烏木", "wood", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 2,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("peat_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "澤地泥炭".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "白茫洲" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("白茫避瘴草", "fog_herb", 4, 8)
            } else {
                ("止血水苔", "herb", 5, 10)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 2,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ore_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "白茫霧晶".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "貼山深潭" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("深潭冷鐵礦", "ore", 5, 10)
            } else {
                ("水底奇石", "water_stone", 4, 8)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("water_moss_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "寒潭水藻".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "青籠關" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("青脊重礦", "ore", 6, 12)
            } else {
                ("金剛砂石", "sand", 5, 10)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("flint_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "關山引火石".to_string(),
                    quantity: 4,
                    max_quantity: 8,
                    regrowth_per_tick: 1,
                });
            }
        }
        "西嶺礦谷" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("赤紋煤石", "coal", 6, 12)
            } else {
                ("耐火紅黏土", "clay", 5, 10)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("marrow_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "山心石髓".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "東海峭壁" => {
            let (name, prefix, kind, qty, max_qty) = if roll % 2 == 0 {
                ("風切石原礦", "ore", GridObjectKind::Mineral, 5, 10)
            } else {
                ("峭壁海燕窩", "wild_fruit", GridObjectKind::Flora, 3, 6)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("cliff_grass_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Flora,
                    name: "絕壁迎風草".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "雲腰雪林" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("耐寒鐵杉", "wood", 5, 10)
            } else {
                ("冰晶雪蓮", "herb", 3, 6)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("ice_crystal_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "雪嶺冰魄晶".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        "白塔雪巔" => {
            let (name, prefix, qty, max_qty) = if roll % 2 == 0 {
                ("極頂星紋石", "ore", 3, 6)
            } else {
                ("萬年冰髓", "ore_2", 2, 5)
            };
            objs.push(GridObject {
                id: format!("{}_{}_{}", prefix, coord.x, coord.y),
                kind: GridObjectKind::Mineral,
                name: name.to_string(),
                quantity: qty,
                max_quantity: max_qty,
                regrowth_per_tick: 1,
            });
            if roll % 3 == 0 {
                objs.push(GridObject {
                    id: format!("holy_ice_{}_{}", coord.x, coord.y),
                    kind: GridObjectKind::Mineral,
                    name: "神山冰晶".to_string(),
                    quantity: 3,
                    max_quantity: 6,
                    regrowth_per_tick: 1,
                });
            }
        }
        _ => {
            objs.push(GridObject {
                id: format!("wild_herb_{}_{}", coord.x, coord.y),
                kind: GridObjectKind::Flora,
                name: "野地百草".to_string(),
                quantity: 4,
                max_quantity: 8,
                regrowth_per_tick: 1,
            });
        }
    }

    // 2. 次要生態／獵徑／野外生存節點（必定依座標散列產出 1 件）
    let secondary_seed = ((coord.x as i64 * 2246822519) ^ (coord.y as i64 * 3266489917)).unsigned_abs();
    let secondary_roll = secondary_seed % 5;
    let (sec_kind, sec_name, sec_prefix, sec_qty, sec_max, sec_regen) = match secondary_roll {
        0 => (GridObjectKind::Other, "獸徑足印", "tracks", 1, 1, 0),
        1 => (GridObjectKind::Other, "野地營火", "campfire", 5, 10, 1),
        2 => (GridObjectKind::WaterSource, "山澗清泉眼", "spring", 8, 12, 2),
        3 => (GridObjectKind::Other, "避風岩龕", "cairn_niche", 1, 1, 0),
        _ => (GridObjectKind::Other, "巡山木樁", "hunter_post", 1, 1, 0),
    };
    objs.push(GridObject {
        id: format!("{}_{}_{}", sec_prefix, coord.x, coord.y),
        kind: sec_kind,
        name: sec_name.to_string(),
        quantity: sec_qty,
        max_quantity: sec_max,
        regrowth_per_tick: sec_regen,
    });

    objs
}

// ─── 道路與主要聯絡線 ───────────────────────────────────────────────

type RoadMap = HashMap<SquareCoord, (Terrain, &'static str, &'static str)>;

fn connect_smooth_segment(
    map: &mut RoadMap,
    from: (i32, i32),
    to: (i32, i32),
    terrain: Terrain,
) {
    let mut x = from.0;
    let mut y = from.1;
    map.entry(SquareCoord::new(x, y))
        .or_insert((terrain, "", ""));

    let dx = (to.0 - x).abs();
    let dy = (to.1 - y).abs();
    let sx = if to.0 > x { 1 } else { -1 };
    let sy = if to.1 > y { 1 } else { -1 };
    let mut err = dx - dy;

    while x != to.0 || y != to.1 {
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
            map.entry(SquareCoord::new(x, y))
                .or_insert((terrain, "", ""));
        }
        if x == to.0 && y == to.1 {
            break;
        }
        if e2 < dx {
            err += dx;
            y += sy;
            map.entry(SquareCoord::new(x, y))
                .or_insert((terrain, "", ""));
        }
    }
}

/// 依序列航點平滑鋪設自然蜿蜒之道路網，消除生硬的百格折角
fn connect_winding_road(
    map: &mut RoadMap,
    waypoints: &[(i32, i32)],
    terrain: Terrain,
) {
    for window in waypoints.windows(2) {
        connect_smooth_segment(map, window[0], window[1], terrain);
    }
}

/// 生成連通 15 座聚落與 19 處獵場的自然蜿蜒道路與橋樑網。
/// 道路格的實際 zone 與「區名・道路」名稱於 `generate_white_tower_grid` 依 `classify_wilderness_cell` 統一賦予。
fn generate_road_network() -> HashMap<SquareCoord, (Terrain, &'static str, &'static str)> {
    let mut map = HashMap::new();

    // 1. 中軸北向：文曜 (0, 18) -> 老堰頭 (0, 125) -> 翠冠巨木林 (0, 240) -> 白塔雪巔 (0, 420)
    connect_winding_road(&mut map, &[(0, 0), (0, 18), (0, 24), (8, 52), (12, 84), (5, 108), (0, 116), (0, 125)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 125), (0, 135), (6, 164), (10, 198), (5, 224), (0, 240)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 240), (10, 282), (16, 332), (8, 378), (0, 406), (0, 420)], Terrain::Road);

    // 2. 中軸南向：文曜 (0, -18) -> 南哨 (0, -220) -> 淺澤泥口 (0, -340) -> 青籠關 (0, -450) -> 南下古隘道 (-4, -475) -> (0, -499)
    connect_winding_road(&mut map, &[(0, 0), (0, -18), (0, -24), (-10, -68), (-16, -118), (-8, -172), (0, -208), (0, -213), (0, -220)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -220), (0, -228), (0, -234), (-8, -268), (-12, -304), (0, -334), (0, -340)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -340), (6, -376), (10, -412), (0, -442), (0, -450)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -450), (-4, -475), (0, -499)], Terrain::Road);

    // 3. 西北西麓林道與風舶雙港：文曜 (-18, 0) -> 西麓林道 (-130, 80) -> 風舶角 (-260, 160) -> 風舶嶼 (-330, 160) -> 驚濤岩岸 (-360, 180) -> 風櫃尾 (-360, 95)
    connect_winding_road(&mut map, &[(0, 0), (-18, 0), (-63, 0), (-88, 16), (-112, 48), (-130, 80), (-158, 98), (-194, 130), (-226, 152), (-245, 160), (-250, 160), (-260, 160)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-270, 160), (-322, 160), (-330, 160)], Terrain::Bridge);
    connect_winding_road(&mut map, &[(-330, 160), (-338, 160), (-344, 160), (-354, 170), (-360, 180)], Terrain::Road);
    connect_winding_road(&mut map, &[(-360, 180), (-354, 152), (-352, 124), (-360, 105), (-360, 95)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-260, 171), (-260, 176), (-272, 206), (-290, 236), (-300, 260)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 160), (-260, 149), (-260, 144), (-246, 112), (-248, 74), (-260, 45), (-260, 35)], Terrain::Road);
    connect_winding_road(&mut map, &[(-260, 35), (-260, 26), (-252, 4), (-260, -20)], Terrain::Road);

    // 4. 西南原（古河巨木林、紅土崁、大圳頭、長風草甸、牛埔、三號坑口、赤石坡、落日岬）
    connect_winding_road(&mut map, &[(-260, -20), (-242, -52), (-212, -82), (-188, -110), (-180, -128), (-180, -133), (-180, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-35, 0), (-55, -28), (-80, -60), (-92, -96), (-85, -128), (-85, -134), (-85, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-85, -140), (-85, -147), (-85, -152), (-78, -166), (-85, -180)], Terrain::Road);
    connect_winding_road(&mut map, &[(-85, -140), (-92, -140), (-96, -140), (-125, -132), (-152, -134), (-168, -139), (-173, -139)], Terrain::Road);
    connect_winding_road(&mut map, &[(-180, -140), (-180, -148), (-180, -152), (-170, -174), (-180, -195), (-190, -218), (-180, -235), (-180, -245)], Terrain::Road);
    connect_winding_road(&mut map, &[(-187, -139), (-194, -139), (-225, -146), (-256, -144), (-274, -140), (-279, -140), (-285, -140)], Terrain::Road);
    connect_winding_road(&mut map, &[(-285, -140), (-285, -147), (-285, -152), (-308, -156), (-326, -148), (-340, -140)], Terrain::Road);

    // 5. 南境黑沼林支線（牛埔 -> 白茫洲 -> 水尾寮 -> 南哨 -> 貼山深潭）
    connect_winding_road(&mut map, &[(-180, -245), (-180, -252), (-180, -256), (-162, -286), (-138, -308), (-120, -320)], Terrain::Road);
    connect_winding_road(&mut map, &[(-120, -320), (-100, -312), (-75, -294), (-75, -287), (-75, -280)], Terrain::Road);
    connect_winding_road(&mut map, &[(-75, -280), (-68, -280), (-64, -280), (-42, -258), (-22, -232), (-12, -220), (-7, -220), (0, -220)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, -220), (7, -220), (14, -220), (48, -252), (86, -288), (120, -320)], Terrain::Road);

    // 6. 東北與青脊山支線（北嶺腳、清泉溪谷、半嶺鎮、霜林口、北坡雲杉林、雲腰雪林、西嶺礦谷、東海峭壁）
    connect_winding_road(&mut map, &[(0, 0), (18, 0), (24, 0), (56, 26), (86, 62), (110, 96), (110, 108), (110, 115)], Terrain::Road);
    connect_winding_road(&mut map, &[(0, 125), (7, 125), (12, 125), (36, 146), (60, 160)], Terrain::Road);
    connect_winding_road(&mut map, &[(60, 160), (78, 140), (94, 115), (103, 115), (110, 115)], Terrain::Road);
    connect_winding_road(&mut map, &[(110, 115), (110, 122), (110, 126), (124, 158), (146, 184), (160, 198), (160, 203), (160, 210)], Terrain::Road);
    connect_winding_road(&mut map, &[(160, 210), (160, 218), (160, 222), (172, 248), (190, 274), (190, 283), (190, 290)], Terrain::Road);
    connect_winding_road(&mut map, &[(190, 290), (190, 297), (190, 302), (180, 320)], Terrain::Road);
    connect_winding_road(&mut map, &[(180, 320), (132, 314), (80, 300)], Terrain::Road);
    connect_winding_road(&mut map, &[(80, 300), (42, 276), (0, 240)], Terrain::Road);
    connect_winding_road(&mut map, &[(110, 108), (118, 98), (148, 68), (180, 40)], Terrain::Road);
    connect_winding_road(&mut map, &[(180, 40), (225, 86), (275, 146), (320, 200)], Terrain::Road);
    connect_winding_road(&mut map, &[(160, 202), (168, 196), (218, 192), (272, 196), (320, 200)], Terrain::Road);
    map
}

/// 根據座標將野外格子歸入正典 19 獵場區域（委派予 `classify_wilderness_cell`）。
#[allow(dead_code)]
fn classify_wilderness_zone(coord: SquareCoord) -> (&'static str, &'static str) {
    let (zone, sub_geo, _) = classify_wilderness_cell(coord.x, coord.y);
    (zone, sub_geo)
}

// ─── 15 座聚落完整鋪設（共 405 棟建築原尺寸攤平）──────────────────────

type SettlementCellMap = HashMap<SquareCoord, GridCell>;

fn generate_all_settlements() -> SettlementCellMap {
    let mut map = SettlementCellMap::new();

    // ── 2 城（各 55 棟建築）─────────────────────────────────────────
    generate_city_wen_yao(&mut map);
    generate_city_feng_bo_jiao(&mut map);

    // ── 4 鎮（各 40 棟建築：10F + 30R）─────────────────────────────
    generate_town_feng_bo_yu(&mut map);
    generate_town_da_zhun_tou(&mut map);
    generate_town_ban_ling(&mut map);
    generate_town_nan_shao(&mut map);

    // ── 9 村（各 15 棟建築：5F + 10R）──────────────────────────────
    generate_village_lao_yan_tou(&mut map);
    generate_village_bei_ling_jiao(&mut map);
    generate_village_shuang_lin_kou(&mut map);
    generate_village_feng_gui_wei(&mut map);
    generate_village_wu_shi_bi(&mut map);
    generate_village_niu_pu(&mut map);
    generate_village_hong_tu_kan(&mut map);
    generate_village_san_hao_keng_kou(&mut map);
    generate_village_shui_wei_liao(&mut map);

    map
}

// ─────────────────────────────────────────────────────────────────
// 城鎮村具體佈局產生器
// ─────────────────────────────────────────────────────────────────

/// 組合聚落名稱與場所/建築名稱，徹底消除任何「聚落・聚落...」雙重前綴
fn format_settlement_cell_name(settlement_name: &str, raw_name: &str) -> String {
    let mut s = raw_name;
    while let Some(rest) = s.strip_prefix(settlement_name) {
        s = rest.strip_prefix('・').unwrap_or(rest);
    }
    format!("{}・{}", settlement_name, s)
}

/// 依據建築名稱與類型產生自然、具生活煙火氣之場所描述
fn generate_settlement_building_desc(settlement_name: &str, bldg_name: &str, terrain: Terrain) -> String {
    if bldg_name.contains("三水總商行") {
        format!("{settlement_name}全島通商貿易與鎂幣匯兌調度的總商行總部，高敞的匯兌大廳內計數板與鋼印聲清脆，櫃檯陳列著防偽鋼印、通行鎂幣存摺與跨鎮通兌匯票，秩序井然。")
    } else if bldg_name.contains("傳訊鐘樓") {
        format!("{settlement_name}高聳厚重的傳訊鐘樓與信使總部，頂層懸掛報時青銅巨鐘、日晷與圭表，正午子夜鳴鐘校時；樓前疾行草獸快騎隨時備鞍出發，將緊急手信與公文發往全島。")
    } else if bldg_name.contains("學院主校區") || bldg_name.contains("開筆大講堂") || (bldg_name.contains("文曜學院") && !bldg_name.contains("觀星台")) {
        format!("{settlement_name}學院的核心開筆講堂殿堂，高大的杉木書架整齊排列，講堂前方的烏木教案上擱著黃銅鋼筆與講義，及齡學生在此行開筆之禮、受贈鋼尖沾水筆與記事手札。")
    } else if bldg_name.contains("曆算講堂") || bldg_name.contains("四時星曆") {
        format!("{settlement_name}講授天文曆算與星度推步之講堂，室內立著黃銅渾天星軌推步儀，前方烏木教案上擺著教案上的黃銅鋼筆與推步講義，學生在此撥動黑檀計數板核對時序。")
    } else if bldg_name.contains("觀星台") || terrain == Terrain::Observatory {
        format!("{settlement_name}開闊高敞的學院觀星台，青石度盤上立著精工鍛造的三百六十星軌銅儀，金光隱現，用於測算及齡學生的本命氣息與星度交角，內設評議議事廳。")
    } else if bldg_name.contains("藏書閣") || bldg_name.contains("印行館") {
        format!("{settlement_name}典藏古卷與編纂地方誌的印行書閣，高聳書架間陳列著《白塔風物誌》與歷期《半島旬報》，長案上整齊擺放著活字木框、拓碑微雕筆與竹綿紙手札。")
    } else if bldg_name.contains("商算") || bldg_name.contains("算學舍") || bldg_name.contains("商務會計") {
        format!("{settlement_name}傳習籌算與帳冊的教室，四壁架上整齊陳列著各地行商貨價公道帳與會計圖表，案几上擺放著烏木便攜計數板與核算圖表。")
    } else if bldg_name.contains("診所") || terrain == Terrain::Clinic {
        format!("{settlement_name}幽靜清涼的脈絡診所，診案上整齊陳列著青鹽薄荷潔牙粉、黑沼水苔止血藥與百草溫灸盒，紫銅火爐溫著熱水，滿室草木清香。")
    } else if bldg_name.contains("剪剃") || bldg_name.contains("理髮") {
        format!("{settlement_name}講堂街的老字號理容鋪，設有紫銅溫水管木躺椅，木架上陳列著精鋼摺疊剃刀、磨刀布與乾淨的蜂格棉毛巾，街坊常在此閒話家常。")
    } else if bldg_name.contains("第一鍛造") || (bldg_name.contains("岑氏") && bldg_name.contains("鍛造")) {
        format!("{settlement_name}爐火通紅的岑氏第一鍛造工坊，千度火爐烈焰灼灼，巨大的鑄鐵砧與風箱井然排列，架上懸掛著剛出爐的耐磨獵具、農具與各色鍛錘。")
    } else if bldg_name.contains("東橋旅社") {
        format!("{settlement_name}臨水三層的東橋旅社與茶屋，一樓茶座散發著烘青茶與糕點香氣，窗邊擺放著烏木白貝圍棋盤，樓上客房配備夜光貝母調光台燈，安詳恬適。")
    } else if bldg_name.contains("巧木居所") || (bldg_name.contains("巧木") && bldg_name.contains("傢俱")) {
        format!("{settlement_name}散發著老雲杉天然樹脂芬芳的木工坊，工作台上陳列著導能刨刀、角尺與各式雕花鑿，四處整齊堆放著防蟲松香木大床與寒水玉保鮮石櫃的半成品。")
    } else if bldg_name.contains("雲錦") {
        format!("{settlement_name}布匹織造與成衣名坊，高大的織布機與裁剪案几整齊陳列，架上展示著塗布防水樹膠的防雨風衣與五色絲線刺繡的精美霞帔。")
    } else if bldg_name.contains("習武堂") || bldg_name.contains("試招") {
        format!("{settlement_name}分家巷剛勁樸實的練武院落，四角立著包革木人樁與沙袋，兵器架上橫陳著試招木刀、護腕與防身革甲。")
    } else if bldg_name.contains("診所") || bldg_name.contains("醫館") {
        format!("{settlement_name}分家巷石砌診所，藥櫃陳列著祛寒薑黃艾草油膏與山風草安神茶包，紫銅火爐溫著熱水，滿室草木清香。")
    } else if bldg_name.contains("烘藥房") || bldg_name.contains("百草") {
        format!("{settlement_name}分家巷溫氏專司焙製草藥之工坊，紅土炭火石灶烘烤著自北嶺採集的新鮮七星草與祛寒艾絨，藥香醇厚四溢。")
    } else if bldg_name.contains("公議堂") && !bldg_name.contains("前街") && !bldg_name.contains("契書局") {
        format!("{settlement_name}巍峨莊嚴的公議堂石殿，正階開闊，堂內掌管全島戶籍登譜、房地契書簽核與民事公議，是全島規矩與誠信之基石。")
    } else if bldg_name.contains("西門車馬聯運總站") || (bldg_name.contains("西門") && bldg_name.contains("車馬")) {
        format!("{settlement_name}西門車馬與駝獸聯運總部，開闊的車馬埕上停駐著整裝待發的雙輪駝獸大車，調度案前整齊收納著通往全島各鎮的路引票單。")
    } else if bldg_name.contains("韓氏聯運") || bldg_name.contains("貨運行") {
        format!("{settlement_name}聯運貨物轉運分流行，庫房堆滿自牛埔、大圳頭運抵的重貨箱與乾草捆，店員們過磅核單，井然有序。")
    } else if bldg_name.contains("修傘") || bldg_name.contains("補靴") {
        format!("{settlement_name}西門街角的老手藝攤鋪，老匠人手腳麻利地削竹更換油布晴雨傘骨，以粗蠟線細密密縫補防浸皮靴，手藝扎實耐用。")
    } else if bldg_name.contains("弄堂") || bldg_name.contains("天井") {
        format!("{settlement_name}三水口青石石板鋪就的弄堂天井與公井，高搭韌竹晾衣架翻曬被褥，清泉從石槽湧出，街坊鄰里清晨常在此汲水寒暄。")
    } else if bldg_name.contains("海味") || bldg_name.contains("乾貨") {
        format!("{settlement_name}三水口臨河的乾貨鋪子，吊掛著成串的鹽烤海苔片、酸甜山果乾、壺底黑豆醬油與紅土崁黑糖，海味與醬香撲鼻。")
    } else if bldg_name.contains("蒙學") || bldg_name.contains("國小") {
        format!("{settlement_name}學院啟蒙國小學童的教室，几案低矮平整，壁上懸掛著晨課雲板與千字文掛軸，案頭整齊擺放著沙盤與習字木筆。")
    } else if bldg_name.contains("陸家宅邸") {
        format!("{settlement_name}三水口最具威望的陸家老宅，古樸青石大門刻著通商信條，庭院中老藤攀架，陸氏長輩常在藤椅上品茗聽泉。")
    } else if bldg_name.contains("陸氏清雅書齋") || bldg_name.contains("陸知微書齋") {
        format!("{settlement_name}屋主清雅的私人書齋，案上整齊陳列著四方採風卷帙、拓印石碑與微型雕刻筆，墨香與竹綿紙香滿室飄散。")
    } else if bldg_name.contains("溫家藥圃居") || bldg_name.contains("溫家宅院") {
        format!("{settlement_name}講堂街溫氏宗族石屋，後院整齊分畦栽種著清心薄荷、水苔與祛寒草木，屋簷下掛滿晾曬風乾的藥草束。")
    } else if bldg_name.contains("秦氏算學居") || bldg_name.contains("秦氏會計事務所") {
        format!("{settlement_name}會計師素雅的私人居所，書齋四壁皆是各地行商貨價公道帳與會計圖表，案上計數板溫潤潔淨。")
    } else if bldg_name.contains("岑氏匠師宗宅") || bldg_name.contains("岑守岳宗宅") || bldg_name.contains("岑家大房") {
        format!("{settlement_name}分家巷岑家大房老宅，條石深厚，門前放著鍛打石砧與整齊柴火，屋內陳列著歷代匠人改良的導能農具與火磚樣品。")
    } else if bldg_name.contains("柯氏木作居") || bldg_name.contains("柯成器木作居") {
        format!("{settlement_name}木作匠人的雅致居所，玄關擺放著齒輪平直畫線儀與各式夾鋼榫卯刨刀，桌椅門窗無不榫卯嚴整、天然生潤。")
    } else if bldg_name.contains("蘇氏繡坊居") || bldg_name.contains("蘇錦心繡坊居") {
        format!("{settlement_name}繡坊女主人的生活雅居，窗明几淨，繃架上繃著正待完工的精緻錦繡霞帔，針線盒內五色絲線流光溢彩。")
    } else if bldg_name.contains("韓氏總站宅院") || bldg_name.contains("韓鐵山宅院") || bldg_name.contains("韓家宅院") {
        format!("{settlement_name}聯運站管事的樸實宅邸，院角整齊碼放著修理大車的厚木輪轂與革套，整潔寬敞。")
    } else if bldg_name.contains("陳氏庖廚居") {
        format!("{settlement_name}大食堂掌勺師傅的家宅，灶上常年溫著砂鍋高湯，案頭陳列著數十種香料瓷罐，充滿濃郁敦實的生活煙火氣。")
    } else if bldg_name.contains("沈氏臨水居") {
        format!("{settlement_name}東橋畔的雅致臨水小樓，推窗即見三水碧波，案上置黑沼烏木與風舶白貝圍棋盤，茶香伴隨水聲寧靜致遠。")
    } else if bldg_name.contains("白氏湯屋居") {
        format!("{settlement_name}浴場主管的溫暖住家，引溫泉餘熱鋪地，滿室乾燥溫潤，簷下常曬著洗淨烘乾的厚長絨蜂格棉面巾。")
    } else if bldg_name.contains("岑氏鍛藝居") || bldg_name.contains("岑拓鍛藝居") {
        format!("{settlement_name}分家巷青年匠師的研藝居所，室內擺放著拆解的水栓閥芯與新式器物圖樣，充滿探索鑽研的朝氣。")
    } else if bldg_name.contains("岑氏武勇舍") || bldg_name.contains("岑雲野武勇舍") {
        format!("{settlement_name}習武堂武師的質樸居室，牆上橫掛著兩柄包革試招木刀與護腕，窗前立著石鎖，剛健整潔。")
    } else if bldg_name.contains("老水夫舍") {
        format!("{settlement_name}三水口河畔老船工的住屋，屋簷下掛著乾燥魚網與巨藻船纜，屋內擺著竹編火籠與防浪竹笠。")
    } else if bldg_name.contains("票號總管居") {
        format!("{settlement_name}三水總商行帳房主管的住宅，書案上放著公道銅尺與鋼印泥盒，門戶端整嚴密。")
    } else if bldg_name.contains("信使快騎宿所") {
        format!("{settlement_name}傳訊鐘樓急件信使休宿院，備有烘乾皮靴架與厚帆布風衣，常有信使在此換裝整裝。")
    } else if bldg_name.contains("晨汲青石宅") {
        format!("{settlement_name}緊鄰三水口公井的街坊石屋，門口青石被歲月磨得溫潤發亮，晨昏汲水洗衣聲不絕於耳。")
    } else if bldg_name.contains("河畔迎客小院") {
        format!("{settlement_name}三水口迎候四方商客的臨水小院，窗臨河道，花木扶疏，常飄出清淡的迎客茶香。")
    } else if bldg_name.contains("蒙學陸先生舍") || bldg_name.contains("陸老師宿舍") {
        format!("{settlement_name}學院國小老師的雅室，案上整齊碼放著《白塔基礎讀本》，壁上懸掛晨課雲板。")
    } else if bldg_name.contains("抄經校勘齋") {
        format!("{settlement_name}印行館校勘工匠居處，桌前擺著弧形吸墨器、金箔紙與細毛刷，滿室皆是墨香。")
    } else if bldg_name.contains("石硯墨香居") {
        format!("{settlement_name}講堂街專事研墨製硯工匠之家，庭中蓄水洗硯，案上陳列著各色黑松脂墨與細石硯台。")
    } else if bldg_name.contains("精工火磚匠廬") {
        format!("{settlement_name}專造導能耐火磚的匠人居所，後院堆著各色黏土與磚坯樣板，磚香撲鼻。")
    } else if bldg_name.contains("金石榫卯木舍") {
        format!("{settlement_name}分家巷木工與五金學徒的合宿之所，工具整齊懸於木架，少年匠人在此切磋手藝。")
    } else if bldg_name.contains("駝獸牧夫宿所") {
        format!("{settlement_name}西門聯運駝獸牧伕的住舍，屋外懸掛著大鬃毛刷與草料韁繩，溫暖厚實。")
    } else if bldg_name.contains("遠行行商棧舍") {
        format!("{settlement_name}隨車押貨行商的歇腳居舍，行囊整齊疊放在通鋪旁，旅客在此互通各地物產市況。")
    } else if bldg_name.contains("大車修理工寮") {
        format!("{settlement_name}維修雙輪重型貨車的工匠宿所，屋前散落著堅韌雲杉軸心與生鐵輪箍，鐵器鏗鏘。")
    } else if bldg_name.contains("皮靴油布匠居") {
        format!("{settlement_name}修傘補靴老匠人的居所，廊下掛著浸透防雨桐油的竹骨雨傘，案頭擺滿蠟線與裁皮刀。")
    } else if bldg_name.contains("行道車馬行舍") {
        format!("{settlement_name}西門車隊管事居所，門口停著調度輕便板車，店長／主事正對著行路簿盤點日程。")
    } else if bldg_name.contains("聯運公秤舍") {
        format!("{settlement_name}西門過磅公秤所，立著大生鐵地秤與砝碼，為進出城大宗貨物校準分量。")
    } else if bldg_name.contains("草料客商居") {
        format!("{settlement_name}販售甜苜蓿草料行商的屋舍，堆疊如山的青綠乾草捆散發著濃郁田野清香。")
    } else if bldg_name.contains("大食堂") || bldg_name.contains("食堂") {
        format!("{settlement_name}熱鬧的食堂，嵌著蓄熱重礦底座的紅鐵大釜正冒著熱氣，早上有現磨溫米漿、淋黑糖漿的熱豆花、蔥香米烙餅與牛埔鮮肉包，午晚則有慢燉原牛肉湯、紫銅鴛鴦火鍋與切仔湯麵配滷蛋，蒸籠裡常備著熱氣騰騰的桂花新米糕與黑芝麻湯圓；桌邊擺著壺底黑豆醬油與白釉瓷碗，牆角寒水玉保鮮石櫃裡冰鎮著豆腐、鮮乳與寒水玉冰鎮仙草凍。")
    } else if bldg_name.contains("浴場") || bldg_name.contains("浴堂") || bldg_name.contains("石湯") || terrain == Terrain::Bathhouse {
        format!("{settlement_name}引流山泉活水的溫水浴場，池底鋪著溫潤的寒水玉、池壁嵌有恆溫導能火磚，水裡飄著祛寒草木湯包的香氣；盥洗台亮銀玻璃鏡前擺滿了青鹽薄荷潔牙粉、軟毛竹柄牙刷、山茶松脂香皂、草木灰柑橘洗潔皂與柔軟的蜂格棉面巾，牆邊立著烘乾衣物的松炭溫風烘衣櫥。")
    } else if bldg_name.contains("服飾") || bldg_name.contains("織造") {
        format!("{settlement_name}陳列各色衣著與隨身小物的服飾坊，架上掛著細棉麻透氣襯衫、北山雪狼絨禦寒冬衣與塗了黑沼樹膠的防雨風衣，櫃台還擺著雙扣帆布斜背包、軟牛皮零錢夾、隨手五色針線盒、軟底居家布拖鞋與竹骨晴雨摺疊傘。")
    } else if bldg_name.contains("傢俱") || bldg_name.contains("巧木") {
        format!("{settlement_name}飄著淡淡松木清香的傢俱工坊，展示著繃好彈力床網的松香木大床、衣櫃、鑲貝書案與內嵌北山寒水玉薄板的保鮮石櫃，架上還擺著旋蓋夜光貝母調光台燈、沉水烏木茶盤、烏木筷與松香木梳。")
    } else if bldg_name.contains("藏書閣") || bldg_name.contains("圖誌館") || bldg_name.contains("書局") || terrain == Terrain::Library {
        format!("{settlement_name}收納古卷、地方風物誌與山海圖冊的書局藏書處，架上擺著竹綿紙印製的《白塔風物誌》、每旬發行的《半島旬報》、白話故事小冊，桌邊還備有竹綿紙記事手札與鋼尖沾水筆。")
    } else if bldg_name.contains("鍛造") || bldg_name.contains("鐵器") || terrain == Terrain::Blacksmith {
        format!("{settlement_name}爐火通紅的鍛造工坊，除了打製獵具與農具，架上也擺滿了家家戶戶必備的黃銅水栓、精鋼摺疊剃刀、防潮紅磷火柴、輕便隨身保溫瓶內膽與小巧的黃銅發條座鐘、懷錶。")
    } else if bldg_name.contains("商行") || bldg_name.contains("商盟") || terrain == Terrain::Bank {
        format!("{settlement_name}負責大宗物資買賣、海事保單與帳契結算的商行，廳堂寬敞明亮，掌事人員正忙著核對貨單、兌付軟牛皮錢夾裡的商行票據與調度各地來貨。")
    } else if bldg_name.contains("車馬聯運") || bldg_name.contains("聯運") || terrain == Terrain::Caravanserai {
        format!("{settlement_name}往來各城鎮的車馬與駝獸聯運站，棚下停著載貨大車與健壯的原牛駝獸，旅客背著雙扣帆布斜背包、提著竹骨晴雨摺疊傘與隨身保溫瓶上下車。")
    } else if bldg_name.contains("旅社") || terrain == Terrain::Inn {
        format!("{settlement_name}乾淨舒適的旅社，一樓茶屋飄著鍋煮鮮奶茶、冰鎮甘蔗檸檬汁、奶油酥餅、黑糖米香與桂花新米糕的甜香，茶客正對弈烏木白貝圍棋、翻看《半島旬報》；樓上獨立銅鎖單人客房鋪著蓬鬆羽絨暖被與山風草安神枕，桌頭立著旋蓋夜光貝母台燈。")
    // ── 區域聚落專屬特色建築描述（針對 14 個非文曜聚落） ──
    } else if bldg_name.contains("風櫃尾燈塔") {
        format!("{settlement_name}高聳巍峨的防風海岬燈塔，塔頂安裝著精工鍛造的長明海燈與聚光銅鏡，強烈光束穿透夜霧，為往來風舶海域的舟楫指引平安航道。")
    } else if bldg_name.contains("冷燻") {
        format!("{settlement_name}海風強勁的水產冷燻工坊，室內整齊掛滿深海大魚與石斑，匠人以乾燥松屑燃起微煙緩慢冷燻，魚香撲鼻，乾爽耐久。")
    } else if bldg_name.contains("鑿石") || (bldg_name.contains("玄武岩") && bldg_name.contains("工坊")) {
        format!("{settlement_name}沉重剛毅的玄武岩採鑿石坊，匠人們揮動大鋼鎬開鑿烏石重錨與防波堤石料，火星迸濺，石屑紛飛。")
    } else if bldg_name.contains("灰窯") || bldg_name.contains("海貝灰") {
        format!("{settlement_name}以巨大海貝殼高溫燒製防水灰漿的灰窯，熱氣騰騰，出產的灰漿堅密防滲，為港埠防波堤與城牆不可或缺之建材。")
    } else if bldg_name.contains("石磨工坊") || bldg_name.contains("水輪石磨") {
        format!("{settlement_name}引流老堰頭急湍活水驅動的巨大石磨工坊，水輪轟鳴旋轉，成排石磨精細碾磨新收稻穀、大豆與乾燥藥材，米香與豆香洋溢。")
    } else if bldg_name.contains("鋸木") || bldg_name.contains("水車鋸木") {
        format!("{settlement_name}高山急流帶動巨大排鋸的原木鋸木坊，巨大的雲杉巨木在水力牽引下被平整剖開，清新的松木樹脂芬芳飄散在整座山谷。")
    } else if bldg_name.contains("切石") || bldg_name.contains("寒水玉切石") {
        format!("{settlement_name}利用山泉落差驅動銀砂水力切石機的工坊，高速運轉的水輪將堅韌的寒水玉原石平整剖切，打磨出溫潤潔淨的玉石磚板與隨身握石。")
    } else if bldg_name.contains("氣象觀測") || bldg_name.contains("測風儀") {
        format!("{settlement_name}高踞岩台的氣象觀測所，青銅測風儀在山風中飛速旋轉，觀測師細緻記錄風速、雲層與氣壓，隨時發送高山風雪預警。")
    } else if bldg_name.contains("農牧總社") {
        format!("{settlement_name}掌管西南原廣袤農牧拓墾之調度中心，開闊大廳掛著各區田畝水利圖冊與草場輪牧日程，農人在此協調農具與肥料分配。")
    } else if bldg_name.contains("乳酪") || bldg_name.contains("羊毛與乳酪") {
        format!("{settlement_name}向陽草甸旁的乳品精製坊，巨大的蓄熱紅鐵鍋慢火濃縮原牛乳汁，木架上整齊陳列著金黃硬實的熟成乳酪塊與壓製厚毛氈。")
    } else if bldg_name.contains("選礦") || bldg_name.contains("選礦場") {
        format!("{settlement_name}階地旁的重型選礦場，坡道上流水沖洗各色礦石，工人們以鐵槌敲選純淨赤紋鐵礦與伴生銅砂，分級裝入大車運往工坊。")
    } else if bldg_name.contains("坑口") || bldg_name.contains("礦務所") || bldg_name.contains("台車維修") {
        format!("{settlement_name}深山礦谷咽喉的礦務作業所與台車棚，重型鋼軌與手搖絞盤整齊架設，通風設備呼呼作響，坑工們在此檢點礦燈與安全支柱。")
    } else if bldg_name.contains("高腳木屋") || bldg_name.contains("竹筏埠") {
        format!("{settlement_name}以粗壯烏木打樁避開積水與泥甲鱷獸的架高聚落，平底竹筏繫於木樁旁，通風涼爽，水草與魚鮮香氣撲面。")
    } else if bldg_name.contains("公所") || bldg_name.contains("公事房") || bldg_name.contains("公議所") || bldg_name.contains("公議廳") || terrain == Terrain::TownHall {
        format!("{settlement_name}凝聚全鄉鎮民心與事務決策之公所，條石樸素堅實，堂內陳列地方墾拓沿革圖與互助規約，居民在此商議公共事務。")
    } else if bldg_name.contains("宅邸") || bldg_name.contains("宅院") || bldg_name.contains("莊園") || bldg_name.contains("別院") || bldg_name.contains("老宅") {
        match settlement_name {
            "風舶角" | "風舶嶼" => format!("{settlement_name}歷代經營海事與造船的世家宅院「{bldg_name}」，以厚重防浪青石與百年雲杉樑柱築成，廳內懸掛著遠洋海圖與夜光羅盤，庭院乾爽開闊。"),
            "大圳頭" | "老堰頭" => format!("{settlement_name}世代主持水利與農耕的宗族大宅「{bldg_name}」，院前活水圳渠清澈流淌，廊下陳列著精良水車構件與翻土重犁，充滿豐饒踏實的田園家風。"),
            "半嶺鎮" | "北嶺腳" | "霜林口" => format!("{settlement_name}依山勢修築的匠師世家老宅「{bldg_name}」，石牆厚實防風，屋內引山泉活水並砌有暖炕，案頭擺放著寒水玉器與山林鍛件。"),
            "南哨" | "水尾寮" => format!("{settlement_name}以千年不腐的沼生烏木與條石高架築成的名門宅院「{bldg_name}」，簷下懸掛著避瘴香草束與精鋼獵具，通風防潮、守望相助。"),
            "牛埔" => format!("{settlement_name}向陽草甸上的牧業世家大院「{bldg_name}」，石砌圍牆內設有恆溫乳酪熟成窖與寬敞駝獸棚，微風送來陣陣牧草與乳香。"),
            "紅土崁" | "三號坑口" => format!("{settlement_name}以千度耐火紅磚與堅岩砌成的礦業宗族宅院「{bldg_name}」，庭中陳列著各色赤紋礦石標本與精良選礦工具，門風剛健敦厚。"),
            _ => format!("{settlement_name}深厚質樸的在地家族宅院「{bldg_name}」，建材質料取自周邊山海山石，庭院開闊，生活陳設紮實耐用，洋溢著經年累月的家風溫情。"),
        }
    } else if bldg_name.contains("民宅") || bldg_name.contains("住宅") || bldg_name.contains("公寓") || bldg_name.contains("街屋") || bldg_name.contains("居所") || bldg_name.contains("家屋") || bldg_name.contains("農舍") || bldg_name.contains("宿舍") || terrain == Terrain::House {
        match settlement_name {
            "文曜" => format!("{settlement_name}街區整潔雅致的居民屋舍「{bldg_name}」，青石牆基配著明亮窗櫺，廚房引流山泉活水，桌上點著旋蓋夜光貝母台燈，生活井然安適。"),
            "風舶角" | "風舶嶼" => format!("{settlement_name}臨港防風的石砌海事屋舍「{bldg_name}」，以海貝灰漿封縫抵禦海潮鹽霧，屋簷下掛著防風提燈與巨藻纜繩，屋內乾爽溫暖。"),
            "風櫃尾" | "烏石鼻" => format!("{settlement_name}依海岬岩岸用黑曜玄武岩壘砌的防浪漁家「{bldg_name}」，厚重石牆能擋狂風巨浪，院裡晾曬著漁網與冷燻魚乾，屋內火塘暖意融融。"),
            "大圳頭" | "老堰頭" => format!("{settlement_name}傍著清澈水圳而建的農家屋舍「{bldg_name}」，門前石階可汲引活水洗菜淘米，院角整齊碼放著脫穀木桶與改良農具，飄散著新米與炊煙香氣。"),
            "半嶺鎮" | "北嶺腳" => format!("{settlement_name}順應陡峭山坡修築的石木山居「{bldg_name}」，屋頂壓著防風青石板，廊下備有登山鐵齒釘鞋與背架，室內松脂暖爐烘得滿屋生春。"),
            "霜林口" => format!("{settlement_name}以粗壯北坡雲杉原木榫接而成的林工木屋「{bldg_name}」，空氣中瀰漫著濃郁松脂木香，屋內鋪著厚實防寒毛氈，壁爐柴火霹啪作響。"),
            "南哨" => format!("{settlement_name}高棧坪上以硬木與山石架高的防禦屋舍「{bldg_name}」，門窗裝有防蚊防瘴細紗與堅固木栓，牆上掛著入林開山刀、皮靴與紅磷信號筒。"),
            "水尾寮" => format!("{settlement_name}以耐水烏木高椿架於淺澤之上的高腳木屋「{bldg_name}」，屋下停靠著輕便平底竹筏，窗前掛著蘆葦編簾與止血水苔乾束，涼爽通風。"),
            "牛埔" => format!("{settlement_name}長風草甸向陽坡上的石基厚氈暖屋「{bldg_name}」，屋頂覆著防風壓石，屋內擺著發酵乳酪木桶與厚實羊毛毯，推窗即見無邊青翠草浪。"),
            "紅土崁" | "三號坑口" => format!("{settlement_name}工礦階地上的耐火紅磚與條石屋舍「{bldg_name}」，門前設有沖洗礦泥的石槽與防風礦燈架，屋內火炕溫熱，陳設結實耐用。"),
            _ => format!("{settlement_name}尋常人家的起居居所「{bldg_name}」，依地勢就地取材建造，門口備有防雨竹編信箱與工具架，屋內生著暖爐，陳設質樸安寧。"),
        }
    } else if bldg_name.contains("大船塢") || bldg_name.contains("船材") || bldg_name.contains("造船") || bldg_name.contains("修船") {
        format!("{settlement_name}臨水而建的造船塢，架上停著正在拼合百年雲杉龍骨、絞掛深海巨藻船纜與烏石重錨的大船，空氣裡滿是杉木與桐油的氣味。")
    } else if bldg_name.contains("鐘樓") || bldg_name.contains("信使") || bldg_name.contains("瞭望") || terrain == Terrain::GuardPost {
        format!("{settlement_name}負責報時、守望與收發各地書信的鐘樓信使站，疾行草獸快騎背著防雨郵袋準時發班，居民也常來此校對隨身黃銅懷錶的時辰。")
    } else if bldg_name.contains("學院") || terrain == Terrain::Academy {
        format!("{settlement_name}講授曆算、字韻與百工學問的學院，學生們背著雙扣帆布斜背包、手捧竹綿紙記事手札與鋼尖沾水筆，在綠蔭庭院間切磋討論。")
    } else if bldg_name.contains("公會") || terrain == Terrain::GuildHall {
        format!("{settlement_name}同行聚會、交換山海情報與張貼差事的公會大廳，桌邊坐滿了經驗老到的前輩與喝著隨身保溫瓶熱茶的年輕人。")
    } else if bldg_name.contains("碼頭") || bldg_name.contains("泊位") || bldg_name.contains("竹筏埠") || terrain == Terrain::Dock {
        format!("{settlement_name}船隻與竹筏停靠的碼頭，水手與漁家熟練地拋出巨藻船纜繫船，將剛捕獲的鮮魚、鹽烤海苔與外地貨物搬上岸。")
    } else if bldg_name.contains("鋪") || bldg_name.contains("行") || bldg_name.contains("店") || terrain == Terrain::Market {
        format!("{settlement_name}熱鬧的市集鋪子，攤上擺滿了壺底黑豆醬油、紅土崁黑糖、牛埔硬乳酪、黑糖米香、鹽烤海苔片、酸甜山果乾與各色家用小物，買賣聲此起彼落。")
    } else {
        format!("座落於{settlement_name}的{bldg_name}，屋舍整潔，人來人往。")
    }
}

/// 建立具有外牆與 4 門的城市格局（55 棟建築）
fn build_city_55(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    city_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        55,
        "City must have exactly 55 buildings"
    );

    if city_name == "文曜" {
        let mut inner_slots = Vec::new();
        let mut outer_slots = Vec::new();

        for dx in -17i32..=17i32 {
            for dy in -17i32..=17i32 {
                let coord = SquareCoord::new(cx + dx, cy + dy);

                // 1. 最外圍城牆與四向城門（dx.abs() == 17 || dy.abs() == 17，共 136 格）
                if dx.abs() == 17 || dy.abs() == 17 {
                    if dx == 0 && dy == 17 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "北城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之北城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == 0 && dy == -17 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "南城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之南城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == 17 && dy == 0 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "東城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之東城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else if dx == -17 && dy == 0 {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "西城門"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之西城門，守衛森嚴，商流行人穿梭不絕。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Wall, format_settlement_cell_name(city_name, "實心城牆"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}高大堅固之條石城牆，上覆防禦機關與哨堞，固若金湯。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                }
                // 2. 外環坊區街道與格位（11 <= max(|dx|, |dy|) <= 16，共 648 格）
                else if dx.abs() >= 11 || dy.abs() >= 11 {
                    let is_cross_outer = dx == 0 || dy == 0;
                    let is_ring_outer = dx.abs() == 13 || dy.abs() == 13;
                    let is_district_outer = dx.abs() == 3 || dy.abs() == 3 || dx.abs() == 5 || dy.abs() == 5 || dx.abs() == 8 || dy.abs() == 8;

                    if is_cross_outer || is_ring_outer || is_district_outer {
                        if is_cross_outer {
                            let (terrain, street_name, desc) = if dx == 0 && dy > 0 {
                            match dy {
                                11 => (
                                    Terrain::Road,
                                    "北門外街南段",
                                    "文曜北門外街南段，南接北內門坊，兩旁青石鋪地，商旅車馬穿梭，北望津渡碼頭與商鋪。",
                                ),
                                12 => (
                                    Terrain::Road,
                                    "北門外街船埠口",
                                    "文曜北門外街船埠路口，西側緊鄰三水外坊貨運碼頭，江風送爽，常有水手拉縴運卸重物。",
                                ),
                                13 => (
                                    Terrain::Road,
                                    "北門外十字街口",
                                    "文曜北門外十字大街交叉口，東西橫貫外環道，北望北城門哨塔，四向人潮車馬絡繹不絕。",
                                ),
                                14 => (
                                    Terrain::Road,
                                    "北門外街學院口",
                                    "文曜北門外街學院分院路口，東臨格物輿圖外院，文士教席與行商在此交匯，竹蔭清幽。",
                                ),
                                15 => (
                                    Terrain::Road,
                                    "北門外街北段",
                                    "文曜北門外街北段，平整寬闊的條石驛道延伸向前，兩側修築石質排水溝與栓馬樁。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "北門外街關前",
                                    "文曜北門外街北端關前石坪，正對高大巍峨的北城門，守衛巡卒駐守檢驗，氣度森嚴。",
                                ),
                            }
                        } else if dx == 0 && dy < 0 {
                            match dy {
                                -11 => (
                                    Terrain::Road,
                                    "南門外街北段",
                                    "文曜南門外街北段，北鄰南內門坊，車馬載運工坊各色物資南來北往，地面車轍隱然。",
                                ),
                                -12 => (
                                    Terrain::Road,
                                    "南門外街鍛坊口",
                                    "文曜南門外街鍛坊路口，東側毗鄰重鐵水錘大鍛坊，陣陣洪亮錘擊聲與火星熱氣隱隱傳來。",
                                ),
                                -13 => (
                                    Terrain::Road,
                                    "南門外十字街口",
                                    "文曜南門外十字大街交匯處，東西橫穿外環大道，南通南哨與淺澤泥口，車水馬龍，商旅駐足歇腳。",
                                ),
                                -14 => (
                                    Terrain::Road,
                                    "南門外街車馬口",
                                    "文曜南門外街車馬行路口，西接駝馬車行棧房，長途大車常在此換韁修轅，人聲鼎沸。",
                                ),
                                -15 => (
                                    Terrain::Road,
                                    "南門外街南段",
                                    "文曜南門外街南段，寬闊條石大道筆直向南鋪展，遠山如黛，道旁列植迎客青松。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "南門外街關前",
                                    "文曜南門外街南端關前廣場，前臨高聳堅固的南城門，守兵巡查出入關牒，防守森嚴。",
                                ),
                            }
                        } else if dy == 0 && dx > 0 {
                            match dx {
                                11 => (
                                    Terrain::Road,
                                    "東門外街西段",
                                    "文曜東門外街西段，西依東內門坊，向東通往紙墨印書諸坊，街面潔淨，槐柳成蔭。",
                                ),
                                12 => (
                                    Terrain::Road,
                                    "東門外街紙坊口",
                                    "文曜東門外街紙坊路口，北側緊鄰青檀宣紙工坊，陣陣草木紙漿清香隨風飄拂。",
                                ),
                                13 => (
                                    Terrain::Road,
                                    "東門外十字街口",
                                    "文曜東門外十字街交會處，南北連接外環林蔭大道，向東直望巍峨東城門，行人如織。",
                                ),
                                14 => (
                                    Terrain::Road,
                                    "東門外街機樞口",
                                    "文曜東門外街機樞工坊路口，南側百工器械機坊轉軸聲隱然傳來，工匠運料絡繹不絕。",
                                ),
                                15 => (
                                    Terrain::Road,
                                    "東門外街東段",
                                    "文曜東門外街東段，平坦堅固的青石長街向東延展，臨近東關城門，街面寬廣平整。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "東門外街關前",
                                    "文曜東門外街東端關前石坪，扼守東城門樞紐，東臨江港外陸通道，關防嚴整。",
                                ),
                            }
                        } else {
                            match dx {
                                -11 => (
                                    Terrain::Road,
                                    "西門外街東段",
                                    "文曜西門外街東段，東接西內門坊，往來商賈駝鈴悠揚，大車載著皮毛藥材出入城關。",
                                ),
                                -12 => (
                                    Terrain::Road,
                                    "西門外街貨棧口",
                                    "文曜西門外街貨棧路口，南北兩側商行貨棧林立，堆積著來自西原大地的厚重貨包。",
                                ),
                                -13 => (
                                    Terrain::Road,
                                    "西門外十字街口",
                                    "文曜西門外十字大街交叉點，南北橫貫駝馬外環道，四方商隊匯聚，叫賣與卸貨聲不絕。",
                                ),
                                -14 => (
                                    Terrain::Road,
                                    "西門外街商埠口",
                                    "文曜西門外街商埠路口，臨近駝馬大旅社與皮毛拍賣行，西行商旅在此打點行裝鞍具。",
                                ),
                                -15 => (
                                    Terrain::Road,
                                    "西門外街西段",
                                    "文曜西門外街西段，開闊堅實的石板長街直貫西方，大道兩側建有防風土壘與栓馬欄木。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "西門外街關前",
                                    "文曜西門外街西端關前石坪，西對高大厚重的西城門，長途商旅於此驗引換符，守備謹嚴。",
                                ),
                            }
                        };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc.to_string();
                            cell.explored = true;
                            map.insert(coord, cell);
                        } else if is_ring_outer {
                            let (terrain, street_name, desc): (Terrain, String, String) = if dx < 0 && dy > 0 {
                                let t = if (dx.abs() + dy.abs()) % 4 == 0 { Terrain::Bridge } else { Terrain::Road };
                                if dx == -13 && dy == 13 {
                                    (
                                        t,
                                        "三水外環西北隅".to_string(),
                                        "三水外坊外環西北隅轉角，三水河口水網匯入，波濤粼粼，設有防汛石堤與航標燈樁。".to_string(),
                                    )
                                } else if dy == 13 {
                                    if dx == -8 {
                                        (
                                            t,
                                            "三水外環北道".to_string(),
                                            "文曜三水外坊外環北道，沿著北灣水岸修築，臨水一側泊滿各色漁舟與小火輪艟。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            -1 => "東口", -2 => "大堤段", -3 => "支巷口", -4 => "堤中段",
                                            -5 => "津渡口", -6 => "堤東段", -7 => "水岸段", -9 => "渡口段",
                                            -10 => "內口", -11 => "外段", -12 => "防汛段", -14 => "北西段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            t,
                                            format!("三水外環北{suf}"),
                                            format!("文曜三水外坊外環北道{suf}，沿著水岸修築，波濤粼粼，設有防汛石堤與停泊木樁。"),
                                        )
                                    }
                                } else {
                                    if dy == 8 {
                                        (
                                            t,
                                            "三水外環西道".to_string(),
                                            "文曜三水外坊外環西道，西臨水路河堤，石路平坦寬闊，漁具修補棚架連綿。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            1 => "南口", 2 => "泊船段", 3 => "支巷口", 4 => "水網段",
                                            5 => "修船口", 6 => "河堤段", 7 => "中段", 9 => "水柵段",
                                            10 => "內口", 11 => "外段", 12 => "燈樁段", 14 => "西北段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            t,
                                            format!("三水外環西{suf}"),
                                            format!("文曜三水外坊外環西道{suf}，西臨水路河堤，石路平坦寬闊，漁具修補棚架連綿。"),
                                        )
                                    }
                                }
                            } else if dx > 0 && dy > 0 {
                                if dx == 13 && dy == 13 {
                                    (
                                        Terrain::Road,
                                        "講堂外環東北隅".to_string(),
                                        "講堂外坊外環東北隅轉角，修竹萬竿，古松環抱，幽靜林道交匯於此，清風習習。".to_string(),
                                    )
                                } else if dy == 13 {
                                    if dx == 8 {
                                        (
                                            Terrain::Road,
                                            "講堂外環北道".to_string(),
                                            "文曜講堂外坊外環北道，修竹夾道，林蔭遮蔽，學堂研討之所星羅棋布，書卷氣濃。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            1 => "西口", 2 => "林蔭段", 3 => "支巷口", 4 => "修竹段",
                                            5 => "校舍口", 6 => "竹林段", 7 => "清幽段", 9 => "書香段",
                                            10 => "內口", 11 => "外段", 12 => "研討段", 14 => "東北段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("講堂外環北{suf}"),
                                            format!("文曜講堂外坊外環北道{suf}，修竹夾道，林蔭遮蔽，校舍研討之所星羅棋布。"),
                                        )
                                    }
                                } else {
                                    if dy == 8 {
                                        (
                                            Terrain::Road,
                                            "講堂外環東道".to_string(),
                                            "文曜講堂外坊外環東道，青石大道平闊幽靜，兩旁多見紙坊、墨莊與著述譯館。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            1 => "南口", 2 => "松柏段", 3 => "支巷口", 4 => "譯館段",
                                            5 => "紙坊口", 6 => "碑帖段", 7 => "中段", 9 => "石道段",
                                            10 => "內口", 11 => "外段", 12 => "著述段", 14 => "東南段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("講堂外環東{suf}"),
                                            format!("文曜講堂外坊外環東道{suf}，青石大道平闊幽靜，兩旁多見紙坊、墨莊與著述譯館。"),
                                        )
                                    }
                                }
                            } else if dx > 0 && dy < 0 {
                                if dx == 13 && dy == -13 {
                                    (
                                        Terrain::Road,
                                        "百工外環東南隅".to_string(),
                                        "百工外坊外環東南隅轉角，臨近重工業區，石料砧台與木料排垛堆疊有序。".to_string(),
                                    )
                                } else if dy == -13 {
                                    if dx == 8 {
                                        (
                                            Terrain::Road,
                                            "百工外環南道".to_string(),
                                            "文曜百工外坊外環南道，工坊密集，大水車與鍛錘運轉聲響徹長空，熱力逼人。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            1 => "西口", 2 => "砧台段", 3 => "支巷口", 4 => "大車段",
                                            5 => "水車口", 6 => "爐火段", 7 => "鍛打段", 9 => "重工段",
                                            10 => "內口", 11 => "外段", 12 => "火磚段", 14 => "東南段",
                                            15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("百工外環南{suf}"),
                                            format!("文曜百工外坊外環南道{suf}，工坊密集，石料砧台與木料排垛堆疊有序，熱力逼人。"),
                                        )
                                    }
                                } else {
                                    if dy == -8 {
                                        (
                                            Terrain::Road,
                                            "百工外環東道".to_string(),
                                            "文曜百工外坊外環東道，條石堅厚，運送礦料石胚與巨木的大車絡繹不絕。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            -1 => "北口", -2 => "礦料段", -3 => "支巷口", -4 => "木料段",
                                            -5 => "巨木口", -6 => "石胚段", -7 => "中段", -9 => "重料段",
                                            -10 => "內口", -11 => "外段", -12 => "冶鑄段", -14 => "東北段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("百工外環東{suf}"),
                                            format!("文曜百工外坊外環東道{suf}，條石堅厚，運送礦料石胚與巨木的大車絡繹不絕。"),
                                        )
                                    }
                                }
                            } else {
                                if dx == -13 && dy == -13 {
                                    (
                                        Terrain::Road,
                                        "西門外環西南隅".to_string(),
                                        "西門外坊外環西南隅轉角，視野開闊，設有大隊馱馬調頭石坪與避風擋牆。".to_string(),
                                    )
                                } else if dy == -13 {
                                    if dx == -8 {
                                        (
                                            Terrain::Road,
                                            "西門外環南道".to_string(),
                                            "文曜西門外坊外環南道，商道通暢，西域香料、山珍藥材與駝隊輜重雲集於此。".to_string(),
                                        )
                                    } else {
                                        let suf = match dx {
                                            -1 => "東口", -2 => "馱馬段", -3 => "支巷口", -4 => "商道段",
                                            -5 => "草料口", -6 => "輜重段", -7 => "避風段", -9 => "歇馬段",
                                            -10 => "內口", -11 => "外段", -12 => "香料段", -14 => "西南段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("西門外環南{suf}"),
                                            format!("文曜西門外坊外環南道{suf}，商道通暢，西域香料、山珍藥材與駝隊輜重雲集於此。"),
                                        )
                                    }
                                } else {
                                    if dy == -8 {
                                        (
                                            Terrain::Road,
                                            "西門外環西道".to_string(),
                                            "文曜西門外坊外環西道，大道開闊宏敞，旅社歇所與飲馬水井整齊分佈。".to_string(),
                                        )
                                    } else {
                                        let suf = match dy {
                                            -1 => "北口", -2 => "大路段", -3 => "支巷口", -4 => "棧房段",
                                            -5 => "水井口", -6 => "旅社段", -7 => "中段", -9 => "行道段",
                                            -10 => "內口", -11 => "外段", -12 => "馬隊段", -14 => "西北段",
                                            -15 => "遠段", _ => "關前段",
                                        };
                                        (
                                            Terrain::Road,
                                            format!("西門外環西{suf}"),
                                            format!("文曜西門外坊外環西道{suf}，大道開闊宏敞，旅社歇所與飲馬水井整齊分佈。"),
                                        )
                                    }
                                }
                            };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc;
                            cell.explored = true;
                            map.insert(coord, cell);
                        } else {
                            let pos_seg = |v: i32| match v {
                                11 => "內口", 12 => "二段", 13 => "中段", 14 => "四段", 15 => "外段", _ => "關前",
                            };
                            let neg_seg = |v: i32| match v {
                                -11 => "內口", -12 => "二段", -13 => "中段", -14 => "四段", -15 => "外段", _ => "關前",
                            };
                            let (terrain, street_name, desc): (Terrain, String, String) = if dx < 0 && dy > 0 {
                                let t = if (dx.abs() + dy.abs()) % 3 == 0 { Terrain::Bridge } else { Terrain::Road };
                                let (sub, seg) = if (dx == -3 || dx == -5 || dx == -8) && dy >= 11 {
                                    let sub = match dx { -3 => "河市街", -5 => "漁舟巷", _ => "通濟道" };
                                    (sub, pos_seg(dy))
                                } else {
                                    let sub = match dy { 3 => "船塢街", 5 => "早市街", _ => "水柵巷" };
                                    (sub, neg_seg(dx))
                                };
                                (
                                    t,
                                    format!("三水外坊・{sub}{seg}"),
                                    format!("三水外坊臨水石街{sub}{seg}，魚市船塢夾道，水波蕩漾。"),
                                )
                            } else if dx > 0 && dy > 0 {
                                let (sub, seg) = if (dx == 3 || dx == 5 || dx == 8) && dy >= 11 {
                                    let sub = match dx { 3 => "校舍街", 5 => "格物巷", _ => "文風道" };
                                    (sub, pos_seg(dy))
                                } else {
                                    let sub = match dy { 3 => "墨莊街", 5 => "印書街", _ => "譯館巷" };
                                    (sub, pos_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("講堂外坊・{sub}{seg}"),
                                    format!("講堂外坊幽雅的校舍石街{sub}{seg}，文人墨客執卷漫步，墨香盈袖。"),
                                )
                            } else if dx > 0 && dy < 0 {
                                let (sub, seg) = if (dx == 3 || dx == 5 || dx == 8) && dy <= -11 {
                                    let sub = match dx { 3 => "水錘街", 5 => "大木巷", _ => "五金道" };
                                    (sub, neg_seg(dy))
                                } else {
                                    let sub = match dy { -3 => "鐵匠街", -5 => "車船街", _ => "鍛打巷" };
                                    (sub, pos_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("百工外坊・{sub}{seg}"),
                                    format!("百工外坊熱絡的工匠街巷{sub}{seg}，叮噹鐵聲與刨木芳香洋溢其間。"),
                                )
                            } else {
                                let (sub, seg) = if (dx == -3 || dx == -5 || dx == -8) && dy <= -11 {
                                    let sub = match dx { -3 => "駝隊街", -5 => "皮毛巷", _ => "糧草道" };
                                    (sub, neg_seg(dy))
                                } else {
                                    let sub = match dy { -3 => "驛站街", -5 => "客商街", _ => "護衛巷" };
                                    (sub, neg_seg(dx))
                                };
                                (
                                    Terrain::Road,
                                    format!("西門外坊・{sub}{seg}"),
                                    format!("西門外坊平實堅固的車馬街市{sub}{seg}，堆棧貨棚連綿，行商熙攘。"),
                                )
                            };
                            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &street_name));
                            cell.zone = city_name.to_string();
                            cell.description = desc;
                            cell.explored = true;
                            map.insert(coord, cell);
                        }
                    } else {
                        outer_slots.push((dx, dy, coord));
                    }
                }
                // 3. 內城環道與四向內門坊（dx.abs() == 10 || dy.abs() == 10，共 80 格）
                else if dx.abs() == 10 || dy.abs() == 10 {
                    let (street_name, desc): (String, String) = if dx == 0 && dy == 10 {
                        (
                            "北內門坊".to_string(),
                            "文曜內外城交界的青石牌坊，北接北門外街，南連講堂街北段，車馬行人絡繹不絕。".to_string(),
                        )
                    } else if dx == 0 && dy == -10 {
                        (
                            "南內門坊".to_string(),
                            "文曜內外城交界的雄渾石坊，南通南門外街，北接南門石板大街。".to_string(),
                        )
                    } else if dx == 10 && dy == 0 {
                        (
                            "東內門坊".to_string(),
                            "文曜內外城交界的飛簷石坊，東連東門外街，西接東門石板大道。".to_string(),
                        )
                    } else if dx == -10 && dy == 0 {
                        (
                            "西內門坊".to_string(),
                            "文曜內外城交界的寬闊石坊，西接西門外街，東連西門聯運大道。".to_string(),
                        )
                    } else if dx < 0 && dy > 0 {
                        if dx == -10 && dy == 10 {
                            (
                                "三水內環西北隅".to_string(),
                                "文曜三水坊內環西北轉角石坪，水波微漾，石階與柳蔭環護，北通三水外坊，東連水坊北街。".to_string(),
                            )
                        } else if dy == 10 {
                            let suf = match dx {
                                -1 => "北門西口", -2 => "北一段", -3 => "北水驛段", -4 => "北二段",
                                -5 => "北中段", -6 => "北三段", -7 => "北泊船段", -8 => "北四段", _ => "北西口",
                            };
                            (
                                format!("三水內環{suf}"),
                                format!("文曜三水坊內環北側{suf}，傍水依岸而建，石路平坦寬闊，茶鋪與水上貨站林立。"),
                            )
                        } else {
                            let suf = match dy {
                                1 => "西門北口", 2 => "西一段", 3 => "西商倉段", 4 => "西二段",
                                5 => "西中段", 6 => "西三段", 7 => "西河堤段", 8 => "西四段", _ => "西北口",
                            };
                            (
                                format!("三水內環{suf}"),
                                format!("文曜三水坊內環西側{suf}，沿護城水渠鋪就，微風拂面，水陸換運車馬絡繹不絕。"),
                            )
                        }
                    } else if dx > 0 && dy > 0 {
                        if dx == 10 && dy == 10 {
                            (
                                "講堂內環東北隅".to_string(),
                                "文曜講堂坊內環東北轉角，古槐參天，竹林掩映，青石坪上偶有求學士子駐足默誦。".to_string(),
                            )
                        } else if dy == 10 {
                            let suf = match dx {
                                1 => "北門東口", 2 => "北一段", 3 => "北學寮段", 4 => "北二段",
                                5 => "北中段", 6 => "北三段", 7 => "北林苑段", 8 => "北四段", _ => "北東口",
                            };
                            (
                                format!("講堂內環{suf}"),
                                format!("文曜講堂坊內環北側{suf}，修竹夾道，林蔭遮蔽，校舍研討之所星羅棋布。"),
                            )
                        } else {
                            let suf = match dy {
                                1 => "東門北口", 2 => "東一段", 3 => "東著述段", 4 => "東二段",
                                5 => "東中段", 6 => "東三段", 7 => "東幽徑段", 8 => "東四段", _ => "東北口",
                            };
                            (
                                format!("講堂內環{suf}"),
                                format!("文曜講堂坊內環東側{suf}，石板路面平整，文士墨客攜卷緩行，字畫碑帖鋪面錯落。"),
                            )
                        }
                    } else if dx > 0 && dy < 0 {
                        if dx == 10 && dy == -10 {
                            (
                                "分家內環東南隅".to_string(),
                                "文曜百工坊內環東南轉角，條石厚重平整，角落設有消防引水石槽與工匠歇息木亭。".to_string(),
                            )
                        } else if dy == -10 {
                            let suf = match dx {
                                1 => "南門東口", 2 => "南一段", 3 => "南工坊段", 4 => "南二段",
                                5 => "南中段", 6 => "南三段", 7 => "南炭場段", 8 => "南四段", _ => "南東口",
                            };
                            (
                                format!("分家內環{suf}"),
                                format!("文曜百工坊內環南側{suf}，車輛運載生鐵木料絡繹不絕，空氣中瀰漫著木炭與熱金氣息。"),
                            )
                        } else {
                            let suf = match dy {
                                -1 => "東門南口", -2 => "東一段", -3 => "東鐵料段", -4 => "東二段",
                                -5 => "東中段", -6 => "東三段", -7 => "東木器段", -8 => "東四段", _ => "東南口",
                            };
                            (
                                format!("分家內環{suf}"),
                                format!("文曜百工坊內環東側{suf}，石板路面經重車碾壓顯得油亮堅實，爐火工聲晝夜不息。"),
                            )
                        }
                    } else {
                        if dx == -10 && dy == -10 {
                            (
                                "西門內環西南隅".to_string(),
                                "文曜西門坊內環西南轉角，設有大車轉向與維修開闊石坪，粗大木樁整齊排列。".to_string(),
                            )
                        } else if dy == -10 {
                            let suf = match dx {
                                -1 => "南門西口", -2 => "南一段", -3 => "南車行段", -4 => "南二段",
                                -5 => "南中段", -6 => "南三段", -7 => "南牧馬段", -8 => "南四段", _ => "南西口",
                            };
                            (
                                format!("西門內環{suf}"),
                                format!("文曜西門坊內環南側{suf}，地勢平曠，沿道設有飲馬水槽與馱貨棧寮，蹄聲嘚嘚。"),
                            )
                        } else {
                            let suf = match dy {
                                -1 => "西門南口", -2 => "西一段", -3 => "西棧道段", -4 => "西二段",
                                -5 => "西中段", -6 => "西三段", -7 => "西馱場段", -8 => "西四段", _ => "西南口",
                            };
                            (
                                format!("西門內環{suf}"),
                                format!("文曜西門坊內環西側{suf}，商會旗幡迎風招展，承載各路客商厚重輜重。"),
                            )
                        }
                    };

                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, &street_name));
                    cell.zone = city_name.to_string();
                    cell.description = desc;
                    cell.explored = true;
                    map.insert(coord, cell);
                }
                // 4. 內核區域（dx.abs() <= 9 && dy.abs() <= 9，共 361 格）
                else {
                    let is_cross_avenue = dx == 0 || dy == 0;
                    let is_ring_street = dx.abs() == 5 || dy.abs() == 5;
                    let is_district_avenue = dx.abs() == 3 || dy.abs() == 3;

                    if is_cross_avenue || is_ring_street || is_district_avenue {
                        let (terrain, street_name, desc): (Terrain, String, String) = if dx == 0 && dy == 0 {
                            (
                                Terrain::Bridge,
                                "中心樞紐石橋".to_string(),
                                "文曜跨越三水匯流處的寬闊青石拱橋，橋下三水川碧波匯聚，橋頭立著報時日晷與指路竹牌，四周商號與鐘樓林立。".to_string(),
                            )
                        } else if dx == 0 && dy > 0 {
                            let (s, d) = match dy {
                                1 => (
                                    "三水口北街",
                                    "文曜三水口北側的平整石板街，南連中心樞紐橋，鄰近傳訊鐘樓，空氣中時常傳來鐘樓悠揚沉穩的整點報時鐘聲。",
                                ),
                                2 => (
                                    "鐘樓前街",
                                    "文曜傳訊鐘樓與藏書閣之間的青石大街，西接鐘樓報時台，東鄰印行書閣，整點鐘聲清越悠揚。",
                                ),
                                3 => (
                                    "講堂街南口",
                                    "文曜講堂街南端入口，北望學院青瓦飛簷，兩側文房墨寶與會計竹籤鋪展，迎面吹來淡雅墨香。",
                                ),
                                4 => (
                                    "講堂街中段",
                                    "文曜講堂街中段石道，兩旁教室林立，清晨與午後常能聽見文曜學院傳出學生齊聲誦讀課本的朗朗書聲。",
                                ),
                                5 => (
                                    "講堂街學院口",
                                    "文曜學院正門前的開闊石坪，古柏參天，修竹成蔭，學生與教席常在石階旁駐足研討學問。",
                                ),
                                6 => (
                                    "講堂街北段",
                                    "文曜講堂街北端典雅的林蔭大道，迎風送來藏書閣與印行館清雅的竹綿紙墨香。",
                                ),
                                7 => (
                                    "講堂街藏書巷口",
                                    "靠近文曜藏書閣後園的幽靜巷口，高聳的石砌書庫隔絕了市井喧囂，白牆青瓦間竹影婆娑。",
                                ),
                                8 => (
                                    "北門林蔭道南段",
                                    "文曜通往北城門的寬闊林蔭大道南段，兩旁列植高大苦楝與青松，綠蔭濃密。",
                                ),
                                _ => (
                                    "北門林蔭道北口",
                                    "文曜通往北城門的寬闊林蔭大道北口，直通北內門坊，車馬往來有序。",
                                ),
                            };
                            (Terrain::Road, s.to_string(), d.to_string())
                        } else if dx == 0 && dy < 0 {
                            let (s, d) = match dy {
                                -1 => (
                                    "三水口南街",
                                    "文曜三水口南側開闊平整的青石街道，北接中心樞紐石橋，東西兩側毗鄰公議堂與分家巷，往來市民絡繹不絕。",
                                ),
                                -2 => (
                                    "公議堂東街",
                                    "文曜公議堂與分家巷交界的青石大街，西鄰房地契書局與車馬聯運站，東接巧木傢俱坊，商旅與匠人往來不絕。",
                                ),
                                -3 => (
                                    "分家巷百工街口",
                                    "文曜分家巷與主街交匯處，深巷內時時傳來岑氏鐵坊清脆的鍛打錘音，以及巧木工坊新刨下的松香木刨花香氣。",
                                ),
                                -4 => (
                                    "分家巷中街",
                                    "分家巷核心段的石板街道，兩側宗族街屋錯落，曬架上掛著新染好的大圳頭棉布，生活氣息醇厚。",
                                ),
                                -5 => (
                                    "分家巷南口",
                                    "分家巷南端出入口，連接著城南百工聚落與通往南門的石道，往來推車滿載木料與熟鐵零件。",
                                ),
                                -6 => (
                                    "南門內街",
                                    "文曜城南內側的整齊街市，民居與街坊商鋪夾道排布，行旅在此整肅行裝準備出城。",
                                ),
                                -7 => (
                                    "南門石板大街",
                                    "文曜南門內寬闊平實的大道，直通南城門出入口，沿途林立著宗族街屋與工匠宅舍，步履踏在青石板上格外踏實。",
                                ),
                                -8 => (
                                    "南門石板街中段",
                                    "文曜南門石板街中段，青石板平整堅固，四方商賈挑擔牽馱絡繹不絕。",
                                ),
                                _ => (
                                    "南門石板街南口",
                                    "文曜南門石板街南端路口，北接南城門出入道，南連南內門坊。",
                                ),
                            };
                            (Terrain::Road, s.to_string(), d.to_string())
                        } else if dy == 0 && dx > 0 {
                            let (t, s, d) = match dx {
                                1 => (
                                    Terrain::Road,
                                    "三水東街",
                                    "文曜三水東街的臨水石板大道，兩旁商號林立，鄰近泉香溫水浴場，空氣中常瀰漫著山茶松脂皂與草木藥湯的溫熱水氣。",
                                ),
                                2 => (
                                    Terrain::Bridge,
                                    "東橋石板拱橋",
                                    "文曜三水東街橫跨三水川東灣的厚重石板拱橋，橋下碧波蕩漾，橋頭即是東橋旅社與山風茶屋，茶香與水聲相伴。",
                                ),
                                3 => (
                                    Terrain::Road,
                                    "東橋茶屋巷口",
                                    "文曜東橋畔熱鬧的茶屋巷口，東橋旅社與茶屋依水而建，微風中飄著慢火煮鮮奶茶與烘青茶的幽雅茶香。",
                                ),
                                4 => (
                                    Terrain::Road,
                                    "三水東街中段",
                                    "三水東街平闊的青石路段，兩旁列有南北貨棧與乾鮮海味行，空氣中帶著河風微鹹與乾貨乾香。",
                                ),
                                5 => (
                                    Terrain::Road,
                                    "三水東街臨水巷",
                                    "傍著三水川支流修建的石板巷道，石砌駁岸下泊著數艘小劃子，水波拍打石階，清脆有聲。",
                                ),
                                6 => (
                                    Terrain::Road,
                                    "東門石板大道",
                                    "文曜通往東城門的寬整石板大道，傍著三水川緩緩流淌的河畔，水面清波粼粼，微風吹拂十分怡人。",
                                ),
                                7 => (
                                    Terrain::Road,
                                    "東門石板道中段",
                                    "文曜東門石板道中段，微風吹拂，兩旁青柳依依，旅人漫步其間。",
                                ),
                                8 => (
                                    Terrain::Road,
                                    "東門石板道東段",
                                    "文曜東門石板道東段，靠近東城門內側，河風徐徐，市聲漸歇。",
                                ),
                                _ => (
                                    Terrain::Road,
                                    "東門石板道東口",
                                    "文曜東門石板道東端路口，西接城內大道，東連東內門坊。",
                                ),
                            };
                            (t, s.to_string(), d.to_string())
                        } else if dy == 0 && dx < 0 {
                            let (s, d) = match dx {
                                -1 => (
                                    "三水口西街",
                                    "文曜三水口西段的繁華街肆，三水大食堂就座落在此，紅鐵大釜翻滾的原牛骨濃湯與現煎蔥烙餅香氣四溢。",
                                ),
                                -2 => (
                                    "三水渡埠街",
                                    "文曜三水大食堂與三水匯流河灣之間的臨水石街，飯菜香氣與河灣水氣交融，船伕與食客絡繹不絕。",
                                ),
                                -3 => (
                                    "西門聯運東口",
                                    "西門聯運大道東端入口，鄰近公議堂與市中心石橋，馱獸輕裝車輛時常在此交會歇腳。",
                                ),
                                -4 => (
                                    "西門聯運大道",
                                    "文曜西側通往聯運站的主幹道，路面寬廣堅厚，一輛輛滿載半島貨物的雙輪駝獸大貨車結隊進出，十分熱鬧。",
                                ),
                                -5 => (
                                    "西門貨棧街口",
                                    "通往西門米糧貨棧與庫房的交匯路口，石砌倉房前常有店員核對庫單與清點麻袋。",
                                ),
                                -6 => (
                                    "西門車馬前街",
                                    "文曜西城門前開闊的車馬調度前街，地上鋪著耐壓重青條石，行商與車隊正抓緊在此檢點韁繩、添草料與辦理出入城手續。",
                                ),
                                -7 => (
                                    "西門車馬街中段",
                                    "文曜西門車馬街中段，兩旁車隊整歇，蹄聲與鐸鈴聲清脆入耳。",
                                ),
                                -8 => (
                                    "西門車馬街西段",
                                    "文曜西門車馬街西段，石坪寬闊，專供大型馱貨大車交會進出。",
                                ),
                                _ => (
                                    "西門車馬街西口",
                                    "文曜西門車馬街西端路口，東接車馬前街，西通西內門坊。",
                                ),
                            };
                            (Terrain::Road, s.to_string(), d.to_string())
                        } else if dx > 0 && dy > 0 {
                            let (name, desc) = if dx == 3 && dy == 3 {
                                ("講堂街學院巷".to_string(), "文曜講堂街幽靜深邃的學院巷道，白牆青瓦間修竹成蔭，學生與教席常在石階旁研討學問。".to_string())
                            } else if dx == 3 && dy == 5 {
                                ("講堂街・文曜國小十字口".to_string(), "文曜講堂街文曜國小前十字街口，稚嫩童音誦讀之聲朗朗傳來，石板路光潔細膩。".to_string())
                            } else if dx == 5 && dy == 3 {
                                ("講堂街・觀星台道東段".to_string(), "文曜講堂街觀星台道東段，地勢微仰，沿途常有星官教席執卷登階。".to_string())
                            } else if dx == 5 && dy == 5 {
                                ("講堂街・修竹十字口".to_string(), "文曜講堂街修竹夾道十字路口，修竹萬竿，清風徐徐。".to_string())
                            } else if dx == 3 {
                                let (suf, d) = match dy {
                                    1 => ("學院巷南口", "講堂街學院巷南端入口，鄰近主街，文風典雅。"),
                                    2 => ("學院巷中段", "講堂街學院巷中段，兩側教室靜謐，修竹掩映。"),
                                    4 => ("文曜國小前巷", "講堂街文曜國小門前石道，學童嬉笑誦讀聲盈耳。"),
                                    6 => ("藏書閣前巷南段", "講堂街靠近藏書閣的幽靜巷道南段，竹影婆娑。"),
                                    7 => ("藏書閣前巷中段", "講堂街藏書閣前巷中段，高聳石砌書庫隔絕喧囂。"),
                                    8 => ("藏書閣前巷北段", "講堂街藏書閣前巷北段，迎風飄來竹綿紙墨香。"),
                                    _ => ("藏書閣前巷北口", "講堂街藏書閣前巷北端出口，連通北內環道。"),
                                };
                                (format!("講堂街・{suf}"), d.to_string())
                            } else if dx == 5 {
                                let (suf, d) = match dy {
                                    1 => ("修竹夾道南口", "講堂街修竹夾道南端入口，翠竹成蔭。"),
                                    2 => ("修竹夾道中段", "講堂街修竹夾道中段石道，清幽寧靜。"),
                                    4 => ("修竹夾道北段", "講堂街修竹夾道北段，微風吹拂竹葉沙沙作響。"),
                                    6 => ("藏書東巷南段", "文曜藏書閣東側石巷南段，幽雅深邃。"),
                                    7 => ("藏書東巷中段", "文曜藏書閣東側石巷中段，陳列碑拓微雕。"),
                                    8 => ("藏書東巷北段", "文曜藏書閣東側石巷北段，綠蔭掩映。"),
                                    _ => ("藏書東巷北口", "文曜藏書閣東側石巷北口，北通內環大道。"),
                                };
                                (format!("講堂街・{suf}"), d.to_string())
                            } else if dy == 3 {
                                let (suf, d) = match dx {
                                    1 => ("印行館北巷", "文曜學院印行館北側的青石深巷，墨香撲鼻。"),
                                    2 => ("文教青石道", "文曜學院前寬敞的文教青石道，兩側排布書局印行館。"),
                                    4 => ("觀星台道西口", "通往觀星台石階道的西端路口，地勢平緩。"),
                                    6 => ("觀星石階前段", "通往學院觀星台的青石階道前段，石質堅硬。"),
                                    7 => ("觀星石階中段", "通往學院觀星台的青石階道中段，視野漸寬。"),
                                    8 => ("觀星石階東段", "通往學院觀星台的青石階道東段，星軌銅儀隱約可見。"),
                                    _ => ("觀星石階東口", "文曜學院東南側通往觀星台的石階起點。"),
                                };
                                (format!("講堂街・{suf}"), d.to_string())
                            } else {
                                let (suf, d) = match dx {
                                    1 => ("文曜國小書聲西口", "文曜國小旁的書聲石道西端路口，誦讀童音清脆。"),
                                    2 => ("文曜國小書聲西段", "文曜國小旁書聲西段，石板路被步履踏得光潔細膩。"),
                                    4 => ("文曜國小書聲中段", "文曜國小前中段石道，花格窗櫺透出朗朗讀書聲。"),
                                    6 => ("觀星東坪西段", "學院觀星台東側平坪西段，開闊平整。"),
                                    7 => ("觀星東坪中段", "學院觀星台東側平坪中段，遠眺東山。"),
                                    8 => ("觀星東坪東段", "學院觀星台東側平坪東段，古松傲立。"),
                                    _ => ("觀星東坪東口", "學院觀星台東側平坪東口，通達東內環道。"),
                                };
                                (format!("講堂街・{suf}"), d.to_string())
                            };
                            (Terrain::Road, name, desc)
                        } else if dx > 0 && dy < 0 {
                            let (name, desc) = if dx == 3 && dy == -3 {
                                ("分家巷・百工轉角坪".to_string(), "分家巷百工聚落東南轉角處的卸貨小坪，常有推車在此搬運鐵胚木材。".to_string())
                            } else if dx == 3 && dy == -5 {
                                ("分家巷・百工十字口".to_string(), "分家巷百工聚落十字路口，匠人們往來呼應，熱絡非常。".to_string())
                            } else if dx == 5 && dy == -3 {
                                ("分家巷・東橋茶屋廊道中段".to_string(), "分家巷鄰近東橋茶屋側廊的步道中段，能聽見臨水茶座傳來的笑語。".to_string())
                            } else if dx == 5 && dy == -5 {
                                ("分家巷・織造十字口".to_string(), "分家巷雲錦織造坊前的十字街口，微風中夾雜著草木染料香氣。".to_string())
                            } else if dx == 3 {
                                let (suf, d) = match dy {
                                    -1 => ("百工北口", "分家巷百工街區北端入口，南望工坊石屋林立。"),
                                    -2 => ("鍛造東巷", "岑氏第一鍛造工坊東側的窄石巷，金鐵交擊聲不絕於耳。"),
                                    -4 => ("百工中段", "分家巷百工聚落中段石道，路旁整齊碼放硬木與鐵件。"),
                                    -6 => ("鍛造爐火巷北段", "分家巷深處緊挨鍛造工坊的石巷北段，地面鋪設耐熱重石。"),
                                    -7 => ("鍛造爐火巷中段", "分家巷鍛造爐火巷中段，工坊內傳來沉穩厚實的鍛錘聲。"),
                                    -8 => ("鍛造爐火巷南段", "分家巷鍛造爐火巷南段，火星與熱金氣息撲面。"),
                                    _ => ("鍛造爐火巷南口", "分家巷鍛造爐火巷南端出口，通向南門大街。"),
                                };
                                (format!("分家巷・{suf}"), d.to_string())
                            } else if dx == 5 {
                                let (suf, d) = match dy {
                                    -1 => ("東橋南廊北口", "東橋南側緊鄰水灣的青石步道北端，水汽溫涼。"),
                                    -2 => ("東橋南廊中段", "東橋南側青石步道中段，能遠眺東橋旅社的飛簷與茶座。"),
                                    -4 => ("診所藥香東巷", "溫氏脈絡診所東側石巷，常年瀰漫焙製草木藥香。"),
                                    -6 => ("鍛造東巷北段", "分家巷東側鍛造作坊外巷北段，工匠運載木炭。"),
                                    -7 => ("鍛造東巷中段", "分家巷東側鍛造作坊外巷中段，條石堅厚。"),
                                    -8 => ("鍛造東巷南段", "分家巷東側鍛造作坊外巷南段，爐煙裊裊。"),
                                    _ => ("鍛造東巷南口", "分家巷東側鍛造作坊外巷南口，南接內環道。"),
                                };
                                (format!("分家巷・{suf}"), d.to_string())
                            } else if dy == -3 {
                                if dx == 1 {
                                    ("分家巷百工石道".to_string(), "文曜分家巷百工聚落的青石通道，路旁整齊碼放著硬木方料與鐵料坯件。".to_string())
                                } else {
                                    let (suf, d) = match dx {
                                        2 => ("巧木前街", "巧木工坊門前的熱鬧石街，陳列著精緻的桌椅木櫃。"),
                                        4 => ("東橋茶屋廊道西口", "東橋茶屋西側廊道入口，茶香悠遠。"),
                                        6 => ("巧木織造街西段", "分家巷串聯傢俱坊與雲錦織造坊的街弄西段。"),
                                        7 => ("巧木織造街中段", "分家巷巧木織造街中段，飄著刨木清香與大圳頭棉麻草木氣息。"),
                                        8 => ("巧木織造街東段", "分家巷巧木織造街東段，匠人細緻運針穿梭。"),
                                        _ => ("巧木織造街東口", "分家巷巧木織造街東端出口，東連東門大道。"),
                                    };
                                    (format!("分家巷・{suf}"), d.to_string())
                                }
                            } else {
                                let (suf, d) = match dx {
                                    1 => ("診所藥香西口", "溫氏脈絡診所西側巷口，薄荷與艾草香氣醇厚。"),
                                    2 => ("診所藥香西段", "脈絡診所與百草烘藥房前青石小巷，寧靜祥和。"),
                                    4 => ("診所藥香中段", "百草烘藥房前中段石道，熱氣烘焙藥草。"),
                                    6 => ("巧木南街西段", "巧木傢俱坊南側工匠通道西段，堆放乾燥木料。"),
                                    7 => ("巧木南街中段", "巧木傢俱坊南側中段，木刨聲響起落。"),
                                    8 => ("巧木南街東段", "巧木傢俱坊南側東段，整齊排列成品榫卯長凳。"),
                                    _ => ("巧木南街東口", "巧木傢俱坊南側東端路口，通往內環街區。"),
                                };
                                (format!("分家巷・{suf}"), d.to_string())
                            };
                            (Terrain::Road, name, desc)
                        } else if dx < 0 && dy > 0 {
                            let (name, desc) = if dx == -3 && dy == 3 {
                                ("三水口・商埠轉角坪".to_string(), "三水口渡埠與西北商市交匯的轉角石坪，搬運員與攤商在此歇肩換手。".to_string())
                            } else if dx == -5 && dy == 3 {
                                ("三水口・商號十字口".to_string(), "三水口商號街十字路口，商行店員進出穿梭，市聲鼎沸。".to_string())
                            } else if dx == -3 && dy == 5 {
                                ("三水口・鐘樓西十字口".to_string(), "三水口鐘樓西側十字街口，青銅巨鐘之聲悠揚回盪。".to_string())
                            } else if dx == -5 && dy == 5 {
                                ("三水口・泉香十字口".to_string(), "三水口泉香浴場前十字路口，溫熱水汽帶著松脂清香。".to_string())
                            } else if dx == -3 {
                                let (suf, d) = match dy {
                                    1 => ("總商西街南口", "三水總商行西側平闊的青石街面南口，車馬通行有序。"),
                                    2 => ("食堂西巷", "三水大食堂西側的條石小巷，後廚煙囪升起裊裊炊煙。"),
                                    4 => ("浴場石道南口", "通往泉香溫水浴場的條石步道南口，水汽溫熱。"),
                                    6 => ("鐘樓報時街南段", "傳訊鐘樓下的報時大街南段，青銅巨鐘沉凝回盪。"),
                                    7 => ("鐘樓報時街中段", "傳訊鐘樓報時街中段，整裝信使策馬由石道飛馳而過。"),
                                    8 => ("鐘樓報時街北段", "傳訊鐘樓報時街北段，兩旁列植高柳。"),
                                    _ => ("鐘樓報時街北口", "傳訊鐘樓報時街北端出口，直通北內環道。"),
                                };
                                (format!("三水口・{suf}"), d.to_string())
                            } else if dx == -5 {
                                let (suf, d) = match dy {
                                    1 => ("商號街南口", "三水口臨河商號街南端入口，店鋪門面櫛比鱗次。"),
                                    2 => ("商號街南段", "三水口商號街南段，店員忙著清點貨單搬運布匹。"),
                                    4 => ("浴場西巷", "泉香溫水浴場西側石巷，清風拂送溫熱水氣。"),
                                    6 => ("報時西街南段", "鐘樓西側報時街南段，信使快騎整備宿所門前。"),
                                    7 => ("報時西街中段", "鐘樓西側報時街中段，石面堅固平整。"),
                                    8 => ("報時西街北段", "鐘樓西側報時街北段，行商問路交談。"),
                                    _ => ("報時西街北口", "鐘樓西側報時街北端出口，北接內環路口。"),
                                };
                                (format!("三水口・{suf}"), d.to_string())
                            } else if dy == 3 {
                                if dx == -4 {
                                    ("三水口商號街".to_string(), "文曜三水口臨河的商號街肆，商行店員正忙著清點貨單，市聲鼎沸。".to_string())
                                } else {
                                    let (suf, d) = match dx {
                                        -1 => ("鐘樓北巷", "傳訊鐘樓北側的高牆幽巷，厚實花崗岩基座巍然矗立。"),
                                        -2 => ("碼頭行商道", "緊依三水口渡埠的繁榮石板道，海味乾貨行與大食堂夾道而設。"),
                                        -6 => ("總商票號街東段", "三水總商行總部與票號前青石街肆東段，客商雲集。"),
                                        -7 => ("總商票號街中段", "三水總商行票號街中段，各路客商提著皮匣出入。"),
                                        -8 => ("總商票號街西段", "三水總商行票號街西段，石坪開闊，商貿井然。"),
                                        _ => ("總商票號街西口", "三水總商行票號街西端出口，通往西門大道。"),
                                    };
                                    (format!("三水口・{suf}"), d.to_string())
                                }
                            } else {
                                let (suf, d) = match dx {
                                    -1 => ("泉香浴場石道東口", "泉香溫水浴場石道東端入口，活水泉汽裊裊。"),
                                    -2 => ("泉香浴場石道東段", "泉香溫水浴場石道東段，帶著淡淡的山茶松脂香。"),
                                    -4 => ("泉香浴場石道中段", "泉香溫水浴場正門前石坪，引活水泉渠流淌。"),
                                    -6 => ("總商北街東段", "三水總商行北側物流石道東段，堆放乾鮮貨包。"),
                                    -7 => ("總商北街中段", "三水總商行北側物流石道中段，店員核對倉冊。"),
                                    -8 => ("總商北街西段", "三水總商行北側物流石道西段，大車裝卸迅速。"),
                                    _ => ("總商北街西口", "三水總商行北側物流石道西口，通達西北內環。"),
                                };
                                (format!("三水口・{suf}"), d.to_string())
                            };
                            (Terrain::Road, name, desc)
                        } else {
                            let (name, desc) = if dx == -3 && dy == -3 {
                                ("西門・貨棧轉角坪".to_string(), "西門貨棧群南端的石鋪大坪，常年停靠著裝卸大車，粗麻繩索堆疊井然。".to_string())
                            } else if dx == -5 && dy == -3 {
                                ("西門・聯運十字口".to_string(), "西門車馬聯運總站門前十字路口，雙輪大車川流不息，駝鈴聲聲。".to_string())
                            } else if dx == -3 && dy == -5 {
                                ("西門・公議十字口".to_string(), "西門公議堂前十字街口，白塔半島居民常在此洽辦民務。".to_string())
                            } else if dx == -5 && dy == -5 {
                                ("西門・車馬十字口".to_string(), "西門車馬大道南側十字路口，駝隊客商整理輜重草料。".to_string())
                            } else if dx == -3 {
                                let (suf, d) = match dy {
                                    -1 => ("契書西街北口", "公議堂地籍契書專局西側石道北口，兩旁多代書書肆。"),
                                    -2 => ("車馬西巷", "車馬聯運總站西側重石板巷道，地面留有深深鐵裹車輪印痕。"),
                                    -4 => ("公議中巷", "公議堂與聯運站之間的石砌深巷，公事嚴整。"),
                                    -6 => ("聯運大道南段", "西門車馬聯運總站門口開闊青石大道南段，車隊往來。"),
                                    -7 => ("聯運大道中段", "西門車馬聯運大道中段，駝獸進出有序。"),
                                    -8 => ("聯運大道遠段", "西門車馬聯運大道遠段，路面寬廣堅厚。"),
                                    _ => ("聯運大道南口", "西門車馬聯運大道南端出口，直通南內門街。"),
                                };
                                (format!("西門・{suf}"), d.to_string())
                            } else if dx == -5 {
                                let (suf, d) = match dy {
                                    -1 => ("貨棧北弄", "西門貨棧北端窄弄，高大石砌庫房綿延。"),
                                    -2 => ("貨棧中弄", "西門貨棧中段巷弄，駝獸在路旁棚架下嚼著乾草。"),
                                    -4 => ("貨棧南巷", "西門貨棧南端通道，駕駛員忙著裝卸貨箱與草料包。"),
                                    -6 => ("車馬大道北段", "通往西門車馬調度場的寬大道北段，耐壓條石平鋪。"),
                                    -7 => ("車馬大道中段", "通往西門車馬調度場的大道中段，客商絡繹不絕。"),
                                    -8 => ("車馬大道南段", "通往西門車馬調度場的大道南段，蹄聲得得。"),
                                    _ => ("車馬大道南口", "通往西門車馬調度場大道南端出口，接連內環大道。"),
                                };
                                (format!("西門・{suf}"), d.to_string())
                            } else if dy == -3 {
                                if dx == -4 {
                                    ("西門貨棧街".to_string(), "文曜西門附近緊鄰聯運站的貨棧石道，高大的石砌庫房綿延，駝獸在棚下嚼草。".to_string())
                                } else {
                                    let (suf, d) = match dx {
                                        -1 => ("聯運南巷", "車馬聯運站南側通往公議堂的青石便道，行商在此歇馬飲水。"),
                                        -2 => ("行車青石道", "連接西城門與市中心的堅固青石道，路面平整寬闊。"),
                                        -6 => ("米糧貨棧街東段", "西門貨棧深處米糧庫房街東段，透著新穀乾燥香氣。"),
                                        -7 => ("米糧貨棧街中段", "西門米糧貨棧街中段，石砌倉房整齊高聳。"),
                                        -8 => ("米糧貨棧街西段", "西門米糧貨棧街西段，店員核對倉單與麻袋。"),
                                        _ => ("米糧貨棧街西口", "西門米糧貨棧街西端出口，通往西城門內街。"),
                                    };
                                    (format!("西門・{suf}"), d.to_string())
                                }
                            } else {
                                let (suf, d) = match dx {
                                    -1 => ("公議前街東口", "公議堂東側平整大街入口，居民往來洽辦事務。"),
                                    -2 => ("公議前街東段", "公議堂與房地契書局前平整規整的大街東段。"),
                                    -4 => ("公議前街中段", "公議堂正門前開闊石坪，青石板潔淨莊嚴。"),
                                    -6 => ("米糧南街東段", "西門米糧倉區南側通道東段，運糧車輛絡繹不絕。"),
                                    -7 => ("米糧南街中段", "西門米糧南街中段，石道平坦耐磨。"),
                                    -8 => ("米糧南街西段", "西門米糧南街西段，堆放草料與帆布。"),
                                    _ => ("米糧南街西口", "西門米糧南街西端出口，連接西南內環道。"),
                                };
                                (format!("西門・{suf}"), d.to_string())
                            };
                            (Terrain::Road, name, desc)
                        };
                        let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &street_name));
                        cell.zone = city_name.to_string();
                        cell.description = desc;
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        inner_slots.push((dx, dy, coord));
                    }
                }
            }
        }

        // 內城四街區配置（125 棟正典地標建築）
        let mut nw_slots = Vec::new();
        let mut ne_slots = Vec::new();
        let mut se_slots = Vec::new();
        let mut sw_slots = Vec::new();

        for (dx, dy, coord) in inner_slots {
            if dx < 0 && dy > 0 {
                nw_slots.push((dx, dy, coord));
            } else if dx > 0 && dy > 0 {
                ne_slots.push((dx, dy, coord));
            } else if dx > 0 && dy < 0 {
                se_slots.push((dx, dy, coord));
            } else {
                sw_slots.push((dx, dy, coord));
            }
        }

        let sort_fn = |&(dx, dy, _): &(i32, i32, SquareCoord)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy.abs(), dx.abs());
        nw_slots.sort_by_key(sort_fn);
        ne_slots.sort_by_key(sort_fn);
        se_slots.sort_by_key(sort_fn);
        sw_slots.sort_by_key(sort_fn);

        // 在不切斷任何街區與建築連通、且不擠佔 (±1,±1)/(±2,±1)/(±1,±2) 十二座核心地標館舍的前提下，
        // 於四象限內河灣對角格 (±2, ±2) 設置真實 Terrain::Water（≈）
        let water_bays = [
            (
                SquareCoord::new(-2, 2),
                Terrain::Water,
                "三水川北渠",
                "文曜三水川流經三水口北側的引流清渠，渠水清澈碧藍，波光瀲灩，沿岸石階整飭，微風徐來水氣沁涼。",
            ),
            (
                SquareCoord::new(2, 2),
                Terrain::Water,
                "三水川東灣",
                "文曜三水川向東折轉形成的開闊河灣，碧波蕩漾，水色澄澈，南側即是東橋石板拱橋與東橋旅社，推窗便見流水浮雲。",
            ),
            (
                SquareCoord::new(2, -2),
                Terrain::Water,
                "三水川南津水脈",
                "自中心石橋南下流經分家巷的天然活水脈絡，清泉奔湧，水勢溫柔，常有白鳥掠水而過。",
            ),
            (
                SquareCoord::new(-2, -2),
                Terrain::Water,
                "三水匯流河灣",
                "白塔半島三大水系在文曜城核心交匯迴旋形成的深水河灣，水流奔騰匯聚，浪花拍岸，乃全城水系之靈魂所在。",
            ),
        ];

        for (coord, terrain, bay_name, bay_desc) in water_bays {
            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bay_name));
            cell.zone = city_name.to_string();
            cell.description = bay_desc.to_string();
            cell.explored = true;
            map.insert(coord, cell);
        }

        // 移除已作為水體之格位，確保其餘格位分配予建築與具名街坊
        nw_slots.retain(|&(dx, dy, _)| !(dx == -2 && dy == 2));
        ne_slots.retain(|&(dx, dy, _)| !(dx == 2 && dy == 2));
        se_slots.retain(|&(dx, dy, _)| !(dx == 2 && dy == -2));
        sw_slots.retain(|&(dx, dy, _)| !(dx == -2 && dy == -2));
        let nw_rem_defs: &[(Terrain, &str, &str)] = &[
            (Terrain::Bridge, "三水口河階渡埠", "三水口臨水修築的條石河階石埠，清澈川流在此迴旋，石階整飭，常有花船與竹筏靠岸歇息。"),
            (Terrain::Road, "三水口納涼棚廊", "三水大食堂外側搭設的竹編遮陽涼棚與長木廊，微風送爽，夏日鎮民常聚集在此喝冰鎮甘蔗汁與黑糖仙草凍。"),
            (Terrain::House, "三水口・漁舟泊家宅", "三水口老船伕依水而建的石砌家屋，簷下晾掛著乾燥的棕櫚雨簑與粗麻纖繩，透著踏實的生活氣息。"),
            (Terrain::Road, "三水口・清江曬網坪", "臨河開闊的青石平坪，漁民將織造細密的漁網攤開晾曬，江風吹拂，波光粼粼。"),
            (Terrain::House, "三水口・水運搬運工舍", "三水口碼頭搬運力伕聚居的整潔石舍，門前擺放著結實的硬木扁擔與推車。"),
            (Terrain::Bridge, "三水口・青苔回瀾石橋", "跨越三水內河回瀾灣的古樸單孔石拱橋，橋欄斑駁長滿青苔，橋下游魚可數。"),
            (Terrain::House, "三水口・河畔水磨居", "引河水帶動石磨研磨麥粉與米糧的臨水作坊宅院，石磨隆隆運轉，米香誘人。"),
            (Terrain::Road, "三水口・鮮魚早市石階", "晨間漁獲靠岸交易的條石駁岸水階，石階洗刷得極為整潔，常年有泉水沖洗。"),
            (Terrain::House, "三水口・老水夫客寮", "供往來內河水手短暫停宿的石磚客舍，屋內乾燥溫暖，備有厚實棉被。"),
            (Terrain::HerbGarden, "三水口・水岸垂柳苑", "沿河岸栽植的垂柳水苑，細長柳絲拂水，青石凳錯落，是岸旁居民閒聚賞景之處。"),
            (Terrain::House, "三水口・竹筏扎作宅", "專門編扎輕便載客竹筏與巡水排筏的巧匠宅舍，院中堆滿烘烤順直的大青竹。"),
            (Terrain::Road, "三水口・沿河洗滌石磯", "伸入清澈川流的條石浣洗磯頭，鎮民早晚在此提汲清水與漂洗布匹。"),
            (Terrain::Workshop, "三水口・修網結繩工坊", "專司編結加固各類水運纖繩與漁網的熟練匠人家屋，牆上懸滿堅韌的黑沼麻線。"),
            (Terrain::Bridge, "三水口・臨流跳石橋", "在平緩清淺水面上鋪設的一連串圓整青石跳石，清流自石隙潺潺穿過，映著岸畔燈影與水色。"),
            (Terrain::House, "三水口・晨汲泉井舍", "圍繞三水口甘洌地底水井修築的民居，井圈磨損光滑，水質清甜沁脾。"),
            (Terrain::Road, "三水口・河口觀瀾石台", "突出於河道轉折處的防浪石台，憑欄遠眺可將三水川匯流之壯麗景色盡收眼底。"),
            (Terrain::House, "三水口・泊船纜樁居", "負責看守碼頭繫纜石樁與維護泊位的渡埠管事宅舍。"),
            (Terrain::Grassland, "三水口・水邊白鷺草坪", "河灣沉積形成的柔軟青草平坪，常有白鷺與水鳥在此棲息覓食。"),
            (Terrain::Warehouse, "三水口・水產乾貨儲庫", "用通風石木架構修築的乾海產貯藏小院，四周飄散著乾燥蝦皮與紫菜香。"),
            (Terrain::Road, "三水口・波光泊船弄", "緊貼河畔的狹長石弄，兩側是白牆灰瓦，弄口直通停泊竹筏的小碼頭。"),
            (Terrain::House, "三水口・擺渡老爹宿所", "三水口資歷最深的老擺渡人住所，門前木架上曬著一件老舊卻耐用的油布雨披。"),
            (Terrain::Bridge, "三水口・雙柳石平橋", "兩端各種有一株挺拔垂柳的小石平橋，水流在此平緩無聲，如鏡面倒映藍天。"),
            (Terrain::House, "三水口・魚苗培育小院", "利用河灣靜水培育優良淡水魚苗的家庭魚池小院。"),
            (Terrain::Road, "三水口・清風聽濤石步道", "順應水岸地勢修築的漫步石道，微風吹來水氣宜人，令人心曠神怡。"),
            (Terrain::Workshop, "三水口・船用桐油作坊", "提煉防水熟桐油並浸泡木料的工坊民宅，空氣中瀰漫著濃郁的桐籽香。"),
            (Terrain::HerbGarden, "三水口・芳草清水池", "引活水匯聚的小巧生態荷花池苑，栽種水生花草與清心薄荷。"),
            (Terrain::House, "三水口・信鴿棲籠宅", "配合鐘樓信使馴養短途通訊信鴿的屋舍，屋頂設有通風寬大的杉木鴿棚。"),
            (Terrain::Road, "三水口・石板迴舟坪", "碼頭後方供板車與搬運員回轉調度的寬闊石坪，整潔平坦。"),
            (Terrain::House, "三水口・水手聚義舍", "往來船伕水手共同出資維護的聚會歇息之所，擺放著結實的大條凳與粗茶陶壺。"),
            (Terrain::Bridge, "三水口・新修渡埠拱橋", "近期由三水總商行出資修繕的青石拱橋，平坦堅固，能容載貨木車暢行。"),
            (Terrain::Workshop, "三水口・浮標竹笛作坊", "雕製水上航道標記浮筒與防霧竹笛的匠人家宅。"),
            (Terrain::Road, "三水口・落霞臨波步徑", "傍晚時分觀賞夕陽映照三水川最佳的河岸石徑。"),
            (Terrain::GuardPost, "三水口・河工巡防哨居", "兼顧防汛巡視與水門調度的水利巡防人員住屋。"),
            (Terrain::House, "三水口・清波安居宅", "三水口水岸旁一處安靜的民家院落，門前花草蔥鬱，生活恬淡。"),
        ];

        let ne_rem_defs: &[(Terrain, &str, &str)] = &[
            (Terrain::Road, "講堂街曬書石坪", "文曜講堂街開闊平整的曬書石坪，日光充足，每逢晴日書閣與學宮便在此搭架攤曬珍貴典籍與竹綿紙卷。"),
            (Terrain::HerbGarden, "學宮青竹苑", "文曜學院旁幽靜的青竹庭苑，微風拂過翠竹沙沙作響，青石几几，是師生課餘靜坐思索的清心場所。"),
            (Terrain::House, "講堂街・研墨小齋", "文曜學生專用之研墨自修書齋，窗明几淨，案上筆架端正，松煙墨香經年不散。"),
            (Terrain::Road, "講堂街・修竹夾道", "講堂街內兩側修竹成行的幽雅石徑，篩落細碎光影，令人步履自然放緩。"),
            (Terrain::House, "講堂街・古籍校勘別室", "學院名師考訂古經與校對善本之專用齋室，四壁皆為防蛀樟木書櫥。"),
            (Terrain::HerbGarden, "講堂街・百草本草園", "文學院附設之藥用植物識別小園，栽植多種辨識經書所載之奇草奇花。"),
            (Terrain::House, "講堂街・金石摹拓居", "專事研習古碑摩崖拓印法度之文士雅居，屋內懸掛著字態遒勁之墨拓拓本。"),
            (Terrain::Road, "講堂街・問學石坪", "學宮前提供學生辯難明理與互相切磋學問的青石空地，時有老師在此開壇解惑。"),
            (Terrain::Workshop, "講堂街・竹綿造紙精廬", "傳承竹綿紙古法抄造手藝之匠人院落，晾紙木牆前鋪陳著張張雪白細滑的佳紙。"),
            (Terrain::HerbGarden, "講堂街・墨荷印池", "引活水匯成之方石印池，池中蓮荷亭亭玉立，池邊石案可供學生臨池洗硯。"),
            (Terrain::Workshop, "講堂街・古琴斫琴齋", "精選北山良桐古木斫製七弦七音琴的雅室，琴音清越，餘韻繞樑。"),
            (Terrain::Road, "講堂街・松風長廊", "依學院花牆修築之木構長廊，廊外古松參天，是雨日漫步賞景之佳處。"),
            (Terrain::Library, "講堂街・藏書別院", "收藏各城鎮方志與山海圖譜之藏書別院，守書人每日細心巡查通風。"),
            (Terrain::HerbGarden, "講堂街・靜思桂花林", "秋日香飄數里的金桂園林，樹下設有石桌石凳，花雨紛紛，暗香盈袖。"),
            (Terrain::Workshop, "講堂街・宣筆制毫坊", "選取純淨羊毫與紫毫精製各品書畫文筆的世家書齋作坊。"),
            (Terrain::Road, "講堂街・文林石徑", "鋪設細密鵝卵石與青條石的曲徑，穿行於各教室齋室之間，環境幽美。"),
            (Terrain::Academy, "講堂街・曆算講堂", "文曜講堂街推算二十四節氣與編訂歲時日曆的曆算講席殿堂，前方烏木教案上擺著教案上的黃銅鋼筆與推步講義。"),
            (Terrain::House, "講堂街・四時星曆舍", "協助觀星台推算二十四節氣與編訂歲時日曆的曆算學者住所，前方烏木教案上擺著教案上的黃銅鋼筆與推步講義。"),
            (Terrain::HerbGarden, "講堂街・洗硯石池苑", "緊鄰教室之活水清池，池底鋪設彩色細卵石，泉流淙淙不絕。"),
            (Terrain::Workshop, "講堂街・端溪石硯坊", "琢磨老坑端石與青重礦以製作傳世名硯的石匠雅居。"),
            (Terrain::Road, "講堂街・策論石階", "拾級而上的寬平條石階梯，直通學院大講堂，象徵登高求索之志。"),
            (Terrain::House, "講堂街・算理推步居", "專門鑽研九章算術與度量衡定式之學者精舍，案頭堆疊算籌與木規。"),
            (Terrain::HerbGarden, "講堂街・幽蘭修性圃", "精心培育深山香蘭的溫潤小圃，蘭草青翠欲滴，室雅何須大。"),
            (Terrain::Workshop, "講堂街・書畫裱褙齋", "以熟漿糊與防潮絹帛裝裱傳世字畫卷軸之老字號工坊，室內通風乾燥。"),
            (Terrain::Road, "講堂街・明倫步廊", "學宮主通道兩側的平實步廊，廊柱上懸有先賢垂範教誨之木刻楹聯。"),
            (Terrain::House, "講堂街・陶冶茶陶居", "燒製素雅紫砂茶壺與白瓷茶盞之文人陶舍，案上陳列著樸雅器物。"),
            (Terrain::HerbGarden, "講堂街・學宮杏壇林", "相傳先哲講學處手植之老銀杏樹林，每至深秋滿地金黃，蔚為壯觀。"),
            (Terrain::House, "講堂街・詩韻唱和閣", "學院詩社同仁每月聚會分韻賦詩之小巧飛簷閣樓，推窗可眺遠山雲嵐。"),
            (Terrain::Road, "講堂街・青雲登科道", "通往學院大門之筆直大道，路面青條石拼嵌得嚴絲合縫，氣勢恢宏。"),
            (Terrain::Library, "講堂街・圖書典藏庫", "存放雕版印刷所用梨木棗木書雕版片的厚牆石庫，防潮防火措施嚴密。"),
            (Terrain::HerbGarden, "講堂街・讀易草廬苑", "被綠竹清溪環抱的草堂小苑，環境清幽寂靜，宜於研讀大易哲思。"),
            (Terrain::House, "講堂街・金石印社宅", "篆刻家匯聚交流之居所，室內四處擺滿各色印泥與壽山石章料。"),
            (Terrain::Road, "講堂街・翰墨書香街", "臨近各印行館之青石街面，兩旁常有書商擺列新印刊本，文客絡繹不絕。"),
            (Terrain::House, "講堂街・禮樂傳習舍", "研習祭祀儀禮與雅樂管弦之專門齋所，擺放著編磬與古樂器。"),
            (Terrain::House, "講堂街・致遠安居院", "退隱教席安享晚年之和樂四合院落，庭前花木扶疏，一派祥和。"),
        ];

        let se_rem_defs: &[(Terrain, &str, &str)] = &[
            (Terrain::Road, "分家巷晾木場", "分家巷木匠坊附設的通風晾木場，高高架疊著整齊裁切的松香木與老雲杉板材，散發著好聞的天然樹脂香。"),
            (Terrain::Bridge, "百工活水渠橋", "分家巷引流三水活水的小巧石砌水渠與石板橋，渠水清澈見底，工坊學徒常在此汲水淬火與洗滌工具。"),
            (Terrain::House, "分家巷・精工火磚匠廬", "專門燒製耐高溫導能火磚以供各鍛爐使用的特種耐火土工坊匠廬。"),
            (Terrain::House, "分家巷・金石榫卯木舍", "研習精微榫卯結構的木作居所，桌上陳列著精緻微縮梁架模型。"),
            (Terrain::House, "分家巷・竹器巧編院", "運用北山青竹編造各類耐用提籃、竹蓆與防雨竹笠的巧手工匠住家。"),
            (Terrain::Road, "分家巷・青石淬火池步道", "串連各個打鐵工坊與公共冷卻水槽的寬整石板小道，地面結實耐磨。"),
            (Terrain::Workshop, "分家巷・染纈織染居", "雲錦織造坊調配天然草木染料與染製經緯彩線之作坊，院內高掛各色絢麗匹練。"),
            (Terrain::HerbGarden, "分家巷・染草苗圃", "培育茜草、藍草與紫草等天然染料作物的向陽小園，花草生機勃勃。"),
            (Terrain::GeneralStore, "分家巷・皮鞘鞍具行", "裁製堅韌牛皮刀鞘、馬鞍與耐磨皮帶之熟皮工匠宅院，門口晾曬熟皮香氣撲鼻。"),
            (Terrain::Bridge, "分家巷・拱水石虹橋", "飛架於分家巷主幹水渠上之半圓石拱橋，橋下清水奔流，為工坊主要通行道。"),
            (Terrain::Workshop, "分家巷・銅錫器皿坊", "打製精緻紫銅水壺、黃銅門環與錫製茶葉罐之匠人居所，叮噹敲擊聲清脆悅耳。"),
            (Terrain::Road, "分家巷・打鐵試刀坪", "鍛造工坊門外專供買家試驗刀斧鋒刃與農具韌性的開闊厚石坪。"),
            (Terrain::House, "分家巷・陶瓷陶罐舍", "捏製耐酸儲藥陶罐與廚房密封油壇之樸實陶工家宅，門口整齊碼放素燒陶坯。"),
            (Terrain::HerbGarden, "分家巷・百草蔭涼庭", "溫氏藥鋪外側種植避暑薄荷與金銀花的清涼小庭院，常有居民在此討碗涼茶。"),
            (Terrain::House, "分家巷・石磨石匠居", "開採北山玄武石打磨上等磨盤、石臼與石階的石作大匠宅邸。"),
            (Terrain::Road, "分家巷・木香通風弄", "穿梭於堆木庫房之間的長長夾弄，終年穿堂風微送，瀰漫清幽杉木芳香。"),
            (Terrain::Workshop, "分家巷・鎖鑰鎖簧鋪", "精製多重防盜青銅彈子鎖與彈簧機巧的精細五金鎖匠居室。"),
            (Terrain::Bridge, "分家巷・跨渠平板石橋", "由三塊巨大的整條青石板平鋪而成之跨渠矮橋，走在上方平穩厚重。"),
            (Terrain::House, "分家巷・雕花門窗齋", "專門雕刻花鳥山水花格窗櫺與精美拔步床門罩的木雕世家住屋。"),
            (Terrain::Road, "分家巷・百工聚工坪", "晨間各作坊店長／主事招募短期幫工與學徒協商工價的熱鬧石板小廣場。"),
            (Terrain::House, "分家巷・炭柴堆棧舍", "分揀儲存優質無煙白炭與硬木薪柴的乾燥石砌庫房，供應全巷爐火不斷。"),
            (Terrain::HerbGarden, "分家巷・艾草收曬園", "專門栽植與收曬端午艾草製作祛寒艾絨的乾燥小圃。"),
            (Terrain::Blacksmith, "分家巷・磨刀利刃鋪", "常年為街坊鄉鄰細磨各品剪刀、菜刀與柴刀之利刃磨坊。"),
            (Terrain::Road, "分家巷・巧織石板巷", "雲錦織造坊外長條形青石小巷，兩旁窗內常傳出札札機杼織布聲。"),
            (Terrain::House, "分家巷・便鞋雨笠居", "用稻草與棕絲編製輕軟布鞋與防雨竹笠的勤苦人家。"),
            (Terrain::Bridge, "分家巷・流水清韻橋", "渠水在橋下穿石而過發出清脆潺潺聲響的小石拱橋。"),
            (Terrain::House, "分家巷・銅鏡磨鏡舍", "運用特製研磨錫粉替女子出嫁古銅鏡重新磨亮生輝的工匠齋房。"),
            (Terrain::Road, "分家巷・工坊迴車道", "方便重載木料與鐵錠之大車轉彎出入工坊區的圓形石鋪彎道。"),
            (Terrain::House, "分家巷・鍛工歇息寮", "鍛爐換班鐵匠暫歇擦汗喝大麥茶之通風歇宿所，備有長條涼蓆木榻。"),
            (Terrain::HerbGarden, "分家巷・活水水芹圃", "沿著水渠淺灘自然繁茂的水芹菜青圃，春季嫩綠可人。"),
            (Terrain::House, "分家巷・傘骨竹篾舍", "以巧手劈削細韌油紙傘骨與竹燈籠骨架的細竹工住所。"),
            (Terrain::Road, "分家巷・爐煙斜石階", "順著山坡微升之條石石階，傍晚時分炊煙與爐火熱氣在晚霞中裊裊升騰。"),
            (Terrain::Workshop, "分家巷・木器榫眼作坊", "鑽研桌椅接角不用一釘一鐵之純木構家具工舍。"),
            (Terrain::House, "分家巷・安居鐵石院", "分家巷世代為匠之家族大院，院落堅固整潔，家風質樸勤勞。"),
        ];

        let sw_rem_defs: &[(Terrain, &str, &str)] = &[
            (Terrain::Grassland, "西門駝獸草料坪", "西門聯運站後方的平坦草坪，堆疊著整齊包紮的甜苜蓿草料，雙輪大車的駝獸在此咀嚼草料、稍事歇息。"),
            (Terrain::Road, "西門卸貨石坪", "西門聯運大車停靠卸貨的寬闊厚石坪，四處堆放著自各鎮運來的粗布捆、礦石箱與木箱，駕駛員正忙著過磅與交割。"),
            (Terrain::Workshop, "西門・車轅大木作", "專門製造與維修各型大車車軸、硬木輪圈與挽具轅木的重木工坊。"),
            (Terrain::Road, "西門・車道過磅台", "聯運總站前埋設地中之重型青石過磅台，專為進城貨車核驗載重總量。"),
            (Terrain::Workshop, "西門・皮帶鞍套坊", "縫製駝獸拉車專用加厚生牛皮鞍墊與胸帶的熟皮手藝人家。"),
            (Terrain::Grassland, "西門・馱獸放牧場", "西城門外圍圈設的青青牧草地，長途跋涉歸來的馱獸在此自由吃草奔跑。"),
            (Terrain::Inn, "西門・行商長途歇舍", "專供各城鎮遠道而來之車隊行商投宿過夜的平價整潔通鋪大旅社。"),
            (Terrain::Road, "西門・貨箱堆疊場", "整齊碼放待裝車之大宗貨物木箱的石鋪空場，四周配備防火水缸。"),
            (Terrain::Blacksmith, "西門・鐵蹄釘掌鋪", "為長途行路的耐磨馱獸與草獸修剪蹄殼並釘扣鐵護掌的鐵匠鋪。"),
            (Terrain::Grassland, "西門・苜蓿甜草圃", "精選優質耐寒苜蓿草種培育之高產牧草園圃，青翠豐茂。"),
            (Terrain::Workshop, "西門・行車雨布作坊", "用生桐油浸透厚帆布以製作大車防雨罩棚與油布包裝袋的工舍。"),
            (Terrain::Road, "西門・車隊列隊坪", "清晨各路遠行車隊整裝待發、清點人員車輛的大型集結石坪。"),
            (Terrain::House, "西門・通事向導舍", "熟悉半島各山路險阻與渡口潮汐之老嚮導聚居之所。"),
            (Terrain::Grassland, "西門・飲馬清草塘", "連通清泉之蓄水青草池塘，專供往來駝獸馱馬暢飲止渴。"),
            (Terrain::Workshop, "西門・乾糧備製作坊", "烤製耐久放之黑糖米香餅、牛肉乾與鹽炒青豆等行路乾糧的家庭食品房。"),
            (Terrain::Road, "西門・聯運公秤道", "前往官方公秤房核驗貨物重量之專用平坦青石路，路面筆直堅硬。"),
            (Terrain::House, "西門・車馬水夫舍", "專責在西門水井打水清洗車身與照料駝獸洗浴的水伕居所。"),
            (Terrain::Grassland, "西門・綠草平波坪", "聯運站側翼供駕駛員打盹小憩的柔軟草坪，常有微風自草原吹拂而來。"),
            (Terrain::GeneralStore, "西門・油膏滑脂鋪", "熬製塗抹車軸防止高溫磨損的特製羊脂松香潤滑膏之專門工舍。"),
            (Terrain::Road, "西門・防滑石板坡", "為利重載大車平穩出入城門而特意雕鑿橫向防滑槽的堅實斜坡道。"),
            (Terrain::House, "西門・繩索編結舍", "以大麻纖維搓揉粗壯耐拉之捆車緊繩與韁繩的麻繩手藝作坊。"),
            (Terrain::Grassland, "西門・歇息草亭苑", "在青草坪中央搭建之茅草遮陽大亭，亭內設有長木凳可供旅人歇足。"),
            (Terrain::House, "西門・防雨竹笠院", "批發販售大號防曬遮雨雙層竹笠與防泥草鞋的商人家宅。"),
            (Terrain::Road, "西門・城門集市街", "臨近西城門口之自發性小集市，沿街售賣當季瓜果與熱煎餅，人氣旺盛。"),
            (Terrain::GuardPost, "西門・長途聯運護衛哨所", "護送貴重商貨車隊之隨隊安保員與護衛駐紮之硬石排屋。"),
            (Terrain::Grassland, "西門・春風牧草甸", "聯運站西南角一處開闊的天然草甸，百草叢生，野花點綴其間。"),
            (Terrain::Workshop, "西門・行囊修補行", "為旅人修補磨破之皮革行囊、水壺背帶與斜背包的老手藝居室。"),
            (Terrain::Road, "西門・大車洗塵石台", "配備長竹水管可引清水沖刷車輪泥濘與車廂浮塵的清洗平台。"),
            (Terrain::Market, "西門・鹽巴草料行", "專門批售添加於駝獸飼料中以增強體力之粗海鹽與乾草餅的商家。"),
            (Terrain::Grassland, "西門・青坪望遠坡", "地勢微隆之草坡，站於坡頂可遠望西南原與西向連綿起伏的山野。"),
            (Terrain::House, "西門・駕駛員同盟舍", "西門常駐駕駛員自發組建互相扶助之行會會舍，牆上掛著行車安全規約。"),
            (Terrain::Road, "西門・西出大道前段", "穿過西城門出入甕城之寬廣石板大道，路旁設有拒馬石樁。"),
            (Terrain::House, "西門・車隊帳房齋", "核對每日長途運費、墊付款項與貨物提單之總帳簿室。"),
            (Terrain::House, "西門・夜歸守更房", "專司看守西門貨場夜間安全、防範火燭盜賊的守更人直宿值房。"),
            (Terrain::House, "西門・安平車馬院", "西門街區一處寬大厚重的四合大院，駕駛員家眷居於此，生活安居樂業。"),
        ];

        let quarters = [
            (nw_slots, &bldg_types[0..14], nw_rem_defs),
            (ne_slots, &bldg_types[14..28], ne_rem_defs),
            (se_slots, &bldg_types[28..42], se_rem_defs),
            (sw_slots, &bldg_types[42..55], sw_rem_defs),
        ];

        for (slots, bldgs, rem_defs) in quarters {
            for (idx, (_, _, coord)) in slots.into_iter().enumerate() {
                if idx < bldgs.len() {
                    let (terrain, bldg_name) = bldgs[idx];
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
                    cell.zone = city_name.to_string();
                    cell.description = generate_settlement_building_desc(city_name, bldg_name, terrain);
                    cell.explored = true;
                    map.insert(coord, cell);
                } else {
                    let rem_idx = idx - bldgs.len();
                    let (terrain, place_label, desc) = rem_defs[rem_idx % rem_defs.len()];
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, place_label));
                    cell.zone = city_name.to_string();
                    cell.description = desc.to_string();
                    cell.explored = true;
                    map.insert(coord, cell);
                }
            }
        }


        // 外城四街區配置（120 棟正典地標建築：40 棟商業／功能地標 + 80 棟民居住宅）
        let mut outer_nw_slots = Vec::new();
        let mut outer_ne_slots = Vec::new();
        let mut outer_se_slots = Vec::new();
        let mut outer_sw_slots = Vec::new();

        for (dx, dy, coord) in outer_slots {
            if dx < 0 && dy > 0 {
                outer_nw_slots.push((dx, dy, coord));
            } else if dx > 0 && dy > 0 {
                outer_ne_slots.push((dx, dy, coord));
            } else if dx > 0 && dy < 0 {
                outer_se_slots.push((dx, dy, coord));
            } else {
                outer_sw_slots.push((dx, dy, coord));
            }
        }

        let outer_sort_fn = |&(dx, dy, _): &(i32, i32, SquareCoord)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy.abs(), dx.abs());
        outer_nw_slots.sort_by_key(outer_sort_fn);
        outer_ne_slots.sort_by_key(outer_sort_fn);
        outer_se_slots.sort_by_key(outer_sort_fn);
        outer_sw_slots.sort_by_key(outer_sort_fn);

        let outer_nw_defs: &[(Terrain, &str, &str)] = &[
            // 商業與功能設施 (10)
            (Terrain::Dock, "三水外坊・北灣貨船大埠", "三水外坊面向北灣的繁忙貨船大埠，條石駁岸整齊延伸，泊滿了自沿河各鎮滿載而來的大型貨船與雙桅帆船。"),
            (Terrain::Market, "三水外坊・早市鮮魚拍賣場", "清晨天未破曉便燈火通明的鮮魚拍賣大場，石板地面沖洗得水光瓦亮，剛出水的銀鱗大魚整框競價，叫賣聲此起彼伏。"),
            (Terrain::Warehouse, "三水外坊・通濟河運總倉", "通濟河運專設的巨型防潮石砌總倉，庫房高大通風，整齊碼放著成批的糧穀、海鹽與布匹，管事穿梭盤點。"),
            (Terrain::Workshop, "三水外坊・大船桅帆作坊", "專門縫製與修補江海大船桅帆的寬闊作坊，地面鋪著整幅厚重帆布，工匠手持骨針與麻線熟練穿引。"),
            (Terrain::Tavern, "三水外坊・臨江水手茶飯館", "臨江而建的水手茶飯館，推窗便見滔滔江水，灶上燉著鮮美滾燙的雜魚豆腐湯，是水手卸貨後聚飲解乏的首選之地。"),
            (Terrain::Inn, "三水外坊・北灣臨水旅社", "為往來船客與押貨行商提供住宿的臨水旅社，客房乾淨清爽，通風良好，夜裡伴著江濤拍岸之聲安然入眠。"),
            (Terrain::GeneralStore, "三水外坊・漁具簑笠商號", "專營各色江釣漁具、堅韌麻繩、桐油防水雨衣與雙層防雨竹笠的商號，門口懸掛著成束的釣絲與青竹竿。"),
            (Terrain::Workshop, "三水外坊・河鮮鹽漬加工坊", "採用傳統古法鹽漬風乾河鮮魚蝦的石砌工坊，院中排滿了整齊的大陶缸與晾曬竹架，鹹香撲鼻。"),
            (Terrain::Bank, "三水外坊・碼頭匯兌分號", "三水總商行設於北灣碼頭的匯兌分號，櫃台以厚重鐵柵防護，專門為船隊行商辦理大額匯票存取與水運保單。"),
            (Terrain::GuardPost, "三水外坊・北水道巡河哨所", "扼守文曜北向水運咽喉的巡河哨所，石砌哨樓高聳，配備快艇與巡防兵勇，日夜查驗過往船隻船引與水路安全。"),
            // 住宅民居 (20)
            (Terrain::House, "三水外坊・老舵手石砌院", "闖蕩江海數十載的老舵手修建的厚實石院，屋內擺放著褪色的水路輿圖與舵輪模型，透著老水手的剛毅。"),
            (Terrain::House, "三水外坊・蘆洲漁家小築", "傍依蘆葦水洲搭建的雅緻小築，竹籬笆圍著青石小徑，門前繫著輕巧捕魚划子，屋頂覆著厚厚茅草。"),
            (Terrain::House, "三水外坊・清灣船木匠舍", "精通修補船殼與水密艙板的船木匠宅舍，院中堆滿了耐腐浸泡的樟木與松木料，散發著木脂清香。"),
            (Terrain::House, "三水外坊・北埠引水人居", "熟悉北灣暗礁與淺灘潮汐的引水員住家，堂屋懸掛著潮汐表與水深竹尺，常有外來船長登門重金求引。"),
            (Terrain::House, "三水外坊・雙槳渡夫家屋", "長年在內河兩岸擺渡客商的渡夫屋舍，門旁立著兩對油光水滑的硬木船槳，門檻被水鞋踏得光滑。"),
            (Terrain::House, "三水外坊・河風曬魚人家", "以曬製小銀魚與蝦皮為業的勤勞人家，平頂屋面上鋪滿了篾席，在陽光下曬著銀白發亮的河鮮。"),
            (Terrain::House, "三水外坊・順流竹筏工舍", "扎製載貨排筏竹匠的工舍，院落寬廣，整齊碼放著通直挺拔的三年生毛竹，工匠正以炭火烤竹定型。"),
            (Terrain::House, "三水外坊・碧波網戶大院", "世代以撒網捕魚為生的網戶家族大院，天井裡晾掛著多張細密絲網，全家老少圍坐修補網眼。"),
            (Terrain::House, "三水外坊・石磯汲水民居", "緊靠天然突起石磯的清幽民居，取水極為便利，婦女常在平整石磯上浣洗衣物，水花飛濺。"),
            (Terrain::House, "三水外坊・水門看閘人舍", "專司啟閉三水北水門蓄水防洪的閘官值宿舍，牆上掛著沉重的鐵鑄水門搖柄與水位標記水牌。"),
            (Terrain::House, "三水外坊・白鷺洲頭人家", "座落於水中小洲洲頭的恬淡居所，四周碧波蕩漾，水鳥常棲息於屋旁垂柳，環境宛若世外桃源。"),
            (Terrain::House, "三水外坊・長堤護岸工居", "負責維護江堤石駁與護坡植被的河工住處，門前整齊排著夯土石碾與修堤鐵鍬，家風質樸。"),
            (Terrain::House, "三水外坊・泊船計籌帳舍", "碼頭調度船位與清點泊位租金的計籌員居舍，桌案上堆放著各色木質泊船籌碼與登記流水簿。"),
            (Terrain::House, "三水外坊・魚簍編織家院", "用細篾青竹編造各式捕魚籠、蝦簍與遮陽防雨竹笠的竹編匠人宅院，空氣中洋溢著新鮮青竹的清甘氣味。"),
            (Terrain::House, "三水外坊・江風茶屋主理人宅", "在碼頭茶屋精研烹茶技藝的茶屋主理人家宅，室內收藏著多套紫砂陶壺與各地名茶，茶香清冽宜人。"),
            (Terrain::House, "三水外坊・平水艙板匠宅", "擅長打造客船底艙水平防漏鋪板的巧匠住宅，屋宇修築得極具水工力學美感，嚴絲合縫。"),
            (Terrain::House, "三水外坊・水巷搬運員宿所", "往來穿梭於水巷狹弄運送重物的搬運員合宿屋舍，廊下整齊擺著硬桑木扁擔與加厚麻布墊肩。"),
            (Terrain::House, "三水外坊・垂柳河灣民宅", "掩映在一彎清溪與依依垂柳後的溫馨民居，推門便是清澈流水，春日柳絮紛飛如雪，景色怡人。"),
            (Terrain::House, "三水外坊・晴波燈火人家", "臨水依岸修築的小巧閣樓，夜間樓前高懸防風氣死風燈，為江上夜航歸來的孤舟指引泊岸方向。"),
            (Terrain::House, "三水外坊・安瀾水手大院", "多名資深水手合資興建的硬石大合院，院落敞亮寬厚，節慶時水手們圍聚烤肉暢飲，豪邁熱絡。"),
        ];

        let outer_ne_defs: &[(Terrain, &str, &str)] = &[
            // 商業與功能設施 (10)
            (Terrain::Academy, "講堂外坊・格物輿圖外院", "文曜學院設立於外坊的格物推步與山川輿圖研習院，堂中懸掛全島大幅精繪地圖，學生在此研習測繪之術。"),
            (Terrain::Library, "講堂外坊・活字排版印書局", "採用活字泥字與錫活字排版印行典籍的宏大印書局，排字架排列整齊，匠人飛速挑字，墨香四溢。"),
            (Terrain::Workshop, "講堂外坊・青檀宣紙大工坊", "以精選青檀皮與沙田稻草古法抄製宣紙的大工坊，抄紙石槽水流清澈，烘壁上張張宣紙潔白如雪。"),
            (Terrain::Workshop, "講堂外坊・松煙徽墨精製坊", "收集古松名脂煉煙、研膠槌打的精墨作坊，工匠揮動重錘千槌百煉，製出的墨錠黑潤光潔，帶有麝香氣。"),
            (Terrain::Clinic, "講堂外坊・杏林百草醫館", "名醫坐堂問診的仁心醫館，藥香濃郁，四壁藥櫃羅列數百種靈草乾植，醫師辨證施治，深得街坊敬重。"),
            (Terrain::GeneralStore, "講堂外坊・文房四寶總號", "匯聚各地名筆、佳墨、名硯、宣紙的文房總號，案頭擺設典雅，文士學生常在此流連品評選購。"),
            (Terrain::Inn, "講堂外坊・青雲求學客舍", "專供四方赴考求學學生居住的清雅客舍，庭院靜謐，室內配有書案明燈，學風濃厚，求學問道者雲集。"),
            (Terrain::Tavern, "講堂外坊・翰墨清茶書肆", "兼營品茗與閱覽書報的雅緻茶屋，清茶一盞，書卷在手，師生常在此圍坐清談，激盪文思。"),
            (Terrain::Warehouse, "講堂外坊・善本紙料儲庫", "防潮防火措施極其完備的善本圖籍與高階紙料儲庫，厚石高牆，樟木隔板，終年乾燥清涼。"),
            (Terrain::Observatory, "講堂外坊・北辰測風高台", "築於高地之上的青石測風觀星台，台上立有銅製候風地動儀與北辰定緯儀，供天文官測算氣象星象。"),
            // 住宅民居 (20)
            (Terrain::House, "講堂外坊・刻書雕版匠居", "畢生精研梨木棗木字版微雕的大匠居室，案几上鋪滿了各型刻刀與放大水晶鏡，運刀如筆，神乎其技。"),
            (Terrain::House, "講堂外坊・修竹讀書山房", "四周修竹環抱的幽僻讀書山房，窗前設琴台石硯，風吹竹葉沙沙作響，是文人學者著書立說之處。"),
            (Terrain::House, "講堂外坊・老教席靜養院", "學院退職名儒安度晚年的恬適庭院，古木參天，階生苔痕，時有昔日高徒攜禮前來問候請益。"),
            (Terrain::House, "講堂外坊・青燈抄書人舍", "專司為藏書閣楷書謄抄孤本珍冊的抄書人家，室內一几一椅一燈，案頭字帖工整如印，筆意沉靜。"),
            (Terrain::House, "講堂外坊・采藥本草人家", "常年攀登北嶺懸崖採集珍稀本草的藥農住家，簷下晾曬著成捆野生草藥，散發著微苦清香。"),
            (Terrain::House, "講堂外坊・澄心洗硯小築", "引清泉穿石而過形成的洗硯石池住宅，水底沉積墨韻，池畔植有水仙菖蒲，清雅不俗。"),
            (Terrain::House, "講堂外坊・古琴調律雅舍", "深諳音律宮商調式的古琴名家精舍，四壁懸掛各朝名琴，指法清越，琴音如鳴佩環。"),
            (Terrain::House, "講堂外坊・會計主計家院", "文曜官方戶籍與度量衡算官的清廉家宅，桌頭堆疊九章算術與算籌木盒，條理分明。"),
            (Terrain::House, "講堂外坊・竹林晒紙人家", "專門承攬晾曬剛抄出紙張的作坊民居，晴日時後山竹坡上白紙連綿成片，宛若鋪雪。"),
            (Terrain::House, "講堂外坊・梅影詩社別院", "外坊文人結社定期分韻賦詩的精巧別院，冬日臘梅吐蕊，暗香浮動，滿壁皆是酬唱佳作。"),
            (Terrain::House, "講堂外坊・石鼓拓碑匠宅", "專事野外石刻碑銘精拓的拓工居所，牆上高懸珍貴前代摩崖拓片，墨氣古雅厚重。"),
            (Terrain::House, "講堂外坊・蘭谿伴讀書舍", "沿著清淺小溪修築的伴讀書舍，溪水淙淙，伴以琅琅書聲，令人心無雜念，專注向學。"),
            (Terrain::House, "講堂外坊・松下煎茶人家", "門前有一株百年偃柏古松的民家，主人擅長以活水慢火烹煎蒙頂新茶，鄰里常聞茶香。"),
            (Terrain::House, "講堂外坊・裝幀線裝工居", "精通古籍線裝、包角與錦綾封面裝幀的匠人家宅，屋內堆滿絲線、熟骨針與天然松香黏劑。"),
            (Terrain::House, "講堂外坊・觀星司曆官舍", "專司修訂四時節氣曆法的曆官宅第，天台設有小型銅儀，夜深人靜時猶在此推步星野。"),
            (Terrain::House, "講堂外坊・白鹿遠遊學寮", "遠道求學學生結伴同宿的石木學寮，臥房整肅，同學在此攻讀切磋，意氣風發。"),
            (Terrain::House, "講堂外坊・丹青畫師庭院", "專精工筆山水與文人墨戲的花甲畫師居所，畫案寬大，宣紙四展，滿目青綠山川之美。"),
            (Terrain::House, "講堂外坊・金石篆刻人居", "以鐵筆篆刻青田、壽山印石的名家書齋，抽屜裡整齊藏著數百方精雕印鈕，朱砂印泥芬芳。"),
            (Terrain::House, "講堂外坊・梧桐書聲家宅", "庭院中植有挺拔梧桐樹的詩禮傳家宅邸，庭訓嚴謹，晨昏常有子弟在桐蔭下朗讀經史。"),
            (Terrain::House, "講堂外坊・文昌清平大院", "講堂外坊一處敦睦和樂的街坊大合院，鄰里多為教席印工家眷，互敬互愛，書香浸潤。"),
        ];

        let outer_se_defs: &[(Terrain, &str, &str)] = &[
            // 商業與功能設施 (10)
            (Terrain::Blacksmith, "百工外坊・重鐵水錘大鍛坊", "引外河湍流帶動巨大木輪驅動重鐵水錘的大鍛坊，鐵砧震天轟鳴，火星如雨，鍛造大宗厚鋼零件。"),
            (Terrain::Workshop, "百工外坊・魯班大木車船坊", "營造載重雙輪大車與內河駁船的宏偉大木坊，立木如林，匠人以墨斗彈線、巨斧劈木，氣魄雄渾。"),
            (Terrain::Workshop, "百工外坊・青釉官窯燒造局", "配備長條斜坡龍窯的陶瓷名局，專造胎質堅細的碧青釉器皿與大缸，爐溫千度，窯火晝夜不滅。"),
            (Terrain::GeneralStore, "百工外坊・五金農具總商行", "批售全島所需鋼鐵鋤、鍬、鐮、鎬等農具與五金件的總行，貨品結實厚重，質優耐磨。"),
            (Terrain::Warehouse, "百工外坊・焦炭生鐵大棧房", "儲備煉鐵生鐵錠、優質焦炭與石灰石的重型厚石倉庫，防火防潮，為全坊鍛爐供應源源不絕之薪火。"),
            (Terrain::Workshop, "百工外坊・雲錦機杼大染坊", "以植物天然青藍、紫草染料染製五彩長綢的大染坊，染架高聳入雲，匹匹彩緞如飛瀑垂掛。"),
            (Terrain::Tavern, "百工外坊・紅爐烈酒大飯鋪", "專為辛苦鍛冶工匠提供高熱量飯食的熱鬧大飯鋪，烈酒燒刀子配大碗熟牛肉，滿堂歡聲笑語。"),
            (Terrain::Inn, "百工外坊・四方匠師行館", "接待半島各鎮前來文曜交流切磋技藝之名匠名師的專用行館，石砌厚實，陳設兼具工藝美感。"),
            (Terrain::Market, "百工外坊・東城器物交易場", "展銷各類精工木器、銅器、鐵器、陶器的開闊露天交易場，四方客商在此比貨議價，熱鬧非凡。"),
            (Terrain::GuardPost, "百工外坊・東門火政巡防所", "因應百工坊鍛爐林立而專設的防火與防盜重裝巡防所，配備滅火水龍、防護皮衣與高架瞭望台。"),
            // 住宅民居 (20)
            (Terrain::House, "百工外坊・司爐老鐵匠院", "長年侍弄千度高溫鍛爐的老鐵匠大院，門前長條石桌上擺著自製的耐用鐵鉗，滿門皆是硬漢。"),
            (Terrain::House, "百工外坊・大木作梁架居", "專事精算高堂大殿大梁架榫卯結構的掌繩大木匠宅邸，院內陳放著精確的大木模型與曲尺。"),
            (Terrain::House, "百工外坊・青瓷拉坯人舍", "能在飛旋陶輪上一手拉出薄如蛋殼陶坯的巧手陶工家宅，簷下排滿了待乾的素陶坯體。"),
            (Terrain::House, "百工外坊・染缸看色匠宅", "憑一雙利眼能辨微小色差的資深調色染匠住宅，院內放著幾口老染缸，雙手常年染著洗不褪的群青。"),
            (Terrain::House, "百工外坊・熟皮鞍具家院", "以特製油劑熟化厚重牛皮裁製馬鞍韁繩的皮匠宅舍，皮料柔韌光亮，散發著皮革天然脂香。"),
            (Terrain::House, "百工外坊・銅壺鏨花工居", "以細鏨鐵錘在紫銅壺面上鏨刻花鳥山水的鏨金巧匠居室，敲擊聲清脆密集，宛若雨打芭蕉。"),
            (Terrain::House, "百工外坊・石橋鑿石匠舍", "擅長開採玄武石並鑿出平整咬合榫槽的石作大師居處，門前階梯雕刻著靈動石獅，工力精純。"),
            (Terrain::House, "百工外坊・炭窯燒炭人家", "在近郊依山築窯燒製上等無煙硬木白炭的炭翁家宅，屋角整齊堆放著黑亮清脆的特等炭條。"),
            (Terrain::House, "百工外坊・箍桶木匠小院", "專精以鐵箍竹篾箍緊水桶、酒桶與浴桶的箍桶匠院落，敲打拼板之聲整日不絕於耳。"),
            (Terrain::House, "百工外坊・織機修梭人居", "擅長微調織布機提花梭子與踏板聯動機構的巧匠住宅，案上滿是精密的黃銅小齒輪與頂針。"),
            (Terrain::House, "百工外坊・琉璃吹製匠宅", "手持鐵吹管將熔融琉璃料吹製為晶瑩瓶盞的玻璃巧匠住宅，架上陳列著各色流光溢彩的琉璃器皿。"),
            (Terrain::House, "百工外坊・金銀錯絲工舍", "精通在青銅鐵器表面鏨槽嵌入極細金銀絲紋飾的細金名師精舍，室內備有微型油燈與精鋼刻針。"),
            (Terrain::House, "百工外坊・車輪鑲鐵家屋", "將燒紅鋼箍趁熱套入大木車輪急速冷縮固定的車輪匠家屋，屋外立著數面一人多高的重載車輪。"),
            (Terrain::House, "百工外坊・瓦作琉璃匠院", "擅長燒製屋頂覆蓋之防雨青筒瓦與彩繪琉璃脊獸的泥瓦匠大院，瓦片相扣，清脆鏗鏘。"),
            (Terrain::House, "百工外坊・磨坊水輪工居", "維護外河巨型提水磨粉木輪的機械工匠宅第，屋內掛著各式齒輪傳動軸草圖，匠心獨運。"),
            (Terrain::House, "百工外坊・竹編大簍人家", "用粗篾竹條編造能盛裝數百斤糧穀礦石大竹簍的編織能手宅舍，院子裡堆滿了堅實的大簍成品。"),
            (Terrain::House, "百工外坊・繩墨魯班學寮", "百工學徒合宿的質樸學寮，青年們在此互相檢驗刨面平整度與鑿眼垂直度，研習繩墨規矩。"),
            (Terrain::House, "百工外坊・鐵砂淘洗工舍", "在近河淺灘淘洗富鐵黑砂以供冶煉的淘砂工人住處，石槽內淘洗出的黑鐵砂沉重如鉛。"),
            (Terrain::House, "百工外坊・百鍊刀劍匠宅", "精於摺疊鍛打百煉夾鋼刀劍的名劍匠住宅，壁上掛著數把寒光凜凜的試斬短刀，鋒銳無匹。"),
            (Terrain::House, "百工外坊・天工安居大院", "百工外坊多個工匠家族同住的深宅大院，鄰里間彼此照應，技術互補，生活樸實繁榮。"),
        ];

        let outer_sw_defs: &[(Terrain, &str, &str)] = &[
            // 商業與功能設施 (10)
            (Terrain::Caravanserai, "西門外坊・萬里駝隊大驛站", "連通半島西部荒原的大型駝隊驛站，寬廣的欄圈內駐足著耐粗飼的原牛駝獸，駝鈴陣陣，商旗飄揚。"),
            (Terrain::Market, "西門外坊・西南原皮毛大市", "專營自西部運抵之原牛皮、狼皮、羊毛及氈毯的露天交易大市，皮貨堆積如山，空氣中瀰漫著熟皮香。"),
            (Terrain::Warehouse, "西門外坊・戰備糧草總儲倉", "文曜城西高築防禦條石的大型糧草總庫，儲備著壓縮草磚、熟炒米麩粉與乾肉餅，防禦完備。"),
            (Terrain::Workshop, "西門外坊・重載雙輪大車廠", "專門打造適合荒原長途跋涉的鐵裹重軸雙輪大馬車廠，車架皆選用陳年老雲杉，堅不可摧。"),
            (Terrain::Inn, "西門外坊・風塵客商大旅社", "專門接待長途跋涉歸來的車隊行商與護衛的大型旅社，備有溫暖通鋪、熱水洗浴槽與安全存車場。"),
            (Terrain::Tavern, "西門外坊・大碗羊肉湯餅鋪", "全日熬煮濃郁原羊骨高湯的老字號湯餅鋪，大塊羊肉配熱騰騰手揪麵片，驅寒暖胃，客座常滿。"),
            (Terrain::Blacksmith, "西門外坊・車軸鐵箍鍛修鋪", "緊鄰驛站的專業鐵匠鋪，專事換修大車受損之鍛鐵車軸、輪箍與馬蹄掌，爐火熊熊，晝夜不息。"),
            (Terrain::GeneralStore, "西門外坊・遠行鞍轡帳篷行", "專售遠行必備之牛皮鞍轡、厚羊毛氈毯、防水帆布帳篷與防風銅燈的行號，品類齊全耐用。"),
            (Terrain::GuildHall, "西門外坊・四方安保總堂", "掌理半島西線各路貨運押送與護衛的四方安保總部，院落寬闊，安保員演武練功，威震四方。"),
            (Terrain::GuardPost, "西門外坊・西城關防驗契所", "扼守西門官道的重要關防驗契哨所，設有拒馬石椿，嚴查進出車輛貨單、通關路引與違禁貨品。"),
            // 住宅民居 (20)
            (Terrain::House, "西門外坊・老護衛四合院", "四方安保老護衛退隱治家的石砌四合院，堂上高懸封刀鐵劍與護航錦旗，威嚴沉著。"),
            (Terrain::House, "西門外坊・駝隊領隊家宅", "常年穿梭西原引導商隊的老領隊家宇，屋內隨處可見長途手繪路線圖與指南羅盤，久歷風霜。"),
            (Terrain::House, "西門外坊・牧草飼養人居", "精於培育高營養乾苜蓿草餅飼料的牧草農戶家舍，廊下整齊碼放著緊實草磚，草香四溢。"),
            (Terrain::House, "西門外坊・大車木軸匠舍", "精選整根老苦楝木削製耐重車轅與輪軸的巧匠住所，院內浸泡木料的石池常年清泉湧流。"),
            (Terrain::House, "西門外坊・長途押運員寮", "負責護送貨物之隨行店員合宿舍，室內掛著硬藤甲與防泥長靴，整裝待發。"),
            (Terrain::House, "西門外坊・皮革硝製家院", "以天然芒硝法軟化皮革的工匠住宅，後院設有多處浸皮木槽，熟化出來的皮料柔若絲綢。"),
            (Terrain::House, "西門外坊・西道斥候宿所", "負責定期巡查西向道路坍方與野獸出沒的斥候居所，配備精良短弓與疾行草獸。"),
            (Terrain::House, "西門外坊・驛馬醫護人居", "擅長調理長途駝獸蹄傷、腸胃寒熱的獸醫民居，屋內擺滿了特大號草藥丸與外敷油膏罐。"),
            (Terrain::House, "西門外坊・麻袋打包工舍", "專事以粗麻布袋嚴密打包散裝礦石與穀物的打包工宿舍，粗麻線球與長骨錐整齊碼放。"),
            (Terrain::House, "西門外坊・車馬過磅吏宅", "掌管西門大秤校驗貨物毛重淨重之秤吏宅第，庭院中立著一座小型標準砝碼石台。"),
            (Terrain::House, "西門外坊・行商貨棧管事居", "西門外多家貨棧聯合聘任之總管事居所，案頭堆疊各地進出貨物提單與貨位號牌。"),
            (Terrain::House, "西門外坊・毛氈編織人家", "將純羊毛反覆搓揉壓實製作保暖防雨毛氈墊的匠人家舍，滿院鋪滿了雪白厚實的羊毛氈。"),
            (Terrain::House, "西門外坊・油布車篷匠院", "以熟桐油細刷帆布製作大車遮雨防曬雙層車篷的作坊人家，院中支起數座巨大試裝車篷。"),
            (Terrain::House, "西門外坊・銅鈴馬具工居", "精鑄駝鈴、馬鐙、嚼子等五金配件的銅匠小院，微風吹過，各音調的清脆銅鈴隨風叮咚。"),
            (Terrain::House, "西門外坊・旱路搬運員聚義舍", "西線各路搬運員互相扶持組建的歇宿聚所，門前設有免費施奉的熱薑茶木桶，溫情暖心。"),
            (Terrain::House, "西門外坊・乾肉焙烤人家", "調配香料慢火烘烤耐儲存乾牛肉餅的勤快人家，土石烤爐終年熱氣騰騰，肉香四溢。"),
            (Terrain::House, "西門外坊・巡城夜更人舍", "掌管西外城牆夜間提燈巡更、敲梆報時的巡更人住舍，門旁懸掛著銅鑼與防風燈籠。"),
            (Terrain::House, "西門外坊・商隊嚮導家屋", "通曉半島各山川小徑與水源地位置的老嚮導寓所，常有外來客商登門討教行路經驗。"),
            (Terrain::House, "西門外坊・西關歇腳小院", "西關門外一處提供過路行人短暫歇足喝水的淳樸農家小院，竹蔭清涼，茶碗整潔。"),
            (Terrain::House, "西門外坊・順風車馬大院", "西門外坊多支長途車隊共同購置的寬闊大宅院，院落能容數輛大車停泊，生活安定和睦。"),
        ];

        let outer_quarters = [
            (outer_nw_slots, outer_nw_defs, "三水外坊"),
            (outer_ne_slots, outer_ne_defs, "講堂外坊"),
            (outer_se_slots, outer_se_defs, "百工外坊"),
            (outer_sw_slots, outer_sw_defs, "西門外坊"),
        ];

        for (slots, bldgs, quad_prefix) in outer_quarters {
            for (idx, (_, _, coord)) in slots.into_iter().enumerate() {
                if idx < bldgs.len() {
                    let (terrain, bldg_name, desc) = bldgs[idx];
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
                    cell.zone = city_name.to_string();
                    cell.description = desc.to_string();
                    cell.explored = true;
                    map.insert(coord, cell);
                } else {
                    let rem_idx = idx - bldgs.len();
                    let (terrain, place_label, desc) = match rem_idx % 4 {
                        0 => (
                            Terrain::HerbGarden,
                            format!("{}・街坊花圃", quad_prefix),
                            format!("{}街坊間青石圍砌的休憩花圃，微風送爽，花木繁盛，街坊常在此歇涼。", quad_prefix),
                        ),
                        1 => (
                            Terrain::Bridge,
                            format!("{}・引水石橋", quad_prefix),
                            format!("跨越{}引流石渠的小巧石板橋，水清見底，水聲潺潺。", quad_prefix),
                        ),
                        2 => (
                            Terrain::Road,
                            format!("{}・街坊廣場", quad_prefix),
                            format!("{}開闊平整的街坊廣場與中庭石坪，石板整潔，坊民在此休憩交談。", quad_prefix),
                        ),
                        _ => (
                            Terrain::Grassland,
                            format!("{}・綠草平坪", quad_prefix),
                            format!("{}外圍開闊柔軟的青草平坪，垂柳依依，清風拂面。", quad_prefix),
                        ),
                    };
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, &place_label));
                    cell.zone = city_name.to_string();
                    cell.description = desc;
                    cell.explored = true;
                    map.insert(coord, cell);
                }
            }
        }
    } else {
        let mut inner_slots = Vec::new();

        // 預留西港 15 棟核心海事地標的專屬臨水與幹道座標（確保全部緊鄰碼頭、長堤與主街）
        let key_slots = [
            (-1, 0),   // 0: 風舶海事商盟總會
            (-1, 1),   // 1: 海事總商行與船貨契據所
            (-1, -1),  // 2: 風舶航海圖誌館
            (-3, -4),  // 3: 林氏大船塢
            (-3, -3),  // 4: 馭風帆具與船材工坊
            (-3, -2),  // 5: 港務脈絡診所
            (-3, 3),   // 6: 海風海鮮食堂
            (-3, 2),   // 7: 港灣溫水浴場
            (-3, 1),   // 8: 海員旅社
            (4, 0),    // 9: 港尾車馬聯運站
            (3, 0),    // 10: 海陸聯運貨運行
            (-3, 0),   // 11: 深水泊位與卸貨碼頭
            (-3, 4),   // 12: 港區魚獲水產交易行
            (-3, 5),   // 13: 築港頭・許家宅邸
            (-3, -5),  // 14: 大船塢・林家宅院
        ];
        let key_slots_set: HashSet<(i32, i32)> = key_slots.iter().copied().collect();

        for dx in -10i32..=10i32 {
            for dy in -10i32..=10i32 {
                let coord = SquareCoord::new(cx + dx, cy + dy);

                // 1. 西側外海敞開港灣水體與三座向西突出的長石堤（徹底廢除西側實心死牆）
                let is_center_pier = dy == 0 && (-10..=-4).contains(&dx);
                let is_north_pier = dy == 4 && (-10..=-4).contains(&dx);
                let is_south_pier = dy == -4 && (-10..=-4).contains(&dx);
                let is_quay = dx == -4 && (-7..=7).contains(&dy);
                let is_west_basin = (-10..=-5).contains(&dx)
                    && ((1..=3).contains(&dy) || (5..=7).contains(&dy) || (-3..=-1).contains(&dy) || (-7..=-5).contains(&dy));

                if is_center_pier {
                    let name = if dx == -10 { "西海跨海橋引道" } else { "西港深水引堤道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, name));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}向西延伸入港灣的寬闊條石長堤，連通跨海長橋與泊船水域。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_north_pier {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "北港長石堤"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}北港突出的堅固防浪石堤，外側浪花拍擊，內側泊滿商貨帆船。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_south_pier {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "南港長石堤"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}南港延伸入海的石條長堤，直抵大船塢外泊位。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, "港前臨海石道"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}面朝內澳深水港池的潮潤石階碼頭大道，水手挑夫往來穿梭。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_west_basin {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(city_name, "內澳深水港池"));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}三面受長石堤庇護的天然深水泊船港灣，碧波蕩漾，停靠大噸位海船。", city_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                // 2. 陸側外圍城牆（僅包覆北、東、南陸側，西側完全向港灣開放）
                let is_landward_wall = (dy == 10 && dx >= -7) || (dy == -10 && dx >= -7) || (dx == 10)
                    || (dx <= -8 && (dy == 10 || dy == -10));
                if is_landward_wall {
                    if (dx == 0 && (dy == 10 || dy == -10)) || (dx == 10 && dy == 0) {
                        let gate_dir = if dy == 10 {
                            "北城門"
                        } else if dy == -10 {
                            "南城門"
                        } else {
                            "東城門"
                        };
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(city_name, gate_dir));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}厚重雄偉之{}，守衛森嚴，商流行人穿梭不絕。", city_name, gate_dir);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Wall, format_settlement_cell_name(city_name, "實心城牆"));
                        cell.zone = city_name.to_string();
                        cell.description = format!("{}高大堅固之條石城牆，上覆防禦機關與哨堞，固若金湯。", city_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                    continue;
                }

                // 3. 市區幹道網絡
                let is_cross_east = dy == 0 && dx > -4 && dx < 10;
                let is_main_north_south = dx == 0 && dy.abs() < 10;
                let is_east_avenue = dx == 5 && dy.abs() < 10;
                let is_north_street = dy == 5 && dx > -4 && dx < 10;
                let is_south_street = dy == -5 && dx > -4 && dx < 10;
                let is_dock_inner = dx == -2 && (-7..=7).contains(&dy);
                let is_mid_lane = dx == 2 && dy.abs() < 9;

                if is_cross_east || is_main_north_south || is_east_avenue || is_north_street || is_south_street || is_dock_inner || is_mid_lane {
                    let (terrain, street_name, desc_suffix) = if dx == 0 && dy == 0 {
                        (Terrain::Bridge, "海事中心樞紐橋", "貫穿港城核心的寬闊石橋，橋下海浪輕湧，四方船貨與商旅匯聚於此。")
                    } else if is_dock_inner {
                        (Terrain::Road, "內港泊位街", "緊鄰西側深水泊位的繁華石道，船匠與水手在此歇腳會商。")
                    } else if is_cross_east {
                        (Terrain::Road, "海陸聯運通街", "連接車馬總站的開闊石道，雙輪大車往來絡繹不絕，裝卸行裝井然有序。")
                    } else if is_main_north_south {
                        (Terrain::Road, "港城中心大街", "貫穿港城南北的大道，兩側商行林立，海風清爽。")
                    } else if is_east_avenue {
                        (Terrain::Road, "東聯運大街", "通往東城門的繁忙運輸通道，運載來自內陸的百年雲杉與生活物資。")
                    } else if is_north_street {
                        (Terrain::Road, "北港商街", "築港頭商貿繁忙的青石長街，商行林立，空氣中飄散著桐油與海風氣息。")
                    } else if is_south_street {
                        (Terrain::Road, "南船塢大街", "鄰近造船巨木區的厚重石街，大車運載百年雲杉與深海巨藻纜繩，熱氣騰騰。")
                    } else {
                        (Terrain::Road, "港中市街", "繁華緊湊的港埠街巷，店面與民居櫛比鱗次，商客與水手往來穿梭。")
                    };
                    let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, street_name));
                    cell.zone = city_name.to_string();
                    cell.description = format!("{}，{}", city_name, desc_suffix);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                // 4. 建築與庭院候選格子
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
            }
        }

        // 先填入 15 棟核心海事地標
        for (idx, &(kdx, kdy)) in key_slots.iter().enumerate() {
            let coord = SquareCoord::new(cx + kdx, cy + kdy);
            let (terrain, bldg_name) = bldg_types[idx];
            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
            cell.zone = city_name.to_string();
            cell.description = generate_settlement_building_desc(city_name, bldg_name, terrain);
            cell.explored = true;
            map.insert(coord, cell);
        }

        // 再填入 40 棟港埠與水手住宅
        inner_slots.sort_by_key(|&(dx, dy, _)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy, dx));
        for (idx, (dx, dy, coord)) in inner_slots.into_iter().enumerate() {
            if idx < 40 {
                let (terrain, bldg_name) = bldg_types[15 + idx];
                let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, bldg_name));
                cell.zone = city_name.to_string();
                cell.description = generate_settlement_building_desc(city_name, bldg_name, terrain);
                cell.explored = true;
                map.insert(coord, cell);
            } else {
                let (terrain, place_label, desc_suffix) = match (idx + (dx.abs() * 3 + dy.abs() * 5) as usize) % 12 {
                    0 => (Terrain::Road, "曬網石坪", "風舶角臨海鋪設的寬闊青石曬網坪，水手與漁婦在此攤開巨型深海漁網曝曬修補。"),
                    1 => (Terrain::Bridge, "引潮石橋", "風舶角跨越引海水通潮石渠的堅固平橋，潮水隨海風漲落拍岸。"),
                    2 => (Terrain::HerbGarden, "避風花圃", "風舶角以厚石矮牆圍護的避風花圃，栽種耐鹽海桐與防風香草。"),
                    3 => (Terrain::Road, "錨鏈儲場", "風舶角碼頭後側整齊盤放粗重烏石錨與鐵索的石板儲場。"),
                    4 => (Terrain::Road, "船木晾場", "風舶角大船塢旁架高通風的百年雲杉與樟木船材自然陰乾場。"),
                    5 => (Terrain::Road, "貝灰曬坪", "風舶角鋪滿雪白海貝殼與防水灰漿原料的平整石坪。"),
                    6 => (Terrain::Road, "纜繩工棚", "風舶角絞製深海巨藻韌纜與粗麻船索的長條石柱敞棚。"),
                    7 => (Terrain::Road, "望海石欄", "風舶角面朝外澳海灣的青石護欄平台，可遠眺跨海長橋與風舶嶼帆影。"),
                    8 => (Terrain::Road, "卸貨石坪", "風舶角供雙輪貨車與碼頭挑夫交接海產乾貨箱籠的厚石空坪。"),
                    9 => (Terrain::HerbGarden, "海風茶亭", "風舶角街區轉角供船員歇腳飲茶、聽取潮汐鐘聲的石砌涼亭。"),
                    10 => (Terrain::Bridge, "活水渠橋", "風舶角引流山泉活水供港區洗滌淡化海鹽的清澈石渠小橋。"),
                    _ => (Terrain::Road, "街坊石坪", "風舶角兩側防浪石屋圍合的街坊中庭石坪，海風清爽。"),
                };
                let desc = format!("{}，{}", city_name, desc_suffix);
                let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(city_name, place_label));
                cell.zone = city_name.to_string();
                cell.description = desc;
                cell.explored = true;
                map.insert(coord, cell);
            }
        }
    }
}

/// 建立依地理環境客製化之城鎮格局（10F + 30R = 40 棟建築）
fn build_town_40(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    town_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        40,
        "Town must have exactly 40 buildings (10F + 30R)"
    );

    // 10 棟核心地標專屬座標（確保緊鄰幹道、水岸或階道）
    let key_slots = match town_name {
        "風舶嶼" => [
            (3, 1),   // 0: 外澳領航公會
            (2, 1),   // 1: 外澳海員旅社
            (3, 2),   // 2: 島嶼修船塢
            (3, -1),  // 3: 外澳水產集散行
            (1, 1),   // 4: 外澳港務脈絡診所
            (2, 0),   // 5: 外澳海島信使中繼站
            (4, 1),   // 6: 外澳石砌碼頭
            (1, 0),   // 7: 風舶嶼公所
            (2, -1),  // 8: 外澳海味食堂
            (1, -1),  // 9: 潮音溫水浴堂
        ],
        "大圳頭" => [
            (1, 0),   // 0: 西南原農牧總社
            (-1, 0),  // 1: 大圳頭車馬聯運站
            (-4, 0),  // 2: 西南原農具與百工坊（緊鄰水車西橋）
            (1, 1),   // 3: 大圳頭脈絡診所
            (0, -1),  // 4: 大圳頭農業商行
            (-1, 1),  // 5: 大圳頭商旅社
            (0, -2),  // 6: 大圳頭農產與糧油市集
            (2, 0),   // 7: 田園風味大食堂
            (3, 1),   // 8: 圳水溫和浴堂
            (4, 0),   // 9: 大圳頭・周家莊園（緊鄰東糧倉道）
        ],
        "半嶺鎮" => [
            (1, 0),   // 0: 半嶺試招場
            (-1, 0),  // 1: 半嶺山泉旅社
            (3, 1),   // 2: 寒水玉切石工坊（緊鄰東嶺採石階）
            (0, 1),   // 3: 半嶺脈絡診所
            (0, -1),  // 4: 半嶺山貨特產行
            (1, 1),   // 5: 半嶺鎮公所
            (2, 4),   // 6: 半嶺氣象觀測所（高台階地）
            (-2, 1),  // 7: 青脊石湯溫水浴場
            (-1, -1), // 8: 半嶺山野茶屋食堂
            (-3, 0),  // 9: 半嶺鎮・岑家二房別院
        ],
        _ => [ // 南哨
            (0, 1),   // 0: 南哨防務公議廳
            (1, 1),   // 1: 南哨傳訊信使站
            (-1, 1),  // 2: 重型獵具工坊
            (1, 0),   // 3: 南哨脈絡解毒診所
            (-1, 0),  // 4: 入林嚮導公會
            (0, -2),  // 5: 高棧坪獵人旅社
            (1, -2),  // 6: 黑沼素材收購行
            (-1, -2), // 7: 南哨原野食堂
            (2, -1),  // 8: 高棧坪溫水浴場
            (-2, -1), // 9: 高棧坪・羅家宅院
        ],
    };
    let key_slots_set: HashSet<(i32, i32)> = key_slots.iter().copied().collect();
    let mut inner_slots = Vec::new();

    for dx in -7i32..=7i32 {
        for dy in -7i32..=7i32 {
            let coord = SquareCoord::new(cx + dx, cy + dy);

            // ── 風舶嶼：海島外港與東側石碼頭 ──
            if town_name == "風舶嶼" {
                let is_east_pier = dy == 0 && (4..=7).contains(&dx);
                let is_harbor_water = dx >= 5 && ((1..=3).contains(&dy.abs()));
                let is_quay = dx == 4 && (-4..=4).contains(&dy);
                let is_outer_beach = (dx.abs() + dy.abs() > 10 || dx < -5 || dy.abs() > 5) && !(dx == -7 && dy == 0);
                let is_seawall = (dx.abs() + dy.abs() == 10 || dx == -5 || dy.abs() == 5) && !is_east_pier && !is_harbor_water && !is_quay && !(dx == -7 && dy == 0);

                if is_east_pier {
                    let name = if dx == 7 { "外澳東港門" } else { "外澳長碼頭" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}連通跨海長橋的深水石砌長碼頭，海風浩蕩。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_harbor_water {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(town_name, "外澳港池"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}平靜的海島泊船水面，停靠往來穿梭的渡海快舟。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "避風港岸石階"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}面朝外澳海灣的避風石階碼頭道，起卸水產物資。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_seawall {
                    let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "海防石柵"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}抵擋海浪與狂風的重型海防石木圍柵。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_outer_beach {
                    let mut cell = GridCell::new(coord, Terrain::Beach, format_settlement_cell_name(town_name, "礁岩潮岸"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}外圍經受浪濤沖刷的玄武礁石潮間帶。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                let is_main_street = (dy == 0 && dx >= -7 && dx <= 4) || (dx == 0 && dy.abs() <= 4)
                    || (dy == 2 && dx >= -3 && dx <= 3) || (dy == -2 && dx >= -3 && dx <= 3)
                    || (dx == -3 && dy.abs() <= 3);
                if is_main_street {
                    let street_name = if dx == 0 && dy == 0 { "島心石板街" } else if dy == 0 { "外澳海港直街" } else { "外澳石道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}平整的海島石街，通連各方避風石屋。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 大圳頭：東西向貫穿大圳主渠與三座跨渠石橋 ──
            if town_name == "大圳頭" {
                let is_canal = dy == 2 && dx.abs() <= 7;
                let is_bridge = dy == 2 && (dx == 0 || dx == -4 || dx == 4);
                let is_bank_street = (dy == 1 || dy == 3) && dx.abs() <= 7;
                let is_cross_street = (dx == 0 && dy.abs() <= 7) || (dx == -4 && dy.abs() <= 6) || (dx == 4 && dy.abs() <= 6)
                    || (dy == -3 && dx.abs() <= 6);
                let is_perimeter = dx.abs() == 7 || dy.abs() == 7;

                if is_bridge {
                    let bridge_name = if dx == 0 { "大圳中橋" } else if dx < 0 { "水車西橋" } else { "水車東橋" };
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(town_name, bridge_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}跨越西南大圳主渠的堅固平整條石橋，橋下活水奔流。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_canal {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(town_name, "西南大圳主渠"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}引古河水脈灌溉八百頃沃野的西南大圳主水渠，清波盪漾。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_perimeter {
                    if (dx == 0 && dy.abs() == 7) || (dy == 1 && dx.abs() == 7) {
                        let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "鎮口道"));
                        cell.zone = town_name.to_string();
                        cell.description = format!("{}對外的平坦出入口，設有水利管制石碑與路標。", town_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    } else {
                        let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "木柵圍欄"));
                        cell.zone = town_name.to_string();
                        cell.description = format!("{}外圍緊密排設的堅固木柵，防護野獸侵擾。", town_name);
                        cell.explored = true;
                        map.insert(coord, cell);
                    }
                    continue;
                }
                if is_bank_street || is_cross_street {
                    let street_name = if dy == 1 { "大圳南岸街" } else if dy == 3 { "大圳北岸街" } else if dx == 0 { "市鎮中心大街" } else if dx < 0 { "西磨坊道" } else if dx > 0 { "東糧倉道" } else { "南農商街" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}繁忙通達的農商大街，水車轉動聲不絕於耳。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 半嶺鎮：東側青脊絕壁與採石坑、階梯台地街網 ──
            if town_name == "半嶺鎮" {
                let is_cliff = dx >= 6 && dy.abs() <= 7;
                let is_quarry = dx == 6 && (dy == 2 || dy == -2);
                let is_mountain_barrier = (dx == -7 || ((dy == 7 || dy == -7) && dx <= 5)) && !(dx == -7 && dy == 0) && !(dx == 0 && (dy == 7 || dy == -7));
                let is_gate = (dx == -7 && dy == 0) || (dx == 0 && (dy == 7 || dy == -7));

                if is_quarry {
                    let mut cell = GridCell::new(coord, Terrain::MinePass, format_settlement_cell_name(town_name, "寒水玉採石坑"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}東壁開鑿的優質寒水玉石採掘坑階，礦質細膩晶瑩。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_cliff {
                    let mut cell = GridCell::new(coord, Terrain::Mountain, format_settlement_cell_name(town_name, "青脊絕壁"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}東側連綿聳立的青脊山絕壁，岩層裸露，阻擋東方烈風。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_gate {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, "鎮口道"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}開鑿於山隘階地的出入口，青石階道平整。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_mountain_barrier {
                    let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "山隘石柵"));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}依山勢梯次構建的厚重防獸石柵。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }

                let is_stepped_street = (dx == 0 && dy.abs() < 7) || (dx == -4 && dy.abs() < 7) || (dx == 3 && dy.abs() < 7)
                    || (dy == 0 && dx > -7 && dx <= 5) || (dy == 4 && dx > -7 && dx <= 5) || (dy == -4 && dx > -7 && dx <= 5);
                if is_stepped_street {
                    let street_name = if dx == 0 { "半嶺中心階道" } else if dx < 0 { "西麓石坊道" } else if dx == 3 { "東嶺切石階" } else { "半嶺市階" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                    cell.zone = town_name.to_string();
                    cell.description = format!("{}層疊向上的石板階道，山風清爽，視野開闊。", town_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if !key_slots_set.contains(&(dx, dy)) {
                    inner_slots.push((dx, dy, coord));
                }
                continue;
            }

            // ── 南哨：南向收窄的黑沼防禦堡壘與高棧木廊 ──
            let is_south_swamp = dy < 0 && dx.abs() > 5;
            let is_bastion_wall = (dy >= 0 && (dx.abs() == 7 || dy == 7))
                || (dy < 0 && (dx.abs() == 5 || dy == -7));
            let is_south_gate = dy == -7 && dx == 0;
            let is_north_gate = dy == 7 && dx == 0;
            let is_side_gate = dy == 0 && dx.abs() == 7;

            if is_south_swamp {
                let mut cell = GridCell::new(coord, Terrain::Swamp, format_settlement_cell_name(town_name, "棧下泥澤"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}堡壘南方外圍的黑沼泥濘水窪，蘆葦深密，險隘難行。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_south_gate || is_north_gate || is_side_gate {
                let name = if is_south_gate { "南哨沼口門" } else { "哨口木道" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, name));
                cell.zone = town_name.to_string();
                cell.description = format!("{}連通外圍泥澤與林道的高架木排路口，設有哨位瞭望棚。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_bastion_wall {
                let mut cell = GridCell::new(coord, Terrain::Fence, format_settlement_cell_name(town_name, "防木重柵"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}以黑沼耐水硬木構建的高大防禦重柵，抵禦瘴獸奔突。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            let is_boardwalk_bridge = dy == -3 && dx.abs() <= 4;
            if is_boardwalk_bridge {
                let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(town_name, "高棧坪木橋"));
                cell.zone = town_name.to_string();
                cell.description = format!("{}橫跨南方窪澤的高架厚木橋廊，獵人與斥候行走平穩。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            let is_boardwalk_street = (dx == 0 && dy.abs() < 7) || (dy == 0 && dx.abs() < 7)
                || (dy == 3 && dx.abs() < 7) || (dx.abs() == 3 && dy.abs() < 6);
            if is_boardwalk_street {
                let street_name = if dx == 0 && dy < 0 { "高棧木廊大道" } else if dx == 0 { "北防務石街" } else if dy == 0 { "哨鎮東西大街" } else { "棧道巡道" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(town_name, street_name));
                cell.zone = town_name.to_string();
                cell.description = format!("{}架空乾燥的巡查防務木廊，通達堡壘各個哨所與庫房。", town_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if !key_slots_set.contains(&(dx, dy)) {
                inner_slots.push((dx, dy, coord));
            }
        }
    }

    // 先填入 10 棟核心地標建築
    for (idx, &(kdx, kdy)) in key_slots.iter().enumerate() {
        let coord = SquareCoord::new(cx + kdx, cy + kdy);
        let (terrain, bldg_name) = bldg_types[idx];
        let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(town_name, bldg_name));
        cell.zone = town_name.to_string();
        cell.description = generate_settlement_building_desc(town_name, bldg_name, terrain);
        cell.explored = true;
        map.insert(coord, cell);
    }

    // 再填入 30 棟住宅
    inner_slots.sort_by_key(|&(dx, dy, _)| (dx.abs().max(dy.abs()), dx.abs() + dy.abs(), dy, dx));
    for (idx, (dx, dy, coord)) in inner_slots.into_iter().enumerate() {
        if idx < 30 {
            let (terrain, bldg_name) = bldg_types[10 + idx];
            let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(town_name, bldg_name));
            cell.zone = town_name.to_string();
            cell.description = generate_settlement_building_desc(town_name, bldg_name, terrain);
            cell.explored = true;
            map.insert(coord, cell);
        } else {
            let slot_idx = (idx + (dx.abs() * 3 + dy.abs() * 5) as usize) % 8;
            let (t, name) = match town_name {
                "大圳頭" => [
                    (Terrain::Road, "曬穀石坪"),
                    (Terrain::FarmField, "阡陌水田"),
                    (Terrain::Road, "農具棚架"),
                    (Terrain::Grassland, "草料堆場"),
                    (Terrain::HerbGarden, "育苗菜畦"),
                    (Terrain::Road, "打穀圓場"),
                    (Terrain::Road, "穀倉前坪"),
                    (Terrain::HerbGarden, "分畦藥圃"),
                ][slot_idx],
                "風舶嶼" => [
                    (Terrain::Road, "曬網石坪"),
                    (Terrain::Road, "礁石望台"),
                    (Terrain::Road, "避風石巷"),
                    (Terrain::Road, "海鹽曬場"),
                    (Terrain::Road, "船帆晾架"),
                    (Terrain::Road, "錨石堆場"),
                    (Terrain::HerbGarden, "耐風草圃"),
                    (Terrain::Road, "燈號石坪"),
                ][slot_idx],
                "半嶺鎮" => [
                    (Terrain::Road, "切石石坪"),
                    (Terrain::Road, "山泉清槽"),
                    (Terrain::Road, "柴薪儲場"),
                    (Terrain::HerbGarden, "高山藥畦"),
                    (Terrain::Road, "觀雲石台"),
                    (Terrain::MinePass, "採石坡道"),
                    (Terrain::Road, "試招石坪"),
                    (Terrain::Road, "晾石前坪"),
                ][slot_idx],
                _ => [
                    (Terrain::Road, "高架木坪"),
                    (Terrain::HerbGarden, "曬藥竹場"),
                    (Terrain::Road, "獵具檢修坪"),
                    (Terrain::Road, "篝火石坪"),
                    (Terrain::HerbGarden, "防瘴艾圃"),
                    (Terrain::Road, "棧道木坪"),
                    (Terrain::Road, "巡哨前坪"),
                    (Terrain::Road, "獸皮晾架"),
                ][slot_idx],
            };
            let mut cell = GridCell::new(coord, t, format_settlement_cell_name(town_name, name));
            cell.zone = town_name.to_string();
            cell.description = format!("{}鋪展整潔的{}，市鎮坊民日常勞作休歇於此。", town_name, name);
            cell.explored = true;
            map.insert(coord, cell);
        }
    }
}

/// 建立依地理環境客製化之村落格局（5F + 10R = 15 棟建築）
fn build_village_15(
    map: &mut SettlementCellMap,
    cx: i32,
    cy: i32,
    vil_name: &str,
    bldg_types: &[(Terrain, &'static str)],
) {
    assert_eq!(
        bldg_types.len(),
        15,
        "Village must have exactly 15 buildings (5F + 10R)"
    );

    // 15 棟建築之專屬幾何座標配置（確保 100% 緊鄰可通行道路或木橋）
    let vil_slots = match vil_name {
        "老堰頭" => [
            (1, 0),   // 0: 老堰頭農產公議所 (鄰 0, 0 堰東巡水街)
            (1, 1),   // 1: 老堰頭脈絡草木診所 (鄰 0, 1)
            (-3, 0),  // 2: 水輪石磨工坊 (鄰 -2, 0 陸家舊堰石橋)
            (1, -1),  // 3: 堰渠水畔食堂 (鄰 0, -1)
            (-3, 1),  // 4: 水門茶憩旅社 (鄰 -2, 1 堰西水磨道)
            (-3, -1), // 5: 水畔碾米磨藥工匠居 (鄰 -2, -1)
            (-3, 2),  // 6: 老堰頭水門守閘水工舍 (鄰 -2, 2)
            (1, 2),   // 7: 陸氏宗族老宅 (鄰 0, 2)
            (-3, -2), // 8: 老水車維修木匠居 (鄰 -2, -2)
            (1, -2),  // 9: 古渠引水石匠居 (鄰 0, -2)
            (-3, 3),  // 10: 堰西磨坊長工宿所 (鄰 -2, 3)
            (1, 3),   // 11: 水磨麵粉倉儲家屋 (鄰 0, 3)
            (1, -3),  // 12: 晨霧茶農小舍 (鄰 0, -3)
            (1, 4),   // 13: 青苔石堰巡查居 (鄰 0, 4)
            (-3, -3), // 14: 老堰頭渡水擺渡人舍 (鄰 -2, -3)
        ],
        "北嶺腳" => [
            (1, 0),   // 0: 北嶺腳山林旅社 (鄰 0, 0 嶺腳主徑)
            (1, 1),   // 1: 北嶺腳脈絡急救診所 (鄰 0, 1)
            (1, -1),  // 2: 北嶺山徑補給行 (鄰 0, -1)
            (-1, 0),  // 3: 嶺腳暖火食堂 (鄰 0, 0)
            (-1, -1), // 4: 北嶺腳公事房 (鄰 0, -1)
            (-1, 1),  // 5: 馬家鐵齒登山釘鞋工坊宅 (鄰 0, 1)
            (1, 2),   // 6: 硬木藤背架老匠居 (鄰 0, 2)
            (-1, 2),  // 7: 松脂暖手爐銅匠舍 (鄰 0, 2)
            (1, -2),  // 8: 高嶺採藥嚮導居 (鄰 0, -2)
            (-1, -2), // 9: 防風厚毛氈編織屋 (鄰 0, -2)
            (1, 3),   // 10: 雪嶺山道挑夫客舍 (鄰 0, 3)
            (-1, 3),  // 11: 獵犬飼育訓獸舍 (鄰 0, 3)
            (1, -3),  // 12: 石坎梯田農家小院 (鄰 0, -3)
            (-1, -3), // 13: 寒林乾燥木料小舍 (鄰 0, -3)
            (-1, 4),  // 14: 背風向陽石板居 (鄰 0, 4)
        ],
        "霜林口" => [
            (1, 0),   // 0: 霜林護林瞭望所 (鄰 0, 0 霜林中道)
            (1, 1),   // 1: 霜林脈絡診所 (鄰 0, 1)
            (-1, 0),  // 2: 原木水車鋸木坊 (鄰 0, 0)
            (1, -1),  // 3: 高山木料調配站 (鄰 0, -1)
            (-1, -1), // 4: 林緣溫暖旅社 (鄰 0, -1)
            (1, 2),   // 5: 老鋸木師傅長居宅 (鄰 0, 2)
            (-1, 2),  // 6: 大口徑圓鋸保養工坊宅 (鄰 0, 2)
            (1, 3),   // 7: 重型拖木巨索編造居 (鄰 0, 3)
            (-1, 3),  // 8: 雲杉油防腐浸料工寮 (鄰 0, 3)
            (1, 4),   // 9: 山間運木雪橇修理家 (鄰 0, 4)
            (-1, 4),  // 10: 巡山伐木領班木樓 (鄰 0, 4)
            (1, 5),   // 11: 鋸木學徒集體宿所 (鄰 0, 5)
            (-1, 5),  // 12: 松木防潮木屋一號 (鄰 0, 5)
            (1, -3),  // 13: 松木防潮木屋二號 (鄰 0, -3)
            (-1, -3), // 14: 林口柴炭自給居 (鄰 0, -3)
        ],
        "風櫃尾" => [
            (-3, 0),  // 0: 風櫃尾燈塔 (突出西端海岬高地，鄰 -2, 0 觀海橫街)
            (1, 1),   // 1: 風櫃尾脈絡草藥診所 (鄰 0, 1 避風街)
            (-1, 1),  // 2: 水產冷燻工坊 (鄰 0, 1)
            (1, 2),   // 3: 風櫃尾海風食堂 (鄰 0, 2)
            (-1, 2),  // 4: 風櫃尾巨藻晾曬場 (鄰 0, 2)
            (1, -1),  // 5: 看守燈塔長工宿所 (鄰 0, -1)
            (-1, -1), // 6: 防海霧鹽蝕厚石大宅 (鄰 0, -1)
            (1, -2),  // 7: 礁岩拾貝采螺老嫗居 (鄰 0, -2)
            (-1, -2), // 8: 深海魚油熬煉作坊宅 (鄰 0, -2)
            (1, 3),   // 9: 浪濤回音聽潮小築 (鄰 0, 3)
            (-1, 3),  // 10: 海風吹拂堅石居一號 (鄰 0, 3)
            (1, 4),   // 11: 海風吹拂堅石居二號 (鄰 0, 4)
            (-1, 4),  // 12: 海風吹拂堅石居三號 (鄰 0, 4)
            (1, -3),  // 13: 海風吹拂堅石居四號 (鄰 0, -3)
            (-1, -3), // 14: 風櫃尾避潮土石居 (鄰 0, -3)
        ],
        "烏石鼻" => [
            (-3, 0),  // 0: 烏石鼻漁船泊位 (黑石漁灣畔石階泊位，鄰 -2, 0 漁港碼頭道)
            (1, 1),   // 1: 烏石鼻脈絡草木診所 (鄰 0, 1 烏石街)
            (-1, 1),  // 2: 玄武岩鑿石工坊 (鄰 0, 1)
            (1, 2),   // 3: 烏石水產集散行 (鄰 0, 2)
            (-1, 2),  // 4: 烏石鼻海貝灰窯 (鄰 0, 2)
            (1, -1),  // 5: 烏石鼻採石工領班宅 (鄰 0, -1)
            (-1, -1), // 6: 打石鐵錘鍛造老鐵匠居 (鄰 0, -1)
            (1, -2),  // 7: 黑石漁船泊位值班房 (鄰 0, -2)
            (-1, -2), // 8: 耐鹽海草灰熬煉作坊居 (鄰 0, -2)
            (1, 3),   // 9: 玄武岩條石雕鑿學徒舍 (鄰 0, 3)
            (-1, 3),  // 10: 黑石防浪厚牆居一號 (鄰 0, 3)
            (1, 4),   // 11: 黑石防浪厚牆居二號 (鄰 0, 4)
            (-1, 4),  // 12: 黑石防浪厚牆居三號 (鄰 0, 4)
            (1, -3),  // 13: 黑石防浪厚牆居四號 (鄰 0, -3)
            (-1, -3), // 14: 烏石鼻依山向海居 (鄰 0, -3)
        ],
        "牛埔" => [
            (3, 0),   // 0: 牛埔牧業公議所 (鄰 2, 0 環牧石板道)
            (3, 1),   // 1: 草甸脈絡調理診所 (鄰 2, 1)
            (-3, 1),  // 2: 羊毛與乳酪精製坊 (鄰 -2, 1)
            (3, -1),  // 3: 牛埔牧畜交易市集 (鄰 2, -1)
            (-3, -1), // 4: 牧野歇腳旅社 (鄰 -2, -1)
            (3, 2),   // 5: 老牧牛人家族大宅 (鄰 2, 2)
            (-3, 0),  // 6: 乳酪木桶箍桶匠居 (鄰 -2, 0)
            (-3, 2),  // 7: 原野剪羊毛大快剪老手舍 (鄰 -2, 2)
            (3, -2),  // 8: 草料青貯防潮圓倉家屋 (鄰 2, -2)
            (-3, -2), // 9: 牛犢接生調養獸醫居 (鄰 -2, -2)
            (0, 3),   // 10: 長風草甸放牧巡哨木屋 (鄰 0, 2)
            (1, 3),   // 11: 苜蓿乾草堆疊工棚舍 (鄰 1, 2)
            (-1, 3),  // 12: 草甸牧家夯土石屋一號 (鄰 -1, 2)
            (0, -3),  // 13: 草甸牧家夯土石屋二號 (鄰 0, -2)
            (-1, -3), // 14: 牛埔晨露向陽安居宅 (鄰 -1, -2)
        ],
        "紅土崁" => [
            (-1, 1),  // 0: 紅土崁選礦場 (鄰 -2, 1 西窯道)
            (1, 1),   // 1: 紅土崁脈絡復原診所 (鄰 2, 1 東選礦道)
            (-1, 2),  // 2: 紅土百工鐵器坊 (鄰 -2, 2)
            (1, 2),   // 3: 鐵礦原料調撥處 (鄰 2, 2)
            (-1, -1), // 4: 紅土崁・方家宅院 (鄰 -2, -1)
            (1, -1),  // 5: 紅土窯老窯頭主宅 (鄰 2, -1)
            (-3, 0),  // 6: 優質耐火紅磚成型作坊 (鄰 -2, 0)
            (-1, 3),  // 7: 選礦跳汰木槽工匠居 (鄰 -2, 3)
            (1, 3),   // 8: 粗選赤鐵礦石過篩工舍 (鄰 2, 3)
            (-1, -2), // 9: 燒窯柴炭堆疊防潮倉宅 (鄰 -2, -2)
            (1, -2),  // 10: 紅土高溫磚窯修護工屋 (鄰 2, -2)
            (-1, 4),  // 11: 礦砂運輸雙輪木車修理居 (鄰 -2, 4)
            (1, 4),   // 12: 紅土崁耐火泥工小舍 (鄰 2, 4)
            (-1, -3), // 13: 階地梯級礦工連排石屋一號 (鄰 -2, -3)
            (1, -3),  // 14: 階地梯級礦工連排石屋二號 (鄰 2, -3)
        ],
        "三號坑口" => [
            (-5, 0),  // 0: 三號坑口礦務所 (主坑口，鄰 -4, 0 礦車鐵軌道)
            (1, 1),   // 1: 三號坑口脈絡急救診所 (鄰 0, 1 坑口中街)
            (-1, 1),  // 2: 坑口台車維修棚 (鄰 0, 1)
            (1, 2),   // 3: 礦山生活物資行 (鄰 0, 2)
            (-1, 2),  // 4: 坑口礦工旅社 (鄰 0, 2)
            (1, -1),  // 5: 深坑通風木管與風箱匠居 (鄰 0, -1)
            (-1, -1), // 6: 坑道烏木支架大木工宅 (鄰 0, -1)
            (1, -2),  // 7: 礦坑提昇絞盤機索工舍 (鄰 0, -2)
            (-1, -2), // 8: 採礦鐵鎬淬火鍛造坊舍 (鄰 0, -2)
            (1, 3),   // 9: 礦工油脂探照燈配發房宅 (鄰 0, 3)
            (-1, 3),  // 10: 三號主坑口老礦頭值班居 (鄰 0, 3)
            (1, 4),   // 11: 礦石品位檢驗帳房宅 (鄰 0, 4)
            (-1, 4),  // 12: 坑道湧水排吸唧筒水工舍 (鄰 0, 4)
            (1, -3),  // 13: 厚實礦岩避震石屋一號 (鄰 0, -3)
            (-1, -3), // 14: 厚實礦岩避震石屋二號 (鄰 0, -3)
        ],
        _ => [ // 水尾寮
            (2, 0),   // 0: 水尾寮平底竹筏埠 (水生竹筏水道邊碼頭，鄰 0, 0 木棧道 與 0, -2 棧橋)
            (1, 1),   // 1: 濕地脈絡防瘴診所 (鄰 0, 1 高腳烏木棧橋)
            (-1, 1),  // 2: 蘆葦與竹編工棚 (鄰 0, 1)
            (1, 2),   // 3: 黑沼生鮮水產行 (鄰 0, 2)
            (-1, 2),  // 4: 水尾浮木旅社 (鄰 0, 2)
            (1, -2),  // 5: 阮家高腳木屋與平底竹筏匠老宅 (鄰 0, -2)
            (-1, -2), // 6: 水尾寮沼生烏木打樁水工舍 (鄰 0, -2)
            (1, 3),   // 7: 水尾寮架高避水木屋一號 (鄰 0, 3)
            (-1, 3),  // 8: 水尾寮架高避水木屋二號 (鄰 0, 3)
            (1, -3),  // 9: 黑沼泥炭提煉乾燥小築 (鄰 0, -3)
            (-1, -3), // 10: 水生菱角採摘漁娘居 (鄰 0, -3)
            (1, 4),   // 11: 耐水防腐桐油浸泡工棚居 (鄰 0, 4)
            (-1, 4),  // 12: 深水立網防鱷護寮人舍 (鄰 0, 4)
            (1, 5),   // 13: 水尾寮避水高樁木舍一號 (鄰 0, 5)
            (-1, 5),  // 14: 水尾寮避水高樁木舍二號 (鄰 0, 5)
        ],
    };
    let vil_slots_set: HashSet<(i32, i32)> = vil_slots.iter().copied().collect();

    for dx in -6i32..=6i32 {
        for dy in -6i32..=6i32 {
            let coord = SquareCoord::new(cx + dx, cy + dy);

            // ── 1. 老堰頭：南北貫穿清泉水脈與陸家舊堰石橋 ──
            if vil_name == "老堰頭" {
                let is_bridge = dy == 0 && (-2..=0).contains(&dx);
                let is_river = dx == -1 && dy.abs() <= 6;
                let is_east_bank = dx == 0 && dy.abs() <= 6;
                let is_west_bank = dx == -2 && dy.abs() <= 5;
                let is_cross_lane = dy == 0 && (dx.abs() <= 6);

                if is_bridge {
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "陸家舊堰石橋"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}橫跨清泉水脈的厚實舊堰條石拱橋，溪水奔流拍擊古堰石基。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_river {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "清泉水脈"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}自白塔雪巔奔流而下的清澈水脈，帶動溪畔連綿水車碾磨。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_east_bank || is_west_bank || is_cross_lane {
                    let name = if is_east_bank { "堰東巡水街" } else if is_west_bank { "堰西水磨道" } else { "老堰石渠路" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}沿著水渠鋪展的清爽石板步道，活水淙淙，寧靜祥和。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                // 周邊田地
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::FarmField, "阡陌水田")
                } else if (3..=5).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::FarmField, "阡陌水田")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::HerbGarden, "分畦藥圃")
                } else {
                    (Terrain::DryField, "平整旱田")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 2. 北嶺腳：向陽山谷階梯碎石路與防風林（乾谷地形） ──
            if vil_name == "北嶺腳" {
                let is_mountain_slope = dx >= 4 && dy >= -2;
                let is_main_path = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -6 && dx <= 3);
                let is_branch_path = dx == -2 && dy.abs() <= 3;

                if is_mountain_slope {
                    let t = if dx == 6 || dy == 6 { Terrain::Mountain } else { Terrain::MinePass };
                    let name = if t == Terrain::Mountain { "嶺上陡崖" } else { "坡地碎石" };
                    let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}東北側聳立的青脊山高陡岩壁，阻擋來自雪巔的高山寒風。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_main_path || is_branch_path {
                    let name = if dx == 0 && dy == 0 { "山村石徑" } else if dx == 0 { "嶺腳主徑" } else if dy == 0 { "橫嶺山徑" } else { "杉木西巷" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}依山谷平整鋪設的乾燥碎石山徑，空氣清冽乾燥。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::Forest, "雲杉苗圃")
                } else if (-5..=-3).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::HerbGarden, "山草藥畦")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "儲木石坪")
                } else {
                    (Terrain::Forest, "護林松坡")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 3. 霜林口：東西橫貫融雪急流與鋸木水車木橋 ──
            if vil_name == "霜林口" {
                let is_creek_bridge = dy == -2 && dx == 0;
                let is_snowmelt_creek = dy == -2 && dx.abs() <= 6;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == -1 && dx.abs() <= 6) || (dy == 2 && dx.abs() <= 5);

                if is_creek_bridge {
                    let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "鋸木水車橋"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}跨越融雪急流的厚重硬木橋，橋下冰涼白浪奔騰激盪。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_snowmelt_creek {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "融雪急流"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}奔流而過的清澈高山融雪急流，推動巨型雲杉水車旋轉帶鋸。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "山村石徑" } else if dx == 0 { "霜林中道" } else if dy == -1 { "鋸木溪前街" } else { "杉林北道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}鋪滿清香松針與堅實碎石的山村道路，伐木大車往來其上。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (2..=5).contains(&dx) && (-5..=-3).contains(&dy) {
                    (Terrain::Road, "儲木石坪")
                } else if (-4..=-2).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::Forest, "雲杉林圃")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::HerbGarden, "山草藥畦")
                } else {
                    (Terrain::Forest, "護林松坡")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 4. 風櫃尾：突出西海之海岬崖壁、燈塔與避風街 ──
            if vil_name == "風櫃尾" {
                let is_sea_cliff = dx <= -4 && dy.abs() <= 6;
                let is_promontory = dx == -3 && dy == 0;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -2 && dx <= 6) || (dx == 2 && dy.abs() <= 4);

                if is_sea_cliff {
                    let (t, name) = if dx == -6 { (Terrain::Water, "海浪拍岸") } else if dx == -5 { (Terrain::Beach, "風化礁岩") } else { (Terrain::Mountain, "海角岩壁") };
                    let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西側直面海潮沖刷的陡峭海岬崖岸，風浪喧囂。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_promontory {
                    // 燈塔保留在 vil_slots
                } else if is_streets {
                    let name = if dx == 0 && dy == 0 { "漁村石階道" } else if dx == 0 { "避風街" } else if dy == 0 { "觀海橫街" } else { "漁港東巷" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}傍海鋪設的防潮玄武岩石板路，海風吹拂，空氣中帶著鹽味。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Beach, "礁石淺灘")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Road, "曬網石坪")
                } else {
                    (Terrain::Beach, "巨藻晾場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 5. 烏石鼻：西南黑石漁灣與東側玄武採石場 ──
            if vil_name == "烏石鼻" {
                let is_cove_water = dx <= -4 && (-5..=1).contains(&dy);
                let is_pier_quay = dx == -2 && dy == 0;
                let is_quarry = dx >= 4 && dy.abs() <= 5;
                let is_streets = (dx == 0 && dy.abs() <= 6) || (dy == 0 && dx >= -2 && dx <= 3);

                if is_cove_water {
                    let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "黑石漁灣"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西南避風的黑石平靜漁灣，天然深水，停靠近海採貝漁舟。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_pier_quay {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, "漁港碼頭道"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}連通烏石鼻漁船泊位的堅實石階起卸道。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_quarry {
                    let mut cell = GridCell::new(coord, Terrain::MinePass, format_settlement_cell_name(vil_name, "玄武採石場"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}東坡層次分明之深黑玄武岩採石階地，石質密實堅固。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "漁村石階道" } else if dx == 0 { "烏石街" } else { "漁村橫道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}厚重玄武石板鋪展的漁村大道，石匠與水手在此歇腳。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "曬網石坪")
                } else {
                    (Terrain::Beach, "礁石淺灘")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 6. 牛埔：中央環形牧場石道與開闊草坪 ──
            if vil_name == "牛埔" {
                let is_center_hub = dx == 0 && dy == 0;
                let is_center_meadow = dx.abs() <= 1 && dy.abs() <= 1 && !is_center_hub;
                let is_ring_road = (dx.abs() == 2 && dy.abs() <= 2) || (dy.abs() == 2 && dx.abs() <= 2);
                let is_radial_road = (dx == 0 && dy.abs() >= 3) || (dy == 0 && dx.abs() >= 3);

                if is_center_hub {
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, "牧坪中道"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}核心中央牧坪交會的平整石板中道，牧人騎馬巡行。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_center_meadow {
                    let mut cell = GridCell::new(coord, Terrain::Grassland, format_settlement_cell_name(vil_name, "中央牧坪"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}中心芳草如茵的柔軟草坪，牛群在微風中悠閒啃草。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_ring_road || is_radial_road {
                    let name = if is_ring_road { "環牧石板道" } else { "牧村出入口道" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}環繞牧場的硬實石板路，大車運載草料奶酪平穩順暢。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Road, "羊毛曬場")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Grassland, "向陽牧坪")
                } else {
                    (Terrain::Grassland, "苜蓿草場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 7. 紅土崁：階梯式紅土紅磚窯場與雙向選礦道 ──
            if vil_name == "紅土崁" {
                let is_streets = (dx == -2 && dy.abs() <= 6) || (dx == 2 && dy.abs() <= 6) || (dy == 0 && dx.abs() <= 6) || (dx == 0 && dy.abs() <= 6);
                if is_streets {
                    let name = if dx == 0 && dy == 0 { "礦道石坪" } else if dx == -2 { "西窯道" } else if dx == 2 { "東選礦道" } else { "橫嶺通階" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}堅實平整的工礦碎石路面，路旁整齊擺放著選礦木筐與推車。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (-5..=-3).contains(&dx) && (2..=5).contains(&dy) {
                    (Terrain::DryField, "紅土磚場")
                } else if (3..=5).contains(&dx) && (-5..=-2).contains(&dy) {
                    (Terrain::Road, "選礦石坪")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 2 == 0 {
                    (Terrain::Road, "礦車儲坪")
                } else {
                    (Terrain::EmptyFurrow, "耐火泥池")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 8. 三號坑口：礦谷岩壁、主坑口與礦車鐵軌道 ──
            if vil_name == "三號坑口" {
                let is_cliff = dx <= -5 || (dy >= 4 && dx <= 1);
                let is_streets = (dy == 0 && dx >= -4 && dx <= 6) || (dx == 0 && dy >= -6 && dy <= 3);

                if is_cliff {
                    let mut cell = GridCell::new(coord, Terrain::Mountain, format_settlement_cell_name(vil_name, "礦谷岩壁"));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}西側與北側陡峭聳立的礦山岩壁，露出富含赤鐵與烏金之礦脈。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if is_streets {
                    let name = if dy == 0 { "礦車鐵軌道" } else if dx == 0 && dy == 0 { "礦道石坪" } else { "坑口中街" };
                    let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                    cell.zone = vil_name.to_string();
                    cell.description = format!("{}鋪設鑄鐵軌條與平整枕木的礦車專用運輸道，直通主坑口。", vil_name);
                    cell.explored = true;
                    map.insert(coord, cell);
                    continue;
                }
                if vil_slots_set.contains(&(dx, dy)) {
                    continue;
                }
                let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                    (Terrain::Road, "選礦石坪")
                } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                    (Terrain::Road, "礦車儲坪")
                } else {
                    (Terrain::DryField, "耐火磚場")
                };
                let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }

            // ── 9. 水尾寮：黑沼竹筏水道與高腳烏木棧道 ──
            let is_canal = dy == -1 && dx.abs() <= 6 && dx != 0;
            let is_bridge = dx == 0 && dy.abs() <= 6;
            let is_boardwalk = (dy == 0 && dx.abs() <= 6) || (dy == 3 && dx.abs() <= 4) || (dy == -4 && dx.abs() <= 4);

            if is_canal {
                let mut cell = GridCell::new(coord, Terrain::Water, format_settlement_cell_name(vil_name, "竹筏水道"));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}穿流於高腳木屋下方的平靜水道，水尾村民撐篙划竹筏往來。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_bridge {
                let mut cell = GridCell::new(coord, Terrain::Bridge, format_settlement_cell_name(vil_name, "高腳烏木棧橋"));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}以耐水烏木高樁打入泥澤架設的通風棧橋，離水乾爽安全。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if is_boardwalk {
                let name = if dy == 0 { "水生木棧道" } else if dy > 0 { "蘆花北棧" } else { "澤南橫棧" };
                let mut cell = GridCell::new(coord, Terrain::Road, format_settlement_cell_name(vil_name, name));
                cell.zone = vil_name.to_string();
                cell.description = format!("{}以厚實烏木打樁架高的通風木棧道，兩旁茂密蘆荻隨風搖曳。", vil_name);
                cell.explored = true;
                map.insert(coord, cell);
                continue;
            }
            if vil_slots_set.contains(&(dx, dy)) {
                continue;
            }
            let (t, name) = if (dx.abs() * 3 + dy.abs() * 7) % 3 == 0 {
                (Terrain::Swamp, "蘆葦淺澤")
            } else if (dx.abs() * 3 + dy.abs() * 7) % 3 == 1 {
                (Terrain::FarmField, "菱角淺塘")
            } else {
                (Terrain::HerbGarden, "水草藥畦")
            };
            let mut cell = GridCell::new(coord, t, format_settlement_cell_name(vil_name, name));
            cell.zone = vil_name.to_string();
            cell.description = format!("{}環繞鋪展的{}，隨四季更迭呈現純樸風貌。", vil_name, name);
            cell.explored = true;
            map.insert(coord, cell);
        }
    }

    // 依序填入 15 棟建築至指定專屬座標
    for (idx, &(kdx, kdy)) in vil_slots.iter().enumerate() {
        let coord = SquareCoord::new(cx + kdx, cy + kdy);
        let (terrain, bldg_name) = bldg_types[idx];
        let mut cell = GridCell::new(coord, terrain, format_settlement_cell_name(vil_name, bldg_name));
        cell.zone = vil_name.to_string();
        cell.description = generate_settlement_building_desc(vil_name, bldg_name, terrain);
        cell.explored = true;
        map.insert(coord, cell);
    }
}

// ─────────────────────────────────────────────────────────────────
// 具體聚落建築陣列配置
// ─────────────────────────────────────────────────────────────────
fn generate_city_wen_yao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(55);

    // ── 西北象限：三水口商業與生活核心（共 14 棟）──
    b.push((Terrain::Bank, "三水總商行總部"));
    b.push((Terrain::GuardPost, "傳訊鐘樓與信使總站"));
    b.push((Terrain::Tavern, "三水大食堂"));
    b.push((Terrain::Bathhouse, "泉香溫水浴場"));
    b.push((Terrain::Market, "三水口乾貨海味行"));
    b.push((Terrain::House, "三水口・陸家宅邸"));
    b.push((Terrain::House, "三水口・石板弄堂天井"));
    b.push((Terrain::House, "三水口・陳氏庖廚居"));
    b.push((Terrain::House, "三水口・白氏湯屋居"));
    b.push((Terrain::House, "三水口・老水夫舍"));
    b.push((Terrain::House, "三水口・票號總管居"));
    b.push((Terrain::House, "三水口・信使快騎宿所"));
    b.push((Terrain::House, "三水口・晨汲青石宅"));
    b.push((Terrain::House, "三水口・河畔迎客小院"));

    // ── 東北象限：講堂街文教、出版與醫療街區（共 14 棟）──
    b.push((Terrain::Academy, "學院主校區"));
    b.push((Terrain::Observatory, "學院觀星台與評議廳"));
    b.push((Terrain::Library, "藏書閣與印行館"));
    b.push((Terrain::Academy, "商務會計教室"));
    b.push((Terrain::Library, "講堂街書局與印行館"));
    b.push((Terrain::GeneralStore, "講堂街老字號剪剃鋪"));
    b.push((Terrain::Library, "講堂街書局"));
    b.push((Terrain::Academy, "文曜國小教室"));
    b.push((Terrain::House, "講堂街・陸氏清雅書齋"));
    b.push((Terrain::House, "講堂街・溫家藥圃居"));
    b.push((Terrain::House, "講堂街・秦氏會計事務所"));
    b.push((Terrain::House, "講堂街・陸老師宿舍"));
    b.push((Terrain::House, "講堂街・抄經校勘齋"));
    b.push((Terrain::House, "講堂街・石硯墨香居"));
    // ── 東南象限：分家巷百工製造與旅社街區（共 14 棟）──
    b.push((Terrain::Blacksmith, "岑氏第一鍛造工坊"));
    b.push((Terrain::Inn, "東橋旅社與山風茶屋"));
    b.push((Terrain::Workshop, "巧木居所傢俱坊"));
    b.push((Terrain::GeneralStore, "雲錦織造與服飾坊"));
    b.push((Terrain::Clinic, "溫氏脈絡診所"));
    b.push((Terrain::Workshop, "溫氏百草烘藥房"));
    b.push((Terrain::Workshop, "分家巷習武堂試招院"));
    b.push((Terrain::GeneralStore, "分家巷口雜貨柑仔店"));
    b.push((Terrain::House, "分家巷・岑氏匠師宗宅"));
    b.push((Terrain::House, "分家巷・柯氏木作居"));
    b.push((Terrain::House, "分家巷・蘇氏繡坊居"));
    b.push((Terrain::House, "東橋畔・沈氏臨水居"));
    b.push((Terrain::House, "分家巷・岑氏鍛藝居"));
    b.push((Terrain::House, "分家巷・岑氏武勇舍"));
    // ── 西南象限：西門聯運站、物流與公議堂（共 13 棟）──
    b.push((Terrain::TownHall, "公議堂"));
    b.push((Terrain::Courthouse, "公議堂房地契書局"));
    b.push((Terrain::Caravanserai, "西門車馬聯運總站"));
    b.push((Terrain::Market, "韓氏聯運貨運行"));
    b.push((Terrain::Workshop, "西門皮靴與晴雨傘工坊"));
    b.push((Terrain::House, "西門・韓氏總站宅院"));
    b.push((Terrain::House, "西門・駝獸牧夫宿所"));
    b.push((Terrain::House, "西門・遠行行商棧舍"));
    b.push((Terrain::House, "西門・大車修理工寮"));
    b.push((Terrain::House, "西門・皮靴油布匠居"));
    b.push((Terrain::House, "西門・行道車馬行舍"));
    b.push((Terrain::House, "西門・聯運公秤舍"));
    b.push((Terrain::House, "西門・草料客商居"));

    build_city_55(map, 0, 0, "文曜", &b);
}

fn generate_city_feng_bo_jiao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(55);
    // 13 棟海事與生活建築：海事總商行/航海圖誌館/商盟總會/大船塢/帆具船材/診所/食堂/溫水浴場/旅社/車馬聯運/貨運行/碼頭/水產行
    b.push((Terrain::TownHall, "風舶海事商盟總會"));
    b.push((Terrain::Bank, "海事總商行與船貨契據所"));
    b.push((Terrain::Library, "風舶航海圖誌館"));
    b.push((Terrain::Workshop, "林氏大船塢"));
    b.push((Terrain::Workshop, "馭風帆具與船材工坊"));
    b.push((Terrain::Clinic, "港務脈絡診所"));
    b.push((Terrain::Tavern, "海風海鮮食堂"));
    b.push((Terrain::Bathhouse, "港灣溫水浴場"));
    b.push((Terrain::Inn, "海員旅社"));
    b.push((Terrain::Caravanserai, "港尾車馬聯運站"));
    b.push((Terrain::Market, "海陸聯運貨運行"));
    b.push((Terrain::Dock, "深水泊位與卸貨碼頭"));
    b.push((Terrain::Market, "港區魚獲水產交易行"));
    // 2 棟家族宅院
    b.push((Terrain::House, "築港頭・許家宅邸"));
    b.push((Terrain::House, "大船塢・林家宅院"));
    // 40 棟港埠與水手生活街屋民宅
    let feng_bo_jiao_houses = [
        "許氏海運門庭", "北冕號船員宿所", "領航副舵手舍", "大船塢老木工居", "帆纜編繩匠舍",
        "港尾車馬行屋", "海陸聯運帳房居", "泊位碼頭工舍", "海味水產乾貨家", "潮間帶補網小築",
        "林家船匠支系宅", "風舶航海錄校對齋", "海員避風客舍", "防浪厚石排屋一號", "防浪厚石排屋二號",
        "港灣晨汲水井居", "海風眺望石樓", "深水錨工家屋", "船具修配鐵匠居", "帆布油蠟作坊舍",
        "海陸過磅員民舍", "潮音茶憩小舍", "外海探險老舵手居", "築港石匠工寮居", "港尾雜貨掌櫃居",
        "港口更夫直宿舍", "海員家眷安居樓", "遠洋物資存備宅", "避浪石巷居所", "大船塢學徒宿舍",
        "風舶海巡隊員居", "迎浪青石街屋", "碼頭起卸工小院", "港尾修靴防浸屋", "海風麵點鋪居",
        "港務文書書吏舍", "海波平寧安居宅", "防潮倉管家舍", "巨藻纜繩存收院", "風舶角落霞臨海居",
    ];
    for name in feng_bo_jiao_houses {
        b.push((Terrain::House, name));
    }
    build_city_55(map, -260, 160, "風舶角", &b);
}

fn generate_town_feng_bo_yu(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::GuildHall, "外澳領航公會"));
    b.push((Terrain::Inn, "外澳海員旅社"));
    b.push((Terrain::Workshop, "島嶼修船塢"));
    b.push((Terrain::Market, "外澳水產集散行"));
    b.push((Terrain::Clinic, "外澳港務脈絡診所"));
    b.push((Terrain::GuardPost, "外澳海島信使中繼站"));
    b.push((Terrain::Dock, "外澳石砌碼頭"));
    b.push((Terrain::TownHall, "風舶嶼公所"));
    b.push((Terrain::Tavern, "外澳海味食堂"));
    b.push((Terrain::Bathhouse, "潮音溫水浴堂"));
    // 30R 民宅
    let feng_bo_yu_houses = [
        "顧氏領航世家老宅", "外澳夜光羅盤工匠舍", "外澳老舵工石屋", "海員修整歇憩所", "修船塢刮蠣匠工居",
        "水產集散帳房舍", "港務巡查員居室", "潮音石階臨海宅", "外澳漁火守望屋", "外澳石碼頭排屋",
        "外澳海風曬網院", "防浪粗石砌舍一號", "防浪粗石砌舍二號", "採貝漁夫溫馨宅", "海員補給小院",
        "避風港船伕家屋", "深水礁岩釣客居", "外澳信使備鞍舍", "潮聲茶水歇腳舍", "外澳老更夫值班居",
        "島嶼石匠切石宅", "風波平伏安樂舍", "海灣晨曦石屋", "外澳船帆修補居", "深海巨藻編織院",
        "外澳海產乾味舍", "外澳雜貨小鋪居", "海港渡頭巡視居", "外澳修網匠家", "外澳迎潮望海樓",
    ];
    for name in feng_bo_yu_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, -330, 160, "風舶嶼", &b);
}

fn generate_town_da_zhun_tou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::TownHall, "西南原農牧總社"));
    b.push((Terrain::Caravanserai, "大圳頭車馬聯運站"));
    b.push((Terrain::Workshop, "西南原農具與百工坊"));
    b.push((Terrain::Clinic, "大圳頭脈絡診所"));
    b.push((Terrain::Bank, "大圳頭農業商行"));
    b.push((Terrain::Inn, "大圳頭商旅社"));
    b.push((Terrain::Market, "大圳頭農產與糧油市集"));
    b.push((Terrain::Tavern, "田園風味大食堂"));
    b.push((Terrain::Bathhouse, "圳水溫和浴堂"));
    b.push((Terrain::House, "大圳頭・周家莊園"));
    // 30R 民宅
    let da_zhun_tou_houses = [
        "田氏農具匠人老宅", "西南原導能翻土犁工坊舍", "水力榨油匠居", "大圳頭農具百工舍", "大圳頭糧油調度員宅",
        "農牧總社管事居", "圳道引水水工舍", "草獸牽引大車伕居", "大圳頭棉麻織造家", "原野收穫儲糧宅",
        "大圳頭田園蔬食居", "西南原拓墾老農舍", "大圳頭甘蔗榨汁小院", "圳水磨坊工匠宅", "農產市集掌櫃居",
        "大圳頭聯運駝獸伕舍", "大圳頭行商會客小舍", "田園風光青石屋", "大圳頭修耙補犁鋪居", "西南原草料集運宅",
        "大圳頭稻香安居舍", "大圳頭育苗育種園居", "大圳頭農耕民家一號", "大圳頭農耕民家二號", "大圳頭農耕民家三號",
        "大圳頭農耕民家四號", "大圳頭農耕民家五號", "大圳頭農耕民家六號", "大圳頭清泉飲水舍", "大圳頭西南原望野樓",
    ];
    for name in da_zhun_tou_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, -180, -140, "大圳頭", &b);
}

fn generate_town_ban_ling(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::Arena, "半嶺試招場"));
    b.push((Terrain::Inn, "半嶺山泉旅社"));
    b.push((Terrain::Workshop, "寒水玉切石工坊"));
    b.push((Terrain::Clinic, "半嶺脈絡診所"));
    b.push((Terrain::Market, "半嶺山貨特產行"));
    b.push((Terrain::TownHall, "半嶺鎮公所"));
    b.push((Terrain::Observatory, "半嶺氣象觀測所"));
    b.push((Terrain::Bathhouse, "青脊石湯溫水浴場"));
    b.push((Terrain::Tavern, "半嶺山野茶屋食堂"));
    b.push((Terrain::House, "半嶺鎮・岑家二房別院"));
    // 30R 民宅
    let ban_ling_houses = [
        "石家寒水玉切石匠宅", "袁家半嶺山泉客舍", "高家青銅測風儀觀測居", "半嶺試招武師居", "半嶺山岳急救草藥舍",
        "半嶺山貨收購商舍", "半嶺氣象風雪預警值房", "寒水玉握石拋光齋", "半嶺石湯溫熱居", "雪嶺暖身茶湯鋪居",
        "半嶺登山鐵齒釘鞋作坊舍", "青脊山山徑嚮導居", "半嶺坡地石砌排屋一號", "半嶺坡地石砌排屋二號", "半嶺坡地石砌排屋三號",
        "半嶺松木柴薪庫居", "半嶺高台聽泉樓", "半嶺山泉汲水石台居", "半嶺山澗釣翁居", "半嶺避雪溫暖石舍",
        "半嶺切石水力水車工舍", "半嶺山野果子乾味舍", "半嶺石壁行商歇腳房", "半嶺林緣巡山衛舍", "半嶺古道老鋪民宅",
        "半嶺青脊礦石辨識齋", "半嶺雲海觀景雅舍", "半嶺山風草曬製院", "半嶺石階清掃工居", "半嶺半山迎客別院",
    ];
    for name in ban_ling_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, 160, 210, "半嶺鎮", &b);
}

fn generate_town_nan_shao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(40);
    // 10F 功能與生活建築
    b.push((Terrain::TownHall, "南哨防務公議廳"));
    b.push((Terrain::GuardPost, "南哨傳訊信使站"));
    b.push((Terrain::Workshop, "重型獵具工坊"));
    b.push((Terrain::Clinic, "南哨脈絡解毒診所"));
    b.push((Terrain::GuildHall, "入林嚮導公會"));
    b.push((Terrain::Inn, "高棧坪獵人旅社"));
    b.push((Terrain::Market, "黑沼素材收購行"));
    b.push((Terrain::Tavern, "南哨原野食堂"));
    b.push((Terrain::Bathhouse, "高棧坪溫水浴場"));
    b.push((Terrain::House, "高棧坪・羅家宅院"));
    // 30R 民宅
    let nan_shao_houses = [
        "雷家重型獵具大匠宅", "葉家白茫避瘴清心齋", "羅家黑沼防務精舍", "南哨鐵絲巨藻獵網工棚舍", "南哨飛爪索精造舍",
        "黑沼紅磷信號筒工匠居", "高棧坪首席嚮導歇宿舍", "南哨防瘴草藥焙製院", "南哨重獵具維護工寮", "南哨獸材分級過磅居",
        "高棧坪高架木石屋一號", "高棧坪高架木石屋二號", "高棧坪高架木石屋三號", "高棧坪高架木石屋四號", "高棧坪高架木石屋五號",
        "南哨防泥浸厚皮靴舍", "南哨隘口值守衛士居", "南哨夜間警備更房", "南哨獵犬與角鹿馴養舍", "高棧坪篝火圍坐大院",
        "黑沼探險老獵夫舍", "南哨驅蚊艾草曬場居", "南哨硬木弓弩修造居", "高棧坪山野乾糧小棧", "南哨解毒藥草培植圃居",
        "高棧坪晨霧瞭望石居", "南哨防濕木料棧舍", "南哨涉水長皮靴手藝居", "高棧坪獵友聚義小院", "南哨黑沼門戶安防石宅",
    ];
    for name in nan_shao_houses {
        b.push((Terrain::House, name));
    }
    build_town_40(map, 0, -220, "南哨", &b);
}

fn generate_village_lao_yan_tou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "老堰頭農產公議所"));
    b.push((Terrain::Clinic, "老堰頭脈絡草木診所"));
    b.push((Terrain::Workshop, "水輪石磨工坊"));
    b.push((Terrain::Tavern, "堰渠水畔食堂"));
    b.push((Terrain::Inn, "水門茶憩旅社"));
    // 10R 民宅
    let lao_yan_tou_houses = [
        "秦氏水輪石磨傳承老宅", "老堰頭水門守閘水工舍", "堰渠石匠維修工舍", "陸家舊堰農契典藏舍", "水畔碾米磨藥工匠居",
        "老堰頭清流甘泉水豆腐坊舍", "老堰頭水圳巡渠員居", "堰上微風迎客小院", "老堰頭水車木匠家屋", "老堰頭古堰安瀾石屋",
    ];
    for name in lao_yan_tou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 0, 125, "老堰頭", &b);
}

fn generate_village_bei_ling_jiao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Inn, "北嶺腳山林旅社"));
    b.push((Terrain::Clinic, "北嶺腳脈絡急救診所"));
    b.push((Terrain::GeneralStore, "北嶺山徑補給行"));
    b.push((Terrain::Tavern, "嶺腳暖火食堂"));
    b.push((Terrain::TownHall, "北嶺腳公事房"));
    // 10R 民宅
    let bei_ling_jiao_houses = [
        "馬家鐵齒登山釘鞋工坊宅", "硬木藤背架老匠居", "松脂暖手爐銅匠舍", "北嶺山風草採藥人居", "北嶺腳急救醫助居",
        "北嶺腳山道背夫歇腳院", "北嶺腳登山行囊補配鋪舍", "嶺腳坡地石砌家屋一號", "嶺腳坡地石砌家屋二號", "北嶺腳山泉甘洌安居宅",
    ];
    for name in bei_ling_jiao_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 110, 115, "北嶺腳", &b);
}

fn generate_village_shuang_lin_kou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::GuardPost, "霜林護林瞭望所"));
    b.push((Terrain::Clinic, "霜林脈絡診所"));
    b.push((Terrain::Workshop, "原木水車鋸木坊"));
    b.push((Terrain::Market, "高山木料調配站"));
    b.push((Terrain::Inn, "林緣溫暖旅社"));
    // 10R 民宅
    let shuang_lin_kou_houses = [
        "梁氏水車鋸木大匠宅", "雲杉木滑道維護工舍", "伐一補三護林員居", "高山融雪水力排鋸工居", "重原木裝車大木匠居",
        "霜林口雪松松脂採集舍", "霜林口耐寒雪狼絨皮匠居", "霜林口林緣伐木工排屋一號", "霜林口林緣伐木工排屋二號", "霜林口林木繁育種苗居",
    ];
    for name in shuang_lin_kou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, 190, 290, "霜林口", &b);
}

fn generate_village_feng_gui_wei(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::GuardPost, "風櫃尾燈塔"));
    b.push((Terrain::Clinic, "風櫃尾脈絡草藥診所"));
    b.push((Terrain::Workshop, "水產冷燻工坊"));
    b.push((Terrain::Tavern, "風櫃尾海風食堂"));
    b.push((Terrain::Workshop, "風櫃尾巨藻晾曬場"));
    // 10R 民宅
    let feng_gui_wei_houses = [
        "洪家守塔與冷燻匠師宅", "風櫃尾燈塔長明海燈維護舍", "風櫃海風松屑冷燻魚乾坊居", "海岬風洞聽潮石屋", "風櫃尾採藻編繩漁家",
        "耐浪防風厚玄武石屋一號", "耐浪防風厚玄武石屋二號", "風櫃尾海浪警報守望舍", "風櫃尾海燕鳥羽採集居", "風櫃尾海角避濤安泰宅",
    ];
    for name in feng_gui_wei_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -360, 95, "風櫃尾", &b);
}

fn generate_village_wu_shi_bi(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Dock, "烏石鼻漁船泊位"));
    b.push((Terrain::Clinic, "烏石鼻脈絡草木診所"));
    b.push((Terrain::Blacksmith, "玄武岩鑿石工坊"));
    b.push((Terrain::Market, "烏石水產集散行"));
    b.push((Terrain::Workshop, "烏石鼻海貝灰窯"));
    // 10R 民宅
    let wu_shi_bi_houses = [
        "鄭家烏石重錨鑿石老宅", "烏石鼻海貝灰窯燒漿匠居", "深水黑曜玄武岩採鑿工舍", "烏石鼻十二眼八卦網老漁夫居", "烏石鼻深海石斑釣客舍",
        "烏石鼻玄武岩堆砌漁舍一號", "烏石鼻玄武岩堆砌漁舍二號", "烏石鼻海鹽結晶提煉作坊居", "烏石鼻防滲灰漿調配舍", "烏石鼻黑石灣觀濤石宅",
    ];
    for name in wu_shi_bi_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -260, 35, "烏石鼻", &b);
}

fn generate_village_niu_pu(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "牛埔牧業公議所"));
    b.push((Terrain::Clinic, "草甸脈絡調理診所"));
    b.push((Terrain::Workshop, "羊毛與乳酪精製坊"));
    b.push((Terrain::Market, "牛埔牧畜交易市集"));
    b.push((Terrain::Inn, "牧野歇腳旅社"));
    // 10R 民宅
    let niu_pu_houses = [
        "牧家牛埔硬乳酪發酵老宅", "向陽草甸原牛厚毛氈壓製居", "牛埔重型駝獸高鞍皮匠舍", "牛埔原生甜苜蓿草種子庫居", "牛埔疾行草獸馴育舍",
        "牛埔重角原牛擠乳工舍", "草甸石基暖屋一號", "草甸石基暖屋二號", "牛埔牧道巡草員舍", "牛埔長風草浪安居宅",
    ];
    for name in niu_pu_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -180, -245, "牛埔", &b);
}

fn generate_village_hong_tu_kan(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Workshop, "紅土崁選礦場"));
    b.push((Terrain::Clinic, "紅土崁脈絡復原診所"));
    b.push((Terrain::Blacksmith, "紅土百工鐵器坊"));
    b.push((Terrain::Market, "鐵礦原料調撥處"));
    b.push((Terrain::House, "紅土崁・方家宅院"));
    // 10R 民宅
    let hong_tu_kan_houses = [
        "方氏分支選礦與耐火磚老宅", "紅土崁千度耐火磚瓦窯居", "赤石階地選礦選石匠居", "紅土崁重型鍛錘鋼模修造舍", "紅土崁赤紋鐵礦調撥員居",
        "紅磚紅瓦工人家屋一號", "紅磚紅瓦工人家屋二號", "紅土崁防潮紅泥倉儲居", "紅土崁耐火泥漿攪拌工舍", "紅土階地向陽安居屋",
    ];
    for name in hong_tu_kan_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -85, -140, "紅土崁", &b);
}

fn generate_village_san_hao_keng_kou(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::TownHall, "三號坑口礦務所"));
    b.push((Terrain::Clinic, "三號坑口脈絡急救診所"));
    b.push((Terrain::Workshop, "坑口台車維修棚"));
    b.push((Terrain::Market, "礦山生活物資行"));
    b.push((Terrain::Inn, "坑口礦工旅社"));
    // 10R 民宅
    let san_hao_keng_kou_houses = [
        "方石礦務辦事與技術研討舍", "三號坑口重型捲揚軌道機工居", "採礦防爆防風礦燈檢修舍", "雙刃碎石手鎬淬火工匠舍", "礦道通風支柱老木匠居",
        "依嶺石砌礦工宿舍一號", "依嶺石砌礦工宿舍二號", "三號坑口礦石樣本檢測齋", "三號坑口洗礦沉澱水工居", "三號坑口西嶺堅石安平宅",
    ];
    for name in san_hao_keng_kou_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -285, -140, "三號坑口", &b);
}

fn generate_village_shui_wei_liao(map: &mut SettlementCellMap) {
    let mut b = Vec::with_capacity(15);
    // 5F 功能建築
    b.push((Terrain::Dock, "水尾寮平底竹筏埠"));
    b.push((Terrain::Clinic, "濕地脈絡防瘴診所"));
    b.push((Terrain::Workshop, "蘆葦與竹編工棚"));
    b.push((Terrain::Market, "黑沼生鮮水產行"));
    b.push((Terrain::Inn, "水尾浮木旅社"));
    // 10R 民宅
    let shui_wei_liao_houses = [
        "阮家高腳木屋與平底竹筏匠老宅", "水尾寮沼生烏木打樁水工舍", "水尾寮柔韌蘆葦編簾工匠居", "白茫洲止血水苔採收人舍", "淺澤泥甲鱷獸防護哨舍",
        "水尾寮架高避水木屋一號", "水尾寮架高避水木屋二號", "水尾寮平底竹筏修配棚舍", "黑沼生鮮水產暫存冷浸舍", "水尾寮浮水蘆荻安樂居",
    ];
    for name in shui_wei_liao_houses {
        b.push((Terrain::House, name));
    }
    build_village_15(map, -75, -280, "水尾寮", &b);
}
// ─── 單元測試 ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_grid_dimensions_and_counts() {
        assert_eq!(TOTAL_CELLS, 800_000);
        assert_eq!(GRID_WIDTH, 800);
        assert_eq!(GRID_HEIGHT, 1000);

        let grid = generate_white_tower_grid();

        // 1. 驗證虛擬總格數剛好為 800,000 格
        assert_eq!(grid.total_virtual_cells(), 800_000);

        // 2. 驗證四個角落座標皆能正常查詢且在地圖範圍內
        let corner_sw = SquareCoord::new(-400, -500);
        let corner_ne = SquareCoord::new(399, 499);
        assert!(in_peninsula_bounds(corner_sw));
        assert!(in_peninsula_bounds(corner_ne));
        assert!(grid.get(corner_sw).is_some());
        assert!(grid.get(corner_ne).is_some());
        assert!(grid.contains(corner_sw));
        assert!(grid.contains(corner_ne));

        // 3. 驗證關鍵正典聚落座標
        let wen_yao = grid.get(SquareCoord::new(0, 0)).expect("文曜中心 (0, 0) 必須存在");
        assert_eq!(wen_yao.zone, "文曜");

        let lao_yan_tou = grid.get(SquareCoord::new(0, 125)).expect("老堰頭中心 (0, 125) 必須存在");
        assert_eq!(lao_yan_tou.zone, "老堰頭");

        let feng_bo_jiao = grid.get(SquareCoord::new(-260, 160)).expect("風舶角中心 (-260, 160) 必須存在");
        assert_eq!(feng_bo_jiao.zone, "風舶角");

        // 4. 驗證 15 座聚落建築總數恰為 595 棟 (245 + 55 + 4x40 + 9x15 = 595)
        let settlements = generate_all_settlements();
        let total_buildings = settlements
            .values()
            .filter(|c| c.terrain.category() == "landmark")
            .count();
        assert_eq!(
            total_buildings, 595,
            "15 聚落建築總數必須精準等於 595 棟"
        );

        // 5. 驗證 19 處自然獵場在全圖顯式錨點中皆有覆蓋
        let mut zones_found: HashSet<String> = HashSet::new();
        for cell in grid.cells() {
            zones_found.insert(cell.zone.clone());
        }

        let required_wild_zones = [
            "長風草甸", "古河巨木林", "赤石坡", "落日岬", "驚濤岩岸",
            "巨藻淺灣", "黑石灣", "翠冠巨木林", "清泉溪谷", "北坡雲杉林",
            "西麓林道", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關",
            "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔", "北冕海", "東海",
        ];
        for zone in required_wild_zones {
            assert!(
                zones_found.contains(zone),
                "遺漏自然獵場區塊：{}",
                zone
            );
        }

        // 6. 驗證 15 聚落皆存在
        let required_settlements = [
            "文曜", "風舶角", "風舶嶼", "大圳頭", "半嶺鎮", "南哨",
            "老堰頭", "北嶺腳", "霜林口", "風櫃尾", "烏石鼻", "牛埔", "紅土崁", "三號坑口", "水尾寮",
        ];
        for s in required_settlements {
            assert!(
                zones_found.contains(s),
                "遺漏聚落：{}",
                s
            );
        }
    }

    #[test]
    fn test_wen_yao_canonical_layout_and_no_duplicate_prefix() {
        let settlements = generate_all_settlements();

        // 1. 驗證 (0, 0) 中心樞紐石橋
        let center = settlements
            .get(&SquareCoord::new(0, 0))
            .expect("文曜中心 (0, 0) 必須存在");
        assert_eq!(center.name, "文曜・中心樞紐石橋");
        assert!(center.description.contains("三水川"));

        // 2. 檢驗文曜全域 (-17..=17, -17..=17) 共 1,225 格
        let forbidden = ["debug", "資料庫", "database", "grid", "scene_stub", "還有", "聖塔"];
        let mut wen_yao_landmarks = Vec::new();
        let mut wen_yao_all_names = HashSet::new();

        for dx in -17..=17 {
            for dy in -17..=17 {
                let coord = SquareCoord::new(dx, dy);
                let cell = settlements.get(&coord).unwrap_or_else(|| {
                    panic!("文曜座標 ({}, {}) 必須存在", dx, dy);
                });
                assert_eq!(cell.zone, "文曜");

                // 不得出現重複前綴
                assert!(
                    !cell.name.contains("文曜・文曜") && !cell.name.contains("文曜文曜"),
                    "發現重複聚落前綴：{}",
                    cell.name
                );
                assert!(
                    cell.name.starts_with("文曜・"),
                    "文曜格子名稱必須以 '文曜・' 開頭：{}",
                    cell.name
                );

                // 不得包含禁用詞
                for word in forbidden {
                    assert!(
                        !cell.name.to_lowercase().contains(word),
                        "名稱含有禁詞 '{}': {}",
                        word,
                        cell.name
                    );
                    assert!(
                        !cell.description.to_lowercase().contains(word),
                        "描述含有禁詞 '{}': {}",
                        word,
                        cell.description
                    );
                }

                wen_yao_all_names.insert(cell.name.clone());

                if cell.terrain.category() == "landmark" {
                    wen_yao_landmarks.push((coord, cell.name.clone()));
                }
            }
        }

        // 3. 驗證文曜 landmark 數量恰為 245 且名稱無重複（95 棟功能／商業地標 + 150 棟民居住宅）
        assert_eq!(
            wen_yao_landmarks.len(),
            245,
            "文曜地標建築數量必須恰好為 245 棟"
        );
        let wen_yao_commercial = wen_yao_landmarks
            .iter()
            .filter(|(coord, _)| {
                let cell = settlements.get(coord).unwrap();
                cell.terrain.category() == "landmark" && cell.terrain != Terrain::House
            })
            .count();
        assert_eq!(
            wen_yao_commercial,
            95,
            "文曜商業與功能地標數量必須恰好為 95 棟"
        );
        let wen_yao_residential = wen_yao_landmarks
            .iter()
            .filter(|(coord, _)| {
                let cell = settlements.get(coord).unwrap();
                cell.terrain == Terrain::House
            })
            .count();
        assert_eq!(
            wen_yao_residential,
            150,
            "文曜住宅民居數量必須恰好為 150 棟"
        );
        let mut unique_landmarks = HashSet::new();
        for (_, name) in &wen_yao_landmarks {
            assert!(
                unique_landmarks.insert(name.clone()),
                "文曜地標建築名稱重複：{}",
                name
            );
        }

        // 4. 驗證四大街區核心標誌建築皆存在
        let expected_canonical_buildings = [
            // 西北三水口
            "文曜・三水總商行總部",
            "文曜・傳訊鐘樓與信使總站",
            "文曜・三水大食堂",
            "文曜・泉香溫水浴場",
            "文曜・三水口乾貨海味行",
            "文曜・三水口・陸家宅邸",
            // 東北講堂街
            "文曜・學院主校區",
            "文曜・學院觀星台與評議廳",
            "文曜・藏書閣與印行館",
            "文曜・商務會計教室",
            "文曜・溫氏脈絡診所",
            "文曜・講堂街・陸氏清雅書齋",
            // 東南分家巷
            "文曜・岑氏第一鍛造工坊",
            "文曜・東橋旅社與山風茶屋",
            "文曜・巧木居所傢俱坊",
            "文曜・雲錦織造與服飾坊",
            "文曜・分家巷・岑氏匠師宗宅",
            // 西南西門
            "文曜・公議堂",
            "文曜・公議堂房地契書局",
            "文曜・西門車馬聯運總站",
            "文曜・韓氏聯運貨運行",
            "文曜・西門・韓氏總站宅院",
            // 西北三水外坊
            "文曜・三水外坊・北灣貨船大埠",
            "文曜・三水外坊・早市鮮魚拍賣場",
            "文曜・三水外坊・通濟河運總倉",
            "文曜・三水外坊・老舵手石砌院",
            // 東北講堂外坊
            "文曜・講堂外坊・格物輿圖外院",
            "文曜・講堂外坊・活字排版印書局",
            "文曜・講堂外坊・杏林百草醫館",
            "文曜・講堂外坊・刻書雕版匠居",
            // 東南百工外坊
            "百工外坊・重鐵水錘大鍛坊",
            "百工外坊・魯班大木車船坊",
            "百工外坊・五金農具總商行",
            "百工外坊・司爐老鐵匠院",
            // 西南西門外坊
            "西門外坊・萬里駝隊大驛站",
            "西門外坊・西南原皮毛大市",
            "西門外坊・戰備糧草總儲倉",
            "西門外坊・老護衛四合院",
        ];
        for b in expected_canonical_buildings {
            let full_name = if b.starts_with("文曜・") {
                b.to_string()
            } else {
                format!("文曜・{}", b)
            };
            assert!(
                unique_landmarks.contains(&full_name),
                "文曜缺少正典標誌建築：{}",
                full_name
            );
        }

        // 5. 驗證分段寫實街道命名存在
        let expected_streets = [
            "文曜・三水口北街",
            "文曜・鐘樓前街",
            "文曜・講堂街南口",
            "文曜・講堂街中段",
            "文曜・講堂街北段",
            "文曜・三水東街",
            "文曜・東橋石板拱橋",
            "文曜・東門石板大道",
            "文曜・公議堂東街",
            "文曜・分家巷百工街口",
            "文曜・南門石板大街",
            "文曜・三水口西街",
            "文曜・西門聯運大道",
            "文曜・西門車馬前街",
            "文曜・講堂街學院巷",
            "文曜・分家巷百工石道",
            "文曜・三水口商號街",
            "文曜・西門貨棧街",
            // 內外門坊與外環道
            "文曜・北內門坊",
            "文曜・南內門坊",
            "文曜・東內門坊",
            "文曜・西內門坊",
            "文曜・北城門",
            "文曜・南城門",
            "文曜・東城門",
            "文曜・西城門",
            "文曜・三水外環北道",
            "文曜・講堂外環北道",
            "文曜・百工外環南道",
            "文曜・西門外環南道",
            "文曜・北門外街南段",
            "文曜・南門外街北段",
            "文曜・東門外街西段",
            "文曜・西門外街東段",
            "文曜・三水內環西北隅",
            "文曜・講堂內環東北隅",
            "文曜・分家內環東南隅",
            "文曜・西門內環西南隅",
        ];
        for s in expected_streets {
            assert!(
                wen_yao_all_names.contains(s),
                "文曜缺少分段街道：{}",
                s
            );
        }
    }

    #[test]
    fn test_canonical_hunting_grounds_and_wilderness_road_naming() {
        let grid = generate_white_tower_grid();

        // 1. 驗證 19 處獵場正典中心皆落在對應獵場區（尤其西南原 4 獵場與西北西麓林道）
        let expected_centers = [
            ("長風草甸", (-180, -195)),
            ("古河巨木林", (-80, -60)),
            ("赤石坡", (-85, -180)),
            ("落日岬", (-340, -140)),
            ("驚濤岩岸", (-360, 180)),
            ("巨藻淺灣", (-300, 260)),
            ("黑石灣", (-260, -20)),
            ("西麓林道", (-130, 80)),
            ("清泉溪谷", (60, 160)),
            ("翠冠巨木林", (0, 240)),
            ("北坡雲杉林", (80, 300)),
            ("淺澤泥口", (0, -340)),
            ("白茫洲", (-120, -320)),
            ("貼山深潭", (120, -320)),
            ("青籠關", (0, -450)),
            ("西嶺礦谷", (180, 40)),
            ("東海峭壁", (320, 200)),
            ("雲腰雪林", (180, 320)),
            ("白塔雪巔", (0, 420)),
        ];
        for (expected_zone, (cx, cy)) in expected_centers {
            let (actual_zone, _, _) = classify_wilderness_cell(cx, cy);
            assert_eq!(
                actual_zone, expected_zone,
                "獵場 {} 中心 ({}, {}) 卻被分類為 {}",
                expected_zone, cx, cy, actual_zone
            );
        }

        // 2. 驗證文曜西城門外 (-63, 0) 為西麓林道・道路，而非長風大道或百工大道
        let west_road = grid
            .get(SquareCoord::new(-63, 0))
            .expect("文曜西城門外 (-63, 0) 必須存在");
        assert_eq!(west_road.zone, "西麓林道");
        assert_eq!(west_road.name, "西麓林道・道路");

        // 3. 驗證西南原長風草甸內道路 (-180, -80) 為長風草甸・道路
        let meadow_road = grid
            .get(SquareCoord::new(-180, -80))
            .expect("西南原長風草甸道路 (-180, -80) 必須存在");
        assert_eq!(meadow_road.zone, "長風草甸");
        assert_eq!(meadow_road.name, "長風草甸・道路");

        // 4. 驗證全圖所有顯式格子皆不含非正典「長風大道／百工大道／百工原／西南原大道／幹道」
        let forbidden = ["長風大道", "百工大道", "百工原", "西南原大道", "西北幹道", "西南幹道", "半島主幹道"];
        for cell in grid.cells() {
            for bad in forbidden {
                assert!(
                    !cell.name.contains(bad) && !cell.zone.contains(bad),
                    "發現非正典道路或區域命名 ({}, {}): zone={}, name={}",
                    cell.coord.x,
                    cell.coord.y,
                    cell.zone,
                    cell.name
                );
            }
        }
    }

    #[test]
    fn test_wilderness_road_and_hunting_ground_objects() {
        let grid = generate_white_tower_grid();

        // 1. 文曜四城門外道路格必有指路石碑與野地營火
        let gate_coords = [(-18, 0), (18, 0), (0, 18), (0, -18)];
        for &(gx, gy) in &gate_coords {
            let coord = SquareCoord::new(gx, gy);
            let cell = grid.get(coord).expect("城門外道路格必須存在");
            assert!(
                cell.objects.iter().any(|o| o.name.contains("指路石碑")),
                "門外道路 ({}, {}) 必須有指路石碑", gx, gy
            );
            assert!(
                cell.objects.iter().any(|o| o.name == "野地營火"),
                "門外道路 ({}, {}) 必須有野地營火", gx, gy
            );
        }

        // 2. 19 處正典獵場非道路格至少有 2 件物件（1 件主要資源 + 1 件次要生態節點）
        let hunting_centers = [
            (-181, -195), // 長風草甸
            (-80, -60),   // 古河巨木林
            (-86, -180),  // 赤石坡
            (-340, -140), // 落日岬
            (-360, 180),  // 驚濤岩岸
            (-300, 260),  // 巨藻淺灣
            (-260, -20),  // 黑石灣
            (-130, 80),   // 西麓林道
            (60, 160),    // 清泉溪谷
            (1, 240),     // 翠冠巨木林
            (80, 301),    // 北坡雲杉林
            (1, -340),    // 淺澤泥口
            (-120, -320), // 白茫洲
            (120, -320),  // 貼山深潭
            (1, -450),    // 青籠關
            (180, 40),    // 西嶺礦谷
            (320, 200),   // 東海峭壁
            (180, 320),   // 雲腰雪林
            (1, 420),     // 白塔雪巔
        ];
        for &(hx, hy) in &hunting_centers {
            let coord = SquareCoord::new(hx, hy);
            let cell = grid.get(coord).expect("獵場中心格必須存在");
            assert!(
                cell.objects.len() >= 2,
                "獵場中心 ({}, {}) 物件數必須 >= 2，實際只有 {}",
                hx, hy, cell.objects.len()
            );
            let sec_names = ["獸徑足印", "野地營火", "山澗清泉眼", "避風岩龕", "巡山木樁"];
            assert!(
                cell.objects.iter().any(|o| sec_names.contains(&o.name.as_str())),
                "獵場中心 ({}, {}) 必須包含次要生態節點之一", hx, hy
            );
        }

        // 3. 道路格皆有物件
        let road_cell = grid.get(SquareCoord::new(-63, 0)).expect("道路格必須存在");
        assert!(!road_cell.objects.is_empty(), "道路格必須有路旁物件");
    }

    #[test]
    fn test_continuous_river_and_canal_system() {
        let grid = generate_white_tower_grid();

        // 1. 北源清泉水脈貫通 (y in 18..=385)
        let north_sample = grid.get(SquareCoord::new(2, 100)).expect("北源清泉水脈 (2, 100) 必須存在");
        assert_eq!(north_sample.terrain, Terrain::Water);
        // 2. 東源清泉溪貫通 (18..=155)
        let east_sample = grid.get(SquareCoord::new(18, 2)).expect("東源清泉溪 (18, 2) 必須存在");
        assert_eq!(east_sample.terrain, Terrain::Water);

        // 3. 北嶺腳 (110, 115) 與周邊 (100, 115) 絕非舊版 24,000 格大水塊溺水
        let bei_ling_jiao = grid.get(SquareCoord::new(110, 115)).expect("北嶺腳中心 (110, 115) 必須存在");
        assert_ne!(bei_ling_jiao.terrain, Terrain::Water);
        let dry_valley = classify_wilderness_cell(100, 115);
        assert_ne!(dry_valley.2, Terrain::Water);

        // 4. 清泉溪谷正典中心 (60, 160) 保持水質溪谷
        let qing_quan = classify_wilderness_cell(60, 160);
        assert_eq!(qing_quan.0, "清泉溪谷");
        assert_eq!(qing_quan.1, "溪谷");
        assert_eq!(qing_quan.2, Terrain::Water);

        // 5. 西南大圳 (x in -235..=-2) 貫穿大圳頭 (-180, -138)
        let canal_sample = grid.get(SquareCoord::new(-100, -100)).expect("西南大圳 (-100, -100) 必須存在");
        assert_eq!(canal_sample.terrain, Terrain::Water);
        assert_eq!(canal_sample.name, "古河巨木林・大圳");
    }

    #[test]
    fn test_settlement_building_counts_and_walkability() {
        let settlements = generate_all_settlements();

        // 1. 建築總數恰為 595 棟
        let total_buildings = settlements
            .values()
            .filter(|c| c.terrain.category() == "landmark")
            .count();
        assert_eq!(total_buildings, 595, "所有 15 座聚落的地標建築總數必須為 595 棟");

        // 2. 每棟建築必須四向相鄰至少一格可通行的道路或橋樑（無孤立受困建築）
        let dirs = [(0, 1), (0, -1), (1, 0), (-1, 0)];
        for (coord, cell) in &settlements {
            if cell.terrain.category() == "landmark" {
                let has_adjacent_walkable = dirs.iter().any(|&(dx, dy)| {
                    let neighbor_coord = SquareCoord::new(coord.x + dx, coord.y + dy);
                    if let Some(neighbor) = settlements.get(&neighbor_coord) {
                        neighbor.terrain == Terrain::Road
                            || neighbor.terrain == Terrain::Bridge
                            || neighbor.terrain.walkable()
                    } else {
                        false
                    }
                });
                assert!(
                    has_adjacent_walkable,
                    "聚落建築 {} ({}, {}) 四向無相鄰之可通行道路或木橋！",
                    cell.name, coord.x, coord.y
                );
            }
        }

        // 3. 聚落內部不得存在孤立 1x1 水體受困格
        for (coord, cell) in &settlements {
            if cell.terrain == Terrain::Water && !["文曜", "風舶角", "風舶嶼", "風櫃尾", "烏石鼻"].contains(&cell.zone.as_str()) {
                // 對於非海港市鎮，內部水體（如老堰頭溪水、大圳頭大渠、霜林口溪流、烏石鼻漁灣、水尾寮水道）
                // 必須緊鄰可通行橋樑或水岸步道，不得為單獨封閉死格
                let adjacent_bridge_or_road = dirs.iter().any(|&(dx, dy)| {
                    let n_coord = SquareCoord::new(coord.x + dx, coord.y + dy);
                    if let Some(neighbor) = settlements.get(&n_coord) {
                        neighbor.terrain == Terrain::Bridge || neighbor.terrain == Terrain::Road
                    } else {
                        false
                    }
                });
                assert!(
                    adjacent_bridge_or_road,
                    "聚落 {} 內的水面格 ({}, {}) 無相鄰道路或橋樑！",
                    cell.name, coord.x, coord.y
                );
            }
        }
    }
    #[test]
    fn test_fengbo_strait_and_barrier_island_geometry() {
        let grid = generate_white_tower_grid();

        // 1. 水道中心點 (-300, 160) 與南北出海口 (-300, 195), (-300, 110) 為天然港灣道・水域
        let strait_sample = classify_wilderness_cell(-300, 160);
        assert_eq!(strait_sample.0, "天然港灣道");
        assert_eq!(strait_sample.2, Terrain::Water);

        let north_mouth = classify_wilderness_cell(-300, 195);
        assert_eq!(north_mouth.0, "天然港灣道");
        assert_eq!(north_mouth.2, Terrain::Water);

        let south_mouth = classify_wilderness_cell(-300, 110);
        assert_eq!(south_mouth.0, "天然港灣道");
        assert_eq!(south_mouth.2, Terrain::Water);

        // 2. 驗證顯式生成的 (-300, 195) 在 grid 中存在且為天然港灣道
        let grid_north_mouth = grid.get(SquareCoord::new(-300, 195)).expect("(-300, 195) 必須顯式存在");
        assert_eq!(grid_north_mouth.zone, "天然港灣道");
        assert_eq!(grid_north_mouth.terrain, Terrain::Water);

        // 3. 驗證驚濤岩岸正典中心 (-360, 180) 位於風舶外島上
        let jing_tao = classify_wilderness_cell(-360, 180);
        assert_eq!(jing_tao.0, "驚濤岩岸");

        // 4. 驗證外海 (-380, 160) 為北冕海・深水
        let deep_sea = classify_wilderness_cell(-380, 160);
        assert_eq!(deep_sea.0, "北冕海");
        assert_eq!(deep_sea.2, Terrain::WaterDeep);

        // 5. 驗證青籠關以南地峽直通南方未知大陸，絕非深海外海
        let south_pass = classify_wilderness_cell(0, -490);
        assert_eq!(south_pass.0, "青籠關");
        assert_ne!(south_pass.2, Terrain::WaterDeep);

        let south_west = classify_wilderness_cell(-150, -480);
        assert_eq!(south_west.0, "青籠關");
        assert_ne!(south_west.2, Terrain::WaterDeep);

        let south_east = classify_wilderness_cell(150, -480);
        assert_eq!(south_east.0, "青籠關");
        assert_ne!(south_east.2, Terrain::WaterDeep);

        // 6. 驗證貼山深潭收束於青脊山西麓盆地，且東南青脊山主脊（西嶺礦谷）向南延伸銜接青籠關
        let tie_shan_basin = classify_wilderness_cell(120, -320);
        assert_eq!(tie_shan_basin.0, "貼山深潭");
        let southeast_ridge = classify_wilderness_cell(230, -320);
        assert_eq!(southeast_ridge.0, "西嶺礦谷");

        // 7. 驗證全島荒野分類器回傳之所有 (zone, sub_geo) 皆已登錄於 WILDERNESS_PROTOTYPES，絕無退回通用名稱
        for y in (-500..=499).step_by(3) {
            for x in (-400..=399).step_by(3) {
                let (z, g, _) = classify_wilderness_cell(x, y);
                assert!(
                    WILDERNESS_PROTOTYPES.contains_key(&(z, g)),
                    "座標 ({}, {}) 分類為 ({}, {}) 未登錄於 WILDERNESS_PROTOTYPES！",
                    x,
                    y,
                    z,
                    g
                );
            }
        }
    }
}

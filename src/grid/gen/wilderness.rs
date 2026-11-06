
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
            (Terrain::HolyTower, "白塔雪巔"),
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


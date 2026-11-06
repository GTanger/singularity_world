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


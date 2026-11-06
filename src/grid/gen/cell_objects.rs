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


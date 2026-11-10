fn build_grid_view_cells(
    x: i32,
    y: i32,
    player_id: &str,
    in_private_room: bool,
    current_coord: SquareCoord,
    current_cell: Option<&crate::grid::GridCell>,
    grid_snap: Option<&crate::grid::SquareGrid>,
    npc_counts: &HashMap<(i32, i32), u16>,
) -> Vec<super::super::protocol::GridCellView> {
    let mut cells: Vec<GridCellView> = Vec::new();
    let mut seen_coords: HashSet<(i32, i32)> = HashSet::new();

    // 1. 周圍 radius = 9 格子
    let cells_raw = game::get_grid_cells_around(x, y, 9);
    for (cx, cy, kind, terrain, category, mut name, explored, raw_walkable) in cells_raw {
        seen_coords.insert((cx, cy));
        let mut symbol = grid_snap
            .as_ref()
            .and_then(|g| g.get(SquareCoord::new(cx, cy)))
            .map(|cell| cell.display_symbol().to_string())
            .unwrap_or_else(|| "⋯".to_string());
        if let Some(cell) = grid_snap.as_ref().and_then(|g| g.get(SquareCoord::new(cx, cy))) {
            if name == "刺竹林" {
                if cell.has_room_tag("zl_campfire:active")
                    || cell.objects.iter().any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
                {
                    name = "竹林營火".to_string();
                    symbol = "🔥".to_string();
                } else if crate::grid::zhulin::has_bamboo_trail_mark(cell) {
                    name = "標記竹徑".to_string();
                } else if crate::grid::zhulin::is_bamboo_trail_cleared(cell) {
                    name = "疏竹小徑".to_string();
                }
            }
        }
        // 對於與當前格 (x, y) 緊鄰的四正鄰格，計算 walkable 為從當前格是否可實際走過去；對角角格絕不可走
        let is_neighbor = (cx - x).abs() + (cy - y).abs() == 1;
        let is_diagonal_corner = (cx - x).abs() == 1 && (cy - y).abs() == 1;
        let effective_walkable = if is_neighbor {
            if in_private_room {
                false
            } else if let Some(cell) = &current_cell {
                if !cell.explicit_exits.is_empty() {
                    cell.explicit_exits.iter().any(|e| e.target.x == cx && e.target.y == cy)
                } else if let Some(g) = &grid_snap {
                    can_step_in_nine_grid(g, current_coord, SquareCoord::new(cx, cy))
                } else {
                    raw_walkable
                }
            } else {
                false
            }
        } else if is_diagonal_corner {
            false
        } else {
            raw_walkable
        };

        let cell_npc_count = if in_private_room {
            0
        } else {
            npc_counts.get(&(cx, cy)).copied().unwrap_or(0)
        };
        cells.push(GridCellView {
            x: cx,
            y: cy,
            kind,
            terrain,
            category,
            name,
            explored,
            walkable: effective_walkable,
            symbol,
            npc_count: cell_npc_count,
        });
    }

    // 2. 超出 radius = 6 的已走過／已目及座標
    if let Some(g) = &grid_snap {
        let extra_coords: Vec<(i32, i32)> = {
            let walked_lock = PLAYER_WALKED_COORDS.read();
            if let Some(player_set) = walked_lock.get(player_id) {
                player_set
                    .iter()
                    .copied()
                    .filter(|coord| !seen_coords.contains(coord))
                    .collect()
            } else {
                Vec::new()
            }
        };

        for (wx, wy) in extra_coords {
            if let Some(cell) = g.get(SquareCoord::new(wx, wy)) {
                let cell_npc_count = if in_private_room {
                    0
                } else {
                    npc_counts.get(&(wx, wy)).copied().unwrap_or(0)
                };
                let mut extra_name = cell.name.clone();
                let mut extra_sym = cell.display_symbol().to_string();
                if extra_name == "刺竹林" {
                    if cell.has_room_tag("zl_campfire:active")
                        || cell.objects.iter().any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
                    {
                        extra_name = "竹林營火".to_string();
                        extra_sym = "🔥".to_string();
                    } else if crate::grid::zhulin::has_bamboo_trail_mark(cell) {
                        extra_name = "標記竹徑".to_string();
                    } else if crate::grid::zhulin::is_bamboo_trail_cleared(cell) {
                        extra_name = "疏竹小徑".to_string();
                    }
                }
                cells.push(GridCellView {
                    x: wx,
                    y: wy,
                    kind: cell.terrain.kind_str().to_string(),
                    terrain: crate::grid::terrain_name_zh(cell.terrain).to_string(),
                    category: cell.terrain.category().to_string(),
                    name: extra_name,
                    explored: cell.explored,
                    walkable: cell.terrain.walkable(),
                    symbol: extra_sym,
                    npc_count: cell_npc_count,
                });
            }
        }
    }

    cells
}

struct GridViewExitsResult {
    exits_v5: serde_json::Map<String, serde_json::Value>,
    grid_exits: Vec<super::super::protocol::ExitView>,
    current_place_json: serde_json::Value,
    final_room_name: String,
    is_lodging: bool,
}

fn build_grid_view_exits(
    x: i32,
    y: i32,
    player_id: &str,
    in_private_room: bool,
    current_coord: SquareCoord,
    current_cell: Option<&crate::grid::GridCell>,
    grid_snap: Option<&crate::grid::SquareGrid>,
    base_room_name: &str,
) -> GridViewExitsResult {
    // 先把 exits_v5 與 grid_exits 所需的周邊鄰接資訊組裝好
    let _current_terrain_key = current_cell
        .as_ref()
        .map(|c| c.terrain.kind_str().to_string())
        .unwrap_or_default();
    let current_terrain_name = current_cell
        .as_ref()
        .map(|c| {
            if c.name.trim().is_empty() {
                crate::grid::terrain_name_zh(c.terrain).to_string()
            } else {
                c.name.clone()
            }
        })
        .unwrap_or_default();
    let current_terrain_category = current_cell
        .as_ref()
        .map(|c| c.terrain.category().to_string())
        .unwrap_or_default();
    let _is_building = current_cell
        .as_ref()
        .map(|c| {
            let sym = c.terrain.display_char();
            matches!(sym, "⌂" | "𖠿" | "∆" | "⚑" | "$" | "⚒︎" | "+")
                || c.terrain.category() == "landmark"
        })
        .unwrap_or(false);

    let mut exits_v5 = serde_json::Map::new();
    let mut grid_exits: Vec<super::super::protocol::ExitView> = Vec::new();

    if let Some(ref g) = grid_snap {
        if let Some(ref cell) = current_cell {
            if !cell.explicit_exits.is_empty() {
                let all_dirs = [
                    ("north", crate::grid::Direction::North),
                    ("south", crate::grid::Direction::South),
                    ("east", crate::grid::Direction::East),
                    ("west", crate::grid::Direction::West),
                ];
                for (dir_key, game_dir) in all_dirs {
                    if let Some(ex) = cell
                        .explicit_exits
                        .iter()
                        .find(|e| e.dir == game_dir && e.target != current_coord)
                    {
                        if let Some(tc) = g.get(ex.target) {
                            let mut name = if tc.name.is_empty() {
                                crate::grid::terrain_name_zh(tc.terrain).to_string()
                            } else {
                                tc.name.clone()
                            };
                            if name == "刺竹林" {
                                if tc.has_room_tag("zl_campfire:active")
                                    || tc.objects.iter().any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
                                {
                                    name = "竹林營火".to_string();
                                } else if crate::grid::zhulin::has_bamboo_trail_mark(tc) {
                                    name = "標記竹徑".to_string();
                                } else if crate::grid::zhulin::is_bamboo_trail_cleared(tc) {
                                    name = "疏竹小徑".to_string();
                                }
                            }
                            exits_v5.insert(
                                dir_key.to_string(),
                                serde_json::json!({
                                    "explored": tc.explored,
                                    "walkable": tc.terrain.walkable(),
                                    "name": name,
                                    "direction": game_dir.label_zh(),
                                    "target_coord": [ex.target.x, ex.target.y]
                                }),
                            );
                            if tc.terrain.walkable() {
                                grid_exits.push(super::super::protocol::ExitView {
                                    direction: game_dir.label_zh().to_string(),
                                    to_room_id: crate::grid::grid_room_id_from_coord(ex.target.x, ex.target.y),
                                    to_room_name: name,
                                    route_id: String::new(),
                                    mode: "walk".to_string(),
                                    travel_cost: 0,
                                });
                            }
                        } else {
                            exits_v5.insert(
                                dir_key.to_string(),
                                serde_json::json!({
                                    "explored": false,
                                    "walkable": false,
                                    "name": null
                                }),
                            );
                        }
                    } else {
                        exits_v5.insert(
                            dir_key.to_string(),
                            serde_json::json!({
                                "explored": false,
                                "walkable": false,
                                "name": null
                            }),
                        );
                    }
                }
            } else {
                let dirs: &[(&str, &str, i32, i32)] = &[
                    ("north", "北", 0, 1),
                    ("south", "南", 0, -1),
                    ("east", "東", 1, 0),
                    ("west", "西", -1, 0),
                ];
                for (dir_key, dir_zh, dx, dy) in dirs {
                    let nx = x + dx;
                    let ny = y + dy;
                    let neighbor = g.get(crate::grid::SquareCoord::new(nx, ny));
                    match neighbor {
                        Some(nc) => {
                            let mut name = if nc.name.trim().is_empty() {
                                crate::grid::terrain_name_zh(nc.terrain).to_string()
                            } else {
                                nc.name.clone()
                            };
                            if name == "刺竹林" {
                                if nc.has_room_tag("zl_campfire:active")
                                    || nc.objects.iter().any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0)
                                {
                                    name = "竹林營火".to_string();
                                } else if crate::grid::zhulin::has_bamboo_trail_mark(nc) {
                                    name = "標記竹徑".to_string();
                                } else if crate::grid::zhulin::is_bamboo_trail_cleared(nc) {
                                    name = "疏竹小徑".to_string();
                                }
                            }
                            let can_step = !in_private_room
                                && g.can_walk(current_coord, crate::grid::SquareCoord::new(nx, ny));
                            exits_v5.insert(
                                dir_key.to_string(),
                                serde_json::json!({
                                    "explored": nc.explored,
                                    "walkable": can_step,
                                    "name": name,
                                    "direction": dir_zh,
                                    "target_coord": [nx, ny]
                                }),
                            );
                            if can_step {
                                grid_exits.push(super::super::protocol::ExitView {
                                    direction: dir_zh.to_string(),
                                    to_room_id: crate::grid::grid_room_id_from_coord(nx, ny),
                                    to_room_name: name,
                                    route_id: String::new(),
                                    mode: "walk".to_string(),
                                    travel_cost: 0,
                                });
                            }
                        }
                        None => {
                            exits_v5.insert(
                                dir_key.to_string(),
                                serde_json::json!({
                                    "explored": false,
                                    "walkable": false,
                                    "name": null
                                }),
                            );
                        }
                    }
                }
            }
        }
    }

    let is_lodging = current_terrain_name.contains("旅社")
        || (current_terrain_name.contains("行館") && !current_terrain_name.contains("印行館"))
        || current_terrain_name.contains("歇舍")
        || current_terrain_name.contains("驛館");
    let final_room_name = if in_private_room {
        if is_lodging {
            format!("{}・客房", current_terrain_name)
        } else {
            format!("{}・內室", current_terrain_name)
        }
    } else {
        base_room_name.to_string()
    };

    // 私人客房特殊出入口（進／出）
    if in_private_room {
        let out_name = if is_lodging {
            format!("走出客房回到{}", current_terrain_name)
        } else {
            format!("走出內室回到{}", current_terrain_name)
        };
        exits_v5.clear();
        grid_exits.clear();
        exits_v5.insert(
            "out".to_string(),
            serde_json::json!({
                "explored": true,
                "walkable": true,
                "name": out_name,
                "direction": "出"
            }),
        );
        grid_exits.push(super::super::protocol::ExitView {
            direction: "出".to_string(),
            to_room_id: crate::grid::grid_room_id_from_coord(x, y),
            to_room_name: current_terrain_name.clone(),
            route_id: String::new(),
            mode: "walk".to_string(),
            travel_cost: 0,
        });
    } else {
        let is_residential = is_residential_building(&current_terrain_name);
        let in_exit_name = if is_lodging {
            Some(format!("投宿{}客房", current_terrain_name))
        } else if is_residential && player_holds_property_deed_or_key(player_id) {
            Some(format!("持契推門入{}內室", current_terrain_name))
        } else {
            None
        };

        if let Some(target_in_name) = in_exit_name {
            exits_v5.insert(
                "in".to_string(),
                serde_json::json!({
                    "explored": true,
                    "walkable": true,
                    "name": target_in_name,
                    "direction": "進"
                }),
            );
            grid_exits.push(super::super::protocol::ExitView {
                direction: "進".to_string(),
                to_room_id: crate::grid::grid_room_id_from_coord(x, y),
                to_room_name: target_in_name,
                route_id: String::new(),
                mode: "walk".to_string(),
                travel_cost: 0,
            });
        }
    }
    // 以 SquareGrid 當前格構造真實 current_place_json
    let is_peninsula = grid_snap
        .as_ref()
        .map_or(false, |g| g.has_peninsula_wilderness());
    let locator_str = if is_peninsula {
        format!(
            "白塔半島 · {}",
            current_cell.as_ref().map(|c| c.zone.as_str()).unwrap_or("荒野")
        )
    } else {
        current_cell
            .as_ref()
            .map(|c| c.zone.as_str())
            .filter(|z| !z.trim().is_empty())
            .unwrap_or("紅土西麓")
            .to_string()
    };
    let current_place_json = serde_json::json!({
        "id": crate::grid::grid_room_id_from_coord(x, y),
        "name": if in_private_room { &final_room_name } else { &current_terrain_name },
        "locator": locator_str,
        "full_name": if in_private_room { &final_room_name } else { &current_terrain_name },
        "kind": current_terrain_category,
        "scale": "grid_cell"
    });


    GridViewExitsResult {
        exits_v5,
        grid_exits,
        current_place_json,
        final_room_name,
        is_lodging,
    }
}

fn build_grid_view_objects(
    in_private_room: bool,
    is_lodging: bool,
    base_objects: &[super::super::protocol::ViewObject],
    base_room_name: &str,
    server_unix: i64,
    current_cell: Option<&crate::grid::GridCell>,
) -> Vec<super::super::protocol::ViewObject> {
    let mut indoor_objects = vec![
        {
            let id = "wt_reg_wenyao_home".to_string();
            let name = "床榻".to_string();
            let actions = vec!["Look".to_string(), "Rest".to_string(), "Sleep".to_string(), "Use".to_string()];
            let tag = crate::model::canonical_object_tag(&id, &name);
            let category_tag = crate::model::object_category_tag(&id, &name).to_string();
            let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
            let tags = crate::model::object_semantic_tags(&id, &name, &actions);
            super::super::protocol::ViewObject {
                id,
                name,
                object_type: "bed".to_string(),
                actions,
                tag,
                category_tag,
                attribute_tags,
                tags,
            }
        },
        {
            let id = "wt_inn_bath_Preview".to_string();
            let name = "沐浴桶".to_string();
            let actions = vec!["Look".to_string(), "Wash".to_string(), "Use".to_string()];
            let tag = crate::model::canonical_object_tag(&id, &name);
            let category_tag = crate::model::object_category_tag(&id, &name).to_string();
            let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
            let tags = crate::model::object_semantic_tags(&id, &name, &actions);
            super::super::protocol::ViewObject {
                id,
                name,
                object_type: "bath".to_string(),
                actions,
                tag,
                category_tag,
                attribute_tags,
                tags,
            }
        },
        {
            let id = "wt_home_shell_lamp".to_string();
            let name = "檯燈".to_string();
            let actions = vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()];
            let tag = crate::model::canonical_object_tag(&id, &name);
            let category_tag = crate::model::object_category_tag(&id, &name).to_string();
            let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
            let tags = crate::model::object_semantic_tags(&id, &name, &actions);
            super::super::protocol::ViewObject {
                id,
                name,
                object_type: "lamp".to_string(),
                actions,
                tag,
                category_tag,
                attribute_tags,
                tags,
            }
        },
        {
            let id = "wt_home_jade_cabinet".to_string();
            let name = "冰鑑櫃".to_string();
            let actions = vec![
                "Look".to_string(),
                "Open".to_string(),
                "StoreStash".to_string(),
                "RetrieveStash".to_string(),
                "Use".to_string(),
            ];
            let tag = crate::model::canonical_object_tag(&id, &name);
            let category_tag = crate::model::object_category_tag(&id, &name).to_string();
            let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
            let tags = crate::model::object_semantic_tags(&id, &name, &actions);
            super::super::protocol::ViewObject {
                id,
                name,
                object_type: "cabinet".to_string(),
                actions,
                tag,
                category_tag,
                attribute_tags,
                tags,
            }
        },
    ];
    if !is_lodging {
        let id = "wt_home_bamboo_mailbox".to_string();
        let name = "信箱".to_string();
        let actions = vec![
            "Look".to_string(),
            "Read".to_string(),
            "Open".to_string(),
            "DispatchCourier".to_string(),
        ];
        let tag = crate::model::canonical_object_tag(&id, &name);
        let category_tag = crate::model::object_category_tag(&id, &name).to_string();
        let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
        let tags = crate::model::object_semantic_tags(&id, &name, &actions);
        indoor_objects.push(super::super::protocol::ViewObject {
            id,
            name,
            object_type: "mailbox".to_string(),
            actions,
            tag,
            category_tag,
            attribute_tags,
            tags,
        });
    }
    {
        let id = "wt_home_charcoal_wardrobe".to_string();
        let name = "衣櫥".to_string();
        let actions = vec!["Look".to_string(), "WarmHands".to_string(), "Use".to_string()];
        let tag = crate::model::canonical_object_tag(&id, &name);
        let category_tag = crate::model::object_category_tag(&id, &name).to_string();
        let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
        let tags = crate::model::object_semantic_tags(&id, &name, &actions);
        indoor_objects.push(super::super::protocol::ViewObject {
            id,
            name,
            object_type: "wardrobe".to_string(),
            actions,
            tag,
            category_tag,
            attribute_tags,
            tags,
        });
    }

    let final_objects = if in_private_room {
        indoor_objects
    } else {
        let is_bamboo_slot = current_cell
            .map(crate::grid::zhulin::is_bamboo_slot_cell)
            .unwrap_or(false);
        let mut objs = if is_bamboo_slot {
            Vec::new()
        } else {
            base_objects.to_vec()
        };
        if objs.is_empty() {
            if let Some(cell) = &current_cell {
                let has_campfire = cell
                    .objects
                    .iter()
                    .any(|o| o.id == "zl_bamboo_campfire" && o.quantity > 0);
                for obj in &cell.objects {
                    if is_bamboo_slot && obj.quantity == 0 {
                        continue;
                    }
                    let actions = if is_bamboo_slot && obj.id == "zl_bamboo_campfire" && obj.quantity > 0 {
                        vec![
                            "Look".to_string(),
                            "WarmHands".to_string(),
                            "CampfireRoastKnife".to_string(),
                            "CampfireRoastShoot".to_string(),
                            "CampfireRoastFungus".to_string(),
                            "CampfireBoilSoup".to_string(),
                            "CampfirePopCulm".to_string(),
                        ]
                    } else if is_bamboo_slot && obj.id == "zl_bamboo_culm" && obj.quantity > 0 {
                        vec![
                            "Look".to_string(),
                            "Gather".to_string(),
                            "ClearBambooTrail".to_string(),
                            "CarveMark".to_string(),
                        ]
                    } else if is_bamboo_slot && obj.id == "zl_dry_leaves" && obj.quantity > 0 {
                        if has_campfire {
                            vec!["Look".to_string(), "Gather".to_string()]
                        } else {
                            vec![
                                "Look".to_string(),
                                "Gather".to_string(),
                                "IgniteCampfire".to_string(),
                            ]
                        }
                    } else if obj.quantity > 0 {
                        vec!["Look".to_string(), "Gather".to_string()]
                    } else {
                        vec!["Look".to_string()]
                    };
                    let id = obj.id.clone();
                    let name = obj.name.clone();
                    let tag = obj.tag().to_string();
                    let category_tag = obj.category_tag().to_string();
                    let attribute_tags = obj.attribute_tags().iter().map(|s| (*s).to_string()).collect();
                    let mut tags: Vec<String> = obj.tags().into_iter().map(str::to_string).collect();
                    if is_bamboo_slot && crate::grid::zhulin::has_bamboo_trail_mark(cell) {
                        if !tags.iter().any(|t| t == "zl_marked_trail") {
                            tags.push("zl_marked_trail".to_string());
                        }
                    }
                    objs.push(super::super::protocol::ViewObject {
                        id,
                        name,
                        object_type: "grid_object".to_string(),
                        actions,
                        tag,
                        category_tag,
                        attribute_tags,
                        tags,
                    });
                }
            }
        }
        let tail_room_name = base_room_name.rsplit('・').next().unwrap_or(base_room_name).trim();
        let is_open_street_or_bridge = crate::grid::grid::is_street_or_gate_room_name(base_room_name)
            || tail_room_name.ends_with('街')
            || tail_room_name.ends_with('巷')
            || tail_room_name.ends_with('路')
            || tail_room_name.ends_with('橋')
            || tail_room_name.ends_with('段')
            || tail_room_name.ends_with('隅')
            || tail_room_name.ends_with('坪')
            || ((tail_room_name.contains("街")
                || tail_room_name.contains("路")
                || tail_room_name.contains("道")
                || tail_room_name.contains("巷")
                || tail_room_name.contains("橋")
                || tail_room_name.contains("外環")
                || tail_room_name.contains("內環")
                || tail_room_name.ends_with("口")
                || tail_room_name.ends_with("花圃")
                || tail_room_name.ends_with("廣場"))
                && !tail_room_name.contains("市")
                && !tail_room_name.contains("鋪")
                && !tail_room_name.contains("工坊")
                && !tail_room_name.contains("作坊")
                && !tail_room_name.contains("堂")
                && !tail_room_name.contains("行")
                && !tail_room_name.contains("館")
                && !tail_room_name.contains("廠")
                && !tail_room_name.contains("社")
                && !tail_room_name.contains("屋")
                && !tail_room_name.contains("診所")
                && !tail_room_name.contains("浴場"));
        let is_new_world_cell = !crate::server::grid_manager::get_runtime_square_grid()
            .map_or(false, |g| g.has_peninsula_wilderness());
        if !is_new_world_cell
            && !is_open_street_or_bridge
            && !crate::grid::grid::is_private_residence_room_name(base_room_name)
        {
            if let Some(venue) = crate::grid::resolve_any_room_venue_object(base_room_name, "") {
                if venue.has_counter_restricted_zone
                    && !objs.iter().any(|o| {
                        o.id == "wt_venue_counter"
                            || o.name.contains("櫃檯")
                            || o.name.contains("櫃台")
                    })
                {
                    let id = "wt_venue_counter".to_string();
                    let name = format!("{}・當值櫃檯", venue.venue_kind);
                    let actions = vec![
                        "Look".to_string(),
                        "Inspect".to_string(),
                        "Operate".to_string(),
                    ];
                    let tag = crate::model::canonical_object_tag(&id, &name);
                    let category_tag = crate::model::object_category_tag(&id, &name).to_string();
                    let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
                    let tags = crate::model::object_semantic_tags(&id, &name, &actions);
                    objs.push(super::super::protocol::ViewObject {
                        id,
                        name,
                        object_type: "counter".to_string(),
                        actions,
                        tag,
                        category_tag,
                        attribute_tags,
                        tags,
                    });
                }
                if let Some((tool_id, tool_name, _)) = venue.lendable_tool
                    && !objs.iter().any(|o| o.id == tool_id || o.name == tool_name)
                {
                    let id = tool_id.to_string();
                    let name = tool_name.to_string();
                    let actions = vec![
                        "Look".to_string(),
                        "Inspect".to_string(),
                        "Use".to_string(),
                    ];
                    let tag = crate::model::canonical_object_tag(&id, &name);
                    let category_tag = crate::model::object_category_tag(&id, &name).to_string();
                    let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
                    let tags = crate::model::object_semantic_tags(&id, &name, &actions);
                    objs.push(super::super::protocol::ViewObject {
                        id,
                        name,
                        object_type: "tool".to_string(),
                        actions,
                        tag,
                        category_tag,
                        attribute_tags,
                        tags,
                    });
                }
            }
            if let Some((ev_pack, _)) = crate::game::peninsula_scripts::active_event_and_script_pack_for_room(
                base_room_name,
                None,
                server_unix,
            ) {
                let all_keys: Vec<&str> = ev_pack
                    .key_objects
                    .split('、')
                    .map(|seg| {
                        let cleaned = seg
                            .split('（')
                            .next()
                            .unwrap_or(seg)
                            .split('(')
                            .next()
                            .unwrap_or(seg)
                            .trim();
                        if let Some(start) = cleaned.find('《')
                            && let Some(end) = cleaned.find('》')
                            && start < end
                        {
                            cleaned[start + '《'.len_utf8()..end].trim()
                        } else {
                            cleaned.trim_matches(['《', '》'])
                        }
                    })
                    .filter(|k| !k.is_empty())
                    .collect();
                let already_has_any_key = all_keys.iter().any(|k| {
                    objs.iter().any(|o| {
                        o.name == *k || o.name.contains(*k) || k.contains(o.name.as_str())
                    })
                });
                if !already_has_any_key
                    && objs.len() < 4
                    && let Some(&first_key) = all_keys.first()
                {
                    let first_archetype = crate::model::object_archetype_tag("", first_key);
                    let cell_ref = current_cell.as_ref();
                    let mismatch = match first_archetype {
                        "grindstone" => cell_ref.map_or(!base_room_name.contains("石磨"), |c| !c.has_room_tag("grindstone")),
                        "stone_cutter" => cell_ref.map_or(!base_room_name.contains("切石"), |c| !c.has_room_tag("masonry")),
                        "sawmill" => cell_ref.map_or(!base_room_name.contains("鋸"), |c| !c.has_room_tag("lumber")),
                        "mine_rail" => cell_ref.map_or(!base_room_name.contains("礦"), |c| !c.has_room_tag("mine")),
                        _ => false,
                    };
                    if !mismatch
                        && !objs.iter().any(|o| {
                            o.name == first_key
                                || o.name.contains(first_key)
                                || first_key.contains(o.name.as_str())
                        })
                    {
                        let id = format!("wt_event_obj_{}", ev_pack.id.to_lowercase());
                        let name = first_key.to_string();
                        let actions = vec![
                            "Look".to_string(),
                            "Inspect".to_string(),
                            "Operate".to_string(),
                            "Use".to_string(),
                        ];
                        let tag = crate::model::canonical_object_tag(&id, &name);
                        let category_tag = crate::model::object_category_tag(&id, &name).to_string();
                        let attribute_tags = crate::model::object_attribute_tags(&id, &name).into_iter().map(str::to_string).collect();
                        let tags = crate::model::object_semantic_tags(&id, &name, &actions);
                        objs.push(super::super::protocol::ViewObject {
                            id,
                            name,
                            object_type: "event_object".to_string(),
                            actions,
                            tag,
                            category_tag,
                            attribute_tags,
                            tags,
                        });
                    }
                }
            }
        }
        objs
    };

    final_objects
}

fn build_grid_view_cells(
    x: i32,
    y: i32,
    current_z: i16,
    player_id: &str,
    pool: Option<&crate::store::sql::DbPool>,
    field: crate::grid::substrate::slots::FieldState,
    current_coord: SquareCoord,
    current_cell: Option<&crate::grid::GridCell>,
    grid_snap: Option<&crate::grid::SquareGrid>,
    npc_counts: &HashMap<(i32, i32), u16>,
) -> Vec<super::super::protocol::GridCellView> {
    let mut cells: Vec<GridCellView> = Vec::new();
    let mut seen_coords: HashSet<(i32, i32)> = HashSet::new();

    if crate::grid::substrate::runtime::global().is_some() {
        let clock = crate::grid::substrate::hydrology::WorldClockSnapshot::current_live();
        let here_snap =
            crate::grid::substrate::facade::CellFacade::snapshot_live_clock(pool, x, y, clock);
        // 1. 周圍 radius = 9 格：100% 由 CellFacade（含高水期 Field + 惰性 settle）動態合成
        for dx in -9..=9 {
            for dy in -9..=9 {
                let cx = x + dx;
                let cy = y + dy;
                seen_coords.insert((cx, cy));
                let snap = if dx == 0 && dy == 0 {
                    here_snap.clone()
                } else {
                    crate::grid::substrate::facade::CellFacade::snapshot_live_clock(
                        pool, cx, cy, clock,
                    )
                };
                let cell_field = crate::grid::substrate::hydrology::derive_cell_field(
                    cx, cy, snap.l0, clock,
                );
                let (kind, category) = crate::grid::substrate::render_synth::kind_and_category(
                    snap.l0,
                    snap.slope,
                    &snap.slots,
                    cell_field,
                );
                let is_neighbor = dx.abs() + dy.abs() == 1;
                let is_diagonal_corner = dx.abs() == 1 && dy.abs() == 1;
                let effective_walkable = if is_neighbor {
                    current_z == 0
                        && crate::grid::substrate::arcade_street::can_step_horizontal(
                            &here_snap, &snap,
                        )
                } else if is_diagonal_corner {
                    false
                } else {
                    snap.synth.walkable_on_landmass_01
                };
                let explored = (dx.abs().max(dy.abs()) <= 2)
                    || crate::grid::substrate::fog::is_player_seen(pool, player_id, cx, cy);
                // 公開層（z == 0）人數：即使觀察者在內室／樓上亦不歸零
                let cell_npc_count = npc_counts.get(&(cx, cy)).copied().unwrap_or(0);
                cells.push(GridCellView {
                    x: cx,
                    y: cy,
                    kind: kind.to_string(),
                    terrain: snap.synth.micro_name.to_string(),
                    category: category.to_string(),
                    name: snap.synth.micro_name.to_string(),
                    explored,
                    walkable: effective_walkable,
                    symbol: snap.synth.symbol.to_string(),
                    npc_count: cell_npc_count,
                    edge_cue: snap.edge_cue.as_ref().map(|c| c.compact_badge()),
                });
            }
        }

        // 2. 超出 radius = 9 之個人迷霧已踏足／目及座標（壓縮點陣圖，不寫入全服 cell_overlay）
        for (wx, wy, _is_visited) in
            crate::grid::substrate::fog::player_known_coords(pool, player_id, 512)
        {
            if !seen_coords.insert((wx, wy)) {
                continue;
            }
            let snap =
                crate::grid::substrate::facade::CellFacade::snapshot_cached(pool, wx, wy, field);
            let (kind, category) = crate::grid::substrate::render_synth::kind_and_category(
                snap.l0,
                snap.slope,
                &snap.slots,
                field,
            );
            let cell_npc_count = npc_counts.get(&(wx, wy)).copied().unwrap_or(0);
            cells.push(GridCellView {
                x: wx,
                y: wy,
                kind: kind.to_string(),
                terrain: snap.synth.micro_name.to_string(),
                category: category.to_string(),
                name: snap.synth.micro_name.to_string(),
                explored: true,
                walkable: snap.synth.walkable_on_landmass_01,
                symbol: snap.synth.symbol.to_string(),
                npc_count: cell_npc_count,
                edge_cue: snap.edge_cue.as_ref().map(|c| c.compact_badge()),
            });
        }
        return cells;
    }

    // 底板未載入時（例如純單元測試環境）優雅回退舊 SquareGrid
    let cells_raw = game::get_grid_cells_around(x, y, 9);
    for (cx, cy, kind, terrain, category, name, explored, raw_walkable) in cells_raw {
        seen_coords.insert((cx, cy));
        let symbol = grid_snap
            .as_ref()
            .and_then(|g| g.get(SquareCoord::new(cx, cy)))
            .map(|cell| cell.display_symbol().to_string())
            .unwrap_or_else(|| "⋯".to_string());
        let is_neighbor = (cx - x).abs() + (cy - y).abs() == 1;
        let is_diagonal_corner = (cx - x).abs() == 1 && (cy - y).abs() == 1;
        let effective_walkable = if is_neighbor {
            if current_z > 0 {
                false
            } else if let Some(cell) = &current_cell {
                if !cell.explicit_exits.is_empty() {
                    cell.explicit_exits
                        .iter()
                        .any(|e| e.target.x == cx && e.target.y == cy)
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
        let cell_npc_count = npc_counts.get(&(cx, cy)).copied().unwrap_or(0);
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
            edge_cue: crate::grid::substrate::runtime::derive_edge_cue(cx, cy),
        });
    }

    cells
}

struct GridViewExitsResult {
    exits_v5: serde_json::Map<String, serde_json::Value>,
    grid_exits: Vec<super::super::protocol::ExitView>,
    current_place_json: serde_json::Value,
    final_room_name: String,
    floor_view: super::super::protocol::FloorView,
}

fn build_grid_view_exits(
    x: i32,
    y: i32,
    current_z: i16,
    player_id: &str,
    pool: Option<&crate::store::sql::DbPool>,
    field: crate::grid::substrate::slots::FieldState,
    current_coord: SquareCoord,
    current_cell: Option<&crate::grid::GridCell>,
    grid_snap: Option<&crate::grid::SquareGrid>,
    base_room_name: &str,
) -> GridViewExitsResult {
    let has_substrate = crate::grid::substrate::runtime::global().is_some();
    let live_clock = crate::grid::substrate::hydrology::WorldClockSnapshot::current_live();
    let here_snap = has_substrate.then(|| {
        crate::grid::substrate::facade::CellFacade::snapshot_live_clock(pool, x, y, live_clock)
    });

    let current_terrain_name = here_snap
        .as_ref()
        .map(|s| s.synth.micro_name.to_string())
        .or_else(|| {
            current_cell.as_ref().map(|c| {
                if c.name.trim().is_empty() {
                    crate::grid::terrain_name_zh(c.terrain).to_string()
                } else {
                    c.name.clone()
                }
            })
        })
        .unwrap_or_else(|| base_room_name.to_string());
    let current_terrain_category = here_snap
        .as_ref()
        .map(|s| {
            crate::grid::substrate::render_synth::kind_and_category(
                s.l0,
                s.slope,
                &s.slots,
                field,
            )
            .1
            .to_string()
        })
        .or_else(|| current_cell.as_ref().map(|c| c.terrain.category().to_string()))
        .unwrap_or_else(|| "terrain".to_string());

    let mut exits_v5 = serde_json::Map::new();
    let mut grid_exits: Vec<super::super::protocol::ExitView> = Vec::new();

    if current_z == 0 {
        if let Some(ref here) = here_snap {
            let dirs: &[(&str, &str, i32, i32)] = &[
                ("north", "北", 0, 1),
                ("south", "南", 0, -1),
                ("east", "東", 1, 0),
                ("west", "西", -1, 0),
            ];
            for &(dir_key, dir_zh, dx, dy) in dirs {
                let nx = x + dx;
                let ny = y + dy;
                let neighbor = crate::grid::substrate::facade::CellFacade::snapshot_live_clock(
                    pool, nx, ny, live_clock,
                );
                let can_step =
                    crate::grid::substrate::arcade_street::can_step_horizontal(here, &neighbor);
                let explored =
                    crate::grid::substrate::fog::is_player_seen(pool, player_id, nx, ny);
                let name = neighbor.synth.micro_name.to_string();
                exits_v5.insert(
                    dir_key.to_string(),
                    serde_json::json!({
                        "explored": explored,
                        "walkable": can_step,
                        "name": name,
                        "direction": dir_zh,
                        "target_coord": [nx, ny]
                    }),
                );
                if can_step {
                    grid_exits.push(super::super::protocol::ExitView {
                        direction: dir_zh.to_string(),
                        to_room_id: crate::grid::grid_room_id_from_coord_z(nx, ny, 0),
                        to_room_name: name,
                        route_id: String::new(),
                        mode: "walk".to_string(),
                        travel_cost: 0,
                    });
                }
            }
        } else if let Some(ref g) = grid_snap
            && let Some(ref cell) = current_cell
        {
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
                        && let Some(tc) = g.get(ex.target)
                    {
                        let name = if tc.name.is_empty() {
                            crate::grid::terrain_name_zh(tc.terrain).to_string()
                        } else {
                            tc.name.clone()
                        };
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
                                to_room_id: crate::grid::grid_room_id_from_coord_z(
                                    ex.target.x,
                                    ex.target.y,
                                    0,
                                ),
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
                    if let Some(nc) = g.get(crate::grid::SquareCoord::new(nx, ny)) {
                        let name = if nc.name.trim().is_empty() {
                            crate::grid::terrain_name_zh(nc.terrain).to_string()
                        } else {
                            nc.name.clone()
                        };
                        let can_step = g.can_walk(current_coord, SquareCoord::new(nx, ny));
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
                                to_room_id: crate::grid::grid_room_id_from_coord_z(nx, ny, 0),
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
                }
            }
        }
    }

    // 立體樓層垂直出口（進 / 出 / 上 / 下）與 FloorView
    let floor_exits_str: Vec<&'static str> = here_snap
        .as_ref()
        .map(|s| crate::grid::substrate::arcade_street::available_floor_exits(s, current_z))
        .unwrap_or_else(|| if current_z > 0 { vec!["出"] } else { Vec::new() });
    let max_z = here_snap.as_ref().map_or(0, |s| s.max_z);
    let floor_label = crate::grid::substrate::arcade_street::floor_label_zh(current_z).to_string();

    for &fdir in &floor_exits_str {
        let (v5_key, target_z, label) = match (current_z, fdir) {
            (0, "進") => ("in", 1, format!("步入{current_terrain_name}內室")),
            (1, "出") => ("out", 0, format!("步出回到{current_terrain_name}")),
            (0, "上") => ("up", 2, format!("上樓至{current_terrain_name}二樓住家")),
            (2, "上") => ("up", 3, format!("上樓至{current_terrain_name}三樓高閣")),
            (2, "下") => ("down", 0, format!("下樓回到{current_terrain_name}")),
            (3, "下") => ("down", 2, format!("下樓至{current_terrain_name}二樓住家")),
            _ => ("out", 0, format!("回到{current_terrain_name}")),
        };
        exits_v5.insert(
            v5_key.to_string(),
            serde_json::json!({
                "explored": true,
                "walkable": true,
                "name": label,
                "direction": fdir
            }),
        );
        grid_exits.push(super::super::protocol::ExitView {
            direction: fdir.to_string(),
            to_room_id: crate::grid::grid_room_id_from_coord_z(x, y, target_z),
            to_room_name: label,
            route_id: String::new(),
            mode: "walk".to_string(),
            travel_cost: 0,
        });
    }

    let final_room_name = if current_z > 0 {
        format!("{current_terrain_name}・{floor_label}")
    } else {
        current_terrain_name.clone()
    };

    let locator_str = here_snap
        .as_ref()
        .map(|s| s.synth.zone_name.to_string())
        .or_else(|| {
            current_cell
                .as_ref()
                .map(|c| c.zone.as_str())
                .filter(|z| !z.trim().is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "西麓湧泉竹帶".to_string());

    let current_place_json = serde_json::json!({
        "id": crate::grid::grid_room_id_from_coord_z(x, y, current_z),
        "name": &final_room_name,
        "locator": locator_str,
        "full_name": &final_room_name,
        "kind": current_terrain_category,
        "scale": "grid_cell",
        "z": current_z
    });

    let floor_view = super::super::protocol::FloorView {
        z: current_z,
        max_z,
        label: floor_label,
        exits: floor_exits_str.into_iter().map(str::to_string).collect(),
    };

    GridViewExitsResult {
        exits_v5,
        grid_exits,
        current_place_json,
        final_room_name,
        floor_view,
    }
}

fn build_grid_view_objects(
    current_z: i16,
    base_objects: &[super::super::protocol::ViewObject],
    base_room_name: &str,
    server_unix: i64,
    current_cell: Option<&crate::grid::GridCell>,
) -> Vec<super::super::protocol::ViewObject> {
    if current_z > 0 {
        return crate::grid::substrate::arcade_street::SUBSPACE_FLOOR_FIXTURES
            .iter()
            .map(|spec| {
                let id = spec.id.to_string();
                let name = spec.name.to_string();
                let actions: Vec<String> = spec.actions.iter().map(|s| (*s).to_string()).collect();
                let tag = crate::model::canonical_object_tag(&id, &name);
                let category_tag = crate::model::object_category_tag(&id, &name).to_string();
                let attribute_tags = crate::model::object_attribute_tags(&id, &name)
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                let tags = crate::model::object_semantic_tags(&id, &name, &actions);
                super::super::protocol::ViewObject {
                    id,
                    name,
                    object_type: spec.object_type.to_string(),
                    actions,
                    tag,
                    category_tag,
                    attribute_tags,
                    tags,
                }
            })
            .collect();
    }
    let final_objects = {
        let is_substrate_authored = crate::grid::substrate::runtime::global().is_some()
            && base_objects.iter().any(|o| {
                o.id == "subspace_arcade_eave"
                    || o.id == "subspace_street_lamp"
                    || o.id.starts_with("zl_")
            });
        if is_substrate_authored {
            return base_objects.to_vec();
        }
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
                            "CampfireDrinkWater".to_string(),
                            "CampfireEatFood".to_string(),
                            "FillBambooCanteen".to_string(),
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
                            "FillBambooCanteen".to_string(),
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

fn step_resident_idles(grid: &crate::grid::SquareGrid) {
    let hour = crate::store::current_game_hour();
    if !(6..=21).contains(&hour) {
        RESIDENT_IDLE_TICKS.write().clear();
        return;
    }

    let Some(store_arc) = crate::store::get_store() else {
        return;
    };

    let active_targets = {
        let r = RESIDENT_MOVE_TARGETS.read();
        r.keys().cloned().collect::<std::collections::HashSet<String>>()
    };

    for def in iter_resident_schedules() {
        let id = def.id;
        if def.display_title.contains("水輪") || def.display_title.contains("石磨") {
            continue;
        }
        if active_targets.contains(id) {
            RESIDENT_IDLE_TICKS.write().remove(id);
            continue;
        }

        // 攤販業者由生活作息表嚴格控管擺攤、備料、採買與返家時段，跳過隨機次級建築漫遊
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
            let mut w = RESIDENT_IDLE_TICKS.write();
            let entry = w.entry(id.to_string()).or_insert(0);
            *entry += 1;
            *entry
        };
        let active_goal = get_resident_goal(id);
        let is_on_expedition = matches!(
            active_goal.as_deref(),
            Some("wilderness_expedition") | Some("caravan_route")
        );

        if is_on_expedition {
            if ticks < 3 {
                continue;
            }
            RESIDENT_IDLE_TICKS.write().insert(id.to_string(), 0);
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
            } else if let Some(exp_def) = find_resident_expedition(id) {
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
                if ob_kind != "repay_debt" && ob_kind != "stranded_cargo" {
                    crate::store::clear_active_npc_obligation(id, &ob_kind);
                }
            }
            clear_resident_goal(id);

            let (prim_bldg, prim_act) = (def.resolve)(hour);
            let prim_cell = resolve_settlement_room_coord(grid, prim_bldg);
            if let Some(pc) = prim_cell {
                RESIDENT_MOVE_TARGETS.write().insert(
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

        RESIDENT_IDLE_TICKS.write().insert(id.to_string(), 0);

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
                if ob_kind != "repay_debt" && ob_kind != "stranded_cargo" {
                    crate::store::clear_active_npc_obligation(id, &ob_kind);
                }
            }
            if let Some(pc) = prim_cell {
                RESIDENT_MOVE_TARGETS.write().insert(
                    id.to_string(),
                    (pc.coord.x, pc.coord.y, prim_bldg.to_string(), prim_act.to_string()),
                );
            }
        } else {
            // 目前正在原駐點
            // 1. 若該人物為排程遠征/巡獵/商隊人物且時刻在日間（6..=20），啟動荒野巡獵或商隊遠征出巡
            if (6..=20).contains(&hour) && let Some(exp_def) = find_resident_expedition(id) {
                set_resident_goal(id, exp_def.goal_kind);
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
                RESIDENT_MOVE_TARGETS.write().insert(
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
                    RESIDENT_MOVE_TARGETS.write().insert(
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
                    RESIDENT_MOVE_TARGETS.write().insert(
                        id.to_string(),
                        (sc.coord.x, sc.coord.y, sc.name.clone(), act.to_string()),
                    );
                }
            }
        }
    }
}

/// 遍歷 RESIDENT_MOVE_TARGETS，對尚未抵達目標建築的排程住民，在 SquareGrid 上以 BFS 邁進 1 格，
/// 並回傳所有發生移動的 (npc_id, from_room_id, to_room_id) 清單，供推播給在場玩家。
pub fn advance_scheduled_residents_one_step() -> Vec<(String, String, String)> {
    let grid = match get_runtime_square_grid() {
        Some(g) => g,
        None => return Vec::new(),
    };
    let Some(store_arc) = crate::store::get_store() else {
        return Vec::new();
    };

    // 日間（06:00..21:59）排程住民停留節拍與中場巡視／因果義務驅動
    step_resident_idles(&grid);

    let targets: Vec<(String, (i32, i32, String, String))> = {
        RESIDENT_MOVE_TARGETS
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
    let online_player_ids = crate::server::session::list_online_player_ids();

    for (id, (target_x, target_y, bldg_name, target_activity)) in targets {
        if online_player_ids.iter().any(|pid| pid == &id) {
            finished.push(id);
            continue;
        }
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
                let (_, _, act) = crate::pg::living_world::step_resident_human_life_loop(
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
            get_resident_goal(&id).as_deref(),
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
                    let (_, _, act) = crate::pg::living_world::step_resident_human_life_loop(
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
        let mut w = RESIDENT_MOVE_TARGETS.write();
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
        "講堂街南口" | "講堂街西口" | "講堂街" => return grid.get(crate::grid::SquareCoord::new(1, 0)),
        "三水渡埠街" => return grid.get(crate::grid::SquareCoord::new(-2, 0)),
        "西門聯運東口" => return grid.get(crate::grid::SquareCoord::new(-3, 0)),
        "西門東街" => return grid.get(crate::grid::SquareCoord::new(-1, 0)),
        "北津石板拱橋" => return grid.get(crate::grid::SquareCoord::new(0, 3)),
        "東橋石板拱橋" => return grid.get(crate::grid::SquareCoord::new(2, 0)),
        "東橋茶屋巷口" => return grid.get(crate::grid::SquareCoord::new(3, 0)),
        "鐘樓前街" => return grid.get(crate::grid::SquareCoord::new(0, 2)),
        "公議堂東街" | "公議堂前街" => return grid.get(crate::grid::SquareCoord::new(-1, -1)),
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
    // 1. 完全或前綴/包含精確匹配（優先建築格，排除街道誤中「公議堂東街」等）
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

pub(crate) fn send_place_view(conn: &WsConnection, player_id: &str) {
    let ent = db::get_entity(player_id).ok().flatten();
    let (gx, gy) = ent
        .as_ref()
        .and_then(|e| e.grid_x.zip(e.grid_y))
        .unwrap_or((0, 0));
    let gh = current_game_hour(&conn.cfg);
    send_grid_view(conn, player_id, gx, gy, gh);
}


pub(super) fn send_grid_view(conn: &WsConnection, player_id: &str, x: i32, y: i32, game_hour: i32) {
    use super::super::broadcast::build_room_view_msg;
    use super::super::protocol::GridViewMsg;
    let view = match game::get_grid_room_view(player_id, x, y, game_hour) {
        Ok(Some(v)) => v,
        _ => return,
    };
    let base = build_room_view_msg(&view, player_id, &conn.cfg);
    let in_private_room = is_player_in_private_room(player_id);
    let season_date_label = format_season_date_label(base.game_days_since_epoch);

    // 將玩家當前格 (x, y) 標記 explored = true
    let current_coord = crate::grid::SquareCoord::new(x, y);
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(current_coord) {
            if !cell.explored {
                cell.explored = true;
            }
        }
    });

    // 將當前格與周圍 3x3 鄰格加入該玩家的足跡記錄
    {
        let mut walked_lock = PLAYER_WALKED_COORDS.write();
        let player_set = walked_lock.entry(player_id.to_string()).or_default();
        for dx in -2..=2 {
            for dy in -2..=2 {
                player_set.insert((x + dx, y + dy));
            }
        }
    }

    let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
    let current_cell = grid_snap
        .as_ref()
        .and_then(|g| g.get(current_coord).cloned());

    let npc_counts = crate::store::get_store()
        .map(|store| store.read().npc_counts_by_grid())
        .unwrap_or_default();
    let cells = build_grid_view_cells(
        x,
        y,
        player_id,
        in_private_room,
        current_coord,
        current_cell.as_ref(),
        grid_snap.as_ref(),
        &npc_counts,
    );
    // 玩家層使用人物在 Store 中的真實 current_activity；不以隨機閒話冒充生活。
    // 過濾離線玩家、測試編號帳號，並規範實體顯示
    let entities: Vec<crate::server::protocol::ViewEntity> = if in_private_room {
        vec![]
    } else {
        base.entities
            .into_iter()
            .filter(|e| {
                if e.kind == "player" || e.kind == "Player" {
                    if e.id.contains('_') || e.display_name.contains('_') {
                        return false;
                    }
                    conn.sessions
                        .get(&e.id)
                        .map(|s| s.outbound.is_some())
                        .unwrap_or(false)
                } else {
                    true
                }
            })
            .collect()
    };

    let GridViewExitsResult {
        exits_v5,
        grid_exits,
        current_place_json,
        final_room_name,
        is_lodging,
    } = build_grid_view_exits(
        x,
        y,
        player_id,
        in_private_room,
        current_coord,
        current_cell.as_ref(),
        grid_snap.as_ref(),
        &base.room_name,
    );

    let final_objects = build_grid_view_objects(
        in_private_room,
        is_lodging,
        &base.objects,
        &base.room_name,
        base.server_unix,
        current_cell.as_ref(),
    );

    let final_exits = if in_private_room {
        grid_exits.clone()
    } else {
        let mut exits = base.exits;
        if let Some(in_exit) = grid_exits
            .iter()
            .find(|e| e.direction == "進" || e.direction == "in")
        {
            if !exits
                .iter()
                .any(|e| e.direction == "進" || e.direction == "in")
            {
                exits.push(in_exit.clone());
            }
        }
        exits
    };
        // ── L4 NPC 觀察態：查 acquaintance 表，計算 display_name + card_state + 甲乙編號 ──
        let npc_entities: Vec<&crate::server::protocol::ViewEntity> = entities
            .iter()
            .filter(|e| e.kind == "npc" || e.kind == "Npc")
            .collect();

        let npc_id_strs: Vec<&str> = npc_entities.iter().map(|e| e.id.as_str()).collect();
        let acq_map: std::collections::HashMap<String, crate::db::NpcAcquaintance> =
            crate::db::get_acquaintances_for_player_npcs(player_id, &npc_id_strs)
                .unwrap_or_default()
                .into_iter()
                .map(|a| (a.npc_id.clone(), a))
                .collect();
        let available_market_operators = crate::pg::pool()
            .and_then(|pool| {
                crate::pg::world_market::available_operator_ids_for_buyer(&pool, player_id).ok()
            })
            .unwrap_or_default();

        // 為尚無記錄的 NPC 建 ensure（INSERT OR IGNORE）
        for npc in &npc_entities {
            if !acq_map.contains_key(&npc.id) {
                let _ = crate::db::ensure_acquaintance(player_id, &npc.id);
            }
        }

        // 計算每個 NPC 的顯示標籤與活動描述
        // 白塔半島具名住民直接視為已知的在地當家／掌事（state = "named"）
        let obsolete_dup_ids = [
            "wt_npc_chen_wanfu",
            "wt_npc_bai_quansheng",
            "wt_npc_shen_qinghe",
            "wt_npc_ke_chengqi",
            "wt_npc_su_jinxin",
        ];
        let scheduled_count = npc_entities
            .iter()
            .filter(|e| {
                !obsolete_dup_ids.contains(&e.id.as_str())
                    && (e.id.starts_with("wt_npc_")
                        || e.id.starts_with("wt_vendor_")
                        || e.display_name.contains('｜')
                        || e.display_name.contains('|'))
            })
            .count();
        let unnamed_cap = 10usize.saturating_sub(scheduled_count);
        let mut unnamed_kept = 0usize;
        let mut seen_scheduled_names = std::collections::HashSet::new();
        let filtered_npc_entities: Vec<_> = npc_entities
            .iter()
            .filter(|e| {
                if obsolete_dup_ids.contains(&e.id.as_str()) {
                    return false;
                }
                if e.id.starts_with("wt_npc_")
                    || e.id.starts_with("wt_vendor_")
                    || e.display_name.contains('｜')
                    || e.display_name.contains('|')
                {
                    let raw_short = e
                        .display_name
                        .rsplit(['｜', '|'])
                        .next()
                        .unwrap_or(e.display_name.as_str())
                        .trim()
                        .trim_end_matches(|c: char| c.is_ascii_digit() || matches!(c, '甲' | '乙' | '丙' | '丁' | '戊' | '己' | '庚' | '辛' | '壬' | '癸'));
                    let dedup_key = if raw_short.is_empty() || raw_short == "陌生人" {
                        crate::store::get_store()
                            .and_then(|s| s.read().get_entity(&e.id).map(|ent| ent.display_title.clone()))
                            .filter(|t| !t.is_empty())
                            .unwrap_or_else(|| e.id.clone())
                    } else {
                        raw_short.to_string()
                    };
                    return seen_scheduled_names.insert(dedup_key);
                }
                if unnamed_kept >= unnamed_cap {
                    return false;
                }
                unnamed_kept += 1;
                true
            })
            .collect();

        let hour = crate::store::current_game_hour();
        let is_night = hour >= 22 || hour < 5;
        let tail_room_for_cards = base.room_name.rsplit('・').next().unwrap_or(&base.room_name).trim();
        let is_open_street_or_bridge_for_cards = (tail_room_for_cards.contains("街")
            || tail_room_for_cards.contains("路")
            || tail_room_for_cards.contains("道")
            || tail_room_for_cards.contains("巷")
            || tail_room_for_cards.contains("橋")
            || tail_room_for_cards.contains("外環")
            || tail_room_for_cards.contains("內環")
            || tail_room_for_cards.ends_with("段")
            || tail_room_for_cards.ends_with("口")
            || tail_room_for_cards.ends_with("隅")
            || tail_room_for_cards.ends_with("坪")
            || tail_room_for_cards.ends_with("花圃")
            || tail_room_for_cards.ends_with("廣場"))
            && !tail_room_for_cards.contains("市")
            && !tail_room_for_cards.contains("鋪")
            && !tail_room_for_cards.contains("工坊")
            && !tail_room_for_cards.contains("作坊")
            && !tail_room_for_cards.contains("堂")
            && !tail_room_for_cards.contains("行")
            && !tail_room_for_cards.contains("館")
            && !tail_room_for_cards.contains("廠")
            && !tail_room_for_cards.contains("社")
            && !tail_room_for_cards.contains("屋")
            && !tail_room_for_cards.contains("診所")
            && !tail_room_for_cards.contains("浴場")
            || crate::grid::grid::is_street_or_gate_room_name(&base.room_name)
            || crate::grid::grid::is_private_residence_room_name(&base.room_name);
        let room_venue_opt = if is_open_street_or_bridge_for_cards {
            None
        } else {
            crate::grid::resolve_any_room_venue_object(&base.room_name, "")
                .filter(|v| v.venue_kind != "街巷橋樑公共空地" && v.default_host_title != "街坊居民")
        };
        let cards: Vec<NpcCard<'_>> = filtered_npc_entities
            .into_iter()
            .enumerate()
            .map(|(npc_idx, e)| {
                let is_scheduled_leader = e.id.starts_with("wt_npc_")
                    || e.id.starts_with("wt_vendor_")
                    || e.display_name.contains('｜')
                    || e.display_name.contains('|');
                let acq = acq_map.get(&e.id);
                let named = acq.map(|a| a.named).unwrap_or(false);
                let is_on_duty_staff = !is_open_street_or_bridge_for_cards
                    && (e.current_activity.contains("櫃檯")
                        || e.current_activity.contains("櫃台")
                        || e.current_activity.contains("當值")
                        || crate::pg::living_world::is_recognizable_room_staff_or_counter_holder(
                            &base.room_name,
                            &e.id,
                            &e.display_name,
                            &e.current_activity,
                            "",
                        )
                        .0
                        || room_venue_opt.as_ref().is_some_and(|v| {
                            e.current_activity == v.on_duty_fixture_activity
                        }));
                let (label, card_state) = if e.id.starts_with("ambient_") || e.id.starts_with("shanhai_") {
                    let clean_name = e.display_name.rsplit(['｜', '|']).next().unwrap_or(&e.display_name).trim().to_string();
                    (clean_name, "named")
                } else if named {
                    let clean_name =
                        crate::store::resolve_npc_person_name(&e.id, &e.display_name, None);
                    (clean_name, "named")
                } else if is_on_duty_staff
                    && let Some(venue) = &room_venue_opt
                {
                    (venue.default_host_title.to_string(), "stranger")
                } else {
                    // 尚未認識該 NPC：顯示為陌生人
                    ("陌生人".to_string(), "stranger")
                };
                let venue_activity: Option<&'static str> = if is_on_duty_staff {
                    room_venue_opt
                        .as_ref()
                        .map(|v| v.on_duty_fixture_activity)
                        .or_else(|| crate::grid::resolve_any_room_venue_object(&base.room_name, &e.display_name).map(|v| v.on_duty_fixture_activity))
                } else if is_night {
                    let r = base.room_name.as_str();
                    if r.contains("居") || r.contains("舍") || r.contains("宅") || r.contains("宿所") || r.contains("院") || r.contains("齋") {
                        Some("正在內室床鋪上熄燈安歇")
                    } else if r.contains("橋") {
                        Some("正提著防風燈沿青石橋面緩步夜行")
                    } else if r.contains("街") || r.contains("道") || r.contains("口") || r.contains("坪") {
                        Some("正提著防風燈沿著街面緩步夜行")
                    } else {
                        Some("正提著防風燈在廊下巡看門戶")
                    }
                } else if is_scheduled_leader || named {
                    crate::grid::resolve_room_ambient_visitor_activity(&base.room_name, npc_idx)
                        .or_else(|| {
                            crate::grid::resolve_any_room_venue_object(&base.room_name, &e.display_name)
                                .map(|v| v.on_duty_fixture_activity)
                        })
                } else {
                    crate::grid::resolve_room_ambient_visitor_activity(&base.room_name, npc_idx)
                        .or_else(|| {
                            crate::grid::resolve_any_room_venue_object(&base.room_name, &e.display_name)
                                .map(|v| v.on_duty_fixture_activity)
                        })
                };

                let current_act = if is_on_duty_staff {
                    venue_activity.unwrap_or(e.current_activity.as_str())
                } else if (e.id.starts_with("wt_vendor_") || !is_scheduled_leader)
                    && !e.current_activity.trim().is_empty()
                    && !e.current_activity.contains("安歇")
                    && !e.current_activity.bytes().any(|b| b.is_ascii_alphabetic())
                {
                    e.current_activity.as_str()
                } else if is_night {
                    venue_activity.unwrap_or("正在內室床鋪上熄燈安歇")
                } else if !is_scheduled_leader && !e.current_activity.trim().is_empty() {
                    e.current_activity.as_str()
                } else {
                    venue_activity.unwrap_or(e.current_activity.as_str())
                };
                NpcCard {
                    id: &e.id,
                    label,
                    card_state,
                    current_activity: current_act,
                    interaction_priority: is_scheduled_leader || is_on_duty_staff || available_market_operators.contains(&e.id),
                }
            })
            .collect();

        let unnamed_total = cards.iter().filter(|c| c.card_state != "named").count();
        let (visible_npcs, _has_hidden_unnamed) = build_player_layer_npc_cards(&cards);
        let mut npcs_json: Vec<serde_json::Value> = Vec::new();
        for c in &visible_npcs {
            npcs_json.push(serde_json::json!({
                "kind": "npc",
                "npc_id": c.id,
                "display_name": c.display_name,
                "card_state": c.card_state,
                "current_activity": c.current_activity
            }));
        }
        let total_present = cards.len();
        let sanitize_public_entry_act = |raw: &str, idx: usize| -> String {
            let trimmed = raw.trim().trim_end_matches('。');
            let r = base.room_name.as_str();
            let is_open_bridge_or_street = r.contains("橋")
                || r.contains("街")
                || r.contains("道")
                || r.contains("口")
                || r.contains("坪")
                || r.contains("原")
                || r.contains("坡");
            let is_indoor_only_act = trimmed.contains("工坊案頭")
                || trimmed.contains("居所木榻")
                || trimmed.contains("回屋舍")
                || trimmed.contains("內室")
                || trimmed.contains("單間")
                || trimmed.contains("浴池");
            let is_private_transit = trimmed.contains("動身前往")
                || trimmed.contains("步行前往")
                || trimmed.contains("沿街步行")
                || trimmed.contains("前往【")
                || trimmed.contains("回返【")
                || trimmed.contains("返回【")
                || trimmed.contains("推進目標：【")
                || trimmed.contains("趕往【")
                || (is_open_bridge_or_street && is_indoor_only_act);
            if is_private_transit {
                if r.contains("橋") {
                    if idx % 2 == 0 {
                        "正沿著青石橋面邁步穿行，往鄰街一帶行去".to_string()
                    } else {
                        "正憑著雕花青石欄杆遠眺三水川水面".to_string()
                    }
                } else if idx % 2 == 0 {
                    "正沿著街面邁步穿行，往鄰街一帶行去".to_string()
                } else {
                    "正順著廊下緩步穿行，打量四周動靜".to_string()
                }
            } else {
                trimmed.to_string()
            }
        };
        let mut present_npc_summaries: Vec<(String, String)> = visible_npcs
            .iter()
            .take(2)
            .enumerate()
            .map(|(idx, c)| {
                let subj = narrative_person_subject(c, idx);
                (subj, sanitize_public_entry_act(c.current_activity, idx))
            })
            .collect();
        let npc_name_map: std::collections::HashMap<String, String> = visible_npcs
            .iter()
            .map(|c| (c.id.to_string(), c.display_name.clone()))
            .collect();
        let entities: Vec<crate::server::protocol::ViewEntity> = entities
            .into_iter()
            .map(|mut e| {
                if e.kind == "npc" || e.kind == "Npc" {
                    if let Some(norm_name) = npc_name_map.get(e.id.as_str()) {
                        e.display_name = norm_name.clone();
                    } else {
                        let is_named = acq_map.get(e.id.as_str()).map(|a| a.named).unwrap_or(false)
                            || e.id.starts_with("shanhai_")
                            || e.id.starts_with("ambient_");
                        if !is_named {
                            e.display_name = "陌生人".to_string();
                        }
                    }
                }
                e
            })
            .collect();

        // 動態攤位物件：依據在場 NPC 與擺攤狀態產出 ViewObject，並按 id 去重併入 final_objects
        let mut final_objects = final_objects;
        let dyn_vendor_objs = crate::game::player_scene::dynamic_vendor_view_objects_for_entities(&entities);
        for v_obj in dyn_vendor_objs {
            if !final_objects.iter().any(|o| o.id == v_obj.id) {
                final_objects.push(v_obj);
            }
        }

        // 移除假群眾卡注入：讓沒人的格子誠實顯示無人，不再塞假群眾卡
        // append_m3_020_public_presence_cards(&base.room_name, &mut npcs_json);
        if present_npc_summaries.is_empty() {
            for (idx, item) in npcs_json.iter().enumerate() {
                if item.get("kind").and_then(|v| v.as_str()) == Some("npc") {
                    let id = item.get("npc_id").and_then(|v| v.as_str()).unwrap_or("");
                    let name = item.get("display_name").and_then(|v| v.as_str()).unwrap_or("").trim();
                    let state = item.get("card_state").and_then(|v| v.as_str()).unwrap_or("stranger");
                    let act = item.get("current_activity").and_then(|v| v.as_str()).unwrap_or("").trim();
                    if !name.is_empty() {
                        let card = VisibleNpcCard {
                            id,
                            display_name: name.to_string(),
                            card_state: if state == "named" { "named" } else { "stranger" },
                            current_activity: act,
                        };
                        let subj = narrative_person_subject(&card, idx);
                        present_npc_summaries.push((subj, sanitize_public_entry_act(act, idx)));
                        if present_npc_summaries.len() == 2 {
                            break;
                        }
                    }
                }
            }
        }
        // 取當前格資料，供前端 L3 渲染用。玩家層不能只收到 icon_key，
        // 否則聚落/道路/建築類地形會退成空白方塊。
        let current_cell =
            crate::server::grid_manager::get_runtime_grid_cell(crate::grid::SquareCoord::new(x, y));
        let current_terrain_key = current_cell
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
        let game_minute_of_day = (base.game_time_sec_since_midnight.rem_euclid(86_400) / 60) as u32;
        let current_desc = current_cell
            .as_ref()
            .map(|c| c.description.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if base.description.trim().is_empty() {
                    let gh = (game_minute_of_day / 60) % 24;
                    if gh <= 4 || gh >= 23 {
                        format!("{}在近前展開，夜色深沉，街面靜謐無人。", current_terrain_name)
                    } else if unnamed_total == 0 {
                        format!("{}在近前展開，四周一時安靜清朗。", current_terrain_name)
                    } else {
                        format!(
                            "{}在近前展開，人聲與腳步聲在周圍交錯。",
                            current_terrain_name
                        )
                    }
                } else {
                    base.description.trim().to_string()
                }
            });
        let scene_climate = current_l3_scene_climate();
        let scene_climate_ref = scene_climate
            .as_ref()
            .map(|(event_type, seed)| (event_type.as_str(), *seed));
        let current_room_id = crate::grid::grid_room_id_from_coord(x, y);
        let recent_scene_events = crate::npcnpc::recent_room_events(&current_room_id, 3);
        let current_desc = if in_private_room {
            let is_lodging = current_terrain_name.contains("旅社")
                || current_terrain_name.contains("茶屋")
                || current_terrain_name.contains("公寓")
                || current_terrain_name.contains("驛");
            if is_lodging {
                "你反手扣上房門的黃銅插銷。這是樓上一間安靜整潔的單人客房，窗畔擺著旋蓋夜光貝母調光台燈與烏木書案，床鋪上鋪著厚實的蓬鬆羽絨暖被與山風草安神枕，將樓下街巷的喧囂人聲隔絕在外。".to_string()
            } else {
                "你穿過門廊走入宅院內室。屋內鋪著擦拭光潔的熟桐油木地板，門口整齊擺著軟底布便鞋。室內設有松香木大床與山風草安神枕，窗畔几案上亮著旋蓋夜光貝母台燈，一側自來活水黃銅水栓與溫石熱水槽水氣氤氳，恬靜怡人。".to_string()
            }
        } else {
            let base_desc = player_layer_scene_desc(
                &current_desc,
                &current_terrain_name,
                game_minute_of_day,
                scene_climate_ref,
                unnamed_total,
                &recent_scene_events,
            );
            weave_scene_fixtures_into_desc(&base_desc, &current_terrain_name, &final_objects)
        };
        let npcs_json = if in_private_room {
            vec![]
        } else {
            npcs_json
        };
        let entry_log = player_layer_entry_log_for_scene(
            &current_terrain_name,
            unnamed_total,
            game_minute_of_day,
            scene_climate_ref,
        );
        let reentry_log = player_layer_reentry_log(&current_terrain_name, unnamed_total);
        let reentry_logs = player_layer_route_reentry_logs(&current_terrain_name, unnamed_total);

        let total_room_npcs = entities.iter().filter(|e| e.kind == "npc" || e.kind == "Npc").count();
        let room_enter_msg = serde_json::json!({
            "type": "room_enter",
            "room_id": current_room_id,
            "place": current_place_json,
            "exits": exits_v5,
            "objects": final_objects,
            "npcs": npcs_json,
            "total_npc_count": total_room_npcs,
            "desc": current_desc,
            "entry_log": entry_log,
            "reentry_log": reentry_log,
            "reentry_logs": reentry_logs,
            "time": {
                "hour_str": format!("{:02}:00", game_hour),
                "hour": game_hour
            },
            "terrain": {
                "icon_key": current_terrain_key,
                "name": if in_private_room { &final_room_name } else { &current_terrain_name },
                "category": current_terrain_category
            }
        });
        if let Ok(bytes) = serde_json::to_vec(&room_enter_msg) {
            let _ = conn.tx.try_send(bytes);
        }

    let grid_msg = GridViewMsg {
        msg_type: "grid_view".into(),
        player_x: x,
        player_y: y,
        cells,
        room_name: final_room_name,
        description: current_desc,
        exits: final_exits,
        entities,
        objects: final_objects,
        server_unix: base.server_unix,
        game_time_sec_since_midnight: base.game_time_sec_since_midnight,
        game_days_since_epoch: base.game_days_since_epoch,
        in_private_room,
        season_date_label,
    };
    conn.send_json(&grid_msg);

    if !in_private_room {
        let (is_new_coord, prev_coord) = {
            let mut last_coord = PLAYER_LAST_ANNOUNCED_COORD.write();
            let prev = last_coord.get(player_id).copied();
            if prev != Some((x, y)) {
                last_coord.insert(player_id.to_string(), (x, y));
                (true, prev)
            } else {
                (false, prev)
            }
        };
        let step_prefix = if let Some((px, py)) = prev_coord {
            if (x - px).abs() + (y - py).abs() == 1 {
                match (x - px, y - py) {
                    (1, 0) => "你往東步入",
                    (-1, 0) => "你往西步入",
                    (0, 1) => "你往北步入",
                    (0, -1) => "你往南步入",
                    _ => "你步入",
                }
            } else {
                "你步入"
            }
        } else {
            "你步入"
        };
        if is_new_coord {
            if !present_npc_summaries.is_empty() {
                let format_npc_act = |raw: &str| -> String {
                    let act = raw.trim().trim_end_matches('。');
                    if act.is_empty() {
                        "正駐足於此".to_string()
                    } else if act.starts_with('正')
                        || act.starts_with('在')
                        || act.starts_with('已')
                    {
                        act.to_string()
                    } else if act.starts_with("站在")
                        || act.starts_with("守在")
                        || act.starts_with("坐在")
                        || act.starts_with("立於")
                    {
                        format!("正{}", act)
                    } else {
                        format!("正在此處{}", act)
                    }
                };

                let is_night_time = game_hour >= 22 || game_hour < 5;
                let all_asleep = is_night_time
                    && !present_npc_summaries.is_empty()
                    && present_npc_summaries.iter().all(|(_, act)| {
                        act.contains("安歇")
                            || act.contains("熟睡")
                            || act.contains("熄燈")
                            || act.contains("就寢")
                            || act.contains("歇息")
                    });

                let line = if all_asleep {
                    format!("{step_prefix}【{}】，夜深人靜，屋內諸人皆已熄燈安歇。", grid_msg.room_name)
                } else if total_present >= 2 && present_npc_summaries.len() >= 2 {
                    let (npc1, act1_raw) = &present_npc_summaries[0];
                    let (npc2, act2_raw) = &present_npc_summaries[1];
                    let act1 = format_npc_act(act1_raw);
                    let act2 = format_npc_act(act2_raw);
                    let night_prefix = if is_night_time && total_present >= 3 {
                        "夜色微涼，"
                    } else {
                        ""
                    };
                    if act1 == act2 {
                        format!(
                            "{step_prefix}【{}】，{}見{}與{}皆{}。",
                            grid_msg.room_name, night_prefix, npc1, npc2, act1
                        )
                    } else {
                        format!(
                            "{step_prefix}【{}】，{}見{}{}；旁側{}{}。",
                            grid_msg.room_name, night_prefix, npc1, act1, npc2, act2
                        )
                    }
                } else {
                    let (npc1, act1_raw) = &present_npc_summaries[0];
                    let act1 = format_npc_act(act1_raw);
                    format!("{step_prefix}【{}】，見{}{}。", grid_msg.room_name, npc1, act1)
                };
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: line,
                });
            } else {
                let empty_line = if game_hour >= 22 || game_hour < 5 {
                    format!("{step_prefix}【{}】，夜色深沉，四周靜謐無人。", grid_msg.room_name)
                } else {
                    format!("{step_prefix}【{}】，四周一時清朗安靜。", grid_msg.room_name)
                };
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: empty_line,
                });
            }
        }
    }
}


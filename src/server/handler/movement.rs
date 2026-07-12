//! 移動／探索／離場觀測清除。六角與正方格雙路徑。

use rand::Rng;

use crate::db;
use crate::entity::Character;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::game::TimePeriod;
use crate::gametext;
use crate::grid::Freshness;

use super::super::protocol::{BlockedMsg, ClientMsg, ExploreLogMsg, LootItem, MovedMsg, StatDelta};
use super::{WsConnection, current_game_hour};

/// 探索當前正方格：標記該格為 explored，結算採集/戰鬥結果，
/// 推送 `explore_result` WS 訊息（log 散文），再推 `grid_view` 同步地圖。
pub(super) fn handle_explore_grid(conn: &WsConnection) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let ent = match db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    let (gx, gy) = match (ent.grid_x, ent.grid_y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            conn.send_error("not in grid");
            return;
        }
    };

    // 取當前格（需要 terrain + freshness）
    let cell_info = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(crate::grid::SquareCoord::new(gx, gy)).cloned());

    let (terrain_str, freshness) = cell_info
        .as_ref()
        .map(|c| (c.terrain.kind_str(), c.freshness))
        .unwrap_or(("unknown", Freshness::Fresh));

    // 把當前格標記為 explored（觸發主動探索語意）
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
            cell.explored = true;
        }
        grid.mark_dirty();
    });

    let gh = current_game_hour(&conn.cfg);
    let tp = TimePeriod::from_hour(gh);

    // ── Empty 強制短路 ───────────────────────────────────────────────
    let explore_msg = if freshness == Freshness::Empty {
        ExploreLogMsg {
            msg_type: "explore_result".into(),
            outcome: "empty".into(),
            log: "此地已被搜刮一空，再怎麼翻也只剩塵土。".into(),
            loot: vec![],
            stat_delta: StatDelta {
                hp_cur: 0,
                inner_cur: 0,
            },
            time_period_after: tp.label().into(),
            freshness_after: "empty".into(),
            warning_prose: String::new(),
        }
    } else {
        // 推進 freshness
        let new_freshness = freshness.advance();
        let _ = crate::server::grid_manager::with_square_grid_mut(|g| {
            if let Some(c) = g.get_mut(crate::grid::SquareCoord::new(gx, gy)) {
                c.freshness = new_freshness;
            }
        });
        if let Err(e) = db::update_cell_freshness(gx, gy, new_freshness) {
            tracing::warn!("[explore_ws] freshness 寫入 PG 失敗：{e}");
        }

        const BASE_HARVEST_PROB: f64 = 0.60;
        const BASE_COMBAT_PROB: f64 = 0.25;
        let roll: f64 = rand::rng().random();

        let (outcome, log, loot) = if roll < BASE_COMBAT_PROB {
            // ── 戰鬥：扣血，判昏倒 ──────────────────────────────────────
            let dmg: i32 = (rand::rng().random_range(10_u32..=25_u32)) as i32;
            let hp_after = (ent.vit - dmg).max(0);
            let _ = db::update_vit(&player_id, hp_after);

            if hp_after == 0 {
                // ── 死亡兜底：昏倒分支 ────────────────────────────
                // 1. 找最近聚落格（Manhattan 距離最短的 landmark 格）
                let (dest_x, dest_y, dest_name) =
                    find_nearest_landmark(gx, gy).unwrap_or((0, 0, "荒野某處".to_string()));

                // 2. 移動玩家到最近聚落
                let _ = db::set_entity_grid(&player_id, dest_x, dest_y);

                // 3. 掉部分物品（隨機移除 1-3 件）
                let dropped_items = drop_random_items(&player_id, &ent.inventory);

                // 4. 恢復 30% hp（避免醒來立刻再被打死），最少 1
                let max_hp = ent.vit.max(30); // vit 為體質值，以現有值 * 恢復比計算
                let recover_to = (max_hp as f64 * 0.3).ceil().max(1.0) as i32;
                let _ = db::update_vit(&player_id, recover_to);

                // 5. 組散文
                let drop_note = if dropped_items.is_empty() {
                    String::new()
                } else {
                    format!("醒來時發現部分物品遺失（{}）。", dropped_items.join("、"))
                };
                let faint_log = format!(
                    "傷勢過重，眼前一黑——待意識重歸，已身處【{}】。{}",
                    dest_name, drop_note
                );

                // 6. 推 explore_result（昏倒散文）
                let faint_msg = ExploreLogMsg {
                    msg_type: "explore_result".into(),
                    outcome: "faint".into(),
                    log: faint_log,
                    loot: vec![],
                    stat_delta: StatDelta {
                        hp_cur: -(ent.vit),
                        inner_cur: 0,
                    },
                    time_period_after: tp.label().into(),
                    freshness_after: freshness.as_str().into(),
                    warning_prose: String::new(),
                };
                conn.send_json(&faint_msg);

                // 7. 推 grid_view 從新座標
                send_grid_view(conn, &player_id, dest_x, dest_y, gh);
                return;
            }

            let prose = match terrain_str {
                "forest" | "forest_heavy" | "jungle" => "林間風動，一道身影竄出——廝殺在瞬間落定。",
                "mountain" => "亂石滾落，伏兵從側翼殺出——生死於呼吸間定奪。",
                _ => "危機猛然迫近——一場遭遇就此展開。",
            };
            let stat = StatDelta {
                hp_cur: -dmg,
                inner_cur: 0,
            };
            // 直接推 explore_result 含傷勢，return 前先組完整訊息
            let combat_msg = ExploreLogMsg {
                msg_type: "explore_result".into(),
                outcome: "combat".into(),
                log: format!("{}（受傷 {} 點）", prose, dmg),
                loot: vec![],
                stat_delta: stat,
                time_period_after: tp.label().into(),
                freshness_after: new_freshness.as_str().into(),
                warning_prose: String::new(),
            };
            conn.send_json(&combat_msg);
            send_grid_view(conn, &player_id, gx, gy, gh);
            return;
        } else if roll < BASE_COMBAT_PROB + BASE_HARVEST_PROB {
            let prose = match terrain_str {
                "forest" | "forest_heavy" | "forest_light" | "jungle" => {
                    "撥開枯葉，拾得野莓三顆，露水未乾。"
                }
                "mountain" | "hills" => "石縫間尋得礦脈，握拳磨碎，帶走幾塊碎礦。",
                "swamp" => "泥水沒過腳踝，摸出一株苦草，黏滑刺鼻。",
                _ => "環顧四周，摸索一番，收穫尚可。",
            };
            let item_name = match terrain_str {
                "forest" | "forest_heavy" | "forest_light" | "jungle" => "野莓",
                "mountain" | "hills" => "礦石碎塊",
                "swamp" => "苦草",
                _ => "雜材",
            };
            let qty = if freshness == Freshness::Half { 2 } else { 3 };
            let loot = vec![LootItem {
                item_id: format!("harvest_{terrain_str}"),
                name: item_name.into(),
                qty,
            }];
            ("harvest".into(), prose.into(), loot)
        } else {
            let prose = match freshness {
                Freshness::Half => "搜尋一圈，所獲寥寥，大半已被先前帶走。",
                _ => "四下無異，偶有蟲鳴，此刻毫無所獲。",
            };
            ("empty".into(), prose.into(), vec![])
        };

        ExploreLogMsg {
            msg_type: "explore_result".into(),
            outcome,
            log,
            loot,
            stat_delta: StatDelta {
                hp_cur: 0,
                inner_cur: 0,
            },
            time_period_after: tp.label().into(),
            freshness_after: new_freshness.as_str().into(),
            warning_prose: String::new(),
        }
    };

    conn.send_json(&explore_msg);
    send_grid_view(conn, &player_id, gx, gy, gh);
}

pub(super) fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    if msg.direction.is_empty() {
        conn.send_error("direction required");
        return;
    }
    let old_ent = db::get_entity(&player_id).ok().flatten();
    let Some(oe) = old_ent else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };

    // 正方格路徑
    if oe.grid_x.is_some() && oe.grid_y.is_some() {
        handle_move_grid(conn, &player_id, &msg.direction, &oe);
        return;
    }
    conn.send_error(gametext::client("move_failed"));
}

fn handle_move_grid(conn: &WsConnection, player_id: &str, direction: &str, _oe: &Character) {
    let (new_room, ok) = match game::move_by_grid_direction(player_id, direction) {
        Ok(v) => v,
        Err(_) => {
            conn.send_error(gametext::client("move_failed"));
            return;
        }
    };
    if !ok {
        let _ = event::append(game::now_unix(), player_id, BLOCKED, direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.to_string(),
        });
        return;
    }
    let Some(coord) = crate::grid::parse_grid_room_id(&new_room) else {
        conn.send_error(gametext::client("move_failed"));
        return;
    };
    let gh = current_game_hour(&conn.cfg);
    send_grid_view(conn, player_id, coord.x, coord.y, gh);
    let room_name = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| {
            g.get(coord).map(|c| {
                if c.name.is_empty() {
                    crate::grid::terrain_name_zh(c.terrain).to_string()
                } else {
                    c.name.clone()
                }
            })
        })
        .unwrap_or_default();
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.to_string(),
        room_id: new_room,
        room_name,
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
}

pub(super) fn send_grid_view(conn: &WsConnection, player_id: &str, x: i32, y: i32, game_hour: i32) {
    use super::super::broadcast::build_room_view_msg;
    use super::super::protocol::{GridCellView, GridViewMsg};
    use crate::npc::narrative;

    let view = match game::get_grid_room_view(player_id, x, y, game_hour) {
        Ok(Some(v)) => v,
        _ => return,
    };
    let base = build_room_view_msg(&view, player_id, &conn.cfg);
    let cells_raw = game::get_grid_cells_around(x, y, 6);
    let cells: Vec<GridCellView> = cells_raw
        .into_iter()
        .map(
            |(cx, cy, kind, terrain, category, name, explored, walkable)| GridCellView {
                x: cx,
                y: cy,
                kind,
                terrain,
                category,
                name,
                explored,
                walkable,
            },
        )
        .collect();

    // 為方格 NPC 填入行為敘事句（以當前 unix 秒 + id 為種子）
    let tick_seed = game::now_unix() as u64;
    let entities: Vec<crate::server::protocol::ViewEntity> = base
        .entities
        .into_iter()
        .map(|mut ent| {
            if (ent.kind == "npc" || ent.kind == "Npc") && ent.behavior_text.is_empty() {
                ent.behavior_text = narrative::idle(&ent.id, &ent.display_name, tick_seed);
            }
            ent
        })
        .collect();

    // 先把 cells 轉為 exits_v5 所需的四方鄰格資訊，讓前端路口 UI 可以顯示 ▒▒▒
    // 若當前格有顯式出口（村落格）則只顯示出口表中的方向；否則查座標鄰格
    {
        let mut exits_v5 = serde_json::Map::new();
        let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
        let current_coord = crate::grid::SquareCoord::new(x, y);

        #[allow(clippy::collapsible_if)]
        if let Some(ref g) = grid_snap {
            if let Some(cell) = g.get(current_coord) {
                if !cell.explicit_exits.is_empty() {
                    // 村落格：以顯式出口表為準
                    let all_dirs = ["north", "south", "east", "west"];
                    for dir_key in &all_dirs {
                        let game_dir = match *dir_key {
                            "north" => crate::grid::Direction::North,
                            "south" => crate::grid::Direction::South,
                            "east" => crate::grid::Direction::East,
                            "west" => crate::grid::Direction::West,
                            _ => unreachable!(),
                        };
                        if let Some(ex) = cell
                            .explicit_exits
                            .iter()
                            .find(|e| e.dir == game_dir && e.target != current_coord)
                        {
                            match g.get(ex.target) {
                                Some(tc) => {
                                    let name = if tc.name.is_empty() {
                                        crate::grid::terrain_name_zh(tc.terrain).to_string()
                                    } else {
                                        tc.name.clone()
                                    };
                                    exits_v5.insert(
                                        dir_key.to_string(),
                                        serde_json::json!({
                                            "explored": tc.explored,
                                            "walkable": true,
                                            "name": name
                                        }),
                                    );
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
                        } else {
                            // 此方向無出口
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
                    // 荒野格：原座標鄰接邏輯
                    let dirs: &[(&str, i32, i32)] = &[
                        ("north", 0, 1),
                        ("south", 0, -1),
                        ("east", 1, 0),
                        ("west", -1, 0),
                    ];
                    for (dir_key, dx, dy) in dirs {
                        let nx = x + dx;
                        let ny = y + dy;
                        let neighbor = g.get(crate::grid::SquareCoord::new(nx, ny));
                        match neighbor {
                            Some(nc) => {
                                exits_v5.insert(
                                    dir_key.to_string(),
                                    serde_json::json!({
                                        "explored": nc.explored,
                                        "walkable": nc.terrain.walkable(),
                                        "name": nc.name
                                    }),
                                );
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

        // 為尚無記錄的 NPC 建 ensure（INSERT OR IGNORE）
        for npc in &npc_entities {
            if !acq_map.contains_key(&npc.id) {
                let _ = crate::db::ensure_acquaintance(player_id, &npc.id);
            }
        }

        // 計算每個 NPC 的顯示標籤（不含甲乙）
        // 陌生 → 職稱（display_name 前段，即 split('|').0 或完整 display_name），但剝除真名
        // 已觀察 → 職稱（display_name 前段）
        // 已知名 → 職稱+真名（完整 display_name）
        let cards: Vec<NpcCard<'_>> = npc_entities
            .iter()
            .map(|e| {
                let acq = acq_map.get(&e.id);
                let named = acq.map(|a| a.named).unwrap_or(false);
                let observed = acq.map(|a| a.observed).unwrap_or(false);
                let (label, card_state) = if named {
                    // 知道姓名：顯示完整 display_name（職稱+真名）
                    (e.display_name.clone(), "named")
                } else if observed {
                    // 已觀察身份：顯示職稱（display_name 裡 | 前段，或完整）
                    let title = observed_label_for_scene(&e.display_name, &base.room_name);
                    (title, "observed")
                } else {
                    // 陌生：顯示「行人」（職稱的通稱化，不洩漏真實職業）
                    // 這裡用「行人」作為陌生態通稱，spec 上只說甲乙範例是行人
                    // 但 NPC 的職業仍不暴露——保留 "行人" 作統一陌生標籤
                    ("行人".to_string(), "stranger")
                };
                NpcCard {
                    id: &e.id,
                    label,
                    card_state,
                }
            })
            .collect();

        let unnamed_total = cards.iter().filter(|c| c.card_state != "named").count();
        let (visible_npcs, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        let mut npcs_json: Vec<serde_json::Value> = Vec::new();
        for c in &visible_npcs {
            npcs_json.push(serde_json::json!({
                "kind": "npc",
                "npc_id": c.id,
                "display_name": c.display_name,
                "card_state": c.card_state
            }));
        }
        if has_hidden_unnamed {
            npcs_json.push(serde_json::json!({
                "kind": "crowd",
                "display_name": crowd_label_for_scene(&base.room_name),
                "card_state": "crowd"
            }));
        }
        append_m3_020_public_presence_cards(&base.room_name, &mut npcs_json);

        // 取當前格資料，供前端 L3 渲染用。玩家層不能只收到 icon_key，
        // 否則聚落/道路/建築類地形會退成空白方塊。
        let current_cell = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(crate::grid::SquareCoord::new(x, y)).cloned());
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
        let current_desc = current_cell
            .as_ref()
            .map(|c| c.description.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| {
                if base.description.trim().is_empty() {
                    format!(
                        "{}在近前展開，人聲與腳步聲在周圍交錯。",
                        current_terrain_name
                    )
                } else {
                    base.description.trim().to_string()
                }
            });
        let game_minute_of_day = (base.game_time_sec_since_midnight.rem_euclid(86_400) / 60) as u32;
        let scene_climate = current_l3_scene_climate();
        let scene_climate_ref = scene_climate
            .as_ref()
            .map(|(event_type, seed)| (event_type.as_str(), *seed));
        let current_room_id = crate::grid::grid_room_id_from_coord(x, y);
        let recent_scene_events = crate::npcnpc::recent_room_events(&current_room_id, 3);
        let current_desc = player_layer_scene_desc(
            &current_desc,
            &current_terrain_name,
            game_minute_of_day,
            scene_climate_ref,
            unnamed_total,
            &recent_scene_events,
        );
        let entry_log = player_layer_entry_log_for_scene(
            &current_terrain_name,
            unnamed_total,
            game_minute_of_day,
            scene_climate_ref,
        );
        let reentry_log = player_layer_reentry_log(&current_terrain_name, unnamed_total);
        let reentry_logs = player_layer_route_reentry_logs(&current_terrain_name, unnamed_total);

        let room_enter_msg = serde_json::json!({
            "type": "room_enter",
            "room_id": current_room_id,
            "exits": exits_v5,
            "objects": base.objects.clone(),
            "npcs": npcs_json,
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
                "name": current_terrain_name,
                "category": current_terrain_category
            }
        });
        if let Ok(bytes) = serde_json::to_vec(&room_enter_msg) {
            let _ = conn.tx.try_send(bytes);
        }
    }

    let grid_msg = GridViewMsg {
        msg_type: "grid_view".into(),
        player_x: x,
        player_y: y,
        cells,
        room_name: base.room_name,
        description: base.description,
        exits: base.exits,
        entities,
        objects: base.objects,
        server_unix: base.server_unix,
        game_time_sec_since_midnight: base.game_time_sec_since_midnight,
        game_days_since_epoch: base.game_days_since_epoch,
    };
    conn.send_json(&grid_msg);
}

fn player_layer_scene_desc(
    desc: &str,
    terrain_name: &str,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> String {
    let text = desc.trim();
    let terrain = terrain_name.trim();
    if let Some(scene_desc) = scene_desc_override(
        terrain,
        game_minute_of_day,
        climate,
        unnamed_total,
        recent_events,
    ) {
        return scene_desc;
    }
    let backend_leaks = [
        "目前登入後的起點",
        "登入後的暫存起點",
        "世界地圖尚未接入",
        "尚未寫入場景描述",
    ];

    if text.is_empty() || backend_leaks.iter().any(|needle| text.contains(needle)) {
        if terrain.contains("北岸") || terrain.contains("等候") {
            return north_bank_waiting_scene_desc(
                game_minute_of_day,
                climate,
                unnamed_total,
                recent_events,
            );
        }
        if terrain.is_empty() {
            return "一處可落腳的地方展在近前，周圍有人聲與腳步聲，幾條去處從眼前分開。"
                .to_string();
        }
        return format!(
            "{}在近前展開，周圍有人聲與腳步聲，幾條去處從眼前分開。",
            terrain
        );
    }

    text.to_string()
}

fn current_l3_scene_climate() -> Option<(String, i64)> {
    db::get_region_climate("default")
        .ok()
        .flatten()
        .map(|c| (c.event_type, c.prose_seed))
}

fn north_bank_waiting_scene_desc(
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> String {
    let (climate_key, climate_seed) = climate.unwrap_or(("clear", 0));
    let game_hour = (game_minute_of_day / 60) % 24;
    let ferry = FerryPhase::from_minute(game_minute_of_day);
    let seed = climate_seed
        .wrapping_add(game_hour as i64 * 31)
        .wrapping_add(unnamed_total as i64 * 17)
        .wrapping_add(ferry.seed_weight());
    let skeleton = "北岸渡口沿著中河鋪開，濕木踏板接著渡船；短市和歇腳處貼在岸邊。";
    let phenomenon = north_bank_phenomenon(game_hour, climate_key, seed);
    let object_surface = north_bank_object_surface(seed);
    let ferry_presence = north_bank_ferry_presence(ferry, seed);
    let presence = north_bank_presence(unnamed_total, seed);
    let trace = north_bank_trace(recent_events, climate_key, ferry, unnamed_total, seed);
    format!("{skeleton}{phenomenon}{object_surface}{ferry_presence}{presence}{trace}")
}

fn north_bank_phenomenon(game_hour: u32, climate_key: &str, seed: i64) -> &'static str {
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "雨後的岸泥被踩得發亮，河霧貼著水面往踏板下鑽。",
                "濕氣從木板縫裡返上來，船繩和價牌都帶著一層潮意。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "河霧沿水面壓上北岸，幾步外的人影只剩輪廓。",
                "霧把對岸收得很遠，連船腹拍水聲都像隔了一層布。",
            ],
            seed,
        ),
        "wind" => pick_scene_variant(
            &[
                "河風捲起濕氣，吹得價牌和船繩一齊輕響。",
                "風從河面推過來，短市棚布低低鼓了一下又垂回去。",
            ],
            seed,
        ),
        "snow" => pick_scene_variant(
            &[
                "冷氣壓在河面上，踏板邊緣泛著薄白。",
                "河霧混著寒意貼住木樁，等船人的袖口都收得更緊。",
            ],
            seed,
        ),
        _ => match scene_phase(game_hour) {
            ScenePhase::Dawn => pick_scene_variant(
                &[
                    "晨霧從水面慢慢抬起，先露出船繩，再露出踏板前的人影。",
                    "天光還薄，河霧貼著北岸不肯散，濕木的氣味比人聲先到。",
                ],
                seed,
            ),
            ScenePhase::Day => pick_scene_variant(
                &[
                    "日光落在河面上，霧氣退到水邊，只剩木板下方還濕亮。",
                    "河面被日色照開，踏板和短市之間的人影變得清楚。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "暮色壓低水面，河霧又從踏板底下漫回來。",
                    "天色往暗處沉，短市的價牌先失去顏色，河面還反著一點光。",
                ],
                seed,
            ),
            ScenePhase::Night => pick_scene_variant(
                &[
                    "夜水反著微光，河霧把渡船邊緣抹得不太分明。",
                    "夜風貼著水面走，濕木踏板在暗處泛出一線冷光。",
                ],
                seed,
            ),
        },
    }
}

fn north_bank_presence(unnamed_total: usize, seed: i64) -> &'static str {
    if unnamed_total >= 12 {
        pick_scene_variant(
            &[
                "候渡的人群壓住岸邊空地，貨聲、問價聲和等船的低語混在一起。",
                "短市邊人影密了些，包袱和貨擔把通向踏板的空隙擠窄。",
            ],
            seed,
        )
    } else if unnamed_total >= 5 {
        pick_scene_variant(
            &[
                "幾撥等船的人散在茶桶和踏板之間，不時有人朝河面張望。",
                "有人守著包袱歇腳，有人往短市那頭問價，視線仍留著渡船方向。",
            ],
            seed,
        )
    } else if unnamed_total > 0 {
        pick_scene_variant(
            &[
                "零散的候渡者站在河風裡，說話聲比水聲低。",
                "岸邊只聚著少數等船的人，踏板前的腳步聲顯得更清楚。",
            ],
            seed,
        )
    } else {
        "人聲暫時退開，只剩水聲和濕木在近處。"
    }
}

fn north_bank_object_surface(seed: i64) -> &'static str {
    pick_scene_variant(
        &[
            "長凳旁的蘆叢和貨擔都貼著踏板邊等人伸手，熱湯攤分出滿碗與等分湯，隊列木牌把下一輪上船的人壓成一線。",
            "歇腳處留著半段乾位，湯桶熱氣貼著河霧上浮，船繩旁的貨擔把路口擠窄；谷地路口的泥印從人群後面斜出去。",
        ],
        seed,
    )
}

fn north_bank_ferry_presence(ferry: FerryPhase, seed: i64) -> &'static str {
    match ferry {
        FerryPhase::NorthDocked => pick_scene_variant(
            &[
                "渡船靠在北岸這側，船腹碰岸時，踏板前的人聲短短收住。",
                "船身貼著北岸木樁輕晃，濕繩繃在踏板邊，等船的人都看著船戶手勢。",
            ],
            seed,
        ),
        FerryPhase::CrossingSouth => pick_scene_variant(
            &[
                "渡船剛離北岸，撐篙在水面攪開一道還沒合攏的痕。",
                "船影正往南岸滑去，北岸踏板前的腳步暫時收住。",
            ],
            seed,
        ),
        FerryPhase::SouthDocked => pick_scene_variant(
            &[
                "渡船停在南岸那頭，北岸這邊的人聲便比水聲更急些。",
                "對岸船邊有人影起落，北岸踏板空著，只剩船繩在木樁旁垂著。",
            ],
            seed,
        ),
        FerryPhase::CrossingNorth => pick_scene_variant(
            &[
                "渡船正從南岸回頭，船頭在河霧裡一點一點放大。",
                "水面傳來撐篙聲，等船的人群跟著把視線轉回踏板前。",
            ],
            seed,
        ),
    }
}

fn north_bank_trace(
    recent_events: &[String],
    climate_key: &str,
    ferry: FerryPhase,
    unnamed_total: usize,
    seed: i64,
) -> &'static str {
    if let Some(trace) = trace_from_recent_room_events(recent_events) {
        return trace;
    }
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "岸泥上新舊腳印疊在一起，靠近踏板的幾道還泛著水光。",
                "棚腳滴下的雨水把泥痕沖淡，又立刻被新的腳步踩亂。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "霧裡偶爾露出一串剛踩過的濕腳印，往踏板方向斷斷續續。",
                "木樁旁的濕繩痕還清楚，像剛被人重新繞緊過。",
            ],
            seed,
        ),
        _ => match ferry {
            FerryPhase::NorthDocked | FerryPhase::CrossingSouth => {
                "踏板邊剛讓出過一條縫，濕木上還留著一排急促腳印。"
            }
            FerryPhase::SouthDocked | FerryPhase::CrossingNorth if unnamed_total >= 5 => {
                "岸邊腳印交疊得密，幾道貨擔拖痕從短市斜斜壓向踏板前。"
            }
            _ => "",
        },
    }
}

fn trace_from_recent_room_events(recent_events: &[String]) -> Option<&'static str> {
    let joined = recent_events.join(" ");
    let text = joined.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains("貨")
        || text.contains("箱")
        || text.contains("袋")
        || text.contains("搬")
        || text.contains("賣")
    {
        Some("泥地上留著剛拖過貨擔的痕，從短市那頭一路壓向踏板前。")
    } else if text.contains("離")
        || text.contains("去")
        || text.contains("往")
        || text.contains("leave")
    {
        Some("踏板前剛空出一小塊地方，濕腳印沿著人群邊緣斷開。")
    } else if text.contains("到")
        || text.contains("來")
        || text.contains("抵")
        || text.contains("arrive")
    {
        Some("近處多了幾道新腳印，泥水還沒來得及把邊緣抹平。")
    } else if text.contains("說")
        || text.contains("問")
        || text.contains("談")
        || text.contains("低聲")
    {
        Some("短市和踏板之間還留著剛壓下去的低語，像水聲一過就會散。")
    } else {
        None
    }
}

fn pick_scene_variant(pool: &'static [&'static str], seed: i64) -> &'static str {
    if pool.is_empty() {
        return "";
    }
    let idx = seed.unsigned_abs() as usize % pool.len();
    pool[idx]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScenePhase {
    Dawn,
    Day,
    Dusk,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FerryPhase {
    NorthDocked,
    CrossingSouth,
    SouthDocked,
    CrossingNorth,
}

impl FerryPhase {
    fn from_minute(game_minute_of_day: u32) -> Self {
        match game_minute_of_day % 40 {
            0..=9 => FerryPhase::NorthDocked,
            10..=19 => FerryPhase::CrossingSouth,
            20..=29 => FerryPhase::SouthDocked,
            _ => FerryPhase::CrossingNorth,
        }
    }

    fn seed_weight(self) -> i64 {
        match self {
            FerryPhase::NorthDocked => 0,
            FerryPhase::CrossingSouth => 19,
            FerryPhase::SouthDocked => 37,
            FerryPhase::CrossingNorth => 53,
        }
    }
}

fn scene_phase(hour: u32) -> ScenePhase {
    match hour % 24 {
        5..=10 => ScenePhase::Dawn,
        11..=16 => ScenePhase::Day,
        17..=19 => ScenePhase::Dusk,
        _ => ScenePhase::Night,
    }
}

fn scene_desc_override(
    room_name: &str,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> Option<String> {
    if room_name.contains("北岸") || room_name.contains("等候") {
        Some(north_bank_waiting_scene_desc(
            game_minute_of_day,
            climate,
            unnamed_total,
            recent_events,
        ))
    } else if room_name.contains("短市") {
        Some("岸邊短市沿著木架排開，價牌被河風推得輕晃；短貨、乾糧與船票消息混在一起，被等船的人低聲議價。".to_string())
    } else if room_name.contains("歇腳") {
        Some("臨河歇腳處擺著幾條長凳，茶桶冒著淡淡熱氣；歇腳的人把包袱壓在腳邊，眼神仍留著河面的動靜。".to_string())
    } else if room_name.contains("踏板") {
        Some("渡船踏板濕滑發亮，一端扣著船繩，一端連著岸上的木樁；等船的人在踏板前收住腳步，聽水聲拍著船腹。".to_string())
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("南岸泥坡被來往腳印踩得發亮，河水退去後留下濕泥與碎草；剛下船的人沿坡上行，也有人回頭看渡船是否還會再靠。".to_string())
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("船戶作業點堆著纜繩、撐篙與修補木片；船邊的人低聲分派活計，偶爾有人伸手試一試船身晃動。".to_string())
    } else if room_name.contains("倉棚前場") {
        Some("倉棚前場半開著棚門，麻袋與木箱靠牆排放；搬貨的人在泥地上留下一串深腳印，等著下一趟船把貨送走。".to_string())
    } else {
        None
    }
}

#[derive(Debug)]
struct NpcCard<'a> {
    id: &'a str,
    label: String,
    card_state: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
struct VisibleNpcCard<'a> {
    id: &'a str,
    display_name: String,
    card_state: &'static str,
}

fn build_player_layer_npc_cards<'a>(cards: &'a [NpcCard<'a>]) -> (Vec<VisibleNpcCard<'a>>, bool) {
    const UNNAMED_VISIBLE_LIMIT: usize = 5;
    const UNNAMED_LABEL_VISIBLE_LIMIT: usize = 3;
    const TIANGAN: [&str; 10] = ["甲", "乙", "丙", "丁", "戊", "己", "庚", "辛", "壬", "癸"];

    let mut selected: Vec<&NpcCard<'a>> = Vec::new();
    let mut unnamed_visible_total = 0usize;
    let mut unnamed_label_visible_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    let mut has_hidden_unnamed = false;

    for card in cards {
        if card.card_state == "named" {
            selected.push(card);
            continue;
        }

        let label_visible_count = unnamed_label_visible_counts
            .get(card.label.as_str())
            .copied()
            .unwrap_or(0);
        if unnamed_visible_total >= UNNAMED_VISIBLE_LIMIT
            || label_visible_count >= UNNAMED_LABEL_VISIBLE_LIMIT
        {
            has_hidden_unnamed = true;
            continue;
        }

        unnamed_visible_total += 1;
        *unnamed_label_visible_counts
            .entry(card.label.as_str())
            .or_insert(0) += 1;
        selected.push(card);
    }

    // 甲乙編號：同 label 桶內 ≥ 2 人按順序附上天干。
    let mut label_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for card in &selected {
        *label_counts.entry(card.label.as_str()).or_insert(0) += 1;
    }

    let mut label_cursor: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let visible = selected
        .into_iter()
        .map(|card| {
            let count = label_counts.get(card.label.as_str()).copied().unwrap_or(1);
            let display_name = if count >= 2 {
                let cursor = label_cursor.entry(card.label.as_str()).or_insert(0);
                let suffix = TIANGAN.get(*cursor).copied().unwrap_or("甲");
                *cursor += 1;
                format!("{}{}", card.label, suffix)
            } else {
                card.label.clone()
            };
            VisibleNpcCard {
                id: card.id,
                display_name,
                card_state: card.card_state,
            }
        })
        .collect();

    (visible, has_hidden_unnamed)
}

fn crowd_label_for_scene(room_name: &str) -> &'static str {
    scene_crowd_label(room_name).unwrap_or("人群")
}

fn m3_020_public_presence_for_room(room_name: &str) -> Option<(&'static str, &'static str)> {
    const M3_020_ROOM_NAMES: [&str; 8] = [
        "城邦外坡道",
        "石壘門",
        "門內券道",
        "旗石廣場",
        "西軸坡街",
        "議棚前階",
        "鑄幣巷口",
        "傭工棚邊",
    ];
    if !M3_020_ROOM_NAMES
        .iter()
        .any(|room| room_name.contains(room))
    {
        return None;
    }
    let role = game::final_map_public_role_for_room(room_name)?;
    let crowd = game::final_map_crowd_label_for_room(room_name)?;
    Some((role, crowd))
}

fn append_m3_020_public_presence_cards(room_name: &str, cards: &mut Vec<serde_json::Value>) {
    let Some((role, crowd)) = m3_020_public_presence_for_room(room_name) else {
        return;
    };
    push_public_presence_card(cards, role);
    push_public_presence_card(cards, crowd);
}

fn push_public_presence_card(cards: &mut Vec<serde_json::Value>, label: &'static str) {
    if cards
        .iter()
        .any(|card| card.get("display_name").and_then(serde_json::Value::as_str) == Some(label))
    {
        return;
    }
    cards.push(serde_json::json!({
        "kind": "crowd",
        "display_name": label,
        "card_state": "crowd"
    }));
}

#[cfg(test)]
fn player_layer_entry_log(room_name: &str, stranger_total: usize) -> String {
    player_layer_entry_log_for_scene(room_name, stranger_total, 0, None)
}

fn player_layer_entry_log_for_scene(
    room_name: &str,
    stranger_total: usize,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
) -> String {
    if stranger_total == 0 {
        return String::new();
    }
    if room_name.contains("北岸") || room_name.contains("等候") {
        return north_bank_entry_scene_flow(game_minute_of_day, climate, stranger_total);
    }
    scene_entry_log(room_name).unwrap_or("").to_string()
}

fn player_layer_reentry_log(room_name: &str, stranger_total: usize) -> &'static str {
    if stranger_total == 0 {
        return "";
    }
    scene_reentry_log(room_name).unwrap_or("")
}

fn player_layer_route_reentry_logs(room_name: &str, stranger_total: usize) -> serde_json::Value {
    let mut out = serde_json::Map::new();
    if stranger_total == 0 {
        return serde_json::Value::Object(out);
    }
    for (from_room, line) in scene_route_reentry_logs(room_name) {
        out.insert(from_room.to_string(), serde_json::json!(line));
    }
    serde_json::Value::Object(out)
}

#[cfg(test)]
fn route_reentry_log_for_room(
    room_name: &str,
    from_room_name: &str,
    stranger_total: usize,
) -> Option<&'static str> {
    if stranger_total == 0 {
        return None;
    }
    scene_route_reentry_logs(room_name)
        .into_iter()
        .find(|(from, _)| *from == from_room_name)
        .map(|(_, line)| line)
}

fn observed_label_for_scene(raw_name: &str, room_name: &str) -> String {
    let title = raw_name.split('|').next().unwrap_or(raw_name).trim();
    if !title.is_empty() && !looks_like_private_name(title) {
        return title.to_string();
    }
    game::public_person_label_for_room(room_name)
        .unwrap_or("行人")
        .to_string()
}

fn scene_crowd_label(room_name: &str) -> Option<&'static str> {
    if let Some(label) = game::final_map_crowd_label_for_room(room_name) {
        Some(label)
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        Some("候渡人群")
    } else if room_name.contains("水網入口") {
        Some("水網人群")
    } else if room_name.contains("湖港木棧") {
        Some("湖港人群")
    } else if room_name.contains("短船泊位") {
        Some("短船人群")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some("分揀人群")
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some("蘆棚人群")
    } else if room_name.contains("水倉") {
        Some("水倉人群")
    } else if room_name.contains("糧倉外秤") {
        Some("外秤人群")
    } else if room_name.contains("倉門前場") {
        Some("倉門人群")
    } else if room_name.contains("曬場邊") {
        Some("曬場人群")
    } else if room_name.contains("車行口") {
        Some("車行人群")
    } else if room_name.contains("守倉棚") {
        Some("守倉人群")
    } else if room_name.contains("糧車泊位") {
        Some("糧車人群")
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some("河口人群")
    } else if room_name.contains("稅站棚") {
        Some("稅站人群")
    } else if room_name.contains("船行門面") {
        Some("船行人群")
    } else if room_name.contains("潮線貨場") {
        Some("貨場人群")
    } else if room_name.contains("河海倉門") {
        Some("河海倉門人群")
    } else if room_name.contains("商船候泊線") {
        Some("候泊人群")
    } else if room_name.contains("東南船塢口") {
        Some("船塢人群")
    } else if room_name.contains("修船架") {
        Some("修船人群")
    } else if room_name.contains("補給倉") {
        Some("補給人群")
    } else if room_name.contains("碼頭木階") {
        Some("木階人群")
    } else if room_name.contains("帆布曬場") {
        Some("補帆人群")
    } else if room_name.contains("船戶街口") {
        Some("船戶人群")
    } else if room_name.contains("外貿碼頭") {
        Some("外貿人群")
    } else if room_name.contains("海關前棚") {
        Some("海關人群")
    } else if room_name.contains("商館街口") {
        Some("商館人群")
    } else if room_name.contains("遠洋倉門") {
        Some("遠洋倉門人群")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯人群")
    } else if room_name.contains("匯兌小屋") {
        Some("匯兌人群")
    } else if room_name.contains("北灣避風碼頭") {
        Some("避風港人群")
    } else if room_name.contains("港灣貨棚") {
        Some("貨棚人群")
    } else if room_name.contains("漁網曬場") {
        Some("補網人群")
    } else if room_name.contains("冷霧魚市") {
        Some("魚市人群")
    } else if room_name.contains("船戶屋排") {
        Some("船戶屋排人群")
    } else if room_name.contains("北灣守望石階") {
        Some("守望人群")
    } else if room_name.contains("礦坑外棚") {
        Some("礦坑人群")
    } else if room_name.contains("洗礦溝") {
        Some("洗礦人群")
    } else if room_name.contains("粗料倉") {
        Some("粗料倉人群")
    } else if room_name.contains("工具修棚") {
        Some("修具人群")
    } else if room_name.contains("礦渣坡") {
        Some("礦渣坡人群")
    } else if room_name.contains("舊道裝車點") {
        Some("裝車人群")
    } else if room_name.contains("谷地路口")
        || room_name.contains("北堤")
        || room_name.contains("外谷地路")
        || room_name.contains("南側谷地路")
        || room_name.contains("渡口貨路")
        || room_name.contains("貨路外彎")
    {
        Some("行路人群")
    } else if room_name.contains("田埂") || room_name.contains("上田") {
        Some("田埂人群")
    } else if room_name.contains("挑擔小徑") {
        Some("挑擔人群")
    } else if room_name.contains("短市") {
        Some("短市人群")
    } else if room_name.contains("歇腳") {
        Some("歇腳人群")
    } else if room_name.contains("踏板") {
        Some("候船人群")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("上岸人群")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("船邊人群")
    } else if room_name.contains("倉棚") {
        Some("倉棚人群")
    } else {
        None
    }
}

fn scene_entry_log(room_name: &str) -> Option<&'static str> {
    if let Some(line) = game::final_map_entry_log_for_room(room_name) {
        Some(line)
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        Some(
            "船繩旁有人把濕繩重新繞上木樁，船身在水霧裡輕輕碰了一下岸。\n短市那頭有人壓低聲音討價，木牌被指節敲得輕響。\n等船人群往踏板口挪了一步，又停在河面傳來的水聲裡。",
        )
    } else if room_name.contains("中河谷地路口") {
        Some("你走到中河谷地路口。渡口人聲從身後貼來，泥路往田埂和短市外分開，腳印在路口交錯。")
    } else if room_name.contains("北堤泥路") {
        Some("你踏上北堤泥路。河霧退到堤邊，挑擔腳印沿泥面排開，往田埂方向的車痕更窄。")
    } else if room_name.contains("田埂水口") {
        Some("你走到田埂水口。細水從木閘下淌過，遠處渡口聲被田間蛙鳴壓低。")
    } else if room_name.contains("上田小路") {
        Some("你沿上田小路停下。稻葉貼著田埂，遠處只剩斷續人影和水聲。")
    } else if room_name.contains("田區路口") {
        Some("你走到田區路口。上田小路在身後收窄，水田、木牌和泥腳印把路分向灌渠、農舍與更深田埂。")
    } else if room_name.contains("灌渠木閘") {
        Some("你停在灌渠木閘旁。木閘下的水聲貼著田埂流過，有人蹲著看水尺，竹耙斜靠在泥邊。")
    } else if room_name.contains("秧苗田邊") {
        Some("你走到秧苗田邊。淺綠秧苗貼著水面鋪開，彎身插秧的人影只露出背脊，蛙聲從水溝裡冒起。")
    } else if room_name.contains("水網入口") {
        Some(
            "你走到水網入口棧。田邊腳印在濕木棧上換成船路痕跡，引船人靠著篙影看水道，霧牌在木柱上輕晃。",
        )
    } else if room_name.contains("湖港木棧") {
        Some(
            "你踏上湖港木棧。濕繩圈和潮線貼在木板邊，候船的人給魚籃與蘆草捆讓路，幾處水聲在棧下碰到一起。",
        )
    } else if room_name.contains("短船泊位") {
        Some("你停在短船泊位旁。內水短船貼著木樁晃動，船戶扶著船舷催人站穩，船槳和篙影在水面交錯。")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some(
            "你走進魚貨分揀場。濕布鋪在魚籃下，分揀工把剛上岸的魚貨推向秤邊，腥味和短賣低語順著木棧散開。",
        )
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some(
            "你走到蘆草貨棚。蘆草捆靠著曬架排開，採集者拖來的濕痕還沒乾，棚下草屑被腳步帶到板路上。",
        )
    } else if room_name.contains("水倉") {
        Some(
            "你來到水倉前場。板車停在潮濕倉門外，倉管把魚貨與蘆草分往不同棚口，木地板下回著悶悶水聲。",
        )
    } else if room_name.contains("糧倉外秤") {
        Some(
            "你走到糧倉外秤。下游田埂路的車轍在雨棚下收住，秤手把糧袋扶上秤盤，秤桿影子壓在稻殼粉裡。",
        )
    } else if room_name.contains("倉門前場") {
        Some(
            "你走到倉門前場。高倉門只開著一線，守倉人照著秤單放行，封條和木栓在門側晃出很短的聲音。",
        )
    } else if room_name.contains("曬場邊") {
        Some("你來到曬場邊。曬席貼著地面鋪開，翻穀人推著木耙走過，穀粒乾響順著風貼到外秤那頭。")
    } else if room_name.contains("車行口") {
        Some("你走到車行口。車轍壓進倉區泥面，車夫扶著車轅等前車挪動，排隊牌旁的麻繩繃得很緊。")
    } else if room_name.contains("守倉棚") {
        Some(
            "你走進守倉棚下。冊簿攤在茶壺旁，倉管把秤單、封條和車次對在一起，倉門前的聲音到這裡便低下去。",
        )
    } else if room_name.contains("糧車泊位") {
        Some(
            "你來到糧車泊位。車架停在泊位邊，搬糧人從車尾接下糧袋，麻繩和木輪聲一前一後往守倉棚那頭傳。",
        )
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some(
            "你走到河口潮泥碼頭。退潮後的黑泥貼著木樁，碼頭工把河上糧袋和海船木箱分到兩側，纜繩聲混著鹹潮味。",
        )
    } else if room_name.contains("稅站棚") {
        Some(
            "你走進稅站棚下。貨單攤在木桌上，稅吏用印章壓住捲起的紙角，棚外候查的木箱和糧袋排得很緊。",
        )
    } else if room_name.contains("船行門面") {
        Some(
            "你來到船行門面。潮濕航線板掛在門下，船行人把船位、牌價和雇船消息一口氣遞向碼頭與貨場。",
        )
    } else if room_name.contains("潮線貨場") {
        Some("你走到潮線貨場。木箱墊在石條上避開潮印，搬貨人趁水位低時把貨往候泊線和稅站棚兩側推。")
    } else if room_name.contains("河海倉門") {
        Some(
            "你停在河海倉門前。雨布半遮著門口，倉管把河貨、海貨和放行貨單分開，門內木箱悶悶靠上牆。",
        )
    } else if room_name.contains("商船候泊線") {
        Some("你來到商船候泊線。等潮的船影停在外頭，船員把錨繩收成圈，候泊牌在風裡敲著木柱。")
    } else if room_name.contains("東南船塢口") {
        Some(
            "你走到東南船塢口。潮線貼著河海倉門外沿過來，船匠把半修的船板靠在入口旁，木屑和瀝青味從修船架那頭飄出。",
        )
    } else if room_name.contains("修船架") {
        Some(
            "你走到修船架下。翻起的船身被木架撐住，修船工沿著肋木敲打，瀝青桶和楔子把通路擠得很窄。",
        )
    } else if room_name.contains("補給倉") {
        Some(
            "你停在補給倉前。淡水桶、乾糧箱和油布包分成幾堆，倉管照著近海船期把份量往碼頭木階那頭推。",
        )
    } else if room_name.contains("碼頭木階") {
        Some("你來到碼頭木階。半濕木階沿著繩柱往下，船客扶著欄繩上下，補給倉的水桶聲從身後貼過來。")
    } else if room_name.contains("帆布曬場") {
        Some("你走進帆布曬場。舊帆鋪在繩架上，補帆人沿裂口穿針，鹹風把外貿碼頭那頭的船旗聲帶過來。")
    } else if room_name.contains("船戶街口") {
        Some(
            "你走到船戶街口。招工牌掛在飯攤旁，船戶在門口比劃工錢與船期，帆布曬場和碼頭木階的人聲從兩側交進來。",
        )
    } else if room_name.contains("外貿碼頭") {
        Some(
            "你走到外貿碼頭。港旗在海風裡扯直，船員把外地貨箱從吊繩下扶穩，海關前棚那邊已有人等著看貨單。",
        )
    } else if room_name.contains("海關前棚") {
        Some("你走進海關前棚下。濕貨單被壓在木桌上，海關吏按住封條，等翻譯桌那邊把貨名和合約說清。")
    } else if room_name.contains("商館街口") {
        Some("你走到商館街口。褪色招牌掛在門前，商館人打開貨樣匣，外語低語和碼頭船旗聲一同靠近。")
    } else if room_name.contains("遠洋倉門") {
        Some(
            "你停在遠洋倉門前。雨布半遮著門口，倉管把異地木箱分成已查與待查兩側，門內拖箱聲沉沉傳出。",
        )
    } else if room_name.contains("翻譯桌") {
        Some(
            "你走到翻譯桌旁。合約攤在鎮紙下，翻譯把陌生貨名轉成在地話，海關前棚的印章聲在旁邊等著落下。",
        )
    } else if room_name.contains("匯兌小屋") {
        Some("你走進匯兌小屋前。銅秤在桌邊輕晃，兌票人把商館街口報來的價碼寫進賬冊，錢票疊在手邊。")
    } else if room_name.contains("北灣避風碼頭") {
        Some(
            "你走到北灣避風碼頭。冷海水貼著木樁晃動，船戶把結霜纜繩重新繞緊，避風船在內側輕碰碼頭。",
        )
    } else if room_name.contains("港灣貨棚") {
        Some("你走進港灣貨棚前。鹽箱和濕木箱靠牆排開，搬魚人把剛卸下的魚籃推向冷霧魚市那頭。")
    } else if room_name.contains("漁網曬場") {
        Some("你走到漁網曬場。半乾濕網掛在木架上，補網人沿破口穿線，海風把避風碼頭的船聲帶過來。")
    } else if room_name.contains("冷霧魚市") {
        Some(
            "你來到冷霧魚市。木秤還沾著水珠，魚販把濕布下的魚貨攤開，買貨人的問價聲貼著霧氣低低傳開。",
        )
    } else if room_name.contains("船戶屋排") {
        Some("你走到船戶屋排。火盆煙氣貼著低屋簷，船戶家屬在排班板前換下濕木牌，屋後風聲很細。")
    } else if room_name.contains("北灣守望石階") {
        Some("你踏上北灣守望石階。霧牌和警鈴掛在風口，守望人扶著濕石欄，看向港外海面的顏色。")
    } else if room_name.contains("礦坑外棚") {
        Some("你走到礦坑外棚。濕支柱撐住低矮棚頂，吊燈光在坑口晃動，礦工把沾泥的鎬靠上木架。")
    } else if room_name.contains("洗礦溝") {
        Some("你走到洗礦溝旁。冷水沿木槽往下流，洗礦工把石籃壓進混濁水裡，碎料聲貼著坡面傳開。")
    } else if room_name.contains("粗料倉") {
        Some("你來到粗料倉前。濕礦袋堆在木牌下，倉管用炭線補上坑口和重量，車輪聲從西側舊道靠近。")
    } else if room_name.contains("工具修棚") {
        Some("你走進工具修棚前。裂柄鎬和車輪鐵圈掛在木架上，修具工把鬆鉚釘敲緊，火星短短一亮。")
    } else if room_name.contains("礦渣坡") {
        Some("你停在礦渣坡邊。灰黑碎石堆到安全線內，推車人把廢料倒下，幾道新滑坡痕沿坡面斜斜拉開。")
    } else if room_name.contains("舊道裝車點") {
        Some("你走到舊道裝車點。礦車停在磨亮車轍旁，車夫把繩索掛上車架，稱重木牌半埋在灰粉裡。")
    } else if room_name.contains("農舍前坪") {
        Some("你走進農舍前坪。鋤具靠在牆下，曬衣繩從低簷拉到木樁，屋前濕泥帶著田裡剛回來的腳印。")
    } else if room_name.contains("穀場小棚") {
        Some("你來到穀場小棚。幾袋稻穀半遮在棚下，木篩靠著棚柱，稻殼被腳步踩得乾乾作響。")
    } else if room_name.contains("田埂深處") {
        Some("你走進田埂深處。主要腳步聲被留在身後，濕草貼著褲腳，水鳥在更低的田面一閃而過。")
    } else if room_name.contains("挑擔小徑") {
        Some("你轉進挑擔小徑。草邊腳印密起來，短貨與水桶像是常避著渡口正面從這裡穿過。")
    } else if room_name.contains("短市外谷地路") {
        Some("你走到短市外谷地路。攤架聲被留在身後，挑擔的人沿泥路把小件貨送進送出。")
    } else if room_name.contains("南側谷地路") {
        Some("你走上南側谷地路。泥坡外的人聲低下去，車轍往田埂與下游方向拉長。")
    } else if room_name.contains("下游田埂路") {
        Some("你走到下游田埂路。車轍變淺，水田和濕草把渡口貨聲接成遠遠的回響。")
    } else if room_name.contains("渡口貨路") {
        Some("你走上渡口貨路。雨布與車輪印把倉棚前場的貨流牽向更遠的河邊。")
    } else if room_name.contains("貨路外彎") {
        Some("你來到貨路外彎。路面在草坡前拐開，貨車痕和腳印都被河風磨得散亂。")
    } else if room_name.contains("短市") {
        Some("你走進岸邊短市。價牌掛在貨攤邊晃動，有人掂著小包短貨，也有人壓低聲音打聽下一班渡船。")
    } else if room_name.contains("歇腳") {
        Some("你來到臨河歇腳處。長凳旁有人捧著茶碗暖手，幾只包袱靠在腳邊，談話聲被河風磨得很低。")
    } else if room_name.contains("踏板") {
        Some("你停在渡船踏板前。濕木板隨船身輕晃，有人扶著繩柱等船戶示意，腳步聲在水面上散開。")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("你踏上南岸泥坡。濕泥黏住鞋底，有人提著包袱往坡上走，也有人站在坡邊等船再度靠穩。")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some(
            "你走近船戶作業點。纜繩被一圈圈收在木樁邊，船邊的人壓低聲音分派活計，篙影在水面上晃過。",
        )
    } else if room_name.contains("倉棚") {
        Some("你來到倉棚前場。半開的棚門旁堆著麻袋與木箱，搬貨的人在泥地上停停走走，等下一趟渡船。")
    } else {
        None
    }
}

fn north_bank_entry_scene_flow(
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
) -> String {
    let (climate_key, climate_seed) = climate.unwrap_or(("clear", 0));
    let game_hour = (game_minute_of_day / 60) % 24;
    let ferry = FerryPhase::from_minute(game_minute_of_day);
    let seed = climate_seed
        .wrapping_add(game_minute_of_day as i64 * 11)
        .wrapping_add(unnamed_total as i64 * 17)
        .wrapping_add(ferry.seed_weight());
    let ferry_line = north_bank_entry_ferry_flow(ferry, seed);
    let market_line = north_bank_entry_market_flow(game_hour, climate_key, seed + 7);
    let crowd_line = north_bank_entry_crowd_flow(unnamed_total, ferry, seed + 13);
    format!("{ferry_line}\n{market_line}\n{crowd_line}")
}

fn north_bank_entry_ferry_flow(ferry: FerryPhase, seed: i64) -> &'static str {
    match ferry {
        FerryPhase::NorthDocked => pick_scene_variant(
            &[
                "船繩旁有人把濕繩重新繞上木樁，船身貼岸時輕輕碰了一下踏板。",
                "踏板口的木板被船腹碰得短短一響，船繩隨即被人往木樁上多繞半圈。",
            ],
            seed,
        ),
        FerryPhase::CrossingSouth => pick_scene_variant(
            &[
                "渡船剛離北岸，船繩在木樁旁鬆下一圈，水面還留著撐篙攪開的痕。",
                "踏板前的人把腳步收住，船尾往南岸滑開，只剩船繩滴水落回木樁邊。",
            ],
            seed,
        ),
        FerryPhase::SouthDocked => pick_scene_variant(
            &[
                "北岸木樁旁的船繩垂著，對岸有人影起落，踏板前只剩等船的低聲催問。",
                "渡船停在南岸那頭，這邊有人把船繩理直，又抬眼看對岸是否放人。",
            ],
            seed,
        ),
        FerryPhase::CrossingNorth => pick_scene_variant(
            &[
                "船繩旁的人先把繩尾理開，船頭從河霧裡近了一截，踏板前的人聲短短壓低。",
                "水面傳來撐篙聲，船繩旁有人把繩圈攤開，等著船腹重新貼岸。",
            ],
            seed,
        ),
    }
}

fn north_bank_entry_market_flow(game_hour: u32, climate_key: &str, seed: i64) -> &'static str {
    match climate_key {
        "rain" | "plum_rain" | "storm" => pick_scene_variant(
            &[
                "短市棚腳還在滴水，有人把價牌往乾處挪，討價聲被雨後濕氣壓得更低。",
                "短市湯攤熱氣混著雨味往上冒，木牌被指節敲了一下，聲音很快被水聲吞掉。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "短市價牌在霧裡只露出邊角，討價的人靠得更近，聲音貼著貨擔傳開。",
                "霧把短市那頭收得很窄，有人摸著木牌邊緣報價，又立刻壓低嗓音。",
            ],
            seed,
        ),
        "wind" => pick_scene_variant(
            &[
                "河風吹得短市棚布低低鼓起，價牌碰在木架上，討價聲斷成幾截。",
                "短市那頭有人按住晃動的木牌，另一手把零貨往棚柱內側推。",
            ],
            seed,
        ),
        _ => match scene_phase(game_hour) {
            ScenePhase::Dawn => pick_scene_variant(
                &[
                    "短市剛把價牌掛起，湯桶熱氣先冒出來，早到的人低聲問第一班船。",
                    "天光還薄，短市的人把木牌翻到外側，貨聲比人聲更早貼上岸邊。",
                ],
                seed,
            ),
            ScenePhase::Day => pick_scene_variant(
                &[
                    "短市那頭有人壓低聲音討價，木牌被指節敲得輕響。",
                    "日光照開短市價牌，有人把小件貨往外推半寸，又立刻被問價聲攔住。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "短市開始收暗，有人把價牌往燈邊移，討價聲貼著棚布低低延續。",
                    "暮色壓到棚口，貨聲變少，剩下幾句趕船前的短價來回咬著。",
                ],
                seed,
            ),
            ScenePhase::Night => pick_scene_variant(
                &[
                    "短市只剩幾圈黃暈，有人隔著燈影問價，木牌輕響後又沉進夜風裡。",
                    "夜裡短市的貨聲收得很低，湯攤火光貼著木牌晃，問船的人不再大聲說話。",
                ],
                seed,
            ),
        },
    }
}

fn north_bank_entry_crowd_flow(unnamed_total: usize, ferry: FerryPhase, seed: i64) -> &'static str {
    if unnamed_total >= 12 {
        match ferry {
            FerryPhase::NorthDocked => {
                "等船人群往踏板口擠出一條窄縫，包袱和貨擔又在船繩聲裡短短合攏。"
            }
            FerryPhase::CrossingSouth => {
                "踏板前的人群一起往後收，剛空出的濕木邊很快被包袱和貨擔佔住。"
            }
            FerryPhase::SouthDocked => {
                "人群壓在岸邊等對岸放船，幾個挑擔客換了肩，空隙立刻被後面腳步補上。"
            }
            FerryPhase::CrossingNorth => "等船人群往踏板口挪了一步，又停在河面傳來的水聲裡。",
        }
    } else if unnamed_total >= 5 {
        pick_scene_variant(
            &[
                "幾撥候渡的人把包袱挪到腳邊，視線在短市、踏板和河面之間來回。",
                "候渡的人從歇腳棚邊站起來，往踏板口看了一眼，又被旁邊貨擔擋住去路。",
            ],
            seed,
        )
    } else {
        pick_scene_variant(
            &[
                "零散候渡者靠著木樁等船，腳步聲在濕木板上顯得很清楚。",
                "岸邊只剩少數人守著包袱，河面一響，幾道視線便同時轉過去。",
            ],
            seed,
        )
    }
}

fn scene_reentry_log(room_name: &str) -> Option<&'static str> {
    if let Some(line) = game::final_map_reentry_log_for_room(room_name) {
        Some(line)
    } else if room_name.contains("北岸") || room_name.contains("等候") {
        Some(
            "你回到北岸渡口。踏板前的人群已重新合攏，包袱和腳步把方才空出的縫隙填回；有人又抬頭望向河面。",
        )
    } else if room_name.contains("中河谷地路口") {
        Some("你回到中河谷地路口。泥路上的腳印又被人踩亂，渡口聲和田埂水聲在這裡重新交在一起。")
    } else if room_name.contains("北堤泥路") {
        Some("你又踏上北堤泥路。堤邊河霧退了些，挑擔腳印和窄車痕仍沿泥面往前排開。")
    } else if room_name.contains("田埂水口") {
        Some("你回到田埂水口。木閘下的細水仍在淌，遠處渡口聲被田間聲響壓得更低。")
    } else if room_name.contains("上田小路") {
        Some("你回到上田小路。田埂旁的稻葉擦過腳邊，遠處人影仍只露出斷續幾點。")
    } else if room_name.contains("田區路口") {
        Some("你回到田區路口。泥腳印又被踩亂，灌渠水聲、農舍低語和深處田埂的濕草味重新交在一起。")
    } else if room_name.contains("灌渠木閘") {
        Some("你回到灌渠木閘。水尺旁換了看水的人，木閘仍壓著細流，把田區路口的腳步聲切成幾段。")
    } else if room_name.contains("秧苗田邊") {
        Some("你回到秧苗田邊。秧盤旁添了新腳印，彎身的人影換了位置，蛙聲仍從水溝裡一陣陣冒出。")
    } else if room_name.contains("水網入口") {
        Some(
            "你回到水網入口棧。棧板上的濕腳印又多了一層，霧牌仍在木柱上輕晃，引船人的視線重新落回水道。",
        )
    } else if room_name.contains("湖港木棧") {
        Some(
            "你回到湖港木棧。候船的人群換了站位，濕繩圈旁多了些滴水痕，泊位和分揀場的聲音又在腳下聚攏。",
        )
    } else if room_name.contains("短船泊位") {
        Some("你回到短船泊位。剛才靠著木樁的船身換了角度，船戶把船槳收近，等下一撥人踩穩棧板。")
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        Some("你回到魚貨分揀場。魚籃位置已被推開一些，濕布邊新添水印，分揀工把低聲短賣接著說下去。")
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        Some(
            "你回到蘆草貨棚。曬架旁的蘆草捆換了靠法，棚下濕痕被腳步拖散，草屑仍黏在通往水倉的板路上。",
        )
    } else if room_name.contains("水倉") {
        Some(
            "你回到水倉前場。倉門外的板車被挪出新角度，倉管仍在棚口分派貨流，木地板下的水聲沒有停。",
        )
    } else if room_name.contains("糧倉外秤") {
        Some("你回到糧倉外秤。秤盤旁又添了一圈稻殼粉，秤手把剛稱過的糧袋指向倉門前場。")
    } else if room_name.contains("倉門前場") {
        Some("你回到倉門前場。半開的倉門仍壓著陰影，守倉人換了站位，封條在門側輕輕晃著。")
    } else if room_name.contains("曬場邊") {
        Some("你回到曬場邊。曬席上的穀粒被翻開一道新痕，木耙聲又從外秤方向慢慢接回來。")
    } else if room_name.contains("車行口") {
        Some("你回到車行口。車轍裡的泥水被新輪印壓亮，車夫把排隊牌旁的空隙又往前讓了一點。")
    } else if room_name.contains("守倉棚") {
        Some("你回到守倉棚。冊簿頁角被壓住，倉管仍在低聲核對，門前的糧袋聲被棚柱分成幾路。")
    } else if room_name.contains("糧車泊位") {
        Some("你回到糧車泊位。泊位邊的麻繩被收短一截，搬糧人換肩接袋，木輪聲仍沉沉壓在濕土上。")
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        Some("你回到河口潮泥碼頭。潮泥上又多了一層腳印，碼頭工把剛靠岸的貨聲分向稅站棚和船行門面。")
    } else if room_name.contains("稅站棚") {
        Some("你回到稅站棚。桌上貨單換了幾張，稅吏仍壓著印章，棚外等待查驗的人群往前挪了一截。")
    } else if room_name.contains("船行門面") {
        Some("你回到船行門面。航線板還在門下潮濕發暗，船行人把剛問到的船位重新報給等候的人。")
    } else if room_name.contains("潮線貨場") {
        Some("你回到潮線貨場。潮印旁新添了木箱拖痕，搬貨人趁水線未漲又把箱包往候泊線推近。")
    } else if room_name.contains("河海倉門") {
        Some("你回到河海倉門。雨布被掀開又落下，倉管把新放行的貨單壓到封牌旁。")
    } else if room_name.contains("商船候泊線") {
        Some("你回到商船候泊線。錨繩被收得更緊，船員仍隔著潮水看稅站棚那頭是否放行。")
    } else if room_name.contains("東南船塢口") {
        Some("你回到東南船塢口。入口旁又多了幾片木屑，船匠把半修船板挪出一條通往補給倉的空隙。")
    } else if room_name.contains("修船架") {
        Some("你回到修船架。木槌聲換了節奏，修船工仍沿著肋木檢查，瀝青味貼著木架沒有散。")
    } else if room_name.contains("補給倉") {
        Some("你回到補給倉。淡水桶少了一排，倉管把新搬出的乾糧箱推向碼頭木階。")
    } else if room_name.contains("碼頭木階") {
        Some("你回到碼頭木階。木階上的水痕被踩得更亮，船客仍扶著繩柱等下一次上下船空隙。")
    } else if room_name.contains("帆布曬場") {
        Some("你回到帆布曬場。舊帆被翻出新的補痕，補帆人壓住布角，針線架在風裡輕輕碰響。")
    } else if room_name.contains("船戶街口") {
        Some("你回到船戶街口。飯攤鍋聲仍在，招工牌下換了幾個等活的人，船戶把船期重新說了一遍。")
    } else if room_name.contains("外貿碼頭") {
        Some("你回到外貿碼頭。吊繩又放下一只外地貨箱，船員壓住箱角，港旗聲重新貼回耳邊。")
    } else if room_name.contains("海關前棚") {
        Some("你回到海關前棚。木桌上的貨單換了一疊，海關吏把封條重新壓平，棚外貨箱往前挪了一截。")
    } else if room_name.contains("商館街口") {
        Some("你回到商館街口。貨樣匣又打開一次，商館人把新問到的價碼送往匯兌小屋。")
    } else if room_name.contains("遠洋倉門") {
        Some("你回到遠洋倉門。雨布被拖箱聲頂起又落下，倉管把新放行的箱包推進門內陰影。")
    } else if room_name.contains("翻譯桌") {
        Some("你回到翻譯桌旁。合約邊角被重新壓住，翻譯把剛才沒說清的貨名再轉了一遍。")
    } else if room_name.contains("匯兌小屋") {
        Some("你回到匯兌小屋。銅秤還在輕晃，兌票人把新價碼添進賬冊，錢票聲很快低下去。")
    } else if room_name.contains("北灣避風碼頭") {
        Some("你回到北灣避風碼頭。結霜纜繩又被拉緊一截，避風船貼著木樁換了角度。")
    } else if room_name.contains("港灣貨棚") {
        Some("你回到港灣貨棚。濕木箱旁多了一道水痕，搬魚人把新送到的魚籃推近棚門。")
    } else if room_name.contains("漁網曬場") {
        Some("你回到漁網曬場。半乾濕網被翻出新補痕，補網人把木架往風少的地方挪了一點。")
    } else if room_name.contains("冷霧魚市") {
        Some("你回到冷霧魚市。木秤旁換了一籃魚貨，魚販把問價聲重新壓回濕布邊。")
    } else if room_name.contains("船戶屋排") {
        Some("你回到船戶屋排。火盆煙氣還貼著屋簷，排班板上的濕木牌換了一格位置。")
    } else if room_name.contains("北灣守望石階") {
        Some("你回到北灣守望石階。霧牌仍在風口晃動，守望人把目光從港外收回石階下。")
    } else if room_name.contains("礦坑外棚") {
        Some("你回到礦坑外棚。吊燈光還貼著坑口晃動，換班礦工把下一把鎬從木架上取下。")
    } else if room_name.contains("洗礦溝") {
        Some("你回到洗礦溝旁。木槽裡的冷水又濁了一層，洗礦工把石籃重新壓進水面。")
    } else if room_name.contains("粗料倉") {
        Some("你回到粗料倉前。濕礦袋換了位置，倉管在木牌上補了一道新的炭線。")
    } else if room_name.contains("工具修棚") {
        Some("你回到工具修棚前。敲鐵聲短短停住，修具工把修好的車輪推回棚口。")
    } else if room_name.contains("礦渣坡") {
        Some("你回到礦渣坡邊。新碎石沿滑坡痕停住，推車人把車把重新壓回掌心。")
    } else if room_name.contains("舊道裝車點") {
        Some("你回到舊道裝車點。礦車又往舊道口挪了一截，稱重木牌上的灰粉被手背抹開。")
    } else if room_name.contains("農舍前坪") {
        Some("你回到農舍前坪。牆下鋤具還在，曬衣繩輕晃，屋前濕泥把田裡回來的腳印留得更深。")
    } else if room_name.contains("穀場小棚") {
        Some("你回到穀場小棚。稻袋被挪開一角，木篩仍靠著棚柱，稻殼在腳下發出短短乾響。")
    } else if room_name.contains("田埂深處") {
        Some("你回到田埂深處。濕草重新貼上腳邊，遠處田區路口的人聲只剩一點斷續回音。")
    } else if room_name.contains("挑擔小徑") {
        Some("你又轉回挑擔小徑。草邊新腳印接上舊痕，像是剛有挑擔人從這裡避過渡口正面。")
    } else if room_name.contains("短市外谷地路") {
        Some("你回到短市外谷地路。攤架聲從身後斷續傳來，泥路上的小件貨腳印又密了一層。")
    } else if room_name.contains("南側谷地路") {
        Some("你回到南側谷地路。車轍裡積著薄水，泥坡那邊的渡船木聲仍遠遠傳來。")
    } else if room_name.contains("下游田埂路") {
        Some("你回到下游田埂路。濕草邊的水聲沒有停，渡口貨聲只剩遠遠一線。")
    } else if room_name.contains("渡口貨路") {
        Some("你又走上渡口貨路。雨布味和車輪印還留在路面，倉棚那邊的搬貨聲已被拉遠。")
    } else if room_name.contains("貨路外彎") {
        Some("你回到貨路外彎。草坡前的路面仍被車輪壓亮，貨流像是從這裡拐向更遠處。")
    } else if room_name.contains("短市") {
        Some("你又走回岸邊短市。價牌仍在河風裡晃，方才談價的人換了位置，打聽渡船的低語還沒散。")
    } else if room_name.contains("歇腳") {
        Some("你回到臨河歇腳處。長凳旁的茶碗換了幾隻手，包袱仍壓在腳邊，談話聲貼著河風低低延續。")
    } else if room_name.contains("踏板") {
        Some("你又停在渡船踏板前。濕木板仍隨水聲輕晃，剛才讓開的人群又把腳步收回繩柱旁。")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("你回到南岸泥坡。濕泥上新舊腳印疊在一起，坡邊等船的人換了站姿，目光仍貼著河面。")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("你又走近船戶作業點。木樁旁的纜繩少了一圈，船邊的人把剛才沒說完的活計接著分派下去。")
    } else if room_name.contains("倉棚") {
        Some("你回到倉棚前場。棚門旁的麻袋被挪出一道新縫，木箱拖痕在泥地上接上剛才那一串腳印。")
    } else {
        None
    }
}

fn scene_route_reentry_logs(room_name: &str) -> Vec<(&'static str, &'static str)> {
    if room_name.contains("北岸") || room_name.contains("等候") {
        vec![
            (
                "岸邊短市",
                "你從短市那頭回到北岸渡口。身後的價牌聲被河風拉遠，踏板前的人群已重新合攏；有人把剛買的短貨塞回包袱裡，又抬頭看船。",
            ),
            (
                "渡船踏板",
                "你從渡船踏板退回北岸渡口。濕木板的晃動還留在腳下，候渡的人群順著繩柱旁的縫隙重新合攏。",
            ),
            (
                "臨河歇腳處",
                "你從歇腳處回到北岸渡口。茶氣和低談被留在身後，踏板前的人聲又把河面動靜壓近了些。",
            ),
            (
                "中河谷地路口",
                "你從谷地路口回到北岸渡口。泥路腳印被留在身後，踏板前的船繩聲又把候渡人群短短壓靜。",
            ),
        ]
    } else if room_name.contains("中河谷地路口") {
        vec![
            (
                "北岸渡口",
                "你從北岸渡口走到中河谷地路口。候渡的人聲退到身後，泥路在腳下分向田埂和短市外。",
            ),
            (
                "北堤泥路",
                "你從北堤泥路回到中河谷地路口。堤邊河霧被留在身後，渡口人聲又順著泥路貼近。",
            ),
            (
                "短市外谷地路",
                "你從短市外谷地路回到中河谷地路口。攤架聲斷在路旁，田埂水聲重新接了上來。",
            ),
        ]
    } else if room_name.contains("北堤泥路") {
        vec![
            (
                "中河谷地路口",
                "你從中河谷地路口踏上北堤泥路。渡口人聲被堤邊河霧壓低，挑擔腳印沿泥面往前排開。",
            ),
            (
                "田埂水口",
                "你從田埂水口回到北堤泥路。木閘水聲退到田間，堤邊泥路重新接住腳下車痕。",
            ),
            (
                "挑擔小徑",
                "你從挑擔小徑折回北堤泥路。草邊密腳印斷在岔口，河堤車痕又把方向拉直。",
            ),
        ]
    } else if room_name.contains("田埂水口") {
        vec![
            (
                "北堤泥路",
                "你從北堤泥路走到田埂水口。堤邊車痕在木閘旁變窄，細水聲把渡口人聲壓到遠處。",
            ),
            (
                "上田小路",
                "你從上田小路回到田埂水口。稻葉聲被留在身後，木閘下的細水又接回腳邊。",
            ),
        ]
    } else if room_name.contains("上田小路") {
        vec![
            (
                "田埂水口",
                "你從田埂水口走上上田小路。木閘水聲退到身後，稻葉貼著田埂把路引向更深的田區。",
            ),
            (
                "田區路口",
                "你從田區路口退回上田小路。農舍和灌渠被留在身後，腳下田埂重新收成一條窄路。",
            ),
        ]
    } else if room_name.contains("田區路口") {
        vec![
            (
                "上田小路",
                "你從上田小路走到田區路口。稻葉旁的窄路忽然分開，水田、木牌和泥腳印把人帶向不同活計。",
            ),
            (
                "灌渠木閘",
                "你從灌渠木閘回到田區路口。木閘水聲貼著腳後跟退開，田邊人聲又從幾條小路湊攏。",
            ),
            (
                "農舍前坪",
                "你從農舍前坪回到田區路口。屋前濕泥和曬衣繩被留在身後，田埂上的水聲重新接住腳步。",
            ),
            (
                "田埂深處",
                "你從田埂深處回到田區路口。濕草退到身後，木牌旁的泥腳印和人聲又變得清楚。",
            ),
        ]
    } else if room_name.contains("灌渠木閘") {
        vec![
            (
                "田區路口",
                "你從田區路口走到灌渠木閘。分岔路的人聲被水聲切開，木閘邊有人低頭看著水尺。",
            ),
            (
                "秧苗田邊",
                "你從秧苗田邊回到灌渠木閘。淺綠秧苗退成一片水光，木閘下的細流重新壓到耳邊。",
            ),
        ]
    } else if room_name.contains("秧苗田邊") {
        vec![
            (
                "灌渠木閘",
                "你從灌渠木閘走到秧苗田邊。水尺和木閘被留在身後，淺綠秧苗沿著田面一路鋪開。",
            ),
            (
                "水網入口棧",
                "你從水網入口棧退回秧苗田邊。船路的濕木聲被留在身後，淺綠秧苗又把腳步收回田面。",
            ),
        ]
    } else if room_name.contains("水網入口") {
        vec![
            (
                "秧苗田邊",
                "你從秧苗田邊走到水網入口棧。田面蛙聲被濕木棧接住，田邊腳印在霧牌下轉成水路方向。",
            ),
            (
                "湖港木棧",
                "你從湖港木棧回到水網入口棧。候船聲被拉到身後，引船人重新把目光放回半開的水道。",
            ),
            (
                "短船泊位",
                "你從短船泊位回到水網入口棧。船身晃動退到身後，篙影和霧牌又把水道入口壓近眼前。",
            ),
        ]
    } else if room_name.contains("湖港木棧") {
        vec![
            (
                "水網入口棧",
                "你從水網入口棧踏上湖港木棧。田邊腳印斷在身後，濕繩圈、潮線和候船低語在棧板上接起來。",
            ),
            (
                "魚貨分揀場",
                "你從魚貨分揀場回到湖港木棧。腥味和秤聲被留在身後，候船的人又把木棧邊的空隙讓出來。",
            ),
            (
                "蘆草貨棚",
                "你從蘆草貨棚回到湖港木棧。草屑黏在腳下，濕繩圈旁的候船聲重新壓過棚下水痕。",
            ),
        ]
    } else if room_name.contains("短船泊位") {
        vec![
            (
                "水網入口棧",
                "你從水網入口棧走到短船泊位。霧牌落在身後，短船貼著木樁輕晃，船戶伸手示意人踩穩。",
            ),
            (
                "魚貨分揀場",
                "你從魚貨分揀場回到短船泊位。魚籃和秤聲退開，船槳貼著船舷把腳步引回水邊。",
            ),
        ]
    } else if room_name.contains("魚貨") || room_name.contains("分揀") {
        vec![
            (
                "湖港木棧",
                "你從湖港木棧走進魚貨分揀場。候船低語被腥味和秤聲蓋過，濕布旁的魚籃正被推向棧邊。",
            ),
            (
                "短船泊位",
                "你從短船泊位走進魚貨分揀場。船槳聲斷在身後，剛上岸的魚貨沿濕布被分往幾處籃口。",
            ),
            (
                "水倉前場",
                "你從水倉前場回到魚貨分揀場。倉門下的悶水聲退遠，秤邊短賣又把魚籃重新聚起來。",
            ),
        ]
    } else if room_name.contains("蘆草") || room_name.contains("蘆棚") {
        vec![
            (
                "湖港木棧",
                "你從湖港木棧走到蘆草貨棚。候船的人聲落在身後，草屑和濕痕把板路引進棚下陰影。",
            ),
            (
                "水倉前場",
                "你從水倉前場回到蘆草貨棚。倉門外的板車聲被留住，曬架旁的蘆草捆又壓回眼前。",
            ),
        ]
    } else if room_name.contains("水倉") {
        vec![
            (
                "魚貨分揀場",
                "你從魚貨分揀場走到水倉前場。腥味和短賣低語退到背後，板車水印把貨流接到倉門口。",
            ),
            (
                "蘆草貨棚",
                "你從蘆草貨棚走到水倉前場。棚下草屑黏著腳步，倉管在門口把蘆草和魚貨分開收進去。",
            ),
        ]
    } else if room_name.contains("下游田埂路") {
        vec![
            (
                "南側谷地路",
                "你從南側谷地路走到下游田埂路。泥坡外的人聲退遠，車轍和濕草把渡口貨流往更低處帶去。",
            ),
            (
                "糧倉外秤",
                "你從糧倉外秤退回下游田埂路。秤桿聲被留在身後，車輪泥水重新接回田埂邊的濕草。",
            ),
        ]
    } else if room_name.contains("糧倉外秤") {
        vec![
            (
                "下游田埂路",
                "你從下游田埂路走到糧倉外秤。田埂濕草被車轍切斷，秤盤、糧袋和雨棚影子把路口收成倉區入口。",
            ),
            (
                "倉門前場",
                "你從倉門前場回到糧倉外秤。倉門陰影退到身後，秤桿和稻殼粉又把糧袋流向壓回眼前。",
            ),
            (
                "曬場邊",
                "你從曬場邊回到糧倉外秤。木耙乾響被風吹散，秤手旁的糧袋聲重新接住腳步。",
            ),
        ]
    } else if room_name.contains("倉門前場") {
        vec![
            (
                "糧倉外秤",
                "你從糧倉外秤走到倉門前場。秤桿聲留在雨棚下，守倉人照著秤單把半開的倉門再讓出一線。",
            ),
            (
                "車行口",
                "你從車行口回到倉門前場。車輪和麻繩聲收在背後，倉門旁的封條又貼近眼前。",
            ),
            (
                "守倉棚",
                "你從守倉棚回到倉門前場。冊簿低語被留在棚下，半開的倉門重新接住糧袋腳步。",
            ),
        ]
    } else if room_name.contains("曬場邊") {
        vec![
            (
                "糧倉外秤",
                "你從糧倉外秤走到曬場邊。秤盤聲退到雨棚下，曬席和木耙把穀粒乾響鋪到腳邊。",
            ),
            (
                "車行口",
                "你從車行口回到曬場邊。車轍泥水被留在身後，曬席邊的稻殼粉重新黏上鞋面。",
            ),
        ]
    } else if room_name.contains("車行口") {
        vec![
            (
                "倉門前場",
                "你從倉門前場走到車行口。倉門陰影被車轍拉開，車夫扶著車轅等前車慢慢挪動。",
            ),
            (
                "曬場邊",
                "你從曬場邊走到車行口。曬席乾響落在身後，排隊牌旁的車輪泥水把路面壓亮。",
            ),
            (
                "糧車泊位",
                "你從糧車泊位回到車行口。卸糧聲退到背後，排隊牌又把車流收回倉區入口。",
            ),
        ]
    } else if room_name.contains("守倉棚") {
        vec![
            (
                "倉門前場",
                "你從倉門前場走到守倉棚。封條和木栓聲落在身後，冊簿、茶壺和值牌把貨流分成幾條。",
            ),
            (
                "糧車泊位",
                "你從糧車泊位回到守倉棚。木輪和麻繩聲退到棚外，倉管把新到的車次壓進冊簿裡。",
            ),
            (
                "河口潮泥碼頭",
                "你從河口潮泥碼頭回到守倉棚。潮泥氣味被留在外頭，守倉棚下重新只剩秤單和封條低語。",
            ),
        ]
    } else if room_name.contains("糧車泊位") {
        vec![
            (
                "車行口",
                "你從車行口走到糧車泊位。排隊牌退到身後，搬糧人把車尾的糧袋接上肩頭。",
            ),
            (
                "守倉棚",
                "你從守倉棚走到糧車泊位。冊簿聲斷在棚柱旁，木輪和麻繩把路帶回卸糧車架。",
            ),
        ]
    } else if room_name.contains("河口潮泥") || room_name.contains("潮泥碼頭") {
        vec![
            (
                "守倉棚",
                "你從守倉棚走到河口潮泥碼頭。秤單和封條聲退到身後，潮泥、纜繩和靠船手勢把糧路接上海口。",
            ),
            (
                "稅站棚",
                "你從稅站棚回到河口潮泥碼頭。印章聲留在棚下，碼頭工把候查貨流重新分到潮泥邊。",
            ),
            (
                "船行門面",
                "你從船行門面回到河口潮泥碼頭。航線板聲被鹹風拉遠，纜繩和木樁重新壓住河海交界的腳步。",
            ),
        ]
    } else if room_name.contains("稅站棚") {
        vec![
            (
                "河口潮泥碼頭",
                "你從河口潮泥碼頭走進稅站棚。潮泥氣味被木桌擋在外頭，貨單、印章和候查箱包把人聲壓低。",
            ),
            (
                "潮線貨場",
                "你從潮線貨場回到稅站棚。木箱拖痕斷在棚外，稅吏把濕了角的貨單重新攤平。",
            ),
            (
                "河海倉門",
                "你從河海倉門回到稅站棚。雨布和封牌聲退到身後，印章又把放行與候查分成兩路。",
            ),
        ]
    } else if room_name.contains("船行門面") {
        vec![
            (
                "河口潮泥碼頭",
                "你從河口潮泥碼頭走到船行門面。纜繩聲落在身後，潮濕航線板和牌價把等船的人聚到門下。",
            ),
            (
                "潮線貨場",
                "你從潮線貨場回到船行門面。木箱擦過石條的聲音退開，船行人把候泊消息接著報下去。",
            ),
        ]
    } else if room_name.contains("潮線貨場") {
        vec![
            (
                "稅站棚",
                "你從稅站棚走到潮線貨場。印章聲被留在棚下，木箱和潮印把查驗後的貨流推向候泊線。",
            ),
            (
                "船行門面",
                "你從船行門面走到潮線貨場。航線板低響斷在身後，搬貨人趁潮水未漲把箱包墊上石條。",
            ),
            (
                "商船候泊線",
                "你從商船候泊線回到潮線貨場。錨繩聲退到外頭，木箱拖痕又接回潮印旁的濕地。",
            ),
        ]
    } else if room_name.contains("河海倉門") {
        vec![
            (
                "稅站棚",
                "你從稅站棚走到河海倉門。放行貨單被帶到雨布下，倉管把河貨和海貨重新分門。",
            ),
            (
                "商船候泊線",
                "你從商船候泊線回到河海倉門。候泊牌聲被留在潮線外，封牌和雨布重新接住入倉貨流。",
            ),
            (
                "東南船塢口",
                "你從東南船塢口回到河海倉門。修船木屑味被留在外頭，倉門前又只剩雨布和封牌低響。",
            ),
        ]
    } else if room_name.contains("商船候泊線") {
        vec![
            (
                "潮線貨場",
                "你從潮線貨場走到商船候泊線。木箱和潮印退到身後，等潮船影與錨繩把腳步帶到外線。",
            ),
            (
                "河海倉門",
                "你從河海倉門走到商船候泊線。雨布聲落在倉門後，船員隔著潮水等稅站棚那頭放行。",
            ),
        ]
    } else if room_name.contains("東南船塢口") {
        vec![
            (
                "河海倉門",
                "你從河海倉門走到東南船塢口。雨布和封牌退到身後，木屑、瀝青味和半修船板把腳步接進船港。",
            ),
            (
                "修船架",
                "你從修船架回到東南船塢口。木槌聲留在架下，入口旁的潮線和補給倉腳步重新分開。",
            ),
            (
                "補給倉",
                "你從補給倉回到東南船塢口。淡水桶聲退到身後，船匠把通往修船架的木板挪開一點。",
            ),
        ]
    } else if room_name.contains("修船架") {
        vec![
            (
                "東南船塢口",
                "你從東南船塢口走到修船架。船塢入口的潮線退在身後，木架和瀝青桶把翻起的船身托到眼前。",
            ),
            (
                "碼頭木階",
                "你從碼頭木階回到修船架。上下船的腳步退到下方，木槌聲和楔子聲重新壓住耳邊。",
            ),
            (
                "帆布曬場",
                "你從帆布曬場回到修船架。舊帆布聲被留在風裡，修船工又把目光放回船肋與木楔。",
            ),
        ]
    } else if room_name.contains("補給倉") {
        vec![
            (
                "東南船塢口",
                "你從東南船塢口走到補給倉。船板木屑退到身後，淡水桶和乾糧箱把路口收成裝船前場。",
            ),
            (
                "碼頭木階",
                "你從碼頭木階回到補給倉。半濕木階聲斷在外頭，倉管把下一批水桶重新排到門邊。",
            ),
        ]
    } else if room_name.contains("碼頭木階") {
        vec![
            (
                "修船架",
                "你從修船架走到碼頭木階。木槌聲留在船身下，半濕木階和繩柱把腳步拉向近海船口。",
            ),
            (
                "補給倉",
                "你從補給倉走到碼頭木階。淡水桶和乾糧箱被帶到身後，船客扶著繩柱讓出一條下階的空隙。",
            ),
            (
                "船戶街口",
                "你從船戶街口回到碼頭木階。飯攤鍋聲退到巷口，木階上的水痕重新接住上下船腳步。",
            ),
        ]
    } else if room_name.contains("帆布曬場") {
        vec![
            (
                "修船架",
                "你從修船架走到帆布曬場。瀝青味被風吹散，舊帆和針線架把修補活計攤到陽面。",
            ),
            (
                "船戶街口",
                "你從船戶街口回到帆布曬場。招工牌和飯香退到身後，補帆人壓住被風吹起的舊帆。",
            ),
            (
                "外貿碼頭",
                "你從外貿碼頭回到帆布曬場。外地貨箱與港旗聲退到東側，舊帆布重新攤回眼前。",
            ),
        ]
    } else if room_name.contains("船戶街口") {
        vec![
            (
                "碼頭木階",
                "你從碼頭木階走到船戶街口。繩柱和水痕退到身後，飯攤、招工牌和船期低語把人聲接住。",
            ),
            (
                "帆布曬場",
                "你從帆布曬場走到船戶街口。針線架聲被留在風裡，招工牌下的船戶把工錢重新比給等活的人。",
            ),
        ]
    } else if room_name.contains("外貿碼頭") {
        vec![
            (
                "帆布曬場",
                "你從帆布曬場走到外貿碼頭。舊帆和針線架被留在身後，港旗、吊繩和外地貨箱把海風接到眼前。",
            ),
            (
                "海關前棚",
                "你從海關前棚回到外貿碼頭。貨單與封條聲退到棚下，船員重新把吊繩下的貨箱扶穩。",
            ),
            (
                "商館街口",
                "你從商館街口回到外貿碼頭。貨樣匣聲被留在門口，港旗和甲板滑輪重新壓住耳邊。",
            ),
        ]
    } else if room_name.contains("海關前棚") {
        vec![
            (
                "外貿碼頭",
                "你從外貿碼頭走進海關前棚。港旗聲退到身後，濕貨單、封條和待查貨箱把腳步收進棚下。",
            ),
            (
                "遠洋倉門",
                "你從遠洋倉門回到海關前棚。雨布和拖箱聲落在身後，海關吏把新放行的貨單壓回桌上。",
            ),
            (
                "翻譯桌",
                "你從翻譯桌回到海關前棚。合約上的陌生貨名剛被說清，印章和封條立刻接住下一步。",
            ),
        ]
    } else if room_name.contains("商館街口") {
        vec![
            (
                "外貿碼頭",
                "你從外貿碼頭走到商館街口。吊繩聲落在海風裡，貨樣匣和褪色招牌把人聲換成更低的議價。",
            ),
            (
                "遠洋倉門",
                "你從遠洋倉門回到商館街口。雨布與木箱漆印退到身後，商館人又把價碼送往匯兌小屋。",
            ),
        ]
    } else if room_name.contains("遠洋倉門") {
        vec![
            (
                "海關前棚",
                "你從海關前棚走到遠洋倉門。封條聲留在棚下，已查和待查的異地箱把門前分成兩側。",
            ),
            (
                "商館街口",
                "你從商館街口走到遠洋倉門。貨樣匣聲退到街口，雨布下的木箱漆印和拖箱聲重新貼近。",
            ),
            (
                "匯兌小屋",
                "你從匯兌小屋回到遠洋倉門。銅秤聲留在身後，倉管照著新價碼把箱包推進門內陰影。",
            ),
        ]
    } else if room_name.contains("翻譯桌") {
        vec![
            (
                "海關前棚",
                "你從海關前棚走到翻譯桌。封條與印章暫時停在身後，合約上的兩種文字把貨名重新攤開。",
            ),
            (
                "匯兌小屋",
                "你從匯兌小屋回到翻譯桌。銅秤聲低下去，翻譯把錢票價碼和合約名目重新對在一起。",
            ),
        ]
    } else if room_name.contains("匯兌小屋") {
        vec![
            (
                "遠洋倉門",
                "你從遠洋倉門走到匯兌小屋。雨布和拖箱聲退到身後，銅秤、賬冊和錢票把貨名換成價碼。",
            ),
            (
                "翻譯桌",
                "你從翻譯桌走到匯兌小屋。合約文字留在桌上，兌票人把剛說清的價碼壓進賬冊欄裡。",
            ),
        ]
    } else if room_name.contains("北灣避風碼頭") {
        vec![
            (
                "農舍前坪",
                "你從農舍前坪走到北灣避風碼頭。田邊濕泥退到身後，冷海水、結霜纜繩和避風船把路接到港邊。",
            ),
            (
                "港灣貨棚",
                "你從港灣貨棚回到北灣避風碼頭。鹽箱和濕木箱聲落在棚下，船戶重新把纜繩收緊。",
            ),
            (
                "漁網曬場",
                "你從漁網曬場回到北灣避風碼頭。濕網木架聲被海風帶遠，避風船的碰岸聲重新貼近腳邊。",
            ),
        ]
    } else if room_name.contains("港灣貨棚") {
        vec![
            (
                "北灣避風碼頭",
                "你從北灣避風碼頭走進港灣貨棚。結霜纜繩聲退在身後，鹽箱和濕魚籃把腳步收進棚門。",
            ),
            (
                "冷霧魚市",
                "你從冷霧魚市回到港灣貨棚。木秤和問價聲落在霧裡，搬魚人把下一籃魚貨推回棚邊。",
            ),
            (
                "船戶屋排",
                "你從船戶屋排回到港灣貨棚。火盆煙氣斷在屋簷後，濕木箱和鹽箱重新靠近。",
            ),
        ]
    } else if room_name.contains("漁網曬場") {
        vec![
            (
                "北灣避風碼頭",
                "你從北灣避風碼頭走到漁網曬場。避風船聲退到木樁那頭，半乾濕網和木架把海風截成碎響。",
            ),
            (
                "冷霧魚市",
                "你從冷霧魚市回到漁網曬場。魚市問價聲隔著霧氣退開，補網人把新破口壓回木架上。",
            ),
        ]
    } else if room_name.contains("冷霧魚市") {
        vec![
            (
                "港灣貨棚",
                "你從港灣貨棚走到冷霧魚市。鹽箱和濕木箱留在棚下，木秤、濕布和問價聲把魚貨接到眼前。",
            ),
            (
                "漁網曬場",
                "你從漁網曬場走到冷霧魚市。濕網木架聲留在風裡，魚販把濕布掀開一角等人看貨。",
            ),
            (
                "北灣守望石階",
                "你從北灣守望石階下到冷霧魚市。霧牌聲還在身後，魚販的秤錘聲很快把人聲重新聚起來。",
            ),
        ]
    } else if room_name.contains("船戶屋排") {
        vec![
            (
                "港灣貨棚",
                "你從港灣貨棚走到船戶屋排。濕木箱聲落在棚門後，火盆煙氣和排班板把港邊活計收進屋簷下。",
            ),
            (
                "北灣守望石階",
                "你從北灣守望石階回到船戶屋排。霧牌和警鈴聲退到風口，排班板前的低語重新接住腳步。",
            ),
        ]
    } else if room_name.contains("北灣守望石階") {
        vec![
            (
                "冷霧魚市",
                "你從冷霧魚市踏上北灣守望石階。木秤和問價聲被甩在霧裡，警鈴與霧牌聲在風口變清。",
            ),
            (
                "船戶屋排",
                "你從船戶屋排走上北灣守望石階。火盆煙氣貼在身後，守望人扶著石欄看向港外海色。",
            ),
        ]
    } else if room_name.contains("礦坑外棚") {
        vec![
            (
                "田埂深處",
                "你從田埂深處走到礦坑外棚。濕草退到身後，山壁陰影、吊燈光和濕支柱把路收進坑口前。",
            ),
            (
                "洗礦溝",
                "你從洗礦溝回到礦坑外棚。冷水槽聲落在身後，換班礦工把沾泥的鎬重新靠上木架。",
            ),
            (
                "粗料倉",
                "你從粗料倉回到礦坑外棚。標記木牌和礦袋退到棚後，坑口吊燈聲重新貼近。",
            ),
        ]
    } else if room_name.contains("洗礦溝") {
        vec![
            (
                "礦坑外棚",
                "你從礦坑外棚走到洗礦溝。吊燈光被留在坑口，冷水槽、石籃和混濁泡沫把粗料接到水邊。",
            ),
            (
                "工具修棚",
                "你從工具修棚下到洗礦溝。敲鐵聲退到棚裡，水槽撞石聲和灰泡重新漫過腳邊。",
            ),
            (
                "礦渣坡",
                "你從礦渣坡回到洗礦溝。碎石滑落聲沉到坡後，洗礦工把下一籃粗料壓進冷水裡。",
            ),
        ]
    } else if room_name.contains("粗料倉") {
        vec![
            (
                "礦坑外棚",
                "你從礦坑外棚走到粗料倉。換班腳步留在坑口，濕礦袋和炭線木牌把粗料分出方向。",
            ),
            (
                "工具修棚",
                "你從工具修棚回到粗料倉。敲鐵聲落在棚後，倉管把剛修好的車架旁邊礦袋重新標記。",
            ),
        ]
    } else if room_name.contains("工具修棚") {
        vec![
            (
                "洗礦溝",
                "你從洗礦溝走進工具修棚前。水槽聲在身後變悶，裂柄鎬、車輪鐵圈和火星接住腳步。",
            ),
            (
                "粗料倉",
                "你從粗料倉走到工具修棚。炭線木牌退到倉邊，修具工把鬆鉚釘敲回車輪鐵圈裡。",
            ),
            (
                "舊道裝車點",
                "你從舊道裝車點回到工具修棚。車軸聲被棚頂壓低，修好的支架和鐵件等著下一批人來領。",
            ),
        ]
    } else if room_name.contains("礦渣坡") {
        vec![
            (
                "洗礦溝",
                "你從洗礦溝走到礦渣坡。冷水槽聲退到坡下，灰黑碎石和新滑坡痕把路邊界壓得很窄。",
            ),
            (
                "舊道裝車點",
                "你從舊道裝車點下到礦渣坡。車輪聲被留在舊道口，推車人把廢料倒進安全線內。",
            ),
        ]
    } else if room_name.contains("舊道裝車點") {
        vec![
            (
                "工具修棚",
                "你從工具修棚走到舊道裝車點。敲鐵聲落在棚後，礦車、車轍和稱重木牌把路接向邊城外門方向。",
            ),
            (
                "礦渣坡",
                "你從礦渣坡回到舊道裝車點。碎石坡聲沉到身後，車夫把繩索掛上灰粉裡的車架。",
            ),
            (
                "邊城外門",
                "你從邊城外門走回舊道裝車點。城門人聲退到舊道後，礦車和稱重木牌重新把腳步拉回礦場。",
            ),
        ]
    } else if room_name.contains("農舍前坪") {
        vec![
            (
                "田區路口",
                "你從田區路口走進農舍前坪。田埂腳印轉到屋前濕泥裡，鋤具和曬衣繩把田裡活計收回院前。",
            ),
            (
                "穀場小棚",
                "你從穀場小棚回到農舍前坪。稻殼乾響被留在棚邊，屋前濕泥又接上農戶進出的腳印。",
            ),
        ]
    } else if room_name.contains("穀場小棚") {
        vec![
            (
                "農舍前坪",
                "你從農舍前坪走到穀場小棚。曬衣繩在身後低低晃著，棚下稻袋和木篩把腳步聲變得更乾。",
            ),
            (
                "田埂深處",
                "你從田埂深處繞上穀場小棚。濕草貼身的感覺退開，稻殼和木棚陰影把路重新收住。",
            ),
        ]
    } else if room_name.contains("田埂深處") {
        vec![
            (
                "田區路口",
                "你從田區路口走入田埂深處。木牌旁的人聲被留在身後，濕草和低田面把路壓得更窄。",
            ),
            (
                "穀場小棚",
                "你從穀場小棚下到田埂深處。稻殼乾響斷在棚後，濕草和水鳥動靜重新貼近腳邊。",
            ),
        ]
    } else if room_name.contains("短市") {
        vec![
            (
                "北岸渡口",
                "你從北岸渡口又走回岸邊短市。身後候渡的人聲還沒散，價牌下的低聲議價已先接了上來。",
            ),
            (
                "倉棚前場",
                "你從倉棚前場回到岸邊短市。木箱拖痕一路斷在貨攤邊，問價聲很快把搬貨的聲音蓋過去。",
            ),
            (
                "船戶作業點",
                "你從船戶作業點折回岸邊短市。纜繩和木楔的氣味還在身後，攤邊已有人壓低聲音問下一班渡船。",
            ),
        ]
    } else {
        Vec::new()
    }
}

fn looks_like_private_name(s: &str) -> bool {
    let mut count = 0usize;
    for ch in s.chars() {
        if !('\u{4e00}'..='\u{9fff}').contains(&ch) {
            return false;
        }
        count += 1;
    }
    (2..=4).contains(&count) && !looks_like_public_role(s)
}

fn looks_like_public_role(s: &str) -> bool {
    if game::is_final_map_public_role(s) {
        return true;
    }
    let roles = [
        "船戶",
        "渡工",
        "挑夫",
        "旅人",
        "商人",
        "攤販",
        "茶客",
        "行人",
        "獵戶",
        "道士",
        "守衛",
        "工匠",
        "學徒",
        "修船者",
        "候渡者",
        "看貨人",
        "歇腳旅人",
        "候船者",
        "上岸客",
        "搬貨人",
        "引船人",
        "分揀工",
        "採集者",
        "倉管",
        "秤手",
        "守倉人",
        "翻穀人",
        "車夫",
        "搬糧人",
        "碼頭工",
        "稅吏",
        "船行人",
        "船員",
        "船匠",
        "修船工",
        "船客",
        "補帆人",
        "海關吏",
        "商館人",
        "翻譯",
        "兌票人",
        "搬魚人",
        "補網人",
        "魚販",
        "船戶家屬",
        "守望人",
        "礦工",
        "洗礦工",
        "修具工",
        "推車人",
    ];
    roles.iter().any(|role| s.contains(role))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn npc_card(id: &'static str, label: &str, card_state: &'static str) -> NpcCard<'static> {
        NpcCard {
            id,
            label: label.to_string(),
            card_state,
        }
    }

    #[test]
    fn folds_unnamed_people_without_hiding_named_cards() {
        let cards = vec![
            npc_card("o1", "候渡者", "observed"),
            npc_card("o2", "候渡者", "observed"),
            npc_card("o3", "候渡者", "observed"),
            npc_card("o4", "候渡者", "observed"),
            npc_card("o5", "候渡者", "observed"),
            npc_card("s1", "行人", "stranger"),
            npc_card("s2", "行人", "stranger"),
            npc_card("s3", "行人", "stranger"),
            npc_card("n1", "渡工江季良", "named"),
        ];

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(has_hidden_unnamed);
        assert!(visible.iter().any(|card| card.id == "n1"));
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.display_name.starts_with("候渡者"))
                .count(),
            3
        );
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.card_state != "named")
                .count(),
            5
        );
        assert!(visible.iter().all(|card| !card.display_name.contains('戊')));
    }

    #[test]
    fn does_not_fold_small_unnamed_groups() {
        let cards = vec![
            npc_card("o1", "候渡者", "observed"),
            npc_card("o2", "候渡者", "observed"),
            npc_card("s1", "行人", "stranger"),
        ];

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(!has_hidden_unnamed);
        let names: Vec<&str> = visible
            .iter()
            .map(|card| card.display_name.as_str())
            .collect();
        assert_eq!(names, vec!["候渡者甲", "候渡者乙", "行人"]);
    }

    #[test]
    fn north_bank_scene_changes_with_time_without_losing_place_anchor() {
        let empty_events: Vec<String> = Vec::new();
        let dawn = north_bank_waiting_scene_desc(6 * 60, Some(("clear", 0)), 8, &empty_events);
        let night = north_bank_waiting_scene_desc(22 * 60, Some(("clear", 0)), 8, &empty_events);

        assert_ne!(dawn, night);
        assert!(dawn.contains("北岸渡口"));
        assert!(night.contains("北岸渡口"));
        assert!(dawn.contains("晨霧") || dawn.contains("天光"));
        assert!(night.contains("夜水") || night.contains("夜風"));
    }

    #[test]
    fn north_bank_scene_projects_climate_and_crowd_without_exact_count() {
        let empty_events: Vec<String> = Vec::new();
        let rainy_crowd =
            north_bank_waiting_scene_desc(13 * 60, Some(("rain", 2)), 14, &empty_events);

        assert!(rainy_crowd.contains("雨後") || rainy_crowd.contains("濕氣"));
        assert!(rainy_crowd.contains("候渡的人群") || rainy_crowd.contains("短市邊"));
        assert!(!rainy_crowd.contains("14"));
    }

    #[test]
    fn north_bank_scene_projects_object_surfaces_as_current_state() {
        let empty_events: Vec<String> = Vec::new();
        let desc = north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 8, &empty_events);

        assert!(desc.contains("長凳"));
        assert!(desc.contains("湯"));
        assert!(desc.contains("貨擔"));
        assert!(desc.contains("船繩") || desc.contains("踏板"));
        for forbidden in ["登入後", "資料庫", "debug", "還有"] {
            assert!(!desc.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn north_bank_scene_projects_ferry_phase() {
        let empty_events: Vec<String> = Vec::new();
        let north_docked =
            north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 8, &empty_events);
        let south_docked =
            north_bank_waiting_scene_desc(8 * 60 + 20, Some(("clear", 0)), 8, &empty_events);

        assert_ne!(north_docked, south_docked);
        assert!(north_docked.contains("船腹碰岸") || north_docked.contains("濕繩繃"));
        assert!(south_docked.contains("南岸那頭") || south_docked.contains("對岸船邊"));
    }

    #[test]
    fn north_bank_scene_casts_recent_events_as_surface_traces_without_names() {
        let events = vec!["王小明，往南岸離開".to_string()];
        let desc = north_bank_waiting_scene_desc(8 * 60, Some(("clear", 0)), 4, &events);

        assert!(desc.contains("濕腳印") || desc.contains("空出"));
        assert!(!desc.contains("王小明"));
    }

    #[test]
    fn north_bank_entry_log_is_multiline_scene_flow_without_self_projection() {
        let entry = player_layer_entry_log("北岸渡口", 8);
        let lines: Vec<&str> = entry
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        assert!(lines.len() >= 3, "北岸第一屏入場動態應是可閱讀的多行場景流");
        assert!(entry.contains("船繩"));
        assert!(entry.contains("短市"));
        assert!(entry.contains("等船人群") || entry.contains("候渡"));
        for forbidden in ["你", "現場動態", "登入後", "debug", "資料庫", "8"] {
            assert!(!entry.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn north_bank_entry_scene_flow_changes_with_time_weather_and_ferry_phase() {
        let fog_dawn = player_layer_entry_log_for_scene("北岸渡口", 8, 6 * 60, Some(("fog", 11)));
        let clear_night =
            player_layer_entry_log_for_scene("北岸渡口", 8, 23 * 60 + 40, Some(("clear", 11)));

        assert_ne!(fog_dawn, clear_night);
        assert_eq!(
            fog_dawn
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            3
        );
        assert_eq!(
            clear_night
                .lines()
                .filter(|line| !line.trim().is_empty())
                .count(),
            3
        );
        assert!(fog_dawn.contains('霧'));
        assert!(clear_night.contains('夜') || clear_night.contains("燈影"));
        for text in [fog_dawn, clear_night] {
            for forbidden in ["你", "現場動態", "debug", "資料庫"] {
                assert!(!text.contains(forbidden), "{forbidden}");
            }
        }
    }

    #[test]
    fn north_bank_reentry_log_is_continuous_not_first_entry_replay() {
        let first = player_layer_entry_log("北岸渡口", 8);
        let again = player_layer_reentry_log("北岸渡口", 8);

        assert_ne!(first, again);
        assert!(again.contains("回到北岸渡口"));
        assert!(!again.contains("站定"));
        assert!(!again.contains('8'));
    }

    #[test]
    fn north_bank_reentry_log_can_reflect_previous_room() {
        let generic = player_layer_reentry_log("北岸渡口", 8);
        let from_market = route_reentry_log_for_room("北岸渡口", "岸邊短市", 8)
            .expect("北岸應有從短市返回的路徑句");

        assert_ne!(generic, from_market);
        assert!(from_market.contains("從短市那頭回到北岸渡口"));
        assert!(from_market.contains("價牌"));
        assert!(!from_market.contains('8'));
    }

    #[test]
    fn outer_road_entry_logs_have_route_presence_not_counts() {
        let road = player_layer_entry_log("北堤泥路", 1);
        let field = player_layer_entry_log("田埂水口", 1);

        assert!(road.contains("北堤泥路"));
        assert!(road.contains("挑擔腳印"));
        assert!(field.contains("田埂水口"));
        assert!(field.contains("細水"));
        assert!(!road.contains('1'));
        assert!(!field.contains('1'));
    }

    #[test]
    fn outer_road_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("北堤泥路", 1);
        let from_crossroad = route_reentry_log_for_room("北堤泥路", "中河谷地路口", 1)
            .expect("北堤泥路應有從谷地路口進入的路徑句");

        assert_ne!(generic, from_crossroad);
        assert!(from_crossroad.contains("從中河谷地路口踏上北堤泥路"));
        assert!(from_crossroad.contains("渡口人聲"));
        assert!(!from_crossroad.contains('1'));
    }

    #[test]
    fn market_outer_road_does_not_use_market_body_log() {
        let entry = player_layer_entry_log("短市外谷地路", 1);

        assert!(entry.contains("短市外谷地路"));
        assert!(entry.contains("挑擔"));
        assert!(!entry.contains("走進岸邊短市"));
    }

    #[test]
    fn guard_shed_scene_keeps_own_description_not_ferry_warehouse() {
        let desc = player_layer_scene_desc(
            "守倉棚半遮著冊簿和茶壺，棚柱旁掛著值牌。",
            "守倉棚",
            18 * 60,
            Some(("clear", 0)),
            3,
            &[],
        );

        assert!(desc.contains("守倉棚"));
        assert!(desc.contains("冊簿"));
        assert!(!desc.contains("倉棚前場"));
        assert!(!desc.contains("下一趟船"));
    }

    #[test]
    fn boathouse_street_scene_keeps_own_description_not_ferry_work_point() {
        let desc = player_layer_scene_desc(
            "船戶街口擠著招工牌、飯攤和晾著鹽味衣物的繩線。",
            "船戶街口",
            6 * 60,
            Some(("clear", 0)),
            3,
            &[],
        );

        assert!(desc.contains("船戶街口"));
        assert!(desc.contains("招工牌"));
        assert!(!desc.contains("船戶作業點"));
        assert!(!desc.contains("撐篙"));
    }

    #[test]
    fn m3_032_entry_logs_have_waterwork_presence_not_counts() {
        let water_entry = player_layer_entry_log("水網入口棧", 7);
        let fish_sorting = player_layer_entry_log("魚貨分揀場", 7);

        assert!(water_entry.contains("水網入口棧"));
        assert!(water_entry.contains("引船人"));
        assert!(fish_sorting.contains("魚貨分揀場"));
        assert!(fish_sorting.contains("分揀工"));
        assert_eq!(crowd_label_for_scene("水倉前場"), "水倉人群");
        assert!(!water_entry.contains('7'));
        assert!(!fish_sorting.contains('7'));
    }

    #[test]
    fn m3_032_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("水網入口棧", 7);
        let from_field = route_reentry_log_for_room("水網入口棧", "秧苗田邊", 7)
            .expect("水網入口棧應有從秧苗田邊進入的路徑句");
        let from_warehouse = route_reentry_log_for_room("魚貨分揀場", "水倉前場", 7)
            .expect("魚貨分揀場應有從水倉返回的路徑句");

        assert_ne!(generic, from_field);
        assert!(from_field.contains("從秧苗田邊走到水網入口棧"));
        assert!(from_field.contains("田面蛙聲"));
        assert!(from_warehouse.contains("從水倉前場回到魚貨分揀場"));
        assert!(!from_field.contains('7'));
        assert!(!from_warehouse.contains('7'));
    }

    #[test]
    fn m3_032_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "水網入口棧"), "引船人");
        assert_eq!(observed_label_for_scene("李四", "魚貨分揀場"), "分揀工");
        assert_eq!(observed_label_for_scene("王五", "水倉前場"), "倉管");
    }

    #[test]
    fn m3_045_entry_logs_have_granary_presence_not_counts() {
        let scale = player_layer_entry_log("糧倉外秤", 9);
        let gate = player_layer_entry_log("倉門前場", 9);

        assert!(scale.contains("糧倉外秤"));
        assert!(scale.contains("秤手"));
        assert!(gate.contains("倉門前場"));
        assert!(gate.contains("守倉人"));
        assert_eq!(crowd_label_for_scene("糧車泊位"), "糧車人群");
        assert!(!scale.contains('9'));
        assert!(!gate.contains('9'));
    }

    #[test]
    fn m3_045_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("糧倉外秤", 9);
        let from_downriver = route_reentry_log_for_room("糧倉外秤", "下游田埂路", 9)
            .expect("糧倉外秤應有從下游田埂路進入的路徑句");
        let from_cart = route_reentry_log_for_room("糧車泊位", "車行口", 9)
            .expect("糧車泊位應有從車行口進入的路徑句");

        assert_ne!(generic, from_downriver);
        assert!(from_downriver.contains("從下游田埂路走到糧倉外秤"));
        assert!(from_downriver.contains("秤盤"));
        assert!(from_cart.contains("從車行口走到糧車泊位"));
        assert!(!from_downriver.contains('9'));
        assert!(!from_cart.contains('9'));
    }

    #[test]
    fn m3_045_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "糧倉外秤"), "秤手");
        assert_eq!(observed_label_for_scene("李四", "倉門前場"), "守倉人");
        assert_eq!(observed_label_for_scene("王五", "糧車泊位"), "搬糧人");
    }

    #[test]
    fn m3_048_entry_logs_have_estuary_port_presence_not_counts() {
        let dock = player_layer_entry_log("河口潮泥碼頭", 11);
        let tax = player_layer_entry_log("稅站棚", 11);

        assert!(dock.contains("河口潮泥碼頭"));
        assert!(dock.contains("碼頭工"));
        assert!(tax.contains("稅站棚"));
        assert!(tax.contains("稅吏"));
        assert_eq!(crowd_label_for_scene("商船候泊線"), "候泊人群");
        assert!(!dock.contains("11"));
        assert!(!tax.contains("11"));
    }

    #[test]
    fn m3_048_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("河口潮泥碼頭", 11);
        let from_granary = route_reentry_log_for_room("河口潮泥碼頭", "守倉棚", 11)
            .expect("河口潮泥碼頭應有從守倉棚進入的路徑句");
        let from_cargo = route_reentry_log_for_room("商船候泊線", "潮線貨場", 11)
            .expect("商船候泊線應有從潮線貨場進入的路徑句");

        assert_ne!(generic, from_granary);
        assert!(from_granary.contains("從守倉棚走到河口潮泥碼頭"));
        assert!(from_granary.contains("海口"));
        assert!(from_cargo.contains("從潮線貨場走到商船候泊線"));
        assert!(!from_granary.contains("11"));
        assert!(!from_cargo.contains("11"));
    }

    #[test]
    fn m3_048_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "河口潮泥碼頭"), "碼頭工");
        assert_eq!(observed_label_for_scene("李四", "稅站棚"), "稅吏");
        assert_eq!(observed_label_for_scene("王五", "商船候泊線"), "船員");
    }

    #[test]
    fn m3_050_entry_logs_have_shipyard_presence_not_counts() {
        let gate = player_layer_entry_log("東南船塢口", 12);
        let repair = player_layer_entry_log("修船架", 12);
        let sailcloth = player_layer_entry_log("帆布曬場", 12);

        assert!(gate.contains("東南船塢口"));
        assert!(gate.contains("船匠"));
        assert!(repair.contains("修船架"));
        assert!(repair.contains("修船工"));
        assert!(sailcloth.contains("帆布曬場"));
        assert!(sailcloth.contains("補帆人"));
        assert!(!sailcloth.contains("翻穀人"));
        assert!(!sailcloth.contains("穀粒"));
        assert_eq!(crowd_label_for_scene("碼頭木階"), "木階人群");
        assert_eq!(crowd_label_for_scene("帆布曬場"), "補帆人群");
        assert!(!gate.contains("12"));
        assert!(!repair.contains("12"));
    }

    #[test]
    fn m3_050_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("東南船塢口", 12);
        let from_warehouse = route_reentry_log_for_room("東南船塢口", "河海倉門", 12)
            .expect("東南船塢口應有從河海倉門進入的路徑句");
        let from_repair = route_reentry_log_for_room("帆布曬場", "修船架", 12)
            .expect("帆布曬場應有從修船架進入的路徑句");
        let from_steps = route_reentry_log_for_room("船戶街口", "碼頭木階", 12)
            .expect("船戶街口應有從碼頭木階進入的路徑句");

        assert_ne!(generic, from_warehouse);
        assert!(from_warehouse.contains("從河海倉門走到東南船塢口"));
        assert!(from_warehouse.contains("船港"));
        assert!(from_repair.contains("從修船架走到帆布曬場"));
        assert!(from_repair.contains("舊帆"));
        assert!(!from_repair.contains("翻穀"));
        assert!(from_steps.contains("從碼頭木階走到船戶街口"));
        assert!(!from_warehouse.contains("12"));
        assert!(!from_repair.contains("12"));
        assert!(!from_steps.contains("12"));
    }

    #[test]
    fn m3_050_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "東南船塢口"), "船匠");
        assert_eq!(observed_label_for_scene("李四", "修船架"), "修船工");
        assert_eq!(observed_label_for_scene("王五", "帆布曬場"), "補帆人");
    }

    #[test]
    fn m3_053_entry_logs_have_trade_port_presence_not_counts() {
        let dock = player_layer_entry_log("外貿碼頭", 9);
        let customs = player_layer_entry_log("海關前棚", 9);
        let exchange = player_layer_entry_log("匯兌小屋", 9);

        assert!(dock.contains("外貿碼頭"));
        assert!(dock.contains("船員"));
        assert!(customs.contains("海關前棚"));
        assert!(customs.contains("海關吏"));
        assert!(exchange.contains("匯兌小屋"));
        assert!(exchange.contains("兌票人"));
        assert_eq!(crowd_label_for_scene("外貿碼頭"), "外貿人群");
        assert_eq!(crowd_label_for_scene("翻譯桌"), "翻譯人群");
        assert!(!dock.contains('9'));
        assert!(!customs.contains('9'));
        assert!(!exchange.contains('9'));
    }

    #[test]
    fn m3_053_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("外貿碼頭", 9);
        let from_sailcloth = route_reentry_log_for_room("外貿碼頭", "帆布曬場", 9)
            .expect("外貿碼頭應有從帆布曬場進入的路徑句");
        let from_customs = route_reentry_log_for_room("翻譯桌", "海關前棚", 9)
            .expect("翻譯桌應有從海關前棚進入的路徑句");
        let from_warehouse = route_reentry_log_for_room("匯兌小屋", "遠洋倉門", 9)
            .expect("匯兌小屋應有從遠洋倉門進入的路徑句");

        assert_ne!(generic, from_sailcloth);
        assert!(from_sailcloth.contains("從帆布曬場走到外貿碼頭"));
        assert!(from_sailcloth.contains("港旗"));
        assert!(from_customs.contains("從海關前棚走到翻譯桌"));
        assert!(from_warehouse.contains("從遠洋倉門走到匯兌小屋"));
        assert!(!from_sailcloth.contains("9"));
        assert!(!from_customs.contains("9"));
        assert!(!from_warehouse.contains("9"));
    }

    #[test]
    fn m3_053_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "外貿碼頭"), "船員");
        assert_eq!(observed_label_for_scene("李四", "海關前棚"), "海關吏");
        assert_eq!(observed_label_for_scene("王五", "翻譯桌"), "翻譯");
        assert_eq!(observed_label_for_scene("趙六", "匯兌小屋"), "兌票人");
    }

    #[test]
    fn m3_006_entry_logs_have_north_harbor_presence_not_counts() {
        let dock = player_layer_entry_log("北灣避風碼頭", 8);
        let shed = player_layer_entry_log("港灣貨棚", 8);
        let watch = player_layer_entry_log("北灣守望石階", 8);

        assert!(dock.contains("北灣避風碼頭"));
        assert!(dock.contains("船戶"));
        assert!(shed.contains("港灣貨棚"));
        assert!(shed.contains("搬魚人"));
        assert!(watch.contains("北灣守望石階"));
        assert!(watch.contains("守望人"));
        assert_eq!(crowd_label_for_scene("冷霧魚市"), "魚市人群");
        assert_eq!(crowd_label_for_scene("漁網曬場"), "補網人群");
        assert!(!dock.contains('8'));
        assert!(!shed.contains('8'));
        assert!(!watch.contains('8'));
    }

    #[test]
    fn m3_006_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("北灣避風碼頭", 8);
        let from_farm = route_reentry_log_for_room("北灣避風碼頭", "農舍前坪", 8)
            .expect("北灣避風碼頭應有從農舍前坪進入的路徑句");
        let from_dock = route_reentry_log_for_room("港灣貨棚", "北灣避風碼頭", 8)
            .expect("港灣貨棚應有從北灣避風碼頭進入的路徑句");
        let from_market = route_reentry_log_for_room("北灣守望石階", "冷霧魚市", 8)
            .expect("北灣守望石階應有從冷霧魚市進入的路徑句");

        assert_ne!(generic, from_farm);
        assert!(from_farm.contains("從農舍前坪走到北灣避風碼頭"));
        assert!(from_farm.contains("冷海水"));
        assert!(from_dock.contains("從北灣避風碼頭走進港灣貨棚"));
        assert!(from_market.contains("從冷霧魚市踏上北灣守望石階"));
        assert!(!from_farm.contains("8"));
        assert!(!from_dock.contains("8"));
        assert!(!from_market.contains("8"));
    }

    #[test]
    fn m3_006_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "港灣貨棚"), "搬魚人");
        assert_eq!(observed_label_for_scene("李四", "漁網曬場"), "補網人");
        assert_eq!(observed_label_for_scene("王五", "冷霧魚市"), "魚販");
        assert_eq!(observed_label_for_scene("趙六", "北灣守望石階"), "守望人");
    }

    #[test]
    fn m3_009_entry_logs_have_mine_presence_not_counts() {
        let pit = player_layer_entry_log("礦坑外棚", 8);
        let wash = player_layer_entry_log("洗礦溝", 8);
        let load = player_layer_entry_log("舊道裝車點", 8);

        assert!(pit.contains("礦坑外棚"));
        assert!(pit.contains("礦工"));
        assert!(wash.contains("洗礦溝"));
        assert!(wash.contains("洗礦工"));
        assert!(load.contains("舊道裝車點"));
        assert!(load.contains("車夫"));
        assert_eq!(crowd_label_for_scene("粗料倉"), "粗料倉人群");
        assert_eq!(crowd_label_for_scene("礦渣坡"), "礦渣坡人群");
        assert!(!pit.contains('8'));
        assert!(!wash.contains('8'));
        assert!(!load.contains('8'));
    }

    #[test]
    fn m3_009_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("礦坑外棚", 8);
        let from_field = route_reentry_log_for_room("礦坑外棚", "田埂深處", 8)
            .expect("礦坑外棚應有從田埂深處進入的路徑句");
        let from_pit = route_reentry_log_for_room("洗礦溝", "礦坑外棚", 8)
            .expect("洗礦溝應有從礦坑外棚進入的路徑句");
        let from_border = route_reentry_log_for_room("舊道裝車點", "邊城外門", 8)
            .expect("舊道裝車點應有從邊城外門進入的路徑句");

        assert_ne!(generic, from_field);
        assert!(from_field.contains("從田埂深處走到礦坑外棚"));
        assert!(from_field.contains("吊燈光"));
        assert!(from_pit.contains("從礦坑外棚走到洗礦溝"));
        assert!(from_border.contains("從邊城外門走回舊道裝車點"));
        assert!(!from_field.contains("8"));
        assert!(!from_pit.contains("8"));
        assert!(!from_border.contains("8"));
    }

    #[test]
    fn m3_009_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "礦坑外棚"), "礦工");
        assert_eq!(observed_label_for_scene("李四", "洗礦溝"), "洗礦工");
        assert_eq!(observed_label_for_scene("王五", "工具修棚"), "修具工");
        assert_eq!(observed_label_for_scene("趙六", "礦渣坡"), "推車人");
        assert_eq!(observed_label_for_scene("錢七", "舊道裝車點"), "車夫");
    }

    #[test]
    fn final_map_entry_logs_have_scene_presence_not_counts() {
        for (room, role, crowd) in [
            ("邊城外門", "守門人", "邊門人群"),
            ("糧倉前場", "倉管", "糧倉人群"),
            ("城邦外坡道", "牽獸人", "坡道人群"),
            ("石壘門", "門衛", "門前人群"),
            ("門內券道", "引路人", "券道人群"),
            ("旗石廣場", "旗手", "廣場人群"),
            ("西軸坡街", "坡街販", "坡街人群"),
            ("議棚前階", "記牌人", "議棚人群"),
            ("鑄幣巷口", "兌牌人", "巷口人群"),
            ("傭工棚邊", "棚頭人", "傭工人群"),
            ("工坊街口", "學徒", "工坊人群"),
            ("牧地柵門", "牧人", "牧地人群"),
            ("牧場門牌", "牧場管事", "牧場人群"),
            ("邊鎮井口", "取水人", "井口人群"),
            ("車隊場", "車夫", "車隊人群"),
        ] {
            let entry = player_layer_entry_log(room, 8);
            let reentry = player_layer_reentry_log(room, 8);

            assert!(entry.contains(room), "{room}");
            assert!(entry.contains(role), "{room}");
            assert!(reentry.contains(room), "{room}");
            assert_eq!(crowd_label_for_scene(room), crowd, "{room}");
            assert!(!entry.contains('8'), "{room}");
            assert!(!reentry.contains('8'), "{room}");
        }
    }

    #[test]
    fn m3_020_public_presence_cards_are_stable_without_counts() {
        let mut cards = Vec::new();
        append_m3_020_public_presence_cards("旗石廣場", &mut cards);
        let labels: Vec<&str> = cards
            .iter()
            .filter_map(|card| card.get("display_name").and_then(serde_json::Value::as_str))
            .collect();
        assert_eq!(labels, vec!["旗手", "廣場人群"]);
        for card in &cards {
            assert_eq!(
                card.get("kind").and_then(serde_json::Value::as_str),
                Some("crowd")
            );
            let label = card
                .get("display_name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            assert!(!label.contains("還有"));
            assert!(!label.contains('8'));
        }

        append_m3_020_public_presence_cards("旗石廣場", &mut cards);
        assert_eq!(cards.len(), 2);

        let mut other_cards = Vec::new();
        append_m3_020_public_presence_cards("北岸渡口", &mut other_cards);
        assert!(other_cards.is_empty());
    }

    #[test]
    fn final_map_observed_labels_fall_back_to_public_roles() {
        for (room, label) in [
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("邊市口", "商旅"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西軸坡街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("鍛造棚", "鐵匠"),
            ("油布庫", "看庫人"),
            ("畜欄口", "牲口販"),
            ("獸醫棚", "獸醫"),
            ("商隊泊位", "商隊人"),
            ("客棧前廊", "店伙"),
            ("補給小市", "小販"),
            ("巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_scene("張三", room), label, "{room}");
        }
    }
}

// ── 昏倒輔助 ──────────────────────────────────────────────────────────────

/// 找曼哈頓距離最近的 landmark 格（TownHall 等）作為聚落傳送目的地。
/// 找不到時回傳 None（呼叫端 fallback 到 (0,0)）。
fn find_nearest_landmark(from_x: i32, from_y: i32) -> Option<(i32, i32, String)> {
    let grid = crate::server::grid_manager::get_runtime_square_grid()?;
    grid.cells()
        .filter(|c| c.terrain.category() == "landmark" && !c.name.is_empty())
        .min_by_key(|c| {
            let dx = (c.coord.x - from_x).abs();
            let dy = (c.coord.y - from_y).abs();
            dx + dy
        })
        .map(|c| (c.coord.x, c.coord.y, c.name.clone()))
}

/// 從背包隨機移除 1-3 件（整堆移除一個 item_id），回傳被移除物品的名稱列表。
fn drop_random_items(entity_id: &str, inventory_json: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Entry {
        item_id: String,
        qty: i32,
    }

    if inventory_json.is_empty() || inventory_json == "[]" {
        return vec![];
    }
    let entries: Vec<Entry> = match serde_json::from_str(inventory_json) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    if entries.is_empty() {
        return vec![];
    }

    let mut rng = rand::rng();
    let drop_count = rng.random_range(1_usize..=3_usize).min(entries.len());

    // Fisher-Yates 取前 drop_count 個隨機索引
    let mut indices: Vec<usize> = (0..entries.len()).collect();
    for i in 0..drop_count {
        let j = rng.random_range(i..entries.len());
        indices.swap(i, j);
    }

    let mut dropped = Vec::new();
    for &idx in &indices[..drop_count] {
        let e = &entries[idx];
        // 移除整堆
        let _ = db::remove_from_inventory(entity_id, &e.item_id, e.qty);
        // item_id 直接作為顯示名（placeholder，正式物品系統完善後可查 items 表）
        dropped.push(e.item_id.replace("harvest_", "").replace('_', " "));
    }
    dropped
}

/// scout 探路：揭露指定方向的單一鄰格，回傳該格名稱供前端更新路口顯示。
/// WS 訊息格式：`{ type: "scout", direction: "北"|"南"|"東"|"西" }`
/// 回傳 `{ type: "scout_result", direction, name, explored, walkable }`
pub(super) fn handle_scout(conn: &WsConnection, msg: &super::super::protocol::ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let ent = match crate::db::get_entity(&player_id) {
        Ok(Some(e)) => e,
        _ => {
            conn.send_error("entity not found");
            return;
        }
    };
    let (gx, gy) = match (ent.grid_x, ent.grid_y) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            conn.send_error("not in grid");
            return;
        }
    };

    let dir_zh = msg.direction.as_str();
    let dir = match crate::grid::Direction::from_zh(dir_zh) {
        Some(d) => d,
        None => {
            conn.send_error(format!("unknown direction: {dir_zh}"));
            return;
        }
    };

    let target = crate::grid::SquareCoord::new(gx, gy).neighbor(dir);

    // Reveal only cells that already exist in the authoritative grid.
    // Missing cells are map boundaries until an explicit map/import pass adds them.
    let (name, walkable) = {
        let result = crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(target) {
                cell.explored = true;
                let data = (cell.name.clone(), cell.terrain.walkable());
                grid.mark_dirty();
                data
            } else {
                ("邊界未開通".to_string(), false)
            }
        });
        result.unwrap_or_else(|| ("邊界未開通".to_string(), false))
    };

    let resp = serde_json::json!({
        "type": "scout_result",
        "direction": dir_zh,
        "name": name,
        "explored": true,
        "walkable": walkable
    });
    if let Ok(bytes) = serde_json::to_vec(&resp) {
        let _ = conn.tx.try_send(bytes);
    }
}

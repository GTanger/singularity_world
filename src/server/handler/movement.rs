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
use super::{current_game_hour, WsConnection};

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
            stat_delta: StatDelta { hp_cur: 0, inner_cur: 0 },
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
                // ── 死亡 110V 兜底：昏倒分支 ────────────────────────────
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
                    stat_delta: StatDelta { hp_cur: -(ent.vit), inner_cur: 0 },
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
            let stat = StatDelta { hp_cur: -dmg, inner_cur: 0 };
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
                "forest" | "forest_heavy" | "forest_light" | "jungle" => "撥開枯葉，拾得野莓三顆，露水未乾。",
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
            stat_delta: StatDelta { hp_cur: 0, inner_cur: 0 },
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
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.to_string(),
        room_id: new_room,
        room_name: String::new(),
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
        .map(|(cx, cy, kind, terrain, category, name, explored, walkable)| GridCellView {
            x: cx,
            y: cy,
            kind,
            terrain,
            category,
            name,
            explored,
            walkable,
        })
        .collect();

    // 為方格 NPC 填入行為敘事句（以當前 unix 秒 + id 為種子）
    let tick_seed = game::now_unix() as u64;
    let entities: Vec<crate::server::protocol::ViewEntity> = base.entities.into_iter().map(|mut ent| {
        if (ent.kind == "npc" || ent.kind == "Npc") && ent.behavior_text.is_empty() {
            ent.behavior_text = narrative::idle(&ent.id, &ent.display_name, tick_seed);
        }
        ent
    }).collect();

    // 先把 cells 轉為 exits_v5 所需的四方鄰格資訊，讓前端路口 UI 可以顯示 ▒▒▒
    // 這裡用 serde_json::json! 直接組，避免新增 protocol 型別
    {
        let dirs: &[(&str, i32, i32)] = &[("north", 0, 1), ("south", 0, -1), ("east", 1, 0), ("west", -1, 0)];
        let mut exits_v5 = serde_json::Map::new();
        for (dir_key, dx, dy) in dirs {
            let nx = x + dx;
            let ny = y + dy;
            // 找對應格（cells_raw 中 (nx, ny) 的資訊）
            // cells_raw 已被消耗，重新查一格
            let neighbor = crate::server::grid_manager::get_runtime_square_grid()
                .and_then(|g| g.get(crate::grid::SquareCoord::new(nx, ny)).cloned());
            // 黑格（尚未生成）= 未揭露，視同 explored:false，前端顯示 ▒▒▒
            match neighbor {
                Some(cell) => {
                    exits_v5.insert(dir_key.to_string(), serde_json::json!({
                        "explored": cell.explored,
                        "walkable": cell.terrain.walkable(),
                        "name": cell.name
                    }));
                }
                None => {
                    exits_v5.insert(dir_key.to_string(), serde_json::json!({
                        "explored": false,
                        "walkable": false,
                        "name": null
                    }));
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
        struct NpcCard<'a> {
            id: &'a str,
            label: String,          // 不含甲乙的標籤
            card_state: &'static str,
        }
        let cards: Vec<NpcCard<'_>> = npc_entities.iter().map(|e| {
            let acq = acq_map.get(&e.id);
            let named = acq.map(|a| a.named).unwrap_or(false);
            let observed = acq.map(|a| a.observed).unwrap_or(false);
            let (label, card_state) = if named {
                // 知道姓名：顯示完整 display_name（職稱+真名）
                (e.display_name.clone(), "named")
            } else if observed {
                // 已觀察身份：顯示職稱（display_name 裡 | 前段，或完整）
                let title = if let Some(pos) = e.display_name.find('|') {
                    e.display_name[..pos].trim().to_string()
                } else {
                    e.display_name.clone()
                };
                (title, "observed")
            } else {
                // 陌生：顯示「行人」（職稱的通稱化，不洩漏真實職業）
                // 這裡用「行人」作為陌生態通稱，spec 上只說甲乙範例是行人
                // 但 NPC 的職業仍不暴露——保留 "行人" 作統一陌生標籤
                ("行人".to_string(), "stranger")
            };
            NpcCard { id: &e.id, label, card_state }
        }).collect();

        // 甲乙編號：同 label 桶內 ≥ 2 人按順序附上天干
        const TIANGAN: [&str; 10] = ["甲","乙","丙","丁","戊","己","庚","辛","壬","癸"];
        let mut label_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for c in &cards {
            *label_counts.entry(c.label.as_str()).or_insert(0) += 1;
        }
        let mut label_cursor: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let npcs_json: Vec<serde_json::Value> = cards.iter().map(|c| {
            let count = label_counts.get(c.label.as_str()).copied().unwrap_or(1);
            let display_name = if count >= 2 {
                let cursor = label_cursor.entry(c.label.clone()).or_insert(0);
                let suffix = TIANGAN.get(*cursor).copied().unwrap_or("甲");
                *cursor += 1;
                format!("{}{}", c.label, suffix)
            } else {
                c.label.clone()
            };
            serde_json::json!({
                "npc_id": c.id,
                "display_name": display_name,
                "card_state": c.card_state
            })
        }).collect();

        // 取當前格地形 key（英文），供前端 L3 渲染用
        let current_terrain_key = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(crate::grid::SquareCoord::new(x, y)).cloned())
            .map(|c| c.terrain.kind_str().to_string())
            .unwrap_or_default();

        let room_enter_msg = serde_json::json!({
            "type": "room_enter",
            "exits": exits_v5,
            "objects": serde_json::json!([]),
            "npcs": npcs_json,
            "time": {
                "hour_str": format!("{:02}:00", game_hour),
                "hour": game_hour
            },
            "terrain": {
                "icon_key": current_terrain_key
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
    struct Entry { item_id: String, qty: i32 }

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

    // 揭露鄰格（標記 explored = true），同步到 PG
    let (name, walkable) = {
        let result = crate::server::grid_manager::with_square_grid_mut(|grid| {
            if let Some(cell) = grid.get_mut(target) {
                cell.explored = true;
                let data = (cell.name.clone(), cell.terrain.walkable());
                grid.mark_dirty();
                data
            } else {
                // 鄰格不存在（地圖邊界），動態生成一格
                let new_cells = crate::grid::reveal_grid_disk(grid, target, 0);
                tracing::info!("[scout] 邊界揭露 {} 新格", new_cells);
                if let Some(cell) = grid.get_mut(target) {
                    cell.explored = true;
                    let data = (cell.name.clone(), cell.terrain.walkable());
                    grid.mark_dirty();
                    data
                } else {
                    ("未知之地".to_string(), false)
                }
            }
        });
        result.unwrap_or_else(|| ("未知之地".to_string(), false))
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


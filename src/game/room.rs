use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::grid::{Direction, SquareCoord, grid_room_id_from_coord, terrain_name_zh};
use crate::model::{Exit, Room, RoomObject};
use crate::store;
use parking_lot::RwLock;
use std::collections::HashSet;
use std::sync::LazyLock;

/// 記錄已被玩家「查探獸跡／尋跡」而破霧顯形的鎮場霸主座標 (x, y) 與顯形時間戳。
static TRACKED_APEX_COORDS: LazyLock<RwLock<HashSet<(i32, i32)>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 標記某處獵場座標之鎮場霸主已被尋跡顯形。
pub fn mark_apex_beast_tracked_at(x: i32, y: i32) {
    TRACKED_APEX_COORDS.write().insert((x, y));
}

/// 檢查某處獵場座標之鎮場霸主是否已因尋跡而破霧顯形。
pub fn is_apex_beast_tracked_at(x: i32, y: i32) -> bool {
    TRACKED_APEX_COORDS.read().contains(&(x, y))
}
/// 當前房間視野。
#[derive(Debug, Clone)]
pub struct RoomView {
    pub room: Room,
    pub exits: Vec<Exit>,
    pub entities: Vec<Character>,
    pub objects: Vec<RoomObject>,
}

/// 過濾視野中不合適顯示的實體：
/// 1. 氣血歸零（無生命）
/// 2. 測試殘留（含底線命名或測試標記）
/// 3. 超過 180 秒無活動觀測的離線殭屍角色
pub fn filter_room_entities(entities: Vec<Character>) -> Vec<Character> {
    let now = crate::game::now_unix();
    entities
        .into_iter()
        .filter(|e| {
            if e.vit <= 0 {
                return false;
            }
            if e.display_char.contains('_') {
                return false;
            }
            match e.kind {
                crate::entity::EntityKind::Player => {
                    if e.id.contains('_') {
                        return false;
                    }
                    let last_active = e.last_observed_at.unwrap_or(e.created_at);
                    if last_active > 0 && now.saturating_sub(last_active) > 180 {
                        return false;
                    }
                }
                crate::entity::EntityKind::Npc => {
                    let lower_id = e.id.to_ascii_lowercase();
                    if lower_id.starts_with("test")
                        || lower_id.contains("test_")
                        || lower_id.contains("_test")
                        || lower_id.starts_with("stub")
                        || lower_id.starts_with("dummy")
                    {
                        return false;
                    }
                }
            }
            true
        })
        .collect()
}

/// 依房間 id 載入視野；`game_hour` 0–23 供 NPC 職稱，`-1` 表示不套用下班規則。
pub fn get_room_view(room_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(room) = db::get_room(room_id)? else {
        return Ok(None);
    };
    let exits = db::get_exits_for_room(room_id)?;
    let entities = filter_room_entities(db::get_entities_in_room(room_id, game_hour)?);
    let objects = db::get_objects_in_room(room_id)?;
    Ok(Some(RoomView {
        room,
        exits,
        entities,
        objects,
    }))
}

/// 以正式 `world_places` / `world_routes` 組裝玩家當前所見。
pub fn get_place_room_view(place_id: &str, game_hour: i32) -> anyhow::Result<Option<RoomView>> {
    let Some(store) = store::get_store() else {
        return Ok(None);
    };
    let (place, exits) = {
        let guard = store.read();
        let Some(place) = guard.get_place(place_id).cloned() else {
            return Ok(None);
        };
        if !place.traversable || place.status != "active" {
            return Ok(None);
        }
        let graph = guard.place_graph_snapshot();
        let exits = graph
            .routes_from(place_id)
            .iter()
            .filter(|route| route.status == "active")
            .filter_map(|route| {
                let target = graph.place(&route.to_place_id)?;
                Some(Exit {
                    direction: route.direction.clone(),
                    to_room_id: target.id.clone(),
                    to_room_name: target.name.clone(),
                    route_id: route.id.clone(),
                    mode: route.mode.clone(),
                    travel_cost: route.travel_cost,
                })
            })
            .collect::<Vec<_>>();
        (place, exits)
    };
    let mut objects = Vec::new();
    if let Some(pool) = crate::pg::pool()
        && let Some(source) = crate::pg::world_resource::get_at_place(&pool, place_id)?
    {
        let sockets = vec!["Look".to_string(), "Gather".to_string()];
        let state = if source.current_stock > 0 {
            format!("目前仍可取得約{}份。", source.current_stock)
        } else {
            "眼前能取用的部分已空，必須等它慢慢恢復。".to_string()
        };
        objects.push(RoomObject {
            id: format!("resource:{}:{}", source.place_id, source.item_id),
            name: format!("可取用的{}", source.name),
            owner: String::new(),
            sockets,
            responses: std::collections::HashMap::from([(
                "Look".to_string(),
                format!("{}；{}", source.origin_label, state),
            )]),
            move_to_room_id: String::new(),
        });
    }
    let entities = filter_room_entities(db::get_entities_in_room(place_id, game_hour)?);
    Ok(Some(RoomView {
        room: Room {
            id: place.id,
            name: place.name,
            tags: place.tags,
            zone: place.zone,
            description: place.description,
            objects: objects.clone(),
        },
        exits,
        entities,
        objects,
    }))
}

/// 若實體尚無房間則設為預設房間，並回傳房間 id。
pub fn ensure_entity_in_room(entity_id: &str, default_room_id: &str) -> anyhow::Result<String> {
    let mut room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        db::set_entity_room(entity_id, default_room_id)?;
        room_id = default_room_id.to_string();
    }
    Ok(room_id)
}

/// 依出口方向移動；回傳 `(新房間 id, 是否成功)`。
pub fn move_by_exit(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if dir.is_empty() {
        return Ok((String::new(), false));
    }
    let room_id = db::get_entity_room(entity_id)?;
    if room_id.is_empty() {
        return Ok((String::new(), false));
    }
    let exits = db::get_exits_for_room(&room_id)?;
    for ex in &exits {
        if ex.direction.trim() == dir {
            db::set_entity_room(entity_id, &ex.to_room_id)?;
            return Ok((ex.to_room_id.clone(), true));
        }
    }
    let objs = db::get_objects_in_room(&room_id)?;
    for o in objs {
        if o.name.trim() != dir {
            continue;
        }
        if o.move_to_room_id.is_empty() || !object_has_socket(&o, "Move") {
            continue;
        }
        db::set_entity_room(entity_id, &o.move_to_room_id)?;
        return Ok((o.move_to_room_id.clone(), true));
    }
    Ok((String::new(), false))
}

// ─── 正方格地圖 ───────────────────────────────────────────────────

fn grid_objects_to_room_objects(objs: &[crate::grid::GridObject]) -> Vec<RoomObject> {
    objs.iter()
        .map(|o| RoomObject {
            id: o.id.clone(),
            name: o.name.clone(),
            owner: String::new(),
            sockets: if o.quantity > 0 {
                vec!["Look".into(), "Gather".into()]
            } else {
                vec!["Look".into()]
            },
            responses: Default::default(),
            move_to_room_id: String::new(),
        })
        .collect()
}

/// 依方位字在正方格上走一步。成功時已寫入 DB。
pub fn move_by_grid_direction(entity_id: &str, direction: &str) -> anyhow::Result<(String, bool)> {
    move_by_place_route(entity_id, direction, None)
}

/// 沿確切相鄰場所移動；未指定目標時才取該方向排序後的第一條路。
pub fn move_by_place_route(
    entity_id: &str,
    direction: &str,
    target_place_id: Option<&str>,
) -> anyhow::Result<(String, bool)> {
    let dir = direction.trim();
    if !matches!(dir, "東" | "西" | "南" | "北" | "上" | "下" | "進" | "出") {
        return Ok((String::new(), false));
    }
    let arc = store::get_store().ok_or_else(|| anyhow::anyhow!("no store"))?;
    let current_room = arc.read().get_entity_room(entity_id);
    let first_move =
        if let Some(target) = target_place_id.filter(|target| !target.trim().is_empty()) {
            let movement = arc
                .write()
                .move_entity_to_adjacent_place(entity_id, target.trim())?;
            if let Some(ref movement) = movement {
                anyhow::ensure!(movement.route.direction == dir, "目標場所與方向不相符");
            }
            movement
        } else {
            arc.write().move_entity_direction(entity_id, dir)?
        };
    if let Some(place_move) = first_move {
        record_command_move(entity_id, &place_move);
        let new_room = {
            let s = arc.read();
            let tz = s
                .get_place(&place_move.to_place_id)
                .map(|p| p.zone.clone())
                .unwrap_or_else(|| "wt-c-wenyao".to_string());
            let _ = s.step_npc_grid_coord(
                entity_id,
                &place_move.route.direction,
                &tz,
                super::now_unix() as u64,
            );
            s.get_entity_room(entity_id)
        };
        return Ok((new_room, true));
    }
    if let Some(gdir) = Direction::from_zh(dir)
        && reveal_missing_grid_route(entity_id, gdir)?
    {
        let second_move = { arc.write().move_entity_direction(entity_id, dir)? };
        if let Some(place_move) = second_move {
            record_command_move(entity_id, &place_move);
            let new_room = {
                let s = arc.read();
                let tz = s
                    .get_place(&place_move.to_place_id)
                    .map(|p| p.zone.clone())
                    .unwrap_or_else(|| "wt-c-wenyao".to_string());
                let _ = s.step_npc_grid_coord(
                    entity_id,
                    &place_move.route.direction,
                    &tz,
                    super::now_unix() as u64,
                );
                s.get_entity_room(entity_id)
            };
            return Ok((new_room, true));
        }
    }
    Ok((current_room, false))
}

fn record_command_move(entity_id: &str, movement: &store::PlaceMove) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    db::set_entity_journey(crate::world::journey::EntityJourney {
        entity_id: entity_id.to_string(),
        reason: "player_command".to_string(),
        destination_place_id: Some(movement.to_place_id.clone()),
        planned_path: Vec::new(),
        consequence: format!(
            "由{}向{}抵達{}，路線成本{}，行動收據{}",
            movement.from_place_id,
            movement.route.direction,
            movement.to_place_id,
            movement.route.travel_cost,
            movement.action_id
        ),
        updated_at: now,
    });
}

fn compatibility_room(
    store: &std::sync::Arc<parking_lot::RwLock<store::Store>>,
    id: &str,
) -> String {
    store
        .read()
        .get_place(id)
        .map(|place| place.compatibility_room_id())
        .unwrap_or_else(|| id.to_string())
}

fn reveal_missing_grid_route(entity_id: &str, direction: Direction) -> anyhow::Result<bool> {
    let Some(store) = store::get_store() else {
        return Ok(false);
    };
    let Some(entity) = store.read().get_entity(entity_id) else {
        return Ok(false);
    };
    let (Some(x), Some(y)) = (entity.grid_x, entity.grid_y) else {
        return Ok(false);
    };
    let from = SquareCoord::new(x, y);
    let changed = crate::server::grid_manager::with_square_grid_mut(|grid| {
        let Some(cell) = grid.get(from) else {
            return false;
        };
        if !cell.explicit_exits.is_empty() {
            return false;
        }
        let target = from.neighbor(direction);
        let before = grid.len();
        crate::grid::reveal_grid_disk(grid, target, 2);
        grid.can_walk(from, target) && grid.len() > before
    })
    .unwrap_or(false);
    if changed {
        let grid = crate::server::grid_manager::get_runtime_square_grid()
            .ok_or_else(|| anyhow::anyhow!("grid not loaded"))?;
        store.write().sync_places_from_grid(&grid)?;
    }
    Ok(changed)
}

/// 組裝正方格 RoomView（當前格的實體、出口、物件）。
pub fn get_grid_room_view(
    _player_id: &str,
    x: i32,
    y: i32,
    game_hour: i32,
) -> anyhow::Result<Option<RoomView>> {
    let grid = crate::server::grid_manager::get_runtime_square_grid()
        .ok_or_else(|| anyhow::anyhow!("grid not loaded"))?;
    let coord = SquareCoord::new(x, y);
    let Some(cell) = grid.get(coord) else {
        return Ok(None);
    };
    let cell_objects = if !grid.has_explicit_cell(coord) {
        crate::grid::r#gen::generate_cell_objects(coord, cell.terrain)
    } else {
        cell.objects.clone()
    };
    let mut room_objs = grid_objects_to_room_objects(&cell_objects);
    room_objs.extend(super::player_scene::player_layer_scene_objects_for_room(
        &cell.name,
    ));
    let mut room = Room {
        id: grid_room_id_from_coord(x, y),
        name: if cell.name.is_empty() {
            crate::grid::terrain_name_zh(cell.terrain).to_string()
        } else {
            cell.name.clone()
        },
        tags: cell.tags.clone(),
        zone: cell.zone.clone(),
        description: cell.description.clone(),
        objects: room_objs.clone(),
    };
    let mut exits: Vec<Exit> = Vec::new();
    if !cell.explicit_exits.is_empty() {
        for ex in &cell.explicit_exits {
            let target_name = grid
                .get(ex.target)
                .map(|tc| {
                    if tc.name.is_empty() {
                        crate::grid::terrain_name_zh(tc.terrain).to_string()
                    } else {
                        tc.name.clone()
                    }
                })
                .unwrap_or_else(|| "▒▒▒".to_string());
            exits.push(Exit {
                direction: ex.dir.label_zh().to_string(),
                to_room_id: grid_room_id_from_coord(ex.target.x, ex.target.y),
                to_room_name: target_name,
                route_id: String::new(),
                mode: String::new(),
                travel_cost: 0,
            });
        }
    } else {
        for n_coord in coord.neighbors() {
            let Some(nc) = grid.get(n_coord) else {
                continue;
            };
            if !grid.can_walk(coord, n_coord) {
                continue;
            }
            let Some(dir) = coord.direction_to(n_coord) else {
                continue;
            };
            let target_name = if nc.name.is_empty() {
                crate::grid::terrain_name_zh(nc.terrain).to_string()
            } else {
                nc.name.clone()
            };
            exits.push(Exit {
                direction: dir.label_zh().to_string(),
                to_room_id: grid_room_id_from_coord(n_coord.x, n_coord.y),
                to_room_name: target_name,
                route_id: String::new(),
                mode: String::new(),
                travel_cost: 0,
            });
        }
    }
    let mut entities = filter_room_entities(db::get_entities_at_grid(x, y, game_hour)?);
    if !grid.has_explicit_cell(coord)
        && !matches!(
            cell.terrain,
            crate::grid::Terrain::Wall | crate::grid::Terrain::WaterDeep
        )
        && (x.abs() > 18 || y.abs() > 18)
        && !entities.iter().any(|e| e.kind == crate::entity::EntityKind::Npc)
    {
        let is_road = room.name.contains("道路")
            || room.name.contains("木橋")
            || cell.terrain == crate::grid::Terrain::Road
            || cell.terrain == crate::grid::Terrain::Bridge;
        // 先檢查此座標是否為全島 1,805 隻《台灣山海經》全域棲地分布表的即時駐點（確保上帝視角地圖點位與房間內實體 100% 同步）
        let time_bucket = (crate::game::now_unix() / 180) as i64;
        let (zone_name, _, _) = crate::grid::r#gen::classify_wilderness_cell(x, y);
        let mut matched_live_slot = false;
        if !is_road
            && let Some(z_idx) = CANONICAL_HUNTING_ZONES
                .iter()
                .position(|(zn, _, _)| *zn == zone_name)
        {
            let slots = live_zone_shanhai_slots(z_idx, time_bucket);
            for sl in slots.iter().filter(|s| s.x == x && s.y == y) {
                if sl.tier == 1 && !is_apex_beast_tracked_at(x, y) {
                    if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                        room_objs.push(RoomObject {
                            id: "shanhai_apex_trace".to_string(),
                            name: "新鮮霸主獸跡".to_string(),
                            owner: String::new(),
                            sockets: vec!["Look".to_string()],
                            responses: std::collections::HashMap::new(),
                            move_to_room_id: String::new(),
                        });
                    }
                } else if let Some(mut amb) = ensure_ambient_npc_at_grid(x, y, sl.tier_tag) {
                    amb.id = format!("shanhai_{}_{}_{}_{}", z_idx, sl.slot_idx, x, y);
                    if !entities.iter().any(|e| e.display_title == amb.display_title) {
                        entities.push(amb);
                    }
                }
                matched_live_slot = true;
            }
        }

        if !matched_live_slot {
            // 霸主威壓屏息格（約 14% 獵場腹地）：尋常小怪驚散匿跡，地上留有【新鮮霸主獸跡】，查探後方會破霧顯形
            let silence_hash = ((x as i64 * 31337) ^ (y as i64 * 65521)).unsigned_abs() % 7;
            if !is_road && is_apex_beast_tracked_at(x, y) {
                if let Some(amb) = ensure_ambient_npc_at_grid(x, y, "__apex__") {
                    entities.push(amb);
                }
            } else if !is_road && silence_hash == 0 {
                if !room_objs.iter().any(|o| o.id == "shanhai_apex_trace") {
                    room_objs.push(RoomObject {
                        id: "shanhai_apex_trace".to_string(),
                        name: "新鮮霸主獸跡".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string()],
                        responses: std::collections::HashMap::new(),
                        move_to_room_id: String::new(),
                    });
                }
            } else {
                // 尋常物種與山海小怪：隨時間片在格內動態游走顯形（約 72% 野外格可見小怪或中型獸群活動，28% 為草木藏跡格）
                let roam_gate = ((x as i64).wrapping_mul(1103515245)
                    ^ (y as i64).wrapping_mul(12345)
                    ^ time_bucket.wrapping_mul(2654435761))
                    .unsigned_abs()
                    % 100;
                if is_road || roam_gate < 72 {
                    if let Some(amb) = ensure_ambient_npc_at_grid(x, y, &room.name) {
                        entities.push(amb);
                    }
                    // 獸群／小怪群聚地塊（約 28% 野外地塊有結群伴生小怪或同群獸類）
                    if !is_road && roam_gate < 28 {
                        let flock_tag = if roam_gate % 2 == 0 { "__tier4a__" } else { "__tier4b__" };
                        if let Some(mut companion) = ensure_ambient_npc_at_grid(x, y, flock_tag) {
                            companion.id = format!("ambient_flock_{x}_{y}");
                            if !entities.iter().any(|e| e.display_title == companion.display_title) {
                                entities.push(companion);
                            }
                        }
                    }
                }
            }
        }
    }
     let vendor_objs = super::player_scene::dynamic_vendor_objects_for_entities(&entities);
     for v_obj in vendor_objs {
         if !room_objs.iter().any(|o| o.id == v_obj.id) {
             room_objs.push(v_obj);
         }
     }
     room.objects = room_objs.clone();
     let objects = room_objs;
     Ok(Some(RoomView {
         room,
         exits,
         entities,
         objects,
     }))
 }
 
 /// 確保方格座標 (x, y) 有駐場環境人物（若該處尚無活體人物且非水域、非城牆）。
pub fn ensure_ambient_npc_at_grid(x: i32, y: i32, room_name: &str) -> Option<Character> {
    let (zone, _sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if terrain == crate::grid::Terrain::WaterDeep || terrain == crate::grid::Terrain::Wall {
        return None;
    }

    let time_bucket = (crate::game::now_unix() / 300) as i64;
    let seed = ((x as i64).wrapping_mul(374761393)
        ^ (y as i64).wrapping_mul(668265263)
        ^ time_bucket.wrapping_mul(1442695040888963407))
        .unsigned_abs();
    // 三層生態顯形分配：
    // - Tier 1 鎮場霸主（roll == 0）：預設隱匿不直接站樁，唯有尋跡觸發（room_name == "__apex__" 或已標記尋跡）方破霧顯形
    // - Tier 4 尋常小怪與靈禽（roll == 3, 4）：佔顯形遭遇 65%（野兔、穿山甲、獼猴、飛鼠、環頸雉、竹雞、招潮蟹、青毛蟹等直接顯形於格內）
    // - Tier 3 尋常中型獸群（roll == 2）：佔顯形遭遇 20%（梅花鹿、水鹿、山羌、長鬃山羊、山豬成群游走顯形）
    // - Tier 2 掠食者（roll == 1）：佔 7%
    // - 山道獵手／採集人（roll == 5）：佔 8%
    let roll = if room_name == "__apex__" || is_apex_beast_tracked_at(x, y) {
        0
    } else if room_name == "__tier2__" {
        1
    } else if room_name == "__tier3__" {
        2
    } else if room_name == "__tier4a__" {
        3
    } else if room_name == "__tier4b__" {
        4
    } else {
        match seed % 100 {
            0..=34 => 3,  // 35% 尋常小型奇獸／甲殼小怪（格內顯形）
            35..=64 => 4, // 30% 尋常山海靈禽小怪（格內顯形）
            65..=84 => 2, // 20% 尋常中型主力獸群（格內顯形）
            85..=91 => 1, // 7% 掠食猛獸
            _ => 5,       // 8% 野外獵手／採集人
        }
    };
    let is_road = !room_name.starts_with("__")
        && (room_name.contains("道路")
            || room_name.contains("木橋")
            || terrain == crate::grid::Terrain::Road
            || terrain == crate::grid::Terrain::Bridge);

    let (creature_name, creature_activity) = if is_road {
        match roll {
            0 => ("行路行商", "正挑著布包雜貨在路旁稍作歇息"),
            1 => ("巡道鄉勇", "按著腰刀警戒四周路況"),
            2 => ("引路長鬃岩羊", "沿著路旁石壘警覺地覓食"),
            3 => ("路旁山羌", "自路側林隙投來警覺平靜的目光"),
            4 => ("林緣竹雞", "在道路草邊低頭啄食草籽並拍動翅膀"),
            _ => ("趕路挑夫", "肩挑竹簍草鞋輕快地疾步前行"),
        }
    } else {
        match zone {
            "長風草甸" => match roll {
                0 => ("長風四目水鹿王", "昂首聳立於草甸深處，雙叉巨角直指蒼穹，四目靈光威震四野"),
                1 => ("淺山錢紋石虎", "伏行於長風草甸灌叢陰影間，耳後白斑微動、伺機撲擊野兔"),
                2 => ("平原梅花鹿", "優雅地在草甸花叢間結群嚼草，白色梅花斑紋隨風微動"),
                3 => ("草甸台灣野兔", "四足如飛，在齊膝芒草間迅疾掠過帶起陣陣草浪"),
                4 => ("草甸環頸雉", "披著絢麗五彩羽衣在芒草中昂首啼鳴，振翅破風"),
                _ => ("放牧老獵手", "蹲在土坡上查看草甸獸跡"),
            },
            "古河巨木林" => match roll {
                0 => ("古河樟林雲豹", "無聲無息潛行於千年古樟巨枝之間，暗影中古玉雲紋隱現"),
                1 => ("錢紋石虎", "踏在千年古樟厚軟腐葉層上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("四目台灣水鹿", "生有雙重眶下靈腺，在巨木霧氣中結群踏水而行"),
                3 => ("古林台灣獼猴", "攀附於千年楠木藤蔓間靈巧擺盪，凝神張望林下動靜"),
                4 => ("巨木領角鴞", "棲息於高枝樹洞旁，夜色中金黃雙瞳凝視林間落葉"),
                _ => ("巡林斥候", "倚著巨木樹幹辨認獸徑"),
            },
            "赤石坡" => match roll {
                0 => ("赤石鯪鯉穿山甲", "盤踞於赤紅土石礦坡，周身烏金厚鱗在烈日下泛著鐵光"),
                1 => ("淺山錢紋石虎", "伏行於赤石灌木叢陰影間，耳後白斑微動、目光如電"),
                2 => ("赤崖長鬃山羊", "腳踏垂直陡峭的赤褐色岩壁，如履平地凌空躍動"),
                3 => ("赤坡台灣山羌", "在赤石坡蕨草間警覺昂首，發出清亮短促的吠鳴"),
                4 => ("赤崖大冠鷲", "展開黑白條紋巨翼在赤石坡高空乘熱氣流盤旋長鳴"),
                _ => ("巡山採石人", "手持鐵鑿觀察崖壁礦脈"),
            },
            "落日岬" => match roll {
                0 => ("落日黑鳶王", "逆著落日海風在岬角萬仞高空盤旋，尾羽如魚尾開合控風"),
                1 => ("海岬百年綠蠵龜", "伏於落日殘照的岬角礫灘，背甲印刻著百載海潮紋理"),
                2 => ("海崖長鬃山羊", "於狂暴海風撕扯的岬角懸崖邊緣穩健覓食耐鹽灌木"),
                3 => ("烏腳香白鼻心", "穿梭於海岬防風林與林投果叢間，鼻端白線分明"),
                4 => ("迎風海角雕", "逆著浩瀚西風在海角高空盤旋，神目如電巡視汪洋"),
                _ => ("觀潮老漁民", "遠眺西海浪潮動向"),
            },
            "驚濤岩岸" => match roll {
                0 => ("驚濤遊隼王", "立於怒濤飛濺的玄武岩尖端，俯衝速度冠絕百禽、破浪裂風"),
                1 => ("裂浪灰海獺", "順著拍岸碎浪滑入急流，在礁石暗湧間靈動捕魚"),
                2 => ("斷崖長鬃山羊", "踏立於海浪拍擊的險峻岩岸峭壁，迎著鹹霧穩立不搖"),
                3 => ("潮間青岩巨蟹", "披著青黑重甲駐守於潮池裂罅，雙螯張合鏗鏘作響"),
                4 => ("驚濤黑岩鷺", "羽色深沉如玄武礁岩，單足凝立於碎浪礁石上伺機啄魚"),
                _ => ("海防望海卒", "立於高岩上注視海平線"),
            },
            "巨藻淺灣" => match roll {
                0 => ("淺灣百年綠蠵龜", "悠然浮游於隨潮搖曳的巨藻林間，背甲泛著翠綠海藻幽光"),
                1 => ("渡海黑面琵鷺", "在淺灣潮間帶優雅左右掃動黑長扁喙，成群涉水覓食"),
                2 => ("淺灣灰海獺", "胸口抱著青石在浮動巨藻叢中翻身仰游，悠閒自得"),
                3 => ("潮汐招潮蟹", "高舉鮮紅大螯在淺灣泥沙灘上整齊揮舞，隨潮水退湧進退"),
                4 => ("白鷺候鳥", "單足凝立於淺灣清流白浪之中，凝視游魚伺機啄擊"),
                _ => ("收藻採集工", "手持木叉整理漂岸海藻"),
            },
            "黑石灣" => match roll {
                0 => ("黑石成雙海鱟", "身披青銅馬蹄硬甲成對伏於黑石潮池，劍尾如戟古意盎然"),
                1 => ("黑背玄武岩龜", "伏在黝黑巨石上吐納海霧，背甲深沉如萬年古墨"),
                2 => ("玄武岩大海蟹", "藏身於黑石灣潮間玄武岩裂隙，甲殼堅硬如鐵石"),
                3 => ("浪蝕九孔鮑", "如鐵盔般緊緊吸附於黑石浪痕之上，殼表珍珠層生輝"),
                4 => ("白腹鰹鳥", "自黑石海灣上空收翼垂直扎入浪花，精準捕食潮間游魚"),
                _ => ("曬鹽老工匠", "巡視黑石隙間的粗鹽析出狀況"),
            },
            "翠冠巨木林" => match roll {
                0 => ("千年樟雲豹王", "立於最高樹冠交疊處俯瞰整座山谷，周身雲紋隱隱發光"),
                1 => ("錢紋石虎", "踏在林下厚軟苔原上悄然潛行，雙耳白斑靈動轉動"),
                2 => ("巨角巡林水鹿", "頭戴分叉如戟的巨型長角，沉穩邁過翠綠灌木叢"),
                3 => ("烏腳香白鼻心", "順著老樟樹皮輕巧滑下，鼻端白線分明，自帶草果幽香"),
                4 => ("丹冠藍腹鷴", "寶藍色羽翼拖著雪白長尾，赤足朱冠在巨木林蔭下踱步"),
                _ => ("守林老藥農", "背著藥簍查看林下草藥生長"),
            },
            "清泉溪谷" => match roll {
                0 => ("七家灣櫻花鉤吻鮭", "冰河孑遺國寶靈魚逆著融雪清泉奮力躍水，身側梅花斑紋熠熠生輝"),
                1 => ("溪澗食蟹獴", "披著棕灰簑衣長毛在清泉卵石灘間敏捷穿梭，破殼取蟹"),
                2 => ("清泉台灣山羌", "自溪邊蕨草叢探出小巧頭顱，低頭飲用冰涼泉水"),
                3 => ("清溪青毛蟹", "螯足生滿濃密青黑絨毛，在清澈見底的卵石溪流中側行"),
                4 => ("溪澗鉛色水鶇", "停在溪心濕石上輕巧展開栗紅尾羽，隨流水聲清脆鳴唱"),
                _ => ("汲水挑夫", "在泉眼旁注滿水囊整裝待發"),
            },
            "北坡雲杉林" => match roll {
                0 => ("胸白台灣月熊王", "人立於百年雲杉之下發出渾厚咆哮，胸口一抹新月白毫如雪"),
                1 => ("雲杉霧林帝雉", "披著深藍光澤羽衣在雲杉冷霧間緩步巡行，氣度如王者"),
                2 => ("剛鬃台灣山豬", "粗壯如石的獠牙向上挑起，前蹄刨掘著雲杉林下堅根"),
                3 => ("北坡台灣獼猴", "掛在雲杉垂枝上抓耳撓腮，朝樹下警覺張望"),
                4 => ("高山赫氏熊鷹", "展開寬闊黑褐花翎，傲視千山盤旋於雲杉密林之上"),
                _ => ("伐木巡山客", "踏著積雪查看雲杉材木"),
            },
            "西麓林道" => match roll {
                0 => ("西麓丹冠藍腹鷴", "朱紅肉冠配寶藍長羽，在西麓古道晨霧中從容踱步"),
                1 => ("長尾台灣藍鵲", "拖著琉璃藍長尾成群掠過林道樹冠，彼此呼應守望"),
                2 => ("林道台灣山羌", "在古道兩側闊葉林蔭下輕步覓食嫩葉，聞聲即躍入林"),
                3 => ("食蛇箱龜", "背甲金黃隆起如盾，從容不迫地在碎石落葉間挪行"),
                4 => ("山林竹雞", "在林道兩側密竹叢間結伴啄食草籽，拍翅低飛"),
                _ => ("護林守衛", "按著腰間佩刀沿林道巡邏"),
            },
            "淺澤泥口" => match roll {
                0 => ("淺澤台灣水牛王", "雙角寬闊如彎月，半身浸在涼爽泥澤中閉目反芻，氣息沉穩如山"),
                1 => ("沼澤食蟹獴", "咬著泥蟹在蘆葦泥渚間翻滾滑行，皮毛防濕油亮"),
                2 => ("泥口柴棺水龜", "背甲呈古銅褐色，成群靜伏於淺澤浮木上曬日吐納"),
                3 => ("紅螯泥沼蟹", "高舉血紅巨螯在泥孔外威嚇對手，吐著細密水泡"),
                4 => ("凌波水雉", "拖著修長飄逸尾羽輕踏菱角浮葉之上，宛若凌波仙禽"),
                _ => ("涉澤行腳客", "手拄長竹竿探路避開暗沼"),
            },
            "白茫洲" => match roll {
                0 => ("大武百步蛇王", "盤踞於迷霧泥澤枯木之上，三角巨首與菱紋鱗甲透出凜然古靈威壓"),
                1 => ("黑沼飯匙倩", "在白茫洲水霧草叢中昂起前半身，頸部張開如飯匙般威懾四方"),
                2 => ("白茫四目水鹿", "通體沾滿晨霧水珠，頭頂雙叉長角踏過泥水無痕"),
                3 => ("翡翠綠樹蛙", "背脊閃爍翠玉光澤，靜靜蹲在濕潤水芋葉片上鳴叫"),
                4 => ("沼洲夜鷺", "縮頸佇立於迷霧淺灘枯木上，赤紅眼睛穿透重霧"),
                _ => ("採苔老採集人", "手持防瘴草團辨識霧中去向"),
            },
            "貼山深潭" => match roll {
                0 => ("寒潭千年花斑鱸鰻", "碗口粗的花斑巨軀潛伏於深碧潭底石洞，游動時捲起暗湧漩渦"),
                1 => ("高山苦花魚王", "銀光閃爍躍出寒潭激流，腹部金線在陽光下熠熠生輝"),
                2 => ("潭畔四目水鹿", "立於深潭岸邊青石上結伴俯身飲水，長角倒映於碧綠潭面"),
                3 => ("阿里山山椒魚", "冰河孑遺靈獸伏於寒潭水際冷石下，褐膚帶淺斑、喜極寒清泉"),
                4 => ("寒潭翠鳥", "披著寶石藍羽衣自潭邊蘆葦疾射入水，銜起銀魚破水而出"),
                _ => ("潭邊垂釣隱士", "握竿靜坐潭石觀照水深"),
            },
            "青籠關" => match roll {
                0 => ("青籠大冠鷲王", "傲立於青籠關千仞險峰之巔，羽冠高聳、長鳴聲震動整條南境山隘"),
                1 => ("關隘錢紋石虎", "潛伏在關道荊棘石縫暗處，矯健斑紋身軀蓄勢待發"),
                2 => ("剛鬃台灣山豬", "粗壯獠牙泛著冷光，在關隘山坳間拱土覓食"),
                3 => ("鯪鯉穿山甲", "捲成堅不可摧的烏金鱗球滾過山隘石坡，刀劍難傷"),
                4 => ("關隘長鬃山羊", "敏捷踏過險峻關石窄徑，灰色長鬃在山風中飛舞"),
                _ => ("戍關巡哨隊員", "按矛立於要衝處檢視往來行跡"),
            },
            "西嶺礦谷" => match roll {
                0 => ("裂石鯪鯉穿山甲王", "全身烏金鱗甲堅硬如百鍛玄鐵，利爪一揮便刨開堅硬赤鐵礦岩"),
                1 => ("礦谷胸白月熊", "在青脊山西側深谷岩洞前巡視領地，胸前新月白斑清晰奪目"),
                2 => ("青脊長鬃山羊", "攀行於近乎垂直的露頭礦脈岩壁上，蹄尖穩扣寸許石縫"),
                3 => ("礦谷台灣山豬", "長吻帶泥在煤石堆旁衝撞覓食，渾身沾滿硬結礦泥"),
                4 => ("高山金翼白眉", "眉間白紋配金橙翼斑，在礦谷冷杉枝頭清脆吟唱"),
                _ => ("馱煤採礦工", "整理背架上的粗礦石準備啟程"),
            },
            "東海峭壁" => match roll {
                0 => ("馭風赫氏熊鷹王", "凌空盤旋俯瞰東海萬頃波浪，雙翼遮雲蔽日、頭頂赫氏角冠威凜"),
                1 => ("破浪遊隼", "自千仞海崖順風俯衝，雙翼收攏如玄鐵利箭掠過海面"),
                2 => ("斷崖長鬃山羊", "腳踏險絕清水斷崖上的寸許石階，成群從容縱躍如飛"),
                3 => ("峭壁大赤飛鼠", "自海風呼嘯的懸崖樹梢縱身一躍，乘著上升氣流滑向對岸"),
                4 => ("清水岩鷚", "羽色如花崗岩斑駁，在臨海峭壁石隙中逆風跳躍覓食"),
                _ => ("攀崖採燕客", "身繫麻繩在崖壁凹處調勻呼吸"),
            },
            "雲腰雪林" => match roll {
                0 => ("霧林黑長尾帝雉", "披著深藍曜黑羽衣拖著雪白橫紋長尾，漫步於玉山圓柏與箭竹積雪中"),
                1 => ("雪線胸白月熊", "頂著嚴寒風雪在鐵杉雪林中巡狩，掌力能折斷百年冰枝"),
                2 => ("踏雪四目水鹿", "在寒霧積雪中安詳漫步，四目顧盼清明出塵"),
                3 => ("阿里山山椒魚", "潛藏於雪林融冰石縫之下，耐受極寒冰泉"),
                4 => ("雪林酒紅朱雀", "全身羽色如陳年紅酒般鮮豔，在覆雪鐵杉枝頭宛若紅梅綻放"),
                _ => ("高山踏雪獵戶", "身裹獸皮斗篷檢視捕獸夾"),
            },
            "白塔雪巔" => match roll {
                0 => ("雪巔霜冠大水鹿", "傲然立於冰封絕頂石柱之巔，霜白巨角如玉樹臨風、吐納天地至純清氣"),
                1 => ("極頂黑長尾帝雉", "在萬仞雪峰雲海邊緣漫步，長尾白紋與萬年玄冰交相輝映"),
                2 => ("雪脊長鬃山羊", "蹄踏玄冰雪脊，在萬丈虛空之上從容齧食高山冰苔"),
                3 => ("雪山冰河山椒魚", "棲息於白塔極頂冰川融水石隙，見證萬載寒暑"),
                4 => ("雪峰岩鷚", "羽如灰石斑駁，在冰封岩頂上不懼酷寒歡快啄食細雪草籽"),
                _ => ("朝山苦修者", "身披粗布在寒風中合十默禱"),
            },
            _ => match roll {
                0 => ("山林梅花鹿", "警覺地站在草丘上環顧四周風吹草動"),
                1 => ("烏腳香白鼻心", "在矮灌木叢中靈巧鑽動尋找甜美野果"),
                2 => ("雜林野灰兔", "在草葉間抽動鼻尖，後肢蹬地飛快奔竄"),
                3 => ("山林竹雞", "在路旁草葉間結隊咕咕低鳴覓食草籽"),
                4 => ("草叢龜殼花", "盤曲於厚軟草叢陰影間，冷眼凝視過往生靈"),
                _ => ("野地採集人", "蹲在路旁整理藥草袋"),
            },
        }
    };

    let display_char = creature_name.chars().next().map(|c| c.to_string()).unwrap_or_default();

    let (vit, qi, dex) = if is_road {
        match roll {
            2 | 3 => (30, 15, 20),
            4 => (16, 10, 24),
            _ => (25, 15, 16),
        }
    } else {
        match roll {
            0 => (120, 60, 30), // Tier 1 Apex
            1 => (55, 28, 26),  // Tier 2 Predator
            2 => (30, 15, 20),  // Tier 3 Medium Game
            3 => (16, 10, 24),  // Tier 4 Small Fauna
            4 => (18, 12, 28),  // Tier 4 Avian
            _ => (25, 15, 16),  // Human NPC
        }
    };
    Some(Character {
        id: format!("ambient_{}_{}", x, y),
        kind: EntityKind::Npc,
        display_char,
        x,
        y,
        move_state: crate::entity::MoveState::Idle,
        target_x: None,
        target_y: None,
        walk_or_run: crate::entity::WalkOrRun::Walk,
        move_started_at: None,
        vit,
        qi,
        dex,
        magnesium: 0,
        last_observed_at: Some(crate::game::now_unix()),
        created_at: crate::game::now_unix(),
        gender: None,
        soul_seed: None,
        display_title: creature_name.to_string(),
        activated_nodes: "[]".to_string(),
        equipment_slots: "{}".to_string(),
        inventory: "[]".to_string(),
        disposition: 0,
        thoughts: vec![],
        current_activity: {
            let gh = crate::store::current_game_hour();
            let mut act = creature_activity.to_string();
            if matches!(
                creature_name,
                "夜嘯大赤飛鼠" | "烏腳香白鼻心" | "錢紋石虎" | "石隙雪鴞" | "沼洲夜鷺" | "峭壁飛鼠"
            ) && !(6..18).contains(&gh)
            {
                act.push_str("（夜行時辰：正乘著夜色幽暗無聲掠出，雙目在月華下爍爍生輝）");
            } else if matches!(
                creature_name,
                "逐日七叉白鹿" | "霧林黑長尾帝雉" | "丹冠藍腹鷴" | "白茫角鹿" | "踏雪四目水鹿"
            ) && (5..9).contains(&gh)
            {
                act.push_str("（晨霧瑞兆：正迎著破曉山嵐吐納朝露，靈氣逼人）");
            } else if matches!(
                creature_name,
                "黑水溝大王烏賊" | "深海皇帶龍魚" | "載嶼百年巨鼇" | "噴玉海翁巨鯨" | "海岬豆腐鯨鯊"
            ) && gh % 6 <= 1
            {
                act.push_str("（大潮現身：正乘著黑水潮汐退湧之際浮現海面，引動暗流轟鳴）");
            }
            act
        },
        grid_x: Some(x),
        grid_y: Some(y),
    })
}

/// 將山海經環境生物名稱映射至千物總典物品 ID、名稱與《台灣山海經》典籍描述。
pub fn ambient_creature_loot(creature_name: &str) -> Option<(&'static str, &'static str, &'static str)> {
    if creature_name.ends_with('客')
        || creature_name.ends_with('人')
        || creature_name.ends_with('手')
        || creature_name.ends_with('民')
        || creature_name.ends_with('卒')
        || creature_name.ends_with('工')
        || creature_name.ends_with('農')
        || creature_name.ends_with('夫')
        || creature_name.ends_with('衛')
        || creature_name.ends_with('士')
        || creature_name.ends_with('員')
        || creature_name.ends_with('戶')
        || creature_name.ends_with('者')
        || creature_name.ends_with('商')
        || creature_name.ends_with('勇')
    {
        return None;
    }
    if creature_name.contains("帝雉") {
        Some(("wt_cat_0214", "霧林帝雉長尾翎", "《台灣山海經》：烏雉生於雲霧鐵杉之巔，尾長三尺而橫白紋。"))
    } else if creature_name.contains("藍腹鷴") {
        Some(("wt_cat_0216", "丹冠藍腹鷴彩羽", "《台灣山海經》：丹冠白尾鷴羽泛寶藍幽光，見之吉祥。"))
    } else if creature_name.contains("豹") {
        Some(("wt_cat_0172", "樟林雲豹軟毛皮", "《台灣山海經》：雲豹身披古玉雲紋，佩其皮則山行不躓。"))
    } else if creature_name.contains("熊") {
        Some(("wt_cat_0205", "胸白月熊厚熊掌", "《台灣山海經》：月熊胸生白毫如新月，其掌食之禦風寒。"))
    } else if creature_name.contains("穿山甲") || creature_name.contains("鯪鯉") {
        Some(("wt_cat_0188", "鯪鯉穿山甲鱗片", "《台灣山海經》：鯪鯉遍體烏金硬鱗，炙之通經絡、綴甲禦刃。"))
    } else if creature_name.contains("蛇") || creature_name.contains("蟒") || creature_name.contains("蝮") {
        Some(("wt_cat_0189", "大武百步蛇乾蛻", "《台灣山海經》：百步大蛇首如三角鐵鏟，其蛻研粉可解瘴毒。"))
    } else if creature_name.contains("水鹿") || creature_name.contains("白鹿") || creature_name.contains("梅花鹿") || creature_name.contains("角鹿") || creature_name.contains("鹿") {
        Some(("wt_cat_0203", "高地水鹿厚鹿皮", "《台灣山海經》：四目靈鹿生於高山溪谷，其皮堅韌耐水火。"))
    } else if creature_name.contains("山羌") || creature_name.contains("羌") {
        Some(("wt_cat_0209", "山羌柔軟麂皮", "《台灣山海經》：山羌鳴聲如犬，其皮細膩溫軟。"))
    } else if creature_name.contains("羊") || creature_name.contains("羚") {
        Some(("wt_cat_0179", "長鬃山羊彎角", "《台灣山海經》：長鬃岩羚善登千仞絕壁，其角堅韌可為號角。"))
    } else if creature_name.contains("地牛") || creature_name.contains("牛") {
        Some(("wt_cat_0187", "大角原牛厚蹄皮", "《台灣山海經》：巨牛伏行草甸與地脈，蹄皮厚韌冠絕百獸。"))
    } else if creature_name.contains("豬") {
        Some(("wt_cat_0211", "海崖黑野豬牙", "《台灣山海經》：剛鬃山豬力能拱石，其獠牙彎如新月。"))
    } else if creature_name.contains("飛鼠") || creature_name.contains("鼯") {
        Some(("wt_cat_0182", "大赤飛鼠厚絨毛", "《台灣山海經》：赤鼯夜飛巨木之間，其絨輕暖勝狐裘。"))
    } else if creature_name.contains("白鼻心") || creature_name.contains("狐") || creature_name.contains("獾") || creature_name.contains("麝") {
        Some(("wt_cat_0183", "白鼻心烏腳香皮", "《台灣山海經》：烏腳香善棲樟楠果林，自帶草果清香。"))
    } else if creature_name.contains("石虎") || creature_name.contains("虎") || creature_name.contains("狼") || creature_name.contains("犬") {
        Some(("wt_cat_0171", "山林石虎斑皮", "《台灣山海經》：石虎耳後生白斑，夜行山林迅捷如電。"))
    } else if creature_name.contains("兔") || creature_name.contains("鼠") {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《台灣山海經》：野兔潛於芒草石縫，其絨護掌不畏寒霜。"))
    } else if creature_name.contains("鱷") || creature_name.contains("沼龍") || creature_name.contains("蜥") {
        Some(("wt_cat_0176", "泥甲巨鱷厚麟皮", "《台灣山海經》：泥甲巨獸伏於腐葉深澤，背麟刀斧難入。"))
    } else if creature_name.contains("烏賊") || creature_name.contains("花枝") {
        Some(("wt_cat_0236", "大花枝鮮黑墨囊", "《台灣山海經》：黑水溝巨鰂眼如銅鑼，墨囊濃黑如漆。"))
    } else if creature_name.contains("鯨") || creature_name.contains("海翁") || creature_name.contains("鯊") || creature_name.contains("海狗") || creature_name.contains("海獺") {
        Some(("wt_cat_0220", "豆腐鯊厚砂魚皮", "《台灣山海經》：海翁巨魚浮海如嶼，其皮粗韌如砂石。"))
    } else if creature_name.contains("龍魚") || creature_name.contains("鬼頭刀") || creature_name.contains("苦花") || creature_name.contains("鯇") || creature_name.contains("鰻") || creature_name.contains("魚") {
        Some(("wt_cat_0224", "銀鱗土魠魚", "《台灣山海經》：外海大魚銀鱗閃爍，食之益氣強身。"))
    } else if creature_name.contains("龜") || creature_name.contains("鼇") || creature_name.contains("蟹") || creature_name.contains("海膽") || creature_name.contains("螺") || creature_name.contains("蛙") || creature_name.contains("蟾") || creature_name.contains("鯢") {
        Some(("wt_cat_0223", "野生九孔鮑", "《台灣山海經》：潮間岩穴甲殼海珍，味極甘鮮。"))
    } else if creature_name.contains("鷹") || creature_name.contains("雕") || creature_name.contains("隼") || creature_name.contains("鴞") {
        Some(("wt_cat_0192", "高山熊鷹翎羽", "《台灣山海經》：赫氏角鷹盤旋絕壁，其翎破空不顫。"))
    } else if creature_name.contains("雉") || creature_name.contains("雞") || creature_name.contains("鳥") || creature_name.contains("鷗") || creature_name.contains("鷺") || creature_name.contains("燕") || creature_name.contains("鷚") {
        Some(("wt_cat_0173", "水鳥蓬鬆絨羽", "《台灣山海經》：山海飛禽絨羽輕柔，禦寒隔濕。"))
    } else if creature_name.contains("草獸") || creature_name.contains("獸") {
        Some(("wt_cat_0185", "灰野兔整張絨皮", "《台灣山海經》：野兔潛於芒草石縫，其絨護掌不畏寒霜。"))
    } else {
        None
    }
}

/// 正典 19 座山海獵場與其深處地脈核心錨點。
pub const CANONICAL_HUNTING_ZONES: &[(&str, i32, i32)] = &[
    ("長風草甸", -180, -195),
    ("古河巨木林", -55, -80),
    ("赤石坡", -85, -180),
    ("落日岬", -320, -180),
    ("驚濤岩岸", -355, 115),
    ("巨藻淺灣", -280, 200),
    ("黑石灣", -280, 5),
    ("白茫洲", -180, -320),
    ("貼山深潭", 120, -260),
    ("淺澤泥口", 20, -180),
    ("青籠關", 0, -450),
    ("西麓林道", -120, 80),
    ("清泉溪谷", 60, 160),
    ("翠冠巨木林", -60, 220),
    ("北坡雲杉林", 80, 280),
    ("西嶺礦谷", 180, 40),
    ("東海峭壁", 320, 200),
    ("雲腰雪林", 180, 320),
    ("白塔雪巔", 0, 420),
];

/// 全島全域自然棲地生靈點位（消除人工同心圓，按 19 獵場真實疆域進行藍噪聲 Poisson-disc 與獸群聚落分布）。
#[derive(Debug, Clone)]
pub struct ShanhaiHabitatSlot {
    pub slot_idx: usize,
    pub base_x: i32,
    pub base_y: i32,
    pub x: i32,
    pub y: i32,
    pub sub_geo: &'static str,
    pub tier: u8,
    pub tier_tag: &'static str,
    pub kind_str: &'static str,
    pub tier_label: &'static str,
    pub act_prefix: &'static str,
}

fn is_inside_settlement_urban_core(x: i32, y: i32) -> bool {
    const SETTLEMENT_CORES: &[(i32, i32, i32)] = &[
        (0, 0, 24),         // 文曜
        (-258, 160, 22),    // 風舶角
        (-348, 160, 16),    // 風舶嶼
        (-120, -135, 18),   // 大圳頭
        (160, 200, 18),     // 半嶺鎮
        (0, -340, 18),      // 南哨
        (0, 120, 14),       // 老堰頭
        (82, 112, 14),      // 北嶺腳
        (180, 120, 14),     // 霜林口
        (-258, 250, 14),    // 風櫃尾
        (-245, 50, 14),     // 烏石鼻
        (-120, -250, 14),   // 牛埔
        (-60, -70, 14),     // 紅土崁
        (-285, -155, 14),   // 三號坑口
        (60, -260, 14),     // 水尾寮
    ];
    SETTLEMENT_CORES
        .iter()
        .any(|&(sx, sy, r)| (x - sx).abs() <= r && (y - sy).abs() <= r)
}

fn habitat_hash(a: i64, b: i64, c: i64) -> u64 {
    let mut h = (a as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((b as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        .wrapping_add((c as u64).wrapping_mul(0x94D0_49BB_1331_11EB));
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}

fn is_valid_wilderness_habitat_cell(x: i32, y: i32, expected_zone: &str) -> Option<&'static str> {
    if !(-392..=392).contains(&x) || !(-492..=492).contains(&y) {
        return None;
    }
    if is_inside_settlement_urban_core(x, y) {
        return None;
    }
    let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(x, y);
    if zone != expected_zone {
        return None;
    }
    if matches!(
        terrain,
        crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall | crate::grid::Terrain::Water
    ) {
        return None;
    }
    Some(sub_geo)
}

type BaseHabitatEntry = (i32, i32, &'static str, u8, &'static str, usize);

static ZONE_HABITAT_LAYOUTS: std::sync::LazyLock<Vec<Vec<BaseHabitatEntry>>> =
    std::sync::LazyLock::new(|| {
        let mut zone_candidates: Vec<Vec<(i32, i32, &'static str)>> =
            vec![Vec::new(); CANONICAL_HUNTING_ZONES.len()];

        // 1. 掃描全島 800x1000 疆域（4 格抖動網格），按 classify_wilderness_cell 歸入 19 獵場真實版圖
        for by in (-488..=468).step_by(4) {
            for bx in (-388..=358).step_by(4) {
                let h = habitat_hash(bx as i64, by as i64, 17);
                let jx = ((h % 3) as i32) - 1;
                let jy = (((h >> 8) % 3) as i32) - 1;
                let gx = bx + jx;
                let gy = by + jy;
                if is_inside_settlement_urban_core(gx, gy) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(gx, gy);
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    zone_candidates[z_idx].push((gx, gy, sub_geo));
                }
            }
        }

        // 針對風舶外島「驚濤岩岸」與狹長海岸帶加密採樣（2 格步距），確保外島與岬角棲地充沛
        for by in (-270..=260).step_by(2) {
            for bx in (-376..=-225).step_by(2) {
                if is_inside_settlement_urban_core(bx, by) {
                    continue;
                }
                let (zone, sub_geo, terrain) = crate::grid::r#gen::classify_wilderness_cell(bx, by);
                if !matches!(zone, "驚濤岩岸" | "落日岬" | "巨藻淺灣" | "黑石灣") {
                    continue;
                }
                if matches!(
                    terrain,
                    crate::grid::Terrain::WaterDeep
                        | crate::grid::Terrain::Wall
                        | crate::grid::Terrain::Water
                ) {
                    continue;
                }
                if let Some(z_idx) = CANONICAL_HUNTING_ZONES
                    .iter()
                    .position(|(zn, _, _)| *zn == zone)
                {
                    if !zone_candidates[z_idx]
                        .iter()
                        .any(|&(ex, ey, _)| ex == bx && ey == by)
                    {
                        zone_candidates[z_idx].push((bx, by, sub_geo));
                    }
                }
            }
        }

        let mut layouts = Vec::with_capacity(CANONICAL_HUNTING_ZONES.len());
        let mut global_chosen: Vec<(i32, i32)> = Vec::with_capacity(1900);

        for (z_idx, &(zone_name, cx, cy)) in CANONICAL_HUNTING_ZONES.iter().enumerate() {
            let cands = &zone_candidates[z_idx];
            let mut entries: Vec<BaseHabitatEntry> = Vec::with_capacity(95);

            // 預先計算每個候選格距獵場邊界／海岸線／城鎮核心的鏡像排斥平方距離上限（防止最遠點採樣貼邊排隊，確保獵場腹地均勻飽滿）
            let cand_boundary_caps: Vec<i64> = cands
                .iter()
                .map(|&(px, py, _)| {
                    const DIRS: &[(i32, i32)] = &[
                        (1, 0),
                        (-1, 0),
                        (0, 1),
                        (0, -1),
                        (1, 1),
                        (-1, 1),
                        (1, -1),
                        (-1, -1),
                    ];
                    for &r in &[4i32, 8, 13, 19, 26, 34] {
                        for &(dx, dy) in DIRS {
                            let tx = px + dx * r;
                            let ty = py + dy * r;
                            if is_inside_settlement_urban_core(tx, ty) {
                                let d2 = ((dx * dx + dy * dy) * r * r) as i64;
                                return d2 * 2;
                            }
                            let (tz, _, tt) =
                                crate::grid::r#gen::classify_wilderness_cell(tx, ty);
                            if tz != zone_name
                                || matches!(
                                    tt,
                                    crate::grid::Terrain::WaterDeep | crate::grid::Terrain::Wall
                                )
                            {
                                let d2 = ((dx * dx + dy * dy) * r * r) as i64;
                                return d2 * 3;
                            }
                        }
                    }
                    4500i64
                })
                .collect();

            // (1) Tier 1 鎮場霸主（1 頭）：駐於獵場深處核心地脈，周圍 2 格設「百鳥屏息圈」
            let (apex_x, apex_y, apex_sub) = if let Some(sub) =
                is_valid_wilderness_habitat_cell(cx + 1, cy - 1, zone_name)
            {
                (cx + 1, cy - 1, sub)
            } else {
                cands
                    .iter()
                    .copied()
                    .min_by_key(|&(x, y, _)| (x - cx).pow(2) + (y - cy).pow(2))
                    .unwrap_or((cx + 1, cy - 1, "深谷"))
            };
            entries.push((apex_x, apex_y, apex_sub, 1, "__apex__", 0));

            let mut chosen_points: Vec<(i32, i32)> = vec![(apex_x, apex_y)];
            global_chosen.push((apex_x, apex_y));

            let pick_next_spread_point =
                |chosen: &mut Vec<(i32, i32)>,
                 all_chosen: &mut Vec<(i32, i32)>,
                 salt: i64|
                 -> (i32, i32, &'static str) {
                    let mut best = (cx + 5, cy - 5, "林地");
                    let mut best_score = -1i64;
                    for (c_idx, &(px, py, psub)) in cands.iter().enumerate() {
                        if (px - apex_x).abs() <= 2 && (py - apex_y).abs() <= 2 {
                            continue;
                        }
                        let min_local_d2 = chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(0);
                        if min_local_d2 == 0 {
                            continue;
                        }
                        let min_global_d2 = all_chosen
                            .iter()
                            .map(|&(ox, oy)| {
                                let dx = (px - ox) as i64;
                                let dy = (py - oy) as i64;
                                dx * dx + dy * dy
                            })
                            .min()
                            .unwrap_or(min_local_d2);
                        let effective_d2 = min_local_d2
                            .min(min_global_d2)
                            .min(cand_boundary_caps[c_idx]);
                        // 加入自然隨機擾動權重（0.65x ~ 1.35x），消除任何機械網格或邊界排隊感
                        let jitter_w = 10 + (habitat_hash(px as i64, py as i64, salt) % 11) as i64;
                        let score = effective_d2 * jitter_w
                            + (habitat_hash(py as i64, px as i64, salt + 99) % 29) as i64;
                        if score > best_score {
                            best_score = score;
                            best = (px, py, psub);
                        }
                    }
                    chosen.push((best.0, best.1));
                    all_chosen.push((best.0, best.1));
                    best
                };

            // (2) Tier 2 次級掠食猛獸／珍獸（6 隻 / 獵場）：分據獵場全境 6 大獨立巡獵領地
            for p_idx in 0..6usize {
                let (px, py, psub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    100 + p_idx as i64,
                );
                entries.push((px, py, psub, 2, "__tier2__", 1 + p_idx));
            }

            // (3) Tier 3 尋常中型獸群（20 隻 / 獵場）：分為 7 處自然放牧／遷徙獸群（6 群x3 隻 + 1 群x2 隻），散佈於全區草甸溪谷
            const HERD_MEMBER_DELTAS: &[(i32, i32)] = &[
                (0, 0),
                (2, 1),
                (-1, 2),
                (1, -2),
                (-2, -1),
                (2, -1),
                (-2, 2),
            ];
            for herd_idx in 0..7usize {
                let (hx, hy, hsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    300 + herd_idx as i64,
                );
                let herd_size = if herd_idx < 6 { 3 } else { 2 };
                for m in 0..herd_size {
                    let (mut mx, mut my, mut msub) = (hx, hy, hsub);
                    if m > 0 {
                        for attempt in 0..HERD_MEMBER_DELTAS.len() {
                            let (dx, dy) = HERD_MEMBER_DELTAS
                                [(m + herd_idx + attempt) % HERD_MEMBER_DELTAS.len()];
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let tx = hx + dx;
                            let ty = hy + dy;
                            if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                                && !chosen_points.contains(&(tx, ty))
                                && let Some(valid_sub) =
                                    is_valid_wilderness_habitat_cell(tx, ty, zone_name)
                            {
                                mx = tx;
                                my = ty;
                                msub = valid_sub;
                                chosen_points.push((mx, my));
                                global_chosen.push((mx, my));
                                break;
                            }
                        }
                    }
                    entries.push((mx, my, msub, 3, "__tier3__", 7 + herd_idx));
                }
            }

            // (4) Tier 4 尋常小怪與山海靈禽（68 隻 / 獵場）：以藍噪聲 Poisson-disc 漫山散佈於獵場全境各角落
            for k in 0..68usize {
                let (fx, fy, fsub) = pick_next_spread_point(
                    &mut chosen_points,
                    &mut global_chosen,
                    700 + k as i64,
                );
                let tier_tag = if k % 2 == 0 {
                    "__tier4a__"
                } else {
                    "__tier4b__"
                };
                entries.push((fx, fy, fsub, 4, tier_tag, 14 + k));
            }

            layouts.push(entries);
        }
        layouts
    });

/// 取得指定獵場（`z_idx in 0..19`）在當前時間片之 95 隻《台灣山海經》生靈即時棲地座標（個體與獸群各自獨立漫遊）。
pub fn live_zone_shanhai_slots(z_idx: usize, time_tick: i64) -> Vec<ShanhaiHabitatSlot> {
    let Some(&(zone_name, _, _)) = CANONICAL_HUNTING_ZONES.get(z_idx) else {
        return Vec::new();
    };
    let base_list = &ZONE_HABITAT_LAYOUTS[z_idx];
    let (apex_x, apex_y) = base_list
        .first()
        .map(|&(x, y, ..)| (x, y))
        .unwrap_or((0, 0));

    let mut out = Vec::with_capacity(base_list.len());
    for (slot_idx, &(bx, by, base_sub, tier, tier_tag, roam_group)) in
        base_list.iter().enumerate()
    {
        let (gx, gy, sub_geo) = if tier == 1 {
            (bx, by, base_sub)
        } else {
            // 每個獸群或獨立個體依 roam_group 產生獨立漫遊向量（-2..=2），絕不全島同步平移
            let rh = habitat_hash(z_idx as i64, roam_group as i64, time_tick);
            let rdx = ((rh % 5) as i32) - 2;
            let rdy = (((rh >> 8) % 5) as i32) - 2;
            let tx = bx + rdx;
            let ty = by + rdy;
            if ((tx - apex_x).abs() > 2 || (ty - apex_y).abs() > 2)
                && let Some(vsub) = is_valid_wilderness_habitat_cell(tx, ty, zone_name)
            {
                (tx, ty, vsub)
            } else {
                (bx, by, base_sub)
            }
        };

        let (kind_str, tier_label, act_prefix) = match tier {
            1 => (
                "shanhai_apex",
                "隱匿鎮場霸主（尋跡顯形）",
                "【隱匿潛伏・百鳥屏息】",
            ),
            2 => (
                "shanhai_predator",
                "顯形次級掠食／珍獸",
                "【格內顯形・伏行巡獵】",
            ),
            3 => (
                "shanhai_herd",
                "顯形尋常中型獸群",
                "【格內顯形・結群游走】",
            ),
            _ => {
                let lbl = if tier_tag == "__tier4a__" {
                    "顯形尋常小怪（奇獸／甲殼）"
                } else {
                    "顯形尋常小怪（靈禽／兩棲）"
                };
                ("shanhai_beast", lbl, "【格內顯形・漫山游走】")
            }
        };

        out.push(ShanhaiHabitatSlot {
            slot_idx,
            base_x: bx,
            base_y: by,
            x: gx,
            y: gy,
            sub_geo,
            tier,
            tier_tag,
            kind_str,
            tier_label,
            act_prefix,
        });
    }
    out
}

/// 為「入山百工」挑選遍布該獵場全境真實疆域的採集／匠作航點（跨度 20~120 格，涵蓋林地、溪谷、礦坡與草藥叢）。
pub fn pick_zone_artisan_waypoint(
    zone_name: &str,
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let z_idx = CANONICAL_HUNTING_ZONES
        .iter()
        .position(|(zn, _, _)| *zn == zone_name)
        .unwrap_or(0);
    let base_list = &ZONE_HABITAT_LAYOUTS[z_idx];
    if base_list.len() > 27 {
        let pool = &base_list[1..]; // 排除 slot 0 鎮場霸主屏息心
        let start = (seed as usize) % pool.len();
        for offset in 0..pool.len() {
            let &(px, py, sub_geo, ..) = &pool[(start + offset * 7) % pool.len()];
            let dist = (px - current.0).abs() + (py - current.1).abs();
            if dist >= 18 {
                return ((px, py), sub_geo);
            }
        }
        let &(px, py, sub_geo, ..) = &pool[start];
        return ((px, py), sub_geo);
    }
    let &(_, cx, cy) = &CANONICAL_HUNTING_ZONES[z_idx];
    ((cx, cy), "深山林地")
}

static ISLAND_ROAD_MILESTONES: std::sync::OnceLock<Vec<(i32, i32, &'static str)>> =
    std::sync::OnceLock::new();

fn get_island_road_milestones() -> &'static [(i32, i32, &'static str)] {
    if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
        return list.as_slice();
    }
    if let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() {
        let mut roads = Vec::new();
        for cell in grid.cells() {
            let coord = cell.coord;
            if (cell.terrain == crate::grid::Terrain::Road
                || cell.terrain == crate::grid::Terrain::Bridge)
                && !is_inside_settlement_urban_core(coord.x, coord.y)
            {
                let (zone, _, _) = crate::grid::r#gen::classify_wilderness_cell(coord.x, coord.y);
                roads.push((coord.x, coord.y, zone));
            }
        }
        roads.sort_by_key(|&(x, y, _)| (y, x));
        // 每隔 4 格取一處古道驛路里程點，確保均勻覆蓋全島 6,287 格驛道與古道網路
        let mut milestones = Vec::new();
        for (idx, item) in roads.into_iter().enumerate() {
            if idx % 4 == 0 {
                milestones.push(item);
            }
        }
        if !milestones.is_empty() {
            let _ = ISLAND_ROAD_MILESTONES.set(milestones);
            if let Some(list) = ISLAND_ROAD_MILESTONES.get() {
                return list.as_slice();
            }
        }
    }
    &[]
}

/// 為「古道行旅（挑夫頭、擔夫、押貨管事、刀牌手、巡隘銃手）」挑選全島驛道與聯外古道網上的真實道路航點。
pub fn pick_island_trail_waypoint(
    zone_name: &str,
    home: (i32, i32),
    errand: (i32, i32),
    seed: u64,
    current: (i32, i32),
) -> ((i32, i32), &'static str) {
    let milestones = get_island_road_milestones();
    if !milestones.is_empty() {
        // 優先挑選屬於該獵場或位於「居所 <-> 獵場 <-> 交割市集」走廊上的真實道路格
        let mid_x = (home.0 + errand.0) / 2;
        let mid_y = (home.1 + errand.1) / 2;
        let local: Vec<(i32, i32, &'static str)> = milestones
            .iter()
            .copied()
            .filter(|&(rx, ry, rzone)| {
                rzone == zone_name
                    || ((rx - mid_x).abs() <= 110 && (ry - mid_y).abs() <= 110)
                    || ((rx - home.0).abs() <= 95 && (ry - home.1).abs() <= 95)
            })
            .collect();
        let pool = if local.len() >= 6 {
            local.as_slice()
        } else {
            milestones
        };
        let start = (seed as usize) % pool.len();
        for step in 0..pool.len() {
            let (rx, ry, rzone) = pool[(start + step * 5) % pool.len()];
            let dist = (rx - current.0).abs() + (ry - current.1).abs();
            if (16..=140).contains(&dist) {
                return ((rx, ry), rzone);
            }
        }
        let (rx, ry, rzone) = pool[start];
        return ((rx, ry), rzone);
    }
    pick_zone_artisan_waypoint(zone_name, seed, current)
}

/// 19 大獵場首領異獸威壓與殘跡異兆（若該地帶或場景對應既定獵場）。
pub fn hunting_zone_apex_omen(zone_or_room: &str) -> Option<&'static str> {
    if zone_or_room.contains("長風草甸") {
        Some("【山海異兆】草浪間隱現深深蹄印與磨角折斷的芒草叢，長風四目水鹿王與梅花鹿群奔踏而過的原野靈氣猶在。")
    } else if zone_or_room.contains("古河巨木林") {
        Some("【山海異兆】千年古樟樹幹三丈高處留有寸深的新鮮爪痕，古河樟林雲豹潛行於樹冠殘存的威壓令林間百鳥屏息。")
    } else if zone_or_room.contains("赤石坡") {
        Some("【山海異兆】赤紅土石礦坡旁留有深邃掘洞與梅花狀貓科足印，赤石鯪鯉穿山甲與淺山錢紋石虎在此交錯出沒。")
    } else if zone_or_room.contains("落日岬") {
        Some("【山海異兆】落日餘暉下的海岬高空盤旋著落日黑鳶王的魚尾剪影，礫灘上留有百年綠蠵龜爬行入海的寬闊軌跡。")
    } else if zone_or_room.contains("驚濤岩岸") {
        Some("【山海異兆】怒濤拍擊的玄武岩尖端傳來破空銳嘯，驚濤遊隼王與裂浪灰海獺在狂風巨浪間各自稱雄。")
    } else if zone_or_room.contains("巨藻淺灣") {
        Some("【山海異兆】隨潮搖曳的巨藻林間浮現百年綠蠵龜的翠綠背甲，淺灘上成群黑面琵鷺正迎著潮水展翅。")
    } else if zone_or_room.contains("黑石灣") {
        Some("【山海異兆】玄武岩潮池中成雙成對的黑石海鱟披著青銅古甲緩步挪移，劍尾在黑石沙痕上劃出千年印記。")
    } else if zone_or_room.contains("翠冠巨木林") {
        Some("【山海異兆】老樟樹幹三丈高處留有寸深的新鮮爪痕，千年樟雲豹王巡行殘存的淡淡樟木威壓令林間鳥雀不敢啼鳴。")
    } else if zone_or_room.contains("清泉溪谷") {
        Some("【山海異兆】融雪清澈溪潭中銀光躍動，冰河孑遺七家灣櫻花鉤吻鮭逆流躍上石階，溪畔卵石留有食蟹獴覓食足跡。")
    } else if zone_or_room.contains("北坡雲杉林") {
        Some("【山海異兆】雲杉樹皮上留有高處抓痕與新鮮松脂，凍土雪泥上印著胸白台灣月熊王如巨盆般的深印掌痕。")
    } else if zone_or_room.contains("西麓林道") {
        Some("【山海異兆】晨霧瀰漫的闊葉林道間閃過一抹寶藍與朱紅光彩，西麓丹冠藍腹鷴與台灣藍鵲正掠過古道林隙。")
    } else if zone_or_room.contains("淺澤泥口") {
        Some("【山海異兆】蘆葦泥澤水面浮現一對寬闊如彎月的巨角，淺澤台灣水牛王靜浸泥水吐納，浮木上柴棺水龜安然曝背。")
    } else if zone_or_room.contains("白茫洲") {
        Some("【山海異兆】腐葉與白瘴迷霧間留有碗口粗的三角菱紋滑痕，排灣聖靈大武百步蛇王盤踞枯木，周遭蟲蛙盡數屏息。")
    } else if zone_or_room.contains("貼山深潭") {
        Some("【山海異兆】碧綠如墨的深潭中央漩渦暗湧，寒潭千年花斑鱸鰻潛伏潭底石洞，水際冷石下隱現阿里山山椒魚蹤跡。")
    } else if zone_or_room.contains("青籠關") {
        Some("【山海異兆】青籠關險峰高空迴盪著大冠鷲王清厲悠長的鳴聲，關道荊棘石縫間隱現錢紋石虎與剛鬃山豬的獸跡。")
    } else if zone_or_room.contains("西嶺礦谷") {
        Some("【山海異兆】赤黑鐵礦岩壁下散落著被利爪刨開的碎礦石，裂石鯪鯉穿山甲王與青脊長鬃山羊在險峻礦脈間穿梭。")
    } else if zone_or_room.contains("東海峭壁") {
        Some("【山海異兆】清水斷崖萬仞高空傳來撕裂長風的清厲長嘯，馭風赫氏熊鷹王展開遮雲巨翼，斷崖長鬃山羊踏立絕壁。")
    } else if zone_or_room.contains("雲腰雪林") {
        Some("【山海異兆】玉山圓柏與冷杉積雪間留有修長三趾爪印，霧林黑長尾帝雉拖著雪白橫紋長尾自雪霧中優雅掠過。")
    } else if zone_or_room.contains("白塔雪巔") {
        Some("【山海異兆】極寒冰峰石柱之上繚繞著純白清氣，雪巔霜冠大水鹿與極頂帝雉傲立雲海之巔，靈氣貫通天地。")
    } else {
        None
    }
}

/// 地理環境吐納感知之天地來源與環境意象對應（供【觀境吐納】）。
/// 高山雪峰、流雲長風對應【氣源】，深潭、水澤、礁岸對應【水源】，其餘山野丘壑亦有微氣共鳴。
pub fn zone_ambient_word_source(zone_or_room: &str) -> Option<(crate::combat::wordplate::WordSource, &'static str)> {
    use crate::combat::wordplate::WordSource;

    if zone_or_room.contains("白塔雪巔")
        || zone_or_room.contains("雲腰雪林")
        || zone_or_room.contains("青籠關")
        || zone_or_room.contains("東海峭壁")
        || zone_or_room.contains("白茫洲")
        || zone_or_room.contains("風舶角")
    {
        Some((WordSource::Qi, "雲海罡風浩蕩，萬仞天風透體而過，天地至純清氣奔湧入神識。"))
    } else if zone_or_room.contains("貼山深潭")
        || zone_or_room.contains("清泉溪谷")
        || zone_or_room.contains("淺澤泥口")
        || zone_or_room.contains("驚濤岩岸")
        || zone_or_room.contains("巨藻淺灣")
        || zone_or_room.contains("落日岬")
        || zone_or_room.contains("黑石灣")
        || zone_or_room.contains("水尾寮")
        || zone_or_room.contains("鹽田澳")
    {
        Some((WordSource::Water, "碧波水精流轉，寒潭暗湧與潮聲拍岸共鳴，潤澤水靈沁潤心脾。"))
    } else if zone_or_room.contains("西嶺礦谷")
        || zone_or_room.contains("赤石坡")
        || zone_or_room.contains("紅土崁")
    {
        Some((WordSource::Mineral, "重岩地脈盤踞，金石沉著剛毅之質理厚重深邃，山骨厚土鎮守識海。"))
    } else if zone_or_room.contains("翠冠巨木林")
        || zone_or_room.contains("古河巨木林")
        || zone_or_room.contains("北坡雲杉林")
        || zone_or_room.contains("西麓林道")
        || zone_or_room.contains("長風草甸")
        || zone_or_room.contains("霜林口")
        || zone_or_room.contains("半嶺")
    {
        Some((WordSource::Plant, "草木葳蕤繁茂，萬木長青生發之氣綿綿不絕，蒼生靈機滋養百骸。"))
    } else {
        Some((WordSource::Qi, "天地吐納有節，微風徐徐流過四肢百骸，隱隱感應到周天游離清氣。"))
    }
}


/// 台灣山海經特產使用效果：
/// 回傳 `(category_tag, vit_gain, qi_gain, consume_item, effect_narrative)`。
pub fn shanhai_item_use_effect(item_id_or_name: &str) -> Option<(&'static str, i32, i32, bool, &'static str)> {
    match item_id_or_name {
        // 護身珍品／靈佩（不消耗道具）
        "wt_cat_0214" | "霧林帝雉長尾翎" => Some((
            "佩之・山海靈佩",
            0,
            10,
            false,
            "你將霧林帝雉長尾翎別於衣襟，周身湧起一陣清靈晨霧，心神寧靜如水。",
        )),
        "wt_cat_0216" | "丹冠藍腹鷴彩羽" => Some((
            "佩之・山海靈佩",
            5,
            5,
            false,
            "你撫摸著丹冠藍腹鷴的寶藍彩羽，靈氣如朝霞瑞彩籠罩周身，祥瑞護持。",
        )),
        "wt_cat_0172" | "樟林雲豹軟毛皮" => Some((
            "佩之・山海靈佩",
            8,
            4,
            false,
            "你披上帶著古玉雲紋的雲豹軟毛皮，步履如風，身形輕靈如山林幽影。",
        )),
        "wt_cat_0171" | "山林石虎斑皮" => Some((
            "佩之・山海靈佩",
            6,
            3,
            false,
            "你將石虎斑皮繫於腰際，耳聰目明，黑夜中感知亦清晰入微。",
        )),
        "wt_cat_0188" | "鯪鯉穿山甲鱗片" => Some((
            "佩之・山海靈佩",
            10,
            0,
            false,
            "你手握金黑堅硬的鯪鯉麟片，筋骨經絡隱隱泛熱，肌理堅實如覆重甲。",
        )),
        "wt_cat_0176" | "泥甲巨鱷厚麟皮" => Some((
            "佩之・山海靈佩",
            12,
            0,
            false,
            "你撫過堅如頑石的巨鱷厚麟皮，如披重鎧，周身氣血穩固如山。",
        )),
        "wt_cat_0207" | "山豬王彎月獠牙" | "wt_cat_0211" | "海崖黑野豬牙" => Some((
            "佩之・山海靈佩",
            6,
            2,
            false,
            "你佩戴上如新月般剛硬的山豬尖牙，胸中湧起一股悍不畏死的山林勇氣。",
        )),
        "wt_cat_0183" | "白鼻心烏腳香皮" => Some((
            "佩之・山海靈佩",
            4,
            6,
            false,
            "你佩帶散發草果幽香的白鼻心皮，清香透鼻，令人百邪不侵、神清氣爽。",
        )),
        "wt_cat_0192" | "高山熊鷹翎羽" => Some((
            "佩之・山海靈佩",
            4,
            8,
            false,
            "你插上高山熊鷹的花翎羽毛，長風貫體，身輕體健仿若能乘風而起。",
        )),

        // 消耗性靈藥／珍餚（消耗道具）
        "wt_cat_0189" | "大武百步蛇乾蛻" => Some((
            "入藥・山海本草",
            10,
            15,
            true,
            "你將百步蛇乾蛻研粉調水服下，喉間微辛，一股清冽靈力順著經脈化開，深層瘴毒盡消。",
        )),
        "wt_cat_0205" | "胸白月熊厚熊掌" => Some((
            "食之・山海珍饈",
            25,
            10,
            true,
            "你細嚼慢嚥烹製的月熊厚掌，溫熱膏脂入口即化，全身暖流滾滾，四肢百骸力量沛然！",
        )),
        "wt_cat_0217" | "月熊凝白腹脂膏" => Some((
            "入藥・山海本草",
            20,
            8,
            true,
            "你將月熊凝白腹脂膏塗抹經絡或服食，肌膚溫潤生光，風寒暗傷頓時撫平。",
        )),
        "wt_cat_0269" | "深山牛樟芝" => Some((
            "入藥・山海本草",
            15,
            25,
            true,
            "你含化一片赤紅深山牛樟芝，苦盡甘來，精純元氣如瀑布般自丹田奔湧直上！",
        )),
        _ => None,
    }
}

/// 檢測玩家背包中是否佩戴山海護身靈佩珍品，提供戰鬥護持加成：
/// 回傳 `(talisman_name, vit_bonus, qi_bonus, bonus_narrative)`。
pub fn player_shanhai_talisman_bonus(inventory_json: &str) -> Option<(&'static str, i32, i32, &'static str)> {
    if inventory_json.contains("wt_cat_0176") {
        Some(("泥甲巨鱷厚麟皮", 12, 0, "泥甲護體，體魄固若金湯"))
    } else if inventory_json.contains("wt_cat_0188") {
        Some(("鯪鯉穿山甲鱗片", 10, 0, "烏金麟甲，氣血經絡通達無阻"))
    } else if inventory_json.contains("wt_cat_0172") {
        Some(("樟林雲豹軟毛皮", 8, 4, "古玉雲紋，身形敏捷化險為夷"))
    } else if inventory_json.contains("wt_cat_0171") {
        Some(("山林石虎斑皮", 6, 3, "石虎通靈，步法如電避實就虛"))
    } else if inventory_json.contains("wt_cat_0207") {
        Some(("山豬王彎月獠牙", 6, 2, "獠牙如月，雄渾剛毅撼動山川"))
    } else if inventory_json.contains("wt_cat_0211") {
        Some(("海崖黑野豬牙", 5, 2, "野豬尖牙，山林剛猛氣息加持"))
    } else if inventory_json.contains("wt_cat_0214") {
        Some(("霧林帝雉長尾翎", 0, 10, "帝雉白翎，清霧拂體元氣長存"))
    } else if inventory_json.contains("wt_cat_0216") {
        Some(("丹冠藍腹鷴彩羽", 5, 5, "藍鷴瑞羽，祥光普照體氣雙益"))
    } else if inventory_json.contains("wt_cat_0192") {
        Some(("高山熊鷹翎羽", 4, 8, "熊鷹花翎，神目如電長風護體"))
    } else {
        None
    }
}

/// 取得正方格附近格子的簡要視野（地圖渲染用）。
///
/// Tuple 欄位順序：`(x, y, kind, terrain, category, name, explored, walkable)`
/// - `kind`：Terrain enum 變體 snake_case（送前端切色/icon）
/// - `terrain`：中文標籤（向後相容）
/// - `category`：`"terrain"` / `"landmark"` / `"infra"`
#[allow(clippy::type_complexity)]
pub fn get_grid_cells_around(
    x: i32,
    y: i32,
    radius: i32,
) -> Vec<(i32, i32, String, String, String, String, bool, bool)> {
    let Some(grid) = crate::server::grid_manager::get_runtime_square_grid() else {
        return Vec::new();
    };
    let center = SquareCoord::new(x, y);
    let mut out = Vec::new();
    for dx in -radius..=radius {
        for dy in -radius..=radius {
            let c = SquareCoord::new(x + dx, y + dy);
            if center.chebyshev_distance(c) > radius {
                continue;
            }
            if let Some(cell) = grid.get(c) {
                out.push((
                    c.x,
                    c.y,
                    cell.terrain.kind_str().to_string(),
                    terrain_name_zh(cell.terrain).to_string(),
                    cell.terrain.category().to_string(),
                    cell.name.clone(),
                    cell.explored,
                    cell.terrain.walkable(),
                ));
            }
        }
    }
    out
}

/// 取得地形對應純符號（供前端或視圖查詢）
pub fn get_grid_cell_symbol(kind: &str) -> &'static str {
    crate::grid::Terrain::from_kind_str(kind)
        .map(|t| t.symbol())
        .unwrap_or("·")
}

#[must_use]
pub fn entity_kind_str(k: &EntityKind) -> &'static str {
    match k {
        EntityKind::Player => "player",
        EntityKind::Npc => "npc",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_ambient_npc_at_grid() {
        // 1. 文曜四城門外道路
        let west_gate = ensure_ambient_npc_at_grid(-18, 0, "西麓林道・道路");
        assert!(west_gate.is_some());
        let npc = west_gate.unwrap();
        assert_eq!(npc.kind, EntityKind::Npc);
        assert!(!npc.display_title.is_empty());
        assert!(!npc.current_activity.is_empty());
        assert!(npc.vit > 0);

        // 2. 獵場深處環境生物
        // 2. 獵場深處環境生物（預設顯形尋常物種／小怪，不直接刷出鎮場霸主）
        let hunting_ground = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野");
        assert!(hunting_ground.is_some());
        let creature = hunting_ground.unwrap();
        assert!(!creature.display_title.is_empty());
        assert_ne!(creature.display_title, "長風四目水鹿王");

        // 3. 尋跡顯形後鎮場霸主破霧現身
        mark_apex_beast_tracked_at(-180, -195);
        let tracked_apex = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野").unwrap();
        assert_eq!(tracked_apex.display_title, "長風四目水鹿王");

        // 4. 深水不應生成環境生物
        let deep_water = ensure_ambient_npc_at_grid(-390, 0, "北冕海・深水");
        assert!(deep_water.is_none());
    }

    #[test]
    fn test_hunting_zone_apex_omen() {
        let all_19_zones = [
            "長風草甸", "古河巨木林", "赤石坡", "落日岬", "驚濤岩岸",
            "巨藻淺灣", "黑石灣", "翠冠巨木林", "清泉溪谷", "北坡雲杉林",
            "西麓林道", "淺澤泥口", "白茫洲", "貼山深潭", "青籠關",
            "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔",
        ];
        for z in all_19_zones {
            let omen = hunting_zone_apex_omen(z);
            assert!(omen.is_some(), "Zone {z} must have an apex omen");
            assert!(omen.unwrap().starts_with("【山海異兆】"));
        }
        assert!(hunting_zone_apex_omen("文曜門前大街").is_none());
    }

    #[test]
    fn test_shanhai_item_use_effect() {
        // 護身珍品：不消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0214").unwrap();
        assert_eq!(cat, "佩之・山海靈佩");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);
        assert!(!consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("丹冠藍腹鷴彩羽").unwrap();
        assert!(!consume);

        // 消耗性靈藥／珍餚：消耗
        let (cat, vit, qi, consume, narr) = shanhai_item_use_effect("wt_cat_0205").unwrap();
        assert_eq!(cat, "食之・山海珍饈");
        assert_eq!(vit, 25);
        assert_eq!(qi, 10);
        assert!(consume);
        assert!(!narr.is_empty());

        let (_, _, _, consume, _) = shanhai_item_use_effect("大武百步蛇乾蛻").unwrap();
        assert!(consume);
        let (_, _, _, consume, _) = shanhai_item_use_effect("深山牛樟芝").unwrap();
        assert!(consume);

        assert!(shanhai_item_use_effect("wt_cat_unknown").is_none());
    }

    #[test]
    fn test_player_shanhai_talisman_bonus() {
        let inv_with_croc = r#"[{"item_id":"wt_cat_0176","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_croc);
        assert!(bonus.is_some());
        let (name, vit, qi, desc) = bonus.unwrap();
        assert_eq!(name, "泥甲巨鱷厚麟皮");
        assert_eq!(vit, 12);
        assert_eq!(qi, 0);
        assert!(!desc.is_empty());

        let inv_with_pheasant = r#"[{"item_id":"wt_cat_0214","quantity":1}]"#;
        let bonus = player_shanhai_talisman_bonus(inv_with_pheasant);
        assert!(bonus.is_some());
        let (name, vit, qi, _) = bonus.unwrap();
        assert_eq!(name, "霧林帝雉長尾翎");
        assert_eq!(vit, 0);
        assert_eq!(qi, 10);

        let inv_empty = r#"[]"#;
        assert!(player_shanhai_talisman_bonus(inv_empty).is_none());
    }

    #[test]
    fn test_zone_ambient_word_source() {
        use crate::combat::wordplate::WordSource;
        let qi_zone = zone_ambient_word_source("白塔雪巔").unwrap();
        assert_eq!(qi_zone.0, WordSource::Qi);
        let water_zone = zone_ambient_word_source("貼山深潭").unwrap();
        assert_eq!(water_zone.0, WordSource::Water);
        let plant_zone = zone_ambient_word_source("翠冠巨木林").unwrap();
        assert_eq!(plant_zone.0, WordSource::Plant);
        let min_zone = zone_ambient_word_source("西嶺礦谷").unwrap();
        assert_eq!(min_zone.0, WordSource::Mineral);
        let fallback_zone = zone_ambient_word_source("文曜街市").unwrap();
        assert_eq!(fallback_zone.0, WordSource::Qi);
    }
}

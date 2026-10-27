use crate::db::{self, object_has_socket};
use crate::entity::{Character, EntityKind};
use crate::grid::{Direction, SquareCoord, grid_room_id_from_coord, terrain_name_zh};
use crate::model::{Exit, Room, RoomObject};
use crate::store;

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
        if let Some(amb) = ensure_ambient_npc_at_grid(x, y, &room.name) {
            entities.push(amb);
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

    let seed = ((x as i64 * 374761393) ^ (y as i64 * 668265263)).unsigned_abs();
    let roll = seed % 6;

    let is_road = room_name.contains("道路")
        || room_name.contains("木橋")
        || terrain == crate::grid::Terrain::Road
        || terrain == crate::grid::Terrain::Bridge;

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
                0 => ("重角原牛王", "昂首聳立於草甸深處，巨大牛角直指蒼穹，威震四野"),
                1 => ("疾行草獸", "四蹄如飛，在齊膝芒草間迅疾掠過帶起陣陣草浪"),
                2 => ("平原梅花鹿", "優雅地在草甸花叢間駐足嚼草，耳翼隨風微動"),
                3 => ("鯪鯉穿山甲", "披著堅硬甲片伏於泥土坡旁，專注地扒掘蟻穴"),
                4 => ("草甸環頸雉", "披著絢麗五彩羽衣在芒草中昂首啼鳴，振翅破風"),
                _ => ("放牧老獵手", "蹲在土坡上查看草甸獸跡"),
            },
            "古河巨木林" => match roll {
                0 => ("大武百步蛇王", "盤踞於千年巨木盤根之上，信子微吐，蛇目寒芒懾人"),
                1 => ("樟林雲豹", "無聲無息潛行於古楠巨枝之間，暗影中雲紋隱現"),
                2 => ("四目水鹿", "生有雙重眶下靈眼，在巨木霧氣中警覺踏水而行"),
                3 => ("夜嘯大赤飛鼠", "展開寬闊皮翼自巨木樹冠滑翔而下，厲聲長嘯"),
                4 => ("巨木五色鳥", "棲息於高枝樹洞旁，發出宛若深山木魚般的清脆啼聲"),
                _ => ("巡林斥候", "倚著巨木樹幹辨認獸徑"),
            },
            "赤石坡" => match roll {
                0 => ("赤甲穿石巨蜥", "盤踞於高聳赤石巔峰，烈日下周身厚甲如火炭般灼紅"),
                1 => ("赤尾竹葉青", "倒懸於風化石縫的枯藤之上，鮮紅尾尖微顫待敵"),
                2 => ("長鬃岩山羊", "腳踏垂直陡峭的赤褐色岩壁，如履平地凌空躍動"),
                3 => ("向陽石龍子", "在滾燙赤石板上飛快竄行，金屬光澤的長尾甩動如鞭"),
                4 => ("赤崖紅隼", "收斂雙翼如箭矢般自赤石懸崖俯衝而下，撕裂長空"),
                _ => ("巡山採石人", "手持鐵鑿觀察崖壁礦脈"),
            },
            "落日岬" => match roll {
                0 => ("海岬豆腐鯨鯊", "浮游於落日殘照的岬角外海，龐大斑紋身軀引動洋流巨浪"),
                1 => ("礁岩裸胸錢鰻", "自浪蝕暗礁洞穴探出森冷尖牙巨口，凶光畢露"),
                2 => ("海崖黑山羊", "於狂暴海風撕扯的岬角懸崖邊緣穩健覓食耐鹽灌木"),
                3 => ("海岸八卦椰子蟹", "揮舞一對開石裂殼的巨螯，在防風林與礫石堆間爬動"),
                4 => ("迎風海角雕", "逆著浩瀚西風在海角萬仞高空盤旋，神目如電巡視汪洋"),
                _ => ("觀潮老漁民", "遠眺西海浪潮動向"),
            },
            "驚濤岩岸" => match roll {
                0 => ("黑水溝大王烏賊", "於滔天狂瀾中若隱若現，十條巨腕翻攪暗流，凶威赫赫"),
                1 => ("載嶼百年巨鼇", "身如浮島般橫臥於驚濤駭浪間，背甲生滿藤壺海藻"),
                2 => ("裂浪海狗", "順著拍岸碎浪滑入急流，身形靈動破浪嬉戲"),
                3 => ("潮間青岩巨蟹", "披著青黑重甲駐守於潮池裂罅，雙螯張合鏗鏘作響"),
                4 => ("赤足巨嘴海鷗", "立於浪花飛濺的突岩尖端，引吭高啼穿透怒濤轟鳴"),
                _ => ("海防望海卒", "立於高岩上注視海平線"),
            },
            "巨藻淺灣" => match roll {
                0 => ("深海皇帶龍魚", "銀白長軀泛著磷火幽光，游弋於巨藻林外緣如龍巡海"),
                1 => ("黑潮鬼頭刀", "金綠交織的背鰭劃破水面，如離弦利箭在藻浪間追逐飛魚"),
                2 => ("淺灣灰海獺", "胸口抱著青石在浮動巨藻叢中翻身仰游，悠閒自得"),
                3 => ("棘皮海膽獸", "周身密布淬毒紫黑長刺，緩慢在海底砂岩上挪移"),
                4 => ("白鷺候鳥", "單足凝立於淺灣清流白浪之中，凝視游魚伺機啄擊"),
                _ => ("收藻採集工", "手持木叉整理漂岸海藻"),
            },
            "黑石灣" => match roll {
                0 => ("噴玉海翁巨鯨", "於玄武岩外海噴出沖天白玉水柱，巨尾拍擊震動整座海灣"),
                1 => ("黑背玄武岩龜", "伏在黝黑巨石上吐納海霧，背甲深沉如萬年古墨"),
                2 => ("近海虎斑花枝", "在黑石珊瑚縫間變換斑斕偽裝體色，靈活噴射水流"),
                3 => ("浪蝕九孔螺獸", "如鐵盔般緊緊吸附於黑石浪痕之上，甲殼層疊生輝"),
                4 => ("潮痕黑冠鳥", "在浪退後的黑石隙間啄食小螺，頭頂墨黑長翎隨風飄揚"),
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
                0 => ("銀鱗護潭鯇王", "身長丈餘，周身片片銀鱗如雪，守護著深溪清潭靈眼"),
                1 => ("山溪大蟾蜍", "蹲坐於濕潤的長滿青苔的巨石上，鼓動下頜發出如擂鼓沉鳴"),
                2 => ("山羌", "自溪邊蕨草叢探出小巧頭顱，發出宛若幼犬般的急促叫聲"),
                3 => ("清溪青毛蟹", "螯足生滿濃密青黑絨毛，在清澈見底的卵石溪流中側行"),
                4 => ("溪澗白條魚鷹", "停在水旁枯石上緊盯水面波紋，眼神銳利如刀"),
                _ => ("汲水挑夫", "在泉眼旁注滿水囊整裝待發"),
            },
            "北坡雲杉林" => match roll {
                0 => ("胸白月熊王", "人立而起發出驚天動地咆哮，胸口一抹新月純白如雪"),
                1 => ("山林豺犬", "成群結隊在雲杉陰影下潛行嗅聞，露出森白利齒"),
                2 => ("剛鬃黑山豬", "粗壯如石的獠牙向上挑起，前蹄刨掘著凍土堅根"),
                3 => ("山林獼猴", "掛在雲杉垂枝上抓耳撓腮，朝樹下警覺張望"),
                4 => ("高山熊鷹", "展開寬闊黑褐花翎，傲視千山盤旋於雲杉密林之上"),
                _ => ("伐木巡山客", "踏著積雪查看雲杉材木"),
            },
            "西麓林道" => match roll {
                0 => ("林道剛鬃豬王", "體型如小丘般橫臥古道旁，背脊針鬃如鐵針倒豎"),
                1 => ("龜殼花毒蝮", "盤在腐葉落枝堆中，烙鐵頭微昂，蛇信吞吐帶出腥風"),
                2 => ("山林野獾", "在坡道土洞旁警惕地抽動短吻，爪尖銳利善於掘石"),
                3 => ("食蛇箱龜", "背甲金黃隆起如盾，從容不迫地在碎石落葉間挪行"),
                4 => ("山林竹雞", "在林道兩側密竹叢間結伴啄食草籽，拍翅低飛"),
                _ => ("護林守衛", "按著腰間佩刀沿林道巡邏"),
            },
            "淺澤泥口" => match roll {
                0 => ("泥甲巨鱷王", "半截如巨木般的泥甲浮出水面，冰冷金黃豎瞳冷酷注視四方"),
                1 => ("過山烏風蛇", "如黑練破水般疾馳過泥沼表面，身形迅捷無匹"),
                2 => ("沼澤水獺", "咬著半截泥鰍在泥渚間翻滾滑行，皮毛光滑油亮"),
                3 => ("紅螯泥沼蟹", "高舉一隻血紅巨螯在泥孔外威嚇對手，吐著水泡"),
                4 => ("澤地長腿泥鷺", "在泥沼淺汊間緩步涉水搜尋，黃金長喙閃爍冷芒"),
                _ => ("涉澤行腳客", "手拄長竹竿探路避開暗沼"),
            },
            "白茫洲" => match roll {
                0 => ("雙翼沼龍獸", "自漫天迷霧深處振翅而起，龍首蛇尾，攪動瀰漫白瘴"),
                1 => ("迷霧白鼻狐", "穿梭於茫茫水霧與菖蒲叢間，身形飄忽如白色幽靈"),
                2 => ("白茫角鹿", "通體雪白生有如珊瑚般的玉角，踏過泥水無痕"),
                3 => ("金線毒樹蛙", "背脊閃爍刺目金黃警戒線，靜靜蹲在濕潤水苔葉片上"),
                4 => ("沼洲夜鷺", "縮頸佇立於迷霧淺灘枯木上，赤紅眼睛穿透重霧"),
                _ => ("採苔老採集人", "手持防瘴草團辨識霧中去向"),
            },
            "貼山深潭" => match roll {
                0 => ("深潭紅頭巨蟒", "巨軀深藏於深碧潭底，火紅巨吻露出水面吐納水精"),
                1 => ("深潭玄水巨龜", "背甲宛若千年沉鐵巨石，浮在深碧水面劃開層層深渦"),
                2 => ("高山苦花魚王", "銀光閃爍躍出寒潭，腹部金線在陽光下熠熠生輝"),
                3 => ("寒潭大鯢", "伏於水底深洞石縫，四足生爪，叫聲宛若嬰啼清幽"),
                4 => ("潭畔黑頭鷗", "俯掠水面激起陣陣白花，利爪勾起水面浮萍"),
                _ => ("潭邊垂釣隱士", "握竿靜坐潭石觀照水深"),
            },
            "青籠關" => match roll {
                0 => ("斷崖鐵角山羊王", "傲立於千仞絕壁險石之巔，雙角黑沉如玄鐵，俯視萬丈深淵"),
                1 => ("峭壁百步巨蝮", "盤繞在關隘石縫暗處，三角巨首紋絲不動等候獵物"),
                2 => ("關隘長鬃山羊", "敏捷踏過險峻關石窄徑，灰色長鬃在山風中飛舞"),
                3 => ("岩縫刺鼠", "於石壁裂隙中探頭探腦，渾身短刺倒豎警惕萬分"),
                4 => ("石隙雪鴞", "隱於關城高岩陰影中，金黃圓目審視關道往來生靈"),
                _ => ("戍關巡哨隊員", "按矛立於要衝處檢視往來行跡"),
            },
            "西嶺礦谷" => match roll {
                0 => ("翻身赤甲地牛", "伏於赤黑煤層地脈深處，每次翻身皆引動地動山搖礦石崩落"),
                1 => ("裂石穿山甲王", "全身鱗甲堅硬如鍛鐵，利爪一揮便碎裂堅硬礦岩"),
                2 => ("礦谷黑野豬", "長吻帶泥在煤石堆旁狂暴衝撞，渾身沾滿硬結黑泥"),
                3 => ("岩穴黑甲蜥", "在赤黑煤石間爬動，尾部粗壯如鐵杵"),
                4 => ("礦谷灰燕", "穿梭於礦坑裂縫與採礦木架之間，唧唧低語"),
                _ => ("馱煤採礦工", "整理背架上的粗礦石準備啟程"),
            },
            "東海峭壁" => match roll {
                0 => ("馭風赫氏熊鷹王", "凌空盤旋俯瞰東海萬頃波浪，雙翼遮雲蔽日破風呼嘯"),
                1 => ("攀崖長鬃羚", "腳踏險絕峭壁上的寸許石階，從容縱躍如飛"),
                2 => ("海崖巡狩狼犬", "沿著懸崖邊緣迎著海風警覺嗅探，毛髮飛揚"),
                3 => ("峭壁飛鼠", "自海風呼嘯的懸崖樹梢縱身一躍，乘著上升氣流滑向對岸"),
                4 => ("岩隙海燕", "掠過潮濕峭壁疾飛入巢，雙翼如剪破浪穿風"),
                _ => ("攀崖採燕客", "身繫麻繩在崖壁凹處調勻呼吸"),
            },
            "雲腰雪林" => match roll {
                0 => ("逐日七叉白鹿", "角如玉樹分生七叉，周身散發柔和白芒，踏雪留香不留痕"),
                1 => ("雪線胸白月熊", "頂著嚴寒風雪在雪林中巡狩，掌力能折斷百年冰樹"),
                2 => ("踏雪四目水鹿", "在寒霧積雪中安詳漫步，四目顧盼清明出塵"),
                3 => ("高山野麝", "穿梭於覆雪冷杉樹幹間，香腺散發濃烈清冷幽香"),
                4 => ("霧林黑長尾帝雉", "披著深藍曜黑羽衣拖著雪白條紋長尾，漫步於玉山箭竹積雪中"),
                _ => ("高山踏雪獵戶", "身裹獸皮斗篷檢視捕獸夾"),
            },
            "白塔雪巔" => match roll {
                0 => ("神山通靈白雕", "傲然立於冰封絕頂石柱之巔，目通天地，羽翼泛著神光"),
                1 => ("冰崖雪豹", "毛色純白雜墨斑，潛伏在萬年冰棱陰影中凝神伺機"),
                2 => ("雪巔白山羊", "蹄踏玄冰雪脊，在萬丈虛空之上從容齧食冰苔"),
                3 => ("極頂雪絨兔", "通體如一團無瑕雪球，在極寒石隙中靈動跳躍"),
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

/// 19 大獵場首領異獸威壓與殘跡異兆（若該地帶或場景對應既定獵場）。
pub fn hunting_zone_apex_omen(zone_or_room: &str) -> Option<&'static str> {
    if zone_or_room.contains("長風草甸") {
        Some("【山海異兆】草浪間隱現數丈寬的深深踏痕與被巨力折斷的芒草叢，空氣中流動著重角原牛王奔踏而過遺留的滾燙泥土氣息。")
    } else if zone_or_room.contains("古河巨木林") {
        Some("【山海異兆】腐葉間留有碗口粗的三角麟紋滑痕，空氣中瀰漫著大武百步蛇王特有的奇異腥甜草香，周遭蟲鳴盡數屏息。")
    } else if zone_or_room.contains("赤石坡") {
        Some("【山海異兆】赤紅岩壁上布滿深深劃痕與如焦炭般的灼燒爪印，赤甲穿石巨蜥留下的灼熱餘溫甚至令石縫草根泛焦。")
    } else if zone_or_room.contains("落日岬") {
        Some("【山海異兆】外海波濤下時而浮現龐大無朋的深沉暗影，海岬豆腐鯨鯊巡游激起的一圈圈巨浪拍打著岬角崖岸。")
    } else if zone_or_room.contains("驚濤岩岸") {
        Some("【山海異兆】怒濤洶湧的黑水漩渦中漂浮著斷裂的粗碩墨黑觸鬚，黑水溝大王烏賊吐出的濃黑墨汁將潮水染成幽暗玄色。")
    } else if zone_or_room.contains("巨藻淺灣") {
        Some("【山海異兆】深灣巨藻林深處閃爍著如龍巡海般的幽微磷光，深海皇帶龍魚那長達數丈的銀白身軀在暗湧中遊弋留痕。")
    } else if zone_or_room.contains("黑石灣") {
        Some("【山海異兆】玄武岩石隙間殘留著如雪般晶瑩的海玉白霧，海面上隱隱傳來噴玉海翁巨鯨噴氣引動的如雷震顫。")
    } else if zone_or_room.contains("翠冠巨木林") {
        Some("【山海異兆】老樟樹幹三丈高處留有寸深的新鮮爪痕，千年樟雲豹王巡行殘存的淡淡樟木威壓令林間鳥雀不敢啼鳴。")
    } else if zone_or_room.contains("清泉溪谷") {
        Some("【山海異兆】寒潭水底銀光閃動，潭眼漩渦深處隱現一片片如冰晶雪片般的巨大魚鱗，銀鱗護潭鯇王正靜潛潭底。")
    } else if zone_or_room.contains("北坡雲杉林") {
        Some("【山海異兆】雲杉樹皮上塗抹著剛硬黑鬃與新鮮松脂，凍土雪泥上留有胸白月熊王如巨盆般的深印掌痕。")
    } else if zone_or_room.contains("西麓林道") {
        Some("【山海異兆】林道側坡土丘被粗暴拱開一道深溝，林道剛鬃豬王那如剛針般的堅硬針鬃零星散落於灌木刺叢上。")
    } else if zone_or_room.contains("淺澤泥口") {
        Some("【山海異兆】腥臭泥沼水面上漂浮著一片片如鐵甲般堅硬的厚鱗，泥甲巨鱷王靜伏暗處時呼出的濁氣在水面泛起連串氣泡。")
    } else if zone_or_room.contains("白茫洲") {
        Some("【山海異兆】瀰漫漫天的白瘴迷霧中傳來低沉如雷鳴的振翅聲，雙翼沼龍獸掠過水面激起的狂風令整片菖蒲伏倒。")
    } else if zone_or_room.contains("貼山深潭") {
        Some("【山海異兆】碧綠如墨的深潭中央漩渦滾湧，深潭紅頭巨蟒火紅如炬的巨吻探出水面吐納水精，水氣泛起層層腥霧。")
    } else if zone_or_room.contains("青籠關") {
        Some("【山海異兆】萬仞絕壁尖端隱現一道如玄鐵長矛般的巍峨羊角剪影，斷崖鐵角山羊王俯瞰關道，飛石隨其蹄步滾落深谷。")
    } else if zone_or_room.contains("西嶺礦谷") {
        Some("【山海異兆】赤黑岩壁下散落著被巨力掀翻的碎裂煤石，地底深處隱約傳來翻身赤甲地牛沉悶如雷的吐息震動。")
    } else if zone_or_room.contains("東海峭壁") {
        Some("【山海異兆】絕壁萬仞高空傳來撕裂長風的清厲長嘯，馭風赫氏熊鷹王展開遮雲巨翼，巨影掠過千頃駭浪。")
    } else if zone_or_room.contains("雲腰雪林") {
        Some("【山海異兆】萬年積雪上殘留著點點瑩潤如珠的白芒清香，逐日七叉白鹿踏雪而過，林間冷杉枝頭落雪紛紛。")
    } else if zone_or_room.contains("白塔雪巔") {
        Some("【山海異兆】極寒冰峰石柱之上繚繞著如神光般的純白靈氣，神山通靈白雕傲然展翼，雙目如電穿透萬里雲海。")
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
        let hunting_ground = ensure_ambient_npc_at_grid(-180, -195, "長風草甸・向陽原野");
        assert!(hunting_ground.is_some());
        let creature = hunting_ground.unwrap();
        assert!(!creature.display_title.is_empty());

        // 3. 深水不應生成環境生物
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

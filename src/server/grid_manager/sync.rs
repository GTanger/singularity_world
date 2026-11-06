
/// 於白晝時段（6..=20）調度西門遠征與商隊人物（岑拓、溫晴、馬行頭）前往城外西麓林道
pub fn dispatch_west_gate_expedition_npcs(store: &mut crate::store::Store) {
    let now_ts = crate::game::now_unix();
    let dispatches = [
        (
            "岑拓",
            "caravan_route",
            (-19, 0),
            (-22, 0),
            "西麓林道",
            "押著滿載鐵器與茶磚的雙輪貨車行進在西麓林道上，隨時留意路旁獸蹤",
            "西麓林道今日風乾路實，趁天色尚早把這批貨趕送出去，回程還能收些山貨",
            "stranded_cargo",
            "押送鐵器與茶磚商隊車巡視西麓林道",
        ),
        (
            "溫晴",
            "wilderness_expedition",
            (-20, 1),
            (-18, 0),
            "西麓林道",
            "背著竹編藥簍在西麓林道旁辨識新冒芽的光葉藥草與山茶籽",
            "百草堂裡的退熱草藥快見底了，趁今日無雨親自出西門採些鮮葉回去炮製",
            "restock_goods",
            "採集光葉藥草與山茶籽以備炮製",
        ),
        (
            "馬行頭",
            "caravan_route",
            (-18, 0),
            (-21, 0),
            "西麓林道",
            "在西城門外的西麓林道起點檢查駝獸蹄鐵與車轍路況",
            "出西門這段林道車轍頗深，得叮囑車夫們過坡時收緊韁繩",
            "repair_tool",
            "檢查西麓林道起點路況與駝獸蹄鐵車軛",
        ),
    ];

    for (id, goal, (sx, sy), (tx, ty), target_name, activity, thought, ob_kind, ob_desc) in dispatches {
        if get_resident_goal(id).is_some() {
            continue;
        }
        set_resident_goal(id, goal);
        RESIDENT_IDLE_TICKS.write().insert(id.to_string(), 0);
        RESIDENT_MOVE_TARGETS.write().insert(
            id.to_string(),
            (tx, ty, target_name.to_string(), activity.to_string()),
        );
        crate::store::open_active_npc_obligation(id, ob_kind, ob_desc);

        if let Some(mut existing) = store.get_entity(id) {
            existing.grid_x = Some(sx);
            existing.grid_y = Some(sy);
            existing.target_x = Some(tx);
            existing.target_y = Some(ty);
            existing.current_activity = activity.to_string();
            crate::db::add_thought_to_entity_with_delta(
                &mut existing,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                thought.to_string(),
            );
            crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);
            let ent_sync = existing.clone();
            let _ = store.upsert_entity_full(existing);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(id, &format!("grid:{sx}:{sy}"));
        } else {
            let display_char = id.chars().next().map(|c| c.to_string()).unwrap_or_else(|| "人".to_string());
            let mut entity = crate::store::Entity {
                id: id.to_string(),
                kind: "npc".to_string(),
                display_char,
                x: 0,
                y: 0,
                move_state: "idle".to_string(),
                target_x: Some(tx),
                target_y: Some(ty),
                walk_or_run: String::new(),
                move_started_at: None,
                vit: 100,
                qi: 100,
                dex: 50,
                magnesium: 500,
                last_observed_at: None,
                created_at: now_ts,
                gender: if id == "溫晴" { "F".to_string() } else { "M".to_string() },
                soul_seed: None,
                display_title: format!("|{}", id),
                activated_nodes: r#"["N000"]"#.to_string(),
                equipment_slots: "{}".to_string(),
                inventory: "[]".to_string(),
                disposition: 0,
                current_activity: activity.to_string(),
                place_id: None,
                grid_x: Some(sx),
                grid_y: Some(sy),
                thoughts: vec![],
            };
            crate::db::add_thought_to_entity_with_delta(
                &mut entity,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                thought.to_string(),
            );
            crate::db::clean_and_recalc_entity_disposition(&mut entity, now_ts);
            let ent_sync = entity.clone();
            let _ = store.upsert_entity_full(entity);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(id, &format!("grid:{sx}:{sy}"));
        }
    }
}

/// 依遊戲時刻將白塔半島 59 位排程住民同步至正方格大地圖對應建築，更新其作息目標。
static HOUSEHOLD_ENTITIES_SEEDED_ONCE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
static LAST_SYNCED_RESIDENT_HOUR: std::sync::atomic::AtomicI32 =
    std::sync::atomic::AtomicI32::new(-1);

pub fn sync_white_tower_scheduled_residents(game_hour: i32) {
    if LAST_SYNCED_RESIDENT_HOUR.swap(game_hour, std::sync::atomic::Ordering::SeqCst) == game_hour
        && HOUSEHOLD_ENTITIES_SEEDED_ONCE.load(std::sync::atomic::Ordering::SeqCst)
    {
        return;
    }
    let grid = match get_runtime_square_grid() {
        Some(g) => g,
        None => return,
    };

    crate::server::handler::inventory::ensure_flagship_specimen_qin_shuisheng_wordplate_initialized();
    let Some(store_arc) = crate::store::get_store() else {
        return;
    };
    let mut store = store_arc.write();
    crate::npc::causal_core::normalize_and_seed_circular_economy_once(&mut store);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // 日間時段（6..=20）若有西麓商隊與採藥人物尚未出發，即時調度派駐至西城門外西麓林道
    if (6..=20).contains(&game_hour) {
        dispatch_west_gate_expedition_npcs(&mut store);
    }

    for def in iter_resident_schedules() {
        if (def.id.starts_with("wt_vendor_") || def.id == "wt_npc_ma_buxue")
            && (game_hour == 5 || game_hour == 15 || game_hour == 23)
        {
            crate::npc::causal_core::enqueue_arrival_effect(
                crate::npc::causal_core::ArrivalEffect::RestockShopItems {
                    seller_id: def.id.to_string(),
                },
            );
        }
        let (bldg_name, mut activity) = (def.resolve)(game_hour);
        if !def.id.starts_with("wt_vendor_") && (game_hour >= 22 || game_hour < 5) {
            let is_night_watch = def.id == "雷千鈞"
                || def.display_title.contains("巡防")
                || def.display_title.contains("哨長")
                || def.display_title.contains("守塔")
                || def.display_title.contains("值守");
            if !is_night_watch {
                if !activity.contains("歇")
                    && !activity.contains("睡")
                    && !activity.contains("寢")
                    && !activity.contains("宿")
                {
                    activity = "正在內室榻上熄燈安歇";
                }
                clear_resident_goal(def.id);
            }
        }
        let cell = resolve_settlement_room_coord(&grid, bldg_name);
        let Some(cell) = cell else {
            tracing::warn!(
                "sync_white_tower_scheduled_residents: 找不到建築「{bldg_name}」，跳過人物「{}」之更新",
                def.id
            );
            continue;
        };
        let (x, y) = (cell.coord.x, cell.coord.y);

        if store.get_entity(def.id).is_some() {
            let (tx, ty, activity_owned) = crate::pg::living_world::step_resident_human_life_loop(
                &mut store,
                def.id,
                x,
                y,
                bldg_name,
                activity,
                game_hour as i32,
                crate::game::now_unix(),
            );
            let activity = activity_owned.as_str();
            let mut existing = store.get_entity(def.id).unwrap();
            existing.kind = "npc".to_string();
            // 保留既有體力/精力/敏捷與鎂幣數值，避免整點被清空或強制覆蓋
            if existing.vit <= 0 {
                existing.vit = 100;
            }
            if existing.qi <= 0 {
                existing.qi = 100;
            }
            if existing.dex <= 0 {
                existing.dex = 50;
            }
            if existing.magnesium < 0 {
                existing.magnesium = 0;
            }
            existing.place_id = None;
            existing.display_title = def.display_title.to_string();

            // 白晝與傍晚時段（6..=21）保護在地自主目標（荒野遠征、商隊行進、因果義務差事等），不被整點時刻重設打斷
            let is_autonomous_active = (6..=21).contains(&game_hour)
                && matches!(
                    existing.goal_kind().as_deref(),
                    Some("wilderness_expedition")
                        | Some("caravan_route")
                        | Some("obligation_errand")
                        | Some("autonomous_relief")
                        | Some("autonomous_livelihood")
                );
            if is_autonomous_active {
                let now_ts = crate::game::now_unix();
                crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);
                let ent_sync = existing.clone();
                let _ = store.upsert_entity_full(existing);
                crate::pg::entity::upsert_async(ent_sync);
                continue;
            }

            let is_on_duty_vendor = def.id.starts_with("wt_vendor_")
                && (bldg_name == "三水口南街" || bldg_name == "三水口北街");
            let cur_coord = (existing.grid_x, existing.grid_y);
            if let (Some(cur_x), Some(cur_y)) = cur_coord {
                if (cur_x, cur_y) == (tx, ty) || is_on_duty_vendor {
                    existing.grid_x = Some(tx);
                    existing.grid_y = Some(ty);
                    existing.current_activity = activity.to_string();
                    RESIDENT_MOVE_TARGETS.write().remove(def.id);
                    let _ = store.move_entity(def.id, &format!("grid:{tx}:{ty}"));
                } else {
                    // 不論日間夜間皆不瞬移，寫入執行期目標表由 advance_scheduled_residents_one_step 逐格走！
                    let dest_bldg_name = grid
                        .get(crate::grid::SquareCoord::new(tx, ty))
                        .map(|c| c.name.as_str())
                        .unwrap_or(bldg_name);
                    RESIDENT_MOVE_TARGETS.write().insert(
                        def.id.to_string(),
                        (tx, ty, dest_bldg_name.to_string(), activity.to_string()),
                    );
                }
            } else {
                existing.grid_x = Some(tx);
                existing.grid_y = Some(ty);
                existing.current_activity = activity.to_string();
                let _ = store.move_entity(def.id, &format!("grid:{tx}:{ty}"));
            }

            // Prune stale broken/daily thoughts and provide fresh character-specific thought
            let now_ts = crate::game::now_unix();
            existing.thoughts.retain(|t| {
                let f = t.flavor.as_deref().unwrap_or("");
                !f.contains("鎂袋空了")
                    && !f.contains("今日花用又見底")
                    && !f.contains("記掛著「內室榻上熄燈安歇」")
                    && !f.contains("記掛著『診所抓藥』")
                    && !f.contains("記掛著『手頭差事』")
            });
            if existing.thoughts.is_empty() {
                let init_flavor = initial_thought_for_npc(def.id, &def.display_title);
                crate::db::add_thought_to_entity_with_delta(
                    &mut existing,
                    crate::db::ThoughtSource::WorkDone,
                    8,
                    now_ts,
                    init_flavor,
                );
            }
            crate::db::clean_and_recalc_entity_disposition(&mut existing, now_ts);

            let ent_sync = existing.clone();
            let _ = store.upsert_entity_full(existing);
            crate::pg::entity::upsert_async(ent_sync);
        } else {
            let display_char = def
                .display_title
                .rsplit('|')
                .next()
                .unwrap_or(def.id)
                .chars()
                .next()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "人".to_string());
            let init_flavor = initial_thought_for_npc(def.id, def.display_title);
            let mut entity = crate::store::Entity {
                id: def.id.to_string(),
                kind: "npc".to_string(),
                display_char,
                x: 0,
                y: 0,
                move_state: "idle".to_string(),
                target_x: None,
                target_y: None,
                walk_or_run: String::new(),
                move_started_at: None,
                vit: 100,
                qi: 100,
                dex: 50,
                magnesium: 500,
                last_observed_at: None,
                created_at: now,
                gender: def.gender.to_string(),
                soul_seed: None,
                display_title: def.display_title.to_string(),
                activated_nodes: r#"["N000"]"#.to_string(),
                equipment_slots: "{}".to_string(),
                inventory: "[]".to_string(),
                disposition: 0,
                current_activity: activity.to_string(),
                place_id: None,
                grid_x: Some(x),
                grid_y: Some(y),
                thoughts: vec![],
            };
            let now_ts = crate::game::now_unix();
            crate::db::add_thought_to_entity_with_delta(
                &mut entity,
                crate::db::ThoughtSource::WorkDone,
                8,
                now_ts,
                init_flavor,
            );
            crate::db::clean_and_recalc_entity_disposition(&mut entity, now_ts);
            let ent_sync = entity.clone();
            let _ = store.upsert_entity_full(entity);
            crate::pg::entity::upsert_async(ent_sync);
            let _ = store.move_entity(def.id, &format!("grid:{x}:{y}"));
        }
    }
    if !HOUSEHOLD_ENTITIES_SEEDED_ONCE.swap(true, std::sync::atomic::Ordering::SeqCst) {
        store.ensure_all_household_entities_seeded();
    }
}

fn find_settlement_secondary_building<'a>(
    grid: &'a crate::grid::SquareGrid,
    (cx, cy): (i32, i32),
    query: &str,
) -> Option<&'a crate::grid::GridCell> {
    let cur_cell = grid.get(crate::grid::SquareCoord::new(cx, cy));
    let cur_zone = cur_cell.map(|c| c.zone.as_str()).unwrap_or("");
    let cur_prefix = cur_cell
        .and_then(|c| c.name.split('・').next())
        .unwrap_or("");

    let is_candidate = |c: &&crate::grid::GridCell| {
        (c.coord.x != cx || c.coord.y != cy)
            && !matches!(
                c.terrain,
                crate::grid::Terrain::Road
                    | crate::grid::Terrain::Bridge
                    | crate::grid::Terrain::Wall
            )
            && c.name.contains(query)
            && (
                (!cur_zone.is_empty() && c.zone == cur_zone)
                    || (!cur_prefix.is_empty() && c.name.starts_with(cur_prefix))
                    || (c.coord.x - cx).abs() + (c.coord.y - cy).abs() <= 20
            )
    };

    grid.cells().filter(is_candidate).min_by_key(|c| {
        (c.coord.x - cx).abs() + (c.coord.y - cy).abs()
    })
}


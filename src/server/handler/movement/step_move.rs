static EXPLORE_COOLDOWN: LazyLock<RwLock<HashMap<(String, i32, i32), i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 玩家足跡與目及座標記錄：player_id -> HashSet<(x, y)>
static PLAYER_WALKED_COORDS: LazyLock<RwLock<HashMap<String, HashSet<(i32, i32)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 記錄玩家上一次發送步入敘事的座標：player_id -> (x, y)
static PLAYER_LAST_ANNOUNCED_COORD: LazyLock<RwLock<HashMap<String, (i32, i32)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
/// 到場 NPC 叫賣／交談反應冷卻：(player_id, room_id) -> 過期 unix timestamp
static PRESENCE_REFLEX_COOLDOWN: LazyLock<RwLock<HashMap<(String, String), i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 玩家與特定 NPC 跨領域漣漪與因果反射冷卻：(player_id, npc_id) -> (過期 unix timestamp, 觸發因由鍵值)
static PAIRWISE_REFLEX_COOLDOWN: LazyLock<RwLock<HashMap<(String, String), (i64, String)>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));


/// 整數轉繁體中文字串（1..=9999）
fn to_chinese_numeral(n: i32) -> String {
    if n <= 0 {
        return "零".to_string();
    }
    let digits = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    if n < 10 {
        return digits[n as usize].to_string();
    }
    if n < 20 {
        return if n == 10 {
            "十".to_string()
        } else {
            format!("十{}", digits[(n % 10) as usize])
        };
    }
    if n < 100 {
        let ten = n / 10;
        let rem = n % 10;
        return if rem == 0 {
            format!("{}十", digits[ten as usize])
        } else {
            format!("{}十{}", digits[ten as usize], digits[rem as usize])
        };
    }
    if n < 1000 {
        let hundred = n / 100;
        let rem = n % 100;
        if rem == 0 {
            return format!("{}百", digits[hundred as usize]);
        } else if rem < 10 {
            return format!("{}百零{}", digits[hundred as usize], digits[rem as usize]);
        } else if rem < 20 {
            let unit = rem % 10;
            return if unit == 0 {
                format!("{}百一十", digits[hundred as usize])
            } else {
                format!("{}百一十{}", digits[hundred as usize], digits[unit as usize])
            };
        } else {
            let ten = rem / 10;
            let unit = rem % 10;
            return if unit == 0 {
                format!("{}百{}十", digits[hundred as usize], digits[ten as usize])
            } else {
                format!(
                    "{}百{}十{}",
                    digits[hundred as usize], digits[ten as usize], digits[unit as usize]
                )
            };
        }
    }
    let thousand = n / 1000;
    let rem1000 = n % 1000;
    if rem1000 == 0 {
        return format!("{}千", digits[thousand as usize]);
    }
    format!("{}千{}", digits[thousand as usize], to_chinese_numeral(rem1000))
}

/// 依據 game_days_since_epoch 計算開譜曆日期字串（春融、夏漲、秋狩、冬凝四季）
fn format_season_date_label(game_days_since_epoch: i32) -> String {
    let (year, season_name, day_in_season) = crate::game::kaipu_calendar_parts(game_days_since_epoch);
    let year_str = if year == 1 {
        "元".to_string()
    } else {
        to_chinese_numeral(year)
    };
    format!(
        "開譜曆 {}年・{}・{}日",
        year_str,
        season_name,
        to_chinese_numeral(day_in_season)
    )
}

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
            conn.send_error("尚未置身於城鎮地圖中");
            return;
        }
    };

    let gh = current_game_hour(&conn.cfg);
    let tp = TimePeriod::from_hour(gh);

    // 取當前格（需要 terrain + freshness）
    let cell_info = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(SquareCoord::new(gx, gy)).cloned());

    let (terrain_str, freshness) = cell_info
        .as_ref()
        .map(|c| (c.terrain.kind_str(), c.freshness))
        .unwrap_or(("unknown", Freshness::Fresh));

    // ── 探索冷卻計時：以 (player_id, gx, gy) 判定 ───────────────────
    let now = crate::game::now_unix();
    {
        let cds = EXPLORE_COOLDOWN.read();
        if let Some(&expire) = cds.get(&(player_id.clone(), gx, gy)) {
            if now < expire {
                let explore_msg = ExploreLogMsg {
                    msg_type: "explore_result".into(),
                    outcome: "cooldown".into(),
                    log: "這片地界已仔細翻找過，草木未復，暫時沒有新的動靜。".into(),
                    loot: vec![],
                    stat_delta: StatDelta::default(),
                    time_period_after: tp.label().into(),
                    freshness_after: freshness.as_str().into(),
                    warning_prose: String::new(),
                };
                conn.send_json(&explore_msg);
                send_grid_view(conn, &player_id, gx, gy, gh);
                return;
            }
        }
    }
    EXPLORE_COOLDOWN
        .write()
        .insert((player_id.clone(), gx, gy), now + 10);

    // 取出當前格子資訊與物件並設 explored = true，不呼叫 mark_dirty
    let mut current_cell_objects = Vec::new();
    crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(SquareCoord::new(gx, gy)) {
            cell.explored = true;
            current_cell_objects = cell.objects.clone();
        }
    });

    // ── Empty 強制短路 ───────────────────────────────────────────────
    let is_bamboo_slot = cell_info.as_ref().map(crate::grid::zhulin::is_bamboo_slot_cell).unwrap_or(false);
    let explore_msg = if freshness == Freshness::Empty && !is_bamboo_slot {
        if let Err(e) = db::update_cell_state(gx, gy, true, Freshness::Empty, &current_cell_objects) {
            tracing::warn!("[explore_ws] cell_state 寫入 PG 失敗：{e}");
        }
        ExploreLogMsg {
            msg_type: "explore_result".into(),
            outcome: "empty".into(),
            log: "此地已被搜刮一空，草木未復，再怎麼翻也只剩塵土。".into(),
            loot: vec![],
            stat_delta: StatDelta::default(),
            time_period_after: tp.label().into(),
            freshness_after: "empty".into(),
            warning_prose: String::new(),
        }
    } else {
        // 推進 freshness
        let new_freshness = freshness.advance();
        crate::server::grid_manager::with_square_grid_mut(|g| {
            if let Some(c) = g.get_mut(SquareCoord::new(gx, gy)) {
                c.freshness = new_freshness;
            }
        });
        if let Err(e) = db::update_cell_state(gx, gy, true, new_freshness, &current_cell_objects) {
            tracing::warn!("[explore_ws] cell_state 寫入 PG 失敗：{e}");
        }
        let roll: f64 = rand::rng().random();

        // 判定是否屬於荒野自然地形
        let is_wilderness = matches!(
            cell_info.as_ref().map(|c| c.terrain),
            Some(
                Terrain::Plain
                    | Terrain::Grassland
                    | Terrain::Desert
                    | Terrain::Tundra
                    | Terrain::Forest
                    | Terrain::ForestHeavy
                    | Terrain::ForestLight
                    | Terrain::Jungle
                    | Terrain::Mountain
                    | Terrain::Hills
                    | Terrain::MinePass
                    | Terrain::Swamp
                    | Terrain::Water
                    | Terrain::WaterDeep
                    | Terrain::Beach
            )
        );

        let is_peninsula = crate::server::grid_manager::get_runtime_square_grid()
            .is_some_and(|g| g.has_peninsula_wilderness());
        let (outcome, log, loot) = if cell_info.as_ref().map(crate::grid::zhulin::is_bamboo_slot_cell).unwrap_or(false) {
            let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
            let north_cell = grid_snap.as_ref().and_then(|g| g.get(SquareCoord::new(gx, gy + 1)));
            let south_cell = grid_snap.as_ref().and_then(|g| g.get(SquareCoord::new(gx, gy - 1)));
            let east_cell = grid_snap.as_ref().and_then(|g| g.get(SquareCoord::new(gx + 1, gy)));
            let west_cell = grid_snap.as_ref().and_then(|g| g.get(SquareCoord::new(gx - 1, gy)));
            let mut neighbor_refs: Vec<(&str, &crate::grid::GridCell)> = Vec::new();
            if let Some(c) = north_cell {
                neighbor_refs.push(("北", c));
            }
            if let Some(c) = south_cell {
                neighbor_refs.push(("南", c));
            }
            if let Some(c) = east_cell {
                neighbor_refs.push(("東", c));
            }
            if let Some(c) = west_cell {
                neighbor_refs.push(("西", c));
            }
            let inv_json = db::get_entity(&player_id)
                .ok()
                .flatten()
                .map(|e| e.inventory)
                .unwrap_or_default();
            let now_unix = crate::game::now_unix();
            let (_, gh, _, _gday) = game::game_time_now(now_unix, conn.cfg.game_time_epoch_unix, conn.cfg.game_time_scale);
            let weather = crate::grid::zhulin::active_zhulin_weather_for_hour(None, gh.max(0) as u32);
            let current_c = cell_info.as_ref().unwrap();
            let (yield_opt, text) = crate::grid::zhulin::explore_bamboo_understory(current_c, &neighbor_refs, &inv_json, weather);
            if let Some((item_id, qty)) = yield_opt {
                let _ = db::add_to_inventory(&player_id, item_id, qty as i32);
                let item_name = crate::grid::zhulin::lookup_zhulin_item_def(item_id)
                    .map(|d| d.name)
                    .unwrap_or(item_id);
                let loot = vec![LootItem {
                    item_id: item_id.into(),
                    name: item_name.into(),
                    qty: qty as i32,
                }];
                ("harvest".into(), text, loot)
            } else {
                ("trace".into(), text, vec![])
            }
        } else if !is_peninsula {
            if let Some(obj) = current_cell_objects.iter().find(|o| o.quantity > 0)
                && let Some(item_id) = crate::grid::zhulin::yield_item_id_for_object(&obj.id)
                && let Some(item_def) = crate::grid::zhulin::lookup_zhulin_item_def(item_id)
            {
                let _ = crate::db::add_to_inventory(&player_id, item_id, 1);
                let prose = format!("你在【{}】周邊細細巡看，順手採得 1 份「{}」收入行囊。", obj.name, item_def.name);
                let loot = vec![LootItem {
                    item_id: item_id.into(),
                    name: item_def.name.into(),
                    qty: 1,
                }];
                ("harvest".into(), prose, loot)
            } else {
                (
                    "trace".into(),
                    "你循著四周地貌細細察看，風聲掠過林梢與土坡，周遭靜謐無異。".into(),
                    vec![],
                )
            }
        } else if !is_wilderness {
            if roll < 0.45 {
                let prose = "循著街巷石階與屋簷廊下細細巡看，往來行旅與街坊步履匆匆，車轍與石縫間留有商隊裝卸貨匣的壓痕，四周秩序井然。";
                ("trace".into(), prose.into(), vec![])
            } else {
                let choice = rand::rng().random_range(0..4);
                let (prose, item_id, item_name, qty) = match choice {
                    0 => (
                        "在街角石隙與鋪路散石間拾得數塊質地平整的緻密硬石，可供取材之用。",
                        "harvest_hardstone",
                        "緻密硬石",
                        1,
                    ),
                    1 => (
                        "於街坊屋簷角落與貨架旁撿拾得些許乾燥枯枝，質地乾燥無損。",
                        "harvest_deadwood",
                        "乾燥枯枝",
                        2,
                    ),
                    2 => (
                        "自坊牆陰涼青苔處取下少許強韌葛藤，柔韌完好。",
                        "harvest_vine",
                        "強韌葛藤",
                        1,
                    ),
                    _ => (
                        "在屋舍背陰石隙中辨得一株清香的野地草藥，順手採下入包。",
                        "harvest_herb",
                        "野地草藥",
                        1,
                    ),
                };
                let _ = crate::db::add_to_inventory(&player_id, item_id, qty);
                let loot = vec![LootItem {
                    item_id: item_id.into(),
                    name: item_name.into(),
                    qty,
                }];
                ("harvest".into(), prose.into(), loot)
            }
        } else if roll < 0.15 {
            // ④ 深入險地驚動護巢猛獸（戰鬥結算）
            let is_marsh = cell_info
                .as_ref()
                .map(|c| c.name.contains("黑沼") || c.zone.contains("黑沼") || c.description.contains("黑沼"))
                .unwrap_or(false);
            if is_marsh {
                crate::pg::living_world::record_entity_affliction(
                    &player_id,
                    crate::pg::living_world::AfflictionKind::Miasma,
                    "explore_marsh",
                    now,
                );
            }
            let dmg: i32 = (rand::rng().random_range(8_u32..=18_u32)) as i32;
            let hp_after = (ent.vit - dmg).max(0);
            let _ = db::update_vit(&player_id, hp_after);

            if hp_after == 0 {
                // 力竭昏迷：送回鄰近聚落脈絡診所 + 救治，純體感傷勢描寫，嚴禁出現數字
                let (dest_x, dest_y, dest_name) = find_nearest_clinic_or_landmark(gx, gy)
                    .unwrap_or((0, 0, "脈絡診所".to_string()));

                let _ = db::set_entity_grid(&player_id, dest_x, dest_y);
                let dropped_names = faint_drop_items_to_ground(&player_id, &ent.inventory, gx, gy, now);

                // 恢復安全氣血量（純後台，文字絕不出現恢復30%等數字）
                let max_hp = ent.vit.max(30);
                let recover_to = (max_hp as f64 * 0.3).ceil().max(10.0) as i32;
                let _ = db::update_vit(&player_id, recover_to);

                let drop_note = if dropped_names.is_empty() {
                    String::new()
                } else {
                    format!(
                        "醒轉時察覺行囊微敞，部分隨身物資（{}）已不知散落何方。",
                        dropped_names.join("、")
                    )
                };
                let faint_log = format!(
                    "凶威暴烈，胸口受此沉重重擊，眼前驟然一黑沉入虛無——待悠悠轉醒，鼻端縈繞著草藥與導能凝膠的清冽香氣，傷口已被細心包紮止血，正臥於【{}】的病榻之上。{}",
                    dest_name, drop_note
                );
                let faint_msg = ExploreLogMsg {
                    msg_type: "explore_result".into(),
                    outcome: "faint".into(),
                    log: faint_log,
                    loot: vec![],
                    stat_delta: StatDelta {
                        hp_cur: -ent.vit,
                        inner_cur: 0,
                    },
                    time_period_after: tp.label().into(),
                    freshness_after: freshness.as_str().into(),
                    warning_prose: String::new(),
                };
                conn.send_json(&faint_msg);
                send_grid_view(conn, &player_id, dest_x, dest_y, gh);
                let faint_room = crate::grid::grid_room_id_from_coord(dest_x, dest_y);
                emit_presence_reflex_on_arrival(conn, &player_id, &faint_room);
                return;
            }

            // 未昏迷：純體感傷勢描寫，嚴禁出現「受傷 15 點」等數字
            let prose = match terrain_str {
                "forest" | "forest_heavy" | "jungle" => {
                    "密林深處狂風驟起，護巢猛獸自樹冠陰影撲殺而下！利爪撕裂襟袖、擦過皮肉，火辣灼痛驟起，所幸抽身迅疾、借勢翻滾，才將兇獸格開逼退，氣息翻湧難平。"
                }
                "mountain" | "hills" | "mine_pass" => {
                    "亂石崩飛，岩隙盤踞的猛獸張口怒咆、暴起衝撞！重擊橫掃之下背脊撞上嶙峋石壁，氣血震盪窒悶，連退數步才穩住架勢，持械將其震退。"
                }
                "swamp" => {
                    "黑泥四濺，蟄伏沼底的巨蜥猛然甩尾噬咬！毒涎擦肩而過、帶出灼痛血痕，泥濘中幾經險象環生的纏鬥，始以硬擊迫使猛獸退回水潭深處。"
                }
                _ => {
                    "四下突起腥風，護巢凶獸猛撲而至！利爪劃破甲冑衣袍，身軀受創沉重，咬緊牙關奮力還擊，終於逼退強敵，周身隱隱作痛、冷汗涔涔。"
                }
            };

            let combat_msg = ExploreLogMsg {
                msg_type: "explore_result".into(),
                outcome: "combat".into(),
                log: prose.into(),
                loot: vec![],
                stat_delta: StatDelta {
                    hp_cur: -dmg,
                    inner_cur: 0,
                },
                time_period_after: tp.label().into(),
                freshness_after: new_freshness.as_str().into(),
                warning_prose: String::new(),
            };
            conn.send_json(&combat_msg);
            send_grid_view(conn, &player_id, gx, gy, gh);
            return;
        } else if roll < 0.30 {
            // ③ 發現大型巨獸蹤跡／掉落獸鱗並提示延伸方向
            let (prose, scale_item) = match terrain_str {
                "forest" | "forest_heavy" | "jungle" => (
                    "林木齊腰折斷，潮濕黑土上深印著數道半尺深的巨獸裂爪印，枝頭刮落一片巴掌大小、泛著灰白冷光的堅硬獸鱗。腥風自北方林岫深處徐徐翻湧，龐然凶兆正盤踞於彼端。",
                    Some(("beast_scale", "巨獸殘鱗")),
                ),
                "mountain" | "hills" | "mine_pass" => (
                    "陡峭崖壁上布滿粗重凌厲的爪痕鑿印，碎石落處留有半截暗沉如鐵的裂岩巨爪痕跡。爪痕一路往東北險峰攀附延伸，深穴之中似有沉重鼻息回蕩。",
                    Some(("beast_claw_fragment", "裂岩巨爪殘片")),
                ),
                "swamp" | "water" | "beach" => (
                    "泥濘退水之處，深凹的巨鰭掌印已被黑水浸滿，浮水處漂浮著一枚色呈墨綠、邊緣鋒銳的巨獸鰭鱗。水流暗湧，痕跡筆直溯游向東南深潭深處。",
                    Some(("beast_scale", "巨獸鰭鱗")),
                ),
                _ => (
                    "地表塵土翻卷，隱約留有一道寬闊蜿蜒的巨型甲軀拖行巨痕，並沿途散落些許焦黑蛻殼碎片。痕跡筆直指向西北荒丘，顯然有巨獸自此處遷徙而過。",
                    Some(("beast_scale", "巨獸殘鱗")),
                ),
            };

            let mut loot = vec![];
            if let Some((item_id, item_name)) = scale_item {
                let _ = crate::db::add_to_inventory(&player_id, item_id, 1);
                loot.push(LootItem {
                    item_id: item_id.into(),
                    name: item_name.into(),
                    qty: 1,
                });
            }
            ("trace".into(), prose.into(), loot)
        } else if roll < 0.55 {
            // ② 發現溫馴獵物（如低頭覓食的草甸角鹿，提示可自行決定是否出手狩獵，絕不強制拉入戰鬥）
            let prose = match terrain_str {
                "forest" | "forest_heavy" | "forest_light" | "jungle" | "plain" | "grassland" => {
                    "薄霧微散，一頭草甸角鹿正伏首細嗅草尖甘露，偶爾警覺揚蹄，神態溫馴而優雅。四下靜謐，此處可自行審度是否凝神出手狩獵，若無殺念亦可安靜離去。"
                }
                "mountain" | "hills" => {
                    "危岩高處，一隻盤角岩羊正矯健躍過石壁，立於突岩上俯瞰群山。山風獵獵，若無狩獵取角之圖，靜觀其蹄影遠去即可。"
                }
                "swamp" | "water" | "beach" => {
                    "水渚泛起微瀾，幾隻長羽白鷺自茂密蘆葦叢間振翅低飛、悠然涉水覓食。遠望生機盎然，行者可自行決斷是否引動殺機，或留其一份自在。"
                }
                _ => {
                    "草叢沙沙作響，一隻靈動的野兔自灌木間探頭張望，蹦跳著四處覓食。四野安適，若無狩獵果腹之需，無須驚動這片刻安寧。"
                }
            };
            ("prey".into(), prose.into(), vec![])
        } else {
            // ① 發現該獵場物產（草藥、礦石、奇木、硬石、枯枝、韌藤，零門檻破死鎖）
            let (prose, item_id, item_name, qty) = match terrain_str {
                "forest" | "forest_heavy" | "forest_light" | "jungle" => {
                    let choice = rand::rng().random_range(0..3);
                    match choice {
                        0 => (
                            "撥開茂密枝葉與覆地青苔，尋得質地緊實的乾燥枯枝與柔韌葛藤，可供取材利用。",
                            "harvest_deadwood",
                            "乾燥枯枝",
                            2,
                        ),
                        1 => (
                            "拂去老樹盤根處的積土，尋得一截帶著淡淡松香的溫木心材，木質溫潤堅實。",
                            "harvest_rare_wood",
                            "溫木心材",
                            1,
                        ),
                        _ => (
                            "穿行於林隙灌木間，自藤蔓掩映處採得一捧新鮮潤澤的甘露野莓，露水未乾。",
                            "harvest_berries",
                            "甘露野莓",
                            2,
                        ),
                    }
                }
                "mountain" | "hills" | "mine_pass" => {
                    let choice = rand::rng().random_range(0..2);
                    if choice == 0 {
                        (
                            "敲鑿裸露岩隙，崩落數塊質地緻密的硬石與伴生碎礦，觸手冰涼沉甸。",
                            "harvest_hardstone",
                            "緻密硬石",
                            2,
                        )
                    } else {
                        (
                            "循著岩層夾縫仔細撥尋，於礦脈表層剝下數塊帶暗紅紋路的赤紋鐵礦，分量沉實。",
                            "harvest_ore",
                            "赤紋鐵礦",
                            1,
                        )
                    }
                }
                "swamp" => {
                    let choice = rand::rng().random_range(0..2);
                    if choice == 0 {
                        (
                            "踩著浮萍泥淖探摸，於糾結盤纏的水澤深處挽出數根強韌葛藤，柔韌異常。",
                            "harvest_vine",
                            "強韌葛藤",
                            2,
                        )
                    } else {
                        (
                            "泥水沒過靴緣，在淺灘浮洲間採得一叢能止血拔毒的止血水苔，微帶辛涼氣息。",
                            "harvest_herb",
                            "止血水苔",
                            1,
                        )
                    }
                }
                "water" | "beach" => {
                    let choice = rand::rng().random_range(0..2);
                    if choice == 0 {
                        (
                            "踏著浪痕與退潮碎石檢視，自水窪中拾得數枚被潮水摩挲得極其平整的硬石。",
                            "harvest_hardstone",
                            "緻密硬石",
                            2,
                        )
                    } else {
                        (
                            "水沫拍岸，自礁灘沙土間拾起一截久浸不腐的沼生烏木，木質堅韌沉水。",
                            "wood_iron_oak",
                            "沼生烏木",
                            1,
                        )
                    }
                }
                _ => {
                    let choice = rand::rng().random_range(0..2);
                    if choice == 0 {
                        (
                            "伏身細巡草石起伏，自砂礫與草根間採得一叢清香微苦的野地草藥，生機充沛。",
                            "harvest_herb",
                            "野地草藥",
                            1,
                        )
                    } else {
                        (
                            "撥開風化碎石與枯枝，拾掇得幾塊分量十足的硬石與乾燥柴薪，備作營地之用。",
                            "harvest_hardstone",
                            "緻密硬石",
                            2,
                        )
                    }
                }
            };

            let _ = crate::db::add_to_inventory(&player_id, item_id, qty);
            let loot = vec![LootItem {
                item_id: item_id.into(),
                name: item_name.into(),
                qty,
            }];
            ("harvest".into(), prose.into(), loot)
        };

        ExploreLogMsg {
            msg_type: "explore_result".into(),
            outcome,
            log,
            loot,
            stat_delta: StatDelta::default(),
            time_period_after: tp.label().into(),
            freshness_after: new_freshness.as_str().into(),
            warning_prose: String::new(),
        }
    };

    conn.send_json(&explore_msg);
    send_grid_view(conn, &player_id, gx, gy, gh);
}


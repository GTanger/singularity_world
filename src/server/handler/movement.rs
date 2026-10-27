//! 移動／探索／離場觀測清除。六角與正方格雙路徑。

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use parking_lot::RwLock;
use rand::Rng;

use crate::db;
use crate::event::{self, types::BLOCKED};
use crate::game;
use crate::game::TimePeriod;
use crate::gametext;
use crate::grid::{Freshness, SquareCoord, Terrain};

use super::super::protocol::{ActionResultMsg, BlockedMsg, ClientMsg, ExploreLogMsg, LootItem, MovedMsg, StatDelta};
use super::{WsConnection, current_game_hour};

/// 線程安全的私人客房狀態集合（記錄處於私人客房中的玩家 ID）
static IN_PRIVATE_ROOM: LazyLock<RwLock<HashSet<String>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

/// 查詢玩家是否處於私人客房狀態
pub fn is_player_in_private_room(player_id: &str) -> bool {
    IN_PRIVATE_ROOM.read().contains(player_id)
}

/// 設定玩家私人客房狀態
pub fn set_player_private_room(player_id: &str, in_room: bool) {
    let mut lock = IN_PRIVATE_ROOM.write();
    if in_room {
        lock.insert(player_id.to_string());
    } else {
        lock.remove(player_id);
    }
}

/// 探索冷卻計時：(player_id, x, y) -> 過期 unix timestamp
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
    let days = game_days_since_epoch.max(0);
    let year = (days / 365) + 1;
    let day_in_year = days % 365;
    let (season_name, day_in_season) = if day_in_year < 91 {
        ("春融", day_in_year + 1)
    } else if day_in_year < 182 {
        ("夏漲", day_in_year - 91 + 1)
    } else if day_in_year < 273 {
        ("秋狩", day_in_year - 182 + 1)
    } else {
        ("冬凝", day_in_year - 273 + 1)
    };
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
    let explore_msg = if freshness == Freshness::Empty {
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

        let (outcome, log, loot) = if !is_wilderness {
            // 聚落／道路／建築／農田安全地帶：絕不觸發猛獸襲擊與巨獸殘鱗
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
                    "凶威暴烈，胸口受此沉重重擊，眼前驟然一黑沉入虛無——待悠悠轉醒，鼻端縈繞著草藥與導能凝膠的清冽香氣，傷口已被細心包紮止血，正臥於【{}】的病床之上。{}",
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
                    "密林深處狂風驟起，護巢猛獸自樹冠陰影撲殺而下！利爪撕裂襟袖、擦過皮肉，火辣灼痛驟起，所幸抽身迅疾、借勢翻滾，方將兇獸格開逼退，氣息翻湧難平。"
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
                    "地表塵土翻卷，隱約留有一道寬闊蜿蜒的巨型甲軀拖行巨痕，並沿途散落些許焦黑蛻殼碎片。痕跡筆直指向西北荒丘，顯然有巨獸方自此處遷徙。",
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
                    "草叢沙沙作響，一隻靈動的野兔自灌木間探頭張望，旋即蹦跳覓食。四野安適，若無狩獵果腹之需，無須驚動這片刻安寧。"
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
                            "harvest_rare_wood",
                            "溫木心材",
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

pub(super) fn handle_move(conn: &WsConnection, msg: &ClientMsg) {
    let pid = conn.player_id.read().ok().and_then(|g| g.clone());
    let Some(player_id) = pid else {
        conn.send_error("login first");
        return;
    };
    let mut direction = msg.direction.clone();
    let ent = db::get_entity(&player_id).ok().flatten();
    let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));

    if direction.is_empty()
        && (msg.x.is_some() || msg.y.is_some())
        && let (Some(tx), Some(ty)) = (msg.x, msg.y)
    {
        match (tx - gx, ty - gy) {
            (0, 1) => direction = "北".to_string(),
            (0, -1) => direction = "南".to_string(),
            (1, 0) => direction = "東".to_string(),
            (-1, 0) => direction = "西".to_string(),
            _ => {}
        }
    }
    if direction.is_empty()
        && !msg.target_place_id.is_empty()
        && let Some(store) = crate::store::get_store()
    {
        let guard = store.read();
        if let Some(from_place) = guard.get_entity_place_id(&player_id)
            && let Some(route) = guard.place_graph.route_to(&from_place, &msg.target_place_id)
        {
            direction = route.direction.clone();
        }
    }
    if direction.is_empty() {
        conn.send_error("direction required");
        return;
    }

    // 私人客房「進／出」分流（支援中英文別名：進/in/enter, 出/out/leave）
    let dir_trimmed = direction.trim();
    let dir_lower = dir_trimmed.to_ascii_lowercase();
    let is_dir_in = dir_trimmed == "進" || dir_lower == "in" || dir_lower == "enter";
    let is_dir_out = dir_trimmed == "出" || dir_lower == "out" || dir_lower == "leave";

    if is_dir_in {
        let gh = current_game_hour(&conn.cfg);
        let can_enter = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(SquareCoord::new(gx, gy)).cloned())
            .map(|c| {
                let name = c.name.trim();
                let is_lodging = name.contains("旅社")
                    || name.contains("客棧")
                    || (name.contains("行館") && !name.contains("印行館"))
                    || name.contains("歇舍")
                    || name.contains("驛館");
                is_lodging
            })
            .unwrap_or(false);

        if can_enter {
            set_player_private_room(&player_id, true);
            send_grid_view(conn, &player_id, gx, gy, gh);
        } else {
            let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
            conn.send_json(&BlockedMsg {
                msg_type: "blocked".into(),
                direction: direction.clone(),
            });
        }
        return;
    } else if is_dir_out {
        set_player_private_room(&player_id, false);
        let gh = current_game_hour(&conn.cfg);
        send_grid_view(conn, &player_id, gx, gy, gh);
        return;
    } else {
        // 移動至其他方向時自動脫離私人客房
        if is_player_in_private_room(&player_id) {
            set_player_private_room(&player_id, false);
        }
    }
    if matches!(direction.as_str(), "東北" | "西北" | "東南" | "西南") {
        let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
        conn.send_json(&BlockedMsg {
            msg_type: "blocked".into(),
            direction: direction.clone(),
        });
        return;
    }
    // 優先在 SquareGrid（5,040 格白塔半島）上進行移動
    let delta_opt = match direction.as_str() {
        "北" => Some((0, 1)),
        "南" => Some((0, -1)),
        "東" => Some((1, 0)),
        "西" => Some((-1, 0)),
        _ => None,
    };


    if let Some((dx, dy)) = delta_opt {
        let (nx, ny) = (gx + dx, gy + dy);
        let target_coord = SquareCoord::new(nx, ny);
        let grid_snap = crate::server::grid_manager::get_runtime_square_grid();
        let target_cell = grid_snap.as_ref().and_then(|g| g.get(target_coord).cloned());

        if let Some(cell) = target_cell {
            let can_move = grid_snap
                .as_ref()
                .map(|g| can_step_in_nine_grid(g, SquareCoord::new(gx, gy), target_coord))
                .unwrap_or_else(|| cell.terrain.walkable());
            if can_move {
                let old_room = crate::grid::grid_room_id_from_coord(gx, gy);
                let (gx, gy) = (nx, ny);

                let is_water_or_coastal = matches!(
                    cell.terrain,
                    Terrain::Water | Terrain::WaterDeep | Terrain::Beach | Terrain::Dock
                ) || cell.name.contains('海')
                    || cell.name.contains('水')
                    || cell.name.contains('灘')
                    || cell.name.contains('渚')
                    || cell.name.contains('渡')
                    || cell.name.contains('港');
                let is_outdoor_wild = matches!(
                    cell.terrain,
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
                        | Terrain::Beach
                        | Terrain::EmptyFurrow
                        | Terrain::DryField
                        | Terrain::FarmField
                );
                let is_outdoor_step = is_outdoor_wild || is_water_or_coastal;
                let is_rugged = matches!(
                    cell.terrain,
                    Terrain::Mountain
                        | Terrain::Hills
                        | Terrain::MinePass
                        | Terrain::ForestHeavy
                        | Terrain::Forest
                        | Terrain::ForestLight
                        | Terrain::Jungle
                        | Terrain::Swamp
                        | Terrain::Desert
                        | Terrain::Tundra
                ) || cell.terrain.move_cost() >= 1.5;

                let now = crate::game::now_unix();
                if is_outdoor_step && (gx + gy).abs() % 6 == 0 {
                    let habit_kind = if is_water_or_coastal {
                        crate::pg::living_world::BreathHabitKind::TideContinuous
                    } else {
                        crate::pg::living_world::BreathHabitKind::ForestLong
                    };
                    crate::pg::living_world::record_breath_activity(&player_id, habit_kind, 1, now);
                }

                let craft_dims = crate::server::handler::entity_total_craft_dims(&player_id);
                let seed = ent.as_ref().and_then(|e| e.soul_seed).unwrap_or(0);
                let breath = crate::pg::living_world::get_or_derive_breath_habit(&player_id, "", seed);
                let has_leg_sprain = crate::pg::living_world::get_entity_afflictions(&player_id)
                    .contains(&crate::pg::living_world::AfflictionKind::LegSprain);

                if has_leg_sprain {
                    if let Ok(Some(current_ent)) = db::get_entity(&player_id) {
                        let new_vit = (current_ent.vit - 1).max(5);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                        }
                    }
                    if is_rugged && (gx + gy).abs() % 4 == 0 {
                        conn.send_json(&super::super::protocol::NarrateMsg {
                            msg_type: "narrate".into(),
                            text: "【下盤扭傷】跋涉間足踝經絡隱隱作痛，步履略顯沉重，宜尋醫館或敷用舒筋藥膏。".into(),
                        });
                    }
                } else if (craft_dims.wayfare >= 0.8
                    || matches!(
                        breath.dominant,
                        crate::pg::living_world::BreathHabitKind::ForestLong
                            | crate::pg::living_world::BreathHabitKind::TideContinuous
                    ))
                    && is_rugged
                    && (gx + gy).abs() % 9 == 0
                {
                    if let Ok(Some(current_ent)) = db::get_entity(&player_id) {
                        let new_vit = (current_ent.vit + 1).min(100);
                        if new_vit != current_ent.vit {
                            let _ = db::update_vit(&player_id, new_vit);
                        }
                    }
                    conn.send_json(&super::super::protocol::NarrateMsg {
                        msg_type: "narrate".into(),
                        text: "【行旅涉險・長息輕身】你順著山海地勢勻長換氣，腳下借力輕省，跋涉不覺疲累。".into(),
                    });
                }
                if let Err(e) = db::set_entity_grid(&player_id, nx, ny) {
                    tracing::error!("set_entity_grid failed for {player_id} to ({nx}, {ny}): {e}");
                    conn.send_error(gametext::client("move_failed"));
                    return;
                }
                let new_room = crate::grid::grid_room_id_from_coord(nx, ny);
                let cell_name = if cell.name.trim().is_empty() {
                    crate::grid::terrain_name_zh(cell.terrain).to_string()
                } else {
                    cell.name.clone()
                };

                let moved = MovedMsg {
                    msg_type: "moved".into(),
                    player_id: player_id.to_string(),
                    room_id: new_room.clone(),
                    room_name: cell_name,
                };
                let bytes = serde_json::to_vec(&moved).unwrap_or_default();
                conn.hub.broadcast(bytes);
                super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &old_room, Some(&player_id));
                super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &new_room, Some(&player_id));

                let gh = current_game_hour(&conn.cfg);
                let claimed = crate::pg::living_world::claim_ground_dropped_items(nx, ny);
                if !claimed.is_empty() {
                    for (item_id, _name, qty) in &claimed {
                        let _ = db::add_to_inventory(&player_id, item_id, *qty);
                    }
                    conn.send_json(&super::super::protocol::NarrateMsg {
                        msg_type: "narrate".into(),
                        text: "你在地上尋回了先前遺落的隨身物資。".into(),
                    });
                    super::push_refresh(conn, &player_id);
                }
                send_grid_view(conn, &player_id, nx, ny, gh);
                emit_presence_reflex_on_arrival(conn, &player_id, &new_room);
                return;
            } else {
                let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
                conn.send_json(&BlockedMsg {
                    msg_type: "blocked".into(),
                    direction: direction.clone(),
                });
                return;
            }
        } else {
            let _ = event::append(game::now_unix(), &player_id, BLOCKED, &direction);
            conn.send_json(&BlockedMsg {
                msg_type: "blocked".into(),
                direction: direction.clone(),
            });
            return;
        }
    }

    if db::get_entity_place_id(&player_id).is_none() {
        conn.send_error(gametext::client("move_failed"));
        return;
    }
    handle_move_place(
        conn,
        &player_id,
        &direction,
        (!msg.target_place_id.is_empty()).then_some(msg.target_place_id.as_str()),
    );
}
fn handle_move_place(
    conn: &WsConnection,
    player_id: &str,
    direction: &str,
    target_place_id: Option<&str>,
) {
    let old_room = db::get_entity_room(player_id).unwrap_or_default();
    let (new_room, ok) = match game::move_by_place_route(player_id, direction, target_place_id) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("player move failed {player_id}: {e}");
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
    if crate::pg::living_world::get_entity_afflictions(player_id)
        .contains(&crate::pg::living_world::AfflictionKind::LegSprain)
    {
        if let Ok(Some(current_ent)) = db::get_entity(player_id) {
            let new_vit = (current_ent.vit - 1).max(5);
            if new_vit != current_ent.vit {
                let _ = db::update_vit(player_id, new_vit);
            }
        }
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: "【下盤扭傷】跋涉間足踝經絡隱隱作痛，步履略顯沉重，宜尋醫館或敷用舒筋藥膏。".into(),
        });
    }
    let gh = current_game_hour(&conn.cfg);
    let room_name = game::get_place_room_view(&new_room, gh)
        .ok()
        .flatten()
        .map(|v| v.room.name)
        .unwrap_or_else(|| new_room.clone());
    let moved = MovedMsg {
        msg_type: "moved".into(),
        player_id: player_id.to_string(),
        room_id: new_room.clone(),
        room_name,
    };
    let bytes = serde_json::to_vec(&moved).unwrap_or_default();
    conn.hub.broadcast(bytes);
    super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &old_room, Some(player_id));
    super::super::broadcast::refresh_room_views_for_room_except(&conn.sessions, &conn.cfg, &new_room, Some(player_id));
    let ent = db::get_entity(player_id).ok().flatten();
    let (gx, gy) = ent.as_ref().and_then(|e| e.grid_x.zip(e.grid_y)).unwrap_or((0, 0));
    send_grid_view(conn, player_id, gx, gy, gh);
    emit_presence_reflex_on_arrival(conn, player_id, &new_room);
}

pub(crate) fn emit_presence_reflex_on_arrival(conn: &WsConnection, player_id: &str, new_room: &str) {
    let now = crate::game::now_unix();
    {
        let mut cds = PRESENCE_REFLEX_COOLDOWN.write();
        if let Some(&expire) = cds.get(&(player_id.to_string(), new_room.to_string())) {
            if now < expire {
                return;
            }
        }
        cds.insert((player_id.to_string(), new_room.to_string()), now + 8);
    }

    let gh = current_game_hour(&conn.cfg);
    let entities = if let Ok(Some(view)) = game::get_place_room_view(new_room, gh) {
        view.entities
    } else if let Ok(ents) = db::get_entities_in_room(new_room, -1) {
        ents
    } else {
        return;
    };

    let mut wild_creatures = Vec::new();
    let mut npcs = Vec::new();
    for e in entities {
        if e.kind == crate::entity::EntityKind::Npc && e.id != player_id {
            if e.id.starts_with("shanhai_") || e.id.starts_with("ambient_") {
                wild_creatures.push(e);
            } else {
                npcs.push(e);
            }
        }
    }

    if npcs.is_empty() {
        if let Some(creature) = wild_creatures.first() {
            let c_act = creature
                .current_activity
                .trim()
                .trim_start_matches("正在")
                .trim_start_matches('正');
            let line = format!(
                "「林隙草木間傳來細碎動靜——不遠處的【{}】正{}，察覺有人踏足此處，頓時警覺地昂首凝視。」",
                creature.display_title,
                if c_act.is_empty() { "於棲地間巡行覓食" } else { c_act }
            );
            conn.send_json(&super::super::protocol::NarrateMsg {
                msg_type: "narrate".into(),
                text: line,
            });
        }
        return;
    }
    let get_display_name = |npc: &crate::entity::Character| -> String {
        let is_named = db::get_acquaintance(player_id, &npc.id)
            .ok()
            .flatten()
            .map(|a| a.named)
            .unwrap_or(false);
        if is_named {
            crate::store::resolve_npc_person_name(&npc.id, &npc.display_title, None)
        } else {
            let tag = crate::store::npc_brief_appearance_tag(&npc.id, &npc.display_title);
            let in_hunting_party = crate::store::LIVE_HUNTING_PARTIES
                .read()
                .values()
                .any(|p| p.member_ids.contains(&npc.id));
            let role = if in_hunting_party || tag.contains("合社獵團") {
                "獵手"
            } else if tag.contains("入山百工") {
                "山林百工"
            } else if tag.contains("古道行旅") {
                "行旅鏢客"
            } else if npc.display_title.contains("鐵") || npc.display_title.contains("匠") {
                "匠人"
            } else if npc.display_title.contains("商") || npc.display_title.contains("販") {
                "攤販"
            } else if npc.display_title.contains("醫") || npc.display_title.contains("藥") {
                "醫工"
            } else if npc.display_title.contains("學") || npc.display_title.contains("書") {
                "學者"
            } else {
                "街坊"
            };
            if !tag.is_empty() {
                format!("{tag}的{role}")
            } else {
                format!("一名{role}")
            }
        }
    };
    let awake_npcs: Vec<_> = npcs
        .iter()
        .filter(|e| !e.current_activity.contains("安歇") && !e.current_activity.contains("熟睡") && !e.current_activity.contains("熄燈") && !e.current_activity.contains("就寢"))
        .cloned()
        .collect();

    // 0. 玩家在場反應（Presence Reflex）優先判定
    // (a) 若在場有熟識的 NPC，主動相認或點頭招呼
    if let Some(known_npc) = awake_npcs.iter().find(|n| crate::store::player_knows_npc(player_id, &n.id)) {
        let name = crate::store::resolve_npc_person_name(&known_npc.id, &known_npc.display_title, None);
        let reflex = match (crate::game::now_unix() as usize + known_npc.id.len()) % 3 {
            0 => format!("「{name}見你邁步走入，神色微動，溫和點頭致意：『別來無恙，先前還在思忖何時能與你會面。』」"),
            1 => format!("「{name}停下手頭的事務，抬眼望見你，含笑招呼：『旅途可還順遂？若有事需要打點，隨時知會一聲。』」"),
            _ => format!("「{name}朝你微微頷首：『來得正好，四方市面平穩，請入內歇步。』」"),
        };
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: reflex,
        });
        return;
    }

    // (a2) 若在場有「合社獵團（`LIVE_HUNTING_PARTIES`）」成員，主動通報圍獵進度並號召同行分鹿
    let hunting_reflex = {
        let parties = crate::store::LIVE_HUNTING_PARTIES.read();
        awake_npcs.iter().find_map(|n| {
            parties
                .values()
                .find(|p| p.member_ids.contains(&n.id))
                .map(|p| (n, p.clone()))
        })
    };
    if let Some((hunter_npc, party)) = hunting_reflex {
        let caller = get_display_name(hunter_npc);
        let (px, py) = party.prey_coord;
        let reflex = match party.phase {
            crate::store::HuntingPartyPhase::Rallying => format!(
                "「{caller}手按腰間山豬牙號角，見你踏入此處，精神一振低聲招呼：『來得正好！領隊【{}】正召集大夥兒（現已聚齊 {} 人）準備合圍座標 ({px}, {py}) 的【{}】，你若願一同助陣分鹿，儘管與我交談（Talk）入團！』」",
                party.leader_name,
                party.member_ids.len(),
                party.prey_name
            ),
            crate::store::HuntingPartyPhase::SurroundingPrey => format!(
                "「{caller}壓低身形、緊盯座標 ({px}, {py}) 方向的【{}】，見你靠近連忙打出手勢低喝：『別驚動獵物！我們【{}】合社獵團正從八方收攏包圍圈，隨同出手合擊者見者有份！』」",
                party.prey_name, party.leader_name
            ),
            crate::store::HuntingPartyPhase::TakedownAndSplit => format!(
                "「{caller}正與【{}】合社獵團夥伴全力圍殺【{}】、刀光與號角齊鳴，見你趕到高呼道：『快搭把手合擊此獸，剝取的【{}】與賞格見者有份！』」",
                party.leader_name, party.prey_name, party.prey_loot_name
            ),
            crate::store::HuntingPartyPhase::ReturningToMarket => format!(
                "「{caller}肩扛圍獵所獲的【{}】，隨【{}】合社獵團結伴趕往【{}】交割分紅，見你迎面走來爽朗笑道：『這趟合圍【{}】大獲全勝，正要進鎮過磅分鎂幣呢！』」",
                party.prey_loot_name, party.leader_name, party.market_name, party.prey_name
            ),
        };
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: reflex,
        });
        return;
    }

    // (b) 工匠/商販對攜帶原料之行旅者的反應
    let artisan_npc = awake_npcs.iter().find(|n| {
        n.id.starts_with("wt_vendor_") || n.id.contains("cen_") || n.id.contains("ke_") || n.id.contains("ma_buxue") || n.id.contains("smith") || n.id.contains("carpenter")
    });
    if let Some(artisan) = artisan_npc {
        let caller = get_display_name(artisan);
        let inv_raw = db::get_entity(player_id).ok().flatten().is_some_and(|ch| {
            db::get_inventory(&ch.inventory, ch.vit).items.iter().any(|it| {
                crate::game::object_ecology::classify_item_stage(&it.item_id) == "raw"
            })
        });
        if inv_raw {
            let reflex = format!("「{caller}見你走入，目光掠過你隨身行囊中的百工原料，主動招呼道：『客友請進，隨身若備有山野金石草木原料，可於爐前案頭煉製；若欲激發器物氣脈異變，亦可隨時開爐！』」");
            conn.send_json(&super::super::protocol::NarrateMsg {
                msg_type: "narrate".into(),
                text: reflex,
            });
            return;
        }
    }

    // (c) NPC 差事/義務探詢
    if let Some(ob_npc) = awake_npcs.iter().find(|n| {
        crate::store::get_active_npc_obligation(&n.id).is_some_and(|(k, _)| {
            matches!(k.as_str(), "restock_goods" | "repair_tool" | "treat_injury" | "repay_debt" | "stranded_cargo")
        })
    }) {
        let npc_name = get_display_name(ob_npc);
        let (kind, subject_ref) = crate::store::get_active_npc_obligation(&ob_npc.id).unwrap();
        let desc = if !subject_ref.is_empty() {
            subject_ref
        } else {
            match kind.as_str() {
                "restock_goods" => "店內現貨見底急需提貨補倉".to_string(),
                "repair_tool" => "手頭工具耗損待修".to_string(),
                "treat_injury" => "連日勞作氣息微滯需調理".to_string(),
                "repay_debt" => "貨單票據核銷結算".to_string(),
                _ => "手頭未了的差事".to_string(),
            }
        };
        let line = format!("「{npc_name}正為【{desc}】發愁，見你踏入此處，隨即投來探詢的目光。」");
        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: line,
        });
        return;
    }

    // 1. 優先檢查是否有正在擺攤的攤販業者
    let vendor_npc = npcs.iter().find(|e| {
        e.id.starts_with("wt_vendor_")
            || e.current_activity.contains("攤前")
            || e.current_activity.contains("擺攤")
            || (e.id == "wt_npc_ma_buxue" && e.current_activity.contains("修補攤"))
    });

    if let Some(v) = vendor_npc {
        let caller = get_display_name(v);
        let bark = if v.id == "wt_vendor_roast_beef" || v.display_title.contains("鄭火旺") || v.current_activity.contains("牛肉") {
            format!("{caller}一邊翻動炭火烤網上滋滋冒油的肉串，一邊高聲叫賣：「街角炭烤原牛肉串配焦蜜烤番薯喔——炭火現烤、焦香鮮嫩，來一串暖暖胃！」")
        } else if v.id == "wt_vendor_noodle_soup" || v.display_title.contains("林罔市") || v.current_activity.contains("切仔麵") {
            format!("{caller}掀開大銅鍋蓋，伴著滾燙大骨白煙清脆招呼：「切仔湯麵配滷蛋熱騰騰出鍋喔！大骨高湯現點現煮，來坐吃麵！」")
        } else if v.id == "wt_vendor_pepper_bun" || v.display_title.contains("張阿發") || v.current_activity.contains("胡椒餅") {
            format!("{caller}用扁鐵鏟從鐵桶內壁鏟起金黃焦脆的熱餅，高聲吆喝：「方出爐的酥皮鮮肉胡椒餅！老麵貼爐、皮酥蔥香肉汁滿，小心燙口喔——」")
        } else if v.id == "wt_vendor_herbal_tea" || v.display_title.contains("蘇秋月") || v.current_activity.contains("烏梅") || v.current_activity.contains("仙草") {
            format!("{caller}手搖清脆銅盞叮噹作響，笑著叫賣：「寒水玉冰鎮黑糖仙草凍來囉！慢熬仙草沁涼滑嫩、生津止渴，解膩消暑最對味喔！」")
        } else if v.id == "wt_vendor_tofu_pudding" || v.display_title.contains("陳順興") || v.current_activity.contains("豆花") {
            format!("{caller}手持扁銅杓片出滑嫩豆花，朗聲招呼：「古早味黑糖薑汁熱豆花——井泉鹽滷、慢火黑糖粉圓配薑汁暖胃補氣，來坐喔！」")
        } else if v.id == "wt_vendor_rice_roll" || v.display_title.contains("吳秀琴") || v.current_activity.contains("飯糰") || v.current_activity.contains("炊飯") {
            format!("{caller}掀開大木甑散發出撲鼻荷葉香氣，清脆吆喝：「方蒸好的荷葉糯米炊飯！粒粒糯米透著竹香，吃一顆飽到中午喔！」")
        } else if v.id == "wt_vendor_soy_milk" || v.display_title.contains("周文貴") || v.current_activity.contains("米漿") {
            format!("{caller}攪拌著滾燙濃香的銅鍋，朗聲招呼：「老堰頭現磨溫米漿配現炸酥脆油條喔！石磨現磨香濃滑順，大清早喝一碗神清氣爽！」")
        } else if v.id == "wt_vendor_morning_produce" || v.display_title.contains("李春生") || v.current_activity.contains("蔬果") || v.current_activity.contains("水芹") {
            format!("{caller}往竹簍鮮菜灑了點清泉水，笑著吆喝：「老堰頭水芹菜脆嫩莖方採下山喔！山泉灌溉、梗脆帶露清甜，隨便挑隨便選！」")
        } else if v.id == "wt_vendor_morning_fish" || v.display_title.contains("許海生") || v.current_activity.contains("河鮮") || v.current_activity.contains("河蝦") {
            format!("{caller}指著木盆裡水花直濺的鮮活河蝦，大聲叫賣：「水尾寮淺澤青殼河蝦現撈上岸！隻隻鮮活生猛、殼薄肉甜，買回去煮湯下酒最正點！」")
        } else if v.id == "wt_npc_ma_buxue" || v.display_title.contains("馬補靴") || v.current_activity.contains("修補攤") {
            format!("{caller}敲了敲硬木楦頭上的厚牛皮掌，朗聲招呼：「釘鞋底、換皮掌、修補韌竹骨桐油晴雨摺疊傘——現修現拿，耐走防雨喔！」")
        } else {
            format!("{caller}招呼著過往行人：「走過路過來瞧瞧，新出爐的手藝，新鮮道地喔！」")
        };

        let other_customer = npcs.iter().find(|e| {
            e.id != v.id
                && !e.id.starts_with("wt_vendor_")
                && !e.current_activity.contains("安歇")
                && !e.current_activity.contains("熟睡")
                && !e.current_activity.contains("熄燈")
                && !e.current_activity.contains("就寢")
        });
        let line = if let Some(cust) = other_customer {
            let is_cust_named = db::get_acquaintance(player_id, &cust.id)
                .ok()
                .flatten()
                .map(|a| a.named)
                .unwrap_or(false);
            let cust_name = if is_cust_named {
                crate::store::resolve_npc_person_name(&cust.id, &cust.display_title, None)
            } else {
                let tag = crate::store::npc_brief_appearance_tag(&cust.id, &cust.display_title);
                format!("那名{}的客人", tag)
            };
            let v_name = &caller;
            let c_name = &cust_name;
            let now_ts = crate::game::now_unix();
            match crate::server::action::trade::settle_live_vendor_npc_purchase(
                &v.id,
                &cust.id,
                new_room,
                now_ts,
            ) {
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::Purchased {
                    item_name,
                    price_mg,
                    remaining_stock,
                    ..
                }) => {
                    let pattern_idx = (now_ts as usize + v.id.len()) % 5;
                    match pattern_idx {
                        0 => format!("{bark} 一旁的{c_name}行色匆匆趕路經過，停下腳步喊道：「老闆，手頭趕工時間緊，幫我俐落外帶一份【{item_name}】！」說著遞上 {price_mg} 鎂。 {v_name}麻利包好遞過，笑道：「好勒收您 {price_mg} 鎂！路上慢走趁熱吃，幹工順遂啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
                        1 => format!("{bark} 一旁的{c_name}深吸一口氣、聞香靠攏過來笑道：「遠遠就聞到香味，實在忍不住嘴饞！老闆，來一份【{item_name}】嚐嚐！」隨即掏出 {price_mg} 鎂遞過去。 {v_name}笑臉盈盈盛妥遞上：「識貨！現起鍋的滋味最正點，收您 {price_mg} 鎂，慢用啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
                        2 => format!("{bark} 一旁的{c_name}駐足端詳片刻，溫言道：「老闆，家裡長輩最念叨你這手藝，幫我包一份【{item_name}】帶回去給老人家嚐鮮！」說罷取出 {price_mg} 鎂付帳。 {v_name}格外細心地用厚紙包妥，樂呵呵道：「孝順！給長輩帶的特別挑了份量足的，收您 {price_mg} 鎂，替我向老人家問好！」（攤上現貨尚餘 {remaining_stock} 份）"),
                        3 => format!("{bark} 一旁的{c_name}拍了拍腰間的工錢袋，朗聲笑道：「辛苦奔波半天，方結了工錢，非得來一份【{item_name}】犒勞犒勞自己不可！」利落地數出 {price_mg} 鎂遞上。 {v_name}連忙接過笑道：「踏實幹活就該吃點好的！收您 {price_mg} 鎂，吃飽喝足渾身都是力氣！」（攤上現貨尚餘 {remaining_stock} 份）"),
                        _ => format!("{bark} 一旁的{c_name}迎面熟絡地點頭招呼：「頭家，老樣子一份【{item_name}】，口味你知道的！」隨手將 {price_mg} 鎂整齊排在攤前。 {v_name}會心一笑，手腳極快地備齊遞上：「熟客就是懂吃！早幫您留心著呢，收您 {price_mg} 鎂，回頭再來坐啊！」（攤上現貨尚餘 {remaining_stock} 份）"),
                    }
                }
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::OutOfStock {
                    item_name,
                }) => {
                    format!("一旁的{c_name}駐足問道：「老闆，想買一份【{item_name}】！」 {v_name}看了一眼見底的鍋具，抱歉笑道：「真不巧，最後一份【{item_name}】賣完啦，我得趕緊去補料，勞駕稍候再來！」")
                }
                Some(crate::server::action::trade::VendorNpcPurchaseOutcome::InsufficientFunds {
                    item_name,
                    price_mg,
                    ..
                }) => {
                    format!("{bark} 一旁的{c_name}聞香駐足，摸了摸錢袋嘆道：「【{item_name}】一份要 {price_mg} 鎂，身上零錢不夠，等領了工錢再來捧場！」")
                }
                None => {
                    let idle_idx = (now_ts as usize + v.id.len()) % 3;
                    match idle_idx {
                        0 => bark,
                        1 => format!("{bark} 一旁的{c_name}在攤前停下打量，好奇問道：「老闆，今天這料看著挺新鮮，火候正好呢！」 {v_name}笑著抬頭應道：「那是自然，天天現備的好料，您先瞧瞧，待會兒想吃隨時吩咐！」"),
                        _ => format!("{bark} 一旁的{c_name}駐足探頭望著攤車，客氣問道：「頭家，今天生意挺熱絡啊？」 {v_name}擦了擦汗爽朗笑道：「承蒙街坊照顧！您先慢慢看，有合意的儘管開口！」"),
                    }
                }
            }
        } else {
            bark
        };

        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: line,
        });
        return;
    }

    // 2. 若同格無攤販、但有 >= 2 名清醒 NPC 在場

    if awake_npcs.len() >= 2 {
        let a = &awake_npcs[0];
        let b = &awake_npcs[1];

        let resolve_person = |npc: &crate::entity::Character| -> String {
            let is_named = db::get_acquaintance(player_id, &npc.id)
                .ok()
                .flatten()
                .map(|a| a.named)
                .unwrap_or(false);
            if is_named {
                crate::store::resolve_npc_person_name(&npc.id, &npc.display_title, None)
            } else {
                let tag = crate::store::npc_brief_appearance_tag(&npc.id, &npc.display_title);
                format!("那名{}的街坊", tag)
            }
        };

        let name_a = resolve_person(a);
        let name_b = resolve_person(b);
        let room_resolved = crate::db::get_room_name(new_room).unwrap_or_else(|_| new_room.to_string());
        let room = room_resolved.as_str();
        static ENTRY_CHAT_SEQ: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(0);
        let dialogues: &[(&str, &str)] = if room.contains("橋") || room.contains("街") || room.contains("巷") || room.contains("道") {
            &[
                ("「方自三水口過來，看著河道水位退得乾淨，這兩天船運順得很。」", "「可不是嘛，連帶這兩天的乾貨瓜果也新鮮便宜不少。」"),
                ("「前頭夜市又擺起炭烤跟切仔麵了，那香味隔了半條街都聞得到！」", "「待會兒辦完手頭這趟差事，正想過去叫碗熱湯暖暖身子呢。」"),
                ("「聽說西門老匠那邊新熬了一批防雨桐油，拿去抹靴子正耐走。」", "「下回得把家裡那把竹骨傘也順便送去給他過過油。」"),
            ]
        } else if room.contains("食堂") || room.contains("茶屋") || room.contains("酒") {
            &[
                ("「這家大骨高湯熬得真醇厚，油蔥酥撒下去那叫一個提味！」", "「再切一份黑白切嘴邊肉，配著這碗熱麵才算過癮。」"),
                ("「聽說南街這幾日來了不少外島客商，茶桌都快坐不下了。」", "「大家都是衝著這幾天風平浪靜、趕在開洋前來談生意的。」"),
            ]
        } else if room.contains("商行") || room.contains("帳房") || room.contains("商算") || room.contains("會計") {
            &[
                ("「這筆東灣進口的苧麻粗布帳目核算過兩遍，總數對上了。」", "「妥當！把籤條壓在庫單上，下午就能發往分家巷作坊了。」"),
                ("「這批烏木與精鋼的公道貨價冊子，今天行商們反應如何？」", "「都照著公議堂紅印市價走，四方貨客沒一個有異議的。」"),
            ]
        } else if room.contains("國小") || room.contains("學院") || room.contains("書局") || room.contains("講堂") || room.contains("藏書") {
            &[
                ("「先前先生講的氣象潮汐篇，那段測風速的法子真精妙。」", "「待會兒去借閱室把《半島風物圖志》找出來，核對一下就全通了。」"),
                ("「這批新到的竹綿紙墨色吃得極好，抄寫筆札不化不透。」", "「我也挑了兩刀，打算謄寫先生留下來的天文算曆課卷。」"),
            ]
        } else if room.contains("鐵坊") || room.contains("鍛造") || room.contains("木作") || room.contains("工坊") {
            &[
                ("「這爐耐火土吸熱勻實，這批精鋼農具淬火出來韌性正好！」", "「再敲兩遍冷縮槽裡的卡榫，保證耕石地三年都不捲刃。」"),
                ("「大水車排鋸切出來的雲杉原木平整得很，刨起來省了大半工夫。」", "「留著給西門車馬行做新大車轅木，結實又吃得住重載。」"),
            ]
        } else if room.contains("浴場") || room.contains("泉") {
            &[
                ("「這寒水玉溫石槽引的活泉水，泡上一刻鐘全身骨頭都鬆快了。」", "「勞作一整天，就盼著這一瓢溫熱泉水把寒氣都洗透。」"),
                ("「裡頭松炭烘衣櫥真方便，洗完澡衣裳早已暖烘烘的了。」", "「可不是嘛，出門吹夜風也不怕著涼！」"),
            ]
        } else if room.contains("診所") || room.contains("藥房") || room.contains("烘藥") {
            &[
                ("「這批百草方焙過，止血水苔的藥性收斂得恰到好處。」", "「分包切片整齊些，巡坊隊跟出海船夫常備著救急呢。」"),
                ("「換季時節夜裡露水重，這幾天來抓避瘴清心草藥的人不少。」", "「慢火溫湯小銅爐一直沒熄火，隨時備著熱藥湯給街坊暖胃。」"),
            ]
        } else {
            &[
                ("「今天手頭這趟活計忙完，可得好好去南街市集逛逛。」", "「聽說今晚各家攤子都熱鬧，走走散步正好吹吹夜風。」"),
                ("「街坊近來大家都在聊那趟新開的貨運班次呢。」", "「路修得平整，往來半嶺跟三水口確實比早先省了一半腳程。」"),
            ]
        };

        let pick_idx = ENTRY_CHAT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % dialogues.len();
        let (dlg_a, dlg_b) = dialogues[pick_idx];
        let line = match pick_idx % 6 {
            0 => format!("{name_a}停下手頭動作，側身對{name_b}言道：{dlg_a}\n{name_b}微微點頭，隨口應道：{dlg_b}"),
            1 => format!("{name_a}整理著手邊物事，向身側的{name_b}開口：{dlg_a}\n{name_b}抬眼笑應：{dlg_b}"),
            2 => format!("{name_a}望向四下街景，同{name_b}搭話：{dlg_a}\n{name_b}順著話頭接道：{dlg_b}"),
            3 => format!("{name_a}朝{name_b}低聲說道：{dlg_a}\n{name_b}神情專注，答道：{dlg_b}"),
            4 => format!("{name_a}手上活計未停，對{name_b}言道：{dlg_a}\n{name_b}語調平穩地回道：{dlg_b}"),
            _ => format!("{name_a}向{name_b}問起：{dlg_a}\n{name_b}略一思索，溫聲回道：{dlg_b}"),
        };

        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: line,
        });
        return;
    }

    // 3. 單一 NPC 舊有差事／打量 fallback
    if let Some(npc) = awake_npcs.first().or_else(|| npcs.first()) {
        let npc_name = get_display_name(npc);
        let active_ob = crate::store::get_active_npc_obligation(&npc.id);
        let personality = crate::db::get_personality_for_entity(&npc.id)
            .or_else(|| npc.soul_seed.map(crate::db::expand_soul_seed_to_personality))
            .unwrap_or(crate::db::Personality {
                boldness: 0.5,
                sensitivity: 0.5,
                orderliness: 0.5,
            });

        let line = if let Some((kind, subject_ref)) = active_ob.filter(|(k, _)| {
            matches!(
                k.as_str(),
                "restock_goods" | "repair_tool" | "treat_injury" | "repay_debt" | "stranded_cargo"
            )
        }) {
            let desc = if !subject_ref.is_empty() {
                subject_ref
            } else {
                match kind.as_str() {
                    "restock_goods" => "店內現貨見底急需提貨補倉".to_string(),
                    "repair_tool" => "手頭工具耗損待修".to_string(),
                    "treat_injury" => "連日勞作氣息微滯需調理".to_string(),
                    "repay_debt" => "貨單票據核銷結算".to_string(),
                    _ => "手頭未了的差事".to_string(),
                }
            };
            format!("「{npc_name}正為【{desc}】發愁，見你踏入此處，隨即投來探詢的目光。」")
        } else if personality.boldness >= 0.65 {
            format!("「{npc_name}聽見腳步聲，抬眼向你投來友善的招呼目光。」")
        } else {
            format!("「{npc_name}抬眼見你走近，微笑著朝你點了點頭。」")
        };

        conn.send_json(&super::super::protocol::NarrateMsg {
            msg_type: "narrate".into(),
            text: line,
        });
    }
}

    fn can_step_in_nine_grid(
        g: &crate::grid::SquareGrid,
        from: SquareCoord,
        to: SquareCoord,
    ) -> bool {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        if dx.abs() + dy.abs() != 1 {
            return false;
        }
        g.can_walk(from, to)
    }
/// 依人物的正式 place_id 重推玩家視野。
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
    use super::super::protocol::{GridCellView, GridViewMsg, NarrateMsg};
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
    let mut cells: Vec<GridCellView> = Vec::new();
    let mut seen_coords: HashSet<(i32, i32)> = HashSet::new();

    // 1. 周圍 radius = 9 格子
    let cells_raw = game::get_grid_cells_around(x, y, 9);
    for (cx, cy, kind, terrain, category, name, explored, raw_walkable) in cells_raw {
        seen_coords.insert((cx, cy));
        let symbol = grid_snap
            .as_ref()
            .and_then(|g| g.get(SquareCoord::new(cx, cy)))
            .map(|cell| cell.terrain.display_char().to_string())
            .unwrap_or_else(|| "⋯".to_string());

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
                cells.push(GridCellView {
                    x: wx,
                    y: wy,
                    kind: cell.terrain.kind_str().to_string(),
                    terrain: crate::grid::terrain_name_zh(cell.terrain).to_string(),
                    category: cell.terrain.category().to_string(),
                    name: cell.name.clone(),
                    explored: cell.explored,
                    walkable: cell.terrain.walkable(),
                    symbol: cell.terrain.display_char().to_string(),
                    npc_count: cell_npc_count,
                });
            }
        }
    }
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
    // 先把 exits_v5 與 grid_exits 所需的相鄰資訊組裝好
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
    let is_building = current_cell
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
                            let name = if nc.name.trim().is_empty() {
                                crate::grid::terrain_name_zh(nc.terrain).to_string()
                            } else {
                                nc.name.clone()
                            };
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
        || current_terrain_name.contains("客棧")
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
        base.room_name.clone()
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
        let in_exit_name = if is_lodging {
            Some(format!("投宿{}客房", current_terrain_name))
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
    let current_place_json = serde_json::json!({
        "id": crate::grid::grid_room_id_from_coord(x, y),
        "name": if in_private_room { &final_room_name } else { &current_terrain_name },
        "locator": format!("白塔半島 · {}", current_cell.as_ref().map(|c| c.zone.as_str()).unwrap_or("荒野")),
        "full_name": if in_private_room { &final_room_name } else { &current_terrain_name },
        "kind": current_terrain_category,
        "scale": "grid_cell"
    });

    let indoor_objects = vec![
        super::super::protocol::ViewObject {
            id: "wt_reg_wenyao_home".to_string(),
            name: "松香木大床與羽絨暖被".to_string(),
            object_type: "bed".to_string(),
            actions: vec!["Look".to_string(), "Rest".to_string(), "Sleep".to_string(), "Use".to_string()],
        },
        super::super::protocol::ViewObject {
            id: "wt_inn_bath_Preview".to_string(),
            name: "杉木沐浴桶與溫泉水".to_string(),
            object_type: "bath".to_string(),
            actions: vec!["Look".to_string(), "Wash".to_string(), "Use".to_string()],
        },
        super::super::protocol::ViewObject {
            id: "wt_home_shell_lamp".to_string(),
            name: "夜光貝母台燈".to_string(),
            object_type: "lamp".to_string(),
            actions: vec!["Look".to_string(), "Operate".to_string(), "Use".to_string()],
        },
    ];

    let final_objects = if in_private_room {
        indoor_objects
    } else {
        let mut objs = base.objects.clone();
        if objs.is_empty() {
            if let Some(cell) = &current_cell {
                for obj in &cell.objects {
                    let actions = if obj.quantity > 0 {
                        vec!["Look".to_string(), "Gather".to_string()]
                    } else {
                        vec!["Look".to_string()]
                    };
                    objs.push(super::super::protocol::ViewObject {
                        id: obj.id.clone(),
                        name: obj.name.clone(),
                        object_type: "grid_object".to_string(),
                        actions,
                    });
                }
            }
        }
        objs
    };

    let final_exits = if in_private_room {
        grid_exits.clone()
    } else {
        base.exits
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
        // 白塔半島正典具名人物直接視為已知的在地當家／掌事（state = "named"）
        let obsolete_dup_ids = [
            "wt_npc_chen_wanfu",
            "wt_npc_bai_quansheng",
            "wt_npc_shen_qinghe",
            "wt_npc_ke_chengqi",
            "wt_npc_su_jinxin",
        ];
        let canonical_count = npc_entities
            .iter()
            .filter(|e| {
                !obsolete_dup_ids.contains(&e.id.as_str())
                    && (e.id.starts_with("wt_npc_")
                        || e.id.starts_with("wt_vendor_")
                        || e.display_name.contains('｜')
                        || e.display_name.contains('|'))
            })
            .count();
        let unnamed_cap = 10usize.saturating_sub(canonical_count);
        let mut unnamed_kept = 0usize;
        let mut seen_canon_names = std::collections::HashSet::new();
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
                    return seen_canon_names.insert(dedup_key);
                }
                if unnamed_kept >= unnamed_cap {
                    return false;
                }
                unnamed_kept += 1;
                true
            })
            .collect();

        let cards: Vec<NpcCard<'_>> = filtered_npc_entities
            .into_iter()
            .enumerate()
            .map(|(npc_idx, e)| {
                let is_canonical_leader = e.id.starts_with("wt_npc_")
                    || e.id.starts_with("wt_vendor_")
                    || e.display_name.contains('｜')
                    || e.display_name.contains('|');
                let acq = acq_map.get(&e.id);
                let named = acq.map(|a| a.named).unwrap_or(false);

                let (label, card_state) = if e.id.starts_with("ambient_") || e.id.starts_with("shanhai_") {
                    let clean_name = e.display_name.rsplit(['｜', '|']).next().unwrap_or(&e.display_name).trim().to_string();
                    (clean_name, "named")
                } else if named {
                    let clean_name =
                        crate::store::resolve_npc_person_name(&e.id, &e.display_name, None);
                    (clean_name, "named")
                } else {
                    // 尚未認識該 NPC：不論 observed 真假、不論是否為正典人物，一律顯示為陌生人
                    ("陌生人".to_string(), "stranger")
                };

                // 為正典人物賦予鮮活實活描述（若當前活動過於通用或空缺）
                let hour = crate::store::current_game_hour();
                let is_night = hour >= 22 || hour < 5;
                let canon_activity: Option<&'static str> = if is_night {
                    let r = base.room_name.as_str();
                    if r.contains("居") || r.contains("舍") || r.contains("宅") || r.contains("宿所") || r.contains("院") || r.contains("齋") {
                        Some("正在內室床鋪上熄燈安歇")
                    } else {
                        Some("正提著防風燈在廊下巡看門戶")
                    }
                } else if is_canonical_leader || named {
                    let n = e.display_name.as_str();
                    if n.contains("陳萬福") {
                        Some("正守在紅鐵大釜前慢火熬煮原牛肉湯")
                    } else if n.contains("陸秉衡") {
                        Some("正於樟木掌事台前核驗跨鎮通兌匯票")
                    } else if n.contains("陸承禮") {
                        Some("正於案前核對《半島全戶總譜》與紅印屋契")
                    } else if n.contains("白泉生") {
                        Some("正往寒水玉湯池投放祛寒草木湯包")
                    } else if n.contains("溫晴") {
                        Some("正於青石藥案前調配青鹽薄荷潔牙粉與水苔敷藥")
                    } else if n.contains("岑守岳") {
                        Some("正指導學徒查看高溫紅鐵爐的火色")
                    } else if n.contains("柯成器") {
                        Some("正用省力導能刨刀推平松香木板")
                    } else if n.contains("蘇錦心") {
                        Some("正為遠行採購商量裁黑沼防雨樹膠風衣")
                    } else if n.contains("韓鐵山") {
                        Some("正調度雙輪駝獸大車裝卸四方貨件")
                    } else if n.contains("陸知微") {
                        Some("正校勘新印出的《白塔風物誌》與《半島旬報》")
                    } else if n.contains("岑拓") {
                        Some("正用游標卡尺測量導能火磚嵌合縫隙")
                    } else if n.contains("秦明誠") {
                        Some("正操作隨身計數板核算大宗行單")
                    } else if n.contains("岑雲野") {
                        Some("正手持包革木刀指點學徒格擋防身步法")
                    } else if n.contains("陸長貴") {
                        Some("正手持木牌巡看街坊水渠與公議告示")
                    } else if n.contains("陸敬軒") {
                        Some("正敲響響板帶領學生朗讀基礎教材")
                    } else if n.contains("溫懷瑾") {
                        Some("正於講堂案前批閱學生呈交的脈絡札記")
                    } else if n.contains("沈清河") {
                        Some("正於臨水茶席沏起頭道山風草茶並擺設棋局")
                    } else if n.contains("林時中") {
                        Some("正校對旋轉日規與報時青銅巨鐘並核驗快騎郵袋")
                    } else if n.contains("趙觀星") {
                        Some("正撥轉三百六十星軌銅儀測算星度交角與節氣日影")
                    } else if n.contains("周一刀") {
                        Some("正推磨精鋼摺疊剃刀並擰乾熱毛巾替客修面理容")
                    } else if n.contains("許阿甘") {
                        Some("正擦拭廣口玻璃柑仔糖罐並清點紅磷火柴與蚊香")
                    } else if n.contains("馬補靴") {
                        Some("正在硬木楦頭上釘補防浸皮靴並為竹骨傘刷上熟桐油")
                    } else {
                        None
                    }
                } else if base.room_name.starts_with("文曜") {
                    let r = base.room_name.as_str();
                    let variants: &[&'static str] = if r.contains("樞紐") || r.contains("石橋") {
                        &[
                            "憑著雕花青石欄杆遠眺三水川往來貨船",
                            "在橋頭日晷石柱旁仰看刻度核對時間",
                            "挑著兩擔山海乾貨穩步走過青石拱橋",
                        ]
                    } else if r.contains("國小") || r.contains("學院") || r.contains("蒙學") {
                        &[
                            "端坐講堂木案前用鋼尖沾水筆抄錄課卷",
                            "手捧新印的《白塔風物誌》與同學低聲切磋",
                            "在杉木書架前翻檢四時物候與山海圖譜",
                        ]
                    } else if r.contains("觀星") {
                        &[
                            "圍在三百六十星軌銅儀旁記錄星度刻度",
                            "在評議廳長案前整理本季的氣象與潮汐觀測表",
                        ]
                    } else if r.contains("藏書") || r.contains("印行") || (r.contains("書局") && !r.contains("契書")) {
                        &[
                            "手持竹骨放大鏡仔細校勘新排印的《半島旬報》",
                            "將裁切整齊的講義冊頁穿線裝訂成冊",
                            "在多層杉木書架間輕步搜尋地方風物誌卷宗",
                        ]
                    } else if r.contains("會計") || r.contains("商算") {
                        &[
                            "專注操作計數板演練帳務核算公式",
                            "對照黑板上的出海貨單核算跨鎮運費與匯差",
                        ]
                    } else if r.contains("剪剃") {
                        &[
                            "半躺在紫銅溫水管木椅上敷著熱毛巾閉目養神",
                            "在亮銀玻璃鏡前用香柏木梳整理先前修剪好的髮鬢",
                        ]
                    } else if r.contains("診所") {
                        &[
                            "在藥櫃前按方稱量烘乾的光葉藥草與止血水苔",
                            "用青石藥碾將薄荷與青鹽細細研成潔牙粉",
                        ]
                    } else if r.contains("總商行") {
                        &[
                            "在厚樟木櫃台後核對防偽鋼印與通兌匯票存根",
                            "將收進的實體鎂幣按面額分格歸入樟木錢箱",
                            "捧著四方貨單向掌事稟報今日大宗物產行情",
                        ]
                    } else if r.contains("鐘樓") || r.contains("信使") {
                        &[
                            "將加蓋火漆印戳的書信分揀裝入雙扣防水郵袋",
                            "在信使站廊下替疾行草獸快騎檢查鞍轡與水囊",
                        ]
                    } else if r.contains("食堂") {
                        &[
                            "端著剛出鍋的原牛骨湯與蔥香米烙餅穿梭送餐",
                            "在紅鐵大釜旁添柴看火並撇去高湯浮沫",
                            "圍在木桌旁大口品嚐熱豆花與切仔湯麵",
                        ]
                    } else if r.contains("浴場") {
                        &[
                            "抱著烘得乾爽蓬鬆的蜂格棉面巾替浴客更換",
                            "在寒水玉湯池邊泡著溫水洗去一身奔波疲乏",
                        ]
                    } else if r.contains("公議") || r.contains("契據") || r.contains("契書") {
                        &[
                            "在長條厚木公議案桌前謄錄今日新簽的屋契地籍",
                            "站在水土民務告示立木前審視各坊修繕委託單",
                        ]
                    } else if r.contains("鍛造") {
                        &[
                            "拉動風箱助燃紅土耐火高爐並觀察赤鐵火色",
                            "舉起熟鐵鍛錘在鐵砧上反覆鍛打農具與船釘",
                            "將燒紅的精鋼刃口浸入冷卻水槽激起一團白霧",
                        ]
                    } else if r.contains("巧木") || r.contains("傢俱") {
                        &[
                            "順著松香木紋理推刨木板，刨花如雪片般捲落",
                            "拿著木槌與竹釘試拼寒水玉保鮮石櫃的榫卯框體",
                        ]
                    } else if r.contains("雲錦") || r.contains("服飾") {
                        &[
                            "在木機旁飛梭走線織造細密挺括的防風棉麻布",
                            "拿著軟尺與粉線為客人量裁黑沼防雨樹膠風衣",
                        ]
                    } else if r.contains("旅社") || r.contains("茶屋") {
                        &[
                            "提著紅銅長嘴壺為對弈茶客添上熱騰騰的山風草茶",
                            "坐在臨水窗邊一邊品嚐鍋煮鮮奶茶一邊翻看旬報",
                        ]
                    } else if r.contains("習武") || r.contains("試招") {
                        &[
                            "手握包革練習木刀在青石院中演練卸力步法",
                            "站在導能測紋石板前調勻呼吸測試掌力勁道",
                        ]
                    } else if r.contains("柑仔店") || r.contains("雜貨") {
                        &[
                            "在貨架前挑選紅磷防風火柴與艾草蚊香盤",
                            "拈起一塊古早黑糖麥芽糖笑咪咪地與鄰里寒暄",
                        ]
                    } else if r.contains("聯運") || r.contains("貨運") || r.contains("西門") {
                        &[
                            "推著獨輪木車將捆紮結實的貨箱推上生鐵地秤過磅",
                            "在青石水槽旁替遠道歸來的雙輪駝獸刷洗毛皮添草料",
                            "對照聯運班次木板核對發往大圳頭與風舶角的車次",
                        ]
                    } else if r.contains("修傘") || r.contains("補靴") {
                        &[
                            "將淋溼的桐油竹骨傘撐開晾在木架上檢視傘骨",
                            "拿著麻線錐子替採購商縫補磨穿的牛皮靴底",
                        ]
                    } else if r.contains("講堂街") || r.contains("學院巷") || r.contains("書院巷") || r.contains("曬書") || r.contains("竹苑") {
                        &[
                            "手捧裝訂講義沿著石造拱廊的石道慢步交談",
                            "駐足在《半島旬報》閱報木欄前細讀最新一期刊物",
                        ]
                    } else if r.contains("分家巷") || r.contains("百工") || r.contains("晾木") {
                        &[
                            "在工坊門前青石水槽邊用天然油磨石打磨隨身工具",
                            "扛著剛裁好的北山松香木板穿過百工石板巷",
                        ]
                    } else if r.contains("烘藥") || r.contains("百草") {
                        &[
                            "在紅泥焙藥爐旁照看火候，藥香濃郁沁鼻",
                            "手持切藥鍘刀仔細修整晾乾的藥草根莖",
                        ]
                    } else if r.contains("造紙") || r.contains("紙廬") || r.contains("精廬") {
                        &[
                            "在石槽旁蕩料抄紙，水花均勻泛起",
                            "拿著毛刷將濕紙平整貼於熱牆焙乾",
                        ]
                    } else if r.contains("書齋") || r.contains("墨香") || r.contains("居舍") || r.contains("民宅") {
                        &[
                            "在沉水烏木書案前研墨習字，神情安恬專注",
                            "推開雕花木窗迎入微風，靜心翻閱案上線裝冊頁",
                        ]
                    } else if r.contains("河灣") || r.contains("北渠") || r.contains("水脈") || r.contains("東灣") || r.contains("竹筏") {
                        &[
                            "在青石埠階旁繫穩竹筏繩纜，掬水洗面",
                            "整理汲水木桶與竹編魚簍，聽水聲潺潺流淌",
                        ]
                    } else {
                        &[
                            "提著竹編提籃在三水口商號間採辦日用雜貨",
                            "沿著平整的青石板街與相熟的街坊點頭寒暄",
                        ]
                    };
                    Some(variants[npc_idx % variants.len()])
                } else {
                    None
                };

                let current_act = if (e.id.starts_with("wt_vendor_") || !is_canonical_leader)
                    && !e.current_activity.trim().is_empty()
                    && !e.current_activity.contains("安歇")
                {
                    e.current_activity.as_str()
                } else if is_night {
                    canon_activity.unwrap_or("正在內室床鋪上熄燈安歇")
                } else if !is_canonical_leader && !e.current_activity.trim().is_empty() {
                    e.current_activity.as_str()
                } else {
                    canon_activity.unwrap_or(e.current_activity.as_str())
                };
                NpcCard {
                    id: &e.id,
                    label,
                    card_state,
                    current_activity: current_act,
                    interaction_priority: is_canonical_leader || available_market_operators.contains(&e.id),
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
                "card_state": c.card_state,
                "current_activity": c.current_activity
            }));
        }
        let total_present = cards.len();
        let mut present_npc_summaries: Vec<(String, String)> = visible_npcs
            .iter()
            .take(2)
            .enumerate()
            .map(|(idx, c)| {
                let subj = narrative_person_subject(c, idx);
                (subj, c.current_activity.to_string())
            })
            .collect();
        let npc_name_map: std::collections::HashMap<String, String> = visible_npcs
            .iter()
            .map(|c| (c.id.to_string(), c.display_name.clone()))
            .collect();
        let entities: Vec<crate::server::protocol::ViewEntity> = entities
            .into_iter()
            .map(|mut e| {
                if (e.kind == "npc" || e.kind == "Npc") && let Some(norm_name) = npc_name_map.get(e.id.as_str()) {
                    e.display_name = norm_name.clone();
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
                        present_npc_summaries.push((subj, act.to_string()));
                        if present_npc_summaries.len() == 2 {
                            break;
                        }
                    }
                }
            }
        }
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
                || current_terrain_name.contains("客棧")
                || current_terrain_name.contains("茶屋")
                || current_terrain_name.contains("公寓")
                || current_terrain_name.contains("驛");
            if is_lodging {
                "你反手扣上房門的黃銅插銷。這是樓上一間安靜整潔的單人客房，窗畔擺著旋蓋夜光貝母調光台燈與烏木書案，床鋪上鋪著厚實的蓬鬆羽絨暖被與山風草安神枕，將樓下街巷的喧囂人聲隔絕在外。".to_string()
            } else {
                "你穿過門廊走入宅院內室。屋內鋪著擦拭光潔的熟桐油木地板，門口整齊擺著軟底布拖鞋。室內設有松香木大床與山風草安神枕，窗畔几案上亮著旋蓋夜光貝母台燈，一側自來活水黃銅水栓與溫石熱水槽水氣氤氳，恬靜怡人。".to_string()
            }
        } else {
            player_layer_scene_desc(
                &current_desc,
                &current_terrain_name,
                game_minute_of_day,
                scene_climate_ref,
                unnamed_total,
                &recent_scene_events,
            )
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

        let room_enter_msg = serde_json::json!({
            "type": "room_enter",
            "room_id": current_room_id,
            "place": current_place_json,
            "exits": exits_v5,
            "objects": final_objects,
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
        let is_new_coord = {
            let mut last_coord = PLAYER_LAST_ANNOUNCED_COORD.write();
            if last_coord.get(player_id).copied() != Some((x, y)) {
                last_coord.insert(player_id.to_string(), (x, y));
                true
            } else {
                false
            }
        };
        if is_new_coord {
            if !present_npc_summaries.is_empty() {
                let format_npc_act = |raw: &str| -> String {
                    let act = raw.trim().trim_end_matches('。');
                    if act.is_empty() {
                        "正駐足於此".to_string()
                    } else if act.starts_with('正')
                        || act.starts_with('在')
                        || act.starts_with('剛')
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
                    format!("你步入【{}】，夜深人靜，屋內諸人皆已熄燈安歇。", grid_msg.room_name)
                } else if total_present >= 3 && present_npc_summaries.len() >= 2 {
                    let (npc1, act1_raw) = &present_npc_summaries[0];
                    let (npc2, act2_raw) = &present_npc_summaries[1];
                    let act1 = format_npc_act(act1_raw);
                    let act2 = format_npc_act(act2_raw);
                    if is_night_time {
                        format!(
                            "你步入【{}】，夜色微涼，見{}{}；旁側{}{}。",
                            grid_msg.room_name, npc1, act1, npc2, act2
                        )
                    } else {
                        format!(
                            "你步入【{}】，見{}{}；旁側{}{}。",
                            grid_msg.room_name, npc1, act1, npc2, act2
                        )
                    }
                } else if total_present == 2 && present_npc_summaries.len() >= 2 {
                    let (npc1, act1_raw) = &present_npc_summaries[0];
                    let (npc2, act2_raw) = &present_npc_summaries[1];
                    let act1 = format_npc_act(act1_raw);
                    let act2 = format_npc_act(act2_raw);
                    format!(
                        "你步入【{}】，見{}{}；一旁的{}{}。",
                        grid_msg.room_name, npc1, act1, npc2, act2
                    )
                } else {
                    let (npc1, act1_raw) = &present_npc_summaries[0];
                    let act1 = format_npc_act(act1_raw);
                    format!("你步入【{}】，見{}{}。", grid_msg.room_name, npc1, act1)
                };
                conn.send_json(&super::super::protocol::NarrateMsg {
                    msg_type: "narrate".into(),
                    text: line,
                });
            }
        }
    }
}

fn player_layer_scene_desc(
    desc: &str,
    terrain_name: &str,
    game_minute_of_day: u32,
    climate: Option<(&str, i64)>,
    unnamed_total: usize,
    recent_events: &[String],
) -> String {
    let mut text = desc.trim().to_string();
    let terrain = terrain_name.trim();

    let gh = (game_minute_of_day / 60) % 24;
    let is_night = gh <= 4 || gh >= 23;
    let is_empty = unnamed_total == 0;
    if is_night || is_empty {
        let replacement = if is_night {
            "，夜色深沉，街面靜謐無人"
        } else {
            "，四周一時安靜清朗"
        };
        for needle in [
            "，行人踏石而過，趣味盎然",
            ", 行人踏石而過，趣味盎然",
            "、行人踏石而過，趣味盎然",
            "。行人踏石而過，趣味盎然",
            "行人踏石而過，趣味盎然",
            "，人來人往",
            ", 人來人往",
            "人來人往",
            "，買賣聲此起彼落",
            ", 買賣聲此起彼落",
            "買賣聲此起彼落",
            "，來往居民互致問候",
            ", 來往居民互致問候",
            "來往居民互致問候",
        ] {
            if text.contains(needle) {
                text = text.replace(needle, replacement);
                break;
            }
        }
    }

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
        if terrain.contains("半嶺") || terrain.contains("北岸") || terrain.contains("等候") {
            return north_bank_waiting_scene_desc(
                game_minute_of_day,
                climate,
                unnamed_total,
                recent_events,
            );
        }
        let is_night = gh <= 4 || gh >= 23;
        let is_empty = unnamed_total == 0;
        if terrain.is_empty() {
            if is_night {
                return "一處可落腳的地方展在近前，夜色深沉，四周靜謐無人，幾條去處從眼前分開。".to_string();
            } else if is_empty {
                return "一處可落腳的地方展在近前，四周一時安靜清朗，幾條去處從眼前分開。".to_string();
            } else {
                return "一處可落腳的地方展在近前，周圍有人聲與腳步聲，幾條去處從眼前分開。".to_string();
            }
        }
        if is_night {
            return format!("{}在近前展開，夜色深沉，街面靜謐無人，幾條去處從眼前分開。", terrain);
        } else if is_empty {
            return format!("{}在近前展開，四周一時安靜清朗，幾條去處從眼前分開。", terrain);
        } else {
            return format!(
                "{}在近前展開，周圍有人聲與腳步聲，幾條去處從眼前分開。",
                terrain
            );
        }
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
    let skeleton = "半嶺水岸渡口沿著中河鋪開，濕木踏板接著渡船；水尾寮渡頭市集和歇腳處貼在岸邊。";
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
                "風從河面推過來，市集棚布低低鼓了一下又垂回去。",
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
                    "河面被日色照開，踏板和市集之間的人影變得清楚。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "暮色壓低水面，河霧又從踏板底下漫回來。",
                    "天色往暗處沉，市集的價牌先失去顏色，河面還反著一點光。",
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
                "市集邊人影密了些，包袱和貨擔把通向踏板的空隙擠窄。",
            ],
            seed,
        )
    } else if unnamed_total >= 5 {
        pick_scene_variant(
            &[
                "幾撥等船的人散在茶桶和踏板之間，不時有人朝河面張望。",
                "有人守著包袱歇腳，有人往市集那頭問價，視線仍留著渡船方向。",
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
                "岸邊腳印交疊得密，幾道貨擔拖痕從市集斜斜壓向踏板前。"
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
        Some("泥地上留著剛拖過貨擔的痕，從市集那頭一路壓向踏板前。")
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
        Some("市集和踏板之間還留著剛壓下去的低語，像水聲一過就會散。")
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
    if room_name.contains("半嶺水岸渡口")
        || room_name.contains("水岸渡口")
        || room_name.contains("北岸")
        || room_name.contains("等候")
    {
        Some(north_bank_waiting_scene_desc(
            game_minute_of_day,
            climate,
            unnamed_total,
            recent_events,
        ))
    } else if room_name.contains("水尾寮渡頭市集")
        || room_name.contains("渡頭市集")
        || room_name.contains("市集")
    {
        Some("水尾寮渡頭市集沿著木架排開，價牌被河風推得輕晃；小件貨、乾糧與船票消息混在一起，被等船的人低聲議價。".to_string())
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
    current_activity: &'a str,
    interaction_priority: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct VisibleNpcCard<'a> {
    id: &'a str,
    display_name: String,
    card_state: &'static str,
    current_activity: &'a str,
}
fn narrative_person_subject(card: &VisibleNpcCard<'_>, index: usize) -> String {
    if card.card_state == "named" && !card.display_name.starts_with("陌生人") {
        return card.display_name.clone();
    }

    let is_chen_wanfu = card.id == "wt_npc_chen_wanfu" || card.id.contains("chen_wanfu") || card.display_name.contains("陳萬福");
    let is_lu_chengli = card.id == "wt_npc_lu_chengli" || card.id.contains("lu_chengli") || card.display_name.contains("陸承禮");

    let sensory = if is_chen_wanfu {
        "潔白雙排扣主廚服與厚布圍裙".to_string()
    } else if is_lu_chengli {
        "身穿深藍色公務外套，身姿端正".to_string()
    } else {
        crate::store::npc_sensory_appearance_line(card.id, &card.display_name)
    };

    let first_attire = sensory
        .split(['，', '、', '。'])
        .next()
        .map(|s| s.trim())
        .unwrap_or("");

    let role_pers = crate::store::npc_persistent_role_label(card.id, &card.display_name);
    let role = if is_lu_chengli {
        "主事"
    } else if is_chen_wanfu {
        "主廚"
    } else if card.id == "wt_npc_lu_bingheng" || card.display_name.contains("陸秉衡") {
        "主事"
    } else if card.id == "wt_npc_lu_zhiwei" || card.display_name.contains("陸知微") {
        "主編"
    } else if card.id == "wt_npc_cen_shouyue" || card.display_name.contains("岑守岳") {
        "師傅"
    } else if card.id == "wt_npc_cen_tuo" || card.display_name.contains("岑拓") {
        "師傅"
    } else if card.id == "wt_npc_wen_huaijin" || card.display_name.contains("溫懷瑾") {
        "主事"
    } else if card.id == "wt_npc_wen_qing" || card.display_name.contains("溫晴") {
        "人員"
    } else if card.id == "wt_npc_han_tieshan" || card.display_name.contains("韓鐵山") {
        "主事"
    } else if card.id.starts_with("wt_vendor_") || role_pers.contains("攤") {
        "攤販"
    } else if role_pers.contains("主事") || role_pers.contains("長") || role_pers.contains("總管") {
        "主事"
    } else if role_pers.contains("廚") {
        "主廚"
    } else if role_pers.contains("匠") || role_pers.contains("工") || role_pers.contains("鍛") {
        "師傅"
    } else if role_pers.contains("生") || role_pers.contains("吏") || role_pers.contains("教") || role_pers.contains("學") {
        "人員"
    } else if role_pers.contains("商") || role_pers.contains("客") {
        "商旅"
    } else {
        "住戶"
    };

    let prefix = if index == 0 { "一名" } else { "另一名" };
    if !first_attire.is_empty() {
        let mut cleaned_attire = first_attire.replace("身穿", "身著").replace("穿著", "身著");
        if !cleaned_attire.starts_with("身著")
            && !cleaned_attire.starts_with("披著")
            && !cleaned_attire.starts_with("戴著")
            && !cleaned_attire.starts_with("繫著")
            && !cleaned_attire.starts_with("手持")
            && !cleaned_attire.starts_with("腰")
            && !cleaned_attire.starts_with("頭戴")
            && !cleaned_attire.starts_with("頭裹")
            && !cleaned_attire.starts_with("圍著")
            && !cleaned_attire.starts_with("面容")
            && !cleaned_attire.starts_with("手握")
            && !cleaned_attire.starts_with("肩")
        {
            cleaned_attire = format!("身著{cleaned_attire}");
        }
        let attire_desc = if cleaned_attire.contains('的') {
            cleaned_attire.replacen('的', "", 1)
        } else {
            cleaned_attire
        };
        format!("{prefix}{attire_desc}的{role}")
    } else {
        let brief = crate::store::npc_brief_appearance_tag(card.id, &card.display_name);
        if !brief.is_empty() && brief != "街坊居民" {
            let brief_clean = if brief.contains('的') {
                brief.replacen('的', "", 1)
            } else {
                brief.to_string()
            };
            format!("{prefix}{brief_clean}的{role}")
        } else if index == 0 {
            format!("一名在場{role}")
        } else {
            format!("另一名在場{role}")
        }
    }
}


fn build_player_layer_npc_cards<'a>(cards: &'a [NpcCard<'a>]) -> (Vec<VisibleNpcCard<'a>>, bool) {
    let mut selected: Vec<&NpcCard<'a>> = Vec::new();

    // 保持排序優先順序：先加入 interaction_priority 為 true 的角色，再加入其餘角色。
    for card in cards.iter().filter(|card| card.interaction_priority) {
        selected.push(card);
    }

    for card in cards {
        if card.interaction_priority {
            continue;
        }
        selected.push(card);
    }

    // 編號：同 label 桶內 ≥ 2 人按順序附上數字（1-based）。
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
                *cursor += 1;
                format!("{}{}", card.label, *cursor)
            } else {
                card.label.clone()
            };
            VisibleNpcCard {
                id: card.id,
                display_name,
                card_state: card.card_state,
                current_activity: card.current_activity,
            }
        })
        .collect();

    (visible, false)
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
        "西門坡道街",
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
    if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
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

fn observed_label_for_scene(_raw_name: &str, _room_name: &str) -> String {
    "陌生人".to_string()
}

fn scene_crowd_label(room_name: &str) -> Option<&'static str> {
    if let Some(label) = game::final_map_crowd_label_for_room(room_name) {
        Some(label)
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
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
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some("港倉門人群")
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
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some("外洋棧倉人群")
    } else if room_name.contains("翻譯桌") {
        Some("翻譯人群")
    } else if room_name.contains("匯兌小屋") {
        Some("匯兌人群")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some("避風港人群")
    } else if room_name.contains("港灣貨棚") {
        Some("貨棚人群")
    } else if room_name.contains("漁網曬場") {
        Some("補網人群")
    } else if room_name.contains("冷霧魚市") {
        Some("魚市人群")
    } else if room_name.contains("船戶屋排") {
        Some("船戶屋排人群")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
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
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
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
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") || room_name.contains("市集") {
        Some("市集人群")
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
    } else if room_name.contains("半嶺水岸渡口") || room_name.contains("水岸渡口") || room_name.contains("北岸") || room_name.contains("等候") {
        Some(
            "船繩旁有人把濕繩重新繞上木樁，船身在水霧裡輕輕碰了一下岸。\n市集那頭有人壓低聲音討價，木牌被指節敲得輕響。\n等船人群往踏板口挪了一步，又停在河面傳來的水聲裡。",
        )
    } else if room_name.contains("中河谷地路口") {
        Some("你走到中河谷地路口。渡口人聲從身後貼來，泥路往田埂和市集外分開，腳印在路口交錯。")
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
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        Some(
            "你走到水尾寮水網口。田邊腳印在濕木棧上換成船路痕跡，引船人靠著篙影看水道，霧牌在木柱上輕晃。",
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
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some(
            "你停在風舶角港倉門前。雨布半遮著門口，倉管把河貨、海貨和放行貨單分開，門內木箱悶悶靠上牆。",
        )
    } else if room_name.contains("商船候泊線") {
        Some("你來到商船候泊線。等潮的船影停在外頭，船員把錨繩收成圈，候泊牌在風裡敲著木柱。")
    } else if room_name.contains("東南船塢口") {
        Some(
            "你走到東南船塢口。潮線貼著風舶角港倉門外沿過來，船匠把半修的船板靠在入口旁，木屑和瀝青味從修船架那頭飄出。",
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
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some(
            "你停在風舶角外洋棧倉前。雨布半遮著門口，倉管把異地木箱分成已查與待查兩側，門內拖箱聲沉沉傳出。",
        )
    } else if room_name.contains("翻譯桌") {
        Some(
            "你走到翻譯桌旁。合約攤在鎮紙下，翻譯把陌生貨名轉成在地話，海關前棚的印章聲在旁邊等著落下。",
        )
    } else if room_name.contains("匯兌小屋") {
        Some("你走進匯兌小屋前。銅秤在桌邊輕晃，兌票人把商館街口報來的價碼寫進賬冊，錢票疊在手邊。")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some(
            "你走到風舶角避風港。冷海水貼著木樁晃動，船戶把結霜纜繩重新繞緊，避風船在內側輕碰碼頭。",
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
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("你踏上烏石鼻守望石階。霧牌和警鈴掛在風口，守望人扶著濕石欄，看向港外海面的顏色。")
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
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("你走到紅土崁選礦車場。礦車停在磨亮車轍旁，車夫把繩索掛上車架，稱重木牌半埋在灰粉裡。")
    } else if room_name.contains("農舍前坪") {
        Some("你走進農舍前坪。鋤具靠在牆下，曬衣繩從低簷拉到木樁，屋前濕泥帶著田裡剛回來的腳印。")
    } else if room_name.contains("穀場小棚") {
        Some("你來到穀場小棚。幾袋稻穀半遮在棚下，木篩靠著棚柱，稻殼被腳步踩得乾乾作響。")
    } else if room_name.contains("田埂深處") {
        Some("你走進田埂深處。主要腳步聲被留在身後，濕草貼著褲腳，水鳥在更低的田面一閃而過。")
    } else if room_name.contains("挑擔小徑") {
        Some("你轉進挑擔小徑。草邊腳印密起來，短貨與水桶像是常避著渡口正面從這裡穿過。")
    } else if room_name.contains("水尾寮外谷地路") || room_name.contains("外谷地路") {
        Some("你走到水尾寮外谷地路。攤架聲被留在身後，挑擔的人沿泥路把小件貨送進送出。")
    } else if room_name.contains("南側谷地路") {
        Some("你走上南側谷地路。泥坡外的人聲低下去，車轍往田埂與下游方向拉長。")
    } else if room_name.contains("下游田埂路") {
        Some("你走到下游田埂路。車轍變淺，水田和濕草把渡口貨聲接成遠遠的回響。")
    } else if room_name.contains("渡口貨路") {
        Some("你走上渡口貨路。雨布與車輪印把倉棚前場的貨流牽向更遠的河邊。")
    } else if room_name.contains("貨路外彎") {
        Some("你來到貨路外彎。路面在草坡前拐開，貨車痕和腳印都被河風磨得散亂。")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("你走進水尾寮渡頭市集。價牌掛在貨攤邊晃動，有人掂著小包零貨，也有人壓低聲音打聽下一班渡船。")
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
                "市集棚腳還在滴水，有人把價牌往乾處挪，討價聲被雨後濕氣壓得更低。",
                "市集湯攤熱氣混著雨味往上冒，木牌被指節敲了一下，聲音很快被水聲吞掉。",
            ],
            seed,
        ),
        "fog" => pick_scene_variant(
            &[
                "市集價牌在霧裡只露出邊角，討價的人靠得更近，聲音貼著貨擔傳開。",
                "霧把市集那頭收得很窄，有人摸著木牌邊緣報價，又立刻壓低嗓音。",
            ],
            seed,
        ),
        "wind" => pick_scene_variant(
            &[
                "河風吹得市集棚布低低鼓起，價牌碰在木架上，討價聲斷成幾截。",
                "市集那頭有人按住晃動的木牌，另一手把零貨往棚柱內側推。",
            ],
            seed,
        ),
        _ => match scene_phase(game_hour) {
            ScenePhase::Dawn => pick_scene_variant(
                &[
                    "市集剛把價牌掛起，湯桶熱氣先冒出來，早到的人低聲問第一班船。",
                    "天光還薄，市集的人把木牌翻到外側，貨聲比人聲更早貼上岸邊。",
                ],
                seed,
            ),
            ScenePhase::Day => pick_scene_variant(
                &[
                    "市集那頭有人壓低聲音討價，木牌被指節敲得輕響。",
                    "日光照開市集價牌，有人把小件貨往外推半寸，又立刻被問價聲攔住。",
                ],
                seed,
            ),
            ScenePhase::Dusk => pick_scene_variant(
                &[
                    "市集開始收暗，有人把價牌往燈邊移，討價聲貼著棚布低低延續。",
                    "暮色壓到棚口，貨聲變少，剩下幾句趕船前的短價來回咬著。",
                ],
                seed,
            ),
            ScenePhase::Night => pick_scene_variant(
                &[
                    "市集只剩幾圈黃暈，有人隔著燈影問價，木牌輕響後又沉進夜風裡。",
                    "夜裡市集的貨聲收得很低，湯攤火光貼著木牌晃，問船的人不再大聲說話。",
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
                "幾撥候渡的人把包袱挪到腳邊，視線在市集、踏板和河面之間來回。",
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
    } else if room_name.contains("半嶺水岸渡口")
        || room_name.contains("水岸渡口")
        || room_name.contains("北岸")
        || room_name.contains("等候")
    {
        Some(
            "你回到半嶺水岸渡口。踏板前的人群已重新合攏，包袱和腳步把方才空出的縫隙填回；有人又抬頭望向河面。",
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
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        Some(
            "你回到水尾寮水網口。棧板上的濕腳印又多了一層，霧牌仍在木柱上輕晃，引船人的視線重新落回水道。",
        )
    } else if room_name.contains("湖港木棧") {
        Some(
            "你回到湖港木棧。候船的人群換了站位，濕繩圈旁多了些滴水痕，泊位和分揀場的聲音又在腳下聚攏。",
        )
    } else if room_name.contains("短船泊位") {
        Some("你回到短船泊位。方才靠著木樁的船身換了角度，船戶把船槳收近，等下一撥人踩穩棧板。")
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
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        Some("你回到風舶角港倉門。雨布被掀開又落下，倉管把新放行的貨單壓到封牌旁。")
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
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        Some("你回到風舶角外洋棧倉。雨布被拖箱聲頂起又落下，倉管把新放行的箱包推進門內陰影。")
    } else if room_name.contains("翻譯桌") {
        Some("你回到翻譯桌旁。合約邊角被重新壓住，翻譯把方才沒說清的貨名再轉了一遍。")
    } else if room_name.contains("匯兌小屋") {
        Some("你回到匯兌小屋。銅秤還在輕晃，兌票人把新價碼添進賬冊，錢票聲很快低下去。")
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        Some("你回到風舶角避風港。結霜纜繩又被拉緊一截，避風船貼著木樁換了角度。")
    } else if room_name.contains("港灣貨棚") {
        Some("你回到港灣貨棚。濕木箱旁多了一道水痕，搬魚人把新送到的魚籃推近棚門。")
    } else if room_name.contains("漁網曬場") {
        Some("你回到漁網曬場。半乾濕網被翻出新補痕，補網人把木架往風少的地方挪了一點。")
    } else if room_name.contains("冷霧魚市") {
        Some("你回到冷霧魚市。木秤旁換了一籃魚貨，魚販把問價聲重新壓回濕布邊。")
    } else if room_name.contains("船戶屋排") {
        Some("你回到船戶屋排。火盆煙氣還貼著屋簷，排班板上的濕木牌換了一格位置。")
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        Some("你回到烏石鼻守望石階。霧牌仍在風口晃動，守望人把目光從港外收回石階下。")
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
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        Some("你回到紅土崁選礦車場。礦車又往舊道口挪了一截，稱重木牌上的灰粉被手背抹開。")
    } else if room_name.contains("農舍前坪") {
        Some("你回到農舍前坪。牆下鋤具還在，曬衣繩輕晃，屋前濕泥把田裡回來的腳印留得更深。")
    } else if room_name.contains("穀場小棚") {
        Some("你回到穀場小棚。稻袋被挪開一角，木篩仍靠著棚柱，稻殼在腳下發出短短乾響。")
    } else if room_name.contains("田埂深處") {
        Some("你回到田埂深處。濕草重新貼上腳邊，遠處田區路口的人聲只剩一點斷續回音。")
    } else if room_name.contains("挑擔小徑") {
        Some("你又轉回挑擔小徑。草邊新腳印接上舊痕，像是剛有挑擔人從這裡避過渡口正面。")
    } else if room_name.contains("水尾寮外谷地路") || room_name.contains("外谷地路") {
        Some("你回到水尾寮外谷地路。攤架聲從身後斷續傳來，泥路上的小件貨腳印又密了一層。")
    } else if room_name.contains("南側谷地路") {
        Some("你回到南側谷地路。車轍裡積著薄水，泥坡那邊的渡船木聲仍遠遠傳來。")
    } else if room_name.contains("下游田埂路") {
        Some("你回到下游田埂路。濕草邊的水聲沒有停，渡口貨聲只剩遠遠一線。")
    } else if room_name.contains("渡口貨路") {
        Some("你又走上渡口貨路。雨布味和車輪印還留在路面，倉棚那邊的搬貨聲已被拉遠。")
    } else if room_name.contains("貨路外彎") {
        Some("你回到貨路外彎。草坡前的路面仍被車輪壓亮，貨流像是從這裡拐向更遠處。")
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        Some("你又走回水尾寮渡頭市集。價牌仍在河風裡晃，方才談價的人換了位置，打聽渡船的低語還沒散。")
    } else if room_name.contains("歇腳") {
        Some("你回到臨河歇腳處。長凳旁的茶碗換了幾隻手，包袱仍壓在腳邊，談話聲貼著河風低低延續。")
    } else if room_name.contains("踏板") {
        Some("你又停在渡船踏板前。濕木板仍隨水聲輕晃，方才讓開的人群又把腳步收回繩柱旁。")
    } else if room_name.contains("南岸") || room_name.contains("泥坡") {
        Some("你回到南岸泥坡。濕泥上新舊腳印疊在一起，坡邊等船的人換了站姿，目光仍貼著河面。")
    } else if room_name.contains("船戶作業點") || room_name.contains("作業") {
        Some("你又走近船戶作業點。木樁旁的纜繩少了一圈，船邊的人把方才沒說完的活計接著分派下去。")
    } else if room_name.contains("倉棚") {
        Some("你回到倉棚前場。棚門旁的麻袋被挪出一道新縫，木箱拖痕在泥地上接上方才那一串腳印。")
    } else {
        None
    }
}

fn scene_route_reentry_logs(room_name: &str) -> Vec<(&'static str, &'static str)> {
    if room_name.contains("半嶺水岸渡口")
        || room_name.contains("水岸渡口")
        || room_name.contains("北岸")
        || room_name.contains("等候")
    {
        vec![
            (
                "水尾寮渡頭市集",
                "你從水尾寮渡頭市集那頭回到半嶺水岸渡口。身後的價牌聲被河風拉遠，踏板前的人群已重新合攏；有人把剛買的零散貨塞回包袱裡，又抬頭看船。",
            ),
            (
                "渡船踏板",
                "你從渡船踏板退回半嶺水岸渡口。濕木板的晃動還留在腳下，候渡的人群順著繩柱旁的縫隙重新合攏。",
            ),
            (
                "臨河歇腳處",
                "你從歇腳處回到半嶺水岸渡口。茶氣和低談被留在身後，踏板前的人聲又把河面動靜壓近了些。",
            ),
            (
                "中河谷地路口",
                "你從谷地路口回到半嶺水岸渡口。泥路腳印被留在身後，踏板前的船繩聲又把候渡人群短短壓靜。",
            ),
        ]
    } else if room_name.contains("中河谷地路口") {
        vec![
            (
                "半嶺水岸渡口",
                "你從半嶺水岸渡口走到中河谷地路口。候渡的人聲退到身後，泥路在腳下分向田埂和市集外。",
            ),
            (
                "北堤泥路",
                "你從北堤泥路回到中河谷地路口。堤邊河霧被留在身後，渡口人聲又順著泥路貼近。",
            ),
            (
                "水尾寮外谷地路",
                "你從水尾寮外谷地路回到中河谷地路口。攤架聲斷在路旁，田埂水聲重新接了上來。",
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
                "水尾寮水網口",
                "你從水尾寮水網口退回秧苗田邊。船路的濕木聲被留在身後，淺綠秧苗又把腳步收回田面。",
            ),
        ]
    } else if room_name.contains("水尾寮水網口") || room_name.contains("水網入口") {
        vec![
            (
                "秧苗田邊",
                "你從秧苗田邊走到水尾寮水網口。田面蛙聲被濕木棧接住，田邊腳印在霧牌下轉成水路方向。",
            ),
            (
                "湖港木棧",
                "你從湖港木棧回到水尾寮水網口。候船聲被拉到身後，引船人重新把目光放回半開的水道。",
            ),
            (
                "短船泊位",
                "你從短船泊位回到水尾寮水網口。船身晃動退到身後，篙影和霧牌又把水道入口壓近眼前。",
            ),
        ]
    } else if room_name.contains("湖港木棧") {
        vec![
            (
                "水尾寮水網口",
                "你從水尾寮水網口踏上湖港木棧。田邊腳印斷在身後，濕繩圈、潮線和候船低語在棧板上接起來。",
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
                "水尾寮水網口",
                "你從水尾寮水網口走到短船泊位。霧牌落在身後，短船貼著木樁輕晃，船戶伸手示意人踩穩。",
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
                "風舶角港倉門",
                "你從風舶角港倉門回到稅站棚。雨布和封牌聲退到身後，印章又把放行與候查分成兩路。",
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
    } else if room_name.contains("風舶角港倉門") || room_name.contains("港倉門") {
        vec![
            (
                "稅站棚",
                "你從稅站棚走到風舶角港倉門。放行貨單被帶到雨布下，倉管把河貨和海貨重新分門。",
            ),
            (
                "商船候泊線",
                "你從商船候泊線回到風舶角港倉門。候泊牌聲被留在潮線外，封牌和雨布重新接住入倉貨流。",
            ),
            (
                "東南船塢口",
                "你從東南船塢口回到風舶角港倉門。修船木屑味被留在外頭，倉門前又只剩雨布和封牌低響。",
            ),
        ]
    } else if room_name.contains("商船候泊線") {
        vec![
            (
                "潮線貨場",
                "你從潮線貨場走到商船候泊線。木箱和潮印退到身後，等潮船影與錨繩把腳步帶到外線。",
            ),
            (
                "風舶角港倉門",
                "你從風舶角港倉門走到商船候泊線。雨布聲落在倉門後，船員隔著潮水等稅站棚那頭放行。",
            ),
        ]
    } else if room_name.contains("東南船塢口") {
        vec![
            (
                "風舶角港倉門",
                "你從風舶角港倉門走到東南船塢口。雨布和封牌退到身後，木屑、瀝青味和半修船板把腳步接進船港。",
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
                "風舶角外洋棧倉",
                "你從風舶角外洋棧倉回到海關前棚。雨布和拖箱聲落在身後，海關吏把新放行的貨單壓回桌上。",
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
                "風舶角外洋棧倉",
                "你從風舶角外洋棧倉回到商館街口。雨布與木箱漆印退到身後，商館人又把價碼送往匯兌小屋。",
            ),
        ]
    } else if room_name.contains("風舶角外洋棧倉") || room_name.contains("外洋棧倉") {
        vec![
            (
                "海關前棚",
                "你從海關前棚走到風舶角外洋棧倉。封條聲留在棚下，已查和待查的異地箱把門前分成兩側。",
            ),
            (
                "商館街口",
                "你從商館街口走到風舶角外洋棧倉。貨樣匣聲退到街口，雨布下的木箱漆印和拖箱聲重新貼近。",
            ),
            (
                "匯兌小屋",
                "你從匯兌小屋回到風舶角外洋棧倉。銅秤聲留在身後，倉管照著新價碼把箱包推進門內陰影。",
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
                "風舶角外洋棧倉",
                "你從風舶角外洋棧倉走到匯兌小屋。雨布和拖箱聲退到身後，銅秤、賬冊和錢票把貨名換成價碼。",
            ),
            (
                "翻譯桌",
                "你從翻譯桌走到匯兌小屋。合約文字留在桌上，兌票人把剛說清的價碼壓進賬冊欄裡。",
            ),
        ]
    } else if room_name.contains("風舶角避風港") || room_name.contains("避風港") {
        vec![
            (
                "農舍前坪",
                "你從農舍前坪走到風舶角避風港。田邊濕泥退到身後，冷海水、結霜纜繩和避風船把路接到港邊。",
            ),
            (
                "港灣貨棚",
                "你從港灣貨棚回到風舶角避風港。鹽箱和濕木箱聲落在棚下，船戶重新把纜繩收緊。",
            ),
            (
                "漁網曬場",
                "你從漁網曬場回到風舶角避風港。濕網木架聲被海風帶遠，避風船的碰岸聲重新貼近腳邊。",
            ),
        ]
    } else if room_name.contains("港灣貨棚") {
        vec![
            (
                "風舶角避風港",
                "你從風舶角避風港走進港灣貨棚。結霜纜繩聲退在身後，鹽箱和濕魚籃把腳步收進棚門。",
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
                "風舶角避風港",
                "你從風舶角避風港走到漁網曬場。避風船聲退到木樁那頭，半乾濕網和木架把海風截成碎響。",
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
                "烏石鼻守望石階",
                "你從烏石鼻守望石階下到冷霧魚市。霧牌聲還在身後，魚販的秤錘聲很快把人聲重新聚起來。",
            ),
        ]
    } else if room_name.contains("船戶屋排") {
        vec![
            (
                "港灣貨棚",
                "你從港灣貨棚走到船戶屋排。濕木箱聲落在棚門後，火盆煙氣和排班板把港邊活計收進屋簷下。",
            ),
            (
                "烏石鼻守望石階",
                "你從烏石鼻守望石階回到船戶屋排。霧牌和警鈴聲退到風口，排班板前的低語重新接住腳步。",
            ),
        ]
    } else if room_name.contains("烏石鼻守望石階") || room_name.contains("守望石階") {
        vec![
            (
                "冷霧魚市",
                "你從冷霧魚市踏上烏石鼻守望石階。木秤和問價聲被甩在霧裡，警鈴與霧牌聲在風口變清。",
            ),
            (
                "船戶屋排",
                "你從船戶屋排走上烏石鼻守望石階。火盆煙氣貼在身後，守望人扶著石欄看向港外海色。",
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
                "紅土崁選礦車場",
                "你從紅土崁選礦車場回到工具修棚。車軸聲被棚頂壓低，修好的支架和鐵件等著下一批人來領。",
            ),
        ]
    } else if room_name.contains("礦渣坡") {
        vec![
            (
                "洗礦溝",
                "你從洗礦溝走到礦渣坡。冷水槽聲退到坡下，灰黑碎石和新滑坡痕把路邊界壓得很窄。",
            ),
            (
                "紅土崁選礦車場",
                "你從紅土崁選礦車場下到礦渣坡。車輪聲被留在舊道口，推車人把廢料倒進安全線內。",
            ),
        ]
    } else if room_name.contains("紅土崁選礦車場") || room_name.contains("選礦車場") {
        vec![
            (
                "工具修棚",
                "你從工具修棚走到紅土崁選礦車場。敲鐵聲落在棚後，礦車、車轍和稱重木牌把路接向邊城外門方向。",
            ),
            (
                "礦渣坡",
                "你從礦渣坡回到紅土崁選礦車場。碎石坡聲沉到身後，車夫把繩索掛上灰粉裡的車架。",
            ),
            (
                "邊城外門",
                "你從邊城外門走回紅土崁選礦車場。城門人聲退到舊道後，礦車和稱重木牌重新把腳步拉回礦場。",
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
    } else if room_name.contains("水尾寮渡頭市集") || room_name.contains("渡頭市集") {
        vec![
            (
                "半嶺水岸渡口",
                "你從半嶺水岸渡口又走回水尾寮渡頭市集。身後候渡的人聲還沒散，價牌下的低聲議價已先接了上來。",
            ),
            (
                "倉棚前場",
                "你從倉棚前場回到水尾寮渡頭市集。木箱拖痕一路斷在貨攤邊，問價聲很快把搬貨的聲音蓋過去。",
            ),
            (
                "船戶作業點",
                "你從船戶作業點折回水尾寮渡頭市集。纜繩和木楔的氣味還在身後，攤邊已有人壓低聲音問下一班渡船。",
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

// 此檔仍保留退役方格的相容處理器；相關小工具暫時位於測試模組後方，
// 待方格相容層整體移除時一併刪除，避免為通過 lint 搬動大段既有程式。
#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    fn npc_card(id: &'static str, label: &str, card_state: &'static str) -> NpcCard<'static> {
        NpcCard {
            id,
            label: label.to_string(),
            card_state,
            current_activity: "",
            interaction_priority: false,
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

        assert!(!has_hidden_unnamed);
        assert_eq!(visible.len(), 9);
        assert!(visible.iter().any(|card| card.id == "n1"));
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.display_name.starts_with("候渡者"))
                .count(),
            5
        );
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.display_name.starts_with("行人"))
                .count(),
            3
        );
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.card_state != "named")
                .count(),
            8
        );
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
        assert_eq!(names, vec!["候渡者1", "候渡者2", "行人"]);
    }

    #[test]
    fn keeps_available_market_operator_out_of_folded_crowd() {
        let mut cards = vec![
            npc_card("s1", "行人", "stranger"),
            npc_card("s2", "行人", "stranger"),
            npc_card("s3", "行人", "stranger"),
            npc_card("s4", "行人", "stranger"),
            npc_card("s5", "行人", "stranger"),
            npc_card("market", "行人", "stranger"),
        ];
        cards[5].interaction_priority = true;

        let (visible, has_hidden_unnamed) = build_player_layer_npc_cards(&cards);

        assert!(!has_hidden_unnamed);
        assert!(visible.iter().any(|card| card.id == "market"));
        assert_eq!(
            visible
                .iter()
                .filter(|card| card.card_state != "named")
                .count(),
            6
        );
    }

    #[test]
    fn north_bank_scene_changes_with_time_without_losing_place_anchor() {
        let empty_events: Vec<String> = Vec::new();
        let dawn = north_bank_waiting_scene_desc(6 * 60, Some(("clear", 0)), 8, &empty_events);
        let night = north_bank_waiting_scene_desc(22 * 60, Some(("clear", 0)), 8, &empty_events);

        assert_ne!(dawn, night);
        assert!(dawn.contains("半嶺水岸渡口"));
        assert!(night.contains("半嶺水岸渡口"));
        assert!(dawn.contains("晨霧") || dawn.contains("天光"));
        assert!(night.contains("夜水") || night.contains("夜風"));
    }

    #[test]
    fn north_bank_scene_projects_climate_and_crowd_without_exact_count() {
        let empty_events: Vec<String> = Vec::new();
        let rainy_crowd =
            north_bank_waiting_scene_desc(13 * 60, Some(("rain", 2)), 14, &empty_events);

        assert!(rainy_crowd.contains("雨後") || rainy_crowd.contains("濕氣"));
        assert!(rainy_crowd.contains("候渡的人群") || rainy_crowd.contains("市集邊"));
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
        let entry = player_layer_entry_log("半嶺水岸渡口", 8);
        let lines: Vec<&str> = entry
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();

        assert!(lines.len() >= 3, "北岸第一屏入場動態應是可閱讀的多行場景流");
        assert!(entry.contains("船繩"));
        assert!(entry.contains("市集"));
        assert!(entry.contains("等船人群") || entry.contains("候渡"));
        for forbidden in ["你", "現場動態", "登入後", "debug", "資料庫", "8"] {
            assert!(!entry.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn north_bank_entry_scene_flow_changes_with_time_weather_and_ferry_phase() {
        let fog_dawn = player_layer_entry_log_for_scene("半嶺水岸渡口", 8, 6 * 60, Some(("fog", 11)));
        let clear_night =
            player_layer_entry_log_for_scene("半嶺水岸渡口", 8, 23 * 60 + 40, Some(("clear", 11)));

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
        let first = player_layer_entry_log("半嶺水岸渡口", 8);
        let again = player_layer_reentry_log("半嶺水岸渡口", 8);

        assert_ne!(first, again);
        assert!(again.contains("回到半嶺水岸渡口"));
        assert!(!again.contains("站定"));
        assert!(!again.contains('8'));
    }

    #[test]
    fn north_bank_reentry_log_can_reflect_previous_room() {
        let generic = player_layer_reentry_log("半嶺水岸渡口", 8);
        let from_market = route_reentry_log_for_room("半嶺水岸渡口", "水尾寮渡頭市集", 8)
            .expect("北岸應有從市集返回的路徑句");

        assert_ne!(generic, from_market);
        assert!(from_market.contains("從水尾寮渡頭市集那頭回到半嶺水岸渡口"));
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
        let entry = player_layer_entry_log("水尾寮外谷地路", 1);

        assert!(entry.contains("水尾寮外谷地路"));
        assert!(entry.contains("挑擔"));
        assert!(!entry.contains("走進水尾寮渡頭市集"));
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
        let water_entry = player_layer_entry_log("水尾寮水網口", 7);
        let fish_sorting = player_layer_entry_log("魚貨分揀場", 7);

        assert!(water_entry.contains("水尾寮水網口"));
        assert!(water_entry.contains("引船人"));
        assert!(fish_sorting.contains("魚貨分揀場"));
        assert!(fish_sorting.contains("分揀工"));
        assert_eq!(crowd_label_for_scene("水倉前場"), "水倉人群");
        assert!(!water_entry.contains('7'));
        assert!(!fish_sorting.contains('7'));
    }

    #[test]
    fn m3_032_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("水尾寮水網口", 7);
        let from_field = route_reentry_log_for_room("水尾寮水網口", "秧苗田邊", 7)
            .expect("水尾寮水網口應有從秧苗田邊進入的路徑句");
        let from_warehouse = route_reentry_log_for_room("魚貨分揀場", "水倉前場", 7)
            .expect("魚貨分揀場應有從水倉返回的路徑句");

        assert_ne!(generic, from_field);
        assert!(from_field.contains("從秧苗田邊走到水尾寮水網口"));
        assert!(from_field.contains("田面蛙聲"));
        assert!(from_warehouse.contains("從水倉前場回到魚貨分揀場"));
        assert!(!from_field.contains('7'));
        assert!(!from_warehouse.contains('7'));
    }

    #[test]
    fn m3_032_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "水尾寮水網口"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "魚貨分揀場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "水倉前場"), "陌生人");
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
        assert_eq!(observed_label_for_scene("張三", "糧倉外秤"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "倉門前場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "糧車泊位"), "陌生人");
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
        assert_eq!(observed_label_for_scene("張三", "河口潮泥碼頭"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "稅站棚"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "商船候泊線"), "陌生人");
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
        let from_warehouse = route_reentry_log_for_room("東南船塢口", "風舶角港倉門", 12)
            .expect("東南船塢口應有從風舶角港倉門進入的路徑句");
        let from_repair = route_reentry_log_for_room("帆布曬場", "修船架", 12)
            .expect("帆布曬場應有從修船架進入的路徑句");
        let from_steps = route_reentry_log_for_room("船戶街口", "碼頭木階", 12)
            .expect("船戶街口應有從碼頭木階進入的路徑句");

        assert_ne!(generic, from_warehouse);
        assert!(from_warehouse.contains("從風舶角港倉門走到東南船塢口"));
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
        assert_eq!(observed_label_for_scene("張三", "東南船塢口"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "修船架"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "帆布曬場"), "陌生人");
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
        let from_warehouse = route_reentry_log_for_room("匯兌小屋", "風舶角外洋棧倉", 9)
            .expect("匯兌小屋應有從風舶角外洋棧倉進入的路徑句");

        assert_ne!(generic, from_sailcloth);
        assert!(from_sailcloth.contains("從帆布曬場走到外貿碼頭"));
        assert!(from_sailcloth.contains("港旗"));
        assert!(from_customs.contains("從海關前棚走到翻譯桌"));
        assert!(from_warehouse.contains("從風舶角外洋棧倉走到匯兌小屋"));
        assert!(!from_sailcloth.contains("9"));
        assert!(!from_customs.contains("9"));
        assert!(!from_warehouse.contains("9"));
    }

    #[test]
    fn m3_053_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "外貿碼頭"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "海關前棚"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "翻譯桌"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "匯兌小屋"), "陌生人");
    }

    #[test]
    fn m3_006_entry_logs_have_north_harbor_presence_not_counts() {
        let dock = player_layer_entry_log("風舶角避風港", 8);
        let shed = player_layer_entry_log("港灣貨棚", 8);
        let watch = player_layer_entry_log("烏石鼻守望石階", 8);

        assert!(dock.contains("風舶角避風港"));
        assert!(dock.contains("船戶"));
        assert!(shed.contains("港灣貨棚"));
        assert!(shed.contains("搬魚人"));
        assert!(watch.contains("烏石鼻守望石階"));
        assert!(watch.contains("守望人"));
        assert_eq!(crowd_label_for_scene("冷霧魚市"), "魚市人群");
        assert_eq!(crowd_label_for_scene("漁網曬場"), "補網人群");
        assert!(!dock.contains('8'));
        assert!(!shed.contains('8'));
        assert!(!watch.contains('8'));
    }

    #[test]
    fn m3_006_route_reentry_log_reflects_previous_room() {
        let generic = player_layer_reentry_log("風舶角避風港", 8);
        let from_farm = route_reentry_log_for_room("風舶角避風港", "農舍前坪", 8)
            .expect("風舶角避風港應有從農舍前坪進入的路徑句");
        let from_dock = route_reentry_log_for_room("港灣貨棚", "風舶角避風港", 8)
            .expect("港灣貨棚應有從風舶角避風港進入的路徑句");
        let from_market = route_reentry_log_for_room("烏石鼻守望石階", "冷霧魚市", 8)
            .expect("烏石鼻守望石階應有從冷霧魚市進入的路徑句");

        assert_ne!(generic, from_farm);
        assert!(from_farm.contains("從農舍前坪走到風舶角避風港"));
        assert!(from_farm.contains("冷海水"));
        assert!(from_dock.contains("從風舶角避風港走進港灣貨棚"));
        assert!(from_market.contains("從冷霧魚市踏上烏石鼻守望石階"));
        assert!(!from_farm.contains("8"));
        assert!(!from_dock.contains("8"));
        assert!(!from_market.contains("8"));
    }

    #[test]
    fn m3_006_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "港灣貨棚"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "漁網曬場"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "冷霧魚市"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "烏石鼻守望石階"), "陌生人");
    }

    #[test]
    fn m3_009_entry_logs_have_mine_presence_not_counts() {
        let pit = player_layer_entry_log("礦坑外棚", 8);
        let wash = player_layer_entry_log("洗礦溝", 8);
        let load = player_layer_entry_log("紅土崁選礦車場", 8);

        assert!(pit.contains("礦坑外棚"));
        assert!(pit.contains("礦工"));
        assert!(wash.contains("洗礦溝"));
        assert!(wash.contains("洗礦工"));
        assert!(load.contains("紅土崁選礦車場"));
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
        let from_border = route_reentry_log_for_room("紅土崁選礦車場", "邊城外門", 8)
            .expect("紅土崁選礦車場應有從邊城外門進入的路徑句");

        assert_ne!(generic, from_field);
        assert!(from_field.contains("從田埂深處走到礦坑外棚"));
        assert!(from_field.contains("吊燈光"));
        assert!(from_pit.contains("從礦坑外棚走到洗礦溝"));
        assert!(from_border.contains("從邊城外門走回紅土崁選礦車場"));
        assert!(!from_field.contains("8"));
        assert!(!from_pit.contains("8"));
        assert!(!from_border.contains("8"));
    }

    #[test]
    fn m3_009_observed_labels_fall_back_to_public_roles() {
        assert_eq!(observed_label_for_scene("張三", "礦坑外棚"), "陌生人");
        assert_eq!(observed_label_for_scene("李四", "洗礦溝"), "陌生人");
        assert_eq!(observed_label_for_scene("王五", "工具修棚"), "陌生人");
        assert_eq!(observed_label_for_scene("趙六", "礦渣坡"), "陌生人");
        assert_eq!(observed_label_for_scene("錢七", "紅土崁選礦車場"), "陌生人");
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
            ("西門坡道街", "坡街販", "坡街人群"),
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
        append_m3_020_public_presence_cards("半嶺水岸渡口", &mut other_cards);
        assert!(other_cards.is_empty());
    }

    #[test]
    fn final_map_observed_labels_fall_back_to_public_roles() {
        for (room, _label) in [
            ("邊城外門", "守門人"),
            ("哨樓梯", "哨兵"),
            ("青籠關邊貿坪", "商旅"),
            ("城邦外坡道", "牽獸人"),
            ("石壘門", "門衛"),
            ("門內券道", "引路人"),
            ("旗石廣場", "旗手"),
            ("西門坡道街", "坡街販"),
            ("議棚前階", "記牌人"),
            ("鑄幣巷口", "兌牌人"),
            ("傭工棚邊", "棚頭人"),
            ("鍛造棚", "鐵匠"),
            ("油布庫", "看庫人"),
            ("畜欄口", "牲口販"),
            ("獸醫棚", "獸醫"),
            ("商隊泊位", "商隊人"),
            ("客棧前廊", "店伙"),
            ("補給市集", "小販"),
            ("巡哨棚", "巡哨"),
        ] {
            assert_eq!(observed_label_for_scene("張三", room), "陌生人", "{room}");
        }
    }

    #[test]
    fn canonical_npc_narrative_subjects_match_roles_and_attire() {
        let chen = VisibleNpcCard {
            id: "wt_npc_chen_wanfu",
            display_name: "陌生人".to_string(),
            card_state: "stranger",
            current_activity: "熬煮濃湯",
        };
        let chen_subj = narrative_person_subject(&chen, 0);
        assert!(chen_subj.contains("主廚"), "{chen_subj} should contain 主廚");
        assert!(chen_subj.contains("潔白雙排扣主廚服與厚布圍裙"), "{chen_subj} should contain 主廚服");
        assert!(!chen_subj.contains("外地商旅"), "{chen_subj} must not be 外地商旅");

        let lu = VisibleNpcCard {
            id: "wt_npc_lu_chengli",
            display_name: "陌生人".to_string(),
            card_state: "stranger",
            current_activity: "審理戶籍",
        };
        let lu_subj = narrative_person_subject(&lu, 0);
        assert!(lu_subj.contains("主事"), "{lu_subj} should contain 主事");
        assert!(!lu_subj.contains("匠人"), "{lu_subj} must not be 匠人");
    }
}

// ── 昏倒輔助 ──────────────────────────────────────────────────────────────

/// 找曼哈頓距離最近的脈絡診所（+）或 landmark 格作為聚落傳送目的地。
/// 找不到時回傳 None（呼叫端 fallback 到 (0, 0, "脈絡診所")）。
pub fn find_nearest_clinic_for_combat(from_x: i32, from_y: i32) -> (i32, i32, String) {
    find_nearest_clinic_or_landmark(from_x, from_y)
        .unwrap_or_else(|| (210, 510, "脈絡診所".to_string()))
}

fn find_nearest_clinic_or_landmark(from_x: i32, from_y: i32) -> Option<(i32, i32, String)> {
    let grid = crate::server::grid_manager::get_runtime_square_grid()?;
    // 優先找脈絡診所（+）
    let clinic = grid
        .cells()
        .filter(|c| c.terrain == crate::grid::Terrain::Clinic || c.terrain.display_char() == "+")
        .min_by_key(|c| (c.coord.x - from_x).abs() + (c.coord.y - from_y).abs());
    if let Some(c) = clinic {
        let name = if c.name.is_empty() {
            "脈絡診所".to_string()
        } else {
            c.name.clone()
        };
        return Some((c.coord.x, c.coord.y, name));
    }

    // 次選其他聚落地標
    grid.cells()
        .filter(|c| c.terrain.category() == "landmark" && !c.name.is_empty())
        .min_by_key(|c| (c.coord.x - from_x).abs() + (c.coord.y - from_y).abs())
        .map(|c| (c.coord.x, c.coord.y, c.name.clone()))
}

#[allow(dead_code)]
fn find_nearest_landmark(from_x: i32, from_y: i32) -> Option<(i32, i32, String)> {
    find_nearest_clinic_or_landmark(from_x, from_y)
}

/// 從背包隨機移除 1-3 件（整堆移除一個 item_id），寫入地面遺落資料表與快取，回傳被移除物品名稱列表。
pub fn faint_drop_items_to_ground(
    player_id: &str,
    inventory_json: &str,
    gx: i32,
    gy: i32,
    now: i64,
) -> Vec<String> {
    let dropped_tuples = drop_random_items(player_id, inventory_json);
    if !dropped_tuples.is_empty() {
        crate::pg::living_world::record_ground_dropped_items(
            gx,
            gy,
            player_id,
            &dropped_tuples,
            now,
        );
    }
    dropped_tuples.into_iter().map(|(_, name, _)| name).collect()
}

/// 從背包隨機移除 1-3 件（整堆移除一個 item_id），回傳被移除物品的 (item_id, item_name, qty) 列表。
fn drop_random_items(entity_id: &str, inventory_json: &str) -> Vec<(String, String, i32)> {
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

    let droppable: Vec<&Entry> = entries
        .iter()
        .filter(|e| {
            !e.item_id.starts_with("manual_")
                && !e.item_id.starts_with("corpse_")
                && !e.item_id.starts_with("crystal_")
                && !e.item_id.starts_with("key_")
                && !e.item_id.starts_with("wt_cat_0793")
        })
        .collect();

    if droppable.is_empty() {
        return vec![];
    }

    let mut rng = rand::rng();
    let drop_count = rng.random_range(1_usize..=3_usize).min(droppable.len());

    // Fisher-Yates 取前 drop_count 個隨機索引
    let mut indices: Vec<usize> = (0..droppable.len()).collect();
    for i in 0..drop_count {
        let j = rng.random_range(i..droppable.len());
        indices.swap(i, j);
    }

    let mut dropped = Vec::new();
    for &idx in &indices[..drop_count] {
        let e = droppable[idx];
        // 移除整堆
        let _ = db::remove_from_inventory(entity_id, &e.item_id, e.qty);
        dropped.push((e.item_id.clone(), db::get_item_name(&e.item_id), e.qty));
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
            conn.send_error("尚未置身於城鎮地圖中");
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

/// 地形改造處理函式：支援動態地形變更
/// 開山 ⛰︎->△、伐林 Φ->𐡚、開墾 𐡚->≡、播種 ≡->▦/▤/☷、造橋 ≈->≍
pub(super) fn handle_mutate_terrain(conn: &WsConnection, msg: &ClientMsg) {
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
    // 目標座標：優先使用 msg.x / msg.y，否則使用玩家當前位置
    let (tx, ty) = match (msg.x, msg.y) {
        (Some(x), Some(y)) => (x, y),
        _ => match (ent.grid_x, ent.grid_y) {
            (Some(x), Some(y)) => (x, y),
            _ => {
                conn.send_error("尚未置身於城鎮地圖中");
                return;
            }
        },
    };

    let current_terrain = crate::server::grid_manager::get_runtime_square_grid()
        .and_then(|g| g.get(SquareCoord::new(tx, ty)).map(|c| c.terrain));

    let Some(current_t) = current_terrain else {
        conn.send_error("目標格子不存在");
        return;
    };

    let action_str = msg.action.trim();
    let target_str = msg.target_terrain.trim();
    let input_str = msg.player_input.trim();

    // 判斷想要進行的地形改造
    let new_terrain = match (current_t, action_str, target_str) {
        // 開山：Mountain (⛰︎) -> Hills (△)
        (Terrain::Mountain, "mine" | "開山" | "quarry", _)
        | (Terrain::Mountain, _, "hills" | "△" | "丘陵") => Some(Terrain::Hills),

        // 伐林：Φ (Forest/ForestHeavy/ForestLight/Jungle) -> Plain (𐡚)
        (
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
            "chop" | "伐林" | "clearcut" | "伐木",
            _,
        )
        | (
            Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle,
            _,
            "plain" | "𐡚" | "平原",
        ) => Some(Terrain::Plain),

        // 開墾：𐡚 (Plain/Grassland/Desert/Tundra) -> EmptyFurrow (≡)
        (
            Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra,
            "till" | "開墾" | "plow" | "耕作",
            _,
        )
        | (
            Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra,
            _,
            "empty_furrow" | "≡" | "荒壟" | "壟溝",
        ) => Some(Terrain::EmptyFurrow),

        // 播種：EmptyFurrow (≡) -> ▦/▤/☷ (FarmField / DryField / HerbGarden)
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", "dry_field" | "▤" | "旱田") => {
            Some(Terrain::DryField)
        }
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", "herb_garden" | "☷" | "藥園" | "草藥園") => {
            Some(Terrain::HerbGarden)
        }
        (Terrain::EmptyFurrow, "sow" | "播種" | "plant", _) => {
            Some(Terrain::FarmField)
        }
        (Terrain::EmptyFurrow, _, "farm_field" | "▦" | "農田" | "水田") => {
            Some(Terrain::FarmField)
        }
        (Terrain::EmptyFurrow, _, "dry_field" | "▤" | "旱田") => Some(Terrain::DryField),
        (Terrain::EmptyFurrow, _, "herb_garden" | "☷" | "藥園" | "草藥園") => {
            Some(Terrain::HerbGarden)
        }

        // 造橋：Water (≈) -> Bridge (≍)
        (Terrain::Water, "bridge" | "造橋" | "build_bridge", _)
        | (Terrain::Water, _, "bridge" | "≍" | "橋樑") => Some(Terrain::Bridge),

        // 亦可相容由 player_input 觸發
        _ if input_str.contains("開山") && current_t == Terrain::Mountain => Some(Terrain::Hills),
        _ if (input_str.contains("伐林") || input_str.contains("伐木"))
            && matches!(
                current_t,
                Terrain::Forest | Terrain::ForestHeavy | Terrain::ForestLight | Terrain::Jungle
            ) =>
        {
            Some(Terrain::Plain)
        }
        _ if input_str.contains("開墾")
            && matches!(
                current_t,
                Terrain::Plain | Terrain::Grassland | Terrain::Desert | Terrain::Tundra
            ) =>
        {
            Some(Terrain::EmptyFurrow)
        }
        _ if input_str.contains("播種") && current_t == Terrain::EmptyFurrow => {
            if input_str.contains("藥") {
                Some(Terrain::HerbGarden)
            } else if input_str.contains("旱") {
                Some(Terrain::DryField)
            } else {
                Some(Terrain::FarmField)
            }
        }
        _ if input_str.contains("造橋") && current_t == Terrain::Water => Some(Terrain::Bridge),

        _ => None,
    };

    let Some(target_terrain) = new_terrain else {
        conn.send_error("此處地形無法執行該項改造或指令不符。");
        return;
    };

    // 使用 with_square_grid_mut 變更地形
    let mutated = crate::server::grid_manager::with_square_grid_mut(|grid| {
        if let Some(cell) = grid.get_mut(SquareCoord::new(tx, ty)) {
            cell.terrain = target_terrain;
            let suffix = match target_terrain {
                Terrain::Hills => "開鑿石坡",
                Terrain::Plain => "開闊平地",
                Terrain::EmptyFurrow => "整地空壟",
                Terrain::FarmField => "阡陌水田",
                Terrain::DryField => "平整旱田",
                Terrain::HerbGarden => "分畦藥圃",
                Terrain::Bridge => "木石便橋",
                other => crate::grid::terrain_name_zh(other),
            };
            if cell.name.contains('・') {
                let prefix = cell.name.split('・').next().unwrap_or("").trim();
                cell.name = format!("{prefix}・{suffix}");
            } else if cell.name.is_empty() || cell.name == crate::grid::terrain_name_zh(current_t) {
                cell.name = suffix.to_string();
            }
            grid.mark_dirty();
            true
        } else {
            false
        }
    })
    .unwrap_or(false);

    if !mutated {
        conn.send_error("地形改造失敗");
        return;
    }

    let narrative = match target_terrain {
        Terrain::EmptyFurrow => {
            "【百工開拓・墾荒闢壟】你揮動鋤具翻開荒草土石，順著地勢理出整齊通水的田壟溝渠。".to_string()
        }
        Terrain::FarmField => {
            "【百工開拓・播種育苗】你順著整好的田壟引水播下稻種，新苗在微風與活水滋潤下吐露青翠生機。".to_string()
        }
        Terrain::DryField => {
            "【百工開拓・整飭旱田】你耙平土塊、修整畦道，將田地整理得疏鬆透氣，便利各色雜糧與耐旱作物深扎根脈。".to_string()
        }
        Terrain::HerbGarden => {
            "【百工開拓・分畦藥圃】你細分苗床、鋪設腐殖沃土與疏水碎石，闢出一壟壟清氣撲鼻、適宜草藥生長的分畦苗圃。".to_string()
        }
        Terrain::Hills => {
            "【百工開拓・開鑿礦道】你順著岩層裂隙鑿開險峻山壁，闢出可供通行的丘陵石道與採石工作面。".to_string()
        }
        Terrain::Plain => {
            "【百工開拓・伐木取材】你揮斧伐開密林枝幹，清理出開闊平坦的林間空地。".to_string()
        }
        Terrain::Bridge => {
            "【百工開拓・架木為橋】你打下固若金湯的石木橋樁，平鋪厚實行水踏板，於湍急水脈上方築起便利往來的通途。".to_string()
        }
        _ => {
            "【百工開拓・地脈變更】你運用百工巧藝順應地勢梳理此方水土，讓地表煥然一新。".to_string()
        }
    };

    conn.send_json(&ActionResultMsg {
        msg_type: "action_result".into(),
        action: "MutateTerrain".into(),
        target_id: "__terrain_pioneer__".into(),
        target_name: "百工開拓".into(),
        narrative,
        success: true,
        actions: vec![],
        move_target_id: String::new(),
        open_trade: false,
        suggested_action: String::new(),
        tactical_choices: vec![],
    });

    // 立即推播最新地圖與視野
    let gh = current_game_hour(&conn.cfg);
    if let (Some(px), Some(py)) = (ent.grid_x, ent.grid_y) {
        send_grid_view(conn, &player_id, px, py, gh);
    } else {
        send_grid_view(conn, &player_id, tx, ty, gh);
    }
    send_place_view(conn, &player_id);
}

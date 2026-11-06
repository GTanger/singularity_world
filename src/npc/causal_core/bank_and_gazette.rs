pub fn step_offline_players_autonomy(now_unix: i64) -> usize {
    let online_ids = crate::server::session::list_online_player_ids();
    let offline_player_ids: Vec<String> = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .values()
            .filter(|e| (e.kind == "player" || e.kind == "Player") && !online_ids.contains(&e.id))
            .map(|e| e.id.clone())
            .collect()
    } else {
        let directives = PLAYER_OFFLINE_DIRECTIVES
            .read()
            .unwrap_or_else(|e| e.into_inner());
        directives
            .keys()
            .filter(|id| !online_ids.contains(*id))
            .cloned()
            .collect()
    };

    let mut stepped_count = 0;
    for pid in offline_player_ids {
        let should_step = {
            let mut last_steps = PLAYER_LAST_OFFLINE_STEP
                .write()
                .unwrap_or_else(|e| e.into_inner());
            let last = last_steps.get(&pid).copied().unwrap_or(0);
            if last == 0 {
                last_steps.insert(pid.clone(), now_unix);
                false
            } else if now_unix.saturating_sub(last) >= 60 {
                last_steps.insert(pid.clone(), now_unix);
                true
            } else {
                false
            }
        };

        if should_step {
            if step_offline_player_once(&pid, now_unix).is_some() {
                stepped_count += 1;
            }
        }
    }

    stepped_count
}

pub fn drain_player_offline_recap(
    player_id: &str,
    _now_unix: i64,
) -> Vec<(String, String, String)> {
    let mut recaps = PLAYER_OFFLINE_RECAPS
        .write()
        .unwrap_or_else(|e| e.into_inner());
    let mut list = recaps.remove(player_id).unwrap_or_default();
    if list.len() > 4 {
        list.truncate(4);
    }
    list
}

/// 田壟與藥圃生長階段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CropGrowthStage {
    Seedling, // "幼苗青翠"
    Growing,  // "抽穗飽滿"
    Mature,   // "熟成待收"
}

/// 動態田壟作物即時狀態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CropPlotState {
    pub coord: (i32, i32),
    pub field_kind: String, // "water_field" | "dry_field" | "herb_garden"
    pub crop_item_id: String,
    pub crop_name: String,
    pub sown_at_unix: i64,
    pub stage: CropGrowthStage,
    pub water_nourished: bool,
}

pub static LIVE_CROP_PLOTS: LazyLock<RwLock<HashMap<(i32, i32), CropPlotState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub fn register_crop_plot_sown(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> CropPlotState {
    let (crop_item_id, crop_name) = match field_kind {
        "water_field" => ("mat_grain_sack", "大圳頭飽滿新米"),
        "dry_field" => ("mat_pasture_seed_sack", "旱壟耐旱雜糧穗"),
        "herb_garden" => ("harvest_herb", "分畦清香靈藥草"),
        _ => ("mat_grain_sack", "大圳頭飽滿新米"),
    };
    let state = CropPlotState {
        coord: (x, y),
        field_kind: field_kind.to_string(),
        crop_item_id: crop_item_id.to_string(),
        crop_name: crop_name.to_string(),
        sown_at_unix: now_unix,
        stage: CropGrowthStage::Seedling,
        water_nourished: field_kind == "water_field",
    };
    let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
    plots.insert((x, y), state.clone());
    state
}

pub fn inspect_or_advance_crop_plot(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> CropPlotState {
    let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
    let entry = plots.entry((x, y)).or_insert_with(|| {
        let (crop_item_id, crop_name) = match field_kind {
            "water_field" => ("mat_grain_sack", "大圳頭飽滿新米"),
            "dry_field" => ("mat_pasture_seed_sack", "旱壟耐旱雜糧穗"),
            "herb_garden" => ("harvest_herb", "分畦清香靈藥草"),
            _ => ("mat_grain_sack", "大圳頭飽滿新米"),
        };
        CropPlotState {
            coord: (x, y),
            field_kind: field_kind.to_string(),
            crop_item_id: crop_item_id.to_string(),
            crop_name: crop_name.to_string(),
            sown_at_unix: now_unix.saturating_sub(120),
            stage: CropGrowthStage::Mature,
            water_nourished: true,
        }
    });

    if entry.stage != CropGrowthStage::Mature {
        let elapsed = now_unix.saturating_sub(entry.sown_at_unix);
        let speed_factor = if entry.water_nourished { 2 } else { 1 };
        let effective_elapsed = elapsed.saturating_mul(speed_factor);
        if effective_elapsed >= 90 {
            entry.stage = CropGrowthStage::Mature;
        } else if effective_elapsed >= 45 {
            entry.stage = CropGrowthStage::Growing;
        } else {
            entry.stage = CropGrowthStage::Seedling;
        }
    }

    entry.clone()
}

pub fn harvest_crop_plot_once(
    x: i32,
    y: i32,
    field_kind: &str,
    now_unix: i64,
) -> Result<(String, String, i32, String), String> {
    let current = inspect_or_advance_crop_plot(x, y, field_kind, now_unix);
    if current.stage == CropGrowthStage::Mature {
        let mut plots = LIVE_CROP_PLOTS.write().unwrap_or_else(|e| e.into_inner());
        plots.remove(&(x, y));
        let qty = 2;
        let prose = format!(
            "你俯身細心收割已然熟成的「{}」，沉甸甸的碩果整齊捆紮收入行囊，田地重歸齊整待耕。",
            current.crop_name
        );
        Ok((current.crop_item_id, current.crop_name, qty, prose))
    } else {
        let not_ready_prose = match current.stage {
            CropGrowthStage::Seedling => format!(
                "田壟間的「{}」正展露青翠幼苗，根系尚嫩，需待日光照耀與甘霖潤澤才能抽穗成熟。",
                current.crop_name
            ),
            CropGrowthStage::Growing => format!(
                "田壟間的「{}」正處抽穗拔節之時，籽粒漸充，尚待時日始至全熟收成。",
                current.crop_name
            ),
            CropGrowthStage::Mature => unreachable!(),
        };
        Err(not_ready_prose)
    }
}

/// 全島玩家票號存摺即時帳本
pub static LIVE_PLAYER_BANK_LEDGER: LazyLock<RwLock<HashMap<String, i32>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 全島玩家家宅玉石櫃存放物產清單（每位玩家至多 24 件）
pub static LIVE_PLAYER_HOME_STASH: LazyLock<RwLock<HashMap<String, Vec<(String, String)>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 全島親善手札竹筒信箱領取時間窗口記錄（每 300 秒可領取一回親善信與隨函薄禮）
pub static LIVE_ACQUAINTANCE_MAILBOX: LazyLock<RwLock<HashMap<String, i64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 查詢指定獵場分區在給定時間刻度下之物候天色
pub fn zone_weather_phase(z_idx: usize, now_unix: i64) -> crate::game::ZoneWeatherPhase {
    crate::game::current_zone_weather_and_tide(z_idx, now_unix / 180).0
}

/// 查詢指定獵場分區在給定時間刻度下之海潮走勢
pub fn zone_tide_phase(z_idx: usize, now_unix: i64) -> crate::game::ZoneTidePhase {
    crate::game::current_zone_weather_and_tide(z_idx, now_unix / 180).1
}

/// 依聚落或場所名稱解析其所屬或鄰近之正典獵場分區編號
pub fn settlement_to_zone_idx(settlement_or_room: &str) -> usize {
    crate::game::resolve_zone_idx_from_name_or_settlement(settlement_or_room)
}

/// 檢查指定分區是否有飢餓猛獸受擾越界下山
pub fn is_zone_predator_spillover_active(z_idx: usize, _now_unix: i64) -> bool {
    crate::game::is_zone_predator_spillover_active(z_idx)
}

/// 查詢玩家在全島票號的存摺餘額（鎂幣）
pub fn get_player_bank_balance(player_id: &str) -> i32 {
    let ledger = LIVE_PLAYER_BANK_LEDGER.read().unwrap_or_else(|e| e.into_inner());
    ledger.get(player_id).copied().unwrap_or(0)
}

/// 存入鎂幣至玩家票號存摺
///
/// 傳回 `(實際存入額, 身上剩餘額, 票號新餘額)`
pub fn deposit_player_bank_magnesium(player_id: &str, requested_mg: i32) -> (i32, i32, i32) {
    let Some(arc) = crate::store::get_store() else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    let mut store = arc.write();
    let Some(ent) = store.entities.get_mut(player_id) else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    if !(ent.kind == "player" || ent.kind == "Player" || ent.kind == "npc") {
        return (0, 0, get_player_bank_balance(player_id));
    }
    let actual = requested_mg.max(0).min(ent.magnesium.max(0));
    if actual > 0 {
        ent.magnesium = ent.magnesium.saturating_sub(actual);
        let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let balance = ledger.entry(player_id.to_string()).or_insert(0);
        *balance = balance.saturating_add(actual);
        (actual, ent.magnesium, *balance)
    } else {
        let cur_bank = {
            let ledger = LIVE_PLAYER_BANK_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.get(player_id).copied().unwrap_or(0)
        };
        (0, ent.magnesium, cur_bank)
    }
}

/// 從玩家票號存摺支取鎂幣
///
/// 傳回 `(實際支取額, 身上新餘額, 票號剩餘額)`
pub fn withdraw_player_bank_magnesium(player_id: &str, requested_mg: i32) -> (i32, i32, i32) {
    let Some(arc) = crate::store::get_store() else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    let mut store = arc.write();
    let Some(ent) = store.entities.get_mut(player_id) else {
        return (0, 0, get_player_bank_balance(player_id));
    };
    if !(ent.kind == "player" || ent.kind == "Player" || ent.kind == "npc") {
        return (0, 0, get_player_bank_balance(player_id));
    }
    let mut ledger = LIVE_PLAYER_BANK_LEDGER.write().unwrap_or_else(|e| e.into_inner());
    let balance = ledger.entry(player_id.to_string()).or_insert(0);
    let actual = requested_mg.max(0).min((*balance).max(0));
    if actual > 0 {
        *balance = balance.saturating_sub(actual);
        ent.magnesium = ent.magnesium.saturating_add(actual);
    }
    (actual, ent.magnesium, *balance)
}

/// 列出玩家家宅玉石櫃中當前存放的物產
pub fn list_player_home_stash(player_id: &str) -> Vec<(String, String)> {
    let stash = LIVE_PLAYER_HOME_STASH.read().unwrap_or_else(|e| e.into_inner());
    stash.get(player_id).cloned().unwrap_or_default()
}

/// 存放物產至玩家家宅玉石櫃（每位玩家上限 24 件）
pub fn store_item_in_home_stash(
    player_id: &str,
    item_id: &str,
    item_name: &str,
) -> Result<usize, String> {
    let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
    let list = stash.entry(player_id.to_string()).or_default();
    if list.len() >= 24 {
        return Err("家宅玉石櫃存放物產已達二十四件上限，無法再行存入。".to_string());
    }
    list.push((item_id.to_string(), item_name.to_string()));
    Ok(list.len())
}

/// 從玩家家宅玉石櫃取出最後存入的一件物產
pub fn retrieve_item_from_home_stash(player_id: &str) -> Option<(String, String, usize)> {
    let mut stash = LIVE_PLAYER_HOME_STASH.write().unwrap_or_else(|e| e.into_inner());
    let list = stash.get_mut(player_id)?;
    let (item_id, item_name) = list.pop()?;
    Some((item_id, item_name, list.len()))
}

/// 動態彙編全島即時《半島旬報》（四段式正典風貌，零生硬術語）
pub fn compile_live_peninsula_gazette(room_name: &str, now_unix: i64) -> String {
    // 一、【四鎮商貿與緊缺行情】
    const GAZETTE_SETTLEMENTS: &[&str] = &[
        "文曜", "風舶角", "半嶺鎮", "大圳頭", "南哨", "水尾寮", "老堰頭",
    ];
    let mut scarcity_list = Vec::new();
    for &s in GAZETTE_SETTLEMENTS {
        if let Some((item_id, item_name, cur_stock, target_stock, price_mg)) =
            crate::game::object_ecology::scarcest_item_for_settlement(s)
        {
            let gap = (target_stock - cur_stock).max(0);
            let ratio = gap as f32 / target_stock.max(1) as f32;
            scarcity_list.push((ratio, s, item_id, item_name, cur_stock, target_stock, price_mg));
        }
    }
    scarcity_list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let positive_scarcity: Vec<_> = scarcity_list.into_iter().filter(|x| x.0 > 0.001).collect();
    let s1_body = if positive_scarcity.is_empty() {
        "全島各聚落物料流轉平穩充盈，各大市集暫無重大緊缺物資，工坊日課常態運轉。".to_string()
    } else {
        let mut lines = Vec::new();
        for (ratio, s, _id, name, cur, tgt, pr) in positive_scarcity.into_iter().take(3) {
            let pct = (ratio * 100.0).round() as i32;
            lines.push(format!(
                "・「{}」商市急缺「{}」：現存 {} 份（目標需求 {} 份，缺口約 {}%），坊間公道收購價 {} 鎂。",
                s, name, cur, tgt, pct, pr
            ));
        }
        lines.join("\n")
    };

    // 二、【山海聯運與受阻急貨】
    let s2_body = {
        let orders = LIVE_STRANDED_CARGO_ORDERS.read().unwrap_or_else(|e| e.into_inner());
        let mut active: Vec<_> = orders
            .values()
            .filter(|o| o.escrow_bounty_mg > 0 && now_unix <= o.expires_at_unix)
            .collect();
        active.sort_by_key(|o| o.expires_at_unix);
        if active.is_empty() {
            "目前全島陸路商道與沿海舟楫往來順暢，各處驛站與渡頭暫無受阻滯留貨單。".to_string()
        } else {
            let mut lines = Vec::new();
            for o in active.into_iter().take(3) {
                lines.push(format!(
                    "・行商「{}」受阻滯留於「{}」：急需送達「{}」之「{}」，公庫託管賞格 {} 鎂。",
                    o.carrier_name, o.waystation_name, o.dest_settlement, o.item_name, o.escrow_bounty_mg
                ));
            }
            lines.join("\n")
        }
    };

    // 三、【山海物候與獵場警訊】
    let z_idx = crate::game::resolve_zone_idx_from_name_or_settlement(room_name);
    let time_tick = now_unix / 180;
    let (weather, tide) = crate::game::current_zone_weather_and_tide(z_idx, time_tick);
    let weather_str = match weather {
        crate::game::ZoneWeatherPhase::RainMist => "微雨薄霧，山林潤濕",
        crate::game::ZoneWeatherPhase::ClearBreeze => "晴和天青，清風舒爽",
        crate::game::ZoneWeatherPhase::DryWarm => "日暖乾燥，山道朗朗",
        crate::game::ZoneWeatherPhase::ColdFrost => "霜寒料峭，林霧凝冷",
    };
    let tide_str = match tide {
        crate::game::ZoneTidePhase::FloodTide => "漲潮，海浪拍岸湧潮滿水",
        crate::game::ZoneTidePhase::EbbTide => "退潮，沿海礁石潮池露灘",
        crate::game::ZoneTidePhase::SlackTide => "潮平水穩，水面浪靜",
    };
    let local_zone_name = crate::game::CANONICAL_HUNTING_ZONES
        .get(z_idx)
        .map(|(n, _, _)| *n)
        .unwrap_or("周邊獵場");
    let mut spillover_alerts = Vec::new();
    for z in 0..=5 {
        if crate::game::is_zone_predator_spillover_active(z) {
            let zn = crate::game::CANONICAL_HUNTING_ZONES.get(z).map(|(n, _, _)| *n).unwrap_or("山林獵場");
            spillover_alerts.push(format!("・【{}】猛獸受驚越界伏行，山道行旅受阻，請往來商隊與巡行者多加戒備！", zn));
        }
    }
    let spillover_section = if spillover_alerts.is_empty() {
        "各主要關隘與山道巡防井然，暫無猛獸越界侵擾通報。".to_string()
    } else {
        spillover_alerts.join("\n")
    };
    let s3_body = format!(
        "・所在水土物候（{}）：氣候呈「{}」，潮汐呈「{}」。\n{}",
        local_zone_name, weather_str, tide_str, spillover_section
    );

    // 四、【街坊新近見聞】
    let mut seen_facts = std::collections::HashSet::new();
    let mut ripples = Vec::new();
    {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        for state in states.values() {
            for rip in &state.active_ripples {
                if seen_facts.insert(rip.fact_id.clone()) {
                    let salience = rip.severity * (0.65_f32).powi(rip.hop_count as i32);
                    ripples.push((salience, rip.prose_summary.clone()));
                }
            }
        }
    }
    ripples.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let ripple_lines: Vec<String> = ripples.into_iter().take(2).map(|(_, s)| format!("・{}", s)).collect();
    let ripple_text = if ripple_lines.is_empty() {
        "・坊間鄰里生活安穩，近期未有重大異動風聲。".to_string()
    } else {
        ripple_lines.join("\n")
    };

    let script_snippet = if let Some((ev, sc)) =
        crate::game::peninsula_scripts::active_event_and_script_pack_for_room(room_name, None, now_unix)
    {
        format!(
            "・街坊風貌【{}】：{}（劇本推進：{}）",
            ev.event_title, ev.trigger_and_causal_chain, sc.outcome_and_world_change
        )
    } else {
        "・街坊風貌：街坊作坊按部就班，鄰里互通有無，各自照料門前營生。".to_string()
    };
    let s4_body = format!("{}\n{}", ripple_text, script_snippet);

    format!(
        "《半島旬報・街坊公議與山海物候》\n\n一、【四鎮商貿與緊缺行情】\n{}\n\n二、【山海聯運與受阻急貨】\n{}\n\n三、【山海物候與獵場警訊】\n{}\n\n四、【街坊新近見聞】\n{}",
        s1_body, s2_body, s3_body, s4_body
    )
}

/// 領取竹筒信箱親善信札與隨信回禮
///
/// 傳回 `(手札內文, Option<(回禮物料ID, 回禮物料名稱)>)`
pub fn claim_acquaintance_mailbox_letter(
    player_id: &str,
    player_name: &str,
    now_unix: i64,
) -> (String, Option<(String, &'static str)>) {
    let mut found_acquaintance = None;
    {
        let states = LIVE_NPC_CAUSAL_STATES.read().unwrap_or_else(|e| e.into_inner());
        for (npc_id, state) in states.iter() {
            for h in &state.hooks {
                let matches_player = (!player_id.is_empty() && h.target_id == player_id)
                    || (!player_name.is_empty() && (h.target_name == player_name || player_name.contains(&h.target_name)));
                let is_positive = !h.kind_label.contains("怨")
                    && (h.intensity >= 10
                        || h.kind_label.contains("恩")
                        || h.kind_label.contains("師")
                        || h.kind_label.contains("契")
                        || h.kind_label.contains("信")
                        || h.kind_label.contains("同行"));
                if matches_player && is_positive {
                    found_acquaintance = Some((npc_id.clone(), h.kind_label.clone(), h.summary.clone()));
                    break;
                }
            }
            if found_acquaintance.is_some() {
                break;
            }
        }
    }

    let (sender_name, summary) = if let Some((npc_id, kind_label, hook_summary)) = found_acquaintance {
        let name = if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            s.entities.get(&npc_id).map(|e| {
                if !e.display_title.is_empty() {
                    e.display_title.clone()
                } else {
                    npc_id.clone()
                }
            }).unwrap_or(npc_id)
        } else {
            npc_id
        };
        let sum = if hook_summary.is_empty() {
            format!("念及昔日「{}」之誼", kind_label)
        } else {
            hook_summary
        };
        (name, sum)
    } else {
        let imprints = crate::pg::living_world::get_entity_lineage_imprints(player_id);
        if let Some(imp) = imprints.first() {
            (
                imp.mentor_name.clone(),
                format!("記掛授藝指點「{}」之傳承", imp.imprint_title),
            )
        } else {
            ("三水口信使總站・韓小駒".to_string(), "街坊安居問候".to_string())
        }
    };

    let letter_text = format!(
        "【竹筒信箱・親善手札】\n展信悅。見字如晤，寄信人：「{}」。\n信中言道：「{}，近來見白塔半島風物調勻，特修書一封問候，盼安居順遂、道途平穩。」",
        sender_name, summary
    );

    let window = now_unix / 300;
    let mut mailbox = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
    let unread_prefix = format!("{player_id}:unread:");
    let unread_keys: Vec<String> = mailbox
        .keys()
        .filter(|k| k.starts_with(&unread_prefix) && mailbox.get(*k).copied().unwrap_or(0) > 0)
        .cloned()
        .collect();
    for k in unread_keys {
        if let Some(cnt) = mailbox.get_mut(&k) {
            if *cnt > 0 {
                *cnt -= 1;
                break;
            }
        }
    }
    let last_window = mailbox.get(player_id).copied();
    if last_window != Some(window) {
        mailbox.insert(player_id.to_string(), window);
        const ROTATING_GIFTS: &[(&str, &'static str)] = &[
            ("wt_cat_0501", "穩火慢煮牛埔鮮奶茶"),
            ("wt_cat_0500", "東橋茶屋山風草配霜林烘青茶"),
            ("wt_cat_0723", "旋蓋夜光貝母調光台燈"),
        ];
        let gift_idx = (window.unsigned_abs() as usize) % ROTATING_GIFTS.len();
        let (gift_id, gift_name) = ROTATING_GIFTS[gift_idx];
        (letter_text, Some((gift_id.to_string(), gift_name)))
    } else {
        (letter_text, None)
    }
}

/// 記錄或更新特定住民對玩家之親善同行關係，並在信箱排入親善問候手札
pub fn record_player_npc_fellowship_hook(
    npc_id: &str,
    player_id: &str,
    summary: &str,
    now_unix: i64,
) {
    let player_name = if let Some(arc) = crate::store::get_store() {
        let s = arc.read();
        s.entities
            .get(player_id)
            .map(|e| {
                if !e.display_title.trim().is_empty() {
                    e.display_title.clone()
                } else {
                    player_id.to_string()
                }
            })
            .unwrap_or_else(|| player_id.to_string())
    } else {
        player_id.to_string()
    };

    {
        let mut states = LIVE_NPC_CAUSAL_STATES.write().unwrap_or_else(|e| e.into_inner());
        let state = states.entry(npc_id.to_string()).or_insert_with(|| {
            NpcCausalState {
                cornerstone: Cornerstone::Honor,
                stress: 10,
                stress_break_active: false,
                hunger: 10,
                thirst: 10,
                fatigue: 10,
                plan_steps: [
                    "巡視工坊物料與庫存".to_string(),
                    "手談切磋與街坊敘舊".to_string(),
                    "回房整理行囊帳目".to_string(),
                ],
                plan_cursor: 0,
                belief_scarcity_item: "常備物料".to_string(),
                last_surprise_delta: None,
                hooks: Vec::new(),
                life_step_tick: 1,
                same_task_repetitions: 1,
                carryover_from_yesterday: "昨日工序延續".to_string(),
                decision_image_code: "interact_or_unwind".to_string(),
                eligible_option_count: 3,
                chosen_action_code: "social_chat".to_string(),
                active_need: "街坊敘舊".to_string(),
                image_decision: "同席手談".to_string(),
                situated_action: "手談切磋".to_string(),
                belief_summary: "常業平順".to_string(),
                active_ripples: Vec::new(),
                immune_fact_ids: Vec::new(),
            }
        });
        let mut found = false;
        for h in &mut state.hooks {
            let matches = (!player_id.is_empty() && h.target_id == player_id)
                || (!player_name.is_empty() && (h.target_name == player_name || player_name.contains(&h.target_name)));
            if matches {
                h.kind_label = "結識同行".to_string();
                h.target_name.clone_from(&player_name);
                h.target_id = player_id.to_string();
                h.summary = summary.to_string();
                h.intensity = 75;
                h.updated_at_unix = now_unix;
                found = true;
                break;
            }
        }
        if !found {
            state.hooks.push(RelationalHook {
                kind_label: "結識同行".to_string(),
                target_name: player_name,
                summary: summary.to_string(),
                intensity: 75,
                target_id: player_id.to_string(),
                updated_at_unix: now_unix,
            });
            if state.hooks.len() > 8 {
                state.hooks.sort_by(|a, b| b.intensity.cmp(&a.intensity));
                state.hooks.truncate(8);
            }
        }
    }

    {
        let mut mailbox = LIVE_ACQUAINTANCE_MAILBOX.write().unwrap_or_else(|e| e.into_inner());
        let act_key = format!("{player_id}:unread:{summary}");
        let unread_count = mailbox.get(&act_key).copied().unwrap_or(0);
        if unread_count < 2 {
            mailbox.insert(act_key, unread_count + 1);
            // 讓玩家能在信箱立即領取此親善手札
            mailbox.remove(player_id);
        }
    }
}

/// 記錄聚落代表工坊之物料產出，向市集補充物料庫存並平抑物價波動
pub fn record_settlement_market_production(settlement: &str, item_id: &str, quantity: i32) {
    let clean_settlement = clean_settlement_name(settlement);
    let _ = crate::game::object_ecology::record_settlement_market_trade(
        clean_settlement,
        item_id,
        10,
        quantity,
        "聚落代表工坊產出",
    );
}


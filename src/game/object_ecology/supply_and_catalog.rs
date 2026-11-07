/// 住民執行勞作、採集、市集交割、工坊打造或跨區調撥時，真實增加聚落物資庫存（或自過剩聚落調撥）並平抑需求 EMA。
pub fn supply_settlement_commodity_from_work(
    settlement_or_room: &str,
    work_kind: &str,
    actor_name: &str,
) -> Option<(String, i32, f32)> {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());

    if work_kind == "arbitrage" {
        let candidate_arbitrage_items: [&'static str; 10] = [
            "mat_rough_ore",
            "mat_iron_scrap",
            "mat_charcoal_sack",
            "mat_timber_log",
            "mat_plank_board",
            "mat_reed_bundle",
            "mat_foraged_herb",
            "mat_coarse_salt_pack",
            "craft_meridian_salve",
            "craft_tool_repair_strip",
        ];

        for &item_id in &candidate_arbitrage_items {
            let key = (settlement.to_string(), item_id.to_string());
            if !ledger.settlement_markets.contains_key(&key) {
                let cell = init_settlement_market_cell(settlement, item_id);
                ledger.settlement_markets.insert(key, cell);
            }
        }

        let mut chosen_item = candidate_arbitrage_items[0];
        let mut min_diff = i32::MAX;
        for &item_id in &candidate_arbitrage_items {
            let key = (settlement.to_string(), item_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get(&key) {
                let diff = cell.stock - cell.target_stock;
                if diff < min_diff {
                    min_diff = diff;
                    chosen_item = item_id;
                }
            }
        }

        let mut from_s = "文曜";
        let mut best_from_surplus = i32::MIN;
        for &s in &CANONICAL_SETTLEMENTS {
            if s == settlement {
                continue;
            }
            let from_key = (s.to_string(), chosen_item.to_string());
            if !ledger.settlement_markets.contains_key(&from_key) {
                let cell = init_settlement_market_cell(s, chosen_item);
                ledger.settlement_markets.insert(from_key.clone(), cell);
            }
            if let Some(cell) = ledger.settlement_markets.get(&from_key) {
                let surplus = cell.stock - cell.target_stock;
                if surplus > best_from_surplus {
                    best_from_surplus = surplus;
                    from_s = s;
                }
            }
        }

        let dest_key = (settlement.to_string(), chosen_item.to_string());
        let ratio_before = if let Some(cell) = ledger.settlement_markets.get(&dest_key) {
            ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                * cell.demand_ema.clamp(0.75, 1.45)
        } else {
            1.0
        };

        let from_key = (from_s.to_string(), chosen_item.to_string());
        let mut transferred = 0;
        if let Some(from_cell) = ledger.settlement_markets.get_mut(&from_key) {
            transferred = from_cell.stock.min(2).max(0);
            from_cell.stock -= transferred;
            from_cell.demand_ema = (from_cell.demand_ema * 1.03).clamp(0.75, 1.45);
        }

        if transferred <= 0 {
            return None;
        }
        let add_qty = transferred;
        let mut new_stock = 0;
        if let Some(to_cell) = ledger.settlement_markets.get_mut(&dest_key) {
            to_cell.stock = (to_cell.stock + add_qty).min(to_cell.target_stock * 4);
            to_cell.demand_ema = (to_cell.demand_ema * 0.94).clamp(0.75, 1.45);
            let ratio_after = ((to_cell.target_stock.max(1) as f32 + 2.0) / (to_cell.stock.max(0) as f32 + 2.0)).powf(0.68) * to_cell.demand_ema;
            let base_p = item_base_price_mg(chosen_item) as f32;
            to_cell.last_price_mg = ((base_p * ratio_after.clamp(0.45, 2.60)).round() as i32).max(1);
            new_stock = to_cell.stock;
        }

        let clean_name = get_clean_item_name(chosen_item);
        let clean_actor = actor_name.trim().replace(['|', '｜'], "・");
        let actor_label = if clean_actor.is_empty() { "行商" } else { clean_actor.as_str() };
        let event = format!(
            "{actor_label}將一批「{clean_name}」自{from_s}運抵{settlement}市集交割，稍解坊間燃眉之急。"
        );
        ledger.recent_events.push_back(event);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
        // 由模擬迴圈背景統一持久化帳本快照

        Some((clean_name, new_stock, ratio_before.clamp(0.65, 2.20)))
    } else {
        let candidates: &'static [&'static str] = if work_kind == "forage" || work_kind == "peddle" {
            &["mat_kindling_bundle", "mat_foraged_herb", "mat_reed_bundle", "mat_rough_ore"]
        } else {
            &["craft_tool_repair_strip", "craft_meridian_salve", "craft_herbal_tea_pack", "mat_plank_board"]
        };

        for &item_id in candidates {
            let key = (settlement.to_string(), item_id.to_string());
            if !ledger.settlement_markets.contains_key(&key) {
                let cell = init_settlement_market_cell(settlement, item_id);
                ledger.settlement_markets.insert(key, cell);
            }
        }

        let mut chosen_item = candidates[0];
        let mut highest_scarcity = -1.0f32;
        for &item_id in candidates {
            let key = (settlement.to_string(), item_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get(&key) {
                let r = ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                    * cell.demand_ema.clamp(0.75, 1.45);
                if r > highest_scarcity {
                    highest_scarcity = r;
                    chosen_item = item_id;
                }
            }
        }

        let key = (settlement.to_string(), chosen_item.to_string());
        let ratio_before = if let Some(cell) = ledger.settlement_markets.get(&key) {
            ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                * cell.demand_ema.clamp(0.75, 1.45)
        } else {
            1.0
        };

        if work_kind == "craft" || work_kind == "restock" {
            let raw_candidates: &[&str] = if chosen_item.contains("herb")
                || chosen_item.contains("salve")
                || chosen_item.contains("tea")
                || chosen_item.contains("藥")
                || chosen_item.contains("草")
                || chosen_item.contains("膏")
                || chosen_item.contains("茶")
            {
                &["mat_foraged_herb"]
            } else if chosen_item.contains("plank")
                || chosen_item.contains("timber")
                || chosen_item.contains("wood")
                || chosen_item.contains("paper")
                || chosen_item.contains("木")
                || chosen_item.contains("板")
                || chosen_item.contains("紙")
            {
                &["mat_timber_log"]
            } else {
                &["mat_rough_ore", "mat_iron_scrap"]
            };

            for &raw_id in raw_candidates {
                let raw_key = (settlement.to_string(), raw_id.to_string());
                if let Some(raw_cell) = ledger.settlement_markets.get_mut(&raw_key) {
                    if raw_cell.stock > 1 {
                        raw_cell.stock -= 1;
                        if let Some(entry) = ledger.items.get_mut(raw_id) {
                            if entry.stock > 0 {
                                entry.stock -= 1;
                            }
                        }
                        break;
                    }
                }
            }
        }

        let clean_name = get_clean_item_name(chosen_item);
        let res_kind = if ["木", "樟", "杉", "柴", "炭", "竹", "藤"]
            .iter()
            .any(|&k| chosen_item.contains(k) || clean_name.contains(k))
        {
            "timber"
        } else if ["礦", "鐵", "銅", "煤", "石", "晶", "砂", "鹽", "土"]
            .iter()
            .any(|&k| chosen_item.contains(k) || clean_name.contains(k))
        {
            "mineral"
        } else if ["魚", "蝦", "蟹", "貝", "鱟", "藻", "蘆", "水"]
            .iter()
            .any(|&k| chosen_item.contains(k) || clean_name.contains(k))
        {
            "fishery"
        } else {
            "flora"
        };

        let mut add_qty = if work_kind == "peddle" || work_kind == "craft" { 2 } else { 1 };
        let mut is_fallow_active = false;
        if work_kind == "forage" || work_kind == "peddle" {
            let (harvested, is_abundant, is_fallow) =
                crate::game::harvest_zone_natural_resource(settlement, res_kind, 3.5);
            if res_kind == "flora" {
                let _ = crate::game::harvest_hunting_zone_flora(settlement, 2.0);
            }
            if is_fallow {
                is_fallow_active = true;
                add_qty = 1;
            } else if is_abundant || harvested >= 3.0 {
                add_qty += 1;
            }
        }
        let mut new_stock = 0;
        if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
            cell.stock = (cell.stock + add_qty).min(cell.target_stock * 4);
            cell.demand_ema = (cell.demand_ema * 0.95).clamp(0.75, 1.45);
            let ratio_after = ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68) * cell.demand_ema;
            let base_p = item_base_price_mg(chosen_item) as f32;
            cell.last_price_mg = ((base_p * ratio_after.clamp(0.45, 2.60)).round() as i32).max(1);
            new_stock = cell.stock;
        }

        if let Some(entry) = ledger.items.get_mut(chosen_item) {
            entry.stock = (entry.stock + add_qty).min(entry.target_stock * 4);
        }

        let action_verb = match work_kind {
            "forage" => "採獲",
            "peddle" => "行商運來",
            "craft" => "於工坊打造",
            "restock" => "批發補貨",
            _ => "物資產出",
        };
        let clean_actor = actor_name.trim().replace(['|', '｜'], "・");
        let actor_label = if clean_actor.is_empty() { "工匠" } else { clean_actor.as_str() };
        let event = if is_fallow_active {
            format!("{actor_label}見山林水土微乏，先於郊野護林育苗、修整採場，順道將整理所得的「{clean_name}」{add_qty} 份送入{settlement}市集交割。")
        } else {
            format!(
                "{actor_label}{action_verb}「{clean_name}」{add_qty} 份並送入{settlement}市集交割，攤頭貨源漸豐，行情趨於平穩。"
            )
        };
        ledger.recent_events.push_back(event);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
        // 由模擬迴圈背景統一持久化帳本快照

        Some((clean_name, new_stock, ratio_before.clamp(0.65, 2.20)))
    }
}

/// 匯總當前市場脈動摘要，供對話生成與世界觀引導。
pub fn market_pulse_summary_for_prompt() -> String {
    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
    let mut tight_items = Vec::new();

    for (id, st) in &ledger.items {
        let ratio = st.stock as f64 / st.target_stock.max(1) as f64;
        let name = get_clean_item_name(id);
        if ratio < 0.70 {
            tight_items.push(name);
        }
    }

    let mut pulse = String::from("白塔半島商市百工運轉熱絡，");
    if !tight_items.is_empty() {
        let items_str = tight_items.iter().take(2).cloned().collect::<Vec<_>>().join("與");
        pulse.push_str(&format!("目下【{items_str}】等料件供不應求、行情持續看漲；各大工坊日夜趕工，"));
    } else {
        pulse.push_str("坊間各色物料供應有序、工造平穩；");
    }

    if let Some(recent) = ledger.recent_events.back() {
        pulse.push_str(&format!("坊間最新動向：「{recent}」"));
    } else {
        pulse.push_str("各大工坊百工齊備，常有匠器在氣脈激盪下凝出異變神彩。");
    }

    pulse
}

/// 產生指定聚落的供需即時實況報告，標示充裕供給與緊缺需求。
pub fn format_settlement_supply_demand_report(settlement_or_room: &str) -> String {
    let raw_s = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let s = if raw_s.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_s)
    };
    let trimmed = settlement_or_room.trim();
    let display_s = match trimmed {
        "文曜" | "風舶角" | "風舶嶼" | "大圳頭" | "半嶺鎮" | "南哨" | "老堰頭"
        | "北嶺腳" | "霜林口" | "風櫃尾" | "烏石鼻" | "牛埔" | "紅土崁" | "三號坑口"
        | "水尾寮" | "下營" => trimmed,
        _ => s,
    };

    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());

    let mut surplus_cells: Vec<&SettlementMarketCell> = ledger
        .settlement_markets
        .values()
        .filter(|c| c.settlement == s && c.stock > c.target_stock)
        .collect();

    surplus_cells.sort_by(|a, b| {
        let diff_a = a.stock - a.target_stock;
        let diff_b = b.stock - b.target_stock;
        diff_b.cmp(&diff_a)
    });

    let surplus_list: Vec<String> = surplus_cells
        .iter()
        .take(3)
        .map(|c| {
            let name = get_clean_item_name(&c.item_id);
            format!("「{name}」")
        })
        .collect();

    let surplus_str = if surplus_list.is_empty() {
        "各色常備物料".to_string()
    } else {
        surplus_list.join("、")
    };

    let mut shortage_cells: Vec<&SettlementMarketCell> = ledger
        .settlement_markets
        .values()
        .filter(|c| c.settlement == s && c.stock < c.target_stock)
        .collect();

    shortage_cells.sort_by(|a, b| {
        let ratio_a = a.stock as f32 / a.target_stock.max(1) as f32;
        let ratio_b = b.stock as f32 / b.target_stock.max(1) as f32;
        ratio_a
            .partial_cmp(&ratio_b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (b.target_stock - b.stock).cmp(&(a.target_stock - a.stock)))
    });

    let shortage_list: Vec<String> = shortage_cells
        .iter()
        .take(3)
        .map(|c| {
            let name = get_clean_item_name(&c.item_id);
            format!("「{name}」")
        })
        .collect();

    if shortage_list.is_empty() {
        format!("{display_s}市集攤頭目下以{surplus_str}較為充裕，坊間百工儲備充足、交割平穩。")
    } else {
        let shortage_str = shortage_list.join("、");
        format!("{display_s}市集攤頭目下以{surplus_str}較為充裕，而{shortage_str}則頗為緊俏。")
    }
}

/// 取得指定聚落之即時市場供需概況摘要（即 format_settlement_supply_demand_report 之簡約別名）
pub fn get_settlement_market_summary(settlement_or_room: &str) -> String {
    format_settlement_supply_demand_report(settlement_or_room)
}

#[cfg(not(test))]
static SCARCEST_ITEM_CACHE: std::sync::LazyLock<
    parking_lot::RwLock<
        std::collections::HashMap<
            String,
            (std::time::Instant, Option<(String, String, i32, i32, i32)>),
        >,
    >,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

/// 查詢指定聚落最緊缺的物料（庫存低於目標且缺口比例最大者），供委託榜與 NPC 收購對話呼叫。
pub fn scarcest_item_for_settlement(
    settlement_or_room: &str,
) -> Option<(String, String, i32, i32, i32)> {
    let raw_s = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let s = if raw_s.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_s)
    };

    #[cfg(not(test))]
    {
        if let Some((ts, cached)) = SCARCEST_ITEM_CACHE.read().get(s) {
            if ts.elapsed() < std::time::Duration::from_millis(1500) {
                return cached.clone();
            }
        }
    }

    let top_tuple = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        let top = ledger
            .settlement_markets
            .values()
            .filter(|c| c.settlement == s)
            .min_by(|a, b| {
                let ratio_a = a.stock as f32 / a.target_stock.max(1) as f32;
                let ratio_b = b.stock as f32 / b.target_stock.max(1) as f32;
                ratio_a
                    .partial_cmp(&ratio_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| (b.target_stock - b.stock).cmp(&(a.target_stock - a.stock)))
            });
        top.map(|t| (t.item_id.clone(), t.stock, t.target_stock, t.last_price_mg))
    };

    let res = top_tuple.map(|(item_id, stock, target_stock, last_price_mg)| {
        let name = get_clean_item_name(&item_id);
        (item_id, name, stock, target_stock, last_price_mg)
    });

    #[cfg(not(test))]
    {
        SCARCEST_ITEM_CACHE
            .write()
            .insert(s.to_string(), (std::time::Instant::now(), res.clone()));
    }

    res
}

/// 原料映射至成品規格。
pub fn map_raw_to_finished(raw_id: &str) -> &'static FinishedGoodDef {
    let clean_name = get_clean_item_name(raw_id);
    let has = |kw: &str| raw_id.contains(kw) || clean_name.contains(kw);

    // 1. 水產蝦蟹與甲殼貝類（但排除純工藝用的貝殼袋）
    if !has("貝殼") && !has("shell_sack") && (has("shrimp") || has("蝦") || has("蟹") || has("鮑") || has("九孔") || has("蛤") || has("蜆") || has("螺") || has("海膽")) {
        &STANDARD_FINISHED_GOODS[12] // craft_roasted_shrimp_skewer
    } else if !has("魚骨") && !has("魚鱗") && !has("fish_bone") && !has("fish_scale") && !has("魚皮") && !has("fishing_line") && !has("漁線") && !has("魚線") && (has("fish") || has("魚") || has("鰻") || has("鯇") || has("苦花") || has("鬼頭刀") || has("土魠") || has("花枝") || has("烏賊")) {
        &STANDARD_FINISHED_GOODS[13] // craft_smoked_fish_fillet
    } else if has("charcoal") || has("白炭") || has("炭") {
        &STANDARD_FINISHED_GOODS[10] // craft_pine_torch
    } else if has("sealing_pitch") || has("松脂") || has("桐油") || has("樹脂") || has("漆") || has("蠟") {
        &STANDARD_FINISHED_GOODS[2] // craft_bamboo_oil_umbrella
    } else if has("stone_powder") || has("shell_sack") || has("貝殼") || has("石粉") || has("石") || has("砂") || has("岩") || has("土") || has("泥") || has("晶") || has("玉") || has("灰") {
        &STANDARD_FINISHED_GOODS[1] // craft_iron_pickaxe
    } else if has("moss") || has("青苔") || has("苔") || has("芝") || has("菌") || has("菇") || has("蓮") || has("參") || has("葛") || has("艾") || has("藻") || has("poultice") || has("敷料") {
        &STANDARD_FINISHED_GOODS[4] // craft_meridian_salve
    } else if has("meat") || has("肉") || has("熊掌") || has("鹿腱") || has("牛蛙") || has("老鱉") {
        &STANDARD_FINISHED_GOODS[14] // craft_spiced_beast_jerky
    } else if has("grain") || has("hardtack") || has("seed") || has("salt") || has("稻穀") || has("穀糧") || has("米") || has("麥") || has("麵") || has("乾餅") || has("草籽") || has("草料") || has("粗鹽") || has("黑糖") || has("蔭油") || has("菜脯") || has("漿果") || has("鮮果") || has("水芹") || has("醬") || has("醋") || has("椒") || has("薑") || has("蒜") || has("蔥") || has("蜜") {
        &STANDARD_FINISHED_GOODS[15] // craft_ration_grain_cake
    } else if has("leather") || has("hide") || has("pelt") || has("feather") || has("bone") || has("scale") || has("claw") || has("皮") || has("毛") || has("絨") || has("羽") || has("翎") || has("骨") || has("鱗") || has("麟") || has("角") || has("牙") || has("爪") {
        &STANDARD_FINISHED_GOODS[9] // craft_heavy_leather_boots
    } else if has("iron") || has("scrap") || has("metal") || has("鐵") || has("礦") || has("鋼") {
        if has("pick") || has("鎬") || has("礦") {
            &STANDARD_FINISHED_GOODS[1] // craft_iron_pickaxe
        } else {
            &STANDARD_FINISHED_GOODS[0] // craft_steel_hunting_knife
        }
    } else if has("timber") || has("reed") || has("wood") || has("竹") || has("木") || has("蘆") || has("柴") || has("板") || has("柄") {
        if has("kindling") || has("torch") || has("柴") {
            &STANDARD_FINISHED_GOODS[10] // craft_pine_torch
        } else if has("plank") || has("handle") || has("板") || has("柄") {
            &STANDARD_FINISHED_GOODS[11] // craft_wooden_tool_handle
        } else if has("box") || has("匣") || has("樟") {
            &STANDARD_FINISHED_GOODS[3] // craft_camphor_storage_box
        } else {
            &STANDARD_FINISHED_GOODS[2] // craft_bamboo_oil_umbrella
        }
    } else if has("herb") || has("草") || has("藥") || has("花") || has("茶") {
        if has("tea") || has("茶") {
            &STANDARD_FINISHED_GOODS[5] // craft_calming_tea_pack
        } else {
            &STANDARD_FINISHED_GOODS[4] // craft_meridian_salve
        }
    } else if has("paper") || has("ink") || has("紙") || has("墨") || has("筆") {
        if has("almanac") || has("書") || has("手札") {
            &STANDARD_FINISHED_GOODS[7] // craft_peninsula_almanac
        } else {
            &STANDARD_FINISHED_GOODS[6] // craft_pine_soot_ink
        }
    } else if has("rope") || has("sackcloth") || has("fishing_line") || has("繩") || has("布") || has("帆") || has("線") {
        &STANDARD_FINISHED_GOODS[8] // craft_storm_canvas_cloak
    } else {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::hash::DefaultHasher::new();
        raw_id.hash(&mut hasher);
        let idx = (hasher.finish() as usize) % STANDARD_FINISHED_GOODS.len();
        &STANDARD_FINISHED_GOODS[idx]
    }
}
pub fn lookup_item_origin(item_id: &str) -> String {
    if item_id.starts_with("crystal_") {
        return "白塔半島・天地五源".to_string();
    }
    for g in STANDARD_FINISHED_GOODS {
        if g.id == item_id {
            return g.origin.to_string();
        }
    }
    for arch in MUTATION_ARCHETYPES {
        let prefix = format!("mut_{}_", arch.key);
        if let Some(base_id) = item_id.strip_prefix(&prefix) {
            for g in STANDARD_FINISHED_GOODS {
                if g.id == base_id {
                    return g.origin.to_string();
                }
            }
        }
    }
    match item_id {
        "wt_beast_bone" => "白塔半島・山海獵場".to_string(),
        "wt_down_feather" => "白塔半島・水尾澤岸".to_string(),
        "mat_iron_scrap" => "文曜萬爐鐵坊棚".to_string(),
        "mat_repair_strip" => "分家巷機巧修車棚".to_string(),
        "mat_timber_log" => "霜林口林場木棧".to_string(),
        "mat_reed_bundle" => "水尾寮河口與老蘆叢".to_string(),
        "mat_foraged_herb" => "半嶺清泉溪谷".to_string(),
        "mat_coarse_paper_ream" => "文曜東市商行街".to_string(),
        "mat_hemp_rope_roll" => "水尾寮渡口與漚麻坊".to_string(),
        "mat_kindling_bundle" => "霜林口農舍前坪".to_string(),
        "mat_plank_board" => "霜林口梁氏鋸木坊".to_string(),
        "mat_charcoal_sack" => "霜林口相思木悶炭窯".to_string(),
        "mat_sackcloth_bolt" => "水尾寮渡頭市集與倉棚".to_string(),
        "mat_wood_offcut_bundle" => "霜林口梁氏鋸木坊".to_string(),
        "mat_rough_ore" => "三號坑口".to_string(),
        "mat_washed_ore" => "半嶺溪谷洗礦溝".to_string(),
        "mat_stone_powder_sack" => "紅土崁高嶺土倉".to_string(),
        "mat_wet_grass_bundle" => "牛埔長風草甸".to_string(),
        "mat_rice_straw_bundle" => "大圳頭良田路口".to_string(),
        "mat_water_moss_sheet" => "水尾寮灌渠木閘".to_string(),
        "mat_rice_husk_sack" => "大圳頭穀場小棚".to_string(),
        "mat_wrapping_paper_pack" => "文曜東市商行街".to_string(),
        "mat_hay_bundle" => "牛埔車馬泊位".to_string(),
        "mat_fishing_line_bundle" => "黑石灣漁網曬場".to_string(),
        "mat_fish_scale_sack" => "風舶角鮮魚拍賣場".to_string(),
        "mat_bamboo_slats" => "水尾寮竹筏棧".to_string(),
        "mat_fish_bone_sack" => "烏石鼻魚貨分揀場".to_string(),
        "mat_coarse_salt_pack" => "落日岬日曬鹽坪".to_string(),
        "mat_grain_sack" => "大圳頭糧倉前場".to_string(),
        "mat_metal_rim_strip" => "分家巷機巧修車棚".to_string(),
        "mat_oilcloth_roll" => "風舶角油布庫".to_string(),
        "mat_pasture_seed_sack" => "牛埔牧地柵門".to_string(),
        "mat_mane_hair_bundle" => "牛埔馬欄前場".to_string(),
        "mat_feed_mix_sack" => "牛埔草料棚".to_string(),
        "mat_leather_strip" => "牛埔鞣皮修棚".to_string(),
        "mat_canvas_roll" => "風舶角車隊場".to_string(),
        "mat_shell_sack" => "烏石鼻潮泥碼頭".to_string(),
        "mat_hardtack_sack" => "南哨高棧補給市".to_string(),
        "mat_sealing_pitch" => "霜林口松脂作業點".to_string(),
        "mat_salted_fish" => "水尾寮魚乾架".to_string(),
        "craft_herbal_tea_pack" => "半嶺茶坊".to_string(),
        "craft_tool_repair_strip" => "文曜萬爐鐵坊棚".to_string(),
        "craft_patched_work_boots" => "半嶺皮匠鋪".to_string(),
        "craft_tarred_hemp_rope" => "風舶角麻繩作".to_string(),
        "craft_caulking_sealant" => "風舶角船塢".to_string(),
        "mat_cedar_plank" => "霜林口林場木棧".to_string(),
        _ => {
            if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
                return format!("白塔半島・{}", cat.workshop);
            }
            if let Some(lbl) = crate::world::materials::material_origin_label(item_id) {
                return lbl.to_string();
            }
            String::new()
        }
    }
}

pub fn ensure_item_in_store(item: Item) {
    if let Some(arc) = store::get_store() {
        let _ = arc.write().put_item(item);
    }
}

pub fn ensure_ecology_items_in_store() {
    let raw_defs = [
        ("mat_iron_scrap", "回收熟鐵料塊", "自高爐鍛坊或鹽鐵倉清整所得的成色純良熟鐵料塊，可於鍛台或百工面板煉製精鋼器具。", 1.5),
        ("mat_repair_strip", "修補熟鐵薄片", "輕便平整的修補用熟鐵薄片，可用於修整加固各式器具設施。", 0.6),
        ("mat_timber_log", "伐採原木", "紋理細密堅實的乾燥良木，可於木作台或百工面板車製行旅長杖與木器。", 2.0),
        ("mat_reed_bundle", "水澤老蘆草綑", "自河岸水澤收割晾乾的柔韌蘆草與青竹篾，可編織提籃或防風雨披。", 0.8),
        ("mat_foraged_herb", "山野鮮藥草", "帶著晨露清香的鮮嫩草藥，可焙製清心安神藥茶包或熬煉生肌膏。", 0.3),
        ("mat_coarse_paper_ream", "原漿宣紙刀", "青竹抄紙槽濾出的純淨宣紙原漿紙刀，可裝訂採風手札或拓印長卷。", 0.5),
        ("mat_hemp_rope_roll", "浸油粗麻繩卷", "經桐油浸泡絞合的強韌苧麻繩卷，可編製攀岩繩或加固遠行麻布囊。", 1.0),
        ("mat_sackcloth_bolt", "粗織麻袋布匹", "粗織結實的麻布匹料，可用於縫製耐磨行囊與防風遮棚。", 1.2),
        ("mat_kindling_bundle", "乾燥柴枝束", "自山林拾撿劈鋸而成的乾燥細柴枝束，引火極佳，為野地生火、工爐添薪與營火保溫之良材。", 0.5),
        ("mat_plank_board", "樟松木板料", "經木作台鋸切刨平的樟松木板料，質地堅韌不易開裂，適用於修補渡船踏板、棧橋與各類建築構件。", 1.2),
        ("mat_charcoal_sack", "硬木白炭袋", "以硬木密閉悶燒凝煉的高溫無煙白炭，熱值極高，為鍛爐深層淬火與精煉工造的必備燃料。", 1.2),
    ];

    if let Some(arc) = store::get_store() {
        let mut s = arc.write();

        // 1. 33 P0 來源材料
        for item in crate::world::materials::material_item_defs() {
            if let Some(existing) = s.items.get_mut(&item.id) {
                existing.origin = item.origin;
                existing.description = item.description;
            } else {
                s.items.insert(item.id.clone(), item);
            }
        }

        // raw_defs 補缺
        for (id, name, desc, weight) in raw_defs {
            if let Some(existing) = s.items.get_mut(id) {
                existing.name = name.to_string();
                existing.description = desc.to_string();
                existing.origin = lookup_item_origin(id);
            } else {
                let it = Item {
                    id: id.to_string(),
                    name: name.to_string(),
                    slot: String::new(),
                    item_type: "material".to_string(),
                    weight,
                    stackable: 99,
                    denomination: 8,
                    description: desc.to_string(),
                    vit_bonus: 0,
                    dex_bonus: 0,
                    atk_bonus: 0,
                    origin: lookup_item_origin(id),
                };
                s.items.insert(id.to_string(), it);
            }
        }

        // 2. 12 項二級工坊標準成品
        for g in STANDARD_FINISHED_GOODS {
            if !s.items.contains_key(g.id) {
                let it = Item {
                    id: g.id.to_string(),
                    name: g.name.to_string(),
                    slot: g.slot.to_string(),
                    item_type: g.item_type.to_string(),
                    weight: g.weight,
                    stackable: if g.slot.is_empty() { 20 } else { 1 },
                    denomination: g.base_price,
                    description: g.description.to_string(),
                    vit_bonus: g.vit_bonus,
                    dex_bonus: g.dex_bonus,
                    atk_bonus: g.atk_bonus,
                    origin: g.origin.to_string(),
                };
                s.items.insert(g.id.to_string(), it);
            }
        }

        // 3. 1,120 件千物總典物品
        let valid_wt_ids: std::collections::HashSet<&str> = crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS
            .iter()
            .map(|e| e.id)
            .collect();
        s.items.retain(|id, _| !id.starts_with("wt_cat_") || valid_wt_ids.contains(id.as_str()));
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            let (item_type, slot, vit_bonus, dex_bonus, atk_bonus) =
                if let Some((wear_type, wear_slot, v, d, a)) =
                    classify_catalog_wearable_slot(entry.name, entry.category_num, entry.daily_function)
                {
                    (wear_type, wear_slot, v, d, a)
                } else if matches!(entry.category_num, "08" | "09" | "10" | "8" | "9")
                    || entry.socket == "Taste"
                {
                    ("consumable", "", 0, 0, 0)
                } else {
                    ("material", "", 0, 0, 0)
                };
            let base_price = crate::game::peninsula_catalog::catalog_entry_price_mg(entry);
            let weight = crate::game::peninsula_catalog::catalog_entry_weight_kg(entry);
            let origin = format!("白塔半島・{}", entry.workshop);
            let description = entry.look_text.replace("，主要用於", "；");
            let stackable = if slot.is_empty() {
                if item_type == "material" { 99 } else { 20 }
            } else {
                1
            };
            let it = Item {
                id: entry.id.to_string(),
                name: entry.name.to_string(),
                slot: slot.to_string(),
                item_type: item_type.to_string(),
                weight,
                stackable,
                denomination: base_price,
                description,
                vit_bonus,
                dex_bonus,
                atk_bonus,
                origin,
            };
            s.items.insert(entry.id.to_string(), it);
        }
        let _ = s.persist_items();
    }
}

/// 判斷半島典範器物是否屬於隨身穿戴或手持百工器具（器物無分工具與武器，手持即載體），
/// 排除重型固定設施（如大床、水車、高爐、大鋸、切石機等）與生鮮食材原料。
pub fn classify_catalog_wearable_slot(
    name: &str,
    category_num: &str,
    _daily_function: &str,
) -> Option<(&'static str, &'static str, i32, i32, i32)> {
    const REJECT_KEYWORDS: &[&str] = &[
        "爐", "磨", "大鋸", "滑道", "切石機", "翻土犁", "榨油車", "槽軌", "台車", "工作台",
        "龍骨車", "防撞砧", "翻砂箱", "葫蘆吊", "篩架", "砂帶機", "裁板機", "灰窯", "活塞泵", "虎鉗",
        "火烘槽", "軋花機", "千斤頂", "水石輪", "轉盤", "配重模具", "地鑽", "石槽", "大龍骨", "船纜",
        "重錨", "灰膏", "竹筏", "大貨車", "渡船", "划艇", "壓捆機", "推車", "馬鞍架", "吊架",
        "大釜", "砂鍋", "石櫃", "大蒸籠", "重石罐", "醬菜罈", "絞肉機", "飯桶", "銅桶", "豆腐框",
        "座鐘", "巨鐘", "壁鐘", "路燈", "壁燈", "吊燈", "檢查台", "壓床", "水缸", "輪椅", "抬架",
    ];

    let is_rejected = REJECT_KEYWORDS.iter().any(|&k| {
        if k == "爐" && name.contains("暖手爐") {
            return false;
        }
        if k == "大鋸" && name.contains("剖木大鋸") {
            return false;
        }
        name.contains(k)
    });
    if is_rejected {
        return None;
    }

    // 排除山海原生礦石、林木菌菇、生藥、家畜皮毛等原料與菜餚茶包 (01 ~ 11)
    if matches!(
        category_num,
        "01" | "02" | "03" | "04" | "05" | "06" | "07" | "08" | "09" | "10" | "11"
            | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
    ) {
        return None;
    }

    // --- Handheld Tools / Blades / Clubs ("equipment", "hold_r" or "hold_l") ---
    // Blades/knives/cleavers/saws/shears
    const BLADES: &[&str] = &[
        "獵刀", "廚刀", "圓刀", "彎刀", "剖魚刀", "剖竹刀", "小刀", "鍘刀", "剪刀", "刨刀", "短鋼鋸",
        "剖木大鋸", "鑿刀", "修邊器", "雕刻筆",
    ];
    if BLADES.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "hold_r", 0, 2, 4));
    }

    // Blunt/axes/picks/hammers/bars/clubs
    const BLUNT_AXES: &[&str] = &[
        "重斧", "大斧", "開山鎬", "手鎬", "破冰鎬", "鍛錘", "羊角錘", "手鏨", "撬棍", "打蹄鉗",
        "木刀", "手杖", "打鳥叉", "旋鑿",
    ];
    if BLUNT_AXES.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "hold_r", 2, 0, 4));
    }

    // Long poles/shovels/oars/rods/nets
    const POLES_RODS: &[&str] = &[
        "鐵鍬", "船槳", "釣竿", "飛爪索", "困獸獵網", "旋轉漁網", "駝鞭", "刮板",
    ];
    if POLES_RODS.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "hold_r", 1, 2, 3));
    }

    // Left-hand / utility / shields / umbrellas / lamps / instruments / books
    const LEFT_HAND_UTILITY: &[&str] = &[
        "摺疊傘", "晴雨傘", "摺扇", "火把", "信號筒", "煤油燈", "手提燈", "馬燈", "暗訪燈", "電筒",
        "小夜燈", "羅盤", "羅經", "六分儀", "望遠鏡", "觀海雙鏡", "助視鏡", "星軌銅儀", "水平尺", "卡尺",
        "懷錶", "秒錶", "手札", "課本", "風物誌", "航海錄", "通典", "地圖扇", "算籌", "計數板",
        "銀針囊", "握石", "刮痧板",
    ];
    if LEFT_HAND_UTILITY.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "hold_l", 2, 2, 1));
    }

    // --- Wearable Clothing, Armor & Accessories (Category 16, 17, 19, 20) ---
    // Head
    const HEAD: &[&str] = &["斗笠", "大盤帽", "遮陽帽", "便帽", "頭巾", "耳罩"];
    if HEAD.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "head", 2, 1, 0));
    }

    // Face
    const FACE: &[&str] = &["面罩", "目鏡", "風鏡", "眼罩"];
    if FACE.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "face", 1, 2, 0));
    }

    // Neck
    const NECK: &[&str] = &["圍巾", "領針"];
    if NECK.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "neck", 1, 1, 0));
    }

    // Cloak
    const CLOAK: &[&str] = &["大氅", "防雨長衫", "雨披", "斗篷"];
    if CLOAK.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "cloak", 4, 2, 0));
    }

    // Body
    const BODY: &[&str] = &["常服", "冬衣", "罩衫", "便服", "短襖", "便衫", "長袍", "短褂", "水靠"];
    if BODY.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "body", 4, 1, 0));
    }

    // Wrist/Hand
    const WRIST: &[&str] = &["護腕", "手套", "套袖"];
    if WRIST.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "wrist", 2, 2, 1));
    }

    // Waist
    const WAIST: &[&str] = &["腰帶", "束帶", "圍裙"];
    if WAIST.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "waist", 2, 1, 0));
    }

    // Legs
    const LEGS: &[&str] = &["短褲", "潛水裙", "長褲"];
    if LEGS.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "legs", 2, 1, 0));
    }

    // Leg armor
    const LEG_ARMOR: &[&str] = &["綁腿", "護膝"];
    if LEG_ARMOR.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "leg_armor", 3, 2, 0));
    }

    // Feet
    const FEET: &[&str] = &["短靴", "長統皮靴", "馬伕靴", "釘鞋", "木屐", "便鞋", "軟鞋", "睡靴", "鞋套"];
    if FEET.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "feet", 2, 3, 0));
    }

    // Trinket/Carry
    const TRINKET: &[&str] = &[
        "斜背包", "零錢夾", "背架", "背簍", "提籃", "卷軸袋", "工具袋", "水壺", "皮水囊", "保溫瓶",
        "暖手爐", "暖手筒", "香囊", "急救皮囊",
    ];
    if TRINKET.iter().any(|&k| name.contains(k)) {
        return Some(("equipment", "trinket", 2, 1, 0));
    }

    None
}


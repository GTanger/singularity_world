pub struct FinishedGoodDef {
    pub id: &'static str,
    pub name: &'static str,
    pub slot: &'static str,
    pub item_type: &'static str,
    pub weight: f64,
    pub atk_bonus: i32,
    pub vit_bonus: i32,
    pub dex_bonus: i32,
    pub base_price: i32,
    pub description: &'static str,
    pub origin: &'static str,
}

pub const STANDARD_FINISHED_GOODS: [FinishedGoodDef; 16] = [
    FinishedGoodDef {
        id: "craft_steel_hunting_knife",
        name: "精鍛防身獵刀",
        slot: "hold_r",
        item_type: "equipment",
        weight: 1.2,
        atk_bonus: 4,
        vit_bonus: 0,
        dex_bonus: 2,
        base_price: 28,
        description: "以百鍊精鋼折疊鍛打而成的隨身短刀，刃口淬火如霜雪，削鐵如泥，防身護行兩相宜。",
        origin: "分家巷",
    },
    FinishedGoodDef {
        id: "craft_iron_pickaxe",
        name: "熟鐵開山鎬",
        slot: "hold_r",
        item_type: "equipment",
        weight: 2.8,
        atk_bonus: 3,
        vit_bonus: 2,
        dex_bonus: 0,
        base_price: 24,
        description: "沉重厚實的開山重鎬，鎬尖覆以高溫淬硬碳鋼，專為穿岩破石、拓荒採掘而製。",
        origin: "紅土崁",
    },
    FinishedGoodDef {
        id: "craft_bamboo_oil_umbrella",
        name: "韌竹骨桐油晴雨傘",
        slot: "hold_l",
        item_type: "equipment",
        weight: 0.9,
        atk_bonus: 0,
        vit_bonus: 3,
        dex_bonus: 0,
        base_price: 20,
        description: "精選三年生重竹為骨，傘面覆以七層熟桐油浸染皮紙，風雨不侵，收展清脆沉著。",
        origin: "水尾寮",
    },
    FinishedGoodDef {
        id: "craft_camphor_storage_box",
        name: "防潮樟木隨身匣",
        slot: "",
        item_type: "utility",
        weight: 0.8,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 22,
        description: "老樟木雕琢而成的隨身多寶匣，四角包以白銅護角，內置暗鎖，散發防蛀清幽木香。",
        origin: "霜林口",
    },
    FinishedGoodDef {
        id: "craft_meridian_salve",
        name: "溫脈舒筋活血膏",
        slot: "",
        item_type: "consumable",
        weight: 0.3,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 18,
        description: "以十二味深山藥草慢火熬製提純的濃縮油膏，塗抹經穴溫熱滲透，能快速舒緩筋骨疲憊，恢復體魄與元氣。",
        origin: "半嶺鎮",
    },
    FinishedGoodDef {
        id: "craft_calming_tea_pack",
        name: "清心安神百草茶包",
        slot: "",
        item_type: "consumable",
        weight: 0.2,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 16,
        description: "採集水畔仙草與山茶幼芽曬製而成的秘製茶包，沖泡後香氣澄淨甘潤，令人心神大定、深幅恢復元氣。",
        origin: "半嶺鎮",
    },
    FinishedGoodDef {
        id: "craft_pine_soot_ink",
        name: "松煙紫光精墨錠",
        slot: "",
        item_type: "utility",
        weight: 0.4,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 2,
        base_price: 26,
        description: "千杵萬槌研合純淨松煙與牛皮骨膠凝成的珍品墨錠，磨開黑潤如漆，隱隱泛出深邃紫光。",
        origin: "文曜",
    },
    FinishedGoodDef {
        id: "craft_peninsula_almanac",
        name: "白塔風物考工手札",
        slot: "hold_l",
        item_type: "equipment",
        weight: 0.7,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 25,
        description: "手造竹麻紙精裝抄本，詳實輯錄半島山川水文、工坊匠造規約與草木金石圖譜，讀之開闊胸臆。",
        origin: "文曜",
    },
    FinishedGoodDef {
        id: "craft_storm_canvas_cloak",
        name: "防風浸油帆布行旅斗篷",
        slot: "body",
        item_type: "equipment",
        weight: 1.6,
        atk_bonus: 0,
        vit_bonus: 5,
        dex_bonus: 2,
        base_price: 32,
        description: "以重磅密織亞麻帆布覆以防潮植物油膠精製的長斗篷，抗寒防浪，抵擋風雨如磐石。",
        origin: "風舶角",
    },
    FinishedGoodDef {
        id: "craft_heavy_leather_boots",
        name: "厚底防滑行旅皮靴",
        slot: "feet",
        item_type: "equipment",
        weight: 1.4,
        atk_bonus: 0,
        vit_bonus: 3,
        dex_bonus: 4,
        base_price: 30,
        description: "嚴選熟成厚實牛皮手工縫製，靴底嵌有防滑硬木齒紋，涉水翻山穩如泰山。",
        origin: "牛埔",
    },
    FinishedGoodDef {
        id: "craft_pine_torch",
        name: "浸脂松明火把",
        slot: "hold_l",
        item_type: "equipment",
        weight: 0.8,
        atk_bonus: 4,
        vit_bonus: 3,
        dex_bonus: 4,
        base_price: 16,
        description: "以乾燥柴木浸塗天然松脂紮製而成的導能照明火把，可於暗夜荒野照明驅獸、引燃營火或驅散濕寒。",
        origin: "霜林口",
    },
    FinishedGoodDef {
        id: "craft_wooden_tool_handle",
        name: "堅韌硬木工具長柄",
        slot: "",
        item_type: "material",
        weight: 1.0,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 15,
        description: "經導能木作刨削打磨的堅韌硬木長柄，既可組裝開山鎬與獵刀，亦可作為修補渡船踏板與漁網架的關鍵構件。",
        origin: "霜林口",
    },
    FinishedGoodDef {
        id: "craft_roasted_shrimp_skewer",
        name: "炭炙水尾鮮蝦串",
        slot: "",
        item_type: "consumable",
        weight: 0.2,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 18,
        description: "以水尾寮鮮活河蝦或礁岸鮮蝦佐以粗鹽、山胡椒馬告穿竹籤文火炭炙而成，蝦殼酥脆焦香、蝦肉彈牙甘鮮，食之立添氣力與元氣。",
        origin: "水尾寮",
    },
    FinishedGoodDef {
        id: "craft_smoked_fish_fillet",
        name: "相思木燻鮮魚乾",
        slot: "",
        item_type: "consumable",
        weight: 0.3,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 20,
        description: "嚴選肥美鮮魚剖洗去骨，抹上海鹽與壺底蔭油，以相思木慢火冷燻收乾，魚肉緊實鹹香，耐儲易攜、開胃益氣。",
        origin: "風舶角",
    },
    FinishedGoodDef {
        id: "craft_spiced_beast_jerky",
        name: "馬告炭炙獸肉脯",
        slot: "",
        item_type: "consumable",
        weight: 0.3,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 22,
        description: "取山海精實獸肉切為厚片，揉入山胡椒馬告與老薑黑糖慢火烘炙成脯，嚼勁十足、肉香醇厚，遠行狩獵最能補益氣血。",
        origin: "牛埔",
    },
    FinishedGoodDef {
        id: "craft_ration_grain_cake",
        name: "行旅耐儲米糧餅",
        slot: "",
        item_type: "consumable",
        weight: 0.4,
        atk_bonus: 0,
        vit_bonus: 0,
        dex_bonus: 0,
        base_price: 16,
        description: "以大圳頭精選稻穀與雜糧磨粉烙製而成的厚實米糧餅，外皮微焦酥香、內裡綿密耐飢，為半島行旅常備乾糧。",
        origin: "大圳頭",
    },
];

/// 判定物品階段：`raw`（原料）、`mutated`（異變）或 `finished`（成品）。
pub fn classify_item_stage(item_id: &str) -> &'static str {
    if item_id.starts_with("mut_")
        || item_id.contains("【異變")
        || item_id.contains("星紋")
        || item_id.contains("雷紋")
        || item_id.contains("寒泉")
        || item_id.contains("紫煙")
        || item_id.contains("赤焰")
        || item_id.contains("潮汐晶化")
    {
        return "mutated";
    }
    if is_raw_material(item_id) {
        return "raw";
    }
    "finished"
}

fn is_raw_material(item_id: &str) -> bool {
    if item_id == "wt_beast_bone" || item_id == "wt_down_feather" {
        return true;
    }
    if item_id.starts_with("mat_") || item_id.starts_with("harvest_") {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (1..=387).contains(&num) {
                return true;
            }
        }
    }
    let raw_keywords = [
        "礦", "砂", "原木", "蘆草", "竹竿", "草藥", "水苔", "紙刀", "墨坯",
        "麻繩", "袋布", "原牛皮", "鮮河蝦", "水芹菜", "黏土", "石料", "粗麻", "桐油", "松煙", "骨膠",
        "絨羽", "獸骨", "老鱉", "毛皮", "鱗片", "獠牙", "彩羽", "尾翎", "香皮", "鹿茸", "山茶籽", "白鮮草", "藥草", "漿果", "獸皮", "獸筋",
    ];
    if raw_keywords.iter().any(|k| item_id.contains(k)) {
        return true;
    }
    if let Some(def) = db::get_item_def(item_id) {
        if raw_keywords.iter().any(|k| def.name.contains(k)) {
            return true;
        }
    }
    false
}

/// 動態市場行情價格：計算即時浮動價格與狀態標籤（緊缺漲價／供需平穩／充裕讓利／異變奇珍）。
pub fn dynamic_market_price_mg(item_id: &str, base_price: i32) -> (i32, &'static str) {
    let base = base_price.max(1);
    if classify_item_stage(item_id) == "mutated" || item_id.starts_with("mut_") {
        let price = ((base as f64) * 3.2).round() as i32;
        return (price.max(base * 3).max(10), "異變奇珍");
    }

    let quote = dynamic_settlement_market_quote("文曜", item_id, base_price);
    (quote.unit_price, quote.status_label)
}

/// 動態聚落自由市場即時行情報價：依據所在聚落即時庫存與目標供需比計算無硬編碼的真實浮動價格。
pub fn dynamic_settlement_market_quote(
    settlement_or_room: &str,
    item_id: &str,
    base_price: i32,
) -> SettlementMarketQuote {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    let mut origin_lookup_id = item_id;
    let mut base_id_buf = String::new();
    if let Some(rest) = item_id.strip_prefix("mut_") {
        for arch in MUTATION_ARCHETYPES {
            let suffix = format!("_{}", arch.key);
            if let Some(base_part) = rest.strip_suffix(&suffix) {
                base_id_buf = base_part.to_string();
                break;
            }
        }
        if !base_id_buf.is_empty() {
            origin_lookup_id = &base_id_buf;
        }
    }

    let mut origin_str = lookup_item_origin(origin_lookup_id);
    if origin_str.is_empty() {
        if let Some(def) = lookup_ecology_item_def(origin_lookup_id) {
            origin_str = def.origin;
        }
    }
    let raw_origin = crate::world::materials::infer_origin_settlement(&origin_str);
    let origin_settlement = if raw_origin.is_empty() {
        if origin_lookup_id == "mat_rough_ore" {
            "紅土崁".to_string()
        } else {
            String::new()
        }
    } else {
        normalize_canonical_settlement(raw_origin).to_string()
    };

    let key = (settlement.to_string(), item_id.to_string());
    let cell = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.settlement_markets.get(&key).cloned()
    };

    let cell = match cell {
        Some(c) => c,
        None => {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let new_cell = init_settlement_market_cell(settlement, item_id);
            ledger.settlement_markets.insert(key, new_cell.clone());
            new_cell
        }
    };

    let ratio = ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
        * cell.demand_ema.clamp(0.75, 1.45);
    let ratio = ratio.clamp(0.45, 2.60);
    let unit_price = (((base_price.max(2) as f32) * ratio).round() as i32).max(1);

    let status_label = if cell.stock <= 1 || ratio >= 1.55 {
        "極度緊缺・搶手高價"
    } else if cell.stock * 3 < cell.target_stock * 2 || ratio >= 1.18 {
        "需求旺盛・行情看漲"
    } else if cell.stock > cell.target_stock * 3 / 2 || ratio <= 0.78 {
        "產地充裕・現貨讓利"
    } else if cell.stock > cell.target_stock || ratio <= 0.90 {
        "供給寬裕・價位親民"
    } else {
        "供需平衡・市價穩定"
    };

    let market_narrative = if !origin_settlement.is_empty() && origin_settlement == settlement {
        format!("【{settlement}】為此物產地，本地市集{}", status_label)
    } else {
        format!(
            "此物產自【{}】，【{settlement}】本地市集{}",
            if origin_settlement.is_empty() { "外地" } else { &origin_settlement },
            status_label
        )
    };

    SettlementMarketQuote {
        settlement: settlement.to_string(),
        origin_settlement,
        unit_price,
        base_price,
        local_stock: cell.stock,
        target_stock: cell.target_stock,
        status_label,
        market_narrative,
    }
}

/// 查詢指定聚落市集特定物品之即時庫存與即時浮動價格
pub fn get_market_item_stock_and_price(
    settlement_or_room: &str,
    item_id: &str,
    base_price: i32,
) -> (i32, i32) {
    let quote = dynamic_settlement_market_quote(settlement_or_room, item_id, base_price);
    (quote.local_stock, quote.unit_price)
}

/// 記錄聚落市集買賣交易後的庫存與需求動態反饋，即時調整庫存與 EMA 價格。
pub fn record_settlement_market_trade(
    settlement_or_room: &str,
    item_id: &str,
    base_price: i32,
    delta_stock: i32,
    actor_label: &str,
) -> SettlementMarketQuote {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };
    let key = (settlement.to_string(), item_id.to_string());

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        if !ledger.settlement_markets.contains_key(&key) {
            let new_cell = init_settlement_market_cell(settlement, item_id);
            ledger.settlement_markets.insert(key.clone(), new_cell);
        }
        if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
            if delta_stock > 0 {
                cell.stock = (cell.stock + delta_stock).min(cell.target_stock * 4);
                cell.demand_ema = (cell.demand_ema * 0.92).max(0.75);
            } else if delta_stock < 0 {
                cell.stock = (cell.stock + delta_stock).max(0);
                cell.demand_ema = (cell.demand_ema * 1.08).min(1.45);
            }
        }
        // 同步全域 ledger.items 庫存
        if let Some(entry) = ledger.items.get_mut(item_id) {
            if delta_stock > 0 {
                entry.stock = (entry.stock + delta_stock).min(entry.target_stock * 4);
            } else if delta_stock < 0 {
                entry.stock = (entry.stock + delta_stock).max(0);
            }
        }
    }

    let quote = dynamic_settlement_market_quote(settlement, item_id, base_price);

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
            cell.last_price_mg = quote.unit_price;
        }
        let item_name = get_clean_item_name(item_id);
        let event = format!(
            "【市集調控】{actor_label}於【{settlement}】交易「{item_name}」，現貨調整為 {}/{}，行情每份 {} 鎂（{}）。",
            quote.local_stock,
            quote.target_stock,
            quote.unit_price,
            quote.status_label
        );
        ledger.recent_events.push_back(event);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
        save_economy_ledger_snapshot(&ledger);
    }
    if quote.local_stock <= (quote.target_stock / 3).max(1) {
        let now_unix = crate::game::now_unix();
        let item_name = get_clean_item_name(item_id);
        let fact = crate::npc::causal_core::WorldRippleFact {
            fact_id: format!("mkt_scarcity_{}_{}_{}", settlement, item_id, now_unix / 300),
            kind: crate::npc::causal_core::RippleFactKind::MarketScarcity,
            subject_zone_or_settlement: settlement.to_string(),
            item_or_actor: item_name.clone(),
            order_id: None,
            severity: 1.0,
            origin_tick: now_unix,
            hop_count: 0,
            max_hops: 4,
            prose_summary: format!("{}市集上的「{}」現貨幾近見底，坊間行情正節節看漲", settlement, item_name),
        };
        let is_valid_id = actor_label.starts_with("npc_")
            || crate::npc::causal_core::LIVE_NPC_CAUSAL_STATES
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(actor_label);
        let injected = if is_valid_id {
            crate::npc::causal_core::inject_ripple_fact_into_npc(actor_label, fact.clone(), now_unix)
        } else {
            false
        };
        if !injected {
            if let Some(arc) = crate::store::get_store() {
                if let Some(s) = arc.try_read() {
                    if let Some(witness) = s.entities.values().find(|e| e.kind == "npc" && e.vit > 0 && e.display_title.contains(&settlement)) {
                        crate::npc::causal_core::inject_ripple_fact_into_npc(&witness.id, fact, now_unix);
                    }
                }
            }
        }
    }

    quote
}

static SCARCITY_CATEGORY_CACHE: std::sync::LazyLock<
    parking_lot::RwLock<
        std::collections::HashMap<
            (String, String),
            (std::time::Instant, (f32, &'static str, String)),
        >,
    >,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

/// 查詢指定聚落與行為類別（"food" | "drink" | "clinic" | "mend" | "forage" | "craft" | "arbitrage"）之即時供需價格倍率 (0.65..=2.20)、代表物資 ID 與中文名稱。
pub fn settlement_scarcity_multiplier_for_category(
    settlement_or_room: &str,
    category: &str,
) -> (f32, &'static str, String) {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    let cache_key = (settlement.to_string(), category.to_string());
    #[cfg(not(test))]
    {
        if let Some(c) = SCARCITY_CATEGORY_CACHE.try_read()
            && let Some((ts, cached)) = c.get(&cache_key)
            && ts.elapsed() < std::time::Duration::from_millis(1500)
        {
            return cached.clone();
        }
    }

    let candidates: &'static [&'static str] = match category {
        "food" => &["mat_salted_fish", "mat_coarse_salt_pack", "mat_charcoal_sack"],
        "drink" => &["craft_herbal_tea_pack", "craft_calming_tea_pack"],
        "clinic" => &["craft_meridian_salve", "mat_foraged_herb"],
        "mend" => &["craft_tool_repair_strip", "mat_iron_scrap", "mat_plank_board", "craft_tarred_hemp_rope"],
        "forage" => &["mat_kindling_bundle", "mat_foraged_herb", "mat_reed_bundle", "mat_rough_ore"],
        "craft" => &["craft_tool_repair_strip", "craft_meridian_salve", "craft_herbal_tea_pack", "mat_plank_board"],
        _ => &["mat_rough_ore", "mat_charcoal_sack", "mat_timber_log", "mat_foraged_herb", "craft_meridian_salve", "craft_tool_repair_strip"],
    };

    // 確保候選單元格存在於市集帳本
    {
        let missing: Vec<&'static str> = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            candidates
                .iter()
                .copied()
                .filter(|&item_id| !ledger.settlement_markets.contains_key(&(settlement.to_string(), item_id.to_string())))
                .collect()
        };
        if !missing.is_empty() {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            for &item_id in &missing {
                let key = (settlement.to_string(), item_id.to_string());
                if !ledger.settlement_markets.contains_key(&key) {
                    let cell = init_settlement_market_cell(settlement, item_id);
                    ledger.settlement_markets.insert(key, cell);
                }
            }
        }
    }

    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());

    let result = if category == "forage" || category == "craft" || category == "arbitrage" {
        let mut best_ratio = -1.0f32;
        let mut best_item = candidates[0];
        for &item_id in candidates {
            let ratio = if let Some(cell) = ledger.settlement_markets.get(&(settlement.to_string(), item_id.to_string())) {
                ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                    * cell.demand_ema.clamp(0.75, 1.45)
            } else {
                1.0
            };
            if ratio > best_ratio {
                best_ratio = ratio;
                best_item = item_id;
            }
        }
        let clean_name = get_clean_item_name(best_item);
        (best_ratio.clamp(0.65, 2.20), best_item, clean_name)
    } else {
        let mut sum_ratio = 0.0f32;
        let mut count = 0usize;
        for &item_id in candidates {
            let ratio = if let Some(cell) = ledger.settlement_markets.get(&(settlement.to_string(), item_id.to_string())) {
                ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                    * cell.demand_ema.clamp(0.75, 1.45)
            } else {
                1.0
            };
            sum_ratio += ratio;
            count += 1;
        }
        let avg_ratio = if count > 0 { sum_ratio / (count as f32) } else { 1.0 };
        let primary_item = candidates[0];
        let clean_name = get_clean_item_name(primary_item);
        (avg_ratio.clamp(0.65, 2.20), primary_item, clean_name)
    };
    drop(ledger);

    if let Some(mut c) = SCARCITY_CATEGORY_CACHE.try_write() {
        c.insert(cache_key, (std::time::Instant::now(), result.clone()));
    }
    result
}
#[cfg(not(test))]
static SCARCITY_SIGNAL_CACHE: std::sync::LazyLock<
    parking_lot::RwLock<
        std::collections::HashMap<
            String,
            (std::time::Instant, Option<(String, String, f32)>),
        >,
    >,
> = std::sync::LazyLock::new(|| parking_lot::RwLock::new(std::collections::HashMap::new()));

/// 查詢指定聚落當前最緊缺物資信號（若最高價格倍率 >= 1.18 則回傳 Some((item_id, clean_name, ratio))）。
pub fn settlement_scarcity_signal(
    settlement_or_room: &str,
) -> Option<(String, String, f32)> {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    #[cfg(not(test))]
    {
        if let Some((ts, cached)) = SCARCITY_SIGNAL_CACHE.read().get(settlement) {
            if ts.elapsed() < std::time::Duration::from_millis(1500) {
                return cached.clone();
            }
        }
    }

    let (highest_ratio, scarcest_item) = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        let mut highest_ratio = 0.0f32;
        let mut scarcest_item: Option<String> = None;

        for ((s, item_id), cell) in &ledger.settlement_markets {
            if s == settlement {
                let ratio = ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68)
                    * cell.demand_ema.clamp(0.75, 1.45);
                if ratio > highest_ratio {
                    highest_ratio = ratio;
                    scarcest_item = Some(item_id.clone());
                }
            }
        }
        (highest_ratio, scarcest_item)
    };

    let res = if highest_ratio >= 1.18 {
        if let Some(item_id) = scarcest_item {
            let clean_name = get_clean_item_name(&item_id);
            Some((item_id, clean_name, highest_ratio.clamp(0.65, 2.20)))
        } else {
            None
        }
    } else {
        None
    };

    #[cfg(not(test))]
    {
        SCARCITY_SIGNAL_CACHE
            .write()
            .insert(settlement.to_string(), (std::time::Instant::now(), res.clone()));
    }

    res
}

/// 住民執行消費行動（飲食、飲茶、看診、修器）時，真實扣減聚落市集對應物資庫存並推升需求 EMA。
pub fn consume_settlement_commodity_for_action(
    settlement_or_room: &str,
    category: &str,
    actor_name: &str,
) -> Option<(String, i32, f32)> {
    let raw_settlement = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let settlement = if raw_settlement.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    let candidates: &'static [&'static str] = match category {
        "food" => &["mat_salted_fish", "mat_coarse_salt_pack", "mat_charcoal_sack"],
        "drink" => &["craft_herbal_tea_pack", "craft_calming_tea_pack"],
        "clinic" => &["craft_meridian_salve", "mat_foraged_herb"],
        "mend" => &["craft_tool_repair_strip", "mat_iron_scrap", "mat_plank_board", "craft_tarred_hemp_rope"],
        "forage" => &["mat_kindling_bundle", "mat_foraged_herb", "mat_reed_bundle", "mat_rough_ore"],
        "craft" => &["craft_tool_repair_strip", "craft_meridian_salve", "craft_herbal_tea_pack", "mat_plank_board"],
        _ => &["mat_rough_ore", "mat_charcoal_sack", "mat_timber_log", "mat_foraged_herb", "craft_meridian_salve", "craft_tool_repair_strip"],
    };

    let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());

    for &item_id in candidates {
        let key = (settlement.to_string(), item_id.to_string());
        if !ledger.settlement_markets.contains_key(&key) {
            let cell = init_settlement_market_cell(settlement, item_id);
            ledger.settlement_markets.insert(key, cell);
        }
    }

    let rot = settlement
        .bytes()
        .chain(actor_name.bytes())
        .fold(0usize, |acc, b| acc.wrapping_mul(31).wrapping_add(b as usize));
    let n_cand = candidates.len().max(1);
    let mut chosen_item = candidates[rot % n_cand];
    let mut max_stock = -1;
    for i in 0..n_cand {
        let item_id = candidates[(rot + i) % n_cand];
        let key = (settlement.to_string(), item_id.to_string());
        if let Some(cell) = ledger.settlement_markets.get(&key)
            && cell.stock > max_stock
        {
            max_stock = cell.stock;
            chosen_item = item_id;
        }
    }

    let key = (settlement.to_string(), chosen_item.to_string());
    let (prev_stock, rem_stock, target_stock, ratio) = if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
        let prev_stock = cell.stock;
        if cell.stock > 0 {
            cell.stock -= 1;
        }
        cell.demand_ema = (cell.demand_ema * 1.04).clamp(0.75, 1.45);
        let ratio = ((cell.target_stock.max(1) as f32 + 2.0) / (cell.stock.max(0) as f32 + 2.0)).powf(0.68) * cell.demand_ema;
        let base_p = item_base_price_mg(chosen_item) as f32;
        cell.last_price_mg = ((base_p * ratio.clamp(0.45, 2.60)).round() as i32).max(1);
        (prev_stock, cell.stock, cell.target_stock, ratio)
    } else {
        (0, 0, 10, 1.0)
    };

    if let Some(entry) = ledger.items.get_mut(chosen_item) {
        if entry.stock > 0 {
            entry.stock -= 1;
        }
    }

    let clean_name = get_clean_item_name(chosen_item);
    if prev_stock > 0 && !actor_name.is_empty() && rem_stock <= 2 {
        let clean_actor = actor_name.trim().replace(['|', '｜'], "・");
        let event = format!(
            "{clean_actor}在{settlement}市集買走「{clean_name}」，攤上現貨所剩無幾，價錢隨之看漲。"
        );
        ledger.recent_events.push_back(event);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }
    let emit_scarcity_ripple = prev_stock > 0 && rem_stock <= (target_stock / 3).max(1);
    drop(ledger);

    if emit_scarcity_ripple {
        let now_unix = crate::game::now_unix();
        let fact = crate::npc::causal_core::WorldRippleFact {
            fact_id: format!("mkt_scarcity_{}_{}_{}", settlement, chosen_item, now_unix / 300),
            kind: crate::npc::causal_core::RippleFactKind::MarketScarcity,
            subject_zone_or_settlement: settlement.to_string(),
            item_or_actor: clean_name.clone(),
            order_id: None,
            severity: 1.0,
            origin_tick: now_unix,
            hop_count: 0,
            max_hops: 4,
            prose_summary: format!("{}市集上的「{}」現貨幾近見底，坊間行情正節節看漲", settlement, clean_name),
        };
        let is_valid_id = actor_name.starts_with("npc_")
            || crate::npc::causal_core::LIVE_NPC_CAUSAL_STATES
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(actor_name);
        let injected = if is_valid_id {
            crate::npc::causal_core::inject_ripple_fact_into_npc(actor_name, fact.clone(), now_unix)
        } else {
            false
        };
        if !injected {
            if let Some(arc) = crate::store::get_store() {
                if let Some(s) = arc.try_read() {
                    if let Some(witness) = s.entities.values().find(|e| e.kind == "npc" && e.vit > 0 && e.display_title.contains(&settlement)) {
                        crate::npc::causal_core::inject_ripple_fact_into_npc(&witness.id, fact, now_unix);
                    }
                }
            }
        }
    }

    Some((clean_name, rem_stock, ratio.clamp(0.65, 2.20)))
}


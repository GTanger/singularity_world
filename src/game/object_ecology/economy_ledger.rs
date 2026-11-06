#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketItemState {
    pub stock: i32,
    pub target_stock: i32,
    pub demand_pressure: i32,
}

#[derive(Debug, Clone)]
pub struct CampfireState {
    pub fuel_level: u32,
    pub is_charcoal: bool,
    pub updated_at: i64,
}

#[derive(Debug, Clone)]
pub struct FieldFacilityState {
    pub facility_id: String,
    pub name: String,
    pub created_at: i64,
    pub expires_at: i64,
}

pub const CANONICAL_SETTLEMENTS: [&str; 9] = [
    "文曜", "風舶角", "水尾寮", "半嶺鎮", "紅土崁", "霜林口", "下營", "南哨", "烏石鼻",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementMarketCell {
    pub settlement: String,
    pub item_id: String,
    pub stock: i32,
    pub target_stock: i32,
    pub local_prod_rate: i32, // autonomous NPC production per cycle
    pub local_cons_rate: i32, // autonomous NPC workshop/daily consumption per cycle
    pub demand_ema: f32,
    pub last_price_mg: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedEconomySnapshot {
    pub settlement_cells: Vec<SettlementMarketCell>,
    #[serde(default)]
    pub items: HashMap<String, MarketItemState>,
    pub recent_events: Vec<String>,
}

pub const ECONOMY_LEDGER_FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/runtime/settlement_economy_ledger.json"
);

pub fn economy_ledger_path() -> std::path::PathBuf {
    let p = std::path::Path::new("data/runtime/settlement_economy_ledger.json");
    if p.exists() || std::path::Path::new("data/runtime").exists() {
        p.to_path_buf()
    } else {
        std::path::PathBuf::from(ECONOMY_LEDGER_FILE)
    }
}

pub fn save_economy_ledger_snapshot_to_path(ledger: &EconomyLedger, path: &std::path::Path) {
    let snapshot = PersistedEconomySnapshot {
        settlement_cells: ledger.settlement_markets.values().cloned().collect(),
        items: ledger.items.clone(),
        recent_events: ledger.recent_events.iter().cloned().collect(),
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json_str) = serde_json::to_string_pretty(&snapshot) {
        let _ = std::fs::write(path, json_str);
    }
}

pub fn save_economy_ledger_snapshot(ledger: &EconomyLedger) {
    #[cfg(not(test))]
    {
        let path = economy_ledger_path();
        save_economy_ledger_snapshot_to_path(ledger, &path);
    }
    #[cfg(test)]
    {
        let _ = ledger;
    }
}

#[derive(Debug, Clone)]
pub struct SettlementMarketQuote {
    pub settlement: String,
    pub origin_settlement: String,
    pub unit_price: i32,
    pub base_price: i32,
    pub local_stock: i32,
    pub target_stock: i32,
    pub status_label: &'static str,
    pub market_narrative: String,
}

pub fn normalize_canonical_settlement(raw_settlement: &str) -> &'static str {
    let resolved_room;
    let s = if raw_settlement.starts_with("grid:") {
        if let Some(coord) = crate::grid::parse_grid_room_id(raw_settlement)
            && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
                .and_then(|g| g.get(coord).cloned())
        {
            resolved_room = format!("{} {}", cell.zone, cell.name);
            resolved_room.as_str()
        } else {
            raw_settlement
        }
    } else {
        raw_settlement
    };
    if s.contains("烏石鼻") || s.contains("鹽田澳") || s.contains("黑石灣") || s.contains("東海峭壁") {
        "烏石鼻"
    } else if s.contains("風舶") || s.contains("巨藻淺灣") || s.contains("驚濤岩岸") || s.contains("落日岬") {
        "風舶角"
    } else if s.contains("水尾寮") || s.contains("淺澤泥口") || s.contains("水尾澤") || s.contains("黑沼林") || s.contains("黑沼林澤") || s.contains("黑沼") {
        "水尾寮"
    } else if s.contains("半嶺") || s.contains("牛埔") || s.contains("白塔") || s.contains("分家巷") || s.contains("西麓林道") || s.contains("長風草甸") {
        "半嶺鎮"
    } else if s.contains("紅土崁") || s.contains("大圳頭") || s.contains("老堰頭") || s.contains("西南原") || s.contains("赤石坡") || s.contains("西嶺礦谷") {
        "紅土崁"
    } else if s.contains("霜林口") || s.contains("翠冠") || s.contains("北嶺腳") || s.contains("樟木驛") || s.contains("竹林寨") || s.contains("青竹坪") || s.contains("古河巨木林") || s.contains("北坡雲杉林") || s.contains("雲腰雪林") {
        "霜林口"
    } else if s.contains("下營") || s.contains("黑沼關隘") || s.contains("三號坑") {
        "下營"
    } else if s.contains("南哨") || s.contains("風櫃尾") || s.contains("白茫洲") || s.contains("貼山深潭") || s.contains("清泉溪谷") {
        "南哨"
    } else {
        "文曜"
    }
}

pub fn is_high_demand_in_settlement(settlement: &str, item_id: &str) -> bool {
    match settlement {
        "文曜" => {
            item_id.starts_with("mat_rough_ore")
                || item_id.starts_with("mat_washed_ore")
                || item_id.starts_with("mat_stone_powder")
                || item_id.starts_with("mat_iron_scrap")
                || item_id.starts_with("mat_charcoal")
                || item_id.starts_with("mat_foraged_herb")
                || item_id.starts_with("mat_herb")
                || item_id.starts_with("mat_reed")
                || item_id.starts_with("mat_hemp")
                || item_id.starts_with("mat_coarse_paper")
                || item_id == "craft_calming_tea_pack"
                || item_id == "craft_meridian_salve"
        }
        "風舶角" | "水尾寮" => {
            item_id.starts_with("mat_timber")
                || item_id.starts_with("mat_plank")
                || item_id.starts_with("mat_wood_offcut")
                || item_id.starts_with("mat_hemp")
                || item_id.starts_with("mat_sealing_pitch")
                || item_id.starts_with("mat_oilcloth")
                || item_id.starts_with("mat_canvas")
                || item_id.starts_with("mat_sackcloth")
                || item_id == "craft_storm_canvas_cloak"
                || item_id == "craft_camphor_storage_box"
                || item_id == "craft_caulking_sealant"
                || item_id == "mat_cedar_plank"
        }
        "下營" | "南哨" => {
            item_id.starts_with("mat_repair_strip")
                || item_id.starts_with("mat_metal_rim")
                || item_id.starts_with("mat_iron")
                || item_id.starts_with("mat_coarse_salt")
                || item_id.starts_with("mat_hardtack")
                || item_id == "craft_steel_hunting_knife"
                || item_id == "craft_meridian_salve"
                || item_id == "craft_calming_tea_pack"
                || item_id == "craft_heavy_leather_boots"
                || item_id == "craft_tool_repair_strip"
                || item_id == "craft_patched_work_boots"
                || item_id == "mat_salted_fish"
        }
        "紅土崁" | "霜林口" | "半嶺鎮" | "烏石鼻" => {
            item_id == "craft_steel_hunting_knife"
                || item_id == "craft_iron_pickaxe"
                || item_id == "craft_heavy_leather_boots"
                || item_id == "craft_meridian_salve"
                || item_id == "craft_calming_tea_pack"
                || item_id == "craft_bamboo_oil_umbrella"
                || item_id == "craft_herbal_tea_pack"
                || item_id == "craft_tarred_hemp_rope"
                || item_id.starts_with("mat_grain")
                || item_id.starts_with("mat_sackcloth")
                || item_id.starts_with("mat_coarse_salt")
        }
        _ => false,
    }
}

pub fn item_base_price_mg(item_id: &str) -> i32 {
    crate::world::materials::trade_price_anchor_mg(item_id)
        .or_else(|| {
            for fg in &STANDARD_FINISHED_GOODS {
                if fg.id == item_id {
                    return Some(fg.base_price);
                }
            }
            None
        })
        .unwrap_or(8)
        .max(2)
}

pub fn init_settlement_market_cell(settlement: &str, item_id: &str) -> SettlementMarketCell {
    let origin_str = lookup_item_origin(item_id);
    let raw_origin = crate::world::materials::infer_origin_settlement(&origin_str);
    let origin_settlement = normalize_canonical_settlement(raw_origin);

    let is_origin = settlement == origin_settlement
        || (item_id == "mat_rough_ore" && (settlement == "紅土崁" || settlement == "下營"))
        || (item_id == "mat_iron_scrap" && (settlement == "文曜" || settlement == "紅土崁"))
        || (item_id == "mat_charcoal_sack" && settlement == "霜林口")
        || (item_id == "mat_timber_log" && settlement == "霜林口")
        || (item_id == "mat_plank_board" && settlement == "霜林口")
        || (item_id == "mat_reed_bundle" && settlement == "水尾寮")
        || (item_id == "mat_hemp_rope_roll" && settlement == "水尾寮")
        || (item_id == "mat_sackcloth_bolt" && settlement == "水尾寮")
        || (item_id == "mat_foraged_herb" && settlement == "半嶺鎮")
        || (item_id == "mat_washed_ore" && settlement == "半嶺鎮")
        || (item_id == "mat_coarse_salt_pack" && settlement == "烏石鼻")
        || (item_id == "mat_shell_sack" && settlement == "烏石鼻")
        || (item_id == "mat_hardtack_sack" && settlement == "南哨")
        || (item_id == "mat_fish_scale_sack" && settlement == "風舶角")
        || (item_id == "mat_canvas_roll" && settlement == "風舶角")
        || (item_id == "mat_salted_fish" && settlement == "水尾寮")
        || (item_id == "mat_cedar_plank" && settlement == "霜林口")
        || (item_id == "craft_tool_repair_strip" && settlement == "文曜")
        || (item_id == "craft_herbal_tea_pack" && settlement == "半嶺鎮")
        || (item_id == "craft_patched_work_boots" && settlement == "半嶺鎮")
        || (item_id == "craft_tarred_hemp_rope" && settlement == "風舶角")
        || (item_id == "craft_caulking_sealant" && settlement == "風舶角");

    let (target_stock, stock, local_prod_rate, local_cons_rate, demand_ema): (i32, i32, i32, i32, f32) = if is_origin {
        (10, 16, 3, 1, 0.82)
    } else if is_high_demand_in_settlement(settlement, item_id) {
        (12, 4, 0, 2, 1.20)
    } else {
        (8, 5, 0, 1, 1.05)
    };

    let base_price = item_base_price_mg(item_id);
    let ratio = ((target_stock.max(1) as f32 + 2.0) / (stock.max(0) as f32 + 2.0)).powf(0.68) * demand_ema.clamp(0.75, 1.45);
    let ratio = ratio.clamp(0.45, 2.60);
    let last_price_mg = (((base_price as f32) * ratio).round() as i32).max(1);

    SettlementMarketCell {
        settlement: settlement.to_string(),
        item_id: item_id.to_string(),
        stock,
        target_stock,
        local_prod_rate,
        local_cons_rate,
        demand_ema,
        last_price_mg,
    }
}

pub fn seed_settlement_markets() -> HashMap<(String, String), SettlementMarketCell> {
    let mut map = HashMap::new();
    let all_items = [
        "mat_reed_bundle",
        "mat_hemp_rope_roll",
        "mat_sackcloth_bolt",
        "mat_sealing_pitch",
        "mat_wet_grass_bundle",
        "mat_rice_straw_bundle",
        "mat_water_moss_sheet",
        "mat_kindling_bundle",
        "mat_rice_husk_sack",
        "mat_wrapping_paper_pack",
        "mat_charcoal_sack",
        "mat_hay_bundle",
        "mat_plank_board",
        "mat_wood_offcut_bundle",
        "mat_fishing_line_bundle",
        "mat_fish_scale_sack",
        "mat_bamboo_slats",
        "mat_fish_bone_sack",
        "mat_rough_ore",
        "mat_washed_ore",
        "mat_stone_powder_sack",
        "mat_iron_scrap_box",
        "mat_iron_scrap",
        "mat_repair_strip",
        "mat_foraged_herb",
        "mat_coarse_paper_ream",
        "mat_coarse_salt_pack",
        "mat_grain_sack",
        "mat_metal_rim_strip",
        "mat_oilcloth_roll",
        "mat_pasture_seed_sack",
        "mat_mane_hair_bundle",
        "mat_feed_mix_sack",
        "mat_leather_strip",
        "mat_canvas_roll",
        "mat_shell_sack",
        "mat_hardtack_sack",
        "craft_steel_hunting_knife",
        "craft_iron_pickaxe",
        "craft_bamboo_oil_umbrella",
        "craft_camphor_storage_box",
        "craft_meridian_salve",
        "craft_calming_tea_pack",
        "craft_pine_soot_ink",
        "craft_peninsula_almanac",
        "craft_storm_canvas_cloak",
        "craft_heavy_leather_boots",
        "craft_pine_torch",
        "craft_wooden_tool_handle",
        "mat_salted_fish",
        "craft_herbal_tea_pack",
        "craft_tool_repair_strip",
        "craft_patched_work_boots",
        "craft_tarred_hemp_rope",
        "craft_caulking_sealant",
        "mat_cedar_plank",
    ];

    for &s in &CANONICAL_SETTLEMENTS {
        for &item_id in &all_items {
            let cell = init_settlement_market_cell(s, item_id);
            map.insert((s.to_string(), item_id.to_string()), cell);
        }
    }
    map
}

pub struct EconomyLedger {
    pub items: HashMap<String, MarketItemState>,
    pub settlement_markets: HashMap<(String, String), SettlementMarketCell>,
    pub recent_events: VecDeque<String>,
    pub active_campfires: HashMap<String, CampfireState>,
    pub active_field_facilities: HashMap<String, Vec<FieldFacilityState>>,
    pub room_furnace_heat: HashMap<String, u32>,
    pub room_structure_repairs: HashMap<String, i64>,
}

impl EconomyLedger {
    pub fn new_seeded() -> Self {
        let mut items = HashMap::new();
        // 原料基準庫存
        items.insert("mat_iron_scrap".to_string(), MarketItemState { stock: 16, target_stock: 25, demand_pressure: 30 });
        items.insert("mat_repair_strip".to_string(), MarketItemState { stock: 12, target_stock: 20, demand_pressure: 25 });
        items.insert("mat_timber_log".to_string(), MarketItemState { stock: 22, target_stock: 30, demand_pressure: 28 });
        items.insert("mat_reed_bundle".to_string(), MarketItemState { stock: 26, target_stock: 25, demand_pressure: 20 });
        items.insert("mat_foraged_herb".to_string(), MarketItemState { stock: 11, target_stock: 20, demand_pressure: 32 });
        items.insert("mat_coarse_paper_ream".to_string(), MarketItemState { stock: 15, target_stock: 20, demand_pressure: 18 });
        items.insert("mat_hemp_rope_roll".to_string(), MarketItemState { stock: 17, target_stock: 22, demand_pressure: 24 });
        items.insert("mat_sackcloth_bolt".to_string(), MarketItemState { stock: 14, target_stock: 20, demand_pressure: 19 });
        items.insert("mat_kindling_bundle".to_string(), MarketItemState { stock: 35, target_stock: 40, demand_pressure: 30 });
        items.insert("mat_plank_board".to_string(), MarketItemState { stock: 20, target_stock: 25, demand_pressure: 25 });
        items.insert("mat_charcoal_sack".to_string(), MarketItemState { stock: 15, target_stock: 20, demand_pressure: 28 });
        items.insert("mat_salted_fish".to_string(), MarketItemState { stock: 18, target_stock: 24, demand_pressure: 20 });
        items.insert("mat_cedar_plank".to_string(), MarketItemState { stock: 14, target_stock: 20, demand_pressure: 18 });
        items.insert("craft_pine_torch".to_string(), MarketItemState { stock: 8, target_stock: 12, demand_pressure: 15 });
        items.insert("craft_wooden_tool_handle".to_string(), MarketItemState { stock: 6, target_stock: 10, demand_pressure: 12 });
        // 成品基準庫存
        items.insert("craft_steel_hunting_knife".to_string(), MarketItemState { stock: 5, target_stock: 10, demand_pressure: 14 });
        items.insert("craft_iron_pickaxe".to_string(), MarketItemState { stock: 4, target_stock: 8, demand_pressure: 10 });
        items.insert("craft_bamboo_oil_umbrella".to_string(), MarketItemState { stock: 8, target_stock: 12, demand_pressure: 12 });
        items.insert("craft_camphor_storage_box".to_string(), MarketItemState { stock: 4, target_stock: 6, demand_pressure: 8 });
        items.insert("craft_meridian_salve".to_string(), MarketItemState { stock: 9, target_stock: 15, demand_pressure: 22 });
        items.insert("craft_calming_tea_pack".to_string(), MarketItemState { stock: 10, target_stock: 15, demand_pressure: 18 });
        items.insert("craft_herbal_tea_pack".to_string(), MarketItemState { stock: 8, target_stock: 14, demand_pressure: 16 });
        items.insert("craft_tool_repair_strip".to_string(), MarketItemState { stock: 7, target_stock: 12, demand_pressure: 14 });
        items.insert("craft_patched_work_boots".to_string(), MarketItemState { stock: 5, target_stock: 8, demand_pressure: 12 });
        items.insert("craft_tarred_hemp_rope".to_string(), MarketItemState { stock: 6, target_stock: 10, demand_pressure: 13 });
        items.insert("craft_caulking_sealant".to_string(), MarketItemState { stock: 6, target_stock: 10, demand_pressure: 15 });
        items.insert("craft_pine_soot_ink".to_string(), MarketItemState { stock: 6, target_stock: 10, demand_pressure: 12 });
        items.insert("craft_peninsula_almanac".to_string(), MarketItemState { stock: 5, target_stock: 8, demand_pressure: 9 });
        items.insert("craft_storm_canvas_cloak".to_string(), MarketItemState { stock: 4, target_stock: 8, demand_pressure: 11 });
        items.insert("craft_heavy_leather_boots".to_string(), MarketItemState { stock: 5, target_stock: 8, demand_pressure: 10 });

        let mut recent_events = VecDeque::new();
        recent_events.push_back("水力聯動重鐵錘日夜趕工，熟鐵料件需求緊俏。".to_string());
        recent_events.push_back("水岸百草藥房焙藥爐爐火通明，精煉舒筋膏廣受青睞。".to_string());
        recent_events.push_back("渾天星軌推步儀日前引發氣脈共鳴，市井傳聞有星紋奇珍現世。".to_string());

        let mut settlement_markets = seed_settlement_markets();

        #[cfg(not(test))]
        let path = economy_ledger_path();
        #[cfg(test)]
        let path = std::path::PathBuf::from("/nonexistent_test_economy_ledger.json");
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(snapshot) = serde_json::from_str::<PersistedEconomySnapshot>(&content) {
                    for mut cell in snapshot.settlement_cells {
                        cell.settlement = normalize_canonical_settlement(&cell.settlement).to_string();
                        cell.stock = cell.stock.clamp(0, (cell.target_stock * 4).max(1));
                        let key = (cell.settlement.clone(), cell.item_id.clone());
                        if let Some(existing) = settlement_markets.get_mut(&key) {
                            existing.stock = cell.stock;
                            existing.target_stock = cell.target_stock;
                            existing.demand_ema = cell.demand_ema;
                            existing.last_price_mg = cell.last_price_mg;
                        } else {
                            settlement_markets.insert(key, cell);
                        }
                    }
                    for (id, mut it_st) in snapshot.items {
                        it_st.stock = it_st.stock.clamp(0, (it_st.target_stock * 4).max(1));
                        if let Some(existing) = items.get_mut(&id) {
                            existing.stock = it_st.stock;
                            existing.target_stock = it_st.target_stock;
                            existing.demand_pressure = it_st.demand_pressure;
                        } else {
                            items.insert(id, it_st);
                        }
                    }
                    if !snapshot.recent_events.is_empty() {
                        recent_events.clear();
                        for ev in snapshot.recent_events {
                            let clean_ev = ev.replace(['|', '｜'], "・");
                            recent_events.push_back(clean_ev);
                        }
                    }
                }
            }
        }

        EconomyLedger {
            items,
            settlement_markets,
            recent_events,
            active_campfires: HashMap::new(),
            active_field_facilities: HashMap::new(),
            room_furnace_heat: HashMap::new(),
            room_structure_repairs: HashMap::new(),
        }
    }
}

pub static LIVE_ECONOMY_LEDGER: LazyLock<RwLock<EconomyLedger>> =
    LazyLock::new(|| RwLock::new(EconomyLedger::new_seeded()));

pub struct MutationArchetype {
    pub key: &'static str,
    pub name_prefix: &'static str,
    pub desc_note: &'static str,
    pub atk_bonus: i32,
    pub vit_bonus: i32,
    pub dex_bonus: i32,
    pub qi_bonus: i32,
    pub price_mult: f64,
}

pub const MUTATION_ARCHETYPES: [MutationArchetype; 6] = [
    MutationArchetype {
        key: "star_resonance",
        name_prefix: "【異變・星紋共鳴】",
        desc_note: "器表浮現細密星軌銀紋，觸手微溫，共鳴大增",
        atk_bonus: 2,
        vit_bonus: 2,
        dex_bonus: 4,
        qi_bonus: 8,
        price_mult: 3.2,
    },
    MutationArchetype {
        key: "thunder_quenched",
        name_prefix: "【異變・雷紋淬火】",
        desc_note: "受大氣游離電脈激盪，凝出暗紫雷紋，揮動有破空聲",
        atk_bonus: 7,
        vit_bonus: 5,
        dex_bonus: 1,
        qi_bonus: 0,
        price_mult: 3.5,
    },
    MutationArchetype {
        key: "frost_condensed",
        name_prefix: "【異變・寒泉凝露】",
        desc_note: "汲深脈寒泉之氣凝結，常溫透出沁涼冰霧，靈效倍增",
        atk_bonus: 0,
        vit_bonus: 6,
        dex_bonus: 3,
        qi_bonus: 10,
        price_mult: 3.0,
    },
    MutationArchetype {
        key: "violet_haze",
        name_prefix: "【異變・紫煙凝魄】",
        desc_note: "千錘百鍊與執念交融，溫潤如玉、水火不侵",
        atk_bonus: 3,
        vit_bonus: 8,
        dex_bonus: 3,
        qi_bonus: 8,
        price_mult: 3.6,
    },
    MutationArchetype {
        key: "crimson_core",
        name_prefix: "【異變・赤焰熔心】",
        desc_note: "爐膛極溫異變，器身隱現赤金火紋，氣血奔湧",
        atk_bonus: 6,
        vit_bonus: 8,
        dex_bonus: 0,
        qi_bonus: 4,
        price_mult: 3.3,
    },
    MutationArchetype {
        key: "tide_crystal",
        name_prefix: "【異變・潮汐晶化】",
        desc_note: "受海風與氣脈交感，結出奇異晶鹽花紋",
        atk_bonus: 1,
        vit_bonus: 4,
        dex_bonus: 6,
        qi_bonus: 6,
        price_mult: 2.8,
    },
];


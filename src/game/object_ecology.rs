//! 物件生命週期（原料 → 成品 → 異變）與自主經濟循環模組。
//!
//! 定義白塔半島全鏈路物質演化：
//! 1. 原料（raw）：採集所得自然物質與粗料。
//! 2. 成品（finished）：工坊匠師以百工技藝切磋琢磨製成之各類器物與靈藥。
//! 3. 異變（mutated）：在天地氣脈共鳴或特定氣場淬火下，引發質理蛻變之神異奇珍。
//! 4. 自主經濟循環（LIVE_ECONOMY_LEDGER）：動態供需庫存、價格浮動與自主工造事件。

use std::collections::{HashMap, VecDeque};
use std::sync::{LazyLock, RwLock};

use serde::{Deserialize, Serialize};

use crate::db;
use crate::store::{self, Item};

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
    "文曜", "風舶角", "水尾寮", "半嶺", "紅土崁", "霜林口", "黑沼關隘", "南哨", "鹽田澳",
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
    if raw_settlement.contains("風舶") || raw_settlement.contains("黑石灣") || raw_settlement.contains("烏石鼻") {
        "風舶角"
    } else if raw_settlement.contains("水尾寮") {
        "水尾寮"
    } else if raw_settlement.contains("半嶺") || raw_settlement.contains("牛埔") {
        "半嶺"
    } else if raw_settlement.contains("紅土崁") {
        "紅土崁"
    } else if raw_settlement.contains("霜林口") || raw_settlement.contains("翠冠林") {
        "霜林口"
    } else if raw_settlement.contains("黑沼關隘") || raw_settlement.contains("青籠關") || raw_settlement.contains("赤石坡") || raw_settlement.contains("三號坑") {
        "黑沼關隘"
    } else if raw_settlement.contains("南哨") || raw_settlement.contains("風櫃尾") {
        "南哨"
    } else if raw_settlement.contains("鹽田澳") {
        "鹽田澳"
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
        "黑沼關隘" | "南哨" => {
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
        "紅土崁" | "霜林口" | "半嶺" | "鹽田澳" => {
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

pub fn init_settlement_market_cell(settlement: &str, item_id: &str) -> SettlementMarketCell {
    let origin_str = lookup_item_origin(item_id);
    let raw_origin = crate::world::materials::infer_origin_settlement(&origin_str);
    let origin_settlement = normalize_canonical_settlement(raw_origin);

    let is_origin = settlement == origin_settlement
        || (item_id == "mat_rough_ore" && (settlement == "紅土崁" || settlement == "黑沼關隘"))
        || (item_id == "mat_iron_scrap" && (settlement == "文曜" || settlement == "紅土崁"))
        || (item_id == "mat_charcoal_sack" && settlement == "霜林口")
        || (item_id == "mat_timber_log" && settlement == "霜林口")
        || (item_id == "mat_plank_board" && settlement == "霜林口")
        || (item_id == "mat_reed_bundle" && settlement == "水尾寮")
        || (item_id == "mat_hemp_rope_roll" && settlement == "水尾寮")
        || (item_id == "mat_sackcloth_bolt" && settlement == "水尾寮")
        || (item_id == "mat_foraged_herb" && settlement == "半嶺")
        || (item_id == "mat_washed_ore" && settlement == "半嶺")
        || (item_id == "mat_coarse_salt_pack" && settlement == "鹽田澳")
        || (item_id == "mat_shell_sack" && settlement == "鹽田澳")
        || (item_id == "mat_hardtack_sack" && settlement == "南哨")
        || (item_id == "mat_fish_scale_sack" && settlement == "風舶角")
        || (item_id == "mat_canvas_roll" && settlement == "風舶角")
        || (item_id == "mat_salted_fish" && settlement == "水尾寮")
        || (item_id == "mat_cedar_plank" && settlement == "霜林口")
        || (item_id == "craft_tool_repair_strip" && settlement == "文曜")
        || (item_id == "craft_herbal_tea_pack" && settlement == "半嶺")
        || (item_id == "craft_patched_work_boots" && settlement == "半嶺")
        || (item_id == "craft_tarred_hemp_rope" && settlement == "風舶角")
        || (item_id == "craft_caulking_sealant" && settlement == "風舶角");

    let (target_stock, stock, local_prod_rate, local_cons_rate, demand_ema): (i32, i32, i32, i32, f32) = if is_origin {
        (10, 16, 3, 1, 0.82)
    } else if is_high_demand_in_settlement(settlement, item_id) {
        (12, 4, 0, 2, 1.20)
    } else {
        (8, 5, 0, 1, 1.05)
    };

    let base_price = crate::world::materials::trade_price_anchor_mg(item_id)
        .or_else(|| {
            for fg in &STANDARD_FINISHED_GOODS {
                if fg.id == item_id {
                    return Some(fg.base_price);
                }
            }
            None
        })
        .unwrap_or(8)
        .max(2);

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
                    for cell in snapshot.settlement_cells {
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
                    for (id, it_st) in snapshot.items {
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
                            recent_events.push_back(ev);
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

pub const STANDARD_FINISHED_GOODS: [FinishedGoodDef; 12] = [
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
        slot: "accessory",
        item_type: "equipment",
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
        description: "以十二味深山藥草慢火熬製提純的濃縮油膏，塗抹經穴溫熱滲透，能快速舒緩筋骨疲憊（使用後恢復體魄與元氣）。",
        origin: "半嶺",
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
        description: "採集水畔仙草與山茶幼芽曬製而成的秘製茶包，沖泡後香氣澄淨甘潤，令人心神大定（使用後深幅恢復元氣）。",
        origin: "半嶺",
    },
    FinishedGoodDef {
        id: "craft_pine_soot_ink",
        name: "松煙紫光精墨錠",
        slot: "accessory",
        item_type: "equipment",
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
        slot: "hold_r",
        item_type: "equipment",
        weight: 1.0,
        atk_bonus: 3,
        vit_bonus: 4,
        dex_bonus: 3,
        base_price: 15,
        description: "經導能木作刨削打磨的堅韌硬木長柄，既可組裝開山鎬與獵刀，亦可作為修補渡船踏板與漁網架的關鍵構件。",
        origin: "霜林口",
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
    if item_id.starts_with("mat_") || item_id.starts_with("harvest_") {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (81..=120).contains(&num) {
                return true;
            }
        }
    }
    let raw_keywords = [
        "礦", "砂", "原木", "蘆草", "竹竿", "草藥", "水苔", "紙刀", "墨坯",
        "麻繩", "袋布", "原牛皮", "鮮河蝦", "水芹菜", "黏土", "石料", "粗麻", "桐油", "松煙", "骨膠",
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
        "文曜"
    } else {
        normalize_canonical_settlement(raw_settlement)
    };

    let origin_str = lookup_item_origin(item_id);
    let raw_origin = crate::world::materials::infer_origin_settlement(&origin_str);
    let origin_settlement = if raw_origin.is_empty() {
        if item_id == "mat_rough_ore" {
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
        format!("【{settlement}】為此物產地，本地市集現貨 {}/{}（{}）", cell.stock, cell.target_stock, status_label)
    } else {
        format!(
            "此物產自【{}】，【{settlement}】本地市集庫存 {}/{}（{}）",
            if origin_settlement.is_empty() { "外地" } else { &origin_settlement },
            cell.stock,
            cell.target_stock,
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
        "文曜"
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

    quote
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
            format!("「{name}」（供應充裕）")
        })
        .collect();

    let surplus_str = if surplus_list.is_empty() {
        "坊間平穩無過剩".to_string()
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
            format!("「{name}」（坊間奇缺）")
        })
        .collect();

    let shortage_str = if shortage_list.is_empty() {
        "市集儲備充足無缺".to_string()
    } else {
        shortage_list.join("、")
    };

    format!("【{s}市集供需實況】充裕供給：{surplus_str} ｜ 緊缺急需：{shortage_str}")
}

/// 查詢指定聚落最緊缺的物料（庫存低於目標且缺口比例最大者），供委託榜與 NPC 收購對話調用。
pub fn scarcest_item_for_settlement(
    settlement_or_room: &str,
) -> Option<(String, String, i32, i32, i32)> {
    let raw_s = crate::world::materials::infer_origin_settlement(settlement_or_room);
    let s = if raw_s.is_empty() {
        normalize_canonical_settlement(settlement_or_room)
    } else {
        normalize_canonical_settlement(raw_s)
    };

    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());

    let mut cells: Vec<&SettlementMarketCell> = ledger
        .settlement_markets
        .values()
        .filter(|c| c.settlement == s)
        .collect();

    if cells.is_empty() {
        return None;
    }

    cells.sort_by(|a, b| {
        let ratio_a = a.stock as f32 / a.target_stock.max(1) as f32;
        let ratio_b = b.stock as f32 / b.target_stock.max(1) as f32;
        ratio_a
            .partial_cmp(&ratio_b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| (b.target_stock - b.stock).cmp(&(a.target_stock - a.stock)))
    });

    let top = cells.first()?;
    let name = get_clean_item_name(&top.item_id);
    Some((
        top.item_id.clone(),
        name,
        top.stock,
        top.target_stock,
        top.last_price_mg,
    ))
}

/// 原料映射至成品規格。
pub fn map_raw_to_finished(raw_id: &str) -> &'static FinishedGoodDef {
    if raw_id.contains("iron")
        || raw_id.contains("scrap")
        || raw_id.contains("metal")
        || raw_id.contains("鐵")
        || raw_id.contains("礦")
        || raw_id.contains("鋼")
    {
        if raw_id.contains("pick") || raw_id.contains("鎬") || raw_id.contains("礦") {
            &STANDARD_FINISHED_GOODS[1] // craft_iron_pickaxe
        } else {
            &STANDARD_FINISHED_GOODS[0] // craft_steel_hunting_knife
        }
    } else if raw_id.contains("timber")
        || raw_id.contains("reed")
        || raw_id.contains("wood")
        || raw_id.contains("竹")
        || raw_id.contains("木")
        || raw_id.contains("蘆")
        || raw_id.contains("柴")
        || raw_id.contains("板")
        || raw_id.contains("柄")
    {
        if raw_id.contains("kindling") || raw_id.contains("torch") || raw_id.contains("柴") {
            &STANDARD_FINISHED_GOODS[10] // craft_pine_torch
        } else if raw_id.contains("plank") || raw_id.contains("handle") || raw_id.contains("板") || raw_id.contains("柄") {
            &STANDARD_FINISHED_GOODS[11] // craft_wooden_tool_handle
        } else if raw_id.contains("box") || raw_id.contains("匣") || raw_id.contains("樟") {
            &STANDARD_FINISHED_GOODS[3] // craft_camphor_storage_box
        } else {
            &STANDARD_FINISHED_GOODS[2] // craft_bamboo_oil_umbrella
        }
    } else if raw_id.contains("herb")
        || raw_id.contains("草")
        || raw_id.contains("藥")
        || raw_id.contains("花")
        || raw_id.contains("茶")
    {
        if raw_id.contains("tea") || raw_id.contains("茶") {
            &STANDARD_FINISHED_GOODS[5] // craft_calming_tea_pack
        } else {
            &STANDARD_FINISHED_GOODS[4] // craft_meridian_salve
        }
    } else if raw_id.contains("paper")
        || raw_id.contains("ink")
        || raw_id.contains("紙")
        || raw_id.contains("墨")
        || raw_id.contains("筆")
    {
        if raw_id.contains("almanac") || raw_id.contains("書") || raw_id.contains("手札") {
            &STANDARD_FINISHED_GOODS[7] // craft_peninsula_almanac
        } else {
            &STANDARD_FINISHED_GOODS[6] // craft_pine_soot_ink
        }
    } else if raw_id.contains("rope")
        || raw_id.contains("sackcloth")
        || raw_id.contains("leather")
        || raw_id.contains("繩")
        || raw_id.contains("布")
        || raw_id.contains("皮")
        || raw_id.contains("帆")
    {
        if raw_id.contains("boot") || raw_id.contains("靴") || raw_id.contains("皮") {
            &STANDARD_FINISHED_GOODS[9] // craft_heavy_leather_boots
        } else {
            &STANDARD_FINISHED_GOODS[8] // craft_storm_canvas_cloak
        }
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
        "mat_rough_ore" => "西南原三號坑口".to_string(),
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
        "mat_shell_sack" => "鹽田澳潮泥碼頭".to_string(),
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
                return format!("{}・{}", cat.origin_material, cat.workshop);
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
        ("mat_coarse_paper_ream", "原漿宣紙刀", "青竹抄紙槽濾出的純淨宣紙原漿紙刀，可裝訂采風手札或拓印長卷。", 0.5),
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
                if existing.origin.is_empty()
                    || existing.origin.contains("北岸")
                    || existing.origin.contains("中鎮")
                    || existing.origin.contains("短市")
                    || existing.origin.contains("小市")
                    || existing.origin.contains("水尾寮渡頭小市")
                    || existing.origin.contains("補給小市")
                    || existing.origin.contains("邊市")
                {
                    existing.origin = item.origin;
                    existing.description = item.description;
                }
            } else {
                s.items.insert(item.id.clone(), item);
            }
        }

        // raw_defs 補缺
        for (id, name, desc, weight) in raw_defs {
            if !s.items.contains_key(id) {
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
        for entry in crate::game::peninsula_catalog::PENINSULA_WORLD_OBJECTS {
            if !s.items.contains_key(entry.id) {
                let (item_type, slot) = match entry.category_num {
                    "08" | "09" | "10" | "11" | "8" | "9" => ("consumable", ""),
                    "12" | "13" => ("armor", "body"),
                    "18" | "19" => ("weapon", "hand"),
                    _ => ("material", ""),
                };
                let base_price = crate::game::peninsula_catalog::catalog_entry_price_mg(entry);
                let weight = crate::game::peninsula_catalog::catalog_entry_weight_kg(entry);
                let origin = format!("{}・{}", entry.origin_material, entry.workshop);
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
                    description: entry.look_text.to_string(),
                    vit_bonus: 0,
                    dex_bonus: 0,
                    atk_bonus: 0,
                    origin,
                };
                s.items.insert(entry.id.to_string(), it);
            }
        }
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
        "龍骨車", "防撞砧", "翻砂箱", "葫蘆吊", "篩架", "砂帶機", "剪板機", "灰窯", "活塞泵", "虎鉗",
        "火烘槽", "軋花機", "千斤頂", "水石輪", "轉盤", "配重模具", "地鑽", "石槽", "大龍骨", "船纜",
        "重錨", "灰膏", "竹筏", "大貨車", "渡船", "划艇", "壓捆機", "推車", "馬鞍架", "吊架",
        "大釜", "砂鍋", "石櫃", "大蒸籠", "重石罐", "醬菜罈", "絞肉機", "飯桶", "銅桶", "豆腐框",
        "座鐘", "巨鐘", "壁鐘", "路燈", "壁燈", "吊燈", "檢查台", "壓床", "水缸", "輪椅", "擔架",
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
        "木刀", "手杖", "打鳥叉", "起子",
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
        "小夜燈", "羅盤", "羅經", "六分儀", "望遠鏡", "觀海雙鏡", "放大鏡", "星軌銅儀", "水平尺", "卡尺",
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
    const CLOAK: &[&str] = &["大氅", "風衣", "雨披", "斗篷"];
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
    const FEET: &[&str] = &["短靴", "長統皮靴", "馬伕靴", "釘鞋", "木屐", "拖鞋", "涼拖", "睡靴", "鞋套"];
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

pub fn lookup_ecology_item_def(item_id: &str) -> Option<crate::db::ItemDefBrief> {
    if let Some((source, quality)) = crate::combat::wordplate::parse_crystal_item_id(item_id) {
        return Some(crate::db::ItemDefBrief {
            name: format!("{}・{}詞元結晶", quality.label_zh(), source.label_zh()),
            item_type: "word_crystal".to_string(),
            slot: String::new(),
            description: format!("自白塔半島{}物產剝名凝結而成的舍利子態結晶（{}）。外顯品階與天地來源，內蘊未顯之真名；運功【碎晶提出】可析出對應來源之裸詞元入【未定字譜（識海試排槽）】，亦可於市集公開交易流通。", source.label_zh(), quality.label_zh()),
            weight: 0.1,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: format!("白塔半島・{}", source.label_zh()),
        });
    }
    if item_id == "blank_manual" || item_id == "wt_blank_manual" {
        return Some(crate::db::ItemDefBrief {
            name: "空白手札".to_string(),
            item_type: "manual".to_string(),
            slot: String::new(),
            description: "裝幀素雅的棉紙手札，可將自身融會貫通之武學套路或百工訣竅著譜立說，流傳於世。".to_string(),
            weight: 0.2,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島・天機院".to_string(),
        });
    }
    if item_id.starts_with("manual_") {
        if let Some(rec) = crate::pg::living_world::get_transcribed_manual(item_id) {
            return Some(crate::db::ItemDefBrief {
                name: format!("《{}》手札", rec.formal_name),
                item_type: "manual".to_string(),
                slot: String::new(),
                description: format!(
                    "由【{}】親手著譜之傳世手札（路數：{}｜字印：{}）。{}",
                    rec.author_name,
                    rec.slot_label,
                    rec.word_chars.join("・"),
                    rec.origin_basis
                ),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: rec.author_name,
            });
        }
        return Some(crate::db::ItemDefBrief {
            name: "傳世手札".to_string(),
            item_type: "manual".to_string(),
            slot: String::new(),
            description: "記載武學行氣路線的手札，字跡古雅。".to_string(),
            weight: 0.3,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "白塔半島".to_string(),
        });
    }
    if item_id.starts_with("corpse_") {
        if let Some(rec) = crate::pg::living_world::get_beast_corpse(item_id) {
            return Some(crate::db::ItemDefBrief {
                name: format!("{}殘軀", rec.creature_title),
                item_type: "corpse".to_string(),
                slot: String::new(),
                description: format!(
                    "尚留餘溫的山海異獸殘軀（{}）。可趁靈韻未散【整軀剝名】一搏高品階結晶，或【肢體分解】取下「{}」等部位。",
                    rec.shanhai_note,
                    rec.primary_loot_name
                ),
                weight: 15.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: rec.creature_title,
            });
        }
        return Some(crate::db::ItemDefBrief {
            name: "異獸殘軀".to_string(),
            item_type: "corpse".to_string(),
            slot: String::new(),
            description: "尚留餘溫的山海異獸殘軀。可趁靈韻未散【整軀剝名】一搏高品階結晶，或【肢體分解】取下「獸皮」等部位。".to_string(),
            weight: 15.0,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: "大荒山海".to_string(),
        });
    }
    if item_id == "craft_wooden_handle" || item_id == "craft_wooden_tool_handle" {
        return Some(crate::db::ItemDefBrief {
            name: "堅韌硬木工具長柄".to_string(),
            item_type: "equipment".to_string(),
            slot: "hold_r".to_string(),
            description: "經導能木作刨削打磨的堅韌硬木長柄，既可組裝開山鎬與獵刀，亦可作為修補渡船踏板與漁網架的關鍵構件。".to_string(),
            weight: 1.0,
            vit_bonus: 4,
            dex_bonus: 3,
            atk_bonus: 3,
            origin: "霜林口".to_string(),
        });
    }
    if item_id == "craft_pine_torch" {
        return Some(crate::db::ItemDefBrief {
            name: "浸脂松明火把".to_string(),
            item_type: "equipment".to_string(),
            slot: "hold_l".to_string(),
            description: "以乾燥柴木浸塗天然松脂紮製而成的導能照明火把，可於暗夜荒野照明驅獸、引燃營火或驅散濕寒。".to_string(),
            weight: 0.8,
            vit_bonus: 3,
            dex_bonus: 4,
            atk_bonus: 4,
            origin: "霜林口".to_string(),
        });
    }
    match item_id {
        "mat_iron_scrap" => {
            return Some(crate::db::ItemDefBrief {
                name: "回收熟鐵料塊".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自高爐鍛坊或鹽鐵倉清整所得的成色純良熟鐵料塊，可於鍛台或百工面板煉製精鋼器具。".to_string(),
                weight: 1.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜萬爐鐵坊棚".to_string(),
            });
        }
        "mat_timber_log" => {
            return Some(crate::db::ItemDefBrief {
                name: "伐採原木".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "紋理細密堅實的乾燥良木，可於木作台或百工面板車製行旅長杖與木器。".to_string(),
                weight: 2.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口林場木棧".to_string(),
            });
        }
        "mat_reed_bundle" => {
            return Some(crate::db::ItemDefBrief {
                name: "水澤老蘆草綑".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自河岸水澤收割晾乾的柔韌蘆草與青竹篾，可編織提籃或防風雨披。".to_string(),
                weight: 0.8,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮河口與老蘆叢".to_string(),
            });
        }
        "mat_foraged_herb" => {
            return Some(crate::db::ItemDefBrief {
                name: "山野鮮藥草".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "帶著晨露清香的鮮嫩草藥，可焙製清心安神藥茶包或熬煉生肌膏。".to_string(),
                weight: 0.3,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "半嶺清泉溪谷".to_string(),
            });
        }
        "mat_coarse_paper_ream" => {
            return Some(crate::db::ItemDefBrief {
                name: "原漿宣紙刀".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "青竹抄紙槽濾出的純淨宣紙原漿紙刀，可裝訂采風手札或拓印長卷。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "文曜東市商行街".to_string(),
            });
        }
        "mat_hemp_rope_roll" => {
            return Some(crate::db::ItemDefBrief {
                name: "浸油粗麻繩卷".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "經桐油浸泡絞合的強韌苧麻繩卷，可編製攀岩繩或加固遠行麻布囊。".to_string(),
                weight: 1.0,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮渡口與漚麻坊".to_string(),
            });
        }
        "mat_kindling_bundle" => {
            return Some(crate::db::ItemDefBrief {
                name: "乾燥柴枝束".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "自山林拾撿劈鋸而成的乾燥細柴枝束，引火極佳，為野地生火、工爐添薪與營火保溫之良材。".to_string(),
                weight: 0.5,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口農舍前坪".to_string(),
            });
        }
        "mat_plank_board" => {
            return Some(crate::db::ItemDefBrief {
                name: "樟松木板料".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "經木作台鋸切刨平的樟松木板料，質地堅韌不易開裂，適用於修補渡船踏板、棧橋與各類建築構件。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口梁氏鋸木坊".to_string(),
            });
        }
        "mat_charcoal_sack" => {
            return Some(crate::db::ItemDefBrief {
                name: "硬木白炭袋".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "以硬木密閉悶燒凝煉的高溫無煙白炭，熱值極高，為鍛爐深層淬火與精煉工造的必備燃料。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "霜林口相思木悶炭窯".to_string(),
            });
        }
        "mat_repair_strip" => {
            return Some(crate::db::ItemDefBrief {
                name: "修補熟鐵薄片".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "輕便平整的修補用熟鐵薄片，可用於修整加固各式器具設施。".to_string(),
                weight: 0.6,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "分家巷機巧修車棚".to_string(),
            });
        }
        "mat_sackcloth_bolt" => {
            return Some(crate::db::ItemDefBrief {
                name: "粗織麻袋布匹".to_string(),
                item_type: "material".to_string(),
                slot: String::new(),
                description: "粗織結實的麻布匹料，可用於縫製耐磨行囊與防風遮棚。".to_string(),
                weight: 1.2,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: "水尾寮渡頭市集與倉棚".to_string(),
            });
        }
        _ => {}
    }
    for g in STANDARD_FINISHED_GOODS {
        if g.id == item_id {
            return Some(crate::db::ItemDefBrief {
                name: g.name.to_string(),
                item_type: g.item_type.to_string(),
                slot: g.slot.to_string(),
                description: g.description.to_string(),
                weight: g.weight,
                vit_bonus: g.vit_bonus,
                dex_bonus: g.dex_bonus,
                atk_bonus: g.atk_bonus,
                origin: g.origin.to_string(),
            });
        }
    }
    if let Some(rest) = item_id.strip_prefix("mut_") {
        for arch in MUTATION_ARCHETYPES {
            let suffix = format!("_{}", arch.key);
            if let Some(base_part) = rest.strip_suffix(&suffix) {
                let base_good = STANDARD_FINISHED_GOODS
                    .iter()
                    .find(|g| g.id == base_part || g.id.starts_with(base_part))
                    .unwrap_or(&STANDARD_FINISHED_GOODS[0]);
                return Some(crate::db::ItemDefBrief {
                    name: format!("{}・{}", arch.name_prefix, base_good.name),
                    item_type: base_good.item_type.to_string(),
                    slot: base_good.slot.to_string(),
                    description: format!(
                        "{}（{}。氣脈異變加成：體魄+{}、身法+{}、攻勢+{}、氣脈+{}）",
                        base_good.description,
                        arch.desc_note,
                        arch.vit_bonus,
                        arch.dex_bonus,
                        arch.atk_bonus,
                        arch.qi_bonus
                    ),
                    weight: base_good.weight,
                    vit_bonus: base_good.vit_bonus + arch.vit_bonus,
                    dex_bonus: base_good.dex_bonus + arch.dex_bonus,
                    atk_bonus: base_good.atk_bonus + arch.atk_bonus,
                    origin: base_good.origin.to_string(),
                });
            }
        }
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        if let Some((item_type, slot, vit_bonus, dex_bonus, atk_bonus)) =
            classify_catalog_wearable_slot(cat.name, cat.category_num, cat.daily_function)
        {
            return Some(crate::db::ItemDefBrief {
                name: cat.name.to_string(),
                item_type: item_type.to_string(),
                slot: slot.to_string(),
                description: cat.look_text.to_string(),
                weight: crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
                vit_bonus,
                dex_bonus,
                atk_bonus,
                origin: format!("{}・{}", cat.origin_material, cat.workshop),
            });
        }
        let (item_type, slot) = match cat.category_num {
            "08" | "09" | "10" | "11" | "8" | "9" => ("consumable", ""),
            _ => ("material", ""),
        };
        return Some(crate::db::ItemDefBrief {
            name: cat.name.to_string(),
            item_type: item_type.to_string(),
            slot: slot.to_string(),
            description: cat.look_text.to_string(),
            weight: crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: format!("{}・{}", cat.origin_material, cat.workshop),
        });
    }
    None
}
/// 玩家或工匠以原料煉製成品，並依條件機率性觸發異變。
pub fn craft_from_raw_material(
    actor_id: &str,
    raw_item_id: &str,
    room_name: &str,
    now_unix: i64,
) -> Result<(String, String, bool, String), String> {
    let Ok(Some(ent)) = db::get_entity(actor_id) else {
        return Err("角色不存在".to_string());
    };
    let inv = db::get_inventory(&ent.inventory, ent.vit);

    let chosen_raw_id = if raw_item_id == "auto" || raw_item_id.trim().is_empty() {
        inv.items
            .iter()
            .find(|it| classify_item_stage(&it.item_id) == "raw")
            .map(|it| it.item_id.clone())
            .ok_or_else(|| {
                "行囊中尚無可用於煉製的原料（可前往河岸、藥圃、碼頭採集，或向攤商收購原料）。".to_string()
            })?
    } else {
        if !db::inventory_has_item(&ent.inventory, raw_item_id) {
            return Err("行囊中未持有指定的原料。".to_string());
        }
        raw_item_id.to_string()
    };

    let mut raw_name = get_clean_item_name(&chosen_raw_id);
    if raw_name.is_ascii() {
        raw_name = crate::world::materials::material_by_item_id(&chosen_raw_id)
            .map(|m| m.name.clone())
            .or_else(|| crate::game::peninsula_catalog::find_catalog_entry(&chosen_raw_id).map(|c| c.name.to_string()))
            .unwrap_or_else(|| "隨身原料".to_string());
        if raw_name.is_ascii() {
            raw_name = "隨身原料".to_string();
        }
    }

    if db::remove_from_inventory(actor_id, &chosen_raw_id, 1).is_err() {
        return Err("扣減原料失敗".to_string());
    }

    let finished_def = map_raw_to_finished(&chosen_raw_id);

    let has_furnace_heat = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.get(room_name).copied().unwrap_or(0) > 0
    };
    let has_charcoal = db::inventory_has_item(&ent.inventory, "mat_charcoal_sack");

    // 檢查異變機率：基礎 35%，工坊或高元氣時提升至 55%，高溫工爐或持有白炭時大幅躍升至 78%
    let is_workshop = room_name.contains("鐵坊")
        || room_name.contains("木作")
        || room_name.contains("印行")
        || room_name.contains("藥房")
        || room_name.contains("織造")
        || room_name.contains("觀星")
        || room_name.contains("工坊")
        || room_name.contains("作坊");
    let threshold = if has_furnace_heat || has_charcoal { 78 } else if is_workshop || ent.qi >= 75 { 55 } else { 35 };

    let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);
    let is_culinary_craft = finished_def.item_type == "food"
        || finished_def.id.contains("tea")
        || finished_def.id.contains("fish")
        || finished_def.id.contains("dish")
        || finished_def.id.contains("soup")
        || room_name.contains("廚")
        || room_name.contains("灶")
        || room_name.contains("食")
        || chosen_raw_id.contains("herb")
        || chosen_raw_id.contains("tea")
        || chosen_raw_id.contains("fish");

    let relevant_craft = if is_culinary_craft {
        craft_dims.culinary
    } else {
        craft_dims.forge
    };
    let craft_mutation_bonus = ((relevant_craft * 0.04).min(0.18) * 100.0).round() as i32;
    let threshold = (threshold + craft_mutation_bonus).min(98);

    let roll = ((now_unix.wrapping_mul(1103515245).wrapping_add(12345) / 65536) % 100).abs() as i32;
    let is_mutated = roll < threshold;

    let (prod_id, prod_name, mut narrative) = if is_mutated {
        let arch_idx = ((now_unix as usize) ^ (chosen_raw_id.len())) % MUTATION_ARCHETYPES.len();
        let arch = &MUTATION_ARCHETYPES[arch_idx];
        let mut_id = format!("mut_{}_{}", finished_def.id, arch.key);
        let mut_name = format!("{}・{}", arch.name_prefix, finished_def.name);
        let desc = format!(
            "{}（{}。靈效加成：體魄+{}、身法+{}、攻勢+{}、氣脈+{}）",
            finished_def.description, arch.desc_note, arch.vit_bonus, arch.dex_bonus, arch.atk_bonus, arch.qi_bonus
        );

        let item = Item {
            id: mut_id.clone(),
            name: mut_name.clone(),
            slot: finished_def.slot.to_string(),
            item_type: finished_def.item_type.to_string(),
            weight: finished_def.weight,
            stackable: 1,
            denomination: (finished_def.base_price as f64 * arch.price_mult).round() as i32,
            description: desc,
            vit_bonus: finished_def.vit_bonus + arch.vit_bonus,
            dex_bonus: finished_def.dex_bonus + arch.dex_bonus,
            atk_bonus: finished_def.atk_bonus + arch.atk_bonus,
            origin: finished_def.origin.to_string(),
        };
        ensure_item_in_store(item);
        let _ = db::add_to_inventory(actor_id, &mut_id, 1);

        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let ev = if mut_name.starts_with('【') {
                format!("百工煉製之際天地交感，成功淬煉出神異之寶{mut_name}！")
            } else {
                format!("百工煉製之際天地交感，成功淬煉出神異之寶【{mut_name}】！")
            };
            ledger.recent_events.push_back(ev);
            if ledger.recent_events.len() > 30 {
                ledger.recent_events.pop_front();
            }
        }
        db::log_npc_event(now_unix, actor_id, "craft_mutation", &mut_name);

        let formatted_mut_name = if mut_name.starts_with('【') {
            mut_name.clone()
        } else {
            format!("【{mut_name}】")
        };
        let narr = format!(
            "你將【{raw_name}】置於工爐中全神貫注煉製。爐火明滅流轉之際，周遭氣脈忽起劇烈共鳴，只見器身神芒驟顯，淬火出世之物赫然引發異變，蛻變為稀世罕見的{formatted_mut_name}！"
        );
        (mut_id, mut_name, narr)
    } else {
        let item = Item {
            id: finished_def.id.to_string(),
            name: finished_def.name.to_string(),
            slot: finished_def.slot.to_string(),
            item_type: finished_def.item_type.to_string(),
            weight: finished_def.weight,
            stackable: if finished_def.slot.is_empty() { 20 } else { 1 },
            denomination: finished_def.base_price,
            description: finished_def.description.to_string(),
            vit_bonus: finished_def.vit_bonus,
            dex_bonus: finished_def.dex_bonus,
            atk_bonus: finished_def.atk_bonus,
            origin: finished_def.origin.to_string(),
        };
        ensure_item_in_store(item);
        let _ = db::add_to_inventory(actor_id, finished_def.id, 1);

        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                entry.stock += 1;
            }
        }

        let narr = format!(
            "你細心運轉工藝，將【{raw_name}】細細切磋打磨淬火，成功煉製出一件質地純良厚重的【{}】收入行囊。",
            finished_def.name
        );
        (finished_def.id.to_string(), finished_def.name.to_string(), narr)
    };

    if craft_dims.forge >= 0.8 || craft_dims.culinary >= 0.8 {
        narrative.push_str(" 【百工藝相共鳴】你周天字印所蘊的藝境氣機自然流轉至指掌，火候與力道拿捏得恰到好處。");
    }

    // 內息底色演進與工藝分類
    let (habit_kind, mentor_name, imprint_title, imprint_prose, slot_label, carrier_code, formula_word_ids, formula_word_chars, station_tool_name) = if finished_def.id.contains("steel")
        || finished_def.id.contains("iron")
        || finished_def.id.contains("metal")
        || finished_def.id.contains("pick")
        || finished_def.id.contains("knife")
        || finished_def.slot == "weapon"
        || room_name.contains("鐵")
        || chosen_raw_id.contains("iron")
        || chosen_raw_id.contains("metal")
        || chosen_raw_id.contains("stone")
        || chosen_raw_id.contains("礦")
    {
        (
            crate::pg::living_world::BreathHabitKind::ForgeHeavy,
            "岑鐵匠",
            "岑氏沉腕落錘法",
            "鐵砧火花明滅間沉腕落錘，將剛猛勁力勻入器胚，剛柔兼濟",
            "重手",
            "blunt",
            vec!["gang".to_string(), "chen_tai".to_string(), "zhong".to_string()],
            vec!["剛".to_string(), "沉".to_string(), "重".to_string()],
            "鍛鐵重錘",
        )
    } else if finished_def.item_type == "food"
        || finished_def.id.contains("tea")
        || finished_def.id.contains("fish")
        || finished_def.id.contains("dish")
        || finished_def.id.contains("soup")
        || room_name.contains("廚")
        || room_name.contains("灶")
        || room_name.contains("食")
        || chosen_raw_id.contains("herb")
        || chosen_raw_id.contains("tea")
        || chosen_raw_id.contains("fish")
    {
        (
            crate::pg::living_world::BreathHabitKind::TideContinuous,
            "溫大娘",
            "三水火候收汁訣",
            "三水調和火候收汁，氣脈如潮汐連綿不絕，滋養臟腑",
            "連攜",
            "tool",
            vec!["shui".to_string(), "huo_shou".to_string(), "he_tai".to_string()],
            vec!["水".to_string(), "火".to_string(), "合".to_string()],
            "庖廚湯杓",
        )
    } else {
        (
            crate::pg::living_world::BreathHabitKind::ChamberClear,
            "柯木匠",
            "柯氏順紋卯合訣",
            "順應木石竹絲紋理卯榫相合，意守方寸凝神聚氣，分毫不差",
            "起手",
            "tool",
            vec!["qiao_shu".to_string(), "mu".to_string(), "he_tai".to_string()],
            vec!["巧".to_string(), "木".to_string(), "合".to_string()],
            "百工刻刀",
        )
    };

    let habit_rec = crate::pg::living_world::record_breath_activity(actor_id, habit_kind, 2, now_unix);
    let vstate = if crate::pg::living_world::is_weapon_or_tool_item(&prod_id, &prod_name) {
        Some(crate::pg::living_world::record_vessel_use(actor_id, &prod_id, 0, 2, now_unix))
    } else {
        None
    };

    let (_imprint_rec, newly_gifted) = crate::pg::living_world::record_mentor_imprint_progress(
        actor_id,
        mentor_name,
        imprint_title,
        imprint_prose,
        now_unix,
    );

    if newly_gifted {
        let _ = db::add_to_inventory(actor_id, "blank_manual", 1);
        let mut hasher = 0xcbf29ce484222325u64;
        for b in format!("{}_{}", mentor_name, imprint_title).bytes() {
            hasher ^= b as u64;
            hasher = hasher.wrapping_mul(0x100000001b3);
        }
        let manual_hash = format!("{:016x}", hasher);
        let manual_rec = crate::pg::living_world::TranscribedManualRecord {
            manual_hash: manual_hash.clone(),
            author_id: mentor_name.to_string(),
            author_name: mentor_name.to_string(),
            formal_name: imprint_title.to_string(),
            slot_label: slot_label.to_string(),
            carrier_code: carrier_code.to_string(),
            word_ids: formula_word_ids.clone(),
            word_chars: formula_word_chars.clone(),
            origin_basis: imprint_prose.to_string(),
            created_at: now_unix,
        };
        crate::pg::living_world::save_transcribed_manual(&manual_rec);
        let manual_item_id = format!("manual_{manual_hash}");
        let _ = db::add_to_inventory(actor_id, &manual_item_id, 1);

        let word_id_refs: Vec<&str> = formula_word_ids.iter().map(|s| s.as_str()).collect();
        crate::server::handler::inventory::grant_words_to_player_casket(actor_id, &word_id_refs);

        narrative.push_str(&format!(
            " {}見你手法沉穩老練，眼中滿是讚許之色，微微頷首道：「此道你已深得精髓。」說罷贈予你一卷親筆手書的《{}》手札與空白手札，並傳下「{}」字印真意，以供日後參悟傳承。",
            mentor_name,
            imprint_title,
            formula_word_chars.join("・")
        ));
    }

    let is_high_tier = is_mutated || finished_def.base_price >= 100 || !finished_def.slot.is_empty();
    if is_high_tier {
        let actor_name = if !ent.display_title.trim().is_empty() {
            ent.display_title.trim().to_string()
        } else {
            "百工匠人".to_string()
        };
        let tool_familiarity = vstate.as_ref().map(|vs| vs.familiarity().label().to_string()).unwrap_or_else(|| "得心應手".to_string());
        let packet = crate::ai::CraftActionFactPacket {
            actor_name,
            tool_name: station_tool_name.to_string(),
            tool_familiarity,
            breath_habit: habit_rec.dominant.label().to_string(),
            material_names: vec![raw_name.clone()],
            output_name: prod_name.clone(),
            highlight_kind: if is_mutated { "器物異變" } else { "精工淬煉" }.to_string(),
            environment_hint: if room_name.is_empty() { "工坊作坊" } else { room_name }.to_string(),
        };
        let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
        if !highlight.is_empty() {
            narrative.push_str("\n\n");
            narrative.push_str(&highlight);
        }
    }

    Ok((prod_id, prod_name, is_mutated, narrative))
}

/// 以氣脈淬煉已成之器物，引導其發生質理異變。
pub fn mutate_finished_item(
    actor_id: &str,
    item_id: &str,
    _room_name: &str,
    now_unix: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(actor_id) else {
        return Err("角色不存在".to_string());
    };
    if ent.qi < 10 {
        return Err("元氣不足（至少需要 10 點元氣方能催動氣脈共鳴進行異變）。".to_string());
    }

    let inv = db::get_inventory(&ent.inventory, ent.vit);

    let chosen_item_id = if item_id == "auto" || item_id.trim().is_empty() {
        inv.items
            .iter()
            .find(|it| classify_item_stage(&it.item_id) == "finished")
            .or_else(|| inv.items.iter().find(|it| classify_item_stage(&it.item_id) == "mutated"))
            .map(|it| it.item_id.clone())
            .ok_or_else(|| "行囊中尚無可用於氣脈異變的工造成品。".to_string())?
    } else {
        if !db::inventory_has_item(&ent.inventory, item_id) {
            return Err("行囊中未持有指定的成品物件。".to_string());
        }
        item_id.to_string()
    };

    let has_furnace_heat = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.get(_room_name).copied().unwrap_or(0) > 0
    };
    let has_charcoal = db::inventory_has_item(&ent.inventory, "mat_charcoal_sack");

    let is_workshop = _room_name.contains("鐵坊")
        || _room_name.contains("木作")
        || _room_name.contains("印行")
        || _room_name.contains("藥房")
        || _room_name.contains("織造")
        || _room_name.contains("觀星")
        || _room_name.contains("工坊")
        || _room_name.contains("作坊");
    let threshold = if has_furnace_heat || has_charcoal { 78 } else if is_workshop || ent.qi >= 75 { 55 } else { 35 };

    let roll = ((now_unix.wrapping_mul(1103515245).wrapping_add(12345) / 65536) % 100).abs() as i32;
    if roll >= threshold {
        return Err("氣脈共鳴未臻極致，未能引發質理異變（可藉由工爐添柴增熱、攜帶硬木白炭或至工坊重試）。".to_string());
    }
    // 扣除 10 點元氣
    if let Some(arc) = store::get_store() {
        let _ = arc.write().update_entity(actor_id, |e| {
            e.qi = (e.qi - 10).max(0);
        });
    }

    if db::remove_from_inventory(actor_id, &chosen_item_id, 1).is_err() {
        return Err("扣減物件失敗".to_string());
    }

    // 取得基礎成品資訊
    let base_def = db::get_item_def(&chosen_item_id);
    let mut base_clean_id = chosen_item_id.trim_start_matches("mut_").to_string();
    for a in MUTATION_ARCHETYPES {
        let suf = format!("_{}", a.key);
        if let Some(stripped) = base_clean_id.strip_suffix(&suf) {
            base_clean_id = stripped.to_string();
        }
    }
    let resolved_base_name = base_def
        .as_ref()
        .map(|d| d.name.clone())
        .or_else(|| crate::world::materials::material_by_item_id(&chosen_item_id).map(|m| m.name.clone()))
        .or_else(|| crate::game::peninsula_catalog::find_catalog_entry(&chosen_item_id).map(|c| c.name.to_string()))
        .unwrap_or_else(|| {
            if chosen_item_id.is_ascii() {
                "隨身器物".to_string()
            } else {
                chosen_item_id.clone()
            }
        });
    let base_name = if resolved_base_name.is_ascii() {
        "隨身器物".to_string()
    } else {
        resolved_base_name
    };
    let base_clean_name = base_name
        .replace("【異變・星紋共鳴】・", "")
        .replace("【異變・雷紋淬火】・", "")
        .replace("【異變・寒泉凝露】・", "")
        .replace("【異變・紫煙凝魄】・", "")
        .replace("【異變・赤焰熔心】・", "")
        .replace("【異變・潮汐晶化】・", "")
        .replace("【異變・星紋共鳴】", "")
        .replace("【異變・雷紋淬火】", "")
        .replace("【異變・寒泉凝露】", "")
        .replace("【異變・紫煙凝魄】", "")
        .replace("【異變・赤焰熔心】", "")
        .replace("【異變・潮汐晶化】", "");

    // 選擇異變原型（避免與原先重複）
    let mut arch_idx = ((now_unix as usize) ^ (chosen_item_id.len())) % MUTATION_ARCHETYPES.len();
    if chosen_item_id.contains(MUTATION_ARCHETYPES[arch_idx].key) {
        arch_idx = (arch_idx + 1) % MUTATION_ARCHETYPES.len();
    }
    let arch = &MUTATION_ARCHETYPES[arch_idx];

    let mut_id = format!("mut_{}_{}", base_clean_id, arch.key);
    let mut_name = format!("{}・{}", arch.name_prefix, base_clean_name.trim());

    let base_desc = base_def.as_ref().map(|d| d.description.as_str()).unwrap_or("做工精湛的工造之物。");
    let desc = format!(
        "{}（{}。氣脈異變加成：體魄+{}、身法+{}、攻勢+{}、氣脈+{}）",
        base_desc, arch.desc_note, arch.vit_bonus, arch.dex_bonus, arch.atk_bonus, arch.qi_bonus
    );

    let (slot, item_type, weight, base_price, base_vit, base_dex, base_atk) = if let Some(d) = &base_def {
        (d.slot.clone(), d.item_type.clone(), d.weight, 25, d.vit_bonus, d.dex_bonus, d.atk_bonus)
    } else {
        ("hold_r".to_string(), "equipment".to_string(), 1.0, 25, 0, 0, 0)
    };

    let item = Item {
        id: mut_id.clone(),
        name: mut_name.clone(),
        slot,
        item_type,
        weight,
        stackable: 1,
        denomination: (base_price as f64 * arch.price_mult).round() as i32,
        description: desc,
        vit_bonus: base_vit + arch.vit_bonus,
        dex_bonus: base_dex + arch.dex_bonus,
        atk_bonus: base_atk + arch.atk_bonus,
        origin: base_def.as_ref().map(|d| d.origin.clone()).unwrap_or_else(|| lookup_item_origin(&base_clean_id)),
    };
    ensure_item_in_store(item);
    let _ = db::add_to_inventory(actor_id, &mut_id, 1);

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let ev = if mut_name.starts_with('【') {
            format!("{mut_name}在氣脈共鳴中破繭出世！")
        } else {
            format!("【{mut_name}】在氣脈共鳴中破繭出世！")
        };
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }
    db::log_npc_event(now_unix, actor_id, "item_mutation", &mut_name);

    let narrative = format!(
        "你凝神調息，將十點元氣徐徐注入【{base_clean_name}】中。隨行氣勢激盪，天地與器物深層質理產生極強共振，微芒自器表寸寸漫溢，赫然蛻變為神異斐然的{mut_name}！"
    );
    Ok((mut_id, mut_name, narrative))
}

/// 自世界物件或地景採集資源時，結合百工四維、吐納底色演進、工具磨損磨合與器況回饋。
/// 回傳 `(額外獲得份數, 質性敘事加註)`。
pub fn harvest_from_world_object(
    actor_id: &str,
    target_id: &str,
    target_name: &str,
    mat_id: &str,
    now_unix: i64,
) -> (i32, String) {
    let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);

    // 依採集物產類型演進吐納底色
    let low_mat = mat_id.to_lowercase();
    let low_tgt_id = target_id.to_lowercase();
    let is_ore = low_mat.contains("ore")
        || low_mat.contains("stone")
        || low_mat.contains("crystal")
        || low_mat.contains("clay")
        || low_mat.contains("salt")
        || low_tgt_id.contains("ore")
        || low_tgt_id.contains("stone")
        || low_tgt_id.contains("crystal")
        || target_name.contains("礦")
        || target_name.contains("石")
        || target_name.contains("岩")
        || target_name.contains("晶")
        || target_name.contains("鐵")
        || target_name.contains("銅")
        || target_name.contains("煤")
        || target_name.contains("玉")
        || target_name.contains("砂")
        || target_name.contains("鹽");

    let is_tide = low_mat.contains("reed")
        || low_mat.contains("water")
        || low_mat.contains("fish")
        || low_tgt_id.contains("water")
        || low_tgt_id.contains("fish")
        || target_name.contains("水")
        || target_name.contains("泉")
        || target_name.contains("葦")
        || target_name.contains("蘆")
        || target_name.contains("渡")
        || target_name.contains("岸")
        || target_name.contains("魚")
        || target_name.contains("海");

    let habit_kind = if is_ore {
        crate::pg::living_world::BreathHabitKind::ForgeHeavy
    } else if is_tide {
        crate::pg::living_world::BreathHabitKind::TideContinuous
    } else {
        crate::pg::living_world::BreathHabitKind::ForestLong
    };

    crate::pg::living_world::record_breath_activity(actor_id, habit_kind, 1, now_unix);

    // 檢視角色所裝備之武器或工器具
    let tool_id_opt = db::get_entity(actor_id).ok().flatten().and_then(|ent| {
        let map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&ent.equipment_slots).ok()?;
        for key in ["weapon", "hold_r", "hand", "tool", "hold_l"] {
            if let Some(s) = map.get(key).and_then(|v| v.as_str())
                && !s.is_empty()
            {
                return Some(s.to_string());
            }
        }
        None
    });

    let mut vstate_opt = None;
    if let Some(tool_id) = &tool_id_opt {
        let tool_name = get_clean_item_name(tool_id);
        if crate::pg::living_world::is_weapon_or_tool_item(tool_id, &tool_name) {
            let vs = crate::pg::living_world::record_vessel_use(actor_id, tool_id, 1, 1, now_unix);
            vstate_opt = Some(vs);
        }
    }

    let mut extra_granted = 0;
    let mut note = String::new();

    let is_rolled = vstate_opt.as_ref().map(|vs| vs.condition == crate::pg::living_world::VesselCondition::RolledEdge).unwrap_or(false);
    let is_familiar = vstate_opt.as_ref().map(|vs| {
        matches!(
            vs.familiarity(),
            crate::pg::living_world::VesselFamiliarity::WellBalanced
                | crate::pg::living_world::VesselFamiliarity::LikeOwnArm
        )
    }).unwrap_or(false);

    if is_rolled {
        note.push_str("（手頭器物刃口捲缺打滑，難以順暢切削，宜前往鐵坊以火候重新回火推磨 ⚒︎）");
    } else if craft_dims.harvest >= 1.0 || is_familiar {
        let _ = db::add_to_inventory(actor_id, mat_id, 1);
        extra_granted = 1;
        note.push_str("手頭器物趁手加上【採集剝名】藝相純熟，順著紋理多採得一份完好物材。");
    }

    (extra_granted, note)
}

/// 運行單步 NPC 自主經濟與工造巡檢。
pub fn run_npc_autonomous_production_step(
    _room_id: &str,
    room_name: &str,
    npc_ids: &[String],
    now_unix: i64,
) -> Option<String> {
    if npc_ids.is_empty() {
        return None;
    }
    let npc_id = &npc_ids[((now_unix as usize) % npc_ids.len())];
    let (npc_name, ent_inventory) = if let Some(ent) = db::get_entity(npc_id).ok().flatten() {
        let name = if npc_id.starts_with("wt_vendor_") {
            ent.display_title
                .split(['|', '｜'])
                .next()
                .unwrap_or(&ent.display_title)
                .trim()
                .to_string()
        } else {
            let tag = crate::store::npc_brief_appearance_tag(npc_id, &ent.display_title);
            let role = crate::store::npc_persistent_role_label(npc_id, &ent.display_title);
            format!("那名{}的{}", tag, role)
        };
        (name, ent.inventory)
    } else {
        (format!("工坊匠師【{npc_id}】"), "[]".to_string())
    };

    // Step A: 原料採集 (渡口/水岸/河/碼頭/藥圃/竹林/山道/埠階/海/林)
    let is_gathering_site = room_name.contains("渡口")
        || room_name.contains("水岸")
        || room_name.contains("河")
        || room_name.contains("碼頭")
        || room_name.contains("藥圃")
        || room_name.contains("竹林")
        || room_name.contains("山道")
        || room_name.contains("埠階")
        || room_name.contains("海")
        || room_name.contains("田")
        || room_name.contains("林");

    // Step B: 工坊工造與異變
    // 排除哨所、巡防、居所、橋樑等非工坊設施，並剝離外坊等行政區域名稱後判定是否為工坊
    let is_non_workshop_facility = room_name.contains("哨所")
        || room_name.contains("巡河")
        || room_name.contains("居")
        || room_name.contains("宅")
        || room_name.contains("橋")
        || room_name.contains("碼頭")
        || room_name.contains("渡口");

    let clean_workshop_site_name = room_name
        .replace("三水外坊", "")
        .replace("講堂外坊", "")
        .replace("百工外坊", "")
        .replace("西門外坊", "");

    let is_workshop_site = !is_non_workshop_facility
        && (clean_workshop_site_name.contains("鐵坊")
            || clean_workshop_site_name.contains("鍛造")
            || clean_workshop_site_name.contains("木作")
            || clean_workshop_site_name.contains("傢俱")
            || clean_workshop_site_name.contains("巧木")
            || clean_workshop_site_name.contains("印行")
            || clean_workshop_site_name.contains("造紙")
            || clean_workshop_site_name.contains("抄紙")
            || clean_workshop_site_name.contains("染坊")
            || clean_workshop_site_name.contains("織坊")
            || clean_workshop_site_name.contains("藥坊")
            || clean_workshop_site_name.contains("烘藥")
            || clean_workshop_site_name.contains("製藥")
            || clean_workshop_site_name.contains("工坊")
            || clean_workshop_site_name.contains("工寮")
            || clean_workshop_site_name.contains("石硯")
            || clean_workshop_site_name.contains("窯")
            || clean_workshop_site_name.contains("作")
            || clean_workshop_site_name.contains("匠")
            || clean_workshop_site_name.contains("爐")
            || clean_workshop_site_name.contains("印")
            || clean_workshop_site_name.contains("藥")
            || clean_workshop_site_name.contains("織")
            || clean_workshop_site_name.contains("造")
            || clean_workshop_site_name.contains("院")
            || clean_workshop_site_name.contains("館"));
    if is_gathering_site && !is_workshop_site {
        let (raw_id, raw_name) = if room_name.contains("藥") || room_name.contains("圃") || room_name.contains("田") {
            ("mat_foraged_herb", "野生採集草藥")
        } else if room_name.contains("竹") || room_name.contains("林") || room_name.contains("木") {
            ("mat_timber_log", "伐採原木")
        } else if room_name.contains("渡") || room_name.contains("岸") || room_name.contains("葦") {
            ("mat_reed_bundle", "水澤老蘆草綑")
        } else if room_name.contains("紙") {
            ("mat_coarse_paper_ream", "粗制竹麻紙料")
        } else {
            ("mat_iron_scrap", "回收熟鐵料塊")
        };

        let _ = db::add_to_inventory(npc_id, raw_id, 1);
        let cur_settlement = normalize_canonical_settlement(crate::world::materials::infer_origin_settlement(room_name));
        {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            if let Some(entry) = ledger.items.get_mut(raw_id) {
                entry.stock += 1;
            }
            let key = (cur_settlement.to_string(), raw_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get_mut(&key) {
                cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
            }
            save_economy_ledger_snapshot(&ledger);
        }

        let lines = [
            format!("{npc_name}手持竹編背簍在水岸草澤間巡視，俐落地割取一捆生機盎然的【{raw_name}】，仔細紮束後妥善收進行囊。"),
            format!("{npc_name}彎腰檢視埠頭旁的物料堆，將幾包質地齊整的【{raw_name}】清點分裝，額角微汗，動作甚是熟練。"),
            format!("{npc_name}在晨光水氣中採集整理【{raw_name}】，抖落泥水細心收納，準備稍後送往集市與工坊。"),
        ];
        let narr = lines[((now_unix as usize) % lines.len())].clone();
        return Some(narr);
    }

    if is_workshop_site {
        let (raw_id, raw_name, finished_def) = if room_name.contains("鐵") || room_name.contains("鍛") {
            ("mat_iron_scrap", "回收熟鐵料塊", &STANDARD_FINISHED_GOODS[0])
        } else if room_name.contains("木") || room_name.contains("竹") {
            ("mat_timber_log", "伐採原木", &STANDARD_FINISHED_GOODS[2])
        } else if room_name.contains("藥") {
            ("mat_foraged_herb", "野生採集草藥", &STANDARD_FINISHED_GOODS[4])
        } else if room_name.contains("印") || room_name.contains("紙") || room_name.contains("墨") {
            ("mat_coarse_paper_ream", "粗制竹麻紙料", &STANDARD_FINISHED_GOODS[6])
        } else {
            ("mat_hemp_rope_roll", "熟桐浸油麻繩圈", &STANDARD_FINISHED_GOODS[8])
        };

        let cur_settlement = normalize_canonical_settlement(crate::world::materials::infer_origin_settlement(room_name));
        let npc_has_raw = db::inventory_has_item(&ent_inventory, raw_id) || db::inventory_has_item(npc_id, raw_id);

        let raw_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            let cons_key = (cur_settlement.to_string(), raw_id.to_string());
            ledger.settlement_markets.get(&cons_key).map(|c| c.stock).unwrap_or(0)
        };

        if raw_stock <= 0 && !npc_has_raw {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let cons_key = (cur_settlement.to_string(), raw_id.to_string());
            if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                cell.demand_ema = (cell.demand_ema * 1.12).min(1.65);
            }
            let shortage_ev = format!("【工坊待料】{cur_settlement}工坊因缺乏「{raw_name}」暫緩開爐，當地收購行情急升！");
            ledger.recent_events.push_back(shortage_ev);
            if ledger.recent_events.len() > 30 {
                ledger.recent_events.pop_front();
            }
            save_economy_ledger_snapshot(&ledger);
            return Some(format!("{npc_name}檢視工坊料架，發現【{raw_name}】已經見底，只得暫停手頭工序，向門外張望是否有行商或旅人運料前來。"));
        }

        if npc_has_raw {
            let _ = db::remove_from_inventory(npc_id, raw_id, 1);
        }
        // 異變檢驗：NPC 工坊製作 25% 異變機率
        let roll = ((now_unix.wrapping_mul(1664525).wrapping_add(1013904223) / 65536) % 100).abs() as i32;
        let is_mut = roll < 25;

        if is_mut {
            let arch_idx = ((now_unix as usize) + npc_name.len()) % MUTATION_ARCHETYPES.len();
            let arch = &MUTATION_ARCHETYPES[arch_idx];
            let mut_id = format!("mut_{}_{}", finished_def.id, arch.key);
            let mut_name = format!("{}・{}", arch.name_prefix, finished_def.name);

            let desc = format!(
                "{}（{}。靈效加成：體魄+{}、身法+{}、攻勢+{}、氣脈+{}）",
                finished_def.description, arch.desc_note, arch.vit_bonus, arch.dex_bonus, arch.atk_bonus, arch.qi_bonus
            );

            let item = Item {
                id: mut_id.clone(),
                name: mut_name.clone(),
                slot: finished_def.slot.to_string(),
                item_type: finished_def.item_type.to_string(),
                weight: finished_def.weight,
                stackable: 1,
                denomination: (finished_def.base_price as f64 * arch.price_mult).round() as i32,
                description: desc,
                vit_bonus: finished_def.vit_bonus + arch.vit_bonus,
                dex_bonus: finished_def.dex_bonus + arch.dex_bonus,
                atk_bonus: finished_def.atk_bonus + arch.atk_bonus,
                origin: finished_def.origin.to_string(),
            };
            ensure_item_in_store(item);
            let _ = db::add_to_inventory(npc_id, &mut_id, 1);
            let _ = crate::server::handler::npc_autonomous_word_and_equip_step(
                npc_id,
                &npc_name,
                &mut_id,
                &mut_name,
            );

            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let cons_key = (cur_settlement.to_string(), raw_id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                    cell.stock = (cell.stock - 1).max(0);
                    cell.demand_ema = (cell.demand_ema * 1.05).min(1.45);
                }
                if let Some(entry) = ledger.items.get_mut(raw_id) {
                    entry.stock = (entry.stock - 1).max(0);
                }
                let prod_key = (cur_settlement.to_string(), finished_def.id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&prod_key) {
                    cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
                }
                if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                    entry.stock += 1;
                }
                let ev = if mut_name.starts_with('【') {
                    format!("{npc_name}於工坊開爐煉製，引發氣脈異變造就{mut_name}！")
                } else {
                    format!("{npc_name}於工坊開爐煉製，引發氣脈異變造就【{mut_name}】！")
                };
                ledger.recent_events.push_back(ev);
                if ledger.recent_events.len() > 30 {
                    ledger.recent_events.pop_front();
                }
                save_economy_ledger_snapshot(&ledger);
            }
            db::log_npc_event(now_unix, npc_id, "npc_workshop_mutation", &mut_name);
            let formatted_mut = if mut_name.starts_with('【') {
                mut_name.clone()
            } else {
                format!("【{mut_name}】")
            };
            let lines = [
                format!("{npc_name}專注拉動風箱、揮動鐵錘捶打器坯；只見火星迸散之際，氣脈流轉激盪，方成型的【{}】竟凝出奇異神彩，化為{formatted_mut}！", finished_def.name),
                format!("{npc_name}守在焙藥爐前細心掌控火候，藥膏收汁成形時，濃郁藥香伴隨冰晶霧氣凝結，蛻變出罕見的{formatted_mut}！"),
            ];
            return Some(lines.join(" "));
        } else {
            let item = Item {
                id: finished_def.id.to_string(),
                name: finished_def.name.to_string(),
                slot: finished_def.slot.to_string(),
                item_type: finished_def.item_type.to_string(),
                weight: finished_def.weight,
                stackable: if finished_def.slot.is_empty() { 20 } else { 1 },
                denomination: finished_def.base_price,
                description: finished_def.description.to_string(),
                vit_bonus: finished_def.vit_bonus,
                dex_bonus: finished_def.dex_bonus,
                atk_bonus: finished_def.atk_bonus,
                origin: finished_def.origin.to_string(),
            };
            ensure_item_in_store(item);
            let _ = db::add_to_inventory(npc_id, finished_def.id, 1);

            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let prod_key = (cur_settlement.to_string(), finished_def.id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&prod_key) {
                    cell.stock = (cell.stock + 1).min(cell.target_stock * 4);
                }
                if let Some(entry) = ledger.items.get_mut(finished_def.id) {
                    entry.stock += 1;
                }
                let cons_key = (cur_settlement.to_string(), raw_id.to_string());
                if let Some(cell) = ledger.settlement_markets.get_mut(&cons_key) {
                    cell.stock = (cell.stock - 1).max(0);
                    cell.demand_ema = (cell.demand_ema * 1.05).min(1.45);
                }
                if let Some(entry) = ledger.items.get_mut(raw_id) {
                    entry.stock = (entry.stock - 1).max(0);
                }
                save_economy_ledger_snapshot(&ledger);
            }
            db::log_npc_event(now_unix, npc_id, "npc_workshop_craft", finished_def.name);
            let craft_narrative = if raw_id == "mat_iron_scrap"
                || finished_def.name.contains('刀')
                || finished_def.name.contains('錘')
                || finished_def.name.contains('鎬')
                || finished_def.name.contains('燈')
            {
                format!("{npc_name}揮動鍛錘在鐵砧上反覆敲打修整，火星迸落間，一件結實耐用的【{}】已然鍛打停當。", finished_def.name)
            } else if raw_id == "mat_timber_log"
                || raw_id == "mat_reed_bundle"
                || finished_def.name.contains('杖')
                || finished_def.name.contains('碗')
                || finished_def.name.contains('籃')
                || finished_def.name.contains('匣')
                || finished_def.name.contains('傘')
                || finished_def.name.contains('披')
            {
                format!("{npc_name}專心致志地推拉刨刀與竹篾刀，碎屑紛飛間，一件做工講究的【{}】已然打磨編製停當。", finished_def.name)
            } else if raw_id == "mat_foraged_herb"
                || finished_def.name.contains('茶')
                || finished_def.name.contains('膏')
            {
                format!("{npc_name}守在藥焙爐旁細心翻動百草、文火慢熬，清香瀰漫間，一份成色純正的【{}】已然焙製停當。", finished_def.name)
            } else if raw_id == "mat_coarse_paper_ream"
                || finished_def.name.contains("手札")
                || finished_def.name.contains("長卷")
                || finished_def.name.contains('墨')
            {
                format!("{npc_name}在案前細心裁紙、壓平裝幀，墨香微漾間，一冊裝訂齊整的【{}】已然整備停當。", finished_def.name)
            } else if raw_id == "mat_hemp_rope_roll"
                || finished_def.name.contains('繩')
                || finished_def.name.contains('囊')
                || finished_def.name.contains("斗篷")
                || finished_def.name.contains('靴')
            {
                format!("{npc_name}手持骨針與浸油麻線飛針走線、刷上熟桐防雨油，片刻後一件厚實耐用的【{}】已然縫製停當。", finished_def.name)
            } else {
                format!("{npc_name}按部就班操持工具，耐心地將【{raw_name}】切磋琢磨，片刻後一件結實耐用的【{}】順利完工。", finished_def.name)
            };
            let mut msg = craft_narrative;
            if let Some(note) = crate::server::handler::npc_autonomous_word_and_equip_step(
                npc_id,
                &npc_name,
                &finished_def.id,
                finished_def.name,
            ) {
                msg.push(' ');
                msg.push_str(&note);
            }
            return Some(msg);
        }
    }

    // Step C: 商市交易與動態調價 (市/街/鋪/行/商/棧/堂/口/場)
    // Step C: 自主行商跨區調撥與商市交易
    {
        let cur_settlement = normalize_canonical_settlement(crate::world::materials::infer_origin_settlement(room_name));
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        execute_settlement_sinks_and_arbitrage(cur_settlement, now_unix, &mut ledger);
        save_economy_ledger_snapshot(&ledger);
    }

    let is_low_stock = {
        let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
        ledger.items.values().any(|st| st.stock < (st.target_stock * 7 / 10))
            || ledger.settlement_markets.values().any(|cell| cell.stock < (cell.target_stock * 7 / 10))
    };

    if is_low_stock {
        crate::store::open_active_npc_obligation(npc_id, "restock_goods", "鋪內庫存見底急需提貨補倉");
        let lines = [
            format!("{npc_name}翻閱案頭的進出貨簿子，發覺幾樣要緊物料庫存吃緊，隨即在簿上記下一筆補貨清單。"),
            format!("{npc_name}在店鋪架前來回巡視，核對今日市價行情，低聲向身旁學徒交代稍後前往工坊再添一批現貨。"),
        ];
        Some(lines[(now_unix as usize) % lines.len()].clone())
    } else {
        let lines = [
            format!("{npc_name}熟練地核對市秤與貨品成色，當場點清貨款收妥，彼此言談爽利，交易順遂。"),
            format!("{npc_name}向來往鄉鄰展示新到的工造物件，雙方爽朗核對價錢，乾脆俐落地結清貨款。"),
        ];
        Some(lines[(now_unix as usize) % lines.len()].clone())
    }
}

fn get_clean_item_name(item_id: &str) -> String {
    if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
        if !mat.name.is_empty() {
            return mat.name.clone();
        }
    }
    for def in &STANDARD_FINISHED_GOODS {
        if def.id == item_id {
            return def.name.to_string();
        }
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        return cat.name.to_string();
    }
    if let Some(eco) = lookup_ecology_item_def(item_id) {
        if !eco.name.is_empty() {
            return eco.name;
        }
    }
    match item_id {
        "mat_iron_scrap" => "熟鐵料塊".to_string(),
        "mat_repair_strip" => "修補鐵片".to_string(),
        "mat_timber_log" => "伐採原木".to_string(),
        "mat_reed_bundle" => "老蘆草綑".to_string(),
        "mat_foraged_herb" => "野生草藥".to_string(),
        "mat_coarse_paper_ream" => "粗竹麻紙".to_string(),
        "mat_hemp_rope_roll" => "浸油麻繩".to_string(),
        "mat_sackcloth_bolt" => "舊麻袋布".to_string(),
        "mat_kindling_bundle" => "柴枝束".to_string(),
        "mat_plank_board" => "木板料".to_string(),
        "mat_charcoal_sack" => "白炭袋".to_string(),
        "mat_salted_fish" => "鹽漬鹹魚".to_string(),
        "craft_herbal_tea_pack" => "百草煎茶包".to_string(),
        "craft_tool_repair_strip" => "工具修補條".to_string(),
        "craft_patched_work_boots" => "補綴工靴".to_string(),
        "craft_tarred_hemp_rope" => "塗焦麻繩".to_string(),
        "craft_caulking_sealant" => "嵌縫膠泥".to_string(),
        "mat_cedar_plank" => "杉木厚板".to_string(),
        "craft_pine_torch" => "松明火把".to_string(),
        "craft_wooden_tool_handle" => "工具木柄".to_string(),
        other => {
            if other.is_ascii() {
                "隨身原料".to_string()
            } else {
                other.to_string()
            }
        }
    }
}

/// 執行 4 大消耗迴路（民生口糧、百工原料、工具磨損、邊關基建）與自主行商跨區調撥。
pub fn execute_settlement_sinks_and_arbitrage(
    cur_settlement: &str,
    now_unix: i64,
    ledger: &mut EconomyLedger,
) {
    // Sink #1 (民生口糧柴炭剛需): Residents consume 1 unit of local food/fuel/tea
    let sink1_items = [
        "mat_salted_fish",
        "mat_coarse_salt_pack",
        "mat_kindling_bundle",
        "mat_charcoal_sack",
        "craft_herbal_tea_pack",
    ];
    let s1_item = sink1_items[((now_unix as usize) + cur_settlement.len()) % sink1_items.len()];
    let k1 = (cur_settlement.to_string(), s1_item.to_string());
    if let Some(cell) = ledger.settlement_markets.get_mut(&k1) {
        if cell.stock > 0 {
            cell.stock -= 1;
            cell.demand_ema = (cell.demand_ema * 1.02).min(1.45);
        }
    }
    if let Some(entry) = ledger.items.get_mut(s1_item) {
        if entry.stock > 0 {
            entry.stock -= 1;
        }
    }

    // Sink #3 (工具衣靴磨損汰換): Workers consume/wear 1 unit of tools/boots/rope
    let sink3_items = [
        "craft_iron_pickaxe",
        "craft_tool_repair_strip",
        "craft_patched_work_boots",
        "craft_storm_canvas_cloak",
        "craft_tarred_hemp_rope",
    ];
    let s3_item = sink3_items[((now_unix as usize / 3) + cur_settlement.len()) % sink3_items.len()];
    let k3 = (cur_settlement.to_string(), s3_item.to_string());
    if let Some(cell) = ledger.settlement_markets.get_mut(&k3) {
        if cell.stock > 0 {
            cell.stock -= 1;
            cell.demand_ema = (cell.demand_ema * 1.02).min(1.45);
        }
    }
    if let Some(entry) = ledger.items.get_mut(s3_item) {
        if entry.stock > 0 {
            entry.stock -= 1;
        }
    }

    // Sink #4 (邊關船塢基建耗損): In 黑沼關隘/南哨/風舶角/水尾寮
    if cur_settlement == "黑沼關隘"
        || cur_settlement == "南哨"
        || cur_settlement == "風舶角"
        || cur_settlement == "水尾寮"
    {
        let sink4_items = [
            "craft_caulking_sealant",
            "mat_cedar_plank",
            "craft_meridian_salve",
            "craft_pine_torch",
        ];
        let s4_item =
            sink4_items[((now_unix as usize / 5) + cur_settlement.len()) % sink4_items.len()];
        let k4 = (cur_settlement.to_string(), s4_item.to_string());
        if let Some(cell) = ledger.settlement_markets.get_mut(&k4) {
            if cell.stock > 0 {
                cell.stock -= 1;
                cell.demand_ema = (cell.demand_ema * 1.02).min(1.45);
            }
        }
        if let Some(entry) = ledger.items.get_mut(s4_item) {
            if entry.stock > 0 {
                entry.stock -= 1;
            }
        }
    }

    // Source #3 (行商跨區套利調撥): When stock > target_stock + 2 and another stock < target_stock - 1, move 1 unit
    let candidate_arbitrage_items = [
        "mat_rough_ore",
        "mat_iron_scrap",
        "mat_charcoal_sack",
        "mat_timber_log",
        "mat_plank_board",
        "mat_reed_bundle",
        "mat_hemp_rope_roll",
        "mat_foraged_herb",
        "mat_coarse_salt_pack",
        "craft_meridian_salve",
        "craft_calming_tea_pack",
        "craft_steel_hunting_knife",
        "craft_heavy_leather_boots",
    ];
    let mut arbitrage = None;
    for &item_id in &candidate_arbitrage_items {
        let mut surplus_s = None;
        let mut shortage_s = None;
        for &s in &CANONICAL_SETTLEMENTS {
            if let Some(cell) =
                ledger.settlement_markets.get(&(s.to_string(), item_id.to_string()))
            {
                if cell.stock > cell.target_stock + 2 && surplus_s.is_none() {
                    surplus_s = Some(s);
                } else if cell.stock < cell.target_stock - 1 && shortage_s.is_none() {
                    shortage_s = Some(s);
                }
            }
            if surplus_s.is_some() && shortage_s.is_some() {
                break;
            }
        }
        if let (Some(from_s), Some(to_s)) = (surplus_s, shortage_s) {
            if from_s != to_s {
                arbitrage = Some((item_id, from_s, to_s));
                break;
            }
        }
    }
    if let Some((item_id, from_s, to_s)) = arbitrage {
        let transfer_qty = 1;
        if let Some(from_cell) =
            ledger.settlement_markets.get_mut(&(from_s.to_string(), item_id.to_string()))
        {
            from_cell.stock = (from_cell.stock - transfer_qty).max(0);
            from_cell.demand_ema = (from_cell.demand_ema * 1.03).min(1.45);
        }
        if let Some(to_cell) =
            ledger.settlement_markets.get_mut(&(to_s.to_string(), item_id.to_string()))
        {
            to_cell.stock = (to_cell.stock + transfer_qty).min(to_cell.target_stock * 4);
            to_cell.demand_ema = (to_cell.demand_ema * 0.95).max(0.75);
        }
        let item_name = get_clean_item_name(item_id);
        let event = format!(
            "【自主行商調撥】行商將 {transfer_qty} 份「{item_name}」自產地【{from_s}】運抵【{to_s}】市集，稍解當地缺料之急。"
        );
        ledger.recent_events.push_back(event);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }
}

/// 判斷物品是否屬於木材、柴火或相關原木料件。
pub fn is_wood_or_firewood_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_timber_log"
            | "mat_kindling_bundle"
            | "mat_plank_board"
            | "mat_wood_offcut_bundle"
            | "mat_repair_strip"
            | "wt-good-001"
            | "wt_cat_0706"
            | "wt_cat_0899"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (57..=112).contains(&num) {
                return true;
            }
        }
    }
    false
}

fn normalize_room_key(room_id: &str) -> String {
    if let Some(coord) = crate::grid::parse_grid_room_id(room_id) {
        if let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
            .and_then(|g| g.get(coord).cloned())
        {
            let name = cell.name.trim();
            if !name.is_empty() {
                return name.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜");
            }
        }
    }
    room_id.replace("文曜・文曜", "文曜・").replace("文曜文曜", "文曜")
}

/// 取得指定房間當前活躍的野地營火狀態。
pub fn get_active_campfire(room_id: &str) -> Option<CampfireState> {
    let norm = normalize_room_key(room_id);
    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
    ledger
        .active_campfires
        .get(&norm)
        .or_else(|| ledger.active_campfires.get(room_id))
        .cloned()
}
/// 點燃或加添野地營火。
pub fn ignite_or_stoke_campfire(
    pid: &str,
    room_id: &str,
    preferred_item_id: Option<&str>,
    now_unix: i64,
) -> Result<(String, bool), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let fuel_candidates = [
        "mat_kindling_bundle",
        "mat_timber_log",
        "mat_charcoal_sack",
        "mat_wood_offcut_bundle",
        "mat_reed_bundle",
        "wt-good-001",
    ];

    let mut consumed_item = None;
    if let Some(pref) = preferred_item_id {
        if db::inventory_has_item(&ent.inventory, pref) {
            consumed_item = Some(pref.to_string());
        }
    }

    if consumed_item.is_none() {
        for candidate in fuel_candidates {
            if db::inventory_has_item(&ent.inventory, candidate) {
                consumed_item = Some(candidate.to_string());
                break;
            }
        }
    }

    let is_charcoal = consumed_item.as_deref() == Some("mat_charcoal_sack");
    let fuel_name = if let Some(it) = &consumed_item {
        let _ = db::remove_from_inventory(pid, it, 1);
        get_clean_item_name(it)
    } else {
        "拾撿枯枝".to_string()
    };

    let norm_room = normalize_room_key(room_id);
    let is_new_fire = {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        let was_new = if let Some(cf) = ledger.active_campfires.get_mut(&norm_room) {
            cf.fuel_level = (cf.fuel_level + 2).min(5);
            cf.is_charcoal = cf.is_charcoal || is_charcoal;
            cf.updated_at = now_unix;
            false
        } else {
            let state = CampfireState {
                fuel_level: 3,
                is_charcoal,
                updated_at: now_unix,
            };
            ledger.active_campfires.insert(norm_room.clone(), state.clone());
            if norm_room != room_id {
                ledger.active_campfires.insert(room_id.to_string(), state);
            }
            true
        };

        let ev = if was_new {
            format!("荒野旅人在【{norm_room}】引燃了野地營火堆，暖光照耀四方。")
        } else {
            format!("【{norm_room}】的野地營火添柴增旺，柴煙裊裊，熱力充沛。")
        };
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
        was_new
    };

    if let Some(arc) = store::get_store() {
        let _ = arc.write().update_entity(pid, |e| {
            e.vit = (e.vit + 12).min(100);
            e.qi = (e.qi + 12).min(100);
        });
    }
    db::log_npc_event(now_unix, pid, "ignite_campfire", room_id);

    let narrative = if is_new_fire {
        format!(
            "你取用【{fuel_name}】引燃火星，在空地中央架起通風柴堆。隨著青煙升騰，金紅烈焰劈啪燃起，生起了一座溫暖明亮的〔野地營火堆〕！周遭寒濕之氣為之一空，融融暖意順著掌心湧入經脈，體魄與元氣大受滋養。"
        )
    } else {
        format!(
            "你將【{fuel_name}】添入營火核心，原本微暗的炭火頓時騰躍起金紅火舌，熱浪撲面而來，火勢更為旺盛（營火柴薪充沛，體魄與元氣獲得溫養）。"
        )
    };

    Ok((narrative, is_new_fire))
}

/// 嘗試烹調《白塔山海志》獵獲與特產名物之合味珍餚：
/// 檢測玩家行囊是否持有山海食材與半島特色佐料，若有則進行料理並恢復體魄元氣。
/// 回傳 `Some((narrative, total_vit, total_qi))`。
pub fn try_camp_shanhai_cooking(
    actor_id: &str,
    venue_label: &str,
    now_unix: i64,
) -> Option<(String, i32, i32)> {
    let ent = db::get_entity(actor_id).ok().flatten()?;

    // (ingredient_id, dish_name, condiment_id, condiment_name, base_vit, base_qi)
    const SHANHAI_RECIPES: &[(&str, &str, &str, &str, i32, i32)] = &[
        ("wt_cat_0205", "老薑黑糖麻油炙月熊掌", "wt_cat_0305", "老薑黑糖磚", 30, 24),
        ("wt_cat_0224", "壺底蔭油紅燒銀鱗土魠", "wt_cat_0311", "陶缸日曬黑豆壺底蔭油", 26, 20),
        ("wt_cat_0223", "馬告清燉野生九孔鮑湯", "wt_cat_0270", "山胡椒馬告粒", 26, 22),
        ("wt_cat_0236", "甘醇蔭油膏爆炒墨玉花枝", "wt_cat_0312", "手工甘醇蔭油膏", 24, 20),
        ("wt_cat_0203", "馬告炭炙高地水鹿腱", "wt_cat_0270", "山胡椒馬告粒", 26, 18),
        ("wt_cat_0209", "陳年老菜脯慢燉山羌盅", "wt_cat_0319", "甕藏陳年老菜脯", 25, 20),
        ("wt_cat_0240", "清溪活水黃金蜆", "wt_cat_0311", "陶缸日曬黑豆壺底蔭油", 22, 18),
        ("wt_cat_0239", "老薑麻油烏魚米粉鍋", "wt_cat_0305", "老薑黑糖磚", 24, 18),
    ];

    for &(ing_id, dish_name, cond_id, cond_name, base_vit, base_qi) in SHANHAI_RECIPES {
        if db::inventory_has_item(&ent.inventory, ing_id) {
            let ing_name = get_clean_item_name(ing_id);
            let _ = db::remove_from_inventory(actor_id, ing_id, 1);

            let has_cond = db::inventory_has_item(&ent.inventory, cond_id);
            let (extra_bonus, cond_narrative, mat_names) = if has_cond {
                let _ = db::remove_from_inventory(actor_id, cond_id, 1);
                (
                    4,
                    format!("你更取出行囊中珍藏的【{}】下鍋調味，特產甘醇芳香四溢", cond_name),
                    vec![ing_name.clone(), cond_name.to_string()],
                )
            } else {
                (
                    0,
                    format!("你順手採集「{}」周遭備妥的在地佐料調和入味", venue_label),
                    vec![ing_name.clone(), "在地佐料".to_string()],
                )
            };

            crate::pg::living_world::record_breath_activity(
                actor_id,
                crate::pg::living_world::BreathHabitKind::TideContinuous,
                2,
                now_unix,
            );

            let craft_dims = crate::server::handler::entity_total_craft_dims(actor_id);
            let culinary_bonus = if craft_dims.culinary >= 0.6 { 5 } else { 0 };

            let total_vit = base_vit + extra_bonus + culinary_bonus;
            let total_qi = base_qi + extra_bonus + culinary_bonus;

            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(actor_id, |e| {
                    e.vit = (e.vit + total_vit).min(100);
                    e.qi = (e.qi + total_qi).min(100);
                });
            }

            let actor_name = if !ent.display_title.trim().is_empty() {
                ent.display_title.trim().to_string()
            } else {
                "荒野行旅".to_string()
            };

            let tool_familiarity = if craft_dims.culinary >= 0.6 {
                "沉穩趁手".to_string()
            } else {
                "得心應手".to_string()
            };

            let packet = crate::ai::CraftActionFactPacket {
                actor_name,
                tool_name: "行旅銅釜".to_string(),
                tool_familiarity,
                breath_habit: "潮汐連息".to_string(),
                material_names: mat_names,
                output_name: dish_name.to_string(),
                highlight_kind: "山海營火合味現煮".to_string(),
                environment_hint: venue_label.to_string(),
            };
            let highlight_prose = crate::ai::compose_craft_highlight_from_data(&packet);

            let resonance_note = if craft_dims.culinary >= 0.6 {
                " 【百工藝相・庖廚調味】你周天字印所蘊的調和氣機暗合潮汐水火，火候與收汁拿捏得恰到好處。"
            } else {
                ""
            };

            let narrative = if highlight_prose.is_empty() {
                format!(
                    "【山海合味・{}】你在「{}」旁支起鍋鑊，取出獵獲名物【{}】細心炙烹，{}，精心料理成一道香氣撲鼻的盛宴名饌「{}」！熱氣騰騰中大快朵頤，濃郁滋味透徹肺腑，渾身百骸如沐暖陽（體魄提升 +{}，元氣充盈 +{}）。{}",
                    dish_name, venue_label, ing_name, cond_narrative, dish_name, total_vit, total_qi, resonance_note
                )
            } else {
                format!(
                    "【山海合味・{}】你在「{}」旁支起鍋鑊，取出獵獲名物【{}】細心炙烹，{}，精心料理成一道香氣撲鼻的盛宴名饌「{}」！熱氣騰騰中大快朵頤，濃郁滋味透徹肺腑，渾身百骸如沐暖陽（體魄提升 +{}，元氣充盈 +{}）。{}\n\n{}",
                    dish_name, venue_label, ing_name, cond_narrative, dish_name, total_vit, total_qi, resonance_note, highlight_prose
                )
            };
            return Some((narrative, total_vit, total_qi));
        }
    }

    None
}

/// 保持相容：嘗試烹調《白塔山海志》獵獲與特產名物之合味珍餚。
pub fn try_cook_shanhai_delicacy(pid: &str, venue_label: &str) -> Option<(String, i32, i32)> {
    try_camp_shanhai_cooking(pid, venue_label, crate::game::now_unix())
}

/// 與野地營火堆的多重插座互動。
pub fn interact_with_campfire(
    pid: &str,
    room_id: &str,
    action: &str,
    now_unix: i64,
) -> Result<String, String> {
    match action {
        "WarmHands" => {
            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 10).min(100);
                    e.qi = (e.qi + 12).min(100);
                });
            }
            Ok("你湊近營火伸出雙手，融融暖流自掌心順著經絡徐徐流轉周身，荒野長途奔波的疲憊寒濕頓時消散大半，整個人神清氣爽（體魄與元氣獲得恢復）。".to_string())
        }
        "RoastFood" => {
            if let Some((shanhai_narr, _v, _q)) = try_camp_shanhai_cooking(pid, "野地營火堆", now_unix) {
                return Ok(shanhai_narr);
            }
            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.vit = (e.vit + 20).min(100);
                    e.qi = (e.qi + 10).min(100);
                });
            }
            Ok("你取出行囊中的乾糧肉脯架在營火紅炭上方細心炙烤，外焦裡嫩、油脂滋滋作響，趁熱食用後腹中溫熱充實，精氣神大振（深幅恢復體魄與元氣）。".to_string())
        }
        "CraftTorch" => {
            let Ok(Some(ent)) = db::get_entity(pid) else {
                return Err("角色不存在".to_string());
            };
            if db::inventory_has_item(&ent.inventory, "mat_kindling_bundle") {
                let _ = db::remove_from_inventory(pid, "mat_kindling_bundle", 1);
            } else if db::inventory_has_item(&ent.inventory, "mat_timber_log") {
                let _ = db::remove_from_inventory(pid, "mat_timber_log", 1);
            }
            let _ = db::add_to_inventory(pid, "craft_pine_torch", 1);
            Ok("你挑選了一根乾燥堅實的硬柴浸塗松脂，伸入營火中引燃並纏裹麻絲封固，製成一柄劈啪燃燒、可用於荒野夜行照明驅獸的【浸脂松明火把】收入行囊。".to_string())
        }
        "AddFuel" => {
            let (narrative, _) = ignite_or_stoke_campfire(pid, room_id, None, now_unix)?;
            Ok(narrative)
        }
        "CollectCharcoal" => {
            let norm_room = normalize_room_key(room_id);
            let had_fire = {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                let r1 = ledger.active_campfires.remove(&norm_room).is_some();
                let r2 = ledger.active_campfires.remove(room_id).is_some();
                r1 || r2
            };
            if !had_fire {
                return Err("此處營火已無餘熱，無法封收木炭。".to_string());
            }
            let _ = db::add_to_inventory(pid, "mat_charcoal_sack", 2);
            Ok("你用長枝撥開營火外層浮灰，將深處熾熱通紅的硬木火炭妥善封土覆泥隔氧熄滅，待其冷卻凝結後，收獲了兩袋熱值極高、敲擊有金屬聲的【硬木白炭袋】收入行囊。".to_string())
        }
        _ => Err(format!("營火無法執行此項操作：{action}")),
    }
}

/// 木作五向精煉多插座派發。
pub fn refine_wood_multi_socket(
    pid: &str,
    mode: &str,
    _location_hint: &str,
    now_unix: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let wood_candidates = [
        "mat_timber_log",
        "mat_kindling_bundle",
        "mat_plank_board",
        "mat_wood_offcut_bundle",
        "mat_repair_strip",
        "wt-good-001",
    ];
    for w in wood_candidates {
        if db::inventory_has_item(&ent.inventory, w) {
            let _ = db::remove_from_inventory(pid, w, 1);
            break;
        }
    }

    let (prod_id, prod_name, qty, narrative) = match mode {
        "SplitWood" => (
            "mat_kindling_bundle".to_string(),
            "乾燥柴枝束".to_string(),
            2,
            "你手起斧落順著木紋順暢剖劈，將木料劈裂分開，紮成兩束易於引燃的【乾燥柴枝束】收入行囊。".to_string(),
        ),
        "RefinePlank" => (
            "mat_plank_board".to_string(),
            "樟松木板料".to_string(),
            2,
            "你拉動雙手長鋸順紋裁鋸刨切，將原木加工為兩塊平整堅實、不易變形的【樟松木板料】收入行囊。".to_string(),
        ),
        "BurnCharcoal" => (
            "mat_charcoal_sack".to_string(),
            "硬木白炭袋".to_string(),
            1,
            "你將硬木截段架入土窯封閉泥封，慢火悶燒凝煉，成功製成一袋成色純淨、熱值極高的【硬木白炭袋】收入行囊。".to_string(),
        ),
        "CraftTorch" => (
            "craft_pine_torch".to_string(),
            "浸脂松明火把".to_string(),
            1,
            "你挑選油脂豐富的松木枝條塗抹脂膏緊密紮裹，製成一柄照明驅寒的【浸脂松明火把】收入行囊。".to_string(),
        ),
        "CraftHandle" => (
            "craft_wooden_tool_handle".to_string(),
            "堅韌硬木工具長柄".to_string(),
            1,
            "你用木工推刀細細車削刨光，打磨出一根手感沉穩堅韌的【堅韌硬木工具長柄】收入行囊。".to_string(),
        ),
        _ => return Err(format!("未知的木作精煉模式：{mode}")),
    };

    let _ = db::add_to_inventory(pid, &prod_id, qty);
    db::log_npc_event(now_unix, pid, "refine_wood", &prod_name);
    Ok((prod_id, prod_name, narrative))
}

/// 工坊高爐添柴加溫。
pub fn stoke_workshop_furnace(
    pid: &str,
    room_id: &str,
    now_unix: i64,
) -> Result<String, String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let (fuel_name, heat_level) = if db::inventory_has_item(&ent.inventory, "mat_charcoal_sack") {
        let _ = db::remove_from_inventory(pid, "mat_charcoal_sack", 1);
        ("硬木白炭", 3)
    } else if db::inventory_has_item(&ent.inventory, "mat_kindling_bundle") {
        let _ = db::remove_from_inventory(pid, "mat_kindling_bundle", 1);
        ("乾燥柴枝束", 2)
    } else if db::inventory_has_item(&ent.inventory, "mat_timber_log") {
        let _ = db::remove_from_inventory(pid, "mat_timber_log", 1);
        ("伐採原木", 2)
    } else {
        ("周遭柴薪", 2)
    };

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        ledger.room_furnace_heat.insert(room_id.to_string(), heat_level);
        let ev = format!("【{room_id}】添柴入爐，爐溫飆升至白熾狀態，百工造物異變機率大幅提升。");
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }
    db::log_npc_event(now_unix, pid, "stoke_furnace", room_id);

    Ok(format!(
        "你將【{fuel_name}】添入高溫工爐核心，狂猛的熱浪夾雜著金紅烈芒瞬間騰躍而起，爐膛劇烈咆哮、火溫轉為純白熾熱，工坊已蓄勢待發，鍛造與淬火異變之氣達至鼎盛！"
    ))
}

/// 修補房間內的損耗設施與建築構件。
pub fn repair_room_structure(
    pid: &str,
    room_id: &str,
    target_name: &str,
    now_unix: i64,
) -> Result<String, String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    let repair_materials = [
        "mat_plank_board",
        "craft_wooden_tool_handle",
        "mat_timber_log",
        "mat_repair_strip",
        "mat_hemp_rope_roll",
    ];

    let mut used_item = None;
    for mat in repair_materials {
        if db::inventory_has_item(&ent.inventory, mat) {
            let _ = db::remove_from_inventory(pid, mat, 1);
            used_item = Some(get_clean_item_name(mat));
            break;
        }
    }

    let mat_desc = used_item.unwrap_or_else(|| "就地取材之構件".to_string());

    {
        let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
        ledger.room_structure_repairs.insert(room_id.to_string(), now_unix);
        let ev = format!("白塔半島設施【{target_name}】經巧手工匠修繕穩固。");
        ledger.recent_events.push_back(ev);
        if ledger.recent_events.len() > 30 {
            ledger.recent_events.pop_front();
        }
    }

    let _ = db::transfer_magnesium("world_market", pid, 8).or_else(|_| db::add_magnesium(pid, 8));
    db::log_npc_event(now_unix, pid, "repair_structure", target_name);

    Ok(format!(
        "你取出【{mat_desc}】細心楔合修補「{target_name}」的破損榫眼與鬆動踏板，敲釘抹膠固定妥當。破損處重歸平整穩固，市場營造司為此核發當班津貼入行囊（設施修復穩妥，資財獲得結算補貼）。"
    ))
}

/// 判斷物品是否屬於礦石、金屬、修件或鍛造料件。
pub fn is_mineral_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_iron_scrap"
            | "mat_repair_strip"
            | "mat_rough_ore"
            | "mat_washed_ore"
            | "mat_stone_powder_sack"
            | "mat_metal_rim_strip"
            | "mat_iron_scrap_box"
            | "mat_flint_stone"
            | "craft_steel_hunting_knife"
            | "craft_iron_pickaxe"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (1..=56).contains(&num) || (337..=392).contains(&num) {
                return true;
            }
        }
    }
    item_id.contains("ore")
        || item_id.contains("iron")
        || item_id.contains("stone")
        || item_id.contains("metal")
        || item_id.contains("flint")
        || item_id.contains("steel")
        || item_id.contains("pickaxe")
}

/// 判斷物品是否屬於黏土、紅磚或瓦陶料件。
pub fn is_clay_brick_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_stone_powder_sack" | "wt_cat_0006" | "wt_cat_0023" | "wt_cat_0677"
    ) {
        return true;
    }
    item_id.contains("clay")
        || item_id.contains("brick")
        || item_id.contains("pottery")
        || item_id.contains("ceramic")
}

/// 將物品分類為五源詞元產出來源與素材品階 (WordSource, tier 2..=4)。
pub fn classify_material_word_source(
    item_id: &str,
    item_name: &str,
) -> Option<(crate::combat::wordplate::WordSource, u8)> {
    use crate::combat::wordplate::WordSource;

    if item_id.starts_with("crystal_")
        || item_id.starts_with("starter_")
        || item_id.starts_with("key_")
    {
        return None;
    }
    if item_id.starts_with("corpse_") {
        let tier = crate::pg::living_world::get_beast_corpse(item_id).map(|r| r.tier).unwrap_or(4);
        return Some((WordSource::Beast, tier));
    }

    // 1. 獸源 (Beast)
    let is_beast_cat = if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            (171..=217).contains(&num)
        } else {
            false
        }
    } else {
        false
    };

    const BEAST_KEYWORDS: &[&str] = &[
        "皮", "毛", "羽", "翎", "掌", "鱗", "麟", "蛻", "角", "牙", "肉", "骨", "脂",
        "鹿", "豹", "熊", "虎", "豬", "牛", "羊", "羌", "兔", "鼠", "鷹", "雉", "鷴", "鱷", "蛇", "鯪鯉",
    ];

    let is_aquatic_override = item_name.contains("魚")
        || item_name.contains("鯊")
        || item_name.contains("鯨")
        || item_name.contains("烏賊")
        || item_name.contains("花枝")
        || item_name.contains("鮑")
        || item_name.contains("蝦")
        || item_name.contains("蟹");
    let is_plant_override = item_name.contains("牛樟")
        || item_name.contains("靈芝")
        || item_name.contains("草藥")
        || is_herb_item(item_id)
        || is_wood_or_firewood_item(item_id);
    let has_beast_kw = !is_aquatic_override
        && !is_plant_override
        && BEAST_KEYWORDS
            .iter()
            .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_beast_cat || has_beast_kw {
        const APEX_BEAST_KW: &[&str] = &[
            "雲豹", "月熊", "穿山甲", "鯪鯉", "百步蛇", "巨鱷", "帝雉", "藍腹鷴", "熊鷹", "大角原牛", "水鹿",
        ];
        const MED_BEAST_KW: &[&str] = &["石虎", "山羊", "野豬", "白鼻心"];

        let tier = if APEX_BEAST_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            4
        } else if MED_BEAST_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            3
        } else {
            2
        };
        return Some((WordSource::Beast, tier));
    }

    // 2. 水源 (Water)
    let is_water_cat = if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            (220..=236).contains(&num)
        } else {
            false
        }
    } else {
        false
    };

    const WATER_KEYWORDS: &[&str] = &[
        "魚", "鮑", "九孔", "烏賊", "花枝", "墨囊", "鯊", "鯨", "海", "蝦", "蛤", "貝",
        "蟹", "螺", "藻", "水", "泉", "露", "冰", "膠", "潮",
    ];

    let has_water_kw = WATER_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_water_cat || has_water_kw {
        const APEX_WATER_KW: &[&str] = &["大花枝", "豆腐鯊", "土魠", "九孔鮑", "龍魚"];
        let tier = if APEX_WATER_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            4
        } else {
            2
        };
        return Some((WordSource::Water, tier));
    }

    // 3. 礦源 (Mineral)
    const MINERAL_KEYWORDS: &[&str] = &[
        "礦", "鐵", "石", "岩", "鹽", "煤", "硫", "砂", "泥", "陶", "磚", "灰", "金", "銅", "晶", "鍛", "錠",
    ];

    let has_mineral_kw = MINERAL_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_mineral_item(item_id) || is_clay_brick_item(item_id) || has_mineral_kw {
        const RARE_MINERAL_KW: &[&str] = &["玄武", "硫磺", "精鐵", "黑曜", "鹽晶"];
        let tier = if RARE_MINERAL_KW.iter().any(|&kw| item_name.contains(kw) || item_id.contains(kw)) {
            3
        } else {
            2
        };
        return Some((WordSource::Mineral, tier));
    }

    // 4. 植源 (Plant)
    const PLANT_KEYWORDS: &[&str] = &[
        "樟", "芝", "愛玉", "馬告", "菜脯", "蔭油", "黑糖", "草", "葉", "木", "板", "柴",
        "炭", "竹", "蘆", "麻", "繩", "布", "紙", "茶", "果", "稻", "穀", "菇", "藤", "花",
        "根", "藥", "薯", "豆", "薑",
    ];

    let has_plant_kw = PLANT_KEYWORDS
        .iter()
        .any(|&kw| item_name.contains(kw) || item_id.contains(kw));

    if is_wood_or_firewood_item(item_id)
        || is_herb_item(item_id)
        || is_fiber_item(item_id)
        || has_plant_kw
    {
        let tier = if item_name.contains("牛樟芝") || item_id.contains("牛樟芝") {
            4
        } else if ["愛玉", "馬告", "老菜脯", "蔭油", "老薑"]
            .iter()
            .any(|&kw| item_name.contains(kw) || item_id.contains(kw))
        {
            3
        } else {
            2
        };
        return Some((WordSource::Plant, tier));
    }

    // 5. 氣源 (Qi) 或 Fallback
    Some((WordSource::Qi, 2))
}


/// 判斷物品是否屬於草藥、煎膏或茶包。
pub fn is_herb_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_foraged_herb"
            | "craft_meridian_salve"
            | "craft_calming_tea_pack"
            | "mat_sealing_pitch"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (393..=448).contains(&num) || (1009..=1064).contains(&num) {
                return true;
            }
        }
    }
    item_id.contains("herb")
        || item_id.contains("salve")
        || item_id.contains("tea")
        || item_id.contains("root")
        || item_id.contains("decoction")
}

/// 判斷物品是否屬於纖維、麻繩、紙張或布料。
pub fn is_fiber_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_reed_bundle"
            | "mat_hemp_rope_roll"
            | "mat_sackcloth_bolt"
            | "mat_coarse_paper_ream"
            | "mat_canvas_roll"
            | "mat_oilcloth_roll"
            | "mat_wrapping_paper_pack"
            | "mat_fishing_line_bundle"
            | "mat_rice_straw_bundle"
            | "craft_storm_canvas_cloak"
            | "craft_peninsula_almanac"
            | "craft_bamboo_oil_umbrella"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (505..=560).contains(&num) {
                return true;
            }
        }
    }
    item_id.contains("reed")
        || item_id.contains("rope")
        || item_id.contains("sackcloth")
        || item_id.contains("paper")
        || item_id.contains("canvas")
        || item_id.contains("oilcloth")
        || item_id.contains("flax")
        || item_id.contains("fiber")
}

/// 判斷物品是否屬於生皮、水產、魚骨或貝類。
pub fn is_hide_fish_item(item_id: &str) -> bool {
    if matches!(
        item_id,
        "mat_leather_strip"
            | "craft_heavy_leather_boots"
            | "mat_fish_bone_sack"
            | "mat_fish_scale_sack"
            | "mat_shell_sack"
            | "mat_mane_hair_bundle"
    ) {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (113..=168).contains(&num) || (169..=224).contains(&num) {
                return true;
            }
        }
    }
    item_id.contains("leather")
        || item_id.contains("fish")
        || item_id.contains("shell")
        || item_id.contains("pelt")
        || item_id.contains("hide")
        || item_id.contains("bone")
        || item_id.contains("boot")
}

/// 礦鐵金石多插座精煉派發：支援 WashOre, SmeltBloom, ForgeRepairStrip, SharpenBlade, BuildFieldAnvil
pub fn refine_mineral_multi_socket(
    pid: &str,
    mode: &str,
    room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    match mode {
        "WashOre" => {
            let candidates = ["mat_rough_ore", "mat_iron_scrap", "mat_stone_powder_sack", "mat_washed_ore"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_washed_ore".to_string();
            let prod_name = "洗選精礦石".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你將礦料置入澄淨流水竹籃中反覆淘洗，洗去表層頑泥雜質與黏土，篩濾出一籃晶芒隱現的【洗選精礦石】收入行囊。".to_string()))
        }
        "SmeltBloom" => {
            let candidates = ["mat_washed_ore", "mat_rough_ore", "mat_iron_scrap", "mat_repair_strip"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_iron_scrap".to_string();
            let prod_name = "回收熟鐵料塊".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你引燃爐炭鼓風增溫，將選洗後的礦石高溫悶燒還原為海綿鐵團，趁熱重錘鍛打擠出爐渣，製成兩塊成色均勻的【回收熟鐵料塊】收入行囊。".to_string()))
        }
        "ForgeRepairStrip" => {
            let candidates = ["mat_iron_scrap", "mat_metal_rim_strip", "mat_rough_ore"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_repair_strip".to_string();
            let prod_name = "修補熟鐵薄片".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_mineral", &prod_name);
            Ok((prod_id, prod_name, "你將熟鐵料段置於砧石之上反覆鍛打延展，裁截整飭為兩片平整勻稱、韌性十足的【修補熟鐵薄片】收入行囊。".to_string()))
        }
        "SharpenBlade" => {
            if db::inventory_has_item(&ent.inventory, "mat_iron_scrap") && !db::inventory_has_item(&ent.inventory, "craft_steel_hunting_knife") {
                let _ = db::remove_from_inventory(pid, "mat_iron_scrap", 1);
                let _ = db::add_to_inventory(pid, "craft_steel_hunting_knife", 1);
                db::log_npc_event(now, pid, "refine_mineral", "淬火開鋒精鍛防身獵刀");
                return Ok(("craft_steel_hunting_knife".to_string(), "精鍛防身獵刀".to_string(), "你將熟鐵料塊反覆鍛打成型、蘸水在青石上細心推磨開鋒，寒芒畢露，鍛出一柄【精鍛防身獵刀】收進行囊！".to_string()));
            }
            let candidates = ["craft_steel_hunting_knife", "craft_iron_pickaxe", "mat_repair_strip", "mat_flint_stone"];
            let mut sharpened = "隨身刃具".to_string();
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    sharpened = get_clean_item_name(c);
                    break;
                }
            }
            if let Some(arc) = store::get_store() {
                let _ = arc.write().update_entity(pid, |e| {
                    e.dex = (e.dex + 2).min(100);
                });
            }
            db::log_npc_event(now, pid, "refine_mineral", "磨礪刃具");
            Ok(("mat_repair_strip".to_string(), "刃面鋒芒".to_string(), format!("你蘸水在油石上細心推磨【{sharpened}】，砂痕層層推勻，刃線寒芒畢露、鋒利無匹（身手敏銳度獲得淬礪提升）！")))
        }
        "BuildFieldAnvil" => {
            let norm_room = normalize_room_key(room);
            let state = FieldFacilityState {
                facility_id: "dyn_field_anvil".to_string(),
                name: "野地修具石砧".to_string(),
                created_at: now,
                expires_at: now + 86400,
            };
            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                ledger.active_field_facilities.entry(norm_room.clone()).or_default().push(state.clone());
                if norm_room != room {
                    ledger.active_field_facilities.entry(room.to_string()).or_default().push(state);
                }
                let ev = format!("【{room}】有人就地架設了一座平整沉重的【野地修具石砧】，可供往來行者鍛打修治鐵件。");
                ledger.recent_events.push_back(ev);
                if ledger.recent_events.len() > 30 {
                    ledger.recent_events.pop_front();
                }
            }
            db::log_npc_event(now, pid, "build_field_anvil", room);
            Ok(("dyn_field_anvil".to_string(), "野地修具石砧".to_string(), "你尋來一塊質地緊密堅實的平整玄武巨石，穩穩楔入地面夯實，架設起一座可供敲打鍛修器具的【野地修具石砧】！".to_string()))
        }
        _ => Err(format!("未知的礦鐵精煉模式：{mode}")),
    }
}

/// 山野草藥多插座精煉派發：支援 WashHerb, DryHerb, CrushHerb, BoilDecoction, SetupHerbStove
pub fn refine_herb_multi_socket(
    pid: &str,
    mode: &str,
    room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    match mode {
        "WashHerb" => {
            let candidates = ["mat_foraged_herb", "mat_wet_grass_bundle", "mat_water_moss_sheet"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_foraged_herb".to_string();
            let prod_name = "山野鮮藥草".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你引清泉細心濯洗藥草根葉，洗淨泥沙雜屑，鮮嫩莖葉翠綠如洗、散發清幽草木芬芳。".to_string()))
        }
        "DryHerb" => {
            let candidates = ["mat_foraged_herb", "mat_wet_grass_bundle"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "craft_calming_tea_pack".to_string();
            let prod_name = "清心安神百草茶包".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你將洗淨的藥草薄攤在竹簾上慢火烘焙陰乾，鎖住草木菁華，揉捻分裝成一包芳香澄澈的【清心安神百草茶包】收入行囊。".to_string()))
        }
        "CrushHerb" => {
            let candidates = ["mat_foraged_herb", "craft_calming_tea_pack"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_foraged_herb".to_string();
            let prod_name = "百草細研藥末".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你取出石臼研杵，將乾燥草藥細細研磨成細膩墨綠粉末，藥香濃郁撲鼻，適宜調和脂膏敷裹。".to_string()))
        }
        "BoilDecoction" => {
            let candidates = ["mat_foraged_herb", "mat_sealing_pitch", "craft_calming_tea_pack"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "craft_meridian_salve".to_string();
            let prod_name = "溫脈舒筋活血膏".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_herb", &prod_name);
            Ok((prod_id, prod_name, "你將研細的草藥粉末配以松脂文火慢熬收膏，藥汁凝為琥珀色溫潤脂膏，製成一盒【溫脈舒筋活血膏】收入行囊。".to_string()))
        }
        "SetupHerbStove" => {
            let norm_room = normalize_room_key(room);
            let state = FieldFacilityState {
                facility_id: "dyn_herb_stove".to_string(),
                name: "野地煎藥陶銚".to_string(),
                created_at: now,
                expires_at: now + 86400,
            };
            {
                let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
                ledger.active_field_facilities.entry(norm_room.clone()).or_default().push(state.clone());
                if norm_room != room {
                    ledger.active_field_facilities.entry(room.to_string()).or_default().push(state);
                }
                let ev = format!("【{room}】架起了一具溫火慢煨的【野地煎藥陶銚】，徐徐散發著舒心藥香。");
                ledger.recent_events.push_back(ev);
                if ledger.recent_events.len() > 30 {
                    ledger.recent_events.pop_front();
                }
            }
            db::log_npc_event(now, pid, "setup_herb_stove", room);
            Ok(("dyn_herb_stove".to_string(), "野地煎藥陶銚".to_string(), "你於避風處支起三足泥爐，架上帶蓋陶銚並注滿山泉活水，生起微火，架設好一具隨時可煎煮藥茶膏湯的【野地煎藥陶銚】！".to_string()))
        }
        _ => Err(format!("未知的草藥精煉模式：{mode}")),
    }
}

/// 纖維麻繩多插座精煉派發：支援 TwistRope, WeaveSackcloth, OilCanvas, PressPaper
pub fn refine_fiber_multi_socket(
    pid: &str,
    mode: &str,
    _room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

     match mode {
        "TwistRope" => {
            let candidates = ["mat_reed_bundle", "mat_sackcloth_bolt", "mat_fishing_line_bundle"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_hemp_rope_roll".to_string();
            let prod_name = "浸油粗麻繩卷".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你手掌合搓順紋牽引，將柔韌草麻纖維緊密絞成股線，盤捲成一卷結實牢靠的【浸油粗麻繩卷】收入行囊。".to_string()))
        }
        "WeaveSackcloth" => {
            let candidates = ["mat_hemp_rope_roll", "mat_reed_bundle"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_sackcloth_bolt".to_string();
            let prod_name = "粗織麻袋布匹".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你以經緯穿梭之法牽引麻絲密密交織，編排平整，織成一匹厚密結實的【粗織麻袋布匹】收入行囊。".to_string()))
        }
        "OilCanvas" => {
            let candidates = ["mat_sackcloth_bolt", "mat_canvas_roll", "mat_oilcloth_roll"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "craft_storm_canvas_cloak".to_string();
            let prod_name = "防風浸油帆布行旅斗篷".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你將密織帆布均勻塗刷熟桐油與天然膠脂，文火烘透，製成一領抗寒防浪的【防風浸油帆布行旅斗篷】收入行囊。".to_string()))
        }
        "PressPaper" => {
            let candidates = ["mat_reed_bundle", "mat_coarse_paper_ream", "mat_wrapping_paper_pack"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "craft_peninsula_almanac".to_string();
            let prod_name = "白塔風物考工手札".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_fiber", &prod_name);
            Ok((prod_id, prod_name, "你將紙漿均勻抄撈濾水、覆簾壓平焙乾，裝訂為一部頁面勻整吸墨的【白塔風物考工手札】收入行囊。".to_string()))
        }
        _ => Err(format!("未知的纖維精煉模式：{mode}")),
    }
}

/// 生皮水產多插座精煉派發：支援 ScrapeHide, BoilFishGlue, CalcifyShell, CobbleBoots
pub fn refine_hide_fish_multi_socket(
    pid: &str,
    mode: &str,
    _room: &str,
    now: i64,
) -> Result<(String, String, String), String> {
    let Ok(Some(ent)) = db::get_entity(pid) else {
        return Err("角色不存在".to_string());
    };

    match mode {
        "ScrapeHide" => {
            let candidates = ["mat_leather_strip", "mat_mane_hair_bundle"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_leather_strip".to_string();
            let prod_name = "修整牛皮帶料".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 2);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將生皮平鋪板上，以弧刃刮刀細細削去殘留油脂與雜質，反覆揉搓軟化，鞣製成兩條柔軟堅韌的【修整牛皮帶料】收入行囊。".to_string()))
        }
        "BoilFishGlue" => {
            let candidates = ["mat_fish_bone_sack", "mat_fish_scale_sack", "mat_sealing_pitch"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_sealing_pitch".to_string();
            let prod_name = "熬煉濃魚膠盒".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將鮮魚皮骨下鍋文火慢熬，濾去骨渣濃縮收汁，凝煉出一盒黏合性極高、遇熱化開的【熬煉濃魚膠盒】收入行囊。".to_string()))
        }
        "CalcifyShell" => {
            let candidates = ["mat_shell_sack", "mat_stone_powder_sack"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "mat_stone_powder_sack".to_string();
            let prod_name = "煅燒貝灰粉袋".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你將厚實海貝投入熾熱火堆高溫煅燒，出爐後研碎成白淨細膩的【煅燒貝灰粉袋】，防潮密封極佳，收入行囊。".to_string()))
        }
        "CobbleBoots" => {
            let candidates = ["mat_leather_strip", "craft_heavy_leather_boots"];
            for c in candidates {
                if db::inventory_has_item(&ent.inventory, c) {
                    let _ = db::remove_from_inventory(pid, c, 1);
                    break;
                }
            }
            let prod_id = "craft_heavy_leather_boots".to_string();
            let prod_name = "厚底防滑行旅皮靴".to_string();
            let _ = db::add_to_inventory(pid, &prod_id, 1);
            db::log_npc_event(now, pid, "refine_hide_fish", &prod_name);
            Ok((prod_id, prod_name, "你以粗麻蠟線手工縫合熟牛皮與硬木齒底，靴幫貼合牢靠，縫製出一雙【厚底防滑行旅皮靴】收入行囊。".to_string()))
        }
        _ => Err(format!("未知的皮魚精煉模式：{mode}")),
    }
}

/// 獲取房間內當前存續的動態野地設施。
pub fn active_field_facility_objects(room_id_or_name: &str, now: i64) -> Vec<crate::model::RoomObject> {
    let mut objs = Vec::new();
    let norm_room = normalize_room_key(room_id_or_name);
    let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());

    let facilities = ledger.active_field_facilities.get(&norm_room)
        .or_else(|| ledger.active_field_facilities.get(room_id_or_name));

    if let Some(list) = facilities {
        for f in list {
            if f.expires_at > now {
                if f.facility_id == "dyn_field_anvil" {
                    let mut responses = HashMap::new();
                    responses.insert("Look".to_string(), "一座穩穩楔入泥地中的平整玄武岩石砧，石面被鐵錘敲擊得光潔如鑑，周圍散落著少許鍛屑，可供往來行者修具磨刃。".to_string());
                    responses.insert("ForgeRepairStrip".to_string(), "你將鐵料放置於石砧上，手起錘落叮噹作響，反覆鍛打修整出平整的熟鐵薄片。".to_string());
                    responses.insert("SharpenBlade".to_string(), "你取過油石在石砧邊緣浸水細磨，金鐵交鳴聲中，刃面浮現冷冽寒芒。".to_string());
                    objs.push(crate::model::RoomObject {
                        id: "dyn_field_anvil".to_string(),
                        name: "野地修具石砧".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "ForgeRepairStrip".to_string(), "SharpenBlade".to_string()],
                        responses,
                        move_to_room_id: String::new(),
                    });
                } else if f.facility_id == "dyn_herb_stove" {
                    let mut responses = HashMap::new();
                    responses.insert("Look".to_string(), "一座以三枚耐火石塊架起的陶質煎藥銚，下襯暗紅微炭，壺嘴徐徐飄散著提神醒腦的草木清香。".to_string());
                    responses.insert("BoilDecoction".to_string(), "你往陶銚中注入山泉並投入藥草末，蓋上陶蓋文火慢煎，濃郁藥香隨蒸氣繚繞飄散。".to_string());
                    responses.insert("DryHerb".to_string(), "你利用陶銚周圍溫熱的石面烘焙藥草，葉片水分迅速蒸散、藥性得以完整鎖存。".to_string());
                    responses.insert("CrushHerb".to_string(), "你借用銚旁的小石臼細心研磨草藥，藥草在清脆的研擊聲中化為均勻細粉。".to_string());
                    objs.push(crate::model::RoomObject {
                        id: "dyn_herb_stove".to_string(),
                        name: "野地煎藥陶銚".to_string(),
                        owner: String::new(),
                        sockets: vec!["Look".to_string(), "BoilDecoction".to_string(), "DryHerb".to_string(), "CrushHerb".to_string()],
                        responses,
                        move_to_room_id: String::new(),
                    });
                }
            }
        }
    }
    objs
}

/// 與野地動態設施互動。
pub fn interact_with_field_facility(
    pid: &str,
    room: &str,
    facility_id: &str,
    action: &str,
    now: i64,
) -> Result<String, String> {
    match facility_id {
        "dyn_field_anvil" => match action {
            "Look" => Ok("這是一座質地密實、表面平整的玄武岩野地石砧，擺放穩妥，可供過路行旅現場鍛整修補鐵條或磨礪武器鋒刃。".to_string()),
            "ForgeRepairStrip" => refine_mineral_multi_socket(pid, "ForgeRepairStrip", room, now).map(|r| r.2),
            "SharpenBlade" => refine_mineral_multi_socket(pid, "SharpenBlade", room, now).map(|r| r.2),
            _ => Err(format!("野地修具石砧不支援操作：{action}")),
        },
        "dyn_herb_stove" => match action {
            "Look" => Ok("這是一具野地架設的煎藥陶銚，微火溫煨，銚口熱氣裊裊，能供行者煎熬藥膏、烘焙藥茶或研碎藥草。".to_string()),
            "BoilDecoction" => refine_herb_multi_socket(pid, "BoilDecoction", room, now).map(|r| r.2),
            "DryHerb" => refine_herb_multi_socket(pid, "DryHerb", room, now).map(|r| r.2),
            "CrushHerb" => refine_herb_multi_socket(pid, "CrushHerb", room, now).map(|r| r.2),
            _ => Err(format!("野地煎藥陶銚不支援操作：{action}")),
        },
        _ => Err(format!("未知的野地設施：{facility_id}")),
    }
}

/// 依物品 ID 查詢其可在行囊中觸發的動作插座按鈕列表。
pub fn inventory_item_actions(item_id: &str) -> Vec<String> {
    if is_mineral_item(item_id) {
        vec![
            "WashOre".into(),
            "SmeltBloom".into(),
            "ForgeRepairStrip".into(),
            "SharpenBlade".into(),
            "BuildFieldAnvil".into(),
        ]
    } else if is_herb_item(item_id) {
        vec![
            "WashHerb".into(),
            "DryHerb".into(),
            "CrushHerb".into(),
            "BoilDecoction".into(),
            "SetupHerbStove".into(),
        ]
    } else if is_fiber_item(item_id) {
        vec![
            "TwistRope".into(),
            "WeaveSackcloth".into(),
            "OilCanvas".into(),
            "PressPaper".into(),
        ]
    } else if is_hide_fish_item(item_id) {
        vec![
            "ScrapeHide".into(),
            "BoilFishGlue".into(),
            "CalcifyShell".into(),
            "CobbleBoots".into(),
        ]
    } else if is_wood_or_firewood_item(item_id) {
        vec![
            "MakeFire".into(),
            "SplitWood".into(),
            "RefinePlank".into(),
            "BurnCharcoal".into(),
            "CraftTorch".into(),
            "AddFuel".into(),
            "RepairStructure".into(),
        ]
    } else {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_base_materials_and_finished_goods_have_canonical_white_tower_origins() {
        for item in crate::world::materials::material_item_defs() {
            assert!(
                !item.origin.is_empty(),
                "Material {} missing origin",
                item.id
            );
            let settlement = crate::world::materials::infer_origin_settlement(&item.origin);
            assert!(
                !settlement.is_empty(),
                "Material {} origin '{}' did not resolve to a canonical White Tower settlement",
                item.id,
                item.origin
            );
            for legacy in ["中鎮", "北岸渡口", "邊市口", "岸邊短市", "短市", "小市", "聖塔"] {
                assert!(
                    !item.origin.contains(legacy) && !item.description.contains(legacy),
                    "Material {} still contains legacy term '{}': origin={}, desc={}",
                    item.id,
                    legacy,
                    item.origin,
                    item.description
                );
            }
        }

        for fg in &STANDARD_FINISHED_GOODS {
            assert!(!fg.origin.is_empty(), "Finished good {} missing origin", fg.id);
            let settlement = crate::world::materials::infer_origin_settlement(fg.origin);
            assert!(
                !settlement.is_empty(),
                "Finished good {} origin '{}' did not resolve to a canonical White Tower settlement",
                fg.id,
                fg.origin
            );
        }
    }

    #[test]
    fn all_five_material_families_expose_tier1_multi_sockets() {
        let wood_actions = inventory_item_actions("mat_timber_log");
        assert!(wood_actions.contains(&"SplitWood".to_string()));
        assert!(wood_actions.contains(&"RefinePlank".to_string()));

        let mineral_actions = inventory_item_actions("mat_rough_ore");
        assert!(mineral_actions.contains(&"WashOre".to_string()));
        assert!(mineral_actions.contains(&"SmeltBloom".to_string()));
        assert!(mineral_actions.contains(&"BuildFieldAnvil".to_string()));

        let herb_actions = inventory_item_actions("mat_foraged_herb");
        assert!(herb_actions.contains(&"WashHerb".to_string()));
        assert!(herb_actions.contains(&"BoilDecoction".to_string()));
        assert!(herb_actions.contains(&"SetupHerbStove".to_string()));

        let fiber_actions = inventory_item_actions("mat_reed_bundle");
        assert!(fiber_actions.contains(&"TwistRope".to_string()));
        assert!(fiber_actions.contains(&"WeaveSackcloth".to_string()));
        assert!(fiber_actions.contains(&"PressPaper".to_string()));

        let hide_fish_actions = inventory_item_actions("mat_fish_scale_sack");
        assert!(hide_fish_actions.contains(&"BoilFishGlue".to_string()));
        assert!(hide_fish_actions.contains(&"CobbleBoots".to_string()));
    }

    #[test]
    fn free_economy_emergent_settlement_pricing_and_saturation_works() {
        // 1) 產地市集 vs 緊缺消費地市集：無硬編碼百分比，純依庫存供需自生浮動價格
        let hongtu_quote = dynamic_settlement_market_quote("紅土崁", "mat_rough_ore", 10);
        let wenyao_quote = dynamic_settlement_market_quote("文曜", "mat_rough_ore", 10);
        assert!(
            wenyao_quote.unit_price > hongtu_quote.unit_price,
            "文曜消費地價格 ({}) 應高於產地紅土崁價格 ({})",
            wenyao_quote.unit_price,
            hongtu_quote.unit_price
        );
        assert!(
            hongtu_quote.local_stock > hongtu_quote.target_stock,
            "紅土崁為產地，現貨應大於基準目標"
        );
        assert!(
            wenyao_quote.local_stock < wenyao_quote.target_stock,
            "文曜為緊缺地，現貨應小於基準目標"
        );

        // 2) 持續向文曜市集注水售料：庫存遞增，價格單調下降，直至市場飽和價格低於產地
        let mut last_price = wenyao_quote.unit_price;
        let mut prev_stock = wenyao_quote.local_stock;
        for _ in 0..20 {
            let after = record_settlement_market_trade("文曜", "mat_rough_ore", 10, 1, "玩家售出");
            assert!(
                after.local_stock > prev_stock,
                "持續售料應使本地庫存實質增加"
            );
            prev_stock = after.local_stock;
            assert!(
                after.unit_price <= last_price,
                "庫存增加應使市集收購行情單調遞減或持平：當前 {} > 前次 {}",
                after.unit_price,
                last_price
            );
            last_price = after.unit_price;
        }
        assert!(
            last_price < hongtu_quote.unit_price,
            "大量注水後文曜市集價格 ({}) 應被壓低至低於產地原始價格 ({})",
            last_price,
            hongtu_quote.unit_price
        );

        // 3) 從霜林口採買提料：庫存減少，市場稀缺度攀升，單價相應上揚
        let before_buy = dynamic_settlement_market_quote("霜林口", "mat_plank_board", 10);
        let after_buy = record_settlement_market_trade("霜林口", "mat_plank_board", 10, -5, "玩家採買");
        assert!(
            after_buy.local_stock < before_buy.local_stock,
            "採買應實質消耗本地庫存"
        );
        assert!(
            after_buy.unit_price > before_buy.unit_price,
            "物資消耗抽空後價格應上漲：採買後 {} 應大於採買前 {}",
            after_buy.unit_price,
            before_buy.unit_price
        );
    }

    #[test]
    fn workshop_halts_when_raw_material_stock_is_zero_and_all_1120_catalog_items_registered() {
        if crate::store::get_store().is_none() {
            let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let rooms = manifest_dir.join("data/rooms");
            let runtime = manifest_dir.join("data/runtime");
            let data = manifest_dir.join("data");
            let _ = crate::store::init(
                rooms.to_str().unwrap_or("data/rooms"),
                runtime.to_str().unwrap_or("data/runtime"),
                data.to_str().unwrap_or("data"),
            );
        }
        ensure_ecology_items_in_store();
        if let Some(arc) = crate::store::get_store() {
            let s = arc.read();
            assert!(s.items.len() >= 1120, "Store items 應至少包含 1,120 件物件");
            assert!(s.items.contains_key("wt_cat_0001"));
            assert!(s.items.contains_key("wt_cat_1109"));
        }

        let raw_id = "mat_iron_scrap";
        let finished_id = STANDARD_FINISHED_GOODS[0].id;
        let settlement = "文曜";
        let room_name = "文曜萬爐鐵坊棚";
        let raw_key = (settlement.to_string(), raw_id.to_string());
        let finished_key = (settlement.to_string(), finished_id.to_string());

        let initial_ema = {
            let mut ledger = LIVE_ECONOMY_LEDGER.write().unwrap_or_else(|e| e.into_inner());
            let cell = ledger
                .settlement_markets
                .entry(raw_key.clone())
                .or_insert_with(|| init_settlement_market_cell(settlement, raw_id));
            cell.stock = 0;
            cell.demand_ema = 1.0;
            cell.demand_ema
        };

        let initial_finished_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&finished_key).map(|c| c.stock).unwrap_or(0)
        };

        let npc_id = "test_artisan_npc";
        let result = run_npc_autonomous_production_step("test_room", room_name, &[npc_id.to_string()], 1729000000);

        assert!(result.is_some(), "工坊待料時應產出短缺敘事反饋");
        let narr = result.unwrap();
        assert!(narr.contains("已經見底") || narr.contains("暫停手頭工序"));

        let after_finished_stock = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&finished_key).map(|c| c.stock).unwrap_or(0)
        };
        assert_eq!(after_finished_stock, initial_finished_stock);

        let after_ema = {
            let ledger = LIVE_ECONOMY_LEDGER.read().unwrap_or_else(|e| e.into_inner());
            ledger.settlement_markets.get(&raw_key).map(|c| c.demand_ema).unwrap_or(1.0)
        };
        assert!(after_ema > initial_ema);

        let report = format_settlement_supply_demand_report("文曜");
        assert!(report.contains("【文曜市集供需實況】"));
        assert!(report.contains("充裕供給：") && !report.contains("充裕供給：尚無顯著過剩物資"));
        assert!(report.contains("緊缺急需：") && !report.contains("緊缺急需：市集百工儲備充裕"));

        let scarcest = scarcest_item_for_settlement("文曜");
        assert!(scarcest.is_some());
    }

    #[test]
    fn test_crystal_items_lookup_and_material_classification() {
        use crate::combat::wordplate::{WordQuality, WordSource};

        // 1. 驗證 15 款詞元結晶之 lookup_ecology_item_def 與 lookup_item_origin
        let sources = [
            WordSource::Beast,
            WordSource::Plant,
            WordSource::Mineral,
            WordSource::Water,
            WordSource::Qi,
        ];
        let qualities = [WordQuality::Bing, WordQuality::Yi, WordQuality::Jia];

        for src in sources {
            for qual in qualities {
                let crystal_id = crate::combat::wordplate::crystal_item_id(src, qual);
                let origin = lookup_item_origin(&crystal_id);
                assert_eq!(origin, "白塔半島・天地五源");

                let def = lookup_ecology_item_def(&crystal_id);
                assert!(def.is_some(), "結晶物品 {crystal_id} 必須有定義");
                let d = def.unwrap();
                assert_eq!(d.item_type, "word_crystal");
                assert_eq!(d.weight, 0.1);
                assert!(d.name.contains(src.label_zh()));
                assert!(d.name.contains(qual.label_zh()));
                assert!(d.description.contains("舍利子態結晶"));
            }
        }

        // 2. 驗證 classify_material_word_source 排除與分類規則
        assert!(classify_material_word_source("crystal_beast_jia", "甲品・獸源詞元結晶").is_none());
        assert!(classify_material_word_source("starter_blade", "初心者長刀").is_none());
        assert!(classify_material_word_source("key_iron", "鐵鑰匙").is_none());

        // 獸源 (Beast)
        let (src, tier) = classify_material_word_source("wt_cat_0176", "泥甲巨鱷厚麟皮").unwrap();
        assert_eq!(src, WordSource::Beast);
        assert_eq!(tier, 4);

        let (src, tier) = classify_material_word_source("wt_cat_0171", "山林石虎斑皮").unwrap();
        assert_eq!(src, WordSource::Beast);
        assert_eq!(tier, 3);

        // 水源 (Water)
        let (src, tier) = classify_material_word_source("wt_cat_0236", "大花枝鮮黑墨囊").unwrap();
        assert_eq!(src, WordSource::Water);
        assert_eq!(tier, 4);

        let (src, tier) = classify_material_word_source("mat_salted_fish", "水尾寮醃鹹魚").unwrap();
        assert_eq!(src, WordSource::Water);
        assert_eq!(tier, 2);

        // 礦源 (Mineral)
        let (src, tier) = classify_material_word_source("mat_rough_ore", "粗鐵礦石").unwrap();
        assert_eq!(src, WordSource::Mineral);
        assert_eq!(tier, 2);

        let (src, tier) = classify_material_word_source("mat_stone", "玄武岩石板").unwrap();
        assert_eq!(src, WordSource::Mineral);
        assert_eq!(tier, 3);

        // 植源 (Plant)
        let (src, tier) = classify_material_word_source("mat_foraged_herb", "山野鮮藥草").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 2);

        let (src, tier) = classify_material_word_source("wt_cat_0319", "甕藏陳年老菜脯").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 3);

        let (src, tier) = classify_material_word_source("wt_cat_herb", "野生深山牛樟芝").unwrap();
        assert_eq!(src, WordSource::Plant);
        assert_eq!(tier, 4);

        // 氣源 (Qi fallback)
        let (src, tier) = classify_material_word_source("mat_incense_dust", "晨嵐清風散").unwrap();
        assert_eq!(src, WordSource::Qi);
        assert_eq!(tier, 2);
    }

    #[test]
    fn test_classify_catalog_wearable_slot_and_lookup() {
        // 1. 排除重型固定設施與生鮮原料
        assert!(classify_catalog_wearable_slot("水力原木大鋸", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("紅土耐火磚高溫鐵爐", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("萬鈞玄武岩重錨", "19", "").is_none());
        assert!(classify_catalog_wearable_slot("老樟木重型木工工作台", "18", "").is_none());
        assert!(classify_catalog_wearable_slot("平底載重八人木渡船", "19", "").is_none());
        assert!(classify_catalog_wearable_slot("赤鐵重礦", "01", "").is_none());

        // 2. 手持百工器具 / 刀具（器物無分工具與武器）
        let blade = classify_catalog_wearable_slot("大型手拉兩用剖木大鋸", "18", "");
        assert_eq!(blade, Some(("equipment", "hold_r", 0, 2, 4)));

        let shears = classify_catalog_wearable_slot("熟鐵裁布大剪刀", "16", "");
        assert_eq!(shears, Some(("equipment", "hold_r", 0, 2, 4)));

        let knife = classify_catalog_wearable_slot("岩羊角手柄折疊獵刀", "19", "");
        assert_eq!(knife, Some(("equipment", "hold_r", 0, 2, 4)));

        let pick = classify_catalog_wearable_slot("雙刃碎石手鎬", "18", "");
        assert_eq!(pick, Some(("equipment", "hold_r", 2, 0, 4)));

        let shovel = classify_catalog_wearable_slot("長把平頭挖泥鐵鍬", "18", "");
        assert_eq!(shovel, Some(("equipment", "hold_r", 1, 2, 3)));

        let torch = classify_catalog_wearable_slot("手拋式三色信號火把", "19", "");
        assert_eq!(torch, Some(("equipment", "hold_l", 2, 2, 1)));

        // 3. 隨身穿戴服飾防具與飾品
        let hat = classify_catalog_wearable_slot("雙層防雨斗笠", "16", "");
        assert_eq!(hat, Some(("equipment", "head", 2, 1, 0)));

        let goggles = classify_catalog_wearable_slot("雙扣防雪盲皮革護目風鏡", "19", "");
        assert_eq!(goggles, Some(("equipment", "face", 1, 2, 0)));

        let boots = classify_catalog_wearable_slot("熟成防水牛皮短靴", "16", "");
        assert_eq!(boots, Some(("equipment", "feet", 2, 3, 0)));

        let warmer = classify_catalog_wearable_slot("滑石隨身暖手爐", "17", "");
        assert_eq!(warmer, Some(("equipment", "trinket", 2, 1, 0)));

        // 4. lookup_ecology_item_def 火把與長柄
        let pine_torch = lookup_ecology_item_def("craft_pine_torch").unwrap();
        assert_eq!(pine_torch.slot, "hold_l");
        assert_eq!(pine_torch.item_type, "equipment");

        let handle1 = lookup_ecology_item_def("craft_wooden_tool_handle").unwrap();
        assert_eq!(handle1.slot, "hold_r");
        assert_eq!(handle1.item_type, "equipment");

        let handle2 = lookup_ecology_item_def("craft_wooden_handle").unwrap();
        assert_eq!(handle2.slot, "hold_r");
        assert_eq!(handle2.item_type, "equipment");
    }

    #[test]
    fn test_craft_dims_and_shanhai_cooking_highlight_integration() {
        let dims = crate::server::handler::entity_total_craft_dims("unit_test_actor");
        assert!(dims.forge >= 0.0);
        assert!(dims.culinary >= 0.0);
        assert!(dims.harvest >= 0.0);
        assert!(dims.wayfare >= 0.0);

        let packet = crate::ai::CraftActionFactPacket {
            actor_name: "行旅刀客".to_string(),
            tool_name: "行旅銅釜".to_string(),
            tool_familiarity: "沉穩趁手".to_string(),
            breath_habit: "潮汐連息".to_string(),
            material_names: vec!["山胡椒馬告粒".to_string(), "馬告炭炙高地水鹿腱".to_string()],
            output_name: "馬告炭炙高地水鹿腱".to_string(),
            highlight_kind: "山海營火合味現煮".to_string(),
            environment_hint: "野地營火堆".to_string(),
        };
        let highlight = crate::ai::compose_craft_highlight_from_data(&packet);
        assert!(!highlight.is_empty(), "LLM craft highlight must generate fallback or local prose");
        assert!(!highlight.contains("系統"), "Highlight cannot contain forbidden word 系統");
        assert!(!highlight.contains("玩家"), "Highlight cannot contain forbidden word 玩家");
        assert!(!highlight.chars().any(|c| c.is_ascii_digit()), "Highlight cannot contain raw digits");

        let (extra, note) = harvest_from_world_object("unit_test_actor", "obj_wild_ore", "露天鐵礦石", "harvest_ore", 1_700_000_000);
        assert!(extra >= 0);
        if extra > 0 {
            assert!(note.contains("【採集剝名】藝相純熟") || note.contains("趁手"));
        }
    }
}

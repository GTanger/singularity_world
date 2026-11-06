//! P0 可交易世界物質：固定來源節點、白板材料、簡單庫存。

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::store::Item;

const P0_MATERIALS_JSON: &str = include_str!("../../data/world/tradable_materials_p0.json");
const DEFAULT_STATE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/data/runtime/material_sources.json"
);

#[derive(Debug, Deserialize)]
struct MaterialConfig {
    #[allow(dead_code)]
    version: i32,
    #[serde(default)]
    sources: Vec<MaterialSource>,
    #[serde(default)]
    materials: Vec<MaterialDef>,
    #[serde(default)]
    legacy_item_remaps: Vec<LegacyItemRemap>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MaterialSource {
    pub id: String,
    pub room_id: String,
    pub cell_id: String,
    pub room_name: String,
    pub source_class: String,
    pub origin_label: String,
    pub material_item_id: String,
    pub stock_max: i32,
    pub recover_per_day: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MaterialDef {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub category: String,
    #[serde(default)]
    pub category_tag: String,
    pub commodity_role: String,
    #[allow(dead_code)]
    pub commodity_group: String,
    pub source_class: String,
    pub source_node_id: String,
    pub origin_label: String,
    pub weight: f64,
    pub stackable: i32,
    pub tradable: bool,
    pub trade_priority: i32,
    pub trade_price_anchor_mg: i32,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
struct LegacyItemRemap {
    from: String,
    to: String,
    #[allow(dead_code)]
    reason: String,
}

#[derive(Debug, Clone)]
pub struct GatheredMaterial {
    pub item_id: String,
    pub name: String,
    pub source_node_id: String,
    pub source_room_name: String,
    pub origin_label: String,
    pub stock_remaining: i32,
}

#[derive(Debug, Clone)]
pub struct MaterialSourceStockView {
    pub source_id: String,
    pub material_item_id: String,
    pub material_name: String,
    pub source_room_name: String,
    pub origin_label: String,
    pub stock: i32,
    pub stock_max: i32,
    pub recover_per_day: i32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SourceStateFile {
    #[serde(default)]
    sources: Vec<SourceStateRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SourceStateRow {
    source_id: String,
    stock: i32,
    updated_day: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct InventoryRow {
    #[serde(rename = "item_id")]
    item_id: String,
    qty: i32,
}

static CONFIG: OnceLock<MaterialConfig> = OnceLock::new();
static SOURCE_STATE: OnceLock<Mutex<SourceStateFile>> = OnceLock::new();

fn config() -> &'static MaterialConfig {
    CONFIG.get_or_init(|| {
        serde_json::from_str(P0_MATERIALS_JSON)
            .expect("data/world/tradable_materials_p0.json must be valid")
    })
}

fn source_state() -> &'static Mutex<SourceStateFile> {
    SOURCE_STATE.get_or_init(|| Mutex::new(load_source_state()))
}

fn state_path() -> PathBuf {
    std::env::var("SINGULARITY_MATERIAL_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(DEFAULT_STATE_PATH))
}

fn load_source_state() -> SourceStateFile {
    let path = state_path();
    let Ok(raw) = fs::read_to_string(path) else {
        return SourceStateFile::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn persist_source_state(state: &SourceStateFile) {
    let path = state_path();
    let Some(dir) = path.parent() else {
        return;
    };
    if let Err(err) = fs::create_dir_all(dir) {
        tracing::warn!("material source state mkdir failed: {err}");
        return;
    }
    let Ok(raw) = serde_json::to_vec_pretty(state) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if let Err(err) = fs::write(&tmp, raw) {
        tracing::warn!("material source state write failed: {err}");
        return;
    }
    if fs::rename(&tmp, &path).is_err() {
        let _ = fs::remove_file(&tmp);
    }
}

fn unix_day(now_unix: i64) -> i64 {
    now_unix.max(0).div_euclid(86_400)
}

fn source_by_room_id(room_id: &str) -> Option<&'static MaterialSource> {
    config().sources.iter().find(|s| s.room_id == room_id)
}

#[allow(dead_code)]
fn source_by_node_id(source_node_id: &str) -> Option<&'static MaterialSource> {
    config().sources.iter().find(|s| s.id == source_node_id)
}

pub fn material_by_item_id(item_id: &str) -> Option<&'static MaterialDef> {
    config().materials.iter().find(|m| m.id == item_id)
}

pub fn material_for_room_id(room_id: &str) -> Option<&'static MaterialDef> {
    let source = source_by_room_id(room_id)?;
    material_by_item_id(&source.material_item_id)
}

pub fn material_for_cell_id(cell_id: &str) -> Option<&'static MaterialDef> {
    let source = config().sources.iter().find(|s| s.cell_id == cell_id)?;
    material_by_item_id(&source.material_item_id)
}

pub fn is_tradable_item(item_id: &str) -> bool {
    material_by_item_id(item_id).is_some_and(|m| m.tradable)
}

pub fn trade_price_anchor_mg(item_id: &str) -> Option<i32> {
    material_by_item_id(item_id)
        .filter(|m| m.tradable)
        .map(|m| m.trade_price_anchor_mg.max(1))
}

pub fn trade_offer_priority(item_id: &str) -> Option<i32> {
    material_by_item_id(item_id)
        .filter(|m| m.tradable)
        .map(|m| m.trade_priority)
}

pub fn material_item_defs() -> Vec<Item> {
    config()
        .materials
        .iter()
        .map(|m| {
            let description = if !m.description.contains("【產地：") && !m.origin_label.is_empty() {
                format!("{}【產地：{}】", m.description, m.origin_label)
            } else {
                m.description.clone()
            };
            Item {
                id: m.id.clone(),
                name: m.name.clone(),
                slot: String::new(),
                item_type: m.item_type.clone(),
                weight: m.weight,
                stackable: m.stackable,
                denomination: 0,
                description,
                vit_bonus: 0,
                dex_bonus: 0,
                atk_bonus: 0,
                origin: m.origin_label.clone(),
            }
        })
        .collect()
}

#[must_use]
pub fn infer_origin_settlement(origin_text: &str) -> &'static str {
    let resolved_room;
    let text = if origin_text.starts_with("grid:") {
        if let Some(coord) = crate::grid::parse_grid_room_id(origin_text)
            && let Some(cell) = crate::server::grid_manager::get_runtime_square_grid()
                .and_then(|g| g.get(coord).cloned())
        {
            resolved_room = format!("{} {}", cell.zone, cell.name);
            resolved_room.as_str()
        } else {
            origin_text
        }
    } else {
        origin_text
    };
    for s in [
        "霜林口", "紅土崁", "三號坑口", "下營", "黑沼關隘", "半嶺鎮", "半嶺", "分家巷", "水尾寮",
        "大圳頭", "牛埔", "烏石鼻", "鹽田澳", "風舶角", "風舶嶼", "北嶺腳", "樟木驛", "竹林寨",
        "老堰頭", "風櫃尾", "白茫洲", "南哨", "文曜", "黑石灣", "赤石坡", "長風草甸", "翠冠巨木林", "青竹坪",
        "古河巨木林", "落日岬", "驚濤岩岸", "巨藻淺灣", "清泉溪谷", "北坡雲杉林", "西麓林道",
        "淺澤泥口", "貼山深潭", "西嶺礦谷", "東海峭壁", "雲腰雪林", "白塔雪巔",
    ] {
        if text.contains(s) {
            return s;
        }
    }
    ""
}

pub fn legacy_item_remap(item_id: &str) -> Option<&'static str> {
    config()
        .legacy_item_remaps
        .iter()
        .find(|r| r.from == item_id)
        .map(|r| r.to.as_str())
}

pub fn normalize_legacy_inventory_json(raw: &str) -> Option<String> {
    if raw.trim().is_empty() || raw.trim() == "[]" {
        return None;
    }
    let rows: Vec<InventoryRow> = serde_json::from_str(raw).ok()?;
    let mut changed = false;
    let mut merged: BTreeMap<String, i32> = BTreeMap::new();
    for row in rows {
        if row.qty <= 0 || row.item_id.is_empty() {
            continue;
        }
        if row.item_id.starts_with("crystal_") {
            changed = true;
            continue;
        }
        let base_id = if row.item_id.starts_with("mut_blank_manual")
            || row.item_id.starts_with("mut_wt_blank_manual")
        {
            "blank_manual"
        } else if row.item_id.starts_with("mut_craft_meridian_salve") {
            "craft_meridian_salve"
        } else if row.item_id.starts_with("mut_craft_pine_soot_ink") {
            "craft_pine_soot_ink"
        } else if row.item_id.starts_with("mut_craft_pine_torch") {
            "craft_pine_torch"
        } else if row.item_id.starts_with("mut_craft_bamboo_oil_umbrella") {
            "craft_bamboo_oil_umbrella"
        } else if row.item_id.starts_with("mut_craft_camphor_storage_box") {
            "craft_camphor_storage_box"
        } else if row.item_id.starts_with("mut_craft_wooden_tool_handle") {
            "craft_wooden_tool_handle"
        } else if row.item_id.starts_with("mut_wt_cat_0793") {
            "wt_cat_0793"
        } else {
            row.item_id.as_str()
        };
        let mapped = legacy_item_remap(base_id).unwrap_or(base_id);
        if mapped != row.item_id {
            changed = true;
        }
        *merged.entry(mapped.to_string()).or_default() += row.qty;
    }
    if !changed {
        return None;
    }
    let normalized: Vec<InventoryRow> = merged
        .into_iter()
        .filter(|(_, qty)| *qty > 0)
        .map(|(item_id, qty)| InventoryRow { item_id, qty })
        .collect();
    serde_json::to_string(&normalized).ok()
}

fn reserve_source_stock(source: &MaterialSource, now_unix: i64, min_remaining: i32) -> Option<i32> {
    let today = unix_day(now_unix);
    let mut guard = source_state().lock().ok()?;
    let floor = min_remaining.max(0);
    let Some(idx) = guard
        .sources
        .iter()
        .position(|row| row.source_id == source.id)
    else {
        guard.sources.push(SourceStateRow {
            source_id: source.id.clone(),
            stock: source.stock_max.max(0),
            updated_day: today,
        });
        let idx = guard.sources.len() - 1;
        if guard.sources[idx].stock <= floor {
            persist_source_state(&guard);
            return None;
        }
        guard.sources[idx].stock -= 1;
        let remaining = guard.sources[idx].stock;
        persist_source_state(&guard);
        return Some(remaining);
    };
    let days = (today - guard.sources[idx].updated_day).max(0);
    if days > 0 {
        let recovered = days.saturating_mul(i64::from(source.recover_per_day.max(0)));
        let stock = i64::from(guard.sources[idx].stock).saturating_add(recovered);
        guard.sources[idx].stock = stock.min(i64::from(source.stock_max.max(0))) as i32;
        guard.sources[idx].updated_day = today;
    }
    if guard.sources[idx].stock <= floor {
        persist_source_state(&guard);
        return None;
    }
    guard.sources[idx].stock -= 1;
    let remaining = guard.sources[idx].stock;
    persist_source_state(&guard);
    Some(remaining)
}

fn peek_source_stock(source: &MaterialSource, now_unix: i64) -> Option<i32> {
    let today = unix_day(now_unix);
    let mut guard = source_state().lock().ok()?;
    let Some(idx) = guard
        .sources
        .iter()
        .position(|row| row.source_id == source.id)
    else {
        guard.sources.push(SourceStateRow {
            source_id: source.id.clone(),
            stock: source.stock_max.max(0),
            updated_day: today,
        });
        let stock = guard
            .sources
            .last()
            .map(|row| row.stock)
            .unwrap_or_default();
        persist_source_state(&guard);
        return Some(stock);
    };
    let days = (today - guard.sources[idx].updated_day).max(0);
    if days > 0 {
        let recovered = days.saturating_mul(i64::from(source.recover_per_day.max(0)));
        let stock = i64::from(guard.sources[idx].stock).saturating_add(recovered);
        guard.sources[idx].stock = stock.min(i64::from(source.stock_max.max(0))) as i32;
        guard.sources[idx].updated_day = today;
        persist_source_state(&guard);
    }
    Some(guard.sources[idx].stock)
}

pub fn material_source_stock_for_room(
    room_id: &str,
    now_unix: i64,
) -> Option<MaterialSourceStockView> {
    let source = source_by_room_id(room_id)?;
    let material = material_by_item_id(&source.material_item_id)?;
    let stock = peek_source_stock(source, now_unix)?;
    Some(MaterialSourceStockView {
        source_id: source.id.clone(),
        material_item_id: material.id.clone(),
        material_name: material.name.clone(),
        source_room_name: source.room_name.clone(),
        origin_label: source.origin_label.clone(),
        stock,
        stock_max: source.stock_max.max(0),
        recover_per_day: source.recover_per_day.max(0),
    })
}

pub fn try_gather_material_for_room(room_id: &str, now_unix: i64) -> Option<GatheredMaterial> {
    try_gather_material_for_room_preserving(room_id, now_unix, 0)
}

pub fn try_gather_material_for_room_preserving(
    room_id: &str,
    now_unix: i64,
    min_remaining: i32,
) -> Option<GatheredMaterial> {
    let source = source_by_room_id(room_id)?;
    let material = material_by_item_id(&source.material_item_id)?;
    let stock_remaining = reserve_source_stock(source, now_unix, min_remaining)?;
    Some(GatheredMaterial {
        item_id: material.id.clone(),
        name: material.name.clone(),
        source_node_id: source.id.clone(),
        source_room_name: source.room_name.clone(),
        origin_label: source.origin_label.clone(),
        stock_remaining,
    })
}

pub fn material_origin_label(item_id: &str) -> Option<&'static str> {
    let material = material_by_item_id(item_id)?;
    Some(material.origin_label.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn p0_sources_point_to_existing_commodity_materials() {
        let material_ids: BTreeSet<&str> =
            config().materials.iter().map(|m| m.id.as_str()).collect();
        for material in &config().materials {
            assert!(
                source_by_node_id(&material.source_node_id).is_some(),
                "{} source_node_id must exist",
                material.id
            );
            assert!(
                !material.category.is_empty(),
                "{} missing category",
                material.id
            );
            assert!(
                !material.commodity_role.is_empty(),
                "{} missing commodity_role",
                material.id
            );
            assert!(
                material.trade_priority >= 0,
                "{} trade_priority must be non-negative",
                material.id
            );
        }
        for source in &config().sources {
            assert!(
                material_ids.contains(source.material_item_id.as_str()),
                "{} points to unknown material {}",
                source.id,
                source.material_item_id
            );
        }
    }

    #[test]
    fn p0_commodity_catalog_covers_existing_world_nodes() {
        let source_ids: BTreeSet<&str> = config().sources.iter().map(|s| s.id.as_str()).collect();
        let room_ids: BTreeSet<&str> = config()
            .sources
            .iter()
            .map(|s| s.room_id.as_str())
            .collect();
        let material_ids: BTreeSet<&str> =
            config().materials.iter().map(|m| m.id.as_str()).collect();

        assert_eq!(source_ids.len(), config().sources.len());
        assert_eq!(room_ids.len(), config().sources.len());
        assert_eq!(material_ids.len(), config().materials.len());
        assert_eq!(config().sources.len(), 70);
        assert!((25..=35).contains(&config().materials.len()));
        assert!(config().sources.len() > config().materials.len() * 2);
        assert!(room_ids.contains("grid:0:0")); // 半嶺水岸渡口
        assert!(room_ids.contains("grid:0:5")); // 田區路口
        assert!(room_ids.contains("grid:-3:5")); // 洗礦溝
        assert!(room_ids.contains("grid:-5:0")); // 牧地柵門
        assert!(room_ids.contains("grid:0:-5")); // 糧倉外秤
        assert!(room_ids.contains("grid:9:-5")); // 外貿碼頭
    }

    #[test]
    fn p0_commodity_categories_cover_trade_layers() {
        let categories: BTreeSet<&str> = config()
            .materials
            .iter()
            .map(|m| m.category.as_str())
            .collect();
        for required in [
            "食糧",
            "鹽",
            "燃料",
            "繩索/纖維",
            "木材/板料",
            "礦料",
            "修船料",
            "建材",
            "畜產副料",
            "水產副料",
            "包裝料",
            "區域特產",
        ] {
            assert!(categories.contains(required), "missing category {required}");
        }
    }

    #[test]
    fn p0_sources_merge_into_shared_player_commodities() {
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for source in &config().sources {
            *counts.entry(source.material_item_id.as_str()).or_default() += 1;
        }
        assert!(
            counts
                .get("mat_hemp_rope_roll")
                .is_some_and(|count| *count >= 6)
        );
        assert!(
            counts
                .get("mat_plank_board")
                .is_some_and(|count| *count >= 4)
        );
        assert!(
            counts
                .get("mat_grain_sack")
                .is_some_and(|count| *count >= 3)
        );
    }

    #[test]
    fn p0_player_commodity_names_drop_source_specific_scraps() {
        let names: BTreeSet<&str> = config().materials.iter().map(|m| m.name.as_str()).collect();
        for rejected in ["渡板濕繩段", "坡街封繩", "牧場木牌片", "湖港苔木屑"] {
            assert!(
                !names.contains(rejected),
                "rejected player commodity leaked: {rejected}"
            );
        }
    }

    #[test]
    fn room_lookup_returns_shared_north_bank_commodity() {
        let material = material_for_room_id("grid:0:0").expect("north bank source");
        assert_eq!(material.id, "mat_reed_bundle");
        assert_eq!(material.name, "蘆草束");
        assert_eq!(trade_price_anchor_mg(&material.id), Some(6));
    }

    #[test]
    fn legacy_inventory_normalizes_and_merges() {
        let raw =
            r#"[{"item_id":"wild_herb","qty":2},{"item_id":"mat_north_bank_reed_bundle","qty":1}]"#;
        let normalized = normalize_legacy_inventory_json(raw).expect("legacy remap");
        assert_eq!(normalized, r#"[{"item_id":"mat_reed_bundle","qty":3}]"#);
        let raw_mut = r#"[{"item_id":"mut_blank_manual_atk_1","qty":1}]"#;
        let norm_mut = normalize_legacy_inventory_json(raw_mut).expect("mut_blank_manual normalize");
        assert_eq!(norm_mut, r#"[{"item_id":"blank_manual","qty":1}]"#);
    }
}

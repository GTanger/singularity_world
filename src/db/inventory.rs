//! 背包讀寫與裝備槽（對齊既有 `db/inventory`）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::store;

use super::ErrNoStore;

/// 每點體質可負重（對齊既有 `weightPerVit`）。
const WEIGHT_PER_VIT: f64 = 10.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryEntry {
    #[serde(rename = "item_id")]
    pub item_id: String,
    pub qty: i32,
}

/// 相容別名供既有模組與規範使用
pub type InvEntry = InventoryEntry;

/// NPC 單一物品堆疊上限（防止割草／採集無限累積）。
pub const MAX_NPC_ITEM_STACK_QTY: i32 = 12;
/// NPC 背包最多容納之相異物品種類數量上限。
pub const MAX_NPC_DISTINCT_ITEMS: usize = 8;

/// 判斷指定實體 ID 是否為 NPC。
pub fn is_npc_entity_id(entity_id: &str) -> bool {
    if entity_id.starts_with("npc_") || entity_id.starts_with("wt_") {
        return true;
    }
    store::get_store()
        .and_then(|arc| arc.read().get_entity(entity_id).map(|e| e.kind == "npc"))
        .unwrap_or(false)
}

/// 判斷物品 ID 是否屬於原物料／採集料。
fn is_raw_material_item_id(item_id: &str) -> bool {
    if item_id.starts_with("mat_") || item_id.starts_with("harvest_") || item_id.starts_with("ore_") || item_id.starts_with("wood_") {
        return true;
    }
    if let Some(num_str) = item_id.strip_prefix("wt_cat_") {
        if let Ok(num) = num_str.parse::<u32>() {
            if (1..=387).contains(&num) {
                return true;
            }
        }
    }
    false
}

/// 對 NPC 背包條目執行規整：
/// - 移除 `qty <= 0` 或 `item_id == "manual_854b8035e7134512"`
/// - 合併重複 `item_id`
/// - 將每項 `qty` 限制在 `MAX_NPC_ITEM_STACK_QTY` (`12`)
/// - 若相異種類數超過 `MAX_NPC_DISTINCT_ITEMS` (`8`)，優先丟棄超額的原料類條目（或超額條目）
/// - 若有任何實質變更則回傳 `true`
pub fn normalize_npc_inventory_entries(entries: &mut Vec<InventoryEntry>) -> bool {
    let original_len = entries.len();
    let mut changed = false;

    // 1. 移除無效或特定過期物品
    entries.retain(|e| {
        if e.qty <= 0 || e.item_id.is_empty() || e.item_id == "manual_854b8035e7134512" {
            changed = true;
            false
        } else {
            true
        }
    });

    // 2. 合併重複 item_id
    let mut merged: Vec<InventoryEntry> = Vec::with_capacity(entries.len());
    for e in entries.drain(..) {
        if let Some(existing) = merged.iter_mut().find(|m| m.item_id == e.item_id) {
            existing.qty += e.qty;
            changed = true;
        } else {
            merged.push(e);
        }
    }
    *entries = merged;

    // 3. Clamp 每一項數量至 MAX_NPC_ITEM_STACK_QTY (12)
    for e in entries.iter_mut() {
        if e.qty > MAX_NPC_ITEM_STACK_QTY {
            e.qty = MAX_NPC_ITEM_STACK_QTY;
            changed = true;
        }
    }

    // 4. 若超過上限，截斷/移除超出的原料條目或最早條目
    while entries.len() > MAX_NPC_DISTINCT_ITEMS {
        changed = true;
        if let Some(pos) = entries.iter().position(|e| is_raw_material_item_id(&e.item_id)) {
            entries.remove(pos);
        } else {
            entries.pop();
        }
    }

    if entries.len() != original_len {
        changed = true;
    }
    changed
}

/// 單列背包顯示（對齊既有 `InventoryDisplayItem`）。
#[derive(Debug, Clone)]
pub struct InventoryLine {
    pub item_id: String,
    pub name: String,
    pub item_type: String,
    pub qty: i32,
    pub weight: f64,
    pub sub_total: f64,
    pub description: String,
    pub slot: String,
    pub origin: String,
}

/// 背包查詢結果（對齊既有 `InventoryResult`）。
#[derive(Debug, Clone)]
pub struct InventorySnapshot {
    pub items: Vec<InventoryLine>,
    pub current_weight: f64,
    pub max_weight: f64,
}

/// 單一物品定義摘要（對齊既有 `GetItemInfo` 回傳欄位）。
#[derive(Debug, Clone)]
pub struct ItemDefBrief {
    pub name: String,
    pub item_type: String,
    pub slot: String,
    pub description: String,
    pub weight: f64,
    pub vit_bonus: i32,
    pub dex_bonus: i32,
    pub atk_bonus: i32,
    pub origin: String,
}

/// 將未在 items.json 或 materials 定義的物品 ID 映射至正體中文名稱
pub fn fallback_item_name(item_id: &str) -> Option<&'static str> {
    match item_id {
        "harvest_herb" => Some("野地草藥"),
        "harvest_hardstone" => Some("緻密硬石"),
        "harvest_rare_wood" => Some("溫木心材"),
        "harvest_deadwood" => Some("乾燥枯枝"),
        "harvest_berries" => Some("原野漿果"),
        "harvest_vine" => Some("深海巨藻藤"),
        "harvest_ore" => Some("赤紋鐵礦"),
        "harvest_wood" => Some("北坡雲杉原木"),
        "harvest_fish" => Some("鮮活河魚"),
        "harvest_crystal" => Some("寒水玉原石"),
        "herb_lingzhi" => Some("光葉藥草"),
        "ore_black_iron" => Some("青脊重礦"),
        "wood_iron_oak" => Some("沼生烏木"),
        "ore_copper" => Some("風切石原礦"),
        "ore_silver" => Some("夜光貝母"),
        "tea_leaf" => Some("翠嶺嫩茶"),
        "wild_fruit" => Some("山野鮮果"),
        "spring_water" => Some("清冽泉露"),
        "beast_scale" => Some("巨獸殘鱗"),
        "beast_claw_fragment" => Some("裂岩巨爪殘片"),
        "deed_white_tower_property" => Some("公議堂紅印屋契"),
        "key_brass_home_lock" => Some("黃銅彈子門鎖鑰匙"),
        "item_personal_astrolabe_scroll" => Some("周天命格星盤拓卷"),
        "item_oiled_bamboo_umbrella" => Some("雙層桐油防雨竹傘"),
        "item_spruce_votive_plaque" => Some("北嶺雲杉祈福木牌"),
        "item_white_tower_gazetteer_scroll" => Some("白塔風物孤本鈔卷"),
        "item_ebony_mortise_tea_tray" => Some("沉水烏木榫卯茶盤"),
        "item_quenched_steel_whetstone" => Some("雙面精鋼油石"),
        "item_indigo_rain_cloak" => Some("山藍防雨斗篷"),
        "item_coastal_sea_chart" => Some("黑潮順時針海圖摺頁"),
        "item_wenyao_cadastral_rubbing" => Some("文曜四坊地籍圖鈔頁"),
        "item_braised_beef_broth_jar" => Some("慢燉原牛肉湯陶盅"),
        "item_roasted_frost_tea_tin" => Some("炭焙霜林烘青茶罐"),
        "item_seven_star_moxa_pack" => Some("九蒸七星艾絨藥包"),
        "item_collated_stele_rubbing" => Some("精校金石碑帖拓本"),
        "item_polished_quartz_lens" => Some("精磨紅土水晶鏡片"),
        "item_smoked_mullet_roe_pack" => Some("風櫃尾冷燻烏魚子禮包"),
        "starter_body_m" | "starter_body_f" => Some("棉麻日常上衣"),
        "starter_legs_m" | "starter_legs_f" => Some("耐磨休閒長褲"),
        "starter_feet_m" | "starter_feet_f" => Some("輕便布鞋"),
        "starter_undershirt_m" | "starter_undershirt_f" => Some("純棉貼身內衫"),
        "starter_inner_armor_m" | "starter_inner_armor_f" => Some("輕便護甲內襯"),
        "starter_leg_armor_m" | "starter_leg_armor_f" => Some("耐磨綁腿護膝"),
        "粗布短褂" => Some("棉麻日常上衣"),
        "粗布長褲" => Some("耐磨休閒長褲"),
        "草鞋" => Some("輕便布鞋"),
        "mat_iron_scrap" => Some("回收熟鐵料塊"),
        "mat_timber_log" => Some("伐採原木"),
        "mat_reed_bundle" => Some("水澤老蘆草綑"),
        "mat_foraged_herb" => Some("山野鮮藥草"),
        "mat_coarse_paper_ream" => Some("原漿宣紙刀"),
        "mat_hemp_rope_roll" => Some("浸油粗麻繩卷"),
        "mat_plank_board" => Some("樟松木板料"),
        "mat_salted_fish" => Some("海鹽醃魚乾"),
        "mat_fresh_shrimp" => Some("水尾寮鮮沼蝦"),
        "shanhai_fresh_fish" => Some("深海鮮魚"),
        "shanhai_beast_meat" => Some("山海鮮獸肉"),
        "mat_grain_sack" => Some("農家精選穀糧袋"),
        "mat_pasture_seed_sack" => Some("優質牧草種籽袋"),
        "mat_herb_poultice" => Some("百草外敷藥膏"),
        "mat_beast_hide" => Some("粗製獸皮"),
        "craft_roasted_shrimp_skewer" => Some("炭炙水尾鮮蝦串"),
        "craft_smoked_fish_fillet" => Some("相思木燻鮮魚乾"),
        "craft_spiced_beast_jerky" => Some("馬告炭炙獸肉脯"),
        "craft_ration_grain_cake" => Some("行旅耐儲米糧餅"),
        "craft_wooden_tool_handle" => Some("魯班平推刨刀"),
        "craft_bamboo_basket" => Some("紫竹日常提籃"),
        "craft_copper_abacus" => Some("紫銅商簿算盤"),
        "craft_kitchen_cleaver" => Some("厚背熟鐵廚刀"),
        "craft_peninsula_almanac" => Some("白塔風物考工手札"),
        "craft_meridian_salve" => Some("青銅搗藥杵"),
        "craft_iron_pickaxe" => Some("百鍛長柄鐵錘"),
        "craft_steel_scissors" => Some("熟鐵裁布大剪刀"),
        "craft_pine_torch" => Some("浸油麻編海釣竿"),
        "craft_steel_hunting_knife" => Some("精鍛防身獵刀"),
        _ => crate::world::author_economy::runtime_good_display_name(item_id),
    }
}


/// 將物品堆疊寫入實體的 `inventory` JSON；`qty` 為 0 或 `item_id` 空則 no-op。成功時觸發 `persist_entities`。
pub fn add_to_inventory(entity_id: &str, item_id: &str, qty: i32) -> anyhow::Result<()> {
    if qty == 0 || item_id.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        let is_npc = e.kind == "npc" || entity_id.starts_with("npc_") || entity_id.starts_with("wt_");
        let mut raw = e.inventory.clone();
        if raw.is_empty() {
            raw = "[]".to_string();
        }
        let mut entries: Vec<InventoryEntry> = serde_json::from_str(&raw).unwrap_or_default();
        let mut found = false;
        for ent in &mut entries {
            if ent.item_id == item_id {
                if is_npc {
                    ent.qty = (ent.qty + qty).min(MAX_NPC_ITEM_STACK_QTY);
                } else {
                    ent.qty += qty;
                }
                found = true;
                break;
            }
        }
        if !found {
            let effective_qty = if is_npc {
                qty.min(MAX_NPC_ITEM_STACK_QTY)
            } else {
                qty
            };
            if is_npc && entries.len() >= MAX_NPC_DISTINCT_ITEMS {
                if let Some(pos) = entries.iter().position(|ent| is_raw_material_item_id(&ent.item_id)) {
                    entries[pos] = InventoryEntry {
                        item_id: item_id.to_string(),
                        qty: effective_qty,
                    };
                } else {
                    entries[0] = InventoryEntry {
                        item_id: item_id.to_string(),
                        qty: effective_qty,
                    };
                }
            } else {
                entries.push(InventoryEntry {
                    item_id: item_id.to_string(),
                    qty: effective_qty,
                });
            }
        }
        if is_npc {
            normalize_npc_inventory_entries(&mut entries);
        }
        if let Ok(b) = serde_json::to_string(&entries) {
            e.inventory = b;
        }
    })
}
/// 便捷別名：將物品新增至背包（等同 `add_to_inventory`）
pub fn add_item(entity_id: &str, item_id: &str, qty: i32) -> anyhow::Result<()> {
    add_to_inventory(entity_id, item_id, qty)
}

/// 自背包 JSON 扣減數量；堆疊歸零則移除該列（對齊既有 `RemoveFromInventory` 語意）。
pub fn remove_from_inventory(entity_id: &str, item_id: &str, qty: i32) -> anyhow::Result<()> {
    if qty <= 0 || item_id.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        let mut raw = e.inventory.clone();
        if raw.is_empty() {
            raw = "[]".to_string();
        }
        let mut entries: Vec<InventoryEntry> = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => return,
        };
        for ent in &mut entries {
            if ent.item_id == item_id {
                ent.qty -= qty;
                break;
            }
        }
        entries.retain(|e| e.qty > 0 && !e.item_id.is_empty());
        if let Ok(b) = serde_json::to_string(&entries) {
            e.inventory = b;
        }
    })
}

/// 由 PostgreSQL 共同行動入口原子完成「買方付鎂、賣方收鎂、物品反向移動」。
pub fn settle_inventory_trade(
    buyer_id: &str,
    seller_id: &str,
    item_id: &str,
    qty: i32,
    price_mg: i32,
    now_unix: i64,
    source_ref: &str,
) -> anyhow::Result<crate::pg::world_trade::TradeSettlement> {
    let pool = crate::pg::pool().ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    let settlement = crate::pg::world_trade::settle(
        &pool, buyer_id, seller_id, item_id, qty, price_mg, now_unix, source_ref,
    )?;
    mirror_trade_settlement(&settlement);
    Ok(settlement)
}

/// NPC 只在市場把實際持物賣給同地、付得起的另一名 NPC；沒有買家就不成交。
pub fn settle_npc_inventory_sale(
    seller_id: &str,
    item_id: &str,
    qty: i32,
    price_mg: i32,
    now_unix: i64,
) -> anyhow::Result<Option<crate::pg::world_trade::TradeSettlement>> {
    let pool = crate::pg::pool().ok_or_else(|| anyhow::anyhow!("PostgreSQL 不可用"))?;
    let settlement = crate::pg::world_trade::settle_npc_sale(
        &pool,
        seller_id,
        item_id,
        qty,
        price_mg,
        now_unix,
        "npc_market_trade",
    )?;
    if let Some(ref settlement) = settlement {
        mirror_trade_settlement(settlement);
    }
    Ok(settlement)
}

fn mirror_trade_settlement(settlement: &crate::pg::world_trade::TradeSettlement) {
    let Some(arc) = store::get_store() else {
        tracing::warn!("交易已提交但 store 尚未就緒：{}", settlement.trade_id);
        return;
    };
    let mut store = arc.write();
    for party in [&settlement.buyer, &settlement.seller] {
        if let Err(error) = store.mirror_economy_from_authority(
            &party.entity_id,
            party.magnesium,
            &party.inventory,
            &party.current_activity,
        ) {
            tracing::warn!(
                "交易 {} 快取校正失敗 {}：{}",
                settlement.trade_id,
                party.entity_id,
                error
            );
        }
    }
}

/// 市場成交後，把 PostgreSQL 權威狀態校正回人物快取；市場庫存與財庫本身不做雙寫。
pub fn mirror_market_trade_settlement(settlement: &crate::pg::world_trade::MarketTradeSettlement) {
    let Some(arc) = store::get_store() else {
        tracing::warn!("市場交易已提交但 store 尚未就緒：{}", settlement.trade_id);
        return;
    };
    let mut store = arc.write();
    for party in [&settlement.buyer, &settlement.operator] {
        if let Err(error) = store.mirror_economy_from_authority(
            &party.entity_id,
            party.magnesium,
            &party.inventory,
            &party.current_activity,
        ) {
            tracing::warn!(
                "市場交易 {} 快取校正失敗 {}：{}",
                settlement.trade_id,
                party.entity_id,
                error
            );
        }
    }
}

/// 背包內所有物品數量加總（僅 `qty > 0`；對齊既有 `DecisionState.InventoryTotalQty`）。
#[must_use]
pub fn inventory_total_qty(inventory_json: &str) -> i32 {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return 0;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    entries
        .iter()
        .filter(|e| e.qty > 0 && !e.item_id.is_empty())
        .map(|e| e.qty)
        .sum()
}

/// 只計算現行白塔市場商品；退役 `mat_*` 與舊測試物不再驅動居民交易決策。
#[must_use]
pub fn active_trade_inventory_total_qty(inventory_json: &str) -> i32 {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return 0;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    entries
        .iter()
        .filter(|entry| {
            entry.qty > 0 && crate::world::author_economy::is_runtime_good_item(&entry.item_id)
        })
        .map(|entry| entry.qty)
        .sum()
}

/// 自 `inventory` JSON 與 `items` 表組裝前端用清單與負重（對齊既有 `GetInventory`）。
#[must_use]
pub fn get_inventory(inventory_json: &str, vit: i32) -> InventorySnapshot {
    let max_weight = vit as f64 * WEIGHT_PER_VIT;
    if inventory_json.is_empty() || inventory_json == "[]" {
        return InventorySnapshot {
            items: Vec::new(),
            current_weight: 0.0,
            max_weight,
        };
    }
    let (raw_json, is_npc) = if inventory_json.trim_start().starts_with('[') {
        (inventory_json.to_string(), false)
    } else if let Some(arc) = store::get_store() {
        if let Some(ent) = arc.read().get_entity(inventory_json) {
            let npc = is_npc_entity_id(inventory_json) || ent.kind == "npc";
            (ent.inventory, npc)
        } else {
            (inventory_json.to_string(), is_npc_entity_id(inventory_json))
        }
    } else {
        (inventory_json.to_string(), is_npc_entity_id(inventory_json))
    };
    let entries: Vec<InventoryEntry> = match serde_json::from_str(&raw_json) {
        Ok(v) => v,
        Err(_) => {
            return InventorySnapshot {
                items: Vec::new(),
                current_weight: 0.0,
                max_weight,
            };
        }
    };
    let store_arc = store::get_store();
    let mut items: Vec<InventoryLine> = Vec::new();
    let mut total_weight = 0.0;
    for e in entries {
        if e.qty <= 0 || e.item_id.is_empty() || e.item_id.starts_with("crystal_") {
            continue;
        }
        let (name, item_type, description, slot, weight, origin) = if let Some(it) = store_arc.as_ref().and_then(|arc| arc.read().get_item(&e.item_id).cloned())
        {
            let origin = if !it.origin.is_empty() {
                it.origin.clone()
            } else {
                crate::game::object_ecology::lookup_item_origin(&e.item_id)
            };
            (
                it.name.clone(),
                it.item_type.clone(),
                it.description.clone(),
                it.slot.clone(),
                it.weight,
                origin,
            )
        } else if let Some(mat) = crate::world::materials::material_by_item_id(&e.item_id) {
            (
                mat.name.clone(),
                mat.item_type.clone(),
                mat.description.clone(),
                String::new(),
                mat.weight,
                mat.origin_label.clone(),
            )
        } else if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(&e.item_id) {
            let eco = crate::game::object_ecology::lookup_ecology_item_def(&e.item_id);
            let (item_type, slot) = if let Some(eco_def) = &eco {
                let itype = if !eco_def.item_type.is_empty() {
                    eco_def.item_type.clone()
                } else {
                    cat.category_name.to_string()
                };
                (itype, eco_def.slot.clone())
            } else {
                (cat.category_name.to_string(), String::new())
            };
            (
                cat.name.to_string(),
                item_type,
                cat.look_text.replace("，主要用於", "；"),
                slot,
                crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
                format!("白塔半島・{}", cat.workshop),
            )
        } else if let Some(eco) = crate::game::object_ecology::lookup_ecology_item_def(&e.item_id) {
            (
                eco.name,
                eco.item_type,
                eco.description,
                eco.slot,
                eco.weight,
                eco.origin,
            )
        } else if let Some(fallback_name) = fallback_item_name(&e.item_id) {
            let (itype, slot_str, desc, weight_val) = match e.item_id.as_str() {
                "item_braised_beef_broth_jar" => (
                    "consumable",
                    "",
                    "三水大食堂紅鐵大釜慢火細燉的大角原牛肉與甜白蘿蔔濃湯，封於保溫粗陶盅內，香氣四溢。",
                    0.6,
                ),
                "item_roasted_frost_tea_tin" => (
                    "consumable",
                    "",
                    "東橋山風茶屋以相思木炭文火慢焙的霜林高地烘青茶葉，錫罐密封，茶湯清冽回甘。",
                    0.3,
                ),
                "item_seven_star_moxa_pack" => (
                    "consumable",
                    "",
                    "溫氏百草烘藥房以紅泥焙爐九蒸九曝精製的陳年七星艾絨與當歸藥包，溫經通絡、芳香避穢。",
                    0.3,
                ),
                "item_collated_stele_rubbing" => (
                    "misc",
                    "",
                    "講堂街校勘齋與藏書閣東巷逐字比勘摹拓的白塔古碑金石拓本，墨色烏金、字口鋒稜畢現。",
                    0.2,
                ),
                "item_polished_quartz_lens" => (
                    "misc",
                    "",
                    "觀星台以紅土崁天然白水晶與細金剛砂精磨校準的雙凸光學鏡片，澄澈無瑕、聚光精準。",
                    0.2,
                ),
                "item_smoked_mullet_roe_pack" => (
                    "consumable",
                    "",
                    "三水口乾貨海味行嚴選的風櫃尾海鹽壓製、相思木冷燻烏魚子，色澤金珀、鹹鮮綿密。",
                    0.5,
                ),
                _ => ("material", "", "採集或探索獲得的物產。", 0.5),
            };
            (
                fallback_name.to_string(),
                itype.into(),
                desc.into(),
                slot_str.into(),
                weight_val,
                "白塔半島".to_string(),
            )
        } else {
            (
                e.item_id.clone(),
                "misc".into(),
                String::new(),
                String::new(),
                0.0,
                String::new(),
            )
        };
        let eff_qty = if is_npc {
            e.qty.min(MAX_NPC_ITEM_STACK_QTY)
        } else {
            e.qty
        };
        let sub = weight * f64::from(eff_qty);
        items.push(InventoryLine {
            item_id: e.item_id,
            name,
            item_type,
            qty: eff_qty,
            weight,
            sub_total: sub,
            description,
            slot,
            origin,
        });
        total_weight += sub;
    }
    InventorySnapshot {
        items,
        current_weight: total_weight,
        max_weight,
    }
}

/// 查單一物品定義；無 store 或無此 id 則 `None`（對齊既有 `GetItemInfo` 之失敗路徑）。
#[must_use]
pub fn get_item_def(item_id: &str) -> Option<ItemDefBrief> {
    if let Some(arc) = store::get_store()
        && let Some(s) = arc.try_read()
    {
        if let Some(it) = s.get_item(item_id) {
            let origin = if !it.origin.is_empty() {
                it.origin.clone()
            } else {
                crate::game::object_ecology::lookup_item_origin(item_id)
            };
            return Some(ItemDefBrief {
                name: it.name.clone(),
                item_type: it.item_type.clone(),
                slot: it.slot.clone(),
                description: it.description.clone(),
                weight: it.weight,
                vit_bonus: it.vit_bonus,
                dex_bonus: it.dex_bonus,
                atk_bonus: it.atk_bonus,
                origin,
            });
        }
    }
    if let Some(mat) = crate::world::materials::material_by_item_id(item_id) {
        return Some(ItemDefBrief {
            name: mat.name.clone(),
            item_type: mat.item_type.clone(),
            slot: String::new(),
            description: mat.description.clone(),
            weight: mat.weight,
            vit_bonus: 0,
            dex_bonus: 0,
            atk_bonus: 0,
            origin: mat.origin_label.clone(),
        });
    }
    if let Some(cat) = crate::game::peninsula_catalog::find_catalog_entry(item_id) {
        let eco = crate::game::object_ecology::lookup_ecology_item_def(item_id);
        let (item_type, slot, vit_bonus, dex_bonus, atk_bonus) = if let Some(eco_def) = eco {
            let itype = if !eco_def.item_type.is_empty() {
                eco_def.item_type
            } else {
                cat.category_name.to_string()
            };
            (itype, eco_def.slot, eco_def.vit_bonus, eco_def.dex_bonus, eco_def.atk_bonus)
        } else {
            (cat.category_name.to_string(), String::new(), 0, 0, 0)
        };
        return Some(ItemDefBrief {
            name: cat.name.to_string(),
            item_type,
            slot,
            description: cat.look_text.replace("，主要用於", "；"),
            weight: crate::game::peninsula_catalog::catalog_entry_weight_kg(cat),
            vit_bonus,
            dex_bonus,
            atk_bonus,
            origin: format!("白塔半島・{}", cat.workshop),
        });
    }
    if let Some(eco) = crate::game::object_ecology::lookup_ecology_item_def(item_id) {
        return Some(eco);
    }
    if let Some(name) = fallback_item_name(item_id) {
        let (item_type, slot, desc) = match item_id {
            "starter_body_m" | "starter_body_f" | "粗布短褂" => ("equipment", "body", "質樸舒適的棉麻日常上衣。"),
            "starter_legs_m" | "starter_legs_f" | "粗布長褲" => ("equipment", "legs", "耐磨且便於活動的日常休閒長褲。"),
            "starter_feet_m" | "starter_feet_f" | "草鞋" => ("equipment", "feet", "輕便透氣的平底布鞋。"),
            "starter_undershirt_m" | "starter_undershirt_f" => ("equipment", "undershirt", "吸汗透氣的純棉內衫。"),
            "starter_inner_armor_m" | "starter_inner_armor_f" => ("equipment", "inner_armor", "輕薄貼身的軟韌護甲。"),
            "starter_leg_armor_m" | "starter_leg_armor_f" => ("equipment", "leg_armor", "保護小腿與膝關節的束帶。"),
            "item_braised_beef_broth_jar" => (
                "consumable",
                "",
                "三水大食堂紅鐵大釜慢火細燉的大角原牛肉與甜白蘿蔔濃湯，封於保溫粗陶盅內，香氣四溢。",
            ),
            "item_roasted_frost_tea_tin" => (
                "consumable",
                "",
                "東橋山風茶屋以相思木炭文火慢焙的霜林高地烘青茶葉，錫罐密封，茶湯清冽回甘。",
            ),
            "item_seven_star_moxa_pack" => (
                "consumable",
                "",
                "溫氏百草烘藥房以紅泥焙爐九蒸九曝精製的陳年七星艾絨與當歸藥包，溫經通絡、芳香避穢。",
            ),
            "item_collated_stele_rubbing" => (
                "misc",
                "",
                "講堂街校勘齋與藏書閣東巷逐字比勘摹拓的白塔古碑金石拓本，墨色烏金、字口鋒稜畢現。",
            ),
            "item_polished_quartz_lens" => (
                "misc",
                "",
                "觀星台以紅土崁天然白水晶與細金剛砂精磨校準的雙凸光學鏡片，澄澈無瑕、聚光精準。",
            ),
            "item_smoked_mullet_roe_pack" => (
                "consumable",
                "",
                "三水口乾貨海味行嚴選的風櫃尾海鹽壓製、相思木冷燻烏魚子，色澤金珀、鹹鮮綿密。",
            ),
            _ => ("material", "", "採集或探索獲得的物產。"),
        };
        let weight = match item_id {
            "item_braised_beef_broth_jar" => 0.6,
            "item_roasted_frost_tea_tin" | "item_seven_star_moxa_pack" => 0.3,
            "item_collated_stele_rubbing" | "item_polished_quartz_lens" => 0.2,
            "item_smoked_mullet_roe_pack" => 0.5,
            _ => 0.5,
        };
        return Some(ItemDefBrief {
            name: name.to_string(),
            item_type: item_type.into(),
            slot: slot.into(),
            description: desc.into(),
            weight,
            dex_bonus: 0,
            atk_bonus: 0,
            vit_bonus: 0,
            origin: "白塔半島".into(),
        });
    }
    None
}

/// 查詢單一物品名稱（若查無則回退為 item_id）
#[must_use]
pub fn get_item_name(item_id: &str) -> String {
    if let Some(def) = get_item_def(item_id) {
        return def.name;
    }
    if let Some(fallback) = fallback_item_name(item_id) {
        return fallback.to_string();
    }
    item_id.to_string()
}

/// 背包目前總重量（對齊既有 `InventoryWeight`）。
#[must_use]
pub fn inventory_weight(inventory_json: &str) -> f64 {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return 0.0;
    }
    let (raw_json, is_npc) = if inventory_json.trim_start().starts_with('[') {
        (inventory_json.to_string(), false)
    } else if let Some(arc) = store::get_store() {
        if let Some(ent) = arc.read().get_entity(inventory_json) {
            let npc = is_npc_entity_id(inventory_json) || ent.kind == "npc";
            (ent.inventory, npc)
        } else {
            (inventory_json.to_string(), is_npc_entity_id(inventory_json))
        }
    } else {
        (inventory_json.to_string(), is_npc_entity_id(inventory_json))
    };
    let entries: Vec<InventoryEntry> = serde_json::from_str(&raw_json).unwrap_or_default();
    let arc_opt = store::get_store();
    let s_opt = arc_opt.as_ref().map(|arc| arc.read());
    let mut total = 0.0;
    for e in entries {
        if e.qty <= 0 || e.item_id.is_empty() || e.item_id.starts_with("crystal_") {
            continue;
        }
        let w = s_opt
            .as_ref()
            .and_then(|s| s.get_item(&e.item_id).map(|it| it.weight))
            .or_else(|| crate::world::materials::material_by_item_id(&e.item_id).map(|m| m.weight))
            .or_else(|| {
                crate::game::peninsula_catalog::find_catalog_entry(&e.item_id)
                    .map(crate::game::peninsula_catalog::catalog_entry_weight_kg)
            })
            .or_else(|| crate::game::object_ecology::lookup_ecology_item_def(&e.item_id).map(|eco| eco.weight))
            .or_else(|| match e.item_id.as_str() {
                "item_braised_beef_broth_jar" => Some(0.6),
                "item_roasted_frost_tea_tin" | "item_seven_star_moxa_pack" => Some(0.3),
                "item_collated_stele_rubbing" | "item_polished_quartz_lens" => Some(0.2),
                "item_smoked_mullet_roe_pack" => Some(0.5),
                _ => fallback_item_name(&e.item_id).map(|_| 0.5),
            })
            .unwrap_or(0.0);
        let eff_qty = if is_npc {
            e.qty.min(MAX_NPC_ITEM_STACK_QTY)
        } else {
            e.qty
        };
        total += w * f64::from(eff_qty);
    }
    total
}

/// 設定單一裝備槽之 `item_id`（對齊既有 `UpdateEquipmentSlot`）。
pub fn update_equipment_slot(entity_id: &str, slot: &str, item_id: &str) -> anyhow::Result<()> {
    if slot.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        let mut map: HashMap<String, String> = if e.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&e.equipment_slots).unwrap_or_default()
        };
        map.insert(slot.to_string(), item_id.to_string());
        if let Ok(b) = serde_json::to_string(&map) {
            e.equipment_slots = b;
        }
    })
}

/// 清空單一裝備槽（對齊既有 `ClearEquipmentSlot`）。
pub fn clear_equipment_slot(entity_id: &str, slot: &str) -> anyhow::Result<()> {
    if slot.is_empty() {
        return Ok(());
    }
    let arc = store::get_store().ok_or(ErrNoStore)?;
    let mut s = arc.write();
    s.update_entity(entity_id, |e| {
        let mut map: HashMap<String, String> = if e.equipment_slots.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&e.equipment_slots).unwrap_or_default()
        };
        map.remove(slot);
        if let Ok(b) = serde_json::to_string(&map) {
            e.equipment_slots = b;
        }
    })
}

/// 背包 JSON 或實體 ID 中是否至少有一件指定 `item_id` 且 `qty > 0`。
#[must_use]
pub fn inventory_has_item(inventory_json_or_entity_id: &str, item_id: &str) -> bool {
    inventory_has_qty(inventory_json_or_entity_id, item_id, 1)
}

/// 背包 JSON 或實體 ID 中是否至少有指定數量的 `item_id`。
#[must_use]
pub fn inventory_has_qty(inventory_json_or_entity_id: &str, item_id: &str, qty: i32) -> bool {
    if inventory_json_or_entity_id.is_empty() || item_id.is_empty() || qty <= 0 {
        return false;
    }
    let raw = if inventory_json_or_entity_id.trim_start().starts_with('[') {
        inventory_json_or_entity_id.to_string()
    } else if let Some(arc) = store::get_store() {
        if let Some(ent) = arc.read().get_entity(inventory_json_or_entity_id) {
            ent.inventory
        } else {
            inventory_json_or_entity_id.to_string()
        }
    } else {
        inventory_json_or_entity_id.to_string()
    };
    if raw.is_empty() || raw == "[]" {
        return false;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(&raw).unwrap_or_default();
    entries.iter().any(|e| e.item_id == item_id && e.qty >= qty)
}
/// 從實體（NPC 或玩家）背包中選取最佳交易商品，並嚴格排除目前裝備中的器物。
#[must_use]
pub fn pick_entity_trade_offer(inventory_json: &str, equipment_slots_json: &str) -> Option<String> {
    if inventory_json.is_empty() || inventory_json == "[]" {
        return None;
    }
    let entries: Vec<InventoryEntry> = serde_json::from_str(inventory_json).unwrap_or_default();
    let equipped_ids: std::collections::HashSet<String> = if !equipment_slots_json.is_empty() {
        serde_json::from_str::<std::collections::HashMap<String, String>>(equipment_slots_json)
            .map(|map| map.into_values().filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    } else {
        std::collections::HashSet::new()
    };

    const PRIORITIZED_FINISHED_GOODS: &[&str] = &[
        "wt_cat_0388",
        "wt_cat_0279",
        "wt_cat_0278",
        "wt_cat_0293",
    ];

    // 1. 首選白塔半島旗艦成品
    for &target_item in PRIORITIZED_FINISHED_GOODS {
        if !equipped_ids.contains(target_item)
            && entries.iter().any(|e| e.item_id == target_item && e.qty > 0)
        {
            return Some(target_item.to_string());
        }
    }

    // 2. 次選各工坊工藝產物／標準製成品（嚴格排除非交易器物與裝備中器物）
    if let Some(craft_item) = entries
        .iter()
        .filter(|e| {
            e.qty > 0
                && !equipped_ids.contains(&e.item_id)
                && e.item_id != "wt_cat_0973"
                && e.item_id != "wt_cat_0984"
                && (e.item_id.starts_with("craft_")
                    || crate::game::object_ecology::STANDARD_FINISHED_GOODS
                        .iter()
                        .any(|fg| fg.id == e.item_id))
        })
        .map(|e| e.item_id.clone())
        .min()
    {
        return Some(craft_item);
    }

    // 3. 最後退回通例可交易商品（如 wt-good-* 等）
    entries
        .into_iter()
        .filter(|entry| {
            entry.qty > 0
                && !equipped_ids.contains(&entry.item_id)
                && entry.item_id != "wt_cat_0973"
                && entry.item_id != "wt_cat_0984"
                && item_is_tradable(&entry.item_id)
        })
        .map(|entry| entry.item_id)
        .min()
}

/// 從 NPC 背包取現行白塔市場商品；退役 `mat_*` 不再進入主動交易。
#[must_use]
pub fn pick_npc_trade_offer(inventory_json: &str) -> Option<String> {
    pick_entity_trade_offer(inventory_json, "")
}

/// 人物持有的現行商品缺少所在地市場報價時才使用的相容喊價。
#[must_use]
pub fn default_trade_ask_mg(item_id: &str) -> i32 {
    crate::world::materials::trade_price_anchor_mg(item_id)
        .or_else(|| {
            for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                if fg.id == item_id {
                    return Some(fg.base_price);
                }
            }
            crate::game::object_ecology::lookup_ecology_item_def(item_id).and_then(|eco| {
                for fg in &crate::game::object_ecology::STANDARD_FINISHED_GOODS {
                    if fg.name == eco.name {
                        return Some(fg.base_price);
                    }
                }
                None
            })
        })
        .or_else(|| {
            crate::game::peninsula_catalog::find_catalog_entry(item_id)
                .map(|c| crate::game::peninsula_catalog::catalog_entry_price_mg(c).max(6))
        })
        .unwrap_or(8)
}

#[must_use]
pub fn item_is_tradable(item_id: &str) -> bool {
    crate::world::author_economy::is_runtime_good_item(item_id)
}

/// 議價底價＝喊價七成（至少 1）（對齊既有 `TradeFloorFromAsk`）。
#[must_use]
pub fn trade_floor_from_ask(ask: i32) -> i32 {
    if ask <= 0 {
        return 1;
    }
    let f = ask * 7 / 10;
    if f < 1 { 1 } else { f }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_has_qty_requires_enough_quantity() {
        let inventory = r#"[{"item_id":"wild_herb","qty":2},{"item_id":"stone","qty":0}]"#;

        assert!(inventory_has_qty(inventory, "wild_herb", 1));
        assert!(inventory_has_qty(inventory, "wild_herb", 2));
        assert!(!inventory_has_qty(inventory, "wild_herb", 3));
        assert!(!inventory_has_qty(inventory, "stone", 1));
        assert!(!inventory_has_qty(inventory, "", 1));
        assert!(!inventory_has_qty(inventory, "wild_herb", 0));
    }

    #[test]
    fn inventory_has_item_is_one_or_more() {
        assert!(inventory_has_item(
            r#"[{"item_id":"wild_herb","qty":1}]"#,
            "wild_herb"
        ));
        assert!(!inventory_has_item(
            r#"[{"item_id":"wild_herb","qty":0}]"#,
            "wild_herb"
        ));
    }

    #[test]
    fn trade_offer_skips_legacy_and_unknown_items() {
        let inventory = r#"[{"item_id":"wild_herb","qty":3},{"item_id":"stone","qty":1}]"#;
        assert_eq!(pick_npc_trade_offer(inventory), None);
    }

    #[test]
    fn trade_offer_uses_runtime_market_good_only() {
        let inventory =
            r#"[{"item_id":"mat_wet_grass_bundle","qty":3},{"item_id":"wt-good-070","qty":1}]"#;
        assert_eq!(
            pick_npc_trade_offer(inventory).as_deref(),
            Some("wt-good-070")
        );
        assert_eq!(default_trade_ask_mg("wt-good-070"), 8);
    }

    #[test]
    fn trade_offer_is_stable_and_ignores_retired_materials() {
        let inventory = r#"[{"item_id":"wt-good-070","qty":1},{"item_id":"mat_grain_sack","qty":9},{"item_id":"wt-good-012","qty":1}]"#;
        assert_eq!(
            pick_npc_trade_offer(inventory).as_deref(),
            Some("wt-good-012")
        );
    }

    #[test]
    fn active_trade_quantity_excludes_legacy_inventory() {
        let inventory =
            r#"[{"item_id":"mat_reed_bundle","qty":5000},{"item_id":"wt-good-012","qty":2}]"#;
        assert_eq!(active_trade_inventory_total_qty(inventory), 2);
    }

    #[test]
    fn test_pick_entity_trade_offer_excludes_equipped_and_prioritizes_craft() {
        let inv = r#"[
            {"item_id":"wt_cat_0984","qty":1},
            {"item_id":"craft_wooden_tool_handle","qty":2},
            {"item_id":"wt-good-012","qty":5}
        ]"#;
        // 當裝備持有 craft_wooden_tool_handle 時，排除它並選擇 wt-good-012
        let equip = r#"{"hold_r":"craft_wooden_tool_handle"}"#;
        assert_eq!(
            pick_entity_trade_offer(inv, equip).as_deref(),
            Some("wt-good-012")
        );
        // 當未裝備時，優先選擇工藝產物 craft_wooden_tool_handle 而非 wt-good-012
        assert_eq!(
            pick_entity_trade_offer(inv, "{}").as_deref(),
            Some("craft_wooden_tool_handle")
        );
    }

    #[test]
    fn fallback_item_name_covers_peninsula_deeds_and_keys() {
        assert_eq!(fallback_item_name("deed_white_tower_property"), Some("公議堂紅印屋契"));
        assert_eq!(fallback_item_name("key_brass_home_lock"), Some("黃銅彈子門鎖鑰匙"));
        assert_eq!(fallback_item_name("item_personal_astrolabe_scroll"), Some("周天命格星盤拓卷"));
        assert_eq!(fallback_item_name("item_oiled_bamboo_umbrella"), Some("雙層桐油防雨竹傘"));
        assert_eq!(fallback_item_name("item_spruce_votive_plaque"), Some("北嶺雲杉祈福木牌"));
        assert_eq!(fallback_item_name("item_white_tower_gazetteer_scroll"), Some("白塔風物孤本鈔卷"));
        assert_eq!(fallback_item_name("item_ebony_mortise_tea_tray"), Some("沉水烏木榫卯茶盤"));
        assert_eq!(fallback_item_name("item_quenched_steel_whetstone"), Some("雙面精鋼油石"));
        assert_eq!(fallback_item_name("item_indigo_rain_cloak"), Some("山藍防雨斗篷"));
        assert_eq!(fallback_item_name("item_coastal_sea_chart"), Some("黑潮順時針海圖摺頁"));
        assert_eq!(fallback_item_name("item_wenyao_cadastral_rubbing"), Some("文曜四坊地籍圖鈔頁"));
        assert_eq!(fallback_item_name("item_braised_beef_broth_jar"), Some("慢燉原牛肉湯陶盅"));
        assert_eq!(fallback_item_name("item_roasted_frost_tea_tin"), Some("炭焙霜林烘青茶罐"));
        assert_eq!(fallback_item_name("item_seven_star_moxa_pack"), Some("九蒸七星艾絨藥包"));
        assert_eq!(fallback_item_name("item_collated_stele_rubbing"), Some("精校金石碑帖拓本"));
        assert_eq!(fallback_item_name("item_polished_quartz_lens"), Some("精磨紅土水晶鏡片"));
        assert_eq!(fallback_item_name("item_smoked_mullet_roe_pack"), Some("風櫃尾冷燻烏魚子禮包"));
        assert_eq!(fallback_item_name("starter_body_m"), Some("棉麻日常上衣"));
        assert_eq!(fallback_item_name("starter_legs_m"), Some("耐磨休閒長褲"));
        assert_eq!(fallback_item_name("starter_feet_m"), Some("輕便布鞋"));
        assert_eq!(fallback_item_name("粗布短褂"), Some("棉麻日常上衣"));
        assert_eq!(fallback_item_name("粗布長褲"), Some("耐磨休閒長褲"));
        assert_eq!(fallback_item_name("草鞋"), Some("輕便布鞋"));
        assert_eq!(fallback_item_name("mat_herb_poultice"), Some("百草外敷藥膏"));
        assert_eq!(fallback_item_name("mat_beast_hide"), Some("粗製獸皮"));
        assert_eq!(fallback_item_name("craft_roasted_shrimp_skewer"), Some("炭炙水尾鮮蝦串"));
        assert_eq!(fallback_item_name("craft_smoked_fish_fillet"), Some("相思木燻鮮魚乾"));
        assert_eq!(fallback_item_name("craft_spiced_beast_jerky"), Some("馬告炭炙獸肉脯"));
        assert_eq!(fallback_item_name("craft_ration_grain_cake"), Some("行旅耐儲米糧餅"));
    }
    #[test]
    fn catalog_item_def_and_weight_resolution() {
        let def = get_item_def("wt_cat_0001").expect("catalog entry wt_cat_0001 must exist");
        assert_eq!(def.name, "赤鐵重礦");
        assert_eq!(def.weight, 1.5);
        assert!(def.description.contains("赤鐵重礦"));

        let inv_json = r#"[{"item_id":"wt_cat_0001","qty":2}]"#;
        let w = inventory_weight(inv_json);
        assert!((w - 3.0).abs() < 1e-6);

        let rod_def = get_item_def("wt_cat_1006").expect("wt_cat_1006 must exist");
        assert_eq!(rod_def.name, "老青竹黃銅輪捲線釣竿");
        assert_eq!(rod_def.slot, "hold_r");
        assert_eq!(rod_def.item_type, "equipment");
        assert!(rod_def.atk_bonus > 0);

        let rod_inv = get_inventory(r#"[{"item_id":"wt_cat_1006","qty":1}]"#, 100);
        assert_eq!(rod_inv.items[0].slot, "hold_r");
        assert_eq!(rod_inv.items[0].item_type, "equipment");
    }

    #[test]
    fn test_npc_inventory_stack_cap_prevents_reed_bloat() {
        let mut entries = vec![
            InventoryEntry {
                item_id: "mat_reed_bundle".to_string(),
                qty: 133_530,
            },
            InventoryEntry {
                item_id: "manual_854b8035e7134512".to_string(),
                qty: 1,
            },
        ];
        assert!(normalize_npc_inventory_entries(&mut entries));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].item_id, "mat_reed_bundle");
        assert_eq!(entries[0].qty, MAX_NPC_ITEM_STACK_QTY);

        for i in 1..=10 {
            entries.push(InventoryEntry {
                item_id: format!("mat_item_{i}"),
                qty: 50,
            });
        }
        assert!(normalize_npc_inventory_entries(&mut entries));
        assert!(entries.len() <= MAX_NPC_DISTINCT_ITEMS);
        for it in &entries {
            assert!(it.qty <= MAX_NPC_ITEM_STACK_QTY);
        }
    }
}

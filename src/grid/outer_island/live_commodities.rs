//! 銀鷗島第二刀：糧、煤、木、鐵真實存量帳戶、鋪倉耗料停擺、城鄉貨運分流與供需連動報價引擎。

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};

use super::cell_resources::{lookup_resource_spec, specialty_sell_unit_price};
use super::household_economy::{
    HOME_RATION_SLUG, MAX_TAVERN_LARDER, count_box_home_rations_only, deduct_box_home_rations,
    find_town_tavern_and_keeper, make_home_ration,
};
use super::movement::{add_carry_item, take_clean_carry_items};
use super::rural_migrate::nearest_core_town_cell_id;
use super::types::{CarryItem, IslandWorldDelta, SpatialNodeDef, WorldStaticDoc};

pub const SMITHY_FUEL_SLUGS: [&str; 4] = ["black_coal", "charcoal_sack", "driftwood", "timber_log"];
pub const APOTHECARY_HERB_SLUGS: [&str; 2] = ["herb_bundle", "quarantine_wormwood"];

const BULK_FOOD_SLUGS: [&str; 6] = [
    "wheat_grain",
    "dairy_cheese",
    "salt_fish_barrel",
    "fresh_fish_basket",
    "wild_honey",
    "sea_kelp",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LiveCommoditiesState {
    pub shop_stocks_seeded: bool,
    pub last_port_barge_day: BTreeMap<String, i64>,
}

fn make_clean_item(slug: &str, name: &str, count: u32, weight_units: u16) -> CarryItem {
    CarryItem {
        slug: slug.to_string(),
        name: name.to_string(),
        count,
        weight_units,
        ..CarryItem::default()
    }
}

fn count_clean_slugs(items: &[CarryItem], slugs: &[&str]) -> u32 {
    items
        .iter()
        .filter(|i| {
            slugs.contains(&i.slug.as_str())
                && i.bound_owner.is_none()
                && i.stolen_from.is_none()
                && i.count > 0
        })
        .map(|i| i.count)
        .sum()
}

/// 冪等初始化全島鐵匠鋪、木匠鋪與草藥堂之開世起始原料庫存（僅派駐實體 `kind == "shop"` 鋪位執行一次，絕不憑空續補）。
pub fn ensure_shop_material_stocks_seeded(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
) -> bool {
    if delta.live_commodities.shop_stocks_seeded {
        return false;
    }
    let actor_ids: Vec<String> = delta.actors.keys().cloned().collect();
    for aid in actor_ids {
        let Some(a) = delta.actors.get_mut(&aid) else {
            continue;
        };
        if !a.alive || a.controller_id.is_some() || aid.starts_with("pl:") {
            continue;
        }
        let work_node = a.work_node.clone();
        if !doc.nodes.get(&work_node).is_some_and(|n| n.kind == "shop") {
            continue;
        }
        if a.role == "鍛鐵匠" {
            let locker = a.inn_lockers.entry(work_node).or_default();
            add_carry_item(locker, make_clean_item("black_coal", "北林精煤塊", 8, 3));
            add_carry_item(
                locker,
                make_clean_item("charcoal_sack", "耐燒硬木炭袋", 6, 2),
            );
            add_carry_item(locker, make_clean_item("driftwood", "異國刻紋漂流木", 4, 2));
            add_carry_item(locker, make_clean_item("iron_ore", "粗鐵礦石", 6, 3));
            a.mark_mutated();
        } else if a.role == "木匠" {
            let locker = a.inn_lockers.entry(work_node).or_default();
            add_carry_item(locker, make_clean_item("timber_log", "灰松厚原木", 6, 4));
            add_carry_item(locker, make_clean_item("driftwood", "異國刻紋漂流木", 4, 2));
            add_carry_item(
                locker,
                make_clean_item("charcoal_sack", "耐燒硬木炭袋", 4, 2),
            );
            a.mark_mutated();
        } else if a.role == "草藥師" {
            let locker = a.inn_lockers.entry(work_node).or_default();
            add_carry_item(locker, make_clean_item("herb_bundle", "山林止血草束", 8, 1));
            add_carry_item(
                locker,
                make_clean_item("quarantine_wormwood", "南岸清冽苦艾束", 4, 1),
            );
            a.mark_mutated();
        }
    }
    delta.live_commodities.shop_stocks_seeded = true;
    delta.extras_dirty = true;
    true
}

/// 檢查指定店鋪節點是否派駐有對應工匠／店員（用於區分無人合成測試與真實開張店鋪）。
pub fn has_npc_shop_staff_assigned(
    delta: &IslandWorldDelta,
    shop_node: &str,
    role_keywords: &[&str],
) -> bool {
    delta.actors.iter().any(|(aid, a)| {
        a.alive
            && a.controller_id.is_none()
            && !aid.starts_with("pl:")
            && a.work_node == shop_node
            && role_keywords.iter().any(|kw| a.role.contains(kw))
    })
}

/// 跨同鋪師徒聚合盤點指定鋪倉節點內特定貨品總存量。
pub fn count_shop_stock_by_slugs(delta: &IslandWorldDelta, shop_node: &str, slugs: &[&str]) -> u32 {
    delta
        .actors
        .iter()
        .filter(|(aid, a)| {
            a.alive
                && a.controller_id.is_none()
                && !aid.starts_with("pl:")
                && (a.work_node == shop_node || a.home_node == shop_node)
        })
        .filter_map(|(_, a)| a.inn_lockers.get(shop_node))
        .map(|items| count_clean_slugs(items, slugs))
        .fold(0u32, u32::saturating_add)
}

/// 按序優先從指定店員或同鋪其他員工木箱實扣 1 件指定類目貨品。
pub fn consume_shop_stock_one(
    delta: &mut IslandWorldDelta,
    preferred_actor: Option<&str>,
    shop_node: &str,
    slugs: &[&str],
) -> Option<(String, CarryItem)> {
    let mut candidate_ids: Vec<String> = Vec::new();
    if let Some(pref) = preferred_actor {
        candidate_ids.push(pref.to_string());
    }
    let mut others: Vec<String> = delta
        .actors
        .iter()
        .filter(|(aid, a)| {
            preferred_actor != Some(aid.as_str())
                && a.alive
                && a.controller_id.is_none()
                && !aid.starts_with("pl:")
                && (a.work_node == shop_node || a.home_node == shop_node)
        })
        .map(|(aid, _)| aid.clone())
        .collect();
    others.sort();
    candidate_ids.extend(others);

    for holder_id in candidate_ids {
        let Some(holder) = delta.actors.get_mut(&holder_id) else {
            continue;
        };
        let Some(locker) = holder.inn_lockers.get_mut(shop_node) else {
            continue;
        };
        for &slug in slugs {
            if let Some(item) = take_clean_carry_items(locker, slug, 1) {
                holder.mark_mutated();
                return Some((holder_id, item));
            }
        }
    }
    None
}

/// 交易回滾：將已預扣之貨品原數退還鋪倉木箱。
pub fn restore_shop_stock_one(
    delta: &mut IslandWorldDelta,
    holder_id: &str,
    shop_node: &str,
    item: CarryItem,
) {
    if let Some(holder) = delta.actors.get_mut(holder_id) {
        let locker = holder.inn_lockers.entry(shop_node.to_string()).or_default();
        add_carry_item(locker, item);
        holder.mark_mutated();
    }
}

pub fn count_smithy_fuel(delta: &IslandWorldDelta, shop_node: &str) -> u32 {
    count_shop_stock_by_slugs(delta, shop_node, &SMITHY_FUEL_SLUGS)
}

pub fn consume_smithy_fuel_one(
    delta: &mut IslandWorldDelta,
    preferred_actor: Option<&str>,
    shop_node: &str,
) -> Option<(String, CarryItem)> {
    consume_shop_stock_one(delta, preferred_actor, shop_node, &SMITHY_FUEL_SLUGS)
}

pub fn count_apothecary_herbs(delta: &IslandWorldDelta, shop_node: &str) -> u32 {
    count_shop_stock_by_slugs(delta, shop_node, &APOTHECARY_HERB_SLUGS)
}

pub fn consume_apothecary_herb_one(
    delta: &mut IslandWorldDelta,
    preferred_actor: Option<&str>,
    shop_node: &str,
) -> Option<(String, CarryItem)> {
    consume_shop_stock_one(delta, preferred_actor, shop_node, &APOTHECARY_HERB_SLUGS)
}

pub fn count_tavern_ration_stock(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
    tav_node: &str,
) -> u32 {
    let (_, keeper_opt) = find_town_tavern_and_keeper(doc, delta, cell_id);
    let Some(kid) = keeper_opt else {
        return 30;
    };
    delta
        .actors
        .get(&kid)
        .map_or(0, |k| count_box_home_rations_only(k, tav_node))
}

pub fn quote_tavern_ration_copper(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
    tav_node: &str,
) -> i64 {
    if !delta.live_commodities.shop_stocks_seeded {
        return 2;
    }
    let stock = count_tavern_ration_stock(doc, delta, cell_id, tav_node);
    if stock >= 15 {
        2
    } else if stock >= 6 {
        3
    } else if stock >= 2 {
        4
    } else {
        5
    }
}

pub fn is_smithy_forge_cold(delta: &IslandWorldDelta, shop_node: &str) -> bool {
    delta.live_commodities.shop_stocks_seeded
        && has_npc_shop_staff_assigned(delta, shop_node, &["鍛鐵", "鐵匠", "木匠"])
        && count_smithy_fuel(delta, shop_node) == 0
}

pub fn quote_weapon_repair_copper(delta: &IslandWorldDelta, shop_node: &str) -> i64 {
    if !delta.live_commodities.shop_stocks_seeded
        || !has_npc_shop_staff_assigned(delta, shop_node, &["鍛鐵", "鐵匠", "木匠"])
    {
        return 4;
    }
    let fuel = count_smithy_fuel(delta, shop_node);
    if fuel >= 6 {
        4
    } else if fuel >= 3 {
        5
    } else {
        6
    }
}

pub fn quote_engrave_rune_copper(delta: &IslandWorldDelta, shop_node: &str) -> i64 {
    if !delta.live_commodities.shop_stocks_seeded
        || !has_npc_shop_staff_assigned(delta, shop_node, &["鍛鐵", "鐵匠", "木匠"])
    {
        return 6;
    }
    let fuel = count_smithy_fuel(delta, shop_node);
    if fuel >= 6 {
        6
    } else if fuel >= 3 {
        7
    } else {
        8
    }
}

pub fn quote_custom_forge_copper(delta: &IslandWorldDelta, shop_node: &str) -> i64 {
    quote_engrave_rune_copper(delta, shop_node)
}

pub fn is_apothecary_out_of_herbs(delta: &IslandWorldDelta, shop_node: &str) -> bool {
    delta.live_commodities.shop_stocks_seeded
        && has_npc_shop_staff_assigned(delta, shop_node, &["草藥"])
        && count_apothecary_herbs(delta, shop_node) == 0
}

pub fn quote_bandage_copper(delta: &IslandWorldDelta, shop_node: &str) -> i64 {
    if !delta.live_commodities.shop_stocks_seeded
        || !has_npc_shop_staff_assigned(delta, shop_node, &["草藥"])
    {
        return 3;
    }
    let herbs = count_apothecary_herbs(delta, shop_node);
    if herbs >= 4 {
        3
    } else if herbs >= 2 {
        4
    } else {
        5
    }
}

fn find_town_shop_node_and_staff(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    town_cid: u32,
    role_exact: &str,
) -> Option<(String, String)> {
    let mut best: Option<(bool, String, String)> = None;
    for (aid, a) in &delta.actors {
        if !a.alive || a.controller_id.is_some() || aid.starts_with("pl:") || a.role != role_exact {
            continue;
        }
        let Some(wnd) = doc.nodes.get(&a.work_node) else {
            continue;
        };
        if nearest_core_town_cell_id(doc, wnd.cell_id) != town_cid {
            continue;
        }
        let is_shop = wnd.kind == "shop";
        let replace = match &best {
            None => true,
            Some((prev_shop, prev_id, _)) => {
                (is_shop && !*prev_shop)
                    || (is_shop == *prev_shop && aid.as_str() < prev_id.as_str())
            }
        };
        if replace {
            best = Some((is_shop, aid.clone(), a.work_node.clone()));
        }
    }
    best.map(|(_, id, wnode)| (id, wnode))
}

fn count_town_stock_for_role(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    town_cid: u32,
    role_exact: &str,
    slugs: &[&str],
) -> Option<u32> {
    let (_staff_id, work_node) = find_town_shop_node_and_staff(doc, delta, town_cid, role_exact)?;
    Some(count_shop_stock_by_slugs(delta, &work_node, slugs))
}

/// 統一特產與材料收購報價（供介面出價籤條與當面過秤結算 100% 共用，確保報價與實收一致）。
pub fn quote_specialty_buyback_copper(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
    _buyer_id: Option<&str>,
    item: &CarryItem,
) -> Option<i64> {
    let base = specialty_sell_unit_price(item)?;
    if !delta.live_commodities.shop_stocks_seeded
        || item.stolen_from.is_some()
        || item.vessel_meta.is_some()
    {
        return Some(base);
    }
    let town_cid = nearest_core_town_cell_id(doc, node.cell_id);
    let adj = match item.slug.as_str() {
        "black_coal" | "charcoal_sack" => {
            match count_town_stock_for_role(doc, delta, town_cid, "鍛鐵匠", &SMITHY_FUEL_SLUGS) {
                Some(0) => base + 2,
                Some(1..=4) => base + 1,
                Some(n) if n >= 28 => (base - 1).max(1),
                _ => base,
            }
        }
        "driftwood" | "timber_log" => {
            let smith_w = count_town_stock_for_role(
                doc,
                delta,
                town_cid,
                "鍛鐵匠",
                &["driftwood", "timber_log"],
            )
            .unwrap_or(0);
            let carp_w = count_town_stock_for_role(
                doc,
                delta,
                town_cid,
                "木匠",
                &["driftwood", "timber_log", "charcoal_sack"],
            )
            .unwrap_or(0);
            let total_w = smith_w.saturating_add(carp_w);
            if total_w == 0 {
                base + 2
            } else if total_w <= 3 {
                base + 1
            } else if total_w >= 28 {
                (base - 1).max(1)
            } else {
                base
            }
        }
        "iron_ore" => {
            match count_town_stock_for_role(doc, delta, town_cid, "鍛鐵匠", &["iron_ore"]) {
                Some(0) => base + 2,
                Some(1..=2) => base + 1,
                Some(n) if n >= 22 => (base - 1).max(1),
                _ => base,
            }
        }
        "herb_bundle" | "quarantine_wormwood" => {
            match count_town_stock_for_role(doc, delta, town_cid, "草藥師", &APOTHECARY_HERB_SLUGS)
            {
                Some(0) => base + 2,
                Some(1..=3) => base + 1,
                Some(n) if n >= 24 => (base - 1).max(1),
                _ => base,
            }
        }
        s if BULK_FOOD_SLUGS.contains(&s) => {
            let (tav_opt, keeper_opt) = find_town_tavern_and_keeper(doc, delta, town_cid);
            if let (Some(tn), Some(_)) = (tav_opt, keeper_opt) {
                let rt = count_tavern_ration_stock(doc, delta, town_cid, &tn);
                if rt < 8 {
                    base + 2
                } else if rt <= 14 {
                    base + 1
                } else if rt >= 90 {
                    (base - 1).max(1)
                } else {
                    base
                }
            } else {
                base
            }
        }
        _ => base,
    };
    Some(adj)
}

pub fn is_bulk_food_slug(slug: &str) -> bool {
    BULK_FOOD_SLUGS.contains(&slug)
}

/// 當買方於市集廣場／碼頭收到急缺原物料時，優先代轉入該鎮對應斷料鋪倉（否則入買方鋪箱）。
pub fn store_purchased_specialty_in_town(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    buyer_id: &str,
    node_cell_id: u32,
    mut item: CarryItem,
) {
    item.stolen_from = None;
    let town_cid = nearest_core_town_cell_id(doc, node_cell_id);
    if is_bulk_food_slug(&item.slug) {
        let buyer_is_tavern = delta
            .actors
            .get(buyer_id)
            .is_some_and(|b| b.role.contains("酒館"));
        let (tav_opt, keeper_opt) = find_town_tavern_and_keeper(doc, delta, town_cid);
        if let (Some(tn), Some(kid)) = (tav_opt, keeper_opt)
            && (buyer_is_tavern || count_tavern_ration_stock(doc, delta, town_cid, &tn) < 12)
            && let Some(ik) = delta.actors.get_mut(&kid)
        {
            let cur = count_box_home_rations_only(ik, &tn);
            let add_n = item
                .count
                .saturating_mul(3)
                .min(MAX_TAVERN_LARDER.saturating_sub(cur));
            if add_n > 0 {
                let tv = ik.inn_lockers.entry(tn).or_default();
                add_carry_item(tv, make_home_ration(add_n));
                ik.mark_mutated();
                return;
            }
        }
    } else if matches!(
        item.slug.as_str(),
        "black_coal" | "charcoal_sack" | "iron_ore"
    ) {
        let buyer_is_smith = delta
            .actors
            .get(buyer_id)
            .is_some_and(|b| b.role.contains("鍛鐵"));
        if !buyer_is_smith
            && let Some((sid, swork)) =
                find_town_shop_node_and_staff(doc, delta, town_cid, "鍛鐵匠")
        {
            let need_route = if item.slug == "iron_ore" {
                count_shop_stock_by_slugs(delta, &swork, &["iron_ore"]) == 0
            } else {
                count_smithy_fuel(delta, &swork) == 0
            };
            if need_route && let Some(smith) = delta.actors.get_mut(&sid) {
                let sv = smith.inn_lockers.entry(swork).or_default();
                add_carry_item(sv, item);
                smith.mark_mutated();
                return;
            }
        }
    } else if APOTHECARY_HERB_SLUGS.contains(&item.slug.as_str()) {
        let buyer_is_herb = delta
            .actors
            .get(buyer_id)
            .is_some_and(|b| b.role.contains("草藥"));
        if !buyer_is_herb
            && let Some((hid, hwork)) =
                find_town_shop_node_and_staff(doc, delta, town_cid, "草藥師")
            && count_apothecary_herbs(delta, &hwork) == 0
            && let Some(herb) = delta.actors.get_mut(&hid)
        {
            let hv = herb.inn_lockers.entry(hwork).or_default();
            add_carry_item(hv, item);
            herb.mark_mutated();
            return;
        }
    }

    if let Some(ba) = delta.actors.get_mut(buyer_id) {
        if item.vessel_meta.is_some() && ba.backpack.len() < 6 {
            add_carry_item(&mut ba.backpack, item);
        } else {
            let wbox = ba.work_node.clone();
            add_carry_item(ba.inn_lockers.entry(wbox).or_default(), item);
        }
        ba.mark_mutated();
    }
}

fn add_to_actor_locker_capped(
    delta: &mut IslandWorldDelta,
    aid: &str,
    locker_node: &str,
    slug: &str,
    name_zh: &str,
    cnt: u32,
    weight_units: u16,
    cap: u32,
) {
    let Some(a) = delta.actors.get_mut(aid) else {
        return;
    };
    let cur = a
        .inn_lockers
        .get(locker_node)
        .map_or(0, |v| count_clean_slugs(v, &[slug]));
    let actual = cnt.min(cap.saturating_sub(cur));
    if actual > 0 {
        let box_vec = a.inn_lockers.entry(locker_node.to_string()).or_default();
        add_carry_item(
            box_vec,
            make_clean_item(slug, name_zh, actual, weight_units),
        );
        a.mark_mutated();
    }
}

/// 城鎮到站貨箱按商品屬性精準分流入鋪：糧魚鮮入酒館轉口糧、煤鐵入鍛鐵鋪、木料分流、草藥入草藥堂。
pub fn route_unloaded_freight_to_town_shops(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    town_cell_id: u32,
    unloaded: Vec<(String, String, u32)>,
) {
    let town_cid = nearest_core_town_cell_id(doc, town_cell_id);
    let (tav_opt, keeper_opt) = find_town_tavern_and_keeper(doc, delta, town_cid);
    let smith_opt = find_town_shop_node_and_staff(doc, delta, town_cid, "鍛鐵匠");
    let carp_opt = find_town_shop_node_and_staff(doc, delta, town_cid, "木匠");
    let herb_opt = find_town_shop_node_and_staff(doc, delta, town_cid, "草藥師");

    for (slug, name_zh, cnt) in unloaded {
        if cnt == 0 {
            continue;
        }
        let w = lookup_resource_spec(&slug).map_or(3, |r| r.weight_units);
        if is_bulk_food_slug(&slug) || slug == HOME_RATION_SLUG {
            if let (Some(tn), Some(kid)) = (tav_opt.as_ref(), keeper_opt.as_ref())
                && let Some(ik) = delta.actors.get_mut(kid)
            {
                let cur = count_box_home_rations_only(ik, tn);
                let mult = if slug == HOME_RATION_SLUG { 1 } else { 3 };
                let add_n = cnt
                    .saturating_mul(mult)
                    .min(MAX_TAVERN_LARDER.saturating_sub(cur));
                if add_n > 0 {
                    let tv = ik.inn_lockers.entry(tn.clone()).or_default();
                    add_carry_item(tv, make_home_ration(add_n));
                    ik.mark_mutated();
                }
            }
        } else if matches!(
            slug.as_str(),
            "black_coal" | "charcoal_sack" | "iron_ore" | "quartz_sand"
        ) {
            if let Some((sid, swork)) = smith_opt.as_ref() {
                add_to_actor_locker_capped(delta, sid, swork, &slug, &name_zh, cnt, w, 48);
            }
        } else if matches!(slug.as_str(), "timber_log" | "driftwood" | "pine_resin") {
            let need_smith_fuel = smith_opt
                .as_ref()
                .is_some_and(|(_, sw)| count_smithy_fuel(delta, sw) < 6);
            if need_smith_fuel && matches!(slug.as_str(), "timber_log" | "driftwood") {
                if let Some((sid, swork)) = smith_opt.as_ref() {
                    add_to_actor_locker_capped(delta, sid, swork, &slug, &name_zh, cnt, w, 48);
                }
            } else if let Some((cid, cwork)) = carp_opt.as_ref().or(smith_opt.as_ref()) {
                add_to_actor_locker_capped(delta, cid, cwork, &slug, &name_zh, cnt, w, 48);
            }
        } else if matches!(
            slug.as_str(),
            "herb_bundle" | "quarantine_wormwood" | "beast_tallow"
        ) {
            if let Some((hid, hwork)) = herb_opt.as_ref() {
                add_to_actor_locker_capped(delta, hid, hwork, &slug, &name_zh, cnt, w, 48);
            } else if let (Some(tn), Some(kid)) = (tav_opt.as_ref(), keeper_opt.as_ref()) {
                add_to_actor_locker_capped(delta, kid, tn, &slug, &name_zh, cnt, w, 48);
            }
        } else if let Some((sid, swork)) = smith_opt.as_ref() {
            add_to_actor_locker_capped(delta, sid, swork, &slug, &name_zh, cnt, w, 48);
        }
    }
}

/// 城鎮酒館若存糧充裕（>= 24），從酒館實扣 2 份餘糧裝上回程下鄉篷車，絕不憑空印糧。
pub fn try_load_town_surplus_rations_for_rural(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    town_cell_id: u32,
) -> u32 {
    let town_cid = nearest_core_town_cell_id(doc, town_cell_id);
    let (Some(tn), Some(kid)) = find_town_tavern_and_keeper(doc, delta, town_cid) else {
        return 0;
    };
    let Some(ik) = delta.actors.get_mut(&kid) else {
        return 0;
    };
    if count_box_home_rations_only(ik, &tn) < 24 {
        return 0;
    }
    if deduct_box_home_rations(ik, &tn, 2) {
        ik.mark_mutated();
        2
    } else {
        0
    }
}

/// 依水患、疫氣、墾田規模與地塊實存計算農漁獵戶當日真實產糧份數，並耗損非城鎮地塊實存。
pub fn harvest_daily_producer_rations(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    raw_cid: u32,
) -> u32 {
    let is_rural = doc
        .cells
        .get(&raw_cid.to_string())
        .is_some_and(|c| c.kind != "settlement");
    let ll = delta.live_land.cells.get(&raw_cid);
    if ll.is_some_and(|c| c.plague_miasmas >= 40) {
        return 2;
    }
    if let Some(c) = ll {
        if c.flood_level >= 2 {
            return 2;
        }
        if c.flood_level == 1 {
            return 4;
        }
    }
    let reclaimed = ll.map_or(0, |c| c.reclaimed_plots);
    if is_rural {
        let cur_stk = delta.cell_stocks.get(&raw_cid).copied().unwrap_or(0);
        if cur_stk == 0 {
            return 2;
        }
        if cur_stk >= 3 {
            delta.cell_stocks.insert(raw_cid, cur_stk - 1);
            delta.extras_dirty = true;
        }
    }
    6 + u32::from(reclaimed >= 1) + u32::from(reclaimed >= 3)
}

/// BFS 核對鄉野產糧點是否可循未斷橋且未封鎖之陸路將餘糧運抵主鎮酒館（斷橋即中斷跨河供糧）。
pub fn can_deliver_surplus_to_town(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    from_cid: u32,
    town_cid: u32,
) -> bool {
    if from_cid == town_cid {
        return true;
    }
    if super::live_land::is_quarantine_restricted(doc, delta, from_cid)
        || super::live_land::is_river_crossing_impassable_for_wheels(delta, from_cid)
    {
        return false;
    }
    let mut visited = BTreeSet::new();
    let mut queue = VecDeque::new();
    visited.insert(from_cid);
    queue.push_back(from_cid);
    while let Some(cid) = queue.pop_front() {
        if cid == town_cid {
            return true;
        }
        let Some(cell) = doc.cells.get(&cid.to_string()) else {
            continue;
        };
        for &nb in &cell.neighbors_f6 {
            if !visited.insert(nb) {
                continue;
            }
            if nb != town_cid
                && (super::live_land::is_quarantine_restricted(doc, delta, nb)
                    || super::live_land::is_river_crossing_impassable_for_wheels(delta, nb))
            {
                continue;
            }
            queue.push_back(nb);
        }
    }
    false
}

/// 產生純世界內語氣的鋪面供需實況後綴，供 `scene.desc` 即時呈現熄火、缺藥或漲價線索。
pub fn live_commodities_scene_prose_suffix(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    node: &SpatialNodeDef,
) -> String {
    if !delta.live_commodities.shop_stocks_seeded {
        return String::new();
    }
    if node.kind == "shop" && (node.name.contains("鍛鐵") || node.name.contains("鐵匠")) {
        if is_smithy_forge_cold(delta, &node.id) {
            return " 紅磚鍛爐內火炭已熄，只剩冷灰餘溫；爐旁鐵牌寫著正急收精煤、木炭與乾木重開爐火。"
                .to_string();
        }
        if count_smithy_fuel(delta, &node.id) <= 5 {
            return " 炭筐裡的精煤所剩不多，爐匠正省著鼓風添炭，修刃鍛工價略有上浮。".to_string();
        }
    } else if node.kind == "shop" && node.name.contains("酒館") {
        let stock = count_tavern_ration_stock(doc, delta, node.cell_id, &node.id);
        if stock == 0 {
            return " 櫃台後的黑麥糧甕與竹簍早已見底，掌櫃只得暫停賣糧，翹首盼著鄉間糧車入鎮。"
                .to_string();
        }
        if stock < 15 {
            return " 櫃頭黑麥乾糧存量吃緊，掌櫃把銅價牌朝外翻轉調高了些許。".to_string();
        }
    } else if node.kind == "shop" && node.name.contains("草藥") {
        if is_apothecary_out_of_herbs(delta, &node.id) {
            return " 藥櫃上幾隻素陶罐蓋得空空如也，草藥師正高價收購山林止血草與苦艾重開臼火。"
                .to_string();
        }
        if count_apothecary_herbs(delta, &node.id) <= 3 {
            return " 藥臼旁的苦艾草束日漸稀疏，敷藥包紮價碼微微看漲。".to_string();
        }
    }
    String::new()
}

/// 檢查全島最迫切的缺料城鎮實況，供茶棚熱茶與酒館旁聽播報真實活情報。
pub fn find_urgent_commodity_rumor_zh(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
) -> Option<String> {
    if !delta.live_commodities.shop_stocks_seeded {
        return None;
    }
    for (aid, a) in &delta.actors {
        if !a.alive || a.controller_id.is_some() || aid.starts_with("pl:") {
            continue;
        }
        if a.role == "鍛鐵匠" && is_smithy_forge_cold(delta, &a.work_node) {
            let sname = doc
                .nodes
                .get(&a.work_node)
                .map_or("鎮上鐵匠鋪", |n| n.name.as_str());
            return Some(format!(
                "聽聞【{}】精煤木炭燒盡熄了爐，師傅正加價 {} 枚銅幣急收北林精煤與木炭呢。",
                sname, 6
            ));
        }
    }
    for c in doc.cells.values() {
        if c.kind == "settlement"
            && let (Some(tn), Some(_)) = find_town_tavern_and_keeper(doc, delta, c.cell_id)
        {
            let st = count_tavern_ration_stock(doc, delta, c.cell_id, &tn);
            if st < 10 {
                let tname = doc.nodes.get(&tn).map_or("城鎮酒館", |n| n.name.as_str());
                let q = quote_tavern_ration_copper(doc, delta, c.cell_id, &tn);
                return Some(format!(
                    "最近官道貨少，【{}】黑麥乾糧存貨吃緊，每包已漲到 {} 枚銅幣，手頭有穀袋魚鮮趁早送過去好賣價。",
                    tname, q
                ));
            }
        }
    }
    None
}

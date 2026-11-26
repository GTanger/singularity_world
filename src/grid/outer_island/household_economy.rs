//! 家戶共享存糧與鎮內三向複式銅幣守恆微循環（零憑空印鈔、嚴格口糧帳本守恆、$O(N)$ 家戶索引）。

use std::collections::{BTreeMap, BTreeSet};

use super::appraisal::ConcernCause;
use super::clock_and_lang::SolarClockSnapshot;
use super::movement::add_carry_item;
use super::persona::derive_innate_persona;
use super::types::{CarryItem, ContractRecord, IslandWorldDelta, UnifiedMindBody, WorldStaticDoc};

pub const HOME_RATION_SLUG: &str = "home_ration";
pub const HOME_RATION_NAME: &str = "家常魚麥口糧";
const LARDER_SEED_MARKER: &str = "larder_seeded_v1";
const MAX_HOUSEHOLD_LARDER: u32 = 18;
pub(super) const MAX_TAVERN_LARDER: u32 = 220;

pub fn effective_home_node(a: &UnifiedMindBody) -> &str {
    a.owned_house_node
        .as_deref()
        .unwrap_or(a.home_node.as_str())
}

pub fn household_bucket_key(a: &UnifiedMindBody) -> String {
    if a.household_id.is_empty() {
        format!("node:{}", effective_home_node(a))
    } else {
        a.household_id.clone()
    }
}

/// C1 修正：精確鎖定一樓營業酒館節點（`kind == "shop"`，排除二樓 `inn_upstairs`）與正職 `酒館掌櫃`（排除 `車行掌櫃`）
pub fn find_town_tavern_and_keeper(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    cell_id: u32,
) -> (Option<String>, Option<String>) {
    let res = {
        let tav_node = doc
            .nodes
            .values()
            .find(|n| {
                n.cell_id == cell_id
                    && !n.dungeon
                    && n.kind == "shop"
                    && (n.name.contains("酒館") || n.name.contains("客棧"))
            })
            .or_else(|| {
                doc.nodes
                    .values()
                    .find(|n| n.cell_id == cell_id && !n.dungeon && n.kind == "plaza")
            })
            .map(|n| n.id.clone());

        let keeper_id = if let Some(tn) = tav_node.as_deref()
            && let Some(lease) = delta.shop_leases.get(tn)
        {
            if lease.state == super::shop_leases::ShopLeaseState::SealedForLease {
                None
            } else {
                lease.operator_actor_id.as_ref().and_then(|op_id| {
                    delta
                        .actors
                        .get(op_id)
                        .is_some_and(|a| a.alive && !op_id.starts_with("pl:"))
                        .then(|| op_id.clone())
                })
            }
        } else {
            let is_tavern_keeper = |a: &UnifiedMindBody| -> bool {
                a.alive && a.controller_id.is_none() && a.role == "酒館掌櫃"
            };
            tav_node
                .as_ref()
                .and_then(|tn| delta.node_occupants.get(tn))
                .and_then(|occ| {
                    occ.iter()
                        .find(|&aid| delta.actors.get(aid).is_some_and(is_tavern_keeper))
                        .cloned()
                })
                .or_else(|| {
                    delta
                        .actors
                        .values()
                        .find(|a| {
                            is_tavern_keeper(a)
                                && doc
                                    .nodes
                                    .get(&a.work_node)
                                    .is_some_and(|n| n.cell_id == cell_id)
                        })
                        .map(|a| a.actor_id.clone())
                })
        };

        (tav_node, keeper_id)
    };
    if res.1.is_some() {
        return res;
    }
    let fallback_cid = super::rural_migrate::nearest_core_town_cell_id(doc, cell_id);
    if fallback_cid != cell_id {
        return find_town_tavern_and_keeper(doc, delta, fallback_cid);
    }
    res
}

pub fn is_food_producer_role(role: &str) -> bool {
    [
        "漁", "農", "獵", "牧", "採", "曬魚", "鹽漬", "燻肉", "刈麥", "補網", "搖櫓",
    ]
    .iter()
    .any(|k| role.contains(k))
}

pub fn is_craft_or_shop_master_role(role: &str) -> bool {
    matches!(
        role,
        "酒館掌櫃" | "車行掌櫃" | "鍛鐵匠" | "草藥師" | "皮匠" | "木匠" | "銀號帳房"
    )
}

pub fn is_public_works_role(role: &str) -> bool {
    matches!(
        role,
        "石工"
            | "巡城衛兵"
            | "碼頭苦力"
            | "觀星領航員"
            | "扛包理艙手"
            | "跑街送貨手"
            | "理貨記籌手"
            | "港務記籌手"
    )
}

pub fn is_helper_or_apprentice_role(role: &str) -> bool {
    !is_food_producer_role(role)
        && !is_craft_or_shop_master_role(role)
        && !is_public_works_role(role)
        && role != "家眷"
}

/// 僅統計指定木箱內之 `home_ration`（供酒館／產糧戶交易驗貨與扣減，H2 修正：絕不混算隨身 `rye_bread`）
pub fn count_box_home_rations_only(a: &UnifiedMindBody, box_key: &str) -> u32 {
    a.inn_lockers.get(box_key).map_or(0, |items| {
        items
            .iter()
            .filter(|i| {
                i.slug == HOME_RATION_SLUG && i.bound_owner.is_none() && i.stolen_from.is_none()
            })
            .map(|i| i.count)
            .sum()
    })
}

pub(super) fn deduct_box_home_rations(a: &mut UnifiedMindBody, box_key: &str, needed: u32) -> bool {
    if count_box_home_rations_only(a, box_key) < needed {
        return false;
    }
    let Some(v) = a.inn_lockers.get_mut(box_key) else {
        return false;
    };
    let mut rem = needed;
    let mut idx = 0usize;
    while idx < v.len() && rem > 0 {
        if v[idx].slug == HOME_RATION_SLUG
            && v[idx].bound_owner.is_none()
            && v[idx].stolen_from.is_none()
        {
            let take_n = v[idx].count.min(rem);
            v[idx].count -= take_n;
            rem -= take_n;
            if v[idx].count == 0 {
                v.remove(idx);
                continue;
            }
        }
        idx += 1;
    }
    rem == 0
}

fn deduct_clean_slug_one(items: &mut Vec<CarryItem>, slug: &str) -> bool {
    for idx in 0..items.len() {
        if items[idx].slug == slug
            && items[idx].bound_owner.is_none()
            && items[idx].stolen_from.is_none()
            && items[idx].count > 0
        {
            items[idx].count -= 1;
            if items[idx].count == 0 {
                items.remove(idx);
            }
            return true;
        }
    }
    false
}

/// R2-L3 修正：存糧計數與實際開伙扣減嚴格共用 `bound_owner.is_none() && stolen_from.is_none()` 過濾條件！
fn count_actor_edible_home_rations(a: &UnifiedMindBody) -> u32 {
    let hk = effective_home_node(a);
    let in_box: u32 = a.inn_lockers.get(hk).map_or(0, |items| {
        items
            .iter()
            .filter(|i| {
                (i.slug == HOME_RATION_SLUG || i.slug == "rye_bread")
                    && i.bound_owner.is_none()
                    && i.stolen_from.is_none()
            })
            .map(|i| i.count)
            .sum()
    });
    let in_bag: u32 = if a.controller_id.is_some() {
        0
    } else {
        a.backpack
            .iter()
            .filter(|i| {
                i.slug == HOME_RATION_SLUG && i.bound_owner.is_none() && i.stolen_from.is_none()
            })
            .map(|i| i.count)
            .sum()
    };
    in_box.saturating_add(in_bag)
}

pub(super) fn make_home_ration(count: u32) -> CarryItem {
    CarryItem {
        slug: HOME_RATION_SLUG.to_string(),
        name: HOME_RATION_NAME.to_string(),
        count,
        weight_units: 1,
        ..CarryItem::default()
    }
}

/// L4 修正：每響鐘一次性 $O(N)$ 建立全島家戶存糧總表，消滅角色迴圈內之 $O(N^2)$ 掃描
pub fn build_household_larder_index(delta: &IslandWorldDelta) -> BTreeMap<String, u32> {
    let mut out: BTreeMap<String, u32> = BTreeMap::new();
    for a in delta.actors.values() {
        if !a.alive || a.controller_id.is_some() || a.actor_id.starts_with("pl:") {
            continue;
        }
        let key = household_bucket_key(a);
        let cnt = count_actor_edible_home_rations(a);
        if cnt > 0 {
            let e = out.entry(key).or_insert(0);
            *e = e.saturating_add(cnt);
        }
    }
    out
}

/// L2 修正：先掃描全島已帶標記之家戶集合，確保日後動態移入成員不會對舊家戶重複灌糧，且統一寫入 `effective_home_node`
pub fn ensure_household_larders_seeded_once(delta: &mut IslandWorldDelta) {
    let needs_any = delta.actors.values().any(|a| {
        a.alive
            && a.controller_id.is_none()
            && !a.actor_id.starts_with("pl:")
            && !a.explored_windows.contains(LARDER_SEED_MARKER)
    });
    if !needs_any {
        return;
    }

    let mut hh_seeded: BTreeSet<String> = delta
        .actors
        .values()
        .filter(|a| a.explored_windows.contains(LARDER_SEED_MARKER))
        .map(household_bucket_key)
        .collect();

    let ids: Vec<String> = delta.actors.keys().cloned().collect();
    for aid in ids {
        let Some(a) = delta.actors.get_mut(&aid) else {
            continue;
        };
        if a.controller_id.is_some()
            || a.actor_id.starts_with("pl:")
            || !a.alive
            || a.explored_windows.contains(LARDER_SEED_MARKER)
        {
            continue;
        }
        a.explored_windows.insert(LARDER_SEED_MARKER.to_string());
        if a.mind.stm.goal_strategy.is_none() && a.mind.stm.active_goal_tag == "work_diligently" {
            a.mind.stm.active_goal_tag.clear();
        }
        if a.mind.stm.affect.hunger >= 75 {
            a.mind.stm.affect.hunger = (16 + (a.soul_seed % 26)) as u8;
        }
        let hh_key = household_bucket_key(a);
        let home_key = effective_home_node(a).to_string();
        if hh_seeded.insert(hh_key) {
            let box_vec = a.inn_lockers.entry(home_key).or_default();
            add_carry_item(box_vec, make_home_ration(6));
        }
        if a.role == "酒館掌櫃" {
            let work_key = a.work_node.clone();
            let box_vec = a.inn_lockers.entry(work_key).or_default();
            add_carry_item(box_vec, make_home_ration(30));
        }
        a.mark_mutated();
    }
}

pub fn count_household_larder_rations(delta: &IslandWorldDelta, actor: &UnifiedMindBody) -> u32 {
    if actor.controller_id.is_some() || actor.actor_id.starts_with("pl:") {
        return count_actor_edible_home_rations(actor);
    }
    let my_bucket = household_bucket_key(actor);
    delta
        .actors
        .values()
        .filter(|o| {
            o.alive
                && o.controller_id.is_none()
                && !o.actor_id.starts_with("pl:")
                && household_bucket_key(o) == my_bucket
        })
        .map(count_actor_edible_home_rations)
        .fold(0u32, u32::saturating_add)
}

/// L5 修正：酒館售出熱食時同步扣減酒館木箱 1 份口糧（無存糧不憑空變出熱食）
pub fn try_consume_tavern_stock_one(
    delta: &mut IslandWorldDelta,
    keeper_id: &str,
    tav_node: &str,
) -> bool {
    let Some(ik) = delta.actors.get_mut(keeper_id) else {
        return false;
    };
    if deduct_box_home_rations(ik, tav_node, 1) {
        ik.mark_mutated();
        true
    } else {
        false
    }
}

/// R2-L1 修正：若扣減酒館口糧後轉帳失敗，立即原數退回酒館木箱確保口糧帳本零誤差
pub fn restore_tavern_stock_one(delta: &mut IslandWorldDelta, keeper_id: &str, tav_node: &str) {
    if let Some(ik) = delta.actors.get_mut(keeper_id) {
        let tv = ik.inn_lockers.entry(tav_node.to_string()).or_default();
        add_carry_item(tv, make_home_ration(1));
        ik.mark_mutated();
    }
}

fn deduct_one_edible_from_actor(a: &mut UnifiedMindBody, home_key: &str) -> bool {
    if deduct_box_home_rations(a, home_key, 1) {
        return true;
    }
    if let Some(v) = a.inn_lockers.get_mut(home_key)
        && deduct_clean_slug_one(v, "rye_bread")
    {
        return true;
    }
    deduct_clean_slug_one(&mut a.backpack, HOME_RATION_SLUG)
}

/// H1 + R2-L5 修正：離線玩家絕不自動扣糧；家戶開伙優先透過 `ltm.social` 同屋親眷 $O(1)$ 查表取糧！
pub fn try_consume_household_meal(
    delta: &mut IslandWorldDelta,
    actor_id: &str,
    abs_bell: i64,
) -> bool {
    let (home_key, hh_bucket, is_player, cohabitant_ids) = {
        let Some(a) = delta.actors.get(actor_id) else {
            return false;
        };
        let cohabs: Vec<String> = a
            .mind
            .ltm
            .social
            .values()
            .filter(|s| s.cohabiting && !s.target_id.starts_with("pl:"))
            .map(|s| s.target_id.clone())
            .collect();
        (
            effective_home_node(a).to_string(),
            household_bucket_key(a),
            a.controller_id.is_some() || a.actor_id.starts_with("pl:"),
            cohabs,
        )
    };
    if is_player {
        return false;
    }

    let mut consumed = delta
        .actors
        .get_mut(actor_id)
        .is_some_and(|a| deduct_one_edible_from_actor(a, &home_key));

    if !consumed {
        for cid in cohabitant_ids {
            let p_home_opt = delta.actors.get(&cid).and_then(|o| {
                (o.alive
                    && o.controller_id.is_none()
                    && !o.actor_id.starts_with("pl:")
                    && household_bucket_key(o) == hh_bucket
                    && count_actor_edible_home_rations(o) > 0)
                    .then(|| effective_home_node(o).to_string())
            });
            if let Some(p_home) = p_home_opt
                && let Some(prov) = delta.actors.get_mut(&cid)
                && deduct_one_edible_from_actor(prov, &p_home)
            {
                prov.mark_mutated();
                consumed = true;
                break;
            }
        }
    }

    if !consumed {
        let provider_opt = delta
            .actors
            .iter()
            .find(|(oid, o)| {
                *oid != actor_id
                    && o.alive
                    && o.controller_id.is_none()
                    && !o.actor_id.starts_with("pl:")
                    && household_bucket_key(o) == hh_bucket
                    && count_actor_edible_home_rations(o) > 0
            })
            .map(|(oid, o)| (oid.clone(), effective_home_node(o).to_string()));

        if let Some((pid, p_home)) = provider_opt
            && let Some(prov) = delta.actors.get_mut(&pid)
            && deduct_one_edible_from_actor(prov, &p_home)
        {
            prov.mark_mutated();
            consumed = true;
        }
    }

    if consumed {
        if let Some(a) = delta.actors.get_mut(actor_id) {
            a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(46);
            a.mind
                .resolve_concerns_matching(ConcernCause::LivelihoodShortfall, None, 28);
            a.mark_mutated();
        }
        return true;
    }

    if let Some(a) = delta.actors.get_mut(actor_id)
        && a.controller_id.is_none()
        && a.mind.stm.affect.hunger >= 62
    {
        let innate = derive_innate_persona(a.soul_seed.max(1));
        if a.mind.appraise_event(
            &innate,
            ConcernCause::LivelihoodShortfall,
            None,
            None,
            48,
            abs_bell,
        ) {
            a.mark_mutated();
        }
    }
    false
}

/// H2 修正：驗貨與扣貨嚴格統一於 `count_box_home_rations_only` / `deduct_box_home_rations`，先成功扣貨再轉帳入倉，零口糧增生！
pub fn try_restock_household_larder(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    buyer_id: &str,
    tav_node_opt: Option<&str>,
    keeper_opt: Option<&str>,
) -> bool {
    let (home_key, dname, cell_id, cur_copper) = {
        let Some(a) = delta.actors.get(buyer_id) else {
            return false;
        };
        if a.controller_id.is_some() || a.actor_id.starts_with("pl:") {
            return false;
        }
        let hk = effective_home_node(a).to_string();
        let cid = doc.nodes.get(&hk).map_or(4616, |n| n.cell_id);
        (hk, a.display_name.clone(), cid, a.copper)
    };
    let can_open_credit = cur_copper < 2
        && count_household_larder_rations(
            delta,
            match delta.actors.get(buyer_id) {
                Some(b) => b,
                None => return false,
            },
        ) == 0
        && !delta.contracts.values().any(|c| {
            c.debtor_id == buyer_id
                && (c.status == "open" || c.status == "defaulted" || c.status == "written_off")
        });
    let tav_price = tav_node_opt.map_or(2, |tn| {
        super::live_commodities::quote_tavern_ration_copper(doc, delta, cell_id, tn)
    });
    let seller_pair: Option<(String, String, i64)> = tav_node_opt
        .zip(keeper_opt)
        .filter(|(tn, kid)| {
            *kid != buyer_id
                && (cur_copper >= tav_price || can_open_credit)
                && delta
                    .actors
                    .get(*kid)
                    .is_some_and(|k| count_box_home_rations_only(k, tn) >= 3)
        })
        .map(|(tn, kid)| (kid.to_string(), tn.to_string(), tav_price))
        .or_else(|| {
            if cur_copper < 2 && !can_open_credit {
                return None;
            }
            delta.actors.iter().find_map(|(oid, o)| {
                let ohk = effective_home_node(o);
                (oid != buyer_id
                    && !oid.starts_with("pl:")
                    && o.alive
                    && o.controller_id.is_none()
                    && is_food_producer_role(&o.role)
                    && doc.nodes.get(ohk).is_some_and(|n| n.cell_id == cell_id)
                    && count_box_home_rations_only(o, ohk) >= 6)
                    .then(|| (oid.clone(), ohk.to_string(), 2))
            })
        });
    let Some((sid, sbk, unit_copper)) = seller_pair else {
        return false;
    };

    // 先由賣方木箱實扣 3 份口糧；若轉帳或賒帳失敗立即退回原箱，100% 雙邊物品與銅幣原子守恆
    if !delta
        .actors
        .get_mut(&sid)
        .is_some_and(|s| deduct_box_home_rations(s, &sbk, 3))
    {
        return false;
    }

    let settled = if cur_copper >= unit_copper {
        delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "larder_restock",
            buyer_id,
            &sid,
            unit_copper,
            format!("{dname}採買三份家常魚麥口糧補入家門"),
        )
    } else if can_open_credit
        && !sid.starts_with("pl:")
        && delta
            .actors
            .get(&sid)
            .is_some_and(|s| s.controller_id.is_none())
    {
        let cid = format!("larder_tab:{}:{}:{}", buyer_id, sid, clock.game_days);
        delta.dirty_contracts.insert(cid.clone());
        delta.contracts.insert(
            cid.clone(),
            ContractRecord {
                contract_id: cid,
                kind: "larder_tab".to_string(),
                creditor_id: sid.clone(),
                debtor_id: buyer_id.to_string(),
                amount_copper: unit_copper,
                due_day: clock.game_days + 3,
                status: "open".to_string(),
                note_zh: format!("{dname}家裡斷糧，賒購三份家常魚麥口糧"),
            },
        );
        true
    } else {
        false
    };
    if !settled {
        if let Some(seller) = delta.actors.get_mut(&sid) {
            let sv = seller.inn_lockers.entry(sbk).or_default();
            add_carry_item(sv, make_home_ration(3));
        }
        return false;
    }

    if let Some(seller) = delta.actors.get_mut(&sid) {
        seller.mark_mutated();
    }
    if let Some(buyer) = delta.actors.get_mut(buyer_id) {
        let bv = buyer.inn_lockers.entry(home_key).or_default();
        add_carry_item(bv, make_home_ration(3));
        buyer
            .mind
            .resolve_concerns_matching(ConcernCause::LivelihoodShortfall, None, 35);
        buyer.mind.stm.today_goal_progressed = true;
        buyer.mark_mutated();
    }
    true
}

pub fn settle_daily_town_livelihood_and_levy(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    cell_tavern_cache: &BTreeMap<u32, (Option<String>, Option<String>)>,
) {
    if clock.bell_index != 5 && clock.bell_index != 8 {
        return;
    }

    if clock.bell_index == 5 {
        let mut cell_masters: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut cell_helpers: BTreeMap<u32, Vec<String>> = BTreeMap::new();
        let mut cell_low_workers: BTreeMap<u32, Vec<(String, String)>> = BTreeMap::new();
        let mut producers: Vec<(String, u32, u32, String, String)> = Vec::new();
        let mut public_workers: Vec<(String, String)> = Vec::new();

        for (aid, a) in &delta.actors {
            if !a.alive || !a.setup_done || a.controller_id.is_some() || aid.starts_with("pl:") {
                continue;
            }
            let hk = effective_home_node(a).to_string();
            let raw_cid = doc.nodes.get(&hk).map_or(4616, |n| n.cell_id);
            let cid = super::rural_migrate::nearest_core_town_cell_id(doc, raw_cid);
            if a.copper < 120 {
                cell_low_workers
                    .entry(cid)
                    .or_default()
                    .push((aid.clone(), a.display_name.clone()));
            }
            if is_food_producer_role(&a.role) {
                producers.push((aid.clone(), raw_cid, cid, hk, a.display_name.clone()));
            } else if is_public_works_role(&a.role)
                || (a.household_id.starts_with("hh:burg_") && a.copper < 72)
            {
                public_workers.push((aid.clone(), a.display_name.clone()));
            } else if is_craft_or_shop_master_role(&a.role) && a.copper >= 28 {
                cell_masters.entry(cid).or_default().push(aid.clone());
            } else if is_helper_or_apprentice_role(&a.role) {
                cell_helpers.entry(cid).or_default().push(aid.clone());
            }
        }

        // 1. 農漁獵戶依水患、墾田與地塊實存產出當日真實口糧；未斷橋未封鎖者餘糧方可運售區域酒館
        for (pid, raw_cid, cid, home_node, pname) in producers {
            let daily_yield =
                super::live_commodities::harvest_daily_producer_rations(doc, delta, raw_cid);
            if let Some(p) = delta.actors.get_mut(&pid) {
                let cur = count_box_home_rations_only(p, &home_node);
                if cur < MAX_HOUSEHOLD_LARDER {
                    let add_n = (MAX_HOUSEHOLD_LARDER - cur).min(daily_yield);
                    let bv = p.inn_lockers.entry(home_node.clone()).or_default();
                    add_carry_item(bv, make_home_ration(add_n));
                    p.mind.stm.today_goal_progressed = true;
                    p.mark_mutated();
                }
            }

            if !super::live_commodities::can_deliver_surplus_to_town(doc, delta, raw_cid, cid) {
                continue;
            }
            if let Some((Some(tav_nid), Some(ik_id))) = cell_tavern_cache.get(&cid)
                && ik_id != &pid
            {
                for _batch in 0..3 {
                    let can_sell = delta
                        .actors
                        .get(&pid)
                        .is_some_and(|p| count_box_home_rations_only(p, &home_node) >= 6);
                    let tav_stk = delta
                        .actors
                        .get(ik_id)
                        .map_or(0, |ik| count_box_home_rations_only(ik, tav_nid));
                    let wholesale_copper = if tav_stk < 15 { 3 } else { 2 };
                    let ik_ok = delta.actors.get(ik_id).is_some_and(|ik| {
                        ik.copper >= wholesale_copper + 12 && tav_stk < MAX_TAVERN_LARDER
                    });
                    if !can_sell || !ik_ok {
                        break;
                    }
                    let deducted = delta
                        .actors
                        .get_mut(&pid)
                        .is_some_and(|p| deduct_box_home_rations(p, &home_node, 3));
                    if !deducted {
                        break;
                    }
                    if delta.transfer_copper(
                        clock.game_days,
                        clock.bell_index,
                        "tavern_provision_buy",
                        ik_id,
                        &pid,
                        wholesale_copper,
                        format!("酒館向{}收購新鮮魚麥食材", pname),
                    ) {
                        if let Some(p) = delta.actors.get_mut(&pid) {
                            p.mark_mutated();
                        }
                        if let Some(ik) = delta.actors.get_mut(ik_id) {
                            let tv = ik.inn_lockers.entry(tav_nid.clone()).or_default();
                            add_carry_item(tv, make_home_ration(3));
                            ik.mark_mutated();
                        }
                    } else if let Some(p) = delta.actors.get_mut(&pid) {
                        let pv = p.inn_lockers.entry(home_node.clone()).or_default();
                        add_carry_item(pv, make_home_ration(3));
                        break;
                    }
                }
            }
        }

        // 2. 鋪主與富裕產糧大戶發放幫手與學徒日常工錢
        for (cid, helpers) in cell_helpers {
            let mut employers = cell_masters.get(&cid).cloned().unwrap_or_default();
            if employers.len() < 4 {
                for (aid, a) in &delta.actors {
                    if a.alive
                        && !aid.starts_with("pl:")
                        && a.controller_id.is_none()
                        && a.copper >= 95
                        && doc
                            .nodes
                            .get(&a.home_node)
                            .is_some_and(|n| n.cell_id == cid)
                    {
                        employers.push(aid.clone());
                    }
                }
            }
            if employers.is_empty() {
                continue;
            }
            for (idx, hid) in helpers.iter().enumerate() {
                let mid =
                    &employers[(idx + (clock.game_days.unsigned_abs() as usize)) % employers.len()];
                if mid == hid || delta.actors.get(mid).is_none_or(|m| m.copper < 24) {
                    continue;
                }
                let hname = delta
                    .actors
                    .get(hid)
                    .map_or("幫手", |h| h.display_name.as_str())
                    .to_string();
                if delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "craft_helper_wage",
                    mid,
                    hid,
                    1 + i64::from(clock.game_days.rem_euclid(2) == 0),
                    format!("發放{}鋪頭當值工錢", hname),
                ) && let Some(h) = delta.actors.get_mut(hid)
                {
                    h.mind.stm.today_goal_progressed = true;
                    h.mark_mutated();
                }
            }
        }

        // 2b. R2-M2 修正：高營收酒館與富裕大鋪每日僱請同鎮手頭偏緊之工匠／車腳修桶添油挑水，讓麥酒收益回流鎮民荷包
        let rich_keepers: Vec<(String, u32, String)> = delta
            .actors
            .iter()
            .filter_map(|(aid, a)| {
                if !a.alive
                    || !a.setup_done
                    || a.controller_id.is_some()
                    || aid.starts_with("pl:")
                    || a.copper < 340
                    || !is_craft_or_shop_master_role(&a.role)
                {
                    return None;
                }
                let hk = effective_home_node(a);
                let raw_cid = doc.nodes.get(hk).map_or(4616, |n| n.cell_id);
                let cid = super::rural_migrate::nearest_core_town_cell_id(doc, raw_cid);
                Some((aid.clone(), cid, a.display_name.clone()))
            })
            .collect();
        for (kid, cid, kname) in rich_keepers {
            let Some(local_workers) = cell_low_workers.get(&cid) else {
                continue;
            };
            if local_workers.is_empty() {
                continue;
            }
            let base_off = (clock.game_days.unsigned_abs() as usize) * 5;
            for slot in 0..6.min(local_workers.len()) {
                if delta.actors.get(&kid).is_none_or(|k| k.copper < 260) {
                    break;
                }
                let (wid, wname) = &local_workers[(base_off + slot) % local_workers.len()];
                if wid == &kid {
                    continue;
                }
                let _ = delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "tavern_upkeep",
                    &kid,
                    wid,
                    4,
                    format!("{kname}僱請{wname}修箍橡木酒桶與挑水添油"),
                );
            }
        }

        // 2c. R4-H1 修正：鎮內鐵匠／木匠／藥師／掌櫃每日將收購之山海原料鍛製配藥交付港務船塢公用，打通「採集 → 工匠加工 → 港務採購回款」完整物理與銅幣閉環！
        let craft_ids: Vec<String> = delta
            .actors
            .iter()
            .filter_map(|(aid, a)| {
                (a.alive
                    && a.setup_done
                    && a.controller_id.is_none()
                    && !aid.starts_with("pl:")
                    && a.inn_lockers.get(&a.work_node).is_some_and(|v| {
                        v.iter().any(|i| {
                            matches!(i.slug.as_str(), "iron_ore" | "driftwood" | "herb_bundle")
                                && i.bound_owner.is_none()
                                && i.stolen_from.is_none()
                                && i.count > 0
                        })
                    }))
                .then(|| aid.clone())
            })
            .collect();
        for cid_actor in craft_ids {
            if delta.municipal_treasury_copper <= 12_500 {
                break;
            }
            for _ in 0..6 {
                if delta.municipal_treasury_copper <= 12_500 {
                    break;
                }
                let raw_hit = delta.actors.get(&cid_actor).and_then(|a| {
                    let wv = a.inn_lockers.get(&a.work_node)?;
                    [
                        ("iron_ore", 6_i64, "港務錨鏈鐵件"),
                        ("driftwood", 5_i64, "修船乾木板料"),
                        ("herb_bundle", 5_i64, "船塢止血藥包"),
                    ]
                    .into_iter()
                    .find_map(|(sl, pay, prod_zh)| {
                        if sl == "iron_ore"
                            && super::live_commodities::is_smithy_forge_cold(delta, &a.work_node)
                        {
                            return None;
                        }
                        let clean_sum: u32 = wv
                            .iter()
                            .filter(|i| {
                                i.slug == sl
                                    && i.bound_owner.is_none()
                                    && i.stolen_from.is_none()
                                    && i.count > 0
                            })
                            .map(|i| i.count)
                            .sum();
                        (clean_sum > 6).then(|| {
                            (
                                a.work_node.clone(),
                                sl,
                                pay,
                                prod_zh,
                                a.display_name.clone(),
                            )
                        })
                    })
                });
                let Some((wnode, sl, pay, prod_zh, cname)) = raw_hit else {
                    break;
                };
                // R5-M4 修正：驗貨與扣貨嚴格統一於乾淨原料，且先實扣原料再轉帳（失敗即退回），封死贓物空付款漏洞！
                let deducted = delta
                    .actors
                    .get_mut(&cid_actor)
                    .and_then(|a| a.inn_lockers.get_mut(&wnode))
                    .is_some_and(|wv| deduct_clean_slug_one(wv, sl));
                if !deducted {
                    break;
                }
                if delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "harbor_craft_supply",
                    "municipal_treasury",
                    &cid_actor,
                    pay,
                    format!("{cname}加工原料交付【{prod_zh}】予港務公倉"),
                ) {
                    if let Some(a) = delta.actors.get_mut(&cid_actor) {
                        a.mark_mutated();
                    }
                } else {
                    if let Some(a) = delta.actors.get_mut(&cid_actor) {
                        let wv = a.inn_lockers.entry(wnode).or_default();
                        add_carry_item(
                            wv,
                            CarryItem {
                                slug: sl.to_string(),
                                name: prod_zh.to_string(),
                                count: 1,
                                weight_units: 1,
                                ..CarryItem::default()
                            },
                        );
                    }
                    break;
                }
            }
        }

        // 3. 港務公庫發放公共勞務薪餉（隨公庫充裕度自動調節 1..=3 銅）
        let wage_amt = if delta.municipal_treasury_copper >= 26_500 {
            3
        } else {
            1 + i64::from(delta.municipal_treasury_copper >= 20_500)
        };
        let pw_len = public_workers.len();
        let pw_off = if pw_len > 0 {
            (clock.game_days.unsigned_abs() as usize * 17) % pw_len
        } else {
            0
        };
        for i in 0..pw_len {
            if delta.municipal_treasury_copper <= 8_000 {
                break;
            }
            let (ref wid, ref wname) = public_workers[(pw_off + i) % pw_len];
            if delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "municipal_wage",
                "municipal_treasury",
                wid,
                wage_amt,
                format!("港務公庫發放{}當值薪餉", wname),
            ) && let Some(w) = delta.actors.get_mut(wid)
            {
                w.mind.stm.today_goal_progressed = true;
                w.mark_mutated();
            }
        }
    }

    if clock.bell_index == 8 {
        let low_treasury = delta.municipal_treasury_copper < 24_800;
        let taxpayers: Vec<(String, String, i64)> = delta
            .actors
            .iter()
            .filter_map(|(aid, a)| {
                if !a.alive || !a.setup_done || a.controller_id.is_some() || aid.starts_with("pl:")
                {
                    return None;
                }
                let operates_active_shop =
                    super::shop_leases::actor_currently_operates_any_shop(doc, delta, a).is_some()
                        || is_craft_or_shop_master_role(&a.role);
                if operates_active_shop && a.copper >= 75 {
                    let levy = if a.copper > 320 {
                        10 + (a.copper - 320) / 10
                    } else {
                        ((a.copper - 60) / 14).clamp(2, 10)
                    };
                    Some((aid.clone(), a.display_name.clone(), levy))
                } else if a.copper >= 220 {
                    Some((
                        aid.clone(),
                        a.display_name.clone(),
                        ((a.copper - 200) / 14).clamp(1, 12),
                    ))
                } else if low_treasury && a.copper >= 72 {
                    Some((
                        aid.clone(),
                        a.display_name.clone(),
                        ((a.copper - 62) / 12).clamp(1, 6),
                    ))
                } else {
                    None
                }
            })
            .collect();

        for (tid, tname, levy) in taxpayers {
            let _ = delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "municipal_stall_levy",
                &tid,
                "municipal_treasury",
                levy,
                format!("{}繳納商鋪攤位與海港堤防修繕公費", tname),
            );
        }
    }
}

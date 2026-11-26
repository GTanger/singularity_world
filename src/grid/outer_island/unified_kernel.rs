//! 全員單一共享動詞核（`UnifiedActionKernel`）與「知情對等 × 正向奔頭 × 符質守恆池 × 空巢易主」自主驅動引擎（Room `7eee7478` 定案）。

use crate::combat::wordplate::splitmix64;

use super::actions::apply_world_action;
use super::appraisal::ConcernCause;
use super::clock_and_lang::SolarClockSnapshot;
use super::explore_and_combat::{apply_explore, resolve_skirmish_turn};
use super::harbor_lineage::CURRENT_SOLAR_YEAR;
use super::mind::EventPublicity;
use super::movement::{
    add_carry_item, find_hidden_entrance_for_rumor, push_breath, shortest_expedition_path,
    take_clean_carry_items,
};
use super::pacts_and_peers::{
    accept_escrow_pact, apply_peer_assist_skirmish, apply_peer_consensual_spar,
    apply_peer_item_trade, fulfill_escrow_pact, is_escrow_pact_kind, parse_pact_note,
    propose_escrow_pact, settle_expired_escrow_pacts_as_world_events,
};
use super::persona::{InnatePersona, ValueAxis, derive_innate_persona};
use super::phase_and_vessels::{
    BuiltHomestead, ensure_chaos_rune_pool_seeded, equip_living_vessel, forge_living_vessel,
};
use super::sim_tick::broadcast_scene_event;
use super::types::{IslandWorldDelta, WorldStaticDoc};
use super::world_weave_bio::racial_adult_age;

/// 單一共享延伸動詞入口：玩家 `actions.rs` 與居民自主決策 100% 走同一路徑
pub fn dispatch_extended_shared_verb(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
    action: &str,
    param: &str,
) -> Result<String, String> {
    ensure_chaos_rune_pool_seeded(delta);
    let p = param.trim();
    match action {
        "forge_vessel" => forge_living_vessel(doc, delta, clock, actor_key),
        "equip_vessel" => equip_living_vessel(delta, actor_key, p),
        "build_homestead" => build_new_homestead(doc, delta, clock, actor_key),
        "propose_courier_pact" => {
            propose_escrow_pact(doc, delta, clock, actor_key, "courier_errand", p, 5)
        }
        "propose_supply_pact" => {
            propose_escrow_pact(doc, delta, clock, actor_key, "supply_commission", p, 8)
        }
        "propose_cover_pact" => {
            propose_escrow_pact(doc, delta, clock, actor_key, "shop_cover", p, 6)
        }
        "propose_escort_pact" => {
            propose_escrow_pact(doc, delta, clock, actor_key, "escort_watch", p, 6)
        }
        "accept_pact" => accept_escrow_pact(doc, delta, clock, actor_key, p),
        "fulfill_pact" => fulfill_escrow_pact(doc, delta, clock, actor_key, p),
        "peer_buy_item" => apply_peer_item_trade(delta, clock, actor_key, "buy_from_peer", param),
        "peer_offer_item" => apply_peer_item_trade(delta, clock, actor_key, "offer_to_peer", param),
        "peer_accept_offer" => {
            apply_peer_item_trade(delta, clock, actor_key, "accept_peer_offer", param)
        }
        "peer_assist" => apply_peer_assist_skirmish(doc, delta, clock, actor_key, p),
        "peer_spar" => apply_peer_consensual_spar(doc, delta, clock, actor_key, p),
        _ => Err("此處暫無這項舉動。".to_string()),
    }
}

/// 耗費 `2 漂流木 + 1 粗鐵礦 + 120 枚銅幣` 於聚落街區營造自立門戶新屋契（玩家與成年居民對等共用）
pub fn build_new_homestead(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_key: &str,
) -> Result<String, String> {
    let aid = delta
        .resolve_actor_id(actor_key)
        .ok_or("請先向港務官核對通行身分。")?;
    let a0 = delta.actors.get(&aid).ok_or("找不到行蹤")?;
    if a0.owned_house_node.is_some() {
        return Err("你在島上已有一處自有屋舍，無須重複起屋。".to_string());
    }
    let (node_id, pname, a_seed) = (a0.node_id.clone(), a0.display_name.clone(), a0.soul_seed);
    let node = doc.nodes.get(&node_id).ok_or("方位不明")?;
    if !matches!(node.kind.as_str(), "plaza" | "alley") || node.dungeon {
        return Err("需在聚落廣場或街巷空地旁才能請木石工匠立柱起屋。".to_string());
    }

    let mat_err = "營造新屋需備齊來歷乾淨的【異國刻紋漂流木 × 2】與【粗鐵礦石 × 1】作樑柱門樞。";
    let (wood_taken, iron_taken) = {
        let a = delta.actors.get_mut(&aid).ok_or("找不到行蹤")?;
        let w = take_clean_carry_items(&mut a.backpack, "driftwood", 2).ok_or(mat_err)?;
        let Some(ir) = take_clean_carry_items(&mut a.backpack, "iron_ore", 1) else {
            add_carry_item(&mut a.backpack, w);
            return Err(mat_err.to_string());
        };
        a.mark_mutated();
        (w, ir)
    };
    let builder_npc = delta
        .actors
        .values()
        .find(|x| {
            x.alive
                && x.actor_id != aid
                && x.controller_id.is_none()
                && doc.nodes.get(&x.home_node).map(|n| n.cell_id) == Some(node.cell_id)
                && (x.role.contains("木") || x.role.contains("石") || x.role.contains("匠"))
        })
        .map(|x| x.actor_id.clone());
    let payee = builder_npc.as_deref().unwrap_or("municipal_treasury");
    let lot_node_id = if node.kind == "alley" {
        node_id.clone()
    } else {
        let alleys: Vec<_> = doc
            .nodes
            .iter()
            .filter(|(_, n)| n.cell_id == node.cell_id && n.kind == "alley")
            .map(|(k, _)| k.clone())
            .collect();
        alleys
            .get((a_seed as usize) % alleys.len().max(1))
            .cloned()
            .unwrap_or_else(|| node_id.clone())
    };
    let lot_name = doc
        .nodes
        .get(&lot_node_id)
        .map_or(node.name.as_str(), |n| n.name.as_str());
    let note = format!("於{}立柱營造自有石木新宅", lot_name);
    if !delta.transfer_copper(
        clock.game_days,
        clock.bell_index,
        "build_homestead",
        &aid,
        payee,
        120,
        note,
    ) {
        if let Some(a) = delta.actors.get_mut(&aid) {
            add_carry_item(&mut a.backpack, wood_taken);
            add_carry_item(&mut a.backpack, iron_taken);
            a.mark_mutated();
        }
        return Err("營造新屋工料契費共需 120 枚銅幣，你手頭銅板尚不足數。".to_string());
    }
    let town_slug = lot_node_id.split(':').nth(1).unwrap_or("white_gull_harbor");
    let pub_owner_tag = delta.actors.get(&aid).map_or_else(
        || format!("h{:08x}", splitmix64(clock.game_days as u64) as u32),
        |a| {
            if a.controller_id.is_some() || aid.starts_with("pl:") {
                a.public_actor_token().replace(':', "_")
            } else {
                aid.replace(':', "_")
            }
        },
    );
    let house_id = format!("homestead:{}:{}", pub_owner_tag, clock.game_days);
    let label_zh = format!("{}・{}新宅", lot_name, pname);
    delta.relocate_actor(&aid, &lot_node_id, node.cell_id, true);
    if let Some(a) = delta.actors.get_mut(&aid) {
        a.owned_house_node = Some(lot_node_id.clone());
        a.home_node = lot_node_id.clone();
        if a.household_bond != "當家屋主" {
            a.household_id = format!("hh:{}:homestead:{}", town_slug, pub_owner_tag);
            a.household_bond = "自立門戶屋主".to_string();
        }
        a.mind.stm.today_goal_progressed = true;
        a.mind.push_turning_point(
            &house_id,
            clock.game_days,
            &format!("攢足木鐵料與百二十枚銅幣，在{}起屋自立門戶", lot_name),
            "work_steady",
        );
        a.mark_mutated();
    }
    delta.built_houses.insert(
        house_id.clone(),
        BuiltHomestead {
            house_id,
            owner_id: aid.clone(),
            anchor_node_id: lot_node_id.clone(),
            label_zh: label_zh.clone(),
            built_day: clock.game_days,
        },
    );
    super::living_chronicler::record_homestead_built(
        doc,
        delta,
        clock,
        &aid,
        &lot_node_id,
        &lot_name,
    );
    broadcast_scene_event(
        doc,
        delta,
        clock,
        &lot_node_id,
        &aid,
        &pname,
        EventPublicity::PublicLoud,
        Some("harbor_trade"),
        84,
        58,
        ValueAxis::Autonomy,
        "build_homestead",
        &format!(
            "{}備齊木鐵工錢立柱上樑，落成【{}】正式自立門戶",
            pname, label_zh
        ),
    );
    let msg = format!(
        "木石匠師於【{}】立柱上樑、嵌妥鐵樞木門，【{}】正式落成領契！你已步入自家新宅，隨時可回屋安睡與鎖藏器物。",
        lot_name, label_zh
    );
    if let Some(a) = delta.actors.get_mut(&aid) {
        push_breath(a, msg.clone());
        a.mark_mutated();
    }
    Ok(msg)
}

/// 每遊戲日地脈混沌潮回補 `chaos_rune_pool`（池內存量＋全島存世原石／穩固鐵紋總和嚴守 `<= 12` 絕對上限！）
pub fn replenish_chaos_rune_pool_on_tide(delta: &mut IslandWorldDelta, game_days: i64) {
    ensure_chaos_rune_pool_seeded(delta);
    if game_days > 0 && game_days % 6 == 0 {
        let active = u16::from(super::phase_and_vessels::count_world_active_runes(delta));
        if delta.chaos_rune_pool.saturating_add(active) < 12 {
            delta.chaos_rune_pool = (delta.chaos_rune_pool + 1).min(12);
        }
    }
}

/// 每步推進全體非在線手操人物之「進行中交鋒（`resolve_skirmish_turn`）」與「乘車推進（`carriage_next`）」——100% 共用玩家動詞！
pub fn step_autonomous_skirmishes_and_transits(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let abs_step = clock.real_unix / 10;
    let mut active_ids: Vec<(u64, String)> = delta
        .actors
        .iter()
        .filter(|(_, a)| {
            a.alive
                && a.setup_done
                && (a.active_skirmish.is_some() || a.active_transit.is_some())
                && a.controller_id.is_none()
        })
        .map(|(id, a)| (splitmix64(a.soul_seed ^ (abs_step as u64)), id.clone()))
        .collect();
    active_ids.sort_by_key(|(rk, _)| *rk);

    for (_, aid) in active_ids {
        let (in_sk, in_tr, node_id) = match delta.actors.get(&aid) {
            Some(a) => (
                a.active_skirmish.clone(),
                a.active_transit.is_some(),
                a.node_id.clone(),
            ),
            None => continue,
        };
        if let Some(sk) = in_sk {
            let move_cmd = {
                let Some(a) = delta.actors.get(&aid) else {
                    continue;
                };
                let inn = derive_innate_persona(a.soul_seed.max(1));
                let has_bread = a
                    .backpack
                    .iter()
                    .any(|i| i.slug == "rye_bread" && i.count > 0);
                if (a.fatigue >= 72 || a.wound_part.is_some())
                    && inn.temperament.approach_avoid < 12
                {
                    "skirmish_retreat"
                } else if sk.foe_slug == "grey_ear_goblin"
                    && sk.can_speak_goblin
                    && has_bread
                    && inn.temperament.approach_avoid < 5
                {
                    "skirmish_parley"
                } else if !sk.player_guard && sk.foe_slug == "stone_spider" && a.fatigue < 55 {
                    "skirmish_guard"
                } else if a.bound_weapon.rune_mark.is_some()
                    || a.bound_weapon.grip_familiarity >= 14
                    || sk.player_guard
                {
                    "skirmish_heavy"
                } else {
                    "skirmish_wrist"
                }
            };
            let _ = resolve_skirmish_turn(doc, delta, clock, &aid, move_cmd);
        } else if in_tr && !doc.nodes.contains_key(&node_id) {
            // 乘車／搭船推進已統一由每拍 `tick_embodied_transit_vehicles` 實體載具驅動，此處僅清理孤兒異常狀態
            if let Some(a) = delta.actors.get_mut(&aid) {
                a.active_transit = None;
                a.mark_mutated();
            }
        }
    }
}

/// 響鐘結算：先於日常 `desire_sim` 排程執行對等共享動詞，回傳本鐘已接管或駐足候結之角色集合（杜絕日常返工路線覆寫探窟／護行！）
pub fn execute_player_parity_bell_actions(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    fair_order_ids: &[String],
) -> std::collections::BTreeSet<String> {
    let mut claimed = std::collections::BTreeSet::new();
    ensure_chaos_rune_pool_seeded(delta);
    settle_expired_escrow_pacts_as_world_events(doc, delta, clock);
    if clock.bell_index == 0 {
        replenish_chaos_rune_pool_on_tide(delta, clock.game_days);
    }
    if clock.is_night {
        return claimed;
    }

    let bridge_nid_opt = find_hidden_entrance_for_rumor(doc, "pit_rumor").map(|(k, _)| k);
    let mut actions_taken_this_bell = 0usize;
    for aid in fair_order_ids {
        let urgent = delta.actors.get(aid).is_some_and(|a| {
            doc.nodes.get(&a.node_id).is_some_and(|n| n.dungeon)
                || bridge_nid_opt == Some(a.node_id.as_str())
                || a.backpack
                    .iter()
                    .any(|i| i.slug == "sealed_errand_pkg" && i.count > 0)
                || delta.contracts.values().any(|c| {
                    c.kind == "escort_watch"
                        && c.status == "in_progress"
                        && (c.creditor_id == *aid || c.debtor_id == *aid)
                })
        });
        if !urgent && actions_taken_this_bell >= 36 {
            continue;
        }
        let Some(plan) = evaluate_autonomous_player_verb(doc, delta, clock, aid) else {
            continue;
        };
        let mut hold_only = false;
        let executed = match plan {
            AutoPlayerVerb::WorldAct(act, param) => {
                apply_world_action(doc, delta, clock, aid, act, &param).is_ok()
            }
            AutoPlayerVerb::Extended(act, param) => {
                dispatch_extended_shared_verb(doc, delta, clock, aid, act, &param).is_ok()
            }
            AutoPlayerVerb::Explore => apply_explore(doc, delta, clock, aid).is_ok(),
            AutoPlayerVerb::WalkToAndLearnNode(dst_nid, reason) => {
                if let Some(a) = delta.actors.get_mut(aid)
                    && let Some(path) =
                        shortest_expedition_path(doc, &a.node_id, &dst_nid, &a.unlocked_secrets)
                    && path.len() > 1
                {
                    for step_n in &path {
                        a.mind.ltm.known_nodes.insert(step_n.clone());
                        if let Some(nd) = doc.nodes.get(step_n) {
                            a.mind.ltm.known_cells.insert(nd.cell_id);
                        }
                    }
                    a.route_plan = path.into_iter().skip(1).collect();
                    if a.mind.stm.intentions.len() >= 4 {
                        a.mind.stm.intentions.pop_front();
                    }
                    a.mind
                        .stm
                        .intentions
                        .push_back(super::mind::ActiveIntention {
                            kind: "player_expedition".to_string(),
                            target_node: dst_nid,
                            reason_zh: reason,
                            created_bell: clock.game_days * 12 + i64::from(clock.bell_index),
                        });
                    a.mark_mutated();
                    true
                } else {
                    false
                }
            }
            AutoPlayerVerb::HoldAtGoal => {
                hold_only = true;
                false
            }
            AutoPlayerVerb::CareerShiftToDock => {
                let shifted_dock = if let Some(a) = delta.actors.get_mut(aid)
                    && let my_cell = doc.nodes.get(&a.home_node).map_or(4616, |n| n.cell_id)
                    && let Some((dock_id, _)) = doc.nodes.iter().find(|(_, n)| {
                        n.cell_id == my_cell && matches!(n.kind.as_str(), "dock" | "plaza")
                    }) {
                    let d_id = dock_id.clone();
                    a.role = "碼頭苦力".to_string();
                    a.work_node = d_id.clone();
                    a.mind.push_turning_point(
                        &format!("career:dock:{}:{}", aid, clock.game_days),
                        clock.game_days,
                        "舊業虧空難支，索性捲起衣袖改赴碼頭扛重活重頭攢起",
                        "clear_debt",
                    );
                    a.mark_mutated();
                    Some(d_id)
                } else {
                    None
                };
                if let Some(ref d_id) = shifted_dock {
                    super::living_chronicler::record_career_shift_dock(
                        doc, delta, clock, aid, d_id,
                    );
                }
                shifted_dock.is_some()
            }
        };
        if executed || hold_only {
            actions_taken_this_bell += usize::from(executed);
            claimed.insert(aid.clone());
        }
    }
    claimed
}

enum AutoPlayerVerb {
    WorldAct(&'static str, String),
    Extended(&'static str, String),
    Explore,
    WalkToAndLearnNode(String, String),
    HoldAtGoal,
    CareerShiftToDock,
}

fn evaluate_autonomous_player_verb(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Option<AutoPlayerVerb> {
    let a = delta.actors.get(aid)?;
    if !a.alive
        || !a.setup_done
        || a.controller_id.is_some()
        || aid.starts_with("pl:")
        || a.active_skirmish.is_some()
        || a.active_transit.is_some()
        || !a.route_plan.is_empty()
    {
        return None;
    }

    let innate: InnatePersona = derive_innate_persona(a.soul_seed.max(1));
    let top_val = innate.top_value_axis(&a.drift);
    let node = doc.nodes.get(&a.node_id)?;

    // 1. 同場有人正遇襲交鋒：重義氣／護短者當場拔刀助戰！
    if a.fatigue < 70
        && a.wound_part.is_none()
        && let Some(occ) = delta.node_occupants.get(&a.node_id)
    {
        for oid in occ {
            if oid != aid
                && let Some(other) = delta.actors.get(oid)
                && other.alive
                && other.active_skirmish.is_some()
            {
                let warm = a.mind.ltm.social.get(oid).map_or(0, |s| s.warmth);
                if warm >= 15
                    || top_val == ValueAxis::KinshipLoyalty
                    || innate.temperament.approach_avoid >= 14
                {
                    return Some(AutoPlayerVerb::Extended(
                        "peer_assist",
                        other.public_actor_token(),
                    ));
                }
            }
        }
    }

    // 2. 地面有遺落行囊（排除待旅人續送之港務封蠟貨箱）：當場彎腰拾取！若懷有未申報拾遺物且重信諾守規矩，赴城門／廣場申報招領
    if delta.dropped_loot_by_node.get(&a.node_id).is_some_and(|v| {
        v.iter()
            .any(|b| b.copper > 0 || b.items.iter().any(|i| i.slug != "dock_cargo_crate"))
    }) {
        return Some(AutoPlayerVerb::WorldAct(
            "pickup_dropped_bag",
            String::new(),
        ));
    }
    if a.backpack.iter().any(|i| i.stolen_from.is_some())
        && matches!(
            top_val,
            ValueAxis::PactIntegrity | ValueAxis::OrderHierarchy
        )
    {
        if node.kind == "gate" || node.kind == "plaza" {
            return Some(AutoPlayerVerb::WorldAct("declare_found_bag", String::new()));
        }
        if let Some(plaza_id) = find_local_node_by_kind(doc, node.cell_id, "plaza") {
            return Some(AutoPlayerVerb::WalkToAndLearnNode(
                plaza_id,
                "攜拾獲行囊赴廣場向衛兵報備招領".to_string(),
            ));
        }
    }

    // 3. 受傷求醫與傷假替班託付：帶傷且有 3 銅則赴草藥堂包紮；掌櫃／匠師帶傷則發起 `shop_cover` 請人替班
    if a.wound_part.is_some() {
        if a.copper >= 10 && (a.role.contains("掌櫃") || a.role.contains("鐵匠")) {
            let has_cover = delta.contracts.values().any(|c| {
                c.creditor_id == aid
                    && c.kind == "shop_cover"
                    && (c.status == "escrowed" || c.status == "in_progress")
            });
            if !has_cover {
                return Some(AutoPlayerVerb::Extended(
                    "propose_cover_pact",
                    String::new(),
                ));
            }
        }
        if let Some((herb_nid, _)) = doc.nodes.iter().find(|(_, n)| n.name.contains("草藥"))
            && !super::live_commodities::is_apothecary_out_of_herbs(delta, herb_nid)
            && a.copper >= super::live_commodities::quote_bandage_copper(delta, herb_nid)
        {
            if node.name.contains("草藥") {
                return Some(AutoPlayerVerb::WorldAct("bandage_wound", String::new()));
            }
            return Some(AutoPlayerVerb::WalkToAndLearnNode(
                herb_nid.clone(),
                "手腕帶傷趕往苦艾草藥堂敷藥包紮".to_string(),
            ));
        }
    }

    // 4. 手頭持有已承接之封緘家書 (`courier_errand`)：搭雙馬篷車或徒步跨鎮送交收信人！
    if let Some(pkg) = a
        .backpack
        .iter()
        .find(|i| i.slug == "sealed_errand_pkg" && i.count > 0)
        && let Some(ref cid) = pkg.bound_owner
        && let Some(c) = delta.contracts.get(cid)
    {
        let (rec_id, _) = parse_pact_note(&c.note_zh);
        if let Some(rec) = delta.actors.get(rec_id)
            && rec.alive
        {
            if a.node_id == rec.node_id {
                return Some(AutoPlayerVerb::Extended("fulfill_pact", cid.clone()));
            }
            return Some(travel_cross_town(
                doc,
                node,
                a.copper,
                &rec.node_id,
                format!("專程趕往{}當面遞交封緘家書", rec.display_name),
            ));
        }
    }

    // 4b. 同行護衛契 (`escort_watch`)：已有人接單 (`in_progress`) 時，委託人或護衛均真實搭車／徒步前往約定廣場並駐足候結！
    if let Some(ec) = delta.contracts.values().find(|c| {
        c.kind == "escort_watch"
            && c.status == "in_progress"
            && (c.creditor_id == aid || c.debtor_id == aid)
    }) {
        let (dst_nid, _) = parse_pact_note(&ec.note_zh);
        if a.node_id == dst_nid {
            if ec.debtor_id == aid
                && delta
                    .actors
                    .get(&ec.creditor_id)
                    .is_some_and(|iss| iss.alive && iss.node_id == dst_nid)
            {
                return Some(AutoPlayerVerb::Extended(
                    "fulfill_pact",
                    ec.contract_id.clone(),
                ));
            }
            return Some(AutoPlayerVerb::HoldAtGoal);
        }
        return Some(travel_cross_town(
            doc,
            node,
            a.copper,
            dst_nid,
            "依護行契動身趕往約定廣場會合".to_string(),
        ));
    }

    // 5. 心事化為真實託付契：外鎮思念高漲且手頭寬裕者於城鎮發起捎信／護行契；缺料匠師發起代購契
    let has_my_pact = |k: &str| {
        delta.contracts.values().any(|c| {
            c.creditor_id == aid
                && c.kind == k
                && matches!(c.status.as_str(), "escrowed" | "in_progress")
        })
    };
    let has_yearning = a
        .mind
        .stm
        .focus_concerns
        .iter()
        .flatten()
        .any(|c| c.cause == ConcernCause::CrossTownYearning && c.intensity >= 46);
    if has_yearning
        && a.copper >= 16
        && matches!(node.kind.as_str(), "plaza" | "shop" | "street")
        && (a.soul_seed + clock.game_days.unsigned_abs()) % 5 == 0
    {
        if innate.temperament.approach_avoid <= -12
            && a.copper >= 26
            && node.kind == "plaza"
            && !has_my_pact("escort_watch")
        {
            return Some(AutoPlayerVerb::Extended(
                "propose_escort_pact",
                String::new(),
            ));
        }
        if !has_my_pact("courier_errand") {
            return Some(AutoPlayerVerb::Extended(
                "propose_courier_pact",
                String::new(),
            ));
        }
    }

    let clean_cnt = |items: &[super::types::CarryItem], pred: fn(&str) -> bool| -> u32 {
        items
            .iter()
            .filter(|i| pred(&i.slug) && i.bound_owner.is_none() && i.stolen_from.is_none())
            .map(|i| i.count)
            .sum()
    };
    let is_artisan = a.role.contains("鍛鐵") || a.role.contains("鐵匠") || a.role.contains("木匠");
    if is_artisan && a.node_id == a.work_node && a.fatigue < 65 {
        fn is_mat(s: &str) -> bool {
            matches!(
                s,
                "iron_ore" | "driftwood" | "spider_carapace" | "lizard_scale" | "rune_stone"
            )
        }
        let raw_cnt = clean_cnt(&a.backpack, is_mat)
            + a.inn_lockers
                .get(&a.work_node)
                .map_or(0, |v| clean_cnt(v, is_mat));
        let vessel_spares = a
            .backpack
            .iter()
            .filter(|i| i.vessel_meta.is_some())
            .count();
        if raw_cnt >= 2 && vessel_spares < 2 {
            return Some(AutoPlayerVerb::Extended("forge_vessel", String::new()));
        }
        if raw_cnt == 0
            && a.copper >= 22
            && !has_my_pact("supply_commission")
            && (a.soul_seed + clock.game_days.unsigned_abs()) % 6 == 0
        {
            return Some(AutoPlayerVerb::Extended(
                "propose_supply_pact",
                "iron_ore".to_string(),
            ));
        }
    }

    // 6. 行商／跑腿／求財居民在廣場或酒館承接或當場交差託管契
    if (node.kind == "plaza" || node.kind == "shop")
        && (a.role.contains("行腳")
            || a.role.contains("貨郎")
            || a.role.contains("苦力")
            || top_val == ValueAxis::TangibleGain
            || a.copper < 14)
    {
        for c in delta.contracts.values() {
            if !is_escrow_pact_kind(&c.kind) || c.creditor_id == aid {
                continue;
            }
            let (meta_t, _) = parse_pact_note(&c.note_zh);
            let active_open_or_mine = (c.status == "escrowed" && c.debtor_id.is_empty())
                || (c.status == "in_progress" && c.debtor_id == aid);
            if c.kind == "supply_commission"
                && active_open_or_mine
                && a.backpack.iter().any(|i| {
                    i.slug == meta_t
                        && i.count > 0
                        && i.bound_owner.is_none()
                        && i.stolen_from.is_none()
                })
                && delta
                    .actors
                    .get(&c.creditor_id)
                    .is_some_and(|iss| iss.node_id == a.node_id || iss.work_node == a.node_id)
            {
                return Some(AutoPlayerVerb::Extended(
                    "fulfill_pact",
                    c.contract_id.clone(),
                ));
            }
            if c.status == "escrowed" && c.kind == "courier_errand" && a.fatigue < 45 {
                let iss_cell = delta
                    .actors
                    .get(&c.creditor_id)
                    .and_then(|iss| doc.nodes.get(&iss.node_id).map(|n| n.cell_id));
                if iss_cell == Some(node.cell_id) {
                    return Some(AutoPlayerVerb::Extended(
                        "accept_pact",
                        c.contract_id.clone(),
                    ));
                }
            }
            if c.status == "escrowed"
                && c.kind == "shop_cover"
                && meta_t == a.node_id
                && a.fatigue < 50
            {
                return Some(AutoPlayerVerb::Extended(
                    "fulfill_pact",
                    c.contract_id.clone(),
                ));
            }
        }
    }

    // 7. 活器物換裝與同場向匠師購置落款良器
    if a.bound_weapon.rune_mark.is_none()
        && let Some(better) = a.backpack.iter().find(|i| {
            i.vessel_meta
                .as_ref()
                .is_some_and(|m| m.rune_mark.is_some() || a.bound_weapon.wear_stage >= 2)
        })
    {
        return Some(AutoPlayerVerb::Extended(
            "equip_vessel",
            better.slug.clone(),
        ));
    }
    if !is_artisan
        && a.copper >= 34
        && (a.bound_weapon.wear_stage >= 1
            || a.bound_weapon.provenance.contains("配發")
            || a.bound_weapon.rune_mark.is_none())
        && let Some(occ) = delta.node_occupants.get(&a.node_id)
    {
        for oid in occ {
            if oid != aid
                && let Some(seller) = delta.actors.get(oid)
                && seller.controller_id.is_none()
                && let Some(v_it) = seller
                    .backpack
                    .iter()
                    .find(|i| i.vessel_meta.is_some() && i.bound_owner.is_none())
            {
                return Some(AutoPlayerVerb::Extended(
                    "peer_buy_item",
                    format!("{}|{}", seller.public_actor_token(), v_it.slug),
                ));
            }
        }
    }
    if a.backpack
        .iter()
        .any(|i| i.slug == "rune_stone" && i.count > 0)
        && a.bound_weapon.rune_mark.is_none()
        && let Some((smith_nid, _)) = doc.nodes.iter().find(|(_, n)| n.name.contains("鍛鐵"))
        && !super::live_commodities::is_smithy_forge_cold(delta, smith_nid)
        && a.copper >= super::live_commodities::quote_engrave_rune_copper(delta, smith_nid)
    {
        if node.name.contains("鍛鐵") || node.name.contains("鐵匠") {
            return Some(AutoPlayerVerb::WorldAct("engrave_rune", String::new()));
        }
        return Some(AutoPlayerVerb::WalkToAndLearnNode(
            smith_nid.clone(),
            "懷揣古符文原石赴鍛鐵鋪請師傅鑲刻鐵紋".to_string(),
        ));
    }

    // 8. 成年非屋主「正向人生奔頭」：攢夠銅幣買下本鎮待售空屋（既有店鋪掌櫃／當家屋主不跨鎮搶屋），或備齊木鐵料起屋自立門戶！
    let lived = CURRENT_SOLAR_YEAR - a.birth_year;
    let is_adult = lived >= racial_adult_age(&a.race);
    let already_heads_shop_or_home = a.household_bond.contains("當家")
        || a.household_bond.contains("屋主")
        || a.household_bond.contains("掌櫃")
        || a.household_id.contains(":shop_");
    if is_adult && a.owned_house_node.is_none() && !already_heads_shop_or_home {
        let my_town = super::world_weave_bio::settlement_slug_of(&a.household_id);
        if a.copper >= 360 {
            if node.kind == "house_vacant"
                && !delta.sold_houses.contains_key(&a.node_id)
                && super::living_chronicler::town_slug_of_node(&a.node_id) == my_town
            {
                return Some(AutoPlayerVerb::WorldAct("buy_vacant_house", String::new()));
            }
            if let Some((vac_nid, _)) = doc.nodes.iter().find(|(nid, n)| {
                n.kind == "house_vacant"
                    && !delta.sold_houses.contains_key(*nid)
                    && super::living_chronicler::town_slug_of_node(nid) == my_town
            }) {
                return Some(AutoPlayerVerb::WalkToAndLearnNode(
                    vac_nid.clone(),
                    "揣著多年積蓄趕往本鎮待售石木屋舍辦理購屋分家".to_string(),
                ));
            }
        }
        if a.copper >= 135
            && clean_cnt(&a.backpack, |s| s == "driftwood") >= 2
            && clean_cnt(&a.backpack, |s| s == "iron_ore") >= 1
            && matches!(node.kind.as_str(), "plaza" | "alley")
        {
            return Some(AutoPlayerVerb::Extended("build_homestead", String::new()));
        }
    }

    // 9. 連番呆帳破產者真實轉行赴碼頭扛活（拒絕鎖死原身分）
    if a.copper == 0
        && !a.role.contains("苦力")
        && !a.role.contains("掌櫃")
        && delta
            .contracts
            .values()
            .any(|c| c.debtor_id == aid && c.status == "written_off")
    {
        return Some(AutoPlayerVerb::CareerShiftToDock);
    }

    // 10. 100% 知情對等之秘境探窟與同場武友切磋（無百分比配額！僅親耳聽聞 `pit_rumor` 且不知 `pit_cleared` 之敢闖者成行）
    let knows_pit = a.known_rumors.contains("pit_rumor") && !a.known_rumors.contains("pit_cleared");
    let heard_casualty = a.known_rumors.contains("pit_casualty");
    let bold_enough = if heard_casualty {
        innate.temperament.approach_avoid >= 18 || top_val == ValueAxis::CuriosityVenture
    } else {
        innate.temperament.approach_avoid >= 18
            || (innate.temperament.approach_avoid >= 8
                && matches!(
                    top_val,
                    ValueAxis::CuriosityVenture | ValueAxis::TangibleGain | ValueAxis::CraftPride
                ))
    };
    if let Some((bridge_nid, hid)) = find_hidden_entrance_for_rumor(doc, "pit_rumor") {
        if node.dungeon || (a.node_id == bridge_nid && knows_pit && bold_enough) {
            if a.wound_part.is_some() || a.fatigue >= 80 {
                if node.dungeon {
                    return Some(AutoPlayerVerb::WalkToAndLearnNode(
                        "oi:white_gull_harbor:plaza".to_string(),
                        "帶著傷乏自試煉坑撤返白鷗港歇息".to_string(),
                    ));
                }
            } else {
                let win_key = format!("{}@{}", a.node_id, clock.six_phase_key);
                let lair_cleared = delta
                    .cleared_nests_until_day
                    .get("oi:dng_pit:lair")
                    .copied()
                    .unwrap_or(-1)
                    > clock.game_days;
                if a.node_id == "oi:dng_pit:altar" && !lair_cleared {
                    return Some(AutoPlayerVerb::WalkToAndLearnNode(
                        "oi:dng_pit:lair".to_string(),
                        "先退回外層巢穴應對盤踞凶獸".to_string(),
                    ));
                }
                let need_exp = !a.explored_windows.contains(&win_key)
                    && !(a.node_id == "oi:dng_pit:lair" && lair_cleared)
                    && !(a.node_id == "oi:dng_pit:altar" && !knows_pit);
                if need_exp {
                    return Some(AutoPlayerVerb::Explore);
                }
                let next_dng = if a.node_id == bridge_nid && a.unlocked_secrets.contains(bridge_nid)
                {
                    Some(hid.target.as_str())
                } else {
                    match a.node_id.as_str() {
                        "oi:dng_pit:lair" if lair_cleared && knows_pit => Some("oi:dng_pit:altar"),
                        "oi:dng_pit:lair" | "oi:dng_pit:altar" => {
                            Some("oi:white_gull_harbor:plaza")
                        }
                        _ if node.dungeon && knows_pit => Some("oi:dng_pit:lair"),
                        _ if node.dungeon => Some("oi:white_gull_harbor:plaza"),
                        _ => None,
                    }
                };
                if let Some(dn) = next_dng {
                    return Some(AutoPlayerVerb::WalkToAndLearnNode(
                        dn.to_string(),
                        "循著親耳聽聞的斷橋秘徑深入試煉坑探查".to_string(),
                    ));
                }
            }
        } else if knows_pit && bold_enough && a.wound_part.is_none() && a.fatigue <= 36 {
            if !a.rescue_pact_active && a.copper >= 12 && node.name.contains("酒館") {
                return Some(AutoPlayerVerb::WorldAct("buy_rescue_pact", String::new()));
            }
            if (a.soul_seed + clock.game_days.unsigned_abs()) % 4 == 0 {
                let pit_crowd = [bridge_nid, "oi:dng_pit:entry", "oi:dng_pit:lair"]
                    .iter()
                    .filter_map(|nid| delta.node_occupants.get(*nid))
                    .map(|s| s.len())
                    .sum::<usize>();
                if pit_crowd < 3 {
                    return Some(AutoPlayerVerb::WalkToAndLearnNode(
                        bridge_nid.to_string(),
                        "憑酒館聽來的斷橋風聲動身前往灰松斷橋探尋暗門".to_string(),
                    ));
                }
            }
        }
    }

    // 11. 閒暇武友／匠徒同場點到為止切磋（自然催生在場目擊成相！）
    if (node.kind == "plaza" || node.kind == "dock")
        && (5..=7).contains(&clock.bell_index)
        && a.fatigue <= 30
        && a.wound_part.is_none()
        && innate.temperament.approach_avoid >= 10
        && (a
            .soul_seed
            .wrapping_add(clock.game_days.unsigned_abs())
            .wrapping_add(u64::from(clock.bell_index)))
            % 19
            == 0
        && let Some(occ) = delta.node_occupants.get(&a.node_id)
    {
        for oid in occ {
            if oid != aid
                && let Some(pa) = delta.actors.get(oid)
                && pa.alive
                && pa.controller_id.is_none()
                && pa.fatigue <= 35
                && pa.wound_part.is_none()
                && a.mind.ltm.social.get(oid).is_some_and(|s| s.warmth >= 12)
            {
                return Some(AutoPlayerVerb::Extended(
                    "peer_spar",
                    pa.public_actor_token(),
                ));
            }
        }
    }

    // 12. 荒野節點順手搜巡開霧
    if node.kind == "wild" && a.fatigue <= 42 {
        let win_key = format!("{}@{}", a.node_id, clock.six_phase_key);
        if !a.explored_windows.contains(&win_key) {
            return Some(AutoPlayerVerb::Explore);
        }
    }

    None
}

fn travel_cross_town(
    doc: &WorldStaticDoc,
    node: &super::types::SpatialNodeDef,
    copper: i64,
    dst_nid: &str,
    reason: String,
) -> AutoPlayerVerb {
    if super::transit_and_cargo::should_board_ew_carriage(doc, node, copper, dst_nid) {
        let dir_act = if node.id == "oi:white_gull_harbor:plaza" {
            "board_carriage_west"
        } else {
            "board_carriage_east"
        };
        return AutoPlayerVerb::WorldAct(dir_act, String::new());
    }
    AutoPlayerVerb::WalkToAndLearnNode(dst_nid.to_string(), reason)
}

use super::transit_and_cargo::find_local_node_by_kind;

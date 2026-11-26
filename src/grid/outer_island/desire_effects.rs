//! 快思維行動抵達結算、債主意向審查展期、到期債契結算與零玩家外洩之 $O(1)$ 傳聞擴散。

use super::appraisal::{ConcernCause, GoalStrategyKind};
use super::clock_and_lang::SolarClockSnapshot;
use super::household_economy::{
    count_box_home_rations_only, deduct_box_home_rations, effective_home_node,
    try_consume_household_meal, try_restock_household_larder,
};
use super::mind::{EventPublicity, EvidenceTier};
use super::movement::{add_carry_item, take_carry_item};
use super::persona::{ResponseTendency, Situation16D, ValueAxis, derive_innate_persona};
use super::sim_tick::broadcast_scene_event;
use super::types::{CarryItem, IslandWorldDelta, WorldStaticDoc};

#[expect(
    clippy::too_many_arguments,
    reason = "arrival resolution requires world, ledger, and tavern context"
)]
pub fn execute_arrived_desire_effect(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    actor_id: &str,
    kind: &str,
    focus_actor: Option<&str>,
    tav_opt: Option<&str>,
    keeper_opt: Option<&str>,
    abs_bell: i64,
) -> bool {
    match kind {
        "home_meal_rest" => {
            // H1 修正：離線玩家絕不自動扣背囊／木箱糧食；NPC 亦須飢餓 >= 40 才開伙，單純疲勞僅歇息！
            let should_eat = delta.actors.get(actor_id).is_some_and(|a| {
                a.controller_id.is_none()
                    && !a.actor_id.starts_with("pl:")
                    && a.mind.stm.affect.hunger >= 40
            });
            let ate = if should_eat {
                try_consume_household_meal(delta, actor_id, abs_bell)
            } else {
                false
            };
            if let Some(a) = delta.actors.get_mut(actor_id) {
                a.fatigue = a.fatigue.saturating_sub(24);
                a.mind.stm.affect.fatigue = a.mind.stm.affect.fatigue.saturating_sub(24);
                if ate || !should_eat {
                    a.mind.stm.today_goal_progressed = true;
                }
                a.mark_mutated();
            }
            true
        }
        "restock_larder" => {
            try_restock_household_larder(doc, delta, clock, actor_id, tav_opt, keeper_opt)
        }
        "forage_wild" => {
            // L3 修正：以純數值 `last_forage_window` 取代往 `explored_windows` 無上限塞字串！
            let cur_win = clock.game_days * 4 + i64::from(clock.bell_index / 3);
            let Some(a) = delta.actors.get_mut(actor_id) else {
                return false;
            };
            if a.controller_id.is_some() || a.mind.stm.last_forage_window >= cur_win {
                return false;
            }
            let cur_bulk = super::transit_and_cargo::count_backpack_bulk_units(&a.backpack);
            a.mind.stm.last_forage_window = cur_win;
            if let Some(nd) = doc.nodes.get(&a.node_id) {
                a.record_step_visit(&nd.id, nd.cell_id, false);
                let cell = doc.cells.get(&nd.cell_id.to_string());
                let (slug, name, w) = if cur_bulk >= 2 {
                    ("herb_bundle", "山林止血草束", 1)
                } else if cell.is_some_and(|c| c.is_coast) {
                    ("driftwood", "異國刻紋漂流木", 2)
                } else if cell.is_some_and(|c| c.height >= 23) {
                    ("iron_ore", "粗鐵礦石", 3)
                } else {
                    ("herb_bundle", "山林止血草束", 1)
                };
                add_carry_item(
                    &mut a.backpack,
                    CarryItem {
                        slug: slug.to_string(),
                        name: name.to_string(),
                        count: 1,
                        weight_units: w,
                        ..CarryItem::default()
                    },
                );
                a.fatigue = (a.fatigue.saturating_add(8)).min(92);
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
            }
            true
        }
        "sell_foraged" => {
            // 鋪主／工匠若親自採得自家鋪倉緊缺物料（炭、鐵、藥草、麥穀）且正站在自己鋪內，優先直接補入自家貨倉復工！
            let self_restock_slug: Option<(String, String)> =
                delta.actors.get(actor_id).and_then(|a| {
                    let wnd = doc.nodes.get(&a.work_node)?;
                    if wnd.kind != "shop" || a.node_id != a.work_node {
                        return None;
                    }
                    let wanted: &[&str] = if a.role.contains("鍛鐵") || a.role.contains("鐵匠")
                    {
                        if super::live_commodities::count_smithy_fuel(delta, &a.work_node) < 8 {
                            &[
                                "black_coal",
                                "charcoal_sack",
                                "driftwood",
                                "timber_log",
                                "iron_ore",
                            ]
                        } else if super::live_commodities::count_shop_stock_by_slugs(
                            delta,
                            &a.work_node,
                            &["iron_ore"],
                        ) < 6
                        {
                            &["iron_ore"]
                        } else {
                            &[]
                        }
                    } else if a.role.contains("草藥")
                        && super::live_commodities::count_apothecary_herbs(delta, &a.work_node) < 6
                    {
                        &["herb_bundle", "quarantine_wormwood"]
                    } else if a.role.contains("酒館")
                        && super::live_commodities::count_tavern_ration_stock(
                            doc,
                            delta,
                            wnd.cell_id,
                            &a.work_node,
                        ) < 20
                    {
                        &["wheat_grain"]
                    } else {
                        &[]
                    };
                    wanted.iter().find_map(|s| {
                        a.backpack
                            .iter()
                            .any(|i| {
                                i.slug == *s
                                    && i.count > 0
                                    && i.bound_owner.is_none()
                                    && i.stolen_from.is_none()
                            })
                            .then(|| ((*s).to_string(), a.work_node.clone()))
                    })
                });
            if let Some((r_slug, wnode)) = self_restock_slug {
                let popped_item = delta.actors.get_mut(actor_id).and_then(|a| {
                    let mut item = take_carry_item(&mut a.backpack, &r_slug, 1)?;
                    item.count = 1;
                    Some((a.display_name.clone(), item))
                });
                if let Some((dname_self, item)) = popped_item {
                    let rn = item.name.clone();
                    let cur_cell = doc.nodes.get(&wnode).map_or(4616, |n| n.cell_id);
                    super::live_commodities::store_purchased_specialty_in_town(
                        doc, delta, actor_id, cur_cell, item,
                    );
                    if let Some(a_after) = delta.actors.get_mut(actor_id) {
                        a_after.mind.resolve_concerns_matching(
                            ConcernCause::LivelihoodShortfall,
                            None,
                            28,
                        );
                        a_after.mind.stm.today_goal_progressed = true;
                        a_after.mark_mutated();
                    }
                    super::sim_tick::broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &wnode,
                        actor_id,
                        &dname_self,
                        EventPublicity::PublicLoud,
                        Some("harbor_trade"),
                        30,
                        10,
                        ValueAxis::CraftPride,
                        "npc_self_restock",
                        &format!("{dname_self}親手將採回的【{rn}】添入鋪內貨倉備料"),
                    );
                    return true;
                }
            }

            let item_opt = delta.actors.get(actor_id).and_then(|a| {
                let cur_nd = doc.nodes.get(&a.node_id)?;
                [
                    ("iron_ore", 5_i64),
                    ("black_coal", 4_i64),
                    ("charcoal_sack", 4_i64),
                    ("timber_log", 4_i64),
                    ("driftwood", 4_i64),
                    ("herb_bundle", 4_i64),
                    ("quarantine_wormwood", 4_i64),
                ]
                .into_iter()
                .find_map(|(s, base_p)| {
                    a.backpack
                        .iter()
                        .find(|i| {
                            i.slug == s
                                && i.count > 0
                                && i.bound_owner.is_none()
                                && i.stolen_from.is_none()
                        })
                        .map(|i| {
                            let dyn_p = super::live_commodities::quote_specialty_buyback_copper(
                                doc, delta, cur_nd, None, i,
                            )
                            .unwrap_or(base_p);
                            (
                                s.to_string(),
                                i.name.clone(),
                                i.weight_units,
                                dyn_p,
                                a.display_name.clone(),
                                a.copper,
                                super::rural_migrate::nearest_core_town_cell_id(
                                    doc,
                                    doc.nodes.get(&a.home_node).map_or(4616, |n| n.cell_id),
                                ),
                            )
                        })
                })
            });
            let Some((slug, iname, w_units, price, dname, copper_before, cell_id)) = item_opt
            else {
                return false;
            };
            // R3-M4 修正：山海物料 100% 由本鎮真實工匠／藥師／掌櫃以銅幣收購並原物入鋪箱（上限 36），絕不憑空變口糧或被公庫吞沒！
            let buyer_opt: Option<(String, String)> = delta
                .actors
                .iter()
                .find_map(|(bid, ba)| {
                    if bid == actor_id
                        || !ba.alive
                        || ba.controller_id.is_some()
                        || bid.starts_with("pl:")
                        || ba.copper < price + 12
                        || super::rural_migrate::nearest_core_town_cell_id(
                            doc,
                            doc.nodes.get(&ba.home_node).map_or(0, |n| n.cell_id),
                        ) != cell_id
                    {
                        return None;
                    }
                    let role_match = match slug.as_str() {
                        "iron_ore" | "black_coal" | "charcoal_sack" => {
                            ba.role.contains("鍛鐵") || ba.role.contains("石工")
                        }
                        "driftwood" | "timber_log" => {
                            ba.role.contains("木匠")
                                || ba.role.contains("車行")
                                || (ba.role.contains("鍛鐵")
                                    && super::live_commodities::count_smithy_fuel(
                                        delta,
                                        &ba.work_node,
                                    ) < 6)
                        }
                        _ => ba.role.contains("草藥") || ba.role == "酒館掌櫃",
                    };
                    if !role_match {
                        return None;
                    }
                    let cur_cnt: u32 = ba.inn_lockers.get(&ba.work_node).map_or(0, |v| {
                        v.iter().filter(|i| i.slug == slug).map(|i| i.count).sum()
                    });
                    (cur_cnt < 36).then(|| (bid.clone(), ba.work_node.clone()))
                })
                .or_else(|| {
                    let kid = keeper_opt?;
                    let ka = delta.actors.get(kid)?;
                    if kid == actor_id || ka.copper < price + 12 {
                        return None;
                    }
                    let wkey = tav_opt.unwrap_or(ka.work_node.as_str()).to_string();
                    let cur_cnt: u32 = ka.inn_lockers.get(&wkey).map_or(0, |v| {
                        v.iter().filter(|i| i.slug == slug).map(|i| i.count).sum()
                    });
                    (cur_cnt < 36).then(|| (kid.to_string(), wkey))
                });
            let cur_win = clock.game_days * 4 + i64::from(clock.bell_index / 3);
            let Some((buyer_id, buyer_box)) = buyer_opt else {
                // R4-H1 修正：暫無收購匠鋪時退避 2 個時窗回本業上工，絕不站在市集每響空等！
                if let Some(a) = delta.actors.get_mut(actor_id) {
                    a.mind.stm.last_forage_window = cur_win + 2;
                    a.mark_mutated();
                }
                return false;
            };
            if delta.transfer_copper(
                clock.game_days,
                clock.bell_index,
                "npc_sell_foraged",
                &buyer_id,
                actor_id,
                price,
                format!("{}出售採集所得【{}】貼補家用", dname, iname),
            ) {
                let transferred = delta
                    .actors
                    .get_mut(actor_id)
                    .and_then(|a| take_carry_item(&mut a.backpack, &slug, 1));
                if let Some(mut item) = transferred {
                    item.count = 1;
                    item.weight_units = w_units;
                    if let Some(buyer) = delta.actors.get_mut(&buyer_id) {
                        let bv = buyer.inn_lockers.entry(buyer_box).or_default();
                        add_carry_item(bv, item);
                        buyer.mark_mutated();
                    }
                }
                if let Some(a) = delta.actors.get_mut(actor_id) {
                    a.mind
                        .resolve_concerns_matching(ConcernCause::LivelihoodShortfall, None, 32);
                    if copper_before < 12 {
                        let innate = derive_innate_persona(a.soul_seed.max(1));
                        let _ = a.mind.appraise_event(
                            &innate,
                            ConcernCause::PactHonored,
                            None,
                            Some(ValueAxis::TangibleGain),
                            36,
                            abs_bell,
                        );
                    }
                    a.mind.stm.today_goal_progressed = true;
                    a.mark_mutated();
                }
                if let Some(nid) = delta.actors.get(actor_id).map(|a| a.node_id.clone()) {
                    super::sim_tick::broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &nid,
                        actor_id,
                        &dname,
                        EventPublicity::PublicLoud,
                        Some("harbor_trade"),
                        34,
                        12,
                        ValueAxis::TangibleGain,
                        "npc_sell_foraged",
                        &format!("{}將採得的【{}】過秤賣給鋪子換取銅板", dname, iname),
                    );
                }
                return true;
            }
            false
        }
        "visit_confidant" => {
            let Some(fid) = focus_actor else {
                return false;
            };
            if fid.starts_with("pl:") {
                return false;
            }
            let my_node = delta.actors.get(actor_id).map(|a| a.node_id.clone());
            let friend_state = delta.actors.get(fid).and_then(|f| {
                if f.controller_id.is_some() {
                    return None;
                }
                let mn = my_node.as_deref()?;
                let rev_w = f.mind.ltm.social.get(actor_id).map_or(10, |s| s.warmth);
                let f_home = effective_home_node(f).to_string();
                let can_spare = count_box_home_rations_only(f, &f_home) >= 5;
                Some((f.alive && f.node_id == mn, rev_w, can_spare, f_home))
            });
            let Some((co_present, rev_warmth, can_spare, f_home)) = friend_state else {
                return false;
            };
            if !co_present {
                return false;
            }
            let needy_hunger = delta
                .actors
                .get(actor_id)
                .is_some_and(|a| a.mind.stm.affect.hunger >= 55);
            let mut shared_food = false;
            if rev_warmth >= 25
                && can_spare
                && needy_hunger
                && let Some(fmut) = delta.actors.get_mut(fid)
                && deduct_box_home_rations(fmut, &f_home, 1)
            {
                fmut.mark_mutated();
                shared_food = true;
            }
            if let Some(a) = delta.actors.get_mut(actor_id) {
                let innate = derive_innate_persona(a.soul_seed.max(1));
                if rev_warmth >= 18 {
                    if shared_food {
                        a.mind.stm.affect.hunger = a.mind.stm.affect.hunger.saturating_sub(42);
                        a.mind.resolve_concerns_matching(
                            ConcernCause::LivelihoodShortfall,
                            None,
                            25,
                        );
                    }
                    let had_heavy_mood = a
                        .mind
                        .stm
                        .affect
                        .fear
                        .max(a.mind.stm.affect.anger)
                        .max(a.mind.stm.affect.grief)
                        >= 26;
                    let on_aid_strat = a
                        .mind
                        .stm
                        .goal_strategy
                        .as_ref()
                        .is_some_and(|g| g.kind == GoalStrategyKind::KinMutualAid);
                    if shared_food || had_heavy_mood || on_aid_strat {
                        let _ = a.mind.appraise_event(
                            &innate,
                            ConcernCause::KinshipWarmth,
                            Some(fid),
                            None,
                            if shared_food { 54 } else { 40 },
                            abs_bell,
                        );
                    }
                    if on_aid_strat {
                        a.mind.stm.goal_strategy = None;
                        a.mind.stm.active_goal_tag.clear();
                    }
                    a.mind.stm.today_goal_progressed = true;
                } else if rev_warmth <= -18 {
                    let _ = a.mind.appraise_event(
                        &innate,
                        ConcernCause::KinshipRift,
                        Some(fid),
                        None,
                        42,
                        abs_bell,
                    );
                }
                a.mark_mutated();
            }
            true
        }
        "craft_hone" => {
            if let Some(a) = delta.actors.get_mut(actor_id) {
                let innate = derive_innate_persona(a.soul_seed.max(1));
                let repaired = a.bound_weapon.wear_stage > 0;
                if repaired {
                    a.bound_weapon.wear_stage = a.bound_weapon.wear_stage.saturating_sub(1);
                }
                // 工匠／藥師閉門鍛打配藥時消耗鋪箱過剩山海原料（維持百日庫存動態平衡）
                let wnode = a.work_node.clone();
                if let Some(wv) = a.inn_lockers.get_mut(&wnode) {
                    for raw_slug in ["iron_ore", "driftwood", "herb_bundle"] {
                        let cnt: u32 = wv
                            .iter()
                            .filter(|i| i.slug == raw_slug)
                            .map(|i| i.count)
                            .sum();
                        if cnt > 10 {
                            let _ = take_carry_item(wv, raw_slug, 1);
                            break;
                        }
                    }
                }
                if a.bound_weapon.grip_familiarity < 35 {
                    a.bound_weapon.grip_familiarity =
                        a.bound_weapon.grip_familiarity.saturating_add(1);
                }
                a.mind
                    .resolve_concerns_matching(ConcernCause::ToolDamaged, None, 35);
                if repaired {
                    let _ = a.mind.appraise_event(
                        &innate,
                        ConcernCause::CraftBreakthrough,
                        None,
                        None,
                        34,
                        abs_bell,
                    );
                }
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
                return true;
            }
            false
        }
        "tavern_unwind" => {
            // R2-L2 + R2-M1 修正：杜絕 `delta.actors[actor_id]` 直接索引，且日常點一杯麥酒僅舒緩疲勞／打聽外鎮消息，絕不灌滿心事槽！
            let dname_opt = delta
                .actors
                .get(actor_id)
                .filter(|a| a.copper >= 8)
                .map(|a| a.display_name.clone());
            let paid = if let (Some(kid), Some(dname)) = (keeper_opt, dname_opt)
                && kid != actor_id
            {
                delta.transfer_copper(
                    clock.game_days,
                    clock.bell_index,
                    "tavern_ale",
                    actor_id,
                    kid,
                    1,
                    format!("{dname}在酒館點了一杯麥酒歇腳打聽風聲"),
                )
            } else {
                false
            };
            if let Some(a) = delta.actors.get_mut(actor_id) {
                a.fatigue = a.fatigue.saturating_sub(16);
                a.mind.stm.affect.fatigue = a.mind.stm.affect.fatigue.saturating_sub(16);
                if paid {
                    a.mind
                        .resolve_concerns_matching(ConcernCause::CrossTownYearning, None, 22);
                    if a.mind
                        .stm
                        .goal_strategy
                        .as_ref()
                        .is_some_and(|g| g.kind == GoalStrategyKind::SeekCrossTownNews)
                    {
                        a.mind.stm.goal_strategy = None;
                        a.mind.stm.active_goal_tag.clear();
                    }
                }
                a.mind.stm.today_goal_progressed = true;
                a.mark_mutated();
            }
            if paid
                && let Some((nid, dname)) = delta
                    .actors
                    .get(actor_id)
                    .map(|a| (a.node_id.clone(), a.display_name.clone()))
            {
                super::sim_tick::broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &nid,
                    actor_id,
                    &dname,
                    EventPublicity::PublicLoud,
                    Some("harbor_trade"),
                    32,
                    10,
                    ValueAxis::KinshipLoyalty,
                    "tavern_ale",
                    &format!("{dname}在酒館櫃前點了杯麥酒歇腳攀談風聲"),
                );
            }
            true
        }
        "confront_chase" | "plead_extension" => {
            let Some(tid) = focus_actor else {
                return false;
            };
            let pair = {
                let Some(a) = delta.actors.get(actor_id) else {
                    return false;
                };
                let Some(t) = delta.actors.get(tid) else {
                    return false;
                };
                if a.node_id != t.node_id {
                    return false;
                }
                let cred_inn = derive_innate_persona(t.soul_seed.max(1));
                let cred_w = t.mind.ltm.social.get(actor_id).map_or(0, |s| s.warmth);
                let is_pl_target = t.controller_id.is_some() || tid.starts_with("pl:");
                (
                    a.node_id.clone(),
                    a.display_name.clone(),
                    if is_pl_target {
                        "外來旅人".to_string()
                    } else {
                        t.display_name.clone()
                    },
                    is_pl_target,
                    cred_inn,
                    t.drift.clone(),
                    cred_w,
                )
            };
            let (nid, aname, tname, is_pl_target, cred_inn, cred_drift, cred_w) = pair;
            if kind == "plead_extension" {
                // M8 修正：債主為真實玩家時絕不單方面改期；NPC 債主須依脾氣審核，且每紙契約終生最多寬限 1 次！
                if is_pl_target {
                    return false;
                }
                let sit = Situation16D::new(
                    0,
                    15,
                    30,
                    i32::from(cred_w),
                    -10,
                    15,
                    20,
                    20,
                    35,
                    0,
                    25,
                    0,
                    15,
                    15,
                    20,
                    15,
                );
                let cred_tend = cred_inn.choose_dominant_tendency(&sit, &cred_drift);
                let approved = cred_w > -25
                    && matches!(
                        cred_tend,
                        ResponseTendency::ShelterWeak
                            | ResponseTendency::Bargain
                            | ResponseTendency::ComplyOrder
                            | ResponseTendency::SeekAlly
                    );
                let mut granted = false;
                if approved {
                    for c in delta.contracts.values_mut() {
                        if c.debtor_id == actor_id
                            && c.creditor_id == tid
                            && (c.status == "open" || c.status == "defaulted")
                            && !c.note_zh.contains("〔已寬限〕")
                        {
                            c.due_day = c.due_day.max(clock.game_days + 2);
                            c.note_zh.push_str("〔已寬限〕");
                            delta.dirty_contracts.insert(c.contract_id.clone());
                            granted = true;
                            break;
                        }
                    }
                }
                if granted {
                    broadcast_scene_event(
                        doc,
                        delta,
                        clock,
                        &nid,
                        actor_id,
                        &aname,
                        EventPublicity::TableWhisper,
                        Some("harbor_trade"),
                        40,
                        30,
                        ValueAxis::PactIntegrity,
                        "plead_ext_ok",
                        &format!("{aname}低聲向{tname}懇請寬限，獲准順延兩日清帳"),
                    );
                    if let Some(a) = delta.actors.get_mut(actor_id) {
                        a.mind.stm.today_goal_progressed = true;
                        a.mark_mutated();
                    }
                } else {
                    // R4-M4 修正：遭債主拒絕寬限後寫入「〔拒寬限〕」標記，避免欠戶每響重複登門死纏！
                    for c in delta.contracts.values_mut() {
                        if c.debtor_id == actor_id
                            && c.creditor_id == tid
                            && (c.status == "open" || c.status == "defaulted")
                            && !c.note_zh.contains("〔拒寬限〕")
                        {
                            c.note_zh.push_str("〔拒寬限〕");
                            delta.dirty_contracts.insert(c.contract_id.clone());
                            break;
                        }
                    }
                    if let Some(a) = delta.actors.get_mut(actor_id) {
                        let a_inn = derive_innate_persona(a.soul_seed.max(1));
                        let _ = a.mind.appraise_event(
                            &a_inn,
                            ConcernCause::DebtDefaulted,
                            Some(tid),
                            None,
                            50,
                            abs_bell,
                        );
                        a.mark_mutated();
                    }
                }
            } else {
                // R2-H1 + R3-M1 修正：雙向 4 日冷卻剎車，且冷暖具均值回歸出口（深怨宣洩後微幅緩和 +3，防單向沉底至 -95）！
                let a_inn = delta
                    .actors
                    .get(actor_id)
                    .map(|a| derive_innate_persona(a.soul_seed.max(1)));
                let calm_temper = a_inn.as_ref().is_some_and(|ai| {
                    i16::from(ai.temperament.ignition) - i16::from(ai.temperament.damping)
                        + i16::from(cred_inn.temperament.ignition)
                        - i16::from(cred_inn.temperament.damping)
                        >= 0
                });
                let stamp = (clock.game_days + 10_000).max(1);
                if let Some(a) = delta.actors.get_mut(actor_id) {
                    a.mind.stm.last_self_confront_stamp = stamp;
                    if let Some(soc) = a.mind.ltm.social.get_mut(tid) {
                        soc.last_confront_day = stamp;
                        let shift: i8 = if soc.warmth <= -38 || calm_temper {
                            3
                        } else {
                            -2
                        };
                        soc.warmth =
                            (i16::from(soc.warmth) + i16::from(shift)).clamp(-95, 95) as i8;
                    }
                    a.mind
                        .resolve_concerns_matching(ConcernCause::RivalEncounter, Some(tid), 18);
                    a.mark_mutated();
                }
                if let Some(t) = delta.actors.get_mut(tid) {
                    t.mind.stm.last_self_confront_stamp = stamp;
                    if let Some(soc) = t.mind.ltm.social.get_mut(actor_id) {
                        soc.last_confront_day = stamp;
                        let shift: i8 = if soc.warmth <= -38 || calm_temper {
                            3
                        } else {
                            -2
                        };
                        soc.warmth =
                            (i16::from(soc.warmth) + i16::from(shift)).clamp(-95, 95) as i8;
                    }
                    t.mark_mutated();
                }
                broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &nid,
                    actor_id,
                    &aname,
                    EventPublicity::PublicLoud,
                    Some("harbor_trade"),
                    50,
                    46,
                    ValueAxis::Autonomy,
                    "confront_scene",
                    &format!("{aname}沉著臉攔住{tname}理論舊事，場間氣氛一時緊繃"),
                );
            }
            true
        }
        _ => false,
    }
}

pub fn settle_due_contracts(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let due_keys: Vec<String> = delta
        .contracts
        .iter()
        .filter(|(_, c)| {
            (c.status == "open" || c.status == "defaulted") && clock.game_days >= c.due_day
        })
        .map(|(k, _)| k.clone())
        .collect();

    for ckey in due_keys {
        let Some(c) = delta.contracts.get(&ckey).cloned() else {
            continue;
        };
        let both_alive = delta
            .actor_by_id_or_controller(&c.debtor_id)
            .is_some_and(|a| a.alive)
            && delta
                .actor_by_id_or_controller(&c.creditor_id)
                .is_some_and(|a| a.alive);
        if !both_alive {
            if let Some(cmut) = delta.contracts.get_mut(&ckey) {
                cmut.status = "written_off".to_string();
                cmut.note_zh.push_str("〔當事人已故結案〕");
                delta.dirty_contracts.insert(ckey);
            }
            continue;
        }
        let (d_real_name, d_pub_name) = delta.actor_by_id_or_controller(&c.debtor_id).map_or(
            ("欠債人".to_string(), "欠債人".to_string()),
            |a| {
                let pub_n = if a.controller_id.is_some() || c.debtor_id.starts_with("pl:") {
                    "外來旅人".to_string()
                } else {
                    a.display_name.clone()
                };
                (a.display_name.clone(), pub_n)
            },
        );
        let (creditor_is_player, c_pub_name) =
            delta.actor_by_id_or_controller(&c.creditor_id).map_or(
                (c.creditor_id.starts_with("pl:"), "債主".to_string()),
                |a| {
                    let is_pl = a.controller_id.is_some() || c.creditor_id.starts_with("pl:");
                    let pub_n = if is_pl {
                        "外來旅人".to_string()
                    } else {
                        a.display_name.clone()
                    };
                    (is_pl, pub_n)
                },
            );

        if delta.transfer_copper(
            clock.game_days,
            clock.bell_index,
            "contract_settle",
            &c.debtor_id,
            &c.creditor_id,
            c.amount_copper,
            format!("{d_pub_name}如期結清欠{c_pub_name}的帳款"),
        ) {
            if let Some(cmut) = delta.contracts.get_mut(&ckey) {
                cmut.status = "settled".to_string();
                delta.dirty_contracts.insert(ckey.clone());
            }
            if !creditor_is_player
                && let Some(cr) = delta.actor_mut_by_id_or_controller(&c.creditor_id)
            {
                let inn = derive_innate_persona(cr.soul_seed.max(1));
                cr.mind.record_pact_or_trade_evidence(
                    &c.debtor_id,
                    &d_real_name,
                    clock.game_days,
                    true,
                    false,
                    "準時結清舊帳",
                );
                cr.mind.resolve_concerns_matching(
                    ConcernCause::DebtPressured,
                    Some(&c.debtor_id),
                    45,
                );
                let _ = cr.mind.appraise_event(
                    &inn,
                    ConcernCause::PactHonored,
                    Some(&c.debtor_id),
                    None,
                    38,
                    clock.game_days * 12 + i64::from(clock.bell_index),
                );
                cr.mark_mutated();
            }
            if let Some(deb) = delta.actor_mut_by_id_or_controller(&c.debtor_id) {
                deb.mind.resolve_concerns_matching(
                    ConcernCause::DebtDefaulted,
                    Some(&c.creditor_id),
                    60,
                );
                deb.mark_mutated();
            }
        } else {
            let abs_bell = clock.game_days * 12 + i64::from(clock.bell_index);
            if let Some(deb) = delta.actor_mut_by_id_or_controller(&c.debtor_id)
                && deb.controller_id.is_none()
            {
                let deb_inn = derive_innate_persona(deb.soul_seed.max(1));
                let _ = deb.mind.appraise_event(
                    &deb_inn,
                    ConcernCause::DebtDefaulted,
                    Some(&c.creditor_id),
                    None,
                    54,
                    abs_bell,
                );
                deb.mark_mutated();
            }
            // R2-L6 + R2-L7 修正：玩家當債主絕不代行催債發言；NPC 記憶槽保留真實玩家姓名；逾限催討滿 2 回轉黑冊結案防無限展延！
            let strategy_note = if !creditor_is_player
                && let Some(cr) = delta.actor_mut_by_id_or_controller(&c.creditor_id)
            {
                let inn = derive_innate_persona(cr.soul_seed.max(1));
                let sit = Situation16D::new(
                    15, 20, 20, -35, -30, 25, 40, 35, 45, 20, 0, 10, 20, 0, 25, 10,
                );
                let tend = inn.choose_dominant_tendency(&sit, &cr.drift);
                let action_desc = match tend {
                    ResponseTendency::Confront => {
                        format!("{c_pub_name}當街攔住{d_pub_name}拍桌催討欠帳")
                    }
                    ResponseTendency::SeekAlly => {
                        format!("{c_pub_name}在酒館向街坊公開點名{d_pub_name}拖欠帳款")
                    }
                    ResponseTendency::Bargain | ResponseTendency::ComplyOrder => {
                        format!("{c_pub_name}要求{d_pub_name}以打雜勞力折抵逾期帳款")
                    }
                    ResponseTendency::ShelterWeak => {
                        format!("{c_pub_name}嘆了口氣准許{d_pub_name}寬限兩日再還")
                    }
                    _ => format!("{c_pub_name}冷著臉把{d_pub_name}記上拒賒黑冊"),
                };
                cr.mind.record_pact_or_trade_evidence(
                    &c.debtor_id,
                    &d_real_name,
                    clock.game_days,
                    false,
                    false,
                    &action_desc,
                );
                let _ = cr.mind.appraise_event(
                    &inn,
                    ConcernCause::DebtPressured,
                    Some(&c.debtor_id),
                    None,
                    52,
                    abs_bell,
                );
                cr.mark_mutated();
                Some((cr.node_id.clone(), action_desc))
            } else {
                None
            };
            let mut newly_defaulted = false;
            if let Some(cmut) = delta.contracts.get_mut(&ckey) {
                if cmut.note_zh.contains("〔催討一回〕") {
                    cmut.status = "written_off".to_string();
                    cmut.note_zh.push_str("〔逾限入黑冊結案〕");
                } else {
                    cmut.due_day = clock.game_days + 2;
                    cmut.status = "defaulted".to_string();
                    cmut.note_zh.push_str("〔催討一回〕");
                    newly_defaulted = true;
                }
                delta.dirty_contracts.insert(ckey.clone());
            }
            if newly_defaulted {
                super::living_chronicler::record_pact_default(
                    doc,
                    delta,
                    clock,
                    &ckey,
                    &c.creditor_id,
                    &c.debtor_id,
                    c.amount_copper,
                );
            }
            if let Some((nid, desc)) = strategy_note {
                broadcast_scene_event(
                    doc,
                    delta,
                    clock,
                    &nid,
                    &c.creditor_id,
                    &c_pub_name,
                    EventPublicity::PublicLoud,
                    Some("harbor_trade"),
                    52,
                    45,
                    ValueAxis::PactIntegrity,
                    "debt_chase",
                    &desc,
                );
            }
        }
    }
}

/// M3 修正：傳聞擴散嚴格排除玩家角色（`pl:` / `controller_id`）代發言與玩家相關私密情節外流！
pub fn propagate_gossip_o1(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
) {
    let crowded_nodes: Vec<(String, Vec<String>)> = delta
        .node_occupants
        .iter()
        .filter(|(_, set)| set.len() >= 2)
        .map(|(nid, set)| (nid.clone(), set.iter().cloned().collect()))
        .collect();

    for (nid, occupants) in crowded_nodes {
        let pick_idx = ((clock.game_days.unsigned_abs() * 7 + u64::from(clock.bell_index))
            as usize)
            % occupants.len();
        let speaker_id = &occupants[pick_idx];
        let Some(speaker) = delta.actors.get(speaker_id) else {
            continue;
        };
        if !speaker.alive
            || !speaker.setup_done
            || speaker.controller_id.is_some()
            || speaker_id.starts_with("pl:")
        {
            continue;
        }
        let Some(best_ep) = speaker
            .mind
            .ltm
            .episodic
            .iter()
            .filter(|e| {
                clock.game_days - e.origin_day <= 12
                    && e.salience >= 48
                    && !e.subject_actor_id.starts_with("pl:")
                    && delta
                        .actors
                        .get(&e.subject_actor_id)
                        .is_none_or(|ta| ta.controller_id.is_none())
            })
            .max_by_key(|e| e.retention_score(clock.game_days))
            .cloned()
        else {
            continue;
        };

        let sp_lang = match speaker.race.as_str() {
            "矮人" if nid.contains("forge") || nid.contains("smithy") => "dwarven",
            "精靈" if nid.contains("herb") || nid.contains("pine") => "elvish",
            "獸人" if nid.contains("dock") && (clock.bell_index % 2 == 1) => "orcish",
            _ => "harbor_trade",
        };
        let sp_name = speaker.display_name.clone();
        // R2-L8 修正：剝到最內層「：」並裁掉外層引號，徹底消滅巢狀「聽某某提起：聽某某提起：…」
        let core_text = best_ep
            .subjective_zh
            .rsplit_once('：')
            .map_or(best_ep.subjective_zh.as_str(), |(_, rhs)| rhs)
            .trim_matches(|c| c == '「' || c == '」' || c == '。');
        let clean_core = super::slow_think::mask_player_names_in_text(delta, core_text);
        let hearsay_text = format!("聽{}提起：{}", sp_name, clean_core);

        let (sp_has_pit, sp_has_cleared, sp_has_casualty) = (
            speaker.known_rumors.contains("pit_rumor"),
            speaker.known_rumors.contains("pit_cleared"),
            speaker.known_rumors.contains("pit_casualty"),
        );
        let start_offset = ((clock.game_days.unsigned_abs() * 3 + u64::from(clock.bell_index))
            as usize)
            % occupants.len();
        for step in 0..occupants.len().min(4) {
            let listener_id = &occupants[(start_offset + step) % occupants.len()];
            if listener_id == speaker_id {
                continue;
            }
            let ingested_hearsay = if let Some(listener) = delta.actors.get_mut(listener_id) {
                if !listener.alive || !listener.setup_done {
                    false
                } else {
                    let langs = listener.languages.clone();
                    if listener.mind.ingest_sensory_event(
                        &langs,
                        EventPublicity::TableWhisper,
                        Some(sp_lang),
                        false,
                        &best_ep.event_id,
                        clock.game_days,
                        clock.bell_index,
                        &nid,
                        &best_ep.subject_actor_id,
                        &best_ep.subject_name,
                        EvidenceTier::TavernHearsay,
                        best_ep.salience.saturating_sub(8),
                        35,
                        ValueAxis::PactIntegrity,
                        &hearsay_text,
                    ) {
                        // 雙向秘境風聲傳播（聽懂語言者方能獲悉斷橋秘徑、坑空與傷退消息，零全知作弊！）
                        if sp_has_pit {
                            listener.known_rumors.insert("pit_rumor".to_string());
                        }
                        if sp_has_cleared {
                            listener.known_rumors.insert("pit_cleared".to_string());
                        } else if sp_has_pit {
                            listener.known_rumors.remove("pit_cleared");
                        }
                        if sp_has_casualty {
                            listener.known_rumors.insert("pit_casualty".to_string());
                        }
                        listener.mark_mutated();
                        true
                    } else {
                        false
                    }
                }
            } else {
                false
            };
            if ingested_hearsay {
                let rumor_key = if sp_has_pit {
                    "pit_rumor"
                } else {
                    best_ep.event_id.as_str()
                };
                super::map_memory::ingest_hearsay_rumor_pin(
                    doc,
                    delta,
                    listener_id,
                    rumor_key,
                    &hearsay_text,
                    clock.game_days,
                );
            }
        }
    }
}

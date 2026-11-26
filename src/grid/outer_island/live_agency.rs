//! 銀鷗島第三刀：撤除櫃台保母硬鎖、店鋪斷料自採／懸賞補倉、居民自主修橋墾荒清瘴與依城鎮缺口轉行引擎。

use super::appraisal::GoalStrategyKind;
use super::clock_and_lang::SolarClockSnapshot;
use super::household_economy::count_household_larder_rations;
use super::live_commodities::{
    count_apothecary_herbs, count_shop_stock_by_slugs, count_smithy_fuel,
    count_tavern_ration_stock, is_apothecary_out_of_herbs, is_smithy_forge_cold,
};
use super::persona::{ValueAxis, derive_innate_persona};
use super::rural_migrate::nearest_core_town_cell_id;
use super::sim_tick::default_tool_for_role;
use super::types::{IslandWorldDelta, SpatialNodeDef, UnifiedMindBody, WorldStaticDoc};

pub enum AgencyActionPlan {
    WorldAct(&'static str, String),
    WalkTo(String, String),
}

fn has_active_pact_of_kind(delta: &IslandWorldDelta, aid: &str, kind: &str) -> bool {
    delta.contracts.values().any(|c| {
        c.creditor_id == aid
            && c.kind == kind
            && matches!(c.status.as_str(), "escrowed" | "in_progress")
    })
}

fn has_clean_slug(a: &UnifiedMindBody, slugs: &[&str]) -> bool {
    a.backpack.iter().any(|i| {
        slugs.contains(&i.slug.as_str())
            && i.count > 0
            && i.bound_owner.is_none()
            && i.stolen_from.is_none()
    })
}

/// 取代早期寫死的「搭檔不在就強制站死櫃台」保母鎖：改由店鋪實料供需與價值取向自然調整效用分數 `(work_adj, forage_adj)`。
pub fn compute_post_duty_and_shortage_adjustment(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    bell_index: u8,
    tavern_stock: u32,
) -> (i32, i32) {
    let at_shop = doc
        .nodes
        .get(&a.work_node)
        .is_some_and(|n| n.kind == "shop");
    let is_day = (3..=9).contains(&bell_index);
    if at_shop {
        if (a.role.contains("鍛鐵") || a.role.contains("鐵匠"))
            && is_smithy_forge_cold(delta, &a.work_node)
        {
            return (-32, 36);
        }
        if a.role.contains("草藥") && is_apothecary_out_of_herbs(delta, &a.work_node) {
            return (-32, 36);
        }
        if a.role.contains("酒館掌櫃") && is_day && tavern_stock == 0 {
            return (-28, 34);
        }
    }

    let is_tavern_staff = a.role.contains("酒館掌櫃");
    let is_day_counter = matches!(
        a.role.as_str(),
        "鍛鐵匠" | "鍛鐵匠學徒" | "草藥師" | "草藥師學徒" | "港務記籌手"
    ) && is_day;
    if !(is_tavern_staff || is_day_counter) || a.node_id != a.work_node {
        return (0, 0);
    }
    let coworker_present = delta.node_occupants.get(&a.work_node).is_some_and(|occ| {
        occ.iter().any(|oid| {
            oid != &a.actor_id
                && delta.actors.get(oid).is_some_and(|o| {
                    o.alive
                        && o.setup_done
                        && o.controller_id.is_none()
                        && o.node_id == a.work_node
                        && o.work_node == a.work_node
                })
        })
    });
    if coworker_present {
        return (0, 0);
    }
    let innate = derive_innate_persona(a.soul_seed.max(1));
    let top_val = innate.top_value_axis(&a.drift);
    let duty_bonus = match top_val {
        ValueAxis::OrderHierarchy | ValueAxis::PactIntegrity | ValueAxis::CraftPride => 10,
        _ => 4,
    };
    (18 + duty_bonus, 0)
}

/// 鋪面斷料時主動於城鎮發起實物收購委託契（`supply_commission`），讓缺煤鐵匠或缺草藥師向全鎮公開懸賞收貨。
pub fn evaluate_shop_shortage_supply_pact(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    aid: &str,
) -> Option<&'static str> {
    let a = delta.actors.get(aid)?;
    if a.controller_id.is_some()
        || aid.starts_with("pl:")
        || has_active_pact_of_kind(delta, aid, "supply_commission")
    {
        return None;
    }
    let wnode = doc.nodes.get(&a.work_node)?;
    if wnode.kind != "shop" {
        return None;
    }
    if a.role == "鍛鐵匠" {
        if is_smithy_forge_cold(delta, &a.work_node) && a.copper >= 16 {
            return Some("black_coal");
        }
        if count_shop_stock_by_slugs(delta, &a.work_node, &["iron_ore"]) == 0 && a.copper >= 22 {
            return Some("iron_ore");
        }
    } else if a.role == "草藥師" {
        if is_apothecary_out_of_herbs(delta, &a.work_node) && a.copper >= 16 {
            return Some("herb_bundle");
        }
    } else if a.role == "酒館掌櫃"
        && count_tavern_ration_stock(doc, delta, wnode.cell_id, &a.work_node) < 6
        && a.copper >= 20
    {
        return Some("wheat_grain");
    }
    None
}

fn find_outdoor_node_in_cell(
    doc: &WorldStaticDoc,
    cid: u32,
    prefer_kinds: &[&str],
) -> Option<String> {
    for &k in prefer_kinds {
        if let Some((nid, _)) = doc
            .nodes
            .iter()
            .find(|(_, n)| n.cell_id == cid && !n.indoor && !n.dungeon && n.kind == k)
        {
            return Some(nid.clone());
        }
    }
    doc.nodes
        .iter()
        .find(|(_, n)| n.cell_id == cid && !n.indoor && !n.dungeon)
        .map(|(nid, _)| nid.clone())
}

fn find_best_damaged_bridge_cell(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    cur_node: &SpatialNodeDef,
) -> Option<u32> {
    let mut cands: Vec<u32> = vec![cur_node.cell_id];
    if let Some(cell) = doc.cells.get(&cur_node.cell_id.to_string()) {
        for &nb in &cell.neighbors_f6 {
            if !cands.contains(&nb) {
                cands.push(nb);
            }
        }
    }
    for nid in &a.mind.ltm.known_nodes {
        if let Some(nd) = doc.nodes.get(nid)
            && !cands.contains(&nd.cell_id)
        {
            cands.push(nd.cell_id);
        }
    }
    cands
        .into_iter()
        .filter_map(|cid| {
            let cl = delta.live_land.cells.get(&cid)?;
            if cl.has_river_crossing && cl.bridge_integrity < 80 {
                Some((cl.bridge_integrity, cid))
            } else {
                None
            }
        })
        .min()
        .map(|(_, cid)| cid)
}

fn find_nearby_timber_forest_node(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    cur_cid: u32,
) -> Option<String> {
    let mut cands = vec![cur_cid];
    if let Some(cell) = doc.cells.get(&cur_cid.to_string()) {
        for &nb in &cell.neighbors_f6 {
            if !cands.contains(&nb) {
                cands.push(nb);
            }
        }
    }
    for nid in &a.mind.ltm.known_nodes {
        if let Some(nd) = doc.nodes.get(nid)
            && !cands.contains(&nd.cell_id)
        {
            cands.push(nd.cell_id);
        }
    }
    for cid in cands {
        if delta
            .live_land
            .cells
            .get(&cid)
            .is_some_and(|cl| cl.forest_density >= 24)
            && let Some(nid) =
                find_outdoor_node_in_cell(doc, cid, &["wild", "road", "village_gate", "gate"])
        {
            return Some(nid);
        }
    }
    None
}

fn find_nearby_reclaimable_land(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    a: &UnifiedMindBody,
    cur_cid: u32,
) -> Option<(u32, String)> {
    let mut cands = vec![cur_cid];
    if let Some(cell) = doc.cells.get(&cur_cid.to_string()) {
        for &nb in &cell.neighbors_f6 {
            if !cands.contains(&nb) {
                cands.push(nb);
            }
        }
    }
    for nid in &a.mind.ltm.known_nodes {
        if let Some(nd) = doc.nodes.get(nid)
            && !cands.contains(&nd.cell_id)
        {
            cands.push(nd.cell_id);
        }
    }
    for cid in cands {
        let cell_ok = doc
            .cells
            .get(&cid.to_string())
            .is_some_and(|c| c.kind != "settlement" && c.height <= 24);
        let land_ok = delta.live_land.cells.get(&cid).is_some_and(|cl| {
            cl.plague_miasmas < 30 && cl.flood_level == 0 && cl.reclaimed_plots < 4
        });
        if cell_ok
            && land_ok
            && let Some(nid) =
                find_outdoor_node_in_cell(doc, cid, &["wild", "village_gate", "road", "yard"])
        {
            return Some((cid, nid));
        }
    }
    None
}

/// 評估居民第三刀自主救災與民事行動：焚艾散瘴、伐木備料修橋、缺糧墾荒。
pub fn evaluate_civic_and_land_agency(
    doc: &WorldStaticDoc,
    delta: &IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> Option<AgencyActionPlan> {
    if !delta.live_land.seeded {
        return None;
    }
    let a = delta.actors.get(aid)?;
    if !a.alive
        || a.controller_id.is_some()
        || aid.starts_with("pl:")
        || a.wound_part.is_some()
        || a.fatigue >= 68
    {
        return None;
    }
    let cur_node = doc.nodes.get(&a.node_id)?;
    if cur_node.dungeon {
        return None;
    }
    let innate = derive_innate_persona(a.soul_seed.max(1));
    let top_val = innate.top_value_axis(&a.drift);
    let strat_kind = a.mind.stm.goal_strategy.as_ref().map(|g| g.kind);

    // 1. 身帶南岸苦艾且所在處有殘留疫氣 -> 順手燃艾驅散
    if !cur_node.indoor
        && delta
            .live_land
            .cells
            .get(&cur_node.cell_id)
            .is_some_and(|cl| cl.plague_miasmas > 0)
        && has_clean_slug(a, &["quarantine_wormwood"])
    {
        return Some(AgencyActionPlan::WorldAct(
            "burn_wormwood_miasma",
            String::new(),
        ));
    }

    // 2. 斷橋／損橋搶修與先伐木備料
    let civic_minded = strat_kind == Some(GoalStrategyKind::RepairCivicWorks)
        || a.role.contains("石工")
        || a.role.contains("木匠")
        || a.role.contains("車行")
        || a.role.contains("苦力")
        || top_val == ValueAxis::OrderHierarchy;
    if civic_minded
        && delta.municipal_treasury_copper >= 6
        && let Some(br_cid) = find_best_damaged_bridge_cell(doc, delta, a, cur_node)
    {
        let has_bridge_mat = has_clean_slug(a, &["timber_log", "driftwood", "quarry_stone"]);
        if has_bridge_mat {
            if cur_node.cell_id == br_cid && !cur_node.indoor {
                return Some(AgencyActionPlan::WorldAct(
                    "repair_river_bridge",
                    br_cid.to_string(),
                ));
            }
            if let Some(br_nid) =
                find_outdoor_node_in_cell(doc, br_cid, &["road", "village_gate", "gate", "wild"])
            {
                return Some(AgencyActionPlan::WalkTo(
                    br_nid,
                    "扛木前往受損溪橋地段動工搶修".to_string(),
                ));
            }
        } else if let Some(forest_nid) =
            find_nearby_timber_forest_node(doc, delta, a, cur_node.cell_id)
        {
            if forest_nid == a.node_id {
                return Some(AgencyActionPlan::WorldAct(
                    "fell_timber_clear",
                    String::new(),
                ));
            }
            return Some(AgencyActionPlan::WalkTo(
                forest_nid,
                "赴鄰近林地伐木備料以搶修斷橋".to_string(),
            ));
        }
    }

    // 3. 區域酒館存糧吃緊或掛「墾荒種糧」方針時，開墾附近肥沃平坡地塊
    let town_cid = nearest_core_town_cell_id(doc, cur_node.cell_id);
    let town_food_tight =
        super::household_economy::find_town_tavern_and_keeper(doc, delta, town_cid)
            .0
            .is_some_and(|tn| count_tavern_ration_stock(doc, delta, town_cid, &tn) < 15);
    let agri_minded = strat_kind == Some(GoalStrategyKind::ReclaimLand)
        || a.role.contains("農")
        || a.role.contains("牧")
        || a.role.contains("獵")
        || (town_food_tight
            && !doc
                .nodes
                .get(&a.work_node)
                .is_some_and(|n| n.kind == "shop"));
    if agri_minded
        && a.fatigue < 55
        && (strat_kind == Some(GoalStrategyKind::ReclaimLand)
            || (a.soul_seed.wrapping_add(clock.game_days.unsigned_abs())) % 4 == 0)
        && let Some((land_cid, land_nid)) =
            find_nearby_reclaimable_land(doc, delta, a, cur_node.cell_id)
    {
        let _ = land_cid;
        if a.node_id == land_nid {
            return Some(AgencyActionPlan::WorldAct(
                "reclaim_farmland",
                String::new(),
            ));
        }
        return Some(AgencyActionPlan::WalkTo(
            land_nid,
            "見城鄉口糧吃緊，趕赴近郊平坡翻土開墾田壟".to_string(),
        ));
    }

    None
}

/// 檢查居民是否處於破產滯銷、斷糧極困或主動謀變狀態，該轉行貼補或承接城鎮緊缺營生。
pub fn should_trigger_livelihood_shift(delta: &IslandWorldDelta, a: &UnifiedMindBody) -> bool {
    if a.controller_id.is_some()
        || a.actor_id.starts_with("pl:")
        || !a.alive
        || a.role.contains("掌櫃")
    {
        return false;
    }
    let strat_shift = a
        .mind
        .stm
        .goal_strategy
        .as_ref()
        .is_some_and(|g| g.kind == GoalStrategyKind::ShiftLivelihood);
    if strat_shift && a.copper <= 6 {
        return true;
    }
    if a.copper == 0
        && !a.role.contains("苦力")
        && delta
            .contracts
            .values()
            .any(|c| c.debtor_id == a.actor_id && c.status == "written_off")
    {
        return true;
    }
    let larder = count_household_larder_rations(delta, a);
    let severe_shortfall = a.mind.stm.focus_concerns.iter().flatten().any(|c| {
        c.cause == super::appraisal::ConcernCause::LivelihoodShortfall && c.intensity >= 68
    });
    if a.copper <= 1 && larder == 0 && severe_shortfall && a.role != "農夫" {
        return true;
    }
    if a.role == "碼頭苦力" && a.copper >= 90 {
        let innate = derive_innate_persona(a.soul_seed.max(1));
        if matches!(
            innate.top_value_axis(&a.drift),
            ValueAxis::TangibleGain | ValueAxis::CuriosityVenture
        ) {
            return true;
        }
    }
    false
}

/// 依所在主鎮實際物資缺口與港口條件執行真實轉行（漁／農／木匠／碼頭苦力／行腳商旅），寫入生平轉折與傳記。
pub fn apply_autonomous_livelihood_shift(
    doc: &WorldStaticDoc,
    delta: &mut IslandWorldDelta,
    clock: &SolarClockSnapshot,
    aid: &str,
) -> bool {
    let Some(a_ro) = delta.actors.get(aid) else {
        return false;
    };
    let my_cell = doc.nodes.get(&a_ro.home_node).map_or(4616, |n| n.cell_id);
    let town_cid = nearest_core_town_cell_id(doc, my_cell);
    let dock_nid_opt = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == town_cid && n.kind == "dock")
        .map(|(id, _)| id.clone());
    let plaza_nid_opt = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == town_cid && n.kind == "plaza")
        .map(|(id, _)| id.clone());
    let rural_gate_opt = find_outdoor_node_in_cell(doc, my_cell, &["village_gate", "gate", "wild"])
        .or_else(|| plaza_nid_opt.clone());

    let tavern_short = super::household_economy::find_town_tavern_and_keeper(doc, delta, town_cid)
        .0
        .is_some_and(|tn| count_tavern_ration_stock(doc, delta, town_cid, &tn) < 12);
    let smith_cold = doc
        .nodes
        .iter()
        .find(|(_, n)| {
            n.cell_id == town_cid
                && n.kind == "shop"
                && (n.name.contains("鍛鐵") || n.name.contains("鐵匠"))
        })
        .is_some_and(|(sid, _)| count_smithy_fuel(delta, sid) < 6);
    let herb_dry = doc
        .nodes
        .iter()
        .find(|(_, n)| n.cell_id == town_cid && n.kind == "shop" && n.name.contains("草藥"))
        .is_some_and(|(hid, _)| count_apothecary_herbs(delta, hid) < 3);

    let (new_role, new_work, lesson, goal_tag, is_dock_shift) =
        if a_ro.role == "碼頭苦力" && a_ro.copper >= 90 {
            let dst = plaza_nid_opt
                .or_else(|| dock_nid_opt.clone())
                .unwrap_or_else(|| a_ro.home_node.clone());
            (
                "行腳商旅",
                dst,
                "扛貨攢足本錢，轉作跑街行腳商旅互通各鎮有無",
                "forage_supplement",
                false,
            )
        } else if tavern_short && !super::household_economy::is_food_producer_role(&a_ro.role) {
            if let Some(dk) = dock_nid_opt.filter(|_| {
                doc.cells
                    .get(&my_cell.to_string())
                    .is_some_and(|c| c.is_coast)
            }) {
                (
                    "漁夫",
                    dk,
                    "見鎮上酒館斷糧價貴，下海捕魚貼補口糧兼賣鮮魚",
                    "reclaim_land",
                    false,
                )
            } else {
                let dst = rural_gate_opt.unwrap_or_else(|| a_ro.home_node.clone());
                (
                    "農夫",
                    dst,
                    "見市集口糧吃緊，改行扛鋤拓田種穀自給並餘糧售鎮",
                    "reclaim_land",
                    false,
                )
            }
        } else if smith_cold && a_ro.role != "木匠" {
            let dst = rural_gate_opt.unwrap_or_else(|| a_ro.home_node.clone());
            (
                "木匠",
                dst,
                "見鍛爐精煤木炭見底，改操鋸斧進山伐木燒炭供爐",
                "repair_civic",
                false,
            )
        } else if herb_dry && a_ro.role != "採藥人" {
            let dst = rural_gate_opt.unwrap_or_else(|| a_ro.home_node.clone());
            (
                "採藥人",
                dst,
                "見藥堂藥草短缺，改背採藥鐮進山林採艾貼補家計",
                "forage_supplement",
                false,
            )
        } else if let Some(dk) = dock_nid_opt.filter(|_| a_ro.role != "碼頭苦力") {
            (
                "碼頭苦力",
                dk,
                "舊業虧空難支，索性捲起衣袖改赴碼頭扛重活重頭攢起",
                "clear_debt",
                true,
            )
        } else {
            let dst = rural_gate_opt.unwrap_or_else(|| a_ro.home_node.clone());
            (
                "農夫",
                dst,
                "手頭吃緊難續舊業，挽起袖子改赴坡地墾田種麥重開生計",
                "reclaim_land",
                false,
            )
        };

    if new_role == a_ro.role {
        return false;
    }
    let work_for_chronicle = new_work.clone();
    if let Some(a) = delta.actors.get_mut(aid) {
        a.role = new_role.to_string();
        a.work_node = new_work.clone();
        if !a.mind.ltm.known_nodes.contains(&new_work) {
            a.mind.ltm.known_nodes.insert(new_work.clone());
        }
        if a.bound_weapon.rune_mark.is_none() && a.bound_weapon.provenance.contains("慣用") {
            a.bound_weapon = default_tool_for_role(new_role, &a.display_name);
        }
        if a.mind
            .stm
            .goal_strategy
            .as_ref()
            .is_some_and(|g| g.kind == GoalStrategyKind::ShiftLivelihood)
        {
            a.mind.stm.goal_strategy = None;
            a.mind.stm.active_goal_tag.clear();
        }
        a.mind.push_turning_point(
            &format!("career:shift:{}:{}:{}", aid, new_role, clock.game_days),
            clock.game_days,
            lesson,
            goal_tag,
        );
        a.mind.stm.today_goal_progressed = true;
        let _bio_ok =
            super::world_weave_bio::refresh_actor_biography_if_needed(doc, a, clock.game_days);
        a.mark_mutated();
    }
    if is_dock_shift {
        super::living_chronicler::record_career_shift_dock(
            doc,
            delta,
            clock,
            aid,
            &work_for_chronicle,
        );
    }
    true
}
